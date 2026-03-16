; strings.asm - Tests string instructions in protected mode with non-zero segment bases
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; GDT:
;   0 (0x00): Null
;   1 (0x08): Code DPL=0, exec/read, base=0, limit=0xFFFF
;   2 (0x10): Data DPL=0, r/w, base=0, limit=0xFFFF (flat)
;   3 (0x18): Stack DPL=0, r/w, base=0, limit=0xFFFF
;   4 (0x20): Source data, base=0x4000, limit=0x00FF
;   5 (0x28): Dest data, base=0x5000, limit=0x00FF
;
; Source data at linear 0x4000, destination at linear 0x5000

section .text
start:

    xor ax, ax
    mov ds, ax

    ; =========================================================
    ; Set up GDT
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

    ; Entry 4: Source data (0x20) - base=0x4000, limit=0xFF
    mov word [0x20], 0x00FF
    mov word [0x22], 0x4000
    mov byte [0x24], 0x00
    mov byte [0x25], 0x92
    mov word [0x26], 0x0000

    ; Entry 5: Dest data (0x28) - base=0x5000, limit=0xFF
    mov word [0x28], 0x00FF
    mov word [0x2A], 0x5000
    mov byte [0x2C], 0x00
    mov byte [0x2D], 0x92
    mov word [0x2E], 0x0000

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
    ; Seed source data at linear 0x4000 via flat DS
    ; =========================================================
    mov byte [0x4000], 0x11
    mov byte [0x4001], 0x22
    mov byte [0x4002], 0x33
    mov byte [0x4003], 0x44
    mov byte [0x4004], 0x55
    mov byte [0x4005], 0x66
    mov byte [0x4006], 0x77
    mov byte [0x4007], 0x88

    ; Clear destination area
    mov word [0x5000], 0x0000
    mov word [0x5002], 0x0000
    mov word [0x5004], 0x0000
    mov word [0x5006], 0x0000
    mov word [0x5008], 0x0000
    mov word [0x5010], 0x0000
    mov word [0x5020], 0x0000
    mov word [0x5022], 0x0000
    mov word [0x5024], 0x0000

    ; =========================================================
    ; Test 1: MOVSB forward across different segment bases
    ; DS=0x20 (base 0x4000), ES=0x28 (base 0x5000)
    ; =========================================================
    mov ax, 0x20
    mov ds, ax
    mov ax, 0x28
    mov es, ax
    cld

    xor si, si                 ; SI=0 -> linear 0x4000
    xor di, di                 ; DI=0 -> linear 0x5000
    movsb                      ; copy 1 byte: [0x4000] -> [0x5000]

    ; Verify SI=1, DI=1
    mov ax, si
    mov bx, 0x0001
    int 0x23
    mov ax, di
    mov bx, 0x0001
    int 0x23

    ; Verify destination via flat DS
    mov ax, 0x10
    mov ds, ax
    mov al, byte [0x5000]
    mov ah, 0x11
    int 0x22

    ; =========================================================
    ; Test 2: REP MOVSB forward (4 bytes)
    ; =========================================================
    ; Re-clear destination
    mov word [0x5000], 0x0000
    mov word [0x5002], 0x0000

    mov ax, 0x20
    mov ds, ax
    cld
    xor si, si
    xor di, di
    mov cx, 4
    rep movsb

    ; Verify CX=0, SI=4, DI=4
    mov ax, cx
    mov bx, 0x0000
    int 0x23
    mov ax, si
    mov bx, 0x0004
    int 0x23
    mov ax, di
    mov bx, 0x0004
    int 0x23

    ; Verify 4 bytes at destination
    mov ax, 0x10
    mov ds, ax
    mov al, byte [0x5000]
    mov ah, 0x11
    int 0x22
    mov al, byte [0x5001]
    mov ah, 0x22
    int 0x22
    mov al, byte [0x5002]
    mov ah, 0x33
    int 0x22
    mov al, byte [0x5003]
    mov ah, 0x44
    int 0x22

    ; =========================================================
    ; Test 3: REP MOVSW forward (2 words)
    ; =========================================================
    mov word [0x5000], 0x0000
    mov word [0x5002], 0x0000

    mov ax, 0x20
    mov ds, ax
    cld
    xor si, si
    xor di, di
    mov cx, 2
    rep movsw

    ; Verify SI=4, DI=4
    mov ax, si
    mov bx, 0x0004
    int 0x23

    ; Verify 2 words at destination
    mov ax, 0x10
    mov ds, ax
    mov ax, word [0x5000]      ; should be 0x2211 (LE: [0x11, 0x22])
    mov bx, 0x2211
    int 0x23
    mov ax, word [0x5002]      ; should be 0x4433
    mov bx, 0x4433
    int 0x23

    ; =========================================================
    ; Test 4: MOVSB backward (DF=1)
    ; =========================================================
    mov word [0x5000], 0x0000
    mov word [0x5002], 0x0000

    mov ax, 0x20
    mov ds, ax
    std                        ; direction flag = 1 (decrement)
    mov si, 3                  ; source offset 3 -> linear 0x4003
    mov di, 3                  ; dest offset 3 -> linear 0x5003
    movsb

    ; SI=2, DI=2
    mov ax, si
    mov bx, 0x0002
    int 0x23
    mov ax, di
    mov bx, 0x0002
    int 0x23

    ; Verify: byte at linear 0x5003 should be 0x44
    mov ax, 0x10
    mov ds, ax
    mov al, byte [0x5003]
    mov ah, 0x44
    int 0x22

    cld                        ; restore direction flag

    ; =========================================================
    ; Test 5: STOSB through ES segment (base 0x5000)
    ; =========================================================
    mov al, 0xAA
    mov di, 0x10
    stosb

    ; DI=0x11
    mov ax, di
    mov bx, 0x0011
    int 0x23

    ; Verify: linear 0x5010 == 0xAA
    mov ax, 0x10
    mov ds, ax
    mov al, byte [0x5010]
    mov ah, 0xAA
    int 0x22

    ; =========================================================
    ; Test 6: REP STOSW (fill 3 words)
    ; =========================================================
    mov ax, 0xBEEF
    mov di, 0x20
    mov cx, 3
    rep stosw

    ; DI=0x26, CX=0
    mov ax, di
    mov bx, 0x0026
    int 0x23
    mov ax, cx
    mov bx, 0x0000
    int 0x23

    ; Verify 3 words at linear 0x5020
    mov ax, 0x10
    mov ds, ax
    mov ax, word [0x5020]
    mov bx, 0xBEEF
    int 0x23
    mov ax, word [0x5022]
    mov bx, 0xBEEF
    int 0x23
    mov ax, word [0x5024]
    mov bx, 0xBEEF
    int 0x23

    ; =========================================================
    ; Test 7: LODSB from DS segment (base 0x4000)
    ; =========================================================
    mov ax, 0x20
    mov ds, ax
    xor si, si
    lodsb

    ; AL should be 0x11, SI should be 1
    mov ah, 0x11
    int 0x22
    mov ax, si
    mov bx, 0x0001
    int 0x23

    ; =========================================================
    ; Test 8: LODSW from DS segment
    ; =========================================================
    xor si, si
    lodsw

    ; AX should be 0x2211 (word at linear 0x4000, LE)
    mov bx, 0x2211
    int 0x23
    ; SI should be 2
    mov ax, si
    mov bx, 0x0002
    int 0x23

    ; =========================================================
    ; Test 9: SCASB through ES segment (base 0x5000)
    ; Seed known value first via flat DS
    ; =========================================================
    mov ax, 0x10
    mov ds, ax
    mov byte [0x5000], 0x11
    mov byte [0x5001], 0x22
    mov byte [0x5002], 0x33

    ; Scan for 0x22 starting at DI=0
    mov al, 0x22
    mov di, 0
    mov cx, 3
    repne scasb

    ; Should stop after matching byte at offset 1
    ; DI=2 (advanced past the match), CX=1 (decremented from 3 to 1)
    mov ax, di
    mov bx, 0x0002
    int 0x23
    mov ax, cx
    mov bx, 0x0001
    int 0x23

    ; ZF should be 1 (match found)
    pushf
    pop ax
    and ax, 0x0040
    mov bx, 0x0040
    int 0x23

    ; =========================================================
    ; Test 10: CMPSB with same data in both segments
    ; Seed identical bytes at source (0x4000) and dest (0x5000)
    ; =========================================================
    mov ax, 0x10
    mov ds, ax
    ; Source already has [0x11,0x22,0x33,0x44] at linear 0x4000
    ; Copy same to linear 0x5000
    mov byte [0x5000], 0x11
    mov byte [0x5001], 0x22
    mov byte [0x5002], 0x33
    mov byte [0x5003], 0x44

    mov ax, 0x20
    mov ds, ax
    xor si, si
    xor di, di
    mov cx, 4
    repe cmpsb

    ; All 4 bytes match: CX=0, SI=4, DI=4, ZF=1
    mov ax, cx
    mov bx, 0x0000
    int 0x23
    mov ax, si
    mov bx, 0x0004
    int 0x23
    mov ax, di
    mov bx, 0x0004
    int 0x23

    pushf
    pop ax
    and ax, 0x0040
    mov bx, 0x0040             ; ZF=1
    int 0x23

    ; =========================================================
    ; Test 11: CMPSB with mismatch at 3rd byte
    ; =========================================================
    mov ax, 0x10
    mov ds, ax
    mov byte [0x5002], 0xFF    ; change 3rd dest byte to mismatch

    mov ax, 0x20
    mov ds, ax
    xor si, si
    xor di, di
    mov cx, 4
    repe cmpsb

    ; Stops at 3rd comparison: CX=1, SI=3, DI=3, ZF=0
    mov ax, cx
    mov bx, 0x0001
    int 0x23
    mov ax, si
    mov bx, 0x0003
    int 0x23

    pushf
    pop ax
    and ax, 0x0040
    mov bx, 0x0000             ; ZF=0 (mismatch)
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
