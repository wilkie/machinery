# What other emulators give up, and where our knobs go

The fast PC emulators are fast because each gives up something the
hardware does, on purpose, at a place it chose. Read together they are a
map of where the knobs of a generated emulator belong, and proof that
each one is worth having: someone needed it badly enough to build it.
This note lists them, says which emulators make each one and how, what a
program could see, and how a description could state the boundary so the
generator can build the same core when a profile asks for it.

Two kinds turn up, and the difference matters:

- **Representations**: a different way to hold the same state, exact. Lazy
  flags, a host-pointer TLB in front of the architectural one, a single
  "attention" flag for every rare condition, skipping idle time. The
  generator should derive these, or take a hint that says where they may
  apply; no profile option gives anything up for them.
- **Concessions**: a behaviour changes in a way a program could see.
  Device sync per slice, interrupts recognised at block ends, instruction-
  count time, unchecked segment limits, the TLB's real size, undefined
  results. These are profile options, each named, as `sync: run` and
  `timing: approximate` already are (SPEC.md section 12).

Sources: the cores' code (DOSBox Staging `src/cpu/core_normal.cpp`,
`core_dynrec/decoder.h`, `lazyflags.h`, `src/hardware/pic.cpp`; Bochs
`cpu/cpu.cc`, `cpu/lazy_flags.h`; QEMU `accel/tcg/cpu-exec.c`,
`target/i386/tcg/translate.c` and the TCG design notes), read November
2026, and what is generally known of MAME's scheduler and 86Box. The 386
runs in `../../bench/i386` put numbers on several of them.

## The emulators at a glance

| | DOSBox normal | DOSBox dynrec | Bochs | QEMU TCG | 86Box | MAME | ours, exact | ours, now (`core: jit`) |
|---|---|---|---|---|---|---|---|---|
| Time | 1 unit an instruction | a block's count, at its end | 1 tick an instruction | none, or an instruction count | per-model cycle tables, wait states | exact cycle tables | exact, bus overlap | exact; `timing: approximate` |
| Devices run | in 1 ms slices, cut short by events | the same | every N instructions | own timers, threads | per-instruction-ish, by cycles | per scheduler quantum | each instruction | each instruction; `sync: run` |
| Interrupts seen | at slice ends; an IRQ ends the slice (about 2 instructions) | at block ends | at trace ends, one flag (`async_event`) | at TB starts, one flag (`icount_decr`) | each instruction | each instruction | each instruction | each instruction, the boundary fn inlined |
| Flags | lazy (`var1`, `var2`, `res`, `type`) | lazy, per block | lazy (`result`, `auxbits`) | lazy (`cc_op`, `cc_src`, `cc_dst`) with liveness; conditions from operands | eager | eager | eager | eager, held as pieces |
| Segment limits | barely | barely | checked | data limits not checked | checked | checked | checked | checked (folds when known) |
| Paging cache | a table over every page | the same | 1024 host-pointer entries | direct-mapped host-pointer TLB | host-pointer TLB | as the CPU core does | the 386's 32-entry TLB | the 386's TLB |
| Self-modifying code | coherent, no prefetch queue (a `prefetch` core for demos) | per-page write maps | per-page write stamps | per-page invalidation | coherent | per-CPU | coherent, 128-byte blocks | the same |
| REP strings | many iterations in one go | the same | an iteration loop, checks the flag each | up to `REP_MAX` iterations in a TB | per iteration | per iteration | per iteration | per iteration |
| FPU | host double (80-bit on x86 hosts) | the same | softfloat 80-bit | softfloat 80-bit | 80-bit | softfloat | (387 not yet described) | |

386 runs (real mode, `../../bench/i386/README.md`, MIPS): Bochs 102 and 83,
DOSBox normal 109 and 117, DOSBox dynamic 249 and 304, QEMU 2142 and 953;
ours now about 125 and 78, exact.

## The concessions and representations, one by one

### 1. Device sync granularity (concession)

- **Exact**: every device is caught up to each instruction's end, so an
  edge it raises is seen at the next boundary.
- **Others**: MAME runs each CPU for a *quantum* and then the other devices,
  and lets a driver ask for a smaller one (`set_perfect_quantum`,
  `boost_interleave`) where two parts talk closely. DOSBox runs the CPU in
  1 ms slices and keeps a queue of device events at fractions of a ms; the
  slice is cut at the next event, and raising an IRQ ends it (`CPU_Cycles =
  0`, or 2 cycles after an I/O write: "real hardware executes 0 to ~13
  NOPs ... before the processor picks up the interrupt"). Bochs syncs time
  every N instructions (`BX_SYNC_TIME_IF_SINGLE_PROCESSOR`).
- **Visible**: a device's output (a timer's IRQ, a status bit) can be seen
  late, by up to the quantum.
- **Ours**: `sync: run` (devices once a compiled run returns). Accesses to a
  device already catch it up (`catch_up_*` before a port access), which is
  the MAME pattern and keeps reads of device state exact.
- **Knob**: `sync: step | run | quantum(<time>)`. A quantum is DOSBox's and
  MAME's form: the run check and the device-due compare become one compare
  against the quantum's end, made once a run (or at a back-edge). A board
  part could say it needs tighter sync (`sync: step` on a component), as a
  MAME driver does.

### 2. Where interrupts are recognised (concession), and one attention flag (representation)

- **Exact**: the boundary fn at every instruction: IF, the line, NMI,
  reset, shutdown, halted, the interrupt shadow.
- **Others**: none of the fast ones evaluate that per instruction. Bochs and
  QEMU keep one word that anything wanting attention sets (`async_event`,
  the high half of `icount_decr`) and test only it: per instruction or
  trace in Bochs, per translation block in QEMU (chained blocks run
  without returning; a kick unchains them). DOSBox tests at slice ends and
  makes IRQs end the slice.
- **Visible**: with a single flag tested per instruction, nothing: it is a
  representation, kept by whoever changes one of the conditions (an edge
  into the PIC line, a write to IF, NMI, reset). Tested per block, an
  interrupt is taken up to a block late; a loop is still interrupted at
  its back-edge.
- **Ours**: the boundary fn is inlined at every step: the reset and
  in-reset loads, the shutdown and halted checks, IF and the line. On the
  386's hot loop, about 12 to 15 host instructions an instruction.
- **Hint**: the istream's `boundary` fn already says what it reads. The
  generator can derive an attention byte: the boundary fn's inputs that
  change rarely (lines, NMI, reset, shutdown, halted) folded into one
  byte that their writers keep, leaving IF to the code that writes it
  (STI, POPF, IRET are known instructions). Exact.
- **Knob**: `events: instruction | block`. With `block`, the test is made
  at a run's entry and at its back-edges only, as QEMU does.

### 3. Time (concession)

- **Exact**: each instruction's cycles from the timing table, its bus
  transfers each charged, overlapped with the code fetches.
- **Others**: DOSBox charges one unit an instruction ("cycles=N" is
  instructions a ms, give or take); the dynrec core charges a block's count
  at its end. Bochs counts one tick an instruction (IPS). QEMU keeps no
  time, or an instruction count (`-icount`) where each instruction is a
  fixed time; it can align that to the host's clock. 86Box keeps per-model
  cycle tables and memory wait states, approximately. MAME keeps exact
  tables.
- **Visible**: timing loops, the speed a program measures, anything that
  races a device.
- **Ours**: exact; `timing: approximate` (no overlap: serial fetch).
- **Knob**: `timing: exact | approximate | instructions(<cycles>)`. With
  `instructions`, a run's time is its step count times a constant, added
  once a run or at its back-edge (the dynrec form), and the device-due test
  goes with it. An instruction's own table entry could still be used
  (`timing: table`: the table, no bus) for programs that measure.

### 4. Flags (representation)

- **Others**: all three fast cores keep the arithmetic flags as the last
  operation's inputs and result, and compute a flag only when it is read.
  DOSBox: `var1`, `var2`, `res` and an operation tag (`t_ADDb` ...), with
  `get_CF()`, `get_ZF()` and `FillFlags()` for a whole read. Bochs: the
  result and a word of carry and delta bits (`auxbits`): an operation
  writes two words. QEMU: `cc_op`, `cc_src`, `cc_dst`, with a liveness
  table that drops inputs no later operation needs, and conditions (`jcc`,
  `setcc`) tested from the operands (after SUB, `jl` compares them) rather
  than from flags.
- **Visible**: nothing; reading EFLAGS gives the same bits.
- **Ours**: flags computed at each write (held as pieces of EFLAGS, put in
  the register only when it is needed whole). Computing them only where
  read did not pay in the compiled loops (they are carried across joins and
  back-edges); see the commit "machine-jit: a test of a flag needed only on
  the way out". The fast cores' answer is different: the *machine state*
  holds the lazy record, so an exit or a loop carries the record (two or
  three values), not six flags.
- **Hint**: the description already computes the group in one place:
  `EFLAGS.arith = AluFlags<W>(op, a, b, raw)`. A `lazy` mark on that view
  (or on the unit) lets the generator keep `(W, op, a, b, raw)` in the
  machine as the view's state, and evaluate the unit's field for a read of
  one flag (inlined, with `op` usually known in compiled code, a flag is
  an expression of the operands: ZF is `raw == 0`). Every core gains: the
  interpreter and the trace core stop computing six flags an instruction.
  A condition can be matched to the record (QEMU's `gen_prepare_cc`) by
  inlining `condition(c)` over the unit: what folds, folds.

### 5. Undefined results (concession)

- **Others**: every emulator returns whatever its implementation happens to
  give for the manual's undefined results (AF after a logic operation, OF
  after a shift by more than one, the flags after MUL and DIV); they
  differ from each other and from the chip.
- **Ours**: the description gives what the chip gives (measured from the
  cycle traces), which costs work.
- **Hint**: `undefined` in the description where the manual says so, with
  the chip's value as the default: `AF = undefined(chip_af)`.
- **Knob**: `undefined: chip | free`. With `free` the generator picks the
  cheapest (leave unchanged, or zero), and lazy flags get simpler.

### 6. Checks a program rarely trips (concession)

- **Others**: QEMU adds the segment base and truncates the address; it does
  not check data segment limits (`gen_lea_v_seg_dest`). DOSBox checks
  little in real mode. Bochs and 86Box check.
- **Visible**: a program that relies on #GP or #SS from a limit (rare;
  "unreal mode" relies on cached limits, which unchecked accesses also
  allow).
- **Ours**: checked. In real mode with the segment known it folds to one
  compare; with the segment unknown it is a load and a compare a read.
- **Hint**: name the check where the description makes it, so a profile can
  drop it: a `check segment_limit { ... }` block (its failure is the
  raise), or a mark on `in_segment`. Alignment checks, I/O permission
  bitmaps and the like would be other names.
- **Knob**: `checks: all | -segment_limit | ...`.

### 7. The paging cache (representation, then concession)

- **Others**: QEMU's softmmu TLB is direct-mapped, keyed by the linear
  page, and holds the guest-to-host offset for RAM: a load is a lookup, a
  compare and a host load, and anything else (MMIO, a miss) calls out.
  Bochs keeps 1024 entries with a host page pointer each; DOSBox a table
  over every page. None models the 386's 32-entry, four-way TLB.
- **Visible**: only a program that changes a page table without flushing
  and relies on the stale entry, which the 386's small TLB makes
  unreliable on hardware anyway.
- **Ours**: the 386's TLB, as described (`page`, `walk`, `TLB_NEXT`).
- **Representation first**: a host-pointer cache *in front of* the
  architectural TLB, keyed by the linear page and flushed when the TLB
  is (a `load_cr3`, a mode change), is exact: a hit means the TLB would
  give the same frame. It merges what `memory: direct` does for physical
  pages with paging, and with the segment base when the segment is known.
- **Hint**: mark the TLB register as a `cache` of memory, with its flush
  points (the description's writes to it); then the generator may put a
  larger cache in front of it, or (the concession) replace it.
- **Knob**: `tlb: chip | host(<entries>)`.

### 8. The prefetch queue and self-modifying code (concession, the other way)

- **Others**: all of them make code writes coherent at once: a store to the
  next instruction is seen. The 386 has a 16-byte queue and does not see a
  store to bytes it has fetched already; DOSBox has a separate `prefetch`
  core for the demos that depend on it.
- **Ours**: coherent (runs end at a write to their code, 128-byte blocks).
  Timing models the queue's overlap, not its contents.
- **Knob**: `prefetch: coherent | queue(16)`, the latter an accuracy option
  rather than a speed one, needing the queue in the description.

### 9. Repeated strings (concession, small)

- **Others**: QEMU translates up to `REP_MAX` iterations into one block.
  Bochs runs the iterations in a loop that tests its event flag each time.
  DOSBox runs as many as its slice allows.
- **Visible**: interrupt latency inside a long REP; with a memcpy-style
  fast path, nothing else when no device memory or code is in range.
- **Ours**: one iteration a step (`repeats()` in the timing table).
- **Knob**: `repeat: iteration | bulk(<n>)`: the iterations of a repeated
  instruction run in a host loop, its time charged per iteration, events
  checked every `n`.

### 10. Idle time (representation)

- **Others**: DOSBox leaves the CPU loop on HLT until the next event; QEMU
  sleeps the vCPU until an interrupt.
- **Ours**: a halted step is one clock (`halted_cycle`), and `after_step`
  each.
- **Derived**: when halted (or in a loop the generator can prove waits on a
  device), jump the time to the next device event. Exact.

### 11. Floating point (concession)

- **Others**: DOSBox uses host doubles (64-bit) for the 80-bit registers on
  hosts without x87; QEMU and Bochs use softfloat.
- **Knob** (when the 387 is described): `float: exact | host`.

## What a profile could say

```
profile dos_fast {
    default: model; memory: direct; core: jit;
    sync: quantum(1 ms);          // DOSBox's slice, MAME's quantum
    events: block;                // QEMU's TB-start check
    timing: instructions(1);      // Bochs's IPS, DOSBox's cycles
    checks: -segment_limit;       // QEMU
    undefined: free;
    tlb: host(256);               // QEMU's softmmu TLB
    repeat: bulk(64);             // QEMU's REP_MAX
}
```

and the exact profile stays the default. The representations (lazy flags,
the attention flag, the host-pointer cache in front of the TLB, idle
skipping) need no option: the generator derives them, from a hint where
the description has to say where (`lazy`, `cache`). Each concession is
named in the crate (`system::EXACT` is false with any), and a test that
compares cores knows which it cannot compare.

## Where to start

From the 386 loops' host instructions (`../../bench/i386`), in order of
what each would remove per instruction:

1. **The attention flag** (exact): about 10 of the 12 to 15 host
   instructions the boundary check takes each step.
2. **`events: block`** and **`sync: quantum`**: the rest of the boundary
   check and the device-due compare move to run entries and back-edges.
3. **`timing: instructions`**: the 128-bit time per step becomes one add a
   run.
4. **Lazy flags from a `lazy` hint** (exact): two or three stores where six
   flags are computed now, in every core, and the loops carry the record.
5. **The host-pointer cache in front of the TLB** (exact) and
   **`checks: -segment_limit`**: a memory access becomes a lookup, a
   compare and a load, protected mode included.
6. `repeat: bulk`, idle skipping, `undefined: free`: smaller, or for other
   workloads (a DOS idle loop, string-heavy code).
