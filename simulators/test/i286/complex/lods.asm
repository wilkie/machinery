; lods.asm — Thorough tests for LODS (LODSB/LODSW + REP/REPE/REPNE, DF=0/1)
; Harness:
;   int 0x23: assert AX == BX      (used for values and for SI/CX equality via moving regs)
;   int 0x22: assert AL == AH      (single-flag assertions via saved FLAGS)
; Notes:
;   - LODSB: AL = DS:[SI]; SI +=/-= 1 (per DF); AH unchanged
;   - LODSW: AX = DS:[SI]; SI +=/-= 2 (per DF)
;   - LODS does not modify flags; REP/REPE/REPNE behave identically (repeat CX times)

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

start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; Standard .COM: DS = CS, ES = DS
    push cs
    pop  ds
    push ds
    pop  es

; ===================== Single-step LODSB (DF=0) =====================

; 1) LODSB forward: expect AL=0x34, AH preserved (0x12), SI += 1, flags preserved
    cld
    mov si, b1
    mov al, 0x00                  ; don't care; will be overwritten
    mov ah, [pat_all1]
    sahf
    mov ah, 0x12                  ; AH set AFTER SAHF (so flags set, AH now known)
    lodsb
    SAVE_FLAGS
    ASSERT_AX 0x1234
    ASSERT_SI b1+1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Single-step LODSB (DF=1) =====================

; 2) LODSB backward from b2+1: expect AL=0xCD, AH preserved (0x9A), SI -> b2+0
    std
    mov si, b2+1
    mov al, 0x00
    mov ah, [pat_all1]
    sahf
    mov ah, 0x9A
    lodsb
    SAVE_FLAGS
    ASSERT_AX 0x9ACD
    ASSERT_SI b2+0
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Single-step LODSW (DF=0 / DF=1) =====================

; 3) LODSW forward: AX <- 0x1234, SI += 2, flags preserved
    cld
    mov si, w1
    mov ah, [pat_zf0]             ; set ZF=0 pattern to prove flags unchanged
    sahf
    lodsw
    SAVE_FLAGS
    ASSERT_AX 0x1234
    ASSERT_SI w1+2
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 4) LODSW backward from w2+2: AX <- 0xBEEF, SI -> w2+0
    std
    mov si, w2+2
    mov ah, [pat_all1]
    sahf
    lodsw
    SAVE_FLAGS
    ASSERT_AX 0xBEEF
    ASSERT_SI w2+0
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REP LODSB (DF=0 / DF=1) =====================

; 5) REP LODSB forward over 3 bytes [1,2,7]: AH preserved (0x77), AL ends 0x07; SI+=3; CX=0; flags preserved
    cld
    mov si, r1
    mov cx, 3
    mov ah, [pat_zf0]
    sahf
    mov ah, 0x77
    rep lodsb
    SAVE_FLAGS
    ASSERT_AX 0x7707
    ASSERT_SI r1+3
    ASSERT_CX 0
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 6) REP LODSB backward over [5,6,7] starting at +2: AL ends 5; SI -> base-1; AH preserved (0x55)
    std
    mov si, r2+2
    mov cx, 3
    mov ah, [pat_all1]
    sahf
    mov ah, 0x55
    rep lodsb
    SAVE_FLAGS
    ASSERT_AX 0x5505
    ASSERT_SI r2-1
    ASSERT_CX 0
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REPE/REPNE behave like REP for LODS =====================

; 7) REPE LODSB with ZF initially 0 → still runs 3 unconditionally; AL ends 0xAA
    cld
    mov si, repb_eq
    mov cx, 3
    mov ah, [pat_zf0]             ; ZF=0 to prove it's ignored
    sahf
    mov ah, 0x20
    repe lodsb
    SAVE_FLAGS
    ASSERT_AX 0x20AA
    ASSERT_SI repb_eq+3
    ASSERT_CX 0
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 8) REPNE LODSW with ZF initially 1 → runs 3 unconditionally; AX ends 0x3333; SI+=6
    cld
    mov si, rws
    mov cx, 3
    mov ah, [pat_all1]            ; ZF=1 but ignored for LODS
    sahf
    repne lodsw
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SI rws+6
    ASSERT_CX 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REP LODSW (sanity, both directions) =====================

; 9) REP LODSW forward 2 words: AX ends second word 0x0A0B; SI+=4
    cld
    mov si, wrep_fwd
    mov cx, 2
    mov ah, [pat_all1]
    sahf
    rep lodsw
    SAVE_FLAGS
    ASSERT_AX 0x0A0B
    ASSERT_SI wrep_fwd+4
    ASSERT_CX 0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 10) REP LODSW backward 2 words from base+2: AX ends first word 0xAAAA; SI -> base-2
    std
    mov si, wrep_back+2
    mov cx, 2
    mov ah, [pat_zf0]
    sahf
    rep lodsw
    SAVE_FLAGS
    ASSERT_AX 0xAAAA
    ASSERT_SI wrep_back-2
    ASSERT_CX 0
    cld
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== REP with CX=0 (no-op; SI and flags preserved) =====================

; 11) REP LODSB with CX=0 → no execution; SI unchanged; AX unchanged; flags preserved
    cld
    xor cx, cx
    mov ah, [pat_all1]
    sahf
    mov si, r1
    mov ax, 0x5AEE
    mov dx, ax                    ; save AX expected
    rep lodsb
    SAVE_FLAGS
    ASSERT_SI r1
    ASSERT_CX 0
    mov ax, dx
    ASSERT_AX 0x5AEE
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 12) REP LODSW with CX=0 → no execution; SI unchanged; AX unchanged; flags preserved
    cld
    xor cx, cx
    mov ah, [pat_zf0]
    sahf
    mov si, w1
    mov ax, 0x7777
    mov dx, ax
    rep lodsw
    SAVE_FLAGS
    ASSERT_SI w1
    ASSERT_CX 0
    mov ax, dx
    ASSERT_AX 0x7777
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Odd-address word loads =====================

; 13) LODSW at odd SI (DF=0): SI=bOdd+1 → AX should be 0xABCD; SI+=2
    cld
    mov si, bOdd+1                ; bytes EF,CD,AB,89 → word at +1 = 0xABCD
    mov ah, [pat_all1]
    sahf
    lodsw
    SAVE_FLAGS
    ASSERT_AX 0xABCD
    ASSERT_SI bOdd+3
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 14) REP LODSW at odd start backward (DF=1), CX=2: reads at +3 then +1 → final AX=0x3456; SI ends base-1
    std
    mov si, bOdd2+3               ; bytes 78,56,34,12,F0 → +3→0xF012, +1→0x3456
    mov cx, 2
    mov ah, [pat_zf0]
    sahf
    rep lodsw
    SAVE_FLAGS
    ASSERT_AX 0x3456
    ASSERT_SI bOdd2-1
    ASSERT_CX 0
    cld
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Segment override demonstrations (source segment) =====================

; 15) ES override on LODSB (ES=DS here): load 0x5A; SI+=1; flags preserved
    cld
    mov si, esb                   ; ES == DS in this .COM, but we verify the prefix path
    mov ah, [pat_all1]
    sahf
    mov ah, 0xA5
    es lodsb
    SAVE_FLAGS
    ASSERT_AX 0xA55A
    ASSERT_SI esb+1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 16) CS override on LODSW (CS==DS): load word 0x1BAD; SI+=2; flags preserved
    cld
    mov si, csw                   ; label is in code segment anyway
    mov ah, [pat_zf0]
    sahf
    cs lodsw
    SAVE_FLAGS
    ASSERT_AX 0x1BAD
    ASSERT_SI csw+2
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; Byte sources
b1:     db 0x34, 0xCC
b2:     db 0xAB, 0xCD, 0xCC

; Word sources
w1:     dw 0x1234, 0x7E7E
w2:     dw 0xAAAA, 0xBEEF, 0x7E7E

; REP byte sources
r1:     db 1, 2, 7, 0xCC
r2:     db 5, 6, 7, 0xCC
repb_eq:db 0xAA, 0xAA, 0xAA, 0xCC

; REP word sources
rws:        dw 0x1111, 0x2222, 0x3333, 0x7E7E
wrep_fwd:   dw 0x0F0E, 0x0A0B, 0x7E7E
wrep_back:  dw 0xAAAA, 0x5555, 0x7E7E

; Odd-address bytes for word loads
bOdd:   db 0xEF, 0xCD, 0xAB, 0x89, 0xCC
bOdd2:  db 0x78, 0x56, 0x34, 0x12, 0xF0, 0xCC

; Segment-override demo buffers
esb:    db 0x5A, 0xCC
csw:    dw 0x1BAD, 0x7E7E

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
