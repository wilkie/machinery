; MIX: a mixed integer workload, N passes: 32-bit arithmetic, calls and
; the stack, multiply and divide, string moves and scans, and a sieve.
        org 100h
%include "common.inc"
start:  MARK 0BEE0h
        mov ebp, N
        xor edi, edi            ; running result
pass:   test ebp, ebp
        jz done
        ; 32-bit arithmetic through calls.
        mov ecx, 400
        mov eax, 12345678h
.a:     call step32
        loop .a
        add edi, eax
        ; Strings: move 1 KB, fill, scan.
        push edi
        mov si, src
        mov di, dst
        mov cx, 512
        cld
        rep movsw
        mov di, dst
        mov al, 5Ah
        mov cx, 1024
        rep stosb
        mov di, dst
        mov al, 0A5h
        mov cx, 1024
        repne scasb
        pop edi
        add edi, ecx
        ; A sieve over 2048 bytes.
        mov di, sieve
        mov al, 1
        mov cx, 2048
        rep stosb
        mov bx, 2
.s:     cmp byte [sieve + bx], 0
        je .n
        mov si, bx
        add si, bx
.m:     cmp si, 2048
        jae .n
        mov byte [sieve + si], 0
        add si, bx
        jmp .m
.n:     inc bx
        cmp bx, 46
        jb .s
        dec ebp
        jmp pass
done:   mov [result], edi
        FINISH

step32: push ebx
        mov ebx, eax
        imul eax, eax, 1103515245
        add eax, 12345
        xor eax, ebx
        mov edx, 0
        mov ebx, 1000003
        div ebx
        mov eax, edx
        shl eax, 3
        add eax, ecx
        pop ebx
        ret

result: dd 0
; The data on pages of its own: writes beside code would make tools that
; watch code by page retranslate it (as no real hot loop does).
        times (4096 - (($ - $$ + 100h) % 4096)) db 0
src:    times 1024 db 3
dst:    times 1024 db 0
sieve:  times 2048 db 0
