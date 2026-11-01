# i286: known divergences from the SingleStepTests corpus

The full real-mode sweep (`machine vectors --isa I286 targets/i286/*.machine`)
reports every vector as passed, failed, unimplemented (opcode not described
yet), an undefined-flag leak, or revoked. This file lists what is not
"passed" for a reason other than "not described yet".

## Undefined-flag leaks: faulting DIV / IDIV / AAM (11,242 vectors)

DIV and IDIV leave all six arithmetic flags undefined, so the register
comparison masks them. When the divide faults (#DE), the 286 pushes FLAGS in
the interrupt frame, and the pushed copy holds whatever the divider's
microcode loop left in the flags. That is data-dependent (it differs from the
initial flags in ~98% of vectors) and no simple rule reproduces it: the flags
of the overflow comparison match ~13% of cases, and divide-by-zero varies with
the dividend. Reproducing it means reverse-engineering the 286 divide
microcode.

`aam 0` (#DE) is the same mechanism with a more regular result: SF, ZF,
CF, AF and OF are cleared and PF is the parity of AL in 9 of the 11
captures. The description produces that; the other 2 are leaks.

The runner classifies a vector as a leak only when the expected fault was
delivered, every register matches, and every differing RAM bit is a flag bit
the executed instruction declares undefined. Leaks are not counted as passed.

## Suspected capture anomalies: 8-bit IDIV that should overflow (4 vectors)

In F6.7 (`idiv r/m8`), 4 vectors with a negative dividend, a positive divisor
and a true quotient below -128 complete without #DE and return AL = 0x80. Of
the 134 register-form vectors with the same sign pattern and an out-of-range
quotient, 132 fault; the non-faulting ones sit between faulting neighbours
with nearly identical operands (q = -287 does not fault, -285 and -292 do).
There is no rule to describe, so the description does not try.

| index | instruction | hash |
|---|---|---|
| 952 | `idiv byte [ds:si]` | `0038b4bacfb75535b5da175f619b0812b16d0601` |
| 1085 | `idiv dh` | `de153d1e3812cdb2c9d25272844b4b28a5adc35f` |
| 2653 | `idiv bh` | `dce03c62813266bf0ba50e3325fa3898132cad1f` |
| 4297 | `idiv byte [ds:bx+si+3Bh]` | `38a27640b8a9475f75998d2cab801d51eb8bb0b2` |

These are candidates for the corpus's `revocation_list.txt`.
