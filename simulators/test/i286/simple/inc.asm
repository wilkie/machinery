; inc.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; INC eb
 ; inc_eb %1 (inc %2 and test that it equals %3)
%macro inc_eb 3
    mov %1, %2
    inc %1
    mov al, %1
    mov ah, %3
%endmacro
    ; go through all the 8-bit registers
    rep_macro_3b inc_eb, al, 0x0, 0x1
    rep_macro_3b inc_eb, al, 0xff, 0x0

    inc_eb byte [bb], 0x0, 0x1
    int 0x22
    inc_eb byte [bb], 0xff, 0x0
    int 0x22

    mov bp, bb
    mov si, 9
    inc_eb byte [bp + si - 9], 0x0, 0x1
    int 0x22
    inc_eb byte [bp + si - 9], 0xff, 0x0
    int 0x22

    ; INC ew
    ; technically, we can form an INC that targets a register
    ; even though there is a shorter encoding (0x4x seen below)
    ; INC rw (AX)
 ; inc_ew %1 (inc %2 and test that it equals %3)
%macro inc_ew_hardcoded 4
    mov %1, %2
    db 0xff
    db (0xc0 + %4)
    mov ax, %1
    mov bx, %3
%endmacro
    inc_ew_hardcoded ax, 0x0, 0x1, 0
    int 0x23
    inc_ew_hardcoded ax, 0xff, 0x100, 0
    int 0x23
    inc_ew_hardcoded ax, 0xffff, 0x0, 0
    int 0x23
    inc_ew_hardcoded cx, 0x0, 0x1, 1
    int 0x23
    inc_ew_hardcoded cx, 0xff, 0x100, 1
    int 0x23
    inc_ew_hardcoded cx, 0xffff, 0x0, 1
    int 0x23
    inc_ew_hardcoded dx, 0x0, 0x1, 2
    int 0x23
    inc_ew_hardcoded dx, 0xff, 0x100, 2
    int 0x23
    inc_ew_hardcoded dx, 0xffff, 0x0, 2
    int 0x23
    inc_ew_hardcoded bx, 0x0, 0x1, 3
    int 0x23
    inc_ew_hardcoded bx, 0xff, 0x100, 3
    int 0x23
    inc_ew_hardcoded bx, 0xffff, 0x0, 3
    int 0x23
    inc_ew_hardcoded sp, 0x0, 0x1, 4
    int 0x23
    inc_ew_hardcoded sp, 0xff, 0x100, 4
    int 0x23
    inc_ew_hardcoded sp, 0xffff, 0x0, 4
    int 0x23
    inc_ew_hardcoded bp, 0x0, 0x1, 5
    int 0x23
    inc_ew_hardcoded bp, 0xff, 0x100, 5
    int 0x23
    inc_ew_hardcoded bp, 0xffff, 0x0, 5
    int 0x23
    inc_ew_hardcoded si, 0x0, 0x1, 6
    int 0x23
    inc_ew_hardcoded si, 0xff, 0x100, 6
    int 0x23
    inc_ew_hardcoded si, 0xffff, 0x0, 6
    int 0x23
    inc_ew_hardcoded di, 0x0, 0x1, 7
    int 0x23
    inc_ew_hardcoded di, 0xff, 0x100, 7
    int 0x23
    inc_ew_hardcoded di, 0xffff, 0x0, 7
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

 ; inc_ew %1 (inc %2 and test that it equals %3)
%macro inc_ew 3
    mov %1, %2
    inc %1
    mov ax, %1
    mov bx, %3
%endmacro
    inc_ew word [bb], 0x0, 0x1
    int 0x23
    inc_ew word [bb], 0xff, 0x100
    int 0x23
    inc_ew word [bb], 0xffff, 0x0
    int 0x23

    mov bp, bb
    mov si, 9
    inc_ew word [bp + si - 9], 0x0, 0x1
    int 0x23
    inc_ew word [bp + si - 9], 0xff, 0x100
    int 0x23
    inc_ew word [bp + si - 9], 0xffff, 0x0
    int 0x23

    ; INC rw
    ; these are just one byte encodings that avoid a ModRM byte
    ; 0x40 (inc AX)
    ; 0x41 (inc CX)
    ; 0x42 (inc DX)
    ; 0x43 (inc BX)
    ; 0x44 (inc SP)
    ; 0x45 (inc BP)
    ; 0x46 (inc SI)
    ; 0x47 (inc DI)
    rep_macro_3w inc_ew, ax, 0x0, 0x1
    rep_macro_3w inc_ew, ax, 0xff, 0x100
    rep_macro_3w inc_ew, ax, 0xffff, 0x0

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
bb db 0x0, 0x0
