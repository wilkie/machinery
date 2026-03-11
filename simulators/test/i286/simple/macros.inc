; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

%macro rep_macro_3b 4
    %define current_reg %2
%rep 8
    %1 current_reg, %3, %4
    int 0x22             ; Call test interrupt
%if current_reg == bh
    %define current_reg al
%endif
%if current_reg == dh
    %define current_reg bh
%endif
%if current_reg == ch
    %define current_reg dh
%endif
%if current_reg == ah
    %define current_reg ch
%endif
%if current_reg == bl
    %define current_reg ah
%endif
%if current_reg == dl
    %define current_reg bl
%endif
%if current_reg == cl
    %define current_reg dl
%endif
%if current_reg == al
    %define current_reg cl
%endif
%endrep
%endmacro

%macro rep_macro_3w 4
    %define current_reg %2
%rep 8
    %1 current_reg, %3, %4
    int 0x23             ; Call test interrupt
%if current_reg == di
    %define current_reg ax
%endif
%if current_reg == si
    %define current_reg di
%endif
%if current_reg == bp
    %define current_reg si
%endif
%if current_reg == sp
    %define current_reg bp
%endif
%if current_reg == bx
    %define current_reg sp
%endif
%if current_reg == dx
    %define current_reg bx
%endif
%if current_reg == cx
    %define current_reg dx
%endif
%if current_reg == ax
    %define current_reg cx
%endif
%endrep
%endmacro

%macro rep_macro_4b 5
    %define current_reg %2
%rep 8
    %1 current_reg, %3, %4, %5
    int 0x22             ; Call test interrupt
%if current_reg == bh
    %define current_reg al
%endif
%if current_reg == dh
    %define current_reg bh
%endif
%if current_reg == ch
    %define current_reg dh
%endif
%if current_reg == ah
    %define current_reg ch
%endif
%if current_reg == bl
    %define current_reg ah
%endif
%if current_reg == dl
    %define current_reg bl
%endif
%if current_reg == cl
    %define current_reg dl
%endif
%if current_reg == al
    %define current_reg cl
%endif
%endrep
%endmacro

%macro rep_macro_4w 5
    %define current_reg %2
%rep 8
    %1 current_reg, %3, %4, %5
    int 0x23             ; Call test interrupt
%if current_reg == di
    %define current_reg ax
%endif
%if current_reg == si
    %define current_reg di
%endif
%if current_reg == bp
    %define current_reg si
%endif
%if current_reg == sp
    %define current_reg bp
%endif
%if current_reg == bx
    %define current_reg sp
%endif
%if current_reg == dx
    %define current_reg bx
%endif
%if current_reg == cx
    %define current_reg dx
%endif
%if current_reg == ax
    %define current_reg cx
%endif
%endrep
%endmacro

%macro rep_macro_5b 6
    %define current_reg %2
%rep 8
    %1 current_reg, %3, %4, %5, %6
    int 0x22             ; Call test interrupt
%if current_reg == bh
    %define current_reg al
%endif
%if current_reg == dh
    %define current_reg bh
%endif
%if current_reg == ch
    %define current_reg dh
%endif
%if current_reg == ah
    %define current_reg ch
%endif
%if current_reg == bl
    %define current_reg ah
%endif
%if current_reg == dl
    %define current_reg bl
%endif
%if current_reg == cl
    %define current_reg dl
%endif
%if current_reg == al
    %define current_reg cl
%endif
%endrep
%endmacro

%macro rep_macro_5b_double 4
    %define current_reg %2
%rep 8
    %1 current_reg, current_reg, %3, %3, %4
    int 0x22             ; Call test interrupt
%if current_reg == bh
    %define current_reg al
%endif
%if current_reg == dh
    %define current_reg bh
%endif
%if current_reg == ch
    %define current_reg dh
%endif
%if current_reg == ah
    %define current_reg ch
%endif
%if current_reg == bl
    %define current_reg ah
%endif
%if current_reg == dl
    %define current_reg bl
%endif
%if current_reg == cl
    %define current_reg dl
%endif
%if current_reg == al
    %define current_reg cl
%endif
%endrep
%endmacro

%macro rep_macro_5w_no_bp_si 6
    %define current_reg %2
%rep 8
    %1 current_reg, %3, %4, %5, %6
    int 0x23             ; Call test interrupt
%if current_reg == di
    %define current_reg ax
%endif
%if current_reg == sp
    %define current_reg di
%endif
%if current_reg == bx
    %define current_reg sp
%endif
%if current_reg == dx
    %define current_reg bx
%endif
%if current_reg == cx
    %define current_reg dx
%endif
%if current_reg == ax
    %define current_reg cx
%endif
%endrep
%endmacro

%macro rep_macro_5w 6
    %define current_reg %2
%rep 8
    %1 current_reg, %3, %4, %5, %6
    int 0x23             ; Call test interrupt
%if current_reg == di
    %define current_reg ax
%endif
%if current_reg == si
    %define current_reg di
%endif
%if current_reg == bp
    %define current_reg si
%endif
%if current_reg == sp
    %define current_reg bp
%endif
%if current_reg == bx
    %define current_reg sp
%endif
%if current_reg == dx
    %define current_reg bx
%endif
%if current_reg == cx
    %define current_reg dx
%endif
%if current_reg == ax
    %define current_reg cx
%endif
%endrep
%endmacro

%macro rep_macro_5w_double 4
    %define current_reg %2
%rep 8
    %1 current_reg, current_reg, %3, %3, %4
    int 0x23             ; Call test interrupt
%if current_reg == di
    %define current_reg ax
%endif
%if current_reg == si
    %define current_reg di
%endif
%if current_reg == bp
    %define current_reg si
%endif
%if current_reg == sp
    %define current_reg bp
%endif
%if current_reg == bx
    %define current_reg sp
%endif
%if current_reg == dx
    %define current_reg bx
%endif
%if current_reg == cx
    %define current_reg dx
%endif
%if current_reg == ax
    %define current_reg cx
%endif
%endrep
%endmacro
