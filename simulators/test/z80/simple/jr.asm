; jr.asm - Test JR (jump relative) instructions
; RST 0x10: assert A == B (8-bit)
; HALT: test complete

org 0x100

; JR e (unconditional relative jump forward)
    jr test1
    ld a, 0xFF      ; should be skipped
    ld b, 0x00
    rst 0x10
test1:
    ld a, 0x01
    ld b, 0x01
    rst 0x10

; JR backward (loop test using DJNZ)
    ld b, 0x05
    ld a, 0x00
djnz_loop:
    inc a
    djnz djnz_loop
    ld b, 0x05        ; a should be 5 after 5 iterations
    rst 0x10

; JR backward again
    jr test2_start
test2_target:
    ld a, 0x03
    ld b, 0x03
    rst 0x10
    jr test2_end
test2_start:
    jr test2_target
test2_end:

    halt
