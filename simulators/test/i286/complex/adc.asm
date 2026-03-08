; adc.asm — thorough ADC tests (r/m8, r/m16) with dual-pass (CF=0, CF=1), DOS exit
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   ADC sets CF, PF, AF, ZF, SF, OF. We snapshot FLAGS right after the op.

bits 16
org 0x100

; ---------- Macros ----------
%macro SAVE_FLAGS 0
    push ax
    pushf
    pop  ax
    mov  [flags_store], ax
    pop  ax
%endmacro

%macro ASSERT_AX 1
    mov bx, %1
    int 0x23
%endmacro

%macro ASSERT_BYTE 1
    ; AL already has the byte result; zero-extend to AX then compare
    xor ah, ah
    mov bx, %1
    int 0x23
%endmacro

%macro CHECK_CF 1
    mov ax, [flags_store]
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

%macro CHECK_PF 1
    mov ax, [flags_store]
    mov cl, 2
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

%macro CHECK_AF 1
    mov ax, [flags_store]
    mov cl, 4
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

%macro CHECK_ZF 1
    mov ax, [flags_store]
    mov cl, 6
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

%macro CHECK_SF 1
    mov ax, [flags_store]
    mov cl, 7
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

%macro CHECK_OF 1
    mov ax, [flags_store]
    mov cl, 11
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro


start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; Standard .COM: DS = CS
    push cs
    pop  ds

; ===================== 8-bit ADC (register) — dual pass =====================

; 1) AL + 00h, starting from 00h
;    CF=0: 00+00+0=00 (ZF=1, PF=1, AF=0, OF=0, SF=0, CF=0)
;    CF=1: 00+00+1=01 (ZF=0, PF=0, AF=0, OF=0, SF=0, CF=0)
    mov al, 0x00
    clc
    adc al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_CF 0

    mov al, 0x00
    stc
    adc al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x01
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_CF 0

; 2) AL + 00h, starting from FFh
;    CF=0: FF+00+0=FF (CF=0, ZF=0, SF=1, PF=1, AF=0, OF=0)
;    CF=1: FF+00+1=00 (CF=1, ZF=1, SF=0, PF=1, AF=1, OF=0)
    mov al, 0xFF
    clc
    adc al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0

    mov al, 0xFF
    stc
    adc al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0

; 3) AL + 00h, starting from 7Fh
;    CF=0: 7F+00+0=7F (no OF, AF=0)
;    CF=1: 7F+00+1=80 (OF=1, SF=1, AF=1)
    mov al, 0x7F
    clc
    adc al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x7F
    CHECK_OF 0
    CHECK_AF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_CF 0

    mov al, 0x7F
    stc
    adc al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_OF 1
    CHECK_AF 1
    CHECK_SF 1
    CHECK_PF 0
    CHECK_CF 0
    CHECK_ZF 0

; 4) AL + 80h, starting from 80h
;    CF=0: 80+80+0=00 (CF=1, OF=1, ZF=1, PF=1, AF=0, SF=0)
;    CF=1: 80+80+1=01 (CF=1, OF=1, ZF=0, PF=0, AF=1, SF=0)
    mov al, 0x80
    clc
    adc al, 0x80
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 0
    CHECK_SF 0

    mov al, 0x80
    stc
    adc al, 0x80
    SAVE_FLAGS
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_SF 0

; 5) AH + 00h, starting from 0Fh — AF focus
;    CF=0: 0F+00+0=0F (AF=0)
;    CF=1: 0F+00+1=10 (AF=1)
    mov ah, 0x0F
    clc
    adc ah, 0x00
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x0F
    CHECK_AF 0
    CHECK_CF 0

    mov ah, 0x0F
    stc
    adc ah, 0x00
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x10
    CHECK_AF 1
    CHECK_CF 0


; ===================== 8-bit ADC (memory, all addressing forms) — dual pass =====================

; 6) [si]: FF + 01
;    CF=0: -> 00 (CF=1, ZF=1, PF=1, AF=1)
;    CF=1: -> 01 (CF=1, ZF=0, PF=0, AF=1)
    lea si, [m8_si]
    mov byte [si], 0xFF
    clc
    adc byte [si], 0x01
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

    mov byte [si], 0xFF
    stc
    adc byte [si], 0x01
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 1

; 7) [di]: 7F + 01
;    CF=0: -> 80 (OF=1, SF=1, AF=1, CF=0, PF=0)
;    CF=1: -> 81 (OF=1, SF=1, AF=1, CF=0, PF=1)
    lea di, [m8_di]
    mov byte [di], 0x7F
    clc
    adc byte [di], 0x01
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x80
    CHECK_OF 1
    CHECK_SF 1
    CHECK_AF 1
    CHECK_CF 0
    CHECK_PF 0

    mov byte [di], 0x7F
    stc
    adc byte [di], 0x01
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x81
    CHECK_OF 1
    CHECK_SF 1
    CHECK_AF 1
    CHECK_CF 0
    CHECK_PF 1

; 8) [bx]: 7E + 01
;    CF=0: -> 7F (no OF/CF)
;    CF=1: -> 80 (OF=1, SF=1, AF=1)
    lea bx, [m8_bx]
    mov byte [bx], 0x7E
    clc
    adc byte [bx], 0x01
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x7F
    CHECK_OF 0
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0

    mov byte [bx], 0x7E
    stc
    adc byte [bx], 0x01
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x80
    CHECK_OF 1
    CHECK_SF 1
    CHECK_AF 1
    CHECK_CF 0
    CHECK_PF 0

; 9) [si+disp]: FE + 01
;    CF=0: -> FF (CF=0, SF=1, PF=1, AF=0)
;    CF=1: -> 00 (CF=1, ZF=1, PF=1, AF=1)
    lea si, [base8sid]
    mov byte [si + (m8_sid - base8sid)], 0xFE
    clc
    adc byte [si + (m8_sid - base8sid)], 0x01
    SAVE_FLAGS
    mov al, [si + (m8_sid - base8sid)]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 0

    mov byte [si + (m8_sid - base8sid)], 0xFE
    stc
    adc byte [si + (m8_sid - base8sid)], 0x01
    SAVE_FLAGS
    mov al, [si + (m8_sid - base8sid)]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 10) [di+disp]: 0F + 01
;     CF=0: -> 10 (AF=1, PF=0)
;     CF=1: -> 11 (AF=1, PF=1)
    lea di, [base8dd]
    mov byte [di + (m8_did - base8dd)], 0x0F
    clc
    adc byte [di + (m8_did - base8dd)], 0x01
    SAVE_FLAGS
    mov al, [di + (m8_did - base8dd)]
    ASSERT_BYTE 0x10
    CHECK_AF 1
    CHECK_PF 0
    CHECK_CF 0

    mov byte [di + (m8_did - base8dd)], 0x0F
    stc
    adc byte [di + (m8_did - base8dd)], 0x01
    SAVE_FLAGS
    mov al, [di + (m8_did - base8dd)]
    ASSERT_BYTE 0x11
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 0

; 11) [bx+disp]: 00 + 00
;     CF=0: -> 00 (all clear/zero parity)
;     CF=1: -> 01
    lea bx, [base8bd]
    mov byte [bx + (m8_bxd - base8bd)], 0x00
    clc
    adc byte [bx + (m8_bxd - base8bd)], 0x00
    SAVE_FLAGS
    mov al, [bx + (m8_bxd - base8bd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_ZF 1
    CHECK_PF 1

    mov byte [bx + (m8_bxd - base8bd)], 0x00
    stc
    adc byte [bx + (m8_bxd - base8bd)], 0x00
    SAVE_FLAGS
    mov al, [bx + (m8_bxd - base8bd)]
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_PF 0

; 12) [bx+si+disp]: 80 + 80
;     CF=0: -> 00 (CF=1, OF=1, ZF=1, PF=1, AF=0)
;     CF=1: -> 01 (CF=1, OF=1, ZF=0, PF=0, AF=1)
    lea bx, [base8]
    lea si, [index8]
    mov byte [bx+si + (m8_bxsi - base8 - index8)], 0x80
    clc
    adc byte [bx+si + (m8_bxsi - base8 - index8)], 0x80
    SAVE_FLAGS
    mov al, [bx+si + (m8_bxsi - base8 - index8)]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 0

    mov byte [bx+si + (m8_bxsi - base8 - index8)], 0x80
    stc
    adc byte [bx+si + (m8_bxsi - base8 - index8)], 0x80
    SAVE_FLAGS
    mov al, [bx+si + (m8_bxsi - base8 - index8)]
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 13) [bp] (ds:): FF + 00
;     CF=0: -> FF (CF=0, AF=0)
;     CF=1: -> 00 (CF=1, AF=1)
    mov bp, m8_bp
    mov byte [ds:bp], 0xFF
    clc
    adc byte [ds:bp], 0x00
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_AF 0

    mov byte [ds:bp], 0xFF
    stc
    adc byte [ds:bp], 0x00
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_AF 1

; 14) [bp+disp] (ds:): 55 + 00
;     CF=0: -> 55 (PF=1, AF=0)
;     CF=1: -> 56 (PF=1, AF=0)
    lea bp, [base_bp_d]
    mov byte [ds:bp + (m8_bpd - base_bp_d)], 0x55
    clc
    adc byte [ds:bp + (m8_bpd - base_bp_d)], 0x00
    SAVE_FLAGS
    mov al, [ds:bp + (m8_bpd - base_bp_d)]
    ASSERT_BYTE 0x55
    CHECK_PF 1
    CHECK_AF 0
    CHECK_CF 0

    mov byte [ds:bp + (m8_bpd - base_bp_d)], 0x55
    stc
    adc byte [ds:bp + (m8_bpd - base_bp_d)], 0x00
    SAVE_FLAGS
    mov al, [ds:bp + (m8_bpd - base_bp_d)]
    ASSERT_BYTE 0x56
    CHECK_PF 1
    CHECK_AF 0
    CHECK_CF 0

; 15) [bp+si+disp] (ds:): 7E + 01
;     CF=0: -> 7F (no OF/CF)
;     CF=1: -> 80 (OF=1, AF=1, SF=1)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov byte [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)], 0x7E
    clc
    adc byte [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)], 0x01
    SAVE_FLAGS
    mov al, [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x7F
    CHECK_OF 0
    CHECK_CF 0
    CHECK_AF 0

    mov byte [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)], 0x7E
    stc
    adc byte [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)], 0x01
    SAVE_FLAGS
    mov al, [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x80
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_SF 1

; 16) [bp+di+disp] (ds:): 7F + 01
;     CF=0: -> 80 (OF=1)
;     CF=1: -> 81 (OF=1)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov byte [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)], 0x7F
    clc
    adc byte [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)], 0x01
    SAVE_FLAGS
    mov al, [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x80
    CHECK_OF 1

    mov byte [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)], 0x7F
    stc
    adc byte [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)], 0x01
    SAVE_FLAGS
    mov al, [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x81
    CHECK_OF 1


; ===================== 16-bit ADC (register) — dual pass =====================

; 17) AX + 0000, starting 0000
;     CF=0: -> 0000 (ZF=1, PF=1)
;     CF=1: -> 0001
    mov ax, 0x0000
    clc
    adc ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_CF 0

    mov ax, 0x0000
    stc
    adc ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0x0001
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_CF 0

; 18) AX + 0000, starting 7FFF
;     CF=0: -> 7FFF
;     CF=1: -> 8000 (OF=1, SF=1, AF=1)
    mov ax, 0x7FFF
    clc
    adc ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0x7FFF
    CHECK_OF 0
    CHECK_AF 0
    CHECK_CF 0

    mov ax, 0x7FFF
    stc
    adc ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1
    CHECK_AF 1
    CHECK_CF 0
    CHECK_PF 1          ; low byte 00

; 19) AX + 0000, starting FFFF
;     CF=0: -> FFFF (CF=0, PF=1, SF=1, AF=0)
;     CF=1: -> 0000 (CF=1, ZF=1, PF=1, AF=1)
    mov ax, 0xFFFF
    clc
    adc ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_PF 1
    CHECK_SF 1
    CHECK_AF 0

    mov ax, 0xFFFF
    stc
    adc ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 20) AX + 8000, starting 8000
;     CF=0: -> 0000 (CF=1, OF=1, ZF=1, PF=1, AF=0)
;     CF=1: -> 0001 (CF=1, OF=1, ZF=0, PF=0, AF=1)
    mov ax, 0x8000
    clc
    adc ax, 0x8000
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 0

    mov ax, 0x8000
    stc
    adc ax, 0x8000
    SAVE_FLAGS
    ASSERT_AX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 21) BX + 0001, starting 00FE
;     CF=0: -> 00FF (no OF/CF; PF=1; AF=0)
;     CF=1: -> 0100 (no OF/CF; PF=1; AF=1)
    mov bx, 0x00FE
    clc
    adc bx, 0x0001
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x00FF
    CHECK_OF 0
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0

    mov bx, 0x00FE
    stc
    adc bx, 0x0001
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x0100
    CHECK_OF 0
    CHECK_CF 0
    CHECK_PF 1          ; low byte 00
    CHECK_AF 1


; ===================== 16-bit ADC (memory, all addressing forms) — dual pass =====================

; 22) [si]: 7FFF + 0001
;     CF=0: -> 8000 (OF=1, SF=1, AF=1)
;     CF=1: -> 8001 (OF=1, SF=1, AF=1)
    lea si, [m16_si]
    mov word [si], 0x7FFF
    clc
    adc word [si], 0x0001
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1
    CHECK_AF 1

    mov word [si], 0x7FFF
    stc
    adc word [si], 0x0001
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x8001
    CHECK_OF 1
    CHECK_SF 1
    CHECK_AF 1

; 23) [di]: FFFF + 0000
;     CF=0: -> FFFF (CF=0, AF=0)
;     CF=1: -> 0000 (CF=1, ZF=1, PF=1, AF=1)
    lea di, [m16_di]
    mov word [di], 0xFFFF
    clc
    adc word [di], 0x0000
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_AF 0

    mov word [di], 0xFFFF
    stc
    adc word [di], 0x0000
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 24) [bx]: 0000 + 0001
;     CF=0: -> 0001
;     CF=1: -> 0002
    lea bx, [m16_bx]
    mov word [bx], 0x0000
    clc
    adc word [bx], 0x0001
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_OF 0

    mov word [bx], 0x0000
    stc
    adc word [bx], 0x0001
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0002
    CHECK_CF 0
    CHECK_OF 0

; 25) [si+disp]: 00FF + 0001
;     CF=0: -> 0100 (AF=1, PF=1)
;     CF=1: -> 0101 (AF=1, PF=0)
    lea si, [base16sid]
    mov word [si + (m16_sid - base16sid)], 0x00FF
    clc
    adc word [si + (m16_sid - base16sid)], 0x0001
    SAVE_FLAGS
    mov ax, [si + (m16_sid - base16sid)]
    ASSERT_AX 0x0100
    CHECK_AF 1
    CHECK_PF 1

    mov word [si + (m16_sid - base16sid)], 0x00FF
    stc
    adc word [si + (m16_sid - base16sid)], 0x0001
    SAVE_FLAGS
    mov ax, [si + (m16_sid - base16sid)]
    ASSERT_AX 0x0101
    CHECK_AF 1
    CHECK_PF 0

; 26) [di+disp]: 7FFE + 0001
;     CF=0: -> 7FFF
;     CF=1: -> 8000 (OF=1)
    lea di, [base16did]
    mov word [di + (m16_did - base16did)], 0x7FFE
    clc
    adc word [di + (m16_did - base16did)], 0x0001
    SAVE_FLAGS
    mov ax, [di + (m16_did - base16did)]
    ASSERT_AX 0x7FFF
    CHECK_OF 0

    mov word [di + (m16_did - base16did)], 0x7FFE
    stc
    adc word [di + (m16_did - base16did)], 0x0001
    SAVE_FLAGS
    mov ax, [di + (m16_did - base16did)]
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1

; 27) [bx+disp]: 8000 + 0000
;     CF=0: -> 8000
;     CF=1: -> 8001 (AF=1)
    lea bx, [base16bd]
    mov word [bx + (m16_bxd - base16bd)], 0x8000
    clc
    adc word [bx + (m16_bxd - base16bd)], 0x0000
    SAVE_FLAGS
    mov ax, [bx + (m16_bxd - base16bd)]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_AF 0

    mov word [bx + (m16_bxd - base16bd)], 0x8000
    stc
    adc word [bx + (m16_bxd - base16bd)], 0x0000
    SAVE_FLAGS
    mov ax, [bx + (m16_bxd - base16bd)]
    ASSERT_AX 0x8001
    CHECK_CF 0
    CHECK_AF 0
    CHECK_SF 1          ; low byte 01 -> SF=0 actually! (correct below)
    ; Adjust: SF follows bit15; here 0x8001 has bit15=1 -> SF=1 is correct.

; 28) [bx+si+disp]: 7FFF + 0001
;     CF=0: -> 8000 (OF=1)
;     CF=1: -> 8001 (OF=1)
    lea bx, [base16]
    lea si, [index16]
    mov word [bx+si + (m16_bxsi - base16 - index16)], 0x7FFF
    clc
    adc word [bx+si + (m16_bxsi - base16 - index16)], 0x0001
    SAVE_FLAGS
    mov ax, [bx+si + (m16_bxsi - base16 - index16)]
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1

    mov word [bx+si + (m16_bxsi - base16 - index16)], 0x7FFF
    stc
    adc word [bx+si + (m16_bxsi - base16 - index16)], 0x0001
    SAVE_FLAGS
    mov ax, [bx+si + (m16_bxsi - base16 - index16)]
    ASSERT_AX 0x8001
    CHECK_OF 1
    CHECK_SF 1

; 29) [bp] (ds:): 00FF + 0001
;     CF=0: -> 0100 (AF=1, PF=1)
;     CF=1: -> 0101 (AF=1, PF=0)
    mov bp, m16_bp
    mov word [ds:bp], 0x00FF
    clc
    adc word [ds:bp], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x0100
    CHECK_AF 1
    CHECK_PF 1

    mov word [ds:bp], 0x00FF
    stc
    adc word [ds:bp], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x0101
    CHECK_AF 1
    CHECK_PF 0

; 30) [bp+disp] (ds:): 1234 + 0001
;     CF=0: -> 1235
;     CF=1: -> 1236
    lea bp, [base_bp16_d]
    mov word [ds:bp + (m16_bpd - base_bp16_d)], 0x1234
    clc
    adc word [ds:bp + (m16_bpd - base_bp16_d)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp + (m16_bpd - base_bp16_d)]
    ASSERT_AX 0x1235
    CHECK_CF 0
    CHECK_OF 0

    mov word [ds:bp + (m16_bpd - base_bp16_d)], 0x1234
    stc
    adc word [ds:bp + (m16_bpd - base_bp16_d)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp + (m16_bpd - base_bp16_d)]
    ASSERT_AX 0x1236
    CHECK_CF 0
    CHECK_OF 0

; 31) [bp+si+disp] (ds:): 7FFE + 0001
;     CF=0: -> 7FFF
;     CF=1: -> 8000 (OF=1)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    mov word [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)], 0x7FFE
    clc
    adc word [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x7FFF
    CHECK_OF 0

    mov word [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)], 0x7FFE
    stc
    adc word [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1

; 32) [bp+di+disp] (ds:): 7FFF + 0001
;     CF=0: -> 8000 (OF=1)
;     CF=1: -> 8001 (OF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov word [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)], 0x7FFF
    clc
    adc word [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1

    mov word [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)], 0x7FFF
    stc
    adc word [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x8001
    CHECK_OF 1
    CHECK_SF 1


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; 8-bit memory locations
m8_si:     db 0
m8_di:     db 0
m8_bx:     db 0

base8sid:
m8_sid:    db 0

base8dd:
m8_did:    db 0

base8bd:
m8_bxd:    db 0

base8:
index8:
m8_bxsi:   db 0

m8_bp:     db 0

base_bp_d:
m8_bpd:    db 0

base_bp_si_A:
base_bp_si_B:
m8_bpsi:   db 0

base_bp_di_A:
base_bp_di_B:
m8_bpdi:   db 0

; 16-bit memory locations
m16_si:      dw 0
m16_di:      dw 0
m16_bx:      dw 0

base16sid:
m16_sid:     dw 0

base16did:
m16_did:     dw 0

base16bd:
m16_bxd:     dw 0

base16:
index16:
m16_bxsi:    dw 0

m16_bp:      dw 0

base_bp16_d:
m16_bpd:     dw 0

base_bp16_si_A:
base_bp16_si_B:
m16_bpsi:    dw 0

base_bp16_di_A:
base_bp16_di_B:
m16_bpdi:    dw 0

db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
stack:
