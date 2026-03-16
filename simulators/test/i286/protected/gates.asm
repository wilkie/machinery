; gates.asm - Tests interrupt gate vs trap gate IF flag behavior
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; GDT:
;   0 (0x00): Null
;   1 (0x08): Code DPL=0, exec/read, base=0, limit=0xFFFF
;   2 (0x10): Data DPL=0, r/w, base=0, limit=0xFFFF (flat)
;   3 (0x18): Stack DPL=0, r/w, base=0, limit=0xFFFF
;
; IDT at 0x0800:
;   Vector 0x30: Interrupt gate (T=0) — clears IF
;   Vector 0x31: Trap gate (T=1) — preserves IF

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

    ; IDT: Vector 0x30 (interrupt gate) at 0x0800 + 0x30*8 = 0x0980
    mov word [0x0980], int_gate_handler
    mov word [0x0982], 0x0008
    mov byte [0x0984], 0x00
    mov byte [0x0985], 0x86         ; P=1, DPL=0, type=0110 (interrupt gate, T=0)
    mov word [0x0986], 0x0000

    ; IDT: Vector 0x31 (trap gate) at 0x0800 + 0x31*8 = 0x0988
    mov word [0x0988], trap_gate_handler
    mov word [0x098A], 0x0008
    mov byte [0x098C], 0x00
    mov byte [0x098D], 0x87         ; P=1, DPL=0, type=0111 (trap gate, T=1)
    mov word [0x098E], 0x0000

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

    ; =========================================================
    ; Test 1: INT through interrupt gate with IF=1
    ; Interrupt gate should clear IF
    ; =========================================================
    sti
    mov word [handler_if], 0xFFFF
    int 0x30

    mov ax, word [handler_if]
    mov bx, 0x0000                  ; expect IF=0 in handler
    int 0x23

    ; =========================================================
    ; Test 2: INT through trap gate with IF=1
    ; Trap gate should preserve IF
    ; =========================================================
    sti
    mov word [handler_if], 0xFFFF
    int 0x31

    mov ax, word [handler_if]
    mov bx, 0x0200                  ; expect IF=1 in handler
    int 0x23

    ; =========================================================
    ; Test 3: INT through interrupt gate with IF=0
    ; IF was 0, interrupt gate still clears it → IF=0
    ; =========================================================
    cli
    mov word [handler_if], 0xFFFF
    int 0x30

    mov ax, word [handler_if]
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; Test 4: INT through trap gate with IF=0
    ; IF was 0, trap gate preserves it → IF=0
    ; =========================================================
    cli
    mov word [handler_if], 0xFFFF
    int 0x31

    mov ax, word [handler_if]
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; Test 5: Verify IF restored after IRET from interrupt gate
    ; STI → IF=1, then INT 0x30 (interrupt gate clears IF)
    ; After IRET, FLAGS restored from stack → IF=1 again
    ; =========================================================
    sti
    int 0x30
    pushf
    pop ax
    and ax, 0x0200
    mov bx, 0x0200                  ; IF should be restored to 1
    int 0x23

    ; =========================================================
    ; Test 6: Verify TF is cleared by interrupt gate
    ; =========================================================
    mov word [handler_tf], 0xFFFF
    ; Set TF via FLAGS manipulation
    pushf
    pop ax
    or ax, 0x0100                   ; set TF
    push ax
    popf
    ; TF is now set; INT 0x30 should clear it
    int 0x30

    mov ax, word [handler_tf]
    mov bx, 0x0000                  ; expect TF=0 in handler
    int 0x23

    ; =========================================================
    ; Test 7: Verify TF is cleared by trap gate
    ; =========================================================
    mov word [handler_tf], 0xFFFF
    pushf
    pop ax
    or ax, 0x0100
    push ax
    popf
    int 0x31

    mov ax, word [handler_tf]
    mov bx, 0x0000                  ; TF cleared by trap gate too
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

int_gate_handler:
    push ax
    pushf
    pop ax
    mov word [handler_if], 0
    mov word [handler_tf], 0
    test ax, 0x0200
    jz .no_if
    mov word [handler_if], 0x0200
.no_if:
    test ax, 0x0100
    jz .no_tf
    mov word [handler_tf], 0x0100
.no_tf:
    pop ax
    iret

trap_gate_handler:
    push ax
    pushf
    pop ax
    mov word [handler_if], 0
    mov word [handler_tf], 0
    test ax, 0x0200
    jz .no_if
    mov word [handler_if], 0x0200
.no_if:
    test ax, 0x0100
    jz .no_tf
    mov word [handler_tf], 0x0100
.no_tf:
    pop ax
    iret

section .data

gdt_ptr:
    dw 0x1F                     ; limit: 4 entries * 8 - 1
    dw 0x0000
    db 0x00

idt_ptr:
    dw 0x18F                    ; covers up to vector 0x31
    dw 0x0800
    db 0x00

handler_if: dw 0
handler_tf: dw 0
