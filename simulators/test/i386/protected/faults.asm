; faults.asm - the 386's double-fault classes with paging. A #GP whose
; IDT gate lies on a page not present raises #PF while it is delivered:
; a page fault during a contributory fault is handled serially (the #PF
; is delivered, CR2 naming the gate). A #PF whose own gate is on a page
; not present makes a double fault (#DF, error code 0).
bits 16
org 0x100
%include "common.inc"

PDIR  equ 0x20000
PTAB  equ 0x21000
IDT_A equ 0x32000 - 112                 ; gate 13 on page 31, gate 14 on 32
IDT_B equ 0x33000 - 112                 ; gate 8 on page 32, gate 14 on 33

start:
    cli
    lgdt [gdt_ptr]
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

    ; Gates: IDT A's 14 (on page 32), IDT B's 8 (on page 32).
    mov esi, pf_gate
    mov edi, IDT_A + 14 * 8
    movsd
    movsd
    mov esi, df_gate
    mov edi, IDT_B + 8 * 8
    movsd
    movsd

    ; Identity paging, with pages 31 and 33 not present.
    mov edi, PTAB
    mov eax, 0x003
    mov ecx, 1024
.fill:
    mov [edi], eax
    add eax, 0x1000
    add edi, 4
    loop .fill
    and dword [PTAB + 0x31 * 4], 0xFFFFFFFE
    and dword [PTAB + 0x33 * 4], 0xFFFFFFFE
    mov dword [PDIR], PTAB | 3
    mov eax, PDIR
    mov cr3, eax
    mov eax, cr0
    or eax, 0x80000000
    mov cr0, eax

    ; Test 1: #GP, its gate not present: the #PF is delivered instead.
    lidt [idt_a]
    mov dword [resume], .t1
    mov ax, 0x1234                      ; past the GDT's limit
    mov ds, ax
.t1:
    CHECK32 [pf_count], 1
    CHECK32 [pf_err], 0
    CHECK32 [pf_cr2], IDT_A + 13 * 8
    CHECK32 [df_count], 0

    ; Test 2: #PF, its own gate not present: a double fault.
    lidt [idt_b]
    mov dword [resume], .t2
    mov eax, [0x31000]
.t2:
    CHECK32 [df_count], 1
    CHECK32 [df_err], 0
    CHECK32 [pf_count], 1

    DONE

pf_handler:
    pop dword [ss:pf_err]
    inc dword [ss:pf_count]
    push eax
    mov eax, cr2
    mov [ss:pf_cr2], eax
    mov eax, [ss:resume]
    mov [esp + 4], eax
    pop eax
    iretd

df_handler:
    pop dword [ss:df_err]
    inc dword [ss:df_count]
    push eax
    mov eax, [ss:resume]
    mov [esp + 4], eax
    pop eax
    iretd

align 8
pf_gate: GATE 0x08, ABS(pf_handler), INT386, 0
df_gate: GATE 0x08, ABS(df_handler), INT386, 0

gdt:
    dq 0
    DESC 0, 0xFFFFF, CODE0, 0xC         ; 08
    DESC 0, 0xFFFFF, DATA0, 0xC         ; 10
gdt_end:

gdt_ptr:
    dw gdt_end - gdt - 1
    dd ABS(gdt)
idt_a:
    dw 15 * 8 - 1
    dd IDT_A
idt_b:
    dw 15 * 8 - 1
    dd IDT_B

resume:   dd 0
pf_err:   dd 0xFFFFFFFF
pf_cr2:   dd 0
pf_count: dd 0
df_err:   dd 0xFFFFFFFF
df_count: dd 0
