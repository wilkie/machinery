; ja.asm — Thorough tests for JA/JNBE (CF==0 && ZF==0) in 16-bit mode
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

; Flag checks (read from [flags_store])
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
%macro CHECK_CF_ZF 2
    CHECK_CF %1
    CHECK_ZF %2
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

; ===================== 1) JA taken (CF=0,ZF=0 via SAHF), forward short =====================
t1:
    PREP
    mov ah, [pat_cf0_zf0]
    sahf
    ja  short t1_taken
    mov ax, 0xDEAD             ; should not run
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,0

; ===================== 2) JA not taken (CF=1,ZF=0 via SAHF), forward short =====================
t2:
    PREP
    mov ah, [pat_cf1_zf0]
    sahf
    ja  short t2_taken         ; NOT taken
    mov ax, 0x2222             ; expected fall-through
    jmp short t2_after
t2_taken:
    mov ax, 0x2BAD
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 1,0
    jmp t3

t3_taken:
    mov ax, 0x3BAD
    jmp t3_after

; ===================== 3) JA not taken (CF=0,ZF=1 via SAHF) =====================
t3:
    PREP
    mov ah, [pat_cf0_zf1]
    sahf
    ja  short t3_taken         ; NOT taken
    mov ax, 0x3333
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,1
    jmp t4

t4_taken:
    mov ax, 0x4BAD
    jmp t4_after

; ===================== 4) JA not taken (CF=1,ZF=1 via SAHF) =====================
t4:
    PREP
    mov ah, [pat_cf1_zf1]
    sahf
    ja  short t4_taken         ; NOT taken
    mov ax, 0x4444
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 1,1
    jmp t5

; ===================== 5) Backward taken (CF=0,ZF=0) =====================
t5_target:
    mov ax, 0x5555
    jmp t5_after
t5:
    PREP
    mov ah, [pat_cf0_zf0]
    sahf
    ja  short t5_target
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,0
    jmp t6

; ===================== 6) Backward not taken (CF=1,ZF=0) =====================
t6_target:
    mov ax, 0x6BAD             ; should not run
    jmp t6_after
t6:
    PREP
    mov ah, [pat_cf1_zf0]
    sahf
    ja  short t6_target        ; NOT taken
    mov ax, 0x6666
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 1,0

; ===================== 7) CMP A>B unsigned → CF=0,ZF=0 → taken =====================
t7:
    PREP
    mov al, 0x30
    cmp al, 0x20               ; 0x30 > 0x20 (unsigned)
    ja  short t7_taken
    mov ax, 0x7BAD
    jmp t7_after
t7_taken:
    mov ax, 0x7777
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7777
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,0
    jmp t8

t8_taken:
    mov ax, 0x8BAD
    jmp t8_after

; ===================== 8) CMP equal → ZF=1 → NOT taken =====================
t8:
    PREP
    mov al, 0x55
    cmp al, 0x55               ; ZF=1, CF=0
    ja  short t8_taken         ; NOT taken
    mov ax, 0x8888
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,1
    jmp t9

t9_taken:
    mov ax, 0x9BAD
    jmp t9_after

; ===================== 9) CMP A<B unsigned → CF=1 → NOT taken =====================
t9:
    PREP
    mov al, 0x10
    cmp al, 0x20               ; CF=1
    ja  short t9_taken         ; NOT taken
    mov ax, 0x9999
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9999
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 1,0
    jmp t10

t10_taken:
    mov ax, 0xABAD
    jmp t10_after

; ===================== 10) SUB with borrow → CF=1 → NOT taken =====================
t10:
    PREP
    mov al, 1
    sub al, 2                  ; CF=1, ZF=0
    ja  short t10_taken
    mov ax, 0xAAAA
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xAAAA
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 1,0

; ===================== 11) SUB without borrow, nonzero → CF=0,ZF=0 → taken =====================
t11:
    PREP
    mov al, 5
    sub al, 2                  ; 3 → CF=0, ZF=0
    ja  short t11_taken
    mov ax, 0xBBAD             ; never run
    jmp t11_after
t11_taken:
    mov ax, 0xBBBB
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xBBBB
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,0

; ===================== 12) ADD without carry, nonzero → CF=0,ZF=0 → taken =====================
t12:
    PREP
    mov al, 1
    add al, 2                  ; 3 → CF=0, ZF=0
    ja  short t12_taken
    mov ax, 0xC0DE
    jmp t12_after
t12_taken:
    mov ax, 0xCCCC
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xCCCC
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,0
    jmp t13

t13_taken:
    mov ax, 0xDBAD
    jmp t13_after

; ===================== 13) ADD with carry/zero → CF=1 & ZF=1 → NOT taken =====================
t13:
    PREP
    mov al, 0xFF
    add al, 1                  ; 0x00, CF=1, ZF=1
    ja  short t13_taken
    mov ax, 0xDDDD
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xDDDD
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 1,1
    jmp t14

t14_taken:
    mov ax, 0xEBAD
    jmp t14_after

; ===================== 14) Logical zero → ZF=1, CF=0 → NOT taken =====================
t14:
    PREP
    xor al, al
    or  al, 0                  ; ZF=1, CF=0
    ja  short t14_taken
    mov ax, 0xEEEE
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xEEEE
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,1
    ; (AF undefined after OR/TEST; we don't assert it.)

; ===================== 15) Flags preserved by JA (use SAHF with PF/AF/SF=1, CF=0,ZF=0) =====================
t15:
    PREP
    mov ah, [pat_mix_ok]       ; CF=0 ZF=0 so jump taken
    sahf
    ja  short t15_taken
    mov ax, 0xBEEF
    jmp t15_after
t15_taken:
    mov ax, 0xF111
t15_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_PF 1
    CHECK_AF 1
    CHECK_SF 1

; ===================== 16) DF preserved across JA =====================
t16:
    PREP
    std
    mov ah, [pat_cf0_zf0]
    sahf
    ja  short t16_taken
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
    mov ah, [pat_cf0_zf0]
    sahf
    ja  short t17_taken
    mov ax, 0x0DAD
    jmp t17_after
t17_taken:
    mov ax, 0xF333
t17_after:
    SAVE_FLAGS
    ASSERT_AX 0xF333
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,0
    jmp t18

t18_mid:
    mov ax, 0xFBAD
    jmp t18_after

; ===================== 18) Chain: first NOT taken (ZF=1), then set CF=0,ZF=0 and taken =====================
t18:
    PREP
    mov ah, [pat_cf0_zf1]      ; ZF=1 → not taken
    sahf
    ja  short t18_mid          ; NOT taken
    ; fall-through
    mov ah, [pat_cf0_zf0]      ; CF=0,ZF=0 → taken
    sahf
    ja  short t18_taken
    mov ax, 0xFACE
    jmp short t18_after
t18_taken:
    mov ax, 0xF444
t18_after:
    SAVE_FLAGS
    ASSERT_AX 0xF444
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,0
    jmp t19

t19_mid:
    mov ax, 0xFBAD
    jmp t19_after

; ===================== 19) Chain: first NOT taken (CF=1), then CLC to CF=0 (ZF=0) → taken =====================
t19:
    PREP
    mov ah, [pat_cf1_zf0]      ; CF=1,ZF=0
    sahf
    ja  short t19_mid          ; NOT taken
    ; fall-through
    ; Keep ZF=0, force CF=0
    mov ah, [pat_cf0_zf0]
    sahf
    ja  short t19_taken
    mov ax, 0xABCD
    jmp short t19_after
t19_taken:
    mov ax, 0xF555
t19_after:
    SAVE_FLAGS
    ASSERT_AX 0xF555
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,0
    jmp t20a

t20a_taken:
    mov ax, 0xFBAD
    jmp t20a_after

; ===================== 20) STC/CLC edge checks =====================
; (a) STC with ZF=0 → NOT taken
t20a:
    PREP
    mov ah, [pat_cf0_zf0]      ; start CF=0,ZF=0
    sahf
    stc                         ; CF=1
    ja  short t20a_taken        ; NOT taken
    mov ax, 0xF666
t20a_after:
    SAVE_FLAGS
    ASSERT_AX 0xF666
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 1,0
; (b) CLC with ZF=0 → taken
t20b:
    PREP
    mov ah, [pat_cf1_zf0]      ; start CF=1,ZF=0
    sahf
    clc                         ; CF=0, ZF still 0
    ja  short t20b_taken
    mov ax, 0x0BEE
    jmp t20b_after
t20b_taken:
    mov ax, 0xF777
t20b_after:
    SAVE_FLAGS
    ASSERT_AX 0xF777
    ASSERT_SP [sp0_store]
    CHECK_CF_ZF 0,0

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

; SAHF patterns (bit0=CF, bit6=ZF)
pat_cf0_zf0: db 0x00           ; CF=0, ZF=0  (JA should be taken)
pat_cf1_zf0: db 0x01           ; CF=1, ZF=0  (not taken)
pat_cf0_zf1: db 0x40           ; CF=0, ZF=1  (not taken)
pat_cf1_zf1: db 0x41           ; CF=1, ZF=1  (not taken)
pat_mix_ok : db 0x94           ; SF=1, AF=1, PF=1, ZF=0, CF=0  (JA should be taken)

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
