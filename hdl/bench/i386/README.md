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

Then (2026-11-09, `blocks`, the script above; the same screen, memory and
tick throughout; host instructions):

| change | host instructions | speed |
|---|---|---|
| before | 382.7G | 4.2x |
| a line's 16 colours worked out once (palette, pel mask, DAC), each dot a lookup | 253.6G | 5.9x |
| 16 colours at 8 dots a character clock, shown once or twice, in a loop of fixed bit order | 228.0G | 5.9x |
| the memo's recorder: a last range for each 64 KB of a memory (the four planes), not one a memory | 201.8G | 7.0x |

Then (2026-11-09): drawing is output alone. The DAC's comparator (Input
Status 0 bit 4) works out the dot under the beam from the memory and
registers when it is read (`dot_index`, `render`'s rules a dot at a
time; real hardware compares the dot being scanned, not the last frame's),
so no per-dot record (`dots`) is kept, and the host may have the VGA draw
one frame in `draw_every` (0: none) with the machine running the same:

| `draw_every` | host instructions | speed | memory, time |
|---|---|---|---|
| 1 (every frame) | 189.0G | 7.4x | as before |
| 4 | 129.9G | 9.4x | the same |
| 0 (none) | 106.3G | 10.1x | the same |

Then (2026-11-09): with the screen undrawn, compiling was a third of the
game's host instructions: its frame loop runs 70 times a second, so its
code crossed 4096 entries within a minute and was compiled, to run too
few times to pay. Runs are now compiled at 16384 entries, quickly
(Cranelift's optimiser off), and compiled again optimised once their
steps have run a million times a step (`HOT2`, `MACHINE_JIT_HOT2`).
Host instructions:

| | game (`blocks`, undrawn) | `blocks` boot | chk | mix | `jit` boot | chk | mix |
|---|---|---|---|---|---|---|---|
| 4096, optimised | 106.3G | 22.2G | 53.1G | 62.1G | 27.9G | 98.6G | 136.6G |
| 16384, quick only | 94.5G | 18.1G | 52.9G | 63.5G | | | |
| 16384, then optimised at 1 M | 94.6G | 18.8G | 49.6G | 59.4G | 23.9G | 94.5G | 132.7G |

(chk and mix whole, boot included.) The `jit` profile ends chk and mix at
the tick `emulate` does. The generated tests compile at 64 entries and
optimise at 1000 steps a step, so their loops run at both tiers.

Then (2026-11-09): the game undrawn, counted: 248 M instructions run
compiled, 44 M stepped (a quarter of the host instructions, about 500 a
step), 12 M reads of the status port, 69 M VGA catch-ups (its memory's
reads and writes, about 120 host instructions each). A fifth of the
stepped instructions were one bit-counting loop (`shr ax,1; jnc; inc dh;
loop`): its runs shared their slots in the run table (4096, direct
mapped) with others', evicting each other before they were entered often
enough to compile. At 16384 slots it is compiled; 36 M instructions are
stepped, a long tail: the game 91.9G host instructions from 94.6G (65536
slots: 91.8G); chk, mix and the boot the same.

Then (2026-11-09): an OPL2 (`targets/pc/opl.machine`) at the AdLib's
ports and the Sound Blaster's: its registers, timers and status (the
AdLib detection finds it), and its nine two-operator channels (phase,
envelopes, the four waveforms from the log-sin and exponent tables,
feedback, FM and additive connection; not yet tremolo, vibrato or rhythm
mode). The game finds the card and plays its music (Jingle Bells, by
ear right), 3647 register writes over the run. The chip makes its
samples as it is caught up (a register written, the status read, the
host draining its output): a program sees none of them, so they are made
in one batch a catch-up, exact to the sample. `play_disk` writes them to
`PLAY_WAV`. The game with its music, undrawn, under `blocks`: 104G host
instructions, 10.1x real time.

Then (2026-11-09): the OPL2's tremolo (a 3.7 Hz triangle, 4.8 or 1 dB)
and vibrato (6.1 Hz, 1/128 or 1/256 of the F-number) for operators that
ask, and rhythm mode (BDh bit 5: the bass drum, snare, tom-tom, top cymbal
and hi-hat, keyed from BDh, the metallic ones' phases made from the
hi-hat's and cymbal's bits and a 23-bit noise register, each heard
doubled). The game sets both depths to their deepest. 104.6G host
instructions with its music, undrawn, under `blocks`: the same as without
them.

## A game with digitised sound: Jetpack

Adept Software's Jetpack 1.5 (freeware since 1998; `build_jetpack.sh`
makes a 1.2 MB boot floppy of it from `~/roms`, DOS 3.30's boot sector
with its geometry made a 1.2 MB disk's) runs with its Sound Blaster
samples (`/S3`, `BLASTER=A220 I5 D1`): it finds the card, plays its music
on the FM chip, its effects through the DSP, and its demo (D at the main
menu) unattended:

    PLAY_DISK=.../jetpack.img PLAY_STEPS=299000000 PLAY_KEYS=115489897:20 \
    cargo test --release --test at386 -- --ignored --nocapture play_disk

2026-11-09, undrawn: `emulate` 6.3x real time, `jit` 7.8x, ending alike
(the memory, the tick, 5692 digitised samples, 3325 FM writes). Its
effects are short single-cycle transfers: half a second of samples in
the run, little load on the DSP.

## A protected-mode game: One Must Fall: 2097

Epic's One Must Fall: 2097 (freeware since 1999; `build_omf.sh` makes a
20 MB fixed disk holding it and a DOS 3.30 boot floppy starting it, with
FILES=30 and BUFFERS=30, from `~/roms`) runs in 32-bit protected mode
under the DOS/4GW extender, its music mixed in software and streamed to
the DSP by auto-initialised DMA (8-bit, 20.4 kHz; its driver polls the
8237's count, and takes no interrupt). Enter at its launcher picks the
regular game:

    PLAY_DISK=.../omf.img PLAY_HD=.../omf_hd.img PLAY_STEPS=800000000 \
    PLAY_KEYS=76737581:1c PLAY_SBWAV=omf.wav \
    cargo test --release --test at386 -- --ignored --nocapture play_disk

It found a bug in the 386: CPL was CS's RPL whenever PE was set, so
setting PE in a real-mode segment whose low bits are not 0 (DOS/4GW runs
in 291Dh) made the far jump into its ring-0 code a #GP, then a shutdown.
CPL is now 0 until a far transfer loads CS. After about 80 s of loading
(DOS 3.30, an MFM disk), the intro plays, its music right by ear; under
`blocks` at about 10x real time (2026-11-09).

Then (2026-11-09): One Must Fall measured. The same 2.4 G instructions
(boot, DOS/4GW, loading, the intro, 572 emulated seconds) end alike on
`emulate` and `jit`: the memory, the tick, 3476108 samples at 800 M.
Where the host instructions go over 800 M (`jit`; `blocks`): interpreted
steps 25%; 30% (its loading through DOS in real mode, and lukewarm
code), compiling 17%; 18%, compiled code 15%; 11%, the DSP and the DMA
controller directly 1.4%; 2.2%, all devices' syncs 3.2%; 4.9%: the sound
is not what costs.

At its main menu, its music playing (instructions 1.6 G to 2.4 G; QEMU
from 60 to 120 s of wall time, its BIOS tick count read through its
monitor, Enter sent by `sendkey`):

| | speed | guest MIPS | instructions an emulated second |
|---|---|---|---|
| ours, `emulate` | 4.0x | 17 | 4.3 M (the 386's timing) |
| ours, `jit` | 8.1x | 35 | 4.3 M |
| ours, `blocks` | 11.4x | 62 | 5.4 M |
| QEMU 8.2, `-icount shift=7,sleep=off` | 6.1x | 47 | 7.8 M (by `shift=7`) |

QEMU in its usual mode keeps time by the host's clock, and so does
DOSBox: the game, waiting on its timer, then runs at its own pace and
spins, and neither gives a speed; under `-icount` QEMU's time follows its
instructions, at some cost to its speed. Bochs (`sync=none`) would, but
it offers no headless way to read the guest's time or press a key. The
game's launcher reads the keyboard itself, so a key stuffed into the
BIOS's buffer does not reach it.

Then (2026-11-09): the replay loop. A step that enters a recorded run
now replays the run's rest in one loop (`trace_burst`), keeping each
step's checks, its interrupt check and the devices' catch-up, but not
returning through `step` and the system's run loop between
instructions. The `jit` profiles use it where they interpret. On `chk`
under `trace`, host instructions per guest instruction (N passes less
none) went from 412.5 to 377.8 (-8.4%); the run ends alike (the tick,
the memory, the samples). On the Lemmings script (every frame drawn), where
compiled code does most of the work, it is about 1%: `jit` 214.1 G to
212.3 G host instructions, `blocks` 188.7 G to 186.8 G, each ending as
before (frame, memory, tick).


Then (2026-11-09): DOSBox measured the way we are. `build_dosbox.sh`
builds DOSBox 0.74-3 with `patches/dosbox-0.74-3.patch` (sources fetched
at pinned releases and checked; none kept here). In the patch, emulated time
follows the cycles alone: fast forward is held on, so `cycles=fixed N` is N
cycles an emulated millisecond however fast the host is. The guest's
clock (CMOS, the BIOS tick's resync, DOS's date) is 1993-01-01 12:00 plus
emulated time. Keys are pressed at emulated times, and a report (cycles,
the normal core's instructions, the BIOS tick, a hash of memory,
optionally a screenshot) comes at set times. A run repeats exactly. The
runner, `dosbox_bench.py IMG T1 T2`, boots the disks `play_disk` boots and
counts host instructions (`perf stat`) between T1 and T2.

Lemmings at its main menu (40 s to 90 s, a banner scrolling; on the same
screen in both by 20 s, by screenshots), host instructions an emulated
second. DOSBox at `cycles=fixed 3000` runs 2.69 M instructions an
emulated second there, ours 2.63 M; both draw every frame:

| | every frame | drawing (almost) none | drawing | the rest, a guest instruction |
|---|---|---|---|---|
| ours, `jit` | 2.30 G | 0.96 G (none) | 1.34 G | 364 |
| ours, `blocks` | 2.13 G | 0.79 G (none) | 1.34 G | 300 |
| DOSBox normal | 0.54 G | 0.33 G (1 in 11) | 0.23 G | about 115 |
| DOSBox dynamic | 0.39 G | 0.19 G (1 in 11) | 0.22 G | about 64 |

Drawing costs us six times what it costs DOSBox, which compares each
line with the last frame's and redraws only what changed (ours draws a
frame dot by dot at the CRTC's timing). Our `blocks` profile costs 4.7
times the dynamic core per guest instruction otherwise, and 2.6 times the
normal interpreter. DOSBox's timing is not the 386's (a cycle is about an
instruction), so its work is not exact; ours (`jit`) is.

Then (2026-11-09): the comparison repeatable from a clone. The disks
boot FreeDOS 1.4 (GPL) instead of MS-DOS 3.30, with HimemX (without it
FreeDOS leaves Jetpack 20 KB short) and DOS=HIGH (first left out: with
the kernel in the high memory area, programs load below 64 KB, and
Lemmings' packed VGAL then wraps at 1 MB, which needs the A20 gate closed,
and the direct-memory profiles ignored the gate: an "Invalid Opcode" on
ours, fine in DOSBox; fixed below). `fetch_games.sh`
fetches FreeDOS's Floppy Edition and HimemX from their releases and the
three freeware games from archive.org, each checked by SHA-256; the
`build_*.sh` scripts make the same disks byte for byte on every build
(files dated 1993-01-01, a fixed volume serial). `chk` costs the same
on FreeDOS (377.9 host instructions a guest instruction, 377.8 before).
Only the board's ROMs (AMI's BIOS, IBM's VGA BIOS) are still yours to
supply.

`compare_games.py` makes the games table: it builds the tree's crates
for the profiles asked for, runs ours and the patched DOSBox over each
game's window (drawing every frame and drawing none or almost none),
finds the DOSBox cycles that match our guest instructions an emulated
second, and prints medians as markdown (saved in hdl/target/compare).
The windows, chosen by screenshots of both every 20 s: Lemmings' main
menu 40-90 s; Jetpack's attract mode (title, demo, high scores; a
cycle is under 120 s on both) 60-180 s; One Must Fall's main menu,
Enter pressed at 45 s, 160-240 s (both there by 80 s). `perfbin.py`
finds a perf that runs (Ubuntu's wrapper refuses a WSL kernel it has
no tools for; a linux-tools perf counts instructions on any kernel).
From a clone:

    sh fetch_games.sh && sh build.sh && sh build_lemm.sh \
      && sh build_jetpack.sh && sh build_omf.sh && sh build_dosbox.sh
    python3 compare_games.py           # with AMI's and IBM's ROMs in ~/roms
    sh build_bios.sh && python3 compare_games.py --free-roms   # or without

The first table (screenshots of both at each window's ends checked):

Xmas Lemmings, its main menu (a banner scrolling), 40 s to 90 s; DOSBox at `cycles=fixed 2940`:

| | every frame | drawing none | drawing | the rest, a guest instruction | guest instructions an emulated second |
|---|---|---|---|---|---|
| ours, `jit` | 2.31 G | 0.97 G (none) | 1.34 G | 369 | 2.62 M |
| ours, `blocks` | 2.13 G | 0.80 G (none) | 1.34 G | 303 | 2.62 M |
| DOSBox 0.74-3, normal | 0.53 G | 0.33 G (1 in 11) | 0.22 G | 115 | 2.65 M |
| DOSBox 0.74-3, dynamic | 0.39 G | 0.19 G (1 in 11) | 0.23 G | 63 | 2.65 M |

Jetpack, its attract mode (title, demo, high scores), 60 s to 180 s; DOSBox at `cycles=fixed 4580`:

| | every frame | drawing none | drawing | the rest, a guest instruction | guest instructions an emulated second |
|---|---|---|---|---|---|
| ours, `jit` | 1.96 G | 1.36 G (none) | 0.61 G | 401 | 3.38 M |
| ours, `blocks` | 1.83 G | 1.23 G (none) | 0.61 G | 363 | 3.38 M |
| DOSBox 0.74-3, normal | 0.53 G | 0.40 G (1 in 11) | 0.15 G | 113 | 3.38 M |
| DOSBox 0.74-3, dynamic | 0.35 G | 0.21 G (1 in 11) | 0.15 G | 59 | 3.38 M |

One Must Fall 2097, its main menu (music playing; Enter at 45 s), 160 s to 240 s; DOSBox at `cycles=fixed 5600`:

| | every frame | drawing none | drawing | the rest, a guest instruction | guest instructions an emulated second |
|---|---|---|---|---|---|
| ours, `jit` | 1.57 G | 1.21 G (none) | 0.35 G | 226 | 5.38 M |
| ours, `blocks` | 1.12 G | 0.77 G (none) | 0.35 G | 143 | 5.38 M |
| DOSBox 0.74-3, normal | 0.69 G | 0.58 G (1 in 11) | 0.13 G | 105 | 5.40 M |
| DOSBox 0.74-3, dynamic | 0.33 G | 0.21 G (1 in 11) | 0.13 G | 36 | 5.40 M |

Drawing is the larger gap where the screen changes most (Lemmings'
scrolling banner: 6 times DOSBox's); the processor elsewhere: Jetpack's
demo costs `blocks` 363 host instructions a guest instruction against 59
for DOSBox's dynamic core. One Must Fall's menu is the closest: `blocks` 143 against 36.

Then (2026-11-10): the A20 gate in direct memory. The 386's model now
declares what it does to addresses on their way to the bus, `on
mem.address(addr) -> u32` (the gate), a fn of the address and its input
wires alone; the emulator profiles' direct memory, which bypasses the
model's handlers, applies it from there (an access straddling the wrap
goes by the handlers). The wires it reads are a key: a change counts as
a code write, runs are kept and compiled for each value of the key (in
their key and slot hash), and compiled code folds the gate into the
address mask it already applies. HimemX now finds the keyboard
controller's A20 method (it found the gate "always on" before), and the
disks boot with DOS=HIGH again: Lemmings runs on every profile, `trace`
and `jit` ending alike (frame, memory, time at 110 M instructions).

Cost: interpreted code pays a load and an AND per access and fetch, and
one more counter in the code-write check: `chk` under `trace` 377.9 to
391.5 host instructions a guest instruction (+3.6%; a first version, the
hook called at each access, +4%). Compiled code pays nothing; on the games
(table below, DOS=HIGH) ours moves within about 2% of the table above.

Xmas Lemmings, its main menu (a banner scrolling), 40 s to 90 s; DOSBox at `cycles=fixed 2940`:

| | every frame | drawing none | drawing | the rest, a guest instruction | guest instructions an emulated second |
|---|---|---|---|---|---|
| ours, `jit` | 2.32 G | 0.98 G (none) | 1.34 G | 373 | 2.62 M |
| ours, `blocks` | 2.14 G | 0.81 G (none) | 1.34 G | 307 | 2.62 M |
| DOSBox 0.74-3, normal | 0.54 G | 0.33 G (1 in 11) | 0.23 G | 116 | 2.65 M |
| DOSBox 0.74-3, dynamic | 0.39 G | 0.19 G (1 in 11) | 0.23 G | 63 | 2.65 M |

Jetpack, its attract mode (title, demo, high scores), 60 s to 180 s; DOSBox at `cycles=fixed 4580`:

| | every frame | drawing none | drawing | the rest, a guest instruction | guest instructions an emulated second |
|---|---|---|---|---|---|
| ours, `jit` | 1.99 G | 1.38 G (none) | 0.61 G | 409 | 3.38 M |
| ours, `blocks` | 1.81 G | 1.21 G (none) | 0.61 G | 357 | 3.38 M |
| DOSBox 0.74-3, normal | 0.53 G | 0.40 G (1 in 11) | 0.15 G | 113 | 3.38 M |
| DOSBox 0.74-3, dynamic | 0.35 G | 0.21 G (1 in 11) | 0.15 G | 58 | 3.38 M |

One Must Fall 2097, its main menu (music playing; Enter at 45 s), 160 s to 240 s; DOSBox at `cycles=fixed 5600`:

| | every frame | drawing none | drawing | the rest, a guest instruction | guest instructions an emulated second |
|---|---|---|---|---|---|
| ours, `jit` | 1.54 G | 1.19 G (none) | 0.35 G | 221 | 5.37 M |
| ours, `blocks` | 1.14 G | 0.79 G (none) | 0.35 G | 146 | 5.38 M |
| DOSBox 0.74-3, normal | 0.70 G | 0.58 G (1 in 11) | 0.13 G | 105 | 5.40 M |
| DOSBox 0.74-3, dynamic | 0.33 G | 0.21 G (1 in 11) | 0.13 G | 36 | 5.40 M |

Then (2026-11-10): free ROMs. `build_bios.sh` builds the Bochs project's
2.7 "legacy" BIOS (LGPL; for ISA machines, without PCI) with
`patches/rombios-2.7-at.patch`, and takes its LGPL VGABIOS 0.8a, into
~/roms/free; built unpatched, the same steps make Ubuntu's bochsbios byte
for byte. The BIOS was written for Bochs's hardware, and the patch makes
it an AT's where it relied on that:
- it seeks before reading, writing or formatting a diskette (Bochs's
  controller moved the head to the cylinder asked for; a 765 reads where
  its head is: Wrong Cylinder at the first read past cylinder 0);
- it gates A20 through the keyboard controller (it used port 92h, the
  PS/2's, which an AT does not have), telling the gate's state by the
  wrap at 1 MB, as HIMEM does;
- it closes the gate before booting, as an AT's POST does (the 8042 comes
  out of reset with it open; HimemX took that for "always on").
The board changed too, as a 386 AT of its time could: the VGA's ROM
window is 64 KB (C0000-CFFFF; the free VGABIOS is 38 KB), and the WD1003
answers ECh, IDENTIFY, as the AT-bus IDE drives that followed it did (the
BIOS finds its disks so), with the IDE signature after a reset. The AT
tests take `PC_BIOS` and `PC_VGABIOS`; `free_roms_boot_freedos` boots
FreeDOS's floppy on them with the fixed disk identified. Lemmings ends
alike on `trace` and `jit` at 120 M instructions; One Must Fall boots
from the fixed disk. `compare_games.py --free-roms` makes the table on
them, needing nothing that is not free (screenshots checked at each
window's ends; the Bochs BIOS, without a memory test, boots some seconds
sooner):

Xmas Lemmings, its main menu (a banner scrolling), 40 s to 90 s; DOSBox at `cycles=fixed 2880`:

| | every frame | drawing none | drawing | the rest, a guest instruction | guest instructions an emulated second |
|---|---|---|---|---|---|
| ours, `jit` | 2.32 G | 0.98 G (none) | 1.34 G | 380 | 2.57 M |
| ours, `blocks` | 2.15 G | 0.81 G (none) | 1.34 G | 314 | 2.57 M |
| DOSBox 0.74-3, normal | 0.53 G | 0.32 G (1 in 11) | 0.22 G | 116 | 2.61 M |
| DOSBox 0.74-3, dynamic | 0.39 G | 0.19 G (1 in 11) | 0.23 G | 63 | 2.61 M |

Jetpack, its attract mode (title, demo, high scores), 60 s to 180 s; DOSBox at `cycles=fixed 4730`:

| | every frame | drawing none | drawing | the rest, a guest instruction | guest instructions an emulated second |
|---|---|---|---|---|---|
| ours, `jit` | 1.92 G | 1.34 G (none) | 0.58 G | 385 | 3.49 M |
| ours, `blocks` | 1.74 G | 1.16 G (none) | 0.58 G | 333 | 3.49 M |
| DOSBox 0.74-3, normal | 0.54 G | 0.41 G (1 in 11) | 0.15 G | 114 | 3.49 M |
| DOSBox 0.74-3, dynamic | 0.35 G | 0.22 G (1 in 11) | 0.15 G | 59 | 3.49 M |

One Must Fall 2097, its main menu (music playing; Enter at 45 s), 160 s to 240 s; DOSBox at `cycles=fixed 5600`:

| | every frame | drawing none | drawing | the rest, a guest instruction | guest instructions an emulated second |
|---|---|---|---|---|---|
| ours, `jit` | 1.55 G | 1.20 G (none) | 0.35 G | 224 | 5.37 M |
| ours, `blocks` | 1.13 G | 0.78 G (none) | 0.35 G | 144 | 5.38 M |
| DOSBox 0.74-3, normal | 0.70 G | 0.58 G (1 in 11) | 0.13 G | 105 | 5.40 M |
| DOSBox 0.74-3, dynamic | 0.33 G | 0.21 G (1 in 11) | 0.14 G | 36 | 5.40 M |
