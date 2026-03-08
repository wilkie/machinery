; push.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
 ; push_ew %1 (push %2 and test that it remains there)
%macro push_ew 3
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; push first value
    mov %1, %2
    push %1

    ; it should be in memory at the stack address - 2
    mov ax, word [stack - 2]
    mov bx, %2
    int 0x23

    ; push a second time with the second value
    mov %1, %3
    push %1

    mov ax, word [stack - 4] ; test that the second value is also on the stack
    mov bx, %3
    int 0x23

    mov ax, word [stack - 2] ; without affecting the first value, which is still there
    mov bx, %2
    int 0x23

    ; check SP
    mov ax, sp
    lea bx, [stack - 4]
    int 0x23
%endmacro

    ; PUSH 0x50+rw (single byte register encoding)
    push_ew ax, 0x1234, 0x5678
    push_ew cx, 0x2345, 0x6789
    push_ew dx, 0x3456, 0x789a
    push_ew bx, 0x4567, 0x89ab
    ; testing the value of SP means pushing 'stack' and 'stack - 2'
    ; so as to not make it super complicated...
    push_ew sp, stack, stack - 2
    push_ew bp, 0x5678, 0x9abc
    push_ew si, 0x6789, 0xabcd
    push_ew di, 0x789a, 0xbcde

    ; PUSH ew (usually just a memory reference but, technically, allows a register encoding)
    ; mod 0x0 and constant DISP
    push_ew word [bb], 0x4321, 0x8765

    ; mod 0x1 and constant signed DISP
    mov bp, bb
    mov si, 9
    push_ew word [bp + si - 9], 0x5432, 0x9876

    ; PUSH ew (mod 0x3 - redundant register encoding via 0xff /6)
 ; push_rw %1 (push %2 and test that it remains there)
%macro push_ew_hardcoded 4
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; push first value
    mov %1, %2
    db 0xff
    db (0xf0 + %4)

    ; it should be in memory at the stack address - 2
    mov ax, word [stack - 2]
    mov bx, %2
    int 0x23

    ; push a second time with the second value
    mov %1, %3
    db 0xff
    db (0xf0 + %4)

    mov ax, word [stack - 4] ; test that the second value is also on the stack
    mov bx, %3
    int 0x23

    mov ax, word [stack - 2] ; without affecting the first value, which is still there
    mov bx, %2
    int 0x23

    ; check SP
    mov ax, sp
    lea bx, [stack - 4]
    int 0x23
%endmacro
    ; PUSH 0xFF /6+rw (double byte register encoding)
    push_ew_hardcoded ax, 0x1234, 0x5678, 0
    push_ew_hardcoded cx, 0x2345, 0x6789, 1
    push_ew_hardcoded dx, 0x3456, 0x789a, 2
    push_ew_hardcoded bx, 0x4567, 0x89ab, 3
    ; testing the value of SP means pushing 'stack' and 'stack - 2'
    ; so as to not make it super complicated...
    push_ew_hardcoded sp, stack, stack - 2, 4
    push_ew_hardcoded bp, 0x5678, 0x9abc, 5
    push_ew_hardcoded si, 0x6789, 0xabcd, 6
    push_ew_hardcoded di, 0x789a, 0xbcde, 7

    ; PUSH dw (16-bit immediate)
 ; push_dw %1 (push %1 and test that it remains there)
%macro push_imm 4
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; push first value
    push %1

    ; it should be in memory at the stack address - 2
    mov ax, word [stack - 2]
    mov bx, %2
    int 0x23

    ; push a second time with the second value
    push %3

    mov ax, word [stack - 4] ; test that the second value is also on the stack
    mov bx, %4
    int 0x23

    mov ax, word [stack - 2] ; without affecting the first value, which is still there
    mov bx, %2
    int 0x23

    ; check SP
    mov ax, sp
    lea bx, [stack - 4]
    int 0x23
%endmacro
    push_imm 0xffff, 0xffff, 0x5400, 0x5400
    push_imm 0x6677, 0x6677, 0x8976, 0x8976

    ; PUSH db (8-bit signed immediate)
    push_imm -1, 0xffff, 0x34, 0x34
    push_imm -5, 0xfffb, -128, 0xff80

    ; PUSH ES
    lea ax, stack
    mov sp, ax
    les ax, [es_data] ; ES = 0x1337
    push es
    mov ax, word [stack - 2]
    mov bx, word [es_data + 2]
    int 0x23
    ; TODO: test fs and gs

    ; difficult to test pushing cs, ds, or ss
    ; just adding 1 to the current DS would shift it 16 bytes which
    ; is not terrible to deal with for cs or ss, but it's tough
    ; TODO: test cs, ds, and ss

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
es_data dw 0x0, 0x1337
bb db 0x0, 0x0
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
