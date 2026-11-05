; CHK: a checksum over 32 KB, N passes (real mode, 16-bit).
        org 100h
%include "common.inc"
start:  MARK 0BEE0h
        mov ebp, N
        xor ax, ax
        xor bx, bx
pass:   test ebp, ebp
        jz done
        mov si, buf
        mov cx, 4000h
.l:     mov dx, [si]
        add ax, dx
        xor bx, ax
        rol bx, 1
        add si, 2
        dec cx
        jnz .l
        dec ebp
        jmp pass
done:   mov [result], bx
        FINISH
result: dw 0
buf:
