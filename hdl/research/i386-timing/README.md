# The 80386's timing, from its cycle traces

Analysis behind `targets/i386/timing.machine` (tier 1: an instruction's
execution cycles beyond its bus transfers), from the SingleStepTests 80386
corpus (a 386EX: 16-bit bus, two clocks a bus cycle, no wait states).

```
python3 -I traces.py 8B 3                      # one vector's trace
python3 -I features.py 8B                      # cycles by candidate features
python3 -I joint.py ea,data,addr\&3 8B 89      # by a joint key
python3 -I by.py "r['ecx'] & 0x1F" D3.4        # by any expression
python3 -I mul.py F7.4 16                      # multiply/divide by operand bits
python3 -I strings.py A4 66A5                  # repeated strings, aligned
python3 -I table.py fit.log                    # `--fit` output -> entries
```

A vector's **measured** cycles are its trace's clocks outside bus cycles,
less one: the first clock of the rig's HLT bus cycle.

## What the traces show

- **Address work.** A ModRM memory operand costs a clock for a 16- or
  32-bit displacement (an 8-bit one costs nothing) and one for an index
  register (`EaWork`, the kinds' `ea` query).
- **Split accesses.** An access that crosses a dword boundary costs four
  more clocks: the core makes it as two. One that crosses only a word
  boundary costs nothing more (the 16-bit bus splits it). In pipelined
  sequences (repeated strings, ENTER) the penalty overlaps and differs
  (a misaligned STOSD destination costs 4 or 6 an element, a MOVSD one 2).
- **Multiply** is early-out on the multiplier's significant bits (of its
  magnitude for IMUL, at least 3): bits + 7 for MUL/IMUL r/m, bits + 5 for
  the two- and three-operand IMULs, whose multiplier is the immediate when
  there is one. Divide is constant.
- **Shifts and rotates** do not depend on the count.
- **BSF/BSR** scan a bit a clock-triple: 7 + 3 per trailing zero, 5 + 3
  per leading zero.
- **Repeated strings**: a first element, then a fixed cost per element
  (MOVS 0, STOS 3, LODS 4, CMPS 5, SCAS 6, INS 3, OUTS 4 beyond the bus;
  1 for STOSD), and 3 for a zero count (INS/OUTS 15, 14).
- **ENTER** costs the same at any level above 1: the copies are bus time.
- **Branches**: Jcc 4 taken, 3 not; LOOPs 8 taken, 10 not.

## What tier 1 cannot see

A clock that alternates with the instruction's length (the number of
prefixes): it depends on where the next instruction (the rig's HLT) falls
in the decoder, and is not the instruction's own. The table takes the value
that most vectors are within a clock of. Over the whole corpus the table
matches 44% of vectors exactly and 77% within a clock (mean error 1.6);
modelling the decoder and prefetch queue (tier 2, as `research/i286-pipeline`
did for the 286) is what would remove the rest.
