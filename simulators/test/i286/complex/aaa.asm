; aaa.asm — Thorough tests for AAA (ASCII Adjust AL after Addition)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Flags:
;   - Only AF and CF are defined by AAA (checked here).
;   - OF/SF/ZF/PF: undefined — not checked.

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

%macro CHECK_AF 1
    mov ax, [flags_store]
    mov cl, 4
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

; ===================== No-adjust path (AF_in=0, low nibble <= 9) =====================

; 1) AL=00, AH=00 -> AX=0000 ; CF=0 AF=0
    mov ah, [pat0]    ; set AF=0 CF=0
    sahf
    mov ax, 0x0000
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_AF 0

; 2) AL=09, AH=00 -> AX=0009 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x0009
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0009
    CHECK_CF 0
    CHECK_AF 0

; 3) AL=15, AH=00 -> AL becomes 05, AH stays 00 ⇒ AX=0005 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x0015
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0005
    CHECK_CF 0
    CHECK_AF 0

; 4) AL=90, AH=12 -> AL=00, AH=12 ⇒ AX=1200 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x1290
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x1200
    CHECK_CF 0
    CHECK_AF 0

; 5) AL=39, AH=7F -> AX=7F09 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x7F39
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x7F09
    CHECK_CF 0
    CHECK_AF 0


; ===================== Adjust due to LSN > 9 (AF_in=0) =====================

; 6) AL=0A, AH=00 -> AL=(0A+06)&0F=00; AH=01 ⇒ AX=0100 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x000A
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0100
    CHECK_CF 1
    CHECK_AF 1

; 7) AL=0F, AH=02 -> AL=(0F+06)=15 -> &0F=05; AH=03 ⇒ AX=0305 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x020F
    xchg al, ah        ; place AH=02, AL=0F → but we want AX=020F
    xchg al, ah        ; (no-op, just keeping pattern consistent)
    mov ax, 0x020F
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0305
    CHECK_CF 1
    CHECK_AF 1

; 8) AL=2C, AH=07 -> (2C LSN=C) ⇒ AL=(2C+06)=32 → &0F=02; AH=08 ⇒ AX=0802 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x072C
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0802
    CHECK_CF 1
    CHECK_AF 1

; 9) AL=AA, AH=00 -> LSN=A → AL=00; AH=01 ⇒ AX=0100 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x00AA
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0100
    CHECK_CF 1
    CHECK_AF 1


; ===================== Adjust forced by AF_in=1 (LSN <= 9) =====================

; 10) AL=11, AF_in=1, AH=00 -> AL=(11+06)=17 → &0F=07; AH=01 ⇒ AX=0107 ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0x0011
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0107
    CHECK_CF 1
    CHECK_AF 1

; 11) AL=12, AF_in=1, AH=00 -> AL=(12+06)=18 → &0F=08; AH=01 ⇒ AX=0108 ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0x0012
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0108
    CHECK_CF 1
    CHECK_AF 1

; 12) AL=00, AF_in=1, AH=34 -> AL=06; AH=35 ⇒ AX=3506 ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0x3400
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x3506
    CHECK_CF 1
    CHECK_AF 1

; 13) AL=09, AF_in=1, AH=00 -> AL=(09+06)=0F; AH=01 ⇒ AX=010F ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0x0009
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x010F
    CHECK_CF 1
    CHECK_AF 1

; 14) AL=05, AF_in=1, AH=22 -> AL=0B; AH=23 ⇒ AX=230B ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0x2205
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x230B
    CHECK_CF 1
    CHECK_AF 1


; ===================== CF_in variations (CF_in ignored by AAA; CF_out depends on adjust) =====================

; 15) CF_in=1, AL=04, AH=10 -> no adjust ⇒ AX=1004 ; CF=0 AF=0
    mov ah, [patC]
    sahf
    mov ax, 0x1004
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x1004
    CHECK_CF 0
    CHECK_AF 0

; 16) CF_in=1, AL=0A, AH=10 -> adjust ⇒ AX=1100 ; CF=1 AF=1
    mov ah, [patC]
    sahf
    mov ax, 0x100A
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x1100
    CHECK_CF 1
    CHECK_AF 1


; ===================== Boundary examples (around 9/10 and carries) =====================

; 17) AL=29, AH=00 (LSN=9) -> no adjust ⇒ AX=0009 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x0029
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0009
    CHECK_CF 0
    CHECK_AF 0

; 18) AL=0A, AH=00 (10) -> adjust ⇒ AX=0100 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x000A
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0100
    CHECK_CF 1
    CHECK_AF 1

; 19) AL=1A, AH=55 -> adjust ⇒ AX=5600 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x551A
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x5600
    CHECK_CF 1
    CHECK_AF 1

; 20) AL=19, AH=55 -> no adjust ⇒ AX=5509 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x5519
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x5509
    CHECK_CF 0
    CHECK_AF 0


; ===================== Verify AH increments only on adjust =====================

; 21) AL=0C, AH=55 -> adjust ⇒ AL=(0C+06)=12→&0F=02; AH=56 ⇒ AX=5602 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x550C
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x5602
    CHECK_CF 1
    CHECK_AF 1

; 22) AL=03, AH=55 -> no adjust ⇒ AX=5503 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x5503
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x5503
    CHECK_CF 0
    CHECK_AF 0


; ===================== Typical “unpacked BCD addition” outcomes =====================

; 23) 7 + 8 = 15 → AL=0F, AH=00 → AAA ⇒ AX=0105 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x000F
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0105
    CHECK_CF 1
    CHECK_AF 1

; 24) 9 + 9 = 18 (AF_in=1 because sum>0x0F): AL=12, AH=00, AF=1 ⇒ AX=0108 ; CF=1 AF=1
    mov ah, [patA]           ; preset AF=1
    sahf
    mov ax, 0x0012
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0108
    CHECK_CF 1
    CHECK_AF 1

; 25) 4 + 6 = 10 → AL=0A, AH=00 ⇒ AX=0100 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x000A
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0100
    CHECK_CF 1
    CHECK_AF 1


; ===================== High-nibble cleared even with no adjust =====================

; 26) AL=D3, AH=10, AF_in=0 → no adjust ⇒ AL=03, AH=10 ⇒ AX=1003 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x10D3
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x1003
    CHECK_CF 0
    CHECK_AF 0


; ===================== Wrap on AH increment (8-bit) =====================

; 27) AL=00, AF_in=1, AH=FF → adjust: AL=06, AH=00 ⇒ AX=0006 ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0xFF00
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0006
    CHECK_CF 1
    CHECK_AF 1

; 28) AL=FF, AF_in=0, AH=EE → adjust: (FF+06)=105→&0F=05; AH=EF ⇒ AX=EF05 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0xEEFF
    aaa
    SAVE_FLAGS
    ASSERT_AX 0xEF05
    CHECK_CF 1
    CHECK_AF 1


; ===================== Mixed AF/CF preset (CF_in irrelevant) =====================

; 29) CF_in=1, AF_in=1, AL=00, AH=00 → adjust ⇒ AX=0106 ; CF=1 AF=1
    mov ah, [patAC]
    sahf
    mov ax, 0x0000
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0106
    CHECK_CF 1
    CHECK_AF 1

; 30) CF_in=1, AF_in=0, AL=29, AH=01 → no adjust ⇒ AX=0109 ; CF=0 AF=0
    mov ah, [patC]
    sahf
    mov ax, 0x0129
    aaa
    SAVE_FLAGS
    ASSERT_AX 0x0109
    CHECK_CF 0
    CHECK_AF 0

; ===================== AAA: ASCII → unpacked BCD → AAA → packed BCD =====================

; 31) '3' + '8' = 11 → packed 0x11 ; CF=AF=1 (adjust)
    mov ah, [pat0]              ; CF=0 AF=0
    sahf
    mov al, '3'                 ; unpack
    sub al, '0'
    mov bl, '8'
    sub bl, '0'
    add al, bl                  ; 3+8=11
    mov ah, 0                   ; clear AH WITHOUT touching flags
    aaa                         ; AL=1, AH=1
    SAVE_FLAGS
    mov dl, ah                  ; pack tens:ones → (AH<<4)|AL
    shl dl, 4
    or  dl, al
    mov al, dl
    mov ah, 0
    ASSERT_AX 0x0011
    CHECK_CF 1
    CHECK_AF 1

; 32) '7' + '8' = 15 → 0x15 ; CF=AF=1
    mov ah, [pat0]
    sahf
    mov al, '7'
    sub al, '0'
    mov bl, '8'
    sub bl, '0'
    add al, bl                  ; 7+8=15
    mov ah, 0
    aaa                         ; AL=5, AH=1
    SAVE_FLAGS
    mov dl, ah
    shl dl, 4
    or  dl, al
    mov al, dl
    mov ah, 0
    ASSERT_AX 0x0015
    CHECK_CF 1
    CHECK_AF 1

; 33) '9' + '9' = 18 → 0x18 ; CF=AF=1
    mov ah, [pat0]
    sahf
    mov al, '9'
    sub al, '0'
    mov bl, '9'
    sub bl, '0'
    add al, bl
    mov ah, 0
    aaa                         ; AL=8, AH=1
    SAVE_FLAGS
    mov dl, ah
    shl dl, 4
    or  dl, al
    mov al, dl
    mov ah, 0
    ASSERT_AX 0x0018
    CHECK_CF 1
    CHECK_AF 1

; 34) '0' + '0' = 0  → 0x00 ; CF=AF=0
    mov ah, [pat0]
    sahf
    mov al, '0'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    add al, bl                  ; 0
    mov ah, 0
    aaa                         ; no adjust
    SAVE_FLAGS
    mov dl, ah
    shl dl, 4
    or  dl, al
    mov al, dl
    mov ah, 0
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_AF 0

; 35) '4' + '5' = 9  → 0x09 ; CF=AF=0
    mov ah, [pat0]
    sahf
    mov al, '4'
    sub al, '0'
    mov bl, '5'
    sub bl, '0'
    add al, bl                  ; 9
    mov ah, 0
    aaa                         ; no adjust
    SAVE_FLAGS
    mov dl, ah
    shl dl, 4
    or  dl, al
    mov al, dl
    mov ah, 0
    ASSERT_AX 0x0009
    CHECK_CF 0
    CHECK_AF 0


; ===================== AAA: 2-digit ASCII + 2-digit ASCII → packed BCD (tens:ones) with carry =====================

; 36) "12" + "34" = 46 → AX=0x0046 ; final CF=AF=0
    mov ah, [pat0]
    sahf
    ; ones: '2' + '4'
    mov al, '2'
    sub al, '0'
    mov bl, '4'
    sub bl, '0'
    add al, bl                  ; 6
    mov ah, 0
    aaa                         ; AL=6, AH=0
    mov bh, al                  ; save ones=6

    ; tens: '1' + '3' + carry_from_ones(AH)
    mov al, '1'
    sub al, '0'
    mov bl, '3'
    sub bl, '0'
    add al, ah                  ; +0
    add al, bl                  ; 4
    mov ah, 0
    aaa                         ; AL=4, AH=0 (no adjust)
    SAVE_FLAGS

    mov dl, al                  ; pack tens:ones
    shl dl, 4
    or  dl, bh
    mov al, dl
    mov ah, 0
    ASSERT_AX 0x0046
    CHECK_CF 0
    CHECK_AF 0

; 37) "59" + "73" = 132 → AX=0x0132 ; final CF=AF=1 (tens adjusted)
    mov ah, [pat0]
    sahf
    ; ones: 9 + 3
    mov al, '9'
    sub al, '0'
    mov bl, '3'
    sub bl, '0'
    add al, bl                  ; 12
    mov ah, 0
    aaa                         ; AL=2, AH=1
    mov bh, al                  ; ones=2

    ; tens: 5 + 7 + carry=1
    mov al, '5'
    sub al, '0'
    mov bl, '7'
    sub bl, '0'
    add al, ah                  ; 1
    add al, bl                  ; 1+12 → 13
    mov ah, 0
    aaa                         ; AL=3, AH=1 (hundreds=1)
    SAVE_FLAGS

    mov dl, al                  ; tens
    shl dl, 4
    or  dl, bh                  ; tens:ones = 0x32
    mov al, dl                  ; AH holds hundreds=1
    ASSERT_AX 0x0132
    CHECK_CF 1
    CHECK_AF 1

; 38) "99" + "99" = 198 → AX=0x0198 ; final CF=AF=1
    mov ah, [pat0]
    sahf
    ; ones: 9+9=18
    mov al, '9'
    sub al, '0'
    mov bl, '9'
    sub bl, '0'
    add al, bl
    mov ah, 0
    aaa                         ; AL=8, AH=1
    mov bh, al                  ; ones=8

    ; tens: 9+9 + carry1=19
    mov al, '9'
    sub al, '0'
    mov bl, '9'
    sub bl, '0'
    add al, ah
    add al, bl
    mov ah, 0
    aaa                         ; AL=9, AH=1 (hundreds=1)
    SAVE_FLAGS

    mov dl, al
    shl dl, 4
    or  dl, bh                  ; 0x98
    mov al, dl
    ASSERT_AX 0x0198
    CHECK_CF 1
    CHECK_AF 1

; 39) "07" + "08" = 15  → AX=0x0015 ; final CF=AF=0 (tens not adjusted)
    mov ah, [pat0]
    sahf
    ; ones: 7+8=15
    mov al, '7'
    sub al, '0'
    mov bl, '8'
    sub bl, '0'
    add al, bl
    mov ah, 0
    aaa                         ; AL=5, AH=1
    mov bh, al                  ; ones=5

    ; tens: 0+0 + carry1 = 1
    mov al, '0'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    add al, ah
    add al, bl
    mov ah, 0
    aaa                         ; AL=1, AH=0
    SAVE_FLAGS

    mov dl, al
    shl dl, 4
    or  dl, bh                  ; 0x15
    mov al, dl
    mov ah, 0
    ASSERT_AX 0x0015
    CHECK_CF 0
    CHECK_AF 0


; ===================== AAA: 3-digit + 3-digit → 3–4 digits (packed BCD across AX) =====================

; Convention: AX packed as AH = (thousands<<4) | hundreds, AL = (tens<<4) | ones

; 40) "499" + "501" = 1000 → AX=0x1000 ; final CF=AF=1 (hundreds adjusted)
    mov ah, [pat0]
    sahf
    ; ones: 9 + 1 = 10
    mov al, '9'
    sub al, '0'
    mov bl, '1'
    sub bl, '0'
    add al, bl
    mov ah, 0
    aaa                         ; AL=0, AH=1
    mov bh, al                  ; ones=0

    ; tens: 9 + 0 + carry1 = 10
    mov al, '9'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    add al, ah
    add al, bl
    mov ah, 0
    aaa                         ; AL=0, AH=1
    mov bl, al                  ; tens=0

    ; hundreds: 4 + 5 + carry1 = 10
    mov al, '4'
    sub al, '0'
    mov dl, '5'
    sub dl, '0'
    add al, ah
    add al, dl
    mov ah, 0
    aaa                         ; AL=0, AH=1 (thousands=1)
    SAVE_FLAGS
    mov dh, al                  ; hundreds=0

    ; Pack: AH=(thousands<<4)|hundreds, AL=(tens<<4)|ones
    shl ah, 4
    or  ah, dh       ; AH = 0x10
    mov al, bl
    shl al, 4
    or  al, bh       ; AL = 0x00
    ASSERT_AX 0x1000
    CHECK_CF 1
    CHECK_AF 1

; 41) "123" + "456" = 579 → AX=0x0579 ; final CF=AF=0 (hundreds not adjusted)
    mov ah, [pat0]
    sahf
    ; ones: 3 + 6 = 9
    mov al, '3'
    sub al, '0'
    mov bl, '6'
    sub bl, '0'
    add al, bl
    mov ah, 0
    aaa                         ; AL=9, AH=0
    mov bh, al                  ; ones=9

    ; tens: 2 + 5 = 7
    mov al, '2'
    sub al, '0'
    mov bl, '5'
    sub bl, '0'
    add al, ah
    add al, bl
    mov ah, 0
    aaa                         ; AL=7, AH=0
    mov bl, al                  ; tens=7

    ; hundreds: 1 + 4 = 5
    mov al, '1'
    sub al, '0'
    mov dl, '4'
    sub dl, '0'
    add al, ah
    add al, dl
    mov ah, 0
    aaa                         ; AL=5, AH=0
    SAVE_FLAGS
    mov dh, al                  ; hundreds=5

    ; Pack: 0x0579
    or  ah, dh       ; AH=0x05
    mov al, bl
    shl al, 4
    or  al, bh       ; AL=0x79
    ASSERT_AX 0x0579
    CHECK_CF 0
    CHECK_AF 0

; 42) "199" + "001" = 200 → AX=0x0200 ; final CF=AF=0 (hundreds not adjusted)
    mov ah, [pat0]
    sahf
    ; ones: 9 + 1 = 10
    mov al, '9'
    sub al, '0'
    mov bl, '1'
    sub bl, '0'
    add al, bl
    mov ah, 0
    aaa                         ; AL=0, AH=1
    mov bh, al                  ; ones=0

    ; tens: 9 + 0 + carry1 = 10
    mov al, '9'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    add al, ah
    add al, bl
    mov ah, 0
    aaa                         ; AL=0, AH=1
    mov bl, al                  ; tens=0

    ; hundreds: 1 + 0 + carry1 = 2
    mov al, '1'
    sub al, '0'
    mov dl, '0'
    sub dl, '0'
    add al, ah
    add al, dl
    mov ah, 0
    aaa                         ; AL=2, AH=0
    SAVE_FLAGS
    mov dh, al                  ; hundreds=2

    ; Pack: 0x0200 (thousands=0)
    or  ah, dh       ; AH = 0x02
    mov al, bl
    shl al, 4
    or  al, bh       ; AL = 0x00
    ASSERT_AX 0x0200
    CHECK_CF 0
    CHECK_AF 0


; ===================== Exit to DOS =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; SAHF patterns to preset AF/CF before AAA
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
