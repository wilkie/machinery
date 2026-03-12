; cond.asm - Test conditional jump, call, and return instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; --- JR cc tests ---

; JR NZ: should jump when Z=0
    ld a, 1             ; A != 0, so Z=0
    or a                ; set flags from A (NZ)
    jr nz, .jr_nz_ok
    ld a, 0xFF          ; should NOT execute
    ld b, 0x00
    rst 0x10
.jr_nz_ok:
    ld a, 0x01
    ld b, 0x01
    rst 0x10

; JR NZ: should NOT jump when Z=1
    xor a               ; A=0, Z=1
    jr nz, .jr_nz_fail
    jr .jr_nz_skip      ; should take this path
.jr_nz_fail:
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jr_nz_skip:
    ld a, 0x02
    ld b, 0x02
    rst 0x10

; JR Z: should jump when Z=1
    xor a               ; Z=1
    jr z, .jr_z_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jr_z_ok:
    ld a, 0x03
    ld b, 0x03
    rst 0x10

; JR Z: should NOT jump when Z=0
    ld a, 1
    or a                ; Z=0
    jr z, .jr_z_fail
    jr .jr_z_skip
.jr_z_fail:
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jr_z_skip:
    ld a, 0x04
    ld b, 0x04
    rst 0x10

; JR C: should jump when C=1
    scf                 ; CF=1
    jr c, .jr_c_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jr_c_ok:
    ld a, 0x05
    ld b, 0x05
    rst 0x10

; JR NC: should jump when C=0
    or a                ; clears carry
    jr nc, .jr_nc_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jr_nc_ok:
    ld a, 0x06
    ld b, 0x06
    rst 0x10

; --- JP cc tests ---

; JP NZ
    ld a, 1
    or a
    jp nz, .jp_nz_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_nz_ok:
    ld a, 0x10
    ld b, 0x10
    rst 0x10

; JP Z
    xor a
    jp z, .jp_z_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_z_ok:
    ld a, 0x11
    ld b, 0x11
    rst 0x10

; JP C
    scf
    jp c, .jp_c_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_c_ok:
    ld a, 0x12
    ld b, 0x12
    rst 0x10

; JP NC
    or a
    jp nc, .jp_nc_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_nc_ok:
    ld a, 0x13
    ld b, 0x13
    rst 0x10

; JP PO (parity odd, PF=0)
    ld a, 0x01          ; one bit set = odd parity
    or a                ; PF=0
    jp po, .jp_po_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_po_ok:
    ld a, 0x14
    ld b, 0x14
    rst 0x10

; JP PE (parity even, PF=1)
    ld a, 0x03          ; two bits set = even parity
    or a                ; PF=1
    jp pe, .jp_pe_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_pe_ok:
    ld a, 0x15
    ld b, 0x15
    rst 0x10

; JP PO: should NOT jump when PF=1
    ld a, 0x03          ; even parity → PF=1
    or a
    jp po, .jp_po_fail
    jr .jp_po_skip
.jp_po_fail:
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_po_skip:
    ld a, 0x16
    ld b, 0x16
    rst 0x10

; JP PE: should NOT jump when PF=0
    ld a, 0x01          ; odd parity → PF=0
    or a
    jp pe, .jp_pe_fail
    jr .jp_pe_skip
.jp_pe_fail:
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_pe_skip:
    ld a, 0x17
    ld b, 0x17
    rst 0x10

; JP P (positive, SF=0)
    ld a, 0x01          ; bit 7 = 0, SF=0
    or a
    jp p, .jp_p_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_p_ok:
    ld a, 0x18
    ld b, 0x18
    rst 0x10

; JP M (minus, SF=1)
    ld a, 0x80          ; bit 7 = 1, SF=1
    or a
    jp m, .jp_m_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_m_ok:
    ld a, 0x19
    ld b, 0x19
    rst 0x10

; JP P: should NOT jump when SF=1
    ld a, 0x80          ; SF=1
    or a
    jp p, .jp_p_fail
    jr .jp_p_skip
.jp_p_fail:
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_p_skip:
    ld a, 0x1A
    ld b, 0x1A
    rst 0x10

; JP M: should NOT jump when SF=0
    ld a, 0x01          ; SF=0
    or a
    jp m, .jp_m_fail
    jr .jp_m_skip
.jp_m_fail:
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.jp_m_skip:
    ld a, 0x1B
    ld b, 0x1B
    rst 0x10

; --- CALL cc / RET cc tests ---

; CALL NZ (should call when Z=0)
    ld a, 1
    or a                ; Z=0
    call nz, .sub_nz
    ld b, 0x20
    rst 0x10
    jp .call_z_test

.sub_nz:
    ld a, 0x20
    ret

; CALL Z (should NOT call when Z=0, so A stays)
.call_z_test:
    ld a, 0x30
    or a                ; Z=0 (since A=0x30)
    call z, .sub_z_bad  ; should NOT call
    ld b, 0x30
    rst 0x10
    jp .ret_cc_test

.sub_z_bad:
    ld a, 0xFF          ; should NOT execute
    ret

; RET NZ (conditional return)
.ret_cc_test:
    call .sub_ret_nz
    ld b, 0x40
    rst 0x10
    jp .ret_z_test

.sub_ret_nz:
    ld a, 0x40
    or a                ; Z=0
    ret nz              ; should return

; RET Z (should NOT return when Z=0)
.ret_z_test:
    call .sub_ret_z
    ld b, 0x50
    rst 0x10
    jp done

.sub_ret_z:
    ld a, 1
    or a                ; Z=0
    ret z               ; should NOT return (Z=0)
    ld a, 0x50          ; should execute
    ret                 ; unconditional return

; CALL C (should call when CF=1)
    scf                 ; CF=1
    call c, .sub_c
    ld b, 0x60
    rst 0x10
    jp .call_nc_test
.sub_c:
    ld a, 0x60
    ret

; CALL NC (should NOT call when CF=1)
.call_nc_test:
    ld a, 0x70
    scf                 ; CF=1
    call nc, .sub_nc_bad ; should NOT call
    ld b, 0x70
    rst 0x10
    jp .call_po_test
.sub_nc_bad:
    ld a, 0xFF
    ret

; CALL PO (should call when PF=0)
.call_po_test:
    ld a, 0x01          ; odd parity → PF=0
    or a
    call po, .sub_po
    ld b, 0x80
    rst 0x10
    jp .call_pe_test
.sub_po:
    ld a, 0x80
    ret

; CALL PE (should NOT call when PF=0)
.call_pe_test:
    ld a, 0x90
    ld b, 0x01          ; odd parity → PF=0
    or b
    call pe, .sub_pe_bad ; should NOT call (PF=0)
    ld b, 0x90
    rst 0x10
    jp .call_p_test
.sub_pe_bad:
    ld a, 0xFF
    ret

; CALL P (should call when SF=0)
.call_p_test:
    ld a, 0x01          ; SF=0
    or a
    call p, .sub_p
    ld b, 0xA0
    rst 0x10
    jp .call_m_test
.sub_p:
    ld a, 0xA0
    ret

; CALL M (should NOT call when SF=0)
.call_m_test:
    ld a, 0xB0
    ld b, 0x01          ; SF=0
    or b
    call m, .sub_m_bad  ; should NOT call (SF=0)
    ld b, 0xB0
    rst 0x10
    jp .ret_po_test

.sub_m_bad:
    ld a, 0xFF
    ret

; RET PO (should return when PF=0)
.ret_po_test:
    call .sub_ret_po
    ld b, 0xC0
    rst 0x10
    jp .ret_m_test
.sub_ret_po:
    ld a, 0xC0
    ld b, 0x01          ; odd parity → PF=0
    or b
    ret po              ; should return

; RET M (should return when SF=1)
.ret_m_test:
    call .sub_ret_m
    ld b, 0xD0
    rst 0x10
    jp done
.sub_ret_m:
    ld a, 0xD0
    ld b, 0x80          ; SF=1
    or b
    ret m               ; should return

done:
    halt
