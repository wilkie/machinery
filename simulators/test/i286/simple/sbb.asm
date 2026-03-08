; sbb.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; SBB eb, rb
 ; sbb_eb_rb %1, %2 (subtract %3 - %4 and sbb %3 - %4 and test that it equals %5)
%macro sbb_eb_rb 5
    mov %1, %3
%if %1 == al
    mov ah, %4
    sub ah, %1
    sbb ah, %1
%else
    mov %2, %4
    sub %2, %1
    sbb %2, %1
    mov ah, %2
%endif
    mov al, %5
%endmacro
    ; SBB eb, rb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b sbb_eb_rb, al, al, 0xf, 0xf, 0xf1

    ; SBB eb, rb (mod 0x3, reg is AL,...BH, result is an overflow)
    rep_macro_5b sbb_eb_rb, al, al, 0xf, 0x0, 0xe1

    ; SBB eb, rb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b sbb_eb_rb, al, byte [bb], 0xf, 0xf, 0xf1

    ; SBB eb, rb (mod 0x0 with constant DISP, reg is AL,...BH, result is an overflow)
    rep_macro_5b sbb_eb_rb, al, byte [bb], 0xf, 0x0, 0xe1

    ; encodes to nop, nop
sbb_rb_eb_b db 0x90, 0x90

    ; SBB eb, rb (mod 0x1 with constant signed DISP, reg is AL,...BH, result is an overflow)
    mov bp, sbb_rb_eb_b
    mov si, 9
    rep_macro_5b sbb_eb_rb, al, byte [bp + si - 9], 0xf, 0x0, 0xe1

    ; SBB rb, eb
 ; sbb_rb_eb %1, %2 (sbb %3 - %4 and test that it equals %5)
%macro sbb_rb_eb 5
    mov bp, sbb_rb_eb_b
    mov si, 9
    mov %1, %3
%if %1 == al
    mov ah, %4
    sub %1, ah
    sbb %1, ah
%else
    mov %2, %4
    sub %1, %2
    sbb %1, %2
%endif
    mov ah, %1
    mov al, %5
%endmacro
    ; SBB rb, eb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b sbb_rb_eb, al, al, 0xf, 0xf, 0xf1

    ; SBB rb, eb (mod 0x3, reg is AL,...BH, result is an overflow)
    rep_macro_5b sbb_rb_eb, al, al, 0x0, 0xf, 0xe1

    ; SBB rb, eb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b sbb_rb_eb, al, byte [bb], 0xf, 0xf, 0xf1

    ; SBB rb, eb (mod 0x0 with constant DISP, reg is AL,...BH, result is an overflow)
    rep_macro_5b sbb_rb_eb, al, byte [bb], 0x0, 0xf, 0xe1

    ; SBB rb, eb (mod 0x1 with constant signed DISP, reg is AL,...BH, result is an overflow)
    rep_macro_5b sbb_rb_eb, al, byte [bp + si - 9], 0x0, 0xf, 0xe1

    ; SBB ew, rw
 ; sbb_ew_rw %1, %2 (sbb %3 - %4 and test that it equals %5)
%macro sbb_ew_rw 5
    mov %1, %3
%if %1 == ax
    mov bx, %4
    sub bx, %1
    sbb bx, %1
    mov ax, bx
%else
    mov %2, %4
    sub %2, %1
    sbb %2, %1
    mov ax, %2
%endif
    mov bx, %5
%endmacro
    ; SBB ew, rw (mod 0x3, reg is AX,CX,DX,BX,SP,BP,SI,DI)
    rep_macro_5w sbb_ew_rw, ax, ax, 0xfff, 0xfff, 0xf001

    ; SBB ew, rw (mod 0x3, reg is AX..., result is an overflow)
    rep_macro_5w sbb_ew_rw, ax, ax, 0xfff, 0x0, 0xe001

    ; SBB ew, rw (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w sbb_ew_rw, ax, word [bb], 0xfff, 0xfff, 0xf001

    ; SBB ew, rw (mod 0x0 with constant DISP, reg is AX..., result is an overflow)
    rep_macro_5w sbb_ew_rw, ax, word [bb], 0xfff, 0x0, 0xe001

    ; encodes to nop, nop
sbb_rw_ew_b db 0x90, 0x90
sbb_rw_ew_w db 0x90, 0x90, 0x90, 0x90

    ; SBB ew, rw (mod 0x1 with constant signed DISP, reg is AX..., result is an overflow)
    mov bp, sbb_rw_ew_b
    mov si, 9
    rep_macro_5w_no_bp_si sbb_ew_rw, ax, word [bp + si - 9], 0xfff, 0x0, 0xe001

    ; SBB rw, ew
 ; sbb_rw_ew %1, %2 (sbb %3 - %4 and test that it equals %5)
%macro sbb_rw_ew 5
    mov bp, sbb_rw_ew_w
    mov si, 9
%if %1 == si
    ; if si is the register we test, we actually need to adjust
    ; bp so that when it adds the initial value that it points to
    ; the right place
    sub bp, si
    sub bp, %3
%endif
%if %1 == bp
    ; ditto when bp is used as the destination
    sub si, bp
    sub si, %3
%endif
    mov %1, %3
    mov %2, %4
%if %1 == bx
    mov cx, %3
    sub cx, %2
%else
    mov bx, %3
    sub bx, %2
%endif
%if %1 == ax
    mov ax, %3
    mov bx, %4
    sbb ax, bx
%else
    sbb %1, %2
    mov ax, %1
%endif
    mov bx, %5
%endmacro
    ; these tests isolate the result from the sub borrow
    ; so the final answer is just %3 - %4 tested against %5
    ; SBB rw, ew (mod 0x3, reg is AX...)
    rep_macro_5w sbb_rw_ew, ax, ax, 0xf, 0xf, 0x0

    ; SBB rw, ew (mod 0x3, reg is AX..., result is an overflow)
    rep_macro_5w sbb_rw_ew, ax, ax, 0x0, 0xfff, 0xf000

    ; SBB rw, ew (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w sbb_rw_ew, ax, word [bb], 0xf, 0xf, 0x0

    ; SBB rw, ew (mod 0x0 with constant DISP, reg is AX..., result is an overflow)
    rep_macro_5w sbb_rw_ew, ax, word [bb], 0x0, 0xfff, 0xf000

    ; SBB rw, ew (mod 0x1 with constant signed DISP, reg is AX..., result is an overflow)
    rep_macro_5w_no_bp_si sbb_rw_ew, ax, word [bp + si - 9], 0x0, 0xfff, 0xf000

    ; SBB AL, db
    mov al, 0x1
    sub al, 0x1
    sbb al, 0x1
    mov ah, 0xff
    int 0x22
    ; SBB AL, db with carry
    mov al, 0x0
    sub al, 0x1
    sbb al, 0x1
    mov ah, 0xfd
    int 0x22

    ; SBB AX, dw
    mov ax, 0xfff
    sub ax, 0x1
    sbb ax, 0xffe
    mov bx, 0x0
    int 0x23
    ; SBB AX, dw with carry
    mov ax, 0x0
    sub ax, 0x1
    sbb ax, 0xffe
    mov bx, 0xf000
    int 0x23

    ; SBB eb, db (mod 0x3, eb is a register AL,CL...BH)
%macro sbb_eb_db 4
    mov %1, %3
    sub %1, %2
    sbb %1, %2
    mov al, %1
    mov ah, %4
%endmacro
    rep_macro_4b sbb_eb_db, al, 0xf, 0xf, 0xf1
    ; with carry
    rep_macro_4b sbb_eb_db, al, 0xf, 0x0, 0xe1

    ; SBB eb, db (mod 0x0 with constant DISP)
    mov byte [bb], 0x1
    sub byte [bb], 0x1
    sbb byte [bb], 0x1
    mov al, byte [bb]
    mov ah, 0xff
    int 0x22
    ; SBB eb, db (mod 0x0 with constant DISP) with carry
    mov byte [bb], 0x0
    sub byte [bb], 0x1
    sbb byte [bb], 0x1
    mov al, byte [bb]
    mov ah, 0xfd
    int 0x22

    ; SBB ew, dw (mod 0x3, ew is a register AX,CL...BP)
%macro sbb_ew_dw 4
    mov %1, %3 ; ax = 0xffff (showing the with carry case)
    sub %1, %2 ; ax = ax + 0xfff = 0xffff + 0xfff = 0x0ffe (CF=1)
    sbb %1, %2 ; ax = ax + CF + 0xfff = 0xffe + 1 + 0xfff = 0x1ffe (CF=1)
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w sbb_ew_dw, ax, 0xfff, 0xfff, 0xf001
    ; with carry
    rep_macro_4w sbb_ew_dw, ax, 0xfff, 0x0, 0xe001

    ; SBB ew, dw (mod 0x0 with constant DISP)
    mov word [bb], 0xfff
    sub word [bb], 0x1   ; [bb] = 0xfff - 0x1 = 0xffe (CF=0)
    sbb word [bb], 0xffe ; [bb] = 0xffe - 0xffe = 0x0 (CF=0)
    mov ax, word [bb]
    mov bx, 0x0
    int 0x23
    ; SBB ew, dw (mod 0x0 with constant DISP) with carry
    mov word [bb], 0x0
    sub word [bb], 0x1   ; [bb] = 0x0 - 0x1 = 0xffff (CF=1)
    sbb word [bb], 0xffe ; [bb] = 0xffff - 0xffe - 1 = 0xf000 (CF=0)
    mov ax, word [bb]
    mov bx, 0xf000
    int 0x23
    ; SBB ew, dw (mod 0x1 with constant DISP)
    mov bp, bb
    mov si, 9
    mov word [bp + si - 9], 0xfff
    sub word [bp + si - 9], 0x1   ; [bb] = 0xfff - 0x1 = 0xffe (CF=0)
    sbb word [bp + si - 9], 0xffe ; [bb] = 0xffe - 0xffe = 0x0 (CF=0)
    mov ax, word [bb]
    mov bx, 0x0
    int 0x23
    ; SBB ew, dw (mod 0x1 with constant DISP) with carry
    mov bp, bb
    mov si, 9
    mov word [bp + si - 9], 0x0
    sub word [bp + si - 9], 0x1   ; [bb] = 0x0 - 0x1 = 0xffff (CF=1)
    sbb word [bp + si - 9], 0xffe ; [bb] = 0xffff - 0xffe - 1 = 0xf000 (CF=0)
    mov ax, word [bb]
    mov bx, 0xf000
    int 0x23

    ; SBB ew, db (mod 0x3, ew is a register AX,CL...BP)
%macro sbb_ew_db 4
    mov %1, %3 ; reg = 0xff                       ; reg = 0xffff (showing with carry)
    sub %1, %2 ; reg = reg + 0xf = 0x10e (CF=0)   ; reg = reg + 0xf = 0x000e (CF=1)
    sbb %1, %2 ; reg = 0x10e + 0xf = 0x11d (CF=0) ; reg = 0xe + 1 + 0xf = 0x001e (CF=0)
    mov ax, %1 ; ax = given answer
    mov bx, %4 ; bx = expected answer
%endmacro
    rep_macro_4w sbb_ew_db, ax, 0xfff, 0xffd, 0xeffe
    ; with carry
    rep_macro_4w sbb_ew_db, ax, 0xffff, 0xfff0, 0xfff1

    ; SBB ew, db (mod 0x0 with constant DISP)
    mov word [bb], 0xfff
    sub word [bb], 0xffd ; 0x2
    sbb word [bb], 0xffff ; 0x3
    mov ax, word [bb]
    mov bx, 0x3
    int 0x23
    ; SBB ew, db (mod 0x0 with constant DISP) with carry
    mov word [bb], 0xfff0
    sub word [bb], 0xffff ; 0xfff1 (CF=1)
    sbb word [bb], 0xffff ; 0xfff1 - -1 - 1 = 0xfff1
    mov ax, word [bb]
    mov bx, 0xfff1
    int 0x23
    ; SBB ew, db (mod 0x1 with constant DISP) with carry
    mov bp, bb
    mov si, 9
    mov word [bp + si - 9], 0xfff0
    sub word [bp + si - 9], 0xffff ; 0xfff1 (CF=1)
    sbb word [bp + si - 9], 0xffff ; 0xfff1 - -1 - 1 = 0xfff1
    mov ax, word [bb]
    mov bx, 0xfff1
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
bb db 0x0, 0x0
