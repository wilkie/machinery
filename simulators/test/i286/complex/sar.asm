; sar.asm — thorough SAR tests (r/m8, r/m16) for count=1 and count=CL (including CL=0 no-op)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   For SAR count=1: CF = old LSB; OF = 0; ZF/SF/PF from result; AF undefined (not checked)
;   For SAR count>=2: CF = last bit shifted out; OF undefined (not checked)
;   For CL=0: operand and all flags preserved (we verify with SAHF + separate OF preservation)

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

; ===================== 8-bit SAR (register, count=1: opcode D0 /7) =====================

; 1) AL=00 >>1 -> 00  (CF=0,OF=0,ZF=1,SF=0,PF=1)
    mov al, 0x00
    sar al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 2) AL=01 >>1 -> 00  (CF=1,OF=0,ZF=1,SF=0,PF=1)
    mov al, 0x01
    sar al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 3) AL=80 >>1 -> C0  (CF=0,OF=0,ZF=0,SF=1,PF=1)
    mov al, 0x80
    sar al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0xC0
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 4) AL=FF >>1 -> FF  (CF=1,OF=0,ZF=0,SF=1,PF=1)
    mov al, 0xFF
    sar al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 5) AH=02 >>1 -> 01  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    mov ah, 0x02
    sar ah, 1
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 6) BL=7F >>1 -> 3F  (CF=1,OF=0,ZF=0,SF=0,PF=1)
    mov bl, 0x7F
    sar bl, 1
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0x3F
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 7) Overwrite prior flags: preset CF=1/OF=1, then AL=00 >>1 -> 00 (CF=0,OF=0)
    stc
    mov bh, 0x7F
    add bh, 0x01              ; OF=1
    mov al, 0x00
    sar al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== 8-bit SAR (memory, count=1) =====================

; 8) [si]=80 -> C0  (CF=0,OF=0,ZF=0,SF=1,PF=1)
    lea si, [sar8_si_80]
    sar byte [si], 1
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xC0
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 9) [di]=01 -> 00  (CF=1,OF=0,ZF=1,SF=0,PF=1)
    lea di, [sar8_di_01]
    sar byte [di], 1
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 10) [bx]=FF -> FF  (CF=1,OF=0,ZF=0,SF=1,PF=1)
    lea bx, [sar8_bx_ff]
    sar byte [bx], 1
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 11) [si+disp]=F0 -> F8  (CF=0,OF=0,ZF=0,SF=1,PF=0)
    lea si, [base8sid]
    sar byte [si + (sar8_sid_f0 - base8sid)], 1
    SAVE_FLAGS
    mov al, [si + (sar8_sid_f0 - base8sid)]
    ASSERT_BYTE 0xF8
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 12) [di+disp]=00 -> 00  (CF=0,OF=0,ZF=1,SF=0,PF=1)
    lea di, [base8dd]
    sar byte [di + (sar8_did_00 - base8dd)], 1
    SAVE_FLAGS
    mov al, [di + (sar8_did_00 - base8dd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 13) [bx+disp]=02 -> 01  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    lea bx, [base8bd]
    sar byte [bx + (sar8_bxd_02 - base8bd)], 1
    SAVE_FLAGS
    mov al, [bx + (sar8_bxd_02 - base8bd)]
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 14) [bx+si+disp]=7F -> 3F  (CF=1,OF=0,ZF=0,SF=0,PF=1)
    lea bx, [base8]
    lea si, [index8]
    sar byte [bx+si + (sar8_bxsi_7f - base8 - index8)], 1
    SAVE_FLAGS
    mov al, [bx+si + (sar8_bxsi_7f - base8 - index8)]
    ASSERT_BYTE 0x3F
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 15) [bp] (ds:)=03 -> 01  (CF=1,OF=0,ZF=0,SF=0,PF=0)
    mov bp, sar8_bp_03
    sar byte [ds:bp], 1
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 16) [bp+disp] (ds:)=C0 -> E0  (CF=0,OF=0,ZF=0,SF=1,PF=0)
    lea bp, [base_bp_d]
    sar byte [ds:bp + (sar8_bpd_c0 - base_bp_d)], 1
    SAVE_FLAGS
    mov al, [ds:bp + (sar8_bpd_c0 - base_bp_d)]
    ASSERT_BYTE 0xE0
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 17) [bp+si+disp] (ds:)=04 -> 02  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    sar byte [ds:bp+si + (sar8_bpsi_04 - base_bp_si_A - base_bp_si_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+si + (sar8_bpsi_04 - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x02
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 18) [bp+di+disp] (ds:)=FE -> FF  (CF=0,OF=0,ZF=0,SF=1,PF=1)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    sar byte [ds:bp+di + (sar8_bpdi_fe - base_bp_di_A - base_bp_di_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+di + (sar8_bpdi_fe - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ===================== 8-bit SAR (register/memory, count=CL: opcode D2 /7) =====================

; 19) CL=2; AL=80 -> E0  (CF=orig bit1=0, ZF=0,SF=1,PF=0)
    mov cl, 2
    mov al, 0x80
    sar al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0xE0
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 20) CL=2; AH=03 -> 00  (CF=orig bit1=1, ZF=1,SF=0,PF=1)
    mov cl, 2
    mov ah, 0x03
    sar ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 21) CL=3; BL=F0 -> FE  (CF=orig bit2=0, ZF=0,SF=1,PF=0)
    mov cl, 3
    mov bl, 0xF0
    sar bl, cl
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0xFE
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 22) CL=7; DL=81 -> FF  (CF=orig bit6=0, ZF=0,SF=1,PF=1)
    mov cl, 7
    mov dl, 0x81
    sar dl, cl
    SAVE_FLAGS
    mov al, dl
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 23) CL=2; [di+disp]=7F -> 1F  (CF=orig bit1=1, ZF=0,SF=0,PF=0)
    mov cl, 2
    lea di, [base8dd2]
    sar byte [di + (sar8_did_7f - base8dd2)], cl
    SAVE_FLAGS
    mov al, [di + (sar8_did_7f - base8dd2)]
    ASSERT_BYTE 0x1F
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 24) CL=3; [bx]=04 -> 00  (CF=orig bit2=1, ZF=1,SF=0,PF=1)
    mov cl, 3
    lea bx, [sar8_bx_04]
    sar byte [bx], cl
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 25) CL=0 no-op (register): flags preserved (SAHF pattern=all 1s), AL unchanged
    mov ah, [pat1]
    sahf
    mov al, 0x12
    mov cl, 0
    sar al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x12
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 26) CL=0 OF-preservation (register): make OF=1 via ADD; SAR must keep OF=1; AH unchanged
    mov bh, 0x7F
    add bh, 0x01            ; OF=1
    mov ah, 0x55
    mov cl, 0
    sar ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x55
    CHECK_OF 1

; 27) CL=0 no-op (memory): SAHF pattern=all 0s; memory unchanged; flags preserved
    mov ah, [pat0]
    sahf
    lea si, [sar8_si_55]
    mov cl, 0
    sar byte [si], cl
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x55
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0


; ===================== 16-bit SAR (register, count=1: opcode D1 /7) =====================

; 28) AX=0001 -> 0000  (CF=1,OF=0,ZF=1,SF=0,PF=1)
    mov ax, 0x0001
    sar ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 29) AX=8000 -> C000  (CF=0,OF=0,ZF=0,SF=1,PF=1)
    mov ax, 0x8000
    sar ax, 1
    SAVE_FLAGS
    ASSERT_AX 0xC000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 30) AX=FFFF -> FFFF  (CF=1,OF=0,ZF=0,SF=1,PF=1)
    mov ax, 0xFFFF
    sar ax, 1
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 31) BX=0002 -> 0001  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    mov bx, 0x0002
    sar bx, 1
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 32) DX=7FFF -> 3FFF  (CF=1,OF=0,ZF=0,SF=0,PF=1)
    mov dx, 0x7FFF
    sar dx, 1
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x3FFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ===================== 16-bit SAR (memory, count=1) =====================

; 33) [si]=8000 -> C000  (CF=0,OF=0,ZF=0,SF=1,PF=1)
    lea si, [sar16_si_8000]
    sar word [si], 1
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0xC000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 34) [di]=0001 -> 0000  (CF=1,OF=0,ZF=1,SF=0,PF=1)
    lea di, [sar16_di_0001]
    sar word [di], 1
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 35) [bx]=FFFF -> FFFF  (CF=1,OF=0,ZF=0,SF=1,PF=1)
    lea bx, [sar16_bx_ffff]
    sar word [bx], 1
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 36) [si+disp]=F000 -> F800  (CF=0,OF=0,ZF=0,SF=1,PF=1)
    lea si, [base16sid]
    sar word [si + (sar16_sid_f000 - base16sid)], 1
    SAVE_FLAGS
    mov ax, [si + (sar16_sid_f000 - base16sid)]
    ASSERT_AX 0xF800
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 37) [di+disp]=0000 -> 0000  (CF=0,OF=0,ZF=1,SF=0,PF=1)
    lea di, [base16did]
    sar word [di + (sar16_did_0000 - base16did)], 1
    SAVE_FLAGS
    mov ax, [di + (sar16_did_0000 - base16did)]
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 38) [bx+disp]=4000 -> 2000  (CF=0,OF=0,ZF=0,SF=0,PF=1)
    lea bx, [base16bd]
    sar word [bx + (sar16_bxd_4000 - base16bd)], 1
    SAVE_FLAGS
    mov ax, [bx + (sar16_bxd_4000 - base16bd)]
    ASSERT_AX 0x2000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 39) [bx+si+disp]=0002 -> 0001  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    lea bx, [base16]
    lea si, [index16]
    sar word [bx+si + (sar16_bxsi_0002 - base16 - index16)], 1
    SAVE_FLAGS
    mov ax, [bx+si + (sar16_bxsi_0002 - base16 - index16)]
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 40) [bp] (ds:)=C000 -> E000  (CF=0,OF=0,ZF=0,SF=1,PF=1)
    mov bp, sar16_bp_c000
    sar word [ds:bp], 1
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0xE000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 41) [bp+disp] (ds:)=0003 -> 0001  (CF=1,OF=0,ZF=0,SF=0,PF=0)
    lea bp, [base_bp16_d]
    sar word [ds:bp + (sar16_bpd_0003 - base_bp16_d)], 1
    SAVE_FLAGS
    mov ax, [ds:bp + (sar16_bpd_0003 - base_bp16_d)]
    ASSERT_AX 0x0001
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 42) [bp+si+disp] (ds:)=8001 -> C000  (CF=1,OF=0,ZF=0,SF=1,PF=1)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    sar word [ds:bp+si + (sar16_bpsi_8001 - base_bp16_si_A - base_bp16_si_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+si + (sar16_bpsi_8001 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0xC000
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 43) [bp+di+disp] (ds:)=7FFF -> 3FFF  (CF=1,OF=0,ZF=0,SF=0,PF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    sar word [ds:bp+di + (sar16_bpdi_7fff - base_bp16_di_A - base_bp16_di_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+di + (sar16_bpdi_7fff - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x3FFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ===================== 16-bit SAR (register/memory, count=CL: opcode D3 /7) =====================

; 44) CL=2; AX=8001 -> E000  (CF=orig bit1=0, ZF=0,SF=1,PF=1)
    mov cl, 2
    mov ax, 0x8001
    sar ax, cl
    SAVE_FLAGS
    ASSERT_AX 0xE000
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 45) CL=3; BX=0008 -> 0001  (CF=orig bit2=0, ZF=0,SF=0,PF=0)
    mov cl, 3
    mov bx, 0x0008
    sar bx, cl
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 46) CL=4; DX=00F8 -> 000F  (CF=orig bit3=1, ZF=0,SF=0,PF=1)
    mov cl, 4
    mov dx, 0x00F8
    sar dx, cl
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x000F
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 47) CL=7; SI=8000 -> FF00  (CF=orig bit6=0, ZF=0,SF=1,PF=1)
    mov cl, 7
    mov si, 0x8000
    sar si, cl
    SAVE_FLAGS
    mov ax, si
    ASSERT_AX 0xFF00
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 48) CL=2; [di+disp]=0003 -> 0000  (CF=orig bit1=1, ZF=1,SF=0,PF=1)
    mov cl, 2
    lea di, [base16did2]
    sar word [di + (sar16_did_0003 - base16did2)], cl
    SAVE_FLAGS
    mov ax, [di + (sar16_did_0003 - base16did2)]
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 49) CL=3; [bx]=F000 -> FE00  (CF=orig bit2=0, ZF=0,SF=1,PF=1)
    mov cl, 3
    lea bx, [sar16_bx_f000]
    sar word [bx], cl
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0xFE00
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 50) CL=0 no-op (register): SAHF=pat1; AX unchanged; flags preserved
    mov ah, [pat1]
    sahf
    mov ax, 0x1234
    mov cl, 0
    sar ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x1234
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 51) CL=0 OF-preservation (memory): make OF=1 via ADD; SAR [bx],0 preserves OF and value
    mov si, 0x7FFF
    add si, 0x0001          ; OF=1
    lea bx, [sar16_bx_aaaa]
    mov cl, 0
    sar word [bx], cl
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
sar8_si_80:     db 0x80
sar8_di_01:     db 0x01
sar8_bx_ff:     db 0xFF

base8sid:
sar8_sid_f0:    db 0xF0

base8dd:
sar8_did_00:    db 0x00

base8bd:
sar8_bxd_02:    db 0x02

base8:
index8:
sar8_bxsi_7f:   db 0x7F

sar8_bp_03:     db 0x03

base_bp_d:
sar8_bpd_c0:    db 0xC0

base_bp_si_A:
base_bp_si_B:
sar8_bpsi_04:   db 0x04

base_bp_di_A:
base_bp_di_B:
sar8_bpdi_fe:   db 0xFE

; ---- 8-bit memory (count=CL) ----
base8dd2:
sar8_did_7f:    db 0x7F

sar8_bx_04:     db 0x04

sar8_si_55:     db 0x55

; ---- 16-bit memory (count=1) ----
sar16_si_8000:    dw 0x8000
sar16_di_0001:    dw 0x0001
sar16_bx_ffff:    dw 0xFFFF

base16sid:
sar16_sid_f000:   dw 0xF000

base16did:
sar16_did_0000:   dw 0x0000

base16bd:
sar16_bxd_4000:   dw 0x4000

base16:
index16:
sar16_bxsi_0002:  dw 0x0002

sar16_bp_c000:    dw 0xC000

base_bp16_d:
sar16_bpd_0003:   dw 0x0003

base_bp16_si_A:
base_bp16_si_B:
sar16_bpsi_8001:  dw 0x8001

base_bp16_di_A:
base_bp16_di_B:
sar16_bpdi_7fff:  dw 0x7FFF

; ---- 16-bit memory (count=CL) ----
base16did2:
sar16_did_0003:   dw 0x0003

sar16_bx_f000:    dw 0xF000

sar16_bx_aaaa:    dw 0xAAAA

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
