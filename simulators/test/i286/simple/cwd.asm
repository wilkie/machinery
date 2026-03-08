; cwd.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; CWD AX
    mov dx, 0x1234
    mov ax, 0x8000 ; should sign extend AX into DX
    cwd
    mov bx, 0x8000
    int 0x23
    mov ax, dx
    mov bx, 0xffff
    int 0x23

    mov dx, 0x1234
    mov ax, 0x0000 ; should zero extend AX into DX (keeping it 0)
    cwd
    mov bx, 0x0000
    int 0x23
    mov ax, dx
    mov bx, 0x0
    int 0x23

    mov dx, 0x1234
    mov ax, 0x6500 ; should zero extend AX into DX (making it 0)
    cwd
    mov bx, 0x6500
    int 0x23
    mov ax, dx
    mov bx, 0x0
    int 0x23

    mov dx, 0x1234
    mov ax, 0xff00 ; should sign extend AX into DX (making it 0xffff)
    cwd
    mov bx, 0xff00
    int 0x23
    mov ax, dx
    mov bx, 0xffff
    int 0x23

    mov dx, 0x1234
    mov ax, 0xffff ; should sign extend AX into AX (keeping it 0xffff)
    cwd
    mov bx, 0xffff
    int 0x23
    mov ax, dx
    mov bx, 0xffff
    int 0x23

    mov dx, 0x1234
    mov ax, 0xff05 ; should sign extend AX into DX (making it 0xffff)
    cwd
    mov bx, 0xff05
    int 0x23
    mov ax, dx
    mov bx, 0xffff
    int 0x23

    mov dx, 0x1234
    mov ax, 0x007f ; should zero extend AX into DX (keeping it 0x7f)
    cwd
    mov bx, 0x007f
    int 0x23
    mov ax, dx
    mov bx, 0x0
    int 0x23

    mov dx, 0x1234
    mov ax, 0xff7f ; should sign extend AX into DX (keeping it to 0xff7f)
    cwd
    mov bx, 0xff7f
    int 0x23
    mov ax, dx
    mov bx, 0xffff
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate
