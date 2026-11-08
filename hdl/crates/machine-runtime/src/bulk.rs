//! Kernels: a repeated instruction's elements run as a block (`repeat ...
//! bulk`, the JIT core's `repeat: bulk(n)`).
//!
//! The generator gives each instruction with a kernel an [`Element`]: its
//! element run dry (its accesses noted, not made), the registers it moves,
//! its `while` condition, and the bus's direct memory. [`run`] does the
//! rest: where the element is now and how it moves, checked at the far
//! ends; then whole periods of elements, each element's accesses costing
//! what the period measured before it cost, its data moved (or its `while`
//! condition tested) in order. Where every access lies in one direct
//! memory whose costs repeat with the period ([`Element::span`]), the
//! elements are made on its cells as a block: a fill, a copy, a scan.
//!
//! What makes this exact is the generator's to show (SPEC 11.2): every way
//! through the element the dry code finishes is one whose registers move by
//! the same each element, whose fault checks hold on an interval of
//! elements, whose results are its own, and whose time does not depend on
//! the element.

/// The registers a kernel moves, at most.
pub const MOVING: usize = 4;

/// What an element does with memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// One write, of the same value each element.
    Fill,
    /// A read, and the write of what it read.
    Copy,
    /// Reads only (a scan, a compare, a load): the `while` condition, when
    /// there is one, on what they read.
    Read,
}

/// An instruction's element, as its kernel sees it.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub kind: Kind,
    /// The moving registers: how many (the count last), the width each
    /// moves in from bit 0 (on any way through the element), and its own.
    pub moving: usize,
    pub widths: [u32; MOVING],
    pub full: [u32; MOVING],
    /// The accesses, in order: how many, and whether each writes.
    pub accesses: usize,
    pub writes: [bool; 2],
    /// Each access's bytes.
    pub bytes: u64,
    /// It has a `while` condition.
    pub cond: bool,
    /// Bytes past which what an access costs repeats, in one direct
    /// memory (its units, and the isa's split charge, by the address's low
    /// bits); 0: not known to.
    pub cost_period: u64,
}

/// The element as it would run: the addresses it reaches (in order), the
/// value it writes, and the moving registers after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dry {
    pub addrs: [u64; 2],
    pub value: u128,
    pub moved: [u64; MOVING],
}

pub trait Element {
    const SHAPE: Shape;
    /// The moving registers now (the count last).
    fn moving(&mut self) -> [u64; MOVING];
    /// Puts them so, for the dry code (nothing else noticing).
    fn place(&mut self, v: &[u64; MOVING]);
    /// Puts them so, the elements made (as a write by the instruction).
    fn commit(&mut self, v: &[u64; MOVING]);
    /// The element, dry, from the registers as they are (left as they
    /// were); None when it faults or takes a way the kernel does not.
    fn dry(&mut self) -> Option<Dry>;
    /// The `while` condition on what an element read.
    fn cont(&mut self, read: [u128; 2]) -> bool;
    /// What an access costs an element (the split cycles, the memory's
    /// time); None where it is not direct memory.
    fn cost(&mut self, a: u64, n: u64, write: bool) -> Option<(u32, u128)>;
    fn peek(&mut self, a: u64, n: u64) -> Option<u128>;
    fn poke(&mut self, a: u64, n: u64, v: u128) -> bool;
    /// The direct memory holding all `len` bytes from `a` (for a write,
    /// when `write`: none of them code, so a store is all a write does),
    /// and where `a` is in its cells; within it, an access costs by its
    /// address's low bits alone (`Shape::cost_period`). None: not so.
    #[allow(unused_variables)]
    fn span(&mut self, a: u64, len: u64, write: bool) -> Option<(u32, usize)> {
        None
    }
    /// A memory `span` named: its cells.
    #[allow(unused_variables)]
    fn cells(&mut self, mem: u32) -> &mut [u8] {
        &mut []
    }
}

fn mask(w: u32) -> u64 {
    if w >= 64 {
        u64::MAX
    } else {
        (1u64 << w) - 1
    }
}

/// `v` (of `w` bits) as signed.
fn signed(v: u64, w: u32) -> i128 {
    let m = mask(w);
    let v = v & m;
    if (v >> (w - 1)) & 1 == 1 {
        v as i128 - (m as i128 + 1)
    } else {
        v as i128
    }
}

/// The dry element `j` elements from now (the registers `r0`, moving `s`
/// each), the registers left as they were.
fn dry_at<E: Element>(e: &mut E, r0: &[u64; MOVING], s: &[i128; MOVING], j: i128) -> Option<Dry> {
    let mut at = *r0;
    for q in 0..E::SHAPE.moving {
        at[q] = (r0[q] as i128 + j * s[q]) as u64;
    }
    e.place(&at);
    let x = e.dry();
    e.place(r0);
    x
}

/// The elements after the current one, up to `max` of them, as a block, in
/// whole periods of `per` elements; the last element (and the one the
/// `while` condition ends at) left for the step. The elements made, the
/// registers moved past them (0: none made).
pub fn run<E: Element>(e: &mut E, max: u64, per: u64) -> u64 {
    let sh = E::SHAPE;
    let (nm, c) = (sh.moving, sh.moving - 1);
    if per == 0 || per > 64 || nm == 0 || nm > MOVING || sh.accesses == 0 || sh.accesses > 2 {
        return 0;
    }
    let r0 = e.moving();
    let count = r0[c] & mask(sh.widths[c]);
    let mut m = max.min(count.saturating_sub(1));
    m -= m % per;
    if m == 0 {
        return 0;
    }
    // Where the element is, and whether it reaches direct memory at all.
    let Some(d0) = e.dry() else { return 0 };
    let n = sh.bytes;
    for q in 0..sh.accesses {
        if e.cost(d0.addrs[q], n, sh.writes[q]).is_none() {
            return 0;
        }
    }
    // How it moves them (the count by one down); from the period measured
    // to the elements made, none carries out of the width it moves in.
    let mut s = [0i128; MOVING];
    for q in 0..nm {
        if sh.full[q] > 64 || sh.widths[q] == 0 {
            return 0;
        }
        let mw = mask(sh.widths[q]);
        s[q] = if q == c { -1 } else { signed(d0.moved[q].wrapping_sub(r0[q]), sh.widths[q]) };
        let lo = (r0[q] & mw) as i128 - per as i128 * s[q];
        let hi = (r0[q] & mw) as i128 + m as i128 * s[q];
        if lo < 0 || hi < 0 || lo > mw as i128 || hi > mw as i128 {
            return 0;
        }
    }
    // The period measured and the last element: where they are (the fault
    // checks holding at the first and the last, they hold between). The
    // addresses move by the same each element: the period's move, a whole
    // multiple of `per`, and the last element on the same line.
    let Some(dm) = dry_at(e, &r0, &s, -(per as i128)) else { return 0 };
    let Some(dl) = dry_at(e, &r0, &s, m as i128 - 1) else { return 0 };
    let mut step = [0i128; 2];
    for (q, st) in step.iter_mut().enumerate().take(sh.accesses) {
        let a0 = d0.addrs[q] as i128;
        let back = a0 - dm.addrs[q] as i128;
        if back % per as i128 != 0 {
            return 0;
        }
        *st = back / per as i128;
        if dl.addrs[q] as i128 != a0 + (m as i128 - 1) * *st {
            return 0;
        }
    }
    let at = |q: usize, j: i128| (d0.addrs[q] as i128 + j * step[q]) as u64;
    // In one direct memory whose costs repeat with the period: a block.
    if let Some(made) = block(e, &d0, &step, m, per) {
        if made == 0 {
            return 0;
        }
        let mut end = r0;
        for q in 0..nm {
            end[q] = (r0[q] as i128 + made as i128 * s[q]) as u64;
        }
        e.commit(&end);
        return made;
    }
    // What each access of the period measured cost.
    let mut paid = [(0u32, 0u128); 128];
    for jj in 0..per as usize {
        let j = jj as i128 - per as i128;
        for q in 0..sh.accesses {
            match e.cost(at(q, j), n, sh.writes[q]) {
                Some(x) => paid[jj * 2 + q] = x,
                None => return 0,
            }
        }
    }
    let mut made: u64 = 0;
    'run: while made < m {
        for jj in 0..per as usize {
            let j = (made as usize + jj) as i128;
            for q in 0..sh.accesses {
                if e.cost(at(q, j), n, sh.writes[q]) != Some(paid[jj * 2 + q]) {
                    break 'run;
                }
            }
        }
        if sh.cond {
            for jj in 0..per {
                let j = (made + jj) as i128;
                let mut read = [0u128; 2];
                for (q, r) in read.iter_mut().enumerate().take(sh.accesses) {
                    *r = e.peek(at(q, j), n).unwrap_or(0);
                }
                if !e.cont(read) {
                    break 'run;
                }
            }
        }
        for jj in 0..per {
            let j = (made + jj) as i128;
            match sh.kind {
                Kind::Fill => {
                    e.poke(at(0, j), n, d0.value);
                }
                Kind::Copy => {
                    let x = e.peek(at(0, j), n).unwrap_or(0);
                    e.poke(at(1, j), n, x);
                }
                Kind::Read => {}
            }
        }
        made += per;
    }
    if made == 0 {
        return 0;
    }
    let mut end = r0;
    for q in 0..nm {
        end[q] = (r0[q] as i128 + made as i128 * s[q]) as u64;
    }
    e.commit(&end);
    made
}

/// `n` bytes at `o` of `cells`, as a little-endian value.
fn get(cells: &[u8], o: usize, n: usize) -> u128 {
    let mut v = 0u128;
    for (i, b) in cells[o..o + n].iter().enumerate() {
        v |= (*b as u128) << (8 * i);
    }
    v
}

/// Elements `0..m` (element 0 at `d0`, each access moving by `step`) made
/// on the cells of the direct memory holding every access of them and of
/// the period measured before them (`Element::span`), when an access there
/// costs by its address's low bits alone and a period moves each access by
/// whole cost periods: then every period costs what the one measured did,
/// as the loop would check element by element. The elements made (a `while`
/// condition's: the whole periods before the element that ends it), or
/// None: the loop makes them.
fn block<E: Element>(e: &mut E, d0: &Dry, step: &[i128; 2], m: u64, per: u64) -> Option<u64> {
    let sh = E::SHAPE;
    let n = sh.bytes as usize;
    if sh.cost_period == 0 || !per.is_power_of_two() || n == 0 || n > 16 || (per * sh.bytes) % sh.cost_period != 0 {
        return None;
    }
    // Each access's memory, and where element 0's is in its cells.
    let mut mem = [0u32; 2];
    let mut off = [0i128; 2];
    for q in 0..sh.accesses {
        if step[q].unsigned_abs() != n as u128 {
            return None;
        }
        let a0 = d0.addrs[q] as i128;
        let first = a0 - per as i128 * step[q];
        let last = a0 + (m as i128 - 1) * step[q];
        let lo = first.min(last);
        let hi = first.max(last) + n as i128 - 1;
        if lo < 0 || hi > u64::MAX as i128 {
            return None;
        }
        let (k, o) = e.span(lo as u64, (hi - lo + 1) as u64, sh.writes[q])?;
        if o.checked_add((hi - lo) as usize)? >= e.cells(k).len() {
            return None;
        }
        mem[q] = k;
        off[q] = o as i128 + (a0 - lo);
    }
    let at = |q: usize, j: u64| (off[q] + j as i128 * step[q]) as usize;
    // The elements' bytes, from the lowest (each access's, m elements).
    let low = |q: usize| at(q, 0).min(at(q, m - 1));
    let len = m as usize * n;
    match sh.kind {
        Kind::Fill => {
            let v = d0.value.to_le_bytes();
            let cells = e.cells(mem[0]);
            let lo = low(0);
            if n == 1 {
                cells[lo..lo + len].fill(v[0]);
            } else {
                for (i, c) in cells[lo..lo + len].iter_mut().enumerate() {
                    *c = v[i % n];
                }
            }
            Some(m)
        }
        Kind::Copy => {
            if sh.accesses != 2 || mem[0] != mem[1] || step[0] != step[1] {
                return None;
            }
            let (src, dst) = (low(0), low(1));
            let cells = e.cells(mem[0]);
            if src + len <= dst || dst + len <= src {
                cells.copy_within(src..src + len, dst);
            } else {
                // Overlapping: element by element, in order (each reads
                // what the ones before wrote).
                for j in 0..m {
                    let (a, b) = (at(0, j), at(1, j));
                    cells.copy_within(a..a + n, b);
                }
            }
            Some(m)
        }
        Kind::Read if !sh.cond => Some(m),
        Kind::Read => {
            for j in 0..m {
                let mut read = [0u128; 2];
                for (q, r) in read.iter_mut().enumerate().take(sh.accesses) {
                    *r = get(e.cells(mem[q]), at(q, j), n);
                }
                if !e.cont(read) {
                    return Some(j - j % per);
                }
            }
            Some(m)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A machine of 16-bit pointers SI and DI and a count CX (each in a
    /// 32-bit register), 64 KB of memory, accesses costing 2 per 4-byte unit
    /// they span (6 in the top 4 KB), and an element per `kind`.
    struct M {
        r: [u64; MOVING],
        mem: Vec<u8>,
        /// Bytes per element, and moving down.
        bytes: u64,
        down: bool,
        /// The fill's value; a scan's sought byte (and REPNE: while not it).
        value: u8,
        /// Faults where SI or DI is at or past this.
        limit: u64,
        dry_runs: u32,
        commits: u32,
        /// Spans offered (`Element::span`: the block path), and the
        /// element-by-element writes made.
        blocks: bool,
        pokes: u32,
    }

    thread_local! {
        static BLOCKS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    /// `f` element by element, then in blocks: the same either way.
    fn both(f: impl Fn()) {
        for b in [false, true] {
            BLOCKS.with(|x| x.set(b));
            f();
        }
    }

    const FILL: Shape = Shape { kind: Kind::Fill, moving: 2, widths: [16, 16, 0, 0], full: [32, 32, 0, 0], accesses: 1, writes: [true, false], bytes: 1, cond: false, cost_period: 4 };
    const COPY: Shape = Shape { kind: Kind::Copy, moving: 3, widths: [16, 16, 16, 0], full: [32, 32, 32, 0], accesses: 2, writes: [false, true], bytes: 1, cond: false, cost_period: 4 };
    const SCAN: Shape = Shape { kind: Kind::Read, moving: 2, widths: [16, 16, 0, 0], full: [32, 32, 0, 0], accesses: 1, writes: [false, false], bytes: 1, cond: true, cost_period: 4 };

    struct Fill(M);
    struct Copy(M);
    struct Scan(M);

    impl M {
        fn new(_kind: Kind) -> M {
            M { r: [0; MOVING], mem: vec![0; 0x10000], bytes: 1, down: false, value: 0, limit: 0x10000, dry_runs: 0, commits: 0, blocks: BLOCKS.with(|x| x.get()), pokes: 0 }
        }
        fn delta(&self) -> u64 {
            if self.down {
                0u64.wrapping_sub(self.bytes)
            } else {
                self.bytes
            }
        }
        /// The element dry: (SI, DI) pointer slots per kind.
        fn dry(&mut self, ptrs: &[usize]) -> Option<Dry> {
            self.dry_runs += 1;
            let mut addrs = [0u64; 2];
            for (k, p) in ptrs.iter().enumerate() {
                let off = self.r[*p] & 0xFFFF;
                if off + self.bytes > self.limit {
                    return None;
                }
                addrs[k] = off;
            }
            let mut moved = self.r;
            for p in ptrs {
                moved[*p] = (self.r[*p] & !0xFFFF) | (self.r[*p].wrapping_add(self.delta()) & 0xFFFF);
            }
            Some(Dry { addrs, value: self.value as u128, moved })
        }
        fn cost(&self, a: u64, n: u64) -> Option<(u32, u128)> {
            if a + n > 0x10000 {
                return None;
            }
            let units = ((a + n - 1) / 4 - a / 4 + 1) as u128;
            Some((0, units * if a >= 0xF000 { 6 } else { 2 }))
        }
    }

    macro_rules! element {
        ($t:ident, $shape:expr, $ptrs:expr) => {
            impl Element for $t {
                const SHAPE: Shape = $shape;
                fn moving(&mut self) -> [u64; MOVING] {
                    let p: &[usize] = &$ptrs;
                    let mut v = [0; MOVING];
                    for (k, x) in p.iter().enumerate() {
                        v[k] = self.0.r[*x];
                    }
                    v
                }
                fn place(&mut self, v: &[u64; MOVING]) {
                    let p: &[usize] = &$ptrs;
                    for (k, x) in p.iter().enumerate() {
                        self.0.r[*x] = v[k];
                    }
                }
                fn commit(&mut self, v: &[u64; MOVING]) {
                    self.0.commits += 1;
                    self.place(v);
                }
                fn dry(&mut self) -> Option<Dry> {
                    let p: &[usize] = &$ptrs;
                    let d = self.0.dry(&p[..p.len() - 1])?;
                    let mut moved = [0; MOVING];
                    for (k, x) in p.iter().enumerate() {
                        moved[k] = d.moved[*x];
                    }
                    Some(Dry { moved, ..d })
                }
                fn cont(&mut self, read: [u128; 2]) -> bool {
                    read[0] != self.0.value as u128
                }
                fn cost(&mut self, a: u64, n: u64, _write: bool) -> Option<(u32, u128)> {
                    self.0.cost(a, n)
                }
                fn peek(&mut self, a: u64, n: u64) -> Option<u128> {
                    let mut v = 0u128;
                    for i in 0..n {
                        v |= (self.0.mem[(a + i) as usize] as u128) << (8 * i);
                    }
                    Some(v)
                }
                fn span(&mut self, a: u64, len: u64, _write: bool) -> Option<(u32, usize)> {
                    // One cost a unit below 0xF000 and above.
                    let last = a + len - 1;
                    (self.0.blocks && last < 0x10000 && (a >= 0xF000) == (last >= 0xF000)).then_some((0, a as usize))
                }
                fn cells(&mut self, _mem: u32) -> &mut [u8] {
                    &mut self.0.mem
                }
                fn poke(&mut self, a: u64, n: u64, v: u128) -> bool {
                    self.0.pokes += 1;
                    for i in 0..n {
                        self.0.mem[(a + i) as usize] = (v >> (8 * i)) as u8;
                    }
                    true
                }
            }
        };
    }
    // Slots: SI 0, DI 1, CX 2.
    element!(Fill, FILL, [1, 2]);
    element!(Copy, COPY, [0, 1, 2]);
    element!(Scan, SCAN, [1, 2]);

    #[test]
    fn a_block_writes_on_the_cells_and_matches_the_loop() {
        BLOCKS.with(|x| x.set(true));
        let mut m = Fill(M::new(Kind::Fill));
        m.0.value = 0x5A;
        m.0.r = [0, 0x100, 1000, 0];
        assert_eq!(run(&mut m, 512, 4), 512);
        assert!(m.0.mem[0x100..0x300].iter().all(|b| *b == 0x5A));
        assert_eq!(m.0.pokes, 0, "made as a block");
        // A copy into the slower top 4 KB: no one span, element by element.
        let mut m = Copy(M::new(Kind::Copy));
        m.0.r = [0x100, 0xEFF0, 64, 0];
        assert_eq!(run(&mut m, 64, 4), 16);
        assert_eq!(m.0.pokes, 16);
    }

    #[test]
    fn a_fill_fills_whole_periods_and_leaves_the_last() {
        both(|| {
            let mut m = Fill(M::new(Kind::Fill));
            m.0.value = 0x5A;
            m.0.r = [0, 0x100, 30, 0];
            let made = run(&mut m, 64, 4);
            // 29 at most (the last is the step's), in fours.
            assert_eq!(made, 28);
            assert!(m.0.mem[0x100..0x100 + 28].iter().all(|b| *b == 0x5A));
            assert_eq!(m.0.mem[0x100 + 28], 0);
            assert_eq!(m.0.r[1], 0x100 + 28);
            assert_eq!(m.0.r[2], 2);
            // And no more than `max`.
            let mut m = Fill(M::new(Kind::Fill));
            m.0.r = [0, 0x100, 1000, 0];
            assert_eq!(run(&mut m, 18, 4), 16);
        });
    }

    #[test]
    fn it_stops_where_the_cost_changes() {
        both(|| {
            // Into the slower top 4 KB: the period before costs 2 a unit, so the
            // kernel stops at the boundary's period.
            let mut m = Fill(M::new(Kind::Fill));
            m.0.r = [0, 0xEFF0, 64, 0];
            let made = run(&mut m, 64, 4);
            assert_eq!(made, 16);
            assert_eq!(m.0.r[1], 0xF000);
        });
    }

    #[test]
    fn nothing_wraps_and_nothing_faults() {
        both(|| {
            // DI's 16 bits would carry out (its register's upper half is not
            // the element's).
            let mut m = Fill(M::new(Kind::Fill));
            m.0.r = [0, 0x1_FFF0, 64, 0];
            assert_eq!(run(&mut m, 64, 4), 0);
            assert_eq!(m.0.r[1], 0x1_FFF0);
            // A fault at the last element: nothing made, the registers as they
            // were.
            let mut m = Fill(M::new(Kind::Fill));
            m.0.r = [0, 0x100, 64, 0];
            m.0.limit = 0x120;
            assert_eq!(run(&mut m, 64, 4), 0);
            assert_eq!(m.0.r, [0, 0x100, 64, 0]);
            assert_eq!(m.0.commits, 0);
        });
    }

    #[test]
    fn an_overlapping_copy_copies_as_the_elements_would() {
        both(|| {
            // Forward by one: each element copies what the last one wrote.
            let mut m = Copy(M::new(Kind::Copy));
            m.0.mem[0x200] = 7;
            m.0.r = [0x200, 0x201, 40, 0];
            let made = run(&mut m, 64, 4);
            assert_eq!(made, 36);
            assert!(m.0.mem[0x201..0x201 + 36].iter().all(|b| *b == 7));
            assert_eq!((m.0.r[0], m.0.r[1], m.0.r[2]), (0x200 + 36, 0x201 + 36, 4));
            // Backward (DF set).
            let mut m = Copy(M::new(Kind::Copy));
            for i in 0..64 {
                m.0.mem[0x300 + i] = i as u8;
            }
            m.0.down = true;
            m.0.r = [0x33F, 0x37F, 64, 0];
            let made = run(&mut m, 64, 4);
            assert_eq!(made, 60);
            for i in 0..60 {
                assert_eq!(m.0.mem[0x37F - i], (63 - i) as u8);
            }
        });
    }

    #[test]
    fn a_scan_stops_before_the_element_that_ends_it() {
        both(|| {
            let mut m = Scan(M::new(Kind::Read));
            m.0.value = 0xA5;
            m.0.r = [0, 0x400, 100, 0];
            m.0.mem[0x400 + 13] = 0xA5;
            let made = run(&mut m, 64, 4);
            // Element 13 ends it: the kernel makes the whole periods before it.
            assert_eq!(made, 12);
            assert_eq!(m.0.r[1], 0x400 + 12);
            assert_eq!(m.0.r[2], 88);
        });
    }
}
