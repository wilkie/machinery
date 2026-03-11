; sub.asm - Test SUB instructions
; RST 0x10: assert A == B (8-bit)
; HALT: test complete

org 0x100

; SUB r (8-bit register)
    ld a, 0x30
    ld c, 0x10
    sub c
    ld b, 0x20
    rst 0x10

; SUB n (8-bit immediate)
    ld a, 0x50
    sub 0x25
    ld b, 0x2B
    rst 0x10

; SUB producing zero
    ld a, 0x42
    sub 0x42
    ld b, 0x00
    rst 0x10

; SUB with underflow (wraps)
    ld a, 0x00
    sub 0x01
    ld b, 0xFF
    rst 0x10

    ld a, 0x10
    sub 0x20
    ld b, 0xF0
    rst 0x10

; SUB A (always zero)
    ld a, 0xFF
    sub a
    ld b, 0x00
    rst 0x10

    halt
