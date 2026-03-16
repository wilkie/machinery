; conforming.asm - Tests conforming code segment behavior
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; GDT:
;   0 (0x00): Null
;   1 (0x08): Code DPL=0, non-conforming, exec/read, base=0, limit=0xFFFF
;   2 (0x10): Data DPL=0, r/w, base=0, limit=0xFFFF (flat)
;   3 (0x18): Stack DPL=0, r/w, base=0, limit=0xFFFF
;   4 (0x20): TSS, base=0x2000, limit=0x2B
;   5 (0x28): Code DPL=3, non-conforming, exec/read, base=0, limit=0xFFFF
;   6 (0x30): Data DPL=3, r/w, base=0, limit=0xFFFF
;   7 (0x38): Stack DPL=3, r/w, base=0, limit=0xFFFF
;   8 (0x40): Code DPL=0, CONFORMING, exec/read, base=0, limit=0xFFFF
;   9 (0x48): Call gate DPL=3 → 0x08:ring0_return (0 params)

section .text
start:

    xor ax, ax
    mov ds, ax

    ; Entry 0: Null
    mov word [0x00], 0x0000
    mov word [0x02], 0x0000
    mov word [0x04], 0x0000
    mov word [0x06], 0x0000

    ; Entry 1: Code DPL=0 (0x08) - non-conforming, exec/read
    ; Access 0x9A: P=1, DPL=00, S=1, type=101
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

    ; Entry 4: TSS (0x20) - base=0x2000, limit=0x2B
    mov word [0x20], 0x002B
    mov word [0x22], 0x2000
    mov byte [0x24], 0x00
    mov byte [0x25], 0x81
    mov word [0x26], 0x0000

    ; Entry 5: Code DPL=3 (0x28) - non-conforming, exec/read
    ; Access 0xFA: P=1, DPL=11, S=1, type=101
    mov word [0x28], 0xFFFF
    mov word [0x2A], 0x0000
    mov byte [0x2C], 0x00
    mov byte [0x2D], 0xFA
    mov word [0x2E], 0x0000

    ; Entry 6: Data DPL=3 (0x30) - r/w
    ; Access 0xF2: P=1, DPL=11, S=1, type=001
    mov word [0x30], 0xFFFF
    mov word [0x32], 0x0000
    mov byte [0x34], 0x00
    mov byte [0x35], 0xF2
    mov word [0x36], 0x0000

    ; Entry 7: Stack DPL=3 (0x38) - r/w
    mov word [0x38], 0xFFFF
    mov word [0x3A], 0x0000
    mov byte [0x3C], 0x00
    mov byte [0x3D], 0xF2
    mov word [0x3E], 0x0000

    ; Entry 8: Conforming code DPL=0 (0x40) - exec/read
    ; Access 0x9E: P=1, DPL=00, S=1, type=111 (exec, conforming, readable)
    mov word [0x40], 0xFFFF
    mov word [0x42], 0x0000
    mov byte [0x44], 0x00
    mov byte [0x45], 0x9E
    mov word [0x46], 0x0000

    ; Entry 9: Call gate DPL=3 (0x48) → 0x08:ring0_return, 0 params
    ; Access 0xE4: P=1, DPL=11, S=0, A=0, type=010 (call gate)
    mov word [0x48], ring0_return
    mov word [0x4A], 0x0008
    mov byte [0x4C], 0x00
    mov byte [0x4D], 0xE4
    mov word [0x4E], 0x0000

    ; Initialize TSS at 0x2000
    mov word [0x2000 + 2],  0x1000  ; SP0
    mov word [0x2000 + 4],  0x0018  ; SS0
    mov word [0x2000 + 42], 0x0000  ; LDTR

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

    ; Load Task Register
    mov ax, 0x20
    ltr ax

    ; =========================================================
    ; Test 1: JMP FAR to conforming code from ring 0
    ; DPL=0 <= CPL=0 → OK
    ; =========================================================
    jmp 0x40:conf_from_ring0

conf_from_ring0:
    ; =========================================================
    ; Test 2: Verify CS = conforming selector with RPL=CPL
    ; CS = (0x40 & 0xFFFC) | CPL(0) = 0x0040
    ; =========================================================
    mov ax, cs
    mov bx, 0x0040
    int 0x23

    ; =========================================================
    ; Test 3: JMP FAR back to non-conforming ring 0 code
    ; From conforming (CPL=0): DPL=0 = CPL=0 → OK
    ; =========================================================
    jmp 0x08:back_to_ring0

back_to_ring0:
    mov ax, cs
    mov bx, 0x0008
    int 0x23

    ; =========================================================
    ; Test 4: Drop to ring 3 via RETF
    ; =========================================================
    push word 0x003B           ; SS (0x38 | RPL=3)
    push word 0x3000           ; SP
    push word 0x002B           ; CS (0x28 | RPL=3)
    push word ring3_code       ; IP
    retf

ring3_code:
    ; Verify ring 3
    mov ax, cs
    and ax, 0x0003
    mov bx, 0x0003
    int 0x23

    ; Load ring 3 data segment
    mov ax, 0x0033
    mov ds, ax

    ; =========================================================
    ; Test 5: JMP FAR to conforming code from ring 3
    ; Conforming: DPL=0 <= CPL=3 → OK (conforming allows this)
    ; CPL stays 3
    ; =========================================================
    jmp 0x40:conf_from_ring3

conf_from_ring3:
    ; =========================================================
    ; Test 6: Verify CS.RPL = 3 (CPL preserved)
    ; CS = (0x40 & 0xFFFC) | CPL(3) = 0x0043
    ; =========================================================
    mov ax, cs
    mov bx, 0x0043
    int 0x23

    ; Verify RPL field is 3
    mov ax, cs
    and ax, 0x0003
    mov bx, 0x0003
    int 0x23

    ; =========================================================
    ; Test 7: Call gate from conforming segment back to ring 0
    ; Gate 0x48 (DPL=3) targets 0x08:ring0_return
    ; =========================================================
    mov word [result], 0
    call 0x4B:0                ; selector 0x48, RPL=3

    ; Back in conforming segment at ring 3
    mov ax, word [result]
    mov bx, 0xAAAA
    int 0x23

    ; =========================================================
    ; Test 8: Verify still at ring 3 after return from call gate
    ; =========================================================
    mov ax, cs
    and ax, 0x0003
    mov bx, 0x0003
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

; =========================================================
; Ring 0 handler for call gate
; =========================================================
ring0_return:
    mov ax, cs
    and ax, 0x0003
    mov bx, 0x0000
    int 0x23

    mov word [result], 0xAAAA
    retf

section .data

gdt_ptr:
    dw 0x4F                     ; limit: 10 entries * 8 - 1
    dw 0x0000
    db 0x00

result: dw 0
