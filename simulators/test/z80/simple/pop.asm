; pop.asm - Test POP instructions (complement to push.asm)
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; --- POP BC ---
    ld bc, 0x1234
    push bc
    ld bc, 0x0000
    pop bc
    ld h, b
    ld l, c
    ld de, 0x1234
    rst 0x18

; --- POP DE ---
    ld de, 0xABCD
    push de
    ld de, 0x0000
    pop de
    ld h, d
    ld l, e
    ld de, 0xABCD
    rst 0x18

; --- POP HL ---
    ld hl, 0x5678
    push hl
    ld hl, 0x0000
    pop hl
    ld de, 0x5678
    rst 0x18

; --- POP AF ---
    ld a, 0x99
    push af
    ld a, 0x00
    pop af
    ld b, 0x99
    rst 0x10

; --- POP IX ---
    ld ix, 0x4321
    push ix
    ld ix, 0x0000
    pop ix
    push ix
    pop hl              ; HL = IX value
    ld de, 0x4321
    rst 0x18

; --- POP IY ---
    ld iy, 0x8765
    push iy
    ld iy, 0x0000
    pop iy
    push iy
    pop hl              ; HL = IY value
    ld de, 0x8765
    rst 0x18

; --- Multiple push/pop LIFO ---
    ld bc, 0x1111
    ld de, 0x2222
    ld hl, 0x3333
    push bc
    push de
    push hl
    pop de              ; DE = 0x3333 (was HL)
    pop hl              ; HL = 0x2222 (was DE)
    ; check HL = 0x2222
    ld de, 0x2222
    rst 0x18
    ; get last value
    pop hl              ; HL = 0x1111 (was BC)
    ld de, 0x1111
    rst 0x18

    halt
