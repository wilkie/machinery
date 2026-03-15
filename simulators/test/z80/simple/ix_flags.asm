; ix_flags.asm - Flag verification for IX-indexed operations
; Uses conditional jumps to verify flags (simulator uses lazy flag eval)
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

    ld ix, data_area

; ==========================================================================
; ADD A, (IX+d) — flags
; ==========================================================================

; ADD producing zero result → ZF set, CF set
    ld [ix+0], 0x01
    ld a, 0xFF
    add a, [ix+0]        ; 0xFF + 0x01 = 0x00
    jr z, .add_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.add_zf_ok:

; ADD producing zero also sets carry (0xFF + 0x01 wraps)
    ld [ix+0], 0x01
    ld a, 0xFF
    add a, [ix+0]
    jr c, .add_cf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.add_cf_ok:

; ADD producing negative result → SF set (jp m = jump if minus)
    ld [ix+0], 0x70
    ld a, 0x10
    add a, [ix+0]        ; 0x10 + 0x70 = 0x80
    jp m, .add_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.add_sf_ok:
    ld b, 0x80
    rst 0x10

; ADD with no carry → CF clear
    ld [ix+0], 0x01
    ld a, 0x02
    add a, [ix+0]        ; 0x02 + 0x01 = 0x03
    jr nc, .add_nc_ok
    ld a, 1
    ld b, 0
    rst 0x10
.add_nc_ok:
    ld b, 0x03
    rst 0x10

; ADD positive result → SF clear (jp p = jump if positive)
    ld [ix+0], 0x01
    ld a, 0x01
    add a, [ix+0]        ; 0x01 + 0x01 = 0x02
    jp p, .add_nosf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.add_nosf_ok:

; ADD non-zero result → ZF clear
    ld [ix+0], 0x01
    ld a, 0x02
    add a, [ix+0]
    jr nz, .add_nz_ok
    ld a, 1
    ld b, 0
    rst 0x10
.add_nz_ok:

; ==========================================================================
; SUB (IX+d) — flags
; ==========================================================================

; SUB producing zero → ZF set
    ld [ix+0], 0x42
    ld a, 0x42
    sub [ix+0]            ; 0x42 - 0x42 = 0x00
    jr z, .sub_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sub_zf_ok:

; SUB with borrow → CF set
    ld [ix+0], 0x80
    ld a, 0x10
    sub [ix+0]            ; 0x10 - 0x80 = 0x90 (borrow)
    jr c, .sub_cf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sub_cf_ok:

; SUB producing negative → SF set
    ld [ix+0], 0x80
    ld a, 0x10
    sub [ix+0]            ; result = 0x90 (bit 7 set)
    jp m, .sub_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sub_sf_ok:

; SUB with no borrow → CF clear
    ld [ix+0], 0x01
    ld a, 0x10
    sub [ix+0]            ; 0x10 - 0x01 = 0x0F
    jr nc, .sub_nc_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sub_nc_ok:
    ld b, 0x0F
    rst 0x10

; ==========================================================================
; AND (IX+d) — CF always clear
; ==========================================================================

; AND producing zero → ZF set
    ld [ix+0], 0x0F
    ld a, 0xF0
    and [ix+0]            ; 0xF0 & 0x0F = 0x00
    jr z, .and_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.and_zf_ok:

; AND result → CF always clear
    ld [ix+0], 0xFF
    ld a, 0xAA
    and [ix+0]            ; 0xAA
    jr nc, .and_nc_ok
    ld a, 1
    ld b, 0
    rst 0x10
.and_nc_ok:
    ld b, 0xAA
    rst 0x10

; ==========================================================================
; OR (IX+d) — flags
; ==========================================================================

; OR producing zero → ZF set
    ld [ix+0], 0x00
    ld a, 0x00
    or [ix+0]             ; 0x00 | 0x00 = 0x00
    jr z, .or_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.or_zf_ok:

; OR producing negative → SF set
    ld [ix+0], 0x80
    ld a, 0x00
    or [ix+0]             ; 0x80
    jp m, .or_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.or_sf_ok:

; ==========================================================================
; XOR (IX+d) — flags
; ==========================================================================

; XOR with self → zero
    ld [ix+0], 0xAA
    ld a, 0xAA
    xor [ix+0]
    jr z, .xor_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.xor_zf_ok:

; XOR non-zero → ZF clear
    ld [ix+0], 0xFF
    ld a, 0x00
    xor [ix+0]
    jr nz, .xor_nz_ok
    ld a, 1
    ld b, 0
    rst 0x10
.xor_nz_ok:
    ld b, 0xFF
    rst 0x10

; ==========================================================================
; CP (IX+d) — flags like SUB but A preserved
; ==========================================================================

; CP equal → ZF set, A unchanged
    ld [ix+0], 0x42
    ld a, 0x42
    cp [ix+0]
    jr z, .cp_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.cp_zf_ok:
    ld b, 0x42            ; A preserved
    rst 0x10

; CP less → CF set, A unchanged
    ld [ix+0], 0x80
    ld a, 0x10
    cp [ix+0]             ; 0x10 < 0x80
    jr c, .cp_cf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.cp_cf_ok:
    ld b, 0x10            ; A preserved
    rst 0x10

; CP greater → CF clear
    ld [ix+0], 0x01
    ld a, 0x80
    cp [ix+0]             ; 0x80 > 0x01
    jr nc, .cp_nc_ok
    ld a, 1
    ld b, 0
    rst 0x10
.cp_nc_ok:

; ==========================================================================
; ADC A, (IX+d) — carry in
; ==========================================================================

; ADC with carry producing zero → ZF set
    scf
    ld [ix+0], 0xFE
    ld a, 0x01
    adc a, [ix+0]        ; 0x01 + 0xFE + 1 = 0x00
    jr z, .adc_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.adc_zf_ok:

; ADC without carry → result correct
    or a                  ; clear carry
    ld [ix+0], 0x20
    ld a, 0x10
    adc a, [ix+0]        ; 0x10 + 0x20 + 0 = 0x30
    ld b, 0x30
    rst 0x10

; ==========================================================================
; SBC A, (IX+d) — borrow in
; ==========================================================================

; SBC with borrow producing negative
    scf
    ld [ix+0], 0x00
    ld a, 0x00
    sbc a, [ix+0]        ; 0x00 - 0x00 - 1 = 0xFF
    jp m, .sbc_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.sbc_sf_ok:

; SBC with borrow, result correct
    scf
    ld [ix+0], 0x01
    ld a, 0x05
    sbc a, [ix+0]        ; 0x05 - 0x01 - 1 = 0x03
    ld b, 0x03
    rst 0x10

; ==========================================================================
; INC (IX+d) — flags: CF preserved
; ==========================================================================

; INC from 0xFF → 0x00, ZF set
    ld [ix+0], 0xFF
    inc [ix+0]
    jr z, .inc_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.inc_zf_ok:
    ld a, [ix+0]
    ld b, 0x00
    rst 0x10

; INC producing negative → SF set
    ld [ix+0], 0x7F
    inc [ix+0]            ; 0x7F + 1 = 0x80
    jp m, .inc_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.inc_sf_ok:
    ld a, [ix+0]
    ld b, 0x80
    rst 0x10

; INC preserves CF: set carry, then inc, carry should still be set
    scf
    ld [ix+0], 0x05
    inc [ix+0]
    jr c, .inc_cf_kept
    ld a, 1
    ld b, 0
    rst 0x10
.inc_cf_kept:
    ld a, [ix+0]
    ld b, 0x06
    rst 0x10

; ==========================================================================
; DEC (IX+d) — flags: CF preserved
; ==========================================================================

; DEC from 0x01 → 0x00, ZF set
    ld [ix+0], 0x01
    dec [ix+0]
    jr z, .dec_zf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.dec_zf_ok:
    ld a, [ix+0]
    ld b, 0x00
    rst 0x10

; DEC from 0x00 → 0xFF, SF set
    ld [ix+0], 0x00
    dec [ix+0]
    jp m, .dec_sf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.dec_sf_ok:
    ld a, [ix+0]
    ld b, 0xFF
    rst 0x10

; ==========================================================================
; ADD IX, rr — 16-bit flags (CF)
; ==========================================================================

; ADD IX, BC with carry
    ld ix, 0xFF00
    ld bc, 0x0200
    add ix, bc            ; 0xFF00 + 0x0200 = 0x0100, CF=1
    jr c, .addix_cf_ok
    ld a, 1
    ld b, 0
    rst 0x10
.addix_cf_ok:
    push ix
    pop hl
    ld de, 0x0100
    rst 0x18

; ADD IX, DE with no carry
    ld ix, 0x1000
    ld de, 0x2000
    add ix, de            ; 0x1000 + 0x2000 = 0x3000, CF=0
    jr nc, .addix_nc_ok
    ld a, 1
    ld b, 0
    rst 0x10
.addix_nc_ok:
    push ix
    pop hl
    ld de, 0x3000
    rst 0x18

; ADD IX, SP
    ld ix, 0x8000
    ld sp, 0x8001
    add ix, sp            ; 0x8000 + 0x8001 = 0x0001, CF=1
    jr c, .addix_sp_ok
    ld a, 1
    ld b, 0
    rst 0x10
.addix_sp_ok:
    push ix
    pop hl
    ld de, 0x0001
    rst 0x18
    ld sp, 0xFFFE         ; restore SP

; ==========================================================================
; Edge case: max positive displacement (+127)
; ==========================================================================

    ld ix, data_area
    ld [ix+127], 0xBE
    ld a, [ix+127]
    ld b, 0xBE
    rst 0x10

; ==========================================================================
; Edge case: max negative displacement (-128)
; ==========================================================================

    ld ix, data_area + 128
    ld [ix-128], 0xEF
    ld a, [data_area]
    ld b, 0xEF
    rst 0x10

; ==========================================================================
; Verify IX not corrupted after indexed operations
; ==========================================================================

    ld ix, data_area
    ld [ix+0], 0x11
    ld [ix+1], 0x22
    ld [ix+2], 0x33
    add a, [ix+0]
    sub [ix+1]
    and [ix+2]
    ; IX should still point to data_area
    push ix
    pop hl
    ld de, data_area
    rst 0x18

    halt

data_area:
    resb 256
