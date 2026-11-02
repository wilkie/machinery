; pm32.asm - 32-bit protected mode: a flat 4 GB segment with page
; granularity, 32-bit code (default operand size), 32-bit and 16-bit
; stacks, LSL and LAR on granular segments, limit faults through a 386
; interrupt gate, a big expand-down segment, and a return to real mode
; that keeps a 4 GB limit in the segment caches ("unreal" mode).
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
    mov ss, ax
    mov esp, 0x9000

    ; Test 1: a 32-bit code segment pushes dwords by default...
    mov ebx, esp
    push eax
    sub ebx, esp
    CHECK32 ebx, 4
    add esp, 4

    ; ...and words with an operand-size prefix.
    mov ebx, esp
    push ax
    sub ebx, esp
    CHECK32 ebx, 2
    add esp, 2

    ; Test 2: memory above 1 MB through the flat segment.
    mov dword [0x200000], 0x12345678
    CHECK32 [0x200000], 0x12345678

    ; Test 3: LSL gives the limit in bytes (G = 1: 4 GB).
    mov ecx, 0x10
    lsl eax, ecx
    mov ebx, 0xFFFFFFFF
    int 0x24

    ; Test 4: LAR gives the access byte (now accessed) and G, D.
    lar eax, ecx
    mov ebx, 0x00C09300
    int 0x24

    ; Test 5: a byte-granular segment with limit FFF: offset 1000 is #GP(0).
    mov dword [resume], .t5
    mov ax, 0x20
    mov gs, ax
    mov eax, [gs:0x0FFC]                ; the last dword: fine
    mov eax, [gs:0x0FFD]                ; runs past the limit
.t5:
    CHECK32 [gp_count], 1
    CHECK32 [gp_err], 0

    ; Test 6: a big expand-down segment: offsets above the limit, up to
    ; 4 GB, are valid; at or below it is #GP.
    mov ax, 0x28
    mov gs, ax
    mov dword [gs:0x2000], 0xCAFEF00D
    CHECK32 [0x302000], 0xCAFEF00D      ; base 300000
    mov dword [resume], .t6
    mov eax, [gs:0x0800]
.t6:
    CHECK32 [gp_count], 2

    ; Test 7: the 32-bit stack crosses 64 KB without wrapping.
    mov esp, 0x10002
    push eax
    CHECK32 esp, 0x0000FFFE

    ; Test 8: a 16-bit stack (B = 0) moves only SP.
    mov ax, 0x30
    mov ss, ax
    mov esp, 0x00120004
    push eax
    mov ecx, esp
    CHECK32 ecx, 0x00120000
    push eax
    mov ecx, esp
    CHECK32 ecx, 0x0012FFFC
    mov ax, 0x10
    mov ss, ax
    mov esp, 0x9000

    ; Test 9: back to real mode through a 16-bit code segment. The
    ; caches keep their 4 GB limits.
    jmp 0x18:pm16

bits 16
pm16:
    mov eax, cr0
    and al, 0xFE
    mov cr0, eax
    jmp 0x0000:real

real:
    xor ax, ax
    mov ds, ax
    mov ss, ax
    mov esp, 0x8000
    mov esi, 0x200000
    CHECK32 [fs:esi], 0x12345678
    CHECK32 [esi], 0x12345678

    ; Real mode reads SMSW with PE clear.
    smsw ax
    CHECK16 ax, 0x0000

    DONE

bits 32
; A #GP handler: records the error code and the count, and resumes at
; [resume].
gp_handler:
    pop dword [gp_err]
    inc dword [gp_count]
    push eax
    mov eax, [resume]
    mov [esp + 4], eax
    pop eax
    iretd

align 8
gdt:
    dq 0
    DESC 0, 0xFFFFF, CODE0, 0xC         ; 08: 32-bit code, 4 GB
    DESC 0, 0xFFFFF, DATA0, 0xC         ; 10: flat data, 4 GB
    DESC 0, 0xFFFF, CODE0, 0x0          ; 18: 16-bit code
    DESC 0x100000, 0xFFF, DATA0, 0x0    ; 20: small data, byte limit FFF
    DESC 0x300000, 0xFFF, 0x96, 0x4     ; 28: big expand-down data
    DESC 0, 0xFFFF, DATA0, 0x0          ; 30: 16-bit stack
gdt_end:

idt:
    times 13 dq 0
    GATE 0x08, ABS(gp_handler), INT386, 0
idt_end:

gdt_ptr:
    dw gdt_end - gdt - 1
    dd ABS(gdt)
idt_ptr:
    dw idt_end - idt - 1
    dd ABS(idt)

resume:   dd 0
gp_err:   dd 0xFFFFFFFF
gp_count: dd 0
