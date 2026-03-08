; mov.asm — Thorough tests for MOV (registers, r/m, immediates, moffs, segments, overrides)
; Harness:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH     (single-flag checks from saved FLAGS)

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
%macro ASSERT_CX 1
    mov ax, cx
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_BX 1
    mov ax, bx
    mov bx, %1
    int 0x23
%endmacro
%macro ASSERT_DX 1
    mov ax, dx
    mov bx, %1
    int 0x23
%endmacro

; r8/r16 viewers (after SAVE_FLAGS so flag clobbers are fine)
%macro ASSERT_R8 2
    push ax
    xor ah, ah
    mov al, %1
    mov bx, %2
    int 0x23
    pop ax
%endmacro
%macro ASSERT_R16 2
    push ax
    mov ax, %1
    mov bx, %2
    int 0x23
    pop ax
%endmacro

; Flag bit checks
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

; Memory asserts (don’t touch flags)
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

; ===================== Register-to-register =====================

; 1) MOV r8,r8: BL <- AL; AL=0x34; flags preserved
    mov al, 0x34
    mov ah, [pat_all1]
    sahf
    mov bl, 0x00
    mov bl, al
    SAVE_FLAGS
    ASSERT_R8 bl, 0x0034
    ASSERT_R8 al, 0x0034
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 2) MOV r16,r16: DX <- AX; AX=0xBEEF; flags preserved
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xBEEF
    mov dx, 0
    mov dx, ax
    SAVE_FLAGS
    ASSERT_R16 dx, 0xBEEF
    ASSERT_R16 ax, 0xBEEF
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 3) MOV to same register (no-op): AX <- AX; flags preserved
    mov ah, [pat_all1]
    sahf
    mov ax, 0x1234
    mov ax, ax
    SAVE_FLAGS
    ASSERT_AX 0x1234
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 4) Cross 8-bit halves: AH <- AL; AL stays; flags preserved
    mov ax, 0x56A5            ; AL=0xA5, AH=0x56
    mov ah, [pat_zf0]
    sahf
    mov ah, al
    SAVE_FLAGS
    ASSERT_AX 0xA5A5
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Immediate to register =====================

; 5) MOV r8, imm8: CL <- 0x7E
    mov ah, [pat_all1]
    sahf
    mov cl, 0x7E
    SAVE_FLAGS
    ASSERT_R8 cl, 0x007E
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 6) MOV r16, imm16: BX <- 0xCAFE
    mov ah, [pat_zf0]
    sahf
    mov bx, 0xCAFE
    SAVE_FLAGS
    ASSERT_BX 0xCAFE
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Memory ↔ register (byte) =====================

; 7) MOV r8, r/m8: AL <- [m8_si]
    mov ah, [pat_all1]
    sahf
    mov si, m8_si
    mov al, 0
    mov al, [si]
    SAVE_FLAGS
    ASSERT_R8 al, 0x0033
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 8) MOV r/m8, r8: [m8_di] <- DL (0x9C)
    mov ah, [pat_zf0]
    sahf
    mov di, m8_di
    mov dl, 0x9C
    mov [di], dl
    SAVE_FLAGS
    ASSERT_MEMB m8_di, 0x009C
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Memory ↔ register (word) =====================

; 9) MOV r16, r/m16: AX <- [m16_bx_si+disp]
    mov ah, [pat_all1]
    sahf
    mov bx, m16_base
    mov si, 2
    mov ax, 0
    mov ax, [bx+si]           ; m16_base+2 = 0x1357
    SAVE_FLAGS
    ASSERT_AX 0x1357
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 10) MOV r/m16, r16: [m16_bx_di+disp] <- DX (0xBABA)
    mov ah, [pat_zf0]
    sahf
    mov dx, 0xBABA
    mov bx, m16_base2
    mov di, 2
    mov [bx+di], dx
    SAVE_FLAGS
    ASSERT_MEMW m16_base2+2, 0xBABA
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Immediate to memory =====================

; 11) MOV byte [mem8_imm], imm8
    mov ah, [pat_all1]
    sahf
    mov byte [mem8_imm], 0xA1
    SAVE_FLAGS
    ASSERT_MEMB mem8_imm, 0x00A1
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 12) MOV word [mem16_imm], imm16
    mov ah, [pat_zf0]
    sahf
    mov word [mem16_imm], 0x0BEE
    SAVE_FLAGS
    ASSERT_MEMW mem16_imm, 0x0BEE
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Accumulator moffs (direct) =====================

; 13) MOV AL, [moffs8]
    mov ah, [pat_all1]
    sahf
    mov al, 0
    mov al, [moffs8]
    SAVE_FLAGS
    ASSERT_R8 al, 0x005A
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 14) MOV [moffs8], AL
    mov ah, [pat_zf0]
    sahf
    mov al, 0xC3
    mov [moffs8], al
    SAVE_FLAGS
    ASSERT_MEMB moffs8, 0x00C3
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 15) MOV AX, [moffs16]
    mov ah, [pat_all1]
    sahf
    mov ax, 0
    mov ax, [moffs16]
    SAVE_FLAGS
    ASSERT_AX 0x1BAD
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 16) MOV [moffs16], AX
    mov ah, [pat_zf0]
    sahf
    mov ax, 0x4242
    mov [moffs16], ax
    SAVE_FLAGS
    ASSERT_MEMW moffs16, 0x4242
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Segment overrides with memory =====================

; 17) Read with ES override (ES==DS here): AL <- ES:[seg_over_b]
    mov ah, [pat_all1]
    sahf
    mov al, 0
    mov al, es:[seg_over_b]
    SAVE_FLAGS
    ASSERT_R8 al, 0x0077
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 18) Write with ES override: ES:[seg_over_w] <- AX
    mov ah, [pat_zf0]
    sahf
    mov ax, 0xDEAD
    mov es:[seg_over_w], ax
    SAVE_FLAGS
    ASSERT_MEMW seg_over_w, 0xDEAD
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Segment register moves (safe set) =====================

; 19) MOV r/m16, Sreg: BX <- DS
    mov ah, [pat_all1]
    sahf
    mov bx, 0
    mov bx, ds
    SAVE_FLAGS
    ASSERT_BX ds             ; compare BX with current DS value
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 20) MOV r/m16, Sreg: [seg_dump_cs] <- CS
    mov ah, [pat_zf0]
    sahf
    mov word [seg_dump_cs], cs
    SAVE_FLAGS
    ; CS equals current code segment; DS=CS in this .COM, so:
    ASSERT_MEMW seg_dump_cs, cs
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 21) MOV Sreg, r/m16: ES <- AX (AX preloaded with ES value so it’s a no-op)
    mov ah, [pat_all1]
    sahf
    mov ax, es
    mov es, ax               ; ES unchanged
    SAVE_FLAGS
    ASSERT_R16 ax, es
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 22) MOV Sreg, r/m16: DS <- [seg_copy_ds] (holds current DS; safe reload)
    mov ah, [pat_zf0]
    sahf
    mov word [seg_copy_ds], ds
    mov ds, [seg_copy_ds]    ; DS reloaded with same value
    SAVE_FLAGS
    ASSERT_R16 ds, [seg_copy_ds]
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Odd-address word moves =====================

; 23) MOV AX, [oddw+1]  (reads 0xABCD)
    mov ah, [pat_all1]
    sahf
    mov ax, 0
    mov ax, [oddw+1]
    SAVE_FLAGS
    ASSERT_AX 0xABCD
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; 24) MOV [oddw2+1], DX (writes 0x1357 at odd address)
    mov ah, [pat_zf0]
    sahf
    mov dx, 0x1357
    mov [oddw2+1], dx
    SAVE_FLAGS
    ASSERT_MEMW oddw2+1, 0x1357
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; ===================== Helpers for the sweeps =====================
%macro ASSERT_EQ_R8 2
    push ax
    push bx
%if %2 == al
    mov bl, %1
    mov bh, 0
    mov al, %2
    mov ah, 0
%elif %2 == ah
    mov bl, %1
    mov bh, 0
    mov al, %2
    mov ah, 0
%else
    mov al, %1
    mov ah, 0
    mov bl, %2
    mov bh, 0
%endif
    int 0x23
    pop bx
    pop ax
%endmacro

%macro ASSERT_EQ_R16 2
    push ax
    push bx
    mov ax, %1
    mov bx, %2
    int 0x23
    pop bx
    pop ax
%endmacro

%macro LOAD_8BASE 0
    mov al, 0x10
    mov cl, 0x21
    mov dl, 0x32
    mov bl, 0x43
    mov ah, 0x54
    mov ch, 0x65
    mov dh, 0x76
    mov bh, 0x87
%endmacro

%macro LOAD_16BASE 0
    mov ax, 0x1001
    mov cx, 0x2002
    mov dx, 0x3003
    mov bx, 0x4004
    mov bp, 0x5005
    mov si, 0x6006
    mov di, 0x7007
%endmacro


; ===================== Exhaustive r8 ← r8 sweep (flags preserved) =====================
; Pattern: for each source, reload baseline registers, do all 8 destinations, assert equality,
; then once per row assert the flags stayed the same (MOV doesn’t modify flags).
; We seed flags with pat_zf0: SF=1 ZF=0 AF=1 PF=1 CF=1.

; Row src = AL
    mov ah, [pat_zf0]     ; ZF=0 proves preservation clearly
    sahf
    LOAD_8BASE
    mov al, al   ; d=AL s=AL
    ASSERT_EQ_R8 al, al
    mov cl, al   ; d=CL s=AL
    ASSERT_EQ_R8 cl, al
    mov dl, al
    ASSERT_EQ_R8 dl, al
    mov bl, al
    ASSERT_EQ_R8 bl, al
    mov ah, al
    ASSERT_EQ_R8 ah, al
    mov ch, al
    ASSERT_EQ_R8 ch, al
    mov dh, al
    ASSERT_EQ_R8 dh, al
    mov bh, al
    ASSERT_EQ_R8 bh, al
    SAVE_FLAGS
    CHECK_SF 1   ; SF=1, ZF=0, AF=1, PF=1, CF=1 preserved
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = CL
    mov ah, [pat_zf0]
    sahf
    LOAD_8BASE
    mov al, cl
    ASSERT_EQ_R8 al, cl
    mov cl, cl
    ASSERT_EQ_R8 cl, cl
    mov dl, cl
    ASSERT_EQ_R8 dl, cl
    mov bl, cl
    ASSERT_EQ_R8 bl, cl
    mov ah, cl
    ASSERT_EQ_R8 ah, cl
    mov ch, cl
    ASSERT_EQ_R8 ch, cl
    mov dh, cl
    ASSERT_EQ_R8 dh, cl
    mov bh, cl
    ASSERT_EQ_R8 bh, cl
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = DL
    mov ah, [pat_zf0]
    sahf
    LOAD_8BASE
    mov al, dl
    ASSERT_EQ_R8 al, dl
    mov cl, dl
    ASSERT_EQ_R8 cl, dl
    mov dl, dl
    ASSERT_EQ_R8 dl, dl
    mov bl, dl
    ASSERT_EQ_R8 bl, dl
    mov ah, dl
    ASSERT_EQ_R8 ah, dl
    mov ch, dl
    ASSERT_EQ_R8 ch, dl
    mov dh, dl
    ASSERT_EQ_R8 dh, dl
    mov bh, dl
    ASSERT_EQ_R8 bh, dl
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = BL
    mov ah, [pat_zf0]
    sahf
    LOAD_8BASE
    mov al, bl
    ASSERT_EQ_R8 al, bl
    mov cl, bl
    ASSERT_EQ_R8 cl, bl
    mov dl, bl
    ASSERT_EQ_R8 dl, bl
    mov bl, bl
    ASSERT_EQ_R8 bl, bl
    mov ah, bl
    ASSERT_EQ_R8 ah, bl
    mov ch, bl
    ASSERT_EQ_R8 ch, bl
    mov dh, bl
    ASSERT_EQ_R8 dh, bl
    mov bh, bl
    ASSERT_EQ_R8 bh, bl
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = AH
    mov ah, [pat_zf0]
    sahf
    LOAD_8BASE
    mov al, ah
    ASSERT_EQ_R8 al, ah
    mov cl, ah
    ASSERT_EQ_R8 cl, ah
    mov dl, ah
    ASSERT_EQ_R8 dl, ah
    mov bl, ah
    ASSERT_EQ_R8 bl, ah
    mov ah, ah
    ASSERT_EQ_R8 ah, ah
    mov ch, ah
    ASSERT_EQ_R8 ch, ah
    mov dh, ah
    ASSERT_EQ_R8 dh, ah
    mov bh, ah
    ASSERT_EQ_R8 bh, ah
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = CH
    mov ah, [pat_zf0]
    sahf
    LOAD_8BASE
    mov al, ch
    ASSERT_EQ_R8 al, ch
    mov cl, ch
    ASSERT_EQ_R8 cl, ch
    mov dl, ch
    ASSERT_EQ_R8 dl, ch
    mov bl, ch
    ASSERT_EQ_R8 bl, ch
    mov ah, ch
    ASSERT_EQ_R8 ah, ch
    mov ch, ch
    ASSERT_EQ_R8 ch, ch
    mov dh, ch
    ASSERT_EQ_R8 dh, ch
    mov bh, ch
    ASSERT_EQ_R8 bh, ch
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = DH
    mov ah, [pat_zf0]
    sahf
    LOAD_8BASE
    mov al, dh
    ASSERT_EQ_R8 al, dh
    mov cl, dh
    ASSERT_EQ_R8 cl, dh
    mov dl, dh
    ASSERT_EQ_R8 dl, dh
    mov bl, dh
    ASSERT_EQ_R8 bl, dh
    mov ah, dh
    ASSERT_EQ_R8 ah, dh
    mov ch, dh
    ASSERT_EQ_R8 ch, dh
    mov dh, dh
    ASSERT_EQ_R8 dh, dh
    mov bh, dh
    ASSERT_EQ_R8 bh, dh
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = BH
    mov ah, [pat_zf0]
    sahf
    LOAD_8BASE
    mov al, bh
    ASSERT_EQ_R8 al, bh
    mov cl, bh
    ASSERT_EQ_R8 cl, bh
    mov dl, bh
    ASSERT_EQ_R8 dl, bh
    mov bl, bh
    ASSERT_EQ_R8 bl, bh
    mov ah, bh
    ASSERT_EQ_R8 ah, bh
    mov ch, bh
    ASSERT_EQ_R8 ch, bh
    mov dh, bh
    ASSERT_EQ_R8 dh, bh
    mov bh, bh
    ASSERT_EQ_R8 bh, bh
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 0
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== Broad r16 ← r16 sweep (AX,CX,DX,BX,BP,SI,DI) =====================
; (SP omitted intentionally to avoid stack disruption in the harness.)

; Row src = AX
    mov ah, [pat_all1]
    sahf
    LOAD_16BASE
    mov ax, ax   ; d=AX s=AX
    ASSERT_EQ_R16 ax, ax
    mov cx, ax
    ASSERT_EQ_R16 cx, ax
    mov dx, ax
    ASSERT_EQ_R16 dx, ax
    mov bx, ax
    ASSERT_EQ_R16 bx, ax
    mov bp, ax
    ASSERT_EQ_R16 bp, ax
    mov si, ax
    ASSERT_EQ_R16 si, ax
    mov di, ax
    ASSERT_EQ_R16 di, ax
    SAVE_FLAGS
    CHECK_SF 1  ; pat_all1 → SF=1,ZF=1,AF=1,PF=1,CF=1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = CX
    mov ah, [pat_all1]
    sahf
    LOAD_16BASE
    mov ax, cx
    ASSERT_EQ_R16 ax, cx
    mov cx, cx
    ASSERT_EQ_R16 cx, cx
    mov dx, cx
    ASSERT_EQ_R16 dx, cx
    mov bx, cx
    ASSERT_EQ_R16 bx, cx
    mov bp, cx
    ASSERT_EQ_R16 bp, cx
    mov si, cx
    ASSERT_EQ_R16 si, cx
    mov di, cx
    ASSERT_EQ_R16 di, cx
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = DX
    mov ah, [pat_all1]
    sahf
    LOAD_16BASE
    mov ax, dx
    ASSERT_EQ_R16 ax, dx
    mov cx, dx
    ASSERT_EQ_R16 cx, dx
    mov dx, dx
    ASSERT_EQ_R16 dx, dx
    mov bx, dx
    ASSERT_EQ_R16 bx, dx
    mov bp, dx
    ASSERT_EQ_R16 bp, dx
    mov si, dx
    ASSERT_EQ_R16 si, dx
    mov di, dx
    ASSERT_EQ_R16 di, dx
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = BX
    mov ah, [pat_all1]
    sahf
    LOAD_16BASE
    mov ax, bx
    ASSERT_EQ_R16 ax, bx
    mov cx, bx
    ASSERT_EQ_R16 cx, bx
    mov dx, bx
    ASSERT_EQ_R16 dx, bx
    mov bx, bx
    ASSERT_EQ_R16 bx, bx
    mov bp, bx
    ASSERT_EQ_R16 bp, bx
    mov si, bx
    ASSERT_EQ_R16 si, bx
    mov di, bx
    ASSERT_EQ_R16 di, bx
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = BP
    mov ah, [pat_all1]
    sahf
    LOAD_16BASE
    mov ax, bp
    ASSERT_EQ_R16 ax, bp
    mov cx, bp
    ASSERT_EQ_R16 cx, bp
    mov dx, bp
    ASSERT_EQ_R16 dx, bp
    mov bx, bp
    ASSERT_EQ_R16 bx, bp
    mov bp, bp
    ASSERT_EQ_R16 bp, bp
    mov si, bp
    ASSERT_EQ_R16 si, bp
    mov di, bp
    ASSERT_EQ_R16 di, bp
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = SI
    mov ah, [pat_all1]
    sahf
    LOAD_16BASE
    mov ax, si
    ASSERT_EQ_R16 ax, si
    mov cx, si
    ASSERT_EQ_R16 cx, si
    mov dx, si
    ASSERT_EQ_R16 dx, si
    mov bx, si
    ASSERT_EQ_R16 bx, si
    mov bp, si
    ASSERT_EQ_R16 bp, si
    mov si, si
    ASSERT_EQ_R16 si, si
    mov di, si
    ASSERT_EQ_R16 di, si
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1

; Row src = DI
    mov ah, [pat_all1]
    sahf
    LOAD_16BASE
    mov ax, di
    ASSERT_EQ_R16 ax, di
    mov cx, di
    ASSERT_EQ_R16 cx, di
    mov dx, di
    ASSERT_EQ_R16 dx, di
    mov bx, di
    ASSERT_EQ_R16 bx, di
    mov bp, di
    ASSERT_EQ_R16 bp, di
    mov si, di
    ASSERT_EQ_R16 si, di
    mov di, di
    ASSERT_EQ_R16 di, di
    SAVE_FLAGS
    CHECK_SF 1
    CHECK_ZF 1
    CHECK_AF 1
    CHECK_PF 1
    CHECK_CF 1


; ===================== Segment-override sweep (safe) =====================
; We only READ with CS: (to avoid writing into code), and we READ/WRITE with ES:.
; DS==ES==CS in this .COM, but these still exercise the prefix decoding & effective addressing.

; 25) Copy 4 bytes using CS: reads → DS: writes
    cld
    mov bx, seg_cs_src     ; 0x11, 0x22, 0x33, 0x44
    mov di, seg_copy_ds3
    mov cx, 4
.copy_cs:
    mov al, cs:[bx]
    mov [di], al
    inc bx
    inc di
    loop .copy_cs
    ; verify
    mov si, seg_copy_ds3
    mov bp, seg_chk        ; 0x11, 0x22, 0x33, 0x44
    mov cx, 4
.v_cs_chk:
    mov al, [si]
    mov ah, 0
    mov bl, [bp]
    mov bh, 0
    int 0x23
    inc si
    inc bp
    loop .v_cs_chk

; 26) Copy 4 bytes using ES: reads → DS: writes
    mov bx, seg_es_src
    mov di, seg_copy_ds2
    mov cx, 4
.copy_es_r:
    mov al, es:[bx]
    mov [di], al
    inc bx
    inc di
    loop .copy_es_r
    ; verify matches source
    mov si, seg_copy_ds2
    mov bp, seg_es_src
    mov cx, 4
.v_es_r_chk:
    mov al, [si]
    mov ah, 0
    mov dl, [bp]
    xor dh, dh
    mov bx, dx
    int 0x23
    inc si
    inc bp
    loop .v_es_r_chk

; 27) Copy 3 words DS: reads → ES: writes (ES: prefix on write path)
    mov si, seg_w_src
    mov di, seg_w_dst_es
    mov cx, 3
.copy_es_w:
    mov ax, [si]
    mov es:[di], ax
    add si, 2
    add di, 2
    loop .copy_es_w
    ; verify words in ES buffer
    mov si, seg_w_dst_es
    mov bp, seg_w_src
    mov cx, 3
.v_es_w_chk:
    mov ax, [si]
    mov dx, [bp]
    mov bx, dx
    int 0x23
    add si, 2
    add bp, 2
    loop .v_es_w_chk


; ===================== Exit =====================
exit:
    mov ax, 0x4C00
    int 0x21

; ---------- Data ----------
flags_store: dw 0

; Byte/word memory used across tests
m8_si:       db 0x33, 0xCC
m8_di:       db 0xCC, 0xCC

m16_base:    dw 0x0000, 0x1357, 0x9999
m16_base2:   dw 0x0000, 0x0000, 0xBEEF, 0x7E7E

mem8_imm:    db 0xCC
mem16_imm:   dw 0x7E7E

moffs8:      db 0x5A
moffs16:     dw 0x1BAD

seg_over_b:  db 0x77
seg_over_w:  dw 0x7E7E

seg_dump_cs: dw 0x0000
seg_copy_ds: dw 0x0000

oddw:        db 0xEF,0xCD,0xAB,0x89,0xCC
oddw2:       db 0xCC,0xCC,0xCC,0xCC

; ---------------- Extra data for the sweeps ----------------
seg_cs_src:   db 0x11,0x22,0x33,0x44
seg_chk:      db 0x11,0x22,0x33,0x44
seg_copy_ds3: db 0xCC,0xCC,0xCC,0xCC

seg_es_src:   db 0xA0,0xB1,0xC2,0xD3
seg_copy_ds2: db 0xCC,0xCC,0xCC,0xCC

seg_w_src:    dw 0x1357,0x2468,0xBEEF
seg_w_dst_es: db 0xCC,0xCC,0xCC,0xCC,0xCC,0xCC

; SAHF patterns (SF/ZF/AF/PF/CF; OF unaffected by SAHF but MOV preserves it anyway)
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
