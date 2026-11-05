; Shared by the benchmark programs: markers and the tools' exits.
; 0000:04F0 holds 0BEE0h when the measured part starts, 0BEEFh when it ends.
%macro MARK 1
        push es
        xor ax, ax
        mov es, ax
        mov word [es:04F0h], %1
        pop es
%endmacro

%macro FINISH 0
        MARK 0BEEFh
        ; QEMU: isa-debug-exit at port 0F4h.
        mov dx, 0F4h
        xor al, al
        out dx, al
        ; Bochs: "Shutdown" to port 8900h.
        mov dx, 8900h
        mov si, shutdown
        mov cx, 8
        cld
        rep outsb
        ; DOS (DOSBox, and our board: the marker says done).
        mov ax, 4C00h
        int 21h
shutdown: db "Shutdown"
%endmacro
