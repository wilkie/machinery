; rol.asm — thorough ROL tests (r/m8, r/m16) for count=1 and count=CL (including CL=0 no-op)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   count=1: CF=old MSB; OF = (new MSB) XOR CF; SF/ZF/PF/AF preserved
;   count>=2: CF=last bit out; OF undefined (don't check); SF/ZF/PF/AF preserved
;   CL=0: operand & ALL flags preserved (verify with SAHF + a separate OF-preservation check)

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

; ===================== 8-bit ROL (register, count=1: opcode D0 /0) =====================

; 1) AL=00 -> 00 ; CF=0, OF=0 ; SF/ZF/PF/AF preserved (all 0s)
    mov al, 0x00
    mov ah, [pat0]
    sahf
    rol al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 2) AL=01 -> 02 ; CF=0, OF=0 ; SF/ZF/PF/AF preserved (all 1s)
    mov al, 0x01
    mov ah, [pat1]
    sahf
    rol al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x02
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 3) AL=80 -> 01 ; CF=1, OF=1 ; flags preserved (0s)
    mov al, 0x80
    mov ah, [pat0]
    sahf
    rol al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 4) AL=40 -> 80 ; CF=0, OF=1 ; flags preserved (1s)
    mov al, 0x40
    mov ah, [pat1]
    sahf
    rol al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 1
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
    rol ah, 1
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 6) BL=7F -> FE ; CF=0, OF=1 ; flags preserved (1s)
    mov bl, 0x7F
    mov ah, [pat1]
    sahf
    rol bl, 1
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0xFE
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 7) Overwrite prior flags: preset CF=1/OF=1 then ROL must set CF/OF to 0 for AL=00
    stc
    mov bh, 0x7F
    add bh, 0x01              ; OF=1
    mov al, 0x00
    mov ah, [pat0]
    sahf
    rol al, 1
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0


; ===================== 8-bit ROL (memory, count=1) =====================

; 8) [si]=7E -> FC ; CF=0, OF=1 ; flags preserved (0s)
    lea si, [rol8_si_7e]
    mov ah, [pat0]
    sahf
    rol byte [si], 1
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xFC
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 9) [di]=80 -> 01 ; CF=1, OF=1 ; flags preserved (1s)
    lea di, [rol8_di_80]
    mov ah, [pat1]
    sahf
    rol byte [di], 1
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x01
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 10) [bx]=01 -> 02 ; CF=0, OF=0 ; flags preserved (0s)
    lea bx, [rol8_bx_01]
    mov ah, [pat0]
    sahf
    rol byte [bx], 1
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x02
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 11) [si+disp]=FF -> FF ; CF=1, OF=0 ; flags preserved (1s)
    lea si, [base8sid]
    mov ah, [pat1]
    sahf
    rol byte [si + (rol8_sid_ff - base8sid)], 1
    SAVE_FLAGS
    mov al, [si + (rol8_sid_ff - base8sid)]
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
    rol byte [di + (rol8_did_00 - base8dd)], 1
    SAVE_FLAGS
    mov al, [di + (rol8_did_00 - base8dd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 13) [bx+disp]=3F -> 7E ; CF=0, OF=0 ; flags preserved (1s)
    lea bx, [base8bd]
    mov ah, [pat1]
    sahf
    rol byte [bx + (rol8_bxd_3f - base8bd)], 1
    SAVE_FLAGS
    mov al, [bx + (rol8_bxd_3f - base8bd)]
    ASSERT_BYTE 0x7E
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 14) [bx+si+disp]=40 -> 80 ; CF=0, OF=1 ; flags preserved (0s)
    lea bx, [base8]
    lea si, [index8]
    mov ah, [pat0]
    sahf
    rol byte [bx+si + (rol8_bxsi_40 - base8 - index8)], 1
    SAVE_FLAGS
    mov al, [bx+si + (rol8_bxsi_40 - base8 - index8)]
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 15) [bp] (ds:)=FF -> FF ; CF=1, OF=0 ; flags preserved (1s)
    mov bp, rol8_bp_ff
    mov ah, [pat1]
    sahf
    rol byte [ds:bp], 1
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 16) [bp+disp] (ds:)=20 -> 40 ; CF=0, OF=0 ; flags preserved (0s)
    lea bp, [base_bp_d]
    mov ah, [pat0]
    sahf
    rol byte [ds:bp + (rol8_bpd_20 - base_bp_d)], 1
    SAVE_FLAGS
    mov al, [ds:bp + (rol8_bpd_20 - base_bp_d)]
    ASSERT_BYTE 0x40
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 17) [bp+si+disp] (ds:)=02 -> 04 ; CF=0, OF=0 ; flags preserved (1s)
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov ah, [pat1]
    sahf
    rol byte [ds:bp+si + (rol8_bpsi_02 - base_bp_si_A - base_bp_si_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+si + (rol8_bpsi_02 - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x04
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 18) [bp+di+disp] (ds:)=C0 -> 81 ; CF=1, OF=0 ; flags preserved (0s)
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov ah, [pat0]
    sahf
    rol byte [ds:bp+di + (rol8_bpdi_c0 - base_bp_di_A - base_bp_di_B)], 1
    SAVE_FLAGS
    mov al, [ds:bp+di + (rol8_bpdi_c0 - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x81
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0


; ===================== 8-bit ROL (register/memory, count=CL: opcode D2 /0) =====================

; 19) CL=2; AL=81 -> 06 ; CF=orig bit6=0 ; flags preserved (0s)
    mov cl, 2
    mov al, 0x81
    mov ah, [pat0]
    sahf
    rol al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x06
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 20) CL=2; AH=C1 -> 07 ; CF=orig bit6=1 ; flags preserved (1s)
    mov cl, 2
    mov ah, 0xC1
    mov al, [pat1]
    xchg al, ah
    sahf
    xchg al, ah
    rol ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x07
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 21) CL=3; BL=03 -> 18 ; CF=orig bit5=0 ; flags preserved (0s)
    mov cl, 3
    mov bl, 0x03
    mov ah, [pat0]
    sahf
    rol bl, cl
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0x18
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 22) CL=7; DL=03 -> 81 ; CF=orig bit1=1 ; flags preserved (1s)
    mov cl, 7
    mov dl, 0x03
    mov ah, [pat1]
    sahf
    rol dl, cl
    SAVE_FLAGS
    mov al, dl
    ASSERT_BYTE 0x81
    CHECK_CF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 23) CL=2; [di+disp]=81 -> 06 ; CF=orig bit6=0 ; flags preserved (0s)
    mov cl, 2
    lea di, [base8dd2]
    mov ah, [pat0]
    sahf
    rol byte [di + (rol8_did_81 - base8dd2)], cl
    SAVE_FLAGS
    mov al, [di + (rol8_did_81 - base8dd2)]
    ASSERT_BYTE 0x06
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 24) CL=3; [bx]=03 -> 18 ; CF=orig bit5=0 ; flags preserved (1s)
    mov cl, 3
    lea bx, [rol8_bx_03]
    mov ah, [pat1]
    sahf
    rol byte [bx], cl
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x18
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
    rol al, cl
    SAVE_FLAGS
    ASSERT_BYTE 0x12
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    ; OF preserved — separate test follows

; 26) CL=0 OF-preservation (register): make OF=1 via ADD; ROL must keep OF=1; AH unchanged
    mov bh, 0x7F
    add bh, 0x01            ; OF=1
    mov ah, 0x55
    mov cl, 0
    rol ah, cl
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x55
    CHECK_OF 1

; 27) CL=0 no-op (memory): SAHF=pat0; mem unchanged; flags preserved
    mov ah, [pat0]
    sahf
    lea si, [rol8_si_55]
    mov cl, 0
    rol byte [si], cl
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0x55
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0


; ===================== 16-bit ROL (register, count=1: opcode D1 /0) =====================

; 28) AX=0000 -> 0000 ; CF=0, OF=0 ; flags preserved (0s)
    mov ax, 0x0000
    mov ah, [pat0]
    sahf
    rol ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 29) AX=8000 -> 0001 ; CF=1, OF=1 ; flags preserved (1s)
    mov ah, [pat1]
    sahf
    mov ax, 0x8000
    rol ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 30) AX=4000 -> 8000 ; CF=0, OF=1 ; flags preserved (0s)
    mov ah, [pat0]
    sahf
    mov ax, 0x4000
    rol ax, 1
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 31) BX=FFFF -> FFFF ; CF=1, OF=0 ; flags preserved (1s)
    mov bx, 0xFFFF
    mov ah, [pat1]
    sahf
    rol bx, 1
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 32) DX=7FFF -> FFFE ; CF=0, OF=1 ; flags preserved (0s)
    mov dx, 0x7FFF
    mov ah, [pat0]
    sahf
    rol dx, 1
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0xFFFE
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0


; ===================== 16-bit ROL (memory, count=1) =====================

; 33) [si]=7FFF -> FFFE ; CF=0, OF=1 ; flags preserved (1s)
    lea si, [rol16_si_7fff]
    mov ah, [pat1]
    sahf
    rol word [si], 1
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0xFFFE
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 34) [di]=8000 -> 0001 ; CF=1, OF=1 ; flags preserved (0s)
    lea di, [rol16_di_8000]
    mov ah, [pat0]
    sahf
    rol word [di], 1
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 35) [bx]=0001 -> 0002 ; CF=0, OF=0 ; flags preserved (1s)
    lea bx, [rol16_bx_0001]
    mov ah, [pat1]
    sahf
    rol word [bx], 1
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0002
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 36) [si+disp]=FFFF -> FFFF ; CF=1, OF=0 ; flags preserved (0s)
    lea si, [base16sid]
    mov ah, [pat0]
    sahf
    rol word [si + (rol16_sid_ffff - base16sid)], 1
    SAVE_FLAGS
    mov ax, [si + (rol16_sid_ffff - base16sid)]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 37) [di+disp]=0100 -> 0200 ; CF=0, OF=0 ; flags preserved (1s)
    lea di, [base16did]
    mov ah, [pat1]
    sahf
    rol word [di + (rol16_did_0100 - base16did)], 1
    SAVE_FLAGS
    mov ax, [di + (rol16_did_0100 - base16did)]
    ASSERT_AX 0x0200
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 38) [bx+disp]=3FFF -> 7FFE ; CF=0, OF=0 ; flags preserved (0s)
    lea bx, [base16bd]
    mov ah, [pat0]
    sahf
    rol word [bx + (rol16_bxd_3fff - base16bd)], 1
    SAVE_FLAGS
    mov ax, [bx + (rol16_bxd_3fff - base16bd)]
    ASSERT_AX 0x7FFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 39) [bx+si+disp]=4000 -> 8000 ; CF=0, OF=1 ; flags preserved (1s)
    lea bx, [base16]
    lea si, [index16]
    mov ah, [pat1]
    sahf
    rol word [bx+si + (rol16_bxsi_4000 - base16 - index16)], 1
    SAVE_FLAGS
    mov ax, [bx+si + (rol16_bxsi_4000 - base16 - index16)]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 40) [bp] (ds:)=FFFF -> FFFF ; CF=1, OF=0 ; flags preserved (0s)
    mov bp, rol16_bp_ffff
    mov ah, [pat0]
    sahf
    rol word [ds:bp], 1
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0xFFFF
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 41) [bp+disp] (ds:)=0002 -> 0004 ; CF=0, OF=0 ; flags preserved (1s)
    lea bp, [base_bp16_d]
    mov ah, [pat1]
    sahf
    rol word [ds:bp + (rol16_bpd_0002 - base_bp16_d)], 1
    SAVE_FLAGS
    mov ax, [ds:bp + (rol16_bpd_0002 - base_bp16_d)]
    ASSERT_AX 0x0004
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 42) [bp+si+disp] (ds:)=2000 -> 4000 ; CF=0, OF=0 ; flags preserved (0s)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    mov ah, [pat0]
    sahf
    rol word [ds:bp+si + (rol16_bpsi_2000 - base_bp16_si_A - base_bp16_si_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+si + (rol16_bpsi_2000 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x4000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 43) [bp+di+disp] (ds:)=C000 -> 8001 ; CF=1, OF=0 ; flags preserved (1s)
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov ah, [pat1]
    sahf
    rol word [ds:bp+di + (rol16_bpdi_c000 - base_bp16_di_A - base_bp16_di_B)], 1
    SAVE_FLAGS
    mov ax, [ds:bp+di + (rol16_bpdi_c000 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x8001
    CHECK_CF 1
    CHECK_OF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1


; ===================== 16-bit ROL (register/memory, count=CL: opcode D3 /0) =====================

; 44) CL=2; AX=8001 -> 0006 ; CF=orig bit14=0 ; flags preserved (0s)
    mov cl, 2
    mov ah, [pat0]
    sahf
    mov ax, 0x8001
    rol ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x0006
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 45) CL=3; BX=1000 -> 8000 ; CF=orig bit13=0 ; flags preserved (1s)
    mov cl, 3
    mov bx, 0x1000
    mov ah, [pat1]
    sahf
    rol bx, cl
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 46) CL=4; DX=0008 -> 0080 ; CF=orig bit12=0 ; flags preserved (0s)
    mov cl, 4
    mov dx, 0x0008
    mov ah, [pat0]
    sahf
    rol dx, cl
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x0080
    CHECK_CF 0
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 47) CL=7; SI=0003 -> 0180 ; CF=orig bit9=0 ; flags preserved (1s)
    mov cl, 7
    mov si, 0x0003
    mov ah, [pat1]
    sahf
    rol si, cl
    SAVE_FLAGS
    mov ax, si
    ASSERT_AX 0x0180
    CHECK_CF 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1

; 48) CL=2; [di+disp]=C000 -> 0003 ; CF=orig bit14=1 ; flags preserved (0s)
    mov cl, 2
    lea di, [base16did2]
    mov ah, [pat0]
    sahf
    rol word [di + (rol16_did_c000 - base16did2)], cl
    SAVE_FLAGS
    mov ax, [di + (rol16_did_c000 - base16did2)]
    ASSERT_AX 0x0003
    CHECK_CF 1
    CHECK_SF 0
    CHECK_ZF 0
    CHECK_PF 0
    CHECK_AF 0

; 49) CL=3; [bx]=0003 -> 0018 ; CF=orig bit13=0 ; flags preserved (1s)
    mov cl, 3
    lea bx, [rol16_bx_0003]
    mov ah, [pat1]
    sahf
    rol word [bx], cl
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x0018
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
    rol ax, cl
    SAVE_FLAGS
    ASSERT_AX 0x1234
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 51) CL=0 OF-preservation (memory): make OF=1 via ADD; ROL [bx],0 preserves OF and value
    mov si, 0x7FFF
    add si, 0x0001          ; OF=1
    lea bx, [rol16_bx_aaaa]
    mov cl, 0
    rol word [bx], cl
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
rol8_si_7e:     db 0x7E
rol8_di_80:     db 0x80
rol8_bx_01:     db 0x01

base8sid:
rol8_sid_ff:    db 0xFF

base8dd:
rol8_did_00:    db 0x00

base8bd:
rol8_bxd_3f:    db 0x3F

base8:
index8:
rol8_bxsi_40:   db 0x40

rol8_bp_ff:     db 0xFF

base_bp_d:
rol8_bpd_20:    db 0x20

base_bp_si_A:
base_bp_si_B:
rol8_bpsi_02:   db 0x02

base_bp_di_A:
base_bp_di_B:
rol8_bpdi_c0:   db 0xC0

; ---- 8-bit memory (count=CL) ----
base8dd2:
rol8_did_81:    db 0x81

rol8_bx_03:     db 0x03

rol8_si_55:     db 0x55

; ---- 16-bit memory (count=1) ----
rol16_si_7fff:    dw 0x7FFF
rol16_di_8000:    dw 0x8000
rol16_bx_0001:    dw 0x0001

base16sid:
rol16_sid_ffff:   dw 0xFFFF

base16did:
rol16_did_0100:   dw 0x0100

base16bd:
rol16_bxd_3fff:   dw 0x3FFF

base16:
index16:
rol16_bxsi_4000:  dw 0x4000

rol16_bp_ffff:    dw 0xFFFF

base_bp16_d:
rol16_bpd_0002:   dw 0x0002

base_bp16_si_A:
base_bp16_si_B:
rol16_bpsi_2000:  dw 0x2000

base_bp16_di_A:
base_bp16_di_B:
rol16_bpdi_c000:  dw 0xC000

; ---- 16-bit memory (count=CL) ----
base16did2:
rol16_did_c000:   dw 0xC000

rol16_bx_0003:    dw 0x0003

rol16_bx_aaaa:    dw 0xAAAA

; Helpers: SAHF patterns for "flags preserved" checks
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
