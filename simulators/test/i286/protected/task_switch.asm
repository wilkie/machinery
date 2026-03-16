; task_switch.asm - Tests task switching via JMP TSS, CALL task gate, IRET, INT task gate
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; Memory layout:
;   0x0000-0x0047: GDT (9 entries)
;   0x0800-0x0A07: IDT (covers through vector 0x40)
;   0x2000-0x202B: TSS0 (initial task)
;   0x2100-0x212B: TSS1 (JMP target)
;   0x2200-0x222B: TSS2 (CALL task gate target)
;   0x2300-0x232B: TSS3 (INT task gate target)

section .text
start:

    ; =========================================================
    ; Set up GDT at linear address 0
    ;
    ;   Entry 0 (0x00): Null descriptor
    ;   Entry 1 (0x08): Code segment - base=0, limit=0xFFFF, exec/read, DPL=0
    ;   Entry 2 (0x10): Data segment - base=0, limit=0xFFFF, read/write, DPL=0
    ;   Entry 3 (0x18): Stack segment - base=0, limit=0xFFFF, read/write, DPL=0
    ;   Entry 4 (0x20): TSS0 - base=0x2000, limit=0x2B (available TSS)
    ;   Entry 5 (0x28): TSS1 - base=0x2100, limit=0x2B (available TSS)
    ;   Entry 6 (0x30): TSS2 - base=0x2200, limit=0x2B (available TSS)
    ;   Entry 7 (0x38): Task gate -> TSS2 (selector 0x30)
    ;   Entry 8 (0x40): TSS3 - base=0x2300, limit=0x2B (available TSS)
    ; =========================================================

    xor ax, ax
    mov ds, ax

    ; Entry 0: Null
    mov word [0x00], 0x0000
    mov word [0x02], 0x0000
    mov word [0x04], 0x0000
    mov word [0x06], 0x0000

    ; Entry 1: Code (0x08) - exec/read, DPL=0
    ; Access 0x9A: P=1, DPL=0, S=1, type=101, A=0
    mov word [0x08], 0xFFFF     ; limit
    mov word [0x0A], 0x0000     ; base low
    mov byte [0x0C], 0x00       ; base high
    mov byte [0x0D], 0x9A       ; access
    mov word [0x0E], 0x0000     ; reserved

    ; Entry 2: Data (0x10) - read/write, DPL=0
    ; Access 0x92: P=1, DPL=0, S=1, type=001, A=0
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

    ; Entry 4: TSS0 (0x20) - base=0x2000, limit=0x2B
    ; Access 0x81: P=1, DPL=0, S=0, type=000, A=1 (available TSS)
    mov word [0x20], 0x002B     ; limit
    mov word [0x22], 0x2000     ; base low
    mov byte [0x24], 0x00       ; base high
    mov byte [0x25], 0x81       ; access (available TSS)
    mov word [0x26], 0x0000

    ; Entry 5: TSS1 (0x28) - base=0x2100, limit=0x2B
    mov word [0x28], 0x002B
    mov word [0x2A], 0x2100
    mov byte [0x2C], 0x00
    mov byte [0x2D], 0x81
    mov word [0x2E], 0x0000

    ; Entry 6: TSS2 (0x30) - base=0x2200, limit=0x2B
    mov word [0x30], 0x002B
    mov word [0x32], 0x2200
    mov byte [0x34], 0x00
    mov byte [0x35], 0x81
    mov word [0x36], 0x0000

    ; Entry 7: Task gate (0x38) -> TSS2 (selector 0x30)
    ; Access 0x85: P=1, DPL=0, S=0, type=010, A=1 (task gate)
    mov word [0x38], 0x0000     ; unused
    mov word [0x3A], 0x0030     ; TSS selector
    mov byte [0x3C], 0x00       ; reserved
    mov byte [0x3D], 0x85       ; access (task gate)
    mov word [0x3E], 0x0000

    ; Entry 8: TSS3 (0x40) - base=0x2300, limit=0x2B
    mov word [0x40], 0x002B
    mov word [0x42], 0x2300
    mov byte [0x44], 0x00
    mov byte [0x45], 0x81
    mov word [0x46], 0x0000

    ; =========================================================
    ; Set up IDT at linear address 0x0800
    ; Vector 0x40: Task gate -> TSS3 (selector 0x40)
    ; IDT needs to cover vectors 0-0x40 (65 entries, limit = 65*8-1 = 0x207)
    ; Vector 0x40 at IDT_BASE + 0x40*8 = 0x0800 + 0x200 = 0x0A00
    ; =========================================================

    ; IDT task gate format: same as GDT task gate
    mov word [0x0A00], 0x0000   ; unused
    mov word [0x0A02], 0x0040   ; TSS selector (GDT entry 8)
    mov byte [0x0A04], 0x00     ; reserved
    mov byte [0x0A05], 0x85     ; P=1, DPL=0, task gate
    mov word [0x0A06], 0x0000

    ; =========================================================
    ; Initialize TSS0 at 0x2000 (initial task - just needs LDTR)
    ; Register fields will be saved by the first task switch
    ; =========================================================

    mov word [0x2000 + 42], 0x0000   ; LDTR = null

    ; =========================================================
    ; Initialize TSS1 at 0x2100 (JMP target)
    ; Will set AX=0x1234 and verify it, then JMP back to TSS0
    ; =========================================================

    mov word [0x2100 + 0],  0x0000              ; back_link (unused for JMP)
    mov word [0x2100 + 14], task1_entry         ; IP
    mov word [0x2100 + 16], 0x0002              ; FLAGS (bit 1 set)
    mov word [0x2100 + 18], 0x1234              ; AX
    mov word [0x2100 + 20], 0x0000              ; CX
    mov word [0x2100 + 22], 0x0000              ; DX
    mov word [0x2100 + 24], 0x1234              ; BX (same as AX for assertion)
    mov word [0x2100 + 26], 0x1800              ; SP
    mov word [0x2100 + 28], 0x0000              ; BP
    mov word [0x2100 + 30], 0x0000              ; SI
    mov word [0x2100 + 32], 0x0000              ; DI
    mov word [0x2100 + 34], 0x0010              ; ES
    mov word [0x2100 + 36], 0x0008              ; CS
    mov word [0x2100 + 38], 0x0018              ; SS
    mov word [0x2100 + 40], 0x0010              ; DS
    mov word [0x2100 + 42], 0x0000              ; LDTR

    ; =========================================================
    ; Initialize TSS2 at 0x2200 (CALL task gate target)
    ; =========================================================

    mov word [0x2200 + 0],  0x0000              ; back_link (set by CALL)
    mov word [0x2200 + 14], task2_entry         ; IP
    mov word [0x2200 + 16], 0x0002              ; FLAGS
    mov word [0x2200 + 18], 0x0000              ; AX
    mov word [0x2200 + 20], 0x0000              ; CX
    mov word [0x2200 + 22], 0x0000              ; DX
    mov word [0x2200 + 24], 0x0000              ; BX
    mov word [0x2200 + 26], 0x1C00              ; SP
    mov word [0x2200 + 28], 0x0000              ; BP
    mov word [0x2200 + 30], 0x0000              ; SI
    mov word [0x2200 + 32], 0x0000              ; DI
    mov word [0x2200 + 34], 0x0010              ; ES
    mov word [0x2200 + 36], 0x0008              ; CS
    mov word [0x2200 + 38], 0x0018              ; SS
    mov word [0x2200 + 40], 0x0010              ; DS
    mov word [0x2200 + 42], 0x0000              ; LDTR

    ; =========================================================
    ; Initialize TSS3 at 0x2300 (INT task gate target)
    ; =========================================================

    mov word [0x2300 + 0],  0x0000              ; back_link (set by INT)
    mov word [0x2300 + 14], task3_entry         ; IP
    mov word [0x2300 + 16], 0x0002              ; FLAGS
    mov word [0x2300 + 18], 0x0000              ; AX
    mov word [0x2300 + 20], 0x0000              ; CX
    mov word [0x2300 + 22], 0x0000              ; DX
    mov word [0x2300 + 24], 0x0000              ; BX
    mov word [0x2300 + 26], 0x1E00              ; SP
    mov word [0x2300 + 28], 0x0000              ; BP
    mov word [0x2300 + 30], 0x0000              ; SI
    mov word [0x2300 + 32], 0x0000              ; DI
    mov word [0x2300 + 34], 0x0010              ; ES
    mov word [0x2300 + 36], 0x0008              ; CS
    mov word [0x2300 + 38], 0x0018              ; SS
    mov word [0x2300 + 40], 0x0010              ; DS
    mov word [0x2300 + 42], 0x0000              ; LDTR

    ; =========================================================
    ; Load GDT, IDT, enter protected mode
    ; =========================================================
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

    ; Load Task Register with TSS0 selector
    mov ax, 0x20
    ltr ax

    ; =========================================================
    ; Test 1: JMP to TSS1 (direct task switch, non-nested)
    ; TSS1 has AX=BX=0x1234. TSS1 handler verifies AX==BX,
    ; then JMPs back to TSS0. CX should be preserved.
    ; =========================================================

    mov cx, 0xBEEF
    jmp 0x28:0                  ; task switch to TSS1 -> saves state to TSS0

    ; Execution resumes here after TSS1 switches back
    ; Verify CX was preserved through the round-trip
    mov ax, cx
    mov bx, 0xBEEF
    int 0x23

    ; =========================================================
    ; Test 2: CALL via task gate (nested task switch)
    ; TSS2 handler checks NT=1 and back_link==0x20, then IRETs
    ; DX should be preserved through the round-trip
    ; =========================================================

    ; TSS2 was made busy by test... wait, TSS2 hasn't been used yet.
    ; But TSS1 was used in test 1. We need TSS2 to be available.
    ; TSS2 is available because we never switched to it.
    ; TSS0 is busy (we LTR'd it, and JMP back made it busy)

    mov dx, 0xCAFE
    call 0x38:0                 ; task gate -> TSS2 (nested)

    ; Execution resumes here after TSS2 IRETs
    ; Verify DX was preserved
    mov ax, dx
    mov bx, 0xCAFE
    int 0x23

    ; =========================================================
    ; Test 3: INT through IDT task gate (nested task switch)
    ; TSS3 handler checks NT=1 and back_link==0x20, then IRETs
    ; SI should be preserved through the round-trip
    ; =========================================================

    mov si, 0xFACE
    int 0x40                    ; IDT task gate -> TSS3 (nested)

    ; Execution resumes here after TSS3 IRETs
    ; Verify SI was preserved
    mov ax, si
    mov bx, 0xFACE
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

; =========================================================
; TSS1 entry point: JMP target
; Verifies AX==BX (both loaded from TSS as 0x1234),
; then JMPs back to TSS0
; =========================================================
task1_entry:
    ; AX and BX were loaded from TSS1 as 0x1234
    int 0x23                    ; assert AX == BX

    ; JMP back to TSS0 (non-nested switch)
    jmp 0x20:0

; =========================================================
; TSS2 entry point: CALL task gate target
; Verifies NT flag is set and back_link points to TSS0,
; then IRETs to return
; =========================================================
task2_entry:
    ; Verify NT flag is set (CALL sets NT=1 in new task)
    pushf
    pop ax
    and ax, 0x4000              ; isolate NT bit (bit 14)
    mov bx, 0x4000
    int 0x23

    ; Verify back_link points to TSS0 (selector 0x20)
    ; back_link is at offset 0 of TSS2 (at 0x2200)
    mov ax, word [0x2200]
    mov bx, 0x0020
    int 0x23

    ; Return via IRET (NT=1 triggers task switch back)
    iret

; =========================================================
; TSS3 entry point: INT task gate target
; Verifies NT flag is set and back_link points to TSS0,
; then IRETs to return
; =========================================================
task3_entry:
    ; Verify NT flag is set
    pushf
    pop ax
    and ax, 0x4000
    mov bx, 0x4000
    int 0x23

    ; Verify back_link points to TSS0 (selector 0x20)
    ; back_link is at offset 0 of TSS3 (at 0x2300)
    mov ax, word [0x2300]
    mov bx, 0x0020
    int 0x23

    ; Return via IRET
    iret

section .data

gdt_ptr:
    dw 0x47                     ; limit: 9 entries * 8 - 1
    dw 0x0000                   ; base low
    db 0x00                     ; base high

idt_ptr:
    dw 0x0207                   ; limit: 65 entries * 8 - 1 (covers vector 0x40)
    dw 0x0800                   ; base low
    db 0x00                     ; base high
