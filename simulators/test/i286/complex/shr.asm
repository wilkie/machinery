; shr.asm — thorough SHR tests (r/m8, r/m16) for count=1 and count=CL (including CL=0 no-op)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   SHR is logical right shift (zero fill).
;   count=1: CF=old LSB, OF=old MSB; ZF/SF/PF from result; AF undefined (not checked)
;   count>=2: CF=last bit shifted out; OF undefined (not checked)
;   CL=0: operand and all flags unchanged (we verify with SAHF + OF-preservation)

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

; ===================== 8-bit SHR (register, count=1: opcode D0 /5) =====================

; 1) AL=00 >>1 -> 00  (CF=0,OF=0,ZF=1,SF=0,PF=1)
    mov al, 0x00
    shr al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 2) AL=01 >>1 -> 00  (CF=1,OF=0,ZF=1,SF=0,PF=1)
    mov al, 0x01
    shr al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 3) AL=80 >>1 -> 40  (CF=0,OF=1,ZF=0,SF=0,PF=0)
    mov al, 0x80
    shr al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x40
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 4) AL=FF >>1 -> 7F  (CF=1,OF=1,ZF=0,SF=0,PF=0)
    mov al, 0xFF
    shr al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x7F
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 5) AH=02 >>1 -> 01  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    mov ah, 0x02
    shr ah, 1
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
    shr bl, 1
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
    shr al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== 8-bit SHR (memory, count=1) =====================

; 8) [si]=80 -> 40  (CF=0,OF=1,ZF=0,SF=0,PF=0)
    lea si, [shr8_si_80]
    shr byte [si], 1
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x40
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 9) [di]=01 -> 00  (CF=1,OF=0,ZF=1,SF=0,PF=1)
    lea di, [shr8_di_01]
    shr byte [di], 1
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 10) [bx]=FF -> 7F  (CF=1,OF=1,ZF=0,SF=0,PF=0)
    lea bx, [shr8_bx_ff]
    shr byte [bx], 1
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x7F
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 11) [si+disp]=F0 -> 78  (CF=0,OF=1,ZF=0,SF=0,PF=1)
    lea si, [base8sid]
    shr byte [si + (shr8_sid_f0 - base8sid)], 1
    SAVE_FLAGS
    mov al, [si + (shr8_sid_f0 - base8sid)]
    ASSERT_BYTE 0x78
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 12) [di+disp]=00 -> 00  (CF=0,OF=0,ZF=1,SF=0,PF=1)
    lea di, [base8dd]
    shr byte [di + (shr8_did_00 - base8dd)], 1
    SAVE_FLAGS
    mov al, [di + (shr8_did_00 - base8dd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 13) [bx+disp]=02 -> 01  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    lea bx, [base8bd]
    shr byte [bx + (shr8_bxd_02 - base8bd)], 1
    SAVE_FLAGS
    mov al, [bx + (shr8_bxd_02 - base8bd)]
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 14) [bx+si+disp]=7F -> 3F  (CF=1,OF=0,ZF=0,SF=0,PF=1)
    lea bx, [base8]
    lea si, [index8]
    shr byte [bx+si + (shr8_bxsi_7f - base8 - index8)], 1
    SAVE_FLAGS
    mov al, [bx+si + (shr8_bxsi_7f - base8 - index8)]
    ASSERT_BYTE 0x3F
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 15) [bp] (ds:)=03 -> 01  (CF=1,OF=0,ZF=0,SF=0,PF=0)
    mov bp, shr8_bp_03
    shr byte [ds:bp], 1
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 16) [bp+disp] (ds:)=C0 -> 60  (CF=0,OF=1,ZF=0,SF=0,PF=1)
    lea bp, [base_bp_d]
    shr byte [ds:bp + (shr8_bpd_c0 - base_bp_d)], 1
    SAVE_FLAGS
    mov al, [ds:bp + (shr8_bpd_c0 - base_bp_d)]
    ASSERT_BYTE 0x60
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 17) [bp+si+disp] (ds:)=04 -> 02  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    shr byte [ds:bp+si + (shr8_bpsi_04 - base_bp_si_A - base_bp_si_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+si + (shr8_bpsi_04 - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x02
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 18) [bp+di+disp] (ds:)=FE -> 7F  (CF=0,OF=1,ZF=0,SF=0,PF=0)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    shr byte [ds:bp+di + (shr8_bpdi_fe - base_bp_di_A - base_bp_di_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+di + (shr8_bpdi_fe - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0


; ===================== 8-bit SHR (register/memory, count=CL: opcode D2 /5) =====================

; 19) CL=2; AL=80 -> 20  (CF=orig bit1=0, ZF=0,SF=0,PF=0)
    mov cl, 2
    mov al, 0x80
    shr al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x20
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 20) CL=2; AH=03 -> 00  (CF=orig bit1=1, ZF=1,SF=0,PF=1)
    mov cl, 2
    mov ah, 0x03
    shr ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 21) CL=3; BL=F0 -> 1E  (CF=orig bit2=0, ZF=0,SF=0,PF=1)
    mov cl, 3
    mov bl, 0xF0
    shr bl, cl
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0x1E
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 22) CL=7; DL=81 -> 01  (CF=orig bit6=0, ZF=0,SF=0,PF=0)
    mov cl, 7
    mov dl, 0x81
    shr dl, cl
    SAVE_FLAGS
    mov al, dl
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 23) CL=2; [di+disp]=7F -> 1F  (CF=orig bit1=1, ZF=0,SF=0,PF=0)
    mov cl, 2
    lea di, [base8dd2]
    shr byte [di + (shr8_did_7f - base8dd2)], cl
    SAVE_FLAGS
    mov al, [di + (shr8_did_7f - base8dd2)]
    ASSERT_BYTE 0x1F
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 24) CL=3; [bx]=04 -> 00  (CF=orig bit2=1, ZF=1,SF=0,PF=1)
    mov cl, 3
    lea bx, [shr8_bx_04]
    shr byte [bx], cl
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
    shr al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x12
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    ; OF preserved too; verified below

; 26) CL=0 OF-preservation (register): make OF=1 via ADD; SHR must keep OF=1; AH unchanged
    mov bh, 0x7F
    add bh, 0x01            ; OF=1
    mov ah, 0x55
    mov cl, 0
    shr ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x55
    CHECK_OF 1

; 27) CL=0 no-op (memory): SAHF pattern=all 0s; memory unchanged; flags preserved
    mov ah, [pat0]
    sahf
    lea si, [shr8_si_55]
    mov cl, 0
    shr byte [si], cl
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x55
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0


; ===================== 16-bit SHR (register, count=1: opcode D1 /5) =====================

; 28) AX=0001 -> 0000  (CF=1,OF=0,ZF=1,SF=0,PF=1)
    mov ax, 0x0001
    shr ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 29) AX=8000 -> 4000  (CF=0,OF=1,ZF=0,SF=0,PF=1)
    mov ax, 0x8000
    shr ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x4000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 30) AX=FFFF -> 7FFF  (CF=1,OF=1,ZF=0,SF=0,PF=1)
    mov ax, 0xFFFF
    shr ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x7FFF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 31) BX=0002 -> 0001  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    mov bx, 0x0002
    shr bx, 1
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
    shr dx, 1
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x3FFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ===================== 16-bit SHR (memory, count=1) =====================

; 33) [si]=8000 -> 4000  (CF=0,OF=1,ZF=0,SF=0,PF=1)
    lea si, [shr16_si_8000]
    shr word [si], 1
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x4000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 34) [di]=0001 -> 0000  (CF=1,OF=0,ZF=1,SF=0,PF=1)
    lea di, [shr16_di_0001]
    shr word [di], 1
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 35) [bx]=FFFF -> 7FFF  (CF=1,OF=1,ZF=0,SF=0,PF=1)
    lea bx, [shr16_bx_ffff]
    shr word [bx], 1
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x7FFF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 36) [si+disp]=F000 -> 7800  (CF=0,OF=1,ZF=0,SF=0,PF=1)
    lea si, [base16sid]
    shr word [si + (shr16_sid_f000 - base16sid)], 1
    SAVE_FLAGS
    mov ax, [si + (shr16_sid_f000 - base16sid)]
    ASSERT_AX 0x7800
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 37) [di+disp]=0000 -> 0000  (CF=0,OF=0,ZF=1,SF=0,PF=1)
    lea di, [base16did]
    shr word [di + (shr16_did_0000 - base16did)], 1
    SAVE_FLAGS
    mov ax, [di + (shr16_did_0000 - base16did)]
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 38) [bx+disp]=4000 -> 2000  (CF=0,OF=0,ZF=0,SF=0,PF=1)
    lea bx, [base16bd]
    shr word [bx + (shr16_bxd_4000 - base16bd)], 1
    SAVE_FLAGS
    mov ax, [bx + (shr16_bxd_4000 - base16bd)]
    ASSERT_AX 0x2000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 39) [bx+si+disp]=0002 -> 0001  (CF=0,OF=0,ZF=0,SF=0,PF=0)
    lea bx, [base16]
    lea si, [index16]
    shr word [bx+si + (shr16_bxsi_0002 - base16 - index16)], 1
    SAVE_FLAGS
    mov ax, [bx+si + (shr16_bxsi_0002 - base16 - index16)]
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 40) [bp] (ds:)=C000 -> 6000  (CF=0,OF=1,ZF=0,SF=0,PF=1)
    mov bp, shr16_bp_c000
    shr word [ds:bp], 1
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x6000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 41) [bp+disp] (ds:)=0003 -> 0001  (CF=1,OF=0,ZF=0,SF=0,PF=0)
    lea bp, [base_bp16_d]
    shr word [ds:bp + (shr16_bpd_0003 - base_bp16_d)], 1
    SAVE_FLAGS
    mov ax, [ds:bp + (shr16_bpd_0003 - base_bp16_d)]
    ASSERT_AX 0x0001
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 42) [bp+si+disp] (ds:)=8001 -> 4000  (CF=1,OF=1,ZF=0,SF=0,PF=1)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    shr word [ds:bp+si + (shr16_bpsi_8001 - base_bp16_si_A - base_bp16_si_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+si + (shr16_bpsi_8001 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x4000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 43) [bp+di+disp] (ds:)=7FFF -> 3FFF  (CF=1,OF=0,ZF=0,SF=0,PF=1)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    shr word [ds:bp+di + (shr16_bpdi_7fff - base_bp16_di_A - base_bp16_di_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+di + (shr16_bpdi_7fff - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x3FFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ===================== 16-bit SHR (register/memory, count=CL: opcode D3 /5) =====================

; 44) CL=2; AX=8001 -> 2000  (CF=orig bit1=0, ZF=0,SF=0,PF=1)
    mov cl, 2
    mov ax, 0x8001
    shr ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x2000
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 45) CL=3; BX=0008 -> 0001  (CF=orig bit2=0, ZF=0,SF=0,PF=0)
    mov cl, 3
    mov bx, 0x0008
    shr bx, cl
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 46) CL=4; DX=00F0 -> 000F  (CF=orig bit3=0, ZF=0,SF=0,PF=1)
    mov cl, 4
    mov dx, 0x00F0
    shr dx, cl
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x000F
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 47) CL=7; SI=8000 -> 0100  (CF=orig bit6=0, ZF=0,SF=0,PF=1)
    mov cl, 7
    mov si, 0x8000
    shr si, cl
    SAVE_FLAGS
    mov ax, si
    ASSERT_AX 0x0100
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 48) CL=2; [di+disp]=0003 -> 0000  (CF=orig bit1=1, ZF=1,SF=0,PF=1)
    mov cl, 2
    lea di, [base16did2]
    shr word [di + (shr16_did_0003 - base16did2)], cl
    SAVE_FLAGS
    mov ax, [di + (shr16_did_0003 - base16did2)]
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 49) CL=3; [bx]=F000 -> 1E00  (CF=orig bit2=0, ZF=0,SF=0,PF=1)
    mov cl, 3
    lea bx, [shr16_bx_f000]
    shr word [bx], cl
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x1E00
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 50) CL=0 no-op (register): SAHF=pat1; AX unchanged; flags preserved
    mov ah, [pat1]
    sahf
    mov ax, 0x1234
    mov cl, 0
    shr ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x1234
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 51) CL=0 OF-preservation (memory): make OF=1 via ADD; SHR [bx],0 preserves OF and value
    mov si, 0x7FFF
    add si, 0x0001          ; OF=1
    lea bx, [shr16_bx_aaaa]
    mov cl, 0
    shr word [bx], cl
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
shr8_si_80:     db 0x80
shr8_di_01:     db 0x01
shr8_bx_ff:     db 0xFF

base8sid:
shr8_sid_f0:    db 0xF0

base8dd:
shr8_did_00:    db 0x00

base8bd:
shr8_bxd_02:    db 0x02

base8:
index8:
shr8_bxsi_7f:   db 0x7F

shr8_bp_03:     db 0x03

base_bp_d:
shr8_bpd_c0:    db 0xC0

base_bp_si_A:
base_bp_si_B:
shr8_bpsi_04:   db 0x04

base_bp_di_A:
base_bp_di_B:
shr8_bpdi_fe:   db 0xFE

; ---- 8-bit memory (count=CL) ----
base8dd2:
shr8_did_7f:    db 0x7F

shr8_bx_04:     db 0x04

shr8_si_55:     db 0x55

; ---- 16-bit memory (count=1) ----
shr16_si_8000:    dw 0x8000
shr16_di_0001:    dw 0x0001
shr16_bx_ffff:    dw 0xFFFF

base16sid:
shr16_sid_f000:   dw 0xF000

base16did:
shr16_did_0000:   dw 0x0000

base16bd:
shr16_bxd_4000:   dw 0x4000

base16:
index16:
shr16_bxsi_0002:  dw 0x0002

shr16_bp_c000:    dw 0xC000

base_bp16_d:
shr16_bpd_0003:   dw 0x0003

base_bp16_si_A:
base_bp16_si_B:
shr16_bpsi_8001:  dw 0x8001

base_bp16_di_A:
base_bp16_di_B:
shr16_bpdi_7fff:  dw 0x7FFF

; ---- 16-bit memory (count=CL) ----
base16did2:
shr16_did_0003:   dw 0x0003

shr16_bx_f000:    dw 0xF000

shr16_bx_aaaa:    dw 0xAAAA

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
