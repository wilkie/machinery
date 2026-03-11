; or.asm - Test OR instructions
; RST 0x10: assert A == B (8-bit)
; HALT: test complete

org 0x100

; OR r
    ld a, 0xF0
    ld c, 0x0F
    or c
    ld b, 0xFF
    rst 0x10

; OR n
    ld a, 0xAA
    or 0x55
    ld b, 0xFF
    rst 0x10

    ld a, 0x00
    or 0x00
    ld b, 0x00
    rst 0x10

    ld a, 0x80
    or 0x01
    ld b, 0x81
    rst 0x10

; OR A (identity)
    ld a, 0x42
    or a
    ld b, 0x42
    rst 0x10

    halt
