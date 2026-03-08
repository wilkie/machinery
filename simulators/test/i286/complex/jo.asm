; jo.asm — Thorough tests for JO (OF == 1) in 16-bit mode
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
%macro ASSERT_EQ_FLAGS 0
    mov ax, [flags_store]
    mov bx, [pre_flags]
    int 0x23
%endmacro

; Read-back checks from [flags_store]
%macro CHECK_OF 1
    mov ax, [flags_store]
    mov cl, 11
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

; Synthetic flag seed via POPF (lets us set OF directly)
%macro SET_FLAGS 1
    push word %1
    popf
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

; ===================== 1) Taken: ADD 0x7F + 1 → 0x80, OF=1 =====================
t1:
    PREP
    mov al, 0x7F
    add al, 1
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t1_taken
    mov ax, 0xDEAD
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_OF 1
    ASSERT_EQ_FLAGS
    jmp t2

t2_taken:
    mov ax, 0x2BAD
    jmp t2_after

; ===================== 2) NOT taken: ADD 1 + 1 → OF=0 =====================
t2:
    PREP
    mov al, 1
    add al, 1
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t2_taken
    mov ax, 0x2222
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t3

; ===================== 3) Taken (backward): SUB 0x80 - 1 → 0x7F, OF=1 =====================
t3_target:
    mov ax, 0x3333
    jmp t3_after
t3:
    PREP
    mov al, 0x80
    sub al, 1
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t3_target
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP [sp0_store]
    CHECK_OF 1
    ASSERT_EQ_FLAGS
    jmp t4

; ===================== 4) NOT taken (backward): SUB 5 - 2 → OF=0 =====================
t4_target:
    mov ax, 0x4BAD             ; should not run
    jmp t4_after
t4:
    PREP
    mov al, 5
    sub al, 2
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t4_target        ; NOT taken
    mov ax, 0x4444
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP [sp0_store]
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 5) Taken: INC 0x7F → 0x80, OF=1 =====================
t5:
    PREP
    mov al, 0x7F
    inc al
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t5_taken
    mov ax, 0x5555
    jmp t5_after
t5_taken:
    mov ax, 0x5051
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5051
    CHECK_OF 1
    ASSERT_EQ_FLAGS
    jmp t6

t6_taken:
    mov ax, 0x6BAD
    jmp t6_after

; ===================== 6) NOT taken: INC 0x7E → OF=0 =====================
t6:
    PREP
    mov al, 0x7E
    inc al
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t6_taken
    mov ax, 0x6666
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 7) Taken: DEC 0x80 → 0x7F, OF=1 =====================
t7:
    PREP
    mov al, 0x80
    dec al
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t7_taken
    mov ax, 0x7777
    jmp t7_after
t7_taken:
    mov ax, 0x7071
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7071
    CHECK_OF 1
    ASSERT_EQ_FLAGS
    jmp t8

t8_taken:
    mov ax, 0x8BAD
    jmp t8_after

; ===================== 8) NOT taken: DEC 0x00 → 0xFF, OF=0 =====================
t8:
    PREP
    mov al, 0x00
    dec al
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t8_taken
    mov ax, 0x8888
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t9

t9_taken:
    mov ax, 0x9BAD
    jmp t9_after

; ===================== 9) NOT taken: XOR clears OF (OF=0) =====================
t9:
    PREP
    mov al, 0x7F
    xor al, al                 ; 0, OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t9_taken
    mov ax, 0x9999
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9999
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t10

t10_taken:
    mov ax, 0xABAD
    jmp t10_after

; ===================== 10) NOT taken: TEST clears OF (OF=0) =====================
t10:
    PREP
    mov al, 1
    test al, al                ; OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t10_taken
    mov ax, 0xAAAA
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xAAAA
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 11) ZF irrelevant: ADD 0x80 + 0x80 → 0x00, ZF=1, OF=1 → taken =====================
t11:
    PREP
    mov al, 0x80
    add al, 0x80
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t11_taken
    mov ax, 0xBEEF
    jmp t11_after
t11_taken:
    mov ax, 0xBBBB
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xBBBB
    CHECK_OF 1
    CHECK_ZF 1
    ASSERT_EQ_FLAGS
    jmp t12

t12_taken:
    mov ax, 0xCBAD
    jmp t12_after

; ===================== 12) CF irrelevant: ADD 0xFF + 0x01 → 0x00, CF=1, OF=0 → NOT taken =====================
t12:
    PREP
    mov al, 0xFF
    add al, 1
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t12_taken
    mov ax, 0xC0C0
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xC0C0
    CHECK_OF 0
    CHECK_ZF 1
    CHECK_CF 1
    ASSERT_EQ_FLAGS

; ===================== 13) 16-bit taken: ADD 0x7FFF + 1 → 0x8000, OF=1 =====================
t13:
    PREP
    mov ax, 0x7FFF
    add ax, 1
    pushf
    pop  dx
    mov [pre_flags], dx
    jo  short t13_taken
    mov ax, 0xD0D0
    jmp t13_after
t13_taken:
    mov ax, 0xD0D1
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xD0D1
    CHECK_OF 1
    ASSERT_EQ_FLAGS
    jmp t14

t14_taken:
    mov ax, 0xE0BD
    jmp t14_after

; ===================== 14) 16-bit NOT taken: ADD 2 + 2 → OF=0 =====================
t14:
    PREP
    mov ax, 2
    add ax, 2
    pushf
    pop  dx
    mov [pre_flags], dx
    jo  short t14_taken
    mov ax, 0xE0E0
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xE0E0
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 15) Taken via CMP: 0x7F - 0xFF → 0x80 → OF=1 =====================
t15:
    PREP
    mov al, 0x7F
    cmp al, 0xFF
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t15_taken
    mov ax, 0xF111
    jmp t15_after
t15_taken:
    mov ax, 0xF115
t15_after:
    SAVE_FLAGS
    ASSERT_AX 0xF115
    CHECK_OF 1
    ASSERT_EQ_FLAGS
    jmp t16

t16_taken:
    mov ax, 0xF2BD
    jmp t16_after

; ===================== 16) NOT taken via CMP: equal → OF=0 =====================
t16:
    PREP
    mov al, 0x22
    cmp al, 0x22
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t16_taken
    mov ax, 0xF222
t16_after:
    SAVE_FLAGS
    ASSERT_AX 0xF222
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 17) Taken with memory ADD: 0x80 + [0x80] → ZF=1, OF=1 =====================
t17:
    PREP
    mov al, [mem8_min]
    add al, [mem8_min]
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t17_taken
    mov ax, 0xF333
    jmp t17_after
t17_taken:
    mov ax, 0xF337
t17_after:
    SAVE_FLAGS
    ASSERT_AX 0xF337
    CHECK_OF 1
    CHECK_ZF 1
    ASSERT_EQ_FLAGS

; ===================== 18) Taken with memory SUB: 0x80 - [1] → OF=1 =====================
t18:
    PREP
    mov al, 0x80
    sub al, [mem8_pos1]
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t18_taken
    mov ax, 0xF444
    jmp t18_after
t18_taken:
    mov ax, 0xF448
t18_after:
    SAVE_FLAGS
    ASSERT_AX 0xF448
    CHECK_OF 1
    ASSERT_EQ_FLAGS

; ===================== 19) Synthetic flags via POPF — taken (OF=1) =====================
t19:
    PREP
    SET_FLAGS 0x0800           ; OF=1 (bit11), others 0 (don't care)
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t19_taken
    mov ax, 0xF555
    jmp t19_after
t19_taken:
    mov ax, 0xF559
t19_after:
    SAVE_FLAGS
    ASSERT_AX 0xF559
    CHECK_OF 1
    ASSERT_EQ_FLAGS
    jmp t20

t20_taken:
    mov ax, 0xF6BD
    jmp t20_after

; ===================== 20) Synthetic flags via POPF — NOT taken (OF=0) =====================
t20:
    PREP
    SET_FLAGS 0x0000           ; OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t20_taken
    mov ax, 0xF666
t20_after:
    SAVE_FLAGS
    ASSERT_AX 0xF666
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 21) DF preserved across JO (taken path) =====================
t21:
    PREP
    std
    mov al, 0x7F
    inc al                     ; OF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t21_taken
    mov ax, 0xFACE
    jmp t21_after
t21_taken:
    mov ax, 0xF777
t21_after:
    SAVE_FLAGS
    ASSERT_AX 0xF777
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    CHECK_DF 1
    cld

; ===================== 22) Odd SP (unaligned) + taken =====================
t22:
    PREP_ODD
    mov al, 0x7F
    add al, 1                  ; OF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jo  short t22_taken
    mov ax, 0xABCD
    jmp t22_after
t22_taken:
    mov ax, 0xF888
t22_after:
    SAVE_FLAGS
    ASSERT_AX 0xF888
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS

; ===================== Exit =====================
exit:
    RESTORE_DOS_STACK
    mov ax, 0x4C00
    int 0x21

; ---------- Data ----------
mem8_pos1:   db 0x01
mem8_min:    db 0x80           ; -128
mem8_max:    db 0x7F           ; +127
mem8_ff:     db 0xFF           ; -1
mem16_pos1:  dw 0x0001
mem16_min:   dw 0x8000         ; -32768
mem16_max:   dw 0x7FFF         ; +32767
mem16_ff:    dw 0xFFFF         ; -1

flags_store: dw 0
pre_flags:   dw 0
orig_ss:     dw 0
orig_sp:     dw 0
sp0_store:   dw 0

; ---------- Scratch stack ----------
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
