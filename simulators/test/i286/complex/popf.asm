; popf.asm — Thorough tests for POPF (16-bit)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH  (single-flag checks via saved FLAGS)

bits 16
org 0x100

; ---------- Macros ----------
%macro SAVE_FLAGS 0
    pushf
    pop ax
    mov [flags_store], ax
%endmacro

%macro DO_POPF 1
    mov ax, %1
    push ax
    popf
    SAVE_FLAGS
%endmacro

%macro ASSERT_SP 1
    mov ax, sp
    mov bx, %1
    int 0x23
%endmacro

%macro ASSERT_AX 1
    mov bx, %1
    int 0x23
%endmacro

; Flag checks read from [flags_store] (so the act of checking doesn't change FLAGS)
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
%macro CHECK_TF 1
    mov ax, [flags_store]
    mov cl, 8
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro
%macro CHECK_IF 1
    mov ax, [flags_store]
    mov cl, 9
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro
%macro CHECK_DF 1
    mov ax, [flags_store]
    mov cl, 10
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

; Reserved/ignored bit checks (bit1 must be 1; bits3 & 5 must be 0)
%macro CHECK_B1_FORCED_1 0
    mov ax, [flags_store]
    mov cl, 1
    shr ax, cl
    and al, 1
    mov ah, 1
    int 0x22
%endmacro
%macro CHECK_B3_ZERO 0
    mov ax, [flags_store]
    mov cl, 3
    shr ax, cl
    and al, 1
    mov ah, 0
    int 0x22
%endmacro
%macro CHECK_B5_ZERO 0
    mov ax, [flags_store]
    mov cl, 5
    shr ax, cl
    and al, 1
    mov ah, 0
    int 0x22
%endmacro

; Scratch stack helpers (SS = DS, SP to our buffer)
%macro SET_SCRATCH_STACK 0
    cli
    mov [orig_ss], ss
    mov [orig_sp], sp
    mov ax, ds
    mov ss, ax
    mov sp, stack_top
    sti
%endmacro
%macro RESTORE_DOS_STACK 0
    cli
    mov ax, [orig_ss]
    mov ss, ax
    mov sp, [orig_sp]
    sti
%endmacro
%macro PREP 0
    mov sp, stack_top - 0x80
    mov [sp0_store], sp
%endmacro

; ---------- Start ----------
start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; DS=CS, ES=DS
    push cs
    pop  ds
    push ds
    pop  es

    SET_SCRATCH_STACK

; Handy constant: all definable flags ON (CF,PF,AF,ZF,SF,IF,DF,OF), TF off
ALL_ON     equ 0x0ED5           ; 0000 1110 1101 0101b

; ===================== 1) POPF ← 0x0000 (all clear; TF stays 0) =====================
t1:
    PREP
    DO_POPF 0x0000
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_TF 0
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 0
    CHECK_B1_FORCED_1
    CHECK_B3_ZERO
    CHECK_B5_ZERO

; ===================== 2) POPF ← ALL_ON with noisy reserved bits set (should normalize) =====================
; We deliberately set bit1=0 and bits3,5=1 and also high bits; TF still 0.
t2:
    PREP
    DO_POPF (ALL_ON | 0x8000 | 0x0028) & ~0x0002    ; 0x8EFD with bit1 cleared
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_TF 0
    CHECK_IF 1
    CHECK_DF 1
    CHECK_OF 1
    CHECK_B1_FORCED_1
    CHECK_B3_ZERO
    CHECK_B5_ZERO

; ===================== 3) POPF ← IF only =====================
t3:
    PREP
    DO_POPF 0x0200
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_TF 0
    CHECK_IF 1
    CHECK_DF 0
    CHECK_OF 0
    CHECK_B1_FORCED_1
    CHECK_B3_ZERO
    CHECK_B5_ZERO

; ===================== 4) POPF ← DF only =====================
t4:
    PREP
    DO_POPF 0x0400
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_TF 0
    CHECK_IF 0
    CHECK_DF 1
    CHECK_OF 0
    CHECK_B1_FORCED_1
    CHECK_B3_ZERO
    CHECK_B5_ZERO

; ===================== 5) POPF ← OF only =====================
t5:
    PREP
    DO_POPF 0x0800
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_TF 0
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 1
    CHECK_B1_FORCED_1
    CHECK_B3_ZERO
    CHECK_B5_ZERO

; ===================== 6) POPF ← SF|ZF =====================
t6:
    PREP
    DO_POPF 0x00C0
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_TF 0
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 0
    CHECK_B1_FORCED_1
    CHECK_B3_ZERO
    CHECK_B5_ZERO

; ===================== 7) POPF ← PF|AF =====================
t7:
    PREP
    DO_POPF 0x0014
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_TF 0
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 0
    CHECK_B1_FORCED_1
    CHECK_B3_ZERO
    CHECK_B5_ZERO

; ===================== 8) POPF ← CF only =====================
t8:
    PREP
    DO_POPF 0x0001
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_TF 0
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 0
    CHECK_B1_FORCED_1
    CHECK_B3_ZERO
    CHECK_B5_ZERO

; ===================== 9) POPF ← mixed CF|PF|IF|DF (0x0605) =====================
t9:
    PREP
    DO_POPF 0x0605
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_TF 0
    CHECK_IF 1
    CHECK_DF 1
    CHECK_OF 0
    CHECK_B1_FORCED_1
    CHECK_B3_ZERO
    CHECK_B5_ZERO

; ===================== 10) POPF with only reserved bits set (bit1 cleared; bits3,5 set) =====================
; Expect: definable flags all 0; bit1 forced 1; bits3,5 forced 0.
t10:
    PREP
    DO_POPF 0x0028              ; 0010 1000b → bits3,5=1; bit1=0
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_TF 0
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 0
    CHECK_B1_FORCED_1
    CHECK_B3_ZERO
    CHECK_B5_ZERO

; ===================== 11) Unaligned SP source (odd SP) =====================
t11:
    mov sp, stack_top - 0x81     ; odd SP allowed
    mov [sp0_store], sp
    DO_POPF 0x0ED5               ; all definable flags on
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_TF 0
    CHECK_IF 1
    CHECK_DF 1
    CHECK_OF 1
    CHECK_B1_FORCED_1
    CHECK_B3_ZERO
    CHECK_B5_ZERO

; ===================== 12) Idempotence: PUSHF → (pop to AX) → PUSH AX → POPF =====================
; Expected: final FLAGS equals the original PUSHF image (normalized).
t12:
    PREP
    ; capture current normalized flags image
    pushf
    pop ax
    mov [flags_store_ref], ax
    ; now re-load it with POPF
    push ax
    popf
    SAVE_FLAGS
    ; compare saved vs final
    mov ax, [flags_store]
    mov bx, [flags_store_ref]
    int 0x23
    ASSERT_SP [sp0_store]
    ; sanity: TF remains 0 (we never set it here)
    CHECK_TF 0
    ; reserved normalization should match in both images, so no extra asserts needed

; ===================== Exit =====================
exit:
    RESTORE_DOS_STACK
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store:      dw 0
flags_store_ref:  dw 0
orig_ss:          dw 0
orig_sp:          dw 0
sp0_store:        dw 0

; Scratch stack (2 KB)
stack_buf:   times 2048 db 0xCC
stack_top    equ stack_buf + 2048

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
