; ld.asm - Test LD (load) instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; LD r, n (immediate 8-bit)
    ld a, 0x42
    ld b, 0x42
    rst 0x10

    ld a, 0xFF
    ld b, 0xFF
    rst 0x10

    ld a, 0x00
    ld b, 0x00
    rst 0x10

; LD r, r (register to register)
    ld a, 0x55
    ld c, a
    ld b, c
    rst 0x10

    ld a, 0xAA
    ld d, a
    ld e, d
    ld h, e
    ld l, h
    ld b, l
    rst 0x10

; LD rr, nn (16-bit immediate)
    ld hl, 0x1234
    ld de, 0x1234
    rst 0x18

    ld hl, 0x0000
    ld de, 0x0000
    rst 0x18

    ld hl, 0xFFFF
    ld de, 0xFFFF
    rst 0x18

; LD SP, nn
    ld sp, 0xFFFE
    ld hl, 0xFFFE
    ld de, 0xFFFE
    rst 0x18

    halt
