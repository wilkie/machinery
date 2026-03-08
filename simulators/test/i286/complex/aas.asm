; aas.asm — Thorough tests for AAS (ASCII Adjust AL after Subtraction)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; Flags:
;   - Only AF and CF are defined by AAS (checked here).
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

; ---------------- No-adjust path (AF_in=0, low nibble <= 9) ----------------

; 1) AL=00, AH=00 -> AX=0000 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x0000
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_AF 0

; 2) AL=09, AH=00 -> AX=0009 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x0009
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0009
    CHECK_CF 0
    CHECK_AF 0

; 3) AL=15, AH=00 -> AL=05, AH=00 ⇒ AX=0005 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x0015
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0005
    CHECK_CF 0
    CHECK_AF 0

; 4) AL=90, AH=12 -> AL=00, AH=12 ⇒ AX=1200 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x1290
    aas
    SAVE_FLAGS
    ASSERT_AX 0x1200
    CHECK_CF 0
    CHECK_AF 0

; 5) AL=39, AH=7F -> AX=7F09 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x7F39
    aas
    SAVE_FLAGS
    ASSERT_AX 0x7F09
    CHECK_CF 0
    CHECK_AF 0


; ---------------- Adjust due to LSN > 9 (AF_in=0) ----------------

; 6) AL=0A, AH=01 -> AL=(0A-06)=04; AH=00 ⇒ AX=0004 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x010A
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0004
    CHECK_CF 1
    CHECK_AF 1

; 7) AL=0F, AH=02 -> AL=(0F-06)=09; AH=01 ⇒ AX=0109 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x020F
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0109
    CHECK_CF 1
    CHECK_AF 1

; 8) AL=2C, AH=07 -> (LSN=C) AL=(2C-06)=26 → &0F=06; AH=06 ⇒ AX=0606 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x072C
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0606
    CHECK_CF 1
    CHECK_AF 1

; 9) AL=AA, AH=01 -> (LSN=A) AL=(AA-06)=A4→&0F=04; AH=00 ⇒ AX=0004 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x01AA
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0004
    CHECK_CF 1
    CHECK_AF 1


; ---------------- Adjust forced by AF_in=1 (even when LSN <= 9) ----------------

; 10) AF_in=1, AL=11, AH=01 -> AL=(11-06)=0B→&0F=0B; AH=00 ⇒ AX=000B ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0x0111
    aas
    SAVE_FLAGS
    ASSERT_AX 0x000B
    CHECK_CF 1
    CHECK_AF 1

; 11) AF_in=1, AL=12, AH=01 -> AL=0C; AH=00 ⇒ AX=000C ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0x0112
    aas
    SAVE_FLAGS
    ASSERT_AX 0x000C
    CHECK_CF 1
    CHECK_AF 1

; 12) AF_in=1, AL=00, AH=34 -> AL=FA→&0F=0A?  (00-06=FA, &0F=0x0A); AH=33 ⇒ AX=330A ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0x3400
    aas
    SAVE_FLAGS
    ASSERT_AX 0x330A
    CHECK_CF 1
    CHECK_AF 1

; 13) AF_in=1, AL=09, AH=01 -> AL=(09-06)=03; AH=00 ⇒ AX=0003 ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0x0109
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0003
    CHECK_CF 1
    CHECK_AF 1

; 14) AF_in=1, AL=05, AH=22 -> AL=FF? (05-06=FF → &0F=0F); AH=21 ⇒ AX=210F ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0x2205
    aas
    SAVE_FLAGS
    ASSERT_AX 0x210F
    CHECK_CF 1
    CHECK_AF 1


; ---------------- CF_in variations (CF_in ignored by AAS) ----------------

; 15) CF_in=1, AL=04, AH=10 -> no adjust ⇒ AX=1004 ; CF=0 AF=0
    mov ah, [patC]
    sahf
    mov ax, 0x1004
    aas
    SAVE_FLAGS
    ASSERT_AX 0x1004
    CHECK_CF 0
    CHECK_AF 0

; 16) CF_in=1, AL=0A, AH=01 -> adjust ⇒ AX=0004 ; CF=1 AF=1
    mov ah, [patC]
    sahf
    mov ax, 0x010A
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0004
    CHECK_CF 1
    CHECK_AF 1


; ---------------- Boundary examples (around 9/10 and borrows) ----------------

; 17) AL=29, AH=00 (LSN=9) -> no adjust ⇒ AX=0009 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x0029
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0009
    CHECK_CF 0
    CHECK_AF 0

; 18) AL=0A, AH=01 -> adjust ⇒ AX=0004 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x010A
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0004
    CHECK_CF 1
    CHECK_AF 1

; 19) AL=1A, AH=55 -> adjust ⇒ AL=(1A-06)=14→&0F=04; AH=54 ⇒ AX=5404 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x551A
    aas
    SAVE_FLAGS
    ASSERT_AX 0x5404
    CHECK_CF 1
    CHECK_AF 1

; 20) AL=19, AH=55 -> no adjust ⇒ AX=5509 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x5519
    aas
    SAVE_FLAGS
    ASSERT_AX 0x5509
    CHECK_CF 0
    CHECK_AF 0


; ---------------- Verify AH decrements only on adjust ----------------

; 21) AL=0C, AH=55 -> adjust ⇒ (0C-06)=06; AH=54 ⇒ AX=5406 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x550C
    aas
    SAVE_FLAGS
    ASSERT_AX 0x5406
    CHECK_CF 1
    CHECK_AF 1

; 22) AL=03, AH=55 -> no adjust ⇒ AX=5503 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x5503
    aas
    SAVE_FLAGS
    ASSERT_AX 0x5503
    CHECK_CF 0
    CHECK_AF 0


; ---------------- Typical “unpacked BCD subtraction” outcomes ----------------

; 23) 7 - 8 = -1 → emulate: AL=0xFF (after SUB), AH=00 → AAS ⇒ AL=09, AH=FF ⇒ AX=FF09 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x00FF
    aas
    SAVE_FLAGS
    ASSERT_AX 0xFF09
    CHECK_CF 1
    CHECK_AF 1

; 24) 9 - 9 = 0 with borrow flag set as AF_in=1 → AL=00, AH=01, AF_in=1 ⇒ AAS ⇒ AL=FA&0F=0A, AH=00 ⇒ AX=000A ; CF=1 AF=1
    mov ah, [patA]           ; preset AF=1
    sahf
    mov ax, 0x0100
    aas
    SAVE_FLAGS
    ASSERT_AX 0x000A
    CHECK_CF 1
    CHECK_AF 1

; 25) 6 - 4 = 2 (no adjust) → AL=12, AH=00 (pre-mask high nibble) ⇒ AAS ⇒ AX=0002 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x0012
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0002
    CHECK_CF 0
    CHECK_AF 0


; ---------------- High-nibble cleared even with no adjust ----------------

; 26) AL=D3, AH=10, AF_in=0 → no adjust ⇒ AL=03, AH=10 ⇒ AX=1003 ; CF=0 AF=0
    mov ah, [pat0]
    sahf
    mov ax, 0x10D3
    aas
    SAVE_FLAGS
    ASSERT_AX 0x1003
    CHECK_CF 0
    CHECK_AF 0


; ---------------- Wrap on AH decrement (8-bit) ----------------

; 27) AL=00, AF_in=1, AH=00 → adjust: AL=FA→&0F=0A; AH=FF ⇒ AX=FF0A ; CF=1 AF=1
    mov ah, [patA]
    sahf
    mov ax, 0x0000
    aas
    SAVE_FLAGS
    ASSERT_AX 0xFF0A
    CHECK_CF 1
    CHECK_AF 1

; 28) AL=FF, AF_in=0, AH=00 → adjust: (FF-06)=F9→&0F=09; AH=FF ⇒ AX=FF09 ; CF=1 AF=1
    mov ah, [pat0]
    sahf
    mov ax, 0x00FF
    aas
    SAVE_FLAGS
    ASSERT_AX 0xFF09
    CHECK_CF 1
    CHECK_AF 1


; ---------------- Mixed AF/CF preset (CF_in irrelevant) ----------------

; 29) CF_in=1, AF_in=1, AL=00, AH=00 → adjust ⇒ AX=FF0A ; CF=1 AF=1
    mov ah, [patAC]
    sahf
    mov ax, 0x0000
    aas
    SAVE_FLAGS
    ASSERT_AX 0xFF0A
    CHECK_CF 1
    CHECK_AF 1

; 30) CF_in=1, AF_in=0, AL=29, AH=01 → no adjust ⇒ AX=0109 ; CF=0 AF=0
    mov ah, [patC]
    sahf
    mov ax, 0x0129
    aas
    SAVE_FLAGS
    ASSERT_AX 0x0109
    CHECK_CF 0
    CHECK_AF 0

; ===================== AAS: ASCII → unpacked BCD → AAS → packed BCD =====================

; 31) '8' - '3' = 5  → packed 0x05 ; CF=AF=0
    mov ah, [pat0]              ; CF=0 AF=0
    sahf
    mov al, '8'                 ; unpack
    sub al, '0'
    mov bl, '3'
    sub bl, '0'
    sub al, bl                  ; 8-3 = 5
    mov ah, 0                   ; prepare AH for AAS
    aas                         ; no adjust
    SAVE_FLAGS
    and al, 0x0F
    mov ah, 0
    ASSERT_AX 0x0005
    CHECK_CF 0
    CHECK_AF 0

; 32) '7' - '8' = -1 → packed 0x09 ; CF=AF=1 (borrow)
    mov ah, [pat0]
    sahf
    mov al, '7'
    sub al, '0'
    mov bl, '8'
    sub bl, '0'
    sub al, bl                  ; 7-8 = -1
    mov ah, 0
    aas                         ; adjust: AL=9, AH=FF
    SAVE_FLAGS
    and al, 0x0F
    mov ah, 0
    ASSERT_AX 0x0009
    CHECK_CF 1
    CHECK_AF 1

; 33) '9' - '9' = 0  → 0x00 ; CF=AF=0
    mov ah, [pat0]
    sahf
    mov al, '9'
    sub al, '0'
    mov bl, '9'
    sub bl, '0'
    sub al, bl
    mov ah, 0
    aas
    SAVE_FLAGS
    and al, 0x0F
    mov ah, 0
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_AF 0

; 34) '0' - '0' = 0  → 0x00 ; CF=AF=0
    mov ah, [pat0]
    sahf
    mov al, '0'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    sub al, bl
    mov ah, 0
    aas
    SAVE_FLAGS
    and al, 0x0F
    mov ah, 0
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_AF 0

; 35) '4' - '5' = -1 → 0x09 ; CF=AF=1
    mov ah, [pat0]
    sahf
    mov al, '4'
    sub al, '0'
    mov bl, '5'
    sub bl, '0'
    sub al, bl
    mov ah, 0
    aas
    SAVE_FLAGS
    and al, 0x0F
    mov ah, 0
    ASSERT_AX 0x0009
    CHECK_CF 1
    CHECK_AF 1


; ===================== AAS: 2-digit ASCII − 2-digit ASCII → packed BCD (tens:ones) with borrow =====================

; (We propagate decimal borrow via CH between digit stages)

; 36) "34" - "12" = 22 → 0x22 ; final CF=AF=0
    mov ah, [pat0]
    sahf
    ; ones: '4' - '2'
    mov al, '4'       ; 4
    sub al, '0'
    mov bl, '2'       ; 2
    sub bl, '0'
    sub al, bl
    mov ah, 0
    aas               ; AL=2
    lahf
    mov ch, ah
    and ch, 1         ; ch = prior decimal borrow (0)

    mov bh, al        ; ones digit = 2

    ; tens: '3' - '1' - borrow(0)
    mov al, '3'
    sub al, '0'
    mov bl, '1'
    sub bl, '0'
    sub al, bl        ; sets AF for AAS
    lahf              ; save AF
    sub al, ch        ; apply borrow
    sahf              ; restore AF for AAS decision
    mov ah, 0
    aas               ; AL=2
    SAVE_FLAGS

    mov bl, al        ; tens digit = 2
    ; pack tens:ones
    mov dl, bl
    shl dl, 4
    or  dl, bh
    mov al, dl
    mov ah, 0
    ASSERT_AX 0x0022
    CHECK_CF 0
    CHECK_AF 0

; 37) "73" - "59" = 14 → 0x14 ; final CF=AF=0 (borrow from ones only)
    mov ah, [pat0]
    sahf
    ; ones: 3 - 9
    mov al, '3'
    sub al, '0'
    mov bl, '9'
    sub bl, '0'
    sub al, bl
    mov ah, 0
    aas               ; AL=4, borrow→CF=1
    lahf
    mov ch, ah
    and ch, 1         ; ch=1
    mov bh, al

    ; tens: 7 - 5 - 1
    mov al, '7'
    sub al, '0'
    mov bl, '5'
    sub bl, '0'
    sub al, bl        ; 2
    lahf
    sub al, ch        ; 2 - 1 = 1
    sahf
    mov ah, 0
    aas               ; no adjust
    SAVE_FLAGS

    mov bl, al
    mov dl, bl
    shl dl, 4
    or  dl, bh        ; 0x14
    mov al, dl
    mov ah, 0
    ASSERT_AX 0x0014
    CHECK_CF 0
    CHECK_AF 0

; 38) "50" - "18" = 32 → 0x32 ; final CF=AF=0
    mov ah, [pat0]
    sahf
    ; ones: 0 - 8
    mov al, '0'
    sub al, '0'
    mov bl, '8'
    sub bl, '0'
    sub al, bl
    mov ah, 0
    aas               ; AL=2, borrow
    lahf
    mov ch, ah
    and ch, 1         ; ch=1
    mov bh, al

    ; tens: 5 - 1 - 1
    mov al, '5'
    sub al, '0'
    mov bl, '1'
    sub bl, '0'
    sub al, bl        ; 4
    lahf
    sub al, ch        ; 3
    sahf
    mov ah, 0
    aas               ; AL=3
    SAVE_FLAGS

    mov bl, al
    mov dl, bl
    shl dl, 4
    or  dl, bh        ; 0x32
    mov al, dl
    mov ah, 0
    ASSERT_AX 0x0032
    CHECK_CF 0
    CHECK_AF 0

; 39) "10" - "01" = 09 → 0x09 ; final CF=AF=0
    mov ah, [pat0]
    sahf
    ; ones: 0 - 1
    mov al, '0'
    sub al, '0'
    mov bl, '1'
    sub bl, '0'
    sub al, bl
    mov ah, 0
    aas               ; AL=9, borrow
    lahf
    mov ch, ah
    and ch, 1         ; ch=1
    mov bh, al

    ; tens: 1 - 0 - 1
    mov al, '1'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    sub al, bl        ; 1
    lahf
    sub al, ch        ; 0
    sahf
    mov ah, 0
    aas               ; AL=0
    SAVE_FLAGS

    ; pack 0x09 (tens=0, ones=9)
    mov dl, 0
    shl dl, 4
    or  dl, bh
    mov al, dl
    mov ah, 0
    ASSERT_AX 0x0009
    CHECK_CF 0
    CHECK_AF 0


; ===================== AAS: 3/4-digit ASCII − ASCII → packed BCD across AX =====================

; Convention: AX packed as AH = (thousands<<4) | hundreds, AL = (tens<<4) | ones

; 40) "500" - "001" = 499 → AX=0x0499 ; final CF=AF=0
    mov ah, [pat0]
    sahf
    ; ones: 0 - 1
    mov al, '0'
    sub al, '0'
    mov bl, '1'
    sub bl, '0'
    sub al, bl
    mov ah, 0
    aas               ; AL=9, borrow
    lahf
    mov ch, ah
    and ch, 1
    mov bh, al        ; ones=9

    ; tens: 0 - 0 - borrow
    mov al, '0'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    sub al, bl        ; 0
    lahf
    sub al, ch        ; -1
    sahf
    mov ah, 0
    aas               ; AL=9, borrow
    lahf
    mov ch, ah
    and ch, 1
    mov dl, al        ; tens=9

    ; hundreds: 5 - 0 - borrow
    mov al, '5'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    sub al, bl        ; 5
    lahf
    sub al, ch        ; 4
    sahf
    mov ah, 0
    aas               ; AL=4, no borrow
    SAVE_FLAGS
    mov dh, al        ; hundreds=4

    ; pack: AH=(0<<4)|4 = 0x04 ; AL=(9<<4)|9 = 0x99
    mov ah, dh
    mov al, dl
    shl al, 4
    or  al, bh
    ASSERT_AX 0x0499
    CHECK_CF 0
    CHECK_AF 0

; 41) "1000" - "0001" = 0999 → AX=0x0999 ; final CF=AF=0
    mov ah, [pat0]
    sahf
    ; ones: 0 - 1
    mov al, '0'
    sub al, '0'
    mov bl, '1'
    sub bl, '0'
    sub al, bl
    mov ah, 0
    aas               ; 9, borrow
    lahf
    mov ch, ah
    and ch, 1
    mov bh, al        ; ones=9

    ; tens: 0 - 0 - 1
    mov al, '0'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    sub al, bl
    lahf
    sub al, ch
    sahf
    mov ah, 0
    aas               ; 9, borrow
    lahf
    mov ch, ah
    and ch, 1
    mov dl, al        ; tens=9

    ; hundreds: 0 - 0 - 1
    mov al, '0'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    sub al, bl
    lahf
    sub al, ch
    sahf
    mov ah, 0
    aas               ; 9, borrow
    lahf
    mov ch, ah
    and ch, 1
    mov dh, al        ; hundreds=9

    ; thousands: 1 - 0 - 1
    mov al, '1'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    sub al, bl
    lahf
    sub al, ch        ; -> 0
    sahf
    mov ah, 0
    aas               ; 0, no borrow
    SAVE_FLAGS
    ; thousands=0 (discard)

    ; pack: AH=(0<<4)|hundreds=0x09 ; AL=(tens<<4)|ones=0x99
    mov ah, dh
    mov al, dl
    shl al, 4
    or  al, bh
    ASSERT_AX 0x0999
    CHECK_CF 0
    CHECK_AF 0

; 42) "200" - "199" = 001 → AX=0x0001 ; final CF=AF=0
    mov ah, [pat0]
    sahf
    ; ones: 0 - 9
    mov al, '0'
    sub al, '0'
    mov bl, '9'
    sub bl, '0'
    sub al, bl
    mov ah, 0
    aas               ; 1, borrow
    lahf
    mov ch, ah
    and ch, 1
    mov bh, al        ; ones=1

    ; tens: 0 - 9 - 1
    mov al, '0'
    sub al, '0'
    mov bl, '9'
    sub bl, '0'
    sub al, bl
    lahf
    sub al, ch
    sahf
    mov ah, 0
    aas               ; 0, borrow
    lahf
    mov ch, ah
    and ch, 1
    mov cl, al        ; tens=0

    ; hundreds: 2 - 1 - 1 → 0
    mov al, '2'
    sub al, '0'
    mov dl, '1'
    sub dl, '0'
    sub al, dl
    lahf
    sub al, ch
    sahf
    mov ah, 0
    aas               ; 0, no borrow
    SAVE_FLAGS
    mov dh, al        ; hundreds=0

    ; pack: 0x0001
    mov ah, dh        ; (0<<4)|0
    mov al, cl
    shl al, 4
    or  al, bh
    ASSERT_AX 0x0001
    CHECK_CF 0
    CHECK_AF 0

; 43) "100" - "100" = 000 → AX=0x0000 ; final CF=AF=0
    mov ah, [pat0]
    sahf
    ; ones: 0 - 0
    mov al, '0'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    sub al, bl
    mov ah, 0
    aas
    lahf
    mov ch, ah
    and ch, 1
    mov bh, al        ; ones=0

    ; tens: 0 - 0 - 0
    mov al, '0'
    sub al, '0'
    mov bl, '0'
    sub bl, '0'
    sub al, bl
    lahf
    sub al, ch
    sahf
    mov ah, 0
    aas
    lahf
    mov ch, ah
    and ch, 1
    mov cl, al        ; tens=0

    ; hundreds: 1 - 1 - 0
    mov al, '1'
    sub al, '0'
    mov dl, '1'
    sub dl, '0'
    sub al, dl
    lahf
    sub al, ch
    sahf
    mov ah, 0
    aas
    SAVE_FLAGS
    mov dh, al        ; hundreds=0

    mov ah, dh        ; (0<<4)|0
    mov al, cl
    shl al, 4
    or  al, bh
    ASSERT_AX 0x0000
    CHECK_CF 0
    CHECK_AF 0


; ---------------- Exit to DOS ----------------
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0

; SAHF patterns to preset AF/CF before AAS (other flag bits irrelevant for AAS)
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
