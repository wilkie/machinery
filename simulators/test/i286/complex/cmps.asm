; cmps.asm — Thorough tests for CMPS (CMPSB/CMPSW + REP/REPE/REPNE)
; Harness:
;   int 0x23: assert AX == BX  (used for bytes/words and for SI/DI/CX)
;   int 0x22: assert AL == AH  (single-flag assertions from saved FLAGS)

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

    ; Standard .COM: DS = CS, ES = DS (so DS:SI and ES:DI see same segment)
    push cs
    pop  ds
    push ds
    pop  es

; ===================== Single-step CMPSB (DF=0) =====================

; 1) Equal bytes: 0x34 vs 0x34 → ZF=1, CF=0, SF=0, PF=1, AF=0, OF=0; SI/DI += 1
    cld
    mov si, b_eq1_a
    mov di, b_eq1_b
    cmpsb
    SAVE_FLAGS
    ASSERT_SI b_eq1_a+1
    ASSERT_DI b_eq1_b+1
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0

; 2) DS:SI < ES:DI: 0x01 - 0x02 = 0xFF → CF=1, ZF=0, SF=1, PF=1, AF=1, OF=0
    cld
    mov si, b_lt_a
    mov di, b_lt_b
    cmpsb
    SAVE_FLAGS
    ASSERT_SI b_lt_a+1
    ASSERT_DI b_lt_b+1
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0

; 3) DS:SI > ES:DI with signed overflow: 0x80 - 0x7F = 0x01
;    → CF=0, ZF=0, SF=0, PF=0, AF=1 (0x00-0x0F nibble borrow), OF=1
    cld
    mov si, b_of_a
    mov di, b_of_b
    cmpsb
    SAVE_FLAGS
    ASSERT_SI b_of_a+1
    ASSERT_DI b_of_b+1
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1
    CHECK_OF 1


; ===================== Single-step CMPSB (DF=1) =====================

; 4) Equal bytes backward: 0xAA vs 0xAA; SI/DI -= 1; ZF=1, CF=0
    std
    mov si, back_eq_a+2
    mov di, back_eq_b+2
    cmpsb
    SAVE_FLAGS
    ASSERT_SI back_eq_a+1
    ASSERT_DI back_eq_b+1
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    cld


; ===================== Single-step CMPSW (DF=0) =====================

; 5) Equal words: 0x1234 vs 0x1234 → ZF=1; SI/DI += 2
    cld
    mov si, w_eq_a
    mov di, w_eq_b
    cmpsw
    SAVE_FLAGS
    ASSERT_SI w_eq_a+2
    ASSERT_DI w_eq_b+2
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0

; 6) 0x0001 - 0x0002 = 0xFFFF → CF=1, ZF=0, SF=1, PF=1, AF=1, OF=0; SI/DI += 2
    cld
    mov si, w_lt_a
    mov di, w_lt_b
    cmpsw
    SAVE_FLAGS
    ASSERT_SI w_lt_a+2
    ASSERT_DI w_lt_b+2
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_OF 0

; 7) 0x8000 - 0x7FFF = 0x0001 → OF=1, CF=0, AF=1, SF=0, PF=0; SI/DI += 2
    cld
    mov si, w_of_a
    mov di, w_of_b
    cmpsw
    SAVE_FLAGS
    ASSERT_SI w_of_a+2
    ASSERT_DI w_of_b+2
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 1
    CHECK_OF 1


; ===================== REP CMPSB (unconditional) =====================

; 8) REP CMPSB over 3 equal bytes → runs 3 times, CX=0; ZF=1 from last compare; SI/DI +=3
    cld
    mov si, rep_eq_src
    mov di, rep_eq_dst
    mov cx, 3
    rep cmpsb
    SAVE_FLAGS
    ASSERT_SI rep_eq_src+3
    ASSERT_DI rep_eq_dst+3
    ASSERT_CX 0
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0

; 9) REP CMPSB over [1,2,9] vs [1,2,8] → last compare mismatches (9-8=1)
;    → runs full 3, CX=0; flags from last: ZF=0, CF=0, SF=0, PF=0, AF=0, OF=0; SI/DI +=3
    cld
    mov si, rep_mislast_src
    mov di, rep_mislast_dst
    mov cx, 3
    rep cmpsb
    SAVE_FLAGS
    ASSERT_SI rep_mislast_src+3
    ASSERT_DI rep_mislast_dst+3
    ASSERT_CX 0
    CHECK_ZF 0
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_OF 0


; ===================== REPE/REPZ CMPSB =====================

; 10) REPE stops early on 2nd element: [1,9,3] vs [1,2,3], CX=3
;     After 2 compares (mismatch 9-2): SI/DI +=2, CX=1; flags from mismatch
    cld
    mov si, repe_early_src
    mov di, repe_early_dst
    mov cx, 3
    repe cmpsb
    SAVE_FLAGS
    ASSERT_SI repe_early_src+2
    ASSERT_DI repe_early_dst+2
    ASSERT_CX 1
    ; mismatch 0x09-0x02 = 0x07
    CHECK_ZF 0
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_OF 0

; 11) REPE over 3 equal bytes → runs all; CX=0; ZF=1; SI/DI +=3
    cld
    mov si, repe_all_eq_src
    mov di, repe_all_eq_dst
    mov cx, 3
    repe cmpsb
    SAVE_FLAGS
    ASSERT_SI repe_all_eq_src+3
    ASSERT_DI repe_all_eq_dst+3
    ASSERT_CX 0
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0


; ===================== REPNE/REPNZ CMPSB =====================

; 12) REPNE stops immediately on equality at 1st element: [9,7] vs [9,5], CX=2
;     After 1 compare: SI/DI +=1, CX=1; flags from equality (ZF=1)
    cld
    mov si, repne_eq1_src
    mov di, repne_eq1_dst
    mov cx, 2
    repne cmpsb
    SAVE_FLAGS
    ASSERT_SI repne_eq1_src+1
    ASSERT_DI repne_eq1_dst+1
    ASSERT_CX 1
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0

; 13) REPNE with no equality in 3 bytes: [1,2,3] vs [4,5,6]
;     → runs all 3, CX=0; flags from last mismatch (3-6=0xFD)
    cld
    mov si, repne_none_src
    mov di, repne_none_dst
    mov cx, 3
    repne cmpsb
    SAVE_FLAGS
    ASSERT_SI repne_none_src+3
    ASSERT_DI repne_none_dst+3
    ASSERT_CX 0
    CHECK_ZF 0
    CHECK_CF 1
    CHECK_SF 1
    CHECK_PF 0
    CHECK_AF 1
    CHECK_OF 0


; ===================== REPE with DF=1 (backward scan) =====================

; 14) REPE over 3 equal bytes backward: start at last element; SI/DI -=3, CX=0; ZF=1
    std
    mov si, back_eq_a+2
    mov di, back_eq_b+2
    mov cx, 3
    repe cmpsb
    SAVE_FLAGS
    ASSERT_SI back_eq_a-1
    ASSERT_DI back_eq_b-1
    ASSERT_CX 0
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    cld

; 15) REPNE backward: equality at last element (first compare) → SI/DI -=1, CX=2; ZF=1
    std
    mov si, repne_back_src+2
    mov di, repne_back_dst+2
    mov cx, 3
    repne cmpsb
    SAVE_FLAGS
    ASSERT_SI repne_back_src+1
    ASSERT_DI repne_back_dst+1
    ASSERT_CX 2
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_SF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_OF 0
    cld


; ===================== REP with CX=0 (no-op; flags and SI/DI preserved) =====================

; 16) REP CMPSB with CX=0 → no execution; SI/DI unchanged; flags preserved
    ; preset flags pattern: SF=1, ZF=1, AF=1, PF=1, CF=1 (OF unaffected by SAHF)
    xor cx, cx
    mov ah, [pat_all1]
    sahf
    mov si, rep_eq_src
    mov di, rep_eq_dst
    rep cmpsb
    SAVE_FLAGS
    ASSERT_SI rep_eq_src
    ASSERT_DI rep_eq_dst
    ASSERT_CX 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== CMPSW with REP (sanity) =====================

; 17) REP CMPSW over 2 equal words → SI/DI +=4, CX=0; ZF=1 from last compare
    cld
    mov si, wrep_eq_src
    mov di, wrep_eq_dst
    mov cx, 2
    rep cmpsw
    SAVE_FLAGS
    ASSERT_SI wrep_eq_src+4
    ASSERT_DI wrep_eq_dst+4
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
pat_all1:    db 0xD5          ; SF=1, ZF=1, AF=1, PF=1, CF=1 via SAHF

; Single-step bytes
b_eq1_a: db 0x34
b_eq1_b: db 0x34

b_lt_a:  db 0x01
b_lt_b:  db 0x02

b_of_a:  db 0x80
b_of_b:  db 0x7F

; Backward equal (3 bytes total; we start at index 2)
back_eq_a: db 0x11, 0x22, 0x33
back_eq_b: db 0x11, 0x22, 0x33

; Single-step words
w_eq_a:  dw 0x1234
w_eq_b:  dw 0x1234

w_lt_a:  dw 0x0001
w_lt_b:  dw 0x0002

w_of_a:  dw 0x8000
w_of_b:  dw 0x7FFF

; REP equal
rep_eq_src:     db 1,2,3
rep_eq_dst:     db 1,2,3

; REP last mismatch
rep_mislast_src: db 1,2,9
rep_mislast_dst: db 1,2,8

; REPE early stop at 2nd
repe_early_src: db 1,9,3
repe_early_dst: db 1,2,3

; REPE all equal
repe_all_eq_src: db 4,5,6
repe_all_eq_dst: db 4,5,6

; REPNE: equal at 1st element
repne_eq1_src: db 9,7
repne_eq1_dst: db 9,5

; REPNE: none equal
repne_none_src: db 1,2,3
repne_none_dst: db 4,5,6

; REPNE backward: equality at last element first
repne_back_src: db 5,6,7
repne_back_dst: db 9,8,7

; CMPSW + REP
wrep_eq_src: dw 0xAAAA, 0x5555
wrep_eq_dst: dw 0xAAAA, 0x5555

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
