; segment_load.asm - Tests segment register loading in protected mode
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
    ;   Entry 4 (0x20): Data segment - base=0x2000, limit=0x00FF, read/write, DPL=0
    ;   Entry 5 (0x28): Data segment - base=0x3000, limit=0x00FF, read/write, DPL=0
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

    ; Entry 2: Data (0x10) - read/write, DPL=0, base=0
    mov word [0x10], 0xFFFF
    mov word [0x12], 0x0000
    mov byte [0x14], 0x00
    mov byte [0x15], 0x92
    mov word [0x16], 0x0000

    ; Entry 3: Stack (0x18) - read/write, DPL=0, base=0
    mov word [0x18], 0xFFFF
    mov word [0x1A], 0x0000
    mov byte [0x1C], 0x00
    mov byte [0x1D], 0x92
    mov word [0x1E], 0x0000

    ; Entry 4: Data (0x20) - read/write, DPL=0, base=0x2000, limit=0x00FF
    mov word [0x20], 0x00FF     ; limit = 0x00FF
    mov word [0x22], 0x2000     ; base low = 0x2000
    mov byte [0x24], 0x00       ; base high = 0x00
    mov byte [0x25], 0x92       ; access: P=1, DPL=0, S=1, Type=0010
    mov word [0x26], 0x0000

    ; Entry 5: Data (0x28) - read/write, DPL=0, base=0x3000, limit=0x00FF
    mov word [0x28], 0x00FF     ; limit = 0x00FF
    mov word [0x2A], 0x3000     ; base low = 0x3000
    mov byte [0x2C], 0x00       ; base high = 0x00
    mov byte [0x2D], 0x92       ; access: P=1, DPL=0, S=1, Type=0010
    mov word [0x2E], 0x0000

    ; =========================================================
    ; Load GDT and enter protected mode
    ; =========================================================
    push cs
    pop ds
    lgdt [gdt_ptr]

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
    ; Test 1: Verify DS loaded correctly (selector 0x10)
    ; =========================================================
    mov ax, ds
    mov bx, 0x10
    int 0x23

    ; =========================================================
    ; Test 2: Load ES with data selector
    ; =========================================================
    mov ax, 0x10
    mov es, ax
    mov ax, es
    mov bx, 0x10
    int 0x23

    ; =========================================================
    ; Test 3: Memory access through DS (base=0, flat model)
    ; Write through DS, read back
    ; =========================================================
    mov word [test_data], 0x1234
    mov ax, word [test_data]
    mov bx, 0x1234
    int 0x23

    ; =========================================================
    ; Test 4: Memory access through ES
    ; Write and read using ES override
    ; =========================================================
    mov word [es:test_data], 0x5678
    mov ax, word [es:test_data]
    mov bx, 0x5678
    int 0x23

    ; =========================================================
    ; Test 5: Load DS with different base segment (0x20, base=0x2000)
    ; Write a known value to linear address 0x2000 via flat DS first
    ; Then switch DS to selector 0x20 and read offset 0
    ; =========================================================
    ; Write 0xCAFE to linear address 0x2000 using flat DS (selector 0x10)
    mov word [0x2000], 0xCAFE

    ; Switch DS to selector 0x20 (base=0x2000)
    mov ax, 0x20
    mov ds, ax

    ; Read offset 0 -> should get 0xCAFE (base 0x2000 + offset 0)
    mov ax, word [0x0000]
    mov bx, 0xCAFE
    int 0x23

    ; =========================================================
    ; Test 6: Write through non-zero base segment
    ; Write 0xBEEF at offset 2 in DS (selector 0x20, base=0x2000)
    ; -> linear address 0x2002
    ; =========================================================
    mov word [0x0002], 0xBEEF

    ; Switch DS back to flat (0x10) and verify at linear 0x2002
    mov ax, 0x10
    mov ds, ax
    mov ax, word [0x2002]
    mov bx, 0xBEEF
    int 0x23

    ; =========================================================
    ; Test 7: Two segments with different bases
    ; DS -> selector 0x20 (base=0x2000)
    ; ES -> selector 0x28 (base=0x3000)
    ; Write through each, verify isolation
    ; =========================================================
    mov ax, 0x20
    mov ds, ax
    mov ax, 0x28
    mov es, ax

    ; Write via DS (base 0x2000) at offset 0x10
    mov word [0x0010], 0xAAAA
    ; Write via ES (base 0x3000) at offset 0x10
    mov word [es:0x0010], 0xBBBB

    ; Read back via DS -> should be 0xAAAA (linear 0x2010)
    mov ax, word [0x0010]
    mov bx, 0xAAAA
    int 0x23

    ; Read back via ES -> should be 0xBBBB (linear 0x3010)
    mov ax, word [es:0x0010]
    mov bx, 0xBBBB
    int 0x23

    ; =========================================================
    ; Test 8: Cross-verify via flat DS
    ; Switch DS to flat (0x10), read linear 0x2010 and 0x3010
    ; =========================================================
    mov ax, 0x10
    mov ds, ax

    mov ax, word [0x2010]
    mov bx, 0xAAAA
    int 0x23

    mov ax, word [0x3010]
    mov bx, 0xBBBB
    int 0x23

    ; =========================================================
    ; Test 9: Load ES with null selector (allowed for ES)
    ; ES becomes 0, but no fault
    ; =========================================================
    xor ax, ax
    mov es, ax
    mov ax, es
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; Test 10: Reload ES with valid selector after null
    ; =========================================================
    mov ax, 0x10
    mov es, ax
    mov ax, es
    mov bx, 0x10
    int 0x23
    ; Verify access works
    mov word [es:test_data], 0x9999
    mov ax, word [es:test_data]
    mov bx, 0x9999
    int 0x23

    ; =========================================================
    ; Test 11: Push/pop segment selectors
    ; =========================================================
    mov ax, 0x20
    mov ds, ax
    push ds
    mov ax, 0x10
    mov ds, ax
    pop ax                      ; should get 0x20
    mov bx, 0x20
    int 0x23

    ; =========================================================
    ; Test 12: Stack operations with SS segment
    ; Verify push/pop work correctly through SS
    ; =========================================================
    mov ax, 0xDEAD
    push ax
    mov ax, 0xFACE
    push ax
    pop bx                      ; should be 0xFACE
    mov ax, 0xFACE
    int 0x23
    pop bx                      ; should be 0xDEAD
    mov ax, 0xDEAD
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

section .data

gdt_ptr:
    dw 0x2F                     ; limit: 6 entries * 8 - 1
    dw 0x0000                   ; base low
    db 0x00                     ; base high

test_data: dw 0
