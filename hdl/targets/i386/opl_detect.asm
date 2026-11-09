        bits 16
        org 0
        cli
        xor ax, ax
        mov ds, ax
        mov dx, 388h
        mov ah, 4                       ; both timers masked, then reset
        mov al, 60h
        call opl
        mov ah, 4
        mov al, 80h
        call opl
        in al, dx
        mov [500h], al                  ; status: 0 (and 6 below)
        mov ah, 2                       ; timer 1 from FFh: 80 us
        mov al, 0FFh
        call opl
        mov ah, 4
        mov al, 21h
        call opl
        mov cx, 200                     ; some 200 us of status reads
.wait:  in al, dx
        loop .wait
        in al, dx
        mov [501h], al                  ; status: C0h
        mov ah, 4
        mov al, 60h
        call opl
        mov ah, 4
        mov al, 80h
        call opl
        in al, dx
        mov [502h], al
        hlt
opl:    xchg al, ah                     ; index (AH) then value (AL)
        out dx, al
        in al, dx                       ; the chip's settling time
        in al, dx
        inc dx
        xchg al, ah
        out dx, al
        dec dx
        mov cx, 35
.d:     in al, dx
        loop .d
        ret
