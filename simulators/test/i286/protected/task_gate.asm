; task_gate.asm - Tests task gate in IDT (task switch via interrupt)
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; Memory layout:
;   0x0000-0x002F: GDT (6 entries)
;   0x0800-0x098F: IDT (covers vectors up to 0x31)
;   0x2000-0x202B: TSS0 (current task)
;   0x3000-0x302B: TSS1 (target task for task gate)
;
; GDT:
;   0 (0x00): Null
;   1 (0x08): Code DPL=0, exec/read, base=0, limit=0xFFFF
;   2 (0x10): Data DPL=0, r/w, base=0, limit=0xFFFF (flat)
;   3 (0x18): Stack DPL=0, r/w, base=0, limit=0xFFFF
;   4 (0x20): TSS0, base=0x2000, limit=0x2B (available)
;   5 (0x28): TSS1, base=0x3000, limit=0x2B (available)
;
; IDT:
;   Vector 0x30: Task gate → TSS1 (selector 0x28)

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

    ; Entry 4: TSS0 (0x20) - base=0x2000, limit=0x2B
    ; Access 0x81: P=1, DPL=0, S=0, A=1, type=000 (available TSS)
    mov word [0x20], 0x002B
    mov word [0x22], 0x2000
    mov byte [0x24], 0x00
    mov byte [0x25], 0x81
    mov word [0x26], 0x0000

    ; Entry 5: TSS1 (0x28) - base=0x3000, limit=0x2B
    ; Access 0x81: available TSS
    mov word [0x28], 0x002B
    mov word [0x2A], 0x3000
    mov byte [0x2C], 0x00
    mov byte [0x2D], 0x81
    mov word [0x2E], 0x0000

    ; IDT: Vector 0x30 = task gate → TSS1 (selector 0x28)
    ; at 0x0800 + 0x30*8 = 0x0980
    mov word [0x0980], 0x0000       ; offset (unused for task gate)
    mov word [0x0982], 0x0028       ; TSS selector
    mov byte [0x0984], 0x00
    mov byte [0x0985], 0x85         ; P=1, DPL=0, type=0101 (task gate)
    mov word [0x0986], 0x0000

    ; =========================================================
    ; Initialize TSS0 at 0x2000
    ; =========================================================
    mov word [0x2000 + 0],  0x0000  ; back_link
    mov word [0x2000 + 2],  0x1000  ; SP0
    mov word [0x2000 + 4],  0x0018  ; SS0
    mov word [0x2000 + 42], 0x0000  ; LDTR

    ; =========================================================
    ; Initialize TSS1 at 0x3000
    ; =========================================================
    mov word [0x3000 + 0],  0x0000  ; back_link (will be set by task switch)
    mov word [0x3000 + 2],  0x4000  ; SP0
    mov word [0x3000 + 4],  0x0018  ; SS0
    mov word [0x3000 + 14], task1_entry  ; IP
    mov word [0x3000 + 16], 0x0002  ; FLAGS (bit 1 always set)
    mov word [0x3000 + 18], 0x0000  ; AX
    mov word [0x3000 + 20], 0x0000  ; CX
    mov word [0x3000 + 22], 0x0000  ; DX
    mov word [0x3000 + 24], 0x0000  ; BX
    mov word [0x3000 + 26], 0x4000  ; SP
    mov word [0x3000 + 28], 0x0000  ; BP
    mov word [0x3000 + 30], 0x0000  ; SI
    mov word [0x3000 + 32], 0x0000  ; DI
    mov word [0x3000 + 34], 0x0010  ; ES
    mov word [0x3000 + 36], 0x0008  ; CS
    mov word [0x3000 + 38], 0x0018  ; SS
    mov word [0x3000 + 40], 0x0010  ; DS
    mov word [0x3000 + 42], 0x0000  ; LDTR

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

    ; Load Task Register with TSS0
    mov ax, 0x20
    ltr ax

    ; =========================================================
    ; Test 1: Verify initial TR = TSS0
    ; =========================================================
    str ax
    mov bx, 0x0020
    int 0x23

    ; =========================================================
    ; Test 2: Set known register values before task switch
    ; =========================================================
    mov word [task1_ran], 0x0000

    ; INT through task gate → TSS1
    int 0x30

    ; =========================================================
    ; After task1 IRETs back here:
    ; =========================================================

    ; =========================================================
    ; Test 3: Verify task1 ran (wrote marker)
    ; =========================================================
    mov ax, word [task1_ran]
    mov bx, 0xBEEF
    int 0x23

    ; =========================================================
    ; Test 4: Verify TR is back to TSS0
    ; =========================================================
    str ax
    mov bx, 0x0020
    int 0x23

    ; =========================================================
    ; Test 5: Verify TSS1.back_link was set to TSS0 selector
    ; (nested task switch sets back_link)
    ; =========================================================
    mov ax, word [0x3000 + 0]
    mov bx, 0x0020
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

; =========================================================
; Task 1 entry point (loaded from TSS1)
; =========================================================
task1_entry:
    ; DS=0x10 (from TSS1 setup)
    mov word [task1_ran], 0xBEEF

    ; =========================================================
    ; Test: Verify TR = TSS1 selector (0x28)
    ; =========================================================
    str ax
    mov bx, 0x0028
    int 0x23

    ; =========================================================
    ; Test: Verify NT flag is set (nested task, bit 14 = 0x4000)
    ; =========================================================
    pushf
    pop ax
    and ax, 0x4000
    mov bx, 0x4000
    int 0x23

    ; IRET with NT=1 → switches back to old task via back_link
    iret

section .data

gdt_ptr:
    dw 0x2F                     ; limit: 6 entries * 8 - 1
    dw 0x0000
    db 0x00

idt_ptr:
    dw 0x18F                    ; covers up to vector 0x31
    dw 0x0800
    db 0x00

task1_ran: dw 0
