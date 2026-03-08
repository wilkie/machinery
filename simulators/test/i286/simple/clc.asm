; clc.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    clc ; carry = 0
    mov al, 0x0
    adc al, 0x1 ; adds 0 + 1 (with no carry)
    mov ah, 0x1
    int 0x22 ; test that the result is 0x1

    ; carry should now be 0... let's do something to make it 1
    mov al, 0x8f
    add al, 0x7f ; carry is definitely 1
    mov al, 0x0
    adc al, 0x1 ; adds 0 + 1 + 1 (with carry)
    mov ah, 0x2
    int 0x22 ; test that the result is 0x2

    ; carry should now be 0 again... let's do something to make it 1
    mov al, 0x8f
    add al, 0x7f ; carry is definitely 1... as it was before
    ; clear carry
    clc
    mov al, 0x0
    adc al, 0x1 ; adds 0 + 1 (with no carry)
    mov ah, 0x1
    int 0x22 ; test that the result is 0x1
    
    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate
