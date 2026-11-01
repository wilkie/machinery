# Machine: a hardware description language for generated emulators

Status: **draft 0.1, for review.** Nothing here is implemented. Open questions
are collected at the end and marked `[open]` inline.

## 1. Purpose

Machine describes computing hardware (processors, buses, devices, whole
systems) precisely enough that a compiler can generate, from one description:

- an **accurate simulator**: externally cycle-accurate, meaning every pin and
  bus signal has the right value on every clock edge of every clock domain,
  as observable from outside the chip;
- a **fast emulator**: a practical, competitive emulator that reproduces
  architectural behaviour and timing to the degree the description allows;
- the supporting tools: assembler, disassembler, decode tables, debugger
  metadata, documentation.

The two machine classes are not two compilations of the same code at different
optimisation levels. A wire-level model cannot be compiled into a fast emulator;
the information needed to do so (what a block of logic is *for*) is not in it.
Instead, a Machine description carries **up to three levels of semantics for
each component**, and the compiler's job is to **derive** what it can between
the levels and to **verify** that they agree.

| Level | Describes | Written as | Used by |
|---|---|---|---|
| Architectural | What instructions, registers and address spaces *mean* | `isa` | Emulator (derived interpreter), assembler, disassembler |
| Transaction | Bus operations with latencies: read, write, interrupt, DMA | `model` bodies, `bus` transactions | Emulator's device and bus layer |
| Wire | Clocked registers, combinational wires, bus handshakes | `rtl` bodies, `bus` wire protocols | Accurate simulator |

A system build (`profile`) picks, per component, which level to run. The
emulator profile runs the highest level each component has and drops to the
wire level where nothing higher exists or where the author pins it. The
accurate profile runs the wire level everywhere, wrapping any component that
has only a transaction model in a generated protocol adapter.

### 1.1 Design principles

1. **Correctness first, speed second.** The description is the source of
   truth; a wrong fast path is a bug, not a trade-off.
2. **Write each fact once.** Flag computation, decode tables, cycle counts,
   operand shapes and documentation are derived, never duplicated.
3. **Verification is a language feature.** Every pair of levels for the same
   component is checked against each other by generated differential tests.
4. **Readable by people and agents.** Rust/C-like surface syntax, braces and
   semicolons, explicit keywords, explicit casts, one way to say each thing,
   canonical formatting, machine-readable diagnostics.
5. **Documentation is data.** Descriptions and references on every construct
   feed the reverse-engineering tools; they are not comments.

### 1.2 Non-goals

- Synthesis to real hardware. Machine borrows from HDLs but targets software.
- Analogue behaviour. Signals are digital values in a clock domain. (Analogue
  video output is produced by a `model`, not simulated as voltages.)
- Reproducing undocumented internals. "Cycle-accurate" means externally
  observable behaviour. Internal structure is the author's choice.

## 2. Vocabulary

| Term | Meaning |
|---|---|
| **unit** | A combinational, stateless function from inputs to outputs. Shared by every level. |
| **component** | A piece of hardware with ports. Has an `rtl` body and/or a `model` body. |
| **rtl** | The wire level of a component: registers, memories, wires, sub-component instances, routines. |
| **model** | The transaction level of a component: handlers for bus transactions and clock ticks. |
| **isa** | The architectural level of a processor: registers, address spaces, instructions, faults. |
| **bus** | A protocol: shared wires, roles, and transactions defined as wire-level sequences. |
| **system** | A composition of components and buses on clocks, with build profiles. |
| **routine** | A multi-cycle procedure inside `rtl`, compiled to a sequencer (state machine). |
| **profile** | A named fidelity selection for a system build. |

## 3. Lexical structure

- UTF-8 source, file extension `.machine`.
- `//` line comments. `/* */` block comments, non-nesting.
- `///` doc comments attach to the following declaration and become its
  `description` metadata. Markdown.
- Identifiers: `[A-Za-z_][A-Za-z0-9_]*`. Convention: `PascalCase` for types,
  components, instructions and enum variants; `snake_case` for values, ports,
  wires, registers, routines; `UPPER_SNAKE` for constants and architectural
  registers.
- Integer literals: `42`, `0xFF`, `0b1010_0101`, `0o17`. Underscores allowed.
  A literal may carry a width suffix: `0xFF_u8`, `1_u1`. Unsuffixed literals
  take their width from context; it is an error if none is available.
- Time and frequency literals in `system` only: `33.333 MHz`, `14.318_18 MHz`,
  `70 ns`.
- Strings: `"..."` with `\"`, `\\`, `\n` escapes. Used for metadata only.
- Statements end with `;`. Blocks are `{ }`. Whitespace is insignificant.

## 4. Types

### 4.1 Scalars

- `u<N>` unsigned, `i<N>` signed two's complement, for `1 <= N <= 128`.
  `u8`, `u16`, `u20`, `u32`, `i16` ... are sugar for `u<8>` etc. `N` may be a
  width expression over const parameters: `u<W + 1>`.
- `bool`. Distinct from `u1`. Comparisons, `&&`, `||`, `!` produce and consume
  `bool`. `if`, `assert`, `wait until` require `bool`. Bit indexing yields
  `u1`. Convert with `as`.
- No implicit widening or narrowing anywhere. Arithmetic requires equal
  widths. This is deliberate: hardware widths are facts, and silent coercion
  hides bugs.

### 4.2 Casts

- `e as u<N>`: if `N` is narrower, truncate; if wider, zero-extend (for
  unsigned `e`) or sign-extend (for signed `e`). `e as i<N>` likewise.
- `e as bool` is legal only for `u1`; `b as u1` for `bool`.
- `e as Enum` from an unsigned of the enum's width; `Enum as u<N>` the reverse.
  Out-of-range values are a checked error in both machine classes.

### 4.3 Composite types

```
enum AluOp : u3 { Add, Or, Adc, Sbb, And, Sub, Xor, PassA }
```
Enums have an explicit backing width. Variants are numbered in order unless
given values (`Add = 0`). Unassigned encodings are illegal values.

```
bundle Flags { cf: u1, pf: u1, af: u1, zf: u1, sf: u1, of: u1 }
```
A bundle is a named struct. Its bit layout is the field order, first field at
bit 0, unless a field carries an `@ offset` (see registers). Literal:
`Flags { cf: 1, pf: 0, af: 0, zf: 1, sf: 0, of: 0 }`. All fields required;
`..default` fills remaining fields from the bundle's declared defaults.

```
union MicroOp { Alu(AluMicro), Read(BusRead), Write(BusWrite), Retire }
```
A tagged union. Width is tag plus the widest payload. Matched with `match`.

- Arrays: `[T; N]`, indexed `a[i]`. Fixed size, constant `N`.
- Tuples: `(u8, bool)`, destructured with `let (x, y) = ...;`.

### 4.4 Memories

```
mem vram: [u8; 0x4000] { ports: 2, read: async, write: sync, init: zero }
```
A memory is state with explicit access ports. `read: async` means the read is
combinational in the cycle it is issued; `read: sync` means the value is
available the next cycle. Writes always commit at the clock edge. The number
of ports bounds concurrent accesses per cycle; exceeding it is a compile error
where statically detectable and a runtime assertion otherwise. In `model`
bodies a memory is simply an array.

### 4.5 Parameters

Units, components and buses take `const` parameters:

```
unit Alu<const W: width>(op: AluOp, a: u<W>, b: u<W>, cin: u1) -> (result: u<W>, raw: u<W + 1>)
```
`width` is the kind of positive integers usable in `u<...>`. Instantiation is
monomorphising: `Alu<8>` and `Alu<16>` are separate generated functions.

## 5. Expressions

Operators, by decreasing precedence, all left-associative unless noted:

| Operators | Notes |
|---|---|
| `a.f`, `a[i]`, `a[hi:lo]`, `f(args)`, `T::V` | field, bit, slice (inclusive, `hi >= lo`), call, enum variant |
| `!`, `~`, `-` | logical not (bool), bitwise not, negate (signed) |
| `as` | cast |
| `*`, `/`, `%` | `/`, `%` are unsigned or signed by operand type; width-preserving; divide by zero is a checked error |
| `+`, `-` | width-preserving, wrap; use `u<W+1>` casts to observe carry |
| `<<`, `>>` | shift amount is any unsigned; `>>` on `i<N>` is arithmetic |
| `&` | |
| `^` | |
| `\|` | |
| `==`, `!=`, `<`, `<=`, `>`, `>=` | produce `bool` |
| `&&` | short-circuit (bool) |
| `\|\|` | short-circuit (bool) |
| `c ? a : b` | right-assoc; `c: bool`, `a`/`b` same type |

Other expression forms:

- `concat(a, b, c)`: bit concatenation, first argument most significant.
- `replicate(x, n)`.
- `popcount(x)`, `clz(x)`, `ctz(x)`, `parity(x)` built-ins returning `u<log2>` or `u1`.
- Unit call: `Alu<8>(op, a, b, cin)`. Returns a tuple if the unit has several
  outputs; access as `.result` / `.raw` or destructure.
- `match` as an expression:
  ```
  let head: u8 = match q_read { 0 => q0, 1 => q1, 2 => q2, _ => q3 };
  ```
  Arms must be exhaustive. Enum matches are exhaustive by variant; integer
  matches need `_`. Ranges `0..=3 =>` and alternatives `A | B =>` allowed.
- `if c { a } else { b }` as an expression, both branches required.
- Bundle literal, array literal `[0_u8; 6]` or `[a, b, c]`.

There is no operator overloading and no user-defined infix.

## 6. Statements

Statements appear in `rtl` bodies, `model` handlers, unit bodies, routines
and `isa` semantics. Not every statement is legal in every context; the
table in each section says which.

- `let name: T = expr;` binds a local **wire** (combinational value). Type
  may be omitted when inferable from `expr`. Immutable; it is a name for a
  value, not storage.
- `name = expr;` **drives** a wire declared as an output or `wire` of the
  enclosing body. May appear more than once; the **last assignment on the
  executed path wins** within an evaluation (Verilog `always @*`
  semantics). Every output must be driven on every path, either by a
  `default` or by assignment; the compiler checks this.
- `name <- expr;` **schedules** a register's next value, committed at the
  clock edge. Last write on the executed path wins. The compiler warns when
  one `<-` unconditionally shadows another.
- `mem.write(addr, value);` schedules a memory write. `mem.read(addr)` is an
  expression.
- `if c { } else if c2 { } else { }`.
- `match e { arms }` as a statement; arms are blocks.
- `for i in 0..N { }` with constant bounds; unrolled. No other loops exist
  outside routines.
- `assert(cond);` / `assert(cond, "message");`. Checked in both machine
  classes; a failure is a simulation fault with location.
- `unreachable("msg");`.

## 7. Units

```
/// Parity over the low byte. The 286 parity-checks only bits 0..7, even for
/// 16-bit results.
unit Parity8(v: u8) -> u1 {
    return ~(v[0] ^ v[1] ^ v[2] ^ v[3] ^ v[4] ^ v[5] ^ v[6] ^ v[7]);
}

unit AluFlags<const W: width>(op: AluOp, a: u<W>, b: u<W>, raw: u<W + 1>) -> Flags {
    let r: u<W> = raw as u<W>;
    let msb = W - 1;
    let (cf, af, of) = match op {
        AluOp::Add | AluOp::Adc => (raw[W], raw[4] ^ a[4] ^ b[4], ~(a[msb] ^ b[msb]) & (a[msb] ^ r[msb])),
        AluOp::Sub | AluOp::Sbb => (raw[W], raw[4] ^ a[4] ^ b[4],  (a[msb] ^ b[msb]) & (a[msb] ^ r[msb])),
        _ => (0, 0, 0),
    };
    return Flags { cf, pf: Parity8(r as u8), af, zf: (r == 0) as u1, sf: r[msb], of };
}
```

A unit is a pure function. It has no state, no clock, no side effects. It may
call other units. Recursion is forbidden. A single return value is written
`-> T`; several are a named tuple `-> (result: u<W>, raw: u<W + 1>)` and set
with `return (result: x, raw: y);` or by driving the names.

Units are the main sharing mechanism between levels: the same `AluFlags<8>` is
called from the ISA semantics (emulator) and from the execute unit's `rtl`
(simulator). Generated code is one Rust function either way.

## 8. Components

```
component Prefetcher {
    ports {
        in  control: PrefetchControl;
        out output: PrefetchOut;
        bus mem: I286LocalBus::Master;
    }
    rtl { ... }
    model { ... }
}
```

### 8.1 Ports

- `in name: T;` / `out name: T;`: wires crossing the component boundary.
- `bus name: Protocol::Role;`: attachment to a bus (section 10) in a role.
  A bus port is a bundle of wires at the rtl level and a transaction
  interface at the model level; the compiler provides both views.
- `clock name;`: a named clock the component needs. A component with one
  clock may omit this and gets an implicit `clk`. A component with several
  clock domains (a VGA has at least the bus clock and the pixel clock) names
  each, and every register, memory port and routine declares which clock it
  belongs to: `reg hcount: u10 @ pixel = 0;`.

### 8.2 The `rtl` body

Contents, in any order:

- `reg name: T [@ clock] = init;` clocked register with reset value.
- `mem ...` memories (section 4.4).
- `wire name: T;` an internal wire driven by statements in the body.
- `default { out_name = ...; }` default drives for outputs and wires.
- `instance name: Component<params> [@ clock] { port_in = expr, ... };`
  sub-component instances. Inputs are driven by expressions; outputs are read
  as `name.port`. Bus ports are connected in the enclosing `system` or by
  `bus_port = parent_bus_port` here.
- Top-level statements: the body's combinational logic, evaluated every cycle
  of the component's clock (or, for multi-clock components, grouped under
  `on clock_name { ... }` blocks; registers may only be written in the block
  of their own clock).
- `routine` declarations (section 9) and the `sequencer` that runs them.
- `architectural { ... }` for processors: the mapping from rtl state to ISA
  state (section 11.5).

Evaluation per clock edge of a domain: all `on` blocks of that domain
evaluate once as combinational logic, in dependency order determined by the
compiler (a wire may not depend on itself in the same cycle; the compiler
rejects combinational loops). Then all scheduled `<-` and `mem.write`
commit. Wires that cross clock domains must be read through `sync(wire)`
(registers it into the reading domain, one cycle of that domain) or declared
`async in` in the ports, in which case the reader sees the last value the
writer committed. `[open]` whether to model metastability at all; proposal:
no.

### 8.3 The `model` body

The transaction level. Contents:

- `state name: T = init;` plain state (no clock semantics).
- `mem` as plain arrays.
- Handlers:
  - `on bus_port.read(addr: u<A>, size: Size) -> (data: u<D>, wait: u8) { ... }`
  - `on bus_port.write(addr, size, data) -> (wait: u8) { ... }`
  - `on tick @ clock_name (n: u32) { ... }` called with the number of cycles
    of that clock that elapsed since the last call; the emulator chooses the
    granularity (often "once per instruction" or "once per scanline"). The
    `@ clock_name` is omitted in a single-clock component.
  - `on reset { ... }`
  - `on bus_port.signal(name, value) { ... }` for non-transaction wires (IRQ
    acknowledges, DMA requests, reset).
- `emit bus_port.irq(n);` and other transaction initiations, including
  `let v = bus_port.read(addr, size);` from a master.
- Ordinary statements plus `while`/`for` with dynamic bounds (models are
  software; they may loop).

The model's return of `wait` cycles is how the emulator accounts time on the
bus. A component with a `model` body but no `rtl` body can still take part in
the accurate simulator: the compiler generates an rtl **adapter** that
implements the bus protocol's slave (or master) side as a state machine and
calls the handlers at the protocol-defined data phase, inserting `wait`
cycles. The adapter is exact with respect to the protocol and approximate
with respect to the device's internal timing; the compiler reports which
components in an accurate build are adapted.

A component with an `rtl` body but no `model` body runs its rtl in the
emulator profile. This is correct and slow; the compiler reports it.

### 8.4 Processors

A processor component declares its ISA:

```
component I286 {
    isa I286Isa;
    ports { bus local: I286LocalBus::Master; in intr: bool; in nmi: bool; ... }
    rtl { ... architectural { ... } }
    model { ... }     // optional overrides; the interpreter itself is derived
}
```

Its `model` is derived from the `isa`: an interpreter (and later a dynamic
recompiler) over the architectural state, issuing bus transactions through
the component's bus port. The author may add a `model` body for things the
ISA cannot express: prefetch-queue visible effects, timing tables,
bus-interface quirks.

## 9. Routines: multi-cycle control inside `rtl`

Microcode, bus sequencers and video timing generators are all "do this, then
next cycle do that, wait here until ready." Machine has one construct for it.

```
routine mem_read8(addr: u20) -> (data: u8) @ clk {
    cycle { bus.request <- BusRequest { valid: 1, address: addr, op: BusOp::Read, size: BusSize::U8, data: 0 }; }
    wait until bus.response.done;
    cycle { return bus.response.data as u8; }
}
```

- A routine runs on one clock. Its body is a sequence of **steps**:
  - `cycle { statements }`: exactly one clock cycle. Statements inside are
    ordinary rtl statements and may drive wires and schedule registers.
  - `wait until cond;`: zero or more cycles until `cond` holds at a clock
    edge. Wires driven in the preceding `cycle` are not held; use registers
    to hold outputs stable across waits (as real hardware does).
  - `let x = call other_routine(args);`: runs the callee to completion; takes
    as many cycles as it takes. Calls are statically nested (no recursion);
    the compiler allocates return slots.
  - `let m = fetch Operand;`: processor-specific sugar (section 11.3).
  - `if`/`match` over steps, `loop { ... break; }` and `for` with dynamic
    bounds: control flow between steps.
  - `return value;` ends the routine.
- Routine arguments and locals are registers allocated by the compiler in
  the sequencer's scratch state. Their lifetime is the routine activation.
- A `sequencer` declaration names the routines that may be started from
  outside the routine graph and gives the dispatch rule:
  ```
  sequencer execute @ clk {
      idle   => dispatch(decode_table, prefetch.output.byte);   // starts a routine by opcode
      fault  => call fault_entry(fault_code);
  }
  ```
  `[open]` exact form; the sample shows one concrete proposal.

The compiler lowers routines to an explicit state machine (state register,
scratch registers, next-state logic). The result is ordinary rtl and
simulates like any other. The routine graph is also the compiler's source for
**derived instruction metadata** (section 11.4) and for **timing derivation**
(section 14).

Writes in routines may be marked for fault semantics, following the i286
rtl's needs:

```
cycle { AX <- result @commit(on_retire); }   // staged, committed when the routine retires
```
Default commit policy is `on_issue`. A routine that executes `fault(code)`
discards all `on_retire` writes of the current activation. This is how
instruction restart after a fault is modelled without the author hand-writing
rollback.

## 10. Buses

A bus is a protocol: shared wires, the roles that may drive them, and the
transactions that occur on them, each defined as a wire-level sequence from
every role's point of view. From this single definition the compiler derives:

- the wire bundles for each role's `bus` port (rtl view);
- the transaction interface for each role (model view);
- the adapters in both directions;
- bus tracing and the "bus analyser" debugger view.

```
bus IsaBus {
    clock clk;                       // BCLK
    params { const ADDR: width = 20; const DATA: width = 8; }

    wires {
        addr:    u<ADDR>  driven_by Master;
        data:    u<DATA>  shared;              // tristate: exactly one driver per cycle, else contention fault
        ale:     bool     driven_by Master;
        memr:    bool     driven_by Master  active_low;
        memw:    bool     driven_by Master  active_low;
        ior:     bool     driven_by Master  active_low;
        iow:     bool     driven_by Master  active_low;
        iochrdy: bool     wired_and         active_low;   // open-collector: any slave can pull low
        irq:     [bool; 11] driven_by Slave;            // one driver per line, assigned in the system
        // dma request/acknowledge, refresh, reset, ... omitted in this draft
    }

    roles { Master; Slave; }

    enum Space : u1 { Mem, Io }

    /// One 8-bit read. Default 4 BCLK cycles for I/O, 6 with the default wait state
    /// inserted by the system board; slaves extend with IOCHRDY.
    transaction read(space: Space, addr: u<ADDR>) -> (data: u<DATA>) {
        Master {
            cycle { ale = true; addr = addr; }
            cycle { ale = false; if space == Space::Mem { memr = true; } else { ior = true; } }
            cycle { }                                  // minimum command width
            wait until iochrdy;
            cycle { return data; }                     // sample data bus
            cycle { memr = false; ior = false; }
        }
        Slave {
            // A slave sees the wires; the protocol says when it must respond.
            wait until (memr || ior) && selected(space, addr);
            respond(data, wait: n)  { iochrdy = false for n cycles; data = value during the sample cycle; }
        }
    }
    transaction write(space: Space, addr: u<ADDR>, data: u<DATA>) { ... }
    transaction interrupt(line: u4) { Slave { irq[line] = true; } }
}
```

`[open]` The `Slave` side notation above is a sketch. The requirement is: a
slave's **model** implements `on isa.read(space, addr) -> (data, wait)` and
the compiler must be able to generate the rtl that drives `data` and
`iochrdy` at the right cycles. That means the protocol must say, for each
role, (a) which wire pattern begins a transaction, (b) at which cycle the
response is sampled, and (c) how a slave stretches the transaction. The
draft's `respond(...)` is the placeholder for that information.

Address decoding (`selected`) is a system-level fact: the `system` assigns
address ranges to slave ports, and the compiler generates the decoder for
both levels.

Bus-level timing in the emulator: a master model's `let v = isa.read(...)`
returns after the slave's handler runs, and the bus layer charges the
protocol's base cycle count plus `wait`. This is how a 386 instruction's
duration in the emulator includes correct ISA wait states without the
emulator simulating the bus wires.

## 11. ISA: the architectural level

```
isa I286Isa {
    space mem: addr u24 -> u8, little_endian;
    space io:  addr u16 -> u8, little_endian;

    register AX: u16 { AL: u8 @ 0; AH: u8 @ 8; }
    register FLAGS: u16 { CF: u1 @ 0; PF: u1 @ 2; AF: u1 @ 4; ZF: u1 @ 6; SF: u1 @ 7; TF: u1 @ 8; IF: u1 @ 9; DF: u1 @ 10; OF: u1 @ 11; }
    register CS: Segment;         // bundle type: selector + cached descriptor
    register IP: u16;
    mode Mode : u1 { Real, Protected }   // a register of a special kind; `mode` is readable in semantics
    ...
}
```

### 11.1 Address translation and faults

```
fault GP(selector: u16) { vector: 13, class: Fault, pushes_error_code: true }
fault UD { vector: 6, class: Fault }

/// Segment-relative access. Real mode: no checks. Protected: limit and rights.
fn translate(seg: Segment, off: u16, size: u8, access: Access) -> u24 {
    if mode == Mode::Protected {
        if !seg.present { raise NP(seg.selector); }
        if off as u32 + size as u32 - 1 > seg.limit as u32 { raise GP(0); }
        ...
    }
    return (seg.base + off as u24) as u24;
}
```

`fn` inside an `isa` is an architectural helper: imperative, may `raise`
faults, may read and write architectural state and address spaces
(`mem.read_u16(addr)`, `mem.write_u8(addr, v)`). `raise` aborts the current
instruction. A fault with `class: Fault` restores IP to the instruction start
and discards architectural writes made by the instruction so far (the
emulator implements this with a write log or by ordering checks before
writes; the rtl implements it with `@commit(on_retire)`). `class: Trap`
commits. This encodes the trap/fault distinction the current `processors`
package carries as separate grammar.

### 11.2 Instructions

```
/// `ADD` performs integer addition on the two operands and stores the result
/// in the first.
instruction Add_rm8_r8 {
    mnemonic "add";
    encoding { opcode 0x00; ModRM { reg: r8 } }   // rm8 is built from the fetched ModRM
    operands (dst: rm8, src: r8);
    flags { modifies: [OF, SF, ZF, AF, PF, CF]; }
    semantics {
        let a = dst;                 // reads through the operand kind (memory, if the form is memory)
        let (result, raw) = Alu<8>(AluOp::Add, a, src, 0);
        dst = result;                // writes through the operand kind
        FLAGS.arith = AluFlags<8>(AluOp::Add, a, src, raw);
    }
    implements add_rm_reg8;        // rtl routine in the component that uses this isa
    references ["Intel 80286 Programmer's Reference Manual, p. B-3"];
}
```

- `encoding` is the byte-level shape: `opcode` bytes and the operands to
  fetch, in order. A fetched operand may bind its fields to **select
  kinds** (`ModRM { reg: r8 }`: the `reg` field selects an `r8`); **from
  kinds** (`rm8 from m: ModRM`) bind implicitly to the fetched operand of
  their source type. Every operand in `operands` must be bound exactly once.
  The assembler, disassembler and decoder are generated from this. When an
  `implements` routine exists, the compiler also **derives** the encoding
  from the routine's `fetch` sequence and reports any mismatch.
- `operands` names operands by **operand kind** (`r8`, `rm8`, `imm16`,
  `moffs16`...). Kinds are declared in the ISA with how they are read and
  written (register select, memory via `translate`, immediate), so reading
  `dst` in semantics means "read through the operand kind" and
  `dst = result;` means "write back through the operand kind's writer",
  including the memory write and its faults. `dst.is_mem` queries the
  kind's `is_mem` rule without reading.
- `semantics` is imperative, over architectural state, operands and units.
  It may call `fn` helpers and `raise`. It has no notion of cycles.
- `flags { modifies, undefined, reads }` is checked against the semantics
  where possible (a flag listed as modified must be written on some path;
  a flag written must be listed).
- `timing { ... }` is optional. Default: derived (section 14). When given,
  it is a cycle count expression over operand shapes, e.g.
  `timing { cycles: if dst.is_mem { 7 } else { 2 } }`, and it overrides.
- `implements` links to the rtl routine for differential testing and
  derivation. Optional; a 386 model may have an `isa` and no rtl at all.
- `mode` restrictions: `allowed_in [Mode::Protected]` and privilege
  `requires cpl == 0`.

### 11.3 Operand kinds and `fetch`

```
operand ModRM {
    size 1;
    fields { mod: u2 @ 6; reg: u3 @ 3; rm: u3 @ 0; }
    /// Extra bytes consumed by the effective-address form.
    tail { if mod == 1 { Disp8 } else if mod == 2 || (mod == 0 && rm == 6) { Disp16 } }
}
```

Operand kinds are declared once in the ISA and used from both levels:

```
/// 8-bit register by 3-bit selector: a select kind.
operand_kind r8 : u8 {
    select(n: u3) => match n { 0 => AL, 1 => CL, 2 => DL, 3 => BL, 4 => AH, 5 => CH, 6 => DH, 7 => BH };
}

/// 8-bit register or memory through ModRM: a from kind.
operand_kind rm8 : u8 from m: ModRM {
    is_mem => m.mod != 3;
    read => if m.mod == 3 { r8.select(m.rm) } else { mem.read_u8(ea(m)) };
    write(v: u8) => if m.mod == 3 { r8.select(m.rm) = v; } else { mem.write_u8(ea(m), v); };
}
```

A select kind names a register by an encoding field; its `select` rule must
yield a *place* (every branch assignable). A from kind is built from a
fetched operand bound to a name (`m`) and has `read`, optionally `write` (a
kind without `write` is read-only, e.g. immediates) and `is_mem`. In the ISA,
`rm8` resolves through `ModRM` to a register or a `translate`d memory
access. In rtl routines, `let m = fetch ModRM;` consumes the operand's bytes
from the instruction stream via the component's `istream` (a routine the
component designates), taking the cycles that costs. Because `fetch`
sequences are typed by the same declarations, the compiler can derive the
instruction's encoding and length from the routine.

### 11.4 Derived metadata

From `isa` + `rtl`, the compiler produces: decode tables (both levels),
assembler syntax tables, disassembler, operand/length tables, flag
read/write sets, documentation (Markdown/JSON), and the conformance harness.
Nothing in this list is written by hand.

### 11.5 Architectural mapping

A processor's `rtl` must say how its state projects onto the ISA so the two
can be compared:

```
architectural {
    AX = ax;  BX = bx;  ...
    IP = execute.ip_of_next_instruction();   // an rtl expression; may differ from internal fetch pointers
    FLAGS = concat(...);
    mode = cr0_pe ? Mode::Protected : Mode::Real;
    at retire;    // when the projection is valid: at each routine retire
}
```

The projection defines "architectural state at instruction boundary" for the
rtl. Conformance testing (section 13) compares it against the derived
interpreter after every instruction.

## 12. Systems, clocks and profiles

```
system PcAt386 {
    clock osc: 14.318_18 MHz;
    clock cpu_clk: 33.333 MHz;
    clock bclk = cpu_clk / 4;             // 8.33 MHz ISA
    clock pixel = 25.175 MHz;

    component cpu: I386 @ cpu_clk;
    component pit: I8254 @ (osc / 12);
    component vga: Vga @ (bclk, pixel);
    component ram: Dram<0x100000> @ cpu_clk;

    bus local: I386LocalBus @ cpu_clk { master cpu.local; slave ram.port @ mem 0x00000..=0xFFFFF; }
    bus isa: IsaBus @ bclk {
        bridge local;                      // generated bridge component local -> isa
        slave vga.isa @ io 0x3B0..=0x3DF, mem 0xA0000..=0xBFFFF;
        slave pit.isa @ io 0x40..=0x43;
        irq 0 = pit.out0;
    }
    wire cpu.intr = pic.intr;

    profile accurate { default: rtl; }
    profile fast     { default: model; vga.crtc: rtl; }
    profile debug    { default: model; cpu: rtl; }
}
```

- Every clock is a frequency or a rational division of another clock. Time
  in the simulator is an integer count of a base unit chosen by the compiler
  (the largest unit in which every clock period is an integer; 1 ps if none
  fits), and clock edges are scheduled on that time line. Where all clocks
  are rational multiples of one another the compiler emits a static
  schedule; otherwise an event queue.
- Profiles select the level per component or per sub-component path.
  `default` applies to everything not named. The compiler reports, for a
  profile, which components run adapted and which run rtl inside a `model`
  build.
- The system also binds bus roles, address ranges and interrupt lines, and
  the compiler generates address decoders for both levels.

## 13. Verification

Built into the language and the compiler:

1. **Static**: type and width checking; exhaustive `match`; every output
   driven on every path; no combinational loops; clock-domain discipline;
   memory port limits; routine recursion; flag set consistency.
2. **Assertions**: `assert` in all bodies, kept in both classes (a build flag
   strips them for release).
3. **Tests in the language**:
   ```
   test "ADD AL, BL sets carry" on I286Isa {
       given { AL = 0xFF; BL = 0x01; }
       run instruction Add_rm8_r8 { dst: AL, src: BL };
       expect { AL == 0x00; FLAGS.CF == 1; FLAGS.ZF == 1; }
   }
   test "prefetcher fills to six bytes" on Prefetcher.rtl {
       drive { control.pop = false; mem.wait = 0; }
       run 20 cycles of clk;
       expect { output.count == 6; }
   }
   ```
4. **External vectors**: the existing single-step JSON vectors are a
   first-class test input: `vectors "tests/i286/*.json" on I286Isa;`.
5. **Conformance (differential)**: for every component with two levels, the
   compiler generates a harness that runs both from the same initial state
   and stimulus and compares the architectural projection (processors) or
   the transaction trace (devices) at every instruction/transaction
   boundary. For a processor this is: feed the same instruction stream to
   the derived interpreter and to the rtl, compare `architectural` after
   each retire. Mismatches are reported with the instruction, the state
   diff and the cycle.
6. **Timing conformance**: when a `timing` override is given, compare it
   against the measured rtl cycle count over the vector corpus.

## 14. What the compiler derives

| Derived | From | Into |
|---|---|---|
| Fast interpreter | `isa` | emulator |
| Decode tables, lengths, operand shapes | `isa.encoding`, cross-checked with routine `fetch` sequences | both, assembler, disassembler |
| Instruction cycle counts | symbolic execution of the `implements` routine (count `cycle`s, sum bus transaction base costs; `wait` becomes a runtime bus cost), or **measurement** of the rtl over the vector corpus at build time | emulator timing tables |
| Transaction view of a bus | `bus` wire protocol | model ports |
| Protocol adapters | `bus` + a component's `model` handlers | accurate build |
| Address decoders | `system` ranges | both |
| Documentation | `///`, `references`, derived tables | docs site, debugger |
| Conformance harnesses | any component with two levels | tests |

Measured timing is the honest fallback where symbolic derivation gives up
(data-dependent loops such as multiply/divide, which the compiler reports as
"data-dependent; measured").

## 15. Toolchain

A Rust workspace in `hdl/`:

| Crate | Role |
|---|---|
| `machine-syntax` | lexer, parser, concrete and abstract syntax, formatter, diagnostics |
| `machine-sema` | name resolution, const evaluation, monomorphisation, type and width checking, clock-domain checking, the typed IR |
| `machine-rtl` | routine lowering to state machines, combinational scheduling, the cycle-accurate code generator (emits Rust) |
| `machine-model` | ISA-to-interpreter derivation, model code generator, bus layer, adapters (emits Rust) |
| `machine-derive` | decode tables, assembler/disassembler tables, timing derivation, docs |
| `machine-verify` | test runner, conformance harness generation |
| `machine-runtime` | library the generated code links against: clocks and scheduler, bus tracing, fault reporting, state snapshot/diff |
| `machine-cli` | `machine build|check|fmt|test|doc` |

Generated code is Rust source, compiled by `cargo` into a crate per
`system` × `profile`. Native first; `wasm32` with TypeScript bindings is a
later layer over the same generated crates. The IR is kept clean enough that
a JIT backend (dynamic recompilation of the derived interpreter) can be added
without touching the front end.

Diagnostics are emitted as JSON (`--message-format=json`) as well as text,
with stable error codes, so agents and editors can act on them.

## 16. Open questions

1. **Bus protocol notation for the slave side** (section 10). Needs a
   worked example with a real ISA slave before the syntax is fixed.
2. **Sequencer/dispatch syntax** (section 9). The sample proposes one form.
3. **Tristate and contention**: model as a fault (strict) or as a
   configurable resolution (`pull_up`, `last_driver`)? Proposal: fault by
   default, with per-wire resolution allowed for buses that are documented
   to rely on it.
4. **Register write-conflict policy**: last-wins with warning (proposed) vs
   hard error.
5. **Timing derivation scope**: which routine shapes are symbolically
   derivable vs measured. Decide after the 286 corpus is ported.
6. **Memory initialisation from files** for ROMs: `mem bios: [u8; 0x10000] { init: file("bios.bin") }` is the obvious form; question is whether ROM contents are part of the description or of the system build.
7. **Metastability / async inputs**: proposal is to model none of it.
8. **Name of the language.** "Machine" with `.machine` files, as the
   exemplar used. Easy to change before any code exists.
