; aad.asm — Thorough tests for AAD (ASCII Adjust before Division)
; Harness:
;   int 0x23: assert AX == BX   (zero-extended AL; AH must be 0 after AAD)
;   int 0x22: assert AL == AH   (used for flag-bit checks via stored FLAGS)
; Notes:
;   - AAD imm8: AL = (AH*imm8 + AL) & 0xFF ; AH = 0
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

; ===================== Base 10 (implicit 'aad') =====================

; 1) AH=00, AL=00 → 0
    mov ax, 0x0000
    aad                     ; default base 10
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 2) AH=00, AL=09 → 9
    mov ax, 0x0009
    aad
    SAVE_FLAGS
    ASSERT_AX 0x0009
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 3) AH=01, AL=00 → 10 (0x0A)
    mov ax, 0x0100
    aad
    SAVE_FLAGS
    ASSERT_AX 0x000A
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 4) AH=01, AL=05 → 15 (0x0F)
    mov ax, 0x0105
    aad
    SAVE_FLAGS
    ASSERT_AX 0x000F
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 5) AH=09, AL=09 → 99 (0x63)
    mov ax, 0x0909
    aad
    SAVE_FLAGS
    ASSERT_AX 0x0063
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 6) AH=02, AL=05 → 25 (0x19)  PF=0
    mov ax, 0x0205
    aad
    SAVE_FLAGS
    ASSERT_AX 0x0019
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 7) AH=08, AL=00 → 80 (0x50)
    mov ax, 0x0800
    aad
    SAVE_FLAGS
    ASSERT_AX 0x0050
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1

; 8) AH=0A, AL=09 (non-BCD tens) → 109 (0x6D)  PF=0
    mov ax, 0x0A09
    aad
    SAVE_FLAGS
    ASSERT_AX 0x006D
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 9) AH=FF, AL=FF → (2550+255)&FF = 0xF5  (SF=1, PF=1)
    mov ax, 0xFFFF
    aad
    SAVE_FLAGS
    ASSERT_AX 0x00F5
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 10) AH=80, AL=80 → 0x80  (SF=1, PF=0)
    mov ax, 0x8080
    aad
    SAVE_FLAGS
    ASSERT_AX 0x0080
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0


; ===================== Base 10 (explicit 'aad 10') =====================

; 11) AH=04, AL=02 → 42 (0x2A)  PF=0
    mov ax, 0x0402
    aad 10
    SAVE_FLAGS
    ASSERT_AX 0x002A
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 12) AH=00, AL=00 → 0
    mov ax, 0x0000
    aad 10
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ===================== Other bases (imm8 variants) =====================

; ---- Base 16 (hex “two nibbles to byte”) ----

; 13) AH=0x0C, AL=0x0D → 0xCD  (SF=1, PF=0)
    mov ax, 0x0C0D
    aad 16
    SAVE_FLAGS
    ASSERT_AX 0x00CD
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0

; 14) AH=0x0F, AL=0x0F → 0xFF  (SF=1, PF=1)
    mov ax, 0x0F0F
    aad 16
    SAVE_FLAGS
    ASSERT_AX 0x00FF
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 15) AH=0x08, AL=0x08 → 0x88  (SF=1, PF=1)
    mov ax, 0x0808
    aad 16
    SAVE_FLAGS
    ASSERT_AX 0x0088
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 16) AH=0x00, AL=0x00 → 0x00
    mov ax, 0x0000
    aad 16
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1

; 17) AH=0x01, AL=0x10 (non-nibble low) → 0x20  (PF=0)
    mov ax, 0x0110
    aad 16
    SAVE_FLAGS
    ASSERT_AX 0x0020
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 18) AH=0x20, AL=0x00 → (0x20*16)=0x200 → 0x00
    mov ax, 0x2000
    aad 16
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ---- Base 1 (AL := AH + AL) ----

; 19) AH=0x7E, AL=0x81 → 0xFF  (SF=1, PF=1)
    mov ax, 0x7E81
    aad 1
    SAVE_FLAGS
    ASSERT_AX 0x00FF
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 20) AH=0x80, AL=0x80 → 0x00  (wrap) (ZF=1, PF=1)
    mov ax, 0x8080
    aad 1
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ---- Base 0 (AL unchanged; AH cleared) ----

; 21) AH=0x55, AL=0xA5 → AL=0xA5 (SF=1, PF=1)
    mov ax, 0x55A5
    aad 0
    SAVE_FLAGS
    ASSERT_AX 0x00A5
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 22) AH=0xFF, AL=0x00 → AL=0x00
    mov ax, 0xFF00
    aad 0
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_PF 1


; ---- Base 255 (0xFF) ----

; 23) AH=0x02, AL=0x03 → (2*255+3)=513 → 0x01  (PF=0)
    mov ax, 0x0203
    aad 255
    SAVE_FLAGS
    ASSERT_AX 0x0001
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0


; ---- Base 100 (decimal-hundreds) ----

; 24) AH=0x01, AL=0x32 (50) → 150 (0x96)  (SF=1, PF=1)
    mov ax, 0x0132
    aad 100
    SAVE_FLAGS
    ASSERT_AX 0x0096
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ---- Base 12 (duodecimal) ----

; 25) AH=0x07, AL=0x0B → 7*12+11=95 (0x5F)  (SF=0, PF=1)
    mov ax, 0x070B
    aad 12
    SAVE_FLAGS
    ASSERT_AX 0x005F
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ---- Base 7 ----

; 26) AH=0x02, AL=0x06 → 2*7+6=20 (0x14)  (PF=1)
    mov ax, 0x0206
    aad 7
    SAVE_FLAGS
    ASSERT_AX 0x0014
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 1


; ===================== More base-10 “non-digit” stress =====================

; 27) AH=0x0A, AL=0x0A → 110 (0x6E)  (PF=0)
    mov ax, 0x0A0A
    aad
    SAVE_FLAGS
    ASSERT_AX 0x006E
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_PF 0

; 28) AH=0x55, AL=0x55 → 85*10+85=935 → 0xA7 (SF=1, PF=0)
    mov ax, 0x5555
    aad
    SAVE_FLAGS
    ASSERT_AX 0x00A7
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 0


; ===================== Extra hex-base edges =====================

; 29) AH=0x0A, AL=0x00 → 0xA0  (SF=1, PF=1)
    mov ax, 0x0A00
    aad 16
    SAVE_FLAGS
    ASSERT_AX 0x00A0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 30) AH=0x00, AL=0xFF (non-nibble) → 0xFF  (SF=1, PF=1)
    mov ax, 0x00FF
    aad 16
    SAVE_FLAGS
    ASSERT_AX 0x00FF
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1


; ===================== Non-decimal low digit under base-10 =====================

; 31) AH=0x00, AL=0xFF → 0xFF  (SF=1, PF=1)
    mov ax, 0x00FF
    aad
    SAVE_FLAGS
    ASSERT_AX 0x00FF
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_PF 1

; 32) AH=0x63, AL=0x9C → (99*10+156)=1146 → 0x7A (PF=0)
    mov ax, 0x639C
    aad
    SAVE_FLAGS
    ASSERT_AX 0x007A
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
