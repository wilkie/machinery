; exceptions.asm - Tests protected-mode exception generation (#GP, #NP)
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; IMPORTANT: The simulator's protection checks are sequential. After one check
; fires an exception (e.g. #GP), subsequent checks in the same instruction still
; execute. Tests must ensure that only ONE check fails per faulting instruction
; to avoid cascading into double faults.

; Memory layout:
;   0x0000-0x003F: GDT (8 entries, limit=0x3F)
;   0x0040-0x0047: Dummy descriptor at beyond-GDT-limit address
;                  (looks valid so no cascade after bounds-check #GP)
;   0x0800-0x086F: IDT (covers vectors 0-0x0D)

section .text
start:

    xor ax, ax
    mov ds, ax

    ; =========================================================
    ; Set up GDT at linear address 0 (8 entries, limit 0x3F)
    ;
    ;   0 (0x00): Null
    ;   1 (0x08): Code DPL=0, exec/read, base=0, limit=0xFFFF
    ;   2 (0x10): Data DPL=0, r/w, base=0, limit=0xFFFF
    ;   3 (0x18): Stack DPL=0, r/w, base=0, limit=0xFFFF
    ;   4 (0x20): Code exec-only DPL=0, base=0, limit=0xFFFF
    ;   5 (0x28): Data DPL=0, r/w, P=0, base=0, limit=0xFFFF
    ;   6 (0x30): Data DPL=0, r/w, base=0, limit=0x00FF
    ;   7 (0x38): System descriptor (S=0, TSS type)
    ; =========================================================

    ; Entry 0: Null
    mov word [0x00], 0x0000
    mov word [0x02], 0x0000
    mov word [0x04], 0x0000
    mov word [0x06], 0x0000

    ; Entry 1: Code DPL=0 (0x08) - exec/read
    mov word [0x08], 0xFFFF
    mov word [0x0A], 0x0000
    mov byte [0x0C], 0x00
    mov byte [0x0D], 0x9A
    mov word [0x0E], 0x0000

    ; Entry 2: Data DPL=0 (0x10) - read/write, flat
    mov word [0x10], 0xFFFF
    mov word [0x12], 0x0000
    mov byte [0x14], 0x00
    mov byte [0x15], 0x92
    mov word [0x16], 0x0000

    ; Entry 3: Stack DPL=0 (0x18) - read/write, flat
    mov word [0x18], 0xFFFF
    mov word [0x1A], 0x0000
    mov byte [0x1C], 0x00
    mov byte [0x1D], 0x92
    mov word [0x1E], 0x0000

    ; Entry 4: Code exec-only DPL=0 (0x20)
    ; Access 0x98: P=1, DPL=0, S=1, type=100 (exec, not readable)
    mov word [0x20], 0xFFFF
    mov word [0x22], 0x0000
    mov byte [0x24], 0x00
    mov byte [0x25], 0x98
    mov word [0x26], 0x0000

    ; Entry 5: Data DPL=0, NOT PRESENT (0x28)
    ; Access 0x12: P=0, DPL=0, S=1, type=001
    mov word [0x28], 0xFFFF
    mov word [0x2A], 0x0000
    mov byte [0x2C], 0x00
    mov byte [0x2D], 0x12
    mov word [0x2E], 0x0000

    ; Entry 6: Data DPL=0 (0x30) - small limit (0x00FF)
    mov word [0x30], 0x00FF
    mov word [0x32], 0x0000
    mov byte [0x34], 0x00
    mov byte [0x35], 0x92
    mov word [0x36], 0x0000

    ; Entry 7: System descriptor (0x38) - S=0
    ; Access 0x81: P=1, DPL=0, S=0, A=1, type=000 (available TSS)
    mov word [0x38], 0x002B
    mov word [0x3A], 0x2000
    mov byte [0x3C], 0x00
    mov byte [0x3D], 0x81
    mov word [0x3E], 0x0000

    ; Dummy descriptor at 0x40 (beyond GDT limit)
    ; Looks like a valid data segment so subsequent checks don't cascade
    ; after the bounds-check #GP fires
    mov word [0x40], 0xFFFF
    mov word [0x42], 0x0000
    mov byte [0x44], 0x00
    mov byte [0x45], 0x92       ; P=1, DPL=0, S=1, data r/w
    mov word [0x46], 0x0000

    ; =========================================================
    ; Set up IDT at linear address 0x0800
    ; Vectors 0x0B (#NP) and 0x0D (#GP)
    ; IDT limit covers vectors 0-0x0D: 14 entries, limit = 0x6F
    ; =========================================================

    ; Vector 0x0B (#NP) at 0x0800 + 0x0B*8 = 0x0858
    mov word [0x0858], np_handler
    mov word [0x085A], 0x0008
    mov byte [0x085C], 0x00
    mov byte [0x085D], 0x86             ; interrupt gate
    mov word [0x085E], 0x0000

    ; Vector 0x0D (#GP) at 0x0800 + 0x0D*8 = 0x0868
    mov word [0x0868], gp_handler
    mov word [0x086A], 0x0008
    mov byte [0x086C], 0x00
    mov byte [0x086D], 0x86             ; interrupt gate
    mov word [0x086E], 0x0000

    ; =========================================================
    ; Load GDT and IDT, enter protected mode
    ; =========================================================
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
    ; Test 1: #GP from selector beyond GDT limit
    ; Selector 0x40 (index 8) exceeds GDT limit 0x3F
    ; A dummy valid-looking descriptor at 0x40 prevents cascading
    ; =========================================================
    mov byte [exception_vector], 0
    mov word [exception_error], 0xFFFF

    mov ax, 0x40
    mov ds, ax                 ; #GP: out of bounds

    ; Restore DS to flat segment
    mov ax, 0x10
    mov ds, ax

    ; Verify #GP fired with error code = 0x40
    mov al, byte [exception_vector]
    mov ah, 0x0D
    int 0x22
    mov ax, word [exception_error]
    mov bx, 0x0040
    int 0x23

    ; =========================================================
    ; Test 2: #GP from loading execute-only code segment into DS
    ; Selector 0x20: code segment, type=100 (exec, not readable)
    ; Only the "exec-only code into data register" check fires
    ; =========================================================
    mov byte [exception_vector], 0
    mov word [exception_error], 0xFFFF

    mov ax, 0x20
    mov ds, ax                 ; #GP: exec-only code into DS

    mov ax, 0x10
    mov ds, ax

    mov al, byte [exception_vector]
    mov ah, 0x0D
    int 0x22
    mov ax, word [exception_error]
    mov bx, 0x0020
    int 0x23

    ; =========================================================
    ; Test 3: #NP from loading not-present data segment into DS
    ; Selector 0x28: data r/w, P=0
    ; All type/DPL checks pass, only the present check fires #NP
    ; =========================================================
    mov byte [exception_vector], 0
    mov word [exception_error], 0xFFFF

    mov ax, 0x28
    mov ds, ax                 ; #NP: not present

    mov ax, 0x10
    mov ds, ax

    ; Verify #NP (vector 0x0B) with error code = 0x28
    mov al, byte [exception_vector]
    mov ah, 0x0B
    int 0x22
    mov ax, word [exception_error]
    mov bx, 0x0028
    int 0x23

    ; =========================================================
    ; Test 4: #GP from loading system descriptor (S=0) into DS
    ; Selector 0x38: S=0, type=000 (TSS-like system descriptor)
    ; Only the S bit check fires
    ; =========================================================
    mov byte [exception_vector], 0
    mov word [exception_error], 0xFFFF

    mov ax, 0x38
    mov ds, ax                 ; #GP: system descriptor into DS

    mov ax, 0x10
    mov ds, ax

    mov al, byte [exception_vector]
    mov ah, 0x0D
    int 0x22
    mov ax, word [exception_error]
    mov bx, 0x0038
    int 0x23

    ; =========================================================
    ; Test 5: #GP from segment limit violation on memory access
    ; Selector 0x30: data r/w, limit=0x00FF
    ; Access at offset 0x100 exceeds the 0xFF limit
    ; Only one limit check fires (offset+1 > LIMIT_MAX)
    ; =========================================================
    mov byte [exception_vector], 0
    mov word [exception_error], 0xFFFF

    mov ax, 0x30
    mov ds, ax                 ; load small-limit segment (succeeds)

    mov si, 0x0100
    mov ax, word [si]         ; #GP: offset 0x100+1 > limit 0xFF

    mov ax, 0x10
    mov ds, ax

    mov al, byte [exception_vector]
    mov ah, 0x0D
    int 0x22

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

; =========================================================
; #GP handler (vector 0x0D)
; Stack layout: [SP]=error_code, [SP+2]=ret_IP, [SP+4]=ret_CS, [SP+6]=FLAGS
; DS cache is still valid (old or updated with base=0), so writes work
; =========================================================
gp_handler:
    push ax
    push bx
    ; Restore DS to flat data segment — the faulting instruction may have
    ; left DS with a bad cache (e.g. system descriptor with tiny limit)
    mov ax, 0x10
    mov ds, ax
    mov bx, sp
    ; Error code is at [BX+4] (past pushed AX and BX)
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
    add sp, 2                  ; pop error code
    iret

; =========================================================
; #NP handler (vector 0x0B)
; =========================================================
np_handler:
    push ax
    push bx
    mov ax, 0x10
    mov ds, ax
    mov bx, sp
    mov ax, word [ss:bx+4]
    mov word [exception_error], ax
    mov byte [exception_vector], 0x0B
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
    dw 0x3F                     ; limit: 8 entries * 8 - 1
    dw 0x0000                   ; base low
    db 0x00                     ; base high

idt_ptr:
    dw 0x6F                     ; limit: 14 entries * 8 - 1 (covers vector 0x0D)
    dw 0x0800                   ; base low
    db 0x00                     ; base high

exception_vector: db 0
exception_error:  dw 0
