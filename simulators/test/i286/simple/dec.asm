; dec.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; DEC eb
 ; dec_eb %1 (dec %2 and test that it equals %3)
%macro dec_eb 3
    mov %1, %2
    dec %1
    mov al, %1
    mov ah, %3
%endmacro
    ; go through all the 8-bit registers
    rep_macro_3b dec_eb, al, 0x1, 0x0
    rep_macro_3b dec_eb, al, 0x0, 0xff

    dec_eb byte [bb], 0x1, 0x0
    int 0x22
    dec_eb byte [bb], 0x0, 0xff
    int 0x22

    mov bp, bb
    mov si, 9
    dec_eb byte [bp + si - 9], 0x1, 0x0
    int 0x22
    dec_eb byte [bp + si - 9], 0x0, 0xff
    int 0x22

    ; DEC ew
    ; technically, we can form an DEC that targets a register
    ; even though there is a shorter encoding (0x4x seen below)
    ; DEC rw (AX)
 ; dec_ew %1 (dec %2 and test that it equals %3)
%macro dec_ew_hardcoded 4
    mov %1, %2
    db 0xff
    db (0xc8 + %4)
    mov ax, %1
    mov bx, %3
%endmacro
    dec_ew_hardcoded ax, 0x1, 0x0, 0
    int 0x23
    dec_ew_hardcoded ax, 0x100, 0xff, 0
    int 0x23
    dec_ew_hardcoded ax, 0x0, 0xffff, 0
    int 0x23
    dec_ew_hardcoded cx, 0x1, 0x0, 1
    int 0x23
    dec_ew_hardcoded cx, 0x100, 0xff, 1
    int 0x23
    dec_ew_hardcoded cx, 0x0, 0xffff, 1
    int 0x23
    dec_ew_hardcoded dx, 0x1, 0x0, 2
    int 0x23
    dec_ew_hardcoded dx, 0x100, 0xff, 2
    int 0x23
    dec_ew_hardcoded dx, 0x0, 0xffff, 2
    int 0x23
    dec_ew_hardcoded bx, 0x1, 0x0, 3
    int 0x23
    dec_ew_hardcoded bx, 0x100, 0xff, 3
    int 0x23
    dec_ew_hardcoded bx, 0x0, 0xffff, 3
    int 0x23
    dec_ew_hardcoded sp, 0x1, 0x0, 4
    int 0x23
    dec_ew_hardcoded sp, 0x100, 0xff, 4
    int 0x23
    dec_ew_hardcoded sp, 0x0, 0xffff, 4
    int 0x23
    dec_ew_hardcoded bp, 0x1, 0x0, 5
    int 0x23
    dec_ew_hardcoded bp, 0x100, 0xff, 5
    int 0x23
    dec_ew_hardcoded bp, 0x0, 0xffff, 5
    int 0x23
    dec_ew_hardcoded si, 0x1, 0x0, 6
    int 0x23
    dec_ew_hardcoded si, 0x100, 0xff, 6
    int 0x23
    dec_ew_hardcoded si, 0x0, 0xffff, 6
    int 0x23
    dec_ew_hardcoded di, 0x1, 0x0, 7
    int 0x23
    dec_ew_hardcoded di, 0x100, 0xff, 7
    int 0x23
    dec_ew_hardcoded di, 0x0, 0xffff, 7
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

 ; dec_ew %1 (dec %2 and test that it equals %3)
%macro dec_ew 3
    mov %1, %2
    dec %1
    mov ax, %1
    mov bx, %3
%endmacro
    dec_ew word [bb], 0x1, 0x0
    int 0x23
    dec_ew word [bb], 0x100, 0xff
    int 0x23
    dec_ew word [bb], 0x0, 0xffff
    int 0x23

    mov bp, bb
    mov si, 9
    dec_ew word [bp + si - 9], 0x1, 0x0
    int 0x23
    dec_ew word [bp + si - 9], 0x100, 0xff
    int 0x23
    dec_ew word [bp + si - 9], 0x0, 0xffff
    int 0x23

    ; DEC rw
    ; these are just one byte encodings that avoid a ModRM byte
    ; 0x40 (dec AX)
    ; 0x41 (dec CX)
    ; 0x42 (dec DX)
    ; 0x43 (dec BX)
    ; 0x44 (dec SP)
    ; 0x45 (dec BP)
    ; 0x46 (dec SI)
    ; 0x47 (dec DI)
    rep_macro_3w dec_ew, ax, 0x1, 0x0
    rep_macro_3w dec_ew, ax, 0x100, 0xff
    rep_macro_3w dec_ew, ax, 0x0, 0xffff

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
bb db 0x0, 0x0
