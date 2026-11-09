# 386 comparison

The same two DOS programs on our 386 AT board and on other emulators, on
one host, unthrottled: guest instructions per second in steady state.

- `chk.asm`: a 16-bit checksum over 32 KB (memory and ALU), 8000 passes,
  about 918 M instructions.
- `mix.asm`: 32-bit arithmetic through calls, multiply and divide, string
  moves and scans, a sieve; 36000 passes, about 994 M instructions. Its
  data is on pages of its own (writes beside code make tools that watch
  code by page retranslate it).
- `snd.asm`: `chk.asm`'s checksum, laid out as it is, while a Sound
  Blaster (220h, IRQ 5, DMA 1) plays a 4 KB buffer auto-initialised at
  22 kHz in blocks of 2 KB: a byte by DMA every 45 us and an interrupt
  every 92 ms. It counts the interrupts on a page of its own and leaves
  the count in EDI. The other tools are given the card at the same
  settings.

Each marks its start and end at 0000:04F0 and then asks every tool to
exit: port 0F4h (QEMU's `isa-debug-exit`), "Shutdown" to port 8900h
(Bochs), and DOS (DOSBox). `build.sh` makes MS-DOS 3.30 boot disks running
them (from `~/roms`; nothing of DOS is in the repository); `run.py` runs
the tools. Ours is timed between the markers (`bench_compare` in
`targets/i386/at386_test.rs`); the others by marginal time (N passes less
0), which cancels startup and boot. Instruction counts are ours (a REP
iteration counts as one).

Run 2026-11-04 (Ryzen 9 3950X, WSL2; three runs, median):

| tool | chk MIPS | mix MIPS | keeps |
|---|---|---|---|
| ours, trace core | 30 | 31 | exact cycles, devices each instruction |
| ours, JIT | 56 | 45 | the same |
| ours, JIT, `approx` | 67 | 50 | devices each run, approximate time |
| Bochs 2.7 (sync=none) | 102 | 83 | instructions; no cycle timing |
| DOSBox 0.74 normal core | 109 | 117 | approximate cycles |
| DOSBox 0.74 dynamic core | 249 | 304 | approximate cycles |
| QEMU 8.2 TCG | 2142 | 953 | no timing; few checks |

Run 2026-11-07 (the same machine, under another user's load: medians
were unreliable, so ours are the best of five interleaved runs; host
instructions for the run, `perf stat`, are not affected by load):

| tool | chk MIPS | mix MIPS | chk host instructions | keeps |
|---|---|---|---|---|
| ours, JIT | 144 | 84 | 111G | exact |
| ours, JIT, `approx` | 126 | 67 | 126G | `sync: run` (no loops go round), approximate time |
| ours, JIT, `blocks` | 206 | 92 | 76G | devices on time, seen at runs' back-edges; interrupts there; exact time |
| ours, JIT, `quantum` | 200 | 92 | 77G | as `blocks`, the VGA caught up each ms |
| Bochs 2.7 | 95 | 83 | | |
| DOSBox 0.74 normal | 105 | 114 | | |
| DOSBox 0.74 dynamic | 239 | 304 | | |
| QEMU 8.2 TCG | 1942 | 879 | | |

mix spends 40% of its host instructions outside compiled code: its REP
string instructions run through the outlined string fns, an iteration a
call, and its runs return to the dispatcher often.

Then (2026-11-07, `blocks`): REP MOVS, STOS, LODS, SCAS and CMPS run
through kernels (SPEC 11.2, `repeat ... bulk`): mix's run takes 134G host
instructions where it took 156G (14% fewer: 139G with the fills and copies
alone; chk has no REPs). The emulated time to the tick, the guest's memory
(a hash of it) and the instruction count are the same with the kernels and
without.

Then (2026-11-07, `blocks`): runs entered only at their first step, and a
run going round knowing what it knew at its end (its first step built
twice): mix's run 107G host instructions, chk's 56G (from 134G and 76G).
The `jit` profile still ends at the tick and with the memory `emulate`
does, on both.

Then (2026-11-07, `blocks`): values made on the common path but used only
on the way out (a compare's flags, overwritten before anything reads them)
computed there instead (`machine-jit`'s `sink`): mix's run 96G, chk's 55G;
compiling costs a little more (mix's boot 37G, from 35G).

Then (2026-11-07, `blocks`): the time counted in the processor's cycles
in a host register, the board's 128-bit time holding it only before a
call, on the way out or where the code reads it (`machine_jit::Clock`):
mix's run 87G, chk's 39G (from 96G and 55G); calls cost a little more
(mix's boot 39G, from 37G). The `jit` profile still ends exactly where
`emulate` does.

Run 2026-11-07, late (the same machine, load about 8 of 32 threads; three
runs interleaved, median; ours `jit`, `blocks` and `quantum` with the
fix to `quantum`'s check, timed apart, the rest by `run.py`):

| tool | chk MIPS | mix MIPS | keeps |
|---|---|---|---|
| ours, trace core | 31 | 30 | exact |
| ours, JIT | 192 | 102 | exact |
| ours, JIT, `approx` | 133 | 73 | `sync: run`, approximate time |
| ours, JIT, `blocks` | 389 | 161 | devices on time, seen at back-edges; exact time |
| ours, JIT, `quantum` | 407 | 130 | as `blocks`, the VGA each ms (no string kernels) |
| Bochs 2.7 | 90 | 74 | |
| DOSBox 0.74 normal | 112 | 117 | |
| DOSBox 0.74 dynamic | 254 | 301 | |
| QEMU 8.2 TCG | 1860 | 910 | |

Then (2026-11-08, `blocks`): a run follows the instructions it reaches
in its page past taken jumps, calls and returns (each step's pc checked
as before), up to one it already holds: mix's `call step32` loop is one
run going round, where it was three each pass through the dispatcher.
mix's run 71G host instructions (from 87G), its boot 34G (from 39G); chk
the same (39G). 222 MIPS on mix, 400 on chk (best of three, load about
8). The `jit` and `trace` profiles still end where `emulate` does.

Then (2026-11-08, `blocks`): a kernel whose elements all lie in one
direct memory (no recorded code where it writes) and whose costs repeat
with the period (the 386's `split` reads `address & 3`) makes them on the
memory's cells as a block (a fill, a copy, a scan), not element by
element through the bus: mix's run 57G host instructions (from 71G), the
kernels 5% of it (from 24%); the time and memory the same to the tick.
About 270 MIPS on mix (under load).

Then (2026-11-08, `blocks`): a `memo fn` call (the VGA's scanline
render) with no versioned write since it is skipped without looking at
the blocks it read, and one found unchanged counts as made now; its calls
kept in a map with a cheap hash. The VGA's share of mix's run 3% (from
7%): mix's run 54G host instructions, chk's 37G.

Then (2026-11-08, `blocks`): `repeat: bulk` with no count: a repeated
string instruction runs within its step until a device is due or an edge
waits (its kernel only the periods that end before the deadline), where
`bulk(64)` returned to the dispatcher every 64 elements. mix's run 45G
host instructions (from 54G), about 400 MIPS (best of three: 399, 384,
403; chk 417-439). Its emulated time moves with where batches end; its
result (EDI, now printed by `bench_compare`) is the exact profiles'.

Run 2026-11-08 (the same machine, `run.py`, three runs interleaved,
median; load 6 at the start and 26 by the end, the last run of each of
the others slowed by it):

| tool | chk MIPS | mix MIPS | keeps |
|---|---|---|---|
| ours, trace core | 31 | 31 | exact |
| ours, JIT | 212 | 134 | exact |
| ours, JIT, `approx` | 122 | 83 | `sync: run`, approximate time |
| ours, JIT, `blocks` | 414 | 345 | devices on time, seen at back-edges and in REPs; exact time but for REPs' batches |
| ours, JIT, `quantum` | 399 | 173 | as `blocks`, the VGA each ms (no string kernels) |
| Bochs 2.7 | 94 | 74 | |
| DOSBox 0.74 normal | 104 | 113 | |
| DOSBox 0.74 dynamic | 249 | 300 | |
| QEMU 8.2 TCG | 1872 | 925 | |

Then (2026-11-08, `quantum`): `repeat: bulk` there too, a batch ending at
the next catch-up: mix's run 48G host instructions (from 96G), 360 MIPS
(from 189, one run); chk the same (37G, and the same emulated time). mix's
result (EDI) is the exact profiles'.

Then (2026-11-08): a Sound Blaster 2.0's DSP on the AT board
(`targets/pc/sb.machine`), and `snd`. Host instructions for the run, snd
against chk:

| profile | chk | snd | sound costs |
|---|---|---|---|
| `jit` (exact) | 79.8G | 93.5G | 17% |
| `blocks` | 37.4G | 45.5G | 22% |
| `quantum` | 36.7G | 44.3G | 21% |

Every profile plays the same 2.7 M samples, none late (`underruns`), and
takes 1318 interrupts; `emulate` and `jit` agree to the tick. `quantum`
cannot defer the card (it raises signals): each sample is a catch-up, as
under `blocks`. Two fixes on the way: `quantum`'s catch-up was set while
the card was idle, and steps made outside compiled code woke it without
pulling the catch-up in (213 samples late, at 30 MIPS: it is clamped to
the urgent devices' deadline as compiled code is entered); and `snd`'s
interrupt count, kept beside its code at first, made every interrupt
recompile the runs on that page (8000 compiles; the JIT at 17 MIPS).

Then (2026-11-08): a write to recorded code costs the runs in its
128-byte block, not the page's others (SPEC 12, the trace core): a run
whose page was written holds while none of its blocks was. `snd` with its
count beside its interrupt handler, as it was at first (host instructions
for the run):

| profile | before | after | count on a page of its own |
|---|---|---|---|
| `jit` | 594G | 92.8G | 93.5G |
| `blocks` | 151G | 45.4G | 45.5G |

The boot costs more (`jit` 58.0G from 53.5G, `blocks` 32.2G from 31.1G):
runs DOS's writes kept invalidating now live to be compiled (210 compiles
from 205), and a compile costs about 110 M host instructions (27 M a guest
instruction compiled): compiling is 23G of the boot's 53G.

Then (2026-11-08): the compile cost. Each step was about 2700 IR
instructions, 220 blocks and 8.7 KB of host code (`MACHINE_JIT_STATS`
reports each compile): 52% of the time in Cranelift, 32% building, 16%
in `sink`. Half the IR was constants, two thirds of them dead: a branch's
end put every known local in its variable, though the join needed only
those the branches disagree on. And every way out of a run settled the
clock itself, where they now meet and settle it once. 1900 IR
instructions a step, 7.9 KB of code; and the code runs better (`jit`'s
chk run 72.3G host instructions from 79.8G, `blocks`' 35.6G from 37.4G).

A compile still costs about 27 M host instructions a step, and a step
compiled saves under a hundred a time: a run must be entered some
hundreds of thousands of times to pay. Runs are now compiled when entered
4096 times, not 64 (host instructions):

| profile | boot | chk run | mix run |
|---|---|---|---|
| `jit`, 64 | 53.6G | 72.3G | 119.6G |
| `jit`, 4096 | 30.2G | 70.9G | 118.4G |
| `blocks`, 64 | 31.5G | 35.6G | 45.6G |
| `blocks`, 4096 | 21.9G | 35.2G | 45.5G |

The `jit` profile still ends chk, mix and snd at the tick `emulate` does.

Then (2026-11-08): experiments on the compile cost, each measured on the
boot with runs compiled at 64 entries (210 compiles: the compile cost
shows) and on chk and mix whole at 4096 (host instructions; chk's and
mix's totals, boot included: with compiles at 4096 the 0-pass run no
longer cancels them, and N less 0 misleads by a few percent). Every row
ends chk and mix at the same tick.

| `jit` profile | boot at 64 | IR a step | code a step | boot | chk | mix |
|---|---|---|---|---|---|---|
| as committed | 53.7G | 1912 | 7.9 KB | 30.2G | 101.1G | 148.7G |
| Cranelift's single-pass allocator | 57.0G | 1912 | 14.0 KB | 36.0G | | (runs twice as slow) |
| no ticks-left recompute after a call that leaves (`call_leaving`) | 47.8G | 1632 | 7.0 KB | 27.5G | 100.3G | 144.8G |
| and IR constants folded by the emitter | 47.9G | 1632 | 7.0 KB | 27.5G | 100.3G | 144.9G |
| `call_leaving`, Cranelift `opt_level=speed` | 52.1G | 1632 | 6.8 KB | 27.9G | 98.6G | 136.6G |

| `blocks` profile | boot at 64 | IR a step | boot | chk | mix |
|---|---|---|---|---|---|
| as committed | 31.6G | 855 | 21.9G | 57.1G | 67.5G |
| `call_leaving` | 30.7G | 773 | 21.8G | 56.6G | 67.1G |
| `call_leaving`, `opt_level=speed` | 33.1G | 773 | 22.2G | 53.1G | 62.1G |

The single-pass allocator compiles 40% faster and spills: the code is
twice the size and runs at half the speed. The constants a branch
condition is found to be come to the emitter as block parameters, which
Cranelift resolves only once the blocks are sealed: the emitter cannot
fold them (no change; not kept). Cranelift's optimiser compiles a third
slower, and at 4096 entries a run that is compiled runs long enough to
repay it: 6-8% off chk and mix under `blocks`, 2% on the boot. Kept:
`call_leaving` (fault raises and the emitter's returns), and
`opt_level=speed` by default (`MACHINE_JIT_OPT=none` without).

Not tried yet: moving the rare paths (18 a step, 47 IR instructions each
on average: guards, slow memory paths and fault exits, each storing what
the run owes) out of line, into shared code.

Run 2026-11-08, late (the same machine, load about 2 of 32 threads;
`run.py`, three runs interleaved, median; snd's card at 220h, IRQ 5,
DMA 1 on every tool):

| tool | chk MIPS | mix MIPS | snd MIPS | keeps |
|---|---|---|---|---|
| ours, trace core | 31 | 30 | 31 | exact |
| ours, JIT | 261 | 178 | 198 | exact |
| ours, JIT, `approx` | 232 | 109 | 187 | `sync: run`, approximate time |
| ours, JIT, `blocks` | 609 | 455 | 434 | devices on time, seen at back-edges and in REPs; exact time but for REPs' batches |
| ours, JIT, `quantum` | 623 | 548 | 452 | as `blocks`, the devices that raise no signal each ms |
| Bochs 2.7 | 105 | 88 | 108 | |
| DOSBox 0.74 normal | 117 | 121 | 118 | |
| DOSBox 0.74 dynamic | 273 | 327 | 262 | |
| QEMU 8.2 TCG | 2217 | 969 | 2397 | |

Since (2026-11-05, our JIT only, two runs): 113 MIPS on chk and 70 on mix.
The flags wait to be put in EFLAGS until it is needed whole, joins carry
fields in registers past the devices' check, and a run whose end leads
to its start goes round without returning to the dispatcher.

## A game: Xmas Lemmings

Psygnosis' Xmas Lemmings 1992 demo (free; `build_lemm.sh` makes a boot
floppy of it from `~/roms`), run by `play_disk` in
`targets/i386/at386_test.rs`: it boots, picks the keyboard (the demo
offers a joystick otherwise, which waits on a game port the board has
not), starts level 1 and lets the lemmings walk. Keys at instruction
counts (`PLAY_STEPS`; a time is reached at a different instruction by
each core, and a key there lands elsewhere):

    PLAY_DISK=.../lemm.img PLAY_STEPS=292000000 \
    PLAY_KEYS=58336855:3e,62787749:04,69604783:3b,99520663:39,110470370:39 \
    cargo test --release --test at386 -- --ignored --nocapture play_disk

It plays in the EGA's 640x350 16-colour mode, AdLib music only (not
built: silent). Run 2026-11-09 (292 M instructions, 99 emulated seconds):

| profile | speed (emulated/wall) | host instructions | screen, RAM, time |
|---|---|---|---|
| `emulate` | 3.5x | 466G | |
| `jit` | 3.4x | 445G | as `emulate`, to the tick |
| `blocks` | 4.0x | 389G | the same screen |
| `quantum` | 4.1x | 389G | as `blocks` |

The game waits for the vertical retrace each frame: 3 M instructions an
emulated second. The processor is little of the cost: under `blocks` the
VGA's beam and scanline renderer (`Vga::fn_advance`, the scanline memo's
`fn_put_rec` and `Rec::block`) take 72% of the host instructions, about
180 a dot, 70 frames a second; the screen changes every frame, so the memo
seldom spares a line.

