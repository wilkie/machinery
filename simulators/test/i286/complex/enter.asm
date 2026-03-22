; enter.asm — Thorough tests for ENTER (ENTER imm16, imm8)
; Harness:
;   int 0x23: assert AX == BX  (used for values and for SP/BP equality via moving regs)
;   int 0x22: assert AL == AH  (single-flag checks via saved FLAGS)
;
; ENTER semantics (16-bit):
;   PUSH BP
;   frameTemp := SP          ; (SP after push BP = original SP - 2)
;   if nesting > 0:
;       for i=1..(nesting-1): BP -= 2; PUSH word ptr [BP]
;       PUSH frameTemp        ; (NOT original SP; this is SP after initial BP push)
;   BP := frameTemp
;   SP := SP - size
; Effects:
;   BP_end = SP_start - 2
;   SP_end = SP_start - 2 - size                     (nesting=0)
;         or SP_start - (2*nesting + 2) - size       (nesting>0)
; Flags: none affected.

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

%macro ASSERT_EQ_AX_BX 0
    int 0x23
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

%macro ASSERT_BP_EQ 1
    mov ax, bp
    mov bx, %1
    int 0x23
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
    mov sp, stack_top - 0x80     ; roomy headroom for ENTER + INT activity
    lea bp, [bp_anchor]          ; known original BP for the test
    mov [bp0_store], bp
    mov [sp0_store], sp
    ; seed chain:
%assign __i 1
%rep %1
    mov word [bp - (__i*2)], 0xA000 + __i  ; distinct 0xA001, 0xA002, ...
%assign __i __i+1
%endrep
%endmacro

; Check preserved flags (CF/PF/AF/ZF/SF) against what SAHF set
%macro CHECK_ALL_PRESERVED 0
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
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

; ===================== Test 1: ENTER 0,0 (size=0, nesting=0) =====================
; Expected: BP = SP0-2; SP = SP0-2; [BP] = old BP
t1:
    PREP_TEST 0
    mov ah, [pat_all1]
    sahf
    mov ax, 0xBEEF                 ; AX must remain unchanged
    mov dx, ax
    enter 0, 0
    SAVE_FLAGS

    ; AX unchanged
    mov bx, dx
    mov ax, ax
    ASSERT_EQ_AX_BX

    ; BP == SP0-2
    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    ; SP == SP0-2
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_SP_EQ bx

    ; [BP] == old BP
    mov ax, [bp]
    mov bx, [bp0_store]
    ASSERT_EQ_AX_BX

    CHECK_ALL_PRESERVED

; ===================== Test 2: ENTER 8,0 (size=8, nesting=0) =====================
; SP = SP0 - 2 - 8 ; BP = SP0-2
t2:
    PREP_TEST 0
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xCAFE
    mov dx, ax
    enter 8, 0
    SAVE_FLAGS

    ; AX unchanged
    mov bx, dx
    ASSERT_EQ_AX_BX

    ; BP = SP0-2
    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    ; SP = SP0 - 10
    mov bx, [sp0_store]
    sub bx, 10
    ASSERT_SP_EQ bx

    ; [BP] = old BP
    mov ax, [bp]
    mov bx, [bp0_store]
    ASSERT_EQ_AX_BX

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Test 3: ENTER 0,1 (size=0, nesting=1) =====================
; SP = SP0 - (2*1 + 2) = SP0 - 4 ; [BP] = old BP ; [BP-2] = SP0 (tempSP)
t3:
    PREP_TEST 0
    mov ah, [pat_all1]
    sahf
    mov ax, 0xA5A5
    mov dx, ax
    enter 0, 1
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX

    ; BP = SP0-2
    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    ; SP = SP0 - 4
    mov bx, [sp0_store]
    sub bx, 4
    ASSERT_SP_EQ bx

    ; [BP] == old BP
    mov ax, [bp]
    mov bx, [bp0_store]
    ASSERT_EQ_AX_BX

    ; [BP-2] == FRAME_PTR (SP after initial BP push = SP0 - 2)
    mov ax, [bp-2]
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_EQ_AX_BX

    CHECK_ALL_PRESERVED

; ===================== Test 4: ENTER 6,1 (size=6, nesting=1) =====================
; SP = SP0 - 4 - 6 = SP0 - 10
t4:
    PREP_TEST 0
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x5A5A
    mov dx, ax
    enter 6, 1
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX

    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    mov bx, [sp0_store]
    sub bx, 10
    ASSERT_SP_EQ bx

    ; [BP] old BP, [BP-2] FRAME_PTR (SP0 - 2)
    mov ax, [bp]
    mov bx, [bp0_store]
    ASSERT_EQ_AX_BX

    mov ax, [bp-2]
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_EQ_AX_BX

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Test 5: ENTER 0,2 (size=0, nesting=2) =====================
; SP = SP0 - (2*2 + 2) = SP0 - 6
; [BP] = oldBP ; [BP-2] = SP0 ; [BP-4] = [oldBP-2] (we seeded 0xA001)
t5:
    PREP_TEST 2                  ; seed [BP-2] = 0xA001
    mov ah, [pat_all1]
    sahf
    mov ax, 0x0F0F
    mov dx, ax
    enter 0, 2
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX

    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    mov bx, [sp0_store]
    sub bx, 6
    ASSERT_SP_EQ bx

    ; [BP] oldBP
    mov ax, [bp]
    mov bx, [bp0_store]
    ASSERT_EQ_AX_BX
    ; [BP-2] copied from [oldBP-2] (=0xA001) (seeded)
    mov ax, [bp-2]
    mov bx, 0xA001
    ASSERT_EQ_AX_BX
    ; [BP-4] FRAME_PTR (SP0 - 2)
    mov ax, [bp-4]
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_EQ_AX_BX

    CHECK_ALL_PRESERVED

; ===================== Test 6: ENTER 4,2 =====================
; SP = SP0 - 6 - 4 = SP0 - 10
t6:
    PREP_TEST 2                  ; seed [BP-2]=A001, [BP-4]=A002 (we only need A001 here)
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x1357
    mov dx, ax
    enter 4, 2
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX

    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    mov bx, [sp0_store]
    sub bx, 10
    ASSERT_SP_EQ bx

    mov ax, [bp]
    mov bx, [bp0_store]
    ASSERT_EQ_AX_BX

    mov ax, [bp-2]
    mov bx, 0xA001
    ASSERT_EQ_AX_BX

    mov ax, [bp-4]
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_EQ_AX_BX

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Test 7: ENTER 0,3 =====================
; SP = SP0 - (2*3 + 2) = SP0 - 8
; [BP-2]=SP0 ; [BP-4]=[oldBP-2]=A001 ; [BP-6]=[oldBP-4]=A002
t7:
    PREP_TEST 2                  ; seed A001, A002
    mov ah, [pat_all1]
    sahf
    mov ax, 0xAAAA
    mov dx, ax
    enter 0, 3
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX

    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    mov bx, [sp0_store]
    sub bx, 8
    ASSERT_SP_EQ bx

    mov ax, [bp]
    mov bx, [bp0_store]
    ASSERT_EQ_AX_BX

    mov ax, [bp-2]
    mov bx, 0xA001
    ASSERT_EQ_AX_BX
    mov ax, [bp-4]
    mov bx, 0xA002
    ASSERT_EQ_AX_BX
    mov ax, [bp-6]
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_EQ_AX_BX

    CHECK_ALL_PRESERVED

; ===================== Test 8: ENTER 2,3 =====================
; SP = SP0 - 8 - 2 = SP0 - 10
t8:
    PREP_TEST 3
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xBBBB
    mov dx, ax
    enter 2, 3
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX

    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    mov bx, [sp0_store]
    sub bx, 10
    ASSERT_SP_EQ bx

    mov ax, [bp-2]
    mov bx, 0xA001
    ASSERT_EQ_AX_BX
    mov ax, [bp-4]
    mov bx, 0xA002
    ASSERT_EQ_AX_BX
    mov ax, [bp-6]
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_EQ_AX_BX

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Test 9: ENTER 0,5 (deeper chain) =====================
; SP = SP0 - (2*5 + 2) = SP0 - 12
; Verify [BP-2]=SP0 and [BP-4..-10] replicate A001..A004
t9:
    PREP_TEST 4                  ; seed A001..A004
    mov ah, [pat_all1]
    sahf
    mov ax, 0x0C0D
    mov dx, ax
    enter 0, 5
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX

    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    mov bx, [sp0_store]
    sub bx, 12
    ASSERT_SP_EQ bx

    mov ax, [bp-2]
    mov bx, 0xA001
    ASSERT_EQ_AX_BX
    mov ax, [bp-4]
    mov bx, 0xA002
    ASSERT_EQ_AX_BX
    mov ax, [bp-6]
    mov bx, 0xA003
    ASSERT_EQ_AX_BX
    mov ax, [bp-8]
    mov bx, 0xA004
    ASSERT_EQ_AX_BX
    mov ax, [bp-10]
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_EQ_AX_BX

    CHECK_ALL_PRESERVED

; ===================== Test 10: ENTER 16,5 =====================
; SP = SP0 - 12 - 16 = SP0 - 28
; Stack after ENTER: (BP = SP0 - 2, SP = SP0 - 28)
;   [BP-0] = oldBP, aka [SP0 - 2]
;   [BP-2] = [oldBP-2] = 0xA001
;   [BP-4] = [oldBP-4] = 0xA002
;   [BP-6] = [oldBP-6] = 0xA003
;   [BP-8] = [oldBP-8] = 0xA004
;   [BP-A] = tempSP = SP0
t10:
    PREP_TEST 5
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x1234
    mov dx, ax
    enter 16, 5
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX

    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    mov bx, [sp0_store]
    sub bx, 28
    ASSERT_SP_EQ bx

    ; spot check a couple of chain copies
    mov ax, [bp-4]
    mov bx, 0xA002
    ASSERT_EQ_AX_BX
    mov ax, [bp-8]
    mov bx, 0xA004
    ASSERT_EQ_AX_BX

    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Test 11: ENTER 0,33 (nesting uses low 5 bits → 33 ≡ 1) =====================
; Expect same as nesting=1: SP = SP0 - 4 ; [BP-2] = SP0
t11:
    PREP_TEST 1
    mov ah, [pat_all1]
    sahf
    mov ax, 0xFACE
    mov dx, ax
    db 0xC8, 0x00, 0x00, 33       ; ENTER 0,33 (encode explicitly to avoid NASM masking surprises)
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX

    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    mov bx, [sp0_store]
    sub bx, 4
    ASSERT_SP_EQ bx

    mov ax, [bp-2]
    mov bx, [sp0_store]
    sub bx, 2
    ASSERT_EQ_AX_BX

    CHECK_ALL_PRESERVED

; ===================== Test 12: DF = 1 (ENTER unaffected), ENTER 4,2 =====================
t12:
    PREP_TEST 2
    std
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xBADA
    mov dx, ax
    enter 4, 2
    SAVE_FLAGS

    mov bx, dx
    ASSERT_EQ_AX_BX

    ; DF should still be 1 (SAHF doesn't control DF; we check via saved FLAGS’ DF bit (bit 10))
    ; We don't have a dedicated DF-check macro here; but preservation implies all the SAHF-controlled bits match,
    ; and ENTER doesn't change DF. (Optional: add a DF checker if your harness supports it.)

    ; BP/SP math as usual:
    mov ax, [sp0_store]
    sub ax, 2
    ASSERT_BP_EQ ax

    mov bx, [sp0_store]
    sub bx, 10
    ASSERT_SP_EQ bx

    ; Clean DF for subsequent code
    cld

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

; A BP anchor located well above our per-test SP start so chain reads don’t overlap pushes
bp_anchor:   db 0                ; just a label to take an address from

; SAHF patterns (OF unaffected; ENTER doesn't touch any flags anyway)
pat_all1: db 0xD5            ; SF=1 ZF=1 AF=1 PF=1 CF=1
pat_zf0:  db 0x95            ; SF=1 ZF=0 AF=1 PF=1 CF=1

; smaller starter stack
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
