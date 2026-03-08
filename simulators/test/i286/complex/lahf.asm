; lahf.asm — Thorough tests for LAHF (16-bit)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH  (single-byte equality: AL == AH)
;
; LAHF → AH bits: SF(7) ZF(6) 0(5) AF(4) 0(3) PF(2) 1(1) CF(0)
; Flags and other regs are not modified by LAHF. AL is preserved.

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
%macro ASSERT_AL 1
    mov ah, 0
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_AH 1
    mov al, %1
    int 0x22
%endmacro
%macro ASSERT_SP_EQ 1
    mov ax, sp
    mov bx, %1
    int 0x23
%endmacro

; Check flags from [flags_store]
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

; Stack helpers (use SS=DS scratch stack so POPF/PUSHF are safe)
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

; Helper: set flags exactly (reserved bits normalize)
%macro SET_FLAGS 1
    push word %1
    popf
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

; ===================== 1) All flags clear → LAHF = 0x02; SP unchanged; AL preserved =====================
t1:
    PREP
    SET_FLAGS 0x0000
    mov ax, 0xBEEF
    lahf
    SAVE_FLAGS
    push ax
    ASSERT_AH 0x02              ; only bit1 set
    pop ax
    push ax
    ASSERT_AL 0xEF              ; AL preserved
    pop ax
    mov ah, 0
    mov bx, 0xEF
    int 0x23                    ; BL held old AL
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 0

; ===================== 2) CF=1 only → LAHF = 0x03 =====================
t2:
    PREP
    SET_FLAGS 0x0000
    stc
    lahf
    SAVE_FLAGS
    ASSERT_AH 0x03
    CHECK_CF 1
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    ASSERT_SP_EQ [sp0_store]

; ===================== 3) PF=1 only → LAHF = 0x06 =====================
t3:
    PREP
    SET_FLAGS 0x0004            ; PF
    lahf
    SAVE_FLAGS
    ASSERT_AH 0x06
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    ASSERT_SP_EQ [sp0_store]

; ===================== 4) AF=1 only → LAHF = 0x12 =====================
t4:
    PREP
    SET_FLAGS 0x0010            ; AF
    lahf
    SAVE_FLAGS
    ASSERT_AH 0x12
    CHECK_AF 1
    CHECK_CF 0
    CHECK_PF 0
    CHECK_ZF 0
    CHECK_SF 0
    ASSERT_SP_EQ [sp0_store]

; ===================== 5) ZF=1 only → LAHF = 0x42 =====================
t5:
    PREP
    SET_FLAGS 0x0040            ; ZF
    lahf
    SAVE_FLAGS
    ASSERT_AH 0x42
    CHECK_ZF 1
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_SF 0

; ===================== 6) SF=1 only → LAHF = 0x82 =====================
t6:
    PREP
    SET_FLAGS 0x0080            ; SF
    lahf
    SAVE_FLAGS
    ASSERT_AH 0x82
    CHECK_SF 1
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0

; ===================== 7) All five (CF|PF|AF|ZF|SF)=1, IF/DF/OF=0 → LAHF = 0xD7 =====================
t7:
    PREP
    SET_FLAGS 0x00D5
    lahf
    SAVE_FLAGS
    ASSERT_AH 0xD7
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 0

; ===================== 8) Only IF|DF|OF set → LAHF ignores them → 0x02 =====================
t8:
    PREP
    SET_FLAGS 0x0E00            ; IF=1 DF=1 OF=1
    lahf
    SAVE_FLAGS
    ASSERT_AH 0x02
    CHECK_IF 1
    CHECK_DF 1
    CHECK_OF 1

; ===================== 9) Mixed: IF|DF|OF + ZF|CF → LAHF = 0x43; flags preserved =====================
t9:
    PREP
    SET_FLAGS 0x0E41            ; IF|DF|OF|ZF|CF
    lahf
    SAVE_FLAGS
    ASSERT_AH 0x43
    CHECK_CF 1
    CHECK_ZF 1
    CHECK_IF 1
    CHECK_DF 1
    CHECK_OF 1

; ===================== 10) SAHF → LAHF round-trip (masking 0xD5 + set bit1) =====================
; Given input AH (any bits), SAHF uses only {7,6,4,2,0}; LAHF must read back:
;   (AH_in & 11010101b) | 00000010b
t10:
    PREP
    mov ah, 0xB7                ; 1011_0111b → SF=1 ZF=0 AF=1 PF=1 CF=1
    mov [ah_in], ah
    sahf
    SAVE_FLAGS
    lahf
    mov [ah_out], ah
    ; Build expected from ah_in
    mov al, [ah_in]
    and al, 11010101b
    or  al, 00000010b
    mov ah, [ah_out]
    int 0x22                    ; AL (expected) == AH (actual)
    ; Flags didn’t change across LAHF
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 11) AL is preserved across LAHF =====================
t11:
    PREP
    SET_FLAGS 0x00D5
    mov ax, 0x1234
    lahf
    SAVE_FLAGS
    push ax
    ASSERT_AL 0x34
    pop ax
    ASSERT_AH 0xD7

; ===================== 12) Other GPRs preserved (BX/CX/DX/SI/DI/BP) =====================
t12:
    PREP
    SET_FLAGS 0x0041            ; ZF|CF
    mov bx, 0xAAAA
    mov cx, 0xBBBB
    mov dx, 0xCCCC
    mov si, 0xDDDD
    mov di, 0xEEEE
    mov bp, 0xF00D
    lahf
    SAVE_FLAGS
    push ax
    mov ax, bx
    mov bx, 0xAAAA
    int 0x23
    mov ax, cx
    mov bx, 0xBBBB
    int 0x23
    mov ax, dx
    mov bx, 0xCCCC
    int 0x23
    mov ax, si
    mov bx, 0xDDDD
    int 0x23
    mov ax, di
    mov bx, 0xEEEE
    int 0x23
    mov ax, bp
    mov bx, 0xF00D
    int 0x23
    pop ax
    ASSERT_AH 0x43              ; ZF|CF + bit1

; ===================== 13) Idempotence: LAHF twice → same AH; flags unchanged =====================
t13:
    PREP
    SET_FLAGS 0x0014            ; AF|PF
    lahf
    mov [ah_once], ah
    lahf
    SAVE_FLAGS
    mov al, [ah_once]
    int 0x22                    ; AL == AH (same image)
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 0
    CHECK_ZF 0
    CHECK_SF 0

; ===================== Exit =====================
exit:
    RESTORE_DOS_STACK
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0
ah_in:       db 0
ah_out:      db 0
ah_once:     db 0
orig_ss:     dw 0
orig_sp:     dw 0
sp0_store:   dw 0

; Scratch stack (2 KB)
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
