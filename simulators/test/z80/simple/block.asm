; block.asm - Test block transfer and search instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; --- LDI: load and increment ---
; Copy one byte from (HL) to (DE), HL++, DE++, BC--

    ; Set up source
    ld hl, src_data
    ld a, 0xAA
    ld [hl], a

    ; Set up destination
    ld de, dst_data
    ld a, 0x00
    ld [de], a

    ld bc, 0x0001
    ldi

    ; Check destination has 0xAA
    ld a, [dst_data]
    ld b, 0xAA
    rst 0x10

; --- LDD: load and decrement ---

    ld hl, src_data + 3
    ld a, 0xBB
    ld [hl], a

    ld de, dst_data + 3
    ld a, 0x00
    ld [dst_data + 3], a

    ld bc, 0x0001
    ldd

    ; Check destination
    ld a, [dst_data + 3]
    ld b, 0xBB
    rst 0x10

; --- LDIR: load, increment, repeat (block copy) ---
; Copy 4 bytes from src to dst

    ; Set up source: 0x11, 0x22, 0x33, 0x44
    ld hl, src_data
    ld [hl], 0x11
    inc hl
    ld [hl], 0x22
    inc hl
    ld [hl], 0x33
    inc hl
    ld [hl], 0x44

    ; Clear destination
    ld hl, dst_data
    ld [hl], 0x00
    inc hl
    ld [hl], 0x00
    inc hl
    ld [hl], 0x00
    inc hl
    ld [hl], 0x00

    ; Copy 4 bytes
    ld hl, src_data
    ld de, dst_data
    ld bc, 4
    ldir

    ; Verify all 4 bytes
    ld a, [dst_data]
    ld b, 0x11
    rst 0x10

    ld a, [dst_data + 1]
    ld b, 0x22
    rst 0x10

    ld a, [dst_data + 2]
    ld b, 0x33
    rst 0x10

    ld a, [dst_data + 3]
    ld b, 0x44
    rst 0x10

; --- LDDR: load, decrement, repeat (block copy backwards) ---

    ; Set up source: 0x55 at src+3, 0x66 at src+2
    ld hl, src_data + 3
    ld [hl], 0x55
    dec hl
    ld [hl], 0x66

    ; Clear destination
    ld a, 0x00
    ld [dst_data + 2], a
    ld [dst_data + 3], a

    ; Copy 2 bytes backwards
    ld hl, src_data + 3
    ld de, dst_data + 3
    ld bc, 2
    lddr

    ld a, [dst_data + 3]
    ld b, 0x55
    rst 0x10

    ld a, [dst_data + 2]
    ld b, 0x66
    rst 0x10

; --- CPI: compare and increment ---
; Search for 0x22 in src_data

    ld hl, src_data
    ld [hl], 0x11
    inc hl
    ld [hl], 0x22
    inc hl
    ld [hl], 0x33

    ld hl, src_data
    ld a, 0x22
    ld bc, 3
    cpi                 ; compare A with (HL)=0x11, HL++, BC--
    ; Not equal, so ZF=0
    jr z, .cpi_fail
    cpi                 ; compare A with (HL)=0x22, HL++, BC--
    ; Equal, ZF=1
    jr z, .cpi_ok
.cpi_fail:
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cpi_ok:
    ; A should still be 0x22
    ld b, 0x22
    rst 0x10

; --- CPD: compare and decrement ---

    ld hl, src_data + 2
    ld a, 0x22
    ld bc, 3
    cpd                 ; compare A with (HL)=0x33, HL--, BC--
    jr z, .cpd_fail     ; not equal
    cpd                 ; compare A with (HL)=0x22, HL--, BC--
    jr z, .cpd_ok       ; equal
.cpd_fail:
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cpd_ok:
    ld b, 0x22
    rst 0x10

; --- CPIR: compare, increment, repeat ---
; Search for 0x33 in src_data (should find it at index 2)

    ld hl, src_data
    ld a, 0x33
    ld bc, 4
    cpir                ; searches until match or BC=0
    ; Should have found 0x33, ZF=1
    jr z, .cpir_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cpir_ok:
    ; A unchanged
    ld b, 0x33
    rst 0x10

; --- CPDR: compare, decrement, repeat ---
; Search backward for 0x11

    ld hl, src_data + 2
    ld a, 0x11
    ld bc, 3
    cpdr                ; searches backward
    jr z, .cpdr_ok
    ld a, 0xFF
    ld b, 0x00
    rst 0x10
.cpdr_ok:
    ld b, 0x11
    rst 0x10

    halt

src_data:
    resb 16
dst_data:
    resb 16
