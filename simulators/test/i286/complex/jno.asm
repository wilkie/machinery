; jno.asm — Thorough tests for JNO (OF == 0) in 16-bit mode
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

; Read-back bits from [flags_store]
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

; Synthetic flag seed via POPF
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

; ===================== 1) Taken: ADD 1+1 → OF=0 =====================
t1:
    PREP
    mov al, 1
    add al, 1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t1_taken
    mov ax, 0xDEAD
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t2

t2_taken:
    mov ax, 0x2BAD
    jmp t2_after

; ===================== 2) NOT taken: ADD 0x7F+1 → OF=1 =====================
t2:
    PREP
    mov al, 0x7F
    add al, 1                  ; → 0x80, OF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t2_taken         ; NOT taken
    mov ax, 0x2222
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    CHECK_OF 1
    ASSERT_EQ_FLAGS
    jmp t3

; ===================== 3) Taken (backward): SUB 5-2 → OF=0 =====================
t3_target:
    mov ax, 0x3333
    jmp t3_after
t3:
    PREP
    mov al, 5
    sub al, 2                  ; → 3, OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t3_target
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t4

; ===================== 4) NOT taken (backward): SUB 0x80-1 → OF=1 =====================
t4_target:
    mov ax, 0x4BAD             ; should not run
    jmp t4_after
t4:
    PREP
    mov al, 0x80
    sub al, 1                  ; → 0x7F, OF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t4_target        ; NOT taken
    mov ax, 0x4444
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    CHECK_OF 1
    ASSERT_EQ_FLAGS

; ===================== 5) INC non-saturation: 0x7E→0x7F (OF=0) → taken =====================
t5:
    PREP
    mov al, 0x7E
    inc al                     ; OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t5_taken
    mov ax, 0x5555
t5_taken:
    mov ax, 0x5051
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5051
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t6

t6_taken:
    mov ax, 0x6BAD
    jmp t6_after

; ===================== 6) INC crossing +127→-128: 0x7F→0x80 (OF=1) → NOT taken =====================
t6:
    PREP
    mov al, 0x7F
    inc al                     ; OF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t6_taken
    mov ax, 0x6666
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    CHECK_OF 1
    ASSERT_EQ_FLAGS

; ===================== 7) DEC non-overflow: 0x00→0xFF (OF=0) → taken =====================
t7:
    PREP
    mov al, 0x00
    dec al                     ; OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t7_taken
    mov ax, 0x7777
    jmp t7_after
t7_taken:
    mov ax, 0x7071
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7071
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t8

t8_taken:
    mov ax, 0x8BAD
    jmp t8_after

; ===================== 8) DEC crossing -128→+127: 0x80→0x7F (OF=1) → NOT taken =====================
t8:
    PREP
    mov al, 0x80
    dec al                     ; OF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t8_taken
    mov ax, 0x8888
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    CHECK_OF 1
    ASSERT_EQ_FLAGS

; ===================== 9) XOR clears OF → taken =====================
t9:
    PREP
    mov al, 0x7F
    xor al, al                 ; OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t9_taken
    mov ax, 0x9999
    jmp t9_after
t9_taken:
    mov ax, 0x9091
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9091
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 10) TEST clears OF → taken =====================
t10:
    PREP
    mov al, 1
    test al, al                ; OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t10_taken
    mov ax, 0xAAAA
    jmp t10_after
t10_taken:
    mov ax, 0xA0A1
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xA0A1
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 11) OR clears OF (even with ZF=1) → taken =====================
t11:
    PREP
    xor al, al                 ; AL=0
    or  al, 0                  ; OF=0, ZF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t11_taken
    mov ax, 0xBEEF
    jmp t11_after
t11_taken:
    mov ax, 0xB0B1
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xB0B1
    CHECK_OF 0
    CHECK_ZF 1
    ASSERT_EQ_FLAGS

; ===================== 12) CF irrelevant: ADD 0xFF+1 → CF=1, OF=0 → taken =====================
t12:
    PREP
    mov al, 0xFF
    add al, 1                  ; → 0x00, CF=1, OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t12_taken
    mov ax, 0xC0C0
    jmp t12_after
t12_taken:
    mov ax, 0xC0C1
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xC0C1
    CHECK_OF 0
    CHECK_CF 1
    CHECK_ZF 1
    ASSERT_EQ_FLAGS

; ===================== 13) 16-bit taken: ADD 2+2 → OF=0 =====================
t13:
    PREP
    mov ax, 2
    add ax, 2
    pushf
    pop  dx
    mov [pre_flags], dx
    jno short t13_taken
    mov ax, 0xD0D0
    jmp t13_after
t13_taken:
    mov ax, 0xD0D1
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xD0D1
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t14

t14_taken:
    mov ax, 0xE0BD
    jmp t14_after

; ===================== 14) 16-bit NOT taken: ADD 0x7FFF+1 → OF=1 =====================
t14:
    PREP
    mov ax, 0x7FFF
    add ax, 1                  ; → 0x8000, OF=1
    pushf
    pop  dx
    mov [pre_flags], dx
    jno short t14_taken
    mov ax, 0xE0E0
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xE0E0
    CHECK_OF 1
    ASSERT_EQ_FLAGS
    jmp t15

t15_taken:
    mov ax, 0xF1BD
    jmp t15_after

; ===================== 15) CMP produce OF=1 → NOT taken =====================
t15:
    PREP
    mov al, 0x7F
    cmp al, 0xFF               ; 0x7F - 0xFF = 0x80 → OF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t15_taken
    mov ax, 0xF111
t15_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
    CHECK_OF 1
    ASSERT_EQ_FLAGS

; ===================== 16) CMP equal (OF=0) → taken =====================
t16:
    PREP
    mov al, 0x22
    cmp al, 0x22               ; OF=0, ZF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t16_taken
    mov ax, 0xF222
t16_taken:
    mov ax, 0xF226
t16_after:
    SAVE_FLAGS
    ASSERT_AX 0xF226
    CHECK_OF 0
    CHECK_ZF 1
    ASSERT_EQ_FLAGS
    jmp t17

t17_taken:
    mov ax, 0xF3BD
    jmp t17_after

; ===================== 17) Memory ADD: [0x80]+[0x80] → OF=1 → NOT taken =====================
t17:
    PREP
    mov al, [mem8_min]
    add al, [mem8_min]         ; → 0x00, OF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t17_taken
    mov ax, 0xF333
t17_after:
    SAVE_FLAGS
    ASSERT_AX 0xF333
    CHECK_OF 1
    ASSERT_EQ_FLAGS
    jmp t18

t18_taken:
    mov ax, 0xF4BD
    jmp t18_after

; ===================== 18) Memory SUB: 0x80 - [1] → OF=1 → NOT taken =====================
t18:
    PREP
    mov al, 0x80
    sub al, [mem8_pos1]        ; → 0x7F, OF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t18_taken
    mov ax, 0xF444
t18_after:
    SAVE_FLAGS
    ASSERT_AX 0xF444
    CHECK_OF 1
    ASSERT_EQ_FLAGS

; ===================== 19) Memory ADD no overflow: 1 + [1] → OF=0 → taken =====================
t19:
    PREP
    mov al, 1
    add al, [mem8_pos1]        ; → 2, OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t19_taken
    mov ax, 0xF555
    jmp t14_after
t19_taken:
    mov ax, 0xF559
t19_after:
    SAVE_FLAGS
    ASSERT_AX 0xF559
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 20) Synthetic via POPF — taken (OF=0) =====================
t20:
    PREP
    SET_FLAGS 0x0002            ; bit1 must be 1 for POPF correctness; OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t20_taken
    mov ax, 0xF666
    jmp t20_after
t20_taken:
    mov ax, 0xF661
t20_after:
    SAVE_FLAGS
    ASSERT_AX 0xF661
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t21

t21_taken:
    mov ax, 0xF7BD
    jmp t21_after

; ===================== 21) Synthetic via POPF — NOT taken (OF=1) =====================
t21:
    PREP
    SET_FLAGS 0x0802            ; OF=1 (bit11), bit1=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t21_taken
    mov ax, 0xF777
t21_after:
    SAVE_FLAGS
    ASSERT_AX 0xF777
    CHECK_OF 1
    ASSERT_EQ_FLAGS

; ===================== 22) DF preserved across JNO (taken) =====================
t22:
    PREP
    std
    mov al, 0x01
    add al, 0x01               ; OF=0 → taken
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t22_taken
    mov ax, 0xFACE
    jmp t22_after
t22_taken:
    mov ax, 0xF888
t22_after:
    SAVE_FLAGS
    ASSERT_AX 0xF888
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    CHECK_DF 1
    cld

; ===================== 23) Odd SP (unaligned) + taken =====================
t23:
    PREP_ODD
    mov al, 5
    sub al, 2                  ; OF=0 → taken
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t23_taken
    mov ax, 0xABCD
    jmp t23_after
t23_taken:
    mov ax, 0xF999
t23_after:
    SAVE_FLAGS
    ASSERT_AX 0xF999
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    jmp t24

t24_mid:
    mov ax, 0xFABD
    jmp t24_after

; ===================== 24) Chain: first NOT taken (OF=1), then taken (OF=0) =====================
t24:
    PREP
    mov al, 0x7F
    add al, 1                  ; OF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t24_mid          ; NOT taken
    ; fall-through: now make OF=0
    mov al, 2
    add al, 2                  ; OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jno short t24_taken        ; taken
    mov ax, 0x1110
    jmp short t24_after
t24_taken:
    mov ax, 0xFAAA
t24_after:
    SAVE_FLAGS
    ASSERT_AX 0xFAAA
    ASSERT_SP [sp0_store]

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
