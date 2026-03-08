; neg.asm — thorough NEG tests (r/m8, r/m16) with saved FLAGS and DOS exit
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   NEG computes 0 - op (in-place). Flags:
;     CF = (op ≠ 0)
;     OF = 1 iff op == 0x80 (byte) or 0x8000 (word)
;     ZF/SF/PF from result
;     AF = ((op ^ result) & 0x10) != 0  (i.e., low nibble of op ≠ 0)

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

; ===================== 8-bit NEG (register) =====================

; 1) AL=00 -> 00  (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0)
    mov al, 0x00
    neg al
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 2) AL=01 -> FF  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=1)
    mov al, 0x01
    neg al
    SAVE_FLAGS
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1

; 3) AL=80 -> 80  (CF=1,OF=1,ZF=0,SF=1,PF=0,AF=0)
    mov al, 0x80
    neg al
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 0

; 4) AH=10 -> F0  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=0)
    mov ah, 0x10
    neg ah
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0xF0
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 0

; 5) BL=7F -> 81  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=1)
    mov bl, 0x7F
    neg bl
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0x81
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1

; 6) Precondition CF=1/OF=1; NEG must overwrite with CF=0,OF=0 for op=0
    stc
    mov bh, 0x7F
    add bh, 0x01            ; OF=1
    mov al, 0x00
    neg al
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 7) CH=0F -> F1  (CF=1,OF=0,ZF=0,SF=1,PF=0,AF=1)
    mov ch, 0x0F
    neg ch
    SAVE_FLAGS
    mov al, ch
    ASSERT_BYTE 0xF1
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1


; ===================== 8-bit NEG (memory; all addressing forms) =====================

; 8) [si]=FF -> 01  (CF=1,OF=0,ZF=0,SF=0,PF=0,AF=1)
    lea si, [neg8_si_ff]
    neg byte [si]
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1

; 9) [di]=80 -> 80  (CF=1,OF=1,ZF=0,SF=1,PF=0,AF=0)
    lea di, [neg8_di_80]
    neg byte [di]
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x80
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 0

; 10) [bx]=00 -> 00  (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0)
    lea bx, [neg8_bx_00]
    neg byte [bx]
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 11) [si+disp]=01 -> FF  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=1)
    lea si, [base8sid]
    neg byte [si + (neg8_sid_01 - base8sid)]
    SAVE_FLAGS
    mov al, [si + (neg8_sid_01 - base8sid)]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1

; 12) [di+disp]=10 -> F0  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=0)
    lea di, [base8dd]
    neg byte [di + (neg8_did_10 - base8dd)]
    SAVE_FLAGS
    mov al, [di + (neg8_did_10 - base8dd)]
    ASSERT_BYTE 0xF0
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 0

; 13) [bx+disp]=7E -> 82  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=1)
    lea bx, [base8bd]
    neg byte [bx + (neg8_bxd_7e - base8bd)]
    SAVE_FLAGS
    mov al, [bx + (neg8_bxd_7e - base8bd)]
    ASSERT_BYTE 0x82
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1

; 14) [bx+si+disp]=7F -> 81  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=1)
    lea bx, [base8]
    lea si, [index8]
    neg byte [bx+si + (neg8_bxsi_7f - base8 - index8)]
    SAVE_FLAGS
    mov al, [bx+si + (neg8_bxsi_7f - base8 - index8)]
    ASSERT_BYTE 0x81
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1

; 15) [bp] (ds:)=FF -> 01  (CF=1,OF=0,ZF=0,SF=0,PF=0,AF=1)
    mov bp, neg8_bp_ff
    neg byte [ds:bp]
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1

; 16) [bp+disp] (ds:)=02 -> FE  (CF=1,OF=0,ZF=0,SF=1,PF=0,AF=1)
    lea bp, [base_bp_d]
    neg byte [ds:bp + (neg8_bpd_02 - base_bp_d)]
    SAVE_FLAGS
    mov al, [ds:bp + (neg8_bpd_02 - base_bp_d)]
    ASSERT_BYTE 0xFE
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1

; 17) [bp+si+disp] (ds:)=40 -> C0  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=0)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    neg byte [ds:bp+si + (neg8_bpsi_40 - base_bp_si_A - base_bp_si_B)]
    SAVE_FLAGS
    mov al, [ds:bp+si + (neg8_bpsi_40 - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0xC0
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 0

; 18) [bp+di+disp] (ds:)=7D -> 83  (CF=1,OF=0,ZF=0,SF=1,PF=0,AF=1)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    neg byte [ds:bp+di + (neg8_bpdi_7d - base_bp_di_A - base_bp_di_B)]
    SAVE_FLAGS
    mov al, [ds:bp+di + (neg8_bpdi_7d - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x83
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1


; ===================== 16-bit NEG (register) =====================

; 19) AX=0000 -> 0000  (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0)
    mov ax, 0x0000
    neg ax
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 20) AX=0001 -> FFFF  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=1)
    mov ax, 0x0001
    neg ax
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1

; 21) AX=8000 -> 8000  (CF=1,OF=1,ZF=0,SF=1,PF=1,AF=0)
    mov ax, 0x8000
    neg ax
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1          ; low byte 00 → even parity
    CHECK_AF 0

; 22) BX=00FF -> FF01  (CF=1,OF=0,ZF=0,SF=1,PF=0,AF=1)
    mov bx, 0x00FF
    neg bx
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0xFF01
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0          ; low byte 01 → odd parity
    CHECK_AF 1

; 23) DX=7FFF -> 8001  (CF=1,OF=0,ZF=0,SF=1,PF=0,AF=1)
    mov dx, 0x7FFF
    neg dx
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x8001
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0          ; low byte 01
    CHECK_AF 1

; 24) CX=0010 -> FFF0  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=0)
    mov cx, 0x0010
    neg cx
    SAVE_FLAGS
    mov ax, cx
    ASSERT_AX 0xFFF0
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1          ; low byte F0 has 4 ones
    CHECK_AF 0

; 25) Precondition CF=1/OF=1; NEG must overwrite with CF=0,OF=0 for op=0 (word)
    stc
    mov si, 0x7FFF
    add si, 0x0001        ; OF=1
    mov ax, 0x0000
    neg ax
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0


; ===================== 16-bit NEG (memory; all addressing forms) =====================

; 26) [si]=FFFF -> 0001  (CF=1,OF=0,ZF=0,SF=0,PF=0,AF=1)
    lea si, [neg16_si_ffff]
    neg word [si]
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x0001
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0          ; low byte 01
    CHECK_AF 1

; 27) [di]=8000 -> 8000  (CF=1,OF=1,ZF=0,SF=1,PF=1,AF=0)
    lea di, [neg16_di_8000]
    neg word [di]
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x8000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1          ; low byte 00
    CHECK_AF 0

; 28) [bx]=0000 -> 0000  (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0)
    lea bx, [neg16_bx_0000]
    neg word [bx]
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 29) [si+disp]=0001 -> FFFF  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=1)
    lea si, [base16sid]
    neg word [si + (neg16_sid_0001 - base16sid)]
    SAVE_FLAGS
    mov ax, [si + (neg16_sid_0001 - base16sid)]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1

; 30) [di+disp]=0100 -> FF00  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=0)
    lea di, [base16did]
    neg word [di + (neg16_did_0100 - base16did)]
    SAVE_FLAGS
    mov ax, [di + (neg16_did_0100 - base16did)]
    ASSERT_AX 0xFF00
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1          ; low byte 00
    CHECK_AF 0

; 31) [bx+disp]=7FFE -> 8002  (CF=1,OF=0,ZF=0,SF=1,PF=0,AF=1)
    lea bx, [base16bd]
    neg word [bx + (neg16_bxd_7ffe - base16bd)]
    SAVE_FLAGS
    mov ax, [bx + (neg16_bxd_7ffe - base16bd)]
    ASSERT_AX 0x8002
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0          ; low byte 02
    CHECK_AF 1

; 32) [bx+si+disp]=7FFF -> 8001  (CF=1,OF=0,ZF=0,SF=1,PF=0,AF=1)
    lea bx, [base16]
    lea si, [index16]
    neg word [bx+si + (neg16_bxsi_7fff - base16 - index16)]
    SAVE_FLAGS
    mov ax, [bx+si + (neg16_bxsi_7fff - base16 - index16)]
    ASSERT_AX 0x8001
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0          ; low byte 01
    CHECK_AF 1

; 33) [bp] (ds:)=00FF -> FF01  (CF=1,OF=0,ZF=0,SF=1,PF=0,AF=1)
    mov bp, neg16_bp_00ff
    neg word [ds:bp]
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0xFF01
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1

; 34) [bp+disp] (ds:)=1234 -> EDCC  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=1)
    lea bp, [base_bp16_d]
    neg word [ds:bp + (neg16_bpd_1234 - base_bp16_d)]
    SAVE_FLAGS
    mov ax, [ds:bp + (neg16_bpd_1234 - base_bp16_d)]
    ASSERT_AX 0xEDCC
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1          ; low byte CC has 4 ones
    CHECK_AF 1

; 35) [bp+si+disp] (ds:)=4000 -> C000  (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=0)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    neg word [ds:bp+si + (neg16_bpsi_4000 - base_bp16_si_A - base_bp16_si_B)]
    SAVE_FLAGS
    mov ax, [ds:bp+si + (neg16_bpsi_4000 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0xC000
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1          ; low byte 00
    CHECK_AF 0

; 36) [bp+di+disp] (ds:)=0002 -> FFFE  (CF=1,OF=0,ZF=0,SF=1,PF=0,AF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    neg word [ds:bp+di + (neg16_bpdi_0002 - base_bp16_di_A - base_bp16_di_B)]
    SAVE_FLAGS
    mov ax, [ds:bp+di + (neg16_bpdi_0002 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0xFFFE
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; 8-bit memory operands
neg8_si_ff:     db 0xFF
neg8_di_80:     db 0x80
neg8_bx_00:     db 0x00

base8sid:
neg8_sid_01:    db 0x01

base8dd:
neg8_did_10:    db 0x10

base8bd:
neg8_bxd_7e:    db 0x7E

base8:
index8:
neg8_bxsi_7f:   db 0x7F

neg8_bp_ff:     db 0xFF

base_bp_d:
neg8_bpd_02:    db 0x02

base_bp_si_A:
base_bp_si_B:
neg8_bpsi_40:   db 0x40

base_bp_di_A:
base_bp_di_B:
neg8_bpdi_7d:   db 0x7D

; 16-bit memory operands
neg16_si_ffff:    dw 0xFFFF
neg16_di_8000:    dw 0x8000
neg16_bx_0000:    dw 0x0000

base16sid:
neg16_sid_0001:   dw 0x0001

base16did:
neg16_did_0100:   dw 0x0100

base16bd:
neg16_bxd_7ffe:   dw 0x7FFE

base16:
index16:
neg16_bxsi_7fff:  dw 0x7FFF

neg16_bp_00ff:    dw 0x00FF

base_bp16_d:
neg16_bpd_1234:   dw 0x1234

base_bp16_si_A:
base_bp16_si_B:
neg16_bpsi_4000:  dw 0x4000

base_bp16_di_A:
base_bp16_di_B:
neg16_bpdi_0002:  dw 0x0002

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
