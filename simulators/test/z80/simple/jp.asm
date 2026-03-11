; jp.asm - Test JP (jump) instructions
; RST 0x10: assert A == B (8-bit)
; HALT: test complete

org 0x100

; JP nn (unconditional)
    jp test1
    ld a, 0xFF      ; should be skipped
    ld b, 0x00
    rst 0x10
test1:
    ld a, 0x01
    ld b, 0x01
    rst 0x10

; JP forward over data
    jp test2
    db 0xFF, 0xFF, 0xFF  ; garbage data
test2:
    ld a, 0x02
    ld b, 0x02
    rst 0x10

; JP backward
    jp test3_start
test3_target:
    ld a, 0x03
    ld b, 0x03
    rst 0x10
    jp test3_end
test3_start:
    jp test3_target
test3_end:

    halt
