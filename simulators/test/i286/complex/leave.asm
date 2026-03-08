; leave.asm — Thorough tests for LEAVE (paired with ENTER)
; Harness:
;   int 0x23: assert AX == BX  (used for value/register equality, SP/BP checks)
;   int 0x22: assert AL == AH  (single-flag checks via saved FLAGS)

bits 16
org 0x100

; ---------- Macros ----------
%macro SAVE_FLAGS 0
    ; retain used stack address
    mov  di, sp
    ; set up throwaway stack address
    lea si, stack
    mov sp, si
    push ax
    pushf
    pop  ax
    mov  [flags_store], ax
    pop  ax
    ; restore stack pointer
    mov  sp, di
%endmacro

%macro ASSERT_EQ_AX_BX 0
    int 0x23
%endmacro

%macro ASSERT_AX 1
    mov bx, %1
    int 0x23
%endmacro

%macro ASSERT_SP_EQ 1
    mov ax, sp
    mov di, bx
    mov bx, %1
    int 0x23
    mov bx, di
%endmacro

%macro ASSERT_BP_EQ 1
    mov di, bx
    mov ax, bp
    mov bx, %1
    int 0x23
    mov bx, di
%endmacro

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

; ---------- Helpers ----------
%macro SET_SCRATCH_STACK 0
    cli
    mov [orig_ss], ss
    mov [orig_sp], sp
    mov ax, ds             ; DS == CS in .COM
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

; Prepare per-test baseline: set SP to a safe start, BP to a known anchor, seed BP-chain words.
%macro PREP_TEST 1
    ; %1 = how many BP-chain words to seed (at [BP-2], [BP-4], ...)
    mov sp, stack_top - 0x80     ; headroom for ENTER/LEAVE + INTs
    lea bp, [bp_anchor]          ; known original BP for the test
    mov [bp0_store], bp
    mov [sp0_store], sp
%assign __i 1
%rep %1
    mov word [bp - (__i*2)], 0xA000 + __i  ; distinct 0xA001, 0xA002, ...
%assign __i __i+1
%endrep
%endmacro

; ---------- Start ----------
start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; Standard .COM: DS = CS, ES = DS
    push cs
    pop  ds
    push ds
    pop  es

    ; install scratch stack
    SET_SCRATCH_STACK

; ===================== 1) ENTER 0,0 → LEAVE =====================
; ENTER: BP=SP0-2, SP=SP0-2
; LEAVE: SP ← BP (=SP0-2), POP BP → BP=oldBP, SP=SP0
t1:
    PREP_TEST 0
    mov ah, [pat_all1]
    sahf
    mov ax, 0xBEEF
    mov dx, ax

    enter 0,0
    leave

    ; AX unchanged
    mov bx, dx
    ASSERT_EQ_AX_BX

    ; Final BP = oldBP ; Final SP = SP0
    mov ax, [bp0_store]
    ASSERT_BP_EQ ax
    mov bx, [sp0_store]
    ASSERT_SP_EQ bx

    SAVE_FLAGS
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_DF 0

; ===================== 2) ENTER 8,0 → LEAVE =====================
; ENTER: BP=SP0-2, SP=SP0-10
; LEAVE: BP=oldBP, SP=SP0 (locals discarded)
t2:
    PREP_TEST 0
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xCAFE
    mov dx, ax

    enter 8,0
    leave
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX
    mov ax, [bp0_store]
    ASSERT_BP_EQ ax
    mov bx, [sp0_store]
    ASSERT_SP_EQ bx

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_DF 0

; ===================== 3) ENTER 0,1 → LEAVE =====================
; ENTER: BP=SP0-2, SP=SP0-4, [BP]=oldBP, [BP-2]=SP0
; LEAVE: BP=oldBP, SP=SP0
t3:
    PREP_TEST 0
    mov ah, [pat_all1]
    sahf
    mov ax, 0xA5A5
    mov dx, ax

    enter 0,1
    ; For curiosity: SP after LEAVE should equal the value stored at [BP-2] (SP0)
    mov bx, [bp-2]           ; capture SP0 as saved by ENTER
    leave
    SAVE_FLAGS

    mov ax, dx
    mov dx, ax               ; keep AX unchanged check form
    push bx
    mov bx, dx
    ASSERT_EQ_AX_BX
    pop bx

    ; BP/SP restored
    mov ax, [bp0_store]
    ASSERT_BP_EQ ax
    ASSERT_SP_EQ bx          ; bx still holds the captured SP0
    ; Also equals sp0_store
    mov bx, [sp0_store]
    ASSERT_SP_EQ bx

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_DF 0

; ===================== 4) ENTER 6,1 → LEAVE =====================
t4:
    PREP_TEST 0
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x5A5A
    mov dx, ax

    enter 6,1
    leave
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX
    mov ax, [bp0_store]
    ASSERT_BP_EQ ax
    mov bx, [sp0_store]
    ASSERT_SP_EQ bx

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_DF 0

; ===================== 5) ENTER 0,2 → LEAVE =====================
; ENTER 0,2 pushes: [BP]=oldBP, [BP-2]=[oldBP-2]=A001, [BP-4]=SP0
; LEAVE ignores the extra copies and restores BP/SP from [BP] and BP respectively.
t5:
    PREP_TEST 2                  ; seed A001, A002 (only A001 used by ENTER 0,2)
    mov ah, [pat_all1]
    sahf
    mov ax, 0x0F0F
    mov dx, ax

    enter 0,2
    leave
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX
    mov ax, [bp0_store]
    ASSERT_BP_EQ ax
    mov bx, [sp0_store]
    ASSERT_SP_EQ bx

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_DF 0

; ===================== 6) ENTER 4,2 → LEAVE =====================
t6:
    PREP_TEST 2
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x1357
    mov dx, ax

    enter 4,2
    leave
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX
    mov ax, [bp0_store]
    ASSERT_BP_EQ ax
    mov bx, [sp0_store]
    ASSERT_SP_EQ bx

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_DF 0

; ===================== 7) ENTER 16,5 → LEAVE =====================
; Deep chain; LEAVE still restores exactly to oldBP/SP0
t7:
    PREP_TEST 5                  ; seed A001..A005
    mov ah, [pat_all1]
    sahf
    mov ax, 0x1234
    mov dx, ax

    enter 16,5
    leave
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX
    mov ax, [bp0_store]
    ASSERT_BP_EQ ax
    mov bx, [sp0_store]
    ASSERT_SP_EQ bx

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_DF 0

; ===================== 8) DF = 1 preserved across LEAVE (DF unaffected) =====================
t8:
    PREP_TEST 1
    std
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xBADA
    mov dx, ax

    enter 4,1
    leave

    mov bx, dx
    ASSERT_EQ_AX_BX
    mov ax, [bp0_store]
    ASSERT_BP_EQ ax
    mov bx, [sp0_store]
    SAVE_FLAGS
    ASSERT_SP_EQ bx
    CHECK_DF 1                ; DF remained set
    cld

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 9) Raw LEAVE semantics without ENTER (manual frame) =====================
; Setup: choose an address M within our stack buffer, set [SS:M] = 0xBEEF, set BP=M, SP arbitrary.
; LEAVE must: SP ← BP (=M), POP BP → BP = 0xBEEF, SP = M+2. Memory contents unchanged.
t9:
    ; Choose a word-aligned slot near top of stack_buf
    mov bx, stack_top
    sub bx, 0x20
    and bl, 0xFE            ; align to even for clarity (not required)
    mov word [bx], 0xBEEF
    mov word [bx+2], 0xCCDD ; neighbor sentinel

    ; Arbitrary SP, then flags
    mov sp, stack_top - 0x60
    mov ah, [pat_all1]
    sahf

    ; Place BP=M
    mov bp, bx

    ; LEAVE
    leave
    SAVE_FLAGS

    ; BP should now be 0xBEEF; SP should be M+2
    mov ax, 0xBEEF
    ASSERT_BP_EQ ax
    mov ax, bx
    add ax, 2
    ASSERT_SP_EQ ax
    ; Memory unchanged
    mov ax, [bx]
    mov bx, 0xBEEF
    ASSERT_EQ_AX_BX

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 10) Raw LEAVE at an odd address (unaligned POP) =====================
; x86 permits unaligned word accesses; verify behavior is identical.
t10:
    mov bx, stack_top
    sub bx, 0x30
    or  bl, 1               ; make it odd
    mov word [bx], 0xACE1

    mov sp, stack_top - 0x70
    mov ah, [pat_zf0]
    sahf
    mov bp, bx

    leave

    mov ax, 0xACE1
    ASSERT_BP_EQ ax
    mov ax, bx
    SAVE_FLAGS
    add ax, 2
    ASSERT_SP_EQ ax

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Done: restore DOS stack, exit =====================
done:
    RESTORE_DOS_STACK
    mov ax, 0x4C00
    int 0x21


; ---------------- Data / scratch ----------------
flags_store: dw 0
orig_ss:     dw 0
orig_sp:     dw 0
sp0_store:   dw 0
bp0_store:   dw 0

; Big scratch stack (2KB) filled with 0xCC for visibility
stack_buf:   times 2048 db 0xCC
stack_top    equ stack_buf + 2048

; An anchor for BP during PREP_TEST
bp_anchor:   db 0

; SAHF patterns (OF unaffected; LEAVE doesn't touch flags anyway)
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
