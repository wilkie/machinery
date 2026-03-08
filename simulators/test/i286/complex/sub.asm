; sub.asm — thorough SUB tests (r/m8, r/m16) with saved FLAGS and DOS exit
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   SUB sets CF, PF, AF, ZF, SF, OF.

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

; ===================== 8-bit SUB (register) =====================

; 1) AL=00 - 00 -> 00  (ZF=1, PF=1, SF=0, CF=0, OF=0, AF=0)
    mov al, 0x00
    sub al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0

; 2) AL=7F - 01 -> 7E  (no CF/OF; PF=0; AF=0; SF=0; ZF=0)
    mov al, 0x7F
    sub al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x7E
    CHECK_CF 0
    CHECK_OF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_SF 0
    CHECK_ZF 0

; 3) AL=80 - 01 -> 7F  (OF=1, CF=0, AF=1, SF=0, PF=0, ZF=0)
    mov al, 0x80
    sub al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x7F
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_SF 0
    CHECK_PF 0
    CHECK_ZF 0

; 4) AL=00 - 01 -> FF  (CF=1, OF=0, AF=1, SF=1, PF=1, ZF=0)
    mov al, 0x00
    sub al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_AF 1
    CHECK_SF 1
    CHECK_PF 1
    CHECK_ZF 0

; 5) AL=80 - 80 -> 00  (CF=0, OF=0, AF=0, ZF=1, PF=1, SF=0)
    mov al, 0x80
    sub al, 0x80
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0

; 6) AH=10 - 01 -> 0F  (AF=1, PF=1, CF=0, OF=0, SF=0, ZF=0)
    mov ah, 0x10
    sub ah, 0x01
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x0F
    CHECK_AF 1
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_ZF 0


; ===================== 8-bit SUB (memory, all addressing forms) =====================

; 7) [si]:  00 - 01 -> FF  (CF=1, AF=1, SF=1, PF=1, OF=0, ZF=0)
    lea si, [s8_si_00]
    mov byte [si], 0x00
    sub byte [si], 0x01
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_AF 1
    CHECK_SF 1
    CHECK_PF 1
    CHECK_OF 0
    CHECK_ZF 0

; 8) [di]:  80 - 01 -> 7F  (OF=1, CF=0, AF=1, SF=0, PF=0)
    lea di, [s8_di_80]
    mov byte [di], 0x80
    sub byte [di], 0x01
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x7F
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_SF 0
    CHECK_PF 0

; 9) [bx]:  01 - 01 -> 00  (CF=0, OF=0, AF=0, ZF=1, PF=1, SF=0)
    lea bx, [s8_bx_01]
    mov byte [bx], 0x01
    sub byte [bx], 0x01
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0

; 10) [si+disp]:  00 - FF -> 01  (CF=1, OF=0, AF=1, SF=0, PF=0)
    lea si, [base8sid]
    mov byte [si + (s8_sid_00 - base8sid)], 0x00
    sub byte [si + (s8_sid_00 - base8sid)], 0xFF
    SAVE_FLAGS
    mov al, [si + (s8_sid_00 - base8sid)]
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 0
    CHECK_AF 1
    CHECK_SF 0
    CHECK_PF 0

; 11) [di+disp]:  10 - 01 -> 0F  (AF=1, CF=0, OF=0, SF=0, ZF=0, PF=1)
    lea di, [base8dd]
    mov byte [di + (s8_did_10 - base8dd)], 0x10
    sub byte [di + (s8_did_10 - base8dd)], 0x01
    SAVE_FLAGS
    mov al, [di + (s8_did_10 - base8dd)]
    ASSERT_BYTE 0x0F
    CHECK_AF 1
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 1

; 12) [bx+disp]:  00 - 00 -> 00  (CF=0, OF=0, AF=0, ZF=1, PF=1, SF=0)
    lea bx, [base8bd]
    mov byte [bx + (s8_bxd_00 - base8bd)], 0x00
    sub byte [bx + (s8_bxd_00 - base8bd)], 0x00
    SAVE_FLAGS
    mov al, [bx + (s8_bxd_00 - base8bd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0

; 13) [bx+si+disp]:  80 - 80 -> 00  (CF=0, OF=0, AF=0, ZF=1, PF=1)
    lea bx, [base8]
    lea si, [index8]
    mov byte [bx+si + (s8_bxsi_80 - base8 - index8)], 0x80
    sub byte [bx+si + (s8_bxsi_80 - base8 - index8)], 0x80
    SAVE_FLAGS
    mov al, [bx+si + (s8_bxsi_80 - base8 - index8)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_ZF 1
    CHECK_PF 1

; 14) [bp] (ds:):  00 - 01 -> FF  (CF=1, AF=1, SF=1, PF=1)
    mov bp, s8_bp_00
    mov byte [ds:bp], 0x00
    sub byte [ds:bp], 0x01
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_AF 1
    CHECK_SF 1
    CHECK_PF 1

; 15) [bp+disp] (ds:):  56 - 01 -> 55  (CF=0, OF=0, AF=0, PF=1, SF=0, ZF=0)
    lea bp, [base_bp_d]
    mov byte [ds:bp + (s8_bpd_56 - base_bp_d)], 0x56
    sub byte [ds:bp + (s8_bpd_56 - base_bp_d)], 0x01
    SAVE_FLAGS
    mov al, [ds:bp + (s8_bpd_56 - base_bp_d)]
    ASSERT_BYTE 0x55
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_PF 1
    CHECK_SF 0
    CHECK_ZF 0

; 16) [bp+si+disp] (ds:):  7F - 01 -> 7E  (no CF/OF; AF=0)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov byte [ds:bp+si + (s8_bpsi_7f - base_bp_si_A - base_bp_si_B)], 0x7F
    sub byte [ds:bp+si + (s8_bpsi_7f - base_bp_si_A - base_bp_si_B)], 0x01
    SAVE_FLAGS
    mov al, [ds:bp+si + (s8_bpsi_7f - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x7E
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0

; 17) [bp+di+disp] (ds:):  80 - 80 -> 00  (CF=0, OF=0, AF=0, ZF=1, PF=1)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov byte [ds:bp+di + (s8_bpdi_80 - base_bp_di_A - base_bp_di_B)], 0x80
    sub byte [ds:bp+di + (s8_bpdi_80 - base_bp_di_A - base_bp_di_B)], 0x80
    SAVE_FLAGS
    mov al, [ds:bp+di + (s8_bpdi_80 - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_ZF 1
    CHECK_PF 1


; ===================== 16-bit SUB (register) =====================

; 18) AX=0000 - 0000 -> 0000  (ZF=1, PF=1, SF=0, CF=0, OF=0, AF=0)
    mov ax, 0x0000
    sub ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0

; 19) AX=8000 - 0001 -> 7FFF  (OF=1, CF=0, AF=1, SF=0, PF=1, ZF=0)
    mov ax, 0x8000
    sub ax, 0x0001
    SAVE_FLAGS
    ASSERT_AX 0x7FFF
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_SF 0
    CHECK_PF 1       ; low byte FF
    CHECK_ZF 0

; 20) AX=0000 - 0001 -> FFFF  (CF=1, OF=0, AF=1, SF=1, PF=1, ZF=0)
    mov ax, 0x0000
    sub ax, 0x0001
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_AF 1
    CHECK_SF 1
    CHECK_PF 1
    CHECK_ZF 0

; 21) BX=0100 - 0001 -> 00FF (CF=0, OF=0, AF=1, PF=1). Move result to AX to assert.
    mov bx, 0x0100
    sub bx, 0x0001
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x00FF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 1
    CHECK_PF 1


; ===================== 16-bit SUB (memory, all addressing forms) =====================

; 22) [si]:  7FFF - 0001 -> 7FFE  (no CF/OF; AF=0; PF=0; SF=0)
    lea si, [s16_si_7fff]
    mov word [si], 0x7FFF
    sub word [si], 0x0001
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x7FFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_PF 0         ; low byte FE has odd parity
    CHECK_SF 0

; 23) [di]:  8000 - 0001 -> 7FFF  (OF=1, CF=0, AF=1, PF=1, SF=0)
    lea di, [s16_di_8000]
    mov word [di], 0x8000
    sub word [di], 0x0001
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x7FFF
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_SF 0

; 24) [bx]:  0000 - 0001 -> FFFF  (CF=1, OF=0, AF=1, PF=1, SF=1)
    lea bx, [s16_bx_0000]
    mov word [bx], 0x0000
    sub word [bx], 0x0001
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_SF 1

; 25) [si+disp]:  0100 - 0001 -> 00FF  (CF=0, OF=0, AF=1, PF=1)
    lea si, [base16sid]
    mov word [si + (s16_sid_0100 - base16sid)], 0x0100
    sub word [si + (s16_sid_0100 - base16sid)], 0x0001
    SAVE_FLAGS
    mov ax, [si + (s16_sid_0100 - base16sid)]
    ASSERT_AX 0x00FF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 1
    CHECK_PF 1

; 26) [di+disp]:  7FFF - 0001 -> 7FFE  (no CF/OF; PF=0; AF=0)
    lea di, [base16did]
    mov word [di + (s16_did_7fff - base16did)], 0x7FFF
    sub word [di + (s16_did_7fff - base16did)], 0x0001
    SAVE_FLAGS
    mov ax, [di + (s16_did_7fff - base16did)]
    ASSERT_AX 0x7FFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_PF 0
    CHECK_AF 0

; 27) [bx+disp]:  8000 - 0000 -> 8000  (CF=0, OF=0, AF=0, SF=1, PF=1)
    lea bx, [base16bd]
    mov word [bx + (s16_bxd_8000 - base16bd)], 0x8000
    sub word [bx + (s16_bxd_8000 - base16bd)], 0x0000
    SAVE_FLAGS
    mov ax, [bx + (s16_bxd_8000 - base16bd)]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_SF 1
    CHECK_PF 1

; 28) [bx+si+disp]:  8000 - 8000 -> 0000  (CF=0, OF=0, AF=0, ZF=1, PF=1, SF=0)
    lea bx, [base16]
    lea si, [index16]
    mov word [bx+si + (s16_bxsi_8000 - base16 - index16)], 0x8000
    sub word [bx+si + (s16_bxsi_8000 - base16 - index16)], 0x8000
    SAVE_FLAGS
    mov ax, [bx+si + (s16_bxsi_8000 - base16 - index16)]
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0

; 29) [bp] (ds:):  00FF - 0001 -> 00FE  (CF=0, OF=0, AF=0, PF=0)
    mov bp, s16_bp_00ff
    mov word [ds:bp], 0x00FF
    sub word [ds:bp], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x00FE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_AF 0
    CHECK_PF 0

; 30) [bp+disp] (ds:):  1234 - 0001 -> 1233  (no CF/OF; PF=1; AF=0; SF=0; ZF=0)
    lea bp, [base_bp16_d]
    mov word [ds:bp + (s16_bpd_1234 - base_bp16_d)], 0x1234
    sub word [ds:bp + (s16_bpd_1234 - base_bp16_d)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp + (s16_bpd_1234 - base_bp16_d)]
    ASSERT_AX 0x1233
    CHECK_CF 0
    CHECK_OF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_SF 0
    CHECK_ZF 0

; 31) [bp+si+disp] (ds:):  8000 - 0001 -> 7FFF  (OF=1, CF=0, AF=1, PF=1)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    mov word [ds:bp+si + (s16_bpsi_8000 - base_bp16_si_A - base_bp16_si_B)], 0x8000
    sub word [ds:bp+si + (s16_bpsi_8000 - base_bp16_si_A - base_bp16_si_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+si + (s16_bpsi_8000 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x7FFF
    CHECK_OF 1
    CHECK_CF 0
    CHECK_AF 1
    CHECK_PF 1

; 32) [bp+di+disp] (ds:):  0000 - 0001 -> FFFF  (CF=1, OF=0, AF=1, PF=1, SF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov word [ds:bp+di + (s16_bpdi_0000 - base_bp16_di_A - base_bp16_di_B)], 0x0000
    sub word [ds:bp+di + (s16_bpdi_0000 - base_bp16_di_A - base_bp16_di_B)], 0x0001
    SAVE_FLAGS
    mov ax, [ds:bp+di + (s16_bpdi_0000 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_SF 1


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; 8-bit memory operands
s8_si_00:     db 0x00
s8_di_80:     db 0x80
s8_bx_01:     db 0x01

base8sid:
s8_sid_00:    db 0x00

base8dd:
s8_did_10:    db 0x10

base8bd:
s8_bxd_00:    db 0x00

base8:
index8:
s8_bxsi_80:   db 0x80

s8_bp_00:     db 0x00

base_bp_d:
s8_bpd_56:    db 0x56

base_bp_si_A:
base_bp_si_B:
s8_bpsi_7f:   db 0x7F

base_bp_di_A:
base_bp_di_B:
s8_bpdi_80:   db 0x80

; 16-bit memory operands
s16_si_7fff:    dw 0x7FFF
s16_di_8000:    dw 0x8000
s16_bx_0000:    dw 0x0000

base16sid:
s16_sid_0100:   dw 0x0100

base16did:
s16_did_7fff:   dw 0x7FFF

base16bd:
s16_bxd_8000:   dw 0x8000

base16:
index16:
s16_bxsi_8000:  dw 0x8000

s16_bp_00ff:    dw 0x00FF

base_bp16_d:
s16_bpd_1234:   dw 0x1234

base_bp16_si_A:
base_bp16_si_B:
s16_bpsi_8000:  dw 0x8000

base_bp16_di_A:
base_bp16_di_B:
s16_bpdi_0000:  dw 0x0000

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
