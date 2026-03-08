; or.asm — thorough OR tests (r/m8, r/m16) with saved FLAGS and DOS exit
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   OR clears CF and OF to 0; sets ZF/SF/PF from result; AF is undefined (not checked).

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

; ===================== 8-bit OR (register) =====================

; 1) AL=FF | 00 -> FF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    mov al, 0xFF
    or  al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 2) AL=7F | 01 -> 7F  (CF=0, OF=0, ZF=0, SF=0, PF=0)
    mov al, 0x7F
    or  al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 3) AL=80 | 80 -> 80  (CF=0, OF=0, ZF=0, SF=1, PF=0)
    mov al, 0x80
    or  al, 0x80
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 4) AL=F0 | 0F -> FF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    mov al, 0xF0
    or  al, 0x0F
    SAVE_FLAGS
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 5) AH=AA | 55 -> FF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    mov ah, 0xAA
    or  ah, 0x55
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 6) Precondition CF=1 and OF=1; OR must clear both
;    Set OF=1 via ADD, leave CF=1 via STC, then OR AL,00 -> 80
;    Expected: CF=0, OF=0, ZF=0, SF=1, PF=0
    stc
    mov bl, 0x7F
    add bl, 0x01          ; OF=1
    mov al, 0x80          ; MOV doesn't modify flags
    or  al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0


; ===================== 8-bit OR (memory, all addressing forms) =====================

; 7) [si]:  FF | 0F -> FF  (ZF=0, SF=1, PF=1)
    lea si, [or8_si_ff]
    or  byte [si], 0x0F
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 8) [di]:  80 | 01 -> 81  (ZF=0, SF=1, PF=1)
    lea di, [or8_di_80]
    or  byte [di], 0x01
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x81
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 9) [bx]:  7E | 01 -> 7F  (ZF=0, SF=0, PF=0)
    lea bx, [or8_bx_7e]
    or  byte [bx], 0x01
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 10) [si+disp]:  FE | 00 -> FE  (ZF=0, SF=1, PF=0)
    lea si, [base8sid]
    or  byte [si + (or8_sid_fe - base8sid)], 0x00
    SAVE_FLAGS
    mov al, [si + (or8_sid_fe - base8sid)]
    ASSERT_BYTE 0xFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 11) [di+disp]:  0F | 01 -> 0F  (ZF=0, SF=0, PF=1)
    lea di, [base8dd]
    or  byte [di + (or8_did_0f - base8dd)], 0x01
    SAVE_FLAGS
    mov al, [di + (or8_did_0f - base8dd)]
    ASSERT_BYTE 0x0F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 12) [bx+disp]:  00 | 00 -> 00  (ZF=1, SF=0, PF=1)
    lea bx, [base8bd]
    or  byte [bx + (or8_bxd_00 - base8bd)], 0x00
    SAVE_FLAGS
    mov al, [bx + (or8_bxd_00 - base8bd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 13) [bx+si+disp]:  80 | 80 -> 80  (ZF=0, SF=1, PF=0)
    lea bx, [base8]
    lea si, [index8]
    or  byte [bx+si + (or8_bxsi_80 - base8 - index8)], 0x80
    SAVE_FLAGS
    mov al, [bx+si + (or8_bxsi_80 - base8 - index8)]
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 14) [bp] (ds:):  FF | 00 -> 00FF? (byte) -> FF  (ZF=0, PF=1)
    mov bp, or8_bp_ff
    or  byte [ds:bp], 0x00
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_PF 1

; 15) [bp+disp] (ds:):  55 | AA -> FF  (ZF=0, SF=1, PF=1)
    lea bp, [base_bp_d]
    or  byte [ds:bp + (or8_bpd_55 - base_bp_d)], 0xAA
    SAVE_FLAGS
    mov al, [ds:bp + (or8_bpd_55 - base_bp_d)]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 16) [bp+si+disp] (ds:):  7E | 3F -> 7F  (ZF=0, SF=0, PF=0)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    or  byte [ds:bp+si + (or8_bpsi_7e - base_bp_si_A - base_bp_si_B)], 0x3F
    SAVE_FLAGS
    mov al, [ds:bp+si + (or8_bpsi_7e - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 17) [bp+di+disp] (ds:):  7F | 80 -> FF  (ZF=0, SF=1, PF=1)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    or  byte [ds:bp+di + (or8_bpdi_7f - base_bp_di_A - base_bp_di_B)], 0x80
    SAVE_FLAGS
    mov al, [ds:bp+di + (or8_bpdi_7f - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ===================== 16-bit OR (register) =====================

; 18) AX=FFFF | 0000 -> FFFF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    mov ax, 0xFFFF
    or  ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 19) AX=7FFF | 8000 -> FFFF  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    mov ax, 0x7FFF
    or  ax, 0x8000
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 20) AX=8000 | 8000 -> 8000  (CF=0, OF=0, ZF=0, SF=1, PF=1)
    mov ax, 0x8000
    or  ax, 0x8000
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1          ; low byte 00h → even parity

; 21) Precondition CF=1 and OF=1; OR must clear both (word)
;     Make OF=1 via ADD, CF=1 via STC, then OR AX,0000 -> 8000
    stc
    mov cx, 0x7FFF
    add cx, 0x0001        ; OF=1
    mov ax, 0x8000        ; MOV doesn't modify flags
    or  ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ===================== 16-bit OR (memory, all addressing forms) =====================

; 22) [si]:  7FFF | 0001 -> 7FFF  (ZF=0, SF=0, PF=1)
    lea si, [or16_si_7fff]
    or  word [si], 0x0001
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 23) [di]:  FFFF | 0000 -> FFFF  (ZF=0, SF=1, PF=1)
    lea di, [or16_di_ffff]
    or  word [di], 0x0000
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 24) [bx]:  8001 | 0001 -> 8001  (ZF=0, SF=1, PF=0)
    lea bx, [or16_bx_8001]
    or  word [bx], 0x0001
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x8001
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 25) [si+disp]:  00FF | 00F0 -> 00FF  (ZF=0, SF=0, PF=1)
    lea si, [base16sid]
    or  word [si + (or16_sid_00ff - base16sid)], 0x00F0
    SAVE_FLAGS
    mov ax, [si + (or16_sid_00ff - base16sid)]
    ASSERT_AX 0x00FF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 26) [di+disp]:  7FFE | 0001 -> 7FFF  (ZF=0, SF=0, PF=1)
    lea di, [base16did]
    or  word [di + (or16_did_7ffe - base16did)], 0x0001
    SAVE_FLAGS
    mov ax, [di + (or16_did_7ffe - base16did)]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 27) [bx+disp]:  8000 | FFFF -> FFFF  (ZF=0, SF=1, PF=1)
    lea bx, [base16bd]
    or  word [bx + (or16_bxd_8000 - base16bd)], 0xFFFF
    SAVE_FLAGS
    mov ax, [bx + (or16_bxd_8000 - base16bd)]
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 28) [bx+si+disp]:  7FFF | 8000 -> FFFF  (ZF=0, SF=1, PF=1)
    lea bx, [base16]
    lea si, [index16]
    or  word [bx+si + (or16_bxsi_7fff - base16 - index16)], 0x8000
    SAVE_FLAGS
    mov ax, [bx+si + (or16_bxsi_7fff - base16 - index16)]
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 29) [bp] (ds:):  00FF | 0101 -> 01FF  (ZF=0, SF=0, PF=1)
    mov bp, or16_bp_00ff
    or  word [ds:bp], 0x0101
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x01FF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 30) [bp+disp] (ds:):  1234 | 00FF -> 12FF  (ZF=0, SF=0, PF=1)
    lea bp, [base_bp16_d]
    or  word [ds:bp + (or16_bpd_1234 - base_bp16_d)], 0x00FF
    SAVE_FLAGS
    mov ax, [ds:bp + (or16_bpd_1234 - base_bp16_d)]
    ASSERT_AX 0x12FF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 31) [bp+si+disp] (ds:):  7FFE | 7FFF -> 7FFF  (ZF=0, SF=0, PF=1)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    or  word [ds:bp+si + (or16_bpsi_7ffe - base_bp16_si_A - base_bp16_si_B)], 0x7FFF
    SAVE_FLAGS
    mov ax, [ds:bp+si + (or16_bpsi_7ffe - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 32) [bp+di+disp] (ds:):  8000 | 8000 -> 8000  (ZF=0, SF=1, PF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    or  word [ds:bp+di + (or16_bpdi_8000 - base_bp16_di_A - base_bp16_di_B)], 0x8000
    SAVE_FLAGS
    mov ax, [ds:bp+di + (or16_bpdi_8000 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; 8-bit memory operands
or8_si_ff:     db 0xFF
or8_di_80:     db 0x80
or8_bx_7e:     db 0x7E

base8sid:
or8_sid_fe:    db 0xFE

base8dd:
or8_did_0f:    db 0x0F

base8bd:
or8_bxd_00:    db 0x00

base8:
index8:
or8_bxsi_80:   db 0x80

or8_bp_ff:     db 0xFF

base_bp_d:
or8_bpd_55:    db 0x55

base_bp_si_A:
base_bp_si_B:
or8_bpsi_7e:   db 0x7E

base_bp_di_A:
base_bp_di_B:
or8_bpdi_7f:   db 0x7F

; 16-bit memory operands
or16_si_7fff:    dw 0x7FFF
or16_di_ffff:    dw 0xFFFF
or16_bx_8001:    dw 0x8001

base16sid:
or16_sid_00ff:   dw 0x00FF

base16did:
or16_did_7ffe:   dw 0x7FFE

base16bd:
or16_bxd_8000:   dw 0x8000

base16:
index16:
or16_bxsi_7fff:  dw 0x7FFF

or16_bp_00ff:    dw 0x00FF

base_bp16_d:
or16_bpd_1234:   dw 0x1234

base_bp16_si_A:
base_bp16_si_B:
or16_bpsi_7ffe:  dw 0x7FFE

base_bp16_di_A:
base_bp16_di_B:
or16_bpdi_8000:  dw 0x8000

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
