; jbe.asm — Thorough tests for JBE/JNA (CF==1 || ZF==1) in 16-bit mode
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

; ===================== 1) JBE taken: CF=1, ZF=0 (via SAHF) =====================
t1:
    PREP
    mov ah, [pat_cf1_zf0]
    sahf
    jbe short t1_taken
    mov ax, 0xDEAD             ; should not run
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_ZF 0

; ===================== 2) JBE taken: CF=0, ZF=1 (via SAHF) =====================
t2:
    PREP
    mov ah, [pat_cf0_zf1]
    sahf
    jbe short t2_taken
    mov ax, 0x2BAD
    jmp t2_after
t2_taken:
    mov ax, 0x2222
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 1

; ===================== 3) JBE taken: CF=1, ZF=1 (via SAHF) =====================
t3:
    PREP
    mov ah, [pat_cf1_zf1]
    sahf
    jbe short t3_taken
    mov ax, 0x3BAD
    jmp t3_after
t3_taken:
    mov ax, 0x3333
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_ZF 1
    jmp t4

t4_not_taken:
    mov ax, 0x4BAD
    jmp t4_after

; ===================== 4) JBE not taken: CF=0, ZF=0 (via SAHF) =====================
t4:
    PREP
    mov ah, [pat_cf0_zf0]
    sahf
    jbe short t4_not_taken     ; NOT taken
    mov ax, 0x4444             ; fall-through expected
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 0
    jmp t5

; ===================== 5) Backward taken (CF=1, ZF=0) =====================
t5_target:
    mov ax, 0x5555
    jmp t5_after
t5:
    PREP
    mov ah, [pat_cf1_zf0]
    sahf
    jbe short t5_target
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_ZF 0

; ===================== 6) Backward not taken (CF=0, ZF=0) =====================
t6_target:
    mov ax, 0x6BAD             ; should not run
t6:
    PREP
    mov ah, [pat_cf0_zf0]
    sahf
    jbe short t6_target        ; NOT taken
    mov ax, 0x6666
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 0

; ===================== 7) CMP equal → ZF=1 (CF=0) → taken =====================
t7:
    PREP
    mov al, 0x20
    cmp al, 0x20               ; equal → ZF=1 CF=0
    jbe short t7_taken
    mov ax, 0x7BAD
    jmp t7_after
t7_taken:
    mov ax, 0x7777
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7777
    ASSERT_SP [sp0_store]
    CHECK_ZF 1
    CHECK_CF 0

; ===================== 8) CMP A<B unsigned → CF=1, ZF=0 → taken =====================
t8:
    PREP
    mov al, 0x10
    cmp al, 0x20               ; borrow → CF=1, ZF=0
    jbe short t8_taken
    mov ax, 0x8BAD
    jmp t8_after
t8_taken:
    mov ax, 0x8888
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_ZF 0

; ===================== 9) CMP A>B unsigned → CF=0, ZF=0 → NOT taken =====================
t9:
    PREP
    mov al, 0x30
    cmp al, 0x20               ; CF=0, ZF=0
    jbe short t9_taken
    mov ax, 0x9999             ; expected path
    jmp short t9_after
t9_taken:
    mov ax, 0x9BAD
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9999
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 0

; ===================== 10) SUB with borrow → CF=1 → taken =====================
t10:
    PREP
    mov al, 1
    sub al, 2                  ; borrow → CF=1, ZF=0
    jbe short t10_taken
    mov ax, 0xABAD
    jmp t10_after
t10_taken:
    mov ax, 0xAAAA
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xAAAA
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_ZF 0

; ===================== 11) ADD with carry → CF=1 → taken =====================
t11:
    PREP
    mov al, 0xFF
    add al, 1                  ; 0x00, CF=1, ZF=1
    jbe short t11_taken
    mov ax, 0xBBAD             ; never run
    jmp t11_after
t11_taken:
    mov ax, 0xBBBB
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xBBBB
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_ZF 1

; ===================== 12) TEST zero → CF=0 (cleared), ZF=1 → taken =====================
t12:
    PREP
    mov al, 0x00
    test al, al                ; ZF=1 PF=1, CF=OF=0
    jbe short t12_taken
    mov ax, 0xCBAD
    jmp t12_after
t12_taken:
    mov ax, 0xCCCC
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xCCCC
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 1
    ; (Don’t assert AF here: undefined after TEST.)

; ===================== 13) OR to zero (AL already 0) → ZF=1 → taken =====================
t13:
    PREP
    xor al, al
    or  al, 0                  ; logicals clear CF,OF; ZF=1
    jbe short t13_taken
    mov ax, 0xDBAD
    jmp t13_after
t13_taken:
    mov ax, 0xDDDD
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xDDDD
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 1

; ===================== 14) Flags preserved by JBE (use SAHF mask with PF/AF/SF=1, ZF=1, CF=0) =====================
t14:
    PREP
    mov ah, [pat_mix_cf0z1]    ; CF=0, ZF=1, PF/AF/SF=1
    sahf
    jbe short t14_taken        ; taken
    mov ax, 0xEBAD
    jmp t14_after
t14_taken:
    mov ax, 0xEEEE
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xEEEE
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_SF 1

; ===================== 15) DF preserved across JBE =====================
t15:
    PREP
    std
    mov ah, [pat_cf0_zf1]      ; ensure taken via ZF=1
    sahf
    jbe short t15_taken
    mov ax, 0xF0F0
    jmp t15_after
t15_taken:
    mov ax, 0xF111
t15_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
    ASSERT_SP [sp0_store]
    CHECK_DF 1
    cld

; ===================== 16) Odd SP (unaligned stack) =====================
t16:
    PREP_ODD
    mov ah, [pat_cf1_zf0]      ; CF=1 → taken
    sahf
    jbe short t16_taken
    mov ax, 0xF2F2
    jmp t16_after
t16_taken:
    mov ax, 0xF222
t16_after:
    SAVE_FLAGS
    ASSERT_AX 0xF222
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_ZF 0
    jmp t17

t17_mid:
    mov ax, 0x1BAD
    jmp t17_after

; ===================== 17) Chain: first NOT taken (CF=0,ZF=0), then set ZF=1 and taken =====================
t17:
    PREP
    mov ah, [pat_cf0_zf0]
    sahf
    jbe short t17_mid          ; not taken
    ; fall-through
    mov ah, [pat_cf0_zf1]      ; now ZF=1
    sahf
    jbe short t17_taken        ; taken
    mov ax, 0xFACE
    jmp short t17_after
t17_taken:
    mov ax, 0xF333
t17_after:
    SAVE_FLAGS
    ASSERT_AX 0xF333
    ASSERT_SP [sp0_store]
    CHECK_ZF 1

; ===================== 18) Backward not-taken with CF/ZF both 0; other flags left intact (from SAHF) =====================
t18_target:
    mov ax, 0xBEEF             ; should not run
t18:
    PREP
    mov ah, [pat_cf0_zf0]
    sahf
    jbe short t18_target       ; NOT taken
    mov ax, 0xF444
t18_after:
    SAVE_FLAGS
    ASSERT_AX 0xF444
    ASSERT_SP [sp0_store]
    CHECK_CF 0
    CHECK_ZF 0

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

; SAHF patterns controlling CF/ZF specifically
pat_cf0_zf0: db 0x00           ; CF=0 ZF=0
pat_cf1_zf0: db 0x01           ; CF=1 ZF=0
pat_cf0_zf1: db 0x40           ; CF=0 ZF=1
pat_cf1_zf1: db 0x41           ; CF=1 ZF=1
pat_mix_cf0z1: db 0xD4         ; SF=ZF=AF=PF=1, CF=0  (jump should be taken)
pat_mix_cf0z0: db 0x95         ; SF=1 ZF=0 AF=1 PF=1 CF=1? -> NO, 0x95 has CF=1; not suitable
                              ; We'll avoid using this one; keep focused patterns above.

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
