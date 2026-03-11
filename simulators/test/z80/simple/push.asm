; push.asm - Test PUSH and POP instructions
; RST 0x10: assert A == B (8-bit)
; RST 0x18: assert HL == DE (16-bit)
; HALT: test complete

org 0x100

; PUSH/POP BC
    ld bc, 0x1234
    push bc
    ld bc, 0x0000
    pop bc
    ld h, b
    ld l, c
    ld de, 0x1234
    rst 0x18

; PUSH/POP DE
    ld de, 0xABCD
    push de
    ld de, 0x0000
    pop de
    ld h, d
    ld l, e
    ld de, 0xABCD
    rst 0x18

; PUSH/POP HL
    ld hl, 0x5678
    push hl
    ld hl, 0x0000
    pop hl
    ld de, 0x5678
    rst 0x18

; PUSH/POP AF
    ld a, 0x42
    push af
    ld a, 0x00
    pop af
    ld b, 0x42
    rst 0x10

; Multiple push/pop (LIFO order)
    ld bc, 0x1111
    ld de, 0x2222
    push bc
    push de
    pop bc          ; BC gets DE's value (0x2222)
    pop de          ; DE gets BC's value (0x1111)
    ; Check BC == 0x2222
    ld h, b
    ld l, c
    ld de, 0x2222
    rst 0x18

    ; Check original DE value now in DE (0x1111)
    ; We need to re-read DE since rst 0x18 doesn't preserve it
    ; DE was popped as 0x1111 but then overwritten by ld de, 0x2222 above
    ; So let's redo: push/pop again
    ld bc, 0xAAAA
    ld de, 0xBBBB
    push bc
    push de
    pop bc          ; BC = 0xBBBB
    pop de          ; DE = 0xAAAA
    ; Check DE = 0xAAAA: copy to HL first, then set DE to expected
    ld h, d
    ld l, e
    ld de, 0xAAAA
    rst 0x18

    halt
