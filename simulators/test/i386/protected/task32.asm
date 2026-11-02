; task32.asm - task switches between 386 TSSs: CALL (nesting: the back
; link, NT, busy bits, CR0.TS, registers from the new TSS) and IRETD back
; (the outgoing task's EIP saved for later); JMP between tasks (no
; nesting; resuming where each left off); a task gate in the IDT; and a
; switch from a 386 task to a 286 one and back.
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
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x9000
    mov ax, 0x18
    ltr ax

    ; Test 1: CALL to task B.
    mov eax, 0xA1A1A1A1
    mov esi, 0xA5A5A5A5
    call 0x20:0
    ; Back from B's IRETD: A's registers as A left them.
    CHECK32 esi, 0xA5A5A5A5
    mov eax, [b_eax]
    CHECK32 eax, 0xB0B0B0B0             ; B's EAX came from its TSS
    CHECK32 [b_nt], 0x4000              ; B ran nested
    CHECK32 [b_link], 0x18              ; B's back link names A
    CHECK32 [b_tr], 0x20
    CHECK32 [b_busy_a], 0x8B            ; A stayed busy during the CALL
    movzx eax, byte [gdt + 0x20 + 5]
    CHECK32 eax, 0x89                   ; B is free again after its IRET
    movzx eax, byte [gdt + 0x18 + 5]
    CHECK32 eax, 0x8B
    pushfd
    pop eax
    and eax, 0x4000
    CHECK32 eax, 0                      ; A is not nested
    smsw ax
    and eax, 8
    CHECK32 eax, 8                      ; the switch set TS
    clts
    mov eax, [tss_b + 0x20]
    CHECK32 eax, task_b.again           ; B's EIP saved after its IRETD

    ; Test 2: JMP to B, which resumes after its IRETD and JMPs back.
    jmp 0x20:0
.back:
    CHECK32 [b_marker], 2
    CHECK32 [b_nt2], 0                  ; reached by JMP: not nested
    movzx eax, byte [gdt + 0x20 + 5]
    CHECK32 eax, 0x89                   ; B left by JMP: free
    movzx eax, byte [gdt + 0x18 + 5]
    CHECK32 eax, 0x8B

    ; Test 3: INT 0x50 through a task gate to task D.
    int 0x50
    CHECK32 [d_marker], 0xD0D0D0D0
    CHECK32 [d_link], 0x18

    ; Test 4: CALL to a 286 task, which IRETs back.
    mov eax, 0x12345678
    call 0x30:0
    CHECK32 eax, 0x12345678
    mov ax, [c_ax]
    CHECK16 ax, 0xC0C0                  ; C's AX from its 286 TSS
    CHECK32 [c_tr], 0x30

    DONE

task_b:
    mov [b_eax], eax
    pushfd
    pop eax
    and eax, 0x4000
    mov [b_nt], eax
    movzx eax, word [tss_b]
    mov [b_link], eax
    str ax
    movzx eax, ax
    mov [b_tr], eax
    movzx eax, byte [gdt + 0x18 + 5]
    mov [b_busy_a], eax
    iretd
.again:
    mov dword [b_marker], 2
    pushfd
    pop eax
    and eax, 0x4000
    mov [b_nt2], eax
    jmp 0x18:0

task_d:
    mov dword [d_marker], 0xD0D0D0D0
    movzx eax, word [tss_d]
    mov [d_link], eax
    iretd

task_c:
    mov [c_ax], ax
    str ax
    movzx eax, ax
    mov [c_tr], eax
    iretd

; A 386 TSS: EIP, ESP, EAX; flat ring 0 segments.
%macro TSS32 3
    dd 0                        ; 00 back link
    dd 0, 0x10, 0, 0, 0, 0      ; 04 ESP0, SS0, ESP1, SS1, ESP2, SS2
    dd 0                        ; 1C CR3
    dd %1                       ; 20 EIP
    dd 0x00000002               ; 24 EFLAGS
    dd %3, 0, 0, 0              ; 28 EAX, ECX, EDX, EBX
    dd %2, 0, 0, 0              ; 38 ESP, EBP, ESI, EDI
    dd 0x10, 0x08, 0x10, 0x10, 0x10, 0x10   ; 48 ES, CS, SS, DS, FS, GS
    dd 0                        ; 60 LDT
    dw 0, 0x68                  ; 64 T, I/O map base
%endmacro

align 8
gdt:
    dq 0
    DESC 0, 0xFFFFF, CODE0, 0xC         ; 08: code
    DESC 0, 0xFFFFF, DATA0, 0xC         ; 10: data
    DESC ABS(tss_a), 0x67, TSS386, 0    ; 18: task A (this program)
    DESC ABS(tss_b), 0x67, TSS386, 0    ; 20: task B
    DESC ABS(tss_d), 0x67, TSS386, 0    ; 28: task D (interrupt task)
    DESC ABS(tss_c), 0x2B, 0x81, 0      ; 30: task C, a 286 TSS
gdt_end:

idt:
    times 0x50 dq 0
    GATE 0x28, 0, TASKG, 0              ; 50: task gate to D
idt_end:

gdt_ptr:
    dw gdt_end - gdt - 1
    dd ABS(gdt)
idt_ptr:
    dw idt_end - idt - 1
    dd ABS(idt)

align 4
tss_a: TSS32 0, 0, 0
tss_b: TSS32 task_b, 0x6000, 0xB0B0B0B0
tss_d: TSS32 task_d, 0x5000, 0
tss_c:
    dw 0                        ; 00 back link
    dw 0, 0x10, 0, 0, 0, 0      ; 02 SP0, SS0, SP1, SS1, SP2, SS2
    dw task_c                   ; 0E IP
    dw 0x0002                   ; 10 FLAGS
    dw 0xC0C0, 0, 0, 0          ; 12 AX, CX, DX, BX
    dw 0x4000, 0, 0, 0          ; 1A SP, BP, SI, DI
    dw 0x10, 0x08, 0x10, 0x10   ; 22 ES, CS, SS, DS
    dw 0                        ; 2A LDT

b_eax:    dd 0
b_nt:     dd 0xFFFFFFFF
b_link:   dd 0
b_tr:     dd 0
b_busy_a: dd 0
b_marker: dd 0
b_nt2:    dd 0xFFFFFFFF
d_marker: dd 0
d_link:   dd 0
c_ax:     dw 0
c_tr:     dd 0
