; xor.asm - Test XOR instructions
; RST 0x10: assert A == B (8-bit)
; HALT: test complete

org 0x100

; XOR r
    ld a, 0xFF
    ld c, 0x0F
    xor c
    ld b, 0xF0
    rst 0x10

; XOR n
    ld a, 0xAA
    xor 0xFF
    ld b, 0x55
    rst 0x10

    ld a, 0xFF
    xor 0xFF
    ld b, 0x00
    rst 0x10

; XOR A (always zero)
    ld a, 0x42
    xor a
    ld b, 0x00
    rst 0x10

    halt
