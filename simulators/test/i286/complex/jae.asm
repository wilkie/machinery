; jae.asm — Thorough tests for JAE/JNB (CF == 0) in 16-bit mode
; Harness:
;   int 0x23: assert AX == BX
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
%macro ASSERT_SP 1
    mov ax, sp
    mov bx, %1
    int 0x23
%endmacro

; Flag checks (read from [flags_store])
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

; ===================== 1) JAE taken (CF=0 via SAHF), forward short =====================
t1:
    PREP
    mov ah, [pat_cf0_all0]
    sahf
    jae short t1_taken
    mov ax, 0xDEAD             ; should not run
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; ===================== 2) JAE not taken (CF=1 via SAHF), forward short =====================
t2:
    PREP
    mov ah, [pat_cf1_only]
    sahf
    jae short t2_taken         ; not taken
    mov ax, 0x2222             ; expected fall-through
    jmp short t2_after
t2_taken:
    mov ax, 0x2BAD
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    CHECK_CF 1

; ===================== 3) JAE taken (CF=0), backward short =====================
t3_target:
    mov ax, 0x3333
    jmp t3_after
t3:
    PREP
    mov ah, [pat_cf0_all0]
    sahf
    jae short t3_target
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP [sp0_store]
    CHECK_CF 0

; ===================== 4) JAE not taken (CF=1), backward short =====================
t4_target:
    mov ax, 0x4BAD             ; should not run
t4:
    PREP
    mov ah, [pat_cf1_only]
    sahf
    jae short t4_target        ; not taken
    mov ax, 0x4444
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP [sp0_store]
    CHECK_CF 1

; ===================== 5) JAE after ADD without carry → taken (CF=0) =====================
; 0x01 + 0x01 = 0x02 → CF=0
t5:
    PREP
    mov al, 1
    add al, 1
    jae short t5_taken
    mov ax, 0x5BAD
    jmp t5_after
t5_taken:
    mov ax, 0x5555
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    ASSERT_SP [sp0_store]
    CHECK_CF 0

; ===================== 6) JAE after ADD with carry → not taken (CF=1) =====================
; 0xFF + 0x01 = 0x00, CF=1, ZF=1
t6:
    PREP
    mov al, 0xFF
    add al, 1
    jae short t6_taken         ; not taken
    mov ax, 0x6666
    jmp short t6_after
t6_taken:
    mov ax, 0x6BAD
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_ZF 1

; ===================== 7) JAE after SUB with borrow → not taken (CF=1) =====================
; 0x01 - 0x02 → CF=1
t7:
    PREP
    mov al, 1
    sub al, 2
    jae short t7_taken
    mov ax, 0x7777             ; expected path
    jmp short t7_after
t7_taken:
    mov ax, 0x7BAD
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7777
    ASSERT_SP [sp0_store]
    CHECK_CF 1

; ===================== 8) JAE after SUB without borrow → taken (CF=0) =====================
; 0x05 - 0x02 → CF=0
t8:
    PREP
    mov al, 5
    sub al, 2
    jae short t8_taken
    mov ax, 0x8BAD
    jmp t8_after
t8_taken:
    mov ax, 0x8888
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    ASSERT_SP [sp0_store]
    CHECK_CF 0

; ===================== 9) JAE after CMP: equal (A==B) → CF=0 → taken =====================
t9:
    PREP
    mov al, 0x10
    cmp al, 0x10               ; equal → CF=0, ZF=1
    jae short t9_taken
    mov ax, 0x9BAD
    jmp t9_after
t9_taken:
    mov ax, 0x9999
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9999
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 1

; ===================== 10) JAE after CMP: A<B unsigned → CF=1 → not taken =====================
t10:
    PREP
    mov al, 0x10
    cmp al, 0x20               ; borrow → CF=1
    jae short t10_taken
    mov ax, 0xAAAA
    jmp short t10_after
t10_taken:
    mov ax, 0xABAD
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xAAAA
    ASSERT_SP [sp0_store]
    CHECK_CF 1

; ===================== 11) Flags preserved by JAE (pattern via SAHF) =====================
; Seed PF/AF/ZF/SF=1, CF=0 so jump is taken.
t11:
    PREP
    mov ah, [pat_mix_cf0]
    sahf
    jae short t11_taken
    mov ax, 0xBADD
    jmp t11_after
t11_taken:
    mov ax, 0xBBBB
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xBBBB
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 12) DF preserved across JAE =====================
t12:
    PREP
    std
    mov ah, [pat_cf0_all0]     ; CF=0
    sahf
    jae short t12_taken
    mov ax, 0xCBAD
    jmp t12_after
t12_taken:
    mov ax, 0xCCCC
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xCCCC
    ASSERT_SP [sp0_store]
    CHECK_DF 1
    cld

; ===================== 13) Odd SP (unaligned stack) =====================
t13:
    PREP_ODD
    mov ah, [pat_cf0_all0]
    sahf
    jae short t13_taken
    mov ax, 0xDBAD
    jmp t13_after
t13_taken:
    mov ax, 0xDDDD
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xDDDD
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    jmp t14

t14_mid:
   mov ax, 0xFBAD
   jmp t14_after

; ===================== 14) Chain: first not taken (CF=1), then set CF=0 and taken =====================
t14:
    PREP
    mov ah, [pat_cf1_only]
    sahf
    jae short t14_mid          ; NOT taken
    ; fall-through
    mov ah, [pat_cf0_all0]     ; CF=0 now
    sahf
    jae short t14_taken        ; taken
    mov ax, 0xEBAD
    jmp short t14_after
t14_taken:
    mov ax, 0xEEEE
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xEEEE
    ASSERT_SP [sp0_store]
    CHECK_CF 0

; ===================== 15) STC/CLC direct CF control =====================
; (a) STC → CF=1 → not taken
t15a:
    PREP
    stc
    jae short t15a_taken
    mov ax, 0xF0F0             ; expected fall-through
    jmp short t15a_after
t15a_taken:
    mov ax, 0xFBAD             ; never run
t15a_after:
    SAVE_FLAGS
    ASSERT_AX 0xF0F0
    ASSERT_SP [sp0_store]
    CHECK_CF 1
; (b) CLC → CF=0 → taken
t15b:
    PREP
    clc
    jae short t15b_taken
    mov ax, 0xFACE
    jmp t15b_after
t15b_taken:
    mov ax, 0xF111
t15b_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
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

; SAHF patterns (affect CF/PF/AF/ZF/SF only)
pat_cf0_all0: db 0x00           ; CF=0, others 0
pat_cf1_only: db 0x01           ; CF=1 only
pat_mix_cf0 : db 0xD4           ; SF=ZF=AF=PF=1, CF=0
pat_mix_cf1 : db 0xD5           ; SF=ZF=AF=PF=CF=1

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
