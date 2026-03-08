; mul.asm — thorough MUL tests with all addressing modes, saved FLAGS, DOS exit
; Harness:
;   int 0x22: assert AL == AH
;   int 0x23: assert AX == BX
;
; Strategy:
;   - After each MUL, SAVE_FLAGS copies FLAGS to [flags_store].
;   - Assert results with int 0x23 (AX vs BX) and, when needed, DX via ASSERT_DX.
;   - CF/OF checks read [flags_store] to avoid clobber from interrupts.

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

; ===================== 8-bit MUL tests =====================

; Direct register/memory & bx+si+disp (from earlier set)
; 1) 00 * 5A = 0000, CF=OF=0
    mov al, 0x00
    mov bl, 0x5A
    mul bl
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 2) 01 * AF = 00AF, CF=OF=0
    mov al, 0x01
    mov bl, 0xAF
    mul bl
    SAVE_FLAGS
    ASSERT_AX 0x00AF
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 3) FF * 02 = 01FE, CF=OF=1
    mov al, 0xFF
    mov bl, 0x02
    mul bl
    SAVE_FLAGS
    ASSERT_AX 0x01FE
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 4) 10 * 10 = 0100, CF=OF=1
    mov al, 0x10
    mov bl, 0x10
    mul bl
    SAVE_FLAGS
    ASSERT_AX 0x0100
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 5) 0F * [var8a]=10 => 00F0, CF=OF=0
    mov al, 0x0F
    mul byte [var8a]
    SAVE_FLAGS
    ASSERT_AX 0x00F0
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 6) 80 * [si]=02 => 0100, CF=OF=1
    lea si, [var8b]
    mov al, 0x80
    mul byte [si]
    SAVE_FLAGS
    ASSERT_AX 0x0100
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 7) 13 * [bx+si+disp]=11 => 0143, CF=OF=1
    lea bx, [base8]
    lea si, [index8]
    mov al, 0x13
    mul byte [bx+si+var8d-base8-index8]
    SAVE_FLAGS
    ASSERT_AX 0x0143
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 8) 22 * [bp] (DS override) =11 => 0242, CF=OF=1  (corrected)
    mov bp, var8c
    mov al, 0x22
    mul byte [ds:bp]
    SAVE_FLAGS
    ASSERT_AX 0x0242
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; ---- Additional 8-bit addressing modes ----

; 9) [si+disp]: 22 * 03 => 0066, CF=OF=0
    lea si, [base8sd]
    mov al, 0x22
    mul byte [si + (var8e - base8sd)]
    SAVE_FLAGS
    ASSERT_AX 0x0066
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 10) [di]: 30 * 07 => 0150, CF=OF=1
    lea di, [var8f]
    mov al, 0x30
    mul byte [di]
    SAVE_FLAGS
    ASSERT_AX 0x0150
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 11) [di+disp]: 40 * 04 => 0100, CF=OF=1
    lea di, [base8dd]
    mov al, 0x40
    mul byte [di + (var8g - base8dd)]
    SAVE_FLAGS
    ASSERT_AX 0x0100
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 12) [bx]: 09 * 11 => 0099, CF=OF=0
    lea bx, [var8h]
    mov al, 0x09
    mul byte [bx]
    SAVE_FLAGS
    ASSERT_AX 0x0099
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 13) [bx+disp]: 03 * 33 => 0099, CF=OF=0
    lea bx, [base8bd]
    mov al, 0x03
    mul byte [bx + (var8i - base8bd)]
    SAVE_FLAGS
    ASSERT_AX 0x0099
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 14) [bp+si+disp] (ds:): 55 * 06 => 01FE, CF=OF=1
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov al, 0x55
    mul byte [ds:bp+si + (var8j - base_bp_si_A - base_bp_si_B)]
    SAVE_FLAGS
    ASSERT_AX 0x01FE
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 15) [bp+di+disp] (ds:): 30 * 02 => 0060, CF=OF=0
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov al, 0x30
    mul byte [ds:bp+di + (var8k - base_bp_di_A - base_bp_di_B)]
    SAVE_FLAGS
    ASSERT_AX 0x0060
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 16) [bp+disp] (ds:): 10 * 10 => 0100, CF=OF=1
    lea bp, [base_bp_d]
    mov al, 0x10
    mul byte [ds:bp + (var8l - base_bp_d)]
    SAVE_FLAGS
    ASSERT_AX 0x0100
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF


; ===================== 16-bit MUL tests =====================

; Original basic set
; 17) 0000 * 5A5A => 0000:0000, CF=OF=0
    xor ax, ax
    mov bx, 0x5A5A
    mul bx
    SAVE_FLAGS
    ASSERT_AX 0x0000
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 18) 0001 * 1234 => 0000:1234, CF=OF=0
    mov ax, 0x0001
    mov bx, 0x1234
    mul bx
    SAVE_FLAGS
    ASSERT_AX 0x1234
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 19) FFFF * 0002 => 0001:FFFE, CF=OF=1
    mov ax, 0xFFFF
    mov bx, 0x0002
    mul bx
    SAVE_FLAGS
    ASSERT_AX 0xFFFE
    ASSERT_DX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 20) 00FF * [var16a]=0100 => 0000:FF00, CF=OF=0
    mov ax, 0x00FF
    mul word [var16a]
    SAVE_FLAGS
    ASSERT_AX 0xFF00
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 21) 8000 * [si]=0002 => 0001:0000, CF=OF=1
    lea si, [var16b]
    mov ax, 0x8000
    mul word [si]
    SAVE_FLAGS
    ASSERT_AX 0x0000
    ASSERT_DX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 22) 5555 * [bp] (ds:)=0003 => 0000:FFFF, CF=OF=0
    mov bp, var16c
    mov ax, 0x5555
    mul word [ds:bp]
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 23) FFFF * FFFF => FFFE:0001, CF=OF=1
    mov ax, 0xFFFF
    mov bx, 0xFFFF
    mul bx
    SAVE_FLAGS
    ASSERT_AX 0x0001
    ASSERT_DX 0xFFFE
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 24) 1234 * [bx+si+disp]=0003 => 0000:369C, CF=OF=0
    lea bx, [base16]
    lea si, [index16]
    mov ax, 0x1234
    mul word [bx+si+var16d-base16-index16]
    SAVE_FLAGS
    ASSERT_AX 0x369C
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; ---- Additional 16-bit addressing modes ----

; 25) [si]: 2222 * 0003 => 0000:6666, CF=OF=0
    lea si, [var16e]
    mov ax, 0x2222
    mul word [si]
    SAVE_FLAGS
    ASSERT_AX 0x6666
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 26) [di]: 4000 * 0004 => 0001:0000, CF=OF=1
    lea di, [var16f]
    mov ax, 0x4000
    mul word [di]
    SAVE_FLAGS
    ASSERT_AX 0x0000
    ASSERT_DX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 27) [bx]: 0009 * 1111 => 0000:9999, CF=OF=0
    lea bx, [var16g]
    mov ax, 0x0009
    mul word [bx]
    SAVE_FLAGS
    ASSERT_AX 0x9999
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 28) [bx+disp]: 00FF * 0100 => 0000:FF00, CF=OF=0
    lea bx, [base16bd]
    mov ax, 0x00FF
    mul word [bx + (var16h - base16bd)]
    SAVE_FLAGS
    ASSERT_AX 0xFF00
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 29) [si+disp]: 0002 * 8000 => 0001:0000, CF=OF=1
    lea si, [base16sid]
    mov ax, 0x0002
    mul word [si + (var16i - base16sid)]
    SAVE_FLAGS
    ASSERT_AX 0x0000
    ASSERT_DX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 30) [di+disp]: 0002 * 7FFF => 0000:FFFE, CF=OF=0
    lea di, [base16did]
    mov ax, 0x0002
    mul word [di + (var16j - base16did)]
    SAVE_FLAGS
    ASSERT_AX 0xFFFE
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 31) [bp+si+disp] (ds:): 5555 * 0003 => 0000:FFFF, CF=OF=0
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    mov ax, 0x5555
    mul word [ds:bp+si + (var16k - base_bp16_si_A - base_bp16_si_B)]
    SAVE_FLAGS
    ASSERT_AX 0xFFFF
    ASSERT_DX 0x0000
    CHECK_CF 0
    CHECK_OF 0
    CHECK_CF_EQ_OF

; 32) [bp+di+disp] (ds:): 4000 * 0004 => 0001:0000, CF=OF=1
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov ax, 0x4000
    mul word [ds:bp+di + (var16l - base_bp16_di_A - base_bp16_di_B)]
    SAVE_FLAGS
    ASSERT_AX 0x0000
    ASSERT_DX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF

; 33) [bp+disp] (ds:): 0800 * 0020 => 0001:0000, CF=OF=1
    lea bp, [base_bp16_d]
    mov ax, 0x0800
    mul word [ds:bp + (var16m - base_bp16_d)]
    SAVE_FLAGS
    ASSERT_AX 0x0000
    ASSERT_DX 0x0001
    CHECK_CF 1
    CHECK_OF 1
    CHECK_CF_EQ_OF


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; 8-bit operands (original + added)
var8a:   db 0x10          ; 16
var8b:   db 0x02          ; 2
var8c:   db 0x11          ; 17
base8:
index8:
var8d:   db 0x11          ; for [bx+si+disp]

base8sd:
var8e:   db 0x03          ; for [si+disp]

base8di:
var8f:   db 0x07          ; for [di]

base8dd:
var8g:   db 0x04          ; for [di+disp]

base8bx:
var8h:   db 0x11          ; for [bx]

base8bd:
var8i:   db 0x33          ; for [bx+disp]

base_bp_si_A:
base_bp_si_B:
var8j:   db 0x06          ; for [bp+si+disp]

base_bp_di_A:
base_bp_di_B:
var8k:   db 0x02          ; for [bp+di+disp]

base_bp_d:
var8l:   db 0x10          ; for [bp+disp]

; 16-bit operands (original + added)
var16a:  dw 0x0100        ; 256
var16b:  dw 0x0002        ; 2
var16c:  dw 0x0003        ; 3
base16:
index16:
var16d:  dw 0x0003        ; for [bx+si+disp]

base16si:
var16e:  dw 0x0003        ; for [si]

base16di:
var16f:  dw 0x0004        ; for [di]

base16bx:
var16g:  dw 0x1111        ; for [bx]

base16bd:
var16h:  dw 0x0100        ; for [bx+disp]

base16sid:
var16i:  dw 0x8000        ; for [si+disp]

base16did:
var16j:  dw 0x7FFF        ; for [di+disp]

base_bp16_si_A:
base_bp16_si_B:
var16k:  dw 0x0003        ; for [bp+si+disp]

base_bp16_di_A:
base_bp16_di_B:
var16l:  dw 0x0004        ; for [bp+di+disp]

base_bp16_d:
var16m:  dw 0x0020        ; for [bp+disp]

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
