; js.asm — Thorough tests for JS (SF == 1) in 16-bit mode
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

; ===================== 1) JS taken (SF=1 via SAHF), forward =====================
t1:
    PREP
    mov ah, [pat_sf1]
    sahf
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t1_taken
    mov ax, 0xDEAD
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t2

t2_taken:
    mov ax, 0x2BAD
    jmp t2_after

; ===================== 2) JS NOT taken (SF=0 via SAHF), forward =====================
t2:
    PREP
    mov ah, [pat_sf0]
    sahf
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t2_taken         ; NOT taken
    mov ax, 0x2222
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    CHECK_SF 0
    ASSERT_EQ_FLAGS
    jmp t3

; ===================== 3) JS taken (SF=1 via SAHF), backward =====================
t3_target:
    mov ax, 0x3333
    jmp t3_after
t3:
    PREP
    mov ah, [pat_sf1]
    sahf
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t3_target
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP [sp0_store]
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t4

; ===================== 4) JS NOT taken (SF=0 via SAHF), backward =====================
t4_target:
    mov ax, 0x4BAD             ; should not run
    jmp t4_after
t4:
    PREP
    mov ah, [pat_sf0]
    sahf
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t4_target        ; NOT taken
    mov ax, 0x4444
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP [sp0_store]
    CHECK_SF 0
    ASSERT_EQ_FLAGS

; ===================== 5) SUB negative → SF=1 → taken =====================
t5:
    PREP
    mov al, 0x00
    sub al, 0x01               ; 0xFF → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t5_taken
    mov ax, 0x5555
    jmp t5_after
t5_taken:
    mov ax, 0x5051
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5051
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t6

t6_taken:
    mov ax, 0x6BAD
    jmp t6_after

; ===================== 6) SUB positive → SF=0 → NOT taken =====================
t6:
    PREP
    mov al, 5
    sub al, 1                  ; 4 → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t6_taken
    mov ax, 0x6666
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    CHECK_SF 0
    ASSERT_EQ_FLAGS

; ===================== 7) ADD to negative (overflow) → SF=1 → taken =====================
t7:
    PREP
    mov al, 0x7F
    add al, 1                  ; 0x80 → SF=1, OF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t7_taken
    mov ax, 0x7777
    jmp t7_after
t7_taken:
    mov ax, 0x7071
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7071
    CHECK_SF 1
    CHECK_OF 1
    ASSERT_EQ_FLAGS
    jmp t8

t8_taken:
    mov ax, 0x8BAD
    jmp t8_after

; ===================== 8) ADD positive → SF=0 → NOT taken =====================
t8:
    PREP
    mov al, 1
    add al, 1                  ; 2 → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t8_taken
    mov ax, 0x8888
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    CHECK_SF 0
    ASSERT_EQ_FLAGS

; ===================== 9) XOR to negative → SF=1 → taken =====================
t9:
    PREP
    mov al, 0xF0
    xor al, 0x0F               ; 0xFF → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t9_taken
    mov ax, 0x9999
    jmp t9_after
t9_taken:
    mov ax, 0x9091
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9091
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t10

t10_taken:
    mov ax, 0xABAD
    jmp t10_after

; ===================== 10) XOR to zero → SF=0 → NOT taken =====================
t10:
    PREP
    mov al, 0x7F
    xor al, al                 ; 0 → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t10_taken
    mov ax, 0xAAAA
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xAAAA
    CHECK_SF 0
    ASSERT_EQ_FLAGS

; ===================== 11) AND to negative → SF=1 → taken =====================
t11:
    PREP
    mov al, 0xF0
    and al, 0xF0               ; 0xF0 → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t11_taken
    mov ax, 0xB0B0
    jmp t11_after
t11_taken:
    mov ax, 0xB0B1
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xB0B1
    CHECK_SF 1
    ASSERT_EQ_FLAGS

; ===================== 12) OR to negative → SF=1 → taken =====================
t12:
    PREP
    mov al, 0x00
    or  al, 0x81               ; 0x81 → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t12_taken
    mov ax, 0xC0C0
    jmp t12_after
t12_taken:
    mov ax, 0xC0C1
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xC0C1
    CHECK_SF 1
    ASSERT_EQ_FLAGS

; ===================== 13) TEST negative → SF=1 → taken =====================
t13:
    PREP
    mov al, 0x80
    test al, 0xFF              ; SF=1, ZF=0, OF=CF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t13_taken
    mov ax, 0xD0D0
    jmp t13_after
t13_taken:
    mov ax, 0xD0D1
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xD0D1
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_OF 0
    CHECK_CF 0
    ASSERT_EQ_FLAGS
    jmp t14

t14_taken:
    mov ax, 0xE0BD
    jmp t14_after

; ===================== 14) TEST zero → SF=0 → NOT taken =====================
t14:
    PREP
    xor al, al
    test al, al                ; zero → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t14_taken
    mov ax, 0xE0E0
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xE0E0
    CHECK_SF 0
    CHECK_ZF 1
    ASSERT_EQ_FLAGS

; ===================== 15) 16-bit SUB negative → SF=1 → taken =====================
t15:
    PREP
    xor ax, ax
    sub ax, 1                  ; 0xFFFF → SF=1
    pushf
    pop  dx
    mov [pre_flags], dx
    js  short t15_taken
    mov ax, 0xF111
    jmp t15_after
t15_taken:
    mov ax, 0xF115
t15_after:
    SAVE_FLAGS
    ASSERT_AX 0xF115
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t16

t16_taken:
    mov ax, 0xE0BD
    jmp t14_after

; ===================== 16) 16-bit SUB zero → SF=0 → NOT taken =====================
t16:
    PREP
    mov ax, 1
    sub ax, 1                  ; 0x0000 → SF=0
    pushf
    pop  dx
    mov [pre_flags], dx
    js  short t16_taken
    mov ax, 0xF222
t16_after:
    SAVE_FLAGS
    ASSERT_AX 0xF222
    CHECK_SF 0
    ASSERT_EQ_FLAGS

; ===================== 17) mem8 compare negative → taken =====================
t17:
    PREP
    mov al, [mem8_neg]         ; -16
    cmp al, [mem8_pos]         ; -16 - +16 = -32 → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t17_taken
    mov ax, 0xF333
    jmp t17_after
t17_taken:
    mov ax, 0xF337
t17_after:
    SAVE_FLAGS
    ASSERT_AX 0xF337
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t18

t18_taken:
    mov ax, 0xF4BD
    jmp t18_after

; ===================== 18) mem8 compare positive → NOT taken =====================
t18:
    PREP
    mov al, [mem8_pos]         ; +16
    cmp al, [mem8_neg]         ; +16 - (-16) = +32 → SF=0
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t18_taken
    mov ax, 0xF444
t18_after:
    SAVE_FLAGS
    ASSERT_AX 0xF444
    CHECK_SF 0
    ASSERT_EQ_FLAGS

; ===================== 19) mem8 logical → negative (OR) → taken =====================
t19:
    PREP
    mov al, [mem8_min]         ; 0x80
    or  al, [mem8_pos]         ; 0x80 | 0x10 = 0x90 → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t19_taken
    mov ax, 0xF555
    jmp t19_after
t19_taken:
    mov ax, 0xF559
t19_after:
    SAVE_FLAGS
    ASSERT_AX 0xF559
    CHECK_SF 1
    ASSERT_EQ_FLAGS

; ===================== 20) DF preserved across JS (taken path) =====================
t20:
    PREP
    std
    mov al, 0
    dec al                     ; 0xFF → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t20_taken
    mov ax, 0xFACE
    jmp t20_after
t20_taken:
    mov ax, 0xF666
t20_after:
    SAVE_FLAGS
    ASSERT_AX 0xF666
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    CHECK_DF 1
    cld

; ===================== 21) Odd SP (unaligned) + taken =====================
t21:
    PREP_ODD
    mov al, 0x7F
    inc al                     ; 0x80 → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t21_taken
    mov ax, 0xABCD
    jmp t21_after
t21_taken:
    mov ax, 0xF777
t21_after:
    SAVE_FLAGS
    ASSERT_AX 0xF777
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS
    jmp t22

t22_mid:
    mov ax, 0xF8BD
    jmp t22_after

; ===================== 22) Chain: first NOT taken (SF=0), then taken (SF=1) =====================
t22:
    PREP
    mov ah, [pat_sf0]
    sahf
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t22_mid          ; NOT taken
    ; fall-through: create SF=1 case
    mov al, 0
    sub al, 1                  ; 0xFF → SF=1
    pushf
    pop  ax
    mov [pre_flags], ax
    js  short t22_taken
    mov ax, 0x1110
    jmp short t22_after
t22_taken:
    mov ax, 0xF888
t22_after:
    SAVE_FLAGS
    ASSERT_AX 0xF888
    ASSERT_SP [sp0_store]

; ===================== 23) 16-bit mem compare negative → taken =====================
t23:
    PREP
    mov ax, [mem16_m1]         ; -1
    cmp ax, [mem16_p1]         ; -1 - +1 = -2 → SF=1
    pushf
    pop  dx
    mov [pre_flags], dx
    js  short t23_taken
    mov ax, 0xF999
    jmp t23_after
t23_taken:
    mov ax, 0xF99A
t23_after:
    SAVE_FLAGS
    ASSERT_AX 0xF99A
    CHECK_SF 1
    ASSERT_EQ_FLAGS
    jmp t24

t24_taken:
    mov ax, 0xFABD
    jmp t24_after

; ===================== 24) 16-bit mem compare positive/equal → NOT taken =====================
t24:
    PREP
    mov ax, [mem16_p1]
    cmp ax, [mem16_m1]         ; +1 - (-1) = +2 → SF=0
    pushf
    pop  dx
    mov [pre_flags], dx
    js  short t24_taken
    mov ax, 0xFAAA
t24_after:
    SAVE_FLAGS
    ASSERT_AX 0xFAAA
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
pat_sf0:    db 0x00            ; SF=0, others 0
pat_sf1:    db 0x80            ; SF=1, others 0
pat_mix_sf1:db 0xD5            ; CF=PF=AF=ZF=SF=1 (JS should be taken)
pat_mix_sf0:db 0x55            ; CF=PF=AF=ZF=1, SF=0 (JS should NOT be taken)

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
