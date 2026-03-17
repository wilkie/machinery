; iopl.asm - Tests IOPL checks on IN/OUT instructions
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; GDT:
;   0 (0x00): Null
;   1 (0x08): Code DPL=0, exec/read, base=0, limit=0xFFFF
;   2 (0x10): Data DPL=0, r/w, base=0, limit=0xFFFF (flat)
;   3 (0x18): Stack DPL=0, r/w, base=0, limit=0xFFFF
;   4 (0x20): TSS, base=0x2000, limit=0x2B
;   5 (0x28): Code DPL=3, exec/read, base=0, limit=0xFFFF
;   6 (0x30): Data DPL=3, r/w, base=0, limit=0xFFFF
;   7 (0x38): Stack DPL=3, r/w, base=0, limit=0xFFFF
;
; IDT at 0x0800:
;   Vector 0x0D (#GP): handler (DPL=0, hardware exception)
;   Vector 0x32: change IOPL handler (DPL=3, software INT from ring 3)

section .text
start:

    xor ax, ax
    mov ds, ax

    ; Entry 0: Null
    mov word [0x00], 0x0000
    mov word [0x02], 0x0000
    mov word [0x04], 0x0000
    mov word [0x06], 0x0000

    ; Entry 1: Code DPL=0 (0x08)
    mov word [0x08], 0xFFFF
    mov word [0x0A], 0x0000
    mov byte [0x0C], 0x00
    mov byte [0x0D], 0x9A
    mov word [0x0E], 0x0000

    ; Entry 2: Data DPL=0 (0x10) - flat
    mov word [0x10], 0xFFFF
    mov word [0x12], 0x0000
    mov byte [0x14], 0x00
    mov byte [0x15], 0x92
    mov word [0x16], 0x0000

    ; Entry 3: Stack DPL=0 (0x18)
    mov word [0x18], 0xFFFF
    mov word [0x1A], 0x0000
    mov byte [0x1C], 0x00
    mov byte [0x1D], 0x92
    mov word [0x1E], 0x0000

    ; Entry 4: TSS (0x20) - base=0x2000, limit=0x2B
    mov word [0x20], 0x002B
    mov word [0x22], 0x2000
    mov byte [0x24], 0x00
    mov byte [0x25], 0x81
    mov word [0x26], 0x0000

    ; Entry 5: Code DPL=3 (0x28)
    mov word [0x28], 0xFFFF
    mov word [0x2A], 0x0000
    mov byte [0x2C], 0x00
    mov byte [0x2D], 0xFA
    mov word [0x2E], 0x0000

    ; Entry 6: Data DPL=3 (0x30)
    mov word [0x30], 0xFFFF
    mov word [0x32], 0x0000
    mov byte [0x34], 0x00
    mov byte [0x35], 0xF2
    mov word [0x36], 0x0000

    ; Entry 7: Stack DPL=3 (0x38)
    mov word [0x38], 0xFFFF
    mov word [0x3A], 0x0000
    mov byte [0x3C], 0x00
    mov byte [0x3D], 0xF2
    mov word [0x3E], 0x0000

    ; IDT: #GP (0x0D) at 0x0800 + 0x0D*8 = 0x0868
    mov word [0x0868], gp_handler
    mov word [0x086A], 0x0008
    mov byte [0x086C], 0x00
    mov byte [0x086D], 0x86         ; P=1, DPL=0, interrupt gate
    mov word [0x086E], 0x0000

    ; IDT: Vector 0x32 at 0x0800 + 0x32*8 = 0x0990
    ; DPL=3 so software INT from ring 3 is allowed
    mov word [0x0990], change_iopl_handler
    mov word [0x0992], 0x0008
    mov byte [0x0994], 0x00
    mov byte [0x0995], 0xE6         ; P=1, DPL=3, type=0110 (interrupt gate)
    mov word [0x0996], 0x0000

    ; Initialize TSS at 0x2000
    mov word [0x2000 + 2],  0x1000  ; SP0
    mov word [0x2000 + 4],  0x0018  ; SS0
    mov word [0x2000 + 42], 0x0000  ; LDTR

    ; Load GDT/IDT, enter protected mode
    push cs
    pop ds
    lgdt [gdt_ptr]
    lidt [idt_ptr]

    mov ax, 0x0001
    lmsw ax
    jmp 0x08:protected_entry

protected_entry:
    mov ax, 0x10
    mov ds, ax
    mov ax, 0x18
    mov ss, ax
    mov sp, 0x1000

    ; Load Task Register
    mov ax, 0x20
    ltr ax

    ; =========================================================
    ; Test 1: IN at ring 0, IOPL=0 — succeeds (CPL=0 <= IOPL=0)
    ; =========================================================
    in al, 0x80
    mov ax, 0xAAAA
    mov bx, 0xAAAA
    int 0x23

    ; =========================================================
    ; Test 2: OUT at ring 0, IOPL=0 — succeeds
    ; =========================================================
    mov al, 0x00
    out 0x80, al
    mov ax, 0xBBBB
    mov bx, 0xBBBB
    int 0x23

    ; =========================================================
    ; Set IOPL=3, then drop to ring 3
    ; RETF does NOT pop FLAGS — must set IOPL before RETF
    ; =========================================================
    pushf
    pop ax
    or ax, 0x3000              ; set IOPL=3
    push ax
    popf                       ; IOPL is now 3 in FLAGS register

    push word 0x003B           ; SS3
    push word 0x3000           ; SP3
    push word 0x002B           ; CS3
    push word ring3_iopl3      ; IP3
    retf

ring3_iopl3:
    ; Load ring 3 data segment
    mov ax, 0x0033
    mov ds, ax

    ; =========================================================
    ; Test 3: IN at ring 3, IOPL=3 — succeeds (CPL=3 <= IOPL=3)
    ; =========================================================
    mov byte [exception_vector], 0
    in al, 0x80
    mov al, byte [exception_vector]
    mov ah, 0x00               ; no exception
    int 0x22

    ; =========================================================
    ; Test 4: OUT at ring 3, IOPL=3 — succeeds
    ; =========================================================
    mov byte [exception_vector], 0
    mov al, 0x00
    out 0x80, al
    mov al, byte [exception_vector]
    mov ah, 0x00
    int 0x22

    ; =========================================================
    ; Change IOPL to 0 via INT 0x32
    ; Handler modifies saved FLAGS on stack and IRETs back
    ; =========================================================
    int 0x32

    ; =========================================================
    ; Test 5: IN at ring 3, IOPL=0 — #GP (CPL=3 > IOPL=0)
    ; =========================================================
    mov byte [exception_vector], 0
    in al, 0x80

    mov al, byte [exception_vector]
    mov ah, 0x0D
    int 0x22

    ; =========================================================
    ; Test 6: OUT at ring 3, IOPL=0 — #GP
    ; =========================================================
    mov byte [exception_vector], 0
    mov al, 0x00
    out 0x80, al

    mov al, byte [exception_vector]
    mov ah, 0x0D
    int 0x22

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

; =========================================================
; #GP handler — works for both ring 0 and ring 3 exceptions
; Ring 3: stack has error_code, IP, CS(RPL=3), FLAGS, SP3, SS3
; Ring 0: stack has error_code, IP, CS, FLAGS
; =========================================================
gp_handler:
    push ax
    push bx
    mov ax, 0x10
    mov ds, ax
    mov bx, sp
    mov ax, word [ss:bx+4]
    mov word [exception_error], ax
    mov byte [exception_vector], 0x0D
    pop bx
    pop ax
    ; Fault IP points to faulting instruction; skip past it (2 bytes)
    push bp
    mov bp, sp
    add word [ss:bp+4], 2     ; adjust IP past faulting instruction
    pop bp
    add sp, 2                  ; skip error code
    iret

; =========================================================
; INT 0x32 handler — clears IOPL in saved FLAGS
; Privilege transition stack: [SP]=IP, [SP+2]=CS, [SP+4]=FLAGS,
;                             [SP+6]=SP3, [SP+8]=SS3
; =========================================================
change_iopl_handler:
    push bp
    mov bp, sp
    ; [BP+2] = ret IP, [BP+4] = ret CS, [BP+6] = ret FLAGS
    mov ax, [bp+6]
    and ax, 0xCFFF             ; clear IOPL bits (bits 12-13)
    mov [bp+6], ax
    pop bp
    iret

section .data

gdt_ptr:
    dw 0x3F                     ; limit: 8 entries * 8 - 1
    dw 0x0000
    db 0x00

idt_ptr:
    dw 0x19F                    ; covers up to vector 0x33
    dw 0x0800
    db 0x00

exception_vector: db 0
exception_error:  dw 0
