//! Test vectors: the SingleStepTests MOO format (version 1), as produced by
//! the ArduinoX86 hardware rigs. A file holds many tests; each test has an
//! initial and final register file and RAM, the instruction bytes, an
//! optional exception, and a hash used by the revocation list.
//!
//! Format: <https://github.com/dbalsom/moo/blob/main/doc/moo_format_v1.md>.
//! Only the chunks needed to check architectural state are decoded.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The 8086/286 register file order used by MOO `REGS` chunks.
pub const MOO_REGS: [&str; 14] = [
    "ax", "bx", "cx", "dx", "cs", "ss", "ds", "es", "sp", "bp", "si", "di", "ip", "flags",
];

/// The 386 register file order used by MOO `RG32` (and `RM32`) chunks.
/// Segment registers hold their selector in the low 16 bits.
pub const MOO_REGS32: [&str; 20] = [
    "cr0", "cr3", "eax", "ebx", "ecx", "edx", "esi", "edi", "ebp", "esp", "cs", "ds", "es", "fs",
    "gs", "ss", "eip", "eflags", "dr6", "dr7",
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CpuState {
    /// `(register name, value)` for the registers present.
    pub regs: Vec<(&'static str, u32)>,
    /// `(linear address, byte)`.
    pub ram: Vec<(u32, u8)>,
    /// `(register name, mask)` of bits the hardware leaves undefined
    /// (`RMSK`/`RM32`, inverted), when the test or its file gives them.
    pub undefined: Vec<(&'static str, u32)>,
}

impl CpuState {
    pub fn reg(&self, name: &str) -> Option<u32> {
        self.regs.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MooTest {
    pub index: u32,
    pub name: String,
    pub bytes: Vec<u8>,
    pub initial: CpuState,
    pub final_: CpuState,
    /// Exception vector raised by the instruction, if any.
    pub exception: Option<u8>,
    /// Lower-case hex.
    pub hash: Option<String>,
    /// The hardware's cycle trace, summarised, when the file has one.
    pub timing: Option<CycleSummary>,
}

/// What timing needs from a captured cycle trace: every cycle from the jump
/// that starts the test to the HALT that ends it, and how many of them
/// belong to bus cycles.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CycleSummary {
    /// Cycles in the trace.
    pub total: u32,
    /// Code-fetch bus cycles (each one starts with ALE).
    pub code: u32,
    /// Memory, I/O and interrupt-acknowledge bus cycles.
    pub data: u32,
    /// Every bus cycle, in order: the cycle it started at (counted from the
    /// test's start) and what it was.
    pub bus: Vec<(u32, BusKind)>,
}

/// What a bus cycle was, from the processor's status lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusKind {
    Code,
    MemRead,
    MemWrite,
    IoRead,
    IoWrite,
    Halt,
    InterruptAck,
    Other,
}

/// Which processor captured a file's traces (the `MOO ` header names it):
/// their status lines are encoded differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceCpu {
    /// The 286: COD/INTA, M/IO, S1, S0.
    I286,
    /// The 386 (the 386EX of the 80386 corpus): M/IO, D/C, W/R.
    I386,
}

impl CycleSummary {
    /// Clocks per bus cycle with no wait states: Ts, Tc on the 286, T1, T2
    /// on the 386.
    pub const BUS_CLOCKS: u32 = 2;

    /// The cycles not spent in bus cycles, less `harness` (the fixed cost
    /// of the test's start and end that no instruction owns): the
    /// execution unit's time beyond what the bus did.
    pub fn execution(&self, harness: u32) -> i64 {
        self.total as i64 - (Self::BUS_CLOCKS * (self.code + self.data)) as i64 - harness as i64
    }
}

/// One record of a `CYCL` chunk (the corpus tool's `<B I B B B B H B B B B>`):
/// pins, address, segment, memory and I/O status, BHE, data, bus status,
/// T-state, queue operation, queue byte.
const CYCLE_RECORD: usize = 1 + 4 + 1 + 1 + 1 + 1 + 2 + 1 + 1 + 1 + 1;

fn decode_cycles(body: &[u8], cpu: TraceCpu) -> Result<CycleSummary, MooError> {
    let mut r = Reader { buf: body, pos: 0 };
    let n = r.u32()? as usize;
    let mut s = CycleSummary {
        total: n as u32,
        ..CycleSummary::default()
    };
    for k in 0..n {
        let rec = r.bytes(CYCLE_RECORD)?;
        let ale = rec[0] & 1 != 0;
        let status = rec[11] & 0x0F;
        if ale {
            // The status as the corpus tool decodes it: the 286's COD/INTA,
            // M/IO, S1, S0; the 386's M/IO, D/C, W/R (ALE is ADS there).
            let kind = match (cpu, status) {
                (TraceCpu::I286, 0x00) => BusKind::InterruptAck,
                (TraceCpu::I286, 0x04) => BusKind::Halt,
                (TraceCpu::I286, 0x05) => BusKind::MemRead,
                (TraceCpu::I286, 0x06) => BusKind::MemWrite,
                (TraceCpu::I286, 0x09) => BusKind::IoRead,
                (TraceCpu::I286, 0x0A) => BusKind::IoWrite,
                (TraceCpu::I286, 0x0D) => BusKind::Code,
                (TraceCpu::I386, 0x00) => BusKind::InterruptAck,
                (TraceCpu::I386, 0x02) => BusKind::IoRead,
                (TraceCpu::I386, 0x03) => BusKind::IoWrite,
                (TraceCpu::I386, 0x04) => BusKind::Code,
                (TraceCpu::I386, 0x05) => BusKind::Halt,
                (TraceCpu::I386, 0x06) => BusKind::MemRead,
                (TraceCpu::I386, 0x07) => BusKind::MemWrite,
                _ => BusKind::Other,
            };
            match kind {
                BusKind::Code => s.code += 1,
                BusKind::InterruptAck
                | BusKind::MemRead
                | BusKind::MemWrite
                | BusKind::IoRead
                | BusKind::IoWrite => s.data += 1,
                _ => {}
            }
            s.bus.push((k as u32, kind));
        }
    }
    Ok(s)
}

#[derive(Debug)]
pub struct MooError(pub String);

impl std::fmt::Display for MooError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MooError {}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn need(&self, n: usize) -> Result<(), MooError> {
        if self.pos + n > self.buf.len() {
            Err(MooError(format!("truncated at offset {}", self.pos)))
        } else {
            Ok(())
        }
    }
    fn u8(&mut self) -> Result<u8, MooError> {
        self.need(1)?;
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }
    fn u16(&mut self) -> Result<u16, MooError> {
        self.need(2)?;
        let v = u16::from_le_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }
    fn u32(&mut self) -> Result<u32, MooError> {
        self.need(4)?;
        let b = &self.buf[self.pos..self.pos + 4];
        self.pos += 4;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], MooError> {
        self.need(n)?;
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn chunk(&mut self) -> Result<([u8; 4], &'a [u8]), MooError> {
        let tag = self.bytes(4)?;
        let len = self.u32()? as usize;
        let body = self.bytes(len)?;
        Ok(([tag[0], tag[1], tag[2], tag[3]], body))
    }
    fn done(&self) -> bool {
        self.pos >= self.buf.len()
    }
}

fn decode_state(body: &[u8]) -> Result<CpuState, MooError> {
    let mut r = Reader { buf: body, pos: 0 };
    let mut st = CpuState::default();
    while !r.done() {
        let (tag, chunk) = r.chunk()?;
        let mut c = Reader { buf: chunk, pos: 0 };
        match &tag {
            b"REGS" => {
                let mask = c.u16()?;
                for (i, name) in MOO_REGS.iter().enumerate() {
                    if mask & (1 << i) != 0 {
                        st.regs.push((name, c.u16()? as u32));
                    }
                }
            }
            b"RG32" | b"RM32" => {
                let mask = c.u32()?;
                for (i, name) in MOO_REGS32.iter().enumerate() {
                    if mask & (1 << i) != 0 {
                        let v = c.u32()?;
                        if &tag == b"RG32" {
                            st.regs.push((name, v));
                        } else {
                            // A mask of the bits to compare.
                            st.undefined.push((name, !v));
                        }
                    }
                }
            }
            b"RAM " => {
                let n = c.u32()?;
                for _ in 0..n {
                    let a = c.u32()?;
                    let v = c.u8()?;
                    st.ram.push((a, v));
                }
            }
            _ => {}
        }
    }
    Ok(st)
}

fn decode_test(body: &[u8], cpu: TraceCpu) -> Result<MooTest, MooError> {
    let mut r = Reader { buf: body, pos: 0 };
    let mut t = MooTest {
        index: r.u32()?,
        ..MooTest::default()
    };
    while !r.done() {
        let (tag, chunk) = r.chunk()?;
        let mut c = Reader { buf: chunk, pos: 0 };
        match &tag {
            b"NAME" => {
                let n = c.u32()? as usize;
                t.name = String::from_utf8_lossy(c.bytes(n)?).into_owned();
            }
            b"BYTS" => {
                let n = c.u32()? as usize;
                t.bytes = c.bytes(n)?.to_vec();
            }
            b"INIT" => t.initial = decode_state(chunk)?,
            b"FINA" => t.final_ = decode_state(chunk)?,
            b"EXCP" => t.exception = Some(c.u8()?),
            b"CYCL" => t.timing = Some(decode_cycles(chunk, cpu)?),
            b"HASH" => t.hash = Some(chunk.iter().map(|b| format!("{b:02x}")).collect()),
            _ => {}
        }
    }
    Ok(t)
}

/// Parse an (uncompressed) MOO file.
pub fn parse_moo(data: &[u8]) -> Result<Vec<MooTest>, MooError> {
    let mut r = Reader { buf: data, pos: 0 };
    let (tag, header) = r.chunk()?;
    if &tag != b"MOO " {
        return Err(MooError("not a MOO file (missing `MOO ` header)".into()));
    }
    // The header ends with the CPU's name (`C286`, `386E`).
    let cpu = if header.windows(3).any(|w| w == b"386") {
        TraceCpu::I386
    } else {
        TraceCpu::I286
    };
    let mut tests = Vec::new();
    // A file-level `RM32` masks every test's final state (the bits to compare).
    let mut file_masks: Vec<(&'static str, u32)> = Vec::new();
    while !r.done() {
        let (tag, body) = r.chunk()?;
        if &tag == b"RM32" {
            let st = decode_state(&[&tag[..], &(body.len() as u32).to_le_bytes(), body].concat())?;
            file_masks = st.undefined;
        } else if &tag == b"TEST" {
            let mut t = decode_test(body, cpu)?;
            t.final_.undefined.extend(file_masks.iter().copied());
            tests.push(t);
        }
    }
    Ok(tests)
}

/// Read a `.MOO` or `.MOO.gz` file.
pub fn read_moo(path: &Path) -> Result<Vec<MooTest>, MooError> {
    let raw = std::fs::read(path).map_err(|e| MooError(format!("{}: {e}", path.display())))?;
    let data = if raw.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&raw[..])
            .read_to_end(&mut out)
            .map_err(|e| MooError(format!("{}: {e}", path.display())))?;
        out
    } else {
        raw
    };
    parse_moo(&data).map_err(|e| MooError(format!("{}: {e}", path.display())))
}

/// Hashes listed in `revocation_list.txt` next to (or one level above)
/// the vector directory.
pub fn revocations(dir: &Path) -> HashSet<String> {
    let mut out = HashSet::new();
    for candidate in [
        dir.join("revocation_list.txt"),
        dir.join("..").join("revocation_list.txt"),
    ] {
        if let Ok(text) = std::fs::read_to_string(&candidate) {
            for line in text.lines() {
                let h = line.trim().to_ascii_lowercase();
                if !h.is_empty() && !h.starts_with('#') {
                    out.insert(h);
                }
            }
        }
    }
    out
}

/// Expand a glob whose `*` wildcards appear only in the final component.
pub fn glob(pattern: &str) -> Vec<PathBuf> {
    let p = Path::new(pattern);
    let dir = p.parent().unwrap_or(Path::new("."));
    let Some(file_pat) = p.file_name().and_then(|f| f.to_str()) else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| wildcard(file_pat, n))
        })
        .map(|e| e.path())
        .collect();
    out.sort();
    out
}

fn wildcard(pat: &str, s: &str) -> bool {
    match pat.split_once('*') {
        None => pat == s,
        Some((head, rest)) => {
            if !s.starts_with(head) {
                return false;
            }
            let s = &s[head.len()..];
            (0..=s.len()).any(|i| s.is_char_boundary(i) && wildcard(rest, &s[i..]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards() {
        assert!(wildcard("*.MOO.gz", "00.MOO.gz"));
        assert!(wildcard("8?", "8?"));
        assert!(wildcard("80.*.MOO.gz", "80.3.MOO.gz"));
        assert!(!wildcard("*.MOO.gz", "metadata.json"));
    }

    #[test]
    fn parses_a_synthetic_file() {
        let mut test = Vec::new();
        test.extend_from_slice(&7u32.to_le_bytes());
        let chunk = |tag: &[u8; 4], body: &[u8], out: &mut Vec<u8>| {
            out.extend_from_slice(tag);
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(body);
        };
        let mut name = 3u32.to_le_bytes().to_vec();
        name.extend_from_slice(b"nop");
        chunk(b"NAME", &name, &mut test);
        let mut regs = 0b1u16.to_le_bytes().to_vec(); // ax only
        regs.extend_from_slice(&0x1234u16.to_le_bytes());
        let mut init = Vec::new();
        chunk(b"REGS", &regs, &mut init);
        chunk(b"INIT", &init, &mut test);
        chunk(b"EXCP", &[6, 0, 0, 0, 0], &mut test);
        let mut file = Vec::new();
        chunk(
            b"MOO ",
            &[1, 1, 0, 0, 1, 0, 0, 0, b'C', b'2', b'8', b'6'],
            &mut file,
        );
        chunk(b"TEST", &test, &mut file);
        let tests = parse_moo(&file).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].index, 7);
        assert_eq!(tests[0].name, "nop");
        assert_eq!(tests[0].initial.reg("ax"), Some(0x1234));
        assert_eq!(tests[0].exception, Some(6));
    }
}
