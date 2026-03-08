; pop.asm — Thorough tests for POP (r16 / r/m16 / Sreg) in 16-bit mode
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH  (single-flag checks via saved FLAGS)
;
; Notes:
;   - POP does not modify flags (CF/PF/AF/ZF/SF preserved).
;   - POP SP final SP == popped value (special case).
;   - POP SS performed safely by popping DS into SS (keeps stack in our scratch segment).
;   - Memory dest with [BP] uses SS by default; others use DS unless overridden.

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

; Scratch stack helpers (SS = DS, SP to our buffer)
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

; ===================== 1) POP AX (basic) =====================
t1:
    PREP
    push word 0x1234
    mov ah, [pat_all1]
    sahf
    pop ax
    SAVE_FLAGS
    ASSERT_AX 0x1234
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_DF 0

; ===================== 2) POP r16 variety (BX, CX, DX, SI, DI, BP) =====================
t2:
    PREP
    push word 0xBABA
    mov ah, [pat_zf0]
    sahf
    pop bx
    SAVE_FLAGS
    ASSERT_BX 0xBABA
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
t2b:
    PREP
    push word 0xCAFE
    mov ah, [pat_all1]
    sahf
    pop cx
    SAVE_FLAGS
    ASSERT_CX 0xCAFE
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
t2c:
    PREP
    push word 0x0F0E
    mov ah, [pat_zf0]
    sahf
    pop dx
    SAVE_FLAGS
    ASSERT_DX 0x0F0E
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
t2d:
    PREP
    push word 0x2468
    mov ah, [pat_all1]
    sahf
    pop si
    SAVE_FLAGS
    ASSERT_SI 0x2468
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
t2e:
    PREP
    push word 0x1357
    mov ah, [pat_zf0]
    sahf
    pop di
    SAVE_FLAGS
    ASSERT_DI 0x1357
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
t2f:
    PREP
    push word 0xABCD
    mov ah, [pat_all1]
    sahf
    pop bp
    SAVE_FLAGS
    ASSERT_BP 0xABCD
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 3) POP r/m16 → DS:mem (simple) =====================
t3:
    PREP
    push word 0xCAFE
    mov ah, [pat_zf0]
    sahf
    pop word [mem_ds_0]
    SAVE_FLAGS
    ASSERT_MEMW mem_ds_0, 0xCAFE
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 4) POP r/m16 with ES override =====================
t4:
    PREP
    push word 0x1BAD
    mov ah, [pat_all1]
    sahf
    pop word [es:mem_es_0]         ; write via ES: (ES==DS here)
    SAVE_FLAGS
    ASSERT_MEMW mem_es_0, 0x1BAD
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 5) POP r/m16 where addr uses [BP] (defaults to SS) =====================
; With SS=DS (scratch), this proves default-segment selection for [BP].
t5:
    PREP
    mov bp, mem_ss_0               ; BP points at our DS buffer; default seg for [BP] is SS
    push word 0xDEAD
    mov ah, [pat_zf0]
    sahf
    pop word [bp]
    SAVE_FLAGS
    ASSERT_MEMW mem_ss_0, 0xDEAD
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 6) LIFO order: push A,B,C then pop → C,B,A =====================
t6:
    PREP
    push word 0x1111
    push word 0x2222
    push word 0x3333
    mov ah, [pat_all1]
    sahf
    pop ax
    pop bx
    pop cx
    SAVE_FLAGS
    mov dx, bx
    ASSERT_AX 0x3333
    mov bx, dx
    ASSERT_BX 0x2222
    ASSERT_CX 0x1111
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 7) POP SP special-case =====================
; Final SP must equal the popped value (not SP0+2).
t7:
    PREP
    mov ax, [sp0_store]
    sub ax, 0x20                   ; newSP = SP0 - 0x20 (still inside our scratch stack)
    push ax                        ; put newSP on stack
    mov ah, [pat_zf0]
    sahf
    pop sp
    SAVE_FLAGS
    ASSERT_SP ax                   ; SP == popped value
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 8) POP SS (safe): pop DS into SS so SS remains our scratch seg =====================
t8:
    PREP
    mov ax, ds
    push ax                        ; value to become SS
    mov ah, [pat_all1]
    sahf
    pop ss                         ; SS := DS ; SP += 2
    SAVE_FLAGS
    ; SS equals DS?
    mov ax, ss
    mov bx, ds
    int 0x23
    ASSERT_SP [sp0_store]          ; net effect: SP back to SP0
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    ; (Hardware masks interrupts for the following instruction; irrelevant to this harness.)

; ===================== 9) POP DS (no change) =====================
; We only pop the current DS value to avoid breaking DS-based addressing.
t9:
    PREP
    mov ax, ds
    push ax
    mov ah, [pat_zf0]
    sahf
    pop ds
    SAVE_FLAGS
    mov ax, ds
    mov bx, [save_ds_dummy]        ; preload equal to current DS
    int 0x23
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 10) POP ES: change ES to DS via stack =====================
t10:
    PREP
    ; Make ES visibly different first
    xor ax, ax
    mov es, ax
    mov ax, ds
    push ax
    mov ah, [pat_all1]
    sahf
    pop es                         ; ES := DS
    SAVE_FLAGS
    mov ax, es
    mov bx, ds
    int 0x23
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 11) Unaligned SP source =====================
t11:
    mov sp, stack_top - 0x81       ; odd SP allowed
    mov [sp0_store], sp
    push word 0x0F0E               ; now SP odd-2
    mov ah, [pat_zf0]
    sahf
    pop dx                          ; read from odd address
    SAVE_FLAGS
    ASSERT_DX 0x0F0E
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 12) POP r/m16 → DS:odd address =====================
t12:
    PREP
    push word 0xBEEF
    mov ah, [pat_all1]
    sahf
    pop word [odd_dst+1]           ; unaligned write
    SAVE_FLAGS
    ASSERT_MEMW odd_dst+1, 0xBEEF
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 13) POP r/m16 with ES override → odd address =====================
t13:
    PREP
    push word 0xACE1
    mov ah, [pat_zf0]
    sahf
    pop word [es:odd_es_dst+1]
    SAVE_FLAGS
    ASSERT_MEMW odd_es_dst+1, 0xACE1
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 14) POP BP (register) =====================
t14:
    PREP
    push word 0x2468
    mov ah, [pat_all1]
    sahf
    pop bp
    SAVE_FLAGS
    ASSERT_BP 0x2468
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== 15) DF preserved across POP =====================
t15:
    PREP
    std
    push word 0x1357
    mov ah, [pat_zf0]
    sahf
    pop ax
    SAVE_FLAGS
    ASSERT_AX 0x1357
    ASSERT_SP [sp0_store]
    CHECK_DF 1
    cld
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== 16) POP to two memory slots (order check) =====================
; push A, push B, then pop → mem[+2]=A, mem[+0]=B
t16:
    PREP
    mov word [mem_arr+0], 0x7E7E
    mov word [mem_arr+2], 0x7E7E
    push word 0xAAAA
    push word 0xBBBB
    mov ah, [pat_all1]
    sahf
    pop word [mem_arr+0]           ; gets 0xBBBB
    pop word [mem_arr+2]           ; gets 0xAAAA
    SAVE_FLAGS
    ASSERT_MEMW mem_arr+0, 0xBBBB
    ASSERT_MEMW mem_arr+2, 0xAAAA
    ASSERT_SP [sp0_store]
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; ===================== Done =====================
done:
    RESTORE_DOS_STACK
    mov ax, 0x4C00
    int 0x21

; ---------------- Data / scratch ----------------
flags_store: dw 0
orig_ss:     dw 0
orig_sp:     dw 0
sp0_store:   dw 0
save_ds_dummy: dw 0              ; (optional holder if you want to preload DS)

mem_ds_0:    dw 0x7E7E
mem_es_0:    dw 0x7E7E
mem_ss_0:    dw 0x7E7E
odd_dst:     db 0xCC,0xCC,0xCC
odd_es_dst:  db 0xCC,0xCC,0xCC
mem_arr:     dw 0x0000,0x0000

; Scratch stack (2 KB)
stack_buf:   times 2048 db 0xCC
stack_top    equ stack_buf + 2048

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
