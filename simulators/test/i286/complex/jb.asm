; jb.asm — Thorough tests for JB/JC/JNAE (CF == 1) in 16-bit mode
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH

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

; Read-back checks from [flags_store]
%macro CHECK_CF 1
    mov ax, [flags_store]
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

; Scratch stack helpers (SS=DS)
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

; ===================== 1) JB taken (CF=1 via SAHF), forward short =====================
t1:
    PREP
    mov ah, [pat_cf1]
    sahf
    jb  short t1_taken
    mov ax, 0xDEAD             ; should not run
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    jmp t2

t2_taken:
    mov ax, 0x2BAD
    jmp t2_after

; ===================== 2) JB not taken (CF=0 via SAHF), forward short =====================
t2:
    PREP
    mov ah, [pat_cf0]
    sahf
    jb  short t2_taken         ; NOT taken
    mov ax, 0x2222             ; expected fall-through
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    jmp t3

; ===================== 3) JB taken (CF=1), backward short =====================
t3_target:
    mov ax, 0x3333
    jmp t3_after
t3:
    PREP
    mov ah, [pat_cf1]
    sahf
    jb  short t3_target
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    jmp t4

; ===================== 4) JB not taken (CF=0), backward short =====================
t4_target:
    mov ax, 0x4BAD             ; should not run
    jmp t4_after
t4:
    PREP
    mov ah, [pat_cf0]
    sahf
    jb  short t4_target        ; NOT taken
    mov ax, 0x4444
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP [sp0_store]
    CHECK_CF 0

; ===================== 5) SUB with borrow → CF=1 → taken =====================
; 0x01 - 0x02 → CF=1
t5:
    PREP
    mov al, 1
    sub al, 2
    jb  short t5_taken
    mov ax, 0x5BAD
    jmp t5_after
t5_taken:
    mov ax, 0x5555
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    jmp t6

t6_taken:
    mov ax, 0x6BAD
    jmp t6_after

; ===================== 6) SUB without borrow → CF=0 → NOT taken =====================
; 0x05 - 0x02 → CF=0
t6:
    PREP
    mov al, 5
    sub al, 2
    jb  short t6_taken
    mov ax, 0x6666
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_SP [sp0_store]
    CHECK_CF 0

; ===================== 7) ADD with carry → CF=1 → taken =====================
; 0xFF + 0x01 → 0x00, CF=1
t7:
    PREP
    mov al, 0xFF
    add al, 1
    jb  short t7_taken
    mov ax, 0x7BAD
    jmp t7_after
t7_taken:
    mov ax, 0x7777
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7777
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_ZF 1                  ; incidental from ADD here
    jmp t8

t8_taken:
    mov ax, 0x8BAD
    jmp t8_after

; ===================== 8) ADD without carry → CF=0 → NOT taken =====================
; 0x01 + 0x01 → CF=0
t8:
    PREP
    mov al, 1
    add al, 1
    jb  short t8_taken
    mov ax, 0x8888
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    ASSERT_SP [sp0_store]
    CHECK_CF 0

; ===================== 9) CMP A<B (unsigned) → CF=1 → taken =====================
t9:
    PREP
    mov al, 0x10
    cmp al, 0x20               ; borrow → CF=1
    jb  short t9_taken
    mov ax, 0x9BAD
    jmp t9_after
t9_taken:
    mov ax, 0x9999
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9999
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_ZF 0
    jmp t10

t10_taken:
    mov ax, 0xABAD
    jmp t10_after

; ===================== 10) CMP equal → CF=0 → NOT taken =====================
t10:
    PREP
    mov al, 0x55
    cmp al, 0x55               ; ZF=1, CF=0
    jb  short t10_taken        ; NOT taken
    mov ax, 0xAAAA
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xAAAA
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 1
    jmp t11

t11_taken:
    mov ax, 0xBBAD
    jmp t11_after

; ===================== 11) CMP A>B → CF=0 → NOT taken =====================
t11:
    PREP
    mov al, 0x30
    cmp al, 0x20               ; CF=0
    jb  short t11_taken
    mov ax, 0xBBBB
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xBBBB
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    jmp t12

t12_taken:
    mov ax, 0xCBAD
    jmp t12_after

; ===================== 12) TEST clears CF → NOT taken =====================
t12:
    PREP
    mov al, 0x01
    test al, al                ; CF=0, OF=0, ZF=0, PF=0/1; AF undefined
    jb  short t12_taken
    mov ax, 0xCCCC
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xCCCC
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    ; (Don’t assert AF after TEST: undefined.)
    jmp t13

t13_taken:
    mov ax, 0xDBAD
    jmp t13_after

; ===================== 13) OR clears CF → NOT taken (even if result zero) =====================
t13:
    PREP
    xor al, al
    or  al, 0                  ; ZF=1, CF=0
    jb  short t13_taken
    mov ax, 0xDDDD
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xDDDD
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 1

; ===================== 14) Flags preserved by JB (seed PF/AF/ZF/SF=1, CF=1) → taken =====================
t14:
    PREP
    mov ah, [pat_mix_cf1]
    sahf
    jb  short t14_taken
    mov ax, 0xEEEE             ; should not run
    jmp t14_after
t14_taken:
    mov ax, 0xE111
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xE111
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    jmp t15

t15_taken:
    mov ax, 0xFBAD
    jmp t15_after

; ===================== 15) Flags preserved when NOT taken (PF/AF/SF=1, CF=0) =====================
t15:
    PREP
    mov ah, [pat_mix_cf0]
    sahf
    jb  short t15_taken        ; NOT taken
    mov ax, 0xF111
t15_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 16) DF preserved across JB =====================
t16:
    PREP
    std
    mov ah, [pat_cf1]
    sahf
    jb  short t16_taken
    mov ax, 0x0BAD
    jmp t16_after
t16_taken:
    mov ax, 0xF222
t16_after:
    SAVE_FLAGS
    ASSERT_AX 0xF222
    ASSERT_SP [sp0_store]
    CHECK_DF 1
    cld

; ===================== 17) Odd SP (unaligned stack) =====================
t17:
    PREP_ODD
    mov ah, [pat_cf1]
    sahf
    jb  short t17_taken
    mov ax, 0x0DAD
    jmp t17_after
t17_taken:
    mov ax, 0xF333
t17_after:
    SAVE_FLAGS
    ASSERT_AX 0xF333
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    jmp t18

t18_mid:
    mov ax, 0xF4BD
    jmp t18_after

; ===================== 18) Chain: first NOT taken (CF=0), then set CF=1 → taken =====================
t18:
    PREP
    mov ah, [pat_cf0]
    sahf
    jb  short t18_mid          ; NOT taken
    ; fall-through
    mov ah, [pat_cf1]
    sahf
    jb  short t18_taken        ; taken
    mov ax, 0xFACE
    jmp short t18_after
t18_taken:
    mov ax, 0xF444
t18_after:
    SAVE_FLAGS
    ASSERT_AX 0xF444
    ASSERT_SP [sp0_store]
    CHECK_CF 1

; ===================== 19) STC/CLC direct control =====================
; (a) STC → CF=1 → taken
t19a:
    PREP
    clc
    stc
    jb  short t19a_taken
    mov ax, 0xBEEF
    jmp t19a_after
t19a_taken:
    mov ax, 0xF555
t19a_after:
    SAVE_FLAGS
    ASSERT_AX 0xF555
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    jmp t19b

t19b_taken:
    mov ax, 0xF6BD
    jmp t19b_after

; (b) CLC → CF=0 → NOT taken
t19b:
    PREP
    stc
    clc
    jb  short t19b_taken
    mov ax, 0xF666
t19b_after:
    SAVE_FLAGS
    ASSERT_AX 0xF666
    ASSERT_SP [sp0_store]
    CHECK_CF 0

; ===================== Exit =====================
exit:
    RESTORE_DOS_STACK
    mov ax, 0x4C00
    int 0x21

; ---------------- Data / scratch ----------------
flags_store: dw 0
orig_ss:     dw 0
orig_sp:     dw 0
sp0_store:   dw 0

; Scratch stack (2 KB)
stack_buf:   times 2048 db 0xCC
stack_top    equ stack_buf + 2048

; SAHF patterns (bit0=CF)
pat_cf0:     db 0x00            ; CF=0 (others 0)
pat_cf1:     db 0x01            ; CF=1 only
pat_mix_cf1: db 0xD5            ; SF=ZF=AF=PF=CF=1 (JB should be taken)
pat_mix_cf0: db 0x94            ; SF=1, AF=1, PF=1, ZF=0, CF=0

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
