; pop.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
 ; pop_ew %1 (place %2 and %3 on the stack and then pop %2 to see if it gets it, then repeat)
%macro pop_ew 3
    ; set up stack address
    lea ax, [stack - 4]
    mov sp, ax

    ; copy over both values
    mov ax, %3
    mov [stack - 4], ax
    mov ax, %2
    mov [stack - 2], ax

    ; pop first value
    pop %1

    ; it should be the same as the second value
    mov ax, %1
    mov bx, %3
    int 0x23

    ; pop a second time for the first value
    pop %1

    ; it should match that first value
    mov ax, %1
    mov bx, %2
    int 0x23

    ; the values on the stack should still be there
    mov ax, word [stack - 2]
    mov bx, %2
    int 0x23
    mov ax, word [stack - 4]
    mov bx, %3
    int 0x23

    ; check SP (should be the top of the stack)
    mov ax, sp
    lea bx, [stack]
    int 0x23
%endmacro

    ; POP 0x58+rw (single byte register encoding)
    pop_ew ax, 0x1234, 0x5678
    pop_ew cx, 0x2345, 0x6789
    pop_ew dx, 0x3456, 0x789a
    pop_ew bx, 0x4567, 0x89ab
    ; testing the value of SP means pushing 'stack' and 'stack - 2'
    ; so as to not make it super complicated...
    pop_ew sp, stack, stack - 2
    pop_ew bp, 0x5678, 0x9abc
    pop_ew si, 0x6789, 0xabcd
    pop_ew di, 0x789a, 0xbcde

    ; PUSH ew (usually just a memory reference but, technically, allows a register encoding)
    ; mod 0x0 and constant DISP
    pop_ew word [bb], 0x4321, 0x8765

    ; mod 0x1 and constant signed DISP
    mov bp, bb
    mov si, 9
    pop_ew word [bp + si - 9], 0x5432, 0x9876

    ; PUSH ew (mod 0x3 - redundant register encoding via 0xff /6)
 ; pop_rw %1 (pop %2 and test that it remains there)
%macro pop_ew_hardcoded 4
    ; set up stack address
    lea ax, [stack - 4]
    mov sp, ax

    ; copy over both values
    mov ax, %3
    mov [stack - 4], ax
    mov ax, %2
    mov [stack - 2], ax

    ; pop first value
    db 0x8f
    db (0xc0 + %4)

    ; it should be the same as the second value
    mov ax, %1
    mov bx, %3
    int 0x23

    ; pop a second time for the first value
    db 0x8f
    db (0xc0 + %4)

    ; it should match that first value
    mov ax, %1
    mov bx, %2
    int 0x23

    ; the values on the stack should still be there
    mov ax, word [stack - 2]
    mov bx, %2
    int 0x23
    mov ax, word [stack - 4]
    mov bx, %3
    int 0x23

    ; check SP (should be the top of the stack)
    mov ax, sp
    lea bx, [stack]
    int 0x23
%endmacro
    ; POP 0x8F /0+rw (double byte register encoding)
    pop_ew_hardcoded ax, 0x1234, 0x5678, 0
    pop_ew_hardcoded cx, 0x2345, 0x6789, 1
    pop_ew_hardcoded dx, 0x3456, 0x789a, 2
    pop_ew_hardcoded bx, 0x4567, 0x89ab, 3
    ; testing the value of SP means poping 'stack' and 'stack - 2'
    ; so as to not make it super complicated...
    pop_ew_hardcoded sp, stack, stack - 2, 4
    pop_ew_hardcoded bp, 0x5678, 0x9abc, 5
    pop_ew_hardcoded si, 0x6789, 0xabcd, 6
    pop_ew_hardcoded di, 0x789a, 0xbcde, 7

    ; POP ES
    lea ax, [stack - 2]
    mov sp, ax
    mov word [stack - 2], 0x1337
    pop es
    mov ax, es
    mov bx, 0x1337
    int 0x23
    ; check SP (should be the top of the stack)
    mov ax, sp
    lea bx, [stack]
    int 0x23
    ; TODO: test fs and gs

    ; POP SS
    mov cx, ss
    lea ax, [stack - 2]
    mov sp, ax
    mov word [stack - 2], 0x4321
    pop ss
    mov ax, ss
    ; we've messed up SS, so let's reset it
    mov ss, cx
    mov bx, 0x4321
    int 0x23
    ; check SP (should be the top of the stack)
    mov ax, sp
    lea bx, [stack]
    int 0x23

    ; POP DS
    mov cx, ds
    lea ax, [stack - 2]
    mov sp, ax
    mov word [stack - 2], 0x4321
    pop ds
    mov ax, ds
    ; we've messed up DS, so let's reset it real quick
    mov ds, cx
    mov bx, 0x4321
    int 0x23
    ; check SP (should be the top of the stack)
    mov ax, sp
    lea bx, [stack]
    int 0x23

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
