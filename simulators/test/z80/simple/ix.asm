; ix.asm - Test IX register instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; LD IX, nn (16-bit immediate load)
    ld ix, 0x1234
    push ix
    pop hl
    ld de, 0x1234
    rst 0x18

; LD IX, nn - another value
    ld ix, 0xABCD
    push ix
    pop hl
    ld de, 0xABCD
    rst 0x18

; PUSH IX / POP IX
    ld ix, 0x5678
    push ix
    ld ix, 0x0000
    pop ix
    push ix
    pop hl
    ld de, 0x5678
    rst 0x18

; LD (nn), IX / LD IX, (nn)
    ld ix, 0xBEEF
    ld [data_area], ix
    ld ix, 0x0000
    ld ix, [data_area]
    push ix
    pop hl
    ld de, 0xBEEF
    rst 0x18

; LD SP, IX
    ld ix, 0xFFF0
    ld sp, ix
    ld hl, 0xFFF0
    ld de, 0xFFF0
    rst 0x18
    ld sp, 0xFFFE      ; restore SP

; INC IX / DEC IX (16-bit)
    ld ix, 0x1000
    inc ix
    push ix
    pop hl
    ld de, 0x1001
    rst 0x18

    ld ix, 0x1000
    dec ix
    push ix
    pop hl
    ld de, 0x0FFF
    rst 0x18

    ld ix, 0xFFFF
    inc ix
    push ix
    pop hl
    ld de, 0x0000
    rst 0x18

; ADD IX, rr (16-bit add)
    ld ix, 0x1000
    ld bc, 0x0234
    add ix, bc
    push ix
    pop hl
    ld de, 0x1234
    rst 0x18

    ld ix, 0x2000
    ld de, 0x3000
    add ix, de
    push ix
    pop hl
    ld de, 0x5000
    rst 0x18

; ADD IX, IX (self-add)
    ld ix, 0x1000
    add ix, ix
    push ix
    pop hl
    ld de, 0x2000
    rst 0x18

; LD r, (IX+d) - indexed memory reads
    ld ix, data_area
    ld [ix+0], 0x42
    ld a, [ix+0]
    ld b, 0x42
    rst 0x10

    ld [ix+1], 0xAA
    ld a, [ix+1]
    ld b, 0xAA
    rst 0x10

; LD (IX+d), r - indexed memory writes
    ld a, 0x55
    ld [ix+2], a
    ld a, 0x00
    ld a, [ix+2]
    ld b, 0x55
    rst 0x10

; LD (IX+d), n - indexed memory write immediate
    ld [ix+3], 0x77
    ld a, [ix+3]
    ld b, 0x77
    rst 0x10

; ADD A, (IX+d)
    ld a, 0x10
    ld [ix+0], 0x20
    add a, [ix+0]
    ld b, 0x30
    rst 0x10

; SUB (IX+d)
    ld a, 0x50
    ld [ix+0], 0x20
    sub [ix+0]
    ld b, 0x30
    rst 0x10

; AND (IX+d)
    ld a, 0xFF
    ld [ix+0], 0x0F
    and [ix+0]
    ld b, 0x0F
    rst 0x10

; OR (IX+d)
    ld a, 0xF0
    ld [ix+0], 0x0F
    or [ix+0]
    ld b, 0xFF
    rst 0x10

; XOR (IX+d)
    ld a, 0xFF
    ld [ix+0], 0xFF
    xor [ix+0]
    ld b, 0x00
    rst 0x10

; CP (IX+d) - compare (A unchanged)
    ld [ix+0], 0x42
    ld a, 0x42
    cp [ix+0]
    ; A is unchanged after CP
    ld b, 0x42
    rst 0x10

; INC (IX+d)
    ld [ix+0], 0x0F
    inc [ix+0]
    ld a, [ix+0]
    ld b, 0x10
    rst 0x10

; DEC (IX+d)
    ld [ix+0], 0x10
    dec [ix+0]
    ld a, [ix+0]
    ld b, 0x0F
    rst 0x10

; Negative displacement
    ld ix, data_area + 4
    ld [ix-4], 0x99
    ld a, [data_area]
    ld b, 0x99
    rst 0x10

; EX (SP), IX
    ld ix, 0x1111
    ld hl, 0x2222
    push hl
    ex [sp], ix
    ; IX should now be 0x2222, stack top should be 0x1111
    pop hl
    ld de, 0x1111
    rst 0x18
    push ix
    pop hl
    ld de, 0x2222
    rst 0x18

; JP (IX)
    ld ix, .jp_target
    jp [ix]
    ld a, 0x01        ; should not reach
    ld b, 0x00
    rst 0x10
.jp_target:

    halt

data_area:
    resb 16
