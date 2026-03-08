; div.asm — thorough DIV tests with all addressing modes, DOS exit
; Harness:
;   int 0x23: assert AX == BX   (we compare packed remainder:quotient in AX)
; Notes:
;   - DIV r/m8:  AX / r/m8 -> AL=quotient, AH=remainder  (AX == (AH<<8 | AL))
;   - DIV r/m16: DX:AX / r/m16 -> AX=quotient, DX=remainder
;   - Flags are undefined after DIV; we do NOT check them.

bits 16
org 0x100

; ---------- Macros ----------
%macro ASSERT_AX 1
    mov bx, %1
    int 0x23
%endmacro

%macro ASSERT_DX 1
    mov ax, dx
    mov bx, %1
    int 0x23
%endmacro

start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; Standard .COM: DS = CS
    push cs
    pop  ds

; ===================== 8-bit DIV tests (AX / r/m8 -> AL:quot, AH:rem) =====================

; 1) 0000 / 7D => q=00, r=00 -> AX=0000
    xor ax, ax
    mov bl, 0x7D
    div bl
    ASSERT_AX 0x0000

; 2) 00AF / 02 => q=57, r=01 -> AX=0157
    mov ax, 0x00AF
    mov bl, 0x02
    div bl
    ASSERT_AX 0x0157

; 3) 0100 / 10 => q=10, r=00 -> AX=0010
    mov ax, 0x0100
    mov bl, 0x10
    div bl
    ASSERT_AX 0x0010

; 4) 01FE / FF => q=02, r=00 -> AX=0002
    mov ax, 0x01FE
    mov bl, 0xFF
    div bl
    ASSERT_AX 0x0002

; 5) 1234 / [var8a]=56 => q=36, r=10 -> AX=1036
    mov ax, 0x1234
    div byte [var8a]
    ASSERT_AX 0x1036

; 6) 00FF / [si]=02 => q=7F, r=01 -> AX=017F
    lea si, [var8b]
    mov ax, 0x00FF
    div byte [si]
    ASSERT_AX 0x017F

; 7) 02FF / [bx+si+disp]=03 => q=FF, r=02 -> AX=02FF (boundary: 256*3-1)
    lea bx, [base8]
    lea si, [index8]
    mov ax, 0x02FF
    div byte [bx+si + (var8d - base8 - index8)]
    ASSERT_AX 0x02FF

; 8) 0242 / [bp] (ds:)=11 => q=22, r=00 -> AX=0022
    mov bp, var8c
    mov ax, 0x0242
    div byte [ds:bp]
    ASSERT_AX 0x0022

; 9) 0150 / [di]=07 => q=30, r=00 -> AX=0030
    lea di, [var8f]
    mov ax, 0x0150
    div byte [di]
    ASSERT_AX 0x0030

; 10) 0100 / [di+disp]=04 => q=40, r=00 -> AX=0040
    lea di, [base8dd]
    mov ax, 0x0100
    div byte [di + (var8g - base8dd)]
    ASSERT_AX 0x0040

; 11) 0099 / [bx]=11 => q=09, r=00 -> AX=0009
    lea bx, [base8bx]
    mov ax, 0x0099
    div byte [bx]
    ASSERT_AX 0x0009

; 12) 0099 / [bx+disp]=33 => q=03, r=00 -> AX=0003
    lea bx, [base8bd]
    mov ax, 0x0099
    div byte [bx + (var8i - base8bd)]
    ASSERT_AX 0x0003

; 13) 07FF / [si+disp]=08 => q=FF, r=07 -> AX=07FF (boundary: 256*8-1)
    lea si, [base8sd]
    mov ax, 0x07FF
    div byte [si + (var8e - base8sd)]
    ASSERT_AX 0x07FF

; 14) 01FE / [bp+si+disp] (ds:)=06 => q=55, r=00 -> AX=0055
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov ax, 0x01FE
    div byte [ds:bp+si + (var8j - base_bp_si_A - base_bp_si_B)]
    ASSERT_AX 0x0055

; 15) 0060 / [bp+di+disp] (ds:)=02 => q=30, r=00 -> AX=0030
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov ax, 0x0060
    div byte [ds:bp+di + (var8k - base_bp_di_A - base_bp_di_B)]
    ASSERT_AX 0x0030

; 16) 0100 / [bp+disp] (ds:)=10 => q=10, r=00 -> AX=0010
    lea bp, [base_bp_d]
    mov ax, 0x0100
    div byte [ds:bp + (var8l - base_bp_d)]
    ASSERT_AX 0x0010


; ===================== 16-bit DIV tests (DX:AX / r/m16 -> AX:quot, DX:rem) =====================

; 17) 0000:0000 / 5A5A => q=0000, r=0000
    xor dx, dx
    xor ax, ax
    mov bx, 0x5A5A
    div bx
    ASSERT_AX 0x0000
    ASSERT_DX 0x0000

; 18) 0000:1234 / 0002 => q=091A, r=0000
    xor dx, dx
    mov ax, 0x1234
    mov bx, 0x0002
    div bx
    ASSERT_AX 0x091A
    ASSERT_DX 0x0000

; 19) 0000:8000 / [si]=0002 => q=4000, r=0000
    lea si, [var16b]
    xor dx, dx
    mov ax, 0x8000
    div word [si]
    ASSERT_AX 0x4000
    ASSERT_DX 0x0000

; 20) 0001:0000 / 0002 => q=8000, r=0000
    mov dx, 0x0001
    mov ax, 0x0000
    mov bx, 0x0002
    div bx
    ASSERT_AX 0x8000
    ASSERT_DX 0x0000

; 21) 0002:FFFF / 0003 => q=FFFF, r=0002 (boundary: 65536*3-1)
    mov dx, 0x0002
    mov ax, 0xFFFF
    mov bx, 0x0003
    div bx
    ASSERT_AX 0xFFFF
    ASSERT_DX 0x0002

; 22) 0000:369C / [bx+si+disp]=0003 => q=1234, r=0000
    lea bx, [base16]
    lea si, [index16]
    xor dx, dx
    mov ax, 0x369C
    div word [bx+si + (var16d - base16 - index16)]
    ASSERT_AX 0x1234
    ASSERT_DX 0x0000

; 23) 0000:6666 / [si]=0003 => q=2222, r=0000
    lea si, [var16e]
    xor dx, dx
    mov ax, 0x6666
    div word [si]
    ASSERT_AX 0x2222
    ASSERT_DX 0x0000

; 24) 0001:0000 / [di]=0004 => q=4000, r=0000
    lea di, [var16f]
    mov dx, 0x0001
    mov ax, 0x0000
    div word [di]
    ASSERT_AX 0x4000
    ASSERT_DX 0x0000

; 25) 0000:9999 / [bx]=1111 => q=0009, r=0000
    lea bx, [var16g]
    xor dx, dx
    mov ax, 0x9999
    div word [bx]
    ASSERT_AX 0x0009
    ASSERT_DX 0x0000

; 26) 0000:FF00 / [bx+disp]=0100 => q=00FF, r=0000
    lea bx, [base16bd]
    xor dx, dx
    mov ax, 0xFF00
    div word [bx + (var16h - base16bd)]
    ASSERT_AX 0x00FF
    ASSERT_DX 0x0000

; 27) 0000:7FFF / [si+disp]=0001 => q=7FFF, r=0000
    lea si, [base16sid]
    xor dx, dx
    mov ax, 0x7FFF
    div word [si + (var16i - base16sid)]
    ASSERT_AX 0x7FFF
    ASSERT_DX 0x0000

; 28) 0000:FFFE / [di+disp]=7FFF => q=0002, r=0000
    lea di, [base16did]
    xor dx, dx
    mov ax, 0xFFFE
    div word [di + (var16j - base16did)]
    ASSERT_AX 0x0002
    ASSERT_DX 0x0000

; 29) 0000:FFFF / [bp+si+disp] (ds:)=0003 => q=5555, r=0000
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    xor dx, dx
    mov ax, 0xFFFF
    div word [ds:bp+si + (var16k - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x5555
    ASSERT_DX 0x0000

; 30) 0001:0000 / [bp+di+disp] (ds:)=0004 => q=4000, r=0000
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov dx, 0x0001
    mov ax, 0x0000
    div word [ds:bp+di + (var16l - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0x4000
    ASSERT_DX 0x0000

; 31) 0001:0000 / [bp+disp] (ds:)=0020 => q=0800, r=0000
    lea bp, [base_bp16_d]
    mov dx, 0x0001
    mov ax, 0x0000
    div word [ds:bp + (var16m - base_bp16_d)]
    ASSERT_AX 0x0800
    ASSERT_DX 0x0000

; 32) 0000:8000 / [bp] (ds:)=0001 => q=8000, r=0000
    mov bp, var16c1
    xor dx, dx
    mov ax, 0x8000
    div word [ds:bp]
    ASSERT_AX 0x8000
    ASSERT_DX 0x0000

; 33) 0001:0000 / [bx+di+disp]=0002 => q=8000, r=0000
    lea bx, [base16bdi_A]
    lea di, [base16bdi_B]
    mov dx, 0x0001
    mov ax, 0x0000
    div word [bx+di + (var16n - base16bdi_A - base16bdi_B)]
    ASSERT_AX 0x8000
    ASSERT_DX 0x0000

; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
; 8-bit divisors & base labels
var8a:   db 0x56          ; 86
var8b:   db 0x02          ; 2
var8c:   db 0x11          ; 17

base8:
index8:
var8d:   db 0x03          ; 3 for [bx+si+disp]

base8sd:
var8e:   db 0x08          ; 8 for [si+disp]

var8f:   db 0x07          ; 7 for [di]

base8dd:
var8g:   db 0x04          ; 4 for [di+disp]

base8bx:
var8h:   db 0x11          ; 17 for [bx]

base8bd:
var8i:   db 0x33          ; 51 for [bx+disp]

base_bp_si_A:
base_bp_si_B:
var8j:   db 0x06          ; 6 for [bp+si+disp] (ds:)

base_bp_di_A:
base_bp_di_B:
var8k:   db 0x02          ; 2 for [bp+di+disp] (ds:)

base_bp_d:
var8l:   db 0x10          ; 16 for [bp+disp] (ds:)

; 16-bit divisors & base labels
var16b:  dw 0x0002        ; [si]
base16:
index16:
var16d:  dw 0x0003        ; [bx+si+disp]

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
var16m:  dw 0x0020        ; [bp+disp] (ds:)

var16c1: dw 0x0001        ; [bp] (ds:)

base16bdi_A:
base16bdi_B:
var16n:  dw 0x0002        ; [bx+di+disp]

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
