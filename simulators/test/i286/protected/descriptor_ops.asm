; descriptor_ops.asm - Tests LAR, LSL, VERR, VERW, ARPL in protected mode
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

section .text
start:

    ; =========================================================
    ; Set up GDT at linear address 0.
    ;
    ; GDT layout:
    ;   Entry 0 (0x00): Null descriptor
    ;   Entry 1 (0x08): Code segment - base=0, limit=0xFFFF, exec/read, DPL=0
    ;   Entry 2 (0x10): Data segment - base=0, limit=0xFFFF, read/write, DPL=0
    ;   Entry 3 (0x18): Stack segment - base=0, limit=0xFFFF, read/write, DPL=0
    ;   Entry 4 (0x20): Code segment - base=0, limit=0x0FFF, exec only (not readable), DPL=0
    ;   Entry 5 (0x28): Data segment - base=0, limit=0x1FFF, read only, DPL=0
    ;   Entry 6 (0x30): Data segment - base=0, limit=0xFFFF, read/write, DPL=3
    ;   Entry 7 (0x38): Not present segment (P=0), data, DPL=0
    ; =========================================================

    xor ax, ax
    mov ds, ax

    ; Entry 0: Null descriptor
    mov word [0x00], 0x0000
    mov word [0x02], 0x0000
    mov word [0x04], 0x0000
    mov word [0x06], 0x0000

    ; Entry 1: Code segment (selector 0x08)
    ; Access 0x9A: P=1, DPL=0, S=1, Type=1010 (code, exec/read)
    mov word [0x08], 0xFFFF     ; limit
    mov word [0x0A], 0x0000     ; base low
    mov byte [0x0C], 0x00       ; base high
    mov byte [0x0D], 0x9A       ; access
    mov word [0x0E], 0x0000     ; reserved

    ; Entry 2: Data segment (selector 0x10)
    ; Access 0x92: P=1, DPL=0, S=1, Type=0010 (data, read/write)
    mov word [0x10], 0xFFFF
    mov word [0x12], 0x0000
    mov byte [0x14], 0x00
    mov byte [0x15], 0x92
    mov word [0x16], 0x0000

    ; Entry 3: Stack segment (selector 0x18)
    ; Access 0x92: P=1, DPL=0, S=1, Type=0010 (data, read/write)
    mov word [0x18], 0xFFFF
    mov word [0x1A], 0x0000
    mov byte [0x1C], 0x00
    mov byte [0x1D], 0x92
    mov word [0x1E], 0x0000

    ; Entry 4: Code segment, exec only (selector 0x20)
    ; Access 0x98: P=1, DPL=0, S=1, Type=1000 (code, exec only)
    mov word [0x20], 0x0FFF     ; limit = 0x0FFF
    mov word [0x22], 0x0000
    mov byte [0x24], 0x00
    mov byte [0x25], 0x98
    mov word [0x26], 0x0000

    ; Entry 5: Data segment, read only (selector 0x28)
    ; Access 0x90: P=1, DPL=0, S=1, Type=0000 (data, read only)
    mov word [0x28], 0x1FFF     ; limit = 0x1FFF
    mov word [0x2A], 0x0000
    mov byte [0x2C], 0x00
    mov byte [0x2D], 0x90
    mov word [0x2E], 0x0000

    ; Entry 6: Data segment, DPL=3 (selector 0x30)
    ; Access 0xF2: P=1, DPL=3, S=1, Type=0010 (data, read/write)
    mov word [0x30], 0xFFFF
    mov word [0x32], 0x0000
    mov byte [0x34], 0x00
    mov byte [0x35], 0xF2
    mov word [0x36], 0x0000

    ; Entry 7: Not present segment (selector 0x38)
    ; Access 0x12: P=0, DPL=0, S=1, Type=0010 (data, read/write, NOT PRESENT)
    mov word [0x38], 0xFFFF
    mov word [0x3A], 0x0000
    mov byte [0x3C], 0x00
    mov byte [0x3D], 0x12       ; P=0
    mov word [0x3E], 0x0000

    ; =========================================================
    ; Load GDT and enter protected mode
    ; =========================================================
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
    ; LAR tests
    ; =========================================================

    ; LAR with valid code selector 0x08 -> ZF=1
    ; Access byte 0x9A: P=1(bit7), DPL=00(bits6-5), S=1(bit4), Type=1010(bits3-0)
    ; After loading CS=0x08, the Accessed bit is set -> 0x9B
    ; LAR returns this in high byte -> 0x9B00
    mov ax, 0x08
    lar bx, ax
    jnz .lar_fail1              ; ZF should be 1
    mov ax, bx
    mov bx, 0x9B00
    int 0x23
    jmp .lar2
.lar_fail1:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.lar2:
    ; LAR with valid data selector 0x10 -> ZF=1
    ; After loading DS=0x10, the Accessed bit is set -> 0x93
    mov ax, 0x10
    lar bx, ax
    jnz .lar_fail2
    mov ax, bx
    mov bx, 0x9300
    int 0x23
    jmp .lar3
.lar_fail2:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.lar3:
    ; LAR with null selector -> ZF=0
    xor ax, ax
    lar bx, ax
    jz .lar_fail3               ; ZF should be 0 for null selector
    jmp .lar4
.lar_fail3:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.lar4:
    ; LAR with exec-only code selector 0x20 -> ZF=1, access=0x9800
    mov ax, 0x20
    lar bx, ax
    jnz .lar_fail4
    mov ax, bx
    mov bx, 0x9800
    int 0x23
    jmp .lar5
.lar_fail4:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.lar5:
    ; LAR with DPL=3 data selector 0x30 -> ZF=1 (DPL >= CPL=0? No, DPL=3 >= CPL=0)
    ; Wait: LAR checks DPL >= CPL AND DPL >= RPL. CPL=0, RPL=0, DPL=3. 3>=0 = true.
    ; Access 0xF2: P=1, DPL=3, S=1, Type=0010
    mov ax, 0x30
    lar bx, ax
    jnz .lar_fail5
    mov ax, bx
    mov bx, 0xF200
    int 0x23
    jmp .lar6
.lar_fail5:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.lar6:
    ; LAR with not-present selector 0x38 -> ZF=1 (LAR reports access byte even if P=0)
    ; Access 0x12: P=0, DPL=0, S=1, Type=0010
    mov ax, 0x38
    lar bx, ax
    jnz .lar_fail6
    mov ax, bx
    mov bx, 0x1200
    int 0x23
    jmp .lsl_tests
.lar_fail6:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; LSL tests
    ; =========================================================

.lsl_tests:
    ; LSL with code selector 0x08 (limit=0xFFFF) -> ZF=1
    mov ax, 0x08
    lsl bx, ax
    jnz .lsl_fail1
    mov ax, bx
    mov bx, 0xFFFF
    int 0x23
    jmp .lsl2
.lsl_fail1:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.lsl2:
    ; LSL with exec-only code selector 0x20 (limit=0x0FFF) -> ZF=1
    mov ax, 0x20
    lsl bx, ax
    jnz .lsl_fail2
    mov ax, bx
    mov bx, 0x0FFF
    int 0x23
    jmp .lsl3
.lsl_fail2:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.lsl3:
    ; LSL with read-only data selector 0x28 (limit=0x1FFF) -> ZF=1
    mov ax, 0x28
    lsl bx, ax
    jnz .lsl_fail3
    mov ax, bx
    mov bx, 0x1FFF
    int 0x23
    jmp .lsl4
.lsl_fail3:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.lsl4:
    ; LSL with null selector -> ZF=0
    xor ax, ax
    lsl bx, ax
    jz .lsl_fail4
    jmp .verr_tests
.lsl_fail4:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; VERR tests
    ; =========================================================

.verr_tests:
    ; VERR with readable code selector 0x08 -> ZF=1
    mov ax, 0x08
    verr ax
    jnz .verr_fail1
    jmp .verr2
.verr_fail1:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.verr2:
    ; VERR with exec-only code selector 0x20 -> ZF=0 (not readable)
    mov ax, 0x20
    verr ax
    jz .verr_fail2
    jmp .verr3
.verr_fail2:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.verr3:
    ; VERR with data selector 0x10 -> ZF=1 (data always readable)
    mov ax, 0x10
    verr ax
    jnz .verr_fail3
    jmp .verr4
.verr_fail3:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.verr4:
    ; VERR with read-only data selector 0x28 -> ZF=1
    mov ax, 0x28
    verr ax
    jnz .verr_fail4
    jmp .verw_tests
.verr_fail4:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; VERW tests
    ; =========================================================

.verw_tests:
    ; VERW with read/write data selector 0x10 -> ZF=1
    mov ax, 0x10
    verw ax
    jnz .verw_fail1
    jmp .verw2
.verw_fail1:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.verw2:
    ; VERW with read-only data selector 0x28 -> ZF=0 (not writable)
    mov ax, 0x28
    verw ax
    jz .verw_fail2
    jmp .verw3
.verw_fail2:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.verw3:
    ; VERW with code selector 0x08 -> ZF=0 (code not writable)
    mov ax, 0x08
    verw ax
    jz .verw_fail3
    jmp .arpl_tests
.verw_fail3:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; ARPL tests
    ; =========================================================

.arpl_tests:
    ; ARPL: source RPL > dest RPL -> adjusts dest, ZF=1
    mov ax, 0x0008              ; selector with RPL=0
    mov bx, 0x0013              ; selector with RPL=3
    arpl ax, bx
    jnz .arpl_fail1             ; ZF should be 1 (adjustment made)
    ; AX should now be 0x000B (0x08 with RPL changed to 3)
    mov bx, 0x000B
    int 0x23
    jmp .arpl2
.arpl_fail1:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.arpl2:
    ; ARPL: source RPL <= dest RPL -> no change, ZF=0
    mov ax, 0x0013              ; selector with RPL=3
    mov bx, 0x0010              ; selector with RPL=0
    arpl ax, bx
    jz .arpl_fail2              ; ZF should be 0 (no adjustment)
    ; AX should still be 0x0013
    mov bx, 0x0013
    int 0x23
    jmp .arpl3
.arpl_fail2:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

.arpl3:
    ; ARPL: equal RPL -> no change, ZF=0
    mov ax, 0x0011              ; RPL=1
    mov bx, 0x0009              ; RPL=1
    arpl ax, bx
    jz .arpl_fail3              ; ZF should be 0 (no adjustment needed)
    mov bx, 0x0011              ; unchanged
    int 0x23
    jmp .done
.arpl_fail3:
    mov ax, 0xFFFF
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
.done:
    mov ah, 0x4C
    int 0x21

section .data

gdt_ptr:
    dw 0x3F                     ; limit: 8 entries * 8 - 1
    dw 0x0000                   ; base low
    db 0x00                     ; base high
