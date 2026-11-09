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
| **model fn** | An imperative helper inside a `model` body: reads and writes the component's state, may `emit` and start transactions. Also the host's way into a device model (section 8.3a). |
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

A block in value position (the branches of an `if` expression, a `match`
arm, a `{ ... }` expression) has the value of its final expression, and a
final `if`/`else` or `match` statement counts as one: `{ if a { 0 } else
{ 1 } }`. The expected type flows into that final expression, so bare
literals there take their width from context.

There is no operator overloading and no user-defined infix.

## 6. Statements

Statements appear in `rtl` bodies, `model` handlers, unit bodies, routines
and `isa` semantics. Not every statement is legal in every context; the
table in each section says which.

- `let name: T = expr;` binds a local **wire** (combinational value). Type
  may be omitted when inferable from `expr`. Immutable; it is a name for a
  value, not storage.
- `let mut name: T = expr;` binds a **mutable local**: a name whose value
  the body may replace with `name = expr;` (section 6.1).
- `place = expr;` **assigns** a place: drives a wire declared as an output
  or `wire` of the enclosing body, writes architectural state in `isa`
  code, writes `state` or a `mem` element in a model, or replaces a
  mutable local. May appear more than once; the **last assignment on the
  executed path wins** within an evaluation (Verilog `always @*`
  semantics). Every output must be driven on every path, either by a
  `default` or by assignment; the compiler checks this. What counts as a
  place is given in section 6.2.
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

### 6.1 Mutable locals

```
let mut acc: u8 = 0;
for i in 0..8 {
    if irr[i] == 1 && imr[i] == 0 { acc = acc | (1 << i); }
}
```

A mutable local is a name whose value changes along the executed path. It
is legal in every body: in a unit, a model handler, a model fn or isa code
it is an ordinary variable; in an `rtl` body or a routine it is a chain of
multiplexers, the way blocking assignments in `always_comb` are. The same
rules hold at every level, so a unit written with one means the same in
the emulator and in the simulator.

- **Declaration.** `let mut name[: T] = init;` or `let mut name: T;`
  without an initial value. The type is the initialiser's when omitted;
  a bare literal has no runtime width, so `let mut n = 0;` is an error
  (E1004) and is written `let mut n: u8 = 0;`. `let mut` with a
  destructuring pattern is not allowed (E0108).
- **Assignment.** `name = expr;` with `expr` of the local's type (no
  implicit widening, as everywhere). A field or element of a mutable
  local of bundle or array type is assignable too: `ch.count = v;`,
  `bytes[i] = b;`.
- **Last assignment wins** on the executed path. Within a `match` arm or
  an `if` branch the assignment is seen only on that branch.
- **Assigned before read.** Reading a mutable local on any path where it
  may not have been assigned is an error (E1051). A `let mut x: T;`
  without initialiser must therefore be assigned in every branch before
  the first read after the branches join. There is no default value.
- **Scope.** A local, mutable or not, is visible to the end of the
  enclosing block. A `let` or `for` local may not share a name with a
  port, wire, `state`, `mem`, register, register field or view, latch,
  mode or address space visible in the body (E1001), so `x = expr;`
  never has two meanings. Parameters are named in signatures and are
  exempt. Shadowing an outer local of the same body is allowed.
- **Not storage.** A mutable local lives within one evaluation. In a
  routine it may not be read after a `cycle` or `wait until` that follows
  its last assignment (E1052: "use a `reg` to carry a value across a
  cycle"; checked once rtl bodies are). In rtl it may not be scheduled
  with `<-` and is not a clock domain member.
- **Assigning an immutable local**, parameters included, is an error
  (E1050), with a note to declare it `let mut` or copy the parameter
  into one.
- **Units stay pure.** A mutable local never escapes its unit; a unit
  with mutable locals is still a pure function and still inlinable into
  rtl.

The compiler lowers a mutable local to a fresh value per assignment
(static single assignment); the model and isa backends emit a Rust
`let mut`, the rtl backend a multiplexer per join point.

### 6.2 Places

A **place** is what may stand on the left of `=`. Places are:

| Place | Where |
|---|---|
| an output port or `wire` of the body | rtl |
| an architectural register, register field, register view, the mode register, a latch | isa |
| a space cell `mem[addr]`, an operand `dst` (through its kind's `write` rule) | isa |
| a `state` of the model | model |
| a mutable local | everywhere |
| `p.field` where `p` is a place of bundle type | everywhere |
| `p[i]` where `p` is a place of array type, or a `mem` | everywhere |
| a `match` or `if` expression every arm of which is a place | isa (operand selection) |

Indexing nests: `ch[c].count = v;` writes one field of one element of a
`state ch: [Channel; 3]`, and `line[x][y] = ...` is legal for arrays of
arrays. The index of a dynamic `p[i]` is any unsigned expression; an
index outside the array is a checked error, like division by zero (in
rtl the compiler additionally requires the index type to fit the array,
so that no check is needed in hardware). Fields of a `mem` element are
assignable when the element type is a bundle: `mem ch: [Channel; 3]` is
written `ch[c].count = v;` exactly like `state`.

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
call other units and use mutable locals (section 6.1), which do not make it
impure. Recursion is forbidden. A single return value is written
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

- `state name: T = init;` plain state (no clock semantics). `T` may be a
  bundle or an array, including an array of bundles; elements and fields
  are assignable (section 6.2): `state ch: [Channel; 3];` then
  `ch[c].count = v;`. Without `= init` the state starts at the type's
  zero (`false`, zeroed bundles).
- `mem name: [T; N];` as plain arrays. `mem` is for large or
  host-visible arrays (video RAM, a ROM, a framebuffer): it is not copied
  with the component's state snapshot, and the generated struct exposes
  it for the embedding program to fill or read. For a handful of channels
  prefer `state`. Both are indexed and assigned the same way.
- `mem cells: [u8; N] { direct: port(addr & MASK), wait: W };` states that
  the slave bus port `port` reads and writes this memory plainly. The
  byte at bus address `addr` is `cells[addr & MASK]` (or `cells[addr]`
  for `port(addr)`), and each transfer takes `W` wait states. `read_only`
  means writes still go through the handlers (a ROM). Like `istream {
  window }`, it is a fact the author states, not something checked
  against the handlers: an emulation profile (`memory: direct`) relies on
  it to bypass them.
- `fn name(params) -> T { ... }` helpers over the component's state
  (section 8.3a).
- `port.clocks(n)` (a `u32`): `n` periods of the component's own clock,
  in clocks of the bus on `port`, rounded up. A slave whose readiness
  follows its own clock rather than the bus's turns a delay in its dots
  into the bus `wait` it returns with this. The CGA uses it for the slot
  it gives the CPU. A context with no bus attached returns `n`.
- Handlers:
  - `on bus_port.read(addr: u<A>, size: Size) -> (data: u<D>, wait: u8) { ... }`
  - `on bus_port.write(addr, size, data) -> (wait: u8) { ... }`
  - `on tick @ clock_name (n: u32) { ... }` called with the number of cycles
    of that clock that elapsed since the last call; the emulator chooses the
    granularity (often "once per instruction" or "once per scanline"). The
    `@ clock_name` is omitted in a single-clock component. With `-> u32`
    it returns the most cycles that may pass before it must run again
    (tick scheduling, section 12): the timer's cycles to its next output
    edge, a video card's dots to the end of the frame, `0xFFFFFFFF` for
    nothing pending.
  - `on reset { ... }`
  - `on bus_port.signal(name, value) { ... }` for non-transaction wires (IRQ
    acknowledges, DMA requests, reset).
  - `on in_port(value: T) { ... }` when the level driven into an `in` port
    changes (delivered after the instruction, like signal edges).
- Wire ports at the model level: an `in` port reads as the current level
  of whatever the system wires to it (the type's zero when nothing is);
  an `out` port is assigned like state (`int = req;`), takes the new
  level at once, and queues an edge for the ports it drives when the
  value changes. A pin that belongs to a chip rather than to a bus (an
  8259's INT, SP/EN and CAS lines, a processor's INTR) is a wire port.
- `emit bus_port.irq(n);` and other transaction initiations, including
  `let v = bus_port.read(addr, size);` from a master.
- `bus_port.signal(args)` as an expression reads the signal's **current
  level** (a `bool`), in any role. Edges are delivered to `on` handlers
  after each instruction; a model that must see a level change sooner
  reads it, as a DMA controller reads DREQ between transfers of a burst.
- `decline;` in a transaction handler: this responder does not drive the
  data, and the transaction goes on (section 10). It ends the handler like
  `return`. Anywhere else it is an error (E1054).
- Ordinary statements plus `while`/`for` with dynamic bounds (models are
  software; they may loop).

A transaction handler runs inside the initiator's transaction. It may
`emit` signals, and it may start transactions on the component's
master-role ports, directly or through a model fn: that is a **bridge**,
whose transaction nests inside the one it answers and takes its time on
its own bus (the AT's bus controller answers on the processor's local bus
and runs the cycle on the ISA bus). A device that masters a bus on its own
account does so from `on tick`.

### 8.3a Model functions

```
// ports { bus port: LocalBus::Slave; }
model {
    state ch: [Channel; 3];

    /// Set a channel's OUT pin; channel 0 is wired to IRQ 0.
    fn set_out(c: u2, level: bool) {
        ch[c].out = level;
        if c == 0 { emit port.interrupt(0) = level; }
    }

    /// Host input: a key went down or up.
    fn key(scancode: u8, down: bool) { ... }

    on tick(n: u32) { ... set_out(c, false); set_out(c, true); ... }
}
```

A model fn is the model-level counterpart of an isa `fn`: imperative,
over the body's own state, with the body's effects. It exists because
device models repeat themselves (the 8254's three channels, the 8237's
four) and because a device needs an entry point for things that are not
bus traffic (keys, incoming serial bytes, a disk image). Units remain the
sharing mechanism between levels; a model fn is for logic that touches
state or has effects and so cannot be a unit.

- **Declaration.** `fn name(p: T, ...) [-> R] { body }` anywhere in a
  `model` body, in any order with `state`, `mem` and handlers. Parameters
  are immutable locals; the body may declare mutable ones. `-> R` is a
  single type or a named tuple, as for units; the body must `return` a
  value on every path (E1015).
- **Access.** A model fn sees the component's ports, `state` and `mem`,
  calls units and other model fns of the same component, and may
  `emit` and start transactions on the component's master-role ports.
  It may not be called from a `unit`, an `rtl` body or another
  component, and not from a `state` initialiser.
- **Effects are transitive.** The compiler computes, for every model fn,
  whether it (or any fn it calls) starts a transaction. Signal `emit`s
  are allowed anywhere.
- **No recursion**, direct or mutual (E1053), as for units and isa fns.
  Every call graph in the language is finite, which the adapter and
  timing derivations rely on.
- **Lint W1005.** A model fn that reads or writes no state, performs no
  effect and calls no fn with effects could be a `unit`; the compiler
  warns, because a unit is shared with the rtl level and a model fn is
  not. The warning names the fn and is silenced by making it a unit.
- **Generated form.** Each fn becomes a public method of the component's
  struct, prefixed like isa fns so it cannot collide with the
  constructor or the handlers, taking the same ports context its
  handlers take: `pub fn fn_set_out<P: Pit8254Ports>(&mut self, ctx:
  &mut P, c: u8, level: bool)`. Calls inside the body pass the context
  through. The generated `System` offers `with_<instance>(|dev, ctx|
  ...)`, which catches the device up, runs the closure with a live
  context and delivers the signal edges it raised, so the embedding
  program can call a device's fns: `sys.with_console(|c, ctx|
  c.fn_clear(ctx))`. This is how a host injects input. There is no
  visibility keyword yet: every model fn is public in the generated
  crate.
- **Processor models** (section 8.4) have model fns too, for example a
  byte-lane helper shared by the space handlers.

In an `rtl` body a `fn` would be inlined, as a Verilog function is. It is
left out until the rtl slice. A `fn` inside a handler, a unit or isa
semantics is an error; in an rtl body it is unchecked like the rest of
the body.

A model's observable behaviour is its **transaction trace** (the
transactions it initiates and the data and `wait` it returns) and the levels
of the signals it emits, both taken at transaction boundaries. The emulator
chooses how often `on tick` runs and a model may catch up in one step, so a
model is never compared with the rtl cycle by cycle: conformance (section
13) compares the two at transaction boundaries and signals at those
boundaries, optionally within a tolerance of cycles.

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

### 8.3b Memoised functions

```
/// Draw scanline `line` into `frame`.
memo fn render(line: u32) { ... }
```

A `memo fn` is a model fn whose call can be skipped. When the last call
with the same arguments read state that hasn't changed since, the call is
skipped, and what it wrote then still stands. This is how a display card
draws only the lines whose memory, registers or blink phase changed,
without changing a single dot. The rules (E1057):

- It returns nothing and takes integer or `bool` arguments.
- It, and every fn it calls, touches no port: no transactions, no
  signal levels, no `emit`.
- What it writes (its outputs) it doesn't read, and nothing else in the
  model writes them. Calls with different arguments are taken to write
  different cells (one scanline each).

The compiler finds the state and memories a memo fn reads, transitively.
Every write to them, from any handler or fn, stamps a version: a whole
state assigned a new value, or the 64-cell block of a memory written. A
call runs a recording copy of the fn that notes each versioned state and
memory block it reads. A later call with the same arguments is skipped
while none of those has a newer version. Reads are recorded where they
happen, so a read whose address depends on data (a text mode's glyph,
found by its character code) is recorded as precisely as any other.
Blocks may be recorded a little wider than read, which can only cost a
redraw. With no write to versioned state since a call (the component's
version unchanged), it is skipped without looking at what it read; a call
found unchanged counts as made at the current version, so after a write
elsewhere each call is looked at once.

The embedding program writes `mem`s directly, outside the versioning,
so after writing state a memo fn reads it calls `memo_clear()` on the
model.

For every component, the compiler also reports what writes each piece of
state: `reset`, `tick`, `bus <port>`, `signal <port>`, the processor, or
`memo`. Nothing at all means it is the host's. The report is a comment
on the generated model struct, and shows what is configuration (written
by I/O), content (by memory transactions), time (by `on tick`) and
derived (by memo fns).

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

The model body says how the isa's address spaces reach the buses, with one
handler per access width:

```
model {
    on mem.read_u8(addr: u24) -> u8 {
        return local.read(space: Space::Mem, addr: addr, size: BusSize::Byte).data as u8;
    }
    /// An aligned word is one transfer; an odd address takes two.
    on mem.read_u16(addr: u24) -> u16 {
        if addr[0] == 0 { return local.read(space: Space::Mem, addr: addr, size: BusSize::Word).data; }
        let lo = local.read(space: Space::Mem, addr: addr, size: BusSize::Byte).data as u8;
        let hi = local.read(space: Space::Mem, addr: addr + 1, size: BusSize::Byte).data as u8;
        return concat(hi, lo);
    }
    on mem.write_u8(addr: u24, data: u8) { ... }
    on io.read_u8(addr: u16) -> u8 { ... }
    ...
}
```

`on <space>.read_uN(addr: A) -> uN` and `on <space>.write_uN(addr: A,
data: uN)` take the space's address type; N is a multiple of the cell
width. Every space needs at least the one-cell read and write (E1041); an
access of a width without a handler is composed from cells in the space's
byte order. This is where bus sizing and alignment live, and with them
the bus time an access costs.

`on <space>.fetch_uN(addr) -> uN` binds instruction-stream reads of the
istream's space (code bus cycles; the 286's COD status) separately from
data reads; without one they use the read handler. The model build times
fetches separately, which is what lets execution overlap them; the i286
board's processor fetches aligned words into a one-word buffer, as its bus
unit does.

The model also supplies every `extern fn` of the isa with a handler of the
same name and signature (E1041), typically from signals and transactions,
and may factor its handlers through model fns (section 8.3a):

```
state intr_level: bool = false;
on local.intr(value: bool) { intr_level = value; }
on intr() -> bool { return intr_level; }
on acknowledge() -> u8 { return local.inta().vector; }
```

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

The **first role in `roles` initiates** transactions: a model attached in
that role calls `port.read(addr)`; a model in any other role responds with
`on port.read(addr) -> (data, wait)`. A signal is a `bool` wire (or one
element of a `bool` array) named for the transaction level; the role that
drives the wire sets it with `emit port.signal(args) = level;`, the other
roles observe it with `on port.signal(args, value: bool)`. Transaction
returns are named tuples so that the handler's return fields have names.

Address decoding (`selected`) is a system-level fact: the `system` assigns
address ranges to slave ports, and the compiler generates the decoder for
both levels.

Bus-level timing in the emulator: a master model's `let v = isa.read(...)`
returns after the slave's handler runs, and the bus layer charges the
protocol's base cycle count plus `wait`. The base cycle count is the
number of `cycle` statements in the initiating role's body; `wait until`
marks where the responder's `wait` is inserted. It is derived only for a
straight-line body (`cycle`, `wait until`, `let` and assignments); a
transaction whose count cannot be derived cannot be generated in a model
build.

Address decoding reads the transaction parameter named `addr` and, when a
range names a space (`io 0x40..=0x43`), the parameter named `space`, whose
enum variant is matched ignoring case. A slave without ranges answers what
no ranged slave does. A transaction without an `addr` parameter is not
decoded: it is offered to every attached responder that handles it, in
the order the system binds them, until one does not `decline` (an
interrupt acknowledge goes to the interrupt controller; a DMA acknowledge
to the device on that channel, which the others decline). A selected
slave that declines a decoded transaction leaves it unanswered. A bus may declare more than
two roles; every role after the first responds, and a role observes the
signals other roles drive, so an interrupt controller in its own role
watches the devices' `irq` lines and drives `intr`. A bus may have
several masters (`master cpu.local; master dma.dma;`). Transactions are
atomic in the model build, so masters need no arbitration there: a
device masters from its `on tick`, which runs between instructions, and
the cycles it uses are charged to the same time line, which is how DMA
steals time from the processor. HOLD/HLDA arbitration is the rtl
level's business. A read nobody answers returns all ones and a write
nobody answers is dropped; both still take the base cycles. This is how a 386 instruction's
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

### 11.1a The instruction stream

```
fn code_address() -> u24 { return translate(CS, IP, 1, Access::Execute); }
fn next_byte() -> u8 { let b = mem.read_u8(code_address()); IP = IP + 1; return b; }
istream { space mem; address code_address; next next_byte; undefined UD; }
```

`istream` tells the derived interpreter (and the test harness) where
instruction bytes come from: the address space, a `fn` giving the address of
the next byte, a `fn` that consumes one byte, the fault raised for an
opcode that decodes to nothing, and (`fault deliver;`) the `fn (vector:
u8, restart: bool)` that **delivers** a fault. After an instruction raises,
the delivery fn runs with the fault's declared `vector`; `restart` is true
for `Fault`-class faults. Declared as `fn (vector: u8, restart: bool,
has_error: bool, error: u16)`, it is also given the error code of a fault
declared with `pushes_error_code` (its first parameter, or 0): the 286's
protected mode pushes it, real mode does not. State is never rolled back implicitly: the ISA
says what restarting means. On the 286 only IP restarts, from a latch
(`latch start_ip: u16 = IP;` snapshots the instruction start, prefixes
included); registers the instruction already changed stay changed, as the
hardware does (a faulting `movsw` keeps SI advanced). Software
interrupts are ordinary instructions that call the same delivery. Delivery
is described once, in the ISA, because it is architecture: on the 286 in
real mode it pushes FLAGS, CS and IP, clears IF and TF, and loads CS:IP
from the IVT at `vector * 4`; in protected mode it goes through the IDT's
interrupt, trap or task gate, switching stacks for an inner level. Multi-byte operands are assembled from
consecutive bytes least-significant first. Architectural registers reset to
zero, then the isa's `fn reset()` runs if it declares one (it takes no
parameters and returns nothing, E1046): on the 286 it loads CS:IP with
F000:FFF0 and the CS base with FF0000. A `given` block in a test sets what
the test needs and does not run `fn reset()`.

**External interrupts.** What the isa needs from outside the processor it
declares as `extern fn`, without a body; the processor model supplies it
(section 8.4), and the plain-memory interpreter used for tests and
vectors returns the type's zero. `boundary` names a `fn () -> bool` that
runs at every instruction boundary, before the next fetch, and returns
true when it took an interrupt in place of the instruction; taking one
ends `halt`. Whether an interrupt is accepted and how it is delivered are
architecture, so they are written in the isa; the pin and the bus cycles
are not:

```
istream { ...; fault deliver; boundary take_interrupt; }
extern fn intr() -> bool;          // the INTR pin
extern fn acknowledge() -> u8;     // the interrupt-acknowledge cycles: the vector

fn take_interrupt() -> bool {
    if INHIBIT == 1 { INHIBIT = 0; return false; }   // after STI, MOV SS, POP SS
    if FLAGS.IF == 1 && intr() {
        deliver(acknowledge(), false);
        return true;
    }
    return false;
}
```

`double` names the double fault: a fault declared `contributory: true`
raised while a contributory fault is delivered becomes it (on the 286,
#DE, #TS, #NP, #SS and #GP are contributory), any other fault raised
during a delivery is delivered in its place, and a fault while the
double fault is delivered **shuts the processor down**: it halts, an
interrupt does not wake it, only reset does, and the bus is told
(`Bus::shutdown`; a PC board resets the processor, which is how AT
software leaves protected mode). A fault declared `page_fault: true` (the 386's #PF)
makes a third class: a page fault or a contributory fault raised while
it is delivered becomes the double fault, but a page fault raised while
a contributory fault is delivered is delivered in its place.

`limit` names a `fn () -> u32`: how many bytes the stream holds from the
current address before its segment ends. It is timing information only:
a processor model's bus unit does not prefetch past it (section 11.2c).
On the 286, `fn code_room() -> u32 { return 0x10000 - (IP as u32); }`.

`window` names a `fn () -> u32`: how many bytes from the current address
follow at consecutive addresses, because translating them would check
and compute the same things. It is a fact about the architecture that
the interpreter uses for speed. While `next` decodes an instruction, the
interpreter calls `address` once and counts on through the window,
translating again only past it. 0 means every byte is translated. On
the 386 the window is the rest of the segment, and with paging on no
further than the end of the page. The first byte's translation leaves
that page in the TLB, and a read that hits changes nothing:

```
fn code_window() -> u32 {
    if CR0.PG == 0 { return code_room(); }
    let room = code_room();
    let page_room = 0x1000 - ((CS.base + EIP) & 0xFFF);
    return if page_room < room { page_room } else { room };
}
```

`reset` names a `fn () -> bool`, usually an `extern fn` the processor
model answers from its RESET pin. The interpreter asks at every
instruction boundary. While it is true, the processor is held in reset
(halted, its registers at their reset values, and `Bus::processor_reset`
lets the processor model empty its own state). When it falls, the
processor starts as after a reset. On the AT the keyboard controller
pulses it, which is how the BIOS brings the 286 back from protected mode.

`opcode` names a `fn ()` the decoder runs after each opcode byte it
fetches (the bytes of an encoding's `opcode`, and a prefix's), not after
an operand's: the Z80 refreshes memory in its M1 cycles, and R counts them
(`fn m1() { R = (R & 0x80) | ((R + 1) & 0x7F); }`). The trace core records
how many a step made and makes them again when it replays it.

The window must be exact: inside it, `address` would return the
window's start plus the offset, and would raise nothing. Getting it
wrong changes behaviour, not just speed.

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
  fetch, in order. An `opcode` is a byte (`0x88`), a **pattern string**, or
  a list of those. In a pattern, `0`/`1` are fixed bits, `-` is a
  don't-care bit, `_` and spaces are separators, and a letter names a
  **field**: one contiguous run of bits within a byte, bound to an operand
  kind (`opcode "00ooo000" { o: alu_op }`). A fetched operand may bind its
  fields to kinds (`ModRM { reg: r8 }`) or **constrain** them to a value
  (`ModRM { reg: 0 }`); constraints are how group opcodes such as `C6 /0`
  are told apart. Field-bound kinds are **select kinds** (the field names
  a register) or **value kinds** (the field carries a value: an ALU
  operation, a condition code). **From kinds** (`rm8 from m: ModRM`) bind
  implicitly to the fetched operand of their source type. Every operand in
  `operands` must be bound exactly once. No byte sequence may decode to two
  instructions: instructions whose leading opcode patterns overlap must
  fetch the same operand first and constrain one of its fields to different
  values, or differ in a decode condition (section 11.2d), or one must be
  **more specific**: a pattern that fixes strictly more bits and refines
  the other wins on the bytes it matches (the Z80's `HALT`, 76h, inside
  `LD r,r'`, `01dddsss`). Two patterns that meet without either refining
  the other may stand when an instruction more specific than both covers
  where they meet (E1045). The assembler, disassembler and decoder are generated
  from this. When an `implements` routine exists, the compiler also
  **derives** the encoding from the routine's `fetch` sequence and reports
  any mismatch.
- `mnemonic` is a string, or a string-valued expression over the operands
  for a family: `mnemonic alu_name(op);` with `fn alu_name(op: AluOp) ->
  str`.
- `operands` names operands by **operand kind** (`r8`, `rm8`, `imm16`,
  `moffs16`...). Kinds are declared in the ISA with how they are read and
  written (register select, memory via `translate`, immediate), so reading
  `dst` in semantics means "read through the operand kind" and
  `dst = result;` means "write back through the operand kind's writer",
  including the memory write and its faults. `dst.is_mem` queries the
  kind's `is_mem` rule without reading.
- `semantics` is imperative, over architectural state, operands and units.
  It may call `fn` helpers and `raise`. It has no notion of cycles.
- `flags { modifies, undefined, reads }` lists register fields; an entry
  may be conditional (`undefined: [AF if is_logic(op)]`, `[OF if (CL &
  0x1F) != 1]`). Conditions are evaluated in the state *before* the
  instruction executes.
  Hardware vectors are compared with exactly the flags the executed
  instruction leaves undefined masked out. (Checking `modifies` against
  the semantics is planned.)
- `timing { ... }` is optional. Default: derived (section 14). When given,
  it is a cycle count expression over operand shapes, e.g.
  `timing { cycles: if dst.is_mem { 7 } else { 2 } }`, and it overrides.
  It may instead come from a timing table (section 11.2b), usually in a
  file of its own; giving both is an error (E1055).
- `implements` links to the rtl routine for differential testing and
  derivation. Optional; a 386 model may have an `isa` and no rtl at all.
- `mode` restrictions: `allowed_in [Mode::Protected]` and privilege
  `requires cpl == 0`.
- `lock;` or `lock if <cond>;` (a condition over the operands): the
  instruction may follow a lock prefix, when the condition holds. With
  `istream { lock <latch>; }` naming the latch the lock prefix sets, any
  other instruction after a lock prefix raises the istream's `undefined`
  fault (the 386 accepts LOCK only on read-modify-write forms with a
  memory destination: `lock if dst.is_mem && op != AluOp::Cmp;`).

**Repeat prefixes.** `repeat when <cond> skip <cond> next <cond> [while
<cond>] [interruptible];` says what a repeat prefix does to an
instruction whose semantics are one element (an x86 string instruction):
while `when` holds (the prefix is in force: a latch it set), nothing is done
when `skip` holds (the count is 0); else an element runs, then `next`
(which counts it, and says whether more remain) and `while` (REPE/REPNE's
condition) decide whether it runs again. `interruptible`: it runs again from
where it started, an instruction boundary between elements (an interrupt
is taken there and the instruction resumes); otherwise the elements run
within the one step. The generator keeps where each instruction started
(the pc as decoding found it) for the interruptible form. A prefix has no
`repeat` (E1032); any other word in it is an error (E0106).

`bulk` at the end of the clause claims the element is a *stream*, which a
core may run many elements of as one block (a fill, a copy, a scan), and
the generator checks the claim when it builds: it evaluates the element
with every fn inlined, values being constants, invariants (the same every
element: latches, operands, state the element does not write), a register
as the element found it plus an invariant (a pointer), or what the element
loaded. A branch on an invariant splits the evaluation (each path a case
the runtime tells apart: the mode, paging, the address size); a branch on
what the element loaded or where it is ends the path, unless one arm
always raises (a fault check, as a segment limit's). Some path must be a
stream: its pointers move by invariants and its accesses are a fill (one
write of an invariant), a copy (a read and the write of it), a scan (one
read, results from it), a compare (two reads), a load (one read into a
register), a port-in or a port-out (a port access and the memory access
of its value). Otherwise building fails, saying why each path is not
(`MACHINE_BULK` prints each path and the shape found). On the 386 every
string instruction is one: MOVS a copy, CMPS a compare, STOS a fill, LODS
a load, SCAS a scan, INS and OUTS port streams.

A fill, copy, scan, compare or load also gets a *kernel* for the JIT
core's `repeat: bulk` (section 13): the elements after the current one, run
as a block. Its loop is the runtime's (`machine_runtime::bulk::run`); the
generator gives it the element (an `rt::bulk::Element`). The kernel runs
the element's own code dry (its accesses noted, not made) at a few
elements (how the registers move, where it reads and writes, whether it
faults), then moves the data through the board's direct memory, element by
element in order (an overlapping copy copies as the elements would). The
paths are told apart as the element's code tells them apart, by running
it, so one kernel serves them all (the 386's MOVS has 1536 stream paths).
Its soundness is the generator's to show, when it builds: every path must
be a fill or copy the kernel can take, or raise (the dry code raises too),
or reach a place where the dry code stops, giving up. That place is the
expression that first made a value the branch that ends the path depends
on (on the 386, the page offset tested on the right of `CR0.PG == 0 ||`),
or a statement a stream does not make. A path the kernel takes moves only
its pointers (by bit 0 of their registers) and, in `next`, the count (by
one, repeating while it is not zero); its time does not depend on the
element (its timing entry with `restarted()` holding is an invariant); and
each fault check on where it is holds over an interval of elements (a
comparison of a pointer with an invariant, or a conjunction of those,
where nothing wraps), so holding at the first and the last element it holds
between. Its results (the flags a compare sets, the register a load
fills) are each element's own, computed from what the element loaded and
invariants alone, so the last element's are the instruction's. A `while`
condition (REPE, REPNE) must depend on nothing else either: the
generator follows units' bodies to show it (the 386's ZF after a CMP does
not depend on the CF it is given), and the kernel tests it on each
element's loaded values as its closed form (the condition with the
element's results in place, a branch on an invariant such as REPE or
REPNE kept a branch). The kernel stops before the element that ends the
repeat; the step runs that element. A period of elements is timed as the step runs them; the kernel
then runs whole periods while each element's accesses cost what the
measured period's did (their split cycles and direct memory time), so the
time is the elements' to the tick. Where every access of the elements and of
the measured period lies in one direct memory, none of what they write
recorded code, and the isa's `split` reads the address only through
`address & (2^b - 1)` (so a period, moving each access by whole such
spans and whole bus units, costs what the measured one did), the
elements are made on the memory's cells as a block: a fill, a copy (in
element order where source and destination overlap), a scan testing the
closed form; a load makes nothing but the registers.

### 11.2a Prefixes and latches

```
enum SegOv : u3 { ES = 0, CS = 1, SS = 2, DS = 3, None = 4 }
latch seg_override: SegOv = SegOv::None;

prefix SegOverride {
    encoding { opcode "001ss110" { s: seg_ov } }
    operands (s: seg_ov);
    semantics { seg_override = s; }
}
```

A `latch` is per-instruction state: its initialiser is evaluated at every
instruction boundary. It may read architectural state (`latch start_ip: u16
= IP;`) but not call functions or touch memory. An instruction that needs
to repeat (a REP string iteration that is not the last) sets `IP =
start_ip`; the next step re-executes it, which is also what makes it
interruptible between iterations. A `prefix` is declared like an
instruction but has only opcode bytes; the decoder runs its semantics and
continues decoding, so any number of prefixes may precede an instruction.
Prefixes communicate with instructions only through latches.

`halt;` in isa code stops the processor: the interpreter's `step` does
nothing until it is restarted (interrupt delivery is not yet described).

### 11.2b Timing tables

```
timing on I286 {
    Mul_rm16 => if src.is_mem { 24 } else { 22 };
    Shift_rm16_cl => if dst.is_mem { 5 + (CL & 0x1F) as u32 } else { 6 + (CL & 0x1F) as u32 };
    Jcc => if condition(c) { 4 } else { 5 };
    Stosb => if !repeating() { 1 } else if restarted() { 1 } else if CX == 0 { 7 } else { 7 };
    fault => match vector { 0 => 8, 5 => 5, 6 => 5, 13 => 20, _ => 9 };
    interrupt => 9;
}
```

An entry is the instruction's **execution cycles beyond its bus
transfers**: the cycles its bus cycles do not account for. It is
evaluated in the context of the instruction's flag conditions: its
operands (shapes and values), registers and latches on the state before
it runs, and the isa's fns. `restarted()` is true when the step starts
where the previous one did, which is how a repeated string instruction's
later iterations are told from its first. `repeats()` is true when the
step leaves the istream's address where it started: a repeating block
instruction (the Z80's LDIR, CPIR) that runs again. Whether it does is
known only after it ran, so an entry that uses `repeats()` is evaluated
after the instruction (CPIR stops early on a match, which the state
before cannot tell without reading memory). An entry for a prefix is added
for each prefix byte decoded. `fault` is added when a fault is delivered,
with the fault's `vector` in scope; `interrupt` is the cost of taking an
interrupt at an instruction boundary. An instruction without an entry
costs 0 and is reported by the timing comparison.

`split` is the cost of how the processor makes a data access: it is
evaluated for each read or write of the instruction stream's space during
a step, with the access's `address` and `width` (in bytes) in scope, and
added to that step. The 386 makes an access that crosses a dword boundary
as two, which costs it four clocks however the bus then splits it:

```
split => if (address & 3) + width > 4 { 4 } else { 0 };
```

`machine vectors --fit` groups vectors by each memory-capable operand's
`is_mem` and, when its kind has one, its `ea` query (the address work), and
fits what the table's entry must explain: the measured cycles less what
the step charged beyond the entry (its prefixes and splits).

The interpreter computes a step's cycles and hands them to the bus
(`Bus::retire`); the plain-memory bus ignores them. The model build charges
an instruction its data transfers plus the larger of its code fetches and
its execution cycles: execution overlaps prefetching, so an instruction
whose execution is shorter than its fetches costs its fetches (section 12).
A processor with no prefetch says so in its table:

```
fetch => serial;
```

and then an instruction costs its code fetches, its data transfers and its
execution cycles in turn (`fetch => overlapped;` is the default). The
Z80's table (`targets/z80/timing.machine`) is of this kind: its transfers
are three T-states each, and each entry carries the instruction's internal
clocks plus one for each opcode fetch's refresh clock, so a board built
with it is exact to the T-state, as all 1.6 million of its vectors are.

The table is meant to be **measured**, not typed in from a datasheet.
For the i286 it comes from the SingleStepTests cycle traces
(`targets/i286/timing.machine`): `machine vectors --fit` suggests an entry
per instruction form from the traces (the most common value, with the
share it explains), the author writes the data-dependent entries
(counts, REP iterations, taken branches) from the same data, and
`machine vectors --timing` compares every vector with the table. A trace
there starts with an empty prefetch queue, so the memory forms vary by a
cycle or two around their entry with how operand transfers overlap the
refill; modelling the queue (cycle-accurate bus timing, tier 2) removes
that, and is the processor model's business with a `schedules` table
(section 11.2c), not this table's.

### 11.2c Schedules: cycle-accurate timing

A timing-table number (section 11.2b) is enough for an emulator's sense
of time. To reproduce a processor's bus activity cycle for cycle, a second
table gives each instruction a **schedule**: the execution unit's work
between the instruction's steps.

```
schedules on I286 {
    /// A sign-extended byte costs the decoder a clock.
    decode Disp8 => 1;

    Mov_rm16_r16 => if dst.is_mem { schedule(dst.ea, access, 0) } else { schedule(2) };
    Alu_rm16_r16 => if dst.is_mem { schedule(dst.ea, access, if op == AluOp::Cmp { 1 } else { 2 }, access, 0) } else { schedule(2) };
    Mul_rm16     => if src.is_mem { schedule(src.ea, access, 20) } else { schedule(21) };
    Call_rel16   => schedule(4, access, 0, jump, 0);
    Call_far     => schedule(5, access, 5, jump, 0, access, 0);
    Ret          => schedule(3, access, 4, jump, 0);
    Jcc          => if condition(c) { schedule(4, branch, 0) } else { schedule(3) };
    Pop_r16      => 3;
}
```

The `timing` table stays the emulator's (tier 1); a build whose processor
model runs schedules uses this one. Entries are evaluated like timing
entries (on the state before the instruction runs, with its operands in
scope). An instruction without a schedule runs its timing entry as work
before its accesses, and a plain number `n` in this table is
`schedule(n)`. `interrupt =>` gives the schedule of an interrupt taken at
a boundary.

**Faults.** `fault => ...` (with the fault's `vector` in scope) is the
schedule of delivering a fault, and `fault <Instruction> => ...` (with
`vector` and the instruction's operands) overrides it for faults that
instruction raises: the 286's IDIV raises #DE two clocks later than DIV.
The delivery follows the faulting instruction's schedule up to where the
instruction stopped: the interpreter counts the data accesses the
instruction made before the fault, and the schedule is cut at the first
`access` it did not make (a #GP on a word at offset FFFF stops at that
access; a divide that overflows has made its accesses and done its work);
the delivery's steps follow. A fault the decoder raises
(an undefined opcode) has no instruction schedule. An instruction whose
operands the semantics reject can say so in its own entry: on the 286,
`Les => if src.is_mem { ... } else { schedule(0) };` puts #UD's delivery
right after decoding. Without a `fault` entry, the timing table's `fault`
cycles are work after the last step.

**Repeated string instructions** run an iteration per interpreter step,
and each iteration retires with its own schedule (`restarted()` tells the
later ones from the first). Since an entry is evaluated before its
iteration runs, it cannot know whether that iteration is the last
(REPE and REPNE stop on a flag); the 286's entries put the last
iteration's tail at the end of every iteration and the extra gap between
iterations at the start of each later one:

```
Movsb => if !repeating() { schedule(3, access, 2, access, 0) }
         else if restarted() { schedule(1, access, 0, access, 1) }
         else if CX == 0 { schedule(7) }
         else { schedule(7, access, 0, access, 1) };
```

`schedule(w0, step1, w1, ..., stepN, wN)` alternates execution cycles
(`u32` expressions) and steps, starting and ending with cycles; at most 8
steps. The steps are:

- `access`: the instruction's next data access, in the order its semantics
  perform them (a read, a write, an I/O transfer; a word split into two
  bus cycles is one access). If the instruction performs more accesses than
  the schedule names, the rest follow the last step with no work between
  them (`Popa => schedule(3)`: eight reads after 3 clocks); if fewer, the
  unmatched `access` steps are skipped and their work joins the step
  before (CMP shares ADD's entry and only reads).
- `jump`: an unconditional control transfer (JMP, CALL, RET). The queue
  is flushed and refilled from where execution continues, and a decoder
  that knows the instruction transfers stops decoding past it.
- `branch`: a conditional transfer that was taken. The same flush, but the
  decoder had carried on past the instruction, not knowing the outcome.
- `access * n`: `n` accesses (a `u8` expression, which may be 0) with no
  work between them; the work after the step follows the last. ENTER
  copies a frame pointer per nesting level:
  `schedule(4, access, 4, access * (2 * (level - 1)), 4, access, 5)`.

An instruction that transfers control without a `jump` or `branch` step
(an unscheduled one, a fault, an interrupt) is given a `jump` after its
last step.

`decode <Operand> => n;` says the decoder spends `n` more clocks on an
operand format's first byte. What those clocks do is the processor
model's rule.

A schedule is a value of the prelude bundle `Schedule`:

```
bundle Schedule {
    /// work[0] before the first step, work[i] after step i.
    work: [u16; 9],
    /// 1 access, 2 jump, 3 branch; 0 past the last step.
    step: [u2; 8],
    /// The accesses an `access` step stands for (1, or `access * n`).
    count: [u8; 8],
    steps: u4,
    // Filled by the interpreter for each instruction:
    /// Bytes decoded, prefixes included.
    length: u8,
    /// Extra decoder clocks on each byte (from `decode` entries).
    decode: [u4; 16],
    /// Where the instruction started and where execution continues
    /// (istream addresses), and whether it halted.
    start: u64,
    next: u64,
    /// Bytes the stream holds from `next` before its segment ends
    /// (`istream { limit ...; }`, section 11.1a; all ones without one).
    room: u32,
    halted: bool,
}
```

The **prelude** is a module every program sees without a `use`; names are
looked up in a module, then its imports, then the prelude.

**Who runs the schedule.** A processor model that declares

```
on retire(s: Schedule) -> u32 { ... }
```

owns its instruction timing. After every instruction (and every interrupt
taken at a boundary) the interpreter calls it with the instruction's
schedule, and the handler returns the clocks that have passed; the model
build advances time by that, and charges the processor's own bus
transactions only their wait states. The handler sees the instruction's
data accesses through its own space handlers, which it can log. The bus
unit, the decoder and the execution unit are model code in the processor
description; the language provides the schedule and the hook, not a
pipeline. A model may time an instruction late: the 286's runs one
instruction behind the interpreter, because its decoder needs to know
where the following instruction ends, and HLT completes it. A board
whose processor model has no `on retire` times instructions from their
cycles alone (`Bus::SCHEDULE` is false), and the interpreter doesn't
build schedules for it.

The i286 description's processor model (`Cpu286` in
`targets/i286/board.machine`) implements the rules found in the
SingleStepTests traces (`research/i286-pipeline`):
- a 6-byte queue refilled a word at a time when a word fits;
- a decoder taking one byte per clock, each the clock after it arrives,
  and decoding at most one instruction ahead;
- execution starting when an instruction's last byte is decoded;
- requests reaching the bus two clocks after the execution unit makes
  them, ahead of prefetch;
- reads waiting for their last bus cycle to start, and writes posted;
- a sign-extended byte (`decode Disp8`) stalling the decoder a clock when
  it starts a word, and otherwise starting the instruction a clock late;
- an odd jump target's word slot holding a dead byte until the next flush;
- no prefetch once HLT runs, and HLT's HALT cycle requested three clocks
  after it starts;
- a repeated string instruction's later iterations decoding nothing (its
  bytes stay decoded) and not transferring;
- no prefetch past the end of the code segment (the schedule's `room`).

Address work before a ModRM access is the operand kinds' `ea` query: 4
clocks for base+index with a displacement, otherwise 3. The model records
its bus cycles, and the rig system `targets/i286/rig.machine` runs each
vector through it to compare that record with the trace: the processor's
conformance at transaction boundaries (section 13). Over the whole
corpus (130,000 vectors, repeated strings and exceptions included), 98%
match every bus cycle.

### 11.2d Instruction families and decode conditions

The 386 runs most of its instructions at two operand widths and two
address widths, chosen per instruction by prefixes (66, 67) and, in
protected mode, by the code segment's default size. One description
covers both with **families**: instructions, operand kinds and isa fns
with a `const W: width` parameter, instantiated at a list of widths.

```
latch opsize: u1 = 0;                  // 66 toggles it
prefix OperandSize { encoding { opcode 0x66; } semantics { opsize = 1; } }
fn operand_size() -> u6 { return if (opsize ^ CS.big) == 1 { 32 } else { 16 }; }

instruction Alu_rm_r<const W: width> in [16, 32] {
    mnemonic alu_name(op);
    encoding { when operand_size() == W; opcode "00ooo001" { o: alu_op }; ModRM { reg: r<W> } }
    operands (op: alu_op, dst: rm<W>, src: r<W>);
    flags { modifies: [OF, SF, ZF, AF, PF, CF]; }
    semantics {
        let a = dst;
        let (result, raw) = Alu<W>(op, a, src, 0);
        if op != AluOp::Cmp { dst = result; }
        FLAGS.arith = AluFlags<W>(op, a, src, raw);
    }
}
```

- **Instances.** `instruction Name<const W: width> in [16, 32] { ... }`
  declares one instruction per listed width, named `Name<16>` and
  `Name<32>` (in tables, the assembler and diagnostics). `W` is a
  compile-time constant inside: in types (`u<W>`), unit instantiations
  (`Alu<W>`), fn instantiations (`push<W>(v)`), kind references
  (`rm<W>`) and expressions; `if W == 32 { ... }` is an ordinary
  `if` on a constant (the generated code folds it), so both branches are
  checked in every instance: each branch names the registers of its own
  width (`EAX` in one, `AX` in the other) rather than relying on the
  other branch being discarded. A timing or schedules entry may name one instance (`Alu_rm_r<32>
  => ...`) or the family (`Alu_rm_r => ...`, with `W` in scope); an
  instance's own entry wins.
- **Kind families.** An operand kind is declared per width with the width
  in its name, `operand_kind r<16> : u16 { ... }` and `operand_kind r<32>
  : u32 { ... }`, or once with a parameter when its rules are uniform,
  `operand_kind rm<const W: width> in [16, 32] : u<W> from m: ModRM { ...
  }`. `r<W>` in an instance resolves to the kind for that width.
- **Fn families.** `fn push<const W: width> in [16, 32](v: u<W>) { ... }`
  is an isa fn per listed width, called as `push<W>(v)` or `push<32>(v)`.
  A kind or fn family without `in [...]` is instantiated at 8, 16 and 32.
- **Names.** An instance is written with its width and no spaces,
  `r<16>`, `Alu_rm_r<32>`; with spaces, `<` is a comparison. A family is
  expanded when it is parsed, by substituting the width for its
  parameter, so every later stage sees ordinary instructions, kinds and
  fns.
- **Width-generic access.** `mem.read<W>(addr)` and `mem.write<W>(addr,
  v)` are `mem.read_u16` and friends with the width a parameter.
- **Decode conditions.** `when <expr>;` in an encoding is a condition the
  decoder checks after matching the instruction's bytes: an expression
  over latches, registers and pure isa fns (no memory, no faults), read at
  that point of decoding (after the prefixes). Instructions whose
  encodings overlap must differ in a field constraint (E1045) or in a
  `when` of the form `E == c` with the same `E` and different constants;
  the decoder tries them in declaration order and takes the first whose
  condition holds. The usual conditions are the operand and address
  sizes.
- **Decode-time tails.** An operand format's `tail` may read latches and
  call pure isa fns as well as its fields, so the ModRM can depend on the
  address size:

```
operand ModRM {
    size 1;
    fields { mod: u2 @ 6; reg: u3 @ 3; rm: u3 @ 0; }
    tail {
        if address_size() == 32 {
            if mod != 3 && rm == 4 { Sib }
            else if mod == 1 { Disp8 }
            else if mod == 2 || (mod == 0 && rm == 5) { Disp32 }
        } else if mod == 1 { Disp8 }
        else if mod == 2 || (mod == 0 && rm == 6) { Disp16 }
    }
}
operand Sib {
    size 1;
    fields { scale: u2 @ 6; index: u3 @ 3; base: u3 @ 0; }
    tail { ... }    // its displacement, which depends on the ModRM's mod
}
```

  A tail may also name its parent's fields (`parent.mod`), since the
  SIB's displacement depends on the ModRM it follows; that operand must
  be the tail of exactly one other. Code reading an operand's tail decides
  which one it is from the same fields the tail did (the 286 and 386
  effective-address fns do), since there is no query for it.

A family is a convenience, not a new kind of instruction: each instance is
an ordinary instruction to the interpreter, the rig, the timing tables and
the conformance tests.

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
yield a *place* (every branch assignable). A value kind turns an encoding
field into a value (`operand_kind alu_op : AluOp { value(n: u3) => n as
AluOp; }`); its operands are read-only. A from kind is built from a
fetched operand bound to a name (`m`) and has `read`, optionally `write` (a
kind without `write` is read-only, e.g. immediates) and `is_mem`. In the ISA,
`rm8` resolves through `ModRM` to a register or a `translate`d memory
access. Any other rule without parameters is a **query**: a pure
expression over the fetched operand that instructions, flag conditions and
timing entries read as `operand.name`, e.g. on the 286

```
    /// The address work before the first access (a unit shared by the
    /// ModRM kinds): 4 for base+index with a displacement, else 3.
    ea => EaWork(m.mod, m.rm);
```

A query may not read memory or call an isa fn (E1037). In rtl routines,
`let m = fetch ModRM;` consumes the operand's bytes
from the instruction stream via the component's `istream` (a routine the
component designates), taking the cycles that costs. Because `fetch`
sequences are typed by the same declarations, the compiler can derive the
instruction's encoding and length from the routine.

**Operand forms.** A kind whose `read` rule is an `if` on a condition of
its operand's fields alone has two forms: the condition true, and false
(the 386's `rm`: `m.mod == 3`, a register, else memory). The compiler
gives the kind a form fn and its rules a copy per form with the `if`
decided: `read`, `write` when it tests the same condition, `is_mem` when
it is the condition or its negation. Each instruction with up to three
such operands gets a copy of its semantics per combination of their
forms, and `run_<name>` picks the copy once from the decoded operands, so
the test is made once per instruction rather than at every access, and
each copy's code is its own. Nothing in the description changes; the
copies behave as the semantics for operands of those forms (the 386's
hardware vectors are unchanged through the generated code). On the 386
board this is 5-10% faster with either core.

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
- `memory: direct;` in a profile builds an emulator. The processor's
  memory accesses in the ranges of slaves whose memory declares `direct`
  read and write the cells without the processor model's handlers or the
  bus's decode. Each access is charged the transfer's clocks plus the
  declared waits once per bus-width unit it touches, and code once per
  unit fetched, as a one-unit buffer would. Time is approximate: a
  processor model's own buffering and its view of writes into it are
  bypassed, and so is anything they do to the address (the AT's A20
  gate: an emulator addresses memory above 1 MB with the gate closed).
  Processor models that time instructions themselves (`on
  retire`) can't be built this way. `memory: bus;` is the default.
- `core: decode | trace;` in a profile chooses how the processor's
  instructions are found and run (**cores**); the description is the same.
  `decode` (the default) decodes each instruction where it runs. `trace`
  (with `memory: direct`) keeps runs of decoded instructions and runs them
  again without decoding. As the processor runs, each instruction decoded
  is recorded with what decoding left (its `Instr`, its length, its
  prefixes' latches, the pc after it, the decoder's clocks) while the
  instructions it reaches stay within its page of code memory: past taken
  jumps, calls and returns (each step's pc is checked as it is replayed:
  a jump taken otherwise ends the run), up to one before its start or
  already in it, where it ends and another starts; when a
  run's start is reached again, its instructions are replayed: pc, latches
  and decoder state set as decoding left them, the fetches charged as they
  cost, then the instruction. The compiler derives what makes a replay
  exactly the run the decoder would have made:
  - What the decoder reads besides the bytes and the pc (on the 386: `CS`,
    for the operand size) is a run's key, compared by value where a run is
    entered.
  - Every assignment the compiler emits to state the decoder or the
    istream's address, window and limit fns read, other than the pc, bumps
    an epoch, field by field (a far jump, a task switch, a `CR0` or `CR3`
    write, a TLB fill; not a carry flag, not `DS`). A run stops when it
    changes; a run that followed another is entered from it directly
    (chained) while the epoch of what is not in the key is unchanged and
    the key holds.
  - A run continues only while the pc is where it goes next (a jump taken
    leaves it).
  - Each direct memory marks the 128-byte blocks of each 4 KB page that
    hold recorded code; a write to a marked block counts against its page,
    unmarks the block and notes the page's count in it. A run is replayed
    while its page's count is unchanged, or none of the blocks it lies in
    has been written since it was recorded (it then takes the page's
    count). Writes to data beside code cost nothing; a write to data in a
    block of code (a counter beside an interrupt handler) costs the runs
    in that block, not the page's others; code that changes is decoded
    again.
  Interrupts, device catch-up and timing run per instruction as with
  `decode`, so the machine's behaviour and time are the same; the host
  calls `code_changed()` after writing memory or the processor's state
  directly. On the 386 board, 32-bit code with paging on runs about 50%
  faster (decoding, fetching and translating the code address are done
  once per run), real-mode code about 27%; a DOS boot, mostly short loops
  polling devices, about the same (-3% to +7%).
- `core: jit;` (with `memory: direct`) is the trace core with runs entered
  often (16384 times: a compile costs what thousands of entries do)
  compiled to host code, quickly (Cranelift's optimiser off), and again
  optimised once their steps have run a million times a step (Cranelift, through the
  `machine-jit` crate, the one that may use `unsafe`). Runs are compiled
  whole and entered only at their first step: when the link from the run
  before leads to them (chained), or the pc is where one starts (found by
  address); elsewhere the trace core steps until it reaches one. One entry
  keeps what the code knows of the machine from step to step. A run whose
  end leads back to its start goes round inside the compiled code, its
  first step built twice: the copy the caller enters knows nothing, the
  copy the back-edge reaches knows what the run knew going round (the
  fields it holds as host values, and the constants it owes, not yet
  stored), and they meet at the second step.
  `System::run_until` checks its stop condition between bursts (a run, as
  far as it goes), so at most a run apart: the machine's behaviour is the
  same, but a host acting on a condition acts up to a run later than with
  the other cores.

  A step is compiled by the machine's own generated code, which builds the
  host code through an emitter (`machine_jit::Em`) with what the step was
  decoded with in hand: its operands, latches, pc and length are constants
  then, and every operation on them folds, so an operand's choice of
  register (`r.select(z)`), a prefix's latch (`match index`), a jump's
  target or a timing entry cost the compiled code nothing. The compiler
  derives this from the description: for each isa fn, operand kind rule
  and instruction it writes an emitter following the interpreter's code
  rule for rule (native widths, wrapping arithmetic brought back to the
  declared width, Rust's shift and cast semantics; registers written
  through to the machine with what is known of them remembered, locals in
  host registers). A lowered step is the run's check, `step`'s prologue (the
  isa's boundary fn inlined), the state as decoding left it, the timing
  entry and the semantics in host code, then one call that retires it
  (`step`'s tail, the devices caught up, the run's code checked for
  writes). Records and tuples lower as their scalar leaves (a record
  register's fields, a register view's bit fields, locals per leaf); a
  register array indexed by a computed value is addressed from the machine,
  bounds-checked; operand structures (a ModRM) are values known when
  compiling, and a tail the decoder did not fetch can only be reached on a
  path the code never takes (a trap there). `raise` calls a shim that
  delivers the fault as the step's tail would, and the run ends. Units are
  inlined like fns. A fn whose inlined size (its HIR and what it inlines)
  passes a threshold is called instead, through a shim that runs the
  interpreter's fn and delivers its fault (`jit_pending` then ends the
  run): the 386's segment and paging work stays one call per access. When
  the istream's prologue does not lower (the 386's reset line, its
  boundary fn reaching the external `intr`), the step calls the
  interpreter's prologue and lowers the rest; a step decoded with a lock
  prefix is the interpreter's. What does not lower (a loop, a 128-bit
  division, a union) is a call of the instruction's step as the
  interpreter makes it; the generator decides per instruction, so nothing
  is approximated. A timing
  entry that asks `repeats()` is emitted after the semantics, comparing the
  istream's address with the step's own (a constant).
  `MACHINE_JIT_LOWERING` set while generating lists what does not lower and
  why; `MACHINE_JIT_DUMP` at run time prints the compiled code.

  A board describes its time keeping to the JIT (`Bus::jit_board`: where
  it keeps the time, the code time and unit, its code-write counters and
  its next device deadline; a clock's time; whether edges can wait and
  whether devices are caught up at all), and answers the time of a direct
  code fetch (`Bus::jit_fetch_time`). A lowered step then retires in host
  code too: an add of its fetch and cycles, a compare with the next
  deadline (or nothing, on a board without devices), the code-write
  counters against the last count seen, and `last` recorded for when the
  code returns. A run counts the time in the processor's clock periods (a
  64-bit count in a host register: `machine_jit::Clock`) while every
  charge is whole periods, as fetches and direct accesses at the
  processor's clock are; the board's 128-bit time holds it only where it
  must (before a call, on the way out, where the code reads it), scaled
  then. The deadline is turned into the periods left before it (a
  division from below: the check comes no later than the deadline, at
  most a period before, and the devices check the exact time), worked out
  again after each call. Exact: the time the board holds is the same.
  `MACHINE_JIT_FINE` at run time keeps the time in the board's units.
  Board fields that calls change (the time) are never remembered by the
  emitter. Facts a lowered step guarantees are not checked again by the
  step after it: the processor runs and is not shut down, the epoch is
  unchanged unless its writes bumped it, and a store of a value a field
  already holds is dropped. Stores to the machine are deferred until it
  must hold them (a call, an exit, a join), so a run of flag writes is one
  store.

  The board also describes its direct memory in the istream space
  (`Bus::jit_mems`: ranges, index masks, costs, which memory) and hands
  over its pointers when compiled code is entered (`Bus::jit_mem_ptrs`). A
  lowered byte access there is a range check (folded when the range is the
  whole space), a load or store, and the time; a write checks the page's
  code bitmap and calls the board only when it hits recorded code
  (`Bus::jit_code_hit`), which then ends the run at its check.

  On the Z80 every instruction lowers; the checksum runs at about 91 M
  instructions/s, against 64 for the decode core and 57 for the trace
  core. What is left is mostly the compiled steps themselves, entering a
  run from Rust (`System::run`, about an eighth) and compiling. Cranelift's optimiser is off: the emitter
  folds what it would, and compiling is cheaper. Every 386 instruction
  lowers too; its board is not yet described to the JIT (overlapped
  fetches, a 16-bit bus), so its steps still call the prologue, the fetch
  charge and the retirement, and a DOS boot runs at about the trace core's
  speed in POST (22 M/s), slower where it compiles or idles. Random straight-line code with
  random operands, registers and memory, run compiled and stepped, must
  agree to the register, byte and clock (`targets/z80/board_test.rs`), as
  must a loop that rewrites its own operand; so must random 386 code
  (`targets/i386/vga_test.rs`: each instruction tried on a copy of the
  machine first, so the loop runs, its faults returning through an IRET).
  **Speculation.** The compiler picks the registers a compiled run
  speculates on: plain registers the isa's fns read, that no istream hook
  writes itself and at most three instructions write (on the 386 CR0, CR2,
  CR3; operand registers, which kinds select, never). They are watched, so
  a write ends the run like a write to the key does. A compiled run checks
  them where it is entered (a guard), and the emitter knows them, and the
  run key's registers (the 386's CS), as constants in every step past its
  check and again past the gate (an interrupt's path, which leaves the run,
  may have forgotten them). A run whose guard turns it away eight times is
  compiled again for the values then.

  **Trial inlining.** At a call of a fn too big to inline when the crate
  is generated, the emitter can try it inlined, where what is known folds
  it, under a budget of host instructions (`Em::try_inline`: a branch on a
  constant, flipped to the call when the trial is over budget; nested
  trials have budgets of their own, and a failed trial's dead code does
  not count against the one around it). Measured, it does not yet pay: the
  386's real-mode memory read, folded, is still some 380 instructions, and
  the budget defaults to 0 (`MACHINE_JIT_TRIAL` sets it at run time,
  `MACHINE_JIT_TRIALS` reports each trial; `MACHINE_JIT_HOT` sets the
  entries before a run is compiled).

  **Branch joins.** A branch of the emitted code no longer stores what it
  deferred when it ends: it jumps to a block of its own, and the join
  decides. When only one branch comes back (the other raised or left the
  run), it goes on with its stores still deferred; only when several do,
  each stores what it owes. A segment check that raises, or an inlined fn
  that returns, no longer forces every deferred store out, so flags that
  a later instruction overwrites before anything reads them are never
  stored, and their computation, unused, is dropped by Cranelift: flag
  liveness, without an analysis.

  **Direct memory, any width.** The board's direct memory is reached in
  host code for 8- to 64-bit accesses and buses wider than a byte: a range
  check on its first and last byte, an unaligned host load or store, the
  bus units it spans at their cost, the 386's split charge (lowered from
  its timing table), and for a write the recorded-code check on its ends.
  An external fn whose wire is not driven is its constant; one that is a
  call cannot change the processor's registers (its handler has only the
  bus), so what the emitter knows survives it.

  Measured: the Z80 checksum runs at 1.67 times the decode core (90 M
  instructions/s against 54); a real-mode 386 checksum loop at about 1.5
  times the trace core (`bench_hot_loop` in `targets/i386/vga_test.rs`:
  some 45 M/s against 32). A DOS boot, mostly code run a few times, is
  bound by compiling and entering runs.

- **Concessions** (with `core: jit`): profile options that give up some
  of what the exact profiles keep, for speed, each one named. The crate
  says whether it has any (`system::EXACT`), and tests that compare time
  across cores look at it.
  - `sync: run` catches the devices up once a compiled run returns,
    rather than after each instruction: a device (and an interrupt it
    raises) can be seen up to a run late. Ordinary steps still sync each.
  - `sync: block` catches the devices up at a compiled run's last step
    (where it goes round or returns) when one is due or an edge waits:
    device events keep their times, and the processor sees them up to a
    run late (DOSBox's slices, which end at the next event).
  - `sync: quantum(<time>)` lets the devices that raise no signal and
    master no bus (seen only through their ports, which catch them up)
    wait there until the quantum since the last catch-up has passed; the
    others keep their deadlines, as with `sync: block` (MAME's quantum
    interleaves processors; its device timers fire on time). Deferring
    every device breaks handshakes the processor waits for by interrupt
    rather than by reading the device: the AT BIOS asks the keyboard for
    its ID and waits on a flag its keyboard interrupt sets, so a reply
    deferred past its timeout reaches the interrupt as a keystroke (DOS
    saw F7 at its date prompt). On the 386 AT, whose devices are due
    milliseconds apart, it runs as `sync: block` does.
  - `repeat: bulk(<n>)` runs an instruction with an interruptible
    `repeat` clause (section 11.2) up to n times within its compiled
    step, each time decoded, fetched, timed and its run's code
    checked as a step; interrupts and devices are seen between batches
    (QEMU's `REP_MAX`). `repeat: bulk` (no count) runs it until a device
    is due or an edge waits: the batch ends at the first element boundary
    at or past the devices' deadline (the cycles left before it, as the
    run counts them; with `sync: quantum`, the next catch-up), where they are caught up and an interrupt taken, as
    between instructions; with nothing due, a whole REP is one batch. An
    instruction with a kernel (section 11.2) runs its elements past the
    third as a block, in whole periods of the direct memories' unit, each
    costing the period the step measured (with `repeat: bulk`, only the
    periods that end before the deadline).
  - `events: block` recognises the reset line and interrupts (the
    istream's `boundary` fn) where a compiled run is entered or goes
    round, not past each compiled step: an interrupt is taken up to a run
    late, a loop's at its back-edge (QEMU's block starts).
  - `timing: approximate` charges a compiled step its code fetch as if
    the bus were serial and its cycles in one add, with no prefetch
    overlap: emulated time runs differently (on the 386 board, POST
    passes 24 s of emulated time where the exact profiles pass 17).
    Accesses still charge their own time.

  Measured (the Z80 checksum, a 386 DOS boot), they give little: the Z80
  runs about 7% faster, the 386's POST no faster and its idle loop about
  10% faster. What exact time keeping and per-instruction device order
  cost is no longer much of what is left; the rest is structural (entering
  runs, compiling, memory accesses through the paging and segment fns).
- `wire inst.in_port = other.out_port;` connects an output to an input of
  the same type; `wire inst.in_port = <constant>;` straps a pin (`wire
  pic1.sp = true;`). Each input is driven at most once; an output may
  drive any number of inputs (E1049 otherwise). A reader sees a new level
  at once, within the same transaction, which is how a slave 8259 reads
  the CAS lines the master set while acknowledging.
- The system also binds bus roles, address ranges and interrupt lines, and
  the compiler generates address decoders for both levels.

**The model build** (`machine build --system <Name> [--profile <p>]`)
generates one crate: the processor's interpreter, one struct per component
model with a ports trait (the transactions it starts and the signals it
drives), and `system::System`:

- The processor runs over a `Board` that owns every other model, the
  signal levels and the time. Each isa space access runs the processor
  model's space handler; its bus calls decode the address and run the
  answering device's handler, charging the bus clock period times (base
  cycles + `wait`).
- Time is an integer count of the base unit above, kept exact: a
  14.31818 MHz crystal and `osc / 12` share a unit. It is 128 bits
  (`system::clock::Time`). Clocks that share no simple ratio need a very
  fine unit (the VGA's 25.175 and 28.322 MHz dot clocks beside the bus's
  crystals give about 1/2.5e20 s), and 64 bits of that runs out within
  a tenth of a second.
- A device catches up lazily: before a transaction reaches it and after
  every instruction, its `on tick` runs with the cycles of its clock that
  elapsed since the last call (in one call, which is what lets a model
  take shortcuts).
- `emit` sets a signal level at once; the edges are delivered to the
  observing ports' `on port.signal(...)` handlers after each instruction.
- **Tick scheduling.** A device whose `on tick` returns `u32` is skipped
  by the after-instruction catch-up until that many cycles of its clock
  have passed. Anything that reaches it still catches it up first and
  then makes it due again, so skipping never changes behaviour: a
  transaction it answers, a signal or wire edge delivered to one of its
  handlers, and a host `with_<instance>` call. A device that must notice
  an edge it has no other use for declares an empty handler for it (the
  8237 wakes on `on dma.dreq(...) { }`). `catch_up_all()` catches every
  device up regardless, for a host about to read a device's state, such
  as a framebuffer mid-frame. A tick that returns nothing runs at every
  catch-up, as before. On the i286 board this took a halted processor
  from 34 to 120 million steps a second, with identical results. When
  every device that ticks is scheduled, the system keeps the least of
  their deadlines (lowered whenever one is set): until it passes and
  while no edge waits, the after-instruction catch-up is one comparison.
  On the 386 AT this made a DOS boot 15-20% faster, with identical
  results.
- `step()` runs one instruction, or an interrupt the isa's `boundary` fn
  took, and catches the devices up; a halted processor waits one cycle
  of its clock per step. `run(n)` runs until the processor halts and
  `run_until(n, stop)` until a condition holds.

- A device that masters a bus (a DMA controller, a bus controller) gets a
  context holding what it can reach, like the processor's: the devices on
  the buses it masters and, through any of those that master buses
  themselves (bridges), the devices on theirs. Its `on tick`, `on reset`,
  signal handlers, transaction handlers and fns may start transactions.
  It is caught up through that context, before a transaction reaches it
  and after every instruction, so a burst it starts in `on tick` happens
  between instructions. Other devices' contexts hold only the signals.
- **Bridges.** A component that answers on one bus and masters another
  (the AT's bus controller: a slave on the processor's local bus, master
  of the ISA bus) runs its own transactions inside the one it answers,
  each charged on its own bus's clock. Before a transaction enters a
  bridge, the masters behind it that are due are caught up (a scheduled
  `on tick` has said it does nothing before its deadline); inside, a master whose
  transfers would have to cross the bridge again (the DMA controller,
  reached through the bridge whose transaction is in progress) is left to
  catch up after the instruction, and a device the context cannot lend
  answers as open bus. Several masters may share a bus (the bus
  controller and the DMA controller on the ISA bus).
- Each component's ports trait gives every method a default (open bus,
  low levels, emits go nowhere), so a test can drive a model directly
  with a context that overrides only what it observes. A transaction
  handler is generated returning `Option`, `None` meaning it declined.

Not generated yet, and reported when a system needs them: more than one
processor, the generated `bridge` binding (a bridge is written as a
component for now, as `targets/i286/board.machine`'s `AtBus286` is), and
an rtl level in the profile.

## 13. Verification

Built into the language and the compiler:

1. **Static**: type and width checking; exhaustive `match`; every output
   driven on every path; mutable locals assigned before read and not
   carried across a cycle; no combinational loops; clock-domain
   discipline; memory port limits; unit, isa fn and model fn recursion;
   transaction starts reachable from a transaction handler; flag set
   consistency.
2. **Assertions**: `assert` in all bodies, kept in both classes (a build flag
   strips them for release).
3. **Tests in the language**:
   ```
   test "ADD AL, BL sets carry" on I286Isa {
       given { AL = 0xFF; BL = 0x01; }
       run instruction Add_rm8_r8 { dst: AL, src: BL };
       expect { AL == 0x00; FLAGS.CF == 1; FLAGS.ZF == 1; }
   }
   ```
   `run instruction` names operands by place or value; the harness picks an
   encoding of each operand kind that denotes that place (so `dst: AL` finds
   the register form of `rm8`). `run bytes [...]` places the bytes at the
   `istream` address and executes one instruction.
   ```
   test "prefetcher fills to six bytes" on Prefetcher.rtl {
       drive { control.pop = false; mem.wait = 0; }
       run 20 cycles of clk;
       expect { output.count == 6; }
   }
   ```
4. **External vectors**: hardware-captured vectors are a first-class test
   input. The declaration says where they are (a glob relative to the
   file), their format, the address space their RAM addresses refer to, and
   how each register of the format maps onto the isa:
   ```
   vectors "../SingleStepTests/v1_real_mode/*.MOO.gz" on I286 {
       format moo;
       memory mem;
       reg ax = AX;                    // load assigns the place, compare reads it
       reg cs { load => CS = Segment { selector: value, base: (value as u24) << 4 };
                read => CS.selector; }
   }
   ```
   `open io = 0xFF;` declares a space on which nothing answers in the test
   environment (reads return the value, writes are discarded); it describes
   the rig, not the processor. `harness_cycles 1;` is how many cycles of
   each captured cycle trace belong to the rig (its start and end) rather
   than to the instruction. With a trace, a vector's measured execution
   cycles are its cycles not spent in bus transfers, less the harness's.
   `transfer_cycles 3;` is how many clocks a transfer takes (by default two,
   the 286's and 386's Ts and Tc, for `moo`; one for `json`, whose 8-bit
   processors' traces mostly make a transfer a clock; three for the Z80's
   T1 to T3); `machine vectors --timing` compares them with
   the timing table and reports exact and within-one-cycle shares per file
   and the worst instructions, and `--fit` suggests table entries.
   The generated runner loads the initial state, steps until `halt`,
   compares every mapped register and every RAM byte the vector mentions
   (registers it omits must be unchanged), masks the executed instruction's
   undefined flags, skips revoked vectors, and reports opcodes that do not
   decode as unimplemented rather than failed. Vectors whose instruction
   raises an exception run like any other: the ISA's delivery fn must
   produce the state the hardware did. `machine vectors --isa I286
   files...` runs it.

   `format json;` reads the SingleStepTests format for 8-bit processors
   (the Z80's): a file is a list of tests, each an initial and final state
   of named registers and RAM, the ports it reads and writes, and a cycle
   trace. The register names are the file's own, and each is as wide as the
   place it maps to. `ports io;` names the space the ports are on: a port
   the test reads answers its value, and one it writes is compared after.
   A JSON test is one instruction (one iteration of a repeating one), so
   the runner makes one step rather than running to a `halt`. Its trace is
   a clock an entry, with the read or write strobe on one clock of each
   transfer; a read whose next clock shows another address is an opcode
   fetch (the refresh address follows it).
5. **Conformance (differential)**: for every component with two levels, the
   compiler generates a harness that runs both from the same initial state
   and stimulus and compares the architectural projection (processors) at
   every instruction boundary, or the transaction trace (devices) at every
   transaction boundary; signals are compared at those boundaries, within
   the tolerance a `compare signals { ... } within N cycles of clk` gives.
   Models are never compared cycle by cycle (section 8.3). For a processor this is: feed the same instruction stream to
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
| Instruction cycle counts | **measurement** of hardware cycle traces into a timing table (section 11.2b; done for the i286), symbolic execution of the `implements` routine (count `cycle`s, sum bus transaction base costs; `wait` becomes a runtime bus cost), or measurement of the rtl over the vector corpus | emulator timing tables |
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
5. **Timing derivation scope**: for the 286, timing is measured from the
   hardware traces (section 11.2b): 73% of vectors match the table exactly
   and 94% within a cycle. Open: whether the prefetch queue (tier 2) is
   modelled in the processor model, which would make the bus sequence
   itself comparable with the traces, and what that leaves for the rtl
   level to add.
6. **Memory initialisation from files** for ROMs: `mem bios: [u8; 0x10000] { init: file("bios.bin") }` is the obvious form; question is whether ROM contents are part of the description or of the system build. For now the generated system's model memories are public, and the embedding program fills them.
7. **Metastability / async inputs**: proposal is to model none of it.
8. **Name of the language.** "Machine" with `.machine` files, as the
   exemplar used. Easy to change before any code exists.
9. **Host-visible output.** A CGA produces pixels; at the rtl level they
   are RGBI and sync pins sampled by a monitor component, but the model
   level has no output notion. Current convention
   (`targets/i286/board.machine`, `Cga`): a display model renders each
   scanline into a public framebuffer `mem` of colour indices as the beam
   finishes it, with `frame_width`, `frame_height` and a `frames`
   counter, and the host reads them after `sync`, the mirror of ROM
   filling (question 6). Rendering on scanline completion keeps raster
   effects (a palette change mid-frame) right at scanline granularity. A
   real mechanism (frame and audio sinks with a declared format) belongs
   to the Wasm/TypeScript binding layer; the open part is whether the
   language should name a component's outputs to the host so bindings
   are generated rather than written.
10. **More than one device master in the model backend.** Resolved for
   one (the 8237, section 12): devices master between instructions and
   charge the cycles they steal. Two device masters would each need the
   other in its context while the other is mid-transaction; the likely
   answer is to give every master a context assembled per call. Needed
   for bus-mastering cards, not for the PC/AT.
11. **Wires and the AT interrupt cascade.** Resolved: wire ports and
   `wire` items (section 12), and one small request-line bus per 8259 so
   the board decides which device feeds which controller line. The slave
   drives its INT onto the master's line named by its ICW3; the master
   names the slave on CAS and declines the acknowledge, which the slave
   answers (`targets/i286/board.machine`). Open: a bus whose first role
   only observes (an interrupt controller on its request lines) is not a
   master; the language could say so with a role attribute instead of
   the backend inferring it from the absence of transactions.
12. **Tick granularity.** Resolved: `on tick` may return its next
   deadline (section 12). Open: a device that renders (the CGA) could
   also say when it is safe to batch more than a frame, and the
   processor could report when it next needs its own time.
