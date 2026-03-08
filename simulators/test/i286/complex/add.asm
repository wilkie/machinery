; add.asm — thorough ADD tests (r/m8, r/m16) with saved FLAGS and DOS exit
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   ADD sets CF, PF, AF, ZF, SF, OF.

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

; ===================== 8-bit ADD (register) =====================

; 1) AL=00 + 00 -> 00  (ZF=1, SF=0, PF=1, AF=0, OF=0, CF=0)
    mov al, 0x00
    add al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    CHECK_CF 0

; 2) AL=7E + 01 -> 7F  (ZF=0, SF=0, PF=0, AF=0, OF=0, CF=0)
    mov al, 0x7E
    add al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x7F
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_OF 0
    CHECK_CF 0

; 3) AL=7F + 01 -> 80  (OF=1, SF=1, ZF=0, PF=0, AF=1, CF=0)
    mov al, 0x7F
    add al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 1
    CHECK_CF 0

; 4) AL=FF + 01 -> 00  (CF=1, ZF=1, OF=0, SF=0, PF=1, AF=1)
    mov al, 0xFF
    add al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 1

; 5) AL=80 + 80 -> 00  (CF=1, OF=1, ZF=1, SF=0, PF=1, AF=0)
    mov al, 0x80
    add al, 0x80
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 6) AH=0F + 01 -> 10  (AF=1, PF=0, OF=0, CF=0, ZF=0, SF=0)
    mov ah, 0x0F
    add ah, 0x01
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x10
    CHECK_AF 1
    CHECK_PF 0
    CHECK_OF 0
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0


; ===================== 8-bit ADD (memory, all addressing forms) =====================

; 7) [si]:  FF + 01 -> 00  (CF=1, ZF=1, PF=1, AF=1, OF=0, SF=0)
    lea si, [a8_si_ff]
    add byte [si], 0x01
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0
    CHECK_SF 0

; 8) [di]:  7F + 01 -> 80  (OF=1, SF=1, PF=0, AF=1, ZF=0, CF=0)
    lea di, [a8_di_7f]
    add byte [di], 0x01
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x80
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_CF 0

; 9) [bx]:  7E + 01 -> 7F  (no OF/CF; PF=0; AF=0)
    lea bx, [a8_bx_7e]
    add byte [bx], 0x01
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x7F
    CHECK_OF 0
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0

; 10) [si+disp]:  FE + 01 -> FF  (CF=0, OF=0, ZF=0, SF=1, PF=1, AF=0)
    lea si, [base8sid]
    add byte [si + (a8_sid_fe - base8sid)], 0x01
    SAVE_FLAGS
    mov al, [si + (a8_sid_fe - base8sid)]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 0

; 11) [di+disp]:  0F + 01 -> 10  (AF=1, PF=0, OF=0, CF=0, SF=0, ZF=0)
    lea di, [base8dd]
    add byte [di + (a8_did_0f - base8dd)], 0x01
    SAVE_FLAGS
    mov al, [di + (a8_did_0f - base8dd)]
    ASSERT_BYTE 0x10
    CHECK_AF 1
    CHECK_PF 0
    CHECK_OF 0
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0

; 12) [bx+disp]:  00 + 01 -> 01  (CF=0, OF=0, ZF=0, SF=0, PF=0, AF=0)
    lea bx, [base8bd]
    add byte [bx + (a8_bxd_00 - base8bd)], 0x01
    SAVE_FLAGS
    mov al, [bx + (a8_bxd_00 - base8bd)]
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 0

; 13) [bx+si+disp]:  80 + 80 -> 00  (CF=1, OF=1, ZF=1, PF=1, SF=0, AF=0)
    lea bx, [base8]
    lea si, [index8]
    add byte [bx+si + (a8_bxsi_80 - base8 - index8)], 0x80
    SAVE_FLAGS
    mov al, [bx+si + (a8_bxsi_80 - base8 - index8)]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0
    CHECK_AF 0

; 14) [bp] (ds:):  FF + 01 -> 00  (CF=1, ZF=1, PF=1, AF=1, OF=0)
    mov bp, a8_bp_ff
    add byte [ds:bp], 0x01
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0

; 15) [bp+disp] (ds:):  55 + 01 -> 56  (PF=1, AF=0, OF=0, CF=0, SF=0, ZF=0)
    lea bp, [base_bp_d]
    add byte [ds:bp + (a8_bpd_55 - base_bp_d)], 0x01
    SAVE_FLAGS
    mov al, [ds:bp + (a8_bpd_55 - base_bp_d)]
    ASSERT_BYTE 0x56
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0

; 16) [bp+si+disp] (ds:):  7E + 01 -> 7F  (no OF/CF; PF=0; AF=0)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    add byte [ds:bp+si + (a8_bpsi_7e - base_bp_si_A - base_bp_si_B)], 0x01
    SAVE_FLAGS
    mov al, [ds:bp+si + (a8_bpsi_7e - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x7F
    CHECK_OF 0
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0

; 17) [bp+di+disp] (ds:):  7F + 01 -> 80  (OF=1, SF=1, PF=0, AF=1, CF=0)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    add byte [ds:bp+di + (a8_bpdi_7f - base_bp_di_A - base_bp_di_B)], 0x01
    SAVE_FLAGS
    mov al, [ds:bp+di + (a8_bpdi_7f - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x80
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1
    CHECK_CF 0


; ===================== 16-bit ADD (register) =====================

; 18) AX=0000 + 0000 -> 0000  (ZF=1, SF=0, PF=1, AF=0, OF=0, CF=0)
    mov ax, 0x0000
    add ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    CHECK_CF 0

; 19) AX=7FFF + 0001 -> 8000  (OF=1, SF=1, PF=1, AF=1, ZF=0, CF=0)
    mov ax, 0x7FFF
    add ax, 0x0001
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 1      ; low byte 00
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_CF 0

; 20) AX=FFFF + 0001 -> 0000  (CF=1, ZF=1, PF=1, AF=1, OF=0, SF=0)
    mov ax, 0xFFFF
    add ax, 0x0001
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0
    CHECK_SF 0

; 21) BX=00FE + 0001 -> 00FF (no OF/CF; PF=1; AF=0) — move to AX to assert
    mov bx, 0x00FE
    add bx, 0x0001
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x00FF
    CHECK_OF 0
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0


; ===================== 16-bit ADD (memory, all addressing forms) =====================

; 22) [si]:  7FFF + 0001 -> 8000  (OF=1, SF=1, PF=1, AF=1, CF=0)
    lea si, [a16_si_7fff]
    add word [si], 0x0001
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_CF 0

; 23) [di]:  FFFF + 0001 -> 0000  (CF=1, ZF=1, PF=1, AF=1, OF=0, SF=0)
    lea di, [a16_di_ffff]
    add word [di], 0x0001
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0
    CHECK_SF 0

; 24) [bx]:  0000 + 0001 -> 0001  (no OF/CF; PF=0; AF=0; ZF=0; SF=0)
    lea bx, [a16_bx_0000]
    add word [bx], 0x0001
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0001
    CHECK_OF 0
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 25) [si+disp]:  00FF + 0001 -> 0100  (AF=1, PF=1, OF=0, SF=0, CF=0)
    lea si, [base16sid]
    add word [si + (a16_sid_00ff - base16sid)], 0x0001
    SAVE_FLAGS
    mov ax, [si + (a16_sid_00ff - base16sid)]
    ASSERT_AX 0x0100
    CHECK_AF 1
    CHECK_PF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_CF 0

; 26) [di+disp]:  7FFE + 0001 -> 7FFF  (no OF/CF; PF=1; AF=0)
    lea di, [base16did]
    add word [di + (a16_did_7ffe - base16did)], 0x0001
    SAVE_FLAGS
    mov ax, [di + (a16_did_7ffe - base16did)]
    ASSERT_AX 0x7FFF
    CHECK_OF 0
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0

; 27) [bx+disp]:  8000 + 0001 -> 8001  (SF=1; PF=0; AF=0; no OF/CF)
    lea bx, [base16bd]
    add word [bx + (a16_bxd_8000 - base16bd)], 0x0001
    SAVE_FLAGS
    mov ax, [bx + (a16_bxd_8000 - base16bd)]
    ASSERT_AX 0x8001
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 0
    CHECK_OF 0
    CHECK_CF 0

; 28) [bx+si+disp]:  7FFF + 0001 -> 8000  (OF=1, SF=1, PF=1, AF=1)
    lea bx, [base16]
    lea si, [index16]
    add word [bx+si + (a16_bxsi_7fff - base16 - index16)], 0x0001
    SAVE_FLAGS
    mov ax, [bx+si + (a16_bxsi_7fff - base16 - index16)]
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1

; 29) [bp] (ds:):  00FF + 0001 -> 0100  (AF=1, PF=1, OF=0, CF=0)
    mov bp, a16_bp_00ff
    add word [ds:bp], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x0100
    CHECK_AF 1
    CHECK_PF 1
    CHECK_OF 0
    CHECK_CF 0

; 30) [bp+disp] (ds:):  1234 + 0001 -> 1235  (no OF/CF; PF=1; AF=0; SF=0; ZF=0)
    lea bp, [base_bp16_d]
    add word [ds:bp + (a16_bpd_1234 - base_bp16_d)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp + (a16_bpd_1234 - base_bp16_d)]
    ASSERT_AX 0x1235
    CHECK_OF 0
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_SF 0
    CHECK_ZF 0

; 31) [bp+si+disp] (ds:):  7FFE + 0001 -> 7FFF  (no OF/CF; PF=1; AF=0)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    add word [ds:bp+si + (a16_bpsi_7ffe - base_bp16_si_A - base_bp16_si_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+si + (a16_bpsi_7ffe - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x7FFF
    CHECK_OF 0
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0

; 32) [bp+di+disp] (ds:):  7FFF + 0001 -> 8000  (OF=1, SF=1, PF=1, AF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    add word [ds:bp+di + (a16_bpdi_7fff - base_bp16_di_A - base_bp16_di_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+di + (a16_bpdi_7fff - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; 8-bit memory operands
a8_si_ff:     db 0xFF
a8_di_7f:     db 0x7F
a8_bx_7e:     db 0x7E

base8sid:
a8_sid_fe:    db 0xFE

base8dd:
a8_did_0f:    db 0x0F

base8bd:
a8_bxd_00:    db 0x00

base8:
index8:
a8_bxsi_80:   db 0x80

a8_bp_ff:     db 0xFF

base_bp_d:
a8_bpd_55:    db 0x55

base_bp_si_A:
base_bp_si_B:
a8_bpsi_7e:   db 0x7E

base_bp_di_A:
base_bp_di_B:
a8_bpdi_7f:   db 0x7F

; 16-bit memory operands
a16_si_7fff:    dw 0x7FFF
a16_di_ffff:    dw 0xFFFF
a16_bx_0000:    dw 0x0000

base16sid:
a16_sid_00ff:   dw 0x00FF

base16did:
a16_did_7ffe:   dw 0x7FFE

base16bd:
a16_bxd_8000:   dw 0x8000

base16:
index16:
a16_bxsi_7fff:  dw 0x7FFF

a16_bp_00ff:    dw 0x00FF

base_bp16_d:
a16_bpd_1234:   dw 0x1234

base_bp16_si_A:
base_bp16_si_B:
a16_bpsi_7ffe:  dw 0x7FFE

base_bp16_di_A:
base_bp16_di_B:
a16_bpdi_7fff:  dw 0x7FFF

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
