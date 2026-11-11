# Performance chronicle

Every approach tried to make the emulators faster, kept or not, with what
it was measured on, what it did, and where its code is. An approach that
did not pay is recorded as carefully as one that did: what was learned,
and a patch that applies again (`experiments/`), so it can be tried again
when what made it fail changes.

## How we measure

- **Host instructions, not wall time.** `perf stat -e instructions:u`
  counts are stable to about 1%; wall time moves ±15% on this host (WSL2,
  shared). `perfbin.py` finds a perf that runs.
- **Interpreter and compiler micro-benchmarks** (`i386/chk.asm`,
  `mix.asm`, `snd.asm`): `bench_compare` in `targets/i386/at386_test.rs`
  counts between the program's markers; host instructions are N passes
  less 0 (startup and boot cancel), over the guest instructions.
- **Games** (`i386/compare_games.py`): host instructions an emulated
  second over a window where the game does the same thing on every tool
  (screenshots checked at the window's ends), drawing every frame and none,
  medians of 3; the same windows run in DOSBox at matched cycles.
- **Where it goes** (`i386/profile_game.py`): a game's window by category
  (compiled code, the JIT's glue, the interpreter, buses, devices, the
  VGA, sound, compiling), per guest instruction.
- **Exactness** before any number: the exact profiles (`emulate`, `trace`,
  `jit`) end alike (frame, memory hash, time) at a fixed instruction count
  (`PLAY_STEPS`); the concession profiles (`blocks`, `quantum`) are not
  exact, and are compared with themselves only for speed.
- **Host:** Ryzen 9 3950X, WSL2, Ubuntu; load noted where it mattered.

## Recording an approach

Add a row to the index, and for anything from 2026-11-10 on an entry
below: the idea, the change, the measurements (before and after, on what,
how), exactness, the outcome (kept, not kept, superseded), and the code: a
commit when kept; when not, `experiments/<date>-<name>.patch`, made with
`git diff` from the repository's root and applying (`git apply`) on the
commit named in its entry. Older approaches' details are in
`i386/README.md`, under the date given.

## Standing figures

Micro-benchmarks (guest MIPS, `run.py`, medians; `trace` host instructions
a guest instruction on chk where measured):

| date | chk `trace` | chk `jit` | mix `jit` | chk `blocks` | mix `blocks` | notes |
|---|---|---|---|---|---|---|
| 2026-11-04 | 30 MIPS | 56 | 45 | | | first comparison |
| 2026-11-07 | | 144 | 84 | 206 | 92 | concessions profiles |
| 2026-11-07, late | 31 | 192 | 102 | 389 | 161 | |
| 2026-11-08 | 31 | 212 | 134 | 414 | 345 | string kernels |
| 2026-11-08, late | 31 | 261 | 178 | 609 | 455 | compile cost; DOSBox dynamic 273 / 327 |
| 2026-11-09 | 412.5 → 377.8 a guest instr | | | | | replay loop |
| 2026-11-10 | 391.5 a guest instr | | | | | A20 gate in direct memory (exactness) |

Games (host instructions an emulated second, `blocks`, drawing none;
`compare_games.py` or the same windows singly):

| date | commit | Lemmings | Jetpack | One Must Fall | notes |
|---|---|---|---|---|---|
| 2026-11-09 | b8b8d64 | 0.80 G | 1.23 G | 0.77 G | first table (DOSBox dynamic, whole: 0.39 / 0.35 / 0.33 G) |
| 2026-11-10 | c92b448 | 0.81 G | 1.21 G | 0.79 G | DOS=HIGH disks, A20 gate |
| 2026-11-10 | ed27d7f | 0.80 G | 1.18 G | 0.74 G | the harness's stop test |
| 2026-11-10 | 857c893 | 0.77 G | 1.17 G | 0.69 G | the VGA's beam, runs back before their start |

## Index

| date | area | approach | measured effect | outcome | record |
|---|---|---|---|---|---|
| 2026-11-04 | JIT | speculation on mode registers, branch joins, wider direct memory | (with the next rows) chk `jit` 56 → 113 MIPS by 11-05 | kept | 61cce3f |
| 2026-11-04 | JIT | trial inlining of large fns under a budget | a folded real-mode read still ~380 host instructions | kept, off by default (budget 0) | 61cce3f |
| 2026-11-04–06 | JIT | no stores at the gate; joins carry fields; runs go round; outlined calls keep what they leave; one inline access for a memory's ranges; reads inlined; branches learn what they test | chk `jit` → 113 MIPS | kept | 6572606 … 1eed22d |
| 2026-11-06 | profiles | concessions: `events: block`, `sync: block`, `sync: quantum` | chk 144 (`jit`) → 206 (`blocks`) MIPS | kept | 1bef730, ec5de33 |
| 2026-11-07 | JIT | `repeat: bulk`: REP string instructions through kernels | mix `blocks` 156 → 134 G | kept | 394d1e5 … b332e63 |
| 2026-11-07 | JIT | runs entered at their first step only; peeled going round | mix 134 → 107 G, chk 76 → 56 G | kept | d598c31 |
| 2026-11-07 | machine-jit | `sink`: values for the way out made there | mix 107 → 96 G, chk 56 → 55 G | kept | fdad7f7 |
| 2026-11-07 | machine-jit | time in the processor's cycles in a register | mix 96 → 87 G, chk 55 → 39 G | kept | 0158e40 |
| 2026-11-08 | trace core | runs follow taken jumps, calls, returns in their page | mix 87 → 71 G | kept | 69816a5 |
| 2026-11-08 | JIT | kernels on a direct memory as a block | mix 71 → 57 G | kept | bc303e5 |
| 2026-11-08 | memo | a `memo fn` call with no write since skipped unlooked | mix 57 → 54 G, chk 39 → 37 G | kept | 49643be |
| 2026-11-08 | JIT | `repeat: bulk` without a count, batches to the next device | mix 54 → 45 G | kept | 91c326c |
| 2026-11-08 | `quantum` | `repeat: bulk` under `quantum` | mix 96 → 48 G | kept | 29ea974 |
| 2026-11-08 | trace core | a write to code costs its 128-byte block's runs only | snd (count beside its handler) `jit` 594 → 92.8 G | kept | 0bd1394 |
| 2026-11-08 | compile | less IR a step (joins define only what differs; one clock settle); compile at 4096 entries | 2700 → 1900 IR a step; chk `jit` 79.8 → 72.3 G; boot 53.6 → 30.2 G | kept | ab215eb |
| 2026-11-08 | compile | a lazy budget recompute | chk regression | not kept (no patch); replaced by `call_leaving` | README 11-08 |
| 2026-11-08 | compile | `call_leaving`: no budget recompute before an exit | boot 53.7 → 47.8 G at 64 entries | kept | 6f3f1d1 |
| 2026-11-08 | compile | Cranelift's single-pass register allocator | compiles 40% faster, code 2x slower | not kept (no patch) | README 11-08 |
| 2026-11-08 | compile | IR constants folded by the emitter | no change (Cranelift resolves them only once sealed) | not kept (no patch) | README 11-08 |
| 2026-11-08 | compile | Cranelift `opt_level=speed` | `blocks` chk, mix −6–8% | kept (then at the second tier) | 6f3f1d1 |
| 2026-11-08 | JIT | a guarded-shift lowering | no gain | not kept (no patch) | |
| 2026-11-08 | interpreter | dropping the step's `last` store | −13 host instructions a guest instruction | measured only, not made | |
| 2026-11-09 | VGA | renderer: a line's 16 colours once, 8-dot loop, memo ranges by 64 KB | Lemmings 382.7 → 201.8 G | kept | 0b8eb05 |
| 2026-11-09 | VGA | drawing is output alone; `draw_every` | Lemmings 189 G every frame, 106 G none | kept (a host knob) | 9d1d18e |
| 2026-11-09 | JIT | tiers: quick at 16384 entries, optimised past 1 M steps a step | Lemmings undrawn 106.3 → 94.6 G | kept | c473c49 |
| 2026-11-09 | trace core | 16384 run slots (from 4096) | Lemmings 94.6 → 91.9 G | kept | 2e31bc7 |
| 2026-11-09 | trace core | the replay loop | chk `trace` 412.5 → 377.8 a guest instr; Lemmings ~1% | kept | 6854627 |
| 2026-11-10 | exactness | the A20 gate in direct memory (an address hook, keyed runs) | chk `trace` 377.9 → 391.5 (first version 393.2); compiled code unchanged | kept (correctness) | c92b448 |
| 2026-11-10 | harness | `play_disk` stops on integer time | Jetpack's window −2.5% (a measuring cost) | kept | ed27d7f |
| 2026-11-10 | VGA | the beam's line end kept: a move within a line an add | Lemmings −3.7% | kept | b95fa93 |
| 2026-11-10 | trace core | runs go on back before their start, in their page | One Must Fall −6.6%, Lemmings −0.6%, Jetpack 0 | kept | 857c893 |
| 2026-11-10 | JIT | the next compiled run entered from `jit_burst` | Jetpack +3% | not kept | experiments/2026-11-10-chain-in-jit-burst.patch |
| 2026-11-10 | trace core | four links a run, the last found first | Jetpack +3% | not kept | experiments/2026-11-10-four-links.patch |
| 2026-11-10 | JIT | compiled runs chained in compiled code | 8% of hops chain; Jetpack +5%, One Must Fall +2.4% | not kept, to retry | experiments/2026-11-10-chain-in-compiled-code.patch |
| 2026-11-10 | trace core | links checked by value (what maps the pc to its address), not an address epoch | Jetpack −2.9%, One Must Fall +1.2%, Lemmings +0.3% | not kept (mixed) | experiments/2026-11-10-value-checked-links.patch |
| 2026-11-10 | JIT | chaining in compiled code, with value-checked links | 40% of hops chain; Jetpack +4%, One Must Fall +3.4%, Lemmings +1% (against value links alone) | not kept | experiments/2026-11-10-chain-with-value-links.patch |
| 2026-11-10 | devices | a catch-up skips a clock whose tick last said nothing pending (the VGA's idle dot clock) | Lemmings 0.768 → 0.772 G (none) | not kept (sized in a generated crate) | entry below |
| 2026-11-10 | devices | the board's sync gate inline, the catch-ups and deadline rescan out of line | Jetpack −1.4%, One Must Fall −0.9%, Lemmings 0 | kept | (this entry's commit) |
| 2026-11-10 | JIT | the direct memories' pointers fetched once, not at each entry | Jetpack, One Must Fall −0.1% | not kept (sized) | entry below |
| 2026-11-10 | JIT | a cache of lookups by pc | goes stale on the epochs links do; value-checked links measured the equivalent | not tried | entry below |
| 2026-11-10 | devices | the PIT's counter read as arithmetic on the time (as DOSBox does) | already so: its tick is constant-time; a read costs the catch-up's machinery | nothing to change | entry below |
| 2026-11-10 | devices | lazy catch-up: a transaction skips its responder's catch-up when it can see nothing the ticks do | Lemmings −4.3% undrawn, −1.6% drawn; Jetpack +0.3%, One Must Fall 0 | kept | (this entry's commit) |

## Entries

### 2026-11-10: where the games' windows go

`profile_game.py`, `blocks`, drawing none, host instructions a guest
instruction:

| | Lemmings | Jetpack | One Must Fall |
|---|---|---|---|
| total (DOSBox dynamic, whole: 63 / 59 / 36) | 306 | 348 | 139 |
| compiled code | 62 | 85 | 73 |
| the JIT's dispatcher and interpreted steps | 41 | 128 | 30 |
| the video memory path (buses, the VGA caught up) | 170 | 71 | 6 |
| devices, sound | 27 | 47 | 11 |
| compiling | 5 | 6 | 9 |

Jetpack's hot loop calls a routine that latches and reads the PIT, with
far calls and returns: runs of 2 to 5 instructions, 56 M entries in 120 s,
about 450 host instructions a hop. The entries below follow from this.

### 2026-11-10: the VGA's beam move within a line (kept, b95fa93)

- Idea: `advance` divided (`pos / line_dots`) at each catch-up, and a
  video memory access catches the VGA up.
- Change: the line's end and the frame's dots kept from the last line
  crossed; a write to the card's ports forgets them.
- Measured: Lemmings' window, `blocks`: 0.804 → 0.774 G drawing none,
  2.143 → 2.114 G drawing every frame.
- Exact: Lemmings' frame, memory and time alike at 240 M instructions.
- Learned: the division was a small part; the catch-up's own work (two
  dot clocks' divisions and calls) and its frequency remain (see the open
  leads: lazy catch-up).

### 2026-11-10: runs back before their start (kept, 857c893)

- Idea: a run ended at an instruction before its start, so loops whose
  head lay before the entry, and calls to routines earlier in the page,
  made runs of a few instructions.
- Change: a step's offset from the run's start is an `i16`; a run ends
  only at an instruction it already holds, within its page.
- Measured (windows, `blocks`, drawing none): One Must Fall 0.743 →
  0.694 G, Lemmings 0.774 → 0.769 G, Jetpack 1.164 → 1.165 G.
- Exact: `jit` and `trace` alike at 300 M instructions of Jetpack, and as
  the crate before.
- Learned: Jetpack's runs grew (13 instructions where 4) but are left
  where its branches and returns go otherwise than recorded: its entries
  did not fall (56.6 M).

### 2026-11-10: the next run entered from `jit_burst` (not kept)

- Idea: return less often to `run_until` between compiled runs.
- Change: after a compiled run, `jit_burst` follows the link (or looks up
  by address) and enters the next compiled run at once, with the checks
  the next burst makes; budget kept.
- Measured: Jetpack's window 1.164 → 1.202 G (+3%). Exact (Jetpack at
  300 M, the crate before).
- Learned: the hop's cost is not the return to the caller: it is
  following the link, looking up by address when it fails, and entering
  the run.
- Code: `experiments/2026-11-10-chain-in-jit-burst.patch`, on 857c893.

### 2026-11-10: four links a run (not kept)

- Idea: a routine returning to several callers churns two links into
  lookups by address.
- Change: `TLINKS = 4`, a new link put first (in place of the one at its
  pc, else a stale one, else the last).
- Measured: Jetpack 1.165 → 1.200 G (+3%); One Must Fall 0.695 G,
  Lemmings 0.769 G (unchanged).
- Learned: the failures were not for want of links (see the next entry:
  the address epoch).
- Code: `experiments/2026-11-10-four-links.patch`, on 857c893.

### 2026-11-10: compiled runs chained in compiled code (not kept, to retry)

- Idea: a run's end checks its link itself and hands the next compiled
  run's code to a small loop in `jit_enter`, without `jit_burst`,
  `jit_follow` or `jit_lookup`.
- Change: a table of compiled runs by slot (`JitTab`: code, address, pc,
  page, stamp, blocks, length), kept as runs are compiled, replaced,
  cleared and refreshed; at a run's last step, the two links checked
  inline (pc, address epoch, key epoch), the target's entry checked (code,
  address, pc, same page and count, blocks), nothing written to code
  since the last check, the steps allowed, running, not in reset; then
  `jit_next`, and the trace core's state as `jit_follow` sets it.
  `jit_enter` loops while a run hands on, its per-run bookkeeping kept.
- Exact: `jit` and `trace` alike at 300 M of Jetpack; the AT tests pass.
- Measured (`blocks`, drawing none, the same binary with
  `MACHINE_JIT_NOCHAIN`): Jetpack 1.175 → 1.237 G (+5%), One Must Fall
  0.697 → 0.714 G (+2.4%), Lemmings 0.769 → 0.776 G (+1%).
- Why: 8% of hops chained (3.0 M of 35.2 M calls). Of those that did not,
  26.5 M failed on the link's address epoch: every load of CS bumps it,
  and Jetpack makes a far call and a far return each iteration, so links
  are stale even when CS comes back to what it was (which is also why
  the Rust path looks up by address so often).
- Retry when: links are checked by value (the address state they were
  made under, as the run key is compared), not by an epoch.
- Code: `experiments/2026-11-10-chain-in-compiled-code.patch`, on
  857c893.

### 2026-11-10: links checked by value (not kept: mixed)

- Idea: a link is stale when the address epoch moved, and every far call
  or return (`CS_REAL`) and `POPF` (`EFLAGS.VM`) moves it, though the pc
  maps where it did. Compare what maps the pc instead.
- Change: what the istream's address, window and limit fns read besides
  the key and the pc is split: scalar registers, fields and the mode
  (`TAddr`; on the 386 `CR0.PE`, `CR0.PG`, `EFLAGS.VM`, `CS_REAL`, `CR3`)
  are snapshotted in each link and compared where it is followed; their
  writes still bump the epoch (a run stops) but not the address epoch;
  arrays (the TLB) still bump the address epoch.
- Exact: `jit` and `trace` alike at 300 M of Jetpack, and as before.
- Measured (`blocks`, drawing none, repeated: identical): Jetpack 1.165 →
  1.131 G (−2.9%), One Must Fall 0.694 → 0.703 G (+1.2%), Lemmings 0.768
  → 0.770 G (+0.3%).
- Learned: far-call-heavy real-mode code gains a little (fewer lookups by
  address); flat protected-mode code pays the snapshot's compare on each
  follow for little. Not worth its machinery alone.
- Code: `experiments/2026-11-10-value-checked-links.patch`, on d61f355.

### 2026-11-10: chaining with value-checked links (not kept)

- Idea: with links that survive far transfers, the chaining patch should
  fire on most hops.
- Change: both patches, and compiled code compares the link's snapshot
  (`jit_av_eq`, generated from the same cells: loads, shifts, masks).
- Exact: `jit` and `trace` alike at 300 M of Jetpack.
- Measured (`blocks`, drawing none, chaining against
  `MACHINE_JIT_NOCHAIN` in the same binary): Jetpack 1.139 → 1.187 G
  (+4%), One Must Fall 0.706 → 0.730 G (+3.4%), Lemmings 0.770 → 0.778 G
  (+1%). 40% of hops chained (14.1 M of 35.2 M; from 8%); the rest failed
  on the key epoch (CS, in the key, 7.4 M: compiled code does not compare
  the key by value), on blocks outside the run's checked mask (5.7 M), or
  ended where no chain code was emitted (6.9 M).
- Learned: a chained hop is not much cheaper than the Rust path's: what
  is left of a hop is the run's state stored and loaded across separate
  functions and the per-run bookkeeping kept in `jit_enter`'s loop (the
  `blocks` profile's device catch-up among it), and the checks emitted at
  every run's end (two links, their snapshots, the table) are paid
  whether it chains or not. Chaining pays only if runs share state across
  the hop (one function for runs that follow each other), not by moving
  the dispatcher's checks into compiled code.
- Code: `experiments/2026-11-10-chain-with-value-links.patch`, on d61f355.

### 2026-11-10: from how DOSBox does it: cheap leads (a)-(e)

DOSBox (0.74-3's source): linked blocks of host code (`CacheBlock::LinkTo`),
time as an instruction count with devices computed when asked (the PIT's
counter from the elapsed time, `timer.cpp`), memory one hop to a page
handler, the VGA's planes expanded to a chunky copy as they are written
(`fastmem`) and drawn in batches of lines, lazy flags. The cheap leads it
suggested, each measured (`blocks`, drawing none):

- (a) A catch-up that skips a clock whose last tick returned
  `0xFFFFFFFF` (sized in a generated crate, phase not kept, not exact):
  Lemmings 0.768 → 0.772 G, drawing 2.114 → 2.111 G. An idle clock's
  catch-up is a few instructions (a division is one); the VGA's 23% is
  its active clock, and how often it is caught up. A first try, the idle
  block deleted, left its deadline 0 and caught the VGA up at every sync
  (0.97 G): measure the deadline too.
- (b) `board_sync`'s gate (nothing due, no edge) inline at every step,
  the catch-ups and the rescan of the deadlines out of line: Jetpack
  1.165 → 1.149 G, One Must Fall 0.694 → 0.688 G, Lemmings 0.768 G.
  Code motion: Jetpack's `blocks` run alike at 300 M. Kept. The rescan
  (a minimum over 11 deadlines, `cmovb`s) is about a third of what is
  left of `after_step`, run after any device was touched.
- (c) `jit_enter` fetched the direct memories' pointers through the bus
  at each entry (55 M on Jetpack's window); fetched once (sized): −0.1%.
  Not kept.
- (d) A cache of `jit_lookup` by pc: not tried; it would go stale on the
  same epochs links do, and value-checked links measured that fix.
- (e) The PIT's counter as arithmetic on the time: our PIT's tick is
  constant-time already (a channel's count from `n`, the refresh
  channel's OUT not wired); a read costs the catch-up's machinery, not
  the counting.

### 2026-11-10: lazy catch-up (kept)

- Idea: a transaction catches its responder up first so that its handler
  sees the device as of now. When the handler can see nothing the ticks
  do, the catch-up only costs.
- Change: the generator works out, for each transaction a device
  answers and each space its ranges cover, the handler's reads and
  writes (states, mems, output ports, signals, through the model fns it
  calls; an `if` settled by the known `space` walked only where it goes,
  and nothing after a branch that returns), and its ticks'. When the
  ticks reach no bus, and the handler reads nothing they write and writes
  nothing they read, those ranges call the handler without the catch-up
  and without making the device due. On the 386 AT: the VGA's memory
  reads, the Sound Blaster's port reads, the fixed disk's `cs16`.
- Exact: `trace` and `jit` as before (frame, memory, time) at 240 M of
  Jetpack and of Lemmings.
- Measured (`blocks`, against `MACHINE_GEN_NOLAZY` from the same tree):
  Lemmings 0.768 → 0.735 G undrawn, 2.107 → 2.074 G drawn; Jetpack 1.149
  → 1.153 G; One Must Fall 0.688 G both.
- Learned: the first analysis walked on past a branch that returned, and
  counted the VGA's I/O path (`pos`, `blink`) against its memory reads.
  The VGA's memory writes stay caught up: the scanlines drawn after read
  the planes they write, which needs the drawing's own deadline (the next
  line's end) to be deferred exactly; that, and a chunky copy of the
  planes, are the VGA leads below.

## Open leads

- **Runs that follow each other compiled into one function** (a
  superblock or trace tree), so a hop keeps its state in registers: what
  chaining would need to pay (above).
- **Lazy device catch-up:** skip a device's catch-up before a transaction
  whose handler reads none of what its tick writes, while the tick reports
  nothing due before then (the VGA's memory accesses, the PIT's reads).
  Lemmings' VGA catch-up is 23% of its window, undrawn.
- **A shorter path to video memory** in the emulator profiles (the
  processor model, the AT bus and the ISA bus layers: 25% of Lemmings).
- **Compiled 32-bit code** (One Must Fall's compiled code, 73 host
  instructions a guest instruction against DOSBox's 36 for all of it).
- **Rare paths out of line** (README 2026-11-08: 18 a step, 47 IR
  instructions each).
- **The interpreter's A20 cost** (+3.6% on chk): record a run's gated
  address with it, keeping `trace` and `decode` timing fetches alike.
