; ror.asm — thorough ROR tests (r/m8, r/m16) for count=1 and count=CL (including CL=0 no-op)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   count=1: CF=old LSB; OF = (result bit7) XOR (result bit6) = old LSB XOR old MSB
;            SF/ZF/PF/AF preserved (we verify via SAHF)
;   count>=2: CF=last bit out; OF undefined (don't check); SF/ZF/PF/AF preserved
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

; ===================== 8-bit ROR (register, count=1: opcode D0 /1) =====================

; 1) AL=00 -> 00 ; CF=0, OF=0 ; SF/ZF/PF/AF preserved (0s)
    mov al, 0x00
    mov ah, [pat0]
    sahf
    ror al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 2) AL=01 -> 80 ; CF=1, OF=1 ; flags preserved (1s)
    mov al, 0x01
    mov ah, [pat1]
    sahf
    ror al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 3) AL=80 -> 40 ; CF=0, OF=1 ; flags preserved (0s)
    mov al, 0x80
    mov ah, [pat0]
    sahf
    ror al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x40
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 4) AL=40 -> 20 ; CF=0, OF=0 ; flags preserved (1s)
    mov al, 0x40
    mov ah, [pat1]
    sahf
    ror al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x20
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 5) AH=FF -> FF ; CF=1, OF=0 ; flags preserved (0s)
    mov ah, 0xFF
    mov al, [pat0]
    xchg al, ah
    sahf
    xchg al, ah
    ror ah, 1
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 6) BL=7F -> BF ; CF=1, OF=1 ; flags preserved (1s)
    mov bl, 0x7F
    mov ah, [pat1]
    sahf
    ror bl, 1
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0xBF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 7) Overwrite prior flags: preset CF=1/OF=1, then AL=00 -> 00 (CF=0,OF=0)
    stc
    mov bh, 0x7F
    add bh, 0x01              ; OF=1
    mov al, 0x00
    mov ah, [pat0]
    sahf
    ror al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0


; ===================== 8-bit ROR (memory, count=1) =====================

; 8) [si]=7E -> 3F ; CF=0, OF=0 ; flags preserved (0s)
    lea si, [ror8_si_7e]
    mov ah, [pat0]
    sahf
    ror byte [si], 1
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x3F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 9) [di]=80 -> 40 ; CF=0, OF=1 ; flags preserved (1s)
    lea di, [ror8_di_80]
    mov ah, [pat1]
    sahf
    ror byte [di], 1
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x40
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 10) [bx]=01 -> 80 ; CF=1, OF=1 ; flags preserved (0s)
    lea bx, [ror8_bx_01]
    mov ah, [pat0]
    sahf
    ror byte [bx], 1
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x80
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 11) [si+disp]=FF -> FF ; CF=1, OF=0 ; flags preserved (1s)
    lea si, [base8sid]
    mov ah, [pat1]
    sahf
    ror byte [si + (ror8_sid_ff - base8sid)], 1
    SAVE_FLAGS
    mov al, [si + (ror8_sid_ff - base8sid)]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 12) [di+disp]=00 -> 00 ; CF=0, OF=0 ; flags preserved (0s)
    lea di, [base8dd]
    mov ah, [pat0]
    sahf
    ror byte [di + (ror8_did_00 - base8dd)], 1
    SAVE_FLAGS
    mov al, [di + (ror8_did_00 - base8dd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 13) [bx+disp]=02 -> 01 ; CF=0, OF=0 ; flags preserved (1s)
    lea bx, [base8bd]
    mov ah, [pat1]
    sahf
    ror byte [bx + (ror8_bxd_02 - base8bd)], 1
    SAVE_FLAGS
    mov al, [bx + (ror8_bxd_02 - base8bd)]
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 14) [bx+si+disp]=7F -> BF ; CF=1, OF=1 ; flags preserved (0s)
    lea bx, [base8]
    lea si, [index8]
    mov ah, [pat0]
    sahf
    ror byte [bx+si + (ror8_bxsi_7f - base8 - index8)], 1
    SAVE_FLAGS
    mov al, [bx+si + (ror8_bxsi_7f - base8 - index8)]
    ASSERT_BYTE 0xBF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 15) [bp] (ds:)=03 -> 81 ; CF=1, OF=1 ; flags preserved (1s)
    mov bp, ror8_bp_03
    mov ah, [pat1]
    sahf
    ror byte [ds:bp], 1
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0x81
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 16) [bp+disp] (ds:)=C0 -> 60 ; CF=0, OF=1 ; flags preserved (0s)
    lea bp, [base_bp_d]
    mov ah, [pat0]
    sahf
    ror byte [ds:bp + (ror8_bpd_c0 - base_bp_d)], 1
    SAVE_FLAGS
    mov al, [ds:bp + (ror8_bpd_c0 - base_bp_d)]
    ASSERT_BYTE 0x60
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 17) [bp+si+disp] (ds:)=04 -> 02 ; CF=0, OF=0 ; flags preserved (1s)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov ah, [pat1]
    sahf
    ror byte [ds:bp+si + (ror8_bpsi_04 - base_bp_si_A - base_bp_si_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+si + (ror8_bpsi_04 - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x02
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 18) [bp+di+disp] (ds:)=FE -> 7F ; CF=0, OF=1 ; flags preserved (0s)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov ah, [pat0]
    sahf
    ror byte [ds:bp+di + (ror8_bpdi_fe - base_bp_di_A - base_bp_di_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+di + (ror8_bpdi_fe - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0


; ===================== 8-bit ROR (register/memory, count=CL: opcode D2 /1) =====================

; 19) CL=2; AL=81 -> 60 ; CF=orig bit1=0 ; flags preserved (0s)
    mov cl, 2
    mov al, 0x81
    mov ah, [pat0]
    sahf
    ror al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x60
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 20) CL=2; AH=C1 -> 70 ; CF=orig bit1=0 ; flags preserved (1s)
    mov cl, 2
    mov ah, 0xC1
    mov al, [pat1]
    xchg al, ah
    sahf
    xchg al, ah
    ror ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x70
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 21) CL=3; BL=03 -> 60 ; CF=orig bit2=0 ; flags preserved (0s)
    mov cl, 3
    mov bl, 0x03
    mov ah, [pat0]
    sahf
    ror bl, cl
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0x60
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 22) CL=7; DL=03 -> 06 ; CF=orig bit6=0 ; flags preserved (1s)
    mov cl, 7
    mov dl, 0x03
    mov ah, [pat1]
    sahf
    ror dl, cl
    SAVE_FLAGS
    mov al, dl
    ASSERT_BYTE 0x06
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 23) CL=2; [di+disp]=81 -> 60 ; CF=orig bit1=0 ; flags preserved (0s)
    mov cl, 2
    lea di, [base8dd2]
    mov ah, [pat0]
    sahf
    ror byte [di + (ror8_did_81 - base8dd2)], cl
    SAVE_FLAGS
    mov al, [di + (ror8_did_81 - base8dd2)]
    ASSERT_BYTE 0x60
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 24) CL=3; [bx]=03 -> 60 ; CF=orig bit2=0 ; flags preserved (1s)
    mov cl, 3
    lea bx, [ror8_bx_03]
    mov ah, [pat1]
    sahf
    ror byte [bx], cl
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x60
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
    ror al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x12
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 26) CL=0 OF-preservation (register): make OF=1 via ADD; ROR must keep OF=1; AH unchanged
    mov bh, 0x7F
    add bh, 0x01            ; OF=1
    mov ah, 0x55
    mov cl, 0
    ror ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x55
    CHECK_OF 1

; 27) CL=0 no-op (memory): SAHF=pat0; memory unchanged; flags preserved
    mov ah, [pat0]
    sahf
    lea si, [ror8_si_55]
    mov cl, 0
    ror byte [si], cl
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x55
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0


; ===================== 16-bit ROR (register, count=1: opcode D1 /1) =====================

; 28) AX=0000 -> 0000 ; CF=0, OF=0 ; flags preserved (0s)
    mov ah, [pat0]
    sahf
    mov ax, 0x0000
    ror ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 29) AX=8000 -> 4000 ; CF=0, OF=1 ; flags preserved (1s)
    mov ah, [pat1]
    sahf
    mov ax, 0x8000
    ror ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x4000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 30) AX=0001 -> 8000 ; CF=1, OF=1 ; flags preserved (0s)
    mov ah, [pat0]
    sahf
    mov ax, 0x0001
    ror ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 31) BX=FFFF -> FFFF ; CF=1, OF=0 ; flags preserved (1s)
    mov bx, 0xFFFF
    mov ah, [pat1]
    sahf
    ror bx, 1
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 32) DX=7FFF -> BFFF ; CF=1, OF=1 ; flags preserved (0s)
    mov dx, 0x7FFF
    mov ah, [pat0]
    sahf
    ror dx, 1
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0xBFFF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0


; ===================== 16-bit ROR (memory, count=1) =====================

; 33) [si]=7FFF -> BFFF ; CF=1, OF=1 ; flags preserved (1s)
    lea si, [ror16_si_7fff]
    mov ah, [pat1]
    sahf
    ror word [si], 1
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0xBFFF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 34) [di]=8000 -> 4000 ; CF=0, OF=1 ; flags preserved (0s)
    lea di, [ror16_di_8000]
    mov ah, [pat0]
    sahf
    ror word [di], 1
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x4000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 35) [bx]=0001 -> 8000 ; CF=1, OF=1 ; flags preserved (1s)
    lea bx, [ror16_bx_0001]
    mov ah, [pat1]
    sahf
    ror word [bx], 1
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x8000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 36) [si+disp]=FFFF -> FFFF ; CF=1, OF=0 ; flags preserved (0s)
    lea si, [base16sid]
    mov ah, [pat0]
    sahf
    ror word [si + (ror16_sid_ffff - base16sid)], 1
    SAVE_FLAGS
    mov ax, [si + (ror16_sid_ffff - base16sid)]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 37) [di+disp]=0100 -> 0080 ; CF=0, OF=0 ; flags preserved (1s)
    lea di, [base16did]
    mov ah, [pat1]
    sahf
    ror word [di + (ror16_did_0100 - base16did)], 1
    SAVE_FLAGS
    mov ax, [di + (ror16_did_0100 - base16did)]
    ASSERT_AX 0x0080
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 38) [bx+disp]=3FFF -> 9FFF ; CF=1, OF=1 ; flags preserved (0s)
    lea bx, [base16bd]
    mov ah, [pat0]
    sahf
    ror word [bx + (ror16_bxd_3fff - base16bd)], 1
    SAVE_FLAGS
    mov ax, [bx + (ror16_bxd_3fff - base16bd)]
    ASSERT_AX 0x9FFF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 39) [bx+si+disp]=4000 -> 2000 ; CF=0, OF=0 ; flags preserved (1s)
    lea bx, [base16]
    lea si, [index16]
    mov ah, [pat1]
    sahf
    ror word [bx+si + (ror16_bxsi_4000 - base16 - index16)], 1
    SAVE_FLAGS
    mov ax, [bx+si + (ror16_bxsi_4000 - base16 - index16)]
    ASSERT_AX 0x2000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 40) [bp] (ds:)=FFFF -> FFFF ; CF=1, OF=0 ; flags preserved (0s)
    mov bp, ror16_bp_ffff
    mov ah, [pat0]
    sahf
    ror word [ds:bp], 1
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 41) [bp+disp] (ds:)=0002 -> 0001 ; CF=0, OF=0 ; flags preserved (1s)
    lea bp, [base_bp16_d]
    mov ah, [pat1]
    sahf
    ror word [ds:bp + (ror16_bpd_0002 - base_bp16_d)], 1
    SAVE_FLAGS
    mov ax, [ds:bp + (ror16_bpd_0002 - base_bp16_d)]
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 42) [bp+si+disp] (ds:)=2000 -> 1000 ; CF=0, OF=0 ; flags preserved (0s)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    mov ah, [pat0]
    sahf
    ror word [ds:bp+si + (ror16_bpsi_2000 - base_bp16_si_A - base_bp16_si_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+si + (ror16_bpsi_2000 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x1000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 43) [bp+di+disp] (ds:)=C000 -> 6000 ; CF=0, OF=1 ; flags preserved (1s)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov ah, [pat1]
    sahf
    ror word [ds:bp+di + (ror16_bpdi_c000 - base_bp16_di_A - base_bp16_di_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+di + (ror16_bpdi_c000 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x6000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1


; ===================== 16-bit ROR (register/memory, count=CL: opcode D3 /1) =====================

; 44) CL=2; AX=8001 -> 6000 ; CF=orig bit1=0 ; flags preserved (0s)
    mov cl, 2
    mov ah, [pat0]
    sahf
    mov ax, 0x8001
    ror ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x6000
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 45) CL=3; BX=0008 -> 0001 ; CF=orig bit2=0 ; flags preserved (1s)
    mov cl, 3
    mov bx, 0x0008
    mov ah, [pat1]
    sahf
    ror bx, cl
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 46) CL=4; DX=00F0 -> 000F ; CF=orig bit3=0 ; flags preserved (0s)
    mov cl, 4
    mov dx, 0x00F0
    mov ah, [pat0]
    sahf
    ror dx, cl
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x000F
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 47) CL=7; SI=8000 -> 0100 ; CF=orig bit6=0 ; flags preserved (1s)
    mov cl, 7
    mov si, 0x8000
    mov ah, [pat1]
    sahf
    ror si, cl
    SAVE_FLAGS
    mov ax, si
    ASSERT_AX 0x0100
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 48) CL=2; [di+disp]=0003 -> C000 ; CF=orig bit1=1 ; flags preserved (0s)
    mov cl, 2
    lea di, [base16did2]
    mov ah, [pat0]
    sahf
    ror word [di + (ror16_did_0003 - base16did2)], cl
    SAVE_FLAGS
    mov ax, [di + (ror16_did_0003 - base16did2)]
    ASSERT_AX 0xC000
    CHECK_CF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 49) CL=3; [bx]=F000 -> 1E00 ; CF=orig bit2=0 ; flags preserved (1s)
    mov cl, 3
    lea bx, [ror16_bx_f000]
    mov ah, [pat1]
    sahf
    ror word [bx], cl
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x1E00
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
    ror ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x1234
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 51) CL=0 OF-preservation (memory): make OF=1 via ADD; ROR [bx],0 preserves OF and value
    mov si, 0x7FFF
    add si, 0x0001          ; OF=1
    lea bx, [ror16_bx_aaaa]
    mov cl, 0
    ror word [bx], cl
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
ror8_si_7e:     db 0x7E
ror8_di_80:     db 0x80
ror8_bx_01:     db 0x01

base8sid:
ror8_sid_ff:    db 0xFF

base8dd:
ror8_did_00:    db 0x00

base8bd:
ror8_bxd_02:    db 0x02

base8:
index8:
ror8_bxsi_7f:   db 0x7F

ror8_bp_03:     db 0x03

base_bp_d:
ror8_bpd_c0:    db 0xC0

base_bp_si_A:
base_bp_si_B:
ror8_bpsi_04:   db 0x04

base_bp_di_A:
base_bp_di_B:
ror8_bpdi_fe:   db 0xFE

; ---- 8-bit memory (count=CL) ----
base8dd2:
ror8_did_81:    db 0x81

ror8_bx_03:     db 0x03

ror8_si_55:     db 0x55

; ---- 16-bit memory (count=1) ----
ror16_si_7fff:    dw 0x7FFF
ror16_di_8000:    dw 0x8000
ror16_bx_0001:    dw 0x0001

base16sid:
ror16_sid_ffff:   dw 0xFFFF

base16did:
ror16_did_0100:   dw 0x0100

base16bd:
ror16_bxd_3fff:   dw 0x3FFF

base16:
index16:
ror16_bxsi_4000:  dw 0x4000

ror16_bp_ffff:    dw 0xFFFF

base_bp16_d:
ror16_bpd_0002:   dw 0x0002

base_bp16_si_A:
base_bp16_si_B:
ror16_bpsi_2000:  dw 0x2000

base_bp16_di_A:
base_bp16_di_B:
ror16_bpdi_c000:  dw 0xC000

; ---- 16-bit memory (count=CL) ----
base16did2:
ror16_did_0003:   dw 0x0003

ror16_bx_f000:    dw 0xF000

ror16_bx_aaaa:    dw 0xAAAA

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
