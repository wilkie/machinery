; dec.asm - Test DEC instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; DEC r (8-bit)
    ld a, 0x02
    dec a
    ld b, 0x01
    rst 0x10

    ld a, 0x10
    dec a
    ld b, 0x0F
    rst 0x10

    ld a, 0x01
    dec a
    ld b, 0x00
    rst 0x10

; DEC wraps from 0x00 to 0xFF
    ld a, 0x00
    dec a
    ld b, 0xFF
    rst 0x10

; DEC other registers
    ld c, 0x42
    dec c
    ld a, 0x41
    ld b, c
    rst 0x10

    ld d, 0x80
    dec d
    ld a, 0x7F
    ld b, d
    rst 0x10

; DEC rr (16-bit)
    ld hl, 0x1000
    dec hl
    ld de, 0x0FFF
    rst 0x18

    ld hl, 0x0000
    dec hl
    ld de, 0xFFFF
    rst 0x18

    ld bc, 0x0100
    dec bc
    ld h, b
    ld l, c
    ld de, 0x00FF
    rst 0x18

    halt
