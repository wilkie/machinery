; aam.asm — Thorough tests for AAM (ASCII Adjust after Multiplication)
; Harness:
;   int 0x23: assert AX == BX   (we compare full AX = (AH:AL))
;   int 0x22: assert AL == AH   (used for flag-bit checks via stored FLAGS)
; Notes:
;   - AAM imm8: AH = AL / imm8 ; AL = AL % imm8
;   - SF/ZF/PF set from AL; AF/CF/OF undefined (not checked)

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

%macro ASSERT_AX 1
    mov bx, %1
    int 0x23
%endmacro

%macro CHECK_PF 1
    mov ax, [flags_store]
    mov cl, 2
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

; ===================== Base 10 (implicit 'aam') =====================

; 1) AL=00 -> AH=0, AL=0
    mov ax, 0x0000
    aam                     ; default base 10
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 2) AL=09 -> AH=0, AL=9
    mov ax, 0x0009
    aam
    SAVE_FLAGS
    ASSERT_AX 0x0009
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 3) AL=10(0x0A) -> AH=1, AL=0
    mov ax, 0x000A
    aam
    SAVE_FLAGS
    ASSERT_AX 0x0100
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 4) AL=15(0xF) -> AH=1, AL=5
    mov ax, 0x000F
    aam
    SAVE_FLAGS
    ASSERT_AX 0x0105
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 5) AL=99 -> AH=9, AL=9
    mov ax, 0x0063          ; AL=99
    aam
    SAVE_FLAGS
    ASSERT_AX 0x0909
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 6) AL=25 -> AH=2, AL=5
    mov ax, 0x0019
    aam
    SAVE_FLAGS
    ASSERT_AX 0x0205
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 7) AL=80 -> AH=8, AL=0
    mov ax, 0x0050
    aam
    SAVE_FLAGS
    ASSERT_AX 0x0800
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 8) AL=100 -> AH=10, AL=0
    mov ax, 0x0064
    aam
    SAVE_FLAGS
    ASSERT_AX 0x0A00
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 9) AL=127 -> AH=12, AL=7
    mov ax, 0x007F
    aam
    SAVE_FLAGS
    ASSERT_AX 0x0C07
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 10) AL=128 -> AH=12, AL=8
    mov ax, 0x0080
    aam
    SAVE_FLAGS
    ASSERT_AX 0x0C08
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 11) AL=200 -> AH=20, AL=0
    mov ax, 0x00C8
    aam
    SAVE_FLAGS
    ASSERT_AX 0x1400
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 12) AL=255 -> AH=25, AL=5
    mov ax, 0x00FF
    aam
    SAVE_FLAGS
    ASSERT_AX 0x1905
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ===================== Base 10 (explicit 'aam 10') =====================

; 13) AL=42 -> AH=4, AL=2
    mov ax, 0x002A
    aam 10
    SAVE_FLAGS
    ASSERT_AX 0x0402
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 14) AL=00 -> AH=0, AL=0
    mov ax, 0x0000
    aam 10
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== Base 16 (nibble split) =====================

; 15) AL=0xCD → AH=0x0C, AL=0x0D
    mov ax, 0x00CD
    aam 16
    SAVE_FLAGS
    ASSERT_AX 0x0C0D
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 16) AL=0xFF → AH=0x0F, AL=0x0F
    mov ax, 0x00FF
    aam 16
    SAVE_FLAGS
    ASSERT_AX 0x0F0F
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 17) AL=0x80 → AH=0x08, AL=0x00
    mov ax, 0x0080
    aam 16
    SAVE_FLAGS
    ASSERT_AX 0x0800
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 18) AL=0x00 → AH=0x00, AL=0x00
    mov ax, 0x0000
    aam 16
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 19) AL=0x01 → AH=0x00, AL=0x01
    mov ax, 0x0001
    aam 16
    SAVE_FLAGS
    ASSERT_AX 0x0001
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0


; ===================== Base 12 (duodecimal) =====================

; 20) AL=95 → 7 * 12 + 11
    mov ax, 0x005F
    aam 12
    SAVE_FLAGS
    ASSERT_AX 0x070B
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 21) AL=47 → 3 * 12 + 11
    mov ax, 0x002F
    aam 12
    SAVE_FLAGS
    ASSERT_AX 0x030B
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 22) AL=36 → 3 * 12 + 0
    mov ax, 0x0024
    aam 12
    SAVE_FLAGS
    ASSERT_AX 0x0300
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== Base 7 =====================

; 23) AL=20 → 2 * 7 + 6
    mov ax, 0x0014
    aam 7
    SAVE_FLAGS
    ASSERT_AX 0x0206
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 24) AL=6 → 0 * 7 + 6
    mov ax, 0x0006
    aam 7
    SAVE_FLAGS
    ASSERT_AX 0x0006
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ===================== Base 2 =====================

; 25) AL=0xFF → 127 * 2 + 1
    mov ax, 0x00FF
    aam 2
    SAVE_FLAGS
    ASSERT_AX 0x7F01
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 26) AL=0x80 → 64 * 2 + 0
    mov ax, 0x0080
    aam 2
    SAVE_FLAGS
    ASSERT_AX 0x4000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== Base 100 =====================

; 27) AL=150 → 1 * 100 + 50 (0x32)
    mov ax, 0x0096
    aam 100
    SAVE_FLAGS
    ASSERT_AX 0x0132
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 28) AL=200 → 2 * 100 + 0
    mov ax, 0x00C8
    aam 100
    SAVE_FLAGS
    ASSERT_AX 0x0200
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== Base 255 =====================

; 29) AL=0xF5 (245) → 0 * 255 + 245
    mov ax, 0x00F5
    aam 255
    SAVE_FLAGS
    ASSERT_AX 0x00F5
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 30) AL=0xFF (255) → 1 * 255 + 0
    mov ax, 0x00FF
    aam 255
    SAVE_FLAGS
    ASSERT_AX 0x0100
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== Base 3 =====================

; 31) AL=255 → 85 * 3 + 0
    mov ax, 0x00FF
    aam 3
    SAVE_FLAGS
    ASSERT_AX 0x5500
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 32) AL=254 → 84 * 3 + 2
    mov ax, 0x00FE
    aam 3
    SAVE_FLAGS
    ASSERT_AX 0x5402
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

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
