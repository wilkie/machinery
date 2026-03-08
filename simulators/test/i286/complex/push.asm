; push.asm — Thorough tests for PUSH (r16 / r/m16 / Sreg / imm16 / imm8) in 16-bit mode
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH  (single-flag checks via saved FLAGS)

bits 16
org 0x100

; ---------- Macros ----------
%macro SAVE_FLAGS 0
    pushf
    pop  ax
    mov  [flags_store], ax
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

; Flag-bit checks (preservation)
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

pat_all1: db 0xD5            ; SF=1 ZF=1 AF=1 PF=1 CF=1
pat_zf0:  db 0x95            ; SF=1 ZF=0 AF=1 PF=1 CF=1

    ; Use a safe scratch stack (SS=DS)
    SET_SCRATCH_STACK

; ===================== 1) PUSH AX (basic) =====================
t1:
    PREP
    mov ah, [pat_all1]
    sahf
    mov ax, 0x1234
    push ax
    SAVE_FLAGS

    ; SP = SP0 - 2, [SS:SP] = 0x1234
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_SP bx
    mov bp, sp                   ; read TOS via [SS:BP]
    ASSERT_MEMW ss:bp, 0x1234

    ; drop the word without POP
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_DF 0

; ===================== 2) PUSH BX/CX/DX/SI/DI/BP variety =====================
t2_bx:
    PREP
    mov bx, 0xBABA
    mov ah, [pat_zf0]
    sahf
    push bx
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0xBABA
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
t2_cx:
    PREP
    mov cx, 0xCAFE
    mov ah, [pat_all1]
    sahf
    push cx
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0xCAFE
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
t2_dx:
    PREP
    mov dx, 0x0F0E
    mov ah, [pat_zf0]
    sahf
    push dx
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0x0F0E
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
t2_si:
    PREP
    mov si, 0x2468
    mov ah, [pat_all1]
    sahf
    push si
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0x2468
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
t2_di:
    PREP
    mov di, 0x1357
    mov ah, [pat_zf0]
    sahf
    push di
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0x1357
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
t2_bp:
    PREP
    mov bp, 0xABCD
    mov ah, [pat_all1]
    sahf
    push bp
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0xABCD
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 3) PUSH SP special-case (value pushed = new SP) =====================
; After PUSH SP:
;   newSP = SP0 - 2 ; memory at [newSP] must contain newSP.
t3:
    PREP
    mov ah, [pat_zf0]
    sahf
    push sp
    SAVE_FLAGS

    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_SP bx          ; SP == SP0-2
    mov bp, sp
    mov bx, [sp0_store]
    ASSERT_MEMW ss:bp, bx ; stored value equals new SP

    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 4) PUSH r/m16 → DS:[mem] =====================
t4_mds:
    PREP
    mov word [mem_ds_0], 0xCAFE
    mov ah, [pat_all1]
    sahf
    push word [mem_ds_0]
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0xCAFE
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 5) PUSH r/m16 where addr uses [BP] (defaults to SS) =====================
t5_mbp:
    PREP
    mov word [mem_ss_0], 0xDEAD
    mov bp, mem_ss_0                ; [BP] uses SS by default (SS=DS here)
    mov ah, [pat_zf0]
    sahf
    push word [bp]
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0xDEAD
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 6) PUSH r/m16 with ES override (read via ES:) =====================
t6_mes:
    PREP
    mov word [mem_es_0], 0x1BAD
    mov ah, [pat_all1]
    sahf
    push word [es:mem_es_0]         ; ES==DS here
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0x1BAD
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 7) PUSH with addressing forms (BX+SI+disp, BP+DI-5) =====================
t7_ea1:
    PREP
    mov bx, base_ea
    mov si, 3
    mov word [base_ea+5], 0xAAAA
    mov ah, [pat_zf0]
    sahf
    push word [bx+si+2]             ; -> [base_ea+5]
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0xAAAA
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

t7_ea2:
    PREP
    mov bp, base_ea
    mov di, 7
    mov word [base_ea+2], 0xBEEF
    mov ah, [pat_all1]
    sahf
    push word [bp+di-5]             ; -> [base_ea+2]
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0xBEEF
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 8) PUSH imm16 (80186+) =====================
t8_imm16:
    PREP
    mov ah, [pat_zf0]
    sahf
    push word 0xBADA
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0xBADA
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 9) PUSH imm8 sign-extended (80186+) =====================
; 0x7F -> 0x007F ; 0x80 -> 0xFF80
t9_imm8_pos:
    PREP
    mov ah, [pat_all1]
    sahf
    db 0x6A, 0x7F                  ; PUSH imm8 0x7F
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0x007F
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

t9_imm8_neg:
    PREP
    mov ah, [pat_zf0]
    sahf
    db 0x6A, 0x80                  ; PUSH imm8 0x80 → 0xFF80
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0xFF80
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 10) PUSH CS/DS/ES/SS =====================
t10_cs:
    PREP
    mov ah, [pat_all1]
    sahf
    push cs
    SAVE_FLAGS
    mov bp, sp
    mov ax, cs
    ASSERT_MEMW ss:bp, ax
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

t10_ds:
    PREP
    mov ah, [pat_zf0]
    sahf
    push ds
    SAVE_FLAGS
    mov bp, sp
    mov ax, ds
    ASSERT_MEMW ss:bp, ax
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

t10_es:
    PREP
    mov ah, [pat_all1]
    sahf
    push es
    SAVE_FLAGS
    mov bp, sp
    mov ax, es
    ASSERT_MEMW ss:bp, ax
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

t10_ss:
    PREP
    mov ah, [pat_zf0]
    sahf
    push ss
    SAVE_FLAGS
    mov bp, sp
    mov ax, ss
    ASSERT_MEMW ss:bp, ax
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    ; (Next-instruction interrupt inhibition is irrelevant for this harness.)

; ===================== 11) Odd SP (unaligned stack) =====================
t11_odd:
    PREP_ODD
    mov ah, [pat_all1]
    sahf
    mov ax, 0x0F0E
    push ax
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp, 0x0F0E
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 12) DF preserved across PUSH =====================
t12_df:
    PREP
    std
    mov dx, 0x1357
    mov ah, [pat_zf0]
    sahf
    push dx
    SAVE_FLAGS
    CHECK_DF 1
    cld
    mov bp, sp
    ASSERT_MEMW ss:bp, 0x1357
    add sp, 2
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 13) LIFO shape check with two pushes (no POP used) =====================
; Push A then B; TOS must be B; [TOS+2] must be A. We use BP=SP to read both.
t13_lifo2:
    PREP
    mov ah, [pat_all1]
    sahf
    mov ax, 0x1111
    mov bx, 0x2222
    push ax
    push bx
    SAVE_FLAGS
    mov bp, sp
    ASSERT_MEMW ss:bp,     0x2222
    ASSERT_MEMW ss:bp+2,   0x1111
    add sp, 4
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 14) SP delta check for multiple pushes =====================
t14_delta:
    PREP
    mov ax, sp
    mov [sp_before], ax
    mov ah, [pat_zf0]
    sahf
    push word 1
    push word 2
    push word 3
    push word 4
    SAVE_FLAGS
    mov ax, [sp_before]
    sub ax, 8
    ASSERT_SP ax
    ; cleanup
    add sp, 8
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
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
sp_before:    dw 0

mem_ds_0:     dw 0x7E7E
mem_es_0:     dw 0x7E7E
mem_ss_0:     dw 0x7E7E

base_ea:      db 0xCC
              dw 0x0000, 0x0000   ; room for +2..+5 words

; Scratch stack (2 KB)
stack_buf:    times 2048 db 0xCC
stack_top     equ stack_buf + 2048

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
