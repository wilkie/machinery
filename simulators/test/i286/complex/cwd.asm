; cwd.asm — Thorough tests for CWD (Convert Word to Doubleword)
; Harness:
;   int 0x23: assert AX == BX  (used for value/register equality)
;   int 0x22: assert AL == AH  (single-flag checks via saved FLAGS)

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

%macro ASSERT_DX 1
    push ax
    mov ax, dx
    mov bx, %1
    int 0x23
    pop ax
%endmacro

%macro ASSERT_AX_EQ_CX 0
    mov bx, cx
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

; DF isn’t settable via SAHF, so we check it explicitly in DF tests
%macro CHECK_DF 1
    mov ax, [flags_store]
    mov cl, 10
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

; ===================== Basics: AX ≥ 0 → DX=0000 =====================

; 1) AX=0000h → DX=0000h; AX unchanged; flags preserved
    cld
    mov ah, [pat_all1]
    sahf
    mov ax, 0x0000
    mov cx, ax                     ; save expected AX
    cwd
    SAVE_FLAGS
    ASSERT_DX 0x0000
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 2) AX=0001h → DX=0000h
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x0001
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0x0000
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 3) AX=7FFFh → DX=0000h (largest non-negative)
    mov ah, [pat_all1]
    sahf
    mov ax, 0x7FFF
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0x0000
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 4) AX=0100h → DX=0000h
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x0100
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0x0000
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 5) AX=00FFh → DX=0000h
    mov ah, [pat_all1]
    sahf
    mov ax, 0x00FF
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0x0000
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; ===================== Negatives: AX < 0 → DX=FFFF =====================

; 6) AX=8000h → DX=FFFFh (most negative)
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x8000
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0xFFFF
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 7) AX=8001h → DX=FFFFh
    mov ah, [pat_all1]
    sahf
    mov ax, 0x8001
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0xFFFF
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 8) AX=FFFFh → DX=FFFFh
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xFFFF
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0xFFFF
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 9) AX=FF00h → DX=FFFFh
    mov ah, [pat_all1]
    sahf
    mov ax, 0xFF00
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0xFFFF
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 10) AX=F000h → DX=FFFFh
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xF000
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0xFFFF
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; ===================== Memory-sourced AX =====================

; 11) AX <- [pos1]=3333h → DX=0000h
    mov ah, [pat_all1]
    sahf
    mov ax, [pos1]
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0x0000
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 12) AX <- [neg1]=C135h → DX=FFFFh
    mov ah, [pat_zf0]
    sahf
    mov ax, [neg1]
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0xFFFF
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== DF = 1 cases (DF unaffected by CWD) =====================

; 13) DF=1, AX positive → DX=0000h; DF stays 1
    std
    mov ah, [pat_all1]
    sahf
    mov ax, 0x1234
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0x0000
    ASSERT_AX_EQ_CX
    CHECK_DF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    cld

; 14) DF=1, AX negative → DX=FFFFh; DF stays 1
    std
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x8002
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0xFFFF
    ASSERT_AX_EQ_CX
    CHECK_DF 1
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    cld

; ===================== Idempotence / repeat =====================

; 15) Repeating CWD doesn’t change state (AX=89ABh → DX=0000h twice)
    mov ah, [pat_all1]
    sahf
    mov ax, 0x89AB & 0x7FFF       ; make it positive (just to be explicit)
    mov ax, 0x09AB                ; AX positive
    mov cx, ax
    cwd
    cwd                            ; do it again
    SAVE_FLAGS
    ASSERT_DX 0x0000
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 16) Repeating CWD with negative AX (AX=E123h) keeps DX=FFFFh
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xE123
    mov cx, ax
    cwd
    cwd
    SAVE_FLAGS
    ASSERT_DX 0xFFFF
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Mixed boundary re-checks =====================

; 17) AX=7FFFh (boundary) again after a negative case
    mov ah, [pat_all1]
    sahf
    mov ax, 0x7FFF
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0x0000
    ASSERT_AX_EQ_CX
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 18) AX=8000h (boundary) again after a positive case
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x8000
    mov cx, ax
    cwd
    SAVE_FLAGS
    ASSERT_DX 0xFFFF
    ASSERT_AX_EQ_CX
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
pos1:       dw 0x3333
neg1:       dw 0xC135

; --------- SAHF flag seed patterns (OF unaffected; CWD doesn’t change any flags anyway) ---------
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
