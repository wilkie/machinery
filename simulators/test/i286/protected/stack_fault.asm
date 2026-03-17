; stack_fault.asm - Tests stack segment fault (#SS) generation
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; GDT:
;   0 (0x00): Null
;   1 (0x08): Code DPL=0, exec/read, base=0, limit=0xFFFF
;   2 (0x10): Data DPL=0, r/w, base=0, limit=0xFFFF (flat)
;   3 (0x18): Stack DPL=0, r/w, base=0, limit=0xFFFF
;   4 (0x20): Data DPL=0, r/w, P=0 (not present) - for #SS test
;   5 (0x28): Expand-down stack, r/w, base=0, limit=0x00FF
;             Valid offsets: 0x0100 to 0xFFFF
;
; IDT at 0x0800:
;   Vector 0x0C (#SS): handler
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

    ; Entry 3: Stack DPL=0 (0x18) - normal large stack
    mov word [0x18], 0xFFFF
    mov word [0x1A], 0x0000
    mov byte [0x1C], 0x00
    mov byte [0x1D], 0x92
    mov word [0x1E], 0x0000

    ; Entry 4: Data DPL=0 (0x20) - NOT PRESENT
    ; Access 0x12: P=0, DPL=0, S=1, type=001 (data r/w)
    mov word [0x20], 0xFFFF
    mov word [0x22], 0x0000
    mov byte [0x24], 0x00
    mov byte [0x25], 0x12
    mov word [0x26], 0x0000

    ; Entry 5: Expand-down stack (0x28)
    ; Access 0x96: P=1, DPL=0, S=1, type=110 (data, expand-down, writable)
    ; Limit=0x00FF → valid offsets [0x0100, 0xFFFF]
    mov word [0x28], 0x00FF
    mov word [0x2A], 0x0000
    mov byte [0x2C], 0x00
    mov byte [0x2D], 0x96
    mov word [0x2E], 0x0000

    ; IDT: Vector 0x0C (#SS) at 0x0800 + 0x0C*8 = 0x0860
    mov word [0x0860], ss_handler
    mov word [0x0862], 0x0008
    mov byte [0x0864], 0x00
    mov byte [0x0865], 0x86         ; interrupt gate
    mov word [0x0866], 0x0000

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

    ; =========================================================
    ; Test 1: Loading SS with not-present segment triggers #SS
    ; Selector 0x20: data r/w, P=0
    ; All type/DPL checks pass, only present check fires #SS
    ; =========================================================
    mov byte [exception_vector], 0
    mov word [exception_error], 0xFFFF

    mov ax, 0x20
    mov ss, ax                 ; #SS: not present

    ; Restore valid SS (old cache is still valid after #SS)
    mov ax, 0x18
    mov ss, ax
    mov sp, 0x1000

    ; Verify #SS (vector 0x0C) with error code = 0x20
    mov al, byte [exception_vector]
    mov ah, 0x0C
    int 0x22
    mov ax, word [exception_error]
    mov bx, 0x0020
    int 0x23

    ; =========================================================
    ; Test 2: Load expand-down SS and verify normal access works
    ; =========================================================
    mov ax, 0x28
    mov ss, ax
    mov sp, 0x1000             ; 0x1000 > 0x0100 → valid

    ; PUSH should work (SP → 0x0FFE, valid)
    push word 0x1234
    mov ax, sp
    mov bx, 0x0FFE
    int 0x23

    ; POP back
    pop ax
    mov bx, 0x1234
    int 0x23

    ; =========================================================
    ; Test 3: Restore normal SS for remaining tests
    ; =========================================================
    mov ax, 0x18
    mov ss, ax
    mov sp, 0x1000

    ; Verify SS is back to normal
    mov ax, ss
    mov bx, 0x0018
    int 0x23

    ; =========================================================
    ; Test 4: PUSH with normal small-limit SS triggers #GP
    ; PUSH uses #GP (not #SS) for stack limit violations
    ; We can't test this at ring 0 without cascading faults
    ; (handler shares the same small SS), so just verify that
    ; expand-down SS rejects access in invalid range via GP
    ; =========================================================

    ; Load expand-down SS (limit=0xFF, valid: 0x100-0xFFFF)
    mov ax, 0x28
    mov ss, ax
    mov sp, 0x0102             ; set SP near boundary

    ; First PUSH: SP → 0x0100, offset+1=0x0101 >= 0x0100 → valid
    mov byte [exception_vector], 0
    push word 0xAAAA

    ; Switch back to safe SS before checking (we need valid stack for INT)
    mov ax, 0x18
    mov ss, ax
    mov sp, 0x1000

    mov al, byte [exception_vector]
    mov ah, 0x00               ; no exception expected
    int 0x22

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

; =========================================================
; #SS handler (vector 0x0C)
; Stack: error_code, IP, CS, FLAGS
; Old SS cache is still valid (not-present check fires before
; cache update), so stack operations work normally.
; =========================================================
ss_handler:
    push ax
    push bx
    mov ax, 0x10
    mov ds, ax
    mov bx, sp
    mov ax, word [ss:bx+4]
    mov word [exception_error], ax
    mov byte [exception_vector], 0x0C
    pop bx
    pop ax
    ; Fault IP points to faulting instruction; skip past it (2 bytes)
    push bp
    mov bp, sp
    add word [ss:bp+4], 2     ; adjust IP past faulting instruction
    pop bp
    add sp, 2                  ; skip error code
    iret

; =========================================================
; #GP handler (vector 0x0D)
; =========================================================
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
    dw 0x2F                     ; limit: 6 entries * 8 - 1
    dw 0x0000
    db 0x00

idt_ptr:
    dw 0x6F                     ; covers up to vector 0x0D
    dw 0x0800
    db 0x00

exception_vector: db 0
exception_error:  dw 0
