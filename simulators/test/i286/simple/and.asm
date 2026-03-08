; and.asm
bits 16
org 0x100                ; Set the andigin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; AND eb, rb
 ; and_eb_rb %1, %2 (and %3 & %4 and test that it equals %5)
%macro and_eb_rb 5
    mov %1, %3
%if %1 == al
    mov ah, %4
    and ah, %1
%else
    mov %2, %4
    and %2, %1
    mov ah, %2
%endif
    mov al, %5
%endmacro
    ; AND eb, rb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b and_eb_rb, al, al, 0xf, 0xf0, 0x0

    ; AND eb, rb (mod 0x3, reg is AL,...BH)
    rep_macro_5b and_eb_rb, al, al, 0x8, 0x5, 0x0

    ; AND eb, rb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b and_eb_rb, al, byte [bb], 0xf, 0xf0, 0x0

    ; AND eb, rb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b and_eb_rb, al, byte [bb], 0x8, 0x5, 0x0

    ; encodes to nop, nop
and_rb_eb_b db 0x90, 0x90

    ; AND eb, rb (mod 0x1 with constant signed DISP, reg is AL,...BH)
    mov bp, and_rb_eb_b
    mov si, 1
    rep_macro_5b and_eb_rb, al, byte [bp + si - 9], 0xf, 0xf0, 0x0

    ; AND rb, eb
 ; and_rb_eb %1, %2 (and %3 - %4 and test that it equals %5)
%macro and_rb_eb 5
    mov %1, %3
%if %1 == al
    mov ah, %4
    and %1, ah
%else
    mov %2, %4
    and %1, %2
%endif
    mov ah, %1
    mov al, %5
%endmacro
    ; AND rb, eb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b and_rb_eb, al, al, 0xf, 0xf0, 0x0

    ; AND rb, eb (mod 0x3, reg is AL,...BH)
    rep_macro_5b and_rb_eb, al, al, 0x8, 0x5, 0x0

    ; AND rb, eb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b and_rb_eb, al, byte [bb], 0xf, 0xf0, 0x0

    ; AND rb, eb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b and_rb_eb, al, byte [bb], 0x8, 0x5, 0x0

    ; AND rb, eb (mod 0x1 with constant signed DISP, reg is AL,...BH)
    mov bp, and_rb_eb_b
    mov si, 1
    rep_macro_5b and_rb_eb, al, byte [bp + si - 9], 0x8, 0x5, 0x0

    ; AND ew, rw
 ; and_ew_rw %1, %2 (and %3 & %4 and test that it equals %5)
%macro and_ew_rw 5
    mov %1, %3
%if %1 == ax
    mov bx, %4
    and bx, %1
    mov ax, bx
%else
    mov %2, %4
    and %2, %1
    mov ax, %2
%endif
    mov bx, %5
%endmacro
    ; AND ew, rw (mod 0x3, reg is AX...)
    rep_macro_5w and_ew_rw, ax, ax, 0x80f, 0x5f0, 0x0

    ; AND ew, rw (mod 0x3, reg is AX...)
    rep_macro_5w and_ew_rw, ax, ax, 0x85f0, 0x580f, 0x0

    ; AND ew, rw (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w and_ew_rw, ax, word [bb], 0x80f, 0x5f0, 0x0

    ; AND ew, rw (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w and_ew_rw, ax, word [bb], 0x85f0, 0x580f, 0x0

    ; encodes to nop, nop
and_rw_ew_b db 0x90, 0x90
and_rw_ew_w db 0x90, 0x90, 0x90, 0x90

    ; AND ew, rw (mod 0x1 with constant signed DISP, reg is AL,...BH)
    mov bp, and_rw_ew_b
    mov si, 1
    rep_macro_5w and_ew_rw, ax, word [bp + si - 9], 0x85f0, 0x580f, 0x0

    ; AND rw, ew
 ; and_rw_ew %1, %2 (and %3 & %4 and test that it equals %5)
%macro and_rw_ew 5
    mov %1, %3
%if %1 == ax
    mov bx, %4
    and %1, bx
%else
    mov %2, %4
    and %1, %2
    mov ax, %1
%endif
    mov bx, %5
%endmacro
    ; AND rw, ew (mod 0x3, reg is AX...)
    rep_macro_5w and_rw_ew, ax, ax, 0x80f, 0x5f0, 0x0

    ; AND rw, ew (mod 0x3, reg is AX...)
    rep_macro_5w and_rw_ew, ax, ax, 0x85f0, 0x580f, 0x0

    ; AND rw, ew (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w and_rw_ew, ax, word [bb], 0x80f, 0x5f0, 0x0

    ; AND rw, ew (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w and_rw_ew, ax, word [bb], 0x85f0, 0x580f, 0x0

    ; AND rw, ew (mod 0x1 with constant signed DISP, reg is AX...)
    mov bp, and_rw_ew_w
    mov si, 1
    rep_macro_5w and_rw_ew, ax, word [bp + si - 9], 0x85f0, 0x580f, 0x0

    ; AND AL, db
    mov al, 0x85
    and al, 0x57
    mov ah, 0x05
    int 0x22

    ; AND AX, dw
    mov ax, 0x80f
    and ax, 0x5f0
    mov bx, 0x0
    int 0x23

    ; AND eb, db (mod 0x3, eb is a register AL,CL...BH)
%macro and_eb_db 4
    mov %1, %3
    and %1, %2
    mov al, %1
    mov ah, %4
%endmacro
    rep_macro_4b and_eb_db, al, 0x85, 0x57, 0x05

    ; AND eb, db (mod 0x0 with constant DISP)
    mov byte [bb], 0x85
    and byte [bb], 0x57
    mov al, byte [bb]
    mov ah, 0x5
    int 0x22

    ; AND ew, dw (mod 0x3, ew is a register AX,CL...BP)
%macro and_ew_dw 4
    mov %1, %3
    and %1, %2
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w and_ew_dw, ax, 0x8f0, 0x50f, 0x0

    ; AND ew, dw (mod 0x0 with constant DISP)
    mov word [bb], 0x8f0
    and word [bb], 0x50f
    mov ax, word [bb]
    mov bx, 0x0
    int 0x23

    ; AND ew, db (mod 0x3, ew is a register AX,CL...BP)
%macro and_ew_db 4
    mov %1, %3
    and %1, %2
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w and_ew_db, ax, 0xff85, 0xf58, 0xf00

    ; AND ew, db (mod 0x0 with constant DISP)
    mov word [bb], 0x888
    and word [bb], 0x57
    mov ax, word [bb]
    mov bx, 0x0
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
bb db 0x0, 0x0
