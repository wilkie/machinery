; bound.asm — Thorough tests for BOUND r16, m16&16 (80286) in 16-bit mode
; Harness conventions:
;   int 0x23: assert AX == BX
;   int 0x22: assert AL == AH
; DOS exit: int 21h / AX=4C00h
;
; BOUND semantics (16-bit): signed compare of r16 against [low],[high] at m16&16
;   if (r < low) OR (r > high): #BR → INT 5h (fault), IP points to the BOUND
;   else: no fault, instruction falls through. Flags are unaffected.

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

%macro ASSERT_SP 1
    mov ax, sp
    mov bx, %1
    int 0x23
%endmacro

%macro ASSERT_EQ_FLAGS 0
    mov ax, [flags_store]
    mov bx, [pre_flags]
    int 0x23
%endmacro

; Prepare: reset SP to known value and remember it
%macro PREP 0
    mov sp, stack_top - 0x80
    mov [sp0_store], sp
%endmacro

%macro PREP_ODD 0
    mov sp, stack_top - 0x81
    mov [sp0_store], sp
%endmacro

; Prepare a BOUND test: clear fault mark, set resume IP, snapshot FLAGS
; Usage: PREP_BOUND post_label
%macro PREP_BOUND 1
    mov word [br_seen], 0
    mov ax, %1
    mov [resume_ip], ax
    pushf
    pop  ax
    mov [pre_flags], ax
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

    ; Use a scratch stack with SS=DS (as in your other suites)
    cli
    mov ax, ds
    mov ss, ax
    mov sp, stack_top
    sti

    ; Install INT 5h handler
    ; Get old vector
    mov ax, 0x3505
    int 0x21                ; ES:BX = old handler
    mov [old5_ofs], bx
    mov [old5_seg], es
    ; Set new vector to our isr_br (DS:DX already DS=CS)
    mov dx, isr_br
    mov ax, 0x2505
    int 0x21

; ===================== Tests =====================

; 1) In-range: AX=7 within [0..10] → no fault
t1:
    PREP
    PREP_BOUND t1_post
    mov ax, 7
    bound ax, [b_0_10]
t1_post:
    SAVE_FLAGS
    ; no fault expected
    ASSERT_AX 7                ; AX unchanged
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS

; 2) Equal to low: AX=0 within [0..10] → no fault
t2:
    PREP
    PREP_BOUND t2_post
    mov ax, 0
    bound ax, [b_0_10]
t2_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

; 3) Equal to high: AX=10 within [0..10] → no fault
t3:
    PREP
    PREP_BOUND t3_post
    mov ax, 10
    bound ax, [b_0_10]
t3_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

; 4) Below low: AX=-11 with [-10..-1] → fault
t4:
    PREP
    PREP_BOUND t4_post
    mov ax, -11
    bound ax, [b_m10_m1]
t4_post:
    SAVE_FLAGS
    ; AX must be preserved across the fault
    mov bx, -11
    int 0x23
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ASSERT_EQ_FLAGS
    ASSERT_SP [sp0_store]

; 5) Above high: AX=6 with [-5..5] → fault
t5:
    PREP
    PREP_BOUND t5_post
    mov ax, 6
    bound ax, [b_m5_p5]
t5_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ASSERT_EQ_FLAGS

; 6) Negative in-range: AX=-3 within [-5..5] → no fault
t6:
    PREP
    PREP_BOUND t6_post
    mov ax, -3
    bound ax, [b_m5_p5]
t6_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

; 7) Singleton low==high: [5..5], AX=5 → no fault; AX=4 → fault
t7a:
    PREP
    PREP_BOUND t7a_post
    mov ax, 5
    bound ax, [b_5_5]
t7a_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

t7b:
    PREP
    PREP_BOUND t7b_post
    mov ax, 4
    bound ax, [b_5_5]
t7b_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ASSERT_EQ_FLAGS

; 8) Min-only bound: [-32768..-32768]; test AX=-32768 → no fault; AX=-32767 → fault
t8a:
    PREP
    PREP_BOUND t8a_post
    mov ax, -32768
    bound ax, [b_min_min]
t8a_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

t8b:
    PREP
    PREP_BOUND t8b_post
    mov ax, -32767
    bound ax, [b_min_min]
t8b_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ASSERT_EQ_FLAGS

; 9) Max-only bound: [32767..32767]; test AX=32767 → no fault; AX=-1 → fault
t9a:
    PREP
    PREP_BOUND t9a_post
    mov ax, 32767
    bound ax, [b_max_max]
t9a_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

t9b:
    PREP
    PREP_BOUND t9b_post
    mov ax, -1
    bound ax, [b_max_max]
t9b_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ASSERT_EQ_FLAGS

; 10) Reversed bounds [10..5] — should fault for ANY r
t10:
    PREP
    PREP_BOUND t10_post
    mov ax, 7
    bound ax, [b_rev_10_5]
t10_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ASSERT_EQ_FLAGS

; ---------- Addressing modes coverage ----------
; We'll load base/index regs to point into bounds_blk and use various ModR/M forms.

; 11) [disp16] — blk0 (-20..-10), AX=-15 → in range
t11:
    PREP
    PREP_BOUND t11_post
    mov ax, -15
    bound ax, [bounds_blk]         ; blk0
t11_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

; 12) [bx] — point BX to blk1 (40..50), AX=51 → fault (above)
t12:
    PREP
    PREP_BOUND t12_post
    mov bx, bounds_blk + 4         ; blk1
    mov ax, 51
    bound ax, [bx]
t12_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ASSERT_EQ_FLAGS

; 13) [bp] — point BP to blk2 (0..0), AX=0 → in range
t13:
    PREP
    PREP_BOUND t13_post
    mov bp, bounds_blk + 8         ; blk2
    mov ax, 0
    bound ax, [bp]
t13_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

; 14) [si] — point SI to blk3 (-1..+1), AX=+2 → fault (above)
t14:
    PREP
    PREP_BOUND t14_post
    mov si, bounds_blk + 12        ; blk3
    mov ax, 2
    bound ax, [si]
t14_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ASSERT_EQ_FLAGS

; 15) [di] — point DI to blk3 (-1..+1), AX=-1 → in range
t15:
    PREP
    PREP_BOUND t15_post
    mov di, bounds_blk + 12
    mov ax, -1
    bound ax, [di]
t15_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

; 16) [bx+si] — BX=bounds_blk, SI=16 → blk4 (100..200), AX=150 → in range
t16:
    PREP
    PREP_BOUND t16_post
    mov bx, bounds_blk
    mov si, 16
    mov ax, 150
    bound ax, [bx+si]
t16_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

; 17) [bp+di] — BP=bounds_blk, DI=20 → blk5 (-7..-7), AX=-7 → in range
t17:
    PREP
    PREP_BOUND t17_post
    mov bp, bounds_blk
    mov di, 20
    mov ax, -7
    bound ax, [bp+di]
t17_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

; 18) [bx+disp8] — BX=bounds_blk, +12 → blk3 (-1..+1), AX=+1 → in range
t18:
    PREP
    PREP_BOUND t18_post
    mov bx, bounds_blk
    mov ax, 1
    bound ax, [bx+12]
t18_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

; 19) [bp+disp8] — BP=bounds_blk, +4 → blk1 (40..50), AX=39 → fault (below)
t19:
    PREP
    PREP_BOUND t19_post
    mov bp, bounds_blk
    mov ax, 39
    bound ax, [bp+4]
t19_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ASSERT_EQ_FLAGS

; 20) [bx+si+disp8] — BX=bounds_blk, SI=8, +4 → blk3 (-1..+1), AX=0 → in range
t20:
    PREP
    PREP_BOUND t20_post
    mov bx, bounds_blk
    mov si, 8
    mov ax, 0
    bound ax, [bx+si+4]
t20_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

; 21) Destination register variety: use CX with [0..10] in range
t21:
    PREP
    PREP_BOUND t21_post
    mov cx, 9
    bound cx, [b_0_10]
t21_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ; Verify CX preserved
    mov ax, cx
    mov bx, 9
    int 0x23
    ASSERT_EQ_FLAGS

; 22) Destination = DX, out-of-range below → fault, DX preserved
t22:
    PREP
    PREP_BOUND t22_post
    mov dx, -21
    bound dx, [blk0]               ; [-20..-10], -21 below
t22_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    mov ax, dx
    mov bx, -21
    int 0x23
    ASSERT_EQ_FLAGS

; 23) Destination = SI, above high → fault, SI preserved
t23:
    PREP
    PREP_BOUND t23_post
    mov si, 201
    bound si, [blk4]               ; [100..200]
t23_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    mov ax, si
    mov bx, 201
    int 0x23
    ASSERT_EQ_FLAGS

; 24) Destination = DI, in range (singleton) → no fault
t24:
    PREP
    PREP_BOUND t24_post
    mov di, -7
    bound di, [blk5]               ; [-7..-7]
t24_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    mov ax, di
    mov bx, -7
    int 0x23
    ASSERT_EQ_FLAGS

; 25) Destination = BP (uses SS), in range → no fault
; Note: SS=DS (scratch stack), so [bp]-based addressing above worked already.
t25:
    PREP
    PREP_BOUND t25_post
    mov bp, 45
    bound bp, [blk1]               ; [40..50]
t25_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    mov ax, bp
    mov bx, 45
    int 0x23
    ASSERT_EQ_FLAGS

; 26) Destination = SP — safe check (read-only), use in-range then out-of-range
; Keep SP stable (PREP set it), choose bounds around that value.
; Build a dynamic bounds pair on the fly.
t26a:
    PREP
    ; create [SP-2 .. SP+2] bounds in temp pair
    mov ax, sp
    mov [dyn_bounds_low], ax
    add ax, 2
    mov [dyn_bounds_high], ax
    PREP_BOUND t26a_post
    ; SP itself is within [SP..SP+2] → no fault
    bound sp, [dyn_bounds_low]
t26a_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS

t26b:
    PREP
    ; bounds [SP+1 .. SP+2], so SP (< low) → fault
    mov ax, sp
    inc ax
    mov [dyn_bounds_low], ax
    inc ax
    mov [dyn_bounds_high], ax
    PREP_BOUND t26b_post
    bound sp, [dyn_bounds_low]
t26b_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ASSERT_SP [sp0_store]
    ASSERT_EQ_FLAGS

; 27) Chain sanity: first fault (above), then no fault (equal)
t27:
    PREP
    PREP_BOUND t27_mid
    mov ax, 201
    bound ax, [blk4]               ; [100..200] → fault
t27_mid:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ; Now equal to high (200) → no fault
    PREP_BOUND t27_post
    mov ax, 200
    bound ax, [blk4]
t27_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 0
    int 0x23
    ASSERT_EQ_FLAGS

; 28) Verify handler captured fault frame IP/CS for a known site
; Cause a fault at label t28_faultsite and confirm fault_ip equals that offset.
t28:
    PREP
    PREP_BOUND t28_post
    mov ax, 11
t28_faultsite:
    bound ax, [b_0_10]             ; 11 > 10 → fault
t28_post:
    SAVE_FLAGS
    mov ax, [br_seen]
    mov bx, 1
    int 0x23
    ; fault_ip should equal offset of t28_faultsite
    mov ax, [fault_ip]
    mov bx, t28_faultsite
    int 0x23
    ; CS should be our CS
    mov ax, [fault_cs]
    mov bx, cs
    int 0x23
    ; FLAGS captured is just informational; no assertion needed here.

; ===================== Cleanup & Exit =====================
exit:
    ; Restore original INT 5 vector
    push ds
    mov ax, [old5_seg]
    mov ds, ax
    mov dx, [old5_ofs]
    mov ax, 0x2505
    int 0x21
    pop ds

    mov ax, 0x4C00
    int 0x21

; ---------- INT 5h handler (Bounds Range Exceeded) ----------
; On entry (real mode fault): stack top is IP, then CS, then FLAGS.
; We:
;   - mark br_seen=1
;   - record fault_ip/cs/flags
;   - write saved IP := [resume_ip] so execution continues after the fault site
;   - IRET (flags unchanged)
isr_br:
    push ax
    mov word [br_seen], 1
    ; capture fault frame (IP,CS,FLAGS)
    mov bp, sp
    mov ax, [ss:bp+2]         ; IP
    mov [fault_ip], ax
    mov ax, [ss:bp+4]         ; CS
    mov [fault_cs], ax
    mov ax, [ss:bp+6]         ; FLAGS
    mov [fault_flags], ax
    ; redirect return IP to caller-provided resume point
    mov ax, [resume_ip]
    mov [ss:bp+2], ax
    pop ax
    iret

; ---------- Scratch / dynamic storage ----------
dyn_bounds_low:   dw 0
dyn_bounds_high:  dw 0

; ---------- Data ----------
; Simple bound pairs (dw low, dw high). All comparisons are SIGNED.
b_0_10:        dw 0, 10
b_5_5:         dw 5, 5
b_m10_m1:      dw -10, -1
b_m5_p5:       dw -5, 5
b_min_min:     dw -32768, -32768
b_max_max:     dw  32767,  32767
b_rev_10_5:    dw 10, 5           ; reversed bounds (always fault for any r)

; A small block to exercise addressing modes via displacements and index regs
bounds_blk:
blk0:  dw -20, -10    ; offset +0
blk1:  dw  40,  50    ; +4
blk2:  dw   0,  0     ; +8
blk3:  dw  -1,  1     ; +12
blk4:  dw 100, 200    ; +16
blk5:  dw -7,  -7     ; +20

; Variables used by the #BR handler and tests
br_seen:       dw 0              ; 0=no fault, 1=fault
resume_ip:     dw 0              ; IP to continue at after a fault
fault_ip:      dw 0              ; captured original IP from the fault frame
fault_cs:      dw 0
fault_flags:   dw 0

flags_store:   dw 0
pre_flags:     dw 0
sp0_store:     dw 0

; INT 5 old vector (for clean restore)
old5_ofs:      dw 0
old5_seg:      dw 0

; ---------- Stack buffer ----------
stack_buf:   times 2048 db 0xCC
stack_top    equ stack_buf + 2048

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
