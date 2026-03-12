; cp.asm - Test CP (compare) instructions
; CP subtracts but discards result, only setting flags. A is not modified.
; We test by using conditional jumps after CP to verify flag state.
; RST 0x10: assert A == B (8-bit)
; HALT: test complete

org 0x100

; --- CP n: equal (ZF=1) ---
    ld a, 0x42
    cp 0x42             ; A == 0x42, ZF=1
    jr z, .cp_eq_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cp_eq_ok:
    ; A should be unchanged
    ld b, 0x42
    rst 0x10

; --- CP n: not equal (ZF=0) ---
    ld a, 0x42
    cp 0x10             ; A != 0x10, ZF=0
    jr nz, .cp_ne_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cp_ne_ok:
    ld b, 0x42          ; A unchanged
    rst 0x10

; --- CP n: A < n sets CF=1 ---
    ld a, 0x10
    cp 0x20             ; 0x10 < 0x20, CF=1
    jr c, .cp_lt_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cp_lt_ok:
    ld b, 0x10          ; A unchanged
    rst 0x10

; --- CP n: A >= n clears CF=0 ---
    ld a, 0x30
    cp 0x20             ; 0x30 >= 0x20, CF=0
    jr nc, .cp_ge_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cp_ge_ok:
    ld b, 0x30          ; A unchanged
    rst 0x10

; --- CP r (register) ---
    ld a, 0x50
    ld c, 0x50
    cp c                ; equal
    jr z, .cp_r_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cp_r_ok:
    ld b, 0x50
    rst 0x10

; --- CP A (always zero/equal) ---
    ld a, 0xFF
    cp a                ; A - A = 0, ZF=1
    jr z, .cp_a_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cp_a_ok:
    ld b, 0xFF          ; A unchanged
    rst 0x10

; --- CP (HL) ---
    ld hl, 0x200
    ld a, 0x33
    ld [hl], 0x33
    cp [hl]             ; equal
    jr z, .cp_hl_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cp_hl_ok:
    ld b, 0x33
    rst 0x10

; --- CP n: sign flag (negative result) ---
    ld a, 0x01
    cp 0x02             ; 0x01 - 0x02 = 0xFF (negative), SF=1
    jp m, .cp_sf_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cp_sf_ok:
    ld b, 0x01
    rst 0x10

    halt
