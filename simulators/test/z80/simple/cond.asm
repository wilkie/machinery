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

done:
    halt
