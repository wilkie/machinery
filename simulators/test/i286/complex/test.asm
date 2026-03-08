; test.asm — thorough TEST instruction checks (r/m8, r/m16) + flags + non-destructive
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   TEST computes (op1 AND op2) for flags only; destination/operands are NOT modified.
;   Flags: CF=0, OF=0, ZF/SF/PF from result; AF is undefined (not checked).

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

; ===================== 8-bit TEST (register/immediate) =====================

; 1) AL=00, TEST AL,00 -> result 00 (CF=0,OF=0,ZF=1,SF=0,PF=1), AL unchanged
    mov al, 0x00
    test al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x00           ; AL unchanged
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 2) AL=7F, TEST AL,01 -> result 01 (ZF=0,SF=0,PF=0), AL unchanged
    mov al, 0x7F
    test al, 0x01
    SAVE_FLAGS
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 3) AL=80, TEST AL,80 -> result 80 (ZF=0,SF=1,PF=0), AL unchanged
    mov al, 0x80
    test al, 0x80
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 4) AH=F0, TEST AH,0F -> result 00 (ZF=1,SF=0,PF=1), AH unchanged
    mov ah, 0xF0
    test ah, 0x0F
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0xF0
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 5) AL=AA, TEST AL,FF -> result AA (ZF=0,SF=1,PF=1), AL unchanged
    mov al, 0xAA
    test al, 0xFF
    SAVE_FLAGS
    ASSERT_BYTE 0xAA
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 6) Precondition CF=1 and OF=1; TEST must clear both (and not modify AL)
;    Set OF=1 via ADD, CF=1 via STC, then TEST AL,00 -> result 00
    stc
    mov bl, 0x7F
    add bl, 0x01            ; OF=1
    mov al, 0x80
    test al, 0x00
    SAVE_FLAGS
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== 8-bit TEST (memory/immediate, all addressing forms) =====================

; 7) [si]=FF, TEST [si],0F -> result 0F (ZF=0,SF=0,PF=1), mem unchanged
    lea si, [tst8_si_ff]
    test byte [si], 0x0F
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 8) [di]=80, TEST [di],01 -> result 00 (ZF=1,SF=0,PF=1), mem unchanged
    lea di, [tst8_di_80]
    test byte [di], 0x01
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 9) [bx]=7E, TEST [bx],01 -> result 00 (ZF=1,PF=1), mem unchanged
    lea bx, [tst8_bx_7e]
    test byte [bx], 0x01
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x7E
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 10) [si+disp]=FE, TEST with F0 -> result F0 (ZF=0,SF=1,PF=1), mem unchanged
    lea si, [base8sid]
    test byte [si + (tst8_sid_fe - base8sid)], 0xF0
    SAVE_FLAGS
    mov al, [si + (tst8_sid_fe - base8sid)]
    ASSERT_BYTE 0xFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 11) [di+disp]=0F, TEST with 01 -> result 01 (ZF=0,SF=0,PF=0), mem unchanged
    lea di, [base8dd]
    test byte [di + (tst8_did_0f - base8dd)], 0x01
    SAVE_FLAGS
    mov al, [di + (tst8_did_0f - base8dd)]
    ASSERT_BYTE 0x0F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 12) [bx+disp]=00, TEST with FF -> result 00 (ZF=1,PF=1), mem unchanged
    lea bx, [base8bd]
    test byte [bx + (tst8_bxd_00 - base8bd)], 0xFF
    SAVE_FLAGS
    mov al, [bx + (tst8_bxd_00 - base8bd)]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 13) [bx+si+disp]=80, TEST with 80 -> result 80 (ZF=0,SF=1,PF=0), mem unchanged
    lea bx, [base8]
    lea si, [index8]
    test byte [bx+si + (tst8_bxsi_80 - base8 - index8)], 0x80
    SAVE_FLAGS
    mov al, [bx+si + (tst8_bxsi_80 - base8 - index8)]
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 14) [bp] (ds:)=FF, TEST with 00 -> result 00 (ZF=1,PF=1), mem unchanged
    mov bp, tst8_bp_ff
    test byte [ds:bp], 0x00
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 15) [bp+disp]=55, TEST with AA -> result 00 (ZF=1,PF=1), mem unchanged
    lea bp, [base_bp_d]
    test byte [ds:bp + (tst8_bpd_55 - base_bp_d)], 0xAA
    SAVE_FLAGS
    mov al, [ds:bp + (tst8_bpd_55 - base_bp_d)]
    ASSERT_BYTE 0x55
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 16) [bp+si+disp]=7E, TEST with 3F -> result 3E (ZF=0,SF=0,PF=0), mem unchanged
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    test byte [ds:bp+si + (tst8_bpsi_7e - base_bp_si_A - base_bp_si_B)], 0x3F
    SAVE_FLAGS
    mov al, [ds:bp+si + (tst8_bpsi_7e - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0x7E
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 17) [bp+di+disp]=7F, TEST with 80 -> result 00 (ZF=1,SF=0,PF=1), mem unchanged
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    test byte [ds:bp+di + (tst8_bpdi_7f - base_bp_di_A - base_bp_di_B)], 0x80
    SAVE_FLAGS
    mov al, [ds:bp+di + (tst8_bpdi_7f - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== 16-bit TEST (register/immediate) =====================

; 18) AX=FFFF, TEST AX,0000 -> result 0000 (CF=0,OF=0,ZF=1,SF=0,PF=1), AX unchanged
    mov ax, 0xFFFF
    test ax, 0x0000
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 19) AX=7FFF, TEST AX,8000 -> result 0000 (ZF=1,PF=1,SF=0), AX unchanged
    mov ax, 0x7FFF
    test ax, 0x8000
    SAVE_FLAGS
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 20) AX=8000, TEST AX,FFFF -> result 8000 (ZF=0,SF=1,PF=1), AX unchanged
    mov ax, 0x8000
    test ax, 0xFFFF
    SAVE_FLAGS
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1           ; low byte 00 → even parity

; 21) Precondition CF=1 and OF=1; TEST must clear both (word), AX unchanged
    stc
    mov cx, 0x7FFF
    add cx, 0x0001        ; OF=1
    mov ax, 0x8001
    test ax, 0x0000       ; result 0000
    SAVE_FLAGS
    ASSERT_AX 0x8001
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== 16-bit TEST (memory/immediate, all addressing forms) =====================

; 22) [si]=7FFF, TEST with 0001 -> result 0001 (ZF=0,SF=0,PF=0), mem unchanged
    lea si, [tst16_si_7fff]
    test word [si], 0x0001
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 23) [di]=FFFF, TEST with 0000 -> result 0000 (ZF=1,PF=1), mem unchanged
    lea di, [tst16_di_ffff]
    test word [di], 0x0000
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 24) [bx]=8001, TEST with 0001 -> result 0001 (ZF=0,SF=0,PF=0), mem unchanged
    lea bx, [tst16_bx_8001]
    test word [bx], 0x0001
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x8001
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 25) [si+disp]=00FF, TEST with 00F0 -> result 00F0 (ZF=0,SF=0,PF=1), mem unchanged
    lea si, [base16sid]
    test word [si + (tst16_sid_00ff - base16sid)], 0x00F0
    SAVE_FLAGS
    mov ax, [si + (tst16_sid_00ff - base16sid)]
    ASSERT_AX 0x00FF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 26) [di+disp]=7FFE, TEST with 0001 -> result 0000 (ZF=1,PF=1), mem unchanged
    lea di, [base16did]
    test word [di + (tst16_did_7ffe - base16did)], 0x0001
    SAVE_FLAGS
    mov ax, [di + (tst16_did_7ffe - base16did)]
    ASSERT_AX 0x7FFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 27) [bx+disp]=8000, TEST with FFFF -> result 8000 (ZF=0,SF=1,PF=1), mem unchanged
    lea bx, [base16bd]
    test word [bx + (tst16_bxd_8000 - base16bd)], 0xFFFF
    SAVE_FLAGS
    mov ax, [bx + (tst16_bxd_8000 - base16bd)]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 28) [bx+si+disp]=7FFF, TEST with 8000 -> result 0000 (ZF=1,PF=1), mem unchanged
    lea bx, [base16]
    lea si, [index16]
    test word [bx+si + (tst16_bxsi_7fff - base16 - index16)], 0x8000
    SAVE_FLAGS
    mov ax, [bx+si + (tst16_bxsi_7fff - base16 - index16)]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 29) [bp] (ds:)=00FF, TEST with 0101 -> result 0001 (ZF=0,SF=0,PF=0), mem unchanged
    mov bp, tst16_bp_00ff
    test word [ds:bp], 0x0101
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x00FF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 30) [bp+disp] (ds:)=1234, TEST with 00FF -> result 0034 (ZF=0,SF=0,PF=0), mem unchanged
    lea bp, [base_bp16_d]
    test word [ds:bp + (tst16_bpd_1234 - base_bp16_d)], 0x00FF
    SAVE_FLAGS
    mov ax, [ds:bp + (tst16_bpd_1234 - base_bp16_d)]
    ASSERT_AX 0x1234
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 31) [bp+si+disp] (ds:)=7FFE, TEST with 7FFF -> result 7FFE (ZF=0,SF=0,PF=0), mem unchanged
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    test word [ds:bp+si + (tst16_bpsi_7ffe - base_bp16_si_A - base_bp16_si_B)], 0x7FFF
    SAVE_FLAGS
    mov ax, [ds:bp+si + (tst16_bpsi_7ffe - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x7FFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0       ; low byte FE has odd parity

; 32) [bp+di+disp] (ds:)=8000, TEST with 8000 -> result 8000 (ZF=0,SF=1,PF=1), mem unchanged
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    test word [ds:bp+di + (tst16_bpdi_8000 - base_bp16_di_A - base_bp16_di_B)], 0x8000
    SAVE_FLAGS
    mov ax, [ds:bp+di + (tst16_bpdi_8000 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; ===================== 8-bit TEST (register–register, opcode 84h) =====================

; 33) AL=F0, AH=0F -> result 00 (ZF=1,PF=1,SF=0), both regs unchanged
    mov al, 0xF0
    mov ah, 0x0F
    test al, ah
    SAVE_FLAGS
    ASSERT_AX 0xFF0        ; AL and AH unchanged
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0

; 34) BL=80, BH=80 -> result 80 (ZF=0,SF=1,PF=0), regs unchanged
    mov bl, 0x80
    mov bh, 0x80
    test bl, bh
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x8080       ; AL and AH unchanged
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 35) CL=7E, DL=01 -> result 00 (ZF=1,PF=1,SF=0), regs unchanged
    mov cl, 0x7E
    mov dl, 0x01
    test cl, dl
    SAVE_FLAGS
    mov al, cl
    ASSERT_BYTE 0x7E
    mov al, dl
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0

; 36) DH=AA, CH=55 -> result 00 (ZF=1,PF=1,SF=0), regs unchanged
    mov dh, 0xAA
    mov ch, 0x55
    test dh, ch
    SAVE_FLAGS
    mov al, dh
    ASSERT_BYTE 0xAA
    mov al, ch
    ASSERT_BYTE 0x55
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0

; ===================== 8-bit TEST (memory–register, opcode 84h) =====================

; 37) [si]=FF, AL=0F -> result 0F (ZF=0,SF=0,PF=1), mem/reg unchanged
    lea si, [rm8_si_ff]
    mov al, 0x0F
    test byte [si], al
    SAVE_FLAGS
    mov al, [si]             ; mem unchanged
    ASSERT_BYTE 0xFF
    mov al, 0x0F             ; AL unchanged
    ASSERT_BYTE 0x0F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 38) [di]=80, AL=01 -> result 00 (ZF=1,PF=1), mem/reg unchanged
    lea di, [rm8_di_80]
    mov al, 0x01
    test byte [di], al
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x80
    mov al, 0x01
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 39) [bx]=7E, DL=01 -> result 00 (ZF=1,PF=1), mem/reg unchanged
    lea bx, [rm8_bx_7e]
    mov dl, 0x01
    test byte [bx], dl
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x7E
    mov al, dl
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 40) [si+disp]=FE, CL=F0 -> result F0 (ZF=0,SF=1,PF=1), mem/reg unchanged
    lea si, [base8sid2]
    mov cl, 0xF0
    test byte [si + (rm8_sid_fe - base8sid2)], cl
    SAVE_FLAGS
    mov al, [si + (rm8_sid_fe - base8sid2)]
    ASSERT_BYTE 0xFE
    mov al, cl
    ASSERT_BYTE 0xF0
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 41) [di+disp]=0F, CH=01 -> result 01 (ZF=0,SF=0,PF=0), mem/reg unchanged
    lea di, [base8dd2]
    mov ch, 0x01
    test byte [di + (rm8_did_0f - base8dd2)], ch
    SAVE_FLAGS
    mov al, [di + (rm8_did_0f - base8dd2)]
    ASSERT_BYTE 0x0F
    mov al, ch
    ASSERT_BYTE 0x01
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 42) [bx+disp]=00, BL=?? -> result 00 (ZF=1,PF=1), mem/reg unchanged
    lea bx, [base8bd2]
    test byte [bx + (rm8_bxd_00 - base8bd2)], bl
    SAVE_FLAGS
    mov ax, bx
    lea cx, [base8bd2]
    ASSERT_AX cx
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 43) [bx+si+disp]=80, BH=0? -> result 00 (ZF=1,SF=0,PF=1), mem/reg unchanged
    lea bx, [base8_2]
    lea si, [index8_2]
    test byte [bx+si + (rm8_bxsi_80 - base8_2 - index8_2)], bh
    SAVE_FLAGS
    mov ax, bx
    lea cx, [base8_2]
    ASSERT_AX cx
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 44) [bp] (ds:)=FF, AL=00 -> result 00 (ZF=1,PF=1), mem/reg unchanged
    mov bp, rm8_bp_ff
    mov al, 0x00
    test byte [ds:bp], al
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xFF
    mov al, 0x00
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 45) [bp+disp] (ds:)=55, AH=AA -> result 00 (ZF=1,PF=1), mem/reg unchanged
    lea bp, [base_bp_d2]
    mov ah, 0xAA
    test byte [ds:bp + (rm8_bpd_55 - base_bp_d2)], ah
    SAVE_FLAGS
    push ax
    mov al, [ds:bp + (rm8_bpd_55 - base_bp_d2)]
    ASSERT_BYTE 0x55
    pop ax
    mov al, ah
    ASSERT_BYTE 0xAA
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 46) [bp+si+disp] (ds:)=7E, DL=3F -> result 3E (ZF=0,SF=0,PF=1), mem/reg unchanged
    lea bp, [base_bp_si_A2]
    lea si, [base_bp_si_B2]
    mov dl, 0x3F
    test byte [ds:bp+si + (rm8_bpsi_7e - base_bp_si_A2 - base_bp_si_B2)], dl
    SAVE_FLAGS
    mov al, [ds:bp+si + (rm8_bpsi_7e - base_bp_si_A2 - base_bp_si_B2)]
    ASSERT_BYTE 0x7E
    mov al, dl
    ASSERT_BYTE 0x3F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 47) [bp+di+disp] (ds:)=7F, DH=80 -> result 00 (ZF=1,PF=1), mem/reg unchanged
    lea bp, [base_bp_di_A2]
    lea di, [base_bp_di_B2]
    mov dh, 0x80
    test byte [ds:bp+di + (rm8_bpdi_7f - base_bp_di_A2 - base_bp_di_B2)], dh
    SAVE_FLAGS
    mov al, [ds:bp+di + (rm8_bpdi_7f - base_bp_di_A2 - base_bp_di_B2)]
    ASSERT_BYTE 0x7F
    mov al, dh
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1


; ===================== 16-bit TEST (register–register, opcode 85h) =====================

; 48) AX=FFFF, BX=0000 -> result 0000 (ZF=1,PF=1,SF=0), regs unchanged
    mov ax, 0xFFFF
    mov bx, 0x0000
    test ax, bx
    SAVE_FLAGS
    push bx
    ASSERT_AX 0xFFFF
    pop bx
    mov ax, bx
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0

; 49) DX=8000, SI=8000 -> result 8000 (ZF=0,SF=1,PF=1), regs unchanged
    mov dx, 0x8000
    mov si, 0x8000
    test dx, si
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0x8000
    mov ax, si
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 50) CX=7FFF, DI=8000 -> result 0000 (ZF=1,PF=1,SF=0), regs unchanged
    mov cx, 0x7FFF
    mov di, 0x8000
    test cx, di
    SAVE_FLAGS
    mov ax, cx
    ASSERT_AX 0x7FFF
    mov ax, di
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_SF 0

; 51) BP=00FF, SP=00F0 -> result 00F0 (ZF=0,SF=0,PF=1), regs unchanged
    mov bp, 0x00FF
    mov sp, 0x00F0
    test bp, sp
    SAVE_FLAGS
    mov ax, bp
    ASSERT_AX 0x00FF
    mov ax, sp
    ASSERT_AX 0x00F0
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ===================== 16-bit TEST (memory–register, opcode 85h) =====================

; 52) [si]=7FFF, AX=0001 -> result 0001 (ZF=0,SF=0,PF=0), mem/reg unchanged
    lea si, [rm16_si_7fff]
    mov ax, 0x0001
    test word [si], ax
    SAVE_FLAGS
    ASSERT_AX 0x0001       ; AX unchanged
    mov ax, [si]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 53) [di]=FFFF, BX=0000 -> result 0000 (ZF=1,PF=1), mem/reg unchanged
    lea di, [rm16_di_ffff]
    mov bx, 0x0000
    test word [di], bx
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x0000
    mov ax, [di]
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 54) [bx]=8001, CX=0001 -> result 0001 (ZF=0,SF=0,PF=0), mem/reg unchanged
    lea bx, [rm16_bx_8001]
    mov cx, 0x0001
    test word [bx], cx
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x8001
    mov ax, cx
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 55) [si+disp]=00FF, DX=00F0 -> result 00F0 (ZF=0,SF=0,PF=1), mem/reg unchanged
    lea si, [base16sid2]
    mov dx, 0x00F0
    test word [si + (rm16_sid_00ff - base16sid2)], dx
    SAVE_FLAGS
    mov ax, [si + (rm16_sid_00ff - base16sid2)]
    ASSERT_AX 0x00FF
    mov ax, dx
    ASSERT_AX 0x00F0
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 56) [di+disp]=7FFE, SI=0001 -> result 0000 (ZF=1,PF=1), mem/reg unchanged
    lea di, [base16did2]
    mov si, 0x0001
    test word [di + (rm16_did_7ffe - base16did2)], si
    SAVE_FLAGS
    mov ax, [di + (rm16_did_7ffe - base16did2)]
    ASSERT_AX 0x7FFE
    mov ax, si
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 57) [bx+disp]=8000, DI=FFFF -> result 8000 (ZF=0,SF=1,PF=1), mem/reg unchanged
    lea bx, [base16bd2]
    mov di, 0xFFFF
    test word [bx + (rm16_bxd_8000 - base16bd2)], di
    SAVE_FLAGS
    mov ax, [bx + (rm16_bxd_8000 - base16bd2)]
    ASSERT_AX 0x8000
    mov ax, di
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 58) [bx+si+disp]=7FFF, BP=8000 -> result 0000 (ZF=1,PF=1), mem/reg unchanged
    lea bx, [base16_2]
    lea si, [index16_2]
    mov bp, 0x8000
    test word [bx+si + (rm16_bxsi_7fff - base16_2 - index16_2)], bp
    SAVE_FLAGS
    mov ax, [bx+si + (rm16_bxsi_7fff - base16_2 - index16_2)]
    ASSERT_AX 0x7FFF
    mov ax, bp
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_PF 1

; 59) [bp] (ds:)=00FF, AX=0101 -> result 0001 (ZF=0,SF=0,PF=0), mem/reg unchanged
    mov bp, rm16_bp_00ff
    mov ax, 0x0101
    test word [ds:bp], ax
    SAVE_FLAGS
    ASSERT_AX 0x0101    ; AX unchanged
    mov ax, [ds:bp]
    ASSERT_AX 0x00FF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 60) [bp+disp] (ds:)=1234, CX=00FF -> result 0034 (ZF=0,SF=0,PF=0), mem/reg unchanged
    lea bp, [base_bp16_d2]
    mov cx, 0x00FF
    test word [ds:bp + (rm16_bpd_1234 - base_bp16_d2)], cx
    SAVE_FLAGS
    mov ax, [ds:bp + (rm16_bpd_1234 - base_bp16_d2)]
    ASSERT_AX 0x1234
    mov ax, cx
    ASSERT_AX 0x00FF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 61) [bp+si+disp] (ds:)=7FFE, DX=7FFF -> result 7FFE (ZF=0,SF=0,PF=0), mem/reg unchanged
    lea bp, [base_bp16_si_A2]
    lea si, [base_bp16_si_B2]
    mov dx, 0x7FFF
    test word [ds:bp+si + (rm16_bpsi_7ffe - base_bp16_si_A2 - base_bp16_si_B2)], dx
    SAVE_FLAGS
    mov ax, [ds:bp+si + (rm16_bpsi_7ffe - base_bp16_si_A2 - base_bp16_si_B2)]
    ASSERT_AX 0x7FFE
    mov ax, dx
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 62) [bp+di+disp] (ds:)=8000, SI=8000 -> result 8000 (ZF=0,SF=1,PF=1), mem/reg unchanged
    lea bp, [base_bp16_di_A2]
    lea di, [base_bp16_di_B2]
    mov si, 0x8000
    test word [ds:bp+di + (rm16_bpdi_8000 - base_bp16_di_A2 - base_bp16_di_B2)], si
    SAVE_FLAGS
    mov ax, [ds:bp+di + (rm16_bpdi_8000 - base_bp16_di_A2 - base_bp16_di_B2)]
    ASSERT_AX 0x8000
    mov ax, si
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; 8-bit memory operands
tst8_si_ff:     db 0xFF
tst8_di_80:     db 0x80
tst8_bx_7e:     db 0x7E

base8sid:
tst8_sid_fe:    db 0xFE

base8dd:
tst8_did_0f:    db 0x0F

base8bd:
tst8_bxd_00:    db 0x00

base8:
index8:
tst8_bxsi_80:   db 0x80

tst8_bp_ff:     db 0xFF

base_bp_d:
tst8_bpd_55:    db 0x55

base_bp_si_A:
base_bp_si_B:
tst8_bpsi_7e:   db 0x7E

base_bp_di_A:
base_bp_di_B:
tst8_bpdi_7f:   db 0x7F

; 16-bit memory operands
tst16_si_7fff:    dw 0x7FFF
tst16_di_ffff:    dw 0xFFFF
tst16_bx_8001:    dw 0x8001

base16sid:
tst16_sid_00ff:   dw 0x00FF

base16did:
tst16_did_7ffe:   dw 0x7FFE

base16bd:
tst16_bxd_8000:   dw 0x8000

base16:
index16:
tst16_bxsi_7fff:  dw 0x7FFF

tst16_bp_00ff:    dw 0x00FF

base_bp16_d:
tst16_bpd_1234:   dw 0x1234

base_bp16_si_A:
base_bp16_si_B:
tst16_bpsi_7ffe:  dw 0x7FFE

base_bp16_di_A:
base_bp16_di_B:
tst16_bpdi_8000:  dw 0x8000

; ------- Extra data for TEST r/m, r coverage -------

; 8-bit memory (second set)
rm8_si_ff:     db 0xFF
rm8_di_80:     db 0x80
rm8_bx_7e:     db 0x7E

base8sid2:
rm8_sid_fe:    db 0xFE

base8dd2:
rm8_did_0f:    db 0x0F

base8bd2:
rm8_bxd_00:    db 0x00

base8_2:
index8_2:
rm8_bxsi_80:   db 0x80

rm8_bp_ff:     db 0xFF

base_bp_d2:
rm8_bpd_55:    db 0x55

base_bp_si_A2:
base_bp_si_B2:
rm8_bpsi_7e:   db 0x7E

base_bp_di_A2:
base_bp_di_B2:
rm8_bpdi_7f:   db 0x7F

; 16-bit memory (second set)
rm16_si_7fff:    dw 0x7FFF
rm16_di_ffff:    dw 0xFFFF
rm16_bx_8001:    dw 0x8001

base16sid2:
rm16_sid_00ff:   dw 0x00FF

base16did2:
rm16_did_7ffe:   dw 0x7FFE

base16bd2:
rm16_bxd_8000:   dw 0x8000

base16_2:
index16_2:
rm16_bxsi_7fff:  dw 0x7FFF

rm16_bp_00ff:    dw 0x00FF

base_bp16_d2:
rm16_bpd_1234:   dw 0x1234

base_bp16_si_A2:
base_bp16_si_B2:
rm16_bpsi_7ffe:  dw 0x7FFE

base_bp16_di_A2:
base_bp16_di_B2:
rm16_bpdi_8000:  dw 0x8000


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
