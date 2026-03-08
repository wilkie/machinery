; xchg.asm — Thorough tests for XCHG (r8/r16 & r/m, DF both ways, overrides, odd addrs)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH   (single-flag checks after SAVE_FLAGS)
; Notes:
;   - XCHG does not modify flags (SF/ZF/AF/PF/CF/OF unchanged).
;   - No mem<->mem form. AX has short-encoding forms with r16.
;   - We never touch SP as an operand to keep the stack/INTs safe.

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

%macro ASSERT_R8 2
    push ax
    mov al, %1         ; reg → AL
    xor ah, ah
    mov bx, %2
    int 0x23
    pop ax
%endmacro

%macro ASSERT_R16 2
    push ax
    mov ax, %1         ; reg → AX
    mov bx, %2
    int 0x23
    pop ax
%endmacro

%macro ASSERT_SI 1
    mov ax, si
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_DI 1
    mov ax, di
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_BX 1
    mov ax, bx
    mov bx, %1
    int 0x23
%endmacro

%macro ASSERT_MEMB 2
    mov al, [%1]
    xor ah, ah
    mov bx, %2
    int 0x23
%endmacro
%macro ASSERT_MEMW 2
    mov ax, [%1]
    mov bx, %2
    int 0x23
%endmacro

; Flag-bit checks (we only check preservation)
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

; ---------- Begin ----------
start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; Standard .COM: DS = CS, ES = DS
    push cs
    pop  ds
    push ds
    pop  es

; ===================== r8 <-> r8 =====================

; 1) AL <-> AH (classic byte swap within AX)
    mov ah, [pat_all1]
    sahf
    mov ax, 0x1234           ; AH=0x12 AL=0x34
    xchg al, ah
    SAVE_FLAGS
    ASSERT_AX 0x3412
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 2) BL <-> CL
    mov bl, 0x55
    mov cl, 0xEE
    mov ah, [pat_zf0]
    sahf
    xchg bl, cl
    SAVE_FLAGS
    ASSERT_R8 bl, 0x00EE
    ASSERT_R8 cl, 0x0055
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 3) DL <-> DL (self-exchange no-op)
    mov dl, 0xA5
    mov ah, [pat_all1]
    sahf
    xchg dl, dl
    SAVE_FLAGS
    ASSERT_R8 dl, 0x00A5
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== r16 <-> r16 =====================

; 4) AX <-> BX  (AX short-encoding variant)
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x1111
    mov bx, 0x2222
    xchg ax, bx
    SAVE_FLAGS
    push bx
    ASSERT_R16 ax, 0x2222
    pop bx
    ASSERT_R16 bx, 0x1111
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 5) SI <-> DI
    mov si, 0xABCD
    mov di, 0x0F0E
    mov ah, [pat_all1]
    sahf
    xchg si, di
    SAVE_FLAGS
    ASSERT_R16 si, 0x0F0E
    ASSERT_R16 di, 0xABCD
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 6) CX <-> CX (self-exchange no-op)
    mov cx, 0xDEAD
    mov ah, [pat_zf0]
    sahf
    xchg cx, cx
    SAVE_FLAGS
    ASSERT_R16 cx, 0xDEAD
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== r8 <-> r/m8 =====================

; 7) [m8_a] <-> AL
    mov al, 0x3C
    mov ah, [pat_all1]
    sahf
    xchg byte [m8_a], al
    SAVE_FLAGS
    ASSERT_R8 al,      0x00A1
    ASSERT_MEMB m8_a,  0x003C
    ASSERT_MEMB m8_a+1,0x00CC   ; sentinel intact
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 8) BL <-> [m8_b]
    mov bl, 0x55
    mov ah, [pat_zf0]
    sahf
    xchg bl, [m8_b]
    SAVE_FLAGS
    ASSERT_R8 bl,      0x00EE
    ASSERT_MEMB m8_b,  0x0055
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 9) DF=1 (irrelevant): [m8_c] <-> AL
    std
    mov al, 0x7E
    mov ah, [pat_all1]
    sahf
    xchg [m8_c], al
    SAVE_FLAGS
    ASSERT_R8 al,      0x002B
    ASSERT_MEMB m8_c,  0x007E
    cld
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== r16 <-> r/m16 =====================

; 10) AX <-> [mw1]
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x1234
    xchg ax, [mw1]                 ; mw1 initially 0xDEAD
    SAVE_FLAGS
    ASSERT_R16 ax,     0xDEAD
    ASSERT_MEMW mw1,   0x1234
    ASSERT_MEMB mw1+2, 0x007E
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 11) [mw2] <-> DX
    mov dx, 0xABCD
    mov ah, [pat_all1]
    sahf
    xchg [mw2], dx                  ; mw2 initially 0x0F0E
    SAVE_FLAGS
    ASSERT_R16 dx,     0x0F0E
    ASSERT_MEMW mw2,   0xABCD
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 12) Odd address: [mw_odd+1] (0xBEEF) <-> DX (0x6060)
    mov dx, 0x6060
    mov ah, [pat_zf0]
    sahf
    xchg [mw_odd+1], dx
    SAVE_FLAGS
    ASSERT_R16 dx,       0xBEEF
    ASSERT_MEMW mw_odd+1,0x6060
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 13) Indexed: AX <-> [BX+SI+disp]
    mov bx, mwi_base
    mov si, 3
    mov ah, [pat_all1]
    sahf
    mov ax, 0xFACE
    xchg [bx+si+2], ax            ; mwi_base+5 word is 0x1357
    SAVE_FLAGS
    push bx
    ASSERT_R16 ax,         0x1357
    ASSERT_MEMW mwi_base+5,0xFACE
    pop bx
    ASSERT_BX  mwi_base          ; indexes unchanged
    ASSERT_SI  3
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== Segment override on memory operand =====================

; 14) ES override byte: ES:[esb] <-> AH
    mov ah, [pat_zf0]
    sahf
    mov ah, 0x88
    es xchg byte [esb], ah        ; esb initially 0x77
    SAVE_FLAGS
    ASSERT_R8 ah,     0x0077
    ASSERT_MEMB esb,  0x0088
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 15) ES override word: BX <-> ES:[esw]
    mov bx, 0x4242
    mov ah, [pat_all1]
    sahf
    es xchg word [esw], bx        ; esw initially 0x1BAD
    SAVE_FLAGS
    ASSERT_R16 bx,    0x1BAD
    ASSERT_MEMW esw,  0x4242
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== Equal-value exchanges (no visible change, still valid) =====================

; 16) AL == [m8_same] initially
    mov al, 0x5A
    mov ah, [pat_zf0]
    sahf
    xchg [m8_same], al            ; both 0x5A
    SAVE_FLAGS
    ASSERT_R8 al,       0x005A
    ASSERT_MEMB m8_same,0x005A
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 17) AX == [mw_same] initially
    mov ah, [pat_all1]
    sahf
    mov ax, 0xCAFE
    xchg [mw_same], ax            ; both 0xCAFE
    SAVE_FLAGS
    ASSERT_R16 ax,      0xCAFE
    ASSERT_MEMW mw_same,0xCAFE
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== Another AX short-encoding sample =====================

; 18) AX <-> SI
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x0A0B
    mov si, 0x0C0D
    xchg ax, si
    SAVE_FLAGS
    ASSERT_R16 ax, 0x0C0D
    ASSERT_R16 si, 0x0A0B
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== Exit =====================
exit:
    mov ax, 0x4C00
    int 0x21


; ---------------- Data ----------------
flags_store: dw 0

; Byte memory
m8_a:     db 0xA1, 0xCC
m8_b:     db 0xEE, 0xCC
m8_c:     db 0x2B, 0xCC
m8_same:  db 0x5A, 0xCC

; Word memory
mw1:      dw 0xDEAD, 0x7E7E
mw2:      dw 0x0F0E, 0x7E7E
mw_same:  dw 0xCAFE, 0x7E7E

; Odd-address word area: word 0xBEEF at +1
mw_odd:   db 0xCC, 0xEF, 0xBE, 0xCC

; Indexed base area:
; Offsets: +0..+1 = sentinel, +3..+4 = 0x2468, +5..+6 = 0x1357
mwi_base: db 0xCC,0xCC,0x00
          dw 0x2468, 0x1357
          db 0xCC

; ES override buffers (ES == DS here)
esb:      db 0x77, 0xCC
esw:      dw 0x1BAD, 0x7E7E

; SAHF flag seed patterns (OF unaffected by SAHF, but XCHG doesn't change any flags anyway)
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
