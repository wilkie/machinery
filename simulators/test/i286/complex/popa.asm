; popa.asm — Thorough tests for POPA (16-bit)
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

; Flag checks (preservation expected)
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

    ; DS = CS, ES = DS
    push cs
    pop  ds
    push ds
    pop  es

    ; Use a safe scratch stack
    SET_SCRATCH_STACK

; ===================== 1) PUSHA → scramble → POPA round-trip =====================
; Expect registers restored to the pre-PUSHA values; SP returns to SP0; flags preserved.
t1:
    PREP
    mov ah, [pat_all1]
    sahf
    ; set known baseline
    mov ax, 0x1111
    mov cx, 0x2222
    mov dx, 0x3333
    mov bx, 0x4444
    mov bp, 0x5555
    mov si, 0x6666
    mov di, 0x7777

    pusha
    ; scramble everything
    mov ax, 0xAAAA
    mov cx, 0xBBBB
    mov dx, 0xCCCC
    mov bx, 0xDDDD
    mov bp, 0xEEEE
    mov si, 0x9999
    mov di, 0x8888

    ; seed flags again right before POPA (not necessary, but explicit)
    mov ah, [pat_all1]
    sahf
    popa
    SAVE_FLAGS

    push bx
    ASSERT_AX 0x1111
    ASSERT_CX 0x2222
    ASSERT_DX 0x3333
    pop bx
    ASSERT_BX 0x4444
    ASSERT_BP 0x5555
    ASSERT_SI 0x6666
    ASSERT_DI 0x7777
    ASSERT_SP [sp0_store]

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_DF 0

; ===================== 2) Same as (1) with odd SP (unaligned stack) =====================
t2:
    PREP_ODD
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
    mov ax, 0xAAAA
    mov cx, 0xBBBB
    mov dx, 0xCCCC
    mov bx, 0xDDDD
    mov bp, 0xEEEE
    mov si, 0x9999
    mov di, 0x8888

    mov ah, [pat_zf0]
    sahf
    popa
    SAVE_FLAGS

    push bx
    ASSERT_AX 0x0A0A
    ASSERT_CX 0x1B1B
    ASSERT_DX 0x2C2C
    pop bx
    ASSERT_BX 0x3D3D
    ASSERT_BP 0x4E4E
    ASSERT_SI 0x5F5F
    ASSERT_DI 0x6060
    ASSERT_SP [sp0_store]

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_DF 0

; ===================== 3) Manual layout: prove SP slot is ignored =====================
; We push: AX, CX, DX, BX, (SP slot sentinel), BP, SI, DI — in that order.
; Top of stack at POPA time is DI; POPA pops DI,SI,BP,(throws SP word),BX,DX,CX,AX.
t3:
    PREP
    ; layout using 80186+ "push imm16"
    push word 0x1111            ; AX
    push word 0x2222            ; CX
    push word 0x3333            ; DX
    push word 0x4444            ; BX
    push word 0xDEAD            ; SP slot (must be ignored)
    push word 0x5555            ; BP
    push word 0x6666            ; SI
    push word 0x7777            ; DI
    mov ah, [pat_all1]
    sahf
    popa
    SAVE_FLAGS

    push bx
    ASSERT_AX 0x1111
    ASSERT_CX 0x2222
    ASSERT_DX 0x3333
    pop bx
    ASSERT_BX 0x4444
    ASSERT_BP 0x5555
    ASSERT_SI 0x6666
    ASSERT_DI 0x7777
    ASSERT_SP [sp0_store]       ; proves SP was not loaded from 0xDEAD

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 4) Manual layout with odd SP; different SP slot sentinel =====================
t4:
    PREP_ODD
    push word 0xAAAA            ; AX
    push word 0xBBBB            ; CX
    push word 0xCCCC            ; DX
    push word 0xDDDD            ; BX
    push word 0xF00D            ; SP slot (ignored)
    push word 0xEEEE            ; BP
    push word 0x9999            ; SI
    push word 0x8888            ; DI
    mov ah, [pat_zf0]
    sahf
    popa
    SAVE_FLAGS

    push bx
    ASSERT_AX 0xAAAA
    ASSERT_CX 0xBBBB
    ASSERT_DX 0xCCCC
    pop bx
    ASSERT_BX 0xDDDD
    ASSERT_BP 0xEEEE
    ASSERT_SI 0x9999
    ASSERT_DI 0x8888
    ASSERT_SP [sp0_store]

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 5) PUSHA → modify SP-slot on stack → POPA =====================
; After PUSHA, the stack at [SP].. holds: DI,SI,BP,SP(orig),BX,DX,CX,AX.
; We corrupt the SP-slot (at [SP+6]) before POPA. POPA must still restore regs and SP to SP0.
t5:
    PREP
    mov ax, 0x0101
    mov cx, 0x0202
    mov dx, 0x0303
    mov bx, 0x0404
    mov bp, 0x0505
    mov si, 0x0606
    mov di, 0x0707
    pusha

    ; SP now = SP0-16; overwrite the SP-slot (4th from top) with a bad value
    mov bp, sp
    mov word [ss:bp+6], 0xEEEE
    mov ah, [pat_all1]
    sahf
    popa
    SAVE_FLAGS

    push bx
    ASSERT_AX 0x0101
    ASSERT_CX 0x0202
    ASSERT_DX 0x0303
    pop bx
    ASSERT_BX 0x0404
    ASSERT_BP 0x0505
    ASSERT_SI 0x0606
    ASSERT_DI 0x0707
    ASSERT_SP [sp0_store]       ; not 0xEEEE, and back to SP0

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 6) DF preserved across POPA =====================
t6:
    PREP
    std
    ; stack content for POPA (manual)
    push word 0xA1A1
    push word 0xB2B2
    push word 0xC3C3
    push word 0xD4D4
    push word 0xF00D      ; SP slot sentinel
    push word 0xE5E5
    push word 0x9696
    push word 0x8787
    mov ah, [pat_zf0]
    sahf
    popa
    SAVE_FLAGS
    push ax
    push cx
    CHECK_DF 1
    cld

    pop cx
    pop ax
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
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 7) SP delta check: net effect +16 no matter the values =====================
; Push 8 words (any values), note SP_after_push, POPA, verify SP returns to SP0 (thus +16 from SP_after_push).
t7:
    PREP
    mov ax, sp
    mov [sp_before_push], ax
    ; push any 8 words
    push word 1
    push word 2
    push word 3
    push word 4
    push word 5
    push word 6
    push word 7
    push word 8
    mov ax, sp
    mov [sp_after_push], ax
    mov ah, [pat_all1]
    sahf
    popa
    SAVE_FLAGS

    ; SP should be SP0 again
    ASSERT_SP [sp0_store]
    ; and the delta from after-push must be +16
    mov ax, [sp_after_push]
    add ax, 16
    ASSERT_SP ax

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
flags_store:   dw 0
orig_ss:       dw 0
orig_sp:       dw 0
sp0_store:     dw 0
sp_before_push: dw 0
sp_after_push:  dw 0

; Scratch stack (2 KB)
stack_buf:     times 2048 db 0xCC
stack_top      equ stack_buf + 2048

pat_all1: db 0xD5            ; SF=1 ZF=1 AF=1 PF=1 CF=1
pat_zf0:  db 0x95            ; SF=1 ZF=0 AF=1 PF=1 CF=1

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
