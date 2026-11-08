# 386 comparison

The same two DOS programs on our 386 AT board and on other emulators, on
one host, unthrottled: guest instructions per second in steady state.

- `chk.asm`: a 16-bit checksum over 32 KB (memory and ALU), 8000 passes,
  about 918 M instructions.
- `mix.asm`: 32-bit arithmetic through calls, multiply and divide, string
  moves and scans, a sieve; 36000 passes, about 994 M instructions. Its
  data is on pages of its own (writes beside code make tools that watch
  code by page retranslate it).

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

Since (2026-11-05, our JIT only, two runs): 113 MIPS on chk and 70 on mix.
The flags wait to be put in EFLAGS until it is needed whole, joins carry
fields in registers past the devices' check, and a run whose end leads
to its start goes round without returning to the dispatcher.
