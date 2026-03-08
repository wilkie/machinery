; daa.asm — Thorough tests for DAA (Decimal Adjust AL after Addition)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Notes:
;   - Only AL is modified by DAA.
;   - CF/AF follow the two-step BCD adjustment rules (see comments per test).
;   - SF/ZF/PF reflect the final AL. OF is undefined → we do not check it.

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

; ===================== Basic “no-adjust” cases (CF=0, AF=0) =====================

; 1) AL=00 -> 00 ; CF=0 AF=0 ; ZF=1 SF=0 PF=1
    mov al, 0x00
    mov ah, [pat0]
    sahf
    daa
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
    daa
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
    daa
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
    daa
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
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x99
    CHECK_CF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ===================== Low-nibble adjust (add +0x06; AF=1) =====================

; 6) AL=0A -> 10 ; CF=0 AF=1 ; ZF=0 SF=0 PF=0
    mov al, 0x0A
    mov ah, [pat0]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x10
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 7) AL=1A -> 20 ; CF=0 AF=1 ; ZF=0 SF=0 PF=0
    mov al, 0x1A
    mov ah, [pat0]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x20
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 8) AL=8A -> 90 ; CF=0 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0x8A
    mov ah, [pat0]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x90
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ===================== Low-nibble adjust forced by AF_in=1 =====================

; 9) AF_in=1, AL=13 -> 19 ; CF=0 AF=1 ; ZF=0 SF=0 PF=0
    mov al, 0x13
    mov ah, [patA]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x19
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 10) AF_in=1, AL=89 -> 8F ; CF=0 AF=1 ; ZF=0 SF=1 PF=0
    mov al, 0x89
    mov ah, [patA]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x8F
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0


; ===================== Upper-digit adjust only (old_CF=1; +0x60; CF=1) =====================

; 11) CF_in=1, AL=00 -> 60 ; CF=1 AF=0 ; ZF=0 SF=0 PF=1
    mov al, 0x00
    mov ah, [patC]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x60
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 12) CF_in=1, AL=12 -> 72 ; CF=1 AF=0 ; ZF=0 SF=0 PF=1
    mov al, 0x12
    mov ah, [patC]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x72
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ===================== Upper-digit adjust only (old_AL > 99h; +0x60; CF=1) =====================

; 13) AL=A0 -> 00 ; CF=1 AF=0 ; ZF=1 SF=0 PF=1
    mov al, 0xA0
    mov ah, [pat0]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== Both adjustments (+0x66; CF=1 AF=1 when low-nibble condition true) =====================

; 14) AL=9A -> 00 ; CF=1 AF=1 ; ZF=1 SF=0 PF=1
    mov al, 0x9A
    mov ah, [pat0]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x00
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 15) AL=FA -> 60 ; CF=1 AF=1 ; ZF=0 SF=0 PF=1
    mov al, 0xFA
    mov ah, [pat0]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x60
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 16) CF_in=1, AL=99 -> F9 ; CF=1 AF=0 ; ZF=0 SF=1 PF=1
    mov al, 0x99
    mov ah, [patC]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0xF9
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ===================== Intel doc examples reproduced =====================

; 17) Example: AL=AE -> 14 ; CF=1 AF=1 ; ZF=0 SF=0 PF=1
    mov al, 0xAE
    mov ah, [pat0]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x14
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 18) Example: AL=2E -> 34 ; CF=0 AF=1 ; ZF=0 SF=0 PF=0
    mov al, 0x2E
    mov ah, [pat0]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x34
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0


; ===================== Mixed AF/CF inputs =====================

; 19) CF_in=1, AF_in=1, AL=93 -> F9 ; CF=1 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0x93
    mov ah, [patAC]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0xF9
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 20) AF_in=1, AL=00 -> 06 ; CF=0 AF=1 ; ZF=0 SF=0 PF=1
    mov al, 0x00
    mov ah, [patA]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x06
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 21) AF_in=1, AL=90 -> 96 ; CF=0 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0x90
    mov ah, [patA]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x96
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 22) AF_in=1, AL=99 -> 9F ; CF=0 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0x99
    mov ah, [patA]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x9F
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ===================== More edges around 0x99..0xA? =====================

; 23) AL=9F -> 05 ; CF=1 AF=1 ; ZF=0 SF=0 PF=1
    mov al, 0x9F
    mov ah, [pat0]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x05
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 24) AL=A9 -> 09 ; CF=1 AF=0 ; ZF=0 SF=0 PF=1
    mov al, 0xA9
    mov ah, [pat0]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x09
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ===================== Heavier combinations =====================

; 25) AF_in=1, AL=B5 -> 1B ; CF=1 AF=1 ; ZF=0 SF=0 PF=1
    mov al, 0xB5
    mov ah, [patA]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x1B
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 26) CF_in=1, AL=F0 -> 50 ; CF=1 AF=0 ; ZF=0 SF=0 PF=1
    mov al, 0xF0
    mov ah, [patC]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x50
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 27) AL=F9 -> 59 ; CF=1 AF=0 ; ZF=0 SF=0 PF=1
    mov al, 0xF9
    mov ah, [pat0]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x59
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 28) CF_in=1, AL=8F -> F5 ; CF=1 AF=1 ; ZF=0 SF=1 PF=1
    mov al, 0x8F
    mov ah, [patC]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0xF5
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 29) CF_in=1 & AF_in=1, AL=00 -> 66 ; CF=1 AF=1 ; ZF=0 SF=0 PF=1
    mov al, 0x00
    mov ah, [patAC]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x66
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 30) AF_in=1, AL=06 -> 0C ; CF=0 AF=1 ; ZF=0 SF=0 PF=1
    mov al, 0x06
    mov ah, [patA]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x0C
    CHECK_CF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 31) CF_in=1, AL=6A -> D0 ; CF=1 AF=1 ; ZF=0 SF=1 PF=0
    mov al, 0x6A
    mov ah, [patC]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0xD0
    CHECK_CF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 32) CF_in=1, AL=03 -> 63 ; CF=1 AF=0 ; ZF=0 SF=0 PF=1
    mov al, 0x03
    mov ah, [patC]
    sahf
    daa
    SAVE_FLAGS
    ASSERT_BYTE 0x63
    CHECK_CF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; SAHF patterns to preset CF/AF (SF/ZF/PF bits here are irrelevant; DAA sets them from AL)
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
