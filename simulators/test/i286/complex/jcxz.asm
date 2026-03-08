; jcxz.asm — Thorough tests for JCXZ (CX == 0) in 16-bit mode
; Harness:
;   int 0x23: assert AX == BX   (general equality check)
;   int 0x22: assert AL == AH   (byte equality, often for single-flag checks)

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
%macro ASSERT_SP 1
    mov ax, sp
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_CX 1
    mov ax, cx
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_EQ_FLAGS 0
    ; Compare [pre_flags] to [flags_store]
    mov ax, [flags_store]
    mov bx, [pre_flags]
    int 0x23
%endmacro

; Read-back individual flags from [flags_store] (for spot checks)
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

; Helper: set flags exactly (including OF/DF/etc) via POPF
%macro SET_FLAGS 1
    push word %1
    popf
%endmacro

; Scratch stack helpers (SS=DS so PUSHF/POPF safe)
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
%macro PREP_ODD 0
    mov sp, stack_top - 0x81
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

; ===================== 1) JCXZ taken (CX=0), forward short; flags preserved =====================
t1:
    PREP
    mov ah, [pat_mix]
    sahf
    xor cx, cx                 ; CX=0, logical sets flags, but we’ll re-seed them:
    mov ah, [pat_mix]
    sahf
    ; Snapshot flags before JCXZ:
    pushf
    pop  ax
    mov  [pre_flags], ax
    jcxz t1_taken
    mov ax, 0xDEAD             ; should not run
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS            ; flags unchanged by JCXZ
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    jmp t2

t2_taken:
    mov ax, 0x2BAD
    jmp t2_after

; ===================== 2) JCXZ not taken (CX!=0), forward short; flags preserved =====================
t2:
    PREP
    mov ah, [pat_all0]
    sahf
    mov cx, 1
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t2_taken        ; NOT taken
    mov ax, 0x2222             ; expected
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    jmp t3

; ===================== 3) JCXZ taken (CX=0), backward short =====================
t3_target:
    mov ax, 0x3333
    jmp t3_after
t3:
    PREP
    mov ah, [pat_pf1]          ; PF=1 only
    sahf
    xor cx, cx
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t3_target
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    CHECK_PF 1                 ; from SAHF
    jmp t4

; ===================== 4) JCXZ not taken (CX!=0), backward short =====================
t4_target:
    mov ax, 0x4BAD             ; should not run
    jmp t4_after
t4:
    PREP
    mov ah, [pat_all0]
    sahf
    mov cx, 0xFFFF
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t4_target       ; NOT taken
    mov ax, 0x4444
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    jmp t5

t5_taken:
    mov ax, 0x5BAD
    jmp t5_after

; ===================== 5) CH nonzero, CL zero → CX!=0 → NOT taken =====================
t5:
    PREP
    mov ah, [pat_all0]
    sahf
    mov ch, 0x12
    xor cl, cl                 ; CX = 0x1200
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t5_taken
    mov ax, 0x5555             ; expected
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    ASSERT_CX 0x1200
    jmp t6

t6_taken:
    mov ax, 0x5BAD
    jmp t6_after

; ===================== 6) CH zero, CL nonzero → CX!=0 → NOT taken =====================
t6:
    PREP
    mov ah, [pat_all0]
    sahf
    xor ch, ch
    mov cl, 0x34               ; CX = 0x0034
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t6_taken
    mov ax, 0x6666
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    ASSERT_CX 0x0034

; ===================== 7) DEC CX → zero → taken =====================
t7:
    PREP
    mov cx, 1
    dec cx                     ; CX=0 (ZF=1, OF unaffected for DEC 1->0)
    ; Snapshot flags from DEC; JCXZ must not change them:
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t7_taken
    mov ax, 0x7BAD
    jmp t7_after
t7_taken:
    mov ax, 0x7777
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7777
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    ASSERT_CX 0x0000
    jmp t8

t8_taken:
    mov ax, 0x8BAD
    jmp t8_after

; ===================== 8) DEC CX (nonzero) → not taken =====================
t8:
    PREP
    mov cx, 2
    dec cx                     ; CX=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t8_taken
    mov ax, 0x8888
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    ASSERT_CX 0x0001

; ===================== 9) INC CX: 0xFFFF -> 0x0000 → taken =====================
t9:
    PREP
    mov cx, 0xFFFF
    inc cx                     ; wraps to 0, CF unaffected by INC
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t9_taken
    mov ax, 0x9BAD
    jmp t9_after
t9_taken:
    mov ax, 0x9999
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9999
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    ASSERT_CX 0x0000

; ===================== 10) MOV CX,0 (MOV doesn’t touch flags) → taken, flags from SAHF preserved =====================
t10:
    PREP
    mov ah, [pat_mix]
    sahf
    mov cx, 0
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t10_taken
    mov ax, 0xABAD
    jmp t10_after
t10_taken:
    mov ax, 0xAAAA
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xAAAA
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 11) XOR CX,CX → taken; logical flags preserved across JCXZ =====================
t11:
    PREP
    mov cx, 0x1234
    xor cx, cx                 ; ZF=1, PF depends, CF/OF=0, AF undefined
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t11_taken
    mov ax, 0xBEEF
    jmp t11_after
t11_taken:
    mov ax, 0xBBBB
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xBBBB
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    ASSERT_CX 0x0000

; ===================== 12) DF preserved across JCXZ =====================
t12:
    PREP
    std
    mov cx, 0
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t12_taken
    mov ax, 0xCBAD
    jmp t12_after
t12_taken:
    mov ax, 0xCCCC
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xCCCC
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    CHECK_DF 1
    cld

; ===================== 13) Odd SP (unaligned stack) =====================
t13:
    PREP_ODD
    mov cx, 0
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t13_taken
    mov ax, 0xDBAD
    jmp t13_after
t13_taken:
    mov ax, 0xDDDD
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xDDDD
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    jmp t14

t14_mid:
    mov ax, 0xDBAD
    jmp t14_after

; ===================== 14) Chain: first NOT taken (CX!=0), then zero CX and taken =====================
t14:
    PREP
    mov ah, [pat_all0]
    sahf
    mov cx, 5
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t14_mid         ; not taken
    ; fall-through
    mov cx, 0
    ; flags must remain same as [pre_flags], so don't touch flags here
    jcxz t14_taken       ; taken
    mov ax, 0xE0E0
    jmp short t14_after
t14_taken:
    mov ax, 0xEEEE
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xEEEE
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS

; ===================== 15) Chain: taken first (CX=0), then set CX!=0 and NOT taken =====================
t15:
    PREP
    mov ah, [pat_pf1]
    sahf
    mov cx, 0
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t15_mid         ; taken to mid
    mov ax, 0xAAAA
    jmp short t15_after
t15_mid:
    ; After landing, set CX!=0 and do another JCXZ (not taken)
    mov cx, 1
    jcxz t15_bad
    mov ax, 0xF111
    jmp short t15_after
t15_bad:
    mov ax, 0xBAD0
t15_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    CHECK_PF 1
    jmp t16a

t16a_taken:
    mov ax, 0xF2BD
    jmp t16a_after

; ===================== 16) Register preservation: JCXZ must not change CX (taken & not-taken) =====================
; (a) not taken — CX should remain 0x1234
t16a:
    PREP
    mov cx, 0x1234
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t16a_taken
    mov ax, 0xF222
t16a_after:
    SAVE_FLAGS
    ASSERT_AX 0xF222
    ASSERT_CX 0x1234
    ASSERT_EQ_FLAGS
; (b) taken — CX should remain 0x0000
t16b:
    PREP
    xor cx, cx
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t16b_taken
    mov ax, 0xBEEF
    jmp t16b_after
t16b_taken:
    mov ax, 0xF333
t16b_after:
    SAVE_FLAGS
    ASSERT_AX 0xF333
    ASSERT_CX 0x0000
    ASSERT_EQ_FLAGS

; ===================== 17) OF/CF preset via POPF — prove JCXZ leaves them unchanged =====================
t17:
    PREP
    SET_FLAGS 0x0801            ; OF=1, CF=1 (others 0 except bit1)
    mov cx, 0                   ; do not modify flags after POPF (MOV doesn't)
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t17_taken
    mov ax, 0xACDC
    jmp t17_after
t17_taken:
    mov ax, 0xF444
t17_after:
    SAVE_FLAGS
    ASSERT_AX 0xF444
    ASSERT_EQ_FLAGS
    CHECK_OF 1
    CHECK_CF 1
    jmp t18

t18_taken:
    mov ax, 0xF5BD
    jmp t18_after

; ===================== 18) Big nonzero value (both bytes nonzero) → NOT taken =====================
t18:
    PREP
    mov ah, [pat_mix]
    sahf
    mov cx, 0xA5A5
    pushf
    pop  ax
    mov [pre_flags], ax
    jcxz t18_taken
    mov ax, 0xF555              ; expected
t18_after:
    SAVE_FLAGS
    ASSERT_AX 0xF555
    ASSERT_CX 0xA5A5
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== Exit =====================
exit:
    RESTORE_DOS_STACK
    mov ax, 0x4C00
    int 0x21

; ---------------- Data / scratch ----------------
flags_store: dw 0
pre_flags:   dw 0
orig_ss:     dw 0
orig_sp:     dw 0
sp0_store:   dw 0

; Scratch stack (2 KB)
stack_buf:   times 2048 db 0xCC
stack_top    equ stack_buf + 2048

; SAHF patterns for quick flag seeding (affects CF/PF/AF/ZF/SF)
pat_all0: db 0x00              ; all 0 among CF,PF,AF,ZF,SF
pat_mix : db 0xD5              ; CF=PF=AF=ZF=SF=1
pat_pf1 : db 0x04              ; PF=1 only (others 0)

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
