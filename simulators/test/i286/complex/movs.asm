; movs.asm — Thorough tests for MOVS (MOVSB/MOVSW + REP/REPE/REPNE, DF=0/1, overlap)
; Harness:
;   int 0x23: assert AX == BX  (used for values and for SI/DI/CX via moving regs)
;   int 0x22: assert AL == AH  (single-flag assertions via saved FLAGS)
; Notes:
;   - MOVSB: [ES:DI] <- [DS:SI]; SI +=/-= 1; DI +=/-= 1
;   - MOVSW: [ES:DI] <- [DS:SI]; SI +=/-= 2; DI +=/-= 2
;   - FLAGS are not modified; REP/REPE/REPNE behave the same (ZF ignored).

bits 16
org 0x100

%define ovb (ovb_all+1)
%define wovb (wovb_all+2)

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

%macro ASSERT_SI 1
    mov ax, si
    mov bx, %1
    int 0x23
%endmacro

%macro ASSERT_DI 1
    mov ax, di
    mov bx, %1
    int 0x23
%endmacro

%macro ASSERT_CX 1
    mov ax, cx
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

; Byte/word mem asserts (don’t touch flags)
%macro ASSERT_MEMB 2
    mov al, [%1]
    mov ah, 0
    mov bx, %2
    int 0x23
%endmacro
%macro ASSERT_MEMW 2
    mov ax, [%1]
    mov bx, %2
    int 0x23
%endmacro

; “AX unchanged” check (compare AX to DX)
%macro ASSERT_AX_EQ_DX 0
    mov bx, dx
    int 0x23
%endmacro


start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; DS = CS; ES = DS
    push cs
    pop  ds
    push ds
    pop  es

; ===================== Single-step MOVSB (DF=0) =====================

; 1) MOVSB forward: copy 0x34 from src1[0] -> dst1[0]; SI/DI +=1; AX unchanged; flags preserved
    cld
    mov ah, [pat_all1]
    sahf
    mov ax, 0x1234            ; AX should remain unchanged
    mov dx, ax
    mov si, src1
    mov di, dst1
    movsb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI src1+1
    ASSERT_DI dst1+1
    ASSERT_MEMB dst1+0, 0x0034
    ASSERT_MEMB dst1+1, 0x00CC
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Single-step MOVSB (DF=1) =====================

; 2) MOVSB backward: SI=src2+1, DI=dst2+1; store 0xCD to dst2+1; SI/DI -=1
    std
    mov ah, [pat_all1]
    sahf
    mov ax, 0xABCD
    mov dx, ax
    mov si, src2+1
    mov di, dst2+1
    movsb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI src2+0
    ASSERT_DI dst2+0
    ASSERT_MEMB dst2+1, 0x00CD
    ASSERT_MEMB dst2+0, 0x00CC
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Single-step MOVSW (DF=0 / DF=1) =====================

; 3) MOVSW forward: copy 0x1234; SI/DI +=2; AX unchanged; flags preserved
    cld
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x8888
    mov dx, ax
    mov si, wsrc1
    mov di, wdst1
    movsw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI wsrc1+2
    ASSERT_DI wdst1+2
    ASSERT_MEMW wdst1, 0x1234
    ASSERT_MEMB wdst1+2, 0x00CC
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 4) MOVSW backward: start at +2; copy 0xBEEF; SI/DI -=2
    std
    mov ah, [pat_all1]
    sahf
    mov ax, 0x7777
    mov dx, ax
    mov si, wsrc2+2
    mov di, wdst2+2
    movsw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI wsrc2+0
    ASSERT_DI wdst2+0
    ASSERT_MEMW wdst2+2, 0xBEEF
    ASSERT_MEMB wdst2+0, 0x00CC
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REP MOVSB (DF=0 / DF=1) =====================

; 5) REP MOVSB forward 3 bytes: [1,2,7] -> dst3[0..2]; SI/DI +=3; CX=0; AX unchanged; flags preserved
    cld
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xAA7E            ; AX signature
    mov dx, ax
    mov si, rsrc1
    mov di, rdst1
    mov cx, 3
    rep movsb                 ; unconditional
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI rsrc1+3
    ASSERT_DI rdst1+3
    ASSERT_CX 0
    ASSERT_MEMB rdst1+0, 0x0001
    ASSERT_MEMB rdst1+1, 0x0002
    ASSERT_MEMB rdst1+2, 0x0007
    ASSERT_MEMB rdst1+3, 0x00CC
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 6) REP MOVSB backward 3 bytes: [5,6,7] -> rdst2[2..0]; SI/DI -=3; CX=0
    std
    mov ah, [pat_all1]
    sahf
    mov ax, 0x557A
    mov dx, ax
    mov si, rsrc2+2           ; start at last
    mov di, rdst2+2
    mov cx, 3
    rep movsb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI rsrc2-1
    ASSERT_DI rdst2-1
    ASSERT_CX 0
    ASSERT_MEMB rdst2+2, 0x0007
    ASSERT_MEMB rdst2+1, 0x0006
    ASSERT_MEMB rdst2+0, 0x0005
    ASSERT_MEMB rdst2+3, 0x00CC
    ASSERT_MEMB rdst2-1, 0x0007
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REP MOVSW (DF=0 / DF=1) =====================

; 7) REP MOVSW forward 2 words
    cld
    mov ah, [pat_all1]
    sahf
    mov ax, 0x0F0E
    mov dx, ax
    mov si, wrep_src1
    mov di, wrep_dst1
    mov cx, 2
    rep movsw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI wrep_src1+4
    ASSERT_DI wrep_dst1+4
    ASSERT_CX 0
    ASSERT_MEMW wrep_dst1+0, 0xAAAA
    ASSERT_MEMW wrep_dst1+2, 0x5555
    ASSERT_MEMB wrep_dst1+4, 0x00CC
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 8) REP MOVSW backward 2 words
    std
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xCAFE
    mov dx, ax
    mov si, wrep_src2+2
    mov di, wrep_dst2+2
    mov cx, 2
    rep movsw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI wrep_src2-2
    ASSERT_DI wrep_dst2-2
    ASSERT_CX 0
    ASSERT_MEMW wrep_dst2+2, 0x0A0B
    ASSERT_MEMW wrep_dst2+0, 0x0C0D
    cld
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REPE/REPNE behave like REP for MOVS =====================

; 9) REPE MOVSB 4 bytes with ZF initially 0 → still runs 4 unconditionally
    cld
    mov ah, [pat_zf0]         ; ZF=0 to prove it's ignored
    sahf
    mov ax, 0x33A1
    mov dx, ax
    mov si, eqsrc_b
    mov di, eqdst_b
    mov cx, 4
    repe movsb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI eqsrc_b+4
    ASSERT_DI eqdst_b+4
    ASSERT_CX 0
    ASSERT_MEMB eqdst_b+0, 0x00A1
    ASSERT_MEMB eqdst_b+3, 0x00A4
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 10) REPNE MOVSW 3 words with ZF initially 1 → runs 3 unconditionally
    cld
    mov ah, [pat_all1]        ; ZF=1 but ignored
    sahf
    mov ax, 0x4242
    mov dx, ax
    mov si, eqsrc_w
    mov di, eqdst_w
    mov cx, 3
    repne movsw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI eqsrc_w+6
    ASSERT_DI eqdst_w+6
    ASSERT_CX 0
    ASSERT_MEMW eqdst_w+0, 0x1111
    ASSERT_MEMW eqdst_w+2, 0x2222
    ASSERT_MEMW eqdst_w+4, 0x3333
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REP with CX=0 (no-op) =====================

; 11) REP MOVSB with CX=0 → no execution; SI/DI unchanged; flags/AX preserved
    cld
    xor cx, cx
    mov ah, [pat_all1]
    sahf
    mov ax, 0x5AEE
    mov dx, ax
    mov si, rsrc1
    mov di, rdst1
    rep movsb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI rsrc1
    ASSERT_DI rdst1
    ASSERT_CX 0
    ASSERT_MEMB rdst1+0, 0x0001    ; unchanged from prior test sequence ok
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 12) REP MOVSW with CX=0 → no execution; SI/DI unchanged; flags/AX preserved
    cld
    xor cx, cx
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x7777
    mov dx, ax
    mov si, wrep_src1
    mov di, wrep_dst1
    rep movsw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI wrep_src1
    ASSERT_DI wrep_dst1
    ASSERT_CX 0
    ASSERT_MEMW wrep_dst1+0, 0xAAAA
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Overlap scenarios (show actual x86 element order) =====================

; 13) Forward overlap bytes (DI = SI+1), CX=4, DF=0 → cascade copies first byte across
;     src: [11,22,33,44,CC] → after: [11,11,11,11,11,CC]
    cld
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xBEEF
    mov dx, ax
    mov si, ovf
    mov di, ovf+1
    mov cx, 4
    rep movsb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI ovf+4
    ASSERT_DI ovf+5
    ASSERT_CX 0
    ASSERT_MEMB ovf+0, 0x0011
    ASSERT_MEMB ovf+1, 0x0011
    ASSERT_MEMB ovf+2, 0x0011
    ASSERT_MEMB ovf+3, 0x0011
    ASSERT_MEMB ovf+4, 0x0011
    ASSERT_MEMB ovf+5, 0x00CC
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 14) Backward overlap bytes (DI = SI-1), CX=3, DF=1 → cascade copies last byte backward
;     padded buffer so DI never underflows; result: leading 3 bytes all = last source byte (0x44)
    std
    mov ah, [pat_all1]
    sahf
    mov ax, 0xFACE
    mov dx, ax
    mov si, ovb+3           ; last data byte (0x44)
    mov di, ovb+2
    mov cx, 3
    rep movsb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI ovb+0
    ASSERT_DI ovb-1         ; lands on pad sentinel
    ASSERT_CX 0
    ASSERT_MEMB ovb+3, 0x0044
    ASSERT_MEMB ovb+2, 0x0044
    ASSERT_MEMB ovb+1, 0x0044
    ASSERT_MEMB ovb+0, 0x0044
    ASSERT_MEMB ovb-1, 0x00CC
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 15) Forward overlap words (DI = SI+2), CX=3, DF=0 → words 1..3 become copy of word0
    cld
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xDEAD
    mov dx, ax
    mov si, wovf
    mov di, wovf+2
    mov cx, 3
    rep movsw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI wovf+6
    ASSERT_DI wovf+8
    ASSERT_CX 0
    ASSERT_MEMW wovf+0, 0x1122
    ASSERT_MEMW wovf+2, 0x1122
    ASSERT_MEMW wovf+4, 0x1122
    ASSERT_MEMW wovf+6, 0x1122
    ASSERT_MEMW wovf+8, 0x7E7E      ; sentinel unchanged
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 16) Backward overlap words (DI = SI-2), CX=3, DF=1 → words 0..2 become copy of last word
    std
    mov ah, [pat_all1]
    sahf
    mov ax, 0xBADA
    mov dx, ax
    mov si, wovb+6          ; word index 3 (0x4444)
    mov di, wovb+4          ; index 2
    mov cx, 3
    rep movsw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI wovb+0
    ASSERT_DI wovb-2        ; falls on pad sentinel word
    ASSERT_CX 0
    ASSERT_MEMW wovb+6, 0x4444
    ASSERT_MEMW wovb+4, 0x4444
    ASSERT_MEMW wovb+2, 0x4444
    ASSERT_MEMW wovb+0, 0x4444
    ASSERT_MEMW wovb-2, 0x7E7E
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Odd-address word copies =====================

; 17) MOVSW at odd addresses forward: SI=oddw_src+1 -> DI=oddw_dst+1
    cld
    mov ah, [pat_all1]
    sahf
    mov ax, 0x0A0A
    mov dx, ax
    mov si, oddw_src+1         ; source word = 0xCDAB
    mov di, oddw_dst+1
    movsw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_SI oddw_src+3
    ASSERT_DI oddw_dst+3
    ASSERT_MEMW oddw_dst+1, 0xABCD
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; Single-step byte src/dst (+sentinels)
src1:   db 0x34
dst1:   db 0xCC, 0xCC

src2:   db 0xAB, 0xCD
dst2:   db 0xCC, 0xCC

; Single-step word src/dst
wsrc1:  dw 0x1234
wdst1:  db 0xCC, 0xCC, 0xCC    ; room for one word + sentinel byte

wsrc2:  dw 0xAAAA, 0xBEEF
wdst2:  db 0xCC, 0xCC, 0xCC, 0xCC

; REP byte src/dst
rsrc1:  db 1,2,7
rdst1:  db 0xCC,0xCC,0xCC,0xCC

rsrc2:  db 5,6,7
rdst2:  db 0xCC,0xCC,0xCC,0xCC

; REP word src/dst
wrep_src1: dw 0xAAAA, 0x5555
wrep_dst1: db 0xCC,0xCC,0xCC,0xCC,0xCC

wrep_src2: dw 0x0C0D, 0x0A0B
wrep_dst2: db 0xCC,0xCC,0xCC,0xCC,0xCC

; REPE/REPNE demo sources
eqsrc_b: db 0xA1,0xA2,0xA3,0xA4
eqdst_b: db 0xCC,0xCC,0xCC,0xCC,0xCC

eqsrc_w: dw 0x1111,0x2222,0x3333
eqdst_w: db 0xCC,0xCC,0xCC,0xCC,0xCC,0xCC,0xCC

; Overlap forward bytes
ovf:     db 0x11,0x22,0x33,0x44,0xCC,0xCC

; Overlap backward bytes with left pad:
ovb_all: db 0xCC, 0x11,0x22,0x33,0x44, 0xCC

; Overlap forward words
wovf:    dw 0x1122,0x3344,0x5566,0x7788,0x7E7E

; Overlap backward words with left pad:
wovb_all: dw 0x7E7E, 0x1111,0x2222,0x3333,0x4444

; Odd-address word copy
oddw_src: db 0xEF,0xCD,0xAB,0x89
oddw_dst: db 0xCC,0xCC,0xCC,0xCC

; ---------- SAHF patterns (SF/ZF/AF/PF/CF; OF unaffected) ----------
pat_all1: db 0xD5            ; SF=1 ZF=1 AF=1 PF=1 CF=1
pat_zf0:  db 0x95            ; SF=1 ZF=0 AF=1 PF=1 CF=1

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
