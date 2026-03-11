; inc.asm - Test INC instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; INC r (8-bit)
    ld a, 0x00
    inc a
    ld b, 0x01
    rst 0x10

    ld a, 0x0F
    inc a
    ld b, 0x10
    rst 0x10

    ld a, 0xFE
    inc a
    ld b, 0xFF
    rst 0x10

; INC wraps from 0xFF to 0x00
    ld a, 0xFF
    inc a
    ld b, 0x00
    rst 0x10

; INC other registers
    ld c, 0x41
    inc c
    ld a, 0x42
    ld b, c
    rst 0x10

    ld d, 0x7F
    inc d
    ld a, 0x80
    ld b, d
    rst 0x10

; INC rr (16-bit)
    ld hl, 0x1000
    inc hl
    ld de, 0x1001
    rst 0x18

    ld hl, 0xFFFF
    inc hl
    ld de, 0x0000
    rst 0x18

    ld bc, 0x00FF
    inc bc
    ld h, b
    ld l, c
    ld de, 0x0100
    rst 0x18

    halt
