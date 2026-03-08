; lea.asm — Thorough tests for LEA (Load Effective Address) in 16-bit mode
; Harness:
;   int 0x23: assert AX == BX  (used for equality of registers/values)
;   int 0x22: assert AL == AH  (single-flag checks via saved FLAGS)
; Notes:
;   - LEA computes offset only; no memory access occurs.
;   - Flags are not modified by LEA (CF/PF/AF/ZF/SF preserved).
;   - Segment override prefixes do not affect LEA’s result.

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

%macro ASSERT_AX_EQ_BX 0
    int 0x23
%endmacro

%macro ASSERT_R16 2
    push ax
    mov ax, %1
    mov bx, %2
    int 0x23
    pop ax
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

; ---------- Start ----------
start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; Standard .COM: DS = CS, ES = DS
    push cs
    pop  ds
    push ds
    pop  es

; ===================== Base/index forms, no displacement =====================

; 1) LEA AX, [BX+SI] → AX = BX + SI
    mov bx, 0x1000
    mov si, 0x0010
    mov ah, [pat_all1]
    sahf
    lea ax, [bx+si]
    SAVE_FLAGS
    ASSERT_AX 0x1010
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 2) LEA DX, [BX+DI] → DX = BX + DI
    mov bx, 0x0200
    mov di, 0x0011
    mov ah, [pat_zf0]
    sahf
    lea dx, [bx+di]
    SAVE_FLAGS
    ASSERT_R16 dx, 0x0211
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; 3) LEA CX, [BP+SI] → CX = BP + SI
    mov bp, 0x0300
    mov si, 0x0020
    mov ah, [pat_all1]
    sahf
    lea cx, [bp+si]
    SAVE_FLAGS
    ASSERT_R16 cx, 0x0320
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 4) LEA AX, [BP+DI] → AX = BP + DI
    mov bp, 0x0100
    mov di, 0x0020
    mov ah, [pat_zf0]
    sahf
    lea ax, [bp+di]
    SAVE_FLAGS
    ASSERT_AX 0x0120
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; 5) LEA DX, [SI] → DX = SI
    mov si, 0x3456
    mov ah, [pat_all1]
    sahf
    lea dx, [si]
    SAVE_FLAGS
    ASSERT_R16 dx, 0x3456
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 6) LEA BX, [DI] → BX = DI
    mov di, 0x00FF
    mov ah, [pat_zf0]
    sahf
    lea bx, [di]
    SAVE_FLAGS
    ASSERT_R16 bx, 0x00FF
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; 7) LEA AX, [BP] (encodes as [BP+disp8=0]) → AX = BP
    mov bp, 0xABCD
    mov ah, [pat_all1]
    sahf
    lea ax, [bp]
    SAVE_FLAGS
    ASSERT_AX 0xABCD
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 8) LEA CX, [BX] → CX = BX
    mov bx, 0x7777
    mov ah, [pat_zf0]
    sahf
    lea cx, [bx]
    SAVE_FLAGS
    ASSERT_R16 cx, 0x7777
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== With disp8 (positive and negative) =====================

; 9) LEA DX, [BX+DI+7] → DX = BX + DI + 7
    mov bx, 0x0200
    mov di, 0x0011
    mov ah, [pat_all1]
    sahf
    lea dx, [bx+di+7]
    SAVE_FLAGS
    ASSERT_R16 dx, 0x0218
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 10) LEA CX, [BP+SI-5] → CX = BP + SI - 5  (disp8 = -5)
    mov bp, 0x0300
    mov si, 0x0020
    mov ah, [pat_zf0]
    sahf
    lea cx, [bp+si-5]
    SAVE_FLAGS
    ASSERT_R16 cx, 0x031B
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; 11) LEA BX, [DI-1] → BX = DI - 1
    mov di, 0x0100
    mov ah, [pat_all1]
    sahf
    lea bx, [di-1]
    SAVE_FLAGS
    ASSERT_R16 bx, 0x00FF
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 12) LEA AX, [BP+0] → AX = BP   (explicit disp8=0 encoding)
    mov bp, 0x4242
    mov ah, [pat_zf0]
    sahf
    lea ax, [bp+0]
    SAVE_FLAGS
    ASSERT_AX 0x4242
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== With disp16 =====================

; 13) LEA AX, [BP+DI+0x1234] → AX = BP + DI + 0x1234
    mov bp, 0x0100
    mov di, 0x0020
    mov ah, [pat_all1]
    sahf
    lea ax, [bp+di+0x1234]
    SAVE_FLAGS
    ASSERT_AX 0x1354
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 14) LEA CX, [BX+0x0200] → CX = BX + 0x0200
    mov bx, 0x0100
    mov ah, [pat_zf0]
    sahf
    lea cx, [bx+0x0200]
    SAVE_FLAGS
    ASSERT_R16 cx, 0x0300
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Direct [disp16] (no base/index) =====================

; 15) LEA AX, [label1] → AX = OFFSET(label1)
    mov ah, [pat_all1]
    sahf
    lea ax, [label1]
    SAVE_FLAGS
    mov bx, label1
    ASSERT_AX_EQ_BX
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 16) LEA DX, [label2+5] → DX = OFFSET(label2) + 5
    mov ah, [pat_zf0]
    sahf
    lea dx, [label2+5]
    SAVE_FLAGS
    mov bx, label2+5
    ; Compare DX (moved to AX in macro) with expected
    ASSERT_R16 dx, label2+5
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Segment overrides (ignored by LEA) =====================

; 17) ES override on [BX+SI+3] — should be identical to no prefix
    mov bx, 0x1111
    mov si, 0x0001
    mov ah, [pat_all1]
    sahf
    es lea ax, [bx+si+3]
    SAVE_FLAGS
    ASSERT_AX 0x1115
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 18) CS override on [DI] — identical result
    mov di, 0x0ABC
    mov ah, [pat_zf0]
    sahf
    cs lea cx, [di]
    SAVE_FLAGS
    ASSERT_R16 cx, 0x0ABC
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Wrap-around cases (mod 16-bit) =====================

; 19) Wrap: BX=FFFE, SI=0005, +3 → result = 0006
    mov bx, 0xFFFE
    mov si, 0x0005
    mov ah, [pat_all1]
    sahf
    lea dx, [bx+si+3]
    SAVE_FLAGS
    ASSERT_R16 dx, 0x0006
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 20) Underflow with negative disp8: BP=0003, SI=0000, -5 → result = FFFE
    mov bp, 0x0003
    mov si, 0x0000
    mov ah, [pat_zf0]
    sahf
    lea ax, [bp+si-5]
    SAVE_FLAGS
    ASSERT_AX 0xFFFE
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; ===================== Destination overlaps with base/index =====================

; 21) Dest = BX: BX ← [BX+SI+8]
    mov bx, 0x0100
    mov si, 0x0005
    mov ah, [pat_all1]
    sahf
    lea bx, [bx+si+8]        ; BX should become 0x0100+0x0005+8 = 0x010D
    SAVE_FLAGS
    ASSERT_R16 bx, 0x010D
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1

; 22) Dest = BP: BP ← [BP+DI-2]  (safe: LEA only reads BP)
    mov bp, 0x2200
    mov di, 0x0007
    mov ah, [pat_zf0]
    sahf
    lea bp, [bp+di-2]        ; 0x2200 + 7 - 2 = 0x2205
    SAVE_FLAGS
    ASSERT_R16 bp, 0x2205
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 1

; (We intentionally avoid LEA into SP to keep the interrupt-based asserts safe.)

; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

label1: db 0xCC,0xCC,0xCC
label2: db 0xCC,0xCC,0xCC,0xCC,0xCC,0xCC

; SAHF patterns (OF unaffected; LEA does not touch flags anyway)
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
