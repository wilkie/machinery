; or.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; OR eb, rb
 ; or_eb_rb %1, %2 (or %3 + %4 and test that it equals %5)
%macro or_eb_rb 5
    mov %1, %3
%if %1 == al
    mov ah, %4
    or ah, %1
%else
    mov %2, %4
    or %2, %1
    mov ah, %2
%endif
    mov al, %5
%endmacro
    ; OR eb, rb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b or_eb_rb, al, al, 0xf, 0xf0, 0xff

    ; OR eb, rb (mod 0x3, reg is AL,...BH)
    rep_macro_5b or_eb_rb, al, al, 0x8, 0x5, 0xd

    ; OR eb, rb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b or_eb_rb, al, byte [bb], 0xf, 0xf0, 0xff

    ; OR eb, rb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b or_eb_rb, al, byte [bb], 0x8, 0x5, 0xd

    ; encodes to nop, nop
or_rb_eb_b db 0x90, 0x90

    ; OR eb, rb (mod 0x1 with constant signed DISP, reg is AL,...BH)
    mov bp, or_rb_eb_b
    mov si, 1
    rep_macro_5b or_eb_rb, al, byte [bp + si - 9], 0xf, 0xf0, 0xff

    ; OR rb, eb
 ; or_rb_eb %1, %2 (or %3 - %4 and test that it equals %5)
%macro or_rb_eb 5
    mov %1, %3
%if %1 == al
    mov ah, %4
    or %1, ah
%else
    mov %2, %4
    or %1, %2
%endif
    mov ah, %1
    mov al, %5
%endmacro
    ; OR rb, eb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b or_rb_eb, al, al, 0xf, 0xf0, 0xff

    ; OR rb, eb (mod 0x3, reg is AL,...BH)
    rep_macro_5b or_rb_eb, al, al, 0x8, 0x5, 0xd

    ; OR rb, eb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b or_rb_eb, al, byte [bb], 0xf, 0xf0, 0xff

    ; OR rb, eb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b or_rb_eb, al, byte [bb], 0x8, 0x5, 0xd

    ; OR rb, eb (mod 0x1 with constant signed DISP, reg is AL,...BH)
    mov bp, or_rb_eb_b
    mov si, 1
    rep_macro_5b or_rb_eb, al, byte [bp + si - 9], 0x8, 0x5, 0xd

    ; OR ew, rw
 ; or_ew_rw %1, %2 (or %3 + %4 and test that it equals %5)
%macro or_ew_rw 5
    mov %1, %3
%if %1 == ax
    mov bx, %4
    or bx, %1
    mov ax, bx
%else
    mov %2, %4
    or %2, %1
    mov ax, %2
%endif
    mov bx, %5
%endmacro
    ; OR ew, rw (mod 0x3, reg is AX...)
    rep_macro_5w or_ew_rw, ax, ax, 0x80f, 0x5f0, 0xdff

    ; OR ew, rw (mod 0x3, reg is AX...)
    rep_macro_5w or_ew_rw, ax, ax, 0x85f0, 0x580f, 0xddff

    ; OR ew, rw (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w or_ew_rw, ax, word [bb], 0x80f, 0x5f0, 0xdff

    ; OR ew, rw (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w or_ew_rw, ax, word [bb], 0x85f0, 0x580f, 0xddff

    ; encodes to nop, nop
or_rw_ew_b db 0x90, 0x90
or_rw_ew_w db 0x90, 0x90, 0x90, 0x90

    ; OR ew, rw (mod 0x1 with constant signed DISP, reg is AX,...DI)
    ; Use no_bp_si variant because [bp + si - 9] addressing clobbers bp/si
    mov bp, or_rw_ew_b
    mov si, 1
    rep_macro_5w_no_bp_si or_ew_rw, ax, word [bp + si - 9], 0x85f0, 0x580f, 0xddff

    ; OR rw, ew
 ; or_rw_ew %1, %2 (or %3 + %4 and test that it equals %5)
%macro or_rw_ew 5
    mov %1, %3
%if %1 == ax
    mov bx, %4
    or %1, bx
%else
    mov %2, %4
    or %1, %2
    mov ax, %1
%endif
    mov bx, %5
%endmacro
    ; OR rw, ew (mod 0x3, reg is AX...)
    rep_macro_5w or_rw_ew, ax, ax, 0x80f, 0x5f0, 0xdff

    ; OR rw, ew (mod 0x3, reg is AX...)
    rep_macro_5w or_rw_ew, ax, ax, 0x85f0, 0x580f, 0xddff

    ; OR rw, ew (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w or_rw_ew, ax, word [bb], 0x80f, 0x5f0, 0xdff

    ; OR rw, ew (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w or_rw_ew, ax, word [bb], 0x85f0, 0x580f, 0xddff

    ; OR rw, ew (mod 0x1 with constant signed DISP, reg is AX,...DI)
    ; Use no_bp_si variant because [bp + si - 9] addressing clobbers bp/si
    mov bp, or_rw_ew_w
    mov si, 1
    rep_macro_5w_no_bp_si or_rw_ew, ax, word [bp + si - 9], 0x85f0, 0x580f, 0xddff

    ; OR AL, db
    mov al, 0x85
    or al, 0x57
    mov ah, 0xd7
    int 0x22

    ; OR AX, dw
    mov ax, 0x80f
    or ax, 0x5f0
    mov bx, 0xdff
    int 0x23

    ; OR eb, db (mod 0x3, eb is a register AL,CL...BH)
%macro or_eb_db 4
    mov %1, %3
    or %1, %2
    mov al, %1
    mov ah, %4
%endmacro
    rep_macro_4b or_eb_db, al, 0x85, 0x57, 0xd7

    ; OR eb, db (mod 0x0 with constant DISP)
    mov byte [bb], 0x85
    or byte [bb], 0x57
    mov al, byte [bb]
    mov ah, 0xd7
    int 0x22

    ; OR ew, dw (mod 0x3, ew is a register AX,CL...BP)
%macro or_ew_dw 4
    mov %1, %3
    or %1, %2
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w or_ew_dw, ax, 0x8f0, 0x50f, 0xdff

    ; OR ew, dw (mod 0x0 with constant DISP)
    mov word [bb], 0x8f0
    or word [bb], 0x50f
    mov ax, word [bb]
    mov bx, 0xdff
    int 0x23

    ; OR ew, db (mod 0x3, ew is a register AX,CL...BP)
%macro or_ew_db 4
    mov %1, %3
    or %1, %2
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w or_ew_db, ax, 0xff85, 0xf58, 0xffdd

    ; OR ew, db (mod 0x0 with constant DISP)
    mov word [bb], 0x888
    or word [bb], 0x57
    mov ax, word [bb]
    mov bx, 0x8df
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
bb db 0x0, 0x0
