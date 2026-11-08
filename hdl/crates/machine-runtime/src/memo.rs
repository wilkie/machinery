//! Memoised model fns (`memo fn`, SPEC 8.3b): what a call read, and whether
//! it is still what the state holds.
//!
//! Every write to state a memo fn reads stamps it with a new version (the
//! component's epoch, counted up). A call records what it read: a bit per
//! state variable, and ranges of blocks of each memory. A later call with
//! the same arguments may be skipped while nothing it read has a version
//! newer than the call.

/// A memo fn's calls, by their arguments. (A multiply-rotate hash: the
/// arguments are the model's own, small and few; SipHash's resistance buys
/// nothing here, at a cost a scanline.)
pub type Map<K> = std::collections::HashMap<K, Entry, std::hash::BuildHasherDefault<FxHasher>>;

/// The hasher `Map` uses.
#[derive(Clone, Copy, Debug, Default)]
pub struct FxHasher(u64);

impl std::hash::Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.write_u64(*b as u64);
        }
    }
    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.write_u64(i as u64);
    }
    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.write_u64(i as u64);
    }
    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.write_u64(i as u64);
    }
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

/// Cells in a block of a memory: a write stamps its block.
pub const BLOCK_SHIFT: u32 = 6;

/// What a memo fn's call is reading, as it runs.
#[derive(Clone, Debug, Default)]
pub struct Rec {
    /// Bit `k`: state variable `k` (the component's numbering).
    pub vars: u64,
    /// `(memory, first block, last block)`, merged as they come.
    pub ranges: Vec<(u8, u32, u32)>,
    /// Per memory (by number, mod 4): the range its last read fell in (most
    /// reads fall in it again).
    last: [usize; 4],
}

impl Rec {
    #[inline]
    pub fn var(&mut self, k: u32) {
        self.vars |= 1 << k;
    }

    /// Cell `i` of memory `m` was read.
    #[inline]
    pub fn cell(&mut self, m: u8, i: usize) {
        let b = (i >> BLOCK_SHIFT) as u32;
        if let Some(&(lm, lo, hi)) = self.ranges.get(self.last[m as usize & 3]) {
            if lm == m && b >= lo && b <= hi {
                return;
            }
        }
        self.block(m, b);
    }

    /// Reads interleave a few streams (the planes of one address, the
    /// DAC): try the last few ranges before starting another. A range
    /// takes in blocks up to `MERGE` away: recording more than was read
    /// only costs a redraw now and then, and keeps the list short.
    #[inline(never)]
    fn block(&mut self, m: u8, b: u32) {
        const MERGE: u32 = 64;
        let n = self.ranges.len();
        for k in (n.saturating_sub(8)..n).rev() {
            let r = &mut self.ranges[k];
            if r.0 == m && b + MERGE >= r.1 && b <= r.2 + MERGE {
                r.1 = r.1.min(b);
                r.2 = r.2.max(b);
                self.last[m as usize & 3] = k;
                return;
            }
        }
        self.ranges.push((m, b, b));
        self.last[m as usize & 3] = n;
    }
}

/// A finished call: when it ran (the epoch after it), and what it read.
#[derive(Clone, Debug)]
pub struct Entry {
    pub at: u64,
    pub vars: u64,
    pub ranges: Vec<(u8, u32, u32)>,
}

impl Entry {
    /// `valid`, the component at epoch `now`: nothing has a version newer
    /// than `now`, so with no write since the call (`at == now`) nothing it
    /// read changed, and none of its range versions need looking at. Found
    /// valid, the call is as good as made now: later checks start from
    /// here.
    #[inline]
    pub fn check<'a>(&mut self, now: u64, var: impl Fn(u32) -> u64, mem: impl Fn(u8) -> (&'a [u64], u64)) -> bool {
        if self.at == now {
            return true;
        }
        if self.valid(var, mem) {
            self.at = now;
            return true;
        }
        false
    }

    /// Whether everything the call read is unchanged: no version newer
    /// than the call. `var(k)` is state variable `k`'s version, `mem(m)` a
    /// memory's block versions and its whole-memory version.
    #[inline]
    pub fn valid<'a>(&self, var: impl Fn(u32) -> u64, mem: impl Fn(u8) -> (&'a [u64], u64)) -> bool {
        let mut v = self.vars;
        while v != 0 {
            let k = v.trailing_zeros();
            if var(k) > self.at {
                return false;
            }
            v &= v - 1;
        }
        for &(m, lo, hi) in &self.ranges {
            let (blocks, all) = mem(m);
            if all > self.at {
                return false;
            }
            if blocks[lo as usize..=hi as usize].iter().any(|&b| b > self.at) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_merge_interleaved_streams() {
        let mut r = Rec::default();
        for a in 0..256usize {
            for p in 0..4usize {
                r.cell(0, p * 0x10000 + a);
            }
        }
        assert_eq!(r.ranges.len(), 4);
        assert!(r.ranges.contains(&(0, 0x10000 >> 6, (0x10000 + 255) >> 6)));
    }

    #[test]
    fn a_newer_version_invalidates() {
        let mut r = Rec::default();
        r.var(3);
        r.cell(1, 130);
        let e = Entry { at: 10, vars: r.vars, ranges: r.ranges };
        let blocks = vec![0u64; 8];
        assert!(e.valid(|_| 5, |_| (&blocks, 0)));
        assert!(!e.valid(|k| if k == 3 { 11 } else { 0 }, |_| (&blocks, 0)));
        let mut b2 = blocks.clone();
        b2[2] = 11;
        assert!(!e.valid(|_| 0, |_| (&b2, 0)));
        b2[2] = 0;
        b2[3] = 11;
        assert!(e.valid(|_| 0, |_| (&b2, 0)), "another block");
        assert!(!e.valid(|_| 0, |_| (&blocks, 11)), "the whole memory");
    }

    #[test]
    fn a_check_found_valid_starts_later_checks_from_now() {
        let mut r = Rec::default();
        r.cell(0, 70);
        let mut e = Entry { at: 10, vars: r.vars, ranges: r.ranges };
        let mut blocks = vec![0u64; 4];
        // Another block written at 12: still valid, and as of 12.
        blocks[3] = 12;
        assert!(e.check(12, |_| 0, |_| (&blocks, 0)));
        assert_eq!(e.at, 12);
        // Nothing written since: valid without looking.
        assert!(e.check(12, |_| 0, |_| (&[][..], 99)));
        // Its block written at 13.
        blocks[1] = 13;
        assert!(!e.check(13, |_| 0, |_| (&blocks, 0)));
    }
}
