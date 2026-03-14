; bit.asm - Test BIT/SET/RES and shift/rotate instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; --- SET instructions (register) ---

    ld a, 0x00
    set 0, a
    ld b, 0x01
    rst 0x10

    ld a, 0x00
    set 7, a
    ld b, 0x80
    rst 0x10

    ld a, 0x01
    set 0, a             ; already set, no change
    ld b, 0x01
    rst 0x10

    ld a, 0x00
    set 3, a
    set 5, a
    ld b, 0x28
    rst 0x10

; SET on other registers
    ld b, 0x00
    set 4, b
    ld a, b
    ld b, 0x10
    rst 0x10

    ld c, 0x00
    set 6, c
    ld a, c
    ld b, 0x40
    rst 0x10

; --- RES instructions (register) ---

    ld a, 0xFF
    res 0, a
    ld b, 0xFE
    rst 0x10

    ld a, 0xFF
    res 7, a
    ld b, 0x7F
    rst 0x10

    ld a, 0xFF
    res 3, a
    res 5, a
    ld b, 0xD7
    rst 0x10

    ld a, 0x00
    res 4, a             ; already clear
    ld b, 0x00
    rst 0x10

; SET then RES same bit
    ld a, 0x00
    set 2, a
    res 2, a
    ld b, 0x00
    rst 0x10

; --- Shift/Rotate instructions (register) ---

; RLC r - rotate left circular
    ld a, 0x80
    rlc a
    ld b, 0x01           ; bit 7 wraps to bit 0
    rst 0x10

    ld a, 0x55
    rlc a
    ld b, 0xAA
    rst 0x10

; RRC r - rotate right circular
    ld a, 0x01
    rrc a
    ld b, 0x80           ; bit 0 wraps to bit 7
    rst 0x10

    ld a, 0xAA
    rrc a
    ld b, 0x55
    rst 0x10

; RL r - rotate left through carry
    scf                   ; set carry
    ld a, 0x80
    rl a                  ; bit 7 -> CF, CF(1) -> bit 0
    ld b, 0x01
    rst 0x10

; RR r - rotate right through carry
    scf                   ; set carry
    ld a, 0x01
    rr a                  ; bit 0 -> CF, CF(1) -> bit 7
    ld b, 0x80
    rst 0x10

; SLA r - shift left arithmetic
    ld a, 0x55
    sla a
    ld b, 0xAA
    rst 0x10

    ld a, 0x80
    sla a
    ld b, 0x00           ; bit 7 shifted out
    rst 0x10

; SRA r - shift right arithmetic (preserves sign)
    ld a, 0x80
    sra a
    ld b, 0xC0           ; sign bit preserved
    rst 0x10

    ld a, 0x02
    sra a
    ld b, 0x01
    rst 0x10

; SRL r - shift right logical
    ld a, 0x80
    srl a
    ld b, 0x40           ; bit 7 becomes 0
    rst 0x10

    ld a, 0x01
    srl a
    ld b, 0x00
    rst 0x10

; --- SET/RES with (HL) ---

    ld hl, data_bit
    ld [hl], 0x00

    set 4, [hl]
    ld a, [hl]
    ld b, 0x10
    rst 0x10

    set 1, [hl]
    ld a, [hl]
    ld b, 0x12
    rst 0x10

    res 4, [hl]
    ld a, [hl]
    ld b, 0x02
    rst 0x10

    res 1, [hl]
    ld a, [hl]
    ld b, 0x00
    rst 0x10

; --- Shift/Rotate with (HL) ---

    ld [hl], 0x80
    rlc [hl]
    ld a, [hl]
    ld b, 0x01
    rst 0x10

    ld [hl], 0x01
    rrc [hl]
    ld a, [hl]
    ld b, 0x80
    rst 0x10

    ld [hl], 0x55
    sla [hl]
    ld a, [hl]
    ld b, 0xAA
    rst 0x10

    ld [hl], 0x80
    sra [hl]
    ld a, [hl]
    ld b, 0xC0
    rst 0x10

    ld [hl], 0x80
    srl [hl]
    ld a, [hl]
    ld b, 0x40
    rst 0x10

; --- SET/RES with (IX+d) ---

    ld ix, data_bit

    ld [ix+0], 0x00
    set 6, [ix+0]
    ld a, [ix+0]
    ld b, 0x40
    rst 0x10

    set 2, [ix+0]
    ld a, [ix+0]
    ld b, 0x44
    rst 0x10

    res 6, [ix+0]
    ld a, [ix+0]
    ld b, 0x04
    rst 0x10

    res 2, [ix+0]
    ld a, [ix+0]
    ld b, 0x00
    rst 0x10

; Shift/Rotate with (IX+d)
    ld [ix+0], 0x80
    rlc [ix+0]
    ld a, [ix+0]
    ld b, 0x01
    rst 0x10

    ld [ix+0], 0x01
    rrc [ix+0]
    ld a, [ix+0]
    ld b, 0x80
    rst 0x10

    ld [ix+0], 0xAA
    sla [ix+0]
    ld a, [ix+0]
    ld b, 0x54           ; 0xAA << 1 = 0x154, truncated to 0x54
    rst 0x10

    ld [ix+0], 0x80
    sra [ix+0]
    ld a, [ix+0]
    ld b, 0xC0
    rst 0x10

    ld [ix+0], 0x80
    srl [ix+0]
    ld a, [ix+0]
    ld b, 0x40
    rst 0x10

; Different displacement
    ld [ix+3], 0x00
    set 7, [ix+3]
    ld a, [ix+3]
    ld b, 0x80
    rst 0x10

    res 7, [ix+3]
    ld a, [ix+3]
    ld b, 0x00
    rst 0x10

; RL/RR with (IX+d)
    scf
    ld [ix+0], 0x80
    rl [ix+0]
    ld a, [ix+0]
    ld b, 0x01           ; CF(1) into bit 0
    rst 0x10

    scf
    ld [ix+0], 0x01
    rr [ix+0]
    ld a, [ix+0]
    ld b, 0x80           ; CF(1) into bit 7
    rst 0x10

; --- SET/RES with (IY+d) ---

    ld iy, data_bit

    ld [iy+0], 0x00
    set 1, [iy+0]
    ld a, [iy+0]
    ld b, 0x02
    rst 0x10

    set 5, [iy+0]
    ld a, [iy+0]
    ld b, 0x22
    rst 0x10

    res 1, [iy+0]
    ld a, [iy+0]
    ld b, 0x20
    rst 0x10

    res 5, [iy+0]
    ld a, [iy+0]
    ld b, 0x00
    rst 0x10

; Shift/Rotate with (IY+d)
    ld [iy+0], 0x80
    rlc [iy+0]
    ld a, [iy+0]
    ld b, 0x01
    rst 0x10

    ld [iy+0], 0x01
    rrc [iy+0]
    ld a, [iy+0]
    ld b, 0x80
    rst 0x10

    ld [iy+0], 0x55
    sla [iy+0]
    ld a, [iy+0]
    ld b, 0xAA
    rst 0x10

    ld [iy+0], 0x80
    sra [iy+0]
    ld a, [iy+0]
    ld b, 0xC0
    rst 0x10

    ld [iy+0], 0x80
    srl [iy+0]
    ld a, [iy+0]
    ld b, 0x40
    rst 0x10

; RL/RR with (IY+d)
    scf
    ld [iy+0], 0x80
    rl [iy+0]
    ld a, [iy+0]
    ld b, 0x01           ; CF(1) into bit 0
    rst 0x10

    scf
    ld [iy+0], 0x01
    rr [iy+0]
    ld a, [iy+0]
    ld b, 0x80           ; CF(1) into bit 7
    rst 0x10

; Different displacement for IY
    ld [iy+5], 0x00
    set 0, [iy+5]
    set 3, [iy+5]
    ld a, [iy+5]
    ld b, 0x09
    rst 0x10

; --- BIT n, (IX+d) ---

    ld ix, data_bit

    ; BIT tests set Z flag if bit is 0
    ld [ix+0], 0x00
    bit 0, [ix+0]        ; bit 0 is 0, Z should be set
    jr z, .ix_bit0_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.ix_bit0_ok:

    ld [ix+0], 0x08       ; bit 3 set
    bit 3, [ix+0]         ; bit 3 is 1, Z should be clear
    jr nz, .ix_bit3_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.ix_bit3_ok:

    bit 7, [ix+0]         ; bit 7 is 0 in 0x08, Z should be set
    jr z, .ix_bit7_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.ix_bit7_ok:

    ld [ix+0], 0x80
    bit 7, [ix+0]         ; bit 7 is 1, Z should be clear
    jr nz, .ix_bit7b_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.ix_bit7b_ok:

    ; BIT with displacement
    ld [ix+4], 0x20        ; bit 5 set
    bit 5, [ix+4]
    jr nz, .ix_bit5d_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.ix_bit5d_ok:

    bit 0, [ix+4]         ; bit 0 is 0 in 0x20
    jr z, .ix_bit0d_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.ix_bit0d_ok:

; --- BIT n, (IY+d) ---

    ld iy, data_bit

    ld [iy+0], 0x00
    bit 4, [iy+0]         ; bit 4 is 0, Z should be set
    jr z, .iy_bit4_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.iy_bit4_ok:

    ld [iy+0], 0x10        ; bit 4 set
    bit 4, [iy+0]          ; bit 4 is 1, Z should be clear
    jr nz, .iy_bit4b_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.iy_bit4b_ok:

    ld [iy+2], 0x42        ; bits 1 and 6 set
    bit 1, [iy+2]
    jr nz, .iy_bit1d_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.iy_bit1d_ok:

    bit 6, [iy+2]
    jr nz, .iy_bit6d_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.iy_bit6d_ok:

    bit 2, [iy+2]          ; bit 2 is 0 in 0x42
    jr z, .iy_bit2d_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.iy_bit2d_ok:

; --- BIT n, (HL) ---

    ld hl, data_bit
    ld [hl], 0xA5          ; bits 0,2,5,7 set

    bit 0, [hl]
    jr nz, .hl_bit0_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.hl_bit0_ok:

    bit 1, [hl]            ; bit 1 is 0
    jr z, .hl_bit1_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.hl_bit1_ok:

    bit 5, [hl]
    jr nz, .hl_bit5_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.hl_bit5_ok:

    bit 7, [hl]
    jr nz, .hl_bit7_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.hl_bit7_ok:

; --- BIT n, r (register) ---

    ld a, 0x81             ; bits 0 and 7 set
    bit 0, a
    jr nz, .r_bit0_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.r_bit0_ok:

    ld a, 0x81
    bit 3, a               ; bit 3 is 0
    jr z, .r_bit3_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.r_bit3_ok:

    ld a, 0x81
    bit 7, a
    jr nz, .r_bit7_ok
    ld a, 0x01
    ld b, 0x00
    rst 0x10
.r_bit7_ok:

    halt

data_bit:
    resb 16
