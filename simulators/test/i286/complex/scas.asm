; scas.asm — Thorough tests for SCAS (SCASB/SCASW + REP/REPE/REPNE)
; Harness:
;   int 0x23: assert AX == BX  (used for DI/CX equality or values we move to AX)
;   int 0x22: assert AL == AH  (single-flag assertions via saved FLAGS)
; Notes:
;   - SCAS compares AL (or AX) with ES:DI and sets flags as if SUB AL/AX - [ES:DI].
;   - DI increments/decrements by element size depending on DF.
;   - REP runs CX times unconditionally; REPE/REPZ continues while ZF=1; REPNE/REPNZ continues while ZF=0.

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

    ; Standard .COM: DS = CS, ES = DS
    push cs
    pop  ds
    push ds
    pop  es

; ===================== Single-step SCASB (DF=0) =====================

; 1) Equal bytes: AL=0x34 vs [DI]=0x34 → ZF=1, CF=0, SF=0, PF=1, AF=0, OF=0; DI += 1
    cld
    mov al, 0x34
    mov di, b_eq1
    scasb
    SAVE_FLAGS
    ASSERT_DI b_eq1+1
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0

; 2) AL < [DI]: 0x01 - 0x02 = 0xFF → CF=1, ZF=0, SF=1, PF=1, AF=1, OF=0; DI += 1
    cld
    mov al, 0x01
    mov di, b_lt
    scasb
    SAVE_FLAGS
    ASSERT_DI b_lt+1
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0

; 3) Signed overflow: 0x80 - 0x7F = 0x01 → OF=1, CF=0, AF=1, SF=0, PF=0; DI += 1
    cld
    mov al, 0x80
    mov di, b_of
    scasb
    SAVE_FLAGS
    ASSERT_DI b_of+1
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1
    CHECK_OF 1


; ===================== Single-step SCASB (DF=1) =====================

; 4) Equal byte backward: AL=0xAA vs [DI]=0xAA; DI -= 1; ZF=1, CF=0, PF=1
    std
    mov al, 0xAA
    mov di, back_eq_b+2         ; start at last element
    scasb
    SAVE_FLAGS
    ASSERT_DI back_eq_b+1
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    cld


; ===================== Single-step SCASW (DF=0) =====================

; 5) Equal words: AX=0x1234 vs [DI]=0x1234 → ZF=1; DI += 2
    cld
    mov ax, 0x1234
    mov di, w_eq
    scasw
    SAVE_FLAGS
    ASSERT_DI w_eq+2
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0

; 6) AX - [DI]: 0x0001 - 0x0002 = 0xFFFF → CF=1, SF=1, PF=1, AF=1, OF=0; DI += 2
    cld
    mov ax, 0x0001
    mov di, w_lt
    scasw
    SAVE_FLAGS
    ASSERT_DI w_lt+2
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0

; 7) 0x8000 - 0x7FFF = 0x0001 → OF=1, CF=0, AF=1, SF=0, PF=0; DI += 2
    cld
    mov ax, 0x8000
    mov di, w_of
    scasw
    SAVE_FLAGS
    ASSERT_DI w_of+2
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1
    CHECK_OF 1


; ===================== REP SCASB (exhausting) =====================

; 8) REP SCASB over 3 bytes, AL=3, mem=[3,3,3] → runs 3, CX=0; ZF=1 from last compare; DI +=3
    cld
    mov al, 3
    mov di, rep_all_eq
    mov cx, 3
    rep scasb
    SAVE_FLAGS
    ASSERT_DI rep_all_eq+3
    ASSERT_CX 0
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0

; 9) REP over [9,9,8], AL=9 → last mismatch (9-8=1) → ZF=0, CF=0, PF=0, AF=0; DI +=3, CX=0
    cld
    mov al, 9
    mov di, rep_mislast
    mov cx, 3
    rep scasb
    SAVE_FLAGS
    ASSERT_DI rep_mislast+3
    ASSERT_CX 0
    CHECK_ZF 0
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_OF 0


; ===================== REPE/REPZ SCASB =====================

; 10) REPE stops on 2nd element: AL=1, mem=[1,2,3] → after 2, DI=+2, CX=1; flags from mismatch (1-2)
    cld
    mov al, 1
    mov di, repe_early
    mov cx, 3
    repe scasb
    SAVE_FLAGS
    ASSERT_DI repe_early+2
    ASSERT_CX 1
    CHECK_ZF 0
    CHECK_CF 1
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0

; 11) REPE over 3 equal bytes: AL=4, mem=[4,4,4] → runs all; DI=+3, CX=0; ZF=1
    cld
    mov al, 4
    mov di, repe_all_eq
    mov cx, 3
    repe scasb
    SAVE_FLAGS
    ASSERT_DI repe_all_eq+3
    ASSERT_CX 0
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0


; ===================== REPNE/REPNZ SCASB =====================

; 12) REPNE stops immediately on equality: AL=9, mem=[9,5] → DI=+1, CX=1; ZF=1
    cld
    mov al, 9
    mov di, repne_eq1
    mov cx, 2
    repne scasb
    SAVE_FLAGS
    ASSERT_DI repne_eq1+1
    ASSERT_CX 1
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0

; 13) REPNE with no equality: AL=4, mem=[1,2,3] → runs all, DI=+3, CX=0; flags from 4-3=1
    cld
    mov al, 4
    mov di, repne_none
    mov cx, 3
    repne scasb
    SAVE_FLAGS
    ASSERT_DI repne_none+3
    ASSERT_CX 0
    CHECK_ZF 0
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_OF 0


; ===================== REPE with DF=1 (backward scan) =====================

; 14) REPE backward over 3 equal bytes: AL=0xAA, mem=[AA,AA,AA]; start last; DI -=3, CX=0; ZF=1
    std
    mov al, 0xAA
    mov di, back_eq_b+2
    mov cx, 3
    repe scasb
    SAVE_FLAGS
    ASSERT_DI back_eq_b-1
    ASSERT_CX 0
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    cld

; 15) REPNE backward: AL=7, mem=[5,6,7]; equality at last element (first compare) → DI=base+1, CX=2
    std
    mov al, 7
    mov di, repne_back+2
    mov cx, 3
    repne scasb
    SAVE_FLAGS
    ASSERT_DI repne_back+1
    ASSERT_CX 2
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    cld


; ===================== REP with CX=0 (no-op; DI and flags preserved) =====================

; 16) REP SCASB with CX=0 → no execution; DI unchanged; flags preserved from SAHF
    xor cx, cx
    mov ah, [pat_all1]          ; SF=1, ZF=1, AF=1, PF=1, CF=1 (OF unaffected by SAHF)
    sahf
    mov al, 0x11
    mov di, rep_bytes
    rep scasb
    SAVE_FLAGS
    ASSERT_DI rep_bytes
    ASSERT_CX 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== SCASW with REP (sanity) =====================

; 17) REP SCASW over 2 equal words: AX=0xAAAA, mem=[AAAA,AAAA] → DI +=4, CX=0; ZF=1
    cld
    mov ax, 0xAAAA
    mov di, wrep_eq
    mov cx, 2
    rep scasw
    SAVE_FLAGS
    ASSERT_DI wrep_eq+4
    ASSERT_CX 0
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0
pat_all1:    db 0xD5                ; SF=1, ZF=1, AF=1, PF=1, CF=1 via SAHF

; Single-step bytes
b_eq1:   db 0x34
b_lt:    db 0x02
b_of:    db 0x7F

; Backward equal block (3 identical bytes)
back_eq_b: db 0xAA, 0xAA, 0xAA

; Single-step words
w_eq:    dw 0x1234
w_lt:    dw 0x0002
w_of:    dw 0x7FFF

; REP sources
rep_bytes:     db 1,2,3
rep_mislast:   db 9,9,8
rep_all_eq:    db 3,3,3

repe_early:    db 1,2,3
repe_all_eq:   db 4,4,4

repne_eq1:     db 9,5
repne_none:    db 1,2,3

repne_back:    db 5,6,7

; SCASW + REP
wrep_eq:       dw 0xAAAA, 0xAAAA

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
