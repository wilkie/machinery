; iy_flags.asm - Flag verification for IY-indexed operations + edge cases
; Uses conditional jumps to verify flags (simulator uses lazy flag eval)
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

    ld iy, data_area

; ==========================================================================
; ADD A, (IY+d) — flags
; ==========================================================================

; ADD producing zero + carry
    ld [iy+0], 0x80
    ld a, 0x80
    add a, [iy+0]        ; 0x80 + 0x80 = 0x00
    jr z, .add_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.add_zf_ok:
    ; Also has carry
    ld [iy+0], 0x80
    ld a, 0x80
    add a, [iy+0]
    jr c, .add_cf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.add_cf_ok:

; ADD producing negative → SF set
    ld [iy+0], 0x01
    ld a, 0x7F
    add a, [iy+0]        ; 0x7F + 0x01 = 0x80
    jp m, .add_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.add_sf_ok:
    ld b, 0x80
    rst 0x10

; ADD no carry
    ld [iy+0], 0x01
    ld a, 0x01
    add a, [iy+0]        ; 0x01 + 0x01 = 0x02
    jr nc, .add_nc_ok
    ld a, 1
    ld b, 0
    rst 0x10
.add_nc_ok:
    jr nz, .add_nz_ok
    ld a, 1
    ld b, 0
    rst 0x10
.add_nz_ok:

; ==========================================================================
; SUB (IY+d) — flags
; ==========================================================================

; SUB producing borrow + negative
    ld [iy+0], 0x50
    ld a, 0x20
    sub [iy+0]            ; 0x20 - 0x50 = 0xD0 (borrow)
    jr c, .sub_cf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sub_cf_ok:
    ; Also negative
    ld [iy+0], 0x50
    ld a, 0x20
    sub [iy+0]
    jp m, .sub_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sub_sf_ok:

; SUB equal values → zero
    ld [iy+0], 0xAB
    ld a, 0xAB
    sub [iy+0]
    jr z, .sub_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sub_zf_ok:

; SUB no borrow, positive result
    ld [iy+0], 0x01
    ld a, 0x10
    sub [iy+0]
    jr nc, .sub_nc_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sub_nc_ok:
    jp p, .sub_nosf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sub_nosf_ok:
    ld b, 0x0F
    rst 0x10

; ==========================================================================
; AND (IY+d) — CF always clear
; ==========================================================================

    ld [iy+0], 0x0F
    ld a, 0x3C
    and [iy+0]            ; 0x3C & 0x0F = 0x0C
    jr nc, .and_nc_ok
    ld a, 1
    ld b, 0
    rst 0x10
.and_nc_ok:
    ld b, 0x0C
    rst 0x10

; AND producing zero
    ld [iy+0], 0xAA
    ld a, 0x55
    and [iy+0]            ; 0x55 & 0xAA = 0x00
    jr z, .and_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.and_zf_ok:

; ==========================================================================
; OR (IY+d) — flags
; ==========================================================================

; OR producing negative
    ld [iy+0], 0xF0
    ld a, 0x0F
    or [iy+0]             ; 0x0F | 0xF0 = 0xFF
    jp m, .or_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.or_sf_ok:
    ld b, 0xFF
    rst 0x10

; OR producing zero
    ld [iy+0], 0x00
    ld a, 0x00
    or [iy+0]
    jr z, .or_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.or_zf_ok:

; ==========================================================================
; XOR (IY+d) — flags
; ==========================================================================

; XOR producing zero
    ld [iy+0], 0xFF
    ld a, 0xFF
    xor [iy+0]
    jr z, .xor_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.xor_zf_ok:

; XOR non-zero, negative
    ld [iy+0], 0x7F
    ld a, 0xFF
    xor [iy+0]            ; 0xFF ^ 0x7F = 0x80
    jp m, .xor_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.xor_sf_ok:
    ld b, 0x80
    rst 0x10

; ==========================================================================
; CP (IY+d) — A preserved
; ==========================================================================

; CP greater → CF clear
    ld [iy+0], 0x10
    ld a, 0x50
    cp [iy+0]
    jr nc, .cp_nc_ok
    ld a, 1
    ld b, 0
    rst 0x10
.cp_nc_ok:
    jr nz, .cp_nz_ok
    ld a, 1
    ld b, 0
    rst 0x10
.cp_nz_ok:
    ld b, 0x50            ; A unchanged
    rst 0x10

; CP less → CF set
    ld [iy+0], 0xF0
    ld a, 0x01
    cp [iy+0]
    jr c, .cp_cf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.cp_cf_ok:

; CP equal → ZF set
    ld [iy+0], 0x77
    ld a, 0x77
    cp [iy+0]
    jr z, .cp_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.cp_zf_ok:
    ld b, 0x77            ; A unchanged
    rst 0x10

; ==========================================================================
; ADC A, (IY+d) — carry propagation
; ==========================================================================

; ADC with carry in wraps to zero
    scf
    ld [iy+0], 0xFE
    ld a, 0x01
    adc a, [iy+0]        ; 0x01 + 0xFE + 1 = 0x00
    jr z, .adc_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.adc_zf_ok:

; ADC without carry
    or a                  ; clear carry
    ld [iy+0], 0x10
    ld a, 0x20
    adc a, [iy+0]        ; 0x20 + 0x10 + 0 = 0x30
    ld b, 0x30
    rst 0x10

; ==========================================================================
; SBC A, (IY+d) — borrow propagation
; ==========================================================================

    scf
    ld [iy+0], 0x01
    ld a, 0x03
    sbc a, [iy+0]        ; 0x03 - 0x01 - 1 = 0x01
    ld b, 0x01
    rst 0x10
    jr nc, .sbc_nc_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sbc_nc_ok:

; SBC producing zero
    or a                  ; clear carry
    ld [iy+0], 0x55
    ld a, 0x55
    sbc a, [iy+0]        ; 0x55 - 0x55 - 0 = 0x00
    jr z, .sbc_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sbc_zf_ok:

; ==========================================================================
; INC (IY+d) — flags
; ==========================================================================

; INC from 0xFF → 0x00, ZF set
    ld [iy+0], 0xFF
    inc [iy+0]
    jr z, .inc_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.inc_zf_ok:
    ld a, [iy+0]
    ld b, 0x00
    rst 0x10

; INC producing negative
    ld [iy+0], 0x7F
    inc [iy+0]
    jp m, .inc_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.inc_sf_ok:
    ld a, [iy+0]
    ld b, 0x80
    rst 0x10

; ==========================================================================
; DEC (IY+d) — flags
; ==========================================================================

; DEC from 0x01 → 0x00, ZF set
    ld [iy+0], 0x01
    dec [iy+0]
    jr z, .dec_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.dec_zf_ok:
    ld a, [iy+0]
    ld b, 0x00
    rst 0x10

; DEC from 0x00 → 0xFF, SF set
    ld [iy+0], 0x00
    dec [iy+0]
    jp m, .dec_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.dec_sf_ok:
    ld a, [iy+0]
    ld b, 0xFF
    rst 0x10

; ==========================================================================
; ADD IY, rr — 16-bit flags
; ==========================================================================

; ADD IY, DE (not in base iy.asm)
    ld iy, 0x1234
    ld de, 0x4321
    add iy, de            ; 0x1234 + 0x4321 = 0x5555
    push iy
    pop hl
    ld de, 0x5555
    rst 0x18

; ADD IY, BC with carry
    ld iy, 0xF000
    ld bc, 0x2000
    add iy, bc            ; 0xF000 + 0x2000 = 0x1000, CF=1
    jr c, .addiy_cf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.addiy_cf_ok:
    push iy
    pop hl
    ld de, 0x1000
    rst 0x18

; ADD IY, IY no carry
    ld iy, 0x1000
    add iy, iy            ; 0x1000 + 0x1000 = 0x2000, CF=0
    jr nc, .addiy_nc_ok
    ld a, 1
    ld b, 0
    rst 0x10
.addiy_nc_ok:
    push iy
    pop hl
    ld de, 0x2000
    rst 0x18

; ADD IY, SP with carry
    ld iy, 0x8000
    ld sp, 0x9000
    add iy, sp            ; 0x8000 + 0x9000 = 0x1000, CF=1
    jr c, .addiy_sp_ok
    ld a, 1
    ld b, 0
    rst 0x10
.addiy_sp_ok:
    push iy
    pop hl
    ld de, 0x1000
    rst 0x18
    ld sp, 0xFFFE         ; restore SP

; ==========================================================================
; Edge case: max positive displacement (+127)
; ==========================================================================

    ld iy, data_area
    ld [iy+127], 0xCD
    ld a, [iy+127]
    ld b, 0xCD
    rst 0x10

; ==========================================================================
; Edge case: max negative displacement (-128)
; ==========================================================================

    ld iy, data_area + 128
    ld [iy-128], 0xDC
    ld a, [data_area]
    ld b, 0xDC
    rst 0x10

; ==========================================================================
; INC/DEC (IY+d) with various displacements
; ==========================================================================

    ld iy, data_area
    ld [iy+10], 0x7F
    inc [iy+10]           ; 0x7F + 1 = 0x80
    ld a, [iy+10]
    ld b, 0x80
    rst 0x10

    ld [iy+10], 0x80
    dec [iy+10]           ; 0x80 - 1 = 0x7F
    ld a, [iy+10]
    ld b, 0x7F
    rst 0x10

; ==========================================================================
; Verify IY not corrupted after many indexed operations
; ==========================================================================

    ld iy, data_area
    ld [iy+0], 0x11
    ld [iy+1], 0x22
    ld [iy+2], 0x33
    ld [iy+3], 0x44
    add a, [iy+0]
    sub [iy+1]
    and [iy+2]
    or [iy+3]
    push iy
    pop hl
    ld de, data_area
    rst 0x18

    halt

data_area:
    resb 256
