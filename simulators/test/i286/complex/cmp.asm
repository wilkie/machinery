; cmp.asm — thorough CMP tests (r/m8,r8 | r8,r/m8 | r/m16,r16 | r16,r/m16 |
;                                      AL,imm8 | AX,imm16 | r/m8,imm8 | r/m16,imm16 | r/m16,imm8[sign])
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   CMP sets flags like SUB (dest - src) and does not modify operands.

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

; ===================== 8-bit CMP (register, immediate) =====================

; 1) AL=00, CMP AL,00 -> res 00 (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0), AL unchanged
    mov al, 0x00
    cmp al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 2) AL=7F, CMP AL,01 -> 7E (CF=0,OF=0,ZF=0,SF=0,PF=1,AF=0)
    mov al, 0x7F
    cmp al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 3) AL=80, CMP AL,01 -> 7F (CF=0,OF=1,ZF=0,SF=0,PF=0,AF=1)
    mov al, 0x80
    cmp al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1

; 4) AL=00, CMP AL,FF -> 01 (CF=1,OF=0,ZF=0,SF=0,PF=0,AF=1)
    mov al, 0x00
    cmp al, 0xFF
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1

; 5) AH=10, CMP AH,01 -> 0F (CF=0,OF=0,ZF=0,SF=0,PF=1,AF=1)
    mov ah, 0x10
    cmp ah, 0x01
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x10
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 1

; 6) AL=80, CMP AL,80 -> 00 (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0)
    mov al, 0x80
    cmp al, 0x80
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 7) AL=01, CMP AL,02 -> FF (CF=1,OF=0,ZF=0,SF=1,PF=1,AF=1)
    mov al, 0x01
    cmp al, 0x02
    SAVE_FLAGS
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1

; 8) AL=80, CMP AL,7F -> 01 (CF=0,OF=1,ZF=0,SF=0,PF=0,AF=1)
    mov al, 0x80
    cmp al, 0x7F
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1


; ===================== 8-bit CMP (memory, immediate; all addressing forms) =====================

; 9) [si]=FF, CMP [si],01 -> FE (CF=0,OF=0,SF=1,PF=0,AF=0), mem unchanged
    lea si, [cmp8_si_ff]
    cmp byte [si], 0x01
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0

; 10) [di]=80, CMP [di],80 -> 00 (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0)
    lea di, [cmp8_di_80]
    cmp byte [di], 0x80
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 11) [bx]=7E, CMP [bx],01 -> 7D (CF=0,OF=0,SF=0,PF=1,AF=0)
    lea bx, [cmp8_bx_7e]
    cmp byte [bx], 0x01
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x7E
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_ZF 0

; 12) [si+disp]=00, CMP with 00 -> 00 (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0)
    lea si, [base8sid]
    cmp byte [si + (cmp8_sid_00 - base8sid)], 0x00
    SAVE_FLAGS
    mov al, [si + (cmp8_sid_00 - base8sid)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 13) [di+disp]=0F, CMP with 01 -> 0E (CF=0,OF=0,ZF=0,SF=0,PF=0,AF=0)
    lea di, [base8dd]
    cmp byte [di + (cmp8_did_0f - base8dd)], 0x01
    SAVE_FLAGS
    mov al, [di + (cmp8_did_0f - base8dd)]
    ASSERT_BYTE 0x0F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 0

; 14) [bx+disp]=01, CMP with 02 -> FF (CF=1,OF=0,SF=1,PF=1,AF=1)
    lea bx, [base8bd]
    cmp byte [bx + (cmp8_bxd_01 - base8bd)], 0x02
    SAVE_FLAGS
    mov al, [bx + (cmp8_bxd_01 - base8bd)]
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0

; 15) [bx+si+disp]=80, CMP with 7F -> 01 (CF=0,OF=1,SF=0,PF=0,AF=1)
    lea bx, [base8]
    lea si, [index8]
    cmp byte [bx+si + (cmp8_bxsi_80 - base8 - index8)], 0x7F
    SAVE_FLAGS
    mov al, [bx+si + (cmp8_bxsi_80 - base8 - index8)]
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1
    CHECK_ZF 0

; 16) [bp] (ds:)=FF, CMP with FF -> 00 (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0)
    mov bp, cmp8_bp_ff
    cmp byte [ds:bp], 0xFF
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0


; ===================== 16-bit CMP (register, immediate) =====================

; 17) AX=0000, CMP AX,0000 -> 0000 (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0)
    mov ax, 0x0000
    cmp ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 18) AX=8000, CMP AX,0001 -> 7FFF (CF=0,OF=1,ZF=0,SF=0,PF=1,AF=1)
    mov ax, 0x8000
    cmp ax, 0x0001
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 1

; 19) AX=0000, CMP AX,FFFF -> 0001 (CF=1,OF=0,ZF=0,SF=0,PF=0,AF=1)
    mov ax, 0x0000
    cmp ax, 0xFFFF
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1

; 20) AX=7FFF, CMP AX,FFFF -> 8000 (CF=1,OF=1,ZF=0,SF=1,PF=1,AF=0)
    mov ax, 0x7FFF
    cmp ax, 0xFFFF
    SAVE_FLAGS
    ASSERT_AX 0x7FFF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 0


; ===================== 16-bit CMP (memory, immediate; all addressing forms) =====================

; 21) [si]=7FFF, CMP with 0001 -> 7FFE (CF=0,OF=0,SF=0,PF=0,AF=0)
    lea si, [cmp16_si_7fff]
    cmp word [si], 0x0001
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0

; 22) [di]=8000, CMP with 0001 -> 7FFF (CF=0,OF=1,SF=0,PF=1,AF=1)
    lea di, [cmp16_di_8000]
    cmp word [di], 0x0001
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0

; 23) [bx]=0000, CMP with 0001 -> FFFF (CF=1,OF=0,SF=1,PF=1,AF=1)
    lea bx, [cmp16_bx_0000]
    cmp word [bx], 0x0001
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0

; 24) [si+disp]=0100, CMP with 0000 -> 0100 (CF=0,OF=0,SF=0,PF=1,AF=0)
    lea si, [base16sid]
    cmp word [si + (cmp16_sid_0100 - base16sid)], 0x0000
    SAVE_FLAGS
    mov ax, [si + (cmp16_sid_0100 - base16sid)]
    ASSERT_AX 0x0100
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_ZF 0

; 25) [di+disp]=7FFF, CMP with 0001 -> 7FFE (CF=0,OF=0,SF=0,PF=0,AF=0)
    lea di, [base16did]
    cmp word [di + (cmp16_did_7fff - base16did)], 0x0001
    SAVE_FLAGS
    mov ax, [di + (cmp16_did_7fff - base16did)]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0

; 26) [bx+disp]=8000, CMP with 0000 -> 8000 (CF=0,OF=0,SF=1,PF=1,AF=0)
    lea bx, [base16bd]
    cmp word [bx + (cmp16_bxd_8000 - base16bd)], 0x0000
    SAVE_FLAGS
    mov ax, [bx + (cmp16_bxd_8000 - base16bd)]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 0
    CHECK_ZF 0

; 27) [bx+si+disp]=8000, CMP with 8000 -> 0000 (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0)
    lea bx, [base16]
    lea si, [index16]
    cmp word [bx+si + (cmp16_bxsi_8000 - base16 - index16)], 0x8000
    SAVE_FLAGS
    mov ax, [bx+si + (cmp16_bxsi_8000 - base16 - index16)]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 28) [bp] (ds:)=00FF, CMP with 0101 -> FFFE (CF=1,OF=0,SF=1,PF=0,AF=0)
    mov bp, cmp16_bp_00ff
    cmp word [ds:bp], 0x0101
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x00FF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0

; 29) [bp+disp] (ds:)=0001, CMP (imm8 sign-ext) byte +2 -> FFFF (CF=1,OF=0,SF=1,PF=1,AF=1)
    lea bp, [base_bp16_d]
    cmp word [ds:bp + (cmp16_bpd_0001 - base_bp16_d)], byte 2
    SAVE_FLAGS
    mov ax, [ds:bp + (cmp16_bpd_0001 - base_bp16_d)]
    ASSERT_AX 0x0001
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0

; 30) [bp+si+disp] (ds:)=8000, CMP (imm8 sign-ext) byte -1 -> 8001 (CF=1,OF=0,SF=1,PF=0,AF=1)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    cmp word [ds:bp+si + (cmp16_bpsi_8000 - base_bp16_si_A - base_bp16_si_B)], byte -1
    SAVE_FLAGS
    mov ax, [ds:bp+si + (cmp16_bpsi_8000 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x8000
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1
    CHECK_ZF 0


; ===================== CMP with r/m and r (covering 38/39/3A/3B) =====================

; 31) r8, r/m8 (opcode 3Ah): AL=7F, [si]=80 -> 7F-80=FF (CF=1,OF=1,SF=1,PF=1,AF=0), no changes
    mov al, 0x7F
    lea si, [cmp8_rm_al_si]
    cmp al, [si]
    SAVE_FLAGS
    ASSERT_BYTE 0x7F            ; AL unchanged
    mov al, [si]
    ASSERT_BYTE 0x80            ; mem unchanged
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 0
    CHECK_ZF 0

; 32) r/m8, r8 (opcode 38h): [bx]=01, BL=02 -> 01-02=FF (CF=1,OF=0,SF=1,PF=1,AF=1), no changes
    lea bx, [cmp8_rm_m_bx]
    cmp byte [bx], bl
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0x02            ; BL unchanged
    lea bx, [cmp8_rm_m_bx]
    mov al, [bx]
    ASSERT_BYTE 0x01            ; mem unchanged
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0

; 33) r16, r/m16 (opcode 3Bh): AX=8000, [bx+si+disp]=8000 -> 0000 (CF=0,OF=0,ZF=1,SF=0,PF=1,AF=0)
    lea bx, [base16]
    lea si, [index16]
    mov ax, 0x8000
    cmp ax, [bx+si + (cmp16_rm_ax_bxsi - base16 - index16)]
    SAVE_FLAGS
    ASSERT_AX 0x8000            ; AX unchanged
    lea bx, [base16]
    mov ax, [bx+si + (cmp16_rm_ax_bxsi - base16 - index16)]
    ASSERT_AX 0x8000            ; mem unchanged
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0

; 34) r/m16, r16 (opcode 39h): [bp+di+disp]=0000, CX=0001 -> FFFF (CF=1,OF=0,SF=1,PF=1,AF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov cx, 0x0001
    cmp word [ds:bp+di + (cmp16_rm_m_bpdi - base_bp16_di_A - base_bp16_di_B)], cx
    SAVE_FLAGS
    mov ax, [ds:bp+di + (cmp16_rm_m_bpdi - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x0000            ; mem unchanged
    mov ax, cx
    ASSERT_AX 0x0001            ; CX unchanged
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0

; 35) reg–reg (8-bit, opcode 38h with r/m=reg): BL=80, BH=7F -> 01 (CF=0,OF=1,SF=0,PF=0,AF=1)
    mov bl, 0x80
    mov bh, 0x7F
    cmp bl, bh
    SAVE_FLAGS
    push bx
    mov al, bl
    ASSERT_BYTE 0x80
    pop bx
    mov al, bh
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1
    CHECK_ZF 0

; 36) reg–reg (16-bit, opcode 39h with r/m=reg): BX=0000, CX=0001 -> FFFF (CF=1,OF=0,SF=1,PF=1,AF=1)
    mov bx, 0x0000
    mov cx, 0x0001
    cmp bx, cx
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x0000
    mov ax, cx
    ASSERT_AX 0x0001
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; 8-bit memory operands (immediate tests)
cmp8_si_ff:     db 0xFF
cmp8_di_80:     db 0x80
cmp8_bx_7e:     db 0x7E

base8sid:
cmp8_sid_00:    db 0x00

base8dd:
cmp8_did_0f:    db 0x0F

base8bd:
cmp8_bxd_01:    db 0x01

base8:
index8:
cmp8_bxsi_80:   db 0x80

cmp8_bp_ff:     db 0xFF

; 16-bit memory operands (immediate tests)
cmp16_si_7fff:    dw 0x7FFF
cmp16_di_8000:    dw 0x8000
cmp16_bx_0000:    dw 0x0000

base16sid:
cmp16_sid_0100:   dw 0x0100

base16did:
cmp16_did_7fff:   dw 0x7FFF

base16bd:
cmp16_bxd_8000:   dw 0x8000

base16:
index16:
cmp16_bxsi_8000:  dw 0x8000

cmp16_bp_00ff:    dw 0x00FF

base_bp16_d:
cmp16_bpd_0001:   dw 0x0001

base_bp16_si_A:
base_bp16_si_B:
cmp16_bpsi_8000:  dw 0x8000

; r/m with r coverage
cmp8_rm_al_si:      db 0x80        ; used by Test 31

; ensure this address' lower byte is exactly 2
align 256
db 0x00
db 0x00
cmp8_rm_m_bx:       db 0x01        ; used by Test 32

cmp16_rm_ax_bxsi:   dw 0x8000      ; used by Test 33

base_bp16_di_A:
base_bp16_di_B:
cmp16_rm_m_bpdi:    dw 0x0000      ; used by Test 34

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
