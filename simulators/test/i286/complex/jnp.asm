; jnp.asm — Thorough tests for JNP/JPO (parity odd) in 16-bit mode
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

; ===================== 1) JNP taken (PF=0 via SAHF), forward short =====================
t1:
    PREP
    mov ah, [pat_all0]         ; PF=0
    sahf
    jnp short t1_taken
    mov ax, 0xDEAD             ; (should not run)
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_PF 0
    CHECK_CF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; ===================== 2) JNP not taken (PF=1 via SAHF), forward short =====================
t2:
    PREP
    mov ah, [pat_pf1]          ; PF=1
    sahf
    jnp short t2_taken         ; not taken
    mov ax, 0x2222             ; expected fall-through
    jmp short t2_after
t2_taken:
    mov ax, 0x2BAD             ; (should not run)
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    CHECK_PF 1
    CHECK_CF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    jmp t3

; ===================== 3) JNP taken (PF=0), backward short =====================
t3_target:
    mov ax, 0x3333
    jmp t3_after
t3:
    PREP
    mov ah, [pat_pf0s]         ; SF=1, PF=0
    sahf
    jnp short t3_target
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP [sp0_store]
    CHECK_PF 0
    CHECK_SF 1

; ===================== 4) JNP not taken (PF=1), backward short =====================
t4_target:
    mov ax, 0x4BAD             ; (should not run)
t4:
    PREP
    mov ah, [pat_pf1]
    sahf                        ; PF=1
    jnp short t4_target         ; NOT taken
    mov ax, 0x4444
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP [sp0_store]
    CHECK_PF 1

; ===================== 5) JNP after TEST (odd parity → taken) =====================
; AL=0x01 → odd → PF=0 → jump taken
t5:
    PREP
    mov al, 0x01
    test al, al
    jnp short t5_taken
    mov ax, 0x5BAD
    jmp t5_after
t5_taken:
    mov ax, 0x5555
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    ASSERT_SP [sp0_store]
    CHECK_PF 0

; ===================== 6) JNP after TEST (even parity → not taken) =====================
; AL=0x03 → two bits → PF=1 → not taken
t6:
    PREP
    mov al, 0x03
    test al, al
    jnp short t6_taken
    mov ax, 0x6666              ; expected
    jmp short t6_after
t6_taken:
    mov ax, 0x6BAD
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_SP [sp0_store]
    CHECK_PF 1

; ===================== 7) JNP after XOR → odd parity result → taken =====================
; 0x03 ^ 0x01 = 0x02 (1 bit set) → PF=0 → taken
t7:
    PREP
    mov al, 0x03
    xor al, 0x01
    jnp short t7_taken
    mov ax, 0x7BAD
    jmp t7_after
t7_taken:
    mov ax, 0x7777
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7777
    ASSERT_SP [sp0_store]
    CHECK_PF 0

; ===================== 8) JNP after XOR → zero result (even) → not taken =====================
; 0x7F ^ 0x7F = 0x00 → PF=1 → not taken
t8:
    PREP
    mov al, 0x7F
    xor al, 0x7F
    jnp short t8_taken
    mov ax, 0x8888              ; expected
    jmp short t8_after
t8_taken:
    mov ax, 0x8BAD
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    ASSERT_SP [sp0_store]
    CHECK_PF 1
    CHECK_ZF 1

; ===================== 9) JNP after ADD → odd parity result → taken =====================
; 0x00 + 0x01 = 0x01 (odd) → PF=0 → taken
t9:
    PREP
    mov al, 0x00
    add al, 0x01
    jnp short t9_taken
    mov ax, 0x9BAD
    jmp t9_after
t9_taken:
    mov ax, 0x9999
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9999
    ASSERT_SP [sp0_store]
    CHECK_PF 0

; ===================== 10) JNP after ADD → zero (even) → not taken =====================
; 0xFF + 0x01 = 0x00 → PF=1 → not taken
t10:
    PREP
    mov al, 0xFF
    add al, 1
    jnp short t10_taken
    mov ax, 0xAAAA              ; expected
    jmp short t10_after
t10_taken:
    mov ax, 0xABAD
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xAAAA
    ASSERT_SP [sp0_store]
    CHECK_PF 1
    CHECK_ZF 1

; ===================== 11) Flags preserved by JNP (pattern via SAHF) =====================
t11:
    PREP
    mov ah, [pat_mix]           ; CF=PF=AF=ZF=SF=1
    sahf
    jnp short t11_skip          ; PF=1 → NOT taken
    mov ax, 0xBEEF              ; should not run
t11_skip:
    mov ax, 0xBBBB
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xBBBB
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 12) DF preserved across JNP =====================
t12:
    PREP
    std
    mov ah, [pat_all0]          ; PF=0
    sahf
    jnp short t12_taken         ; taken
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
    mov ah, [pat_all0]          ; PF=0
    sahf
    jnp short t13_taken         ; taken
    mov ax, 0xDBAD
    jmp t13_after
t13_taken:
    mov ax, 0xDDDD
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xDDDD
    ASSERT_SP [sp0_store]
    CHECK_PF 0
    jmp t14

t14_mid:
    mov ax, 0xFBAD
    jmp t14_after

; ===================== 14) Chain: first not taken (PF=1), then set PF=0 and taken =====================
t14:
    PREP
    mov ah, [pat_pf1]           ; PF=1 → not taken
    sahf
    jnp short t14_mid           ; NOT taken
    ; fall-through
    mov ah, [pat_all0]          ; PF=0 now
    sahf
    jnp short t14_taken         ; taken
    mov ax, 0xEBAD
    jmp short t14_after
t14_taken:
    mov ax, 0xEEEE
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xEEEE
    ASSERT_SP [sp0_store]
    CHECK_PF 0

; ===================== 15) JNP taken after OR producing odd parity =====================
; Start AL=1, OR 0 keeps 1 → PF=0 → taken
t15:
    PREP
    mov al, 1
    or  al, 0
    jnp short t15_taken
    mov ax, 0xFBAD             ; never run; (just keep code clean)
    jmp t15_after
t15_taken:
    mov ax, 0xF111
t15_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
    ASSERT_SP [sp0_store]
    CHECK_PF 0

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

; Patterns for SAHF (affect CF/PF/AF/ZF/SF only)
pat_all0: db 0x00              ; PF=0 (and others 0)
pat_pf1 : db 0x04              ; PF=1 only
pat_pf0s: db 0x80              ; SF=1 only → PF=0
pat_mix : db 0xD5              ; CF,PF,AF,ZF,SF = 1

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
