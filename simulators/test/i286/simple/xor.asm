; xor.asm
bits 16
org 0x100                ; Set the xorigin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; XOR eb, rb
 ; xor_eb_rb %1, %2 (xor %3 ^ %4 and test that it equals %5)
%macro xor_eb_rb 5
    mov %1, %3
%if %1 == al
    mov ah, %4
    xor ah, %1
%else
    mov %2, %4
    xor %2, %1
    mov ah, %2
%endif
    mov al, %5
%endmacro
    ; XOR eb, rb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b xor_eb_rb, al, al, 0x8f, 0x77, 0xf8

    ; XOR eb, rb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH) zeroing
    rep_macro_5b_double xor_eb_rb, al, 0x8f, 0x0

    ; XOR eb, rb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b xor_eb_rb, al, byte [bb], 0x8f, 0x77, 0xf8

    ; encodes to nop, nop
xor_rb_eb_b db 0x90, 0x90

    ; XOR eb, rb (mod 0x1 with constant signed DISP, reg is AL,...BH)
    mov bp, xor_rb_eb_b
    mov si, 1
    rep_macro_5b xor_eb_rb, al, byte [bp + si - 9], 0x8f, 0x77, 0xf8

    ; XOR rb, eb
 ; xor_rb_eb %1, %2 (xor %3 - %4 and test that it equals %5)
%macro xor_rb_eb 5
    mov %1, %3
%if %1 == al
    mov ah, %4
    xor %1, ah
%else
    mov %2, %4
    xor %1, %2
%endif
    mov ah, %1
    mov al, %5
%endmacro
    ; XOR rb, eb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b xor_rb_eb, al, al, 0x8f, 0x77, 0xf8

    ; XOR rb, eb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH) zeroing
    rep_macro_5b_double xor_rb_eb, al, 0x8f, 0x0

    ; XOR rb, eb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b xor_rb_eb, al, byte [bb], 0x8f, 0x77, 0xf8

    ; XOR rb, eb (mod 0x1 with constant signed DISP, reg is AL,...BH)
    mov bp, xor_rb_eb_b
    mov si, 1
    rep_macro_5b xor_rb_eb, al, byte [bp + si - 9], 0x8f, 0x77, 0xf8

    ; XOR ew, rw
 ; xor_ew_rw %1, %2 (xor %3 ^ %4 and test that it equals %5)
%macro xor_ew_rw 5
    mov %1, %3
%if %1 == ax
    mov bx, %4
    xor bx, %1
    mov ax, bx
%else
    mov %2, %4
    xor %2, %1
    mov ax, %2
%endif
    mov bx, %5
%endmacro
    ; XOR ew, rw (mod 0x3, reg is AX...)
    rep_macro_5w xor_ew_rw, ax, ax, 0x8f8f, 0x7777, 0xf8f8

    ; XOR ew, rw (mod 0x3, reg is AX...) zeroing
    rep_macro_5w_double xor_ew_rw, ax, 0x8f8f, 0x0

    ; XOR ew, rw (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w xor_ew_rw, ax, word [bb], 0x8f8f, 0x7777, 0xf8f8

    ; encodes to nop, nop
xor_rw_ew_b db 0x90, 0x90
xor_rw_ew_w db 0x90, 0x90, 0x90, 0x90

    ; XOR ew, rw (mod 0x1 with constant signed DISP, reg is AX,...DI)
    mov bp, xor_rw_ew_b
    mov si, 9
    ; Use no_bp_si variant because [bp + si - 9] addressing clobbers bp/si
    rep_macro_5w_no_bp_si xor_ew_rw, ax, word [bp + si - 9], 0x8f8f, 0x7777, 0xf8f8

    ; XOR rw, ew
 ; xor_rw_ew %1, %2 (xor %3 ^ %4 and test that it equals %5)
%macro xor_rw_ew 5
    mov %1, %3
%if %1 == ax
    mov bx, %4
    xor %1, bx
%else
    mov %2, %4
    xor %1, %2
    mov ax, %1
%endif
    mov bx, %5
%endmacro
    ; XOR rw, ew (mod 0x3, reg is AX...)
    rep_macro_5w xor_rw_ew, ax, ax, 0x8f8f, 0x7777, 0xf8f8

    ; XOR rw, ew (mod 0x3, reg is AX...)
    rep_macro_5w_double xor_rw_ew, ax, 0x8f8f, 0x0

    ; XOR rw, ew (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w xor_rw_ew, ax, word [bb], 0x8f8f, 0x7777, 0xf8f8

    ; XOR rw, ew (mod 0x1 with constant signed DISP, reg is AX,...DI)
    mov bp, xor_rw_ew_w
    mov si, 1
    ; Use no_bp_si variant because [bp + si - 9] addressing clobbers bp/si
    rep_macro_5w_no_bp_si xor_rw_ew, ax, word [bp + si - 9], 0x8f8f, 0x7777, 0xf8f8

    ; XOR AL, db
    mov al, 0x8f
    xor al, 0x77
    mov ah, 0xf8
    int 0x22

    ; XOR AX, dw
    mov ax, 0x8f
    xor ax, 0x77
    mov bx, 0xf8
    int 0x23

    ; XOR eb, db (mod 0x3, eb is a register AL,CL...BH)
%macro xor_eb_db 4
    mov %1, %3
    xor %1, %2
    mov al, %1
    mov ah, %4
%endmacro
    rep_macro_4b xor_eb_db, al, 0x8f, 0x77, 0xf8

    ; XOR eb, db (mod 0x0 with constant DISP)
    mov byte [bb], 0x8f
    xor byte [bb], 0x77
    mov al, byte [bb]
    mov ah, 0xf8
    int 0x22

    ; XOR ew, dw (mod 0x3, ew is a register AX,CL...BP)
%macro xor_ew_dw 4
    mov %1, %3
    xor %1, %2
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w xor_ew_dw, ax, 0x8f8f, 0x7777, 0xf8f8

    ; XOR ew, dw (mod 0x0 with constant DISP)
    mov word [bb], 0x8f8f
    xor word [bb], 0x7777
    mov ax, word [bb]
    mov bx, 0xf8f8
    int 0x23

    ; XOR ew, db (mod 0x3, ew is a register AX,CL...BP)
%macro xor_ew_db 4
    mov %1, %3
    xor %1, %2
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w xor_ew_db, ax, 0xff85, 0x1234, 0xedb1

    ; XOR ew, db (mod 0x0 with constant DISP)
    mov word [bb], 0x8f77
    xor word [bb], 0x57
    mov ax, word [bb]
    mov bx, 0x8f20
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
bb db 0x0, 0x0
