; call.asm
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

    ; CALL cw (jump over this failing assert)
    mov ax, 0xabcd
    mov bx, 0xffff
    call after
never:
    int 0x23

before:
    mov bx, 0xabcd
    int 0x23

    ; investigate the stack conditions
    ; the most recent item on the stack must refer also to 'next'
    mov ax, word [stack - 4]
    mov bx, next
    int 0x23

    ; the item before that refers to the instruction after the first call
    mov ax, word [stack - 2]
    mov bx, never
    int 0x23
    
    jmp next

after:
    ; CALL cw (negative)
    call before

next:

    ; CALL cw
    call succeed

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
    ; reset stack address
    lea ax, stack
    mov sp, ax
    mov ax, 0x7654
    mov [stack - 2], ax
    mov [stack - 4], ax

    ; CALL FAR cd
    ; self-modify the jmp to go to the right place
    lea ax, far_exit_1
    mov word [call_cd + 1], ax
    mov ax, cs
    mov word [call_cd + 3], ax
call_cd:
    call 0x1234:0x4321
call_cd_fail:
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
    ; first item on the stack is the old CS (which is the current CS)
    mov ax, cs
    mov bx, [stack - 2]
    int 0x23

    ; next item is the instruction after the call
    mov ax, call_cd_fail
    mov bx, [stack - 4]
    int 0x23

    ; reset stack address
    lea ax, stack
    mov sp, ax
    mov ax, 0x7654
    mov [stack - 2], ax
    mov [stack - 4], ax

    ; CALL FAR ed (mod 0x0, constant DISP)
    lea ax, far_exit_2
    mov word [ptr_ip_cs], ax
    mov ax, cs
    mov word [ptr_ip_cs + 2], ax
    call far [ptr_ip_cs]
call_far_fail:
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
    ; first item on the stack is the old CS (which is the current CS)
    mov ax, cs
    mov bx, [stack - 2]
    int 0x23

    ; next item is the instruction after the call
    mov ax, call_far_fail
    mov bx, [stack - 4]
    int 0x23

    ; reset stack address
    lea ax, stack
    mov sp, ax
    mov ax, 0x7654
    mov [stack - 2], ax
    mov [stack - 4], ax

    ; CALL FAR ed (mod 0x1, constant signed DISP)
    lea ax, far_exit_3
    mov word [ptr_ip_cs], ax
    mov ax, cs
    mov word [ptr_ip_cs + 2], ax
    lea bp, ptr_ip_cs
    mov si, 9
    call far [bp + si - 9]
call_far_fail_2:
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

    ; ... a pit of jmps to avoid!
%rep 256
    jmp fail
%endrep

far_exit_3:

    ; check stack conditions
    ; first item on the stack is the old CS (which is the current CS)
    mov ax, cs
    mov bx, [stack - 2]
    int 0x23

    ; next item is the instruction after the call
    mov ax, call_far_fail_2
    mov bx, [stack - 4]
    int 0x23

    ; reset stack address
    lea ax, stack
    mov sp, ax
    mov ax, 0x7654
    mov [stack - 2], ax
    mov [stack - 4], ax

    ; CALL NEAR ew (mod 0x3, constant DISP)
    lea ax, near_exit_1
    call ax
call_near_fail:
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

near_exit_1:

    ; check stack conditions
    ; item is the instruction after the call
    mov ax, call_near_fail
    mov bx, [stack - 2]
    int 0x23

    ; reset stack address
    lea ax, stack
    mov sp, ax
    mov ax, 0x7654
    mov [stack - 2], ax
    mov [stack - 4], ax

    ; CALL NEAR ew (mod 0x1, constant signed DISP, BP/SI form)
    lea ax, near_exit_2
    mov word [ptr_ip_cs], ax
    lea bp, ptr_ip_cs
    mov si, 9
    call near [bp + si - 9]
call_near_fail_2:
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

near_exit_2:

    ; check stack conditions
    ; item is the instruction after the call
    mov ax, call_near_fail_2
    mov bx, [stack - 2]
    int 0x23

    ; reset stack address
    lea ax, stack
    mov sp, ax
    mov ax, 0x7654
    mov [stack - 2], ax
    mov [stack - 4], ax

    ; JMP NEAR ew (mod 0x1, constant signed DISP, BX/DI form)
    lea ax, near_exit_3
    mov word [ptr_ip_cs], ax
    lea bx, ptr_ip_cs
    mov di, 9
    call near [bx + di - 9]
call_near_fail_3:
    ; assert a failure
    mov ax, 0xabcd
    mov bx, 0xffff
    int 0x23

near_exit_3:
    ; check stack conditions
    ; item is the instruction after the call
    mov ax, call_near_fail_3
    mov bx, [stack - 2]
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
