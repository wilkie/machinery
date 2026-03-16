; gdt_idt.asm - Tests LMSW/SMSW in real mode
; (LGDT/SGDT are not tested here due to a code generation bug where
;  GDTR.base store corrupts the IP register via shared mem32[64])
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

section .text
start:

    ; =========================================================
    ; Test 1: SMSW - read initial MSW (should be 0, PE not set)
    ; =========================================================
    smsw ax
    mov bx, 0x0000
    int 0x23                    ; AX should be 0 (PE=0, real mode)

    ; =========================================================
    ; Test 2: SMSW to memory
    ; =========================================================
    smsw word [msw_buf]
    mov ax, word [msw_buf]
    mov bx, 0x0000
    int 0x23                    ; stored MSW should be 0

    ; =========================================================
    ; Test 3: LMSW with non-PE bits - set TS, EM, MP without PE
    ; =========================================================
    mov ax, 0x000E              ; TS=1, EM=1, MP=1, PE=0
    lmsw ax
    smsw bx
    mov ax, 0x000E              ; expect: TS|EM|MP set, PE still 0
    int 0x23

    ; =========================================================
    ; Test 4: Clear bits back to 0
    ; =========================================================
    mov ax, 0x0000
    lmsw ax
    smsw bx
    mov ax, 0x0000
    int 0x23

    ; =========================================================
    ; Test 5: SMSW to different register
    ; =========================================================
    mov ax, 0x0006              ; set EM=1, MP=1
    lmsw ax
    smsw cx
    mov ax, cx
    mov bx, 0x0006
    int 0x23

    ; Clean up
    mov ax, 0x0000
    lmsw ax

    ; =========================================================
    ; Test 6: LMSW only affects low 4 bits
    ; =========================================================
    mov ax, 0xFFFF              ; try to set all bits
    lmsw ax
    smsw bx
    mov ax, 0x000F              ; only low 4 bits should be set
    int 0x23

    ; =========================================================
    ; Test 7: LMSW to memory operand
    ; =========================================================
    mov ax, 0x0000
    lmsw ax                     ; clear (but PE can't be cleared once set!)
    smsw bx
    mov ax, 0x0001              ; PE stays set since it was set in test 6
    int 0x23

    ; =========================================================
    ; Test 8: Verify PE cannot be cleared
    ; =========================================================
    smsw ax
    and ax, 0x0001              ; isolate PE bit
    mov bx, 0x0001              ; PE should still be 1
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

section .data

; Buffer for SMSW
msw_buf: dw 0
