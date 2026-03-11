; call.asm - Test CALL and RET instructions
; RST 0x10: assert A == B (8-bit)
; HALT: test complete

org 0x100

; CALL nn and RET
    call sub1
    ld b, 0x42
    rst 0x10
    jp test2

sub1:
    ld a, 0x42
    ret

; Nested calls
test2:
    call outer
    ld b, 0xAA
    rst 0x10
    jp test3

outer:
    call inner
    ret

inner:
    ld a, 0xAA
    ret

; CALL and RET preserve SP correctly
test3:
    ld bc, 0x1234
    push bc             ; save BC on stack
    call sub2           ; call modifies stack
    pop bc              ; restore BC
    ld h, b
    ld l, c
    ld de, 0x1234
    rst 0x18
    jp done

sub2:
    ld a, 0x55
    ret

done:
    halt
