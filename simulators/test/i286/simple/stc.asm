; stc.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    stc ; carry = 1
    mov al, 0x0
    adc al, 0x1 ; adds 0 + 1 + 1 (with carry)
    mov ah, 0x2
    int 0x22 ; test that the result is 0x2

    ; carry should now be 0
    mov al, 0x0
    adc al, 0x1 ; adds 0 + 1 (with no carry)
    mov ah, 0x1
    int 0x22 ; test that the result is 0x1

    ; should now be able to set carry again
    stc ; carry = 1
    mov al, 0x0
    adc al, 0x1 ; adds 0 + 1 + 1 (with carry)
    mov ah, 0x2
    int 0x22 ; test that the result is 0x2
    
    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate
