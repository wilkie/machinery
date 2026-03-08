; sahf.asm — Thorough tests for SAHF (16-bit)
; Harness:
;   int 0x23: assert AX == BX  (value/register equality)
;   int 0x22: assert AL == AH  (single-flag/byte equality)

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

%macro ASSERT_EQ_AX_BX 0
    int 0x23
%endmacro

; Compare AH-after with a saved byte in memory (using int 0x22)
%macro ASSERT_AH_EQ_MEM 1
    mov al, ah
    mov ah, [%1]
    int 0x22
%endmacro

; Flag checks read from [flags_store]
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
%macro CHECK_IF 1
    mov ax, [flags_store]
    mov cl, 9
    shr ax, cl
    and al, 1
    mov ah, %1
    int 0x22
%endmacro
%macro CHECK_DF 1
    mov ax, [flags_store]
    mov cl, 10
    shr ax, cl
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

; Helper: set IF/DF/OF etc. precisely via POPF
%macro SET_FLAGS 1
    push word %1
    popf
%endmacro

; ---------- Start ----------
start:
    ; set up stack address
    lea ax, stack
    mov sp, ax

    ; DS=CS, ES=DS
    push cs
    pop  ds
    push ds
    pop  es

; Patterns (for convenience in some tests)
pat_all0: db 0x00
pat_all1: db 0xFF
pat_mix : db 0xD5      ; 1101_0101b → SF=ZF=AF=PF=CF=1

; ===================== 1) SAHF with AH=00 → clear SF/ZF/AF/PF/CF; OF/IF/DF preserved =====================
t1:
    ; Baseline: all flags 0
    SET_FLAGS 0x0000
    mov ah, [pat_all0]
    mov [ah_saved], ah
    sahf
    push ax
    SAVE_FLAGS
    ; Check definable flags
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    ; Unaffected flags (still 0)
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 0
    ; AH unchanged
    pop ax
    ASSERT_AH_EQ_MEM ah_saved

; ===================== 2) SAHF with AH=FF → set SF/ZF/AF/PF/CF; OF/IF/DF preserved (0) =====================
t2:
    SET_FLAGS 0x0000
    mov ah, [pat_all1]
    mov [ah_saved], ah
    sahf
    push ax
    SAVE_FLAGS
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 0
    pop ax
    ASSERT_AH_EQ_MEM ah_saved

; ===================== 3) CF only (AH=01); OF preset to 1 must remain 1 =====================
t3:
    ; Preset OF=1 only
    SET_FLAGS 0x0800
    mov ah, 0x01
    mov [ah_saved], ah
    sahf
    push ax
    SAVE_FLAGS
    CHECK_CF 1
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_OF 1       ; unaffected
    CHECK_IF 0
    CHECK_DF 0
    pop ax
    ASSERT_AH_EQ_MEM ah_saved

; ===================== 4) PF only (AH=04); IF preset to 1 must remain 1 =====================
t4:
    SET_FLAGS 0x0200
    mov ah, 0x04
    mov [ah_saved], ah
    sahf
    push ax
    SAVE_FLAGS
    CHECK_CF 0
    CHECK_PF 1
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_IF 1
    CHECK_DF 0
    CHECK_OF 0
    pop ax
    ASSERT_AH_EQ_MEM ah_saved

; ===================== 5) AF only (AH=10); DF preset to 1 must remain 1 =====================
t5:
    SET_FLAGS 0x0400
    mov ah, 0x10
    mov [ah_saved], ah
    sahf
    push ax
    SAVE_FLAGS
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 1
    CHECK_ZF 0
    CHECK_SF 0
    CHECK_IF 0
    CHECK_DF 1
    CHECK_OF 0
    pop ax
    ASSERT_AH_EQ_MEM ah_saved

; ===================== 6) ZF only (AH=40) =====================
t6:
    SET_FLAGS 0x0000
    mov ah, 0x40
    mov [ah_saved], ah
    sahf
    push ax
    SAVE_FLAGS
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 1
    CHECK_SF 0
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 0
    pop ax
    ASSERT_AH_EQ_MEM ah_saved

; ===================== 7) SF only (AH=80); ensure OF remains 0 =====================
t7:
    SET_FLAGS 0x0000
    mov ah, 0x80
    mov [ah_saved], ah
    sahf
    push ax
    SAVE_FLAGS
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_OF 0
    pop ax
    ASSERT_AH_EQ_MEM ah_saved

; ===================== 8) Mixed pattern (AH=D5) → all five set; IF/DF/OF=0 preserved =====================
t8:
    SET_FLAGS 0x0000
    mov ah, [pat_mix]           ; 0xD5 → SF=ZF=AF=PF=CF=1
    mov [ah_saved], ah
    sahf
    push ax
    SAVE_FLAGS
    CHECK_CF 1
    CHECK_PF 1
    CHECK_AF 1
    CHECK_ZF 1
    CHECK_SF 1
    CHECK_IF 0
    CHECK_DF 0
    CHECK_OF 0
    pop ax
    ASSERT_AH_EQ_MEM ah_saved

; ===================== 9) Random mask: AH=82 (SF=1, CF=0, others 0 except reserved bits) =====================
t9:
    SET_FLAGS 0x0E00            ; preset OF=DF=IF=1 to prove unaffected
    mov ah, 0x82
    mov [ah_saved], ah
    sahf
    push ax
    SAVE_FLAGS
    CHECK_CF 0
    CHECK_PF 0
    CHECK_AF 0
    CHECK_ZF 0
    CHECK_SF 1
    CHECK_IF 1
    CHECK_DF 1
    CHECK_OF 1
    pop ax
    ASSERT_AH_EQ_MEM ah_saved

; ===================== 10) AX unchanged across SAHF =====================
t10:
    SET_FLAGS 0x0000
    mov ax, 0xBEEF
    mov [ah_saved], ah
    sahf                        ; AH is source; AX must remain 0xBEEF
    push ax
    SAVE_FLAGS
    ASSERT_AX 0xBEEF
    pop ax
    ASSERT_AH_EQ_MEM ah_saved

; ===================== 11) LAHF/SAHF round-trip normalization check =====================
; After SAHF with an arbitrary AH, LAHF must read back AH' with:
;   bit7=SF, bit6=ZF, bit5=0, bit4=AF, bit3=0, bit2=PF, bit1=1, bit0=CF
t11:
    SET_FLAGS 0x0800            ; OF=1 (shouldn't matter for LAHF image)
    mov ah, 0xF3                ; 1111_0011b → SF=1 ZF=1 AF=0 PF=0 CF=1
    mov [ah_in], ah
    sahf
    lahf                        ; capture normalized flags image to AH
    mov [ah_out], ah
    ; Build expected: keep bits 7,6,4,2,0 from ah_in; set bit1=1; clear bits3&5
    mov al, [ah_in]
    and al, 11010101b           ; mask 7,6,4,2,0
    or  al, 00000010b           ; set bit1
    ; expected is in AL, actual in [ah_out]
    mov ah, [ah_out]
    int 0x22                    ; AL == AH  (expected == actual)

; ===================== 12) OF unaffected both ways (preset 1 then 0) =====================
t12a:
    SET_FLAGS 0x0800            ; OF=1
    mov ah, 0x00
    sahf
    SAVE_FLAGS
    CHECK_OF 1
t12b:
    SET_FLAGS 0x0000            ; OF=0
    mov ah, 0xFF
    sahf
    SAVE_FLAGS
    CHECK_OF 0

; ===================== 13) IF/DF unaffected both ways =====================
t13a:
    SET_FLAGS 0x0600            ; IF=1, DF=1
    mov ah, 0x00
    sahf
    SAVE_FLAGS
    CHECK_IF 1
    CHECK_DF 1
t13b:
    SET_FLAGS 0x0000            ; IF=0, DF=0
    mov ah, 0xFF
    sahf
    SAVE_FLAGS
    CHECK_IF 0
    CHECK_DF 0

; ===================== Exit =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------------- Data ----------------
flags_store: dw 0
ah_saved:    db 0
ah_in:       db 0
ah_out:      db 0

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
