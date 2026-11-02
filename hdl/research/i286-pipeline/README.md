# The 80286 pipeline, from its cycle traces

A cycle-level model of the 286's bus unit, decoder and execution unit,
fitted against the SingleStepTests cycle traces. It is the groundwork for
tier 2 timing (cycle-accurate bus activity from the processor model), done
in Python first because a model change here refits in seconds.

```
python3 -I fit.py 89 01 E8 C3 F7.4      # opcode files of the corpus
```

`traces.py` reads the corpus, `pipeline.py` is the model, `fit.py` fits each
instruction form's schedule and reports how many vectors it reproduces
exactly: every bus cycle (code fetch, memory or I/O read or write, HALT) at
the hardware's start cycle.

## What the traces show

Every vector starts at a jump (an empty queue) and ends at an injected HLT,
so each trace shows one instruction's interaction with the queue refill.

**Bus.** A bus cycle is two clocks (Ts, Tc) and cycles run back to back.
The address of the next cycle goes out during the current Tc, so the
choice of the next cycle is made one clock ahead.

**Bus unit (prefetch).** The queue holds 6 bytes. The bus unit fetches the
next aligned word whenever a word's worth of room is free (counting
fetches in flight), and starts fetching the moment a jump flushes the
queue. A fetch at an odd target brings one byte, and its word slot's other
half stays occupied until the next flush, which is why an odd target
fetches no more than an even one. After a jump, the target's fetch is
decided one clock after the flush. Once HLT is executing, no more fetches
start.

**Decoder.** One byte per clock, in order, each no earlier than the clock
after it arrives; the bus unit sees the freed space a clock later. A
**sign-extended byte** (an 8-bit displacement, a short branch's offset, the
immediate of `83 /r`, `6B` and `6A`) costs the decoder a clock: when it
starts a word the decoder stalls on it, delaying the bytes after it too;
otherwise the instruction starts a clock late. After an unconditional
transfer (JMP, CALL, RET, IRET) the decoder stops taking sequential bytes;
after a conditional one it carries on, not yet knowing the outcome.

**Execution unit.** An instruction starts when its last byte is decoded.
Its time is a schedule: cycles of work before its first step, and after
each step. A step is a data access or the jump. A request reaches the bus
two clocks after the execution unit makes it, ahead of any prefetch.
**Reads wait** until their last bus cycle starts; **writes are posted** and
the execution unit carries on (PUSH's successor starts before the write is
on the bus). A split word access is two bus cycles.

**Prefixes** are bytes the decoder takes like any other, which is all the
effect they have on timing.

**Address work** before a ModRM operand's first access is 3 clocks, or 4
for base+index with a displacement, whatever the instruction.

## How well it fits

Schedules per form are short: MOV [m], r is the address work, then the
posted write; MOV r, [m] is the work, then a waiting read; MUL r16 is 21
cycles, MUL m16 the address work, a waiting read and 20 cycles; ADD [m], r
reads, waits, works 2 cycles and posts the write (CMP works 1 and does not
write); CALL near works 4 cycles, posts the push and jumps; a far CALL
pushes CS, jumps, then pushes IP; RET reads the return address, works 4
cycles and jumps.

Fitting pools the prefix counts: an unprefixed vector often cannot tell
two values apart (POP's 2 or 3 clocks before its read), and the prefixed
ones settle it.

With one set of global constants (queue 6, request latency 2, jump latency
1, HLT 3), these reproduce the traces exactly:

| Opcodes | Vectors | Exact |
|---|---|---|
| NOP, INC, MOV r, imm16, MOV [m], r (all modes and prefix counts), MOV r, [m], ADD (register and read-modify-write), POP, PUSH, MUL r/m16 | 13,177 | 99.95% |
| JE (taken and not), JMP short and near | 2,374 | 100% |
| CALL near | 600 | 100% |
| RET | 600 | about 94% |
| IRET | 581 | 89% |

A fit of every opcode file (`RESULTS.md`, 200 vectors each) reproduces
nearly all vectors exactly. The schedules are in the description,
`targets/i286/schedules.machine`, and the processor model there runs the
same rules: over the whole corpus (string instructions and exceptions
aside, 123,600 vectors) it reproduces 97% of the traces bus cycle for bus
cycle (`rig.machine`, `rig_test.rs`).

## Not covered yet

- Repeated string instructions (several iterations per vector).
- Vectors that raise an exception (delivery: pushes, IVT reads, the jump),
  INT among them.
- INTO without overflow: the decoder waits for the outcome without a
  transfer, which a schedule does not say yet.
- ENTER above level 0: work after a variable number of accesses.
- ESC: the hardware writes the instruction to the coprocessor's ports.
- The corner cases behind RET's and IRET's misses (a return to the last
  byte of a segment, where the next fetch wraps), and shifts by a count
  through memory (about 93%).

## In the language

SPEC 11.2c: the processor model (`Cpu286` in `targets/i286/board.machine`)
holds the bus unit, decoder and execution unit as model state and code, and
runs each instruction's schedule from the `schedules` table when the
interpreter retires it. Whether a step waits follows from whether it is a
read, so a schedule is a short list of numbers and steps. The decoder's
sign-extended-byte rule is `decode Disp8 => 1;`: the interpreter marks the
bytes of that operand format.
