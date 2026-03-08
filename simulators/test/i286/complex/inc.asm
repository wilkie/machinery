; inc.asm — thorough INC tests (r/m8, r/m16) with saved FLAGS and DOS exit
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   INC affects OF, SF, ZF, AF, PF; does NOT affect CF (must remain unchanged).

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
    ; AL already has the byte; zero-extend to AX then compare to %1
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

; ===================== 8-bit INC (register) =====================

; 1) clc; AL=FF -> INC -> 00  (ZF=1, SF=0, PF=1, AF=1, OF=0, CF unchanged=0)
    clc
    mov al, 0xFF
    inc al
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0
    CHECK_CF 0

; 2) stc; AL=7E -> INC -> 7F  (ZF=0, SF=0, PF=0, AF=0, OF=0, CF unchanged=1)
    stc
    mov al, 0x7E
    inc al
    SAVE_FLAGS
    ASSERT_BYTE 0x7F
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_OF 0
    CHECK_CF 1

; 3) clc; AL=7F -> INC -> 80  (OF=1, SF=1, ZF=0, PF=0, AF=1, CF unchanged=0)
    clc
    mov al, 0x7F
    inc al
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 1
    CHECK_CF 0

; 4) stc; AH=0F -> INC -> 10  (AF=1) — exercise r/m8 on AH
    stc
    mov ah, 0x0F
    inc ah
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x10
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_OF 0
    CHECK_CF 1

; ===================== 8-bit INC (memory, all addressing forms) =====================

; 5) [si]:  FF -> 00  (ZF=1, PF=1, AF=1, OF=0, SF=0)
    lea si, [m8_si_ff]
    clc
    inc byte [si]
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x00
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_CF 0

; 6) [di]:  7F -> 80  (OF=1, SF=1, PF=0, AF=1, ZF=0)
    lea di, [m8_di_7f]
    stc
    inc byte [di]
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x80
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_CF 1

; 7) [bx]:  7E -> 7F  (no OF, PF=0, AF=0)
    lea bx, [m8_bx_7e]
    clc
    inc byte [bx]
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x7F
    CHECK_OF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_CF 0

; 8) [si+disp]:  FE -> FF  (SF=1, PF=1, ZF=0, AF=0, OF=0)
    lea si, [base8sid]
    inc byte [si + (m8_sid_fe - base8sid)]
    SAVE_FLAGS
    mov al, [si + (m8_sid_fe - base8sid)]
    ASSERT_BYTE 0xFF
    CHECK_SF 1
    CHECK_PF 1
    CHECK_ZF 0
    CHECK_AF 0
    CHECK_OF 0

; 9) [di+disp]:  0F -> 10  (AF=1, PF=0, OF=0, SF=0)
    lea di, [base8dd]
    inc byte [di + (m8_did_0f - base8dd)]
    SAVE_FLAGS
    mov al, [di + (m8_did_0f - base8dd)]
    ASSERT_BYTE 0x10
    CHECK_AF 1
    CHECK_PF 0
    CHECK_OF 0
    CHECK_SF 0

; 10) [bx+disp]:  00 -> 01  (ZF=0, SF=0, PF=0, AF=0, OF=0)
    lea bx, [base8bd]
    inc byte [bx + (m8_bxd_00 - base8bd)]
    SAVE_FLAGS
    mov al, [bx + (m8_bxd_00 - base8bd)]
    ASSERT_BYTE 0x01
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_OF 0

; 11) [bx+si+disp]:  7F -> 80  (OF=1, SF=1, PF=0, AF=1)
    lea bx, [base8]
    lea si, [index8]
    inc byte [bx+si + (m8_bxsi_7f - base8 - index8)]
    SAVE_FLAGS
    mov al, [bx+si + (m8_bxsi_7f - base8 - index8)]
    ASSERT_BYTE 0x80
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1

; 12) [bp] (ds:):  FF -> 00  (ZF=1, PF=1, AF=1, OF=0)
    mov bp, m8_bp_ff
    inc byte [ds:bp]
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0x00
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0

; 13) [bp+disp] (ds:):  55 -> 56  (PF=1, AF=0, others 0)
    lea bp, [base_bp_d]
    inc byte [ds:bp + (m8_bpd_55 - base_bp_d)]
    SAVE_FLAGS
    mov al, [ds:bp + (m8_bpd_55 - base_bp_d)]
    ASSERT_BYTE 0x56
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0

; 14) [bp+si+disp] (ds:):  7E -> 7F  (no OF, PF=0, AF=0)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    inc byte [ds:bp+si + (m8_bpsi_7e - base_bp_si_A - base_bp_si_B)]
    SAVE_FLAGS
    mov al, [ds:bp+si + (m8_bpsi_7e - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x7F
    CHECK_OF 0
    CHECK_PF 0
    CHECK_AF 0

; 15) [bp+di+disp] (ds:):  7F -> 80  (OF=1, SF=1, AF=1)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    inc byte [ds:bp+di + (m8_bpdi_7f - base_bp_di_A - base_bp_di_B)]
    SAVE_FLAGS
    mov al, [ds:bp+di + (m8_bpdi_7f - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x80
    CHECK_OF 1
    CHECK_SF 1
    CHECK_AF 1

; ===================== 16-bit INC (register) =====================

; 16) clc; AX=FFFF -> 0000  (ZF=1, SF=0, PF=1, AF=1, OF=0, CF unchanged=0)
    clc
    mov ax, 0xFFFF
    inc ax
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1          ; parity from low byte 00
    CHECK_AF 1
    CHECK_OF 0
    CHECK_CF 0

; 17) stc; AX=7FFF -> 8000  (OF=1, SF=1, PF=1, AF=1, ZF=0, CF unchanged=1)
    stc
    mov ax, 0x7FFF
    inc ax
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 1          ; low byte 00
    CHECK_AF 1          ; FF->00 in AL
    CHECK_ZF 0
    CHECK_CF 1

; 18) clc; BX=00FE -> 00FF (no OF; PF=1; AF=0). Move result to AX to assert.
    clc
    mov bx, 0x00FE
    inc bx
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x00FF
    CHECK_OF 0
    CHECK_PF 1          ; low byte FF (8 ones)
    CHECK_AF 0

; ===================== 16-bit INC (memory, all addressing forms) =====================

; 19) [si]:  7FFF -> 8000  (OF=1, SF=1, PF=1, AF=1)
    lea si, [m16_si_7fff]
    inc word [si]
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 1          ; low byte 00
    CHECK_AF 1

; 20) [di]:  FFFF -> 0000  (ZF=1, PF=1, AF=1, OF=0, SF=0)
    lea di, [m16_di_ffff]
    inc word [di]
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0
    CHECK_SF 0

; 21) [bx]:  0000 -> 0001  (no OF; PF=0; AF=0; ZF=0; SF=0)
    lea bx, [m16_bx_0000]
    inc word [bx]
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0001
    CHECK_OF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 22) [si+disp]:  00FF -> 0100  (AF=1, PF=1, no OF, SF=0)
    lea si, [base16sid]
    inc word [si + (m16_sid_00ff - base16sid)]
    SAVE_FLAGS
    mov ax, [si + (m16_sid_00ff - base16sid)]
    ASSERT_AX 0x0100
    CHECK_AF 1
    CHECK_PF 1          ; low byte 00
    CHECK_OF 0
    CHECK_SF 0

; 23) [di+disp]:  7FFE -> 7FFF  (no OF; PF=1; AF=0)
    lea di, [base16did]
    inc word [di + (m16_did_7ffe - base16did)]
    SAVE_FLAGS
    mov ax, [di + (m16_did_7ffe - base16did)]
    ASSERT_AX 0x7FFF
    CHECK_OF 0
    CHECK_PF 1          ; low byte FF
    CHECK_AF 0

; 24) [bx+disp]:  8000 -> 8001  (SF=1; PF=0; AF=0; no OF)
    lea bx, [base16bd]
    inc word [bx + (m16_bxd_8000 - base16bd)]
    SAVE_FLAGS
    mov ax, [bx + (m16_bxd_8000 - base16bd)]
    ASSERT_AX 0x8001
    CHECK_SF 1
    CHECK_PF 0          ; low byte 01
    CHECK_AF 0
    CHECK_OF 0

; 25) [bx+si+disp]:  7FFF -> 8000  (OF=1, SF=1, PF=1, AF=1)
    lea bx, [base16]
    lea si, [index16]
    inc word [bx+si + (m16_bxsi_7fff - base16 - index16)]
    SAVE_FLAGS
    mov ax, [bx+si + (m16_bxsi_7fff - base16 - index16)]
    ASSERT_AX 0x8000
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1

; 26) [bp] (ds:):  00FF -> 0100  (AF=1, PF=1, no OF)
    mov bp, m16_bp_00ff
    inc word [ds:bp]
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x0100
    CHECK_AF 1
    CHECK_PF 1
    CHECK_OF 0

; 27) [bp+disp] (ds:):  1234 -> 1235 (no OF; PF=1; AF=0; SF=0; ZF=0)
    lea bp, [base_bp16_d]
    inc word [ds:bp + (m16_bpd_1234 - base_bp16_d)]
    SAVE_FLAGS
    mov ax, [ds:bp + (m16_bpd_1234 - base_bp16_d)]
    ASSERT_AX 0x1235
    CHECK_OF 0
    CHECK_PF 1          ; low byte 35 has even parity
    CHECK_AF 0
    CHECK_SF 0
    CHECK_ZF 0

; 28) [bp+si+disp] (ds:):  7FFE -> 7FFF  (no OF; PF=1; AF=0)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    inc word [ds:bp+si + (m16_bpsi_7ffe - base_bp16_si_A - base_bp16_si_B)]
    SAVE_FLAGS
    mov ax, [ds:bp+si + (m16_bpsi_7ffe - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x7FFF
    CHECK_OF 0
    CHECK_PF 1
    CHECK_AF 0

; 29) [bp+di+disp] (ds:):  7FFF -> 8000  (OF=1; SF=1; PF=1; AF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    inc word [ds:bp+di + (m16_bpdi_7fff - base_bp16_di_A - base_bp16_di_B)]
    SAVE_FLAGS
    mov ax, [ds:bp+di + (m16_bpdi_7fff - base_bp16_di_A - base_bp16_di_B)]
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
m8_si_ff:     db 0xFF
m8_di_7f:     db 0x7F
m8_bx_7e:     db 0x7E

base8sid:
m8_sid_fe:    db 0xFE

base8dd:
m8_did_0f:    db 0x0F

base8bd:
m8_bxd_00:    db 0x00

base8:
index8:
m8_bxsi_7f:   db 0x7F

m8_bp_ff:     db 0xFF

base_bp_d:
m8_bpd_55:    db 0x55

base_bp_si_A:
base_bp_si_B:
m8_bpsi_7e:   db 0x7E

base_bp_di_A:
base_bp_di_B:
m8_bpdi_7f:   db 0x7F

; 16-bit memory operands
m16_si_7fff:    dw 0x7FFF
m16_di_ffff:    dw 0xFFFF
m16_bx_0000:    dw 0x0000

base16sid:
m16_sid_00ff:   dw 0x00FF

base16did:
m16_did_7ffe:   dw 0x7FFE

base16bd:
m16_bxd_8000:   dw 0x8000

base16:
index16:
m16_bxsi_7fff:  dw 0x7FFF

m16_bp_00ff:    dw 0x00FF

base_bp16_d:
m16_bpd_1234:   dw 0x1234

base_bp16_si_A:
base_bp16_si_B:
m16_bpsi_7ffe:  dw 0x7FFE

base_bp16_di_A:
base_bp16_di_B:
m16_bpdi_7fff:  dw 0x7FFF

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
