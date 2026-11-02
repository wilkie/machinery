; gates32.asm - privilege transitions through 386 and 286 gates: IRETD to
; ring 3 (which nulls a DPL 0 DS), a 386 call gate copying dword
; parameters onto the ring 0 stack from the 386 TSS, a 286 call gate
; pushing a 16-bit frame, a 386 interrupt gate from ring 3 (stack switch,
; frame layout, IF cleared), #GP for CLI at IOPL 0 and for INT through a
; DPL 0 gate, and RETF to ring 3 nulling DS but not a DPL 3 ES.
bits 16
org 0x100
%include "common.inc"

start:
    cli
    lgdt [gdt_ptr]
    lidt [idt_ptr]
    mov eax, cr0
    or al, 1
    mov cr0, eax
    jmp dword 0x08:pm32

bits 32
pm32:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov esp, 0x9000
    mov ax, 0x28
    ltr ax

    ; Test 1: IRETD to ring 3. DS (DPL 0) is nulled; IF set from the frame.
    push dword 0x23                     ; SS
    push dword 0x7000                   ; ESP
    pushfd
    or dword [esp], 0x200               ; IF
    push dword 0x1B                     ; CS
    push dword ring3
    iretd

ring3:
    mov ax, cs
    and eax, 3
    CHECK32 eax, 3
    xor eax, eax
    mov ax, ds
    CHECK32 eax, 0
    CHECK32 esp, 0x7000
    mov ax, 0x23
    mov ds, ax
    mov es, ax

    ; Test 2: a 386 call gate with two dword parameters.
    push dword 0x11111111
    push dword 0x22222222
    call 0x33:0
.ret1:
    CHECK32 esp, 0x7000                 ; RETF 8 released the parameters
    ; RETF to ring 3 nulled DS (the procedure loaded DPL 0 data) but kept ES.
    xor eax, eax
    mov ax, ds
    CHECK32 eax, 0
    mov ax, es
    CHECK32 eax, 0x23
    mov ax, 0x23
    mov ds, ax
    CHECK32 [r0_cpl], 0
    CHECK32 [r0_esp], 0x7FE8            ; 8000 - SS, ESP, 2 params, CS, EIP
    CHECK32 [r0_frame + 0], .ret1
    CHECK32 [r0_frame + 4], 0x1B
    CHECK32 [r0_frame + 8], 0x22222222
    CHECK32 [r0_frame + 12], 0x11111111
    CHECK32 [r0_frame + 16], 0x6FF8
    CHECK32 [r0_frame + 20], 0x23

    ; Test 3: INT through a DPL 3 386 interrupt gate from ring 3.
    int 0x40
.ret2:
    CHECK32 [i_esp], 0x7FEC             ; 8000 - SS, ESP, EFLAGS, CS, EIP
    CHECK32 [i_frame + 0], .ret2
    CHECK32 [i_frame + 4], 0x1B
    mov eax, [i_frame + 8]
    and eax, 0x200
    CHECK32 eax, 0x200                  ; the pushed EFLAGS has IF
    CHECK32 [i_frame + 12], 0x7000
    CHECK32 [i_frame + 16], 0x23
    CHECK32 [i_if], 0                   ; cleared in the handler

    ; Test 4: CLI at CPL 3 with IOPL 0 is #GP(0).
    mov dword [resume], .t4
    cli
.t4:
    CHECK32 [gp_count], 1
    CHECK32 [gp_err], 0

    ; Test 5: INT through a DPL 0 gate from ring 3: #GP(vector * 8 + 2).
    mov dword [resume], .t5
    int 0x41
.t5:
    CHECK32 [gp_count], 2
    CHECK32 [gp_err], 0x41 * 8 + 2

    ; Test 6: a 286 call gate: a 16-bit frame, one word parameter.
    push word 0x5555
    call 0x43:0
.ret3:
    CHECK32 esp, 0x7000
    CHECK32 [w_esp], 0x7FF6             ; 8000 - SS, SP, param, CS, IP
    mov ax, [w_frame + 0]
    CHECK16 ax, .ret3
    mov ax, [w_frame + 2]
    CHECK16 ax, 0x1B
    mov ax, [w_frame + 4]
    CHECK16 ax, 0x5555
    mov ax, [w_frame + 6]
    CHECK16 ax, 0x6FFE
    mov ax, [w_frame + 8]
    CHECK16 ax, 0x23

    DONE

; Ring 0 procedure through the 386 call gate.
r0_proc:
    mov ax, cs
    and eax, 3
    mov [ss:r0_cpl], eax
    mov [ss:r0_esp], esp
    mov eax, [esp + 0]
    mov [ss:r0_frame + 0], eax
    mov eax, [esp + 4]
    mov [ss:r0_frame + 4], eax
    mov eax, [esp + 8]
    mov [ss:r0_frame + 8], eax
    mov eax, [esp + 12]
    mov [ss:r0_frame + 12], eax
    mov eax, [esp + 16]
    mov [ss:r0_frame + 16], eax
    mov eax, [esp + 20]
    mov [ss:r0_frame + 20], eax
    mov ax, 0x10                        ; DPL 0 data: nulled on the way out
    mov ds, ax
    retf 8

; Ring 0 procedure through the 286 call gate.
r0_proc16:
    mov [ss:w_esp], esp
    mov ax, [esp + 0]
    mov [ss:w_frame + 0], ax
    mov ax, [esp + 2]
    mov [ss:w_frame + 2], ax
    mov ax, [esp + 4]
    mov [ss:w_frame + 4], ax
    mov ax, [esp + 6]
    mov [ss:w_frame + 6], ax
    mov ax, [esp + 8]
    mov [ss:w_frame + 8], ax
    o16 retf 2

; INT 0x40 handler.
int40:
    mov [ss:i_esp], esp
    push eax
    mov eax, [esp + 4]
    mov [ss:i_frame + 0], eax
    mov eax, [esp + 8]
    mov [ss:i_frame + 4], eax
    mov eax, [esp + 12]
    mov [ss:i_frame + 8], eax
    mov eax, [esp + 16]
    mov [ss:i_frame + 12], eax
    mov eax, [esp + 20]
    mov [ss:i_frame + 16], eax
    pushfd
    pop eax
    and eax, 0x200
    mov [ss:i_if], eax
    pop eax
    iretd

; #GP: records the error code and count, resumes at [resume].
gp_handler:
    pop dword [ss:gp_err]
    inc dword [ss:gp_count]
    push eax
    mov eax, [ss:resume]
    mov [esp + 4], eax
    pop eax
    iretd

align 8
gdt:
    dq 0
    DESC 0, 0xFFFFF, CODE0, 0xC         ; 08: ring 0 code
    DESC 0, 0xFFFFF, DATA0, 0xC         ; 10: ring 0 data
    DESC 0, 0xFFFFF, CODE3, 0xC         ; 18: ring 3 code
    DESC 0, 0xFFFFF, DATA3, 0xC         ; 20: ring 3 data
    DESC ABS(tss), 0x67, TSS386, 0      ; 28: the 386 TSS
    GATE 0x08, ABS(r0_proc), 0xEC, 2    ; 30: 386 call gate, DPL 3, 2 dwords
    dq 0                                ; 38
    GATE 0x08, ABS(r0_proc16), 0xE4, 1  ; 40: 286 call gate, DPL 3, 1 word
gdt_end:

idt:
    times 13 dq 0
    GATE 0x08, ABS(gp_handler), INT386, 0   ; 0D
    times (0x40 - 14) dq 0
    GATE 0x08, ABS(int40), 0xEE, 0          ; 40: DPL 3 interrupt gate
    GATE 0x08, ABS(int40), INT386, 0        ; 41: DPL 0
idt_end:

gdt_ptr:
    dw gdt_end - gdt - 1
    dd ABS(gdt)
idt_ptr:
    dw idt_end - idt - 1
    dd ABS(idt)

align 4
tss:
    dd 0                ; back link
    dd 0x8000           ; ESP0
    dd 0x10             ; SS0
    times 22 dd 0
    dw 0, 0x68          ; T, I/O map base (none)

resume:   dd 0
gp_err:   dd 0xFFFFFFFF
gp_count: dd 0
r0_cpl:   dd 0xFFFFFFFF
r0_esp:   dd 0
r0_frame: times 6 dd 0
w_esp:    dd 0
w_frame:  times 5 dw 0
i_esp:    dd 0
i_frame:  times 5 dd 0
i_if:     dd 0xFFFFFFFF
