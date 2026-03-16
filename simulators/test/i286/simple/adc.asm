; adc.asm
bits 16
org 0x100                ; Set the origin to 0x100, the start of a .COM file

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%include "macros.inc"

section .text
start:
    ; ADC eb, rb
 ; adc_eb_rb %1, %2 (add %3 + %4 and adc %3 + %4 and test that it equals %5)
%macro adc_eb_rb 5
    mov %1, %3
%if %1 == al
    mov ah, %4
    add ah, %1
    adc ah, %1
%else
    mov %2, %4
    add %2, %1
    adc %2, %1
    mov ah, %2
%endif
    mov al, %5
%endmacro
    ; ADC eb, rb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b adc_eb_rb, al, al, 0xf, 0xf, 0x2d

    ; ADC eb, rb (mod 0x3, reg is AL,...BH, result is an overflow)
    rep_macro_5b adc_eb_rb, al, al, 0xff, 0xff, 0xfe

    ; ADC eb, rb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b adc_eb_rb, al, byte [bb], 0xf, 0xf, 0x2d

    ; ADC eb, rb (mod 0x0 with constant DISP, reg is AL,...BH, result is an overflow)
    rep_macro_5b adc_eb_rb, al, byte [bb], 0xff, 0xff, 0xfe

    ; encodes to nop, nop
adc_rb_eb_b db 0x90, 0x90

    ; ADC eb, rb (mod 0x1 with constant signed DISP, reg is AL,...BH, result is an overflow)
    mov bp, adc_rb_eb_b
    mov si, 9
    rep_macro_5b adc_eb_rb, al, byte [bp + si - 9], 0xff, 0xff, 0xfe

    ; ADC rb, eb
 ; adc_rb_eb %1, %2 (adc %3 + %4 and test that it equals %5)
%macro adc_rb_eb 5
    mov bp, adc_rb_eb_b
    mov si, 9
    mov %1, %3
%if %1 == al
    mov ah, %4
    add %1, ah
    adc %1, ah
%else
    mov %2, %4
    add %1, %2
    adc %1, %2
%endif
    mov ah, %1
    mov al, %5
%endmacro
    ; ADC rb, eb (mod 0x3, reg is AL,CL,DL,BL,AH,CH,DH,BH)
    rep_macro_5b adc_rb_eb, al, al, 0xf, 0xf, 0x2d

    ; ADC rb, eb (mod 0x3, reg is AL,...BH, result is an overflow)
    rep_macro_5b adc_rb_eb, al, al, 0xff, 0xff, 0xfe

    ; ADC rb, eb (mod 0x0 with constant DISP, reg is AL,...BH)
    rep_macro_5b adc_rb_eb, al, byte [bb], 0xf, 0xf, 0x2d

    ; ADC rb, eb (mod 0x0 with constant DISP, reg is AL,...BH, result is an overflow)
    rep_macro_5b adc_rb_eb, al, byte [bb], 0xff, 0xff, 0xfe

    ; ADC rb, eb (mod 0x1 with constant signed DISP, reg is AL,...BH, result is an overflow)
    rep_macro_5b adc_rb_eb, al, byte [bp + si - 9], 0xff, 0xff, 0xfe

    ; ADC ew, rw
 ; adc_ew_rw %1, %2 (adc %3 + %4 and test that it equals %5)
%macro adc_ew_rw 5
    mov %1, %3
%if %1 == ax
    mov bx, %4
    add bx, %1
    adc bx, %1
    mov ax, bx
%else
    mov %2, %4
    add %2, %1
    adc %2, %1
    mov ax, %2
%endif
    mov bx, %5
%endmacro
    ; ADC ew, rw (mod 0x3, reg is AX,CX,DX,BX,SP,BP,SI,DI)
    rep_macro_5w adc_ew_rw, ax, ax, 0xf, 0xf, 0x2d

    ; ADC ew, rw (mod 0x3, reg is AX..., result is an overflow)
    rep_macro_5w adc_ew_rw, ax, ax, 0xffff, 0xffff, 0xfffe

    ; ADC ew, rw (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w adc_ew_rw, ax, word [bb], 0xf, 0xf, 0x2d

    ; ADC ew, rw (mod 0x0 with constant DISP, reg is AX..., result is an overflow)
    rep_macro_5w adc_ew_rw, ax, word [bb], 0xffff, 0xffff, 0xfffe

    ; encodes to nop, nop
adc_rw_ew_b db 0x90, 0x90
adc_rw_ew_w db 0x90, 0x90, 0x90, 0x90

    ; ADC ew, rw (mod 0x1 with constant signed DISP, reg is AX,...DI, result is an overflow)
    mov bp, adc_rw_ew_b
    mov si, 9
    ; [add] -2 + -2 = -4 (C = 1)
    ; [adc] -4 + -2 + 1 = -5
    ; Use no_bp_si variant because [bp + si - 9] addressing clobbers bp/si
    rep_macro_5w_no_bp_si adc_ew_rw, ax, word [bp + si - 9], 0xfffe, 0xfffe, 0xfffb

    ; ADC rw, ew
 ; adc_rw_ew %1, %2 (adc %3 + %4 and test that it equals %5)
%macro adc_rw_ew 5
    mov bp, adc_rw_ew_w
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
    mov %2, %4
%if %1 == bx
    mov cx, %3
    add cx, %2
%else
    mov bx, %3
    add bx, %2
%endif
    adc %1, %2
    mov ax, %1
    mov bx, %5
%endmacro
    ; ADC rw, ew (mod 0x3, reg is AX...)
    rep_macro_5w adc_rw_ew, ax, ax, 0xf, 0xf, 0x1e

    ; ADC rw, ew (mod 0x3, reg is AX..., result is an overflow)
    rep_macro_5w adc_rw_ew, ax, ax, 0xffff, 0xffff, 0xffff

    ; ADC rw, ew (mod 0x0 with constant DISP, reg is AX...)
    rep_macro_5w adc_rw_ew, ax, word [bb], 0xf, 0xf, 0x1e

    ; ADC rw, ew (mod 0x0 with constant DISP, reg is AX..., result is an overflow)
    rep_macro_5w adc_rw_ew, ax, word [bb], 0xffff, 0xffff, 0xffff

    ; ADC rw, ew (mod 0x1 with constant signed DISP, reg is AX,...DI, result is an overflow)
    ; Use no_bp_si variant because [bp + si - 9] addressing clobbers bp/si
    rep_macro_5w_no_bp_si adc_rw_ew, ax, word [bp + si - 9], 0xffff, 0xffff, 0xffff

    ; ADC AL, db
    mov al, 0x0
    add al, 0x1
    adc al, 0x1
    mov ah, 0x2
    int 0x22
    ; ADC AL, db with carry
    mov al, 0x1
    add al, 0xff
    adc al, 0x1
    mov ah, 0x2
    int 0x22

    ; ADC AX, dw
    mov ax, 0x0
    add ax, 0x1
    adc ax, 0xffe
    mov bx, 0xfff
    int 0x23
    ; ADC AX, dw with carry
    mov ax, 0x1
    add ax, 0xffff
    adc ax, 0xffe
    mov bx, 0xfff
    int 0x23

    ; ADC eb, db (mod 0x3, eb is a register AL,CL...BH)
%macro adc_eb_db 4
    mov %1, %3
    add %1, %2
    adc %1, %2
    mov al, %1
    mov ah, %4
%endmacro
    rep_macro_4b adc_eb_db, al, 0xf, 0xf, 0x2d
    ; with carry
    rep_macro_4b adc_eb_db, al, 0xff, 0xff, 0xfe

    ; ADC eb, db (mod 0x0 with constant DISP)
    mov byte [bb], 0x0
    add byte [bb], 0xf
    adc byte [bb], 0xf
    mov al, byte [bb]
    mov ah, 0x1e
    int 0x22
    ; ADC eb, db (mod 0x0 with constant DISP) with carry
    mov byte [bb], 0xf
    add byte [bb], 0xff
    adc byte [bb], 0xf
    mov al, byte [bb]
    mov ah, 0x1e
    int 0x22

    ; ADC ew, dw (mod 0x3, ew is a register AX,CL...BP)
%macro adc_ew_dw 4
    mov %1, %3 ; ax = 0xffff (showing the with carry case)
    add %1, %2 ; ax = ax + 0xfff = 0xffff + 0xfff = 0x0ffe (CF=1)
    adc %1, %2 ; ax = ax + CF + 0xfff = 0xffe + 1 + 0xfff = 0x1ffe (CF=1)
    mov ax, %1
    mov bx, %4
%endmacro
    rep_macro_4w adc_ew_dw, ax, 0xfff, 0xfff, 0x2ffd
    ; with carry
    rep_macro_4w adc_ew_dw, ax, 0xfff, 0xffff, 0x1ffe

    ; ADC ew, dw (mod 0x0 with constant DISP)
    mov word [bb], 0xfff
    add word [bb], 0xfff ; [bb] = 0xfff + 0xfff = 0x1ffe (CF=0)
    adc word [bb], 0xfff ; [bb] = 0x1ffe + 0xfff = 0x2ffd (CF=0)
    mov ax, word [bb]
    mov bx, 0x2ffd
    int 0x23
    ; ADC ew, dw (mod 0x0 with constant DISP) with carry
    mov word [bb], 0xffff
    add word [bb], 0xfff ; [bb] = 0xffff + 0xfff = 0x0ffe (CF=1)
    adc word [bb], 0xfff ; [bb] = 0xffe + 0xfff + 1 = 0x1ffe (CF=1)
    mov ax, word [bb]
    mov bx, 0x1ffe
    int 0x23
    ; ADC ew, dw (mod 0x1 with constant DISP)
    mov bp, bb
    mov si, 9
    mov word [bp + si - 9], 0xfff
    add word [bp + si - 9], 0xfff ; [bb] = 0xfff + 0xfff = 0x1ffe (CF=0)
    adc word [bp + si - 9], 0xfff ; [bb] = 0x1ffe + 0xfff = 0x2ffd (CF=0)
    mov ax, word [bb]
    mov bx, 0x2ffd
    int 0x23
    ; ADC ew, dw (mod 0x1 with constant DISP) with carry
    mov bp, bb
    mov si, 9
    mov word [bp + si - 9], 0xffff
    add word [bp + si - 9], 0xfff ; [bb] = 0xffff + 0xfff = 0x0ffe (CF=1)
    adc word [bp + si - 9], 0xfff ; [bb] = 0xffe + 0xfff + 1 = 0x1ffe (CF=1)
    mov ax, word [bb]
    mov bx, 0x1ffe
    int 0x23

    ; ADC ew, db (mod 0x3, ew is a register AX,CL...BP)
%macro adc_ew_db 4
    mov %1, %3 ; reg = 0xff                       ; reg = 0xffff (showing with carry)
    add %1, %2 ; reg = reg + 0xf = 0x10e (CF=0)   ; reg = reg + 0xf = 0x000e (CF=1)
    adc %1, %2 ; reg = 0x10e + 0xf = 0x11d (CF=0) ; reg = 0xe + 1 + 0xf = 0x001e (CF=0)
    mov ax, %1 ; ax = given answer
    mov bx, %4 ; bx = expected answer
%endmacro
    rep_macro_4w adc_ew_db, ax, 0xf, 0xff, 0x11d
    ; with carry
    rep_macro_4w adc_ew_db, ax, 0xf, 0xffff, 0x1e

    ; ADC ew, db (mod 0x0 with constant DISP)
    mov word [bb], 0xff
    add word [bb], 0xf  ; 0x10e
    adc word [bb], 0xf  ; 0x11d
    mov ax, word [bb]
    mov bx, 0x11d
    int 0x23
    ; ADC ew, db (mod 0x0 with constant DISP) with carry
    mov word [bb], 0xffff
    add word [bb], 0xf  ; 0xe (CF=1)
    adc word [bb], 0xf  ; 0xe + 1 + 0xf = 0x1e
    mov ax, word [bb]
    mov bx, 0x1e
    int 0x23
    ; ADC ew, db (mod 0x1 with constant DISP) with carry
    mov bp, bb
    mov si, 9
    mov word [bp + si - 9], 0xffff
    add word [bp + si - 9], 0xf  ; 0xe (CF=1)
    adc word [bp + si - 9], 0xf  ; 0xe + 1 + 0xf = 0x1e
    mov ax, word [bb]
    mov bx, 0x1e
    int 0x23

    mov ah, 0x4C         ; DOS terminate program function
    int 0x21             ; Call DOS interrupt to terminate

section .data
bb db 0x0, 0x0
