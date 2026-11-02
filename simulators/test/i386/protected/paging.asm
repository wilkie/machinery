; paging.asm - two-level paging: an aliased page, accessed and dirty bits,
; #PF for a page not present (CR2, error code) on a read and a write, a
; dword straddling two pages mapped to separate frames, a straddling write
; whose second page is missing (nothing written), the supervisor writing
; a read-only page (allowed on the 386), and from ring 3: a supervisor
; page and a write to a read-only page (#PF with U/S, P set).
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

    ; Identity-map the first 4 MB (user, writable), then the exceptions.
    mov edi, PTAB
    mov eax, 0x007
    mov ecx, 1024
.fill:
    mov [edi], eax
    add eax, 0x1000
    add edi, 4
    loop .fill
    mov dword [PTAB + 0x30 * 4], 0x50000 | 7    ; 30000 -> 50000
    mov dword [PTAB + 0x31 * 4], 0x31000        ; not present
    mov dword [PTAB + 0x32 * 4], 0x32000 | 5    ; user, read-only
    mov dword [PTAB + 0x33 * 4], 0x33000 | 3    ; supervisor only
    mov dword [PTAB + 0x34 * 4], 0x60000 | 7    ; 34000 -> 60000
    mov dword [PTAB + 0x35 * 4], 0x40000 | 7    ; 35000 -> 40000
    mov dword [PDIR], PTAB | 7
    mov eax, PDIR
    mov cr3, eax
    mov eax, cr0
    or eax, 0x80000000
    mov cr0, eax
    jmp .paged
.paged:
    mov eax, cr3
    CHECK32 eax, PDIR

    ; Test 1: an aliased page.
    mov dword [0x30010], 0xABCD1234
    CHECK32 [0x50010], 0xABCD1234

    ; Test 2: accessed and dirty bits.
    mov eax, [PTAB + 0x30 * 4]
    and eax, 0x60
    CHECK32 eax, 0x60                   ; written: accessed and dirty
    mov eax, [0x36000]
    mov eax, [PTAB + 0x36 * 4]
    and eax, 0x60
    CHECK32 eax, 0x20                   ; read: accessed only
    mov eax, [PDIR]
    and eax, 0x20
    CHECK32 eax, 0x20

    ; Test 3: a page not present, read then write.
    mov dword [resume], .t3a
    mov eax, [0x31004]
.t3a:
    CHECK32 [pf_count], 1
    CHECK32 [pf_err], 0
    CHECK32 [pf_cr2], 0x31004
    mov dword [resume], .t3b
    mov dword [0x31008], 1
.t3b:
    CHECK32 [pf_count], 2
    CHECK32 [pf_err], 2
    CHECK32 [pf_cr2], 0x31008

    ; Test 4: a dword across two pages in separate frames.
    mov dword [0x34FFE], 0x11223344
    mov ax, [0x60FFE]
    CHECK16 ax, 0x3344
    mov ax, [0x40000]
    CHECK16 ax, 0x1122
    CHECK32 [0x34FFE], 0x11223344

    ; Test 5: a straddling write whose second page is missing faults on
    ; that page and writes nothing.
    mov dword [0x50FFC], 0
    mov dword [resume], .t5
    mov dword [0x30FFE], 0xFFFFFFFF
.t5:
    CHECK32 [pf_count], 3
    CHECK32 [pf_err], 2
    CHECK32 [pf_cr2], 0x31000
    CHECK32 [0x50FFC], 0

    ; Test 6: the supervisor may write a read-only page on the 386.
    mov dword [0x32000], 0x5EC0
    CHECK32 [0x32000], 0x5EC0

    ; To ring 3.
    push dword 0x23
    push dword 0x7000
    pushfd
    push dword 0x1B
    push dword ring3
    iretd

ring3:
    mov ax, 0x23
    mov ds, ax
    mov es, ax

    ; Test 7: ring 3 reads a supervisor page: #PF, P | U/S.
    mov dword [resume], .t7
    mov eax, [0x33000]
.t7:
    CHECK32 [pf_count], 4
    CHECK32 [pf_err], 5
    CHECK32 [pf_cr2], 0x33000

    ; Test 8: ring 3 reads a read-only page, but may not write it.
    CHECK32 [0x32000], 0x5EC0
    mov dword [resume], .t8
    mov dword [0x32004], 1
.t8:
    CHECK32 [pf_count], 5
    CHECK32 [pf_err], 7
    CHECK32 [pf_cr2], 0x32004

    DONE

; #PF: records the error code, CR2 and the count; resumes at [resume].
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
pf_err:   dd 0xFFFFFFFF
pf_cr2:   dd 0
pf_count: dd 0
