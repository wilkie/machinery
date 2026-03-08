; rcl.asm — thorough RCL tests (r/m8, r/m16) for count=1 and count=CL (including CL=0 no-op)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   count=1: CF=old MSB; OF = (new MSB) XOR CF = old bit(n-2) XOR old MSB
;            SF/ZF/PF/AF preserved (verified via SAHF)
;   count>=2: CF=last bit out (through the CF chain); OF undefined (don't check); SF/ZF/PF/AF preserved
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

; ===================== 8-bit RCL (register, count=1: opcode D0 /2) =====================

; 1) AL=00, CF_in=0 -> 00 ; CF=0, OF=0 ; flags preserved (0s)
    mov al, 0x00
    mov ah, [pat0]
    sahf
    rcl al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 2) AL=00, CF_in=1 -> 01 ; CF=0, OF=0 ; flags preserved (1s)
    mov al, 0x00
    mov ah, [pat1]
    sahf
    rcl al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 3) AL=80, CF_in=0 -> 00 ; CF=1, OF=1 ; flags preserved (0s)
    mov al, 0x80
    mov ah, [pat0]
    sahf
    rcl al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 4) AL=40, CF_in=0 -> 80 ; CF=0, OF=1 ; flags preserved (0s)
    mov al, 0x40
    mov ah, [pat0]
    sahf
    rcl al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 1
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
    rcl ah, 1
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 6) BL=7F, CF_in=1 -> FF ; CF=0, OF=1 ; flags preserved (1s)
    mov bl, 0x7F
    mov ah, [pat1]
    sahf
    rcl bl, 1
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 7) Overwrite prior flags: set CF=1, make OF=1; RCL AL=00 -> 01 (CF=0,OF=0)
    mov bh, 0x7F
    add bh, 0x01              ; OF=1
    mov al, 0x00
    stc
    rcl al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_OF 0


; ===================== 8-bit RCL (memory, count=1) =====================

; 8) [si]=7E, CF_in=0 -> FC ; CF=0, OF=1 ; flags preserved (0s)
    lea si, [rcl8_si_7e]
    mov ah, [pat0]
    sahf
    rcl byte [si], 1
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xFC
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 9) [di]=80, CF_in=1 -> 01 ; CF=1, OF=1 ; flags preserved (1s)
    lea di, [rcl8_di_80]
    mov ah, [pat1]
    sahf
    rcl byte [di], 1
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 10) [bx]=01, CF_in=0 -> 02 ; CF=0, OF=0 ; flags preserved (0s)
    lea bx, [rcl8_bx_01]
    mov ah, [pat0]
    sahf
    rcl byte [bx], 1
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x02
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 11) [si+disp]=FF, CF_in=1 -> FF ; CF=1, OF=0 ; flags preserved (1s)
    lea si, [base8sid]
    mov ah, [pat1]
    sahf
    rcl byte [si + (rcl8_sid_ff - base8sid)], 1
    SAVE_FLAGS
    mov al, [si + (rcl8_sid_ff - base8sid)]
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
    rcl byte [di + (rcl8_did_00 - base8dd)], 1
    SAVE_FLAGS
    mov al, [di + (rcl8_did_00 - base8dd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 13) [bx+disp]=3F, CF_in=1 -> 7F ; CF=0, OF=0 ; flags preserved (1s)
    lea bx, [base8bd]
    mov ah, [pat1]
    sahf
    rcl byte [bx + (rcl8_bxd_3f - base8bd)], 1
    SAVE_FLAGS
    mov al, [bx + (rcl8_bxd_3f - base8bd)]
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 14) [bx+si+disp]=40, CF_in=0 -> 80 ; CF=0, OF=1 ; flags preserved (0s)
    lea bx, [base8]
    lea si, [index8]
    mov ah, [pat0]
    sahf
    rcl byte [bx+si + (rcl8_bxsi_40 - base8 - index8)], 1
    SAVE_FLAGS
    mov al, [bx+si + (rcl8_bxsi_40 - base8 - index8)]
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 15) [bp] (ds:)=FF, CF_in=1 -> FF ; CF=1, OF=0 ; flags preserved (1s)
    mov bp, rcl8_bp_ff
    mov ah, [pat1]
    sahf
    rcl byte [ds:bp], 1
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 16) [bp+disp] (ds:)=20, CF_in=0 -> 40 ; CF=0, OF=0 ; flags preserved (0s)
    lea bp, [base_bp_d]
    mov ah, [pat0]
    sahf
    rcl byte [ds:bp + (rcl8_bpd_20 - base_bp_d)], 1
    SAVE_FLAGS
    mov al, [ds:bp + (rcl8_bpd_20 - base_bp_d)]
    ASSERT_BYTE 0x40
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 17) [bp+si+disp] (ds:)=02, CF_in=1 -> 05 ; CF=0, OF=0 ; flags preserved (1s)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov ah, [pat1]
    sahf
    rcl byte [ds:bp+si + (rcl8_bpsi_02 - base_bp_si_A - base_bp_si_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+si + (rcl8_bpsi_02 - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x05
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 18) [bp+di+disp] (ds:)=C0, CF_in=0 -> 80 ; CF=1, OF=0 ; flags preserved (0s)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov ah, [pat0]
    sahf
    rcl byte [ds:bp+di + (rcl8_bpdi_c0 - base_bp_di_A - base_bp_di_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+di + (rcl8_bpdi_c0 - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x80
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0


; ===================== 8-bit RCL (register/memory, count=CL: opcode D2 /2) =====================

; 19) CL=2; AL=81, CF_in=0 -> 05 ; CF=0 ; flags preserved (0s)
    mov cl, 2
    mov al, 0x81
    mov ah, [pat0]
    sahf
    rcl al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x05
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 20) CL=2; AH=C1, CF_in=1 -> 07 ; CF=1 ; flags preserved (1s)
    mov cl, 2
    mov ah, 0xC1
    mov al, [pat1]
    xchg al, ah
    sahf
    xchg al, ah
    rcl ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x07
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 21) CL=3; BL=03, CF_in=0 -> 18 ; CF=0 ; flags preserved (0s)
    mov cl, 3
    mov bl, 0x03
    mov ah, [pat0]
    sahf
    rcl bl, cl
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0x18
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 22) CL=7; DL=03, CF_in=1 -> C0 ; CF=1 ; flags preserved (1s)
    mov cl, 7
    mov dl, 0x03
    mov ah, [pat1]
    sahf
    rcl dl, cl
    SAVE_FLAGS
    mov al, dl
    ASSERT_BYTE 0xC0
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 23) CL=2; [di+disp]=81, CF_in=0 -> 05 ; CF=0 ; flags preserved (0s)
    mov cl, 2
    lea di, [base8dd2]
    mov ah, [pat0]
    sahf
    rcl byte [di + (rcl8_did_81 - base8dd2)], cl
    SAVE_FLAGS
    mov al, [di + (rcl8_did_81 - base8dd2)]
    ASSERT_BYTE 0x05
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 24) CL=3; [bx]=03, CF_in=1 -> 1C ; CF=0 ; flags preserved (1s)
    mov cl, 3
    lea bx, [rcl8_bx_03]
    mov ah, [pat1]
    sahf
    rcl byte [bx], cl
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x1C
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
    rcl al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x12
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 26) CL=0 OF-preservation (register): make OF=1 via ADD; RCL must keep OF=1; AH unchanged
    mov bh, 0x7F
    add bh, 0x01            ; OF=1
    mov ah, 0x55
    mov cl, 0
    rcl ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x55
    CHECK_OF 1

; 27) CL=0 no-op (memory): SAHF=pat0; memory unchanged; flags preserved
    mov ah, [pat0]
    sahf
    lea si, [rcl8_si_55]
    mov cl, 0
    rcl byte [si], cl
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x55
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0


; ===================== 16-bit RCL (register, count=1: opcode D1 /2) =====================

; 28) AX=0000, CF_in=0 -> 0000 ; CF=0, OF=0 ; flags preserved (0s)
    mov ax, 0x0000
    mov ah, [pat0]
    sahf
    rcl ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 29) AX=8000, CF_in=1 -> 0001 ; CF=1, OF=1 ; flags preserved (1s)
    mov ah, [pat1]
    sahf
    mov ax, 0x8000
    rcl ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 30) AX=4000, CF_in=0 -> 8000 ; CF=0, OF=1 ; flags preserved (0s)
    mov ah, [pat0]
    sahf
    mov ax, 0x4000
    rcl ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 31) BX=FFFF, CF_in=1 -> FFFF ; CF=1, OF=0 ; flags preserved (1s)
    mov bx, 0xFFFF
    mov ah, [pat1]
    sahf
    rcl bx, 1
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 32) DX=7FFF, CF_in=0 -> FFFE ; CF=0, OF=1 ; flags preserved (0s)
    mov dx, 0x7FFF
    mov ah, [pat0]
    sahf
    rcl dx, 1
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0xFFFE
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0


; ===================== 16-bit RCL (memory, count=1) =====================

; 33) [si]=7FFF, CF_in=1 -> FFFF ; CF=0, OF=1 ; flags preserved (1s)
    lea si, [rcl16_si_7fff]
    mov ah, [pat1]
    sahf
    rcl word [si], 1
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 34) [di]=8000, CF_in=0 -> 0000 ; CF=1, OF=1 ; flags preserved (0s)
    lea di, [rcl16_di_8000]
    mov ah, [pat0]
    sahf
    rcl word [di], 1
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 35) [bx]=0001, CF_in=1 -> 0003 ; CF=0, OF=0 ; flags preserved (1s)
    lea bx, [rcl16_bx_0001]
    mov ah, [pat1]
    sahf
    rcl word [bx], 1
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0003
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 36) [si+disp]=FFFF, CF_in=0 -> FFFE ; CF=1, OF=0 ; flags preserved (0s)
    lea si, [base16sid]
    mov ah, [pat0]
    sahf
    rcl word [si + (rcl16_sid_ffff - base16sid)], 1
    SAVE_FLAGS
    mov ax, [si + (rcl16_sid_ffff - base16sid)]
    ASSERT_AX 0xFFFE
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 37) [di+disp]=0100, CF_in=1 -> 0201 ; CF=0, OF=0 ; flags preserved (1s)
    lea di, [base16did]
    mov ah, [pat1]
    sahf
    rcl word [di + (rcl16_did_0100 - base16did)], 1
    SAVE_FLAGS
    mov ax, [di + (rcl16_did_0100 - base16did)]
    ASSERT_AX 0x0201
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 38) [bx+disp]=3FFF, CF_in=0 -> 7FFE ; CF=0, OF=0 ; flags preserved (0s)
    lea bx, [base16bd]
    mov ah, [pat0]
    sahf
    rcl word [bx + (rcl16_bxd_3fff - base16bd)], 1
    SAVE_FLAGS
    mov ax, [bx + (rcl16_bxd_3fff - base16bd)]
    ASSERT_AX 0x7FFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 39) [bx+si+disp]=4000, CF_in=1 -> 8001 ; CF=0, OF=1 ; flags preserved (1s)
    lea bx, [base16]
    lea si, [index16]
    mov ah, [pat1]
    sahf
    rcl word [bx+si + (rcl16_bxsi_4000 - base16 - index16)], 1
    SAVE_FLAGS
    mov ax, [bx+si + (rcl16_bxsi_4000 - base16 - index16)]
    ASSERT_AX 0x8001
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 40) [bp] (ds:)=FFFF, CF_in=0 -> FFFE ; CF=1, OF=0 ; flags preserved (0s)
    mov bp, rcl16_bp_ffff
    mov ah, [pat0]
    sahf
    rcl word [ds:bp], 1
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0xFFFE
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 41) [bp+disp] (ds:)=0002, CF_in=1 -> 0005 ; CF=0, OF=0 ; flags preserved (1s)
    lea bp, [base_bp16_d]
    mov ah, [pat1]
    sahf
    rcl word [ds:bp + (rcl16_bpd_0002 - base_bp16_d)], 1
    SAVE_FLAGS
    mov ax, [ds:bp + (rcl16_bpd_0002 - base_bp16_d)]
    ASSERT_AX 0x0005
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 42) [bp+si+disp] (ds:)=2000, CF_in=0 -> 4000 ; CF=0, OF=0 ; flags preserved (0s)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    mov ah, [pat0]
    sahf
    rcl word [ds:bp+si + (rcl16_bpsi_2000 - base_bp16_si_A - base_bp16_si_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+si + (rcl16_bpsi_2000 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x4000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 43) [bp+di+disp] (ds:)=C000, CF_in=1 -> 8001 ; CF=1, OF=0 ; flags preserved (1s)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov ah, [pat1]
    sahf
    rcl word [ds:bp+di + (rcl16_bpdi_c000 - base_bp16_di_A - base_bp16_di_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+di + (rcl16_bpdi_c000 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x8001
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1


; ===================== 16-bit RCL (register/memory, count=CL: opcode D3 /2) =====================

; 44) CL=2; AX=8001, CF_in=0 -> 0005 ; CF=0 ; flags preserved (0s)
    mov cl, 2
    mov ah, [pat0]
    sahf
    mov ax, 0x8001
    rcl ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x0005
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 45) CL=3; BX=1000, CF_in=1 -> 8004 ; CF=0 ; flags preserved (1s)
    mov cl, 3
    mov bx, 0x1000
    mov ah, [pat1]
    sahf
    rcl bx, cl
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x8004
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 46) CL=4; DX=00F0, CF_in=0 -> 0F00 ; CF=0 ; flags preserved (0s)
    mov cl, 4
    mov dx, 0x00F0
    mov ah, [pat0]
    sahf
    rcl dx, cl
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x0F00
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 47) CL=7; SI=0003, CF_in=1 -> 01C0 ; CF=0 ; flags preserved (1s)
    mov cl, 7
    mov si, 0x0003
    mov ah, [pat1]
    sahf
    rcl si, cl
    SAVE_FLAGS
    mov ax, si
    ASSERT_AX 0x01C0
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 48) CL=2; [di+disp]=C000, CF_in=0 -> 0001 ; CF=1 ; flags preserved (0s)
    mov cl, 2
    lea di, [base16did2]
    mov ah, [pat0]
    sahf
    rcl word [di + (rcl16_did_c000 - base16did2)], cl
    SAVE_FLAGS
    mov ax, [di + (rcl16_did_c000 - base16did2)]
    ASSERT_AX 0x0001
    CHECK_CF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 49) CL=3; [bx]=0003, CF_in=1 -> 001C ; CF=0 ; flags preserved (1s)
    mov cl, 3
    lea bx, [rcl16_bx_0003]
    mov ah, [pat1]
    sahf
    rcl word [bx], cl
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x001C
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
    rcl ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x1234
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 51) CL=0 OF-preservation (memory): make OF=1 via ADD; RCL [bx],0 preserves OF and value
    mov si, 0x7FFF
    add si, 0x0001          ; OF=1
    lea bx, [rcl16_bx_aaaa]
    mov cl, 0
    rcl word [bx], cl
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0xAAAA
    CHECK_OF 1

; ===================== RCL corner-case counts (add before exit) =====================

; 52) 8-bit: CL=9 (full 9-bit cycle) — value & CF preserved (CF_in=0)
    mov ah, [pat0]          ; CF_in=0; SF/ZF/AF/PF=0
    sahf
    mov cl, 9
    mov al, 0x5A
    rcl al, cl
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
    lea bx, [rcl8_ex_a5]
    rcl byte [bx], cl
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0xA5
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 54) 8-bit: CL=10 (≡ 1 mod 9) — behaves like 1-step; OF not checked (count>=2)
    mov ah, [pat0]          ; CF_in=0
    sahf
    mov cl, 10
    mov al, 0x01
    rcl al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x02        ; 0x01 -> 0x02, CF_out=0
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 55) 8-bit mem: CL=10 (≡ 1 mod 9), CF_in=1
    mov ah, [pat1]
    sahf
    mov cl, 10
    lea si, [rcl8_ex_00]
    rcl byte [si], cl
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x01        ; 0x00 -> 0x01, CF_out=0
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
    rcl ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x0002        ; 0x0001 -> 0x0002, CF_out=0
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 57) 16-bit mem: CL=17 (full 17-bit cycle), CF_in=1 — value & CF preserved
    mov ah, [pat1]
    sahf
    mov cl, 17
    lea di, [rcl16_ex_beef]
    rcl word [di], cl
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0xBEEF
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 58) 16-bit mem: CL=31 (≡ 14 mod 17) with all-ones + CF=1 → unchanged
    mov ah, [pat1]
    sahf
    mov cl, 31
    lea si, [rcl16_ex_ffff]
    rcl word [si], cl
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 59) 16-bit: CL=33 (masked to 1 by count mask) — like 1-step; OF not checked
    mov ah, [pat1]          ; CF_in=1
    sahf
    mov cl, 33
    mov dx, 0x8000
    rcl dx, cl
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x0001        ; 0x8000 -> 0x0001, CF_out=1
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 60) 8-bit mem: CL=25 (≡ 7 mod 9) with all-ones + CF=1 → unchanged
    mov ah, [pat1]
    sahf
    mov cl, 25
    lea bx, [rcl8_ex_ff]
    rcl byte [bx], cl
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
    rcl si, cl
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
rcl8_si_7e:     db 0x7E
rcl8_di_80:     db 0x80
rcl8_bx_01:     db 0x01

base8sid:
rcl8_sid_ff:    db 0xFF

base8dd:
rcl8_did_00:    db 0x00

base8bd:
rcl8_bxd_3f:    db 0x3F

base8:
index8:
rcl8_bxsi_40:   db 0x40

rcl8_bp_ff:     db 0xFF

base_bp_d:
rcl8_bpd_20:    db 0x20

base_bp_si_A:
base_bp_si_B:
rcl8_bpsi_02:   db 0x02

base_bp_di_A:
base_bp_di_B:
rcl8_bpdi_c0:   db 0xC0

; ---- 8-bit memory (count=CL) ----
base8dd2:
rcl8_did_81:    db 0x81

rcl8_bx_03:     db 0x03

rcl8_si_55:     db 0x55

; ---- 16-bit memory (count=1) ----
rcl16_si_7fff:    dw 0x7FFF
rcl16_di_8000:    dw 0x8000
rcl16_bx_0001:    dw 0x0001

base16sid:
rcl16_sid_ffff:   dw 0xFFFF

base16did:
rcl16_did_0100:   dw 0x0100

base16bd:
rcl16_bxd_3fff:   dw 0x3FFF

base16:
index16:
rcl16_bxsi_4000:  dw 0x4000

rcl16_bp_ffff:    dw 0xFFFF

base_bp16_d:
rcl16_bpd_0002:   dw 0x0002

base_bp16_si_A:
base_bp16_si_B:
rcl16_bpsi_2000:  dw 0x2000

base_bp16_di_A:
base_bp16_di_B:
rcl16_bpdi_c000:  dw 0xC000

; ---- 16-bit memory (count=CL) ----
base16did2:
rcl16_did_c000:   dw 0xC000

rcl16_bx_0003:    dw 0x0003

rcl16_bx_aaaa:    dw 0xAAAA

; ---- Extra data for RCL corner-case tests ----
rcl8_ex_a5:      db 0xA5
rcl8_ex_00:      db 0x00
rcl8_ex_ff:      db 0xFF
rcl16_ex_beef:   dw 0xBEEF
rcl16_ex_ffff:   dw 0xFFFF

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
