; djnz.asm - Test DJNZ (decrement B and jump if not zero)
; RST 0x10: assert A == B (8-bit)
; HALT: test complete

org 0x100

; --- DJNZ as loop counter ---
; Loop 5 times, adding 0x10 each iteration
    ld a, 0x00
    ld b, 5
.loop1:
    add a, 0x10
    djnz .loop1
    ; A should be 0x50 (5 * 0x10)
    ld b, 0x50
    rst 0x10

; --- DJNZ with B=1 (loops once, then falls through) ---
    ld a, 0x00
    ld b, 1
.loop2:
    add a, 0x07
    djnz .loop2
    ; Should execute body once, B decrements to 0, falls through
    ld b, 0x07
    rst 0x10

; --- DJNZ with B=0 (wraps to 255 loops!) ---
; B=0 decrements to 0xFF, so loops 256 times
; 256 * 1 mod 256 = 0
    ld a, 0x00
    ld b, 0
.loop3:
    add a, 0x01
    djnz .loop3
    ; A = 256 mod 256 = 0x00
    ld b, 0x00
    rst 0x10

; --- DJNZ doesn't affect flags ---
; Set carry, then DJNZ should not clear it
    scf                 ; CF=1
    ld b, 1
.loop4:
    djnz .loop4
    ; CF should still be 1
    jr c, .djnz_cf_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.djnz_cf_ok:
    ld a, 0x01
    ld b, 0x01
    rst 0x10

; --- DJNZ counting down, B ends at 0 ---
    ld b, 3
    ld a, 0x00
.loop5:
    inc a
    djnz .loop5
    ; A = 3, B = 0
    ld b, 0x03
    rst 0x10

    halt
