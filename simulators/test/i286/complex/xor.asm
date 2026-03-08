; xor.asm — thorough XOR tests (r/m8, r/m16) with saved FLAGS and DOS exit
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   XOR clears CF and OF to 0; sets ZF/SF/PF from result; AF is undefined (not checked).

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

; ===================== 8-bit XOR (register) =====================

; 1) AL=00 ^ 00 -> 00  (CF=0, OF=0, ZF=1, SF=0, PF=1)
    mov al, 0x00
    xor al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 2) AL=7F ^ 01 -> 7E  (CF=0, OF=0, ZF=0, SF=0, PF=1)
    mov al, 0x7F
    xor al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x7E
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 3) AL=80 ^ 80 -> 00  (CF=0, OF=0, ZF=1, SF=0, PF=1)
    mov al, 0x80
    xor al, 0x80
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 4) AL=F0 ^ 0F -> FF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    mov al, 0xF0
    xor al, 0x0F
    SAVE_FLAGS
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 5) AH=AA ^ 55 -> FF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    mov ah, 0xAA
    xor ah, 0x55
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 6) Precondition CF=1 and OF=1; XOR must clear both
;    Make OF=1 via ADD, set CF=1 via STC, then XOR AL,AL -> 00
    stc
    mov bl, 0x7F
    add bl, 0x01          ; OF=1
    mov al, 0x55          ; MOV doesn't modify flags
    xor al, al
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== 8-bit XOR (memory, all addressing forms) =====================

; 7) [si]:  FF ^ 0F -> F0  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    lea si, [xor8_si_ff]
    xor byte [si], 0x0F
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xF0
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 8) [di]:  80 ^ 01 -> 81  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    lea di, [xor8_di_80]
    xor byte [di], 0x01
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x81
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 9) [bx]:  7E ^ 01 -> 7F  (CF=0, OF=0, ZF=0, SF=0, PF=0)
    lea bx, [xor8_bx_7e]
    xor byte [bx], 0x01
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 10) [si+disp]:  FE ^ 0F -> F1  (CF=0, OF=0, ZF=0, SF=1, PF=0)
    lea si, [base8sid]
    xor byte [si + (xor8_sid_fe - base8sid)], 0x0F
    SAVE_FLAGS
    mov al, [si + (xor8_sid_fe - base8sid)]
    ASSERT_BYTE 0xF1
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 11) [di+disp]:  0F ^ 01 -> 0E  (CF=0, OF=0, ZF=0, SF=0, PF=0)
    lea di, [base8dd]
    xor byte [di + (xor8_did_0f - base8dd)], 0x01
    SAVE_FLAGS
    mov al, [di + (xor8_did_0f - base8dd)]
    ASSERT_BYTE 0x0E
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 12) [bx+disp]:  00 ^ 00 -> 00  (CF=0, OF=0, ZF=1, SF=0, PF=1)
    lea bx, [base8bd]
    xor byte [bx + (xor8_bxd_00 - base8bd)], 0x00
    SAVE_FLAGS
    mov al, [bx + (xor8_bxd_00 - base8bd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 13) [bx+si+disp]:  80 ^ 80 -> 00  (CF=0, OF=0, ZF=1, SF=0, PF=1)
    lea bx, [base8]
    lea si, [index8]
    xor byte [bx+si + (xor8_bxsi_80 - base8 - index8)], 0x80
    SAVE_FLAGS
    mov al, [bx+si + (xor8_bxsi_80 - base8 - index8)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 14) [bp] (ds:):  FF ^ 00 -> FF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    mov bp, xor8_bp_ff
    xor byte [ds:bp], 0x00
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 15) [bp+disp] (ds:):  55 ^ AA -> FF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    lea bp, [base_bp_d]
    xor byte [ds:bp + (xor8_bpd_55 - base_bp_d)], 0xAA
    SAVE_FLAGS
    mov al, [ds:bp + (xor8_bpd_55 - base_bp_d)]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 16) [bp+si+disp] (ds:):  7E ^ 3F -> 41  (CF=0, OF=0, ZF=0, SF=0, PF=1)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    xor byte [ds:bp+si + (xor8_bpsi_7e - base_bp_si_A - base_bp_si_B)], 0x3F
    SAVE_FLAGS
    mov al, [ds:bp+si + (xor8_bpsi_7e - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x41
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 17) [bp+di+disp] (ds:):  7F ^ 80 -> FF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    xor byte [ds:bp+di + (xor8_bpdi_7f - base_bp_di_A - base_bp_di_B)], 0x80
    SAVE_FLAGS
    mov al, [ds:bp+di + (xor8_bpdi_7f - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ===================== 16-bit XOR (register) =====================

; 18) AX=0000 ^ 0000 -> 0000  (CF=0, OF=0, ZF=1, SF=0, PF=1)
    mov ax, 0x0000
    xor ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 19) AX=7FFF ^ 8000 -> FFFF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    mov ax, 0x7FFF
    xor ax, 0x8000
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 20) AX=8000 ^ 8000 -> 0000  (CF=0, OF=0, ZF=1, SF=0, PF=1)
    mov ax, 0x8000
    xor ax, 0x8000
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 21) Precondition CF=1 and OF=1; XOR must clear both (word)
;     Make OF=1 via ADD, CF=1 via STC, then XOR AX,AX -> 0000
    stc
    mov cx, 0x7FFF
    add cx, 0x0001        ; OF=1
    mov ax, 0x1234
    xor ax, ax
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== 16-bit XOR (memory, all addressing forms) =====================

; 22) [si]:  7FFF ^ 0001 -> 7FFE  (CF=0, OF=0, ZF=0, SF=0, PF=0)
    lea si, [xor16_si_7fff]
    xor word [si], 0x0001
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x7FFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0           ; low byte FE → odd parity

; 23) [di]:  FFFF ^ 0000 -> FFFF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    lea di, [xor16_di_ffff]
    xor word [di], 0x0000
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 24) [bx]:  8001 ^ 0001 -> 8000  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    lea bx, [xor16_bx_8001]
    xor word [bx], 0x0001
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1           ; low byte 00 → even parity

; 25) [si+disp]:  00FF ^ 00F0 -> 000F  (CF=0, OF=0, ZF=0, SF=0, PF=1)
    lea si, [base16sid]
    xor word [si + (xor16_sid_00ff - base16sid)], 0x00F0
    SAVE_FLAGS
    mov ax, [si + (xor16_sid_00ff - base16sid)]
    ASSERT_AX 0x000F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 26) [di+disp]:  7FFE ^ 0001 -> 7FFF  (CF=0, OF=0, ZF=0, SF=0, PF=1)
    lea di, [base16did]
    xor word [di + (xor16_did_7ffe - base16did)], 0x0001
    SAVE_FLAGS
    mov ax, [di + (xor16_did_7ffe - base16did)]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 27) [bx+disp]:  8000 ^ FFFF -> 7FFF  (CF=0, OF=0, ZF=0, SF=0, PF=1)
    lea bx, [base16bd]
    xor word [bx + (xor16_bxd_8000 - base16bd)], 0xFFFF
    SAVE_FLAGS
    mov ax, [bx + (xor16_bxd_8000 - base16bd)]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 28) [bx+si+disp]:  7FFF ^ 8000 -> FFFF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    lea bx, [base16]
    lea si, [index16]
    xor word [bx+si + (xor16_bxsi_7fff - base16 - index16)], 0x8000
    SAVE_FLAGS
    mov ax, [bx+si + (xor16_bxsi_7fff - base16 - index16)]
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 29) [bp] (ds:):  00FF ^ 0101 -> 01FE  (CF=0, OF=0, ZF=0, SF=0, PF=0)
    mov bp, xor16_bp_00ff
    xor word [ds:bp], 0x0101
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x01FE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0          ; low byte FE → odd parity

; 30) [bp+disp] (ds:):  1234 ^ 00FF -> 12CB  (CF=0, OF=0, ZF=0, SF=0, PF=0)
    lea bp, [base_bp16_d]
    xor word [ds:bp + (xor16_bpd_1234 - base_bp16_d)], 0x00FF
    SAVE_FLAGS
    mov ax, [ds:bp + (xor16_bpd_1234 - base_bp16_d)]
    ASSERT_AX 0x12CB
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0          ; low byte CB has 5 ones → odd

; 31) [bp+si+disp] (ds:):  7FFE ^ 7FFF -> 0001  (CF=0, OF=0, ZF=0, SF=0, PF=0)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    xor word [ds:bp+si + (xor16_bpsi_7ffe - base_bp16_si_A - base_bp16_si_B)], 0x7FFF
    SAVE_FLAGS
    mov ax, [ds:bp+si + (xor16_bpsi_7ffe - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 32) [bp+di+disp] (ds:):  8000 ^ 8000 -> 0000  (CF=0, OF=0, ZF=1, SF=0, PF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    xor word [ds:bp+di + (xor16_bpdi_8000 - base_bp16_di_A - base_bp16_di_B)], 0x8000
    SAVE_FLAGS
    mov ax, [ds:bp+di + (xor16_bpdi_8000 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; 8-bit memory operands
xor8_si_ff:     db 0xFF
xor8_di_80:     db 0x80
xor8_bx_7e:     db 0x7E

base8sid:
xor8_sid_fe:    db 0xFE

base8dd:
xor8_did_0f:    db 0x0F

base8bd:
xor8_bxd_00:    db 0x00

base8:
index8:
xor8_bxsi_80:   db 0x80

xor8_bp_ff:     db 0xFF

base_bp_d:
xor8_bpd_55:    db 0x55

base_bp_si_A:
base_bp_si_B:
xor8_bpsi_7e:   db 0x7E

base_bp_di_A:
base_bp_di_B:
xor8_bpdi_7f:   db 0x7F

; 16-bit memory operands
xor16_si_7fff:    dw 0x7FFF
xor16_di_ffff:    dw 0xFFFF
xor16_bx_8001:    dw 0x8001

base16sid:
xor16_sid_00ff:   dw 0x00FF

base16did:
xor16_did_7ffe:   dw 0x7FFE

base16bd:
xor16_bxd_8000:   dw 0x8000

base16:
index16:
xor16_bxsi_7fff:  dw 0x7FFF

xor16_bp_00ff:    dw 0x00FF

base_bp16_d:
xor16_bpd_1234:   dw 0x1234

base_bp16_si_A:
base_bp16_si_B:
xor16_bpsi_7ffe:  dw 0x7FFE

base_bp16_di_A:
base_bp16_di_B:
xor16_bpdi_8000:  dw 0x8000

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
