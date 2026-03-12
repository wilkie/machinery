; misc.asm - Test miscellaneous instructions (CPL, NEG, SCF, CCF)
; RST 0x10: assert A == B (8-bit)
; HALT: test complete

org 0x100

; --- CPL: complement (invert all bits) ---

    ld a, 0x00
    cpl
    ld b, 0xFF
    rst 0x10

    ld a, 0xFF
    cpl
    ld b, 0x00
    rst 0x10

    ld a, 0x55          ; 01010101
    cpl                 ; -> 10101010
    ld b, 0xAA
    rst 0x10

    ld a, 0x0F
    cpl
    ld b, 0xF0
    rst 0x10

; --- NEG: negate (two's complement) ---

    ld a, 0x01
    neg                 ; 0 - 1 = 0xFF
    ld b, 0xFF
    rst 0x10

    ld a, 0x00
    neg                 ; 0 - 0 = 0x00
    ld b, 0x00
    rst 0x10

    ld a, 0x80
    neg                 ; 0 - 0x80 = 0x80
    ld b, 0x80
    rst 0x10

    ld a, 0x7F
    neg                 ; 0 - 0x7F = 0x81
    ld b, 0x81
    rst 0x10

    ld a, 0x42
    neg                 ; 0 - 0x42 = 0xBE
    ld b, 0xBE
    rst 0x10

; --- SCF: set carry flag ---

    or a                ; clear CF
    scf                 ; CF=1
    jr c, .scf_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.scf_ok:
    ld a, 0x01
    ld b, 0x01
    rst 0x10

; --- CCF: complement carry flag ---

; CF=1 -> CCF -> CF=0
    scf
    ccf
    jr nc, .ccf1_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.ccf1_ok:
    ld a, 0x02
    ld b, 0x02
    rst 0x10

; CF=0 -> CCF -> CF=1
    or a                ; clear CF
    ccf
    jr c, .ccf2_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.ccf2_ok:
    ld a, 0x03
    ld b, 0x03
    rst 0x10

; Double CCF: CF=1 -> 0 -> 1
    scf
    ccf
    ccf
    jr c, .ccf3_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.ccf3_ok:
    ld a, 0x04
    ld b, 0x04
    rst 0x10

; --- CCF after ALU operation (tests lazy CF resolution) ---
; ADD that carries, then CCF should complement
    ld a, 0xFF
    add a, 0x01         ; CF=1 (lazy)
    ccf                 ; resolves CF=1, then complements to CF=0
    jr nc, .ccf_alu_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.ccf_alu_ok:
    ld a, 0x05
    ld b, 0x05
    rst 0x10

    halt
