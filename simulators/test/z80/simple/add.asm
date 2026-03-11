; add.asm - Test ADD instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; ADD A, r (8-bit register)
    ld a, 0x10
    ld c, 0x20
    add a, c
    ld b, 0x30
    rst 0x10

; ADD A, n (8-bit immediate)
    ld a, 0x05
    add a, 0x03
    ld b, 0x08
    rst 0x10

; ADD A, r with zero result
    ld a, 0x00
    add a, 0x00
    ld b, 0x00
    rst 0x10

; ADD A, r with overflow (wraps around 8 bits)
    ld a, 0xFF
    add a, 0x01
    ld b, 0x00
    rst 0x10

    ld a, 0x80
    add a, 0x80
    ld b, 0x00
    rst 0x10

; ADD HL, rr (16-bit add)
    ld hl, 0x1000
    ld bc, 0x0234
    add hl, bc
    ld de, 0x1234
    rst 0x18

    ld hl, 0xFFFF
    ld bc, 0x0001
    add hl, bc
    ld de, 0x0000
    rst 0x18

    ld hl, 0x8000
    ld de, 0x8000
    add hl, de
    ld de, 0x0000
    rst 0x18

; ADD A, A (self-add, effectively doubles)
    ld a, 0x40
    add a, a
    ld b, 0x80
    rst 0x10

    halt
