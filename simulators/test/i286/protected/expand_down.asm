; expand_down.asm - Tests expand-down segment limit behavior
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; GDT:
;   0 (0x00): Null
;   1 (0x08): Code DPL=0, exec/read, base=0, limit=0xFFFF
;   2 (0x10): Data DPL=0, r/w, base=0, limit=0xFFFF (flat)
;   3 (0x18): Stack DPL=0, r/w, base=0, limit=0xFFFF
;   4 (0x20): Expand-down data, r/w, base=0, limit=0x00FF
;             Valid offsets: 0x0100 to 0xFFFF
;
; IDT at 0x0800:
;   Vector 0x0D (#GP): handler

section .text
start:

    xor ax, ax
    mov ds, ax

    ; Entry 0: Null
    mov word [0x00], 0x0000
    mov word [0x02], 0x0000
    mov word [0x04], 0x0000
    mov word [0x06], 0x0000

    ; Entry 1: Code DPL=0 (0x08)
    mov word [0x08], 0xFFFF
    mov word [0x0A], 0x0000
    mov byte [0x0C], 0x00
    mov byte [0x0D], 0x9A
    mov word [0x0E], 0x0000

    ; Entry 2: Data DPL=0 (0x10) - flat
    mov word [0x10], 0xFFFF
    mov word [0x12], 0x0000
    mov byte [0x14], 0x00
    mov byte [0x15], 0x92
    mov word [0x16], 0x0000

    ; Entry 3: Stack DPL=0 (0x18)
    mov word [0x18], 0xFFFF
    mov word [0x1A], 0x0000
    mov byte [0x1C], 0x00
    mov byte [0x1D], 0x92
    mov word [0x1E], 0x0000

    ; Entry 4: Expand-down data (0x20)
    ; Access 0x96: P=1, DPL=0, S=1, type=110 (data, expand-down, writable)
    ; Limit=0x00FF → valid range [0x0100, 0xFFFF]
    mov word [0x20], 0x00FF
    mov word [0x22], 0x0000
    mov byte [0x24], 0x00
    mov byte [0x25], 0x96
    mov word [0x26], 0x0000

    ; IDT: Vector 0x0D (#GP) at 0x0800 + 0x0D*8 = 0x0868
    mov word [0x0868], gp_handler
    mov word [0x086A], 0x0008
    mov byte [0x086C], 0x00
    mov byte [0x086D], 0x86
    mov word [0x086E], 0x0000

    ; Load GDT/IDT, enter protected mode
    push cs
    pop ds
    lgdt [gdt_ptr]
    lidt [idt_ptr]

    mov ax, 0x0001
    lmsw ax
    jmp 0x08:protected_entry

protected_entry:
    mov ax, 0x10
    mov ds, ax
    mov ax, 0x18
    mov ss, ax
    mov sp, 0x1000

    ; Load expand-down segment into ES
    mov ax, 0x20
    mov es, ax

    ; =========================================================
    ; Test 1: Write at valid offset (0x1000) through expand-down
    ; 0x1000 >= 0x0100 (LIMIT_MIN) → valid
    ; =========================================================
    mov word [0x1000], 0x0000       ; clear via flat DS
    mov si, 0x1000
    mov word [es:si], 0xABCD

    ; Verify via flat DS
    mov ax, word [0x1000]
    mov bx, 0xABCD
    int 0x23

    ; =========================================================
    ; Test 2: Read at valid offset (0x2000)
    ; =========================================================
    mov word [0x2000], 0xCAFE
    mov si, 0x2000
    mov ax, word [es:si]
    mov bx, 0xCAFE
    int 0x23

    ; =========================================================
    ; Test 3: Byte access at lowest valid offset (0x0100)
    ; 0x0100 >= 0x0100 → valid
    ; =========================================================
    mov byte [0x0100], 0x42
    mov si, 0x0100
    mov al, byte [es:si]
    mov ah, 0x42
    int 0x22

    ; =========================================================
    ; Test 4: Byte access at offset 0x00FF — should #GP
    ; 0x00FF < 0x0100 (LIMIT_MIN) → #GP
    ; =========================================================
    mov byte [exception_vector], 0
    mov si, 0x00FF
    mov al, byte [es:si]

    mov ax, 0x10
    mov ds, ax
    mov al, byte [exception_vector]
    mov ah, 0x0D
    int 0x22

    ; =========================================================
    ; Test 5: Byte access at offset 0x0000 — should #GP
    ; 0x0000 < 0x0100 → #GP
    ; =========================================================
    mov byte [exception_vector], 0
    mov si, 0x0000
    mov al, byte [es:si]

    mov ax, 0x10
    mov ds, ax
    mov al, byte [exception_vector]
    mov ah, 0x0D
    int 0x22

    ; =========================================================
    ; Test 6: Word access at lowest valid word offset (0x0100)
    ; Check: (offset+1) = 0x0101 >= 0x0100 and <= 0xFFFF → valid
    ; =========================================================
    mov word [0x0100], 0xDEAD
    mov si, 0x0100
    mov ax, word [es:si]
    mov bx, 0xDEAD
    int 0x23

    ; =========================================================
    ; Test 7: Word access at highest valid offset (0xFFFE)
    ; (offset+1) = 0xFFFF <= 0xFFFF → valid
    ; =========================================================
    mov word [0xFFFE], 0x9876
    mov ax, word [es:0xFFFE]
    mov bx, 0x9876
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

gp_handler:
    push ax
    push bx
    mov ax, 0x10
    mov ds, ax
    mov bx, sp
    mov ax, word [ss:bx+4]
    mov word [exception_error], ax
    mov byte [exception_vector], 0x0D
    pop bx
    pop ax
    ; Fault IP points to faulting instruction; skip past it (2 bytes)
    push bp
    mov bp, sp
    add word [ss:bp+4], 2     ; adjust IP past faulting instruction
    pop bp
    add sp, 2
    iret

section .data

gdt_ptr:
    dw 0x27                     ; limit: 5 entries * 8 - 1
    dw 0x0000
    db 0x00

idt_ptr:
    dw 0x6F                     ; covers up to vector 0x0D
    dw 0x0800
    db 0x00

exception_vector: db 0
exception_error:  dw 0
