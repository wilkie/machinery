; jle.asm — Thorough tests for JLE/JNG (ZF==1 || SF!=OF) in 16-bit mode
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

; ---------- Data ----------
mem8_pos:  db 0x10             ; +16
mem8_neg:  db 0xF0             ; -16
mem8_m7:   db 0xF9             ; -7
mem8_p4:   db 0x04
mem16_m1:  dw 0xFFFF           ; -1
mem16_p1:  dw 0x0001
mem16_max: dw 0x7FFF           ; +32767
mem16_min: dw 0x8000           ; -32768

flags_store: dw 0
pre_flags:   dw 0
orig_ss:     dw 0
orig_sp:     dw 0
sp0_store:   dw 0

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

; ===================== 1) Taken: equal (ZF=1) =====================
t1:
    PREP
    mov al, 0x34
    cmp al, 0x34               ; ZF=1, SF=0, OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t1_taken
    mov ax, 0xDEAD
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t2

t2_taken:
    mov ax, 0x2BAD
    jmp t2_after

; ===================== 2) NOT taken: +1 <= -1 ? (false) =====================
t2:
    PREP
    mov al, 0x01               ; +1
    cmp al, 0xFF               ; -1 → ZF=0, SF=0, OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t2_taken         ; NOT taken
    mov ax, 0x2222
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 3) Taken: -2 <= -1 (ZF=0, SF=1, OF=0 → SF!=OF) =====================
t3:
    PREP
    mov al, 0xFE               ; -2
    cmp al, 0xFF               ; -1 → result 0xFF
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t3_taken
    mov ax, 0x3333
    jmp t3_after
t3_taken:
    mov ax, 0x3331
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3331
    ASSERT_SP [sp0_store]
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 4) Taken (overflow): -128 <= +1 (ZF=0, OF=1, SF=0 → SF!=OF) =====================
t4:
    PREP
    mov al, 0x80               ; -128
    cmp al, 0x01               ; → 0x7F, OF=1, SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t4_taken
    mov ax, 0x4444
    jmp t4_after
t4_taken:
    mov ax, 0x4441
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4441
    ASSERT_SP [sp0_store]
    CHECK_ZF 0
    CHECK_OF 1
    CHECK_SF 0
    ASSERT_EQ_FLAGS
    jmp t5

t5_taken:
    mov ax, 0x5BAD
    jmp t5_after

; ===================== 5) NOT taken (overflow): +127 <= -1 ? (false) (ZF=0, OF=1, SF=1) =====================
t5:
    PREP
    mov al, 0x7F               ; +127
    cmp al, 0xFF               ; -1 → 0x80, OF=1, SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t5_taken         ; NOT taken
    mov ax, 0x5555
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    ASSERT_SP [sp0_store]
    CHECK_ZF 0
    CHECK_OF 1
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t6

; ===================== 6) Backward taken: -7 <= -5 =====================
t6_target:
    mov ax, 0x6666
    jmp t6_after
t6:
    PREP
    mov al, [mem8_m7]          ; -7
    cmp al, 0xFB               ; -5
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t6_target
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_SP [sp0_store]
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t7

; ===================== 7) Backward NOT taken: +4 <= +3 ? (false) =====================
t7_target:
    mov ax, 0x7777             ; should not run
    jmp t7_after
t7:
    PREP
    mov al, 0x04
    cmp al, 0x03               ; → 0x01, SF=0, OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t7_target        ; NOT taken
    mov ax, 0x7010
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7010
    ASSERT_SP [sp0_store]
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 8) 16-bit taken: -1 <= +1 =====================
t8:
    PREP
    mov ax, [mem16_m1]         ; -1
    cmp ax, [mem16_p1]         ; → 0xFFFE, SF=1, OF=0
    pushf
    pop  dx
    mov [pre_flags], dx
    jle short t8_taken
    mov ax, 0x8888
    jmp t8_after
t8_taken:
    mov ax, 0x8081
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8081
    ASSERT_SP [sp0_store]
    CHECK_SF 1
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 9) 16-bit taken: equal =====================
t9:
    PREP
    mov ax, 0x1234
    cmp ax, 0x1234             ; ZF=1
    pushf
    pop  dx
    mov [pre_flags], dx
    jle short t9_taken
    mov ax, 0x9090
    jmp t9_after
t9_taken:
    mov ax, 0x9091
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9091
    ASSERT_SP [sp0_store]
    CHECK_ZF 1
    ASSERT_EQ_FLAGS

; ===================== 10) 16-bit taken (overflow): -32768 <= +1 (SF=0,OF=1) =====================
t10:
    PREP
    mov ax, [mem16_min]        ; -32768
    cmp ax, [mem16_p1]         ; → 0x7FFF, OF=1, SF=0
    pushf
    pop  dx
    mov [pre_flags], dx
    jle short t10_taken
    mov ax, 0xA0A0
    jmp t10_after
t10_taken:
    mov ax, 0xA0A1
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xA0A1
    ASSERT_SP [sp0_store]
    CHECK_OF 1
    CHECK_SF 0
    ASSERT_EQ_FLAGS
    jmp t11

t11_taken:
    mov ax, 0xB0BD
    jmp t11_after

; ===================== 11) 16-bit NOT taken (overflow): +32767 <= -1 ? (false) (SF=1,OF=1) =====================
t11:
    PREP
    mov ax, [mem16_max]        ; +32767
    cmp ax, [mem16_m1]         ; → 0x8000, OF=1, SF=1
    pushf
    pop  dx
    mov [pre_flags], dx
    jle short t11_taken
    mov ax, 0xB0B0
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xB0B0
    ASSERT_SP [sp0_store]
    CHECK_OF 1
    CHECK_SF 1
    ASSERT_EQ_FLAGS

; ===================== 12) Taken with mem8: -16 <= +16 =====================
t12:
    PREP
    mov al, [mem8_neg]
    cmp al, [mem8_pos]         ; -16 - +16 → SF=1, OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t12_taken
    mov ax, 0xC0C0
    jmp t12_after
t12_taken:
    mov ax, 0xC0C1
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xC0C1
    ASSERT_SP [sp0_store]
    CHECK_SF 1
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t13

t13_taken:
    mov ax, 0xD0BD
    jmp t13_after

; ===================== 13) NOT taken with mem8: +4 <= -7 ? (false; OF=1,SF=1) =====================
t13:
    PREP
    mov al, 0x7F
    cmp al, [mem8_m7]          ; +127 - (-7) → 0x86, OF=1, SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t13_taken
    mov ax, 0xD0D0
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xD0D0
    ASSERT_SP [sp0_store]
    CHECK_OF 1
    CHECK_SF 1
    ASSERT_EQ_FLAGS

; ===================== 14) CF irrelevant #1: 0 <= 1 (CF=1, SF=1, OF=0) → taken =====================
t14:
    PREP
    mov al, 0x00
    cmp al, 0x01               ; result 0xFF → CF=1, SF=1, OF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t14_taken
    mov ax, 0xE0E0
    jmp t14_after
t14_taken:
    mov ax, 0xE0E1
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xE0E1
    CHECK_CF 1
    CHECK_SF 1
    CHECK_OF 0
    ASSERT_EQ_FLAGS
    jmp t15

t15_taken:
    mov ax, 0xE5BD
    jmp t15_after

; ===================== 15) CF irrelevant #2: 0 <= -1 ? (false) even though CF=1 =====================
; 0 - (-1) = +1 → CF=1, ZF=0, SF=0, OF=0 → not taken
t15:
    PREP
    mov al, 0x00
    cmp al, 0xFF
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t15_taken
    mov ax, 0xE5E5
t15_after:
    SAVE_FLAGS
    ASSERT_AX 0xE5E5
    CHECK_CF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_OF 0
    ASSERT_EQ_FLAGS

; ===================== 16) Flags preserved by JLE (synthetic via POPF) — taken =====================
; Set ZF=0, SF=1, OF=0 → SF!=OF ⇒ taken
t16:
    PREP
    SET_FLAGS 0x0080           ; SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t16_taken
    mov ax, 0xEEEE
    jmp t16_after
t16_taken:
    mov ax, 0xF111
t16_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
    CHECK_SF 1
    CHECK_OF 0
    CHECK_ZF 0
    ASSERT_EQ_FLAGS
    jmp t17

t17_taken:
    mov ax, 0xF2BD
    jmp t17_after

; ===================== 17) Flags preserved by JLE (synthetic) — NOT taken =====================
; Set ZF=0, SF=1, OF=1 → SF==OF and ZF=0 ⇒ NOT taken
t17:
    PREP
    SET_FLAGS 0x0880           ; OF=1, SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t17_taken        ; NOT taken
    mov ax, 0xF222
t17_after:
    SAVE_FLAGS
    ASSERT_AX 0xF222
    CHECK_SF 1
    CHECK_OF 1
    CHECK_ZF 0
    ASSERT_EQ_FLAGS

; ===================== 18) DF preserved across JLE =====================
t18:
    PREP
    std
    mov al, 0x01
    cmp al, 0x02               ; +1 <= +2 → taken
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t18_taken
    mov ax, 0xFACE
    jmp t18_after
t18_taken:
    mov ax, 0xF333
t18_after:
    SAVE_FLAGS
    ASSERT_AX 0xF333
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    CHECK_DF 1
    cld

; ===================== 19) Odd SP (unaligned stack) =====================
t19:
    PREP_ODD
    mov al, 0x05
    cmp al, 0x06               ; taken (less)
    pushf
    pop  ax
    mov [pre_flags], ax
    jle short t19_taken
    mov ax, 0xABCD
    jmp t19_after
t19_taken:
    mov ax, 0xF444
t19_after:
    SAVE_FLAGS
    ASSERT_AX 0xF444
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    jmp t20a

t20a_mid:
    mov ax, 0xF5BD
    jmp t20a_after

; ===================== 20) Chains =====================
; (a) First NOT taken (greater), then taken (equal)
t20a:
    PREP
    mov al, 0x03
    cmp al, 0x01               ; greater → NOT taken
    jle short t20a_mid
    ; fall-through: make equal
    mov al, 0x55
    cmp al, 0x55               ; ZF=1 → taken
    jle short t20a_taken
    mov ax, 0x1110
    jmp short t20a_after
t20a_taken:
    mov ax, 0xF555
t20a_after:
    SAVE_FLAGS
    ASSERT_AX 0xF555

; (b) First taken (less), then NOT taken (greater)
t20b:
    PREP
    mov al, 0x01
    cmp al, 0x02               ; less → taken
    jle short t20b_mid
    mov ax, 0xBEEF
    jmp short t20b_after
t20b_mid:
    mov al, 0x10
    cmp al, 0x0F               ; greater → NOT taken
    jle short t20b_bad
    mov ax, 0xF666
    jmp short t20b_after
t20b_bad:
    mov ax, 0xBAD0
t20b_after:
    SAVE_FLAGS
    ASSERT_AX 0xF666

; ===================== Exit =====================
exit:
    RESTORE_DOS_STACK
    mov ax, 0x4C00
    int 0x21

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
