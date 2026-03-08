; idiv.asm — thorough IDIV tests (signed divide) with all addressing modes, DOS exit
; Harness:
;   int 0x23: assert AX == BX
; Notes:
;   - IDIV r/m8:  AX (signed) / r/m8 (signed) -> AL=quotient, AH=remainder (same sign as dividend)
;                  We compare AX packed as AH:AL = remainder:quotient.
;   - IDIV r/m16: DX:AX (signed) / r/m16 (signed) -> AX=quotient, DX=remainder (same sign as dividend)
;                  We compare AX and DX separately.

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

; ===================== 8-bit IDiv tests (AX / r/m8 -> AL:quot, AH:rem) =====================

; 1)  0000 / 7D  => q=00, r=00  -> AX=0000
    xor ax, ax
    mov bl, 0x7D
    idiv bl
    ASSERT_AX 0x0000

; 2)  007F / 01  => q=7F, r=00  -> AX=007F
    mov ax, 0x007F
    mov bl, 0x01
    idiv bl
    ASSERT_AX 0x007F

; 3)  FF80(-128) / 02  => q=C0(-64), r=00 -> AX=00C0
    mov ax, 0xFF80
    mov bl, 0x02
    idiv bl
    ASSERT_AX 0x00C0

; 4)  FF81(-127) / 0A  => q=F4(-12), r=F9(-7) -> AX=F9F4
    mov ax, 0xFF81
    mov bl, 0x0A
    idiv bl
    ASSERT_AX 0xF9F4

; 5)  0005 / [var8a]=-2 => q=FE(-2), r=01 -> AX=01FE
    mov ax, 0x0005
    idiv byte [var8a]
    ASSERT_AX 0x01FE

; 6)  FFF6(-10) / [si]=-3 => q=03, r=FF(-1) -> AX=FF03
    lea si, [var8b]
    mov ax, 0xFFF6
    idiv byte [si]
    ASSERT_AX 0xFF03

; 7)  017F(383) / [bx+si+disp]=03 => q=7F, r=02 -> AX=027F
    lea bx, [base8]
    lea si, [index8]
    mov ax, 0x017F
    idiv byte [bx+si + (var8d - base8 - index8)]
    ASSERT_AX 0x027F

; 8)  FF80(-128) / [bp] (ds:)=01 => q=80(-128), r=00 -> AX=0080
    mov bp, var8c
    mov ax, 0xFF80
    idiv byte [ds:bp]
    ASSERT_AX 0x0080

; 9)  FF9C(-100) / [di]=07 => q=F2(-14), r=FE(-2) -> AX=FEF2
    lea di, [var8f]
    mov ax, 0xFF9C
    idiv byte [di]
    ASSERT_AX 0xFEF2

; 10) 00FD(253) / [di+disp]=04 => q=3F(63), r=01 -> AX=013F
    lea di, [base8dd]
    mov ax, 0x00FD
    idiv byte [di + (var8g - base8dd)]
    ASSERT_AX 0x013F

; 11) FF67(-153) / [bx]=11 => q=F7(-9), r=00 -> AX=00F7
    lea bx, [base8bx]
    mov ax, 0xFF67
    idiv byte [bx]
    ASSERT_AX 0x00F7

; 12) FF67(-155) / [bx+disp]=33 => q=FD(-3), r=FE(-2) -> AX=FEFD
    lea bx, [base8bd]
    mov ax, 0xFF65
    idiv byte [bx + (var8i - base8bd)]
    ASSERT_AX 0xFEFD

; 13) FC00(-1024) / [si+disp]=08 => q=80(-128), r=00 -> AX=0080
    lea si, [base8sd]
    mov ax, 0xFC00
    idiv byte [si + (var8e - base8sd)]
    ASSERT_AX 0x0080

; 14) FE02(-510) / [bp+si+disp] (ds:)=06 => q=AB(-85), r=00 -> AX=00AB
    lea bp, [base_bp_si_A]
    lea si, [base_bp_si_B]
    mov ax, 0xFE02
    idiv byte [ds:bp+si + (var8j - base_bp_si_A - base_bp_si_B)]
    ASSERT_AX 0x00AB

; 15) FF81(-127) / [bp+di+disp] (ds:)=FE(-2) => q=3F(63), r=FF(-1) -> AX=FF3F
    lea bp, [base_bp_di_A]
    lea di, [base_bp_di_B]
    mov ax, 0xFF81
    idiv byte [ds:bp+di + (var8k - base_bp_di_A - base_bp_di_B)]
    ASSERT_AX 0xFF3F

; 16) 007F(127) / [bp+disp] (ds:)=10 => q=0C(12), r=07 -> AX=070C
    lea bp, [base_bp_d]
    mov ax, 0x007F
    idiv byte [ds:bp + (var8l - base_bp_d)]
    ASSERT_AX 0x0F07


; ===================== 16-bit IDiv tests (DX:AX / r/m16 -> AX:quot, DX:rem) =====================

; 17) 0000:0000 / 7FFF => q=0000, r=0000
    xor dx, dx
    xor ax, ax
    mov bx, 0x7FFF
    idiv bx
    ASSERT_AX 0x0000
    ASSERT_DX 0x0000

; 18) 0000:1234 / 0002 => q=091A, r=0000
    xor dx, dx
    mov ax, 0x1234
    mov bx, 0x0002
    idiv bx
    ASSERT_AX 0x091A
    ASSERT_DX 0x0000

; 19) 0000:8000 / [si]=0002 => q=4000, r=0000
    lea si, [var16b]
    xor dx, dx
    mov ax, 0x8000
    idiv word [si]
    ASSERT_AX 0x4000
    ASSERT_DX 0x0000

; 20) FFFF:0000(-65536) / 0002 => q=8000(-32768), r=0000
    mov dx, 0xFFFF
    mov ax, 0x0000
    mov bx, 0x0002
    idiv bx
    ASSERT_AX 0x8000
    ASSERT_DX 0x0000

; 21) 0001:7FFF(98303) / 0003 => q=7FFF, r=0002
    mov dx, 0x0001
    mov ax, 0x7FFF
    mov bx, 0x0003
    idiv bx
    ASSERT_AX 0x7FFF
    ASSERT_DX 0x0002

; 22) 0000:369C / [bx+si+disp]=0003 => q=1234, r=0000
    lea bx, [base16]
    lea si, [index16]
    xor dx, dx
    mov ax, 0x369C
    idiv word [bx+si + (var16d - base16 - index16)]
    ASSERT_AX 0x1234
    ASSERT_DX 0x0000

; 23) 0000:6666 / [si]=0003 => q=2222, r=0000
    lea si, [var16e]
    xor dx, dx
    mov ax, 0x6666
    idiv word [si]
    ASSERT_AX 0x2222
    ASSERT_DX 0x0000

; 24) 0001:0000(65536) / [di]=0004 => q=4000, r=0000
    lea di, [var16f]
    mov dx, 0x0001
    mov ax, 0x0000
    idiv word [di]
    ASSERT_AX 0x4000
    ASSERT_DX 0x0000

; 25) 0000:9999 / [bx]=1111 => q=0009, r=0000
    lea bx, [var16g]
    xor dx, dx
    mov ax, 0x9999
    idiv word [bx]
    ASSERT_AX 0x0009
    ASSERT_DX 0x0000

; 26) 0000:FF00 / [bx+disp]=0100 => q=00FF, r=0000
    lea bx, [base16bd]
    xor dx, dx
    mov ax, 0xFF00
    idiv word [bx + (var16h - base16bd)]
    ASSERT_AX 0x00FF
    ASSERT_DX 0x0000

; 27) 0000:7FFF / [si+disp]=0001 => q=7FFF, r=0000
    lea si, [base16sid]
    xor dx, dx
    mov ax, 0x7FFF
    idiv word [si + (var16i - base16sid)]
    ASSERT_AX 0x7FFF
    ASSERT_DX 0x0000

; 28) 0000:FFFE / [di+disp]=7FFF => q=0002, r=0000
    lea di, [base16did]
    xor dx, dx
    mov ax, 0xFFFE
    idiv word [di + (var16j - base16did)]
    ASSERT_AX 0x0002
    ASSERT_DX 0x0000

; 29) FFFF:FFFF(-1) / [bp+si+disp] (ds:)=0003 => q=0000, r=FFFF(-1)
    lea bp, [base_bp16_si_A]
    lea si, [base_bp16_si_B]
    mov dx, 0xFFFF
    mov ax, 0xFFFF
    idiv word [ds:bp+si + (var16k - base_bp16_si_A - base_bp16_si_B)]
    ASSERT_AX 0x0000
    ASSERT_DX 0xFFFF

; 30) FFFF:0000(-65536) / [bp+di+disp] (ds:)=0004 => q=C000(-16384), r=0000
    lea bp, [base_bp16_di_A]
    lea di, [base_bp16_di_B]
    mov dx, 0xFFFF
    mov ax, 0x0000
    idiv word [ds:bp+di + (var16l - base_bp16_di_A - base_bp16_di_B)]
    ASSERT_AX 0xC000
    ASSERT_DX 0x0000

; 31) 0001:0000 / [bp+disp] (ds:)=0020 => q=0800, r=0000
    lea bp, [base_bp16_d]
    mov dx, 0x0001
    mov ax, 0x0000
    idiv word [ds:bp + (var16m - base_bp16_d)]
    ASSERT_AX 0x0800
    ASSERT_DX 0x0000

; 32) 0000:7FFF / [bp] (ds:)=0001 => q=7FFF, r=0000
    mov bp, var16c1
    xor dx, dx
    mov ax, 0x7FFF
    idiv word [ds:bp]
    ASSERT_AX 0x7FFF
    ASSERT_DX 0x0000

; 33) 0000:FFFE / [bx+di+disp]=0002 => q=7FFF, r=0000
    lea bx, [base16bdi_A]
    lea di, [base16bdi_B]
    xor dx, dx
    mov ax, 0xFFFE
    idiv word [bx+di + (var16n - base16bdi_A - base16bdi_B)]
    ASSERT_AX 0x7FFF
    ASSERT_DX 0x0000

; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
; 8-bit divisors & base labels
var8a:   db 0xFE          ; -2
var8b:   db 0xFD          ; -3

base8:
index8:
var8d:   db 0x03          ; +3 for [bx+si+disp]

var8c:   db 0x01          ; +1 for [bp] (ds:)

base8sd:
var8e:   db 0x08          ; +8 for [si+disp]

var8f:   db 0x07          ; +7 for [di]

base8dd:
var8g:   db 0x04          ; +4 for [di+disp]

base8bx:
var8h:   db 0x11          ; +17 for [bx]

base8bd:
var8i:   db 0x33          ; +51 for [bx+disp]

base_bp_si_A:
base_bp_si_B:
var8j:   db 0x06          ; +6 for [bp+si+disp] (ds:)

base_bp_di_A:
base_bp_di_B:
var8k:   db 0xFE          ; -2 for [bp+di+disp] (ds:)

base_bp_d:
var8l:   db 0x10          ; +16 for [bp+disp] (ds:)

; 16-bit divisors & base labels
var16b:  dw 0x0002        ; [si] = +2

base16:
index16:
var16d:  dw 0x0003        ; [bx+si+disp] = +3

var16e:  dw 0x0003        ; [si] = +3
var16f:  dw 0x0004        ; [di] = +4
var16g:  dw 0x1111        ; [bx] = +0x1111
base16bd:
var16h:  dw 0x0100        ; [bx+disp] = +256
base16sid:
var16i:  dw 0x0001        ; [si+disp] = +1
base16did:
var16j:  dw 0x7FFF        ; [di+disp] = +32767

base_bp16_si_A:
base_bp16_si_B:
var16k:  dw 0x0003        ; [bp+si+disp] (ds:) = +3

base_bp16_di_A:
base_bp16_di_B:
var16l:  dw 0x0004        ; [bp+di+disp] (ds:) = +4

base_bp16_d:
var16m:  dw 0x0020        ; [bp+disp] (ds:) = +32

var16c1: dw 0x0001        ; [bp] (ds:) = +1

base16bdi_A:
base16bdi_B:
var16n:  dw 0x0002        ; [bx+di+disp] = +2

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
