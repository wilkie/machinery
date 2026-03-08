; ret.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; set up stack address
    lea ax, stack
    mov sp, ax
    mov ax, 0x7654
    mov [stack - 2], ax
    mov [stack - 4], ax

    ; just a valid assert
    mov ax, 0x0
    mov bx, 0x0
    int 0x23

    ; RET near (jump over this failing assert)
    mov ax, after
    mov [stack], ax
    mov ax, 0xabcd
    mov bx, 0xffff
    ret
never:
    int 0x23

before:
    mov bx, 0xabcd
    int 0x23

    ; investigate the stack conditions
    ; the stack pointer should have advanced
    mov ax, stack + 2
    mov bx, sp
    int 0x23
    
    jmp next

after:
    ; reset stack address
    lea cx, stack
    mov sp, cx
    mov cx, 0x7654
    mov [stack - 2], cx
    mov [stack - 4], cx

    ; RET near (backward)
    mov cx, before
    mov [stack], cx
    ret

next:
    ; reset stack address
    lea cx, stack
    mov sp, cx
    mov cx, 0x7654
    mov [stack + 2], cx
    mov [stack], cx
    mov [stack - 2], cx
    mov [stack - 4], cx

    ; RET near dw
    mov ax, succeed
    mov [stack], ax
    ret 0x4

    ; ... a pit of jmps to avoid!
%rep 256
    jmp fail
%endrep

fail:
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

succeed:
    ; investigate the stack conditions
    ; the stack pointer should have advanced (2 normally, and 4 from the imm value)
    mov ax, stack + 2 + 0x4
    mov bx, sp
    int 0x23

    ; reset stack address
    lea ax, stack
    mov sp, ax
    mov ax, 0x7654
    mov [stack + 2], ax
    mov [stack], ax
    mov [stack - 2], ax
    mov [stack - 4], ax

    ; RET FAR
    ; self-modify the ret to go to the right place
    lea ax, far_exit_1
    mov word [stack], ax
    mov ax, cs
    mov word [stack + 2], ax
ret_far:
    retf
ret_far_fail:
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

    ; ... a pit of jmps to avoid!
%rep 256
    jmp fail
%endrep

far_exit_1:
    ; check stack conditions
    ; the stack pointer should have advanced (2 for ip and 2 for cs)
    mov ax, stack + 4
    mov bx, sp
    int 0x23

    ; reset stack address
    lea ax, stack
    mov sp, ax
    mov ax, 0x7654
    mov [stack + 2], ax
    mov [stack], ax
    mov [stack - 2], ax
    mov [stack - 4], ax

    ; RET FAR dw (mod 0x0, constant DISP)
    ; self-modify the ret to go to the right place
    lea ax, far_exit_2
    mov word [stack], ax
    mov ax, cs
    mov word [stack + 2], ax
    retf 0x8
ret_far_fail_2:
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

    ; ... a pit of jmps to avoid!
%rep 256
    jmp fail
%endrep

far_exit_2:

    ; check stack conditions
    ; the stack pointer should have advanced (2 for ip and 2 for cs, and then 0x8 for the imm value)
    mov ax, stack + 12
    mov bx, sp
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
ptr_ip_cs: dw 0x1234, 0x4321
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
db 0x0, 0x0, 0x0, 0x0
