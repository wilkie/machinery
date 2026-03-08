; jp.asm — Thorough tests for JP/JPE (parity even) in 16-bit mode
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

; ===================== 1) JP taken (PF=1 via SAHF), forward short =====================
t1:
    PREP
    mov ah, [pat_pf1]          ; PF=1, others 0
    sahf
    jp  short t1_taken
    mov ax, 0xDEAD             ; (should not run)
    jmp t1_after
t1_taken:
    mov ax, 0x1111
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_PF 1
    CHECK_CF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0

; ===================== 2) JP not taken (PF=0 via SAHF), forward short =====================
t2:
    PREP
    mov ah, [pat_pf0s]         ; SF=1, PF=0
    sahf
    jp  short t2_taken
    mov ax, 0x2222             ; fall-through path (expected)
    jmp short t2_after
t2_taken:
    mov ax, 0x2BAD             ; (should not run)
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP [sp0_store]
    CHECK_PF 0
    CHECK_SF 1                 ; from SAHF
    CHECK_ZF 0
    CHECK_AF 0
    CHECK_CF 0

; ===================== 3) JP taken (PF=1), backward short =====================
t3_target:
    mov ax, 0x3333
    jmp t3_after
t3:
    PREP
    mov ah, [pat_pf1]
    sahf
    jp  short t3_target
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP [sp0_store]
    CHECK_PF 1

; ===================== 4) JP not taken (PF=0), backward short =====================
t4_target:
    mov ax, 0x4BAD             ; (should not run)
t4:
    PREP
    mov ah, [pat_all0]
    sahf                        ; PF=0
    jp  short t4_target
    mov ax, 0x4444
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP [sp0_store]
    CHECK_PF 0

; ===================== 5) JP after TEST (even parity → taken) =====================
; AL=0x03 has two bits set → even → PF=1
t5:
    PREP
    mov al, 0x03
    test al, al                 ; sets PF from AL parity
    jp   short t5_taken
    mov ax, 0x5BAD
    jmp t5_after
t5_taken:
    mov ax, 0x5555
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    ASSERT_SP [sp0_store]
    CHECK_PF 1

; ===================== 6) JP after TEST (odd parity → not taken) =====================
; AL=0x01 has one bit set → odd → PF=0
t6:
    PREP
    mov al, 0x01
    test al, al
    jp   short t6_taken
    mov ax, 0x6666              ; expected
    jmp short t6_after
t6_taken:
    mov ax, 0x6BAD
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_SP [sp0_store]
    CHECK_PF 0

; ===================== 7) JP after XOR AL,AL → result 0 → even parity → taken =====================
t7:
    PREP
    mov al, 0x7F
    xor al, al                  ; 0 → even → PF=1, ZF=1
    jp  short t7_taken
    mov ax, 0x7BAD
    jmp t7_after
t7_taken:
    mov ax, 0x7777
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7777
    ASSERT_SP [sp0_store]
    CHECK_PF 1
    CHECK_ZF 1                  ; from XOR

; ===================== 8) JP after ADD → wrap to 0x00 → even parity → taken =====================
t8:
    PREP
    mov al, 0xFF
    add al, 1                   ; 0x00 → even parity, ZF=1, PF=1
    jp  short t8_taken
    mov ax, 0x8BAD
    jmp t8_after
t8_taken:
    mov ax, 0x8888
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    ASSERT_SP [sp0_store]
    CHECK_PF 1
    CHECK_ZF 1

; ===================== 9) Flags preserved by JP (pattern via SAHF) =====================
t9:
    PREP
    mov ah, [pat_mix]           ; set CF,PF,AF,ZF,SF=1
    sahf
    jp  short t9_taken          ; PF=1 → taken
    mov ax, 0x9BAD
    jmp t9_after
t9_taken:
    mov ax, 0x9999
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9999
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 10) DF preserved across JP =====================
t10:
    PREP
    std
    mov ah, [pat_pf1]           ; PF=1
    sahf
    jp  short t10_taken
    mov ax, 0xABAD
    jmp t10_after
t10_taken:
    mov ax, 0xAAAA
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xAAAA
    ASSERT_SP [sp0_store]
    CHECK_DF 1
    cld

; ===================== 11) Odd SP (unaligned stack) =====================
t11:
    PREP_ODD
    mov ah, [pat_pf1]           ; PF=1
    sahf
    jp  short t11_taken
    mov ax, 0xBBAD
    jmp t11_after
t11_taken:
    mov ax, 0xBBBB
t11_after:
    SAVE_FLAGS
    ASSERT_AX 0xBBBB
    ASSERT_SP [sp0_store]
    CHECK_PF 1

; ===================== 12) Chain: first JP taken, second JP not taken =====================
; Step 1: PF=1 → jump to mid; Step 2 at mid: set PF=0, JP should NOT jump.
t12:
    PREP
    mov ah, [pat_pf1]           ; PF=1
    sahf
    jp  short t12_mid
    mov ax, 0xCBAD              ; should not run
    jmp short t12_after
t12_mid:
    mov ah, [pat_all0]          ; PF=0 now
    sahf
    jp  short t12_bad           ; should NOT take
    mov ax, 0xCCCC              ; expected result
    jmp short t12_after
t12_bad:
    mov ax, 0xC0DE
t12_after:
    SAVE_FLAGS
    ASSERT_AX 0xCCCC
    ASSERT_SP [sp0_store]
    CHECK_PF 0                  ; final PF from last SAHF

; ===================== 13) Backward not-taken with mixed flags intact =====================
t13_tgt:
    mov ax, 0xD0D0              ; (should not run)
t13:
    PREP
    mov ah, [pat_mix]           ; PF=1 initially
    sahf
    ; flip PF to 0 using TEST with odd parity
    mov al, 0x01
    test al, al                 ; PF=0 now
    jp  short t13_tgt           ; not taken
    mov ax, 0xDDDD
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xDDDD
    ASSERT_SP [sp0_store]
    CHECK_PF 0
    ; the other flags depend on TEST: ZF=0, SF=0, AF is undefined by TEST (but Intel leaves AF undefined).
    ; We avoid asserting AF/ZF/SF here beyond PF.

; ===================== 14) JP taken after zero-result from OR (0→PF=1) =====================
t14:
    PREP
    xor al, al
    or  al, al                  ; result 0, PF=1 (even)
    jp  short t14_taken
    mov ax, 0xEBAD
    jmp t14_after
t14_taken:
    mov ax, 0xEEEE
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xEEEE
    ASSERT_SP [sp0_store]
    CHECK_PF 1

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

pat_all0: db 0x00
pat_pf1 : db 0x04              ; SAHF → PF=1 only
pat_pf0s: db 0x80              ; SAHF → SF=1 only (PF=0)
pat_mix : db 0xD5              ; SAHF → CF,PF,AF,ZF,SF = 1

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
