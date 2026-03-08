; dec.asm — thorough DEC tests (r/m8, r/m16) with saved FLAGS and DOS exit
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   DEC affects OF, SF, ZF, AF, PF; does NOT affect CF (must remain unchanged).

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

; ===================== 8-bit DEC (register) =====================

; 1) clc; AL=00 -> DEC -> FF  (ZF=0, SF=1, PF=1, AF=1, OF=0, CF unchanged=0)
    clc
    mov al, 0x00
    dec al
    SAVE_FLAGS
    ASSERT_BYTE 0xFF
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0
    CHECK_CF 0

; 2) stc; AL=80 -> DEC -> 7F  (OF=1, SF=0, ZF=0, PF=0, AF=1, CF unchanged=1)
    stc
    mov al, 0x80
    dec al
    SAVE_FLAGS
    ASSERT_BYTE 0x7F
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 1
    CHECK_CF 1

; 3) clc; AL=10 -> DEC -> 0F  (AF=1, PF=1, SF=0, ZF=0, OF=0, CF=0)
    clc
    mov al, 0x10
    dec al
    SAVE_FLAGS
    ASSERT_BYTE 0x0F
    CHECK_AF 1
    CHECK_PF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_OF 0
    CHECK_CF 0

; 4) stc; AH=01 -> DEC -> 00  (ZF=1, SF=0, PF=1, AF=0, OF=0, CF=1)
    stc
    mov ah, 0x01
    dec ah
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x00
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    CHECK_CF 1


; ===================== 8-bit DEC (memory, all addressing forms) =====================

; 5) [si]:  00 -> FF  (ZF=0, SF=1, PF=1, AF=1, OF=0, CF=0)
    lea si, [d8_si_00]
    clc
    dec byte [si]
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xFF
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0
    CHECK_CF 0

; 6) [di]:  80 -> 7F  (OF=1, SF=0, PF=0, AF=1, ZF=0, CF=1)
    lea di, [d8_di_80]
    stc
    dec byte [di]
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x7F
    CHECK_OF 1
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_CF 1

; 7) [bx]:  01 -> 00  (ZF=1, SF=0, PF=1, AF=0, OF=0, CF=0)
    lea bx, [d8_bx_01]
    clc
    dec byte [bx]
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x00
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    CHECK_CF 0

; 8) [si+disp]:  10 -> 0F  (AF=1, PF=1, OF=0, SF=0)
    lea si, [base8sid]
    dec byte [si + (d8_sid_10 - base8sid)]
    SAVE_FLAGS
    mov al, [si + (d8_sid_10 - base8sid)]
    ASSERT_BYTE 0x0F
    CHECK_AF 1
    CHECK_PF 1
    CHECK_OF 0
    CHECK_SF 0

; 9) [di+disp]:  7F -> 7E  (OF=0, SF=0, PF=1, AF=0)
    lea di, [base8dd]
    dec byte [di + (d8_did_7f - base8dd)]
    SAVE_FLAGS
    mov al, [di + (d8_did_7f - base8dd)]
    ASSERT_BYTE 0x7E
    CHECK_OF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 10) [bx+disp]:  F0 -> EF  (SF=1, PF=0, AF=1, OF=0)
    lea bx, [base8bd]
    dec byte [bx + (d8_bxd_f0 - base8bd)]
    SAVE_FLAGS
    mov al, [bx + (d8_bxd_f0 - base8bd)]
    ASSERT_BYTE 0xEF
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1
    CHECK_OF 0

; 11) [bx+si+disp]:  80 -> 7F  (OF=1, SF=0, PF=0, AF=1)
    lea bx, [base8]
    lea si, [index8]
    dec byte [bx+si + (d8_bxsi_80 - base8 - index8)]
    SAVE_FLAGS
    mov al, [bx+si + (d8_bxsi_80 - base8 - index8)]
    ASSERT_BYTE 0x7F
    CHECK_OF 1
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1

; 12) [bp] (ds:):  00 -> FF  (ZF=0, SF=1, PF=1, AF=1, OF=0)
    mov bp, d8_bp_00
    dec byte [ds:bp]
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFF
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0

; 13) [bp+disp] (ds:):  55 -> 54  (PF=0, AF=0, OF=0, SF=0, ZF=0)
    lea bp, [base_bp_d]
    dec byte [ds:bp + (d8_bpd_55 - base_bp_d)]
    SAVE_FLAGS
    mov al, [ds:bp + (d8_bpd_55 - base_bp_d)]
    ASSERT_BYTE 0x54
    CHECK_PF 0
    CHECK_AF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0

; 14) [bp+si+disp] (ds:):  01 -> 00  (ZF=1, PF=1, AF=0, OF=0, SF=0)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    dec byte [ds:bp+si + (d8_bpsi_01 - base_bp_si_A - base_bp_si_B)]
    SAVE_FLAGS
    mov al, [ds:bp+si + (d8_bpsi_01 - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x00
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    CHECK_SF 0

; 15) [bp+di+disp] (ds:):  80 -> 7F  (OF=1, SF=0, PF=0, AF=1)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    dec byte [ds:bp+di + (d8_bpdi_80 - base_bp_di_A - base_bp_di_B)]
    SAVE_FLAGS
    mov al, [ds:bp+di + (d8_bpdi_80 - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x7F
    CHECK_OF 1
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1


; ===================== 16-bit DEC (register) =====================

; 16) clc; AX=0000 -> FFFF  (ZF=0, SF=1, PF=1, AF=1, OF=0, CF=0)
    clc
    mov ax, 0x0000
    dec ax
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1          ; low byte FF
    CHECK_AF 1
    CHECK_OF 0
    CHECK_CF 0

; 17) stc; AX=8000 -> 7FFF  (OF=1, SF=0, PF=1, AF=1, ZF=0, CF=1)
    stc
    mov ax, 0x8000
    dec ax
    SAVE_FLAGS
    ASSERT_AX 0x7FFF
    CHECK_OF 1
    CHECK_SF 0
    CHECK_PF 1          ; low byte FF
    CHECK_AF 1          ; 00 -> FF in low byte
    CHECK_ZF 0
    CHECK_CF 1

; 18) clc; BX=0100 -> 00FF  (OF=0, PF=1, AF=1). Move result to AX to assert.
    clc
    mov bx, 0x0100
    dec bx
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x00FF
    CHECK_OF 0
    CHECK_PF 1          ; low byte FF
    CHECK_AF 1


; ===================== 16-bit DEC (memory, all addressing forms) =====================

; 19) [si]:  0000 -> FFFF  (ZF=0, SF=1, PF=1, AF=1)
    lea si, [d16_si_0000]
    dec word [si]
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0xFFFF
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1

; 20) [di]:  8000 -> 7FFF  (OF=1, SF=0, PF=1, AF=1)
    lea di, [d16_di_8000]
    dec word [di]
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x7FFF
    CHECK_OF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 1

; 21) [bx]:  0001 -> 0000  (ZF=1, SF=0, PF=1, AF=0, OF=0)
    lea bx, [d16_bx_0001]
    dec word [bx]
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0

; 22) [si+disp]:  0010 -> 000F  (AF=1, PF=1, SF=0, OF=0)
    lea si, [base16sid]
    dec word [si + (d16_sid_0010 - base16sid)]
    SAVE_FLAGS
    mov ax, [si + (d16_sid_0010 - base16sid)]
    ASSERT_AX 0x000F
    CHECK_AF 1
    CHECK_PF 1          ; low byte 0F
    CHECK_SF 0
    CHECK_OF 0

; 23) [di+disp]:  7FFF -> 7FFE  (OF=0, SF=0, PF=0, AF=0)
    lea di, [base16did]
    dec word [di + (d16_did_7fff - base16did)]
    SAVE_FLAGS
    mov ax, [di + (d16_did_7fff - base16did)]
    ASSERT_AX 0x7FFE
    CHECK_OF 0
    CHECK_SF 0
    CHECK_PF 0          ; low byte FE has 7 ones
    CHECK_AF 0

; 24) [bx+disp]:  8001 -> 8000  (SF=1, PF=1, AF=0, OF=0)
    lea bx, [base16bd]
    dec word [bx + (d16_bxd_8001 - base16bd)]
    SAVE_FLAGS
    mov ax, [bx + (d16_bxd_8001 - base16bd)]
    ASSERT_AX 0x8000
    CHECK_SF 1
    CHECK_PF 1          ; low byte 00
    CHECK_AF 0
    CHECK_OF 0

; 25) [bx+si+disp]:  8000 -> 7FFF  (OF=1, SF=0, PF=1, AF=1)
    lea bx, [base16]
    lea si, [index16]
    dec word [bx+si + (d16_bxsi_8000 - base16 - index16)]
    SAVE_FLAGS
    mov ax, [bx+si + (d16_bxsi_8000 - base16 - index16)]
    ASSERT_AX 0x7FFF
    CHECK_OF 1
    CHECK_SF 0
    CHECK_PF 1          ; low byte FF
    CHECK_AF 1

; 26) [bp] (ds:):  0010 -> 000F  (AF=1, PF=1, OF=0)
    mov bp, d16_bp_0010
    dec word [ds:bp]
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x000F
    CHECK_AF 1
    CHECK_PF 1
    CHECK_OF 0

; 27) [bp+disp] (ds:):  1235 -> 1234  (OF=0, PF=0, AF=0, SF=0, ZF=0)
    lea bp, [base_bp16_d]
    dec word [ds:bp + (d16_bpd_1235 - base_bp16_d)]
    SAVE_FLAGS
    mov ax, [ds:bp + (d16_bpd_1235 - base_bp16_d)]
    ASSERT_AX 0x1234
    CHECK_OF 0
    CHECK_PF 0          ; low byte 34 (3 ones)
    CHECK_AF 0
    CHECK_SF 0
    CHECK_ZF 0

; 28) [bp+si+disp] (ds:):  8000 -> 7FFF  (OF=1, SF=0, PF=1, AF=1)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    dec word [ds:bp+si + (d16_bpsi_8000 - base_bp16_si_A - base_bp16_si_B)]
    SAVE_FLAGS
    mov ax, [ds:bp+si + (d16_bpsi_8000 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x7FFF
    CHECK_OF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 1

; 29) [bp+di+disp] (ds:):  0000 -> FFFF  (ZF=0, SF=1, PF=1, AF=1, OF=0)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    dec word [ds:bp+di + (d16_bpdi_0000 - base_bp16_di_A - base_bp16_di_B)]
    SAVE_FLAGS
    mov ax, [ds:bp+di + (d16_bpdi_0000 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0xFFFF
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0

; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; 8-bit memory operands
d8_si_00:     db 0x00
d8_di_80:     db 0x80
d8_bx_01:     db 0x01

base8sid:
d8_sid_10:    db 0x10

base8dd:
d8_did_7f:    db 0x7F

base8bd:
d8_bxd_f0:    db 0xF0

base8:
index8:
d8_bxsi_80:   db 0x80

d8_bp_00:     db 0x00

base_bp_d:
d8_bpd_55:    db 0x55

base_bp_si_A:
base_bp_si_B:
d8_bpsi_01:   db 0x01

base_bp_di_A:
base_bp_di_B:
d8_bpdi_80:   db 0x80

; 16-bit memory operands
d16_si_0000:    dw 0x0000
d16_di_8000:    dw 0x8000
d16_bx_0001:    dw 0x0001

base16sid:
d16_sid_0010:   dw 0x0010

base16did:
d16_did_7fff:   dw 0x7FFF

base16bd:
d16_bxd_8001:   dw 0x8001

base16:
index16:
d16_bxsi_8000:  dw 0x8000

d16_bp_0010:    dw 0x0010

base_bp16_d:
d16_bpd_1235:   dw 0x1235

base_bp16_si_A:
base_bp16_si_B:
d16_bpsi_8000:  dw 0x8000

base_bp16_di_A:
base_bp16_di_B:
d16_bpdi_0000:  dw 0x0000

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
