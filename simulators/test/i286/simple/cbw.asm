; cbw.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; CBW AH
    mov ax, 0x0080 ; should sign extend AL into AH
    cbw
    mov bx, 0xff80
    int 0x23

    mov ax, 0x0000 ; should zero extend AL into AH (keeping it 0)
    cbw
    mov bx, 0x0000
    int 0x23

    mov ax, 0x6500 ; should zero extend AL into AH (making it 0)
    cbw
    mov bx, 0x0000
    int 0x23

    mov ax, 0xff00 ; should zero extend AL into AH (making it 0)
    cbw
    mov bx, 0x0000
    int 0x23

    mov ax, 0xffff ; should sign extend AL into AH (keeping it 0xffff)
    cbw
    mov bx, 0xffff
    int 0x23

    mov ax, 0xff05 ; should zero extend AL into AH (masking it to just 5)
    cbw
    mov bx, 0x0005
    int 0x23

    mov ax, 0x007f ; should zero extend AL into AH (keeping it 0x7f)
    cbw
    mov bx, 0x007f
    int 0x23

    mov ax, 0xff7f ; should zero extend AL into AH (masking it to 0x7f)
    cbw
    mov bx, 0x007f
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate
