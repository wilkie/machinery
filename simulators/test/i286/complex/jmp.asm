; jmp.asm — Thorough tests for JMP (near/far) in 16-bit mode
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
    jmp t1

; ---------- Near targets ----------
n_tgt1:  mov ax, 0x1111          ; generic target pattern: set AX and fall-through
         ; (no flag-modifying ALU)
         jmp n1_after

; ===================== 1) Near JMP short (rel8) forward =====================
t1:
    PREP
    mov ah, [pat_all1]
    sahf
    jmp short n_tgt1
n1_cont:
n1_after:
    SAVE_FLAGS
    ASSERT_AX 0x1111
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    jmp t2

n_tgt2:  mov ax, 0x2222
         jmp n2_after

; ===================== 2) Near JMP short (rel8) backward =====================
t2:
    PREP
    mov ah, [pat_zf0]
    sahf
    jmp short n_tgt2
n2_cont:
n2_after:
    SAVE_FLAGS
    ASSERT_AX 0x2222
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    jmp t3

n_tgt3:  mov ax, 0x3333
         jmp n3_after

; ===================== 3) Near JMP rel16 forward (force 'near') =====================
near_ptr_dummy: dw 0
t3:
    PREP
    mov ah, [pat_all1]
    sahf
    jmp near n_tgt3
n3_after:
    SAVE_FLAGS
    ASSERT_AX 0x3333
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    jmp t4

n_tgt4:  mov ax, 0x4444
         jmp n4_after

; ===================== 4) Near JMP rel16 backward =====================
t4:
    PREP
    mov ah, [pat_zf0]
    sahf
    jmp near n_tgt4
n4_after:
    SAVE_FLAGS
    ASSERT_AX 0x4444
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    jmp t5

n_tgt5:  mov ax, 0x5555
         jmp n5_after

; ===================== 5) Near JMP r/m16 (register-indirect: DI) =====================
t5:
    PREP
    mov ah, [pat_all1]
    sahf
    mov di, n_tgt5
    jmp di
n5_after:
    SAVE_FLAGS
    ASSERT_AX 0x5555
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    jmp t6

n_tgt6:  mov ax, 0x6666
         jmp n6_after

; ===================== 6) Near JMP m16 (DS:) =====================
near_ptr1: dw n_tgt6
t6:
    PREP
    mov ah, [pat_zf0]
    sahf
    jmp word [near_ptr1]
n6_after:
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    jmp t7

n_tgt7:  mov ax, 0x7777
         jmp n7_after

; ===================== 7) Near JMP m16 using [BP] (defaults to SS) =====================
near_ptr_ss: dw n_tgt7
t7:
    PREP
    mov bp, near_ptr_ss
    mov ah, [pat_all1]
    sahf
    jmp word [bp]
n7_after:
    SAVE_FLAGS
    ASSERT_AX 0x7777
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    jmp t8

n_tgt8:  mov ax, 0x8888
         jmp n8_after

; ===================== 8) Near JMP m16 with ES override =====================
near_ptr_es: dw n_tgt8
t8:
    PREP
    mov ah, [pat_zf0]
    sahf
    jmp word [es:near_ptr_es]    ; ES == DS here
n8_after:
    SAVE_FLAGS
    ASSERT_AX 0x8888
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    jmp t9

n_tgt9:  mov ax, 0x9999
         jmp n9_after

; ===================== 9) Near JMP via unaligned memory pointer (odd address) =====================
odd_ptr: db 0x00,0x00,0x00       ; make [odd_ptr+1] unaligned
t9:
    PREP
    mov word [odd_ptr+1], n_tgt9
    mov ah, [pat_all1]
    sahf
    jmp word [odd_ptr+1]
n9_after:
    SAVE_FLAGS
    ASSERT_AX 0x9999
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    jmp t10

; ===================== 10) Near JMP chaining: jmp -> (target jmp) -> final =====================
mid_tgt: jmp n_tgtA              ; extra jump inside target (still flag-neutral)
t10:
    PREP
    mov ah, [pat_zf0]
    sahf
    jmp mid_tgt
nA_after:
    SAVE_FLAGS
    ASSERT_AX 0xAAA0
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    jmp t11

n_tgtA:  mov ax, 0xAAA0
         jmp nA_after

n_tgt11: mov ax, 0x3333
         jmp n11_post

; ===================== 11) Near JMP does not affect DF =====================
t11:
    PREP
    std
    mov ah, [pat_all1]
    sahf
    jmp n_tgt11 ; bounce there and back again (AX= 0x3333)
n11_post:
    SAVE_FLAGS
    push ax
    CHECK_DF 1
    cld
    pop ax
    ASSERT_AX 0x3333
    ASSERT_SP_EQ [sp0_store]
    jmp prep_far

; ---------- Far stubs (relocated to CS+1) ----------
; Stub #1: set AX, capture CS in DS var, far-jump back via [back_ptr1]
far1_start:
    mov ax, cs
    mov [far_seen_cs1], ax
    mov ax, 0xF111
    jmp far [back_ptr1]
far1_end:

; Stub #2: same via another back pointer (for ES override / [BP] cases)
far2_start:
    mov ax, cs
    mov [far_seen_cs2], ax
    mov ax, 0xF222
    jmp far [back_ptr2]
far2_end:

FAR1_OFF  equ 0x0200
FAR2_OFF  equ 0x0300

; ===================== Prepare far stubs at CS+1 =====================
prep_far:
    mov ax, cs
    inc ax
    mov es, ax                  ; ES = CS+1

    ; copy far1
    mov si, far1_start
    mov di, FAR1_OFF
    mov cx, far1_end - far1_start
    rep movsb

    ; copy far2
    mov si, far2_start
    mov di, FAR2_OFF
    mov cx, far2_end - far2_start
    rep movsb
    jmp t12

; ===================== 12) Far JMP m16:16 to CS+1, return via far jmp back =====================
far_ptr_mem: dw FAR1_OFF, 0      ; seg to be filled with ES (CS+1)
back_ptr1:   dw 0, 0             ; filled with (t12_after, CS)
caller_cs:   dw 0
t12:
    PREP
    ; set far target seg = CS+1
    mov [far_ptr_mem+2], es
    ; fill back_ptr1 with our return (offset + CS)
    mov ax, t12_after
    mov [back_ptr1+0], ax
    mov ax, cs
    mov [back_ptr1+2], ax
    mov [caller_cs], ax

    mov ah, [pat_all1]
    sahf
    jmp far [far_ptr_mem]
t12_after:
    SAVE_FLAGS
    ; AX set by stub; stub recorded its own CS
    ASSERT_AX 0xF111
    ; Verify the stub ran in CS+1
    mov ax, [caller_cs]
    inc ax
    ASSERT_MEMW far_seen_cs1, ax
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    jmp t13

; ===================== 13) Far JMP m16:16 with ES override (pointer read via ES:) =====================
far_ptr_es:  dw FAR2_OFF, 0
back_ptr2:   dw 0, 0
t13:
    PREP
    ; Make ES back to DS so we can write pointer then restore ES=CS+1 for seg field
    push ds
    pop  es
    mov word [far_ptr_es+0], FAR2_OFF
    mov ax, cs
    inc ax
    mov [far_ptr_es+2], ax       ; seg = CS+1
    ; set back target
    mov ax, t13_after
    mov [back_ptr2+0], ax
    mov ax, cs
    mov [back_ptr2+2], ax
    mov [caller_cs], ax

    mov ah, [pat_zf0]
    sahf
    jmp far [es:far_ptr_es]
t13_after:
    SAVE_FLAGS
    ASSERT_AX 0xF222
    mov ax, [caller_cs]
    inc ax
    ASSERT_MEMW far_seen_cs2, ax
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    jmp t14

; ===================== 14) Far JMP m16:16 via [BP] (defaults to SS) =====================
far_ptr_ss:  dw FAR1_OFF, 0
t14:
    PREP
    ; ensure ES is CS+1
    mov ax, cs
    inc ax
    mov es, ax                  ; ES = CS+1
    mov [far_ptr_ss+2], es
    mov bp, far_ptr_ss
    ; set back pointer to t14_after
    mov ax, t14_after
    mov [back_ptr1+0], ax
    mov ax, cs
    mov [back_ptr1+2], ax
    mov [caller_cs], ax

    mov ah, [pat_all1]
    sahf
    jmp far [bp]
t14_after:
    SAVE_FLAGS
    ASSERT_AX 0xF111
    mov ax, [caller_cs]
    inc ax
    ASSERT_MEMW far_seen_cs1, ax
    ASSERT_SP_EQ [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
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

; Far CS observations from stubs (written while running in CS+1)
far_seen_cs1:  dw 0
far_seen_cs2:  dw 0

; Scratch stack (2 KB)
stack_buf:     times 2048 db 0xCC
stack_top      equ stack_buf + 2048

pat_all1: db 0xD5
pat_zf0:  db 0x95

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
