; sal.asm — thorough SAL/SHL tests (r/m8, r/m16) for count=1 and count=CL (including CL=0 no-op)
; Harness:
;   int 0x23: assert AX == BX        (we use ASSERT_AX / ASSERT_BYTE)
;   int 0x22: assert AL == AH        (single-flag checks)
; Notes:
;   For count=1: check CF/OF/ZF/SF/PF (AF undefined -> not checked)
;   For count>=2: check CF/ZF/SF/PF; do NOT check OF (undefined)
;   For CL=0: verify operand and ALL flags unchanged (including OF)

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

; ===================== 8-bit SAL (register, count=1: opcodes D0 /4) =====================

; 1) AL=00 <<1 -> 00  (CF=0,OF=0,ZF=1,SF=0,PF=1)
    mov al, 0x00
    sal al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 2) AL=01 <<1 -> 02  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    mov al, 0x01
    sal al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x02
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 3) AL=80 <<1 -> 00  (CF=1,OF=1,ZF=1,SF=0,PF=1)
    mov al, 0x80
    sal al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 4) AL=40 <<1 -> 80  (CF=0,OF=1,ZF=0,SF=1,PF=0)
    mov al, 0x40
    sal al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 5) AH=FF <<1 -> FE  (CF=1,OF=0,ZF=0,SF=1,PF=0)
    mov ah, 0xFF
    sal ah, 1
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0xFE
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 6) BL=7F <<1 -> FE  (CF=0,OF=1,ZF=0,SF=1,PF=0)
    mov bl, 0x7F
    sal bl, 1
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0xFE
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 7) Overwrite prior flags: preset CF=1/OF=1, then AL=00 <<1 -> 00 (CF=0,OF=0)
    stc
    mov bh, 0x7F
    add bh, 0x01              ; OF=1
    mov al, 0x00
    sal al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== 8-bit SAL (memory, count=1) =====================

; 8) [si]=7E -> FC  (CF=0,OF=1,ZF=0,SF=1,PF=1)
    lea si, [sal8_si_7e]
    sal byte [si], 1
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xFC
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 9) [di]=80 -> 00  (CF=1,OF=1,ZF=1,SF=0,PF=1)
    lea di, [sal8_di_80]
    sal byte [di], 1
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 10) [bx]=01 -> 02  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    lea bx, [sal8_bx_01]
    sal byte [bx], 1
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x02
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 11) [si+disp]=FF -> FE  (CF=1,OF=0,ZF=0,SF=1,PF=0)
    lea si, [base8sid]
    sal byte [si + (sal8_sid_ff - base8sid)], 1
    SAVE_FLAGS
    mov al, [si + (sal8_sid_ff - base8sid)]
    ASSERT_BYTE 0xFE
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 12) [di+disp]=00 -> 00  (CF=0,OF=0,ZF=1,SF=0,PF=1)
    lea di, [base8dd]
    sal byte [di + (sal8_did_00 - base8dd)], 1
    SAVE_FLAGS
    mov al, [di + (sal8_did_00 - base8dd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 13) [bx+disp]=3F -> 7E  (CF=0,OF=0,ZF=0,SF=0,PF=1)
    lea bx, [base8bd]
    sal byte [bx + (sal8_bxd_3f - base8bd)], 1
    SAVE_FLAGS
    mov al, [bx + (sal8_bxd_3f - base8bd)]
    ASSERT_BYTE 0x7E
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 14) [bx+si+disp]=40 -> 80  (CF=0,OF=1,ZF=0,SF=1,PF=0)
    lea bx, [base8]
    lea si, [index8]
    sal byte [bx+si + (sal8_bxsi_40 - base8 - index8)], 1
    SAVE_FLAGS
    mov al, [bx+si + (sal8_bxsi_40 - base8 - index8)]
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 15) [bp] (ds:)=FF -> FE  (CF=1,OF=0,ZF=0,SF=1,PF=0)
    mov bp, sal8_bp_ff
    sal byte [ds:bp], 1
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFE
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 16) [bp+disp] (ds:)=20 -> 40  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    lea bp, [base_bp_d]
    sal byte [ds:bp + (sal8_bpd_20 - base_bp_d)], 1
    SAVE_FLAGS
    mov al, [ds:bp + (sal8_bpd_20 - base_bp_d)]
    ASSERT_BYTE 0x40
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 17) [bp+si+disp] (ds:)=02 -> 04  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    sal byte [ds:bp+si + (sal8_bpsi_02 - base_bp_si_A - base_bp_si_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+si + (sal8_bpsi_02 - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x04
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 18) [bp+di+disp] (ds:)=C0 -> 80  (CF=1,OF=0,ZF=0,SF=1,PF=0)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    sal byte [ds:bp+di + (sal8_bpdi_c0 - base_bp_di_A - base_bp_di_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+di + (sal8_bpdi_c0 - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x80
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0


; ===================== 8-bit SAL (register/memory, count=CL: opcode D2) =====================

; 19) CL=2; AL=81 -> 04  (CF=orig bit6=0, ZF=0,SF=0,PF=0)
    mov cl, 2
    mov al, 0x81
    sal al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x04
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0          ; OF undefined for counts>1 -> not checked

; 20) CL=2; AH=C1 -> 04  (CF=orig bit6=1, ZF=0,SF=0,PF=0)
    mov cl, 2
    mov ah, 0xC1
    sal ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x04
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 21) CL=3; BL=03 -> 18  (CF=orig bit5=0, ZF=0,SF=0,PF=1)
    mov cl, 3
    mov bl, 0x03
    sal bl, cl
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0x18
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 22) CL=7; DL=03 -> 80  (CF=orig bit1=1, ZF=0,SF=1,PF=0)
    mov cl, 7
    mov dl, 0x03
    sal dl, cl
    SAVE_FLAGS
    mov al, dl
    ASSERT_BYTE 0x80
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 23) CL=2; [di+disp]=81 -> 04  (CF=orig bit6=0, ZF=0,SF=0,PF=0)
    mov cl, 2
    lea di, [base8dd2]
    sal byte [di + (sal8_did_81 - base8dd2)], cl
    SAVE_FLAGS
    mov al, [di + (sal8_did_81 - base8dd2)]
    ASSERT_BYTE 0x04
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 24) CL=3; [bx]=03 -> 18  (CF=orig bit5=0, ZF=0,SF=0,PF=1)
    mov cl, 3
    lea bx, [sal8_bx_03]
    sal byte [bx], cl
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x18
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 25) CL=0 no-op (register): flags preserved (using SAHF pattern=all 1s), AL unchanged
    mov ah, [pat1]
    sahf
    mov al, 0x12
    mov cl, 0
    sal al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x12
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    ; OF preserved too; verified separately below

; 26) CL=0 OF-preservation: make OF=1 via ADD; SAL must keep OF=1, AH unchanged
    mov bh, 0x7F
    add bh, 0x01            ; OF=1
    mov ah, 0x55
    mov cl, 0
    sal ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x55
    CHECK_OF 1

; 27) CL=0 no-op (memory): preset flags to 0 via SAHF; memory unchanged; flags preserved
    mov ah, [pat0]
    sahf
    lea si, [sal8_si_55]
    mov cl, 0
    sal byte [si], cl
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x55
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0


; ===================== 16-bit SAL (register, count=1: opcode D1) =====================

; 28) AX=0000 -> 0000  (CF=0,OF=0,ZF=1,SF=0,PF=1)
    mov ax, 0x0000
    sal ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 29) AX=8000 -> 0000  (CF=1,OF=1,ZF=1,SF=0,PF=1)
    mov ax, 0x8000
    sal ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 30) AX=4000 -> 8000  (CF=0,OF=1,ZF=0,SF=1,PF=1)
    mov ax, 0x4000
    sal ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 31) BX=FFFF -> FFFE  (CF=1,OF=0,ZF=0,SF=1,PF=0)
    mov bx, 0xFFFF
    sal bx, 1
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0xFFFE
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 32) DX=7FFF -> FFFE  (CF=0,OF=1,ZF=0,SF=1,PF=0)
    mov dx, 0x7FFF
    sal dx, 1
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0xFFFE
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0


; ===================== 16-bit SAL (memory, count=1) =====================

; 33) [si]=7FFF -> FFFE  (CF=0,OF=1,ZF=0,SF=1,PF=0)
    lea si, [sal16_si_7fff]
    sal word [si], 1
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0xFFFE
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 34) [di]=8000 -> 0000  (CF=1,OF=1,ZF=1,SF=0,PF=1)
    lea di, [sal16_di_8000]
    sal word [di], 1
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 35) [bx]=0001 -> 0002  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    lea bx, [sal16_bx_0001]
    sal word [bx], 1
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0002
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 36) [si+disp]=FFFF -> FFFE  (CF=1,OF=0,ZF=0,SF=1,PF=0)
    lea si, [base16sid]
    sal word [si + (sal16_sid_ffff - base16sid)], 1
    SAVE_FLAGS
    mov ax, [si + (sal16_sid_ffff - base16sid)]
    ASSERT_AX 0xFFFE
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 37) [di+disp]=0100 -> 0200  (CF=0,OF=0,ZF=0,SF=0,PF=1)
    lea di, [base16did]
    sal word [di + (sal16_did_0100 - base16did)], 1
    SAVE_FLAGS
    mov ax, [di + (sal16_did_0100 - base16did)]
    ASSERT_AX 0x0200
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 38) [bx+disp]=3FFF -> 7FFE  (CF=0,OF=1,ZF=0,SF=1,PF=0)
    lea bx, [base16bd]
    sal word [bx + (sal16_bxd_3fff - base16bd)], 1
    SAVE_FLAGS
    mov ax, [bx + (sal16_bxd_3fff - base16bd)]
    ASSERT_AX 0x7FFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 39) [bx+si+disp]=4000 -> 8000  (CF=0,OF=1,ZF=0,SF=1,PF=1)
    lea bx, [base16]
    lea si, [index16]
    sal word [bx+si + (sal16_bxsi_4000 - base16 - index16)], 1
    SAVE_FLAGS
    mov ax, [bx+si + (sal16_bxsi_4000 - base16 - index16)]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 40) [bp] (ds:)=FFFF -> FFFE  (CF=1,OF=0,ZF=0,SF=1,PF=0)
    mov bp, sal16_bp_ffff
    sal word [ds:bp], 1
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0xFFFE
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 41) [bp+disp] (ds:)=0002 -> 0004  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    lea bp, [base_bp16_d]
    sal word [ds:bp + (sal16_bpd_0002 - base_bp16_d)], 1
    SAVE_FLAGS
    mov ax, [ds:bp + (sal16_bpd_0002 - base_bp16_d)]
    ASSERT_AX 0x0004
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 42) [bp+si+disp] (ds:)=2000 -> 4000  (CF=0,OF=0,ZF=0,SF=0,PF=1)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    sal word [ds:bp+si + (sal16_bpsi_2000 - base_bp16_si_A - base_bp16_si_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+si + (sal16_bpsi_2000 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x4000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 43) [bp+di+disp] (ds:)=C000 -> 8000  (CF=1,OF=0,ZF=0,SF=1,PF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    sal word [ds:bp+di + (sal16_bpdi_c000 - base_bp16_di_A - base_bp16_di_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+di + (sal16_bpdi_c000 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x8000
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ===================== 16-bit SAL (register/memory, count=CL: opcode D3) =====================

; 44) CL=2; AX=8001 -> 0004  (CF=orig bit14=0, ZF=0,SF=0,PF=0)
    mov cl, 2
    mov ax, 0x8001
    sal ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x0004
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 45) CL=3; BX=1000 -> 8000  (CF=orig bit13=0, ZF=0,SF=1,PF=1)
    mov cl, 3
    mov bx, 0x1000
    sal bx, cl
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 46) CL=4; DX=0008 -> 0080  (CF=orig bit12=0, ZF=0,SF=0,PF=0)
    mov cl, 4
    mov dx, 0x0008
    sal dx, cl
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x0080
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 47) CL=7; SI=0003 -> 0180  (CF=orig bit9=0, ZF=0,SF=0,PF=0)
    mov cl, 7
    mov si, 0x0003
    sal si, cl
    SAVE_FLAGS
    mov ax, si
    ASSERT_AX 0x0180
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 48) CL=2; [di+disp]=C000 -> 0000  (CF=orig bit14=1, ZF=1,SF=0,PF=1)
    mov cl, 2
    lea di, [base16did2]
    sal word [di + (sal16_did_c000 - base16did2)], cl
    SAVE_FLAGS
    mov ax, [di + (sal16_did_c000 - base16did2)]
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 49) CL=3; [bx]=0003 -> 0018  (CF=orig bit13=0, ZF=0,SF=0,PF=1)
    mov cl, 3
    lea bx, [sal16_bx_0003]
    sal word [bx], cl
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0018
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 50) CL=0 no-op (register): SAHF=pat1; AX unchanged; flags preserved
    mov ah, [pat1]
    sahf
    mov ax, 0x1234
    mov cl, 0
    sal ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x1234
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 51) CL=0 OF-preservation (memory): make OF=1 via ADD; SAL [si],0 preserves OF and value
    mov si, 0x7FFF
    add si, 0x0001          ; OF=1
    lea bx, [sal16_bx_aaaa]
    mov cl, 0
    sal word [bx], cl
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0xAAAA
    CHECK_OF 1


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; ---- 8-bit memory (count=1) ----
sal8_si_7e:     db 0x7E
sal8_di_80:     db 0x80
sal8_bx_01:     db 0x01

base8sid:
sal8_sid_ff:    db 0xFF

base8dd:
sal8_did_00:    db 0x00

base8bd:
sal8_bxd_3f:    db 0x3F

base8:
index8:
sal8_bxsi_40:   db 0x40

sal8_bp_ff:     db 0xFF

base_bp_d:
sal8_bpd_20:    db 0x20

base_bp_si_A:
base_bp_si_B:
sal8_bpsi_02:   db 0x02

base_bp_di_A:
base_bp_di_B:
sal8_bpdi_c0:   db 0xC0

; ---- 8-bit memory (count=CL) ----
base8dd2:
sal8_did_81:    db 0x81

sal8_bx_03:     db 0x03

sal8_si_55:     db 0x55

; ---- 16-bit memory (count=1) ----
sal16_si_7fff:    dw 0x7FFF
sal16_di_8000:    dw 0x8000
sal16_bx_0001:    dw 0x0001

base16sid:
sal16_sid_ffff:   dw 0xFFFF

base16did:
sal16_did_0100:   dw 0x0100

base16bd:
sal16_bxd_3fff:   dw 0x3FFF

base16:
index16:
sal16_bxsi_4000:  dw 0x4000

sal16_bp_ffff:    dw 0xFFFF

base_bp16_d:
sal16_bpd_0002:   dw 0x0002

base_bp16_si_A:
base_bp16_si_B:
sal16_bpsi_2000:  dw 0x2000

base_bp16_di_A:
base_bp16_di_B:
sal16_bpdi_c000:  dw 0xC000

; ---- 16-bit memory (count=CL) ----
base16did2:
sal16_did_c000:   dw 0xC000

sal16_bx_0003:    dw 0x0003

sal16_bx_aaaa:    dw 0xAAAA

; Helpers: SAHF patterns for CL=0 "flags preserved" tests
pat0: db 0x00            ; SF=0 ZF=0 AF=0 PF=0 CF=0
pat1: db 0xD5            ; SF=1 ZF=1 AF=1 PF=1 CF=1

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
