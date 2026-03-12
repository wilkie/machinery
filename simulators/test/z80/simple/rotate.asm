; rotate.asm - Test accumulator rotate instructions (RLCA, RRCA, RLA, RRA)
; RST 0x10: assert A == B (8-bit)
; HALT: test complete

org 0x100

; --- RLCA: rotate left circular (bit 7 -> CF and bit 0) ---

    ld a, 0x80          ; 10000000
    rlca                ; -> 00000001, CF=1
    ld b, 0x01
    rst 0x10

    ld a, 0x55          ; 01010101
    rlca                ; -> 10101010, CF=0
    ld b, 0xAA
    rst 0x10

    ld a, 0x01          ; 00000001
    rlca                ; -> 00000010, CF=0
    ld b, 0x02
    rst 0x10

    ld a, 0xFF          ; 11111111
    rlca                ; -> 11111111, CF=1
    ld b, 0xFF
    rst 0x10

; --- RRCA: rotate right circular (bit 0 -> CF and bit 7) ---

    ld a, 0x01          ; 00000001
    rrca                ; -> 10000000, CF=1
    ld b, 0x80
    rst 0x10

    ld a, 0xAA          ; 10101010
    rrca                ; -> 01010101, CF=0
    ld b, 0x55
    rst 0x10

    ld a, 0x02          ; 00000010
    rrca                ; -> 00000001, CF=0
    ld b, 0x01
    rst 0x10

; --- RLA: rotate left through carry (bit 7 -> CF, old CF -> bit 0) ---

; CF=0 going in
    or a                ; clear CF
    ld a, 0x80          ; 10000000
    rla                 ; -> 00000000, CF=1 (old CF=0 -> bit 0)
    ld b, 0x00
    rst 0x10

; CF=1 going in
    scf                 ; CF=1
    ld a, 0x00          ; 00000000
    rla                 ; -> 00000001, CF=0 (old CF=1 -> bit 0)
    ld b, 0x01
    rst 0x10

; Chain: RLCA sets CF, then RLA uses it
    ld a, 0x80
    rlca                ; A=0x01, CF=1
    ld a, 0x00
    rla                 ; A=0x01 (CF was 1 -> bit 0), CF=0
    ld b, 0x01
    rst 0x10

; CF=1, A=0x55
    scf
    ld a, 0x55          ; 01010101
    rla                 ; -> 10101011, CF=0
    ld b, 0xAB
    rst 0x10

; --- RRA: rotate right through carry (bit 0 -> CF, old CF -> bit 7) ---

; CF=0 going in
    or a                ; clear CF
    ld a, 0x01          ; 00000001
    rra                 ; -> 00000000, CF=1 (old CF=0 -> bit 7)
    ld b, 0x00
    rst 0x10

; CF=1 going in
    scf                 ; CF=1
    ld a, 0x00          ; 00000000
    rra                 ; -> 10000000, CF=0 (old CF=1 -> bit 7)
    ld b, 0x80
    rst 0x10

; CF=1, A=0xAA
    scf
    ld a, 0xAA          ; 10101010
    rra                 ; -> 11010101, CF=0
    ld b, 0xD5
    rst 0x10

; --- RLA/RRA carry chain: verify carry propagates correctly ---
; After ADD with carry-out, RLA should use that carry
    ld a, 0xFF
    add a, 0x01         ; A=0x00, CF=1
    ld a, 0x00
    rla                 ; old CF(1) -> bit 0, A=0x01
    ld b, 0x01
    rst 0x10

; After ADD without carry-out, RRA should not inject carry
    ld a, 0x01
    add a, 0x01         ; A=0x02, CF=0
    ld a, 0x00
    rra                 ; old CF(0) -> bit 7, A=0x00
    ld b, 0x00
    rst 0x10

; --- RRD: rotate right decimal ---
; Before: A=0x12, (HL)=0x34 -> A=0x14, (HL)=0x23
    ld hl, 0x200
    ld a, 0x34
    ld [hl], a
    ld a, 0x12
    rrd
    ld b, 0x14          ; low nibble of (HL) -> low nibble of A
    rst 0x10
    ld a, [hl]
    ld b, 0x23          ; A_low -> (HL)_high, (HL)_high -> (HL)_low
    rst 0x10

; RRD: A=0xAB, (HL)=0xCD -> A=0xAD, (HL)=0xBC
    ld a, 0xCD
    ld [hl], a
    ld a, 0xAB
    rrd
    ld b, 0xAD
    rst 0x10
    ld a, [hl]
    ld b, 0xBC
    rst 0x10

; --- RLD: rotate left decimal ---
; Before: A=0x12, (HL)=0x34 -> A=0x13, (HL)=0x42
    ld a, 0x34
    ld [hl], a
    ld a, 0x12
    rld
    ld b, 0x13          ; high nibble of (HL) -> low nibble of A
    rst 0x10
    ld a, [hl]
    ld b, 0x42          ; (HL)_low -> (HL)_high, A_low -> (HL)_low
    rst 0x10

; RLD: A=0xAB, (HL)=0xCD -> A=0xAC, (HL)=0xDB
    ld a, 0xCD
    ld [hl], a
    ld a, 0xAB
    rld
    ld b, 0xAC
    rst 0x10
    ld a, [hl]
    ld b, 0xDB
    rst 0x10

    halt
