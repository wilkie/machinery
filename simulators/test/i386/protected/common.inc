; common.inc - shared definitions for the 386 test programs.
;
; The programs are .com images loaded at 0000:0100 with everything in one
; section, so a label's linear address is its offset (ABS makes it a
; plain number nasm can shift).
;
; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx
; int 0x24 - asserts eax == ebx
; int 0x21 with ah = 0x4c - ends the program

%define ABS(x) ((x) - $$ + 0x100)

; A segment descriptor: base, limit (20 bits), access byte, flags nibble
; (G = 8, D/B = 4).
%macro DESC 4
    dw (%2) & 0xFFFF
    dw (%1) & 0xFFFF
    db ((%1) >> 16) & 0xFF
    db %3
    db (((%2) >> 16) & 0x0F) | ((%4) << 4)
    db ((%1) >> 24) & 0xFF
%endmacro

; A gate: selector, offset, access byte (P, DPL, type), parameter count.
%macro GATE 4
    dw (%2) & 0xFFFF
    dw %1
    db %4
    db %3
    dw ((%2) >> 16) & 0xFFFF
%endmacro

; Assert a 32-bit value: eax = %1, ebx = %2.
%macro CHECK32 2
    mov eax, %1
    mov ebx, %2
    int 0x24
%endmacro

; Assert a 16-bit value: ax = %1, bx = %2.
%macro CHECK16 2
    mov ax, %1
    mov bx, %2
    int 0x23
%endmacro

%macro DONE 0
    mov ax, 0x4C00
    int 0x21
%endmacro

; Access bytes.
%define CODE0   0x9A        ; present, DPL 0, code, readable
%define DATA0   0x92        ; present, DPL 0, data, writable
%define CODE3   0xFA
%define DATA3   0xF2
%define TSS386  0x89        ; available 386 TSS
%define INT386  0x8E        ; 386 interrupt gate, DPL 0
%define TRAP386 0x8F        ; 386 trap gate, DPL 0
%define CALL386 0x8C        ; 386 call gate, DPL 0
%define TASKG   0x85        ; task gate, DPL 0
