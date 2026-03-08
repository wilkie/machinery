; pusha.asm — Thorough tests for PUSHA (16-bit)
; Harness:
;   int 0x23: AX == BX  (values/regs equality, SP/BP checks)
;   int 0x22: AL == AH  (single-bit flag checks from saved flags)

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
%macro ASSERT_BX 1
    mov ax, bx
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_CX 1
    mov ax, cx
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_DX 1
    mov ax, dx
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_SI 1
    mov ax, si
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_DI 1
    mov ax, di
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_BP 1
    mov ax, bp
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

; Flag-bit checks (preservation expected)
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

; Scratch stack helpers (SS=DS to our buffer)
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
    mov sp, stack_top - 0x81     ; odd SP allowed
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

    ; Use a safe scratch stack (SS=DS)
    SET_SCRATCH_STACK

; ===================== 1) Basic layout & SP delta =====================
; Preload regs; PUSHA should:
;   SP_final = SP0 - 16
;   [SS:SP]      = DI    (0x7777)
;   [SS:SP+2]    = SI    (0x6666)
;   [SS:SP+4]    = BP    (0x5555)
;   [SS:SP+6]    = SP0   (original SP)
;   [SS:SP+8]    = BX    (0x4444)
;   [SS:SP+10]   = DX    (0x3333)
;   [SS:SP+12]   = CX    (0x2222)
;   [SS:SP+14]   = AX    (0x1111)
; Flags preserved; GP regs unchanged.
t1:
    PREP
    mov ah, [pat_all1]
    sahf
    mov ax, 0x1111
    mov cx, 0x2222
    mov dx, 0x3333
    mov bx, 0x4444
    mov bp, 0x5555
    mov si, 0x6666
    mov di, 0x7777

    pusha
    SAVE_FLAGS

    ; SP delta
    mov bx, [sp0_store]
    sub bx, 16
    ASSERT_SP bx

    ; Read stack image via BP=SP
    mov bp, sp
    ASSERT_MEMW ss:bp,     0x7777
    ASSERT_MEMW ss:bp+2,   0x6666
    ASSERT_MEMW ss:bp+4,   0x5555
    ASSERT_MEMW ss:bp+6,   [sp0_store]
    ASSERT_MEMW ss:bp+8,   0x4444
    ASSERT_MEMW ss:bp+10,  0x3333
    ASSERT_MEMW ss:bp+12,  0x2222
    ASSERT_MEMW ss:bp+14,  0x1111

    ; A GP register (e.g., AX) unchanged
    ASSERT_AX 0x1111

    ; Flags preserved (as set by SAHF)
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_DF 0

    ; cleanup: pop 8 words to restore SP (don’t clobber regs)
    add sp, 16
    ASSERT_SP [sp0_store]

; ===================== 2) Second pattern, different flags =====================
t2:
    PREP
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x0A0A
    mov cx, 0x1B1B
    mov dx, 0x2C2C
    mov bx, 0x3D3D
    mov bp, 0x4E4E
    mov si, 0x5F5F
    mov di, 0x6060

    pusha
    SAVE_FLAGS

    mov bx, [sp0_store]
    sub bx, 16
    ASSERT_SP bx
    mov bp, sp
    ASSERT_MEMW ss:bp,     0x6060
    ASSERT_MEMW ss:bp+2,   0x5F5F
    ASSERT_MEMW ss:bp+4,   0x4E4E
    ASSERT_MEMW ss:bp+6,   [sp0_store]
    ASSERT_MEMW ss:bp+8,   0x3D3D
    ASSERT_MEMW ss:bp+10,  0x2C2C
    ASSERT_MEMW ss:bp+12,  0x1B1B
    ASSERT_MEMW ss:bp+14,  0x0A0A

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

    add sp, 16
    ASSERT_SP [sp0_store]

; ===================== 3) Odd SP (unaligned) =====================
t3:
    PREP_ODD
    mov ah, [pat_all1]
    sahf
    mov ax, 0xAAAA
    mov cx, 0xBBBB
    mov dx, 0xCCCC
    mov bx, 0xDDDD
    mov bp, 0xEEEE
    mov si, 0x9999
    mov di, 0x8888

    pusha
    SAVE_FLAGS

    mov bx, [sp0_store]
    sub bx, 16
    ASSERT_SP bx
    mov bp, sp
    ASSERT_MEMW ss:bp,     0x8888
    ASSERT_MEMW ss:bp+2,   0x9999
    ASSERT_MEMW ss:bp+4,   0xEEEE
    ASSERT_MEMW ss:bp+6,   [sp0_store]
    ASSERT_MEMW ss:bp+8,   0xDDDD
    ASSERT_MEMW ss:bp+10,  0xCCCC
    ASSERT_MEMW ss:bp+12,  0xBBBB
    ASSERT_MEMW ss:bp+14,  0xAAAA

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

    add sp, 16
    ASSERT_SP [sp0_store]

; ===================== 4) Round-trip with POPA (scramble regs in between) =====================
; PUSHA stacks the baseline values; scrambling regs doesn’t matter — POPA must restore from stack.
t4:
    PREP
    mov ah, [pat_zf0]
    sahf
    ; Baseline
    mov ax, 0x1111
    mov cx, 0x2222
    mov dx, 0x3333
    mov bx, 0x4444
    mov bp, 0x5555
    mov si, 0x6666
    mov di, 0x7777

    pusha
    ; Scramble
    mov ax, 0xAAAA
    mov cx, 0xBBBB
    mov dx, 0xCCCC
    mov bx, 0xDDDD
    mov bp, 0xEEEE
    mov si, 0x9999
    mov di, 0x8888

    popa
    SAVE_FLAGS

    push bx
    ASSERT_AX 0x1111
    ASSERT_CX 0x2222
    ASSERT_DX 0x3333
    pop  bx
    ASSERT_BX 0x4444
    ASSERT_BP 0x5555
    ASSERT_SI 0x6666
    ASSERT_DI 0x7777
    ASSERT_SP [sp0_store]

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 5) Prove SP-slot holds original SP (not SP0-2) =====================
; We expect [SS:SP_after + 6] == SP0 exactly.
t5:
    PREP
    mov ah, [pat_all1]
    sahf
    mov ax, 0x0101
    mov cx, 0x0202
    mov dx, 0x0303
    mov bx, 0x0404
    mov bp, 0x0505
    mov si, 0x0606
    mov di, 0x0707

    pusha
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp+6, [sp0_store]   ; proves “original SP” captured

    ; cleanup
    add sp, 16
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 6) DF preserved across PUSHA =====================
t6:
    PREP
    std
    mov ah, [pat_zf0]
    sahf
    mov ax, 1
    mov cx, 2
    mov dx, 3
    mov bx, 4
    mov bp, 5
    mov si, 6
    mov di, 7

    pusha
    SAVE_FLAGS
    CHECK_DF 1
    cld
    ; cleanup
    add sp, 16
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 7) LIFO shape: two PUSHA in a row =====================
; After two PUSHA: lower frame (oldest) at [SP0-16], newer at [SP0-32].
; Top TOS = DI from 2nd PUSHA; [TOS+16] = DI from 1st PUSHA.
t7:
    PREP
    mov ah, [pat_zf0]
    sahf
    ; First set of regs
    mov ax, 0x1010
    mov cx, 0x2020
    mov dx, 0x3030
    mov bx, 0x4040
    mov bp, 0x5050
    mov si, 0x6060
    mov di, 0x7070
    pusha
    ; Second set of regs
    mov ax, 0x1111
    mov cx, 0x2222
    mov dx, 0x3333
    mov bx, 0x4444
    mov bp, 0x5555
    mov si, 0x6666
    mov di, 0x7777
    pusha
    SAVE_FLAGS

    ; SP = SP0 - 32
    mov bx, [sp0_store]
    sub bx, 32
    ASSERT_SP bx

    mov bp, sp
    ; Top DI (from second PUSHA)
    ASSERT_MEMW ss:bp,     0x7777
    ; The DI from first PUSHA sits 16 bytes above the second frame’s base
    ASSERT_MEMW ss:bp+16,  0x7070

    ; cleanup both frames
    add sp, 32
    ASSERT_SP [sp0_store]

    ; flags preserved
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 8) PUSHA then peek each word and POPA back =====================
; Full-cycle correctness with memory peek between.
t8:
    PREP
    mov ah, [pat_all1]
    sahf
    mov ax, 0xA1A1
    mov cx, 0xB2B2
    mov dx, 0xC3C3
    mov bx, 0xD4D4
    mov bp, 0xE5E5
    mov si, 0x9696
    mov di, 0x8787

    pusha
    ; peek all 8 words
    mov bp, sp
    ASSERT_MEMW ss:bp,     0x8787
    ASSERT_MEMW ss:bp+2,   0x9696
    ASSERT_MEMW ss:bp+4,   0xE5E5
    ASSERT_MEMW ss:bp+6,   [sp0_store]
    ASSERT_MEMW ss:bp+8,   0xD4D4
    ASSERT_MEMW ss:bp+10,  0xC3C3
    ASSERT_MEMW ss:bp+12,  0xB2B2
    ASSERT_MEMW ss:bp+14,  0xA1A1

    popa
    SAVE_FLAGS

    push bx
    ASSERT_AX 0xA1A1
    ASSERT_CX 0xB2B2
    ASSERT_DX 0xC3C3
    pop bx
    ASSERT_BX 0xD4D4
    ASSERT_BP 0xE5E5
    ASSERT_SI 0x9696
    ASSERT_DI 0x8787
    ASSERT_SP [sp0_store]

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

; ---------------- Data / scratch ----------------
flags_store:  dw 0
orig_ss:      dw 0
orig_sp:      dw 0
sp0_store:    dw 0

; Scratch stack (2 KB)
stack_buf:    times 2048 db 0xCC
stack_top     equ stack_buf + 2048

pat_all1: db 0xD5            ; SF=1 ZF=1 AF=1 PF=1 CF=1
pat_zf0:  db 0x95            ; SF=1 ZF=0 AF=1 PF=1 CF=1
pat_all0: db 0x00

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
