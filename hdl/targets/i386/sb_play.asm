        bits 16
        org 0
        cli
        xor ax, ax
        mov ds, ax
        mov ss, ax
        mov sp, 7C00h
        mov word [34h], isr             ; IRQ 5 (vector 0Dh)
        mov word [36h], 0F000h
        mov al, 11h                     ; ICW1..4: vectors from 08h
        out 20h, al
        mov al, 08h
        out 21h, al
        mov al, 04h
        out 21h, al
        mov al, 01h
        out 21h, al
        mov al, 0DFh                    ; IRQ 5 alone
        out 21h, al
        mov di, 1000h                   ; 256 samples: 0, 1, ... 255
        xor al, al
        mov cx, 256
.fill:  mov [di], al
        inc al
        inc di
        loop .fill
        mov al, 07h                     ; DMA 3: masked, then programmed
        out 0Ah, al
        out 0Ch, al
        mov al, 5Bh                     ; single, auto-init, read memory, ch 3
        out 0Bh, al
        xor al, al
        out 06h, al                     ; address 1000h
        mov al, 10h
        out 06h, al
        mov al, 0FFh                    ; count 256 - 1
        out 07h, al
        xor al, al
        out 07h, al
        out 82h, al                     ; page 0
        mov al, 03h
        out 0Ah, al                     ; unmasked
        mov dx, 226h                    ; DSP reset
        mov al, 1
        out dx, al
        xor al, al
        out dx, al
        mov dx, 22Eh
.rdy:   in al, dx
        test al, 80h
        jz .rdy
        mov dx, 22Ah
        in al, dx
        mov [500h], al                  ; 0AAh
        mov al, 40h                     ; time constant: 256 - 50 (20 kHz)
        call dsp
        mov al, 206
        call dsp
        mov al, 48h                     ; blocks of 64
        call dsp
        mov al, 63
        call dsp
        xor al, al
        call dsp
        mov al, 0D1h
        call dsp
        mov al, 1Ch                     ; auto-init output
        call dsp
        sti
.wait:  hlt
        cmp word [502h], 4
        jb .wait
        mov al, 0DAh                    ; the next block is the last
        call dsp
.last:  hlt
        cmp word [502h], 5
        jb .last
        cli
        hlt
dsp:    mov dx, 22Ch
        push ax
.busy:  in al, dx
        test al, 80h
        jnz .busy
        pop ax
        out dx, al
        ret
isr:    push ax
        push dx
        mov dx, 22Eh
        in al, dx
        inc word [502h]
        mov al, 20h
        out 20h, al
        pop dx
        pop ax
        iret
