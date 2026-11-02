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
ModRM followed by a displacement costs the decoder one more clock in the
8-bit-displacement forms. After an unconditional transfer (JMP, CALL, RET,
IRET) the decoder stops taking sequential bytes; after a conditional one it
carries on, not yet knowing the outcome.

**Execution unit.** An instruction starts when its last byte is decoded.
Its time is a schedule: cycles of work before its first step, and after
each step. A step is a data access or the jump. A request reaches the bus
two clocks after the execution unit makes it, ahead of any prefetch.
**Reads wait** until their last bus cycle starts; **writes are posted** and
the execution unit carries on (PUSH's successor starts before the write is
on the bus). A split word access is two bus cycles.

**Prefixes** are bytes the decoder takes like any other, which is all the
effect they have on timing.

## How well it fits

Schedules per form are short: MOV [m], r is 3 to 5 cycles of work, then
the posted write; MOV r, [m] is the work, then a waiting read; MUL r16 is
21 cycles, MUL m16 the address work, a waiting read and 20 cycles; ADD
[m], r reads, waits, works 2 cycles and posts the write; CALL near works 4
cycles, jumps, then posts the push; RET reads the return address, works 4
cycles and jumps.

With one set of global constants (queue 6, request latency 2, jump latency
1, HLT 3), these reproduce the traces exactly:

| Opcodes | Vectors | Exact |
|---|---|---|
| NOP, INC, MOV r, imm16, MOV [m], r (all modes and prefix counts), MOV r, [m], ADD (register and read-modify-write), POP, PUSH, MUL r/m16 | 13,177 | 99.95% |
| JE (taken and not), JMP short and near | 2,374 | 100% |
| CALL near | 600 | 100% |
| RET | 600 | about 94% |
| IRET | 581 | 89% |

A broad run over 74 representative opcode files (`RESULTS.md`) reproduces
25,282 of 25,636 vectors exactly (98.6%); every file but one is at 90% or
better and most at 100%.

## Not covered yet

- Repeated string instructions (several iterations per vector).
- Vectors that raise an exception (delivery: pushes, IVT reads, the jump).
- The corner cases behind RET's and IRET's misses (a return to the last
  byte of a segment, where the next fetch wraps).

## Toward the language

The model needs two things from a description. The processor model holds
the bus unit, decoder and execution-unit timeline as model state and code.
Each instruction form's schedule, which is the work before and after each of
its steps and where its jump falls among its accesses, belongs in the timing
table in place of tier 1's single number. Whether a step waits follows from
whether it is a read, so the schedule is a short list of numbers and a jump
position. The decoder also needs to know where the ModRM is and whether a
displacement follows it, which the interpreter knows when it decodes.
