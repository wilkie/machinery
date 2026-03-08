; cbw.asm — Thorough tests for CBW (Convert Byte to Word)
; Harness:
;   int 0x23: assert AX == BX  (used for value/register equality)
;   int 0x22: assert AL == AH  (single-flag checks via saved FLAGS)

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
; DF isn’t controllable via SAHF; we verify it explicitly using the saved FLAGS
%macro CHECK_DF 1
    mov ax, [flags_store]
    mov cl, 10
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; Standard .COM: DS = CS, ES = DS
    push cs
    pop  ds
    push ds
    pop  es

; ===================== Non-negative AL → AH=00 =====================

; 1) AL=00h → AX=0000h; flags preserved; DF=0
    cld
    mov ax, 0xFF00           ; AH prefilled (should be overwritten to 00)
    mov al, 0x00
    mov ah, [pat_all1]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 2) AL=01h → AX=0001h
    mov ax, 0xFF00
    mov al, 0x01
    mov ah, [pat_zf0]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0x0001
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 3) AL=7Fh (127) → AX=007Fh
    mov ax, 0x1234
    mov al, 0x7F
    mov ah, [pat_all1]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0x007F
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 4) AL=08h (+8) → AX=0008h
    mov ax, 0x5678
    mov al, 0x08
    mov ah, [pat_zf0]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0x0008
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0


; ===================== Negative AL (bit7=1) → AH=FF =====================

; 5) AL=80h (−128) → AX=FF80h (boundary)
    mov ax, 0x0000
    mov al, 0x80
    mov ah, [pat_all1]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0xFF80
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 6) AL=FFh (−1) → AX=FFFFh
    mov ax, 0x0000
    mov al, 0xFF
    mov ah, [pat_zf0]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 7) AL=F0h (−16) → AX=FFF0h
    mov ax, 0xAAAA
    mov al, 0xF0
    mov ah, [pat_all1]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0xFFF0
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 8) AL=AAh (−86) with AH preloaded ≠ FF → AH must become FF, AX=FFAA
    mov ax, 0x12AA            ; AH=12 (will be overwritten)
    mov ah, [pat_zf0]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0xFFAA
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0


; ===================== Memory-sourced AL =====================

; 9) AL <- [pos_b]=0x33 → AX=0033
    mov ax, 0xF033
    mov al, [pos_b]
    mov ah, [pat_all1]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0x0033
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0

; 10) AL <- [neg_b]=0xC1 (−63) → AX=FFC1
    mov ax, 0x00C1
    mov al, [neg_b]
    mov ah, [pat_zf0]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0xFFC1
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    CHECK_DF 0


; ===================== DF = 1 (unchanged by CBW) =====================

; 11) DF=1, AL=05h → AX=0005; DF must remain 1
    std
    mov ax, 0x9905
    mov ah, [pat_all1]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0x0005
    CHECK_DF 1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    cld

; 12) DF=1, AL=E7h (−25) → AX=FFE7; DF must remain 1
    std
    mov ax, 0x77E7
    mov ah, [pat_zf0]
    sahf
    cbw
    SAVE_FLAGS
    ASSERT_AX 0xFFE7
    CHECK_DF 1
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1
    cld


; ===================== Idempotence / repeat =====================

; 13) Repeating CBW with AL=22h → still AX=0022
    mov ax, 0xAA22
    mov ah, [pat_all1]
    sahf
    cbw
    cbw
    SAVE_FLAGS
    ASSERT_AX 0x0022
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 14) Repeating CBW with AL=9Bh (−101) → still AX=FF9B
    mov ax, 0x009B
    mov ah, [pat_zf0]
    sahf
    cbw
    cbw
    SAVE_FLAGS
    ASSERT_AX 0xFF9B
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0
pos_b:      db 0x33, 0xCC
neg_b:      db 0xC1, 0xCC

; --------- SAHF flag seed patterns (OF unaffected; CBW doesn’t change any flags anyway) ---------
pat_all1: db 0xD5            ; SF=1 ZF=1 AF=1 PF=1 CF=1
pat_zf0:  db 0x95            ; SF=1 ZF=0 AF=1 PF=1 CF=1

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
