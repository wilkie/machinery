; not.asm — thorough NOT tests (r/m8, r/m16) with saved FLAGS and DOS exit
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   NOT inverts bits of the destination and DOES NOT affect any flags.
;   We use SAHF to set (SF,ZF,AF,PF,CF) to known patterns, then verify they persist.
;   OF is checked in separate tests (set via ADD or cleared via XOR) to ensure it's preserved as well.

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

; ===================== 8-bit NOT (register) =====================

; 1) AL=00 -> FF ; flags preset to all 0 via SAHF and must remain 0
    mov al, 0x00
    mov ah, [pat0]
    sahf
    not al
    SAVE_FLAGS
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_OF 0           ; OF should remain 0 here (was 0)

; 2) AL=FF -> 00 ; flags preset to all 1 via SAHF and must remain 1
    mov al, 0xFF
    mov ah, [pat1]
    sahf
    not al
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    ; OF not settable by SAHF; skip here

; 3) AH=80 -> 7F ; flags preset to all 0
    mov ah, 0x80
    mov al, [pat0]
    xchg al, ah          ; AL=80, AH=pat0
    sahf
    xchg al, ah          ; restore AH operand=80
    not ah
    SAVE_FLAGS
    mov al, ah
    ASSERT_BYTE 0x7F
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 4) BL=7E -> 81 ; flags preset to all 1
    mov bl, 0x7E
    mov ah, [pat1]
    sahf
    not bl
    SAVE_FLAGS
    mov al, bl
    ASSERT_BYTE 0x81
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 5) CL=01 -> FE ; flags preset to all 0
    mov cl, 0x01
    mov ah, [pat0]
    sahf
    not cl
    SAVE_FLAGS
    mov al, cl
    ASSERT_BYTE 0xFE
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 6) DL=AA -> 55 ; flags preset to all 1
    mov dl, 0xAA
    mov ah, [pat1]
    sahf
    not dl
    SAVE_FLAGS
    mov al, dl
    ASSERT_BYTE 0x55
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1


; ---------- OF preservation (byte) ----------

; 7) Make OF=1 via ADD; NOT must keep OF=1
    mov bl, 0x7F
    add bl, 0x01          ; OF=1, CF=0 (others set by ADD)
    mov al, 0x33
    not al
    SAVE_FLAGS
    ASSERT_BYTE 0xCC
    CHECK_OF 1

; 8) Clear OF=0 via XOR; NOT must keep OF=0
    xor ax, ax            ; OF=0, CF=0, ZF=1, PF=1, SF=0, AF=0
    mov al, 0x00
    not al
    SAVE_FLAGS
    ASSERT_BYTE 0xFF
    CHECK_OF 0


; ===================== 8-bit NOT (memory; all addressing forms) =====================

; 9) [si]=00 -> FF ; flags preset 0
    lea si, [not8_si_00]
    mov ah, [pat0]
    sahf
    not byte [si]
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xFF
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 10) [di]=80 -> 7F ; flags preset 1
    lea di, [not8_di_80]
    mov ah, [pat1]
    sahf
    not byte [di]
    SAVE_FLAGS
    mov al, [di]
    ASSERT_BYTE 0x7F
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 11) [bx]=FF -> 00 ; flags preset 0
    lea bx, [not8_bx_ff]
    mov ah, [pat0]
    sahf
    not byte [bx]
    SAVE_FLAGS
    mov al, [bx]
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 12) [si+disp]=7E -> 81 ; flags preset 1
    lea si, [base8sid]
    mov ah, [pat1]
    sahf
    not byte [si + (not8_sid_7e - base8sid)]
    SAVE_FLAGS
    mov al, [si + (not8_sid_7e - base8sid)]
    ASSERT_BYTE 0x81
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 13) [di+disp]=01 -> FE ; flags preset 0
    lea di, [base8dd]
    mov ah, [pat0]
    sahf
    not byte [di + (not8_did_01 - base8dd)]
    SAVE_FLAGS
    mov al, [di + (not8_did_01 - base8dd)]
    ASSERT_BYTE 0xFE
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 14) [bx+disp]=AA -> 55 ; flags preset 1
    lea bx, [base8bd]
    mov ah, [pat1]
    sahf
    not byte [bx + (not8_bxd_aa - base8bd)]
    SAVE_FLAGS
    mov al, [bx + (not8_bxd_aa - base8bd)]
    ASSERT_BYTE 0x55
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 15) [bx+si+disp]=F0 -> 0F ; flags preset 0
    lea bx, [base8]
    lea si, [index8]
    mov ah, [pat0]
    sahf
    not byte [bx+si + (not8_bxsi_f0 - base8 - index8)]
    SAVE_FLAGS
    mov al, [bx+si + (not8_bxsi_f0 - base8 - index8)]
    ASSERT_BYTE 0x0F
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 16) [bp] (ds:)=0F -> F0 ; flags preset 1
    mov bp, not8_bp_0f
    mov ah, [pat1]
    sahf
    not byte [ds:bp]
    SAVE_FLAGS
    mov al, [ds:bp]
    ASSERT_BYTE 0xF0
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 17) [bp+disp] (ds:)=55 -> AA ; flags preset 0
    lea bp, [base_bp_d]
    mov ah, [pat0]
    sahf
    not byte [ds:bp + (not8_bpd_55 - base_bp_d)]
    SAVE_FLAGS
    mov al, [ds:bp + (not8_bpd_55 - base_bp_d)]
    ASSERT_BYTE 0xAA
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 18) [bp+si+disp] (ds:)=01 -> FE ; flags preset 1
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov ah, [pat1]
    sahf
    not byte [ds:bp+si + (not8_bpsi_01 - base_bp_si_A - base_bp_si_B)]
    SAVE_FLAGS
    mov al, [ds:bp+si + (not8_bpsi_01 - base_bp_si_A - base_bp_si_B)]
    ASSERT_BYTE 0xFE
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 19) [bp+di+disp] (ds:)=7F -> 80 ; flags preset 0
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov ah, [pat0]
    sahf
    not byte [ds:bp+di + (not8_bpdi_7f - base_bp_di_A - base_bp_di_B)]
    SAVE_FLAGS
    mov al, [ds:bp+di + (not8_bpdi_7f - base_bp_di_A - base_bp_di_B)]
    ASSERT_BYTE 0x80
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0


; ---------- OF preservation (byte, memory) ----------

; 20) Make OF=1 via ADD; NOT [si] must keep OF=1
    lea si, [not8_si_33]
    mov bl, 0x7F
    add bl, 0x01          ; OF=1
    not byte [si]
    SAVE_FLAGS
    mov al, [si]
    ASSERT_BYTE 0xCC
    CHECK_OF 1


; ===================== 16-bit NOT (register) =====================

; 21) AX=0000 -> FFFF ; flags preset 0
    mov ax, 0x0000
    mov ah, [pat0]
    sahf
    not ax
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 22) AX=FFFF -> 0000 ; flags preset 1
    mov ah, [pat1]
    sahf
    mov ax, 0xFFFF
    not ax
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 23) BX=8000 -> 7FFF ; flags preset 0
    mov bx, 0x8000
    mov ah, [pat0]
    sahf
    not bx
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 24) CX=00FF -> FF00 ; flags preset 1
    mov cx, 0x00FF
    mov ah, [pat1]
    sahf
    not cx
    SAVE_FLAGS
    mov ax, cx
    ASSERT_AX 0xFF00
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 25) DX=1234 -> EDCB ; flags preset 0
    mov dx, 0x1234
    mov ah, [pat0]
    sahf
    not dx
    SAVE_FLAGS
    mov ax, dx
    ASSERT_AX 0xEDCB
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0


; ---------- OF preservation (word) ----------

; 26) Make OF=1 via ADD; NOT AX must keep OF=1
    mov si, 0x7FFF
    add si, 0x0001        ; OF=1
    mov ax, 0x1357
    not ax
    SAVE_FLAGS
    ASSERT_AX 0xECA8
    CHECK_OF 1

; 27) Clear OF=0 via XOR; NOT BX must keep OF=0
    xor bx, bx            ; OF=0, others set by XOR
    mov bx, 0x00F0
    not bx
    SAVE_FLAGS
    mov ax, bx
    ASSERT_AX 0xFF0F
    CHECK_OF 0


; ===================== 16-bit NOT (memory; all addressing forms) =====================

; 28) [si]=0000 -> FFFF ; flags preset 0
    lea si, [not16_si_0000]
    mov ah, [pat0]
    sahf
    not word [si]
    SAVE_FLAGS
    mov ax, [si]
    ASSERT_AX 0xFFFF
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 29) [di]=FFFF -> 0000 ; flags preset 1
    lea di, [not16_di_ffff]
    mov ah, [pat1]
    sahf
    not word [di]
    SAVE_FLAGS
    mov ax, [di]
    ASSERT_AX 0x0000
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 30) [bx]=8000 -> 7FFF ; flags preset 0
    lea bx, [not16_bx_8000]
    mov ah, [pat0]
    sahf
    not word [bx]
    SAVE_FLAGS
    mov ax, [bx]
    ASSERT_AX 0x7FFF
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 31) [si+disp]=00FF -> FF00 ; flags preset 1
    lea si, [base16sid]
    mov ah, [pat1]
    sahf
    not word [si + (not16_sid_00ff - base16sid)]
    SAVE_FLAGS
    mov ax, [si + (not16_sid_00ff - base16sid)]
    ASSERT_AX 0xFF00
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 32) [di+disp]=7FFF -> 8000 ; flags preset 0
    lea di, [base16did]
    mov ah, [pat0]
    sahf
    not word [di + (not16_did_7fff - base16did)]
    SAVE_FLAGS
    mov ax, [di + (not16_did_7fff - base16did)]
    ASSERT_AX 0x8000
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 33) [bx+disp]=0101 -> FEFE ; flags preset 1
    lea bx, [base16bd]
    mov ah, [pat1]
    sahf
    not word [bx + (not16_bxd_0101 - base16bd)]
    SAVE_FLAGS
    mov ax, [bx + (not16_bxd_0101 - base16bd)]
    ASSERT_AX 0xFEFE
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 34) [bx+si+disp]=F0F0 -> 0F0F ; flags preset 0
    lea bx, [base16]
    lea si, [index16]
    mov ah, [pat0]
    sahf
    not word [bx+si + (not16_bxsi_f0f0 - base16 - index16)]
    SAVE_FLAGS
    mov ax, [bx+si + (not16_bxsi_f0f0 - base16 - index16)]
    ASSERT_AX 0x0F0F
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 35) [bp] (ds:)=AAAA -> 5555 ; flags preset 1
    mov bp, not16_bp_aaaa
    mov ah, [pat1]
    sahf
    not word [ds:bp]
    SAVE_FLAGS
    mov ax, [ds:bp]
    ASSERT_AX 0x5555
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 36) [bp+disp] (ds:)=1357 -> ECA8 ; flags preset 0
    lea bp, [base_bp16_d]
    mov ah, [pat0]
    sahf
    not word [ds:bp + (not16_bpd_1357 - base_bp16_d)]
    SAVE_FLAGS
    mov ax, [ds:bp + (not16_bpd_1357 - base_bp16_d)]
    ASSERT_AX 0xECA8
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; 37) [bp+si+disp] (ds:)=0001 -> FFFE ; flags preset 1
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    mov ah, [pat1]
    sahf
    not word [ds:bp+si + (not16_bpsi_0001 - base_bp16_si_A - base_bp16_si_B)]
    SAVE_FLAGS
    mov ax, [ds:bp+si + (not16_bpsi_0001 - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0xFFFE
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 38) [bp+di+disp] (ds:)=FE00 -> 01FF ; flags preset 0
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov ah, [pat0]
    sahf
    not word [ds:bp+di + (not16_bpdi_fe00 - base_bp16_di_A - base_bp16_di_B)]
    SAVE_FLAGS
    mov ax, [ds:bp+di + (not16_bpdi_fe00 - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x01FF
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

; 8-bit memory operands
not8_si_00:      db 0x00
not8_di_80:      db 0x80
not8_bx_ff:      db 0xFF

base8sid:
not8_sid_7e:     db 0x7E

base8dd:
not8_did_01:     db 0x01

base8bd:
not8_bxd_aa:     db 0xAA

base8:
index8:
not8_bxsi_f0:    db 0xF0

not8_bp_0f:      db 0x0F

base_bp_d:
not8_bpd_55:     db 0x55

base_bp_si_A:
base_bp_si_B:
not8_bpsi_01:    db 0x01

base_bp_di_A:
base_bp_di_B:
not8_bpdi_7f:    db 0x7F

; extra for OF-preserve byte memory test
not8_si_33:      db 0x33

; 16-bit memory operands
not16_si_0000:     dw 0x0000
not16_di_ffff:     dw 0xFFFF
not16_bx_8000:     dw 0x8000

base16sid:
not16_sid_00ff:    dw 0x00FF

base16did:
not16_did_7fff:    dw 0x7FFF

base16bd:
not16_bxd_0101:    dw 0x0101

base16:
index16:
not16_bxsi_f0f0:   dw 0xF0F0

not16_bp_aaaa:     dw 0xAAAA

base_bp16_d:
not16_bpd_1357:    dw 0x1357

base_bp16_si_A:
base_bp16_si_B:
not16_bpsi_0001:   dw 0x0001

base_bp16_di_A:
base_bp16_di_B:
not16_bpdi_fe00:   dw 0xFE00

; Helpers: SAHF patterns
; pat0 = 0x00 -> SF=0 ZF=0 AF=0 PF=0 CF=0
; pat1 = 0xD5 -> SF=1 ZF=1 AF=1 PF=1 CF=1
pat0: db 0x00
pat1: db 0xD5

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
