; rcr.asm — thorough RCR tests (r/m8, r/m16) for count=1 and count=CL (including CL=0 no-op)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   count=1: CF_out=old LSB; new MSB=old CF_in; OF = new MSB XOR CF_out
;            SF/ZF/PF/AF preserved (we verify via SAHF)
;   count>=2: CF_out = last bit out through CF chain; OF undefined (don't check); SF/ZF/PF/AF preserved
;   CL=0: operand & ALL flags preserved (verify with SAHF + OF-preservation)

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

; ===================== 8-bit RCR (register, count=1: opcode D0 /3) =====================

; 1) AL=00, CF_in=0 -> 00 ; CF=0, OF=0 ; flags preserved (0s)
    mov al, 0x00
    mov ah, [pat0]
    sahf
    rcr al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 2) AL=00, CF_in=1 -> 80 ; CF=0, OF=1 ; flags preserved (1s)
    mov al, 0x00
    mov ah, [pat1]
    sahf
    rcr al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 3) AL=01, CF_in=0 -> 00 ; CF=1, OF=1 ; flags preserved (0s)
    mov al, 0x01
    mov ah, [pat0]
    sahf
    rcr al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 4) AL=80, CF_in=0 -> 40 ; CF=0, OF=0 ; flags preserved (0s)
    mov al, 0x80
    mov ah, [pat0]
    sahf
    rcr al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x40
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 5) AH=FF, CF_in=1 -> FF ; CF=1, OF=0 ; flags preserved (1s)
    mov ah, 0xFF
    mov al, [pat1]
    xchg al, ah
    sahf
    xchg al, ah
    rcr ah, 1
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 6) BL=7F, CF_in=1 -> BF ; CF=1, OF=0 ; flags preserved (1s)
    mov bl, 0x7F
    mov ah, [pat1]
    sahf
    rcr bl, 1
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0xBF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 7) Overwrite prior flags: make OF=1 via ADD, then CF_in=0; RCR AL=00 -> 00 (CF=0,OF=0)
    mov bh, 0x7F
    add bh, 0x01          ; OF=1, CF=0
    clc                   ; ensure CF_in=0 for RCR
    mov al, 0x00
    rcr al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0


; ===================== 8-bit RCR (memory, count=1) =====================

; 8) [si]=7E, CF_in=0 -> 3F ; CF=0, OF=0 ; flags preserved (0s)
    lea si, [rcr8_si_7e]
    mov ah, [pat0]
    sahf
    rcr byte [si], 1
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x3F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 9) [di]=80, CF_in=1 -> C0 ; CF=0, OF=1 ; flags preserved (1s)
    lea di, [rcr8_di_80]
    mov ah, [pat1]
    sahf
    rcr byte [di], 1
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0xC0
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 10) [bx]=01, CF_in=0 -> 00 ; CF=1, OF=1 ; flags preserved (0s)
    lea bx, [rcr8_bx_01]
    mov ah, [pat0]
    sahf
    rcr byte [bx], 1
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 11) [si+disp]=FF, CF_in=1 -> FF ; CF=1, OF=0 ; flags preserved (1s)
    lea si, [base8sid]
    mov ah, [pat1]
    sahf
    rcr byte [si + (rcr8_sid_ff - base8sid)], 1
    SAVE_FLAGS
    mov al, [si + (rcr8_sid_ff - base8sid)]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 12) [di+disp]=00, CF_in=0 -> 00 ; CF=0, OF=0 ; flags preserved (0s)
    lea di, [base8dd]
    mov ah, [pat0]
    sahf
    rcr byte [di + (rcr8_did_00 - base8dd)], 1
    SAVE_FLAGS
    mov al, [di + (rcr8_did_00 - base8dd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 13) [bx+disp]=02, CF_in=1 -> 81 ; CF=0, OF=1 ; flags preserved (1s)
    lea bx, [base8bd]
    mov ah, [pat1]
    sahf
    rcr byte [bx + (rcr8_bxd_02 - base8bd)], 1
    SAVE_FLAGS
    mov al, [bx + (rcr8_bxd_02 - base8bd)]
    ASSERT_BYTE 0x81
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 14) [bx+si+disp]=7F, CF_in=0 -> 3F ; CF=1, OF=1 ; flags preserved (0s)
    lea bx, [base8]
    lea si, [index8]
    mov ah, [pat0]
    sahf
    rcr byte [bx+si + (rcr8_bxsi_7f - base8 - index8)], 1
    SAVE_FLAGS
    mov al, [bx+si + (rcr8_bxsi_7f - base8 - index8)]
    ASSERT_BYTE 0x3F
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 15) [bp] (ds:)=03, CF_in=1 -> 81 ; CF=1, OF=0 ; flags preserved (1s)
    mov bp, rcr8_bp_03
    mov ah, [pat1]
    sahf
    rcr byte [ds:bp], 1
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0x81
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 16) [bp+disp] (ds:)=C0, CF_in=0 -> 60 ; CF=0, OF=0 ; flags preserved (0s)
    lea bp, [base_bp_d]
    mov ah, [pat0]
    sahf
    rcr byte [ds:bp + (rcr8_bpd_c0 - base_bp_d)], 1
    SAVE_FLAGS
    mov al, [ds:bp + (rcr8_bpd_c0 - base_bp_d)]
    ASSERT_BYTE 0x60
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 17) [bp+si+disp] (ds:)=04, CF_in=1 -> 82 ; CF=0, OF=1 ; flags preserved (1s)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov ah, [pat1]
    sahf
    rcr byte [ds:bp+si + (rcr8_bpsi_04 - base_bp_si_A - base_bp_si_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+si + (rcr8_bpsi_04 - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x82
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 18) [bp+di+disp] (ds:)=FE, CF_in=0 -> 7F ; CF=0, OF=0 ; flags preserved (0s)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov ah, [pat0]
    sahf
    rcr byte [ds:bp+di + (rcr8_bpdi_fe - base_bp_di_A - base_bp_di_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+di + (rcr8_bpdi_fe - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0


; ===================== 8-bit RCR (register/memory, count=CL: opcode D2 /3) =====================

; 19) CL=2; AL=81, CF_in=0 -> A0 ; CF=0 ; flags preserved (0s)
    mov cl, 2
    mov al, 0x81
    mov ah, [pat0]
    sahf
    rcr al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0xA0
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 20) CL=2; AH=C1, CF_in=1 -> F0 ; CF=0 ; flags preserved (1s)
    mov cl, 2
    mov ah, 0xC1
    mov al, [pat1]
    xchg al, ah
    sahf
    xchg al, ah
    rcr ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0xF0
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 21) CL=3; BL=03, CF_in=0 -> C0 ; CF=0 ; flags preserved (0s)
    mov cl, 3
    mov bl, 0x03
    mov ah, [pat0]
    sahf
    rcr bl, cl
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0xC0
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 22) CL=7; DL=03, CF_in=1 -> 0E ; CF=0 ; flags preserved (1s)
    mov cl, 7
    mov dl, 0x03
    mov ah, [pat1]
    sahf
    rcr dl, cl
    SAVE_FLAGS
    mov al, dl
    ASSERT_BYTE 0x0E
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 23) CL=2; [di+disp]=81, CF_in=0 -> A0 ; CF=0 ; flags preserved (0s)
    mov cl, 2
    lea di, [base8dd2]
    mov ah, [pat0]
    sahf
    rcr byte [di + (rcr8_did_81 - base8dd2)], cl
    SAVE_FLAGS
    mov al, [di + (rcr8_did_81 - base8dd2)]
    ASSERT_BYTE 0xA0
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 24) CL=3; [bx]=03, CF_in=1 -> E0 ; CF=0 ; flags preserved (1s)
    mov cl, 3
    lea bx, [rcr8_bx_03]
    mov ah, [pat1]
    sahf
    rcr byte [bx], cl
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0xE0
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 25) CL=0 no-op (register): SAHF=pat1; AL unchanged; flags preserved incl. OF
    mov ah, [pat1]
    sahf
    mov al, 0x12
    mov cl, 0
    rcr al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x12
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 26) CL=0 OF-preservation (register): make OF=1 via ADD; RCR must keep OF=1; AH unchanged
    mov bh, 0x7F
    add bh, 0x01            ; OF=1
    mov ah, 0x55
    mov cl, 0
    rcr ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x55
    CHECK_OF 1

; 27) CL=0 no-op (memory): SAHF=pat0; memory unchanged; flags preserved
    mov ah, [pat0]
    sahf
    lea si, [rcr8_si_55]
    mov cl, 0
    rcr byte [si], cl
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x55
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0


; ===================== 16-bit RCR (register, count=1: opcode D1 /3) =====================

; 28) AX=0000, CF_in=0 -> 0000 ; CF=0, OF=0 ; flags preserved (0s)
    mov ax, 0x0000
    mov ah, [pat0]
    sahf
    rcr ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 29) AX=8000, CF_in=1 -> C000 ; CF=0, OF=1 ; flags preserved (1s)
    mov ah, [pat1]
    sahf
    mov ax, 0x8000
    rcr ax, 1
    SAVE_FLAGS
    ASSERT_AX 0xC000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 30) AX=4000, CF_in=0 -> 2000 ; CF=0, OF=0 ; flags preserved (0s)
    mov ah, [pat0]
    sahf
    mov ax, 0x4000
    rcr ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x2000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 31) BX=FFFF, CF_in=1 -> FFFF ; CF=1, OF=0 ; flags preserved (1s)
    mov bx, 0xFFFF
    mov ah, [pat1]
    sahf
    rcr bx, 1
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 32) DX=7FFF, CF_in=0 -> 3FFF ; CF=1, OF=1 ; flags preserved (0s)
    mov dx, 0x7FFF
    mov ah, [pat0]
    sahf
    rcr dx, 1
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x3FFF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0


; ===================== 16-bit RCR (memory, count=1) =====================

; 33) [si]=7FFF, CF_in=1 -> BFFF ; CF=1, OF=0 ; flags preserved (1s)
    lea si, [rcr16_si_7fff]
    mov ah, [pat1]
    sahf
    rcr word [si], 1
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0xBFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 34) [di]=8000, CF_in=0 -> 4000 ; CF=0, OF=0 ; flags preserved (0s)
    lea di, [rcr16_di_8000]
    mov ah, [pat0]
    sahf
    rcr word [di], 1
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x4000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 35) [bx]=0001, CF_in=1 -> 8000 ; CF=1, OF=0 ; flags preserved (1s)
    lea bx, [rcr16_bx_0001]
    mov ah, [pat1]
    sahf
    rcr word [bx], 1
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x8000
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 36) [si+disp]=FFFF, CF_in=0 -> 7FFF ; CF=1, OF=1 ; flags preserved (0s)
    lea si, [base16sid]
    mov ah, [pat0]
    sahf
    rcr word [si + (rcr16_sid_ffff - base16sid)], 1
    SAVE_FLAGS
    mov ax, [si + (rcr16_sid_ffff - base16sid)]
    ASSERT_AX 0x7FFF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 37) [di+disp]=0100, CF_in=1 -> 8080 ; CF=0, OF=1 ; flags preserved (1s)
    lea di, [base16did]
    mov ah, [pat1]
    sahf
    rcr word [di + (rcr16_did_0100 - base16did)], 1
    SAVE_FLAGS
    mov ax, [di + (rcr16_did_0100 - base16did)]
    ASSERT_AX 0x8080
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 38) [bx+disp]=3FFF, CF_in=0 -> 1FFF ; CF=1, OF=1 ; flags preserved (0s)
    lea bx, [base16bd]
    mov ah, [pat0]
    sahf
    rcr word [bx + (rcr16_bxd_3fff - base16bd)], 1
    SAVE_FLAGS
    mov ax, [bx + (rcr16_bxd_3fff - base16bd)]
    ASSERT_AX 0x1FFF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 39) [bx+si+disp]=4000, CF_in=1 -> A000 ; CF=0, OF=1 ; flags preserved (1s)
    lea bx, [base16]
    lea si, [index16]
    mov ah, [pat1]
    sahf
    rcr word [bx+si + (rcr16_bxsi_4000 - base16 - index16)], 1
    SAVE_FLAGS
    mov ax, [bx+si + (rcr16_bxsi_4000 - base16 - index16)]
    ASSERT_AX 0xA000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 40) [bp] (ds:)=FFFF, CF_in=0 -> 7FFF ; CF=1, OF=1 ; flags preserved (0s)
    mov bp, rcr16_bp_ffff
    mov ah, [pat0]
    sahf
    rcr word [ds:bp], 1
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x7FFF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 41) [bp+disp] (ds:)=0002, CF_in=1 -> 8001 ; CF=0, OF=1 ; flags preserved (1s)
    lea bp, [base_bp16_d]
    mov ah, [pat1]
    sahf
    rcr word [ds:bp + (rcr16_bpd_0002 - base_bp16_d)], 1
    SAVE_FLAGS
    mov ax, [ds:bp + (rcr16_bpd_0002 - base_bp16_d)]
    ASSERT_AX 0x8001
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 42) [bp+si+disp] (ds:)=2000, CF_in=0 -> 1000 ; CF=0, OF=0 ; flags preserved (0s)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    mov ah, [pat0]
    sahf
    rcr word [ds:bp+si + (rcr16_bpsi_2000 - base_bp16_si_A - base_bp16_si_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+si + (rcr16_bpsi_2000 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x1000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 43) [bp+di+disp] (ds:)=C000, CF_in=1 -> E000 ; CF=0, OF=1 ; flags preserved (1s)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov ah, [pat1]
    sahf
    rcr word [ds:bp+di + (rcr16_bpdi_c000 - base_bp16_di_A - base_bp16_di_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+di + (rcr16_bpdi_c000 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0xE000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1


; ===================== 16-bit RCR (register/memory, count=CL: opcode D3 /3) =====================

; 44) CL=2; AX=8001, CF_in=0 -> A000 ; CF=0 ; flags preserved (0s)
    mov cl, 2
    mov ah, [pat0]
    sahf
    mov ax, 0x8001
    rcr ax, cl
    SAVE_FLAGS
    ASSERT_AX 0xA000
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 45) CL=3; BX=1000, CF_in=1 -> 2200 ; CF=0 ; flags preserved (1s)
    mov cl, 3
    mov bx, 0x1000
    mov ah, [pat1]
    sahf
    rcr bx, cl
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x2200
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 46) CL=4; DX=00F0, CF_in=0 -> 000F ; CF=0 ; flags preserved (0s)
    mov cl, 4
    mov dx, 0x00F0
    mov ah, [pat0]
    sahf
    rcr dx, cl
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x000F
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 47) CL=7; SI=0003, CF_in=1 -> 0E00 ; CF=0 ; flags preserved (1s)
    mov cl, 7
    mov si, 0x0003
    mov ah, [pat1]
    sahf
    rcr si, cl
    SAVE_FLAGS
    mov ax, si
    ASSERT_AX 0x0E00
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 48) CL=2; [di+disp]=C000, CF_in=0 -> 3000 ; CF=0 ; flags preserved (0s)
    mov cl, 2
    lea di, [base16did2]
    mov ah, [pat0]
    sahf
    rcr word [di + (rcr16_did_c000 - base16did2)], cl
    SAVE_FLAGS
    mov ax, [di + (rcr16_did_c000 - base16did2)]
    ASSERT_AX 0x3000
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 49) CL=3; [bx]=0003, CF_in=1 -> E000 ; CF=0 ; flags preserved (1s)
    mov cl, 3
    lea bx, [rcr16_bx_0003]
    mov ah, [pat1]
    sahf
    rcr word [bx], cl
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0xE000
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 50) CL=0 no-op (register): SAHF=pat1; AX unchanged; flags preserved
    mov ah, [pat1]
    sahf
    mov ax, 0x1234
    mov cl, 0
    rcr ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x1234
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 51) CL=0 OF-preservation (memory): make OF=1 via ADD; RCR [bx],0 preserves OF and value
    mov si, 0x7FFF
    add si, 0x0001          ; OF=1
    lea bx, [rcr16_bx_aaaa]
    mov cl, 0
    rcr word [bx], cl
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0xAAAA
    CHECK_OF 1

; ===================== RCR corner-case counts (add after test 51, before exit) =====================

; 52) 8-bit: CL=9 (full 9-bit cycle) — value & CF preserved
    mov ah, [pat0]          ; CF_in=0; SF/ZF/AF/PF=0
    sahf
    mov cl, 9
    mov al, 0x5A
    rcr al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x5A
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 53) 8-bit mem: CL=9 (full cycle) — value & CF preserved (CF_in=1)
    mov ah, [pat1]          ; CF_in=1; SF/ZF/AF/PF=1
    sahf
    mov cl, 9
    lea bx, [rcr8_ex_a5]
    rcr byte [bx], cl
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0xA5
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 54) 8-bit: CL=10 (≡ 1 mod 9) — same as 1-step, OF undefined so not checked
    mov ah, [pat0]          ; CF_in=0
    sahf
    mov cl, 10
    mov al, 0x01
    rcr al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x00        ; 0x01 -> 0x00 with CF_out=1
    CHECK_CF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 55) 8-bit mem: CL=10 (≡ 1 mod 9), CF_in=1
    mov ah, [pat1]
    sahf
    mov cl, 10
    lea si, [rcr8_ex_00]
    rcr byte [si], cl
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x80        ; 0x00 -> 0x80, CF_out=0
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 56) 16-bit: CL=18 (≡ 1 mod 17), CF_in=0
    mov ah, [pat0]
    sahf
    mov cl, 18
    mov ax, 0x0001
    rcr ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x0000        ; 0x0001 -> 0x0000, CF_out=1
    CHECK_CF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 57) 16-bit mem: CL=17 (full 17-bit cycle), CF_in=1 — value & CF preserved
    mov ah, [pat1]
    sahf
    mov cl, 17
    lea di, [rcr16_ex_beef]
    rcr word [di], cl
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0xBEEF
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 58) 16-bit mem: CL=31 (≡ 14 mod 17) with all-ones + CF=1 → stays all-ones, CF=1
    mov ah, [pat1]
    sahf
    mov cl, 31
    lea si, [rcr16_ex_ffff]
    rcr word [si], cl
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 59) 16-bit: CL=33 (masked to 1) — behaves like 1-step, OF undefined (do not check)
    mov ah, [pat1]          ; CF_in=1
    sahf
    mov cl, 33
    mov dx, 0x8000
    rcr dx, cl
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0xC000        ; 0x8000 -> 0xC000, CF_out=0
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 60) 8-bit mem: CL=25 (≡ 7 mod 9) with all-ones + CF=1 → unchanged
    mov ah, [pat1]
    sahf
    mov cl, 25
    lea bx, [rcr8_ex_ff]
    rcr byte [bx], cl
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 61) 16-bit: CL=32 (masked to 0) — explicit no-op via count masking
    mov ah, [pat0]
    sahf
    mov si, 0x1234
    mov cl, 32
    rcr si, cl
    SAVE_FLAGS
    mov ax, si
    ASSERT_AX 0x1234
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; ---- 8-bit memory (count=1) ----
rcr8_si_7e:     db 0x7E
rcr8_di_80:     db 0x80
rcr8_bx_01:     db 0x01

base8sid:
rcr8_sid_ff:    db 0xFF

base8dd:
rcr8_did_00:    db 0x00

base8bd:
rcr8_bxd_02:    db 0x02

base8:
index8:
rcr8_bxsi_7f:   db 0x7F

rcr8_bp_03:     db 0x03

base_bp_d:
rcr8_bpd_c0:    db 0xC0

base_bp_si_A:
base_bp_si_B:
rcr8_bpsi_04:   db 0x04

base_bp_di_A:
base_bp_di_B:
rcr8_bpdi_fe:   db 0xFE

; ---- 8-bit memory (count=CL) ----
base8dd2:
rcr8_did_81:    db 0x81

rcr8_bx_03:     db 0x03

rcr8_si_55:     db 0x55

; ---- 16-bit memory (count=1) ----
rcr16_si_7fff:    dw 0x7FFF
rcr16_di_8000:    dw 0x8000
rcr16_bx_0001:    dw 0x0001

base16sid:
rcr16_sid_ffff:   dw 0xFFFF

base16did:
rcr16_did_0100:   dw 0x0100

base16bd:
rcr16_bxd_3fff:   dw 0x3FFF

base16:
index16:
rcr16_bxsi_4000:  dw 0x4000

rcr16_bp_ffff:    dw 0xFFFF

base_bp16_d:
rcr16_bpd_0002:   dw 0x0002

base_bp16_si_A:
base_bp16_si_B:
rcr16_bpsi_2000:  dw 0x2000

base_bp16_di_A:
base_bp16_di_B:
rcr16_bpdi_c000:  dw 0xC000

; ---- 16-bit memory (count=CL) ----
base16did2:
rcr16_did_c000:   dw 0xC000

rcr16_bx_0003:    dw 0x0003

rcr16_bx_aaaa:    dw 0xAAAA

; ---- Extra data for corner-case tests ----
rcr8_ex_a5:      db 0xA5
rcr8_ex_00:      db 0x00
rcr8_ex_ff:      db 0xFF
rcr16_ex_beef:   dw 0xBEEF
rcr16_ex_ffff:   dw 0xFFFF

; SAHF patterns to verify preservation of SF/ZF/AF/PF/CF (OF unaffected by SAHF)
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
