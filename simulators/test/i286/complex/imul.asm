; imul.asm — thorough IMUL tests (one-operand form) with saved FLAGS and DOS exit
; Harness:
;   int 0x22: assert AL == AH
;   int 0x23: assert AX == BX
;
; Strategy:
;   - After each IMUL, SAVE_FLAGS copies FLAGS to [flags_store].
;   - Assert AX (and DX when 16-bit) with int 0x23.
;   - CF/OF checks read [flags_store] to avoid clobber from interrupts.
;   - IMUL one-operand semantics: CF==OF and indicate whether upper half is
;     the sign-extension of the lower half (0 => sign-extended; 1 => not).

bits 16
org 0x100

; ------------- Macros -------------

%macro SAVE_FLAGS 0
    push ax
    pushf
    pop ax
    mov [flags_store], ax
    pop ax
%endmacro

%macro ASSERT_AX 1
    mov bx, %1
    int 0x23
%endmacro

%macro ASSERT_DX 1
    mov ax, dx
    mov bx, %1
    int 0x23
%endmacro

%macro CHECK_CF 1
    mov ax, [flags_store]
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

%macro CHECK_OF 1
    mov ax, [flags_store]
    mov cl, 11
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro

%macro CHECK_CF_EQ_OF 0
    mov ax, [flags_store]
    mov dx, ax
    and dl, 1          ; DL = CF
    mov cl, 11
    shr ax, cl
    and al, 1          ; AL = OF
    mov ah, al         ; AH = OF
    mov al, dl         ; AL = CF
    int 0x22
%endmacro

start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; Standard .COM: DS=CS
    push cs
    pop ds

; ===================== 8-bit IMUL tests (AL * r/m8 -> AX) =====================

; 1) 00 * 7D = 0000, CF=OF=0
    mov al, 0x00
    mov bl, 0x7D
    imul bl
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 2) 01 * 7F = 007F, CF=OF=0
    mov al, 0x01
    mov bl, 0x7F
    imul bl
    SAVE_FLAGS
    ASSERT_AX 0x007F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 3) 02 * 40 = 0080, CF=OF=1 (doesn't sign-extend)
    mov al, 0x02
    mov bl, 0x40
    imul bl
    SAVE_FLAGS
    ASSERT_AX 0x0080
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 4) FF(-1) * 02 = FFFE(-2), CF=OF=0 (sign-extends)
    mov al, 0xFF
    mov bl, 0x02
    imul bl
    SAVE_FLAGS
    ASSERT_AX 0xFFFE
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 5) 80(-128) * 02 = FF00(-256), CF=OF=1
    mov al, 0x80
    mov bl, 0x02
    imul bl
    SAVE_FLAGS
    ASSERT_AX 0xFF00
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 6) FE(-2) * FE(-2) = 0004, CF=OF=0
    mov al, 0xFE
    mov bl, 0xFE
    imul bl
    SAVE_FLAGS
    ASSERT_AX 0x0004
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 7) 80(-128) * [bx+si+disp]=FF(-1) => 0080 (+128), CF=OF=1
    lea bx, [base8]
    lea si, [index8]
    mov al, 0x80
    imul byte [bx+si+var8d-base8-index8]
    SAVE_FLAGS
    ASSERT_AX 0x0080
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 8) 7F * [bp] (ds:)=01 => 007F, CF=OF=0
    mov bp, var8c
    mov al, 0x7F
    imul byte [ds:bp]
    SAVE_FLAGS
    ASSERT_AX 0x007F
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 9) F0(-16) * [si+disp]=08 => FF80(-128), CF=OF=0
    lea si, [base8sd]
    mov al, 0xF0
    imul byte [si + (var8e - base8sd)]
    SAVE_FLAGS
    ASSERT_AX 0xFF80
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 10) 30 * [di]=07 => 0150, CF=OF=1
    lea di, [var8f]
    mov al, 0x30
    imul byte [di]
    SAVE_FLAGS
    ASSERT_AX 0x0150
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 11) 10 * [bp+disp] (ds:)=10 => 0100, CF=OF=1
    lea bp, [base_bp_d]
    mov al, 0x10
    imul byte [ds:bp + (var8l - base_bp_d)]
    SAVE_FLAGS
    ASSERT_AX 0x0100
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 12) 09 * [bx]=11 => 0099, CF=OF=0
    lea bx, [var8h]
    mov al, 0x09
    imul byte [bx]
    SAVE_FLAGS
    ASSERT_AX 0x0099
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 13) 03 * [bx+disp]=33 => 0099, CF=OF=0
    lea bx, [base8bd]
    mov al, 0x03
    imul byte [bx + (var8i - base8bd)]
    SAVE_FLAGS
    ASSERT_AX 0x0099
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 14) 55 * [bp+si+disp] (ds:)=06 => 01FE, CF=OF=1
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov al, 0x55
    imul byte [ds:bp+si + (var8j - base_bp_si_A - base_bp_si_B)]
    SAVE_FLAGS
    ASSERT_AX 0x01FE
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 15) 30 * [bp+di+disp] (ds:)=02 => 0060, CF=OF=0
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov al, 0x30
    imul byte [ds:bp+di + (var8k - base_bp_di_A - base_bp_di_B)]
    SAVE_FLAGS
    ASSERT_AX 0x0060
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 16) FF(-1) * [di+disp]=FF(-1) => 0001, CF=OF=0
    lea di, [base8dd]
    mov al, 0xFF
    imul byte [di + (var8g - base8dd)]
    SAVE_FLAGS
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF


; ===================== 16-bit IMUL tests (AX * r/m16 -> DX:AX) =====================

; 17) 0000 * 7FFF => 0000:0000, CF=OF=0
    xor ax, ax
    mov bx, 0x7FFF
    imul bx
    SAVE_FLAGS
    ASSERT_AX 0x0000
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 18) 0001 * 7FFF => 0000:7FFF, CF=OF=0
    mov ax, 0x0001
    mov bx, 0x7FFF
    imul bx
    SAVE_FLAGS
    ASSERT_AX 0x7FFF
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 19) 0002 * 4000 => 0000:8000, CF=OF=1
    mov ax, 0x0002
    mov bx, 0x4000
    imul bx
    SAVE_FLAGS
    ASSERT_AX 0x8000
    ASSERT_DX 0x0000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 20) FFFF(-1) * [var16a]=0002 => FFFF:FFFE(-2), CF=OF=0
    mov ax, 0xFFFF
    imul word [var16a]
    SAVE_FLAGS
    ASSERT_AX 0xFFFE
    ASSERT_DX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 21) 8000(-32768) * [si]=0002 => FFFF:0000(-65536), CF=OF=1
    lea si, [var16b]
    mov ax, 0x8000
    imul word [si]
    SAVE_FLAGS
    ASSERT_AX 0x0000
    ASSERT_DX 0xFFFF
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 22) 8000(-32768) * [bp] (ds:)=0001 => FFFF:8000, CF=OF=0
    mov bp, var16c1
    mov ax, 0x8000
    imul word [ds:bp]
    SAVE_FLAGS
    ASSERT_AX 0x8000
    ASSERT_DX 0xFFFF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 23) FFFF(-1) * FFFF(-1) => 0000:0001, CF=OF=0
    mov ax, 0xFFFF
    mov bx, 0xFFFF
    imul bx
    SAVE_FLAGS
    ASSERT_AX 0x0001
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 24) 7FFF * [bx+si+disp]=0002 => 0000:FFFE, CF=OF=1
    lea bx, [base16]
    lea si, [index16]
    mov ax, 0x7FFF
    imul word [bx+si+var16d-base16-index16]
    SAVE_FLAGS
    ASSERT_AX 0xFFFE
    ASSERT_DX 0x0000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 25) 2222h * [si]=0003 => 0000:6666, CF=OF=0
    lea si, [var16e]
    mov ax, 0x2222
    imul word [si]
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 26) 4000h * [di]=0004 => 0001:0000, CF=OF=1
    lea di, [var16f]
    mov ax, 0x4000
    imul word [di]
    SAVE_FLAGS
    ASSERT_AX 0x0000
    ASSERT_DX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 27) 0009 * [bx]=1111h => 0000:9999, CF=OF=1
    lea bx, [var16g]
    mov ax, 0x0009
    imul word [bx]
    SAVE_FLAGS
    ASSERT_AX 0x9999
    ASSERT_DX 0x0000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 28) 00FF * [bx+disp]=0100 => 0000:FF00, CF=OF=1
    lea bx, [base16bd]
    mov ax, 0x00FF
    imul word [bx + (var16h - base16bd)]
    SAVE_FLAGS
    ASSERT_AX 0xFF00
    ASSERT_DX 0x0000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 29) 7FFF * [si+disp]=0001 => 0000:7FFF, CF=OF=0
    lea si, [base16sid]
    mov ax, 0x7FFF
    imul word [si + (var16i - base16sid)]
    SAVE_FLAGS
    ASSERT_AX 0x7FFF
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 30) 0002 * [di+disp]=7FFF => 0000:FFFE, CF=OF=1
    lea di, [base16did]
    mov ax, 0x0002
    imul word [di + (var16j - base16did)]
    SAVE_FLAGS
    ASSERT_AX 0xFFFE
    ASSERT_DX 0x0000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 31) 5555h * [bp+si+disp] (ds:)=0003 => 0000:FFFF, CF=OF=1
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    mov ax, 0x5555
    imul word [ds:bp+si + (var16k - base_bp16_si_A - base_bp16_si_B)]
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    ASSERT_DX 0x0000
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 32) 4000h * [bp+di+disp] (ds:)=0004 => 0001:0000, CF=OF=1
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov ax, 0x4000
    imul word [ds:bp+di + (var16l - base_bp16_di_A - base_bp16_di_B)]
    SAVE_FLAGS
    ASSERT_AX 0x0000
    ASSERT_DX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 33) 0800h * [bp+disp] (ds:)=0001 => 0000:0800, CF=OF=0
    lea bp, [base_bp16_d]
    mov ax, 0x0800
    imul word [ds:bp + (var16m - base_bp16_d)]
    SAVE_FLAGS
    ASSERT_AX 0x0800
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; 8-bit operands (for addressing coverage)
base8:
index8:
var8d:   db 0xFF          ; -1 for [bx+si+disp]

var8c:   db 0x01          ; [bp] (ds:)

base8sd:
var8e:   db 0x08          ; [si+disp]

var8f:   db 0x07          ; [di]

base8dd:
var8g:   db 0xFF          ; [di+disp] = -1

var8h:   db 0x11          ; [bx]

base8bd:
var8i:   db 0x33          ; [bx+disp]

base_bp_si_A:
base_bp_si_B:
var8j:   db 0x06          ; [bp+si+disp] (ds:)

base_bp_di_A:
base_bp_di_B:
var8k:   db 0x02          ; [bp+di+disp] (ds:)

base_bp_d:
var8l:   db 0x10          ; [bp+disp] (ds:)

; 16-bit operands (for addressing coverage)
var16a:  dw 0x0002        ; used in test 20
var16b:  dw 0x0002        ; [si]
var16c1: dw 0x0001        ; [bp] (ds:)

base16:
index16:
var16d:  dw 0x0002        ; [bx+si+disp]

var16e:  dw 0x0003        ; [si]
var16f:  dw 0x0004        ; [di]
var16g:  dw 0x1111        ; [bx]

base16bd:
var16h:  dw 0x0100        ; [bx+disp]

base16sid:
var16i:  dw 0x0001        ; [si+disp]

base16did:
var16j:  dw 0x7FFF        ; [di+disp]

base_bp16_si_A:
base_bp16_si_B:
var16k:  dw 0x0003        ; [bp+si+disp] (ds:)

base_bp16_di_A:
base_bp16_di_B:
var16l:  dw 0x0004        ; [bp+di+disp] (ds:)

base_bp16_d:
var16m:  dw 0x0001        ; [bp+disp] (ds:)

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
