; jns.asm — Thorough tests for JNS (SF == 0) in 16-bit mode
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

; Read-back helpers from [flags_store]
%macro CHECK_SF 1
    mov ax, [flags_store]
    mov cl, 7
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
%macro CHECK_OF 1
    mov ax, [flags_store]
    mov cl, 11
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

; Flag seeding via POPF (for synthetic cases)
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

; ===================== 1) JNS taken (SF=0 via SAHF), forward =====================
t1:
    PREP
    mov ah, [pat_sf0]
    sahf
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t1_taken
    mov ax, 0xDEAD
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_SF 0
    ASSERT_EQ_FLAGS
    jmp t2

t2_taken:
    mov ax, 0x2BAD
    jmp t2_after

; ===================== 2) JNS NOT taken (SF=1 via SAHF), forward =====================
t2:
    PREP
    mov ah, [pat_sf1]
    sahf
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t2_taken         ; NOT taken
    mov ax, 0x2222
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t3

; ===================== 3) JNS taken (SF=0), backward =====================
t3_target:
    mov ax, 0x3333
    jmp t3_after
t3:
    PREP
    mov ah, [pat_sf0]
    sahf
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t3_target
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP [sp0_store]
    CHECK_SF 0
    ASSERT_EQ_FLAGS
    jmp t4

; ===================== 4) JNS NOT taken (SF=1), backward =====================
t4_target:
    mov ax, 0x4BAD             ; should not run
    jmp t4_after
t4:
    PREP
    mov ah, [pat_sf1]
    sahf
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t4_target        ; NOT taken
    mov ax, 0x4444
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP [sp0_store]
    CHECK_SF 1
    ASSERT_EQ_FLAGS

; ===================== 5) SUB positive → SF=0 → taken =====================
t5:
    PREP
    mov al, 5
    sub al, 1                  ; 4 → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t5_taken
    mov ax, 0x5555
    jmp t5_after
t5_taken:
    mov ax, 0x5051
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5051
    CHECK_SF 0
    ASSERT_EQ_FLAGS
    jmp t6

t6_taken:
    mov ax, 0x6BAD
    jmp t6_after

; ===================== 6) SUB negative → SF=1 → NOT taken =====================
t6:
    PREP
    mov al, 0
    sub al, 1                  ; 0xFF → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t6_taken
    mov ax, 0x6666
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t7

t7_taken:
    mov ax, 0x7BAD
    jmp t7_after

; ===================== 7) ADD negative (overflow) → SF=1 → NOT taken =====================
t7:
    PREP
    mov al, 0x7F
    add al, 1                  ; 0x80 → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t7_taken
    mov ax, 0x7777
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7777
    CHECK_SF 1
    CHECK_OF 1
    ASSERT_EQ_FLAGS

; ===================== 8) ADD positive → SF=0 → taken =====================
t8:
    PREP
    mov al, 1
    add al, 1                  ; 2 → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t8_taken
    mov ax, 0x8888
    jmp t8_after
t8_taken:
    mov ax, 0x8081
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8081
    CHECK_SF 0
    ASSERT_EQ_FLAGS
    jmp t9

t9_taken:
    mov ax, 0x9BAD
    jmp t9_after

; ===================== 9) XOR to negative → SF=1 → NOT taken =====================
t9:
    PREP
    mov al, 0xF0
    xor al, 0x0F               ; 0xFF → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t9_taken
    mov ax, 0x9999
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9999
    CHECK_SF 1
    ASSERT_EQ_FLAGS

; ===================== 10) XOR to zero → SF=0 → taken =====================
t10:
    PREP
    mov al, 0x7F
    xor al, al                 ; 0 → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t10_taken
    mov ax, 0xAAAA
    jmp t10_after
t10_taken:
    mov ax, 0xA0A1
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xA0A1
    CHECK_SF 0
    ASSERT_EQ_FLAGS
    jmp t11

t11_taken:
    mov ax, 0xB0BD
    jmp t11_after

; ===================== 11) AND to negative → SF=1 → NOT taken =====================
t11:
    PREP
    mov al, 0xF0
    and al, 0xF0               ; 0xF0 → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t11_taken
    mov ax, 0xB0B0
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xB0B0
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t12

t12_taken:
    mov ax, 0xC0BD
    jmp t12_after

; ===================== 12) OR to negative → SF=1 → NOT taken =====================
t12:
    PREP
    mov al, 0x00
    or  al, 0x81               ; 0x81 → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t12_taken
    mov ax, 0xC0C0
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xC0C0
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t13

t13_taken:
    mov ax, 0xD0BD
    jmp t13_after

; ===================== 13) TEST negative → SF=1 → NOT taken =====================
t13:
    PREP
    mov al, 0x80
    test al, 0xFF              ; SF=1, ZF=0, OF=CF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t13_taken
    mov ax, 0xD0D0
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xD0D0
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_OF 0
    CHECK_CF 0
    ASSERT_EQ_FLAGS

; ===================== 14) TEST zero → SF=0 → taken =====================
t14:
    PREP
    xor al, al
    test al, al                ; zero → SF=0 (ZF=1)
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t14_taken
    mov ax, 0xE0E0
    jmp t14_after
t14_taken:
    mov ax, 0xE0E1
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xE0E1
    CHECK_SF 0
    CHECK_ZF 1
    ASSERT_EQ_FLAGS

; ===================== 15) 16-bit SUB zero → SF=0 → taken (ZF=1, ZF irrelevant) =====================
t15:
    PREP
    mov ax, 1
    sub ax, 1                  ; 0 → SF=0
    pushf
    pop  dx
    mov [pre_flags], dx
    jns short t15_taken
    mov ax, 0xF111
    jmp t15_after
t15_taken:
    mov ax, 0xF115
t15_after:
    SAVE_FLAGS
    ASSERT_AX 0xF115
    CHECK_SF 0
    CHECK_ZF 1
    ASSERT_EQ_FLAGS
    jmp t16

t16_taken:
    mov ax, 0xF2BD
    jmp t16_after

; ===================== 16) 16-bit SUB negative → SF=1 → NOT taken =====================
t16:
    PREP
    xor ax, ax
    sub ax, 1                  ; 0xFFFF → SF=1
    pushf
    pop  dx
    mov [pre_flags], dx
    jns short t16_taken
    mov ax, 0xF222
t16_after:
    SAVE_FLAGS
    ASSERT_AX 0xF222
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t17

t17_taken:
    mov ax, 0xF3BD
    jmp t17_after

; ===================== 17) mem8 compare negative → NOT taken =====================
t17:
    PREP
    mov al, [mem8_neg]         ; -16
    cmp al, [mem8_pos]         ; -16 - +16 = -32 → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t17_taken
    mov ax, 0xF333
t17_after:
    SAVE_FLAGS
    ASSERT_AX 0xF333
    CHECK_SF 1
    ASSERT_EQ_FLAGS

; ===================== 18) mem8 compare positive → taken =====================
t18:
    PREP
    mov al, [mem8_pos]         ; +16
    cmp al, [mem8_neg]         ; +16 - (-16) = +32 → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t18_taken
    mov ax, 0xF444
    jmp t18_after
t18_taken:
    mov ax, 0xF448
t18_after:
    SAVE_FLAGS
    ASSERT_AX 0xF448
    CHECK_SF 0
    ASSERT_EQ_FLAGS

; ===================== 19) CF irrelevant: 0 - 0xFF = 0x01 (CF=1, SF=0) → taken =====================
t19:
    PREP
    mov al, 0x00
    sub al, 0xFF               ; result 1 → CF=1, SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t19_taken
    mov ax, 0xF555
    jmp t19_after
t19_taken:
    mov ax, 0xF559
t19_after:
    SAVE_FLAGS
    ASSERT_AX 0xF559
    CHECK_CF 1
    CHECK_SF 0
    ASSERT_EQ_FLAGS

; ===================== 20) Synthetic via POPF — taken (SF=0) =====================
t20:
    PREP
    SET_FLAGS 0x0002           ; bit1=1, SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t20_taken
    mov ax, 0xF666
    jmp t20_after
t20_taken:
    mov ax, 0xF661
t20_after:
    SAVE_FLAGS
    ASSERT_AX 0xF661
    CHECK_SF 0
    ASSERT_EQ_FLAGS
    jmp t21

t21_taken:
    mov ax, 0xF7BD
    jmp t21_after

; ===================== 21) Synthetic via POPF — NOT taken (SF=1) =====================
t21:
    PREP
    SET_FLAGS 0x0082           ; bit1=1, SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t21_taken
    mov ax, 0xF777
t21_after:
    SAVE_FLAGS
    ASSERT_AX 0xF777
    CHECK_SF 1
    ASSERT_EQ_FLAGS

; ===================== 22) DF preserved across JNS (taken path) =====================
t22:
    PREP
    std
    mov al, 0x7E
    inc al                     ; 0x7F → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t22_taken
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
    sub al, 2                  ; 3 → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t23_taken
    mov ax, 0xABCD
    jmp t23_after
t23_taken:
    mov ax, 0xF999
t23_after:
    SAVE_FLAGS
    ASSERT_AX 0xF999
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    jmp t24a

t24a_mid:
    mov ax, 0xFABD
    jmp t24a_after

; ===================== 24) Chains =====================
; (a) First NOT taken (SF=1), then taken (SF=0)
t24a:
    PREP
    mov ah, [pat_mix_sf1]
    sahf
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t24a_mid         ; NOT taken
    ; fall-through: create SF=0 case
    mov al, 1
    add al, 1                  ; 2 → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t24a_taken
    mov ax, 0x1110
    jmp short t24a_after
t24a_taken:
    mov ax, 0xFAAA
t24a_after:
    SAVE_FLAGS
    ASSERT_AX 0xFAAA

; (b) First taken (SF=0), then NOT taken (SF=1)
t24b:
    PREP
    mov ah, [pat_mix_sf0]
    sahf
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t24b_mid         ; taken
    mov ax, 0xBEEF
    jmp short t24b_after
t24b_mid:
    mov al, 0
    sub al, 1                  ; 0xFF → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jns short t24b_bad         ; NOT taken
    mov ax, 0xF666
    jmp short t24b_after
t24b_bad:
    mov ax, 0xBAD0
t24b_after:
    SAVE_FLAGS
    ASSERT_AX 0xF666
    jmp t25

t25_taken:
    mov ax, 0xF9BD
    jmp t25_after

; ===================== 25) 16-bit mem compare negative → NOT taken =====================
t25:
    PREP
    mov ax, [mem16_m1]         ; -1
    cmp ax, [mem16_p1]         ; -1 - +1 = -2 → SF=1
    pushf
    pop  dx
    mov [pre_flags], dx
    jns short t25_taken
    mov ax, 0xF9F9
t25_after:
    SAVE_FLAGS
    ASSERT_AX 0xF9F9
    CHECK_SF 1
    ASSERT_EQ_FLAGS

; ===================== 26) 16-bit mem compare positive → taken =====================
t26:
    PREP
    mov ax, [mem16_p1]         ; +1
    cmp ax, [mem16_m1]         ; +1 - (-1) = +2 → SF=0
    pushf
    pop  dx
    mov [pre_flags], dx
    jns short t26_taken
    mov ax, 0xFAFA
    jmp t26_after
t26_taken:
    mov ax, 0xFAF1
t26_after:
    SAVE_FLAGS
    ASSERT_AX 0xFAF1
    CHECK_SF 0
    ASSERT_EQ_FLAGS

; ===================== Exit =====================
exit:
    RESTORE_DOS_STACK
    mov ax, 0x4C00
    int 0x21

; ---------- Data ----------
mem8_pos:   db 0x10            ; +16
mem8_neg:   db 0xF0            ; -16
mem8_min:   db 0x80            ; -128
mem8_ff:    db 0xFF            ; -1
mem8_0f:    db 0x0F
mem16_p1:   dw 0x0001
mem16_m1:   dw 0xFFFF
mem16_min:  dw 0x8000
mem16_max:  dw 0x7FFF

; SAHF patterns (bit7 = SF)
pat_sf0:      db 0x00          ; SF=0, others 0
pat_sf1:      db 0x80          ; SF=1, others 0
pat_mix_sf0:  db 0x55          ; CF=PF=AF=ZF=1, SF=0  (JNS taken)
pat_mix_sf1:  db 0xD5          ; CF=PF=AF=ZF=SF=1     (JNS NOT taken)

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
