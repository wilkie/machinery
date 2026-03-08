; jmp.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; just a valid assert
    mov ax, 0x0
    mov bx, 0x0
    int 0x23

    ; JMP cb (jump over this failing assert)
    mov ax, 0xabcd
    mov bx, 0xffff
    jmp after
    int 0x23

before:
    mov bx, 0xabcd
    int 0x23
    jmp next

after:
    ; JMP cb (negative)
    jmp before

next:

    ; JMP cw
    jmp succeed

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

    ; JMP FAR cd
    ; self-modify the jmp to go to the right place
    lea ax, far_exit_1
    mov word [jmp_cd + 1], ax
    mov ax, cs
    mov word [jmp_cd + 3], ax
jmp_cd:
    jmp 0x1234:0x4321
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

    ; ... a pit of jmps to avoid!
%rep 256
    jmp fail
%endrep

far_exit_1:

    ; JMP FAR ed (mod 0x0, constant DISP)
    lea ax, far_exit_2
    mov word [ptr_ip_cs], ax
    mov ax, cs
    mov word [ptr_ip_cs + 2], ax
    jmp far [ptr_ip_cs]
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

    ; ... a pit of jmps to avoid!
%rep 256
    jmp fail
%endrep

far_exit_2:

    ; JMP FAR ed (mod 0x1, constant signed DISP)
    lea ax, far_exit_3
    mov word [ptr_ip_cs], ax
    mov ax, cs
    mov word [ptr_ip_cs + 2], ax
    lea bp, ptr_ip_cs
    mov si, 9
    jmp far [bp + si - 9]
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

    ; ... a pit of jmps to avoid!
%rep 256
    jmp fail
%endrep

far_exit_3:

    ; JMP NEAR ew (mod 0x3, constant DISP)
    lea ax, near_exit_1
    jmp ax
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

near_exit_1:

    ; JMP NEAR ew (mod 0x1, constant signed DISP, BP/SI form)
    lea ax, near_exit_2
    mov word [ptr_ip_cs], ax
    lea bp, ptr_ip_cs
    mov si, 9
    jmp near [bp + si - 9]
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

near_exit_2:

    ; JMP NEAR ew (mod 0x1, constant signed DISP, BX/DI form)
    lea ax, near_exit_3
    mov word [ptr_ip_cs], ax
    lea bx, ptr_ip_cs
    mov di, 9
    jmp near [bx + di - 9]
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

near_exit_3:

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
ptr_ip_cs: dw 0x1234, 0x4321
