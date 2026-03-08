; not.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; NOT eb
 ; not_eb %1 (not %2)
%macro not_eb 3
    mov %1, %2
    not %1
    mov ah, %1
    mov al, %3
%endmacro
    ; NOT eb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_3b not_eb, al, 0x8f, 0x70

    ; NOT eb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_3b not_eb, al, 0x0, 0xff

    ; NOT eb (mod 0x0 with constant DISP, reg is AL,...BH)
    not_eb byte [bb], 0x8f, 0x70
    int 0x22

    ; NOT eb (mod 0x0 with constant DISP, reg is AL,...BH)
    not_eb byte [bb], 0x0, 0xff
    int 0x22

    ; encodes to nop, nop
not_eb_b db 0x90, 0x90

    ; NOT eb, rb (mod 0x1 with constant signed DISP, reg is AL,...BH)
    mov bp, not_eb_b
    mov si, 9
    not_eb byte [bp + si - 9], 0x8f, 0x70
    int 0x22
    not_eb byte [bp + si - 9], 0x0, 0xff
    int 0x22

    ; NOT ew
 ; not_ew %1 (not %2)
%macro not_ew 3
    mov %1, %2
    not %1
    mov ax, %1
    mov bx, %3
%endmacro
    ; NOT ew (mod 0x3, reg is AX,CX,DX,BX,SP,BP,SI,DI)
    rep_macro_3w not_ew, ax, 0xff8f, 0x70

    ; NOT ew (mod 0x3, reg is AX,CX,DX,BX,SP,BP,SI,DI)
    rep_macro_3w not_ew, ax, 0x0, 0xffff

    ; NOT ew (mod 0x0 with constant DISP)
    not_ew word [bb], 0xff8f, 0x70
    int 0x23

    ; NOT ew (mod 0x0 with constant DISP)
    not_ew word [bb], 0x0, 0xffff
    int 0x23

    ; encodes to nop, nop
not_ew_w db 0x90, 0x90, 0x90, 0x90

    ; SUB eb, rb (mod 0x1 with constant signed DISP, reg is AL,...BH, result is an overflow)
    mov bp, not_ew_w
    mov si, 9
    not_ew word [bp + si - 9], 0xff8f, 0x70
    int 0x23
    not_ew word [bp + si - 9], 0x0, 0xffff
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
bb db 0x0, 0x0
