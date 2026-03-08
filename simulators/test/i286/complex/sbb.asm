; sbb.asm — thorough SBB tests (r/m8, r/m16) with dual-pass (CF=0, CF=1), DOS exit
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   SBB sets CF, PF, AF, ZF, SF, OF. We snapshot FLAGS right after SBB.

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

; ===================== 8-bit SBB (register) — dual pass =====================

; 1) AL start 00h, SBB 00h
;    CF=0: 00-00-0=00 (CF=0, ZF=1, PF=1, SF=0, OF=0, AF=0)
;    CF=1: 00-00-1=FF (CF=1, ZF=0, PF=1, SF=1, OF=0, AF=1)
    mov al, 0x00
    clc
    sbb al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0
    CHECK_OF 0
    CHECK_AF 0

    mov al, 0x00
    stc
    sbb al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_PF 1
    CHECK_SF 1
    CHECK_OF 0
    CHECK_AF 1

; 2) AL start 7Fh, SBB 00h
;    CF=0: 7F-00-0=7F (CF=0, OF=0, AF=0, PF=0)
;    CF=1: 7F-00-1=7E (CF=0, OF=0, AF=0, PF=1)
    mov al, 0x7F
    clc
    sbb al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_PF 0

    mov al, 0x7F
    stc
    sbb al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x7E
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_PF 1

; 3) AL start 80h, SBB 01h
;    CF=0: 80-01-0=7F (OF=1, CF=0, AF=1, PF=0, SF=0)
;    CF=1: 80-01-1=7E (OF=1, CF=0, AF=1, PF=1, SF=0)
    mov al, 0x80
    clc
    sbb al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x7F
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_PF 0
    CHECK_SF 0

    mov al, 0x80
    stc
    sbb al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x7E
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_SF 0

; 4) AL start 00h, SBB FFh
;    CF=0: 00-FF-0=01 (CF=1, OF=0, AF=1, PF=0, SF=0)
;    CF=1: 00-FF-1=00 (CF=1, OF=0, AF=1, PF=1, ZF=1, SF=0)
    mov al, 0x00
    clc
    sbb al, 0xFF
    SAVE_FLAGS
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 0
    CHECK_AF 1
    CHECK_PF 0
    CHECK_SF 0

    mov al, 0x00
    stc
    sbb al, 0xFF
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_ZF 1
    CHECK_SF 0

; 5) AH start 10h, SBB 00h — AF focus
;    CF=0: 10-00-0=10 (AF=0)
;    CF=1: 10-00-1=0F (AF=1, PF=1)
    mov ah, 0x10
    clc
    sbb ah, 0x00
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x10
    CHECK_AF 0
    CHECK_CF 0

    mov ah, 0x10
    stc
    sbb ah, 0x00
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x0F
    CHECK_AF 1
    CHECK_CF 0
    CHECK_PF 1


; ===================== 8-bit SBB (memory, all addressing forms) — dual pass =====================

; 6) [si]: start FF, SBB 01
;    CF=0: FF-01-0=FE (CF=0, OF=0, AF=0, SF=1, PF=0)
;    CF=1: FF-01-1=FD (CF=0, OF=0, AF=0, SF=1, PF=0)
    lea si, [m8_si]
    mov byte [si], 0xFF
    clc
    sbb byte [si], 0x01
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_SF 1
    CHECK_PF 0

    mov byte [si], 0xFF
    stc
    sbb byte [si], 0x01
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xFD
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_SF 1
    CHECK_PF 0

; 7) [di]: start 80, SBB 01
;    CF=0: 80-01-0=7F (OF=1, AF=1, PF=0, SF=0)
;    CF=1: 80-01-1=7E (OF=1, AF=1, PF=1, SF=0)
    lea di, [m8_di]
    mov byte [di], 0x80
    clc
    sbb byte [di], 0x01
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x7F
    CHECK_OF 1
    CHECK_AF 1
    CHECK_PF 0
    CHECK_SF 0

    mov byte [di], 0x80
    stc
    sbb byte [di], 0x01
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x7E
    CHECK_OF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_SF 0

; 8) [bx]: start 01, SBB 01
;    CF=0: 01-01-0=00 (CF=0, ZF=1, PF=1)
;    CF=1: 01-01-1=FF (CF=1, ZF=0, PF=1, AF=1, SF=1)
    lea bx, [m8_bx]
    mov byte [bx], 0x01
    clc
    sbb byte [bx], 0x01
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_ZF 1
    CHECK_PF 1

    mov byte [bx], 0x01
    stc
    sbb byte [bx], 0x01
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_PF 1
    CHECK_AF 1
    CHECK_SF 1

; 9) [si+disp]: start 00, SBB FF
;    CF=0: 00-FF-0=01 (CF=1, AF=1, PF=0)
;    CF=1: 00-FF-1=00 (CF=1, AF=1, PF=1, ZF=1)
    lea si, [base8sid]
    mov byte [si + (m8_sid - base8sid)], 0x00
    clc
    sbb byte [si + (m8_sid - base8sid)], 0xFF
    SAVE_FLAGS
    mov al, [si + (m8_sid - base8sid)]
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_AF 1
    CHECK_PF 0

    mov byte [si + (m8_sid - base8sid)], 0x00
    stc
    sbb byte [si + (m8_sid - base8sid)], 0xFF
    SAVE_FLAGS
    mov al, [si + (m8_sid - base8sid)]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_ZF 1

; 10) [di+disp]: start 10, SBB 01
;     CF=0: 10-01-0=0F (AF=1, PF=1)
;     CF=1: 10-01-1=0E (AF=1, PF=0)
    lea di, [base8dd]
    mov byte [di + (m8_did - base8dd)], 0x10
    clc
    sbb byte [di + (m8_did - base8dd)], 0x01
    SAVE_FLAGS
    mov al, [di + (m8_did - base8dd)]
    ASSERT_BYTE 0x0F
    CHECK_AF 1
    CHECK_PF 1

    mov byte [di + (m8_did - base8dd)], 0x10
    stc
    sbb byte [di + (m8_did - base8dd)], 0x01
    SAVE_FLAGS
    mov al, [di + (m8_did - base8dd)]
    ASSERT_BYTE 0x0E
    CHECK_AF 1
    CHECK_PF 0

; 11) [bx+disp]: start 00, SBB 00
;     CF=0: -> 00 (CF=0, ZF=1, PF=1)
;     CF=1: -> FF (CF=1, ZF=0, PF=1, AF=1, SF=1)
    lea bx, [base8bd]
    mov byte [bx + (m8_bxd - base8bd)], 0x00
    clc
    sbb byte [bx + (m8_bxd - base8bd)], 0x00
    SAVE_FLAGS
    mov al, [bx + (m8_bxd - base8bd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_ZF 1
    CHECK_PF 1

    mov byte [bx + (m8_bxd - base8bd)], 0x00
    stc
    sbb byte [bx + (m8_bxd - base8bd)], 0x00
    SAVE_FLAGS
    mov al, [bx + (m8_bxd - base8bd)]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_PF 1
    CHECK_AF 1
    CHECK_SF 1

; 12) [bx+si+disp]: start 80, SBB 80
;     CF=0: 80-80-0=00 (CF=0, ZF=1, PF=1, OF=0, AF=0)
;     CF=1: 80-80-1=FF (CF=1, ZF=0, PF=1, OF=0, AF=1, SF=1)
    lea bx, [base8]
    lea si, [index8]
    mov byte [bx+si + (m8_bxsi - base8 - index8)], 0x80
    clc
    sbb byte [bx+si + (m8_bxsi - base8 - index8)], 0x80
    SAVE_FLAGS
    mov al, [bx+si + (m8_bxsi - base8 - index8)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_OF 0
    CHECK_AF 0

    mov byte [bx+si + (m8_bxsi - base8 - index8)], 0x80
    stc
    sbb byte [bx+si + (m8_bxsi - base8 - index8)], 0x80
    SAVE_FLAGS
    mov al, [bx+si + (m8_bxsi - base8 - index8)]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_PF 1
    CHECK_OF 0
    CHECK_AF 1
    CHECK_SF 1

; 13) [bp] (ds:): start 00, SBB 01
;     CF=0: 00-01-0=FF (CF=1, AF=1, PF=1, SF=1)
;     CF=1: 00-01-1=FE (CF=1, AF=1, PF=0, SF=1)
    mov bp, m8_bp
    mov byte [ds:bp], 0x00
    clc
    sbb byte [ds:bp], 0x01
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_SF 1

    mov byte [ds:bp], 0x00
    stc
    sbb byte [ds:bp], 0x01
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFE
    CHECK_CF 1
    CHECK_AF 1
    CHECK_PF 0
    CHECK_SF 1

; 14) [bp+disp] (ds:): start 56, SBB 00
;     CF=0: -> 56 (CF=0, PF=1, AF=0)
;     CF=1: -> 55 (CF=0, PF=1, AF=1)
    lea bp, [base_bp_d]
    mov byte [ds:bp + (m8_bpd - base_bp_d)], 0x56
    clc
    sbb byte [ds:bp + (m8_bpd - base_bp_d)], 0x00
    SAVE_FLAGS
    mov al, [ds:bp + (m8_bpd - base_bp_d)]
    ASSERT_BYTE 0x56
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0

    mov byte [ds:bp + (m8_bpd - base_bp_d)], 0x56
    stc
    sbb byte [ds:bp + (m8_bpd - base_bp_d)], 0x00
    SAVE_FLAGS
    mov al, [ds:bp + (m8_bpd - base_bp_d)]
    ASSERT_BYTE 0x55
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0

; 15) [bp+si+disp] (ds:): start 7F, SBB 01
;     CF=0: -> 7E (no OF/CF; AF=0; PF=1)
;     CF=1: -> 7D (no OF/CF; AF=0; PF=0)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov byte [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)], 0x7F
    clc
    sbb byte [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)], 0x01
    SAVE_FLAGS
    mov al, [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x7E
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_PF 1

    mov byte [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)], 0x7F
    stc
    sbb byte [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)], 0x01
    SAVE_FLAGS
    mov al, [ds:bp+si + (m8_bpsi - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x7D
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_PF 1

; 16) [bp+di+disp] (ds:): start 80, SBB 80
;     CF=0: -> 00 (CF=0, ZF=1, PF=1, OF=0, AF=0)
;     CF=1: -> FF (CF=1, ZF=0, PF=1, OF=0, AF=1, SF=1)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov byte [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)], 0x80
    clc
    sbb byte [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)], 0x80
    SAVE_FLAGS
    mov al, [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_OF 0
    CHECK_AF 0

    mov byte [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)], 0x80
    stc
    sbb byte [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)], 0x80
    SAVE_FLAGS
    mov al, [ds:bp+di + (m8_bpdi - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_PF 1
    CHECK_OF 0
    CHECK_AF 1
    CHECK_SF 1


; ===================== 16-bit SBB (register) — dual pass =====================

; 17) AX start 0000, SBB 0000
;     CF=0: -> 0000 (CF=0, ZF=1, PF=1)
;     CF=1: -> FFFF (CF=1, ZF=0, PF=1, AF=1, SF=1)
    mov ax, 0x0000
    clc
    sbb ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_ZF 1
    CHECK_PF 1

    mov ax, 0x0000
    stc
    sbb ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_PF 1
    CHECK_AF 1
    CHECK_SF 1

; 18) AX start 8000, SBB 0001
;     CF=0: -> 7FFF (OF=1, CF=0, AF=1, PF=1, SF=0)
;     CF=1: -> 7FFE (OF=1, CF=0, AF=1, PF=0, SF=0)
    mov ax, 0x8000
    clc
    sbb ax, 0x0001
    SAVE_FLAGS
    ASSERT_AX 0x7FFF
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_SF 0

    mov ax, 0x8000
    stc
    sbb ax, 0x0001
    SAVE_FLAGS
    ASSERT_AX 0x7FFE
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_PF 0
    CHECK_SF 0

; 19) BX start 0100, SBB 0000
;     CF=0: -> 0100 (CF=0, PF=0)
;     CF=1: -> 00FF (CF=0, PF=1, AF=1)
    mov bx, 0x0100
    clc
    sbb bx, 0x0000
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x0100
    CHECK_CF 0
    CHECK_PF 1

    mov bx, 0x0100
    stc
    sbb bx, 0x0000
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x00FF
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 1


; ===================== 16-bit SBB (memory, all addressing forms) — dual pass =====================

; 20) [si]: start 7FFF, SBB 0001
;     CF=0: -> 7FFE (no OF/CF; AF=0; PF=0)
;     CF=1: -> 7FFD (no OF/CF; AF=0; PF=1)
    lea si, [m16_si]
    mov word [si], 0x7FFF
    clc
    sbb word [si], 0x0001
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x7FFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_PF 0

    mov word [si], 0x7FFF
    stc
    sbb word [si], 0x0001
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x7FFD
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_PF 0

; 21) [di]: start 8000, SBB 0001
;     CF=0: -> 7FFF (OF=1, AF=1, PF=1)
;     CF=1: -> 7FFE (OF=1, AF=1, PF=0)
    lea di, [m16_di]
    mov word [di], 0x8000
    clc
    sbb word [di], 0x0001
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x7FFF
    CHECK_OF 1
    CHECK_AF 1
    CHECK_PF 1

    mov word [di], 0x8000
    stc
    sbb word [di], 0x0001
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x7FFE
    CHECK_OF 1
    CHECK_AF 1
    CHECK_PF 0

; 22) [bx]: start 0000, SBB 0001
;     CF=0: -> FFFF (CF=1, AF=1, PF=1, SF=1)
;     CF=1: -> FFFE (CF=1, AF=1, PF=0, SF=1)
    lea bx, [m16_bx]
    mov word [bx], 0x0000
    clc
    sbb word [bx], 0x0001
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_SF 1

    mov word [bx], 0x0000
    stc
    sbb word [bx], 0x0001
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0xFFFE
    CHECK_CF 1
    CHECK_AF 1
    CHECK_PF 0
    CHECK_SF 1

; 23) [si+disp]: start 0100, SBB 0000
;     CF=0: -> 0100 (CF=0, PF=0)
;     CF=1: -> 00FF (CF=0, PF=1, AF=1)
    lea si, [base16sid]
    mov word [si + (m16_sid - base16sid)], 0x0100
    clc
    sbb word [si + (m16_sid - base16sid)], 0x0000
    SAVE_FLAGS
    mov ax, [si + (m16_sid - base16sid)]
    ASSERT_AX 0x0100
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0

    mov word [si + (m16_sid - base16sid)], 0x0100
    stc
    sbb word [si + (m16_sid - base16sid)], 0x0000
    SAVE_FLAGS
    mov ax, [si + (m16_sid - base16sid)]
    ASSERT_AX 0x00FF
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 1

; 24) [di+disp]: start 7FFF, SBB 0001
;     CF=0: -> 7FFE (no OF/CF; PF=0)
;     CF=1: -> 7FFD (no OF/CF; PF=1)
    lea di, [base16did]
    mov word [di + (m16_did - base16did)], 0x7FFF
    clc
    sbb word [di + (m16_did - base16did)], 0x0001
    SAVE_FLAGS
    mov ax, [di + (m16_did - base16did)]
    ASSERT_AX 0x7FFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_PF 0

    mov word [di + (m16_did - base16did)], 0x7FFF
    stc
    sbb word [di + (m16_did - base16did)], 0x0001
    SAVE_FLAGS
    mov ax, [di + (m16_did - base16did)]
    ASSERT_AX 0x7FFD
    CHECK_CF 0
    CHECK_OF 0
    CHECK_PF 0

; 25) [bx+disp]: start 8000, SBB 0000
;     CF=0: -> 8000 (CF=0, OF=0, PF=1, SF=1, AF=0)
;     CF=1: -> 7FFF (CF=0, OF=1, PF=1, SF=0, AF=1)
    lea bx, [base16bd]
    mov word [bx + (m16_bxd - base16bd)], 0x8000
    clc
    sbb word [bx + (m16_bxd - base16bd)], 0x0000
    SAVE_FLAGS
    mov ax, [bx + (m16_bxd - base16bd)]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_PF 1
    CHECK_SF 1
    CHECK_AF 0

    mov word [bx + (m16_bxd - base16bd)], 0x8000
    stc
    sbb word [bx + (m16_bxd - base16bd)], 0x0000
    SAVE_FLAGS
    mov ax, [bx + (m16_bxd - base16bd)]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 1
    CHECK_PF 1
    CHECK_SF 0
    CHECK_AF 1

; 26) [bx+si+disp]: start 8000, SBB 8000
;     CF=0: -> 0000 (CF=0, ZF=1, PF=1, OF=0, AF=0)
;     CF=1: -> FFFF (CF=1, ZF=0, PF=1, OF=0, AF=1, SF=1)
    lea bx, [base16]
    lea si, [index16]
    mov word [bx+si + (m16_bxsi - base16 - index16)], 0x8000
    clc
    sbb word [bx+si + (m16_bxsi - base16 - index16)], 0x8000
    SAVE_FLAGS
    mov ax, [bx+si + (m16_bxsi - base16 - index16)]
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_OF 0
    CHECK_AF 0

    mov word [bx+si + (m16_bxsi - base16 - index16)], 0x8000
    stc
    sbb word [bx+si + (m16_bxsi - base16 - index16)], 0x8000
    SAVE_FLAGS
    mov ax, [bx+si + (m16_bxsi - base16 - index16)]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_PF 1
    CHECK_OF 0
    CHECK_AF 1
    CHECK_SF 1

; 27) [bp] (ds:): start 00FF, SBB 0000
;     CF=0: -> 00FF (CF=0, PF=1, AF=0)
;     CF=1: -> 00FE (CF=0, PF=0, AF=1)
    mov bp, m16_bp
    mov word [ds:bp], 0x00FF
    clc
    sbb word [ds:bp], 0x0000
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x00FF
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0

    mov word [ds:bp], 0x00FF
    stc
    sbb word [ds:bp], 0x0000
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x00FE
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0

; 28) [bp+disp] (ds:): start 1234, SBB 0001
;     CF=0: -> 1233 (CF=0, PF=0, AF=0)
;     CF=1: -> 1232 (CF=0, PF=1, AF=0)
    lea bp, [base_bp16_d]
    mov word [ds:bp + (m16_bpd - base_bp16_d)], 0x1234
    clc
    sbb word [ds:bp + (m16_bpd - base_bp16_d)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp + (m16_bpd - base_bp16_d)]
    ASSERT_AX 0x1233
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0

    mov word [ds:bp + (m16_bpd - base_bp16_d)], 0x1234
    stc
    sbb word [ds:bp + (m16_bpd - base_bp16_d)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp + (m16_bpd - base_bp16_d)]
    ASSERT_AX 0x1232
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0

; 29) [bp+si+disp] (ds:): start 8000, SBB 0001
;     CF=0: -> 7FFF (OF=1, CF=0, AF=1, PF=1)
;     CF=1: -> 7FFE (OF=1, CF=0, AF=1, PF=0)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    mov word [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)], 0x8000
    clc
    sbb word [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x7FFF
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_PF 1

    mov word [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)], 0x8000
    stc
    sbb word [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+si + (m16_bpsi - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x7FFE
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_PF 0

; 30) [bp+di+disp] (ds:): start 0000, SBB 0001
;     CF=0: -> FFFF (CF=1, AF=1, PF=1, SF=1)
;     CF=1: -> FFFE (CF=1, AF=1, PF=0, SF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov word [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)], 0x0000
    clc
    sbb word [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_SF 1

    mov word [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)], 0x0000
    stc
    sbb word [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+di + (m16_bpdi - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0xFFFE
    CHECK_CF 1
    CHECK_AF 1
    CHECK_PF 0
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
