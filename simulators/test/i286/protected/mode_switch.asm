; mode_switch.asm - Tests entering protected mode via LMSW and far JMP
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

section .text
start:

    ; =========================================================
    ; Set up GDT at linear address 0 (base is always 0 for now).
    ; Use DS=0 to write to linear address 0.
    ;
    ; GDT layout (at linear address 0):
    ;   Entry 0 (0x00): Null descriptor (required)
    ;   Entry 1 (0x08): Code segment - base=0, limit=0xFFFF, exec/read
    ;   Entry 2 (0x10): Data segment - base=0, limit=0xFFFF, read/write
    ;   Entry 3 (0x18): Stack segment - base=0, limit=0xFFFF, read/write
    ; =========================================================

    xor ax, ax
    mov ds, ax

    ; Entry 0: Null descriptor
    mov word [0x00], 0x0000
    mov word [0x02], 0x0000
    mov word [0x04], 0x0000
    mov word [0x06], 0x0000

    ; Entry 1: Code segment (selector 0x08)
    ; Access byte 0x9A: P=1, DPL=00, S=1, Type=1010 (code, exec/read)
    mov word [0x08], 0xFFFF
    mov word [0x0A], 0x0000
    mov byte [0x0C], 0x00
    mov byte [0x0D], 0x9A
    mov word [0x0E], 0x0000

    ; Entry 2: Data segment (selector 0x10)
    ; Access byte 0x92: P=1, DPL=00, S=1, Type=0010 (data, read/write)
    mov word [0x10], 0xFFFF
    mov word [0x12], 0x0000
    mov byte [0x14], 0x00
    mov byte [0x15], 0x92
    mov word [0x16], 0x0000

    ; Entry 3: Stack segment (selector 0x18)
    ; Access byte 0x96: P=1, DPL=00, S=1, Type=0110 (data, read/write, expand-down)
    mov word [0x18], 0xFFFF
    mov word [0x1A], 0x0000
    mov byte [0x1C], 0x00
    mov byte [0x1D], 0x92      ; 0x92 for normal stack (non expand-down)
    mov word [0x1E], 0x0000

    ; =========================================================
    ; Load GDT register
    ; =========================================================
    push cs
    pop ds                      ; Restore DS for data access
    lgdt [gdt_ptr]

    ; =========================================================
    ; Verify SMSW shows PE=0 before switch
    ; =========================================================
    smsw ax
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; Switch to protected mode
    ; =========================================================
    mov ax, 0x0001
    lmsw ax

    ; =========================================================
    ; Verify PE is set
    ; =========================================================
    smsw ax
    mov bx, 0x0001
    int 0x23

    ; =========================================================
    ; Far JMP to flush prefetch and load CS with code selector
    ; =========================================================
    jmp 0x08:protected_entry

protected_entry:
    ; =========================================================
    ; Verify we're still running (far JMP didn't crash)
    ; =========================================================
    mov ax, 0xAAAA
    mov bx, 0xAAAA
    int 0x23

    ; =========================================================
    ; Load data segment (selector 0x10)
    ; =========================================================
    mov ax, 0x10
    mov ds, ax

    ; =========================================================
    ; Load stack segment (selector 0x18) with valid SP
    ; =========================================================
    mov ax, 0x18
    mov ss, ax
    mov sp, 0x1000

    ; =========================================================
    ; Test: basic register ops in protected mode
    ; =========================================================
    mov ax, 0x1234
    mov bx, 0x1234
    int 0x23

    mov al, 0x42
    mov ah, 0x42
    int 0x22

    ; =========================================================
    ; Test: memory access through data segment
    ; =========================================================
    mov word [test_data], 0xBEEF
    mov ax, word [test_data]
    mov bx, 0xBEEF
    int 0x23

    ; =========================================================
    ; Test: push/pop through stack segment
    ; =========================================================
    mov ax, 0x5678
    push ax
    pop bx
    mov ax, 0x5678
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

section .data

gdt_ptr:
    dw 0x1F                     ; limit: 4 entries * 8 - 1
    dw 0x0000                   ; base low
    db 0x00                     ; base high

test_data: dw 0
