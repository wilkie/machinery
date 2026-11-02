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
- `fn name(params) -> T { ... }` helpers over the component's state
  (section 8.3a).
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

A transaction handler runs inside the initiator's transaction, so it may
`emit` signals but not start a transaction of its own (E1048), directly or
through a model fn; a device that masters a bus does so from `on tick`.

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
  whether it (or any fn it calls) starts a transaction. Calling such a fn
  from a transaction handler is the E1048 error, reported at the call
  with a note naming the fn that starts the transaction. Signal `emit`s
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
for `Fault`-class faults. State is never rolled back implicitly: the ISA
says what restarting means. On the 286 only IP restarts, from a latch
(`latch start_ip: u16 = IP;` snapshots the instruction start, prefixes
included); registers the instruction already changed stay changed, as the
hardware does (a faulting `movsw` keeps SI advanced). Software
interrupts are ordinary instructions that call the same fn. Delivery is
described once, in the ISA, because it is architecture: on the 286 in real
mode it pushes FLAGS, CS and IP, clears IF and TF, and loads CS:IP from the
IVT at `vector * 4`. Multi-byte operands are assembled from
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
  values (E1045). The assembler, disassembler and decoder are generated
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
- `implements` links to the rtl routine for differential testing and
  derivation. Optional; a 386 model may have an `isa` and no rtl at all.
- `mode` restrictions: `allowed_in [Mode::Protected]` and privilege
  `requires cpl == 0`.

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
  14.31818 MHz crystal and `osc / 12` share a unit.
- A device catches up lazily: before a transaction reaches it and after
  every instruction, its `on tick` runs with the cycles of its clock that
  elapsed since the last call (in one call, which is what lets a model
  take shortcuts).
- `emit` sets a signal level at once; the edges are delivered to the
  observing ports' `on port.signal(...)` handlers after each instruction.
- `step()` runs one instruction, or an interrupt the isa's `boundary` fn
  took, and catches the devices up; a halted processor waits one cycle
  of its clock per step. `run(n)` runs until the processor halts and
  `run_until(n, stop)` until a condition holds.

- A device that masters a bus (a DMA controller) gets a context holding
  every other device, like the processor's; its `on tick`, `on reset`,
  signal handlers and fns may start transactions. It is caught up through
  that context, before a transaction reaches it and after every
  instruction, so a burst it starts in `on tick` happens between
  instructions. Other devices' contexts hold only the signals.
- Each component's ports trait gives every method a default (open bus,
  low levels, emits go nowhere), so a test can drive a model directly
  with a context that overrides only what it observes. A transaction
  handler is generated returning `Option`, `None` meaning it declined.

Not generated yet, and reported when a system needs them: more than one
processor, more than one device master, `bridge`, an rtl level
in the profile, and processor time between bus transfers (an instruction
costs only its bus time until timing is derived, section 14).

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
   the rig, not the processor.
   The generated runner loads the initial state, steps until `halt`,
   compares every mapped register and every RAM byte the vector mentions
   (registers it omits must be unchanged), masks the executed instruction's
   undefined flags, skips revoked vectors, and reports opcodes that do not
   decode as unimplemented rather than failed. Vectors whose instruction
   raises an exception run like any other: the ISA's delivery fn must
   produce the state the hardware did. `machine vectors --isa I286
   files...` runs it.
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
12. **Tick granularity.** Devices are caught up after every instruction.
   For a CGA that is a handful of pixels per call. Proposal: `on tick`
   may return the number of its cycles until it next needs to run, and
   the scheduler skips it until then or until a transaction reaches it.
