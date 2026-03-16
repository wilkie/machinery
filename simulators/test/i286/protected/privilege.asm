; privilege.asm - Tests privilege level transitions via call gates and RETF
bits 16
org 0x100

; int 0x22 - asserts al == ah
; int 0x23 - asserts ax == bx

; Memory layout:
;   0x0000-0x0057: GDT (11 entries)
;   0x2000-0x202B: TSS (initial task)

section .text
start:

    ; =========================================================
    ; Set up GDT at linear address 0
    ;
    ;   Entry 0  (0x00): Null descriptor
    ;   Entry 1  (0x08): Code DPL=0 - base=0, limit=0xFFFF, exec/read
    ;   Entry 2  (0x10): Data DPL=0 - base=0, limit=0xFFFF, read/write
    ;   Entry 3  (0x18): Stack DPL=0 - base=0, limit=0xFFFF, read/write
    ;   Entry 4  (0x20): TSS - base=0x2000, limit=0x2B
    ;   Entry 5  (0x28): Code DPL=3 - base=0, limit=0xFFFF, exec/read
    ;   Entry 6  (0x30): Data DPL=3 - base=0, limit=0xFFFF, read/write
    ;   Entry 7  (0x38): Stack DPL=3 - base=0, limit=0xFFFF, read/write
    ;   Entry 8  (0x40): Call gate DPL=0 -> 0x08:gate_same_priv (0 params)
    ;   Entry 9  (0x48): Call gate DPL=3 -> 0x08:gate_priv_xfer (0 params)
    ;   Entry 10 (0x50): Call gate DPL=3 -> 0x08:gate_with_params (2 params)
    ; =========================================================

    xor ax, ax
    mov ds, ax

    ; Entry 0: Null
    mov word [0x00], 0x0000
    mov word [0x02], 0x0000
    mov word [0x04], 0x0000
    mov word [0x06], 0x0000

    ; Entry 1: Code DPL=0 (0x08) - exec/read
    ; Access 0x9A: P=1, DPL=00, S=1, type=101, A=0
    mov word [0x08], 0xFFFF
    mov word [0x0A], 0x0000
    mov byte [0x0C], 0x00
    mov byte [0x0D], 0x9A
    mov word [0x0E], 0x0000

    ; Entry 2: Data DPL=0 (0x10) - read/write
    ; Access 0x92: P=1, DPL=00, S=1, type=001, A=0
    mov word [0x10], 0xFFFF
    mov word [0x12], 0x0000
    mov byte [0x14], 0x00
    mov byte [0x15], 0x92
    mov word [0x16], 0x0000

    ; Entry 3: Stack DPL=0 (0x18) - read/write
    mov word [0x18], 0xFFFF
    mov word [0x1A], 0x0000
    mov byte [0x1C], 0x00
    mov byte [0x1D], 0x92
    mov word [0x1E], 0x0000

    ; Entry 4: TSS (0x20) - base=0x2000, limit=0x2B
    ; Access 0x81: P=1, DPL=0, S=0, A=1, type=000 (available TSS)
    mov word [0x20], 0x002B
    mov word [0x22], 0x2000
    mov byte [0x24], 0x00
    mov byte [0x25], 0x81
    mov word [0x26], 0x0000

    ; Entry 5: Code DPL=3 (0x28) - exec/read
    ; Access 0xFA: P=1, DPL=11, S=1, type=101, A=0
    mov word [0x28], 0xFFFF
    mov word [0x2A], 0x0000
    mov byte [0x2C], 0x00
    mov byte [0x2D], 0xFA
    mov word [0x2E], 0x0000

    ; Entry 6: Data DPL=3 (0x30) - read/write
    ; Access 0xF2: P=1, DPL=11, S=1, type=001, A=0
    mov word [0x30], 0xFFFF
    mov word [0x32], 0x0000
    mov byte [0x34], 0x00
    mov byte [0x35], 0xF2
    mov word [0x36], 0x0000

    ; Entry 7: Stack DPL=3 (0x38) - read/write
    ; Access 0xF2: P=1, DPL=11, S=1, type=001, A=0
    mov word [0x38], 0xFFFF
    mov word [0x3A], 0x0000
    mov byte [0x3C], 0x00
    mov byte [0x3D], 0xF2
    mov word [0x3E], 0x0000

    ; Entry 8: Call gate DPL=0 (0x40) -> 0x08:gate_same_priv, 0 params
    ; Access 0x84: P=1, DPL=00, S=0, A=0, type=010 (call gate)
    mov word [0x40], gate_same_priv   ; offset
    mov word [0x42], 0x0008           ; target CS selector
    mov byte [0x44], 0x00             ; word count = 0
    mov byte [0x45], 0x84             ; access
    mov word [0x46], 0x0000

    ; Entry 9: Call gate DPL=3 (0x48) -> 0x08:gate_priv_xfer, 0 params
    ; Access 0xE4: P=1, DPL=11, S=0, A=0, type=010 (call gate)
    mov word [0x48], gate_priv_xfer
    mov word [0x4A], 0x0008
    mov byte [0x4C], 0x00             ; word count = 0
    mov byte [0x4D], 0xE4
    mov word [0x4E], 0x0000

    ; Entry 10: Call gate DPL=3 (0x50) -> 0x08:gate_with_params, 2 params
    ; Access 0xE4: P=1, DPL=11, S=0, A=0, type=010 (call gate)
    mov word [0x50], gate_with_params
    mov word [0x52], 0x0008
    mov byte [0x54], 0x02             ; word count = 2
    mov byte [0x55], 0xE4
    mov word [0x56], 0x0000

    ; =========================================================
    ; Initialize TSS at 0x2000
    ; SP0/SS0 provide ring 0 stack for ring 3->0 transitions
    ; =========================================================
    mov word [0x2000 + 2],  0x1000    ; SP0
    mov word [0x2000 + 4],  0x0018    ; SS0
    mov word [0x2000 + 42], 0x0000    ; LDTR = null

    ; =========================================================
    ; Load GDT, enter protected mode
    ; =========================================================
    push cs
    pop ds
    lgdt [gdt_ptr]

    mov ax, 0x0001
    lmsw ax
    jmp 0x08:protected_entry

protected_entry:
    mov ax, 0x10
    mov ds, ax
    mov ax, 0x18
    mov ss, ax
    mov sp, 0x1000

    ; Load Task Register with TSS selector
    mov ax, 0x20
    ltr ax

    ; =========================================================
    ; Test 1: Same-privilege call gate (ring 0 -> ring 0)
    ; CALL through gate 0x40 (DPL=0), handler writes marker, RETFs
    ; =========================================================
    mov word [result], 0
    call 0x40:0
    ; Verify handler ran
    mov ax, word [result]
    mov bx, 0xABCD
    int 0x23

    ; =========================================================
    ; Test 2: Drop to ring 3 via RETF with fake outer-return frame
    ; Stack layout (low to high): ret_IP, ret_CS, new_SP, new_SS
    ; =========================================================
    push word 0x003B           ; ring 3 SS (selector 0x38, RPL=3)
    push word 0x3000           ; ring 3 SP
    push word 0x002B           ; ring 3 CS (selector 0x28, RPL=3)
    push word ring3_entry      ; ring 3 IP
    retf

ring3_entry:
    ; =========================================================
    ; Test 3: Verify we're running at ring 3
    ; =========================================================
    mov ax, cs
    and ax, 0x0003
    mov bx, 0x0003
    int 0x23

    ; Load DS with ring 3 data selector for memory access
    mov ax, 0x0033             ; selector 0x30, RPL=3
    mov ds, ax

    ; =========================================================
    ; Test 4: Call gate from ring 3 to ring 0 (no params)
    ; Gate 0x48 (DPL=3) transitions to ring 0 handler
    ; Handler verifies CPL=0, writes result, RETFs back to ring 3
    ; =========================================================
    mov word [result], 0
    call 0x48:0

    ; Back in ring 3 — verify handler wrote the marker
    mov ax, word [result]
    mov bx, 0x1234
    int 0x23

    ; =========================================================
    ; Test 5: Verify we returned to ring 3
    ; =========================================================
    mov ax, cs
    and ax, 0x0003
    mov bx, 0x0003
    int 0x23

    ; =========================================================
    ; Test 6: Verify ring 3 stack was restored correctly
    ; SP should be 0x3000 (what we set before the RETF to ring 3,
    ; minus 0 because the call gate + RETF are balanced)
    ; =========================================================
    mov ax, sp
    mov bx, 0x3000
    int 0x23

    ; =========================================================
    ; Test 7: Call gate with parameter copying (2 words)
    ; Push 2 params on ring 3 stack, call gate 0x50 copies them
    ; to the new ring 0 stack. Handler reads and verifies them.
    ; =========================================================
    push word 0xAAAA           ; param 1 (pushed first, higher address)
    push word 0xBBBB           ; param 2 (pushed second, lower address)
    call 0x50:0                ; call gate with 2 word params
    ; RETF 4 in handler skips the 2 param words on ring 0 stack
    ; but the ring 3 stack still has the pushed params
    add sp, 4                  ; clean up params from ring 3 stack

    ; Verify handler set result
    mov ax, word [result]
    mov bx, 0x5678
    int 0x23

    ; =========================================================
    ; Done
    ; =========================================================
    mov ah, 0x4C
    int 0x21

; =========================================================
; Same-privilege call gate handler (ring 0 -> ring 0)
; Just writes a marker and returns
; =========================================================
gate_same_priv:
    ; Verify we're still at ring 0
    mov ax, cs
    and ax, 0x0003
    mov bx, 0x0000
    int 0x23

    ; Write result marker
    mov word [result], 0xABCD
    retf

; =========================================================
; Privilege transfer handler (ring 3 -> ring 0, no params)
; Verifies CPL=0, stack came from TSS, writes marker, returns
; =========================================================
gate_priv_xfer:
    ; Verify we're at ring 0
    mov ax, cs
    and ax, 0x0003
    mov bx, 0x0000
    int 0x23

    ; Verify SS is the ring 0 stack from TSS (SS0=0x18)
    mov ax, ss
    and ax, 0xFFFC             ; mask off RPL
    mov bx, 0x0018
    int 0x23

    ; Write result marker
    mov word [result], 0x1234
    retf

; =========================================================
; Call gate handler with 2 parameters
; Stack frame on ring 0 stack:
;   [SP]:   return IP
;   [SP+2]: return CS (ring 3, RPL=3)
;   [SP+4]: param[0] (copied from ring 3 stack top = 0xBBBB)
;   [SP+6]: param[1] (copied from ring 3 stack next = 0xAAAA)
;   [SP+8]: old SP (ring 3 SP)
;   [SP+10]: old SS (ring 3 SS)
; =========================================================
gate_with_params:
    push bp
    mov bp, sp
    ; BP+0: saved BP
    ; BP+2: return IP
    ; BP+4: return CS
    ; BP+6: param[0]
    ; BP+8: param[1]

    ; Verify param[0] (top of ring 3 stack at call time = 0xBBBB)
    mov ax, word [bp+6]
    mov bx, 0xBBBB
    int 0x23

    ; Verify param[1] (next on ring 3 stack = 0xAAAA)
    mov ax, word [bp+8]
    mov bx, 0xAAAA
    int 0x23

    ; Write result marker
    mov word [result], 0x5678

    pop bp
    retf 4                     ; skip 2 param words (4 bytes)

section .data

gdt_ptr:
    dw 0x57                     ; limit: 11 entries * 8 - 1 = 0x57
    dw 0x0000                   ; base low
    db 0x00                     ; base high

result: dw 0
