//! `machine-runtime`: what generated code links against.
//!
//! Kept deliberately small: a sparse memory for address spaces, the
//! identity of architectural places (for tests that name operands by
//! register), and the fault classes.

use std::collections::HashMap;

pub mod bulk;
pub mod memo;
#[cfg(feature = "vectors")]
pub mod vectors;

/// The class of a fault, as declared in the isa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultClass {
    /// Restores the instruction pointer and discards the instruction's
    /// architectural writes.
    Fault,
    /// Reported after the instruction commits.
    Trap,
    /// Not restartable.
    Abort,
}

/// The identity of an architectural place: a register, a register field, a
/// register view or the mode register. Indices are the isa's declaration
/// order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlaceRef {
    Reg(u16),
    Field(u16, u16),
    View(u16, u16),
    Mode,
}

const PAGE_BITS: u32 = 12;
const PAGE_SIZE: usize = 1 << PAGE_BITS;

/// A sparse address space: cells of `cell_bits` bits, allocated in pages on
/// first write. Unwritten cells read as zero.
#[derive(Clone, Debug)]
pub struct Memory {
    cell_bits: u32,
    cell_bytes: usize,
    little_endian: bool,
    pages: HashMap<u64, Box<[u8]>>,
    /// Nothing attached: every read returns this, writes are discarded.
    open_bus: Option<u128>,
}

impl Memory {
    pub fn new(cell_bits: u32, little_endian: bool) -> Memory {
        assert!((1..=128).contains(&cell_bits), "cell width {cell_bits}");
        Memory {
            cell_bits,
            cell_bytes: cell_bits.div_ceil(8) as usize,
            little_endian,
            pages: HashMap::new(),
            open_bus: None,
        }
    }

    /// Detach the space: reads return `value` (masked to a cell), writes are
    /// discarded. Models an empty bus, as in a test rig.
    pub fn set_open_bus(&mut self, value: u128) {
        self.open_bus = Some(value & self.cell_mask());
    }

    pub fn cell_bits(&self) -> u32 {
        self.cell_bits
    }

    fn cell_mask(&self) -> u128 {
        if self.cell_bits >= 128 {
            u128::MAX
        } else {
            (1u128 << self.cell_bits) - 1
        }
    }

    fn byte(&self, index: u64) -> u8 {
        let page = index >> PAGE_BITS;
        let off = (index as usize) & (PAGE_SIZE - 1);
        self.pages.get(&page).map(|p| p[off]).unwrap_or(0)
    }

    fn set_byte(&mut self, index: u64, v: u8) {
        let page = index >> PAGE_BITS;
        let off = (index as usize) & (PAGE_SIZE - 1);
        let p = self
            .pages
            .entry(page)
            .or_insert_with(|| vec![0u8; PAGE_SIZE].into_boxed_slice());
        p[off] = v;
    }

    /// One cell.
    pub fn read_cell(&self, addr: u64) -> u128 {
        if let Some(v) = self.open_bus {
            return v;
        }
        let base = addr.wrapping_mul(self.cell_bytes as u64);
        let mut v: u128 = 0;
        for i in 0..self.cell_bytes {
            v |= (self.byte(base.wrapping_add(i as u64)) as u128) << (8 * i);
        }
        v & self.cell_mask()
    }

    pub fn write_cell(&mut self, addr: u64, v: u128) {
        if self.open_bus.is_some() {
            return;
        }
        let base = addr.wrapping_mul(self.cell_bytes as u64);
        let v = v & self.cell_mask();
        for i in 0..self.cell_bytes {
            self.set_byte(base.wrapping_add(i as u64), (v >> (8 * i)) as u8);
        }
    }

    /// `bits / cell_bits` consecutive cells starting at `addr`, assembled in
    /// the space's byte order.
    pub fn read(&self, addr: u64, bits: u32) -> u128 {
        let n = (bits / self.cell_bits) as u64;
        let mut v: u128 = 0;
        for i in 0..n {
            let cell = self.read_cell(addr.wrapping_add(i));
            let shift = if self.little_endian { i } else { n - 1 - i } as u32 * self.cell_bits;
            v |= cell << shift;
        }
        v
    }

    pub fn write(&mut self, addr: u64, bits: u32, v: u128) {
        let n = (bits / self.cell_bits) as u64;
        let mask = self.cell_mask();
        for i in 0..n {
            let shift = if self.little_endian { i } else { n - 1 - i } as u32 * self.cell_bits;
            self.write_cell(addr.wrapping_add(i), (v >> shift) & mask);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_little_endian() {
        let mut m = Memory::new(8, true);
        m.write(0x100, 16, 0x1234);
        assert_eq!(m.read_cell(0x100), 0x34);
        assert_eq!(m.read_cell(0x101), 0x12);
        assert_eq!(m.read(0x100, 16), 0x1234);
        assert_eq!(m.read(0x200, 32), 0);
    }

    #[test]
    fn bytes_big_endian() {
        let mut m = Memory::new(8, false);
        m.write(0, 16, 0x1234);
        assert_eq!(m.read_cell(0), 0x12);
        assert_eq!(m.read(0, 16), 0x1234);
    }

    #[test]
    fn open_bus_reads_its_value() {
        let mut m = Memory::new(8, true);
        m.set_open_bus(0xFF);
        m.write(0x10, 16, 0x1234);
        assert_eq!(m.read(0x10, 16), 0xFFFF);
    }

    #[test]
    fn wide_cells() {
        let mut m = Memory::new(12, true);
        m.write_cell(5, 0xFFFF);
        assert_eq!(m.read_cell(5), 0xFFF);
        assert_eq!(m.read_cell(4), 0);
    }
}
