; sub.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; SUB eb, rb
 ; sub_eb_rb %1, %2 (sub %4 - %3 and test that it equals %5)
%macro sub_eb_rb 5
    mov %1, %3
%if %1 == al
    mov ah, %4
    sub ah, %1
%else
    mov %2, %4
    sub %2, %1
    mov ah, %2
%endif
    mov al, %5
%endmacro
    ; SUB eb, rb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b sub_eb_rb, al, al, 0xe, 0xf, 0x1

    ; SUB eb, rb (mod 0x3, reg is AL,...BH, result is an overflow)
    rep_macro_5b sub_eb_rb, al, al, 0xff, 0x0, 0x1

    ; SUB eb, rb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b sub_eb_rb, al, byte [bb], 0xe, 0xf, 0x1

    ; SUB eb, rb (mod 0x0 with constant DISP, reg is AL,...BH, result is an overflow)
    rep_macro_5b sub_eb_rb, al, byte [bb], 0xff, 0x0, 0x1

    ; encodes to nop, nop
sub_rb_eb_b db 0x90, 0x90

    ; SUB eb, rb (mod 0x1 with constant signed DISP, reg is AL,...BH, result is an overflow)
    mov bp, sub_rb_eb_b
    mov si, 9
    rep_macro_5b sub_eb_rb, al, byte [bp + si - 9], 0xff, 0x0, 0x1

    ; SUB rb, eb
 ; sub_rb_eb %1, %2 (sub %3 - %4 and test that it equals %5)
%macro sub_rb_eb 5
    mov %1, %3
%if %1 == al
    mov ah, %4
    sub %1, ah
%else
    mov %2, %4
    sub %1, %2
%endif
    mov ah, %1
    mov al, %5
%endmacro
    ; SUB rb, eb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b sub_rb_eb, al, al, 0xf, 0xe, 0x1

    ; SUB rb, eb (mod 0x3, reg is AL,...BH, result is an overflow)
    rep_macro_5b sub_rb_eb, al, al, 0x0, 0xff, 0x1

    ; SUB rb, eb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b sub_rb_eb, al, byte [bb], 0xf, 0xe, 0x1

    ; SUB rb, eb (mod 0x0 with constant DISP, reg is AL,...BH, result is an overflow)
    rep_macro_5b sub_rb_eb, al, byte [bb], 0x0, 0xff, 0x1

    ; SUB rb, eb (mod 0x1 with constant signed DISP, reg is AL,...BH, result is an overflow)
    mov bp, sub_rb_eb_b
    mov si, 9
    rep_macro_5b sub_rb_eb, al, byte [bp + si - 9], 0x0, 0xff, 0x1

    ; SUB ew, rw
 ; sub_ew_rw %1, %2 (sub %3 + %4 and test that it equals %5)
%macro sub_ew_rw 5
    mov %1, %3
%if %1 == ax
    mov bx, %4
    sub bx, %1
    mov ax, bx
%else
    mov %2, %4
    sub %2, %1
    mov ax, %2
%endif
    mov bx, %5
%endmacro
    ; SUB ew, rw (mod 0x3, reg is AX,CX,DX,BX,SP,BP,SI,DI)
    rep_macro_5w sub_ew_rw, ax, ax, 0xe, 0xf, 0x1

    ; SUB ew, rw (mod 0x3, reg is AX..., result is an overflow)
    rep_macro_5w sub_ew_rw, ax, ax, 0xffff, 0x0, 0x1

    ; SUB ew, rw (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w sub_ew_rw, ax, word [bb], 0xe, 0xf, 0x1

    ; SUB ew, rw (mod 0x0 with constant DISP, reg is AX..., result is an overflow)
    rep_macro_5w sub_ew_rw, ax, word [bb], 0xffff, 0x0, 0x1

    ; encodes to nop, nop
sub_rw_ew_b db 0x90, 0x90
sub_rw_ew_w db 0x90, 0x90, 0x90, 0x90

    ; SUB ew, rw (mod 0x1 with constant signed DISP, reg is AL,...BH, result is an overflow)
    mov bp, sub_rw_ew_b
    mov si, 9
    rep_macro_5w sub_ew_rw, ax, word [bp + si - 9], 0xfffe, 0x0, 0x2

    ; SUB rw, ew
 ; sub_rw_ew %1, %2 (sub %3 - %4 and test that it equals %5)
%macro sub_rw_ew 5
    mov bp, sub_rw_ew_w
    mov si, 9
%if %1 == si
    ; if si is the register we test, we actually need to adjust
    ; bp so that when it adds the initial value that it points to
    ; the right place
    add bp, si
    sub bp, %3
%endif
%if %1 == bp
    ; ditto when bp is used as the destination
    add si, bp
    sub si, %3
%endif
    mov %1, %3
%if %1 == ax
    mov bx, %4
    sub %1, bx
%else
    mov %2, %4
    sub %1, %2
    mov ax, %1
%endif
    mov bx, %5
%endmacro
    ; SUB rw, ew (mod 0x3, reg is AX...)
    rep_macro_5w sub_rw_ew, ax, ax, 0xf, 0xe, 0x1

    ; SUB rw, ew (mod 0x3, reg is AX..., result is an overflow)
    rep_macro_5w sub_rw_ew, ax, ax, 0x0, 0xffff, 0x1

    ; SUB rw, ew (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w sub_rw_ew, ax, word [bb], 0xf, 0xe, 0x1

    ; SUB rw, ew (mod 0x0 with constant DISP, reg is AX..., result is an overflow)
    rep_macro_5w sub_rw_ew, ax, word [bb], 0x0, 0xffff, 0x1

    ; SUB rw, ew (mod 0x1 with constant signed DISP, reg is AX..., result is an overflow)
    rep_macro_5w sub_rw_ew, ax, word [bp + si - 9], 0x0, 0xffff, 0x1

    ; SUB AL, db
    mov al, 0x0
    sub al, 0x1
    mov ah, 0xff
    int 0x22

    ; SUB AX, dw
    mov ax, 0xffe
    sub ax, 0xfff
    mov bx, 0xffff
    int 0x23

    ; SUB eb, db (mod 0x3, eb is a register AL,CL...BH)
%macro sub_eb_db 4
    mov %1, %3
    sub %1, %2
    mov al, %1
    mov ah, %4
%endmacro
    rep_macro_4b sub_eb_db, al, 0xf, 0xe, 0xff

    ; SUB eb, db (mod 0x0 with constant DISP)
    mov byte [bb], 0xe
    sub byte [bb], 0xf
    mov al, byte [bb]
    mov ah, 0xff
    int 0x22

    ; SUB ew, dw (mod 0x3, ew is a register AX,CL...BP)
%macro sub_ew_dw 4
    mov %1, %3
    sub %1, %2
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w sub_ew_dw, ax, 0xfff, 0xffe, 0xffff

    ; SUB ew, dw (mod 0x0 with constant DISP)
    mov word [bb], 0xffe
    sub word [bb], 0xfff
    mov ax, word [bb]
    mov bx, 0xffff
    int 0x23

    ; SUB ew, db (mod 0x3, ew is a register AX,CL...BP)
%macro sub_ew_db 4
    mov %1, %3
    sub %1, %2
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w sub_ew_db, ax, 0xf, 0xff, 0xf0

    ; SUB ew, db (mod 0x0 with constant DISP)
    mov word [bb], 0xff
    sub word [bb], 0xf
    mov ax, word [bb]
    mov bx, 0xf0
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
bb db 0x0, 0x0
