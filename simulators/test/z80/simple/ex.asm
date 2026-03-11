; ex.asm - Test EX (exchange) instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; EX DE, HL
    ld de, 0x1234
    ld hl, 0x5678
    ex de, hl
    ; now HL = 0x1234, DE = 0x5678
    ld de, 0x1234
    rst 0x18

; EX DE, HL - check the other side
    ld de, 0x1234
    ld hl, 0x5678
    ex de, hl
    ; DE = 0x5678, copy to HL to check
    ld h, d
    ld l, e
    ld de, 0x5678
    rst 0x18

; EX AF, AF'
    ld a, 0x42
    ex af, af'
    ld a, 0x00       ; clobber A
    ex af, af'       ; swap back
    ld b, 0x42
    rst 0x10

; EXX - check BC
    ld bc, 0x1111
    ld de, 0x2222
    ld hl, 0x3333
    exx
    ld bc, 0x0000
    ld de, 0x0000
    ld hl, 0x0000
    exx               ; swap back
    ; Check BC = 0x1111
    ld h, b
    ld l, c
    ld de, 0x1111
    rst 0x18

; EXX - check DE (redo from scratch to avoid clobber)
    ld bc, 0x4444
    ld de, 0x5555
    ld hl, 0x6666
    exx
    ld bc, 0x0000
    ld de, 0x0000
    ld hl, 0x0000
    exx               ; swap back
    ; Check DE = 0x5555: copy to HL, set DE to expected
    ld h, d
    ld l, e
    ld de, 0x5555
    rst 0x18

; EXX - check HL (redo from scratch)
    ld bc, 0x7777
    ld de, 0x8888
    ld hl, 0x9999
    exx
    ld bc, 0x0000
    ld de, 0x0000
    ld hl, 0x0000
    exx               ; swap back
    ; HL should be 0x9999
    ld de, 0x9999
    rst 0x18

    halt
