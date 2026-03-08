; stos.asm — Thorough tests for STOS (STOSB/STOSW + REP/REPE/REPNE, DF=0/1)
; Harness:
;   int 0x23: assert AX == BX  (used for values and for DI/CX compares)
;   int 0x22: assert AL == AH  (single-flag assertions via saved FLAGS)
; Notes:
;   - STOS stores AL/AX into ES:DI, then DI +=/-= element size per DF.
;   - Flags are **not modified** by STOS (or by REP STOS). REPE/REPNE act like REP (ZF ignored).

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

; Byte memory assert (doesn't touch flags)
%macro ASSERT_MEMB 2
    mov al, [%1]
    mov ah, 0
    mov bx, %2
    int 0x23
%endmacro

; Word memory assert (doesn't touch flags)
%macro ASSERT_MEMW 2
    mov ax, [%1]
    mov bx, %2
    int 0x23
%endmacro

; Compare AX against DX (for "AX unchanged" checks)
%macro ASSERT_AX_EQ_DX 0
    mov bx, dx
    int 0x23
%endmacro


start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; Standard .COM: DS = CS, ES = DS
    push cs
    pop  ds
    push ds
    pop  es

; ===================== Single-step STOSB (DF=0) =====================

; 1) STOSB forward: AL=0x34 into buf1[0]; DI+=1; AX unchanged; flags preserved
    cld
    mov ah, [pat_all1]
    sahf
    mov al, 0x34
    mov ah, 0x12            ; keep AX distinctive
    mov dx, ax              ; save expected AX
    mov di, buf1
    stosb
    SAVE_FLAGS              ; capture preserved flags
    ASSERT_AX_EQ_DX         ; AX unchanged
    ASSERT_DI buf1+1
    ASSERT_MEMB buf1, 0x0034
    ASSERT_MEMB buf1+1, 0x00CC   ; sentinel intact
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Single-step STOSB (DF=1) =====================

; 2) STOSB backward: DI starts at buf2+1; stores to [buf2+1]; DI-=1 → buf2+0
    std
    mov ah, [pat_all1]
    sahf
    mov ax, 0xABCD          ; AL=CD to store
    mov dx, ax
    mov di, buf2+1
    stosb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI buf2+0
    ASSERT_MEMB buf2+1, 0x00CD
    ASSERT_MEMB buf2+0, 0x00CC
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Single-step STOSW (DF=0) =====================

; 3) STOSW forward: AX=0x1234 → [bufw1]=0x1234; DI+=2; AX unchanged; flags preserved
    cld
    mov ah, [pat_zf0]       ; seed flags with ZF=0
    sahf
    mov ax, 0x1234
    mov dx, ax
    mov di, bufw1
    stosw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI bufw1+2
    ASSERT_MEMW bufw1, 0x1234
    ASSERT_MEMB bufw1+2, 0x00CC
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Single-step STOSW (DF=1) =====================

; 4) STOSW backward: start at bufw2+2; store at +2 then DI-=2 → bufw2+0
    std
    mov ah, [pat_all1]
    sahf
    mov ax, 0xBEEF
    mov dx, ax
    mov di, bufw2+2
    stosw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI bufw2+0
    ASSERT_MEMW bufw2+2, 0xBEEF
    ASSERT_MEMB bufw2+0, 0x00CC
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REP STOSB (DF=0) =====================

; 5) REP STOSB 3 bytes: AL=0x7E → buf3[0..2]; DI+=3; CX→0; AX unchanged; flags preserved
    cld
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xAA7E          ; AL=7E, AH distinctive
    mov dx, ax
    mov di, buf3
    mov cx, 3
    rep stosb               ; unconditional 3 stores
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI buf3+3
    ASSERT_CX 0
    ASSERT_MEMB buf3+0, 0x007E
    ASSERT_MEMB buf3+1, 0x007E
    ASSERT_MEMB buf3+2, 0x007E
    ASSERT_MEMB buf3+3, 0x00CC
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REP STOSB (DF=1) =====================

; 6) REP STOSB 3 bytes backward: start at buf4+2; writes [2],[1],[0]; DI ends buf4-1
    std
    mov ah, [pat_all1]
    sahf
    mov ax, 0x557A          ; AL=7A
    mov dx, ax
    mov di, buf4+2
    mov cx, 3
    rep stosb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI buf4-1
    ASSERT_CX 0
    ASSERT_MEMB buf4+2, 0x007A
    ASSERT_MEMB buf4+1, 0x007A
    ASSERT_MEMB buf4+0, 0x007A
    ASSERT_MEMB buf4-1, 0x00CC
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REP STOSW (DF=0) =====================

; 7) REP STOSW 2 words: AX=0x0F0E → bufw3[0], bufw3[2]; DI+=4; CX→0
    cld
    mov ah, [pat_all1]
    sahf
    mov ax, 0x0F0E
    mov dx, ax
    mov di, bufw3
    mov cx, 2
    rep stosw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI bufw3+4
    ASSERT_CX 0
    ASSERT_MEMW bufw3+0, 0x0F0E
    ASSERT_MEMW bufw3+2, 0x0F0E
    ASSERT_MEMB bufw3+4, 0x00CC
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REP STOSW (DF=1) =====================

; 8) REP STOSW 2 words backward: start at bufw4+2; writes at +2 then +0; DI ends bufw4-2
    std
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xCAFE
    mov dx, ax
    mov di, bufw4+2
    mov cx, 2
    rep stosw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI bufw4-2
    ASSERT_CX 0
    ASSERT_MEMW bufw4+2, 0xCAFE
    ASSERT_MEMW bufw4+0, 0xCAFE
    ASSERT_MEMB bufw4-2, 0x00CC
    cld
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REPE/REPNE behave like REP for STOS =====================

; 9) REPE STOSB 4 bytes (ZF initially 0) → still runs 4 unconditionally; DI+=4; flags preserved
    cld
    mov ah, [pat_zf0]       ; ZF=0 to prove it's ignored
    sahf
    mov ax, 0x33A1          ; AL=A1
    mov dx, ax
    mov di, buf5
    mov cx, 4
    repe stosb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI buf5+4
    ASSERT_CX 0
    ASSERT_MEMB buf5+0, 0x00A1
    ASSERT_MEMB buf5+3, 0x00A1
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 10) REPNE STOSW 3 words (ZF initially 1) → runs 3 unconditionally; DI+=6; flags preserved
    cld
    mov ah, [pat_all1]      ; ZF=1 but ignored
    sahf
    mov ax, 0x4242
    mov dx, ax
    mov di, bufw5
    mov cx, 3
    repne stosw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI bufw5+6
    ASSERT_CX 0
    ASSERT_MEMW bufw5+0, 0x4242
    ASSERT_MEMW bufw5+2, 0x4242
    ASSERT_MEMW bufw5+4, 0x4242
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REP with CX=0 (no-op) =====================

; 11) REP STOSB with CX=0 → no execution; DI unchanged; flags preserved
    cld
    xor cx, cx
    mov ah, [pat_all1]
    sahf
    mov ax, 0x55EE          ; AL=EE
    mov dx, ax
    mov di, buf6
    rep stosb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI buf6
    ASSERT_CX 0
    ASSERT_MEMB buf6, 0x00CC
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 12) REP STOSW with CX=0 → no execution; DI unchanged; flags preserved
    cld
    xor cx, cx
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x7777
    mov dx, ax
    mov di, bufw6
    rep stosw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI bufw6
    ASSERT_CX 0
    ASSERT_MEMW bufw6, 0x7E7E     ; sentinel word
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Unaligned & odd-address word stores =====================

; 13) STOSW at odd DI (DF=0): store AX=0xA55A at bufw_odd+1; DI += 2
    cld
    mov ah, [pat_all1]
    sahf
    mov ax, 0xA55A
    mov dx, ax
    mov di, bufw_odd+1
    stosw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI bufw_odd+3
    ASSERT_MEMB bufw_odd+0, 0xCC    ; untouched first byte
    ASSERT_MEMW bufw_odd+1, 0xA55A
    ASSERT_MEMB bufw_odd+3, 0xCC    ; untouched final byte
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 14) REP STOSW 2 words at odd start backward (DF=1): start bufw_odd2+3 → writes at +3, +1; DI ends +(-1)
    std
    mov ah, [pat_all1]
    sahf
    mov ax, 0x1357
    mov dx, ax
    mov di, bufw_odd2+3
    mov cx, 2
    rep stosw
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI bufw_odd2-1
    ASSERT_CX 0
    ASSERT_MEMB bufw_odd2+0, 0xCC    ; untouched first byte
    ASSERT_MEMW bufw_odd2+1, 0x1357
    ASSERT_MEMW bufw_odd2+3, 0x1357
    ASSERT_MEMB bufw_odd2+5, 0x00    ; untouched beyond +4
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Larger counts / sentinel boundaries =====================

; 15) REP STOSB 5 bytes: fill buf7[0..4] with 0x3C; neighbors untouched
    cld
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x7A3C          ; AL=3C
    mov dx, ax
    mov di, buf7
    mov cx, 5
    rep stosb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI buf7+5
    ASSERT_CX 0
    ASSERT_MEMB buf7-1, 0x00CC
    ASSERT_MEMB buf7+0, 0x003C
    ASSERT_MEMB buf7+2, 0x003C
    ASSERT_MEMB buf7+4, 0x003C
    ASSERT_MEMB buf7+5, 0x00CC
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 16) REPNE STOSB 2 bytes backward: start buf8+1; DI ends buf8-1; fill 0xFE
    std
    mov ah, [pat_all1]
    sahf
    mov ax, 0x00FE
    mov dx, ax
    mov di, buf8+1
    mov cx, 2
    repne stosb
    SAVE_FLAGS
    ASSERT_AX_EQ_DX
    ASSERT_DI buf8-1
    ASSERT_CX 0
    ASSERT_MEMB buf8+1, 0x00FE
    ASSERT_MEMB buf8+0, 0x00FE
    ASSERT_MEMB buf8-1, 0x00CC
    cld
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

; Byte buffers (0xCC sentinel)
buf1:     db 0xCC, 0xCC
buf2:     db 0xCC, 0xCC
buf3:     db 0xCC, 0xCC, 0xCC, 0xCC
buf4:     db 0xCC, 0xCC, 0xCC, 0xCC
buf5:     db 0xCC, 0xCC, 0xCC, 0xCC, 0xCC
buf6:     db 0xCC, 0xCC
buf7:     db 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC
buf8:     db 0xCC, 0xCC, 0xCC

; Word buffers (0x7E7E sentinel words, and 0xCC sentinels around)
bufw1:    db 0xCC, 0xCC, 0xCC, 0xCC
bufw2:    db 0xCC, 0xCC, 0xCC, 0xCC
bufw3:    db 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC
bufw4:    db 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC
bufw5:    db 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC
bufw6:    dw 0x7E7E
bufw_odd: db 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0x00
bufw_odd2:db 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0x00

; ---------------- Patterns to seed FLAGS (SAHF controls SF ZF AF PF CF; OF unaffected) ----------------
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
