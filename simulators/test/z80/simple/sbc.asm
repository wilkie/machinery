; sbc.asm - Test SBC (subtract with carry/borrow) instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; --- SBC A, n with carry set via SCF ---

; SCF then SBC should subtract extra 1
    scf
    ld a, 0x33
    sbc a, 0x22         ; 0x33 - 0x22 - 1 = 0x10
    ld b, 0x10
    rst 0x10

; --- SBC A, n after ADD (no carry out) ---

; ADD that doesn't carry, then SBC should not subtract extra
    ld a, 0x01
    add a, 0x01         ; A=2, CF=0
    ld a, 0x33
    sbc a, 0x22         ; 0x33 - 0x22 - 0 = 0x11
    ld b, 0x11
    rst 0x10

; --- SBC A, n after ADD (with carry out) ---

; ADD that carries, then SBC should subtract extra 1
    ld a, 0xFF
    add a, 0x01         ; A=0, CF=1
    ld a, 0x33
    sbc a, 0x22         ; 0x33 - 0x22 - 1 = 0x10
    ld b, 0x10
    rst 0x10

; --- SBC A, r (register operand) ---

    scf
    ld a, 0x10
    ld c, 0x03
    sbc a, c            ; 0x10 - 0x03 - 1 = 0x0C
    ld b, 0x0C
    rst 0x10

; --- SBC A, n without carry (CF=0 via OR A) ---

    ld a, 0x10
    or a                ; clears CF
    ld a, 0x30
    sbc a, 0x05         ; 0x30 - 0x05 - 0 = 0x2B
    ld b, 0x2B
    rst 0x10

; --- SBC A, n wrapping 8-bit ---

    scf
    ld a, 0x00
    sbc a, 0x00         ; 0x00 - 0x00 - 1 = 0xFF
    ld b, 0xFF
    rst 0x10

; --- SBC A, A ---

; SBC A, A with carry = 0 gives 0
    or a                ; CF=0
    ld a, 0x42
    sbc a, a            ; 0x42 - 0x42 - 0 = 0x00
    ld b, 0x00
    rst 0x10

; SBC A, A with carry = 1 gives 0xFF
    scf
    ld a, 0x42
    sbc a, a            ; 0x42 - 0x42 - 1 = 0xFF
    ld b, 0xFF
    rst 0x10

; --- SBC A, (HL) ---

    ld hl, 0x200
    ld a, 0x20
    ld [hl], a          ; store 0x20 at (0x200)
    scf
    ld a, 0x55
    sbc a, [hl]         ; 0x55 - 0x20 - 1 = 0x34
    ld b, 0x34
    rst 0x10

; --- SBC HL, rr ---

    scf
    ld hl, 0x1235
    ld bc, 0x0234
    sbc hl, bc          ; 0x1235 - 0x0234 - 1 = 0x1000
    ld de, 0x1000
    rst 0x18

; SBC HL without carry
    or a                ; clear CF
    ld hl, 0x1234
    ld bc, 0x0234
    sbc hl, bc          ; 0x1234 - 0x0234 - 0 = 0x1000
    ld de, 0x1000
    rst 0x18

    halt
