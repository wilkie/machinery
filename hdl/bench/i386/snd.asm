; SND: CHK's checksum (N passes) while a Sound Blaster at 220h (IRQ 5,
; DMA 1) plays a 4 KB buffer, auto-initialised, at 22 kHz in blocks of
; 2 KB. The interrupts it took are left in EDI; it counts them on a page
; of its own, as MIX keeps its data.
        org 100h
%include "common.inc"
BUFLEN  equ 4096
SB      equ 220h
start:  mov dword [irqs], 0
        mov ax, 350Dh                   ; IRQ 5's vector, kept
        int 21h
        mov [old_isr], bx
        mov [old_isr + 2], es
        mov dx, isr
        mov ax, 250Dh
        int 21h
        ; The buffer's physical address, moved up to the next 64 KB page
        ; if it would cross one (DMA cannot).
        xor eax, eax
        mov ax, ds
        shl eax, 4
        mov ebx, eax
        add ebx, area
        mov ecx, ebx
        and ecx, 0FFFFh
        cmp ecx, 10000h - BUFLEN
        jbe .placed
        or ebx, 0FFFFh
        inc ebx
.placed: mov edi, ebx
        sub edi, eax                    ; its offset here
        mov cx, BUFLEN                  ; a ramp
        xor al, al
.fill:  mov [di], al
        inc al
        inc di
        loop .fill
        mov al, 05h                     ; DMA 1: masked, then programmed
        out 0Ah, al
        out 0Ch, al
        mov al, 59h                     ; single, auto-init, read memory, ch 1
        out 0Bh, al
        mov al, bl
        out 02h, al
        mov al, bh
        out 02h, al
        mov al, (BUFLEN - 1) & 0FFh
        out 03h, al
        mov al, (BUFLEN - 1) >> 8
        out 03h, al
        shr ebx, 16
        mov al, bl
        out 83h, al
        mov al, 01h
        out 0Ah, al
        in al, 21h                      ; IRQ 5 unmasked
        and al, 0DFh
        out 21h, al
        call reset
        mov al, 40h                     ; time constant: 256 - 45 (22.2 kHz)
        call dsp
        mov al, 211
        call dsp
        mov al, 48h                     ; blocks of 2 KB
        call dsp
        mov al, (BUFLEN / 2 - 1) & 0FFh
        call dsp
        mov al, (BUFLEN / 2 - 1) >> 8
        call dsp
        mov al, 0D1h
        call dsp
        mov al, 1Ch                     ; auto-init output
        call dsp
        ; The measured part as CHK lays it out (its loop and its reads
        ; aligned the same: the 386's prefetch and bus see the same code).
        times (0 - ($ - $$)) & 15 nop
        MARK 0BEE0h
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
        call reset                      ; output stops
        mov al, 05h
        out 0Ah, al
        in al, 21h
        or al, 20h
        out 21h, al
        push ds
        lds dx, [old_isr]
        mov ax, 250Dh
        int 21h
        pop ds
        mov edi, [irqs]
        FINISH

reset:  mov dx, SB + 6
        mov al, 1
        out dx, al
        xor al, al
        out dx, al
        mov dx, SB + 0Eh
.rdy:   in al, dx
        test al, 80h
        jz .rdy
        mov dx, SB + 0Ah
        in al, dx
        ret

dsp:    mov dx, SB + 0Ch
        push ax
.busy:  in al, dx
        test al, 80h
        jnz .busy
        pop ax
        out dx, al
        ret

isr:    push ax
        push dx
        mov dx, SB + 0Eh
        in al, dx
        inc dword [cs:irqs]
        mov al, 20h
        out 20h, al
        pop dx
        pop ax
        iret

old_isr: dd 0
result: dw 0
        times (2 - ($ - $$)) & 3 db 0
buf:                                    ; 32 KB checksummed (as CHK: what
area    equ buf + 8000h                 ; memory holds), then the sound,
irqs    equ area + 2 * BUFLEN           ; and the count, on a page of its
                                        ; own (beside code, its writes would
                                        ; make tools that watch code by page
                                        ; retranslate it)
