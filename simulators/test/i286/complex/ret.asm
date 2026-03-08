; ret.asm — Thorough tests for RET (near) and RETF (far) in 16-bit mode
; Harness:
;   int 0x23: assert AX == BX  (values/regs equality, SP equality)
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
%macro ASSERT_SP_EQ 1
    mov ax, sp
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_MEMW 2
    mov ax, [%1]
    mov bx, %2
    int 0x23
%endmacro

; Flag checks (expect preservation)
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

; Scratch stack helpers (SS=DS to our local buffer)
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

start:
    jmp common_start

; ---------- Near callees (use MOV only; record return IP; then RET) ----------
near_ret_plain:
    mov bx, sp
    mov ax, [ss:bx]
    mov [near_cap_ip1], ax
    mov ax, 0x1111
    ret

near_ret_imm4:
    mov bx, sp
    mov ax, [ss:bx]          ; return IP
    mov [near_cap_ip2], ax
    mov ax, [ss:bx+2]        ; top arg (pushed last)
    mov [near_cap_arg_top], ax
    mov ax, [ss:bx+4]        ; next arg
    mov [near_cap_arg_next], ax
    mov ax, 0x2222
    ret 4

near_ret_plain2:
    mov bx, sp
    mov ax, [ss:bx]
    mov [near_cap_ip3], ax
    mov ax, 0x3333
    ret

; Nested: A calls B then returns
near_inner:
    mov bx, sp
    mov ax, [ss:bx]
    mov [near_cap_ip_inner], ax
    mov ax, 0x4444
    ret
near_outer:
    mov bx, sp
    mov ax, [ss:bx]
    mov [near_cap_ip_outer], ax
    ; call inner and return
    call near_inner
    mov ax, 0x5555
    ret

near_ret_imm8:
    mov bx, sp
    mov ax, [ss:bx]
    mov [near_cap_ip4], ax
    mov ax, [ss:bx+2]
    mov [near_cap_a1], ax
    mov ax, [ss:bx+4]
    mov [near_cap_a2], ax
    mov ax, [ss:bx+6]
    mov [near_cap_a3], ax
    mov ax, [ss:bx+8]
    mov [near_cap_a4], ax
    mov ax, 0x6666
    ret 8

; ---------- Far callee templates (relocated to CS+1 at runtime) ----------
; #1: RETF (no immediate)
far_plain_start:
    mov bx, sp
    mov ax, [ss:bx]          ; return IP
    mov [far_cap_ip1], ax
    mov ax, [ss:bx+2]        ; return CS
    mov [far_cap_cs1], ax
    mov ax, 0xF111
    retf
far_plain_end:

; #2: RETF 4 (discard 2 words of params)
far_imm4_start:
    mov bx, sp
    mov ax, [ss:bx]
    mov [far_cap_ip2], ax
    mov ax, [ss:bx+2]
    mov [far_cap_cs2], ax
    mov ax, [ss:bx+4]        ; top arg (pushed last)
    mov [far_cap_a1], ax
    mov ax, [ss:bx+6]        ; next arg
    mov [far_cap_a2], ax
    mov ax, 0xF222
    retf 4
far_imm4_end:

FAR_OFF1  equ 0x0200    ; CS+1:FAR_OFF1 → far_plain
FAR_OFF2  equ 0x0300    ; CS+1:FAR_OFF2 → far_imm4

; ---------- Start ----------
common_start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; DS=CS, ES=DS
    push cs
    pop  ds
    push ds
    pop  es

    SET_SCRATCH_STACK

; ===================== Near RET =====================

; 1) Basic near RET (rel16 call) — return IP captured, SP restored, flags preserved
t1:
    PREP
    mov ah, [pat_all1]
    sahf
    call near_ret_plain
t1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_MEMW near_cap_ip1, t1_after
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 2) Near RET 4 — two word args discarded, SP back to SP0
t2:
    PREP
    push word 0xAAAA
    push word 0xBBBB
    mov ah, [pat_zf0]
    sahf
    call near_ret_imm4
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_MEMW near_cap_ip2, t2_after
    ASSERT_MEMW near_cap_arg_top, 0xBBBB
    ASSERT_MEMW near_cap_arg_next, 0xAAAA
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; 3) Near RET with odd SP
t3:
    PREP_ODD
    mov ah, [pat_all1]
    sahf
    call near_ret_plain2
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_MEMW near_cap_ip3, t3_after
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 4) Nested near returns (outer calls inner)
t4:
    PREP
    mov ah, [pat_zf0]
    sahf
    call near_outer
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    ASSERT_MEMW near_cap_ip_outer, t4_after
    ; inner’s return target is instruction after the CALL inside near_outer
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; 5) RET imm8 (8 bytes = 4 words) — four args discarded
t5:
    PREP
    push word 1
    push word 2
    push word 3
    push word 4
    mov ah, [pat_all1]
    sahf
    call near_ret_imm8
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_MEMW near_cap_ip4, t5_after
    ASSERT_MEMW near_cap_a1, 4
    ASSERT_MEMW near_cap_a2, 3
    ASSERT_MEMW near_cap_a3, 2
    ASSERT_MEMW near_cap_a4, 1
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== Prepare far callees at CS+1 =====================
prep_far:
    mov ax, cs
    inc ax
    mov es, ax                  ; ES = CS+1

    ; copy far_plain
    mov si, far_plain_start
    mov di, FAR_OFF1
    mov cx, far_plain_end - far_plain_start
    rep movsb

    ; copy far_imm4
    mov si, far_imm4_start
    mov di, FAR_OFF2
    mov cx, far_imm4_end - far_imm4_start
    rep movsb
    jmp t6

; ===================== Far RETF =====================

; 6) RETF (plain): far callee at CS+1, return IP/CS captured
far_ptr1: dw FAR_OFF1, 0
caller_cs: dw 0
exp_ip:    dw 0
t6:
    PREP
    mov [far_ptr1+2], es        ; seg = CS+1
    mov ax, cs
    mov [caller_cs], ax
    mov ax, t6_after
    mov [exp_ip], ax
    mov ah, [pat_all1]
    sahf
    call far [far_ptr1]
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
    ASSERT_MEMW far_cap_ip1, [exp_ip]
    ASSERT_MEMW far_cap_cs1, [caller_cs]
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    jmp t7

; 7) RETF 4: two args discarded in far callee
far_ptr2: dw FAR_OFF2, 0
t7:
    PREP
    mov [far_ptr2+2], es        ; seg = CS+1
    mov ax, cs
    mov [caller_cs], ax
    mov ax, t7_after
    mov [exp_ip], ax
    ; params
    push word 0xDEAD
    push word 0xBEEF
    mov ah, [pat_zf0]
    sahf
    call far [far_ptr2]
t7_after:
    SAVE_FLAGS
    ASSERT_AX 0xF222
    ASSERT_MEMW far_cap_ip2, [exp_ip]
    ASSERT_MEMW far_cap_cs2, [caller_cs]
    ASSERT_MEMW far_cap_a1, 0xBEEF
    ASSERT_MEMW far_cap_a2, 0xDEAD
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    jmp t8

; 8) RETF (plain) with odd SP
far_ptr3: dw FAR_OFF1, 0
t8:
    PREP_ODD
    mov [far_ptr3+2], es
    mov ax, cs
    mov [caller_cs], ax
    mov ax, t8_after
    mov [exp_ip], ax
    mov ah, [pat_all1]
    sahf
    call far [far_ptr3]
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
    ASSERT_MEMW far_cap_ip1, [exp_ip]
    ASSERT_MEMW far_cap_cs1, [caller_cs]
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    jmp t9

; 9) Near→Far→RETf→back (nested path sanity)
;    Call a small near stub that immediately does a far call to CS+1: FAR_OFF1 and returns.
near_stub_far:
    ; ES needs to be set to the CS we want to far call
    ; This call (and the call it performs) will not set flags

    ; capture our own return IP (for curiosity)
    mov bx, sp
    mov ax, [ss:bx]
    mov [near_stub_ret_ip], ax
    ; call far callee (plain)
    mov ax, cs
    mov word [near_tmp_ptr], FAR_OFF1
    mov [near_tmp_ptr+2], es
    call far [near_tmp_ptr]
    mov ax, 0x7777
    ret
    jmp t9

near_tmp_ptr: dw 0,0

t9:
    PREP
    ; precompute CS+1 and set ES to it
    mov ax, cs
    inc ax
    mov es, ax
    mov ah, [pat_zf0]
    sahf
    call near_stub_far
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0x7777
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Done =====================
exit:
    RESTORE_DOS_STACK
    mov ax, 0x4C00
    int 0x21

; ---------- Data & captures ----------
flags_store: dw 0
orig_ss:     dw 0
orig_sp:     dw 0
sp0_store:   dw 0

; Near captures
near_cap_ip1:       dw 0
near_cap_ip2:       dw 0
near_cap_arg_top:   dw 0
near_cap_arg_next:  dw 0
near_cap_ip3:       dw 0
near_cap_ip_outer:  dw 0
near_cap_ip_inner:  dw 0
near_cap_ip4:       dw 0
near_cap_a1:        dw 0
near_cap_a2:        dw 0
near_cap_a3:        dw 0
near_cap_a4:        dw 0
near_stub_ret_ip:   dw 0

; Far captures
far_cap_ip1:        dw 0
far_cap_cs1:        dw 0
far_cap_ip2:        dw 0
far_cap_cs2:        dw 0
far_cap_a1:         dw 0
far_cap_a2:         dw 0

; Scratch stack (2 KB)
stack_buf:  times 2048 db 0xCC
stack_top   equ stack_buf + 2048

pat_all1: db 0xD5        ; SF=1 ZF=1 AF=1 PF=1 CF=1
pat_zf0:  db 0x95        ; SF=1 ZF=0 AF=1 PF=1 CF=1

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
