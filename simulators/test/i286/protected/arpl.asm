; arpl.asm - Tests ARPL (Adjust RPL Field of Selector)
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; GDT:
;   0 (0x00): Null
;   1 (0x08): Code DPL=0, exec/read, base=0, limit=0xFFFF
;   2 (0x10): Data DPL=0, r/w, base=0, limit=0xFFFF (flat)
;   3 (0x18): Stack DPL=0, r/w, base=0, limit=0xFFFF

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

    ; Load GDT, enter protected mode
    push cs
    pop ds
    lgdt [gdt_ptr]

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
    ; Test 1: ARPL with dest RPL < src RPL — adjusts, ZF=1
    ; dest=0x0010 (RPL=0), src=0x0013 (RPL=3)
    ; =========================================================
    mov ax, 0x0010
    mov bx, 0x0013
    arpl ax, bx

    ; AX should be 0x0013 (RPL adjusted to 3)
    mov bx, 0x0013
    int 0x23

    ; ZF should be 1
    pushf
    pop ax
    and ax, 0x0040
    mov bx, 0x0040
    int 0x23

    ; =========================================================
    ; Test 2: ARPL with dest RPL >= src RPL — no change, ZF=0
    ; dest=0x0013 (RPL=3), src=0x0010 (RPL=0)
    ; =========================================================
    mov ax, 0x0013
    mov bx, 0x0010
    arpl ax, bx

    mov bx, 0x0013
    int 0x23

    ; ZF should be 0
    pushf
    pop ax
    and ax, 0x0040
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; Test 3: ARPL with equal RPLs — no change, ZF=0
    ; dest=0x0012 (RPL=2), src=0x0022 (RPL=2)
    ; =========================================================
    mov ax, 0x0012
    mov bx, 0x0022
    arpl ax, bx

    mov bx, 0x0012
    int 0x23

    pushf
    pop ax
    and ax, 0x0040
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; Test 4: ARPL adjusts only RPL bits, preserves upper bits
    ; dest=0xFF00 (RPL=0), src=0x0002 (RPL=2)
    ; Result: 0xFF02
    ; =========================================================
    mov ax, 0xFF00
    mov bx, 0x0002
    arpl ax, bx

    mov bx, 0xFF02
    int 0x23

    pushf
    pop ax
    and ax, 0x0040
    mov bx, 0x0040
    int 0x23

    ; =========================================================
    ; Test 5: ARPL with memory operand
    ; =========================================================
    mov word [arpl_target], 0x0020    ; RPL=0
    mov bx, 0x0023                    ; RPL=3
    arpl [arpl_target], bx

    mov ax, word [arpl_target]
    mov bx, 0x0023
    int 0x23

    pushf
    pop ax
    and ax, 0x0040
    mov bx, 0x0040
    int 0x23

    ; =========================================================
    ; Test 6: ARPL memory operand, no adjustment needed
    ; =========================================================
    mov word [arpl_target], 0x0033    ; RPL=3
    mov bx, 0x0001                    ; RPL=1
    arpl [arpl_target], bx

    mov ax, word [arpl_target]
    mov bx, 0x0033                    ; unchanged
    int 0x23

    pushf
    pop ax
    and ax, 0x0040
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

section .data

gdt_ptr:
    dw 0x1F                     ; limit: 4 entries * 8 - 1
    dw 0x0000                   ; base low
    db 0x00                     ; base high

arpl_target: dw 0
