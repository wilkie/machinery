; das.asm — Thorough tests for DAS (Decimal Adjust AL after Subtraction)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   - Only AL is modified by DAS.
;   - CF/AF per 2-step BCD correction (see comments).
;   - SF/ZF/PF from final AL. OF undefined → not checked.

bits 16
org 0x100

; ---------- Macros ----------
%macro SAVE_FLAGS 0
    push ax
    pushf
    pop  ax
    mov  [flags_store], ax
    pop  ax
%endmacro

%macro ASSERT_BYTE 1
    xor ah, ah
    mov bx, %1
    int 0x23
%endmacro

%macro CHECK_CF 1
    mov ax, [flags_store]
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

%macro CHECK_PF 1
    mov ax, [flags_store]
    mov cl, 2
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

%macro CHECK_AF 1
    mov ax, [flags_store]
    mov cl, 4
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

%macro CHECK_ZF 1
    mov ax, [flags_store]
    mov cl, 6
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

%macro CHECK_SF 1
    mov ax, [flags_store]
    mov cl, 7
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro


start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; Standard .COM: DS = CS
    push cs
    pop  ds

; ---------------- Basic “no-adjust” cases (CF=0, AF=0) ----------------

; 1) AL=00 -> 00 ; CF=0 AF=0 ; ZF=1 SF=0 PF=1
    mov al, 0x00
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_AF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 2) AL=09 -> 09 ; CF=0 AF=0 ; ZF=0 SF=0 PF=1
    mov al, 0x09
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x09
    CHECK_CF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 3) AL=12 -> 12 ; CF=0 AF=0 ; ZF=0 SF=0 PF=1
    mov al, 0x12
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x12
    CHECK_CF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 4) AL=90 -> 90 ; CF=0 AF=0 ; ZF=0 SF=1 PF=1
    mov al, 0x90
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x90
    CHECK_CF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 5) AL=99 -> 99 ; CF=0 AF=0 ; ZF=0 SF=1 PF=1
    mov al, 0x99
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x99
    CHECK_CF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ---------------- Low-nibble adjust only (LSN > 9) ----------------

; 6) AL=0A -> 04 ; CF=0 AF=1 ; ZF=0 SF=0 PF=0
    mov al, 0x0A
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x04
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 7) AL=1A -> 14 ; CF=0 AF=1 ; ZF=0 SF=0 PF=1
    mov al, 0x1A
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x14
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 8) AL=8A -> 84 ; CF=0 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0x8A
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x84
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ---------------- Low-nibble adjust forced by AF_in=1 ----------------

; 9) AF_in=1, AL=13 -> 0D ; CF=0 AF=1 ; ZF=0 SF=0 PF=0
    mov al, 0x13
    mov ah, [patA]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x0D
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 10) AF_in=1, AL=89 -> 83 ; CF=0 AF=1 ; ZF=0 SF=1 PF=0
    mov al, 0x89
    mov ah, [patA]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x83
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0


; ---------------- Upper-digit adjust only (CF_in=1) ----------------

; 11) CF_in=1, AL=00 -> A0 ; CF=1 AF=0 ; ZF=0 SF=1 PF=1
    mov al, 0x00
    mov ah, [patC]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0xA0
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 12) CF_in=1, AL=12 -> B2 ; CF=1 AF=0 ; ZF=0 SF=1 PF=1
    mov al, 0x12
    mov ah, [patC]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0xB2
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ---------------- Upper-digit adjust only (old_AL > 0x99) ----------------

; 13) AL=A0 -> 40 ; CF=1 AF=0 ; ZF=0 SF=0 PF=0
    mov al, 0xA0
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x40
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 14) AL=F0 -> 90 ; CF=1 AF=0 ; ZF=0 SF=1 PF=1
    mov al, 0xF0
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x90
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ---------------- Both adjustments (LSN>9 and old_AL>0x99) ----------------

; 15) AL=9A -> 34 ; CF=1 AF=1 ; ZF=0 SF=0 PF=0
    mov al, 0x9A
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x34
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 16) AL=FA -> 94 ; CF=1 AF=1 ; ZF=0 SF=1 PF=0
    mov al, 0xFA
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x94
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 17) AL=9B -> 35 ; CF=1 AF=1 ; ZF=0 SF=0 PF=1
    mov al, 0x9B
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x35
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 18) AL=FB -> 95 ; CF=1 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0xFB
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x95
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ---------------- Mixed AF/CF inputs ----------------

; 19) AF_in=1, AL=00 -> FA ; CF=0 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0x00
    mov ah, [patA]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0xFA
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 20) AF_in=1, AL=90 -> 8A ; CF=0 AF=1 ; ZF=0 SF=1 PF=0
    mov al, 0x90
    mov ah, [patA]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x8A
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 21) AF_in=1, AL=99 -> 93 ; CF=0 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0x99
    mov ah, [patA]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x93
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 22) CF_in=1 & AF_in=1, AL=00 -> 9A ; CF=1 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0x00
    mov ah, [patAC]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x9A
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 23) CF_in=1, AL=F9 -> 99 ; CF=1 AF=0 ; ZF=0 SF=1 PF=1
    mov al, 0xF9
    mov ah, [patC]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x99
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 24) CF_in=1, AL=8F -> 29 ; CF=1 AF=1 ; ZF=0 SF=0 PF=0
    mov al, 0x8F
    mov ah, [patC]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x29
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0


; ---------------- Edges near zero/round numbers ----------------

; 25) AF_in=1, AL=06 -> 00 ; CF=0 AF=1 ; ZF=1 SF=0 PF=1
    mov al, 0x06
    mov ah, [patA]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 26) AF_in=1, AL=66 -> 60 ; CF=0 AF=1 ; ZF=0 SF=0 PF=1
    mov al, 0x66
    mov ah, [patA]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x60
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 27) CF_in=1, AL=70 -> 10 ; CF=1 AF=0 ; ZF=0 SF=0 PF=0
    mov al, 0x70
    mov ah, [patC]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x10
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 28) CF_in=1, AL=01 -> A1 ; CF=1 AF=0 ; ZF=0 SF=1 PF=0
    mov al, 0x01
    mov ah, [patC]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0xA1
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 29) CF_in=1 & AF_in=1, AL=10 -> AA ; CF=1 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0x10
    mov ah, [patAC]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0xAA
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 30) AL=FF -> 99 ; CF=1 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0xFF
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x99
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 31) AL=9F -> 39 ; CF=1 AF=1 ; ZF=0 SF=0 PF=1
    mov al, 0x9F
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x39
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 32) AL=A9 -> 49 ; CF=1 AF=0 ; ZF=0 SF=0 PF=0
    mov al, 0xA9
    mov ah, [pat0]
    sahf
    das
    SAVE_FLAGS
    ASSERT_BYTE 0x49
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0


; ---------------- Exit to DOS ----------------
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; SAHF patterns to preset CF/AF (SF/ZF/PF bits here are irrelevant; DAS sets them from AL)
pat0:  db 0x00        ; CF=0 AF=0
patC:  db 0x01        ; CF=1 AF=0
patA:  db 0x10        ; CF=0 AF=1
patAC: db 0x11        ; CF=1 AF=1

db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
db 0x0, 0x0, 0x0, 0x0
stack:
