; call.asm — Thorough tests for CALL (near & far) in 16-bit mode
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH  (single-flag checks via saved FLAGS)
;
; Strategy:
; - Near callees store the caller's return IP (word at [SS:SP] on entry) to [near_ip_capN], then set AX to a signature and RET.
; - Far callee (relocated via copy (prep_far) to CS+1) stores both return IP and return CS, sets AX, then RETF.
; - All flag checks are done in the caller; callees avoid flag-modifying ALU ops.

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

; Flag checkers (preservation expected)
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
    mov sp, stack_top - 0x81
    mov [sp0_store], sp
%endmacro

start:
    jmp common_start

; ---------- Near callees (defined first so calls encode as backward rel16) ----------

near_callee1:
    ; Capture return IP and signal via AX
    mov bx, sp
    mov ax, [ss:bx]              ; return IP (word)
    mov [near_ip_cap1], ax
    mov ax, 0x1111
    ret

near_callee2:
    mov bx, sp
    mov ax, [ss:bx]
    mov [near_ip_cap2], ax
    mov ax, 0x2222
    ret

near_callee3:
    mov bx, sp
    mov ax, [ss:bx]
    mov [near_ip_cap3], ax
    mov ax, 0x3333
    ret

near_callee4:
    mov bx, sp
    mov ax, [ss:bx]
    mov [near_ip_cap4], ax
    mov ax, 0x4444
    ret

near_callee5:
    mov bx, sp
    mov ax, [ss:bx]
    mov [near_ip_cap5], ax
    mov ax, 0x5555
    ret

near_callee6:
    mov bx, sp
    mov ax, [ss:bx]
    mov [near_ip_cap6], ax
    mov ax, 0x6666
    ret

; ---------- Far callee template (to be relocated to CS+1:far_dst_off) ----------
far_callee_start:
    ; On entry (far CALL): stack holds return IP at [SS:SP], return CS at [SS:SP+2]
    mov bx, sp
    mov ax, [ss:bx]              ; return IP
    mov [far_ip_cap], ax
    mov ax, [ss:bx+2]            ; return CS
    mov [far_cs_cap], ax
    mov ax, 0xF001               ; visible effect
    retf
far_callee_end:

FAR_DST_OFF equ 0x0200           ; where we copy far callee in CS+1

; ---------- Start ----------
common_start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; Standard .COM: DS=CS, ES=DS
    push cs
    pop  ds
    push ds
    pop  es

    ; Use a safe scratch stack (SS=DS)
    SET_SCRATCH_STACK

; ===================== 1) Near CALL rel16 (backward), verify IP pushed =====================
t1:
    PREP
    mov ah, [pat_all1]
    sahf
    mov ax, 0xAAAA
    call near_callee1
t1_after:
    SAVE_FLAGS
    ; AX set by callee
    ASSERT_AX 0x1111
    ; Captured return IP equals offset of t1_after
    mov ax, [near_ip_cap1]
    mov bx, t1_after
    int 0x23
    ; SP restored
    ASSERT_SP_EQ [sp0_store]
    ; Flags preserved
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 2) Near CALL r16 (register-indirect: DI) =====================
t2:
    PREP
    mov ah, [pat_zf0]
    sahf
    mov di, near_callee2
    call di
t2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    mov ax, [near_ip_cap2]
    mov bx, t2_after
    int 0x23
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 3) Near CALL m16 (DS:) =====================
near_ptr1: dw near_callee3
t3:
    PREP
    mov ah, [pat_all1]
    sahf
    call word [near_ptr1]
t3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    mov ax, [near_ip_cap3]
    mov bx, t3_after
    int 0x23
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 4) Near CALL m16 using [BP] (defaults to SS) =====================
near_ptr_ss: dw near_callee4
t4:
    PREP
    mov bp, near_ptr_ss          ; [BP] uses SS (SS=DS here)
    mov ah, [pat_zf0]
    sahf
    call word [bp]
t4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    mov ax, [near_ip_cap4]
    mov bx, t4_after
    int 0x23
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 5) Near CALL m16 with ES override =====================
near_ptr_es: dw near_callee5
t5:
    PREP
    mov ah, [pat_all1]
    sahf
    call word [es:near_ptr_es]   ; ES==DS here
t5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    mov ax, [near_ip_cap5]
    mov bx, t5_after
    int 0x23
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 6) Near CALL with odd SP =====================
t6:
    PREP_ODD
    mov ah, [pat_zf0]
    sahf
    call near_callee6
t6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    mov ax, [near_ip_cap6]
    mov bx, t6_after
    int 0x23
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Prepare far callee relocated to CS+1 =====================
; Copy bytes [far_callee_start, far_callee_end) to ES:(CS+1):FAR_DST_OFF
prep_far:
    mov ax, cs
    inc ax
    mov es, ax                   ; ES = CS+1
    mov si, far_callee_start
    mov di, FAR_DST_OFF
    mov cx, far_callee_end - far_callee_start
    rep movsb
    jmp t7

; ===================== 7) Far CALL m16:16 (pointer in DS memory) to CS+1 =====================
far_ptr1:    dw FAR_DST_OFF, 0    ; to be filled with ES=CS+1
caller_cs:   dw 0
exp_ret_off: dw 0

t7:
    PREP
    ; Fill far pointer seg with ES (CS+1)
    mov [far_ptr1+2], es
    ; Record caller CS and expected return IP
    mov ax, cs
    mov [caller_cs], ax
    mov ax, t7_after
    mov [exp_ret_off], ax

    mov ah, [pat_all1]
    sahf
    call far [far_ptr1]
t7_after:
    SAVE_FLAGS

    ; AX set by far callee
    ASSERT_AX 0xF001
    ; Far callee captured return IP & CS
    ASSERT_MEMW far_ip_cap, [exp_ret_off]
    ASSERT_MEMW far_cs_cap, [caller_cs]
    ; SP restored
    ASSERT_SP_EQ [sp0_store]
    ; Flags preserved
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 8) Far CALL m16:16 from odd SP =====================
far_ptr2:    dw FAR_DST_OFF, 0
t8:
    PREP_ODD
    mov [far_ptr2+2], es         ; ES still CS+1
    mov ax, cs
    mov [caller_cs], ax
    mov ax, t8_after
    mov [exp_ret_off], ax

    mov ah, [pat_zf0]
    sahf
    call far [far_ptr2]
t8_after:
    SAVE_FLAGS
    ASSERT_AX 0xF001
    ASSERT_MEMW far_ip_cap, [exp_ret_off]
    ASSERT_MEMW far_cs_cap, [caller_cs]
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 9) Far CALL m16:16 using [BP] (defaults to SS) =====================
far_ptr_ss:  dw FAR_DST_OFF, 0
t9:
    PREP
    mov [far_ptr_ss+2], es
    mov bp, far_ptr_ss           ; [BP] → SS:
    mov ax, cs
    mov [caller_cs], ax
    mov ax, t9_after
    mov [exp_ret_off], ax

    mov ah, [pat_all1]
    sahf
    call far [bp]
t9_after:
    SAVE_FLAGS
    ASSERT_AX 0xF001
    ASSERT_MEMW far_ip_cap, [exp_ret_off]
    ASSERT_MEMW far_cs_cap, [caller_cs]
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 10) Far CALL m16:16 with ES override (pointer read via ES:) =====================
far_ptr_es:  dw FAR_DST_OFF, 0
t10:
    PREP
    ; Make ES point back to DS so ES: reads the pointer in our data segment
    push ds
    pop  es
    mov [far_ptr_es+2], cs       ; temporarily set seg wrong...
    ; then fix it to CS+1 using temp register:
    mov ax, cs
    inc ax
    mov [far_ptr_es+2], ax       ; seg = CS+1

    mov ax, cs
    mov [caller_cs], ax
    mov ax, t10_after
    mov [exp_ret_off], ax

    mov ah, [pat_zf0]
    sahf
    call far [es:far_ptr_es]
t10_after:
    SAVE_FLAGS
    ASSERT_AX 0xF001
    ASSERT_MEMW far_ip_cap, [exp_ret_off]
    ASSERT_MEMW far_cs_cap, [caller_cs]
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

; ---------------- Data / captures ----------------
flags_store:   dw 0
orig_ss:       dw 0
orig_sp:       dw 0
sp0_store:     dw 0

near_ip_cap1:  dw 0
near_ip_cap2:  dw 0
near_ip_cap3:  dw 0
near_ip_cap4:  dw 0
near_ip_cap5:  dw 0
near_ip_cap6:  dw 0

far_ip_cap:    dw 0
far_cs_cap:    dw 0

; Scratch stack (2 KB)
stack_buf:     times 2048 db 0xCC
stack_top      equ stack_buf + 2048

pat_all1: db 0xD5                ; SF=1 ZF=1 AF=1 PF=1 CF=1
pat_zf0:  db 0x95                ; SF=1 ZF=0 AF=1 PF=1 CF=1

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
