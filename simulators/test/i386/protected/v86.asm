; v86.asm - virtual-8086 mode: entered by IRETD with VM set; segments
; load as in real mode; INT n at IOPL 3 reaches a ring 0 handler through a
; 386 interrupt gate (the frame holds GS, FS, DS, ES, SS:ESP, EFLAGS with
; VM, CS:EIP; the data segments are null in the handler) and IRETD
; returns; I/O follows the TSS's permission bitmap (a port past the TSS's
; limit is denied); and at IOPL 0, CLI, PUSHF and INT n are #GP while I/O
; still follows the bitmap.
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
    mov ax, 0x18
    ltr ax

    ; Enter virtual-8086 mode at IOPL 3.
    push dword 0                        ; GS
    push dword 0                        ; FS
    push dword 0                        ; DS
    push dword 0                        ; ES
    push dword 0                        ; SS
    push dword 0x7000                   ; ESP
    push dword 0x00023002               ; EFLAGS: VM, IOPL 3
    push dword 0                        ; CS
    push dword v86
    iretd

bits 16
v86:
    ; Test 1: protected mode underneath (SMSW is allowed), flags readable.
    smsw ax
    and ax, 1
    CHECK16 ax, 1
    pushfd
    pop eax
    and eax, 0x3000
    CHECK32 eax, 0x3000

    ; Test 2: segments load as in real mode.
    mov ax, 0x1000
    mov es, ax
    mov word [es:0x10], 0x1234
    mov ax, [es:0x10]
    CHECK16 ax, 0x1234
    push ds
    mov ax, 0x1001
    mov ds, ax
    mov ax, [0]                         ; 10010 again, through DS
    pop ds
    CHECK16 ax, 0x1234

    ; Test 3: INT 0x30 at IOPL 3 to the ring 0 handler and back.
    int 0x30
.ret3:
    CHECK32 [h_frame + 0], .ret3
    CHECK32 [h_frame + 4], 0
    mov eax, [h_frame + 8]
    and eax, 0x00020000
    CHECK32 eax, 0x00020000             ; VM in the pushed EFLAGS
    CHECK32 [h_frame + 12], 0x7000
    CHECK32 [h_frame + 16], 0
    CHECK32 [h_frame + 20], 0x1000      ; ES
    CHECK32 [h_frame + 24], 0           ; DS
    CHECK32 [h_esp], 0x8000 - 36
    CHECK32 [h_ds], 0                   ; null in the handler
    CHECK32 [h_vm], 0                   ; and VM clear
    mov ax, es
    CHECK16 ax, 0x1000                  ; restored by IRETD

    ; Test 4: I/O through the bitmap: port 20 allowed, 40 denied, 90 past
    ; the end of the TSS.
    mov word [resume], .t4a
    in al, 0x20
.t4a:
    CHECK32 [gp_count], 0
    mov word [resume], .t4b
    in al, 0x40
.t4b:
    CHECK32 [gp_count], 1
    CHECK32 [gp_err], 0
    mov word [resume], .t4c
    mov dx, 0x90
    in al, dx
.t4c:
    CHECK32 [gp_count], 2

    ; Test 5: down to IOPL 0 (by the ring 0 handler of INT 0x31).
    int 0x31
    mov word [resume], .t5a
    cli
.t5a:
    CHECK32 [gp_count], 3
    mov word [resume], .t5b
    pushf
.t5b:
    CHECK32 [gp_count], 4
    mov word [resume], .t5c
    int 0x30
.t5c:
    CHECK32 [gp_count], 5
    CHECK32 [gp_err], 0
    mov word [resume], .t5d
    in al, 0x21                         ; still allowed by the bitmap
.t5d:
    CHECK32 [gp_count], 5

    DONE

bits 32
; INT 0x30: records the frame, ESP, DS and VM.
int30:
    mov [ss:h_esp], esp
    push eax
    mov eax, [esp + 4]
    mov [ss:h_frame + 0], eax
    mov eax, [esp + 8]
    mov [ss:h_frame + 4], eax
    mov eax, [esp + 12]
    mov [ss:h_frame + 8], eax
    mov eax, [esp + 16]
    mov [ss:h_frame + 12], eax
    mov eax, [esp + 20]
    mov [ss:h_frame + 16], eax
    mov eax, [esp + 24]
    mov [ss:h_frame + 20], eax
    mov eax, [esp + 28]
    mov [ss:h_frame + 24], eax
    xor eax, eax
    mov ax, ds
    mov [ss:h_ds], eax
    pushfd
    pop eax
    and eax, 0x00020000
    mov [ss:h_vm], eax
    pop eax
    iretd

; INT 0x31: lowers the virtual-8086 task's IOPL to 0.
int31:
    and dword [esp + 8], 0xFFFFCFFF
    iretd

; #GP: records the error code and count, resumes at [resume] (a 16-bit
; offset in the virtual-8086 code).
gp_handler:
    pop dword [ss:gp_err]
    inc dword [ss:gp_count]
    push eax
    movzx eax, word [ss:resume]
    mov [esp + 4], eax
    pop eax
    iretd

align 8
gdt:
    dq 0
    DESC 0, 0xFFFFF, CODE0, 0xC         ; 08
    DESC 0, 0xFFFFF, DATA0, 0xC         ; 10
    DESC ABS(tss), tss_end - tss - 1, TSS386, 0     ; 18
gdt_end:

idt:
    times 13 dq 0
    GATE 0x08, ABS(gp_handler), INT386, 0   ; 0D
    times (0x30 - 14) dq 0
    GATE 0x08, ABS(int30), 0xEE, 0          ; 30: DPL 3
    GATE 0x08, ABS(int31), 0xEE, 0          ; 31: DPL 3
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
    dw 0, iomap - tss
iomap:
    ; Ports 00-7F: all allowed but 40.
    db 0, 0, 0, 0, 0, 0, 0, 0
    db 0x01, 0, 0, 0, 0, 0, 0, 0
tss_end:
    db 0xFF

resume:   dw 0
gp_err:   dd 0xFFFFFFFF
gp_count: dd 0
h_frame:  times 7 dd 0
h_esp:    dd 0
h_ds:     dd 0xFFFFFFFF
h_vm:     dd 0xFFFFFFFF
