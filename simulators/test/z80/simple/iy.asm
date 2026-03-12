; iy.asm - Test IY register instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; LD IY, nn
    ld iy, 0x9876
    push iy
    pop hl
    ld de, 0x9876
    rst 0x18

; PUSH IY / POP IY
    ld iy, 0x4321
    push iy
    ld iy, 0x0000
    pop iy
    push iy
    pop hl
    ld de, 0x4321
    rst 0x18

; LD (nn), IY / LD IY, (nn)
    ld iy, 0xCAFE
    ld [data_iy], iy
    ld iy, 0x0000
    ld iy, [data_iy]
    push iy
    pop hl
    ld de, 0xCAFE
    rst 0x18

; INC IY / DEC IY
    ld iy, 0x2000
    inc iy
    inc iy
    inc iy
    push iy
    pop hl
    ld de, 0x2003
    rst 0x18

    ld iy, 0x0000
    dec iy
    push iy
    pop hl
    ld de, 0xFFFF
    rst 0x18

; ADD IY, rr
    ld iy, 0x1000
    ld bc, 0x0500
    add iy, bc
    push iy
    pop hl
    ld de, 0x1500
    rst 0x18

    ld iy, 0x3000
    add iy, sp
    ; SP is 0xFFFE, so IY = 0x3000 + 0xFFFE = 0x2FFE (wraps)
    push iy
    pop hl
    ld de, 0x2FFE
    rst 0x18

; ADD IY, IY (self-add)
    ld iy, 0x0800
    add iy, iy
    push iy
    pop hl
    ld de, 0x1000
    rst 0x18

; LD r, (IY+d) - indexed memory reads
    ld iy, data_iy
    ld [iy+0], 0x33
    ld a, [iy+0]
    ld b, 0x33
    rst 0x10

    ld [iy+5], 0xDD
    ld a, [iy+5]
    ld b, 0xDD
    rst 0x10

; LD (IY+d), r
    ld c, 0xEE
    ld [iy+2], c
    ld a, [iy+2]
    ld b, 0xEE
    rst 0x10

; ADD A, (IY+d)
    ld a, 0x11
    ld [iy+0], 0x22
    add a, [iy+0]
    ld b, 0x33
    rst 0x10

; AND (IY+d), OR (IY+d), XOR (IY+d)
    ld a, 0xAA
    ld [iy+0], 0x55
    and [iy+0]
    ld b, 0x00
    rst 0x10

    ld a, 0xA0
    ld [iy+0], 0x05
    or [iy+0]
    ld b, 0xA5
    rst 0x10

    ld a, 0xFF
    ld [iy+0], 0xAA
    xor [iy+0]
    ld b, 0x55
    rst 0x10

; INC (IY+d)
    ld [iy+0], 0xFF
    inc [iy+0]
    ld a, [iy+0]
    ld b, 0x00
    rst 0x10

; DEC (IY+d)
    ld [iy+0], 0x00
    dec [iy+0]
    ld a, [iy+0]
    ld b, 0xFF
    rst 0x10

; EX (SP), IY
    ld iy, 0xAAAA
    ld hl, 0xBBBB
    push hl
    ex [sp], iy
    pop hl
    ld de, 0xAAAA
    rst 0x18
    push iy
    pop hl
    ld de, 0xBBBB
    rst 0x18

; JP (IY)
    ld iy, .jp_target
    jp [iy]
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.jp_target:

; LD SP, IY
    ld iy, 0xFFF0
    ld sp, iy
    ld hl, 0xFFF0
    ld de, 0xFFF0
    rst 0x18
    ld sp, 0xFFFE      ; restore SP

    halt

data_iy:
    resb 16
