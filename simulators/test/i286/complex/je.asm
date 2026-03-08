; je.asm — Thorough tests for JE/JZ (ZF == 1) in 16-bit mode
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
%macro CHECK_ZF 1
    mov ax, [flags_store]
    mov cl, 6
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro
%macro CHECK_CF 1
    mov ax, [flags_store]
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

; ===================== 1) JE taken (ZF=1 via SAHF), forward short =====================
t1:
    PREP
    mov ah, [pat_zf1]
    sahf
    je  short t1_taken
    mov ax, 0xDEAD             ; should not run
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_ZF 1
    jmp t2

t2_taken:
    mov ax, 0x2BAD
    jmp t2_after

; ===================== 2) JE not taken (ZF=0 via SAHF), forward short =====================
t2:
    PREP
    mov ah, [pat_zf0]
    sahf
    je  short t2_taken         ; NOT taken
    mov ax, 0x2222             ; expected fall-through
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    CHECK_ZF 0
    jmp t3

; ===================== 3) JE taken (ZF=1), backward short =====================
t3_target:
    mov ax, 0x3333
    jmp t3_after
t3:
    PREP
    mov ah, [pat_zf1]
    sahf
    je  short t3_target
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP [sp0_store]
    CHECK_ZF 1
    jmp t4

; ===================== 4) JE not taken (ZF=0), backward short =====================
t4_target:
    mov ax, 0x4BAD             ; should not run
    jmp t4_after
t4:
    PREP
    mov ah, [pat_zf0]
    sahf
    je  short t4_target        ; NOT taken
    mov ax, 0x4444
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP [sp0_store]
    CHECK_ZF 0

; ===================== 5) CMP equal → ZF=1 → taken =====================
t5:
    PREP
    mov al, 0x34
    cmp al, 0x34               ; ZF=1
    je  short t5_taken
    mov ax, 0x5BAD
    jmp t5_after
t5_taken:
    mov ax, 0x5555
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    ASSERT_SP [sp0_store]
    CHECK_ZF 1
    CHECK_CF 0
    jmp t6

t6_taken:
    mov ax, 0x6BAD
    jmp t6_after

; ===================== 6) CMP not equal → ZF=0 → NOT taken =====================
t6:
    PREP
    mov al, 0x12
    cmp al, 0x34               ; ZF=0 (and CF=1)
    je  short t6_taken
    mov ax, 0x6666
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_SP [sp0_store]
    CHECK_ZF 0

; ===================== 7) SUB to zero → ZF=1 → taken =====================
t7:
    PREP
    mov al, 0x50
    sub al, 0x50               ; 0 → ZF=1
    je  short t7_taken
    mov ax, 0x7BAD
    jmp t7_after
t7_taken:
    mov ax, 0x7777
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7777
    ASSERT_SP [sp0_store]
    CHECK_ZF 1
    jmp t8

t8_taken:
    mov ax, 0x8BAD
    jmp t8_after

; ===================== 8) SUB to nonzero → ZF=0 → NOT taken =====================
t8:
    PREP
    mov al, 0x55
    sub al, 1                  ; 0x54 → ZF=0
    je  short t8_taken
    mov ax, 0x8888
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    ASSERT_SP [sp0_store]
    CHECK_ZF 0

; ===================== 9) ADD wrap to zero → ZF=1 → taken =====================
t9:
    PREP
    mov al, 0xFF
    add al, 1                  ; 0x00 → ZF=1
    je  short t9_taken
    mov ax, 0x9BAD
    jmp t9_after
t9_taken:
    mov ax, 0x9999
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9999
    ASSERT_SP [sp0_store]
    CHECK_ZF 1
    ; (CF=1 incidentally; we only care about ZF here.)
    jmp t10

t10_taken:
    mov ax, 0xABAD
    jmp t10_after

; ===================== 10) ADD nonzero → ZF=0 → NOT taken =====================
t10:
    PREP
    mov al, 1
    add al, 2                  ; 3 → ZF=0
    je  short t10_taken
    mov ax, 0xAAAA
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xAAAA
    ASSERT_SP [sp0_store]
    CHECK_ZF 0

; ===================== 11) XOR AL,AL → zero → ZF=1 → taken =====================
t11:
    PREP
    mov al, 0x7F
    xor al, al                 ; 0 → ZF=1, PF=1, CF/OF=0
    je  short t11_taken
    mov ax, 0xBEEF
    jmp t11_after
t11_taken:
    mov ax, 0xBBBB
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xBBBB
    ASSERT_SP [sp0_store]
    CHECK_ZF 1

; ===================== 12) AND to zero → ZF=1 → taken =====================
t12:
    PREP
    mov al, 0xF0
    and al, 0x0F               ; 0 → ZF=1
    je  short t12_taken
    mov ax, 0xCBAD
    jmp t12_after
t12_taken:
    mov ax, 0xCCCC
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xCCCC
    ASSERT_SP [sp0_store]
    CHECK_ZF 1
    ; (AF undefined after logicals; we don’t assert it.)
    jmp t13

t13_taken:
    mov ax, 0xDBAD
    jmp t13_after

; ===================== 13) AND to nonzero → ZF=0 → NOT taken =====================
t13:
    PREP
    mov al, 0xF0
    and al, 0xF0               ; 0xF0 → ZF=0
    je  short t13_taken
    mov ax, 0xDDDD
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xDDDD
    ASSERT_SP [sp0_store]
    CHECK_ZF 0

; ===================== 14) TEST zero → ZF=1 → taken =====================
t14:
    PREP
    mov al, 0
    test al, al                ; ZF=1, CF/OF=0
    je  short t14_taken
    mov ax, 0xEBAD
    jmp t14_after
t14_taken:
    mov ax, 0xEEEE
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xEEEE
    ASSERT_SP [sp0_store]
    CHECK_ZF 1

; ===================== 15) OR yielding zero (AL already zero) → ZF=1 → taken =====================
t15:
    PREP
    xor al, al
    or  al, 0                  ; still 0 → ZF=1
    je  short t15_taken
    mov ax, 0xF0F0
    jmp t15_after
t15_taken:
    mov ax, 0xF111
t15_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
    ASSERT_SP [sp0_store]
    CHECK_ZF 1

; ===================== 16) Flags preserved by JE (pattern via SAHF) → taken =====================
t16:
    PREP
    mov ah, [pat_mix_zf1]      ; CF=PF=AF=ZF=SF=1
    sahf
    je  short t16_taken
    mov ax, 0xFACE
    jmp t16_after
t16_taken:
    mov ax, 0xF222
t16_after:
    SAVE_FLAGS
    ASSERT_AX 0xF222
    ASSERT_SP [sp0_store]
    CHECK_ZF 1
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_SF 1
    jmp t17

t17_taken:
    mov ax, 0xF3BD
    jmp t17_after

; ===================== 17) Flags preserved when NOT taken (ZF=0) =====================
t17:
    PREP
    mov ah, [pat_mix_zf0]      ; ZF=0, other flags mostly 1
    sahf
    je  short t17_taken        ; NOT taken
    mov ax, 0xF333
t17_after:
    SAVE_FLAGS
    ASSERT_AX 0xF333
    ASSERT_SP [sp0_store]
    CHECK_ZF 0
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_SF 1

; ===================== 18) DF preserved across JE =====================
t18:
    PREP
    std
    mov ah, [pat_zf1]          ; ZF=1
    sahf
    je  short t18_taken
    mov ax, 0x0BAD
    jmp t18_after
t18_taken:
    mov ax, 0xF444
t18_after:
    SAVE_FLAGS
    ASSERT_AX 0xF444
    ASSERT_SP [sp0_store]
    CHECK_DF 1
    cld

; ===================== 19) Odd SP (unaligned stack) =====================
t19:
    PREP_ODD
    mov ah, [pat_zf1]
    sahf
    je  short t19_taken
    mov ax, 0x0DAD
    jmp t19_after
t19_taken:
    mov ax, 0xF555
t19_after:
    SAVE_FLAGS
    ASSERT_AX 0xF555
    ASSERT_SP [sp0_store]
    CHECK_ZF 1
    jmp t20

t20_mid:
    mov ax, 0xF6BD
    jmp t20_after

; ===================== 20) Chain: first NOT taken (ZF=0), then set ZF=1 → taken =====================
t20:
    PREP
    mov ah, [pat_zf0]
    sahf
    je  short t20_mid          ; NOT taken
    ; fall-through
    mov ah, [pat_zf1]          ; ZF=1
    sahf
    je  short t20_taken        ; taken
    mov ax, 0xABCD
    jmp short t20_after
t20_taken:
    mov ax, 0xF666
t20_after:
    SAVE_FLAGS
    ASSERT_AX 0xF666
    ASSERT_SP [sp0_store]
    CHECK_ZF 1

; ===================== 21) INC wrap to zero → ZF=1 → taken =====================
t21:
    PREP
    mov al, 0xFF
    inc al                     ; 0x00 → ZF=1
    je  short t21_taken
    mov ax, 0x1110
    jmp t21_after
t21_taken:
    mov ax, 0xF777
t21_after:
    SAVE_FLAGS
    ASSERT_AX 0xF777
    ASSERT_SP [sp0_store]
    CHECK_ZF 1

; ===================== 22) DEC to zero → ZF=1 → taken =====================
t22:
    PREP
    mov al, 1
    dec al                     ; 0x00 → ZF=1
    je  short t22_taken
    mov ax, 0x2220
    jmp t22_after
t22_taken:
    mov ax, 0xF888
t22_after:
    SAVE_FLAGS
    ASSERT_AX 0xF888
    ASSERT_SP [sp0_store]
    CHECK_ZF 1

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

; SAHF patterns (bit6 = ZF)
pat_zf0:     db 0x00            ; ZF=0, others 0
pat_zf1:     db 0x40            ; ZF=1, others 0
pat_mix_zf1: db 0xD5            ; CF=PF=AF=ZF=SF=1 (JE should be taken)
pat_mix_zf0: db 0x95            ; SF=1 AF=1 PF=1 ZF=0 CF=1 (JE should NOT be taken)

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
