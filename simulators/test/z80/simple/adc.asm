; adc.asm - Test ADC (add with carry) instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; --- ADC A, n with carry set via SCF ---

; SCF then ADC should include carry
    scf
    ld a, 0x10
    adc a, 0x22         ; 0x10 + 0x22 + 1 = 0x33
    ld b, 0x33
    rst 0x10

; --- ADC A, n after ADD (no carry out) ---

; ADD that doesn't carry, then ADC should not add carry
    ld a, 0x01
    add a, 0x01         ; A=2, CF=0
    ld a, 0x10
    adc a, 0x22         ; 0x10 + 0x22 + 0 = 0x32
    ld b, 0x32
    rst 0x10

; --- ADC A, n after ADD (with carry out) ---

; ADD that carries, then ADC should add carry
    ld a, 0xFF
    add a, 0x01         ; A=0, CF=1
    ld a, 0x10
    adc a, 0x22         ; 0x10 + 0x22 + 1 = 0x33
    ld b, 0x33
    rst 0x10

; --- ADC A, r (register operand) ---

    scf
    ld a, 0x05
    ld c, 0x03
    adc a, c            ; 0x05 + 0x03 + 1 = 0x09
    ld b, 0x09
    rst 0x10

; --- ADC A, n without carry (CF=0 via OR A) ---

    ld a, 0x10
    or a                ; clears CF
    ld a, 0x10
    adc a, 0x05         ; 0x10 + 0x05 + 0 = 0x15
    ld b, 0x15
    rst 0x10

; --- ADC A, n wrapping 8-bit ---

    scf
    ld a, 0xFF
    adc a, 0x00         ; 0xFF + 0x00 + 1 = 0x00
    ld b, 0x00
    rst 0x10

; --- ADC A, A ---

    scf
    ld a, 0x20
    adc a, a            ; 0x20 + 0x20 + 1 = 0x41
    ld b, 0x41
    rst 0x10

; --- ADC A, (HL) ---

    ld hl, 0x200
    ld a, 0x55
    ld [hl], a          ; store 0x55 at (0x200)
    scf
    ld a, 0x10
    adc a, [hl]         ; 0x10 + 0x55 + 1 = 0x66
    ld b, 0x66
    rst 0x10

; --- ADC HL, rr ---

    scf
    ld hl, 0x1000
    ld bc, 0x0234
    adc hl, bc          ; 0x1000 + 0x0234 + 1 = 0x1235
    ld de, 0x1235
    rst 0x18

; ADC HL without carry
    or a                ; clear CF
    ld hl, 0x1000
    ld bc, 0x0234
    adc hl, bc          ; 0x1000 + 0x0234 + 0 = 0x1234
    ld de, 0x1234
    rst 0x18

    halt
