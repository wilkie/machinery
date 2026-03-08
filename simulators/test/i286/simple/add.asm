; add.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; ADD eb, rb
 ; add_eb_rb %1, %2 (add %3 + %4 and test that it equals %5)
%macro add_eb_rb 5
    mov %1, %3
    mov %2, %4
    add %2, %1
    mov ah, %2
    mov al, %5
%endmacro
    ; ADD eb, rb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b add_eb_rb, al, al, 0xf, 0xf, 0x1e

    ; ADD eb, rb (mod 0x3, reg is AL,...BH, result is an overflow)
    rep_macro_5b add_eb_rb, al, al, 0xff, 0xff, 0xfe

    ; ADD eb, rb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b add_eb_rb, al, byte [bb], 0xf, 0xf, 0x1e

    ; ADD eb, rb (mod 0x0 with constant DISP, reg is AL,...BH, result is an overflow)
    rep_macro_5b add_eb_rb, al, byte [bb], 0xff, 0xff, 0xfe

    ; encodes to nop, nop
add_rb_eb_b db 0x90, 0x90

    ; ADD eb, rb (mod 0x1 with constant signed DISP, reg is AL,...BH, result is an overflow)
    mov bp, add_rb_eb_b
    mov si, 1
    rep_macro_5b add_eb_rb, al, byte [bp + si - 9], 0xff, 0xff, 0xfe

    ; ADD rb, eb
 ; add_rb_eb %1, %2 (add %3 + %4 and test that it equals %5)
%macro add_rb_eb 5
    mov %1, %3
    mov %2, %4
    add %1, %2
    mov ah, %1
    mov al, %5
%endmacro
    ; ADD rb, eb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b add_rb_eb, al, al, 0xf, 0xf, 0x1e

    ; ADD rb, eb (mod 0x3, reg is AL,...BH, result is an overflow)
    rep_macro_5b add_rb_eb, al, al, 0xff, 0xff, 0xfe

    ; ADD rb, eb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b add_rb_eb, al, byte [bb], 0xf, 0xf, 0x1e

    ; ADD rb, eb (mod 0x0 with constant DISP, reg is AL,...BH, result is an overflow)
    rep_macro_5b add_rb_eb, al, byte [bb], 0xff, 0xff, 0xfe

    ; ADD rb, eb (mod 0x1 with constant signed DISP, reg is AL,...BH, result is an overflow)
    mov bp, add_rb_eb_b
    mov si, 1
    rep_macro_5b add_rb_eb, al, byte [bp + si - 9], 0xff, 0xff, 0xfe

    ; ADD ew, rw
 ; add_ew_rw %1, %2 (add %3 + %4 and test that it equals %5)
%macro add_ew_rw 5
    mov %1, %3
    mov %2, %4
    add %2, %1
    mov ax, %2
    mov bx, %5
%endmacro
    ; ADD ew, rw (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5w add_ew_rw, ax, ax, 0xf, 0xf, 0x1e

    ; ADD ew, rw (mod 0x3, reg is AL,...BH, result is an overflow)
    rep_macro_5w add_ew_rw, ax, ax, 0xffff, 0xffff, 0xfffe

    ; ADD ew, rw (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5w add_ew_rw, ax, word [bb], 0xf, 0xf, 0x1e

    ; ADD ew, rw (mod 0x0 with constant DISP, reg is AL,...BH, result is an overflow)
    rep_macro_5w add_ew_rw, ax, word [bb], 0xffff, 0xffff, 0xfffe

    ; encodes to nop, nop
add_rw_ew_b db 0x90, 0x90
add_rw_ew_w db 0x90, 0x90, 0x90, 0x90

    ; ADD ew, rw (mod 0x1 with constant signed DISP, reg is AL,...BH, result is an overflow)
    mov bp, add_rw_ew_b
    mov si, 1
    rep_macro_5w add_ew_rw, ax, word [bp + si - 9], 0xffff, 0xffff, 0xfffe

    ; ADD rw, ew
 ; add_rw_ew %1, %2 (add %3 + %4 and test that it equals %5)
%macro add_rw_ew 5
    mov %1, %3
    mov %2, %4
    add %1, %2
    mov ax, %1
    mov bx, %5
%endmacro
    ; ADD rw, ew (mod 0x3, reg is AX...)
    rep_macro_5w add_rw_ew, ax, ax, 0xf, 0xf, 0x1e

    ; ADD rw, ew (mod 0x3, reg is AX..., result is an overflow)
    rep_macro_5w add_rw_ew, ax, ax, 0xffff, 0xffff, 0xfffe

    ; ADD rw, ew (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w add_rw_ew, ax, word [bb], 0xf, 0xf, 0x1e

    ; ADD rw, ew (mod 0x0 with constant DISP, reg is AX..., result is an overflow)
    rep_macro_5w add_rw_ew, ax, word [bb], 0xffff, 0xffff, 0xfffe

    ; ADD rw, ew (mod 0x1 with constant signed DISP, reg is AX..., result is an overflow)
    mov bp, add_rw_ew_w
    mov si, 1
    rep_macro_5w add_rw_ew, ax, word [bp + si - 9], 0xffff, 0xffff, 0xfffe

    ; ADD AL, db
    mov al, 0x0
    add al, 0x1
    mov ah, 0x1
    int 0x22

    ; ADD AX, dw
    mov ax, 0x1
    add ax, 0xffe
    mov bx, 0xfff
    int 0x23

    ; ADD eb, db (mod 0x3, eb is a register AL,CL...BH)
%macro add_eb_db 4
    mov %1, %3
    add %1, %2
    mov al, %1
    mov ah, %4
%endmacro
    rep_macro_4b add_eb_db, al, 0xf, 0xf, 0x1e

    ; ADD eb, db (mod 0x0 with constant DISP)
    mov byte [bb], 0xf
    add byte [bb], 0xf
    mov al, byte [bb]
    mov ah, 0x1e
    int 0x22

    ; ADD ew, dw (mod 0x3, ew is a register AX,CL...BP)
%macro add_ew_dw 4
    mov %1, %3
    add %1, %2
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w add_ew_dw, ax, 0xfff, 0xfff, 0x1ffe

    ; ADD ew, dw (mod 0x0 with constant DISP)
    mov word [bb], 0xfff
    add word [bb], 0xfff
    mov ax, word [bb]
    mov bx, 0x1ffe
    int 0x23

    ; ADD ew, db (mod 0x3, ew is a register AX,CL...BP)
%macro add_ew_db 4
    mov %1, %3
    add %1, %2
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w add_ew_db, ax, 0xf, 0xff, 0x10e

    ; ADD ew, db (mod 0x0 with constant DISP)
    mov word [bb], 0xff
    add word [bb], 0xf
    mov ax, word [bb]
    mov bx, 0x10e
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
bb db 0x0, 0x0
