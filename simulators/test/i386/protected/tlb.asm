; tlb.asm - the translation lookaside buffer: a translation stays cached
; after its page table entry changes, until CR3 is reloaded; a write to a
; page first cached by a read still sets the entry's dirty bit (the 386
; walks again for it); a page that faulted as not present was not cached,
; so making it present takes effect without a flush. (Which entry a fill
; replaces is not documented and not tested.)
bits 16
org 0x100
%include "common.inc"

PDIR  equ 0x20000
PTAB  equ 0x21000

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

    ; Identity-map the first 4 MB, then 30000 -> 50000 and 31000 missing.
    mov edi, PTAB
    mov eax, 0x007
    mov ecx, 1024
.fill:
    mov [edi], eax
    add eax, 0x1000
    add edi, 4
    loop .fill
    mov dword [PTAB + 0x30 * 4], 0x50000 | 7
    mov dword [PTAB + 0x31 * 4], 0x31000
    mov dword [PDIR], PTAB | 7
    mov dword [0x50000], 0x5555     ; what 30000 shows first
    mov dword [0x60000], 0x6666     ; and after the remap
    mov dword [0x31000], 0x3131
    mov eax, PDIR
    mov cr3, eax
    mov eax, cr0
    or eax, 0x80000000
    mov cr0, eax
    jmp .paged
.paged:

    ; Test 1: remapped without a flush, the old translation stands; a CR3
    ; load flushes it.
    CHECK32 [0x30000], 0x5555
    mov dword [PTAB + 0x30 * 4], 0x60000 | 7
    CHECK32 [0x30000], 0x5555
    mov eax, cr3
    mov cr3, eax
    CHECK32 [0x30000], 0x6666

    ; Test 2: read first (accessed, not dirty), then written: dirty.
    mov eax, [0x36000]
    mov eax, [PTAB + 0x36 * 4]
    and eax, 0x60
    CHECK32 eax, 0x20
    mov dword [0x36000], 1
    mov eax, [PTAB + 0x36 * 4]
    and eax, 0x60
    CHECK32 eax, 0x60

    ; Test 3: a page that faulted as not present was not cached.
    mov dword [resume], .t3
    mov eax, [0x31000]
.t3:
    CHECK32 [pf_count], 1
    CHECK32 [pf_cr2], 0x31000
    mov dword [PTAB + 0x31 * 4], 0x31000 | 7
    CHECK32 [0x31000], 0x3131
    CHECK32 [pf_count], 1

    DONE

; #PF: records CR2 and the count; resumes at [resume].
pf_handler:
    add esp, 4
    inc dword [ss:pf_count]
    push eax
    mov eax, cr2
    mov [ss:pf_cr2], eax
    mov eax, [ss:resume]
    mov [esp + 4], eax
    pop eax
    iretd

align 8
gdt:
    dq 0
    DESC 0, 0xFFFFF, CODE0, 0xC         ; 08
    DESC 0, 0xFFFFF, DATA0, 0xC         ; 10
    DESC 0, 0xFFFFF, CODE3, 0xC         ; 18
    DESC 0, 0xFFFFF, DATA3, 0xC         ; 20
    DESC ABS(tss), 0x67, TSS386, 0      ; 28
gdt_end:

idt:
    times 14 dq 0
    GATE 0x08, ABS(pf_handler), INT386, 0   ; 0E
idt_end:

gdt_ptr:
    dw gdt_end - gdt - 1
    dd ABS(gdt)
idt_ptr:
    dw idt_end - idt - 1
    dd ABS(idt)

align 4
tss:
    dd 0, 0x8000, 0x10
    times 22 dd 0
    dw 0, 0x68

resume:   dd 0
pf_cr2:   dd 0
pf_count: dd 0
