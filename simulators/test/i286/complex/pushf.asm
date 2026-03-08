; pushf.asm — Thorough tests for PUSHF (16-bit)
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
%macro ASSERT_MEMW 2
    mov ax, [%1]
    mov bx, %2
    int 0x23
%endmacro

; Check flag bits from [flags_store] (so checking doesn't alter FLAGS)
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
%macro CHECK_IF 1
    mov ax, [flags_store]
    mov cl, 9
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
%macro CHECK_OF 1
    mov ax, [flags_store]
    mov cl, 11
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

; Reserved/ignored bit checks on the PUSHF image (via [pushed_word]):
%macro CHECK_PUSHED_B1_FORCED_1 0
    mov ax, [pushed_word]
    mov cl, 1
    shr ax, cl
    and al, 1
    mov ah, 1
    int 0x22
%endmacro
%macro CHECK_PUSHED_B3_ZERO 0
    mov ax, [pushed_word]
    mov cl, 3
    shr ax, cl
    and al, 1
    mov ah, 0
    int 0x22
%endmacro
%macro CHECK_PUSHED_B5_ZERO 0
    mov ax, [pushed_word]
    mov cl, 5
    shr ax, cl
    and al, 1
    mov ah, 0
    int 0x22
%endmacro

; Stack helpers (SS=DS scratch stack)
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

; Helper: capture top-of-stack PUSHF image into [pushed_word] without altering it
%macro SNAP_PUSHED 0
    mov bp, sp
    mov ax, [ss:bp]
    mov [pushed_word], ax
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

; ========== 1) Basic: SAHF all-ones, IF=0 → PUSHF (expect 0x00D7) ==========
t1:
    PREP
    cli
    mov ah, [pat_all1]
    sahf
    ; AX used later to prove it’s unchanged
    mov ax, 0x1234
    pushf
    SAVE_FLAGS
    ; AX unchanged
    ASSERT_AX 0x1234
    ; SP delta & content
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_SP bx
    SNAP_PUSHED
    ASSERT_MEMW pushed_word, 0x00D7 ; 0xD5 + bit1
    CHECK_PUSHED_B1_FORCED_1
    CHECK_PUSHED_B3_ZERO
    CHECK_PUSHED_B5_ZERO
    ; cleanup
    add sp, 2
    ASSERT_SP [sp0_store]
    ; flags preserved as seeded by SAHF
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ========== 2) Basic: SAHF ZF=0 pattern → expect 0x0097 ==========
t2:
    PREP
    mov ah, [pat_zf0]
    sahf
    pushf
    SAVE_FLAGS
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_SP bx
    SNAP_PUSHED
    ASSERT_MEMW pushed_word, 0x0097
    CHECK_PUSHED_B1_FORCED_1
    CHECK_PUSHED_B3_ZERO
    CHECK_PUSHED_B5_ZERO
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ========== 3) IF only (via POPF), then PUSHF → expect 0x0202 ==========
t3:
    PREP
    mov ah, [pat_all0]
    sahf
    mov ax, 0x0200
    push ax
    popf                      ; IF=1, others 0 (normalized)
    pushf
    SAVE_FLAGS
    SNAP_PUSHED
    ASSERT_MEMW pushed_word, 0x0202
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_IF 1
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_DF 0
    CHECK_OF 0

; ========== 4) DF only (via POPF), then PUSHF → expect 0x0402 ==========
t4:
    PREP
    mov ah, [pat_all0]
    sahf
    mov ax, 0x0400
    push ax
    popf                      ; DF=1
    pushf
    SAVE_FLAGS
    SNAP_PUSHED
    ASSERT_MEMW pushed_word, 0x0402
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_DF 1
    CHECK_IF 0
    CHECK_OF 0

; ========== 5) OF only (via POPF), then PUSHF → expect 0x0802 ==========
t5:
    PREP
    mov ah, [pat_all0]
    sahf
    mov ax, 0x0800
    push ax
    popf                      ; OF=1
    pushf
    SAVE_FLAGS
    SNAP_PUSHED
    ASSERT_MEMW pushed_word, 0x0802
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_OF 1
    CHECK_IF 0
    CHECK_DF 0

; ========== 6) Mixed CF|PF|IF|DF (0x0605) → expect 0x0607 ==========
t6:
    PREP
    mov ax, 0x0605
    push ax
    popf                      ; CF=1 PF=1 IF=1 DF=1
    pushf
    SAVE_FLAGS
    SNAP_PUSHED
    ASSERT_MEMW pushed_word, 0x0607
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_IF 1
    CHECK_DF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_OF 0

; ========== 7) Reserved bits noise via POPF (attempt b1=0, b3/b5=1) then PUSHF → normalized image ==========
; POPF will already normalize; PUSHF must show bit1=1, bits3&5=0, others 0.
t7:
    PREP
    mov ax, 0x0028            ; bits3,5 set; bit1=0
    push ax
    popf                      ; → all definable flags 0, reserved normalized
    pushf
    SNAP_PUSHED
    ASSERT_MEMW pushed_word, 0x0002
    CHECK_PUSHED_B1_FORCED_1
    CHECK_PUSHED_B3_ZERO
    CHECK_PUSHED_B5_ZERO
    add sp, 2
    ASSERT_SP [sp0_store]

; ========== 8) Odd SP source ==========
t8:
    PREP_ODD
    mov ah, [pat_all1]
    sahf
    pushf
    SAVE_FLAGS
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_SP bx
    SNAP_PUSHED
    ASSERT_MEMW pushed_word, 0x00D7
    CHECK_PUSHED_B1_FORCED_1
    CHECK_PUSHED_B3_ZERO
    CHECK_PUSHED_B5_ZERO
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ========== 9) LIFO: PUSHF (A), then change flags, PUSHF (B) → [TOS]=B, [TOS+2]=A ==========
t9:
    PREP
    ; A: CF only → 0x0003 in image
    mov ah, [pat_all0]
    sahf
    stc
    pushf
    ; B: ZF only → image 0x0042
    mov ah, [pat_all0]
    sahf
    ; set only ZF via POPF to avoid instruction side effects
    mov ax, 0x0040
    push ax
    popf
    pushf
    ; check top two words
    mov bp, sp
    ASSERT_MEMW ss:bp,   0x0042
    ASSERT_MEMW ss:bp+2, 0x0003
    ; cleanup
    add sp, 4
    ASSERT_SP [sp0_store]

; ========== 10) AX unchanged across PUSHF ==========
t10:
    PREP
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xBEEF
    pushf
    SAVE_FLAGS
    ASSERT_AX 0xBEEF
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ========== 11) Multiple PUSHF delta: four pushes → SP0-8; last image matches ==========
t11:
    PREP
    mov ah, [pat_all0]
    sahf
    pushf                      ; #1 → 0x0002
    ; #2: set IF only
    mov ax, 0x0200
    push ax
    popf
    pushf                      ; #2 → 0x0202
    ; #3: set DF only
    mov ax, 0x0400
    push ax
    popf
    pushf                      ; #3 → 0x0402
    ; #4: set OF only
    mov ax, 0x0800
    push ax
    popf
    pushf                      ; #4 → 0x0802
    ; check SP and top image
    mov bx, [sp0_store]
    sub bx, 8
    ASSERT_SP bx
    SNAP_PUSHED
    ASSERT_MEMW pushed_word, 0x0802
    ; cleanup
    add sp, 8
    ASSERT_SP [sp0_store]

; ========== 12) Round-trip: PUSHF → POP AX → compare, SP restored ==========
t12:
    PREP
    mov ah, [pat_all1]
    sahf
    pushf
    ; read the pushed image via POP into AX
    pop ax
    ASSERT_AX 0x00D7
    ASSERT_SP [sp0_store]
    ; flags remain as seeded
    SAVE_FLAGS
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== Exit =====================
exit:
    RESTORE_DOS_STACK
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store:  dw 0
pushed_word:  dw 0
orig_ss:      dw 0
orig_sp:      dw 0
sp0_store:    dw 0

; Scratch stack (2 KB)
stack_buf:    times 2048 db 0xCC
stack_top     equ stack_buf + 2048

; SAHF patterns (control CF/PF/AF/ZF/SF only)
pat_all1: db 0xD5            ; SF=1 ZF=1 AF=1 PF=1 CF=1
pat_zf0:  db 0x95            ; SF=1 ZF=0 AF=1 PF=1 CF=1
pat_all0: db 0x00            ; SF=0 ZF=0 AF=0 PF=0 CF=0

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
