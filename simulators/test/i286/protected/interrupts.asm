; interrupts.asm - Tests IDT setup, INT dispatch, and IRET in protected mode
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

section .text
start:

    ; =========================================================
    ; Set up GDT at linear address 0
    ;
    ;   Entry 0 (0x00): Null descriptor
    ;   Entry 1 (0x08): Code segment - base=0, limit=0xFFFF, exec/read, DPL=0
    ;   Entry 2 (0x10): Data segment - base=0, limit=0xFFFF, read/write, DPL=0
    ;   Entry 3 (0x18): Stack segment - base=0, limit=0xFFFF, read/write, DPL=0
    ; =========================================================

    xor ax, ax
    mov ds, ax

    ; Entry 0: Null
    mov word [0x00], 0x0000
    mov word [0x02], 0x0000
    mov word [0x04], 0x0000
    mov word [0x06], 0x0000

    ; Entry 1: Code (0x08) - exec/read, DPL=0
    mov word [0x08], 0xFFFF
    mov word [0x0A], 0x0000
    mov byte [0x0C], 0x00
    mov byte [0x0D], 0x9A
    mov word [0x0E], 0x0000

    ; Entry 2: Data (0x10) - read/write, DPL=0
    mov word [0x10], 0xFFFF
    mov word [0x12], 0x0000
    mov byte [0x14], 0x00
    mov byte [0x15], 0x92
    mov word [0x16], 0x0000

    ; Entry 3: Stack (0x18) - read/write, DPL=0
    mov word [0x18], 0xFFFF
    mov word [0x1A], 0x0000
    mov byte [0x1C], 0x00
    mov byte [0x1D], 0x92
    mov word [0x1E], 0x0000

    ; =========================================================
    ; Set up IDT at linear address 0x0800
    ;
    ; IDT entry format (8 bytes):
    ;   Word 0: offset (handler IP)
    ;   Word 1: segment selector (handler CS)
    ;   Byte 4: reserved (0)
    ;   Byte 5: access (P, DPL, 0, gate_type[3], T)
    ;   Word 3: reserved (0)
    ;
    ; Interrupt gate: byte 5 = 0x86 (P=1, DPL=0, type=011, T=0)
    ; Trap gate:      byte 5 = 0x87 (P=1, DPL=0, type=011, T=1)
    ;
    ;   Vector 0x30: Interrupt gate -> int_handler
    ;   Vector 0x31: Trap gate -> trap_handler
    ;   Vector 0x32: Interrupt gate -> counter_handler
    ; =========================================================

    ; Vector 0x30 at IDT_BASE + 0x30*8 = 0x0800 + 0x0180 = 0x0980
    mov word [0x0980], int_handler      ; offset
    mov word [0x0982], 0x0008           ; code selector
    mov byte [0x0984], 0x00             ; reserved
    mov byte [0x0985], 0x86             ; interrupt gate (P=1, DPL=0, type=011, T=0)
    mov word [0x0986], 0x0000           ; reserved

    ; Vector 0x31 at IDT_BASE + 0x31*8 = 0x0800 + 0x0188 = 0x0988
    mov word [0x0988], trap_handler     ; offset
    mov word [0x098A], 0x0008           ; code selector
    mov byte [0x098C], 0x00             ; reserved
    mov byte [0x098D], 0x87             ; trap gate (P=1, DPL=0, type=011, T=1)
    mov word [0x098E], 0x0000           ; reserved

    ; Vector 0x32 at IDT_BASE + 0x32*8 = 0x0800 + 0x0190 = 0x0990
    mov word [0x0990], counter_handler  ; offset
    mov word [0x0992], 0x0008           ; code selector
    mov byte [0x0994], 0x00             ; reserved
    mov byte [0x0995], 0x86             ; interrupt gate
    mov word [0x0996], 0x0000           ; reserved

    ; =========================================================
    ; Load GDT and IDT
    ; =========================================================
    push cs
    pop ds
    lgdt [gdt_ptr]
    lidt [idt_ptr]

    ; =========================================================
    ; Enter protected mode
    ; =========================================================
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
    ; Test 1: INT through interrupt gate
    ; Handler sets flag_int to 1
    ; =========================================================
    mov byte [flag_int], 0
    int 0x30
    ; Verify handler ran
    mov al, byte [flag_int]
    mov ah, 1
    int 0x22

    ; =========================================================
    ; Test 2: INT through trap gate
    ; Handler sets flag_trap to 1
    ; =========================================================
    mov byte [flag_trap], 0
    int 0x31
    ; Verify handler ran
    mov al, byte [flag_trap]
    mov ah, 1
    int 0x22

    ; =========================================================
    ; Test 3: Interrupt gate clears IF
    ; Set IF=1, trigger INT 0x30, handler checks IF=0
    ; After IRET, IF should be restored to 1
    ; =========================================================
    mov byte [observed_if], 0xFF    ; sentinel
    sti                             ; set IF=1
    int 0x30                        ; interrupt gate -> handler checks IF
    ; After IRET, IF should be restored
    pushf
    pop ax
    and ax, 0x0200                  ; isolate IF bit
    mov bx, 0x0200                  ; expect IF=1
    int 0x23

    ; The handler stored the IF value it observed
    mov al, byte [observed_if]
    mov ah, 0                       ; interrupt gate should have IF=0
    int 0x22

    ; =========================================================
    ; Test 4: Trap gate preserves IF
    ; Set IF=1, trigger INT 0x31, handler checks IF=1
    ; =========================================================
    mov byte [observed_if], 0xFF    ; sentinel
    sti                             ; set IF=1
    int 0x31                        ; trap gate -> handler checks IF
    ; After IRET, IF should still be 1
    pushf
    pop ax
    and ax, 0x0200
    mov bx, 0x0200                  ; expect IF=1
    int 0x23

    ; The handler stored IF value (trap gate preserves IF)
    mov al, byte [observed_if]
    mov ah, 1                       ; trap gate should have IF=1
    int 0x22

    ; =========================================================
    ; Test 5: IRET restores FLAGS correctly
    ; Set CF=1 before INT, verify CF=1 after IRET
    ; =========================================================
    stc                             ; set CF=1
    int 0x30
    ; After IRET, CF should be restored
    pushf
    pop ax
    and ax, 0x0001                  ; isolate CF
    mov bx, 0x0001                  ; expect CF=1
    int 0x23

    ; =========================================================
    ; Test 6: Multiple interrupts (counter)
    ; Call INT 0x32 three times, counter should be 3
    ; =========================================================
    mov word [counter], 0
    int 0x32
    int 0x32
    int 0x32
    mov ax, word [counter]
    mov bx, 3
    int 0x23

    ; =========================================================
    ; Test 7: Stack pointer is preserved through INT/IRET
    ; =========================================================
    mov word [saved_sp], sp
    int 0x30
    mov ax, sp
    mov bx, word [saved_sp]
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

; =========================================================
; Interrupt handler (interrupt gate - clears IF)
; =========================================================
int_handler:
    ; Set flag to indicate handler ran
    mov byte [flag_int], 1

    ; Observe IF flag
    pushf
    pop ax
    and ax, 0x0200
    mov cl, 9
    shr ax, cl
    mov byte [observed_if], al

    iret

; =========================================================
; Trap handler (trap gate - preserves IF)
; =========================================================
trap_handler:
    ; Set flag to indicate handler ran
    mov byte [flag_trap], 1

    ; Observe IF flag
    pushf
    pop ax
    and ax, 0x0200
    mov cl, 9
    shr ax, cl
    mov byte [observed_if], al

    iret

; =========================================================
; Counter handler (increments counter)
; =========================================================
counter_handler:
    push ax
    mov ax, word [counter]
    inc ax
    mov word [counter], ax
    pop ax
    iret

section .data

gdt_ptr:
    dw 0x1F                     ; limit: 4 entries * 8 - 1
    dw 0x0000                   ; base low
    db 0x00                     ; base high

idt_ptr:
    dw 0x0197                   ; limit: covers vectors 0-0x32 (51*8-1 = 0x197)
    dw 0x0800                   ; base low
    db 0x00                     ; base high

flag_int:    db 0
flag_trap:   db 0
observed_if: db 0
counter:     dw 0
saved_sp:    dw 0
