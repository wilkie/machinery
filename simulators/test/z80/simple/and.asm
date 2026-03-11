; and.asm - Test AND instructions
; RST 0x10: assert A == B (8-bit)
; HALT: test complete

org 0x100

; AND r
    ld a, 0xFF
    ld c, 0x0F
    and c
    ld b, 0x0F
    rst 0x10

; AND n
    ld a, 0xAA
    and 0x55
    ld b, 0x00
    rst 0x10

    ld a, 0xFF
    and 0xFF
    ld b, 0xFF
    rst 0x10

    ld a, 0xF0
    and 0x3C
    ld b, 0x30
    rst 0x10

; AND A (identity)
    ld a, 0x42
    and a
    ld b, 0x42
    rst 0x10

    halt
