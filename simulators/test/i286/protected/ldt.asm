; ldt.asm - Tests LDT operations: LLDT, SLDT, LDT-based segment loading
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; Memory layout:
;   0x0000-0x002F: GDT (6 entries)
;   0x2000-0x202F: LDT (6 entries)
;   0x4000-0x40FF: LDT data segment A (base for LDT entry 1)
;   0x5000-0x50FF: LDT data segment B (base for LDT entry 2)
;
; GDT:
;   0 (0x00): Null
;   1 (0x08): Code DPL=0, exec/read, base=0, limit=0xFFFF
;   2 (0x10): Data DPL=0, r/w, base=0, limit=0xFFFF (flat)
;   3 (0x18): Stack DPL=0, r/w, base=0, limit=0xFFFF
;   4 (0x20): LDT descriptor, base=0x2000, limit=0x2F
;   5 (0x28): Data DPL=0, r/w, base=0x4000, limit=0x00FF
;
; LDT (at linear 0x2000):
;   0 (0x04): Null
;   1 (0x0C): Data DPL=0, r/w, base=0x4000, limit=0x00FF
;   2 (0x14): Data DPL=0, r/w, base=0x5000, limit=0x00FF
;   3 (0x1C): Data DPL=0, read-only, base=0x6000, limit=0x00FF
;   4 (0x24): Code DPL=0, exec/read, base=0, limit=0xFFFF
;   5 (0x2C): Data DPL=0, r/w, P=0 (not present), base=0x7000, limit=0x00FF
;
; LDT selectors have TI=1 (bit 2): index*8 | 0x04

section .text
start:

    xor ax, ax
    mov ds, ax

    ; =========================================================
    ; Set up GDT at linear address 0
    ; =========================================================

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

    ; Entry 4: LDT descriptor (0x20) - base=0x2000, limit=0x2F
    ; Access 0x82: P=1, DPL=0, S=0, type=001, A=0 (LDT)
    mov word [0x20], 0x002F     ; limit = 6*8-1
    mov word [0x22], 0x2000     ; base low
    mov byte [0x24], 0x00       ; base high
    mov byte [0x25], 0x82       ; LDT descriptor
    mov word [0x26], 0x0000

    ; Entry 5: Data DPL=0 (0x28) - base=0x4000, limit=0xFF
    mov word [0x28], 0x00FF
    mov word [0x2A], 0x4000
    mov byte [0x2C], 0x00
    mov byte [0x2D], 0x92
    mov word [0x2E], 0x0000

    ; =========================================================
    ; Set up LDT at linear address 0x2000
    ; =========================================================

    ; LDT Entry 0: Null
    mov word [0x2000], 0x0000
    mov word [0x2002], 0x0000
    mov word [0x2004], 0x0000
    mov word [0x2006], 0x0000

    ; LDT Entry 1 (sel 0x0C): Data r/w, base=0x4000, limit=0xFF
    mov word [0x2008], 0x00FF
    mov word [0x200A], 0x4000
    mov byte [0x200C], 0x00
    mov byte [0x200D], 0x92
    mov word [0x200E], 0x0000

    ; LDT Entry 2 (sel 0x14): Data r/w, base=0x5000, limit=0xFF
    mov word [0x2010], 0x00FF
    mov word [0x2012], 0x5000
    mov byte [0x2014], 0x00
    mov byte [0x2015], 0x92
    mov word [0x2016], 0x0000

    ; LDT Entry 3 (sel 0x1C): Data read-only, base=0x6000, limit=0xFF
    ; Access 0x90: P=1, DPL=0, S=1, type=000 (data, read-only), A=0
    mov word [0x2018], 0x00FF
    mov word [0x201A], 0x6000
    mov byte [0x201C], 0x00
    mov byte [0x201D], 0x90
    mov word [0x201E], 0x0000

    ; LDT Entry 4 (sel 0x24): Code exec/read, base=0, limit=0xFFFF
    mov word [0x2020], 0xFFFF
    mov word [0x2022], 0x0000
    mov byte [0x2024], 0x00
    mov byte [0x2025], 0x9A
    mov word [0x2026], 0x0000

    ; LDT Entry 5 (sel 0x2C): Data r/w, NOT PRESENT, base=0x7000
    ; Access 0x12: P=0, DPL=0, S=1, type=001 (data r/w), A=0
    mov word [0x2028], 0x00FF
    mov word [0x202A], 0x7000
    mov byte [0x202C], 0x00
    mov byte [0x202D], 0x12
    mov word [0x202E], 0x0000

    ; =========================================================
    ; Load GDT, enter protected mode
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
    ; Test 1: LLDT with valid LDT selector
    ; =========================================================
    mov ax, 0x20
    lldt ax
    ; If we reach here, LLDT succeeded
    mov ax, 0xAAAA
    mov bx, 0xAAAA
    int 0x23

    ; =========================================================
    ; Test 2: SLDT reads back the loaded selector
    ; =========================================================
    sldt ax
    mov bx, 0x0020
    int 0x23

    ; =========================================================
    ; Test 3: Load DS from LDT (selector 0x0C, TI=1)
    ; Write 0xCAFE at linear 0x4000 via flat DS, then read
    ; through LDT-based DS at offset 0 (base 0x4000)
    ; =========================================================
    mov word [0x4000], 0xCAFE

    mov ax, 0x0C               ; LDT entry 1, TI=1
    mov ds, ax

    mov ax, word [0x0000]      ; base 0x4000 + offset 0
    mov bx, 0xCAFE
    int 0x23

    ; =========================================================
    ; Test 4: Verify DS selector value
    ; =========================================================
    mov ax, ds
    mov bx, 0x000C
    int 0x23

    ; =========================================================
    ; Test 5: Write through LDT-based DS, verify via flat DS
    ; =========================================================
    mov word [0x10], 0xBEEF    ; base 0x4000 + offset 0x10 = linear 0x4010

    mov ax, 0x10
    mov ds, ax                 ; restore flat DS

    mov ax, word [0x4010]
    mov bx, 0xBEEF
    int 0x23

    ; =========================================================
    ; Test 6: Load ES from LDT (selector 0x14, base=0x5000)
    ; =========================================================
    mov word [0x5000], 0xFACE

    mov ax, 0x14
    mov es, ax

    mov ax, word [es:0x0000]
    mov bx, 0xFACE
    int 0x23

    ; =========================================================
    ; Test 7: Isolation between LDT segments
    ; DS=0x0C (base 0x4000), ES=0x14 (base 0x5000)
    ; =========================================================
    mov ax, 0x0C
    mov ds, ax

    mov word [0x20], 0x1111           ; -> linear 0x4020
    mov word [es:0x20], 0x2222        ; -> linear 0x5020

    mov ax, word [0x20]
    mov bx, 0x1111
    int 0x23

    mov ax, word [es:0x20]
    mov bx, 0x2222
    int 0x23

    ; =========================================================
    ; Test 8: GDT and LDT segments with same base
    ; GDT 0x28 (base 0x4000) and LDT 0x0C (base 0x4000) overlap
    ; =========================================================
    mov ax, 0x10
    mov ds, ax                 ; flat DS
    mov word [0x4030], 0x0000  ; clear target

    mov ax, 0x28               ; GDT entry, base=0x4000
    mov ds, ax
    mov word [0x30], 0x3333    ; -> linear 0x4030

    mov ax, 0x0C               ; LDT entry, base=0x4000
    mov es, ax
    mov ax, word [es:0x30]     ; should see 0x3333
    mov bx, 0x3333
    int 0x23

    mov word [es:0x30], 0x4444
    mov ax, word [0x30]        ; GDT-based DS should see 0x4444
    mov bx, 0x4444
    int 0x23

    ; =========================================================
    ; Test 9: VERR on LDT r/w data selector (0x0C) — readable
    ; =========================================================
    mov ax, 0x10
    mov ds, ax

    mov ax, 0x0C
    verr ax
    pushf
    pop ax
    and ax, 0x0040             ; ZF bit
    mov bx, 0x0040             ; expect ZF=1
    int 0x23

    ; =========================================================
    ; Test 10: VERW on LDT r/w data selector (0x0C) — writable
    ; =========================================================
    mov ax, 0x0C
    verw ax
    pushf
    pop ax
    and ax, 0x0040
    mov bx, 0x0040             ; expect ZF=1
    int 0x23

    ; =========================================================
    ; Test 11: VERW on LDT read-only selector (0x1C) — not writable
    ; =========================================================
    mov ax, 0x1C
    verw ax
    pushf
    pop ax
    and ax, 0x0040
    mov bx, 0x0000             ; expect ZF=0
    int 0x23

    ; =========================================================
    ; Test 12: LAR on LDT selector (0x0C)
    ; Access byte was 0x92, but loading DS=0x0C set accessed bit -> 0x93
    ; LAR returns access byte in high byte -> 0x9300
    ; =========================================================
    mov ax, 0x0C
    lar bx, ax
    mov ax, bx
    mov bx, 0x9300
    int 0x23

    ; =========================================================
    ; Test 13: LSL on LDT selector (0x0C) — limit=0x00FF
    ; =========================================================
    mov ax, 0x0C
    lsl bx, ax
    mov ax, bx
    mov bx, 0x00FF
    int 0x23

    ; =========================================================
    ; Test 14: LLDT with null — clears LDTR
    ; =========================================================
    xor ax, ax
    lldt ax
    sldt ax
    mov bx, 0x0000
    int 0x23

    ; =========================================================
    ; Test 15: Re-load LDT and verify access still works
    ; =========================================================
    mov ax, 0x20
    lldt ax
    sldt ax
    mov bx, 0x0020
    int 0x23

    ; Read through LDT-based segment (value from test 3)
    mov ax, 0x0C
    mov ds, ax
    mov ax, word [0x0000]
    mov bx, 0xCAFE
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ax, 0x10
    mov ds, ax
    mov ah, 0x4C
    int 0x21

section .data

gdt_ptr:
    dw 0x2F                     ; limit: 6 entries * 8 - 1
    dw 0x0000                   ; base low
    db 0x00                     ; base high
