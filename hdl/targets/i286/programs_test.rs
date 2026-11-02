//! Runs the self-checking 286 test programs of `simulators/test/i286`
//! (`simple`, `complex` and `protected`, assembled to `.com` files by
//! `assemble-tests.sh`) through the interpreter generated from
//! `i286.machine` (`machine build --isa I286`).
//!
//! The programs' harness, as the earlier TypeScript simulator provided it:
//! loaded at 0000:0100; `INT 0x23` asserts AX == BX and `INT 0x22` asserts
//! AL == AH; `INT 0x21` with AH = 0x25 sets a real-mode interrupt vector and
//! with AH = 0x4C (or 0) ends the program. Those three are handled here in
//! place of the instruction; every other one runs as the processor would.
//!
//! The programs were written against that simulator, whose machine status
//! word started at 0; the 286 resets it to FFF0, which SMSW reads in real
//! mode. The harness starts from MSW = 0, as the programs expect.
//!
//! `KNOWN` lists the programs whose expectations differ from the 286 itself,
//! with the reason; they are run, and reported, but not required to pass.
//!
//! `PROGRAMS` (comma-separated `suite/name`, `protected/ldt`) chooses a
//! subset; `PROGRAMS_LOG` prints each failure's details; `PROGRAMS_DIR`
//! names the suite directory when the crate is not under the repository.

use std::path::{Path, PathBuf};

use machine_i286::{Segment, I286};

fn suite_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("PROGRAMS_DIR") {
        return Some(PathBuf::from(d));
    }
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = here.as_path();
    loop {
        let c = dir.join("simulators/test/i286");
        if c.join("simple").exists() {
            return Some(c);
        }
        dir = dir.parent()?;
    }
}

/// Programs that expect what the 286 does not do.
const KNOWN: &[(&str, &str)] = &[(
    "protected/iopl",
    "its #GP handler loads DS with a DPL 0 selector and IRETs to ring 3, where the 286 nulls DS (a segment the outer level may not use); the next store through DS faults",
)];

/// How a program ended.
#[derive(Debug, PartialEq)]
enum End {
    Passed,
    /// An assertion failed at this CS:IP.
    Assertion(u16, u16, String),
    /// The processor faulted out of the interpreter, or ran too long.
    Stuck(String),
}

fn run(image: &[u8]) -> End {
    let mut m = I286::new();
    m.reset();
    m.r.MSW = 0;
    m.r.CS = Segment { selector: 0, base: 0, ..m.r.CS };
    m.r.DS = Segment { selector: 0, base: 0, ..m.r.DS };
    m.r.ES = Segment { selector: 0, base: 0, ..m.r.ES };
    m.r.SS = Segment { selector: 0, base: 0, ..m.r.SS };
    m.r.IP = 0x100;
    for (i, b) in image.iter().enumerate() {
        m.bus.mem.write_cell(0x100 + i as u64, *b as u128);
    }
    for _ in 0..1_000_000 {
        let at = m.r.CS.base as u64 + m.r.IP as u64;
        let op = m.bus.mem.read_cell(at) as u8;
        let n = m.bus.mem.read_cell(at + 1) as u8;
        if op == 0xCD && (0x21..=0x23).contains(&n) {
            let (ax, bx) = (m.r.AX, m.r.BX);
            match n {
                0x23 if ax != bx => {
                    return End::Assertion(m.r.CS.selector, m.r.IP, format!("AX={ax:04x} BX={bx:04x}"))
                }
                0x22 if ax as u8 != (ax >> 8) as u8 => {
                    return End::Assertion(m.r.CS.selector, m.r.IP, format!("AL={:02x} AH={:02x}", ax as u8, ax >> 8))
                }
                0x21 => match (ax >> 8) as u8 {
                    0x25 => {
                        let entry = (ax & 0xFF) as u64 * 4;
                        let (dx, ds) = (m.r.DX, m.r.DS.selector);
                        m.bus.mem.write_cell(entry, (dx & 0xFF) as u128);
                        m.bus.mem.write_cell(entry + 1, (dx >> 8) as u128);
                        m.bus.mem.write_cell(entry + 2, (ds & 0xFF) as u128);
                        m.bus.mem.write_cell(entry + 3, (ds >> 8) as u128);
                    }
                    0x4C | 0x00 => return End::Passed,
                    _ => {}
                },
                _ => {}
            }
            m.r.IP = m.r.IP.wrapping_add(2);
            continue;
        }
        if m.halted {
            return End::Stuck(format!("halted at {:04x}:{:04x}", m.r.CS.selector, m.r.IP));
        }
        if let Err(f) = m.step() {
            return End::Stuck(format!("{f:?} at {:04x}:{:04x}", m.r.CS.selector, m.r.IP));
        }
    }
    End::Stuck(format!("still running at {:04x}:{:04x}", m.r.CS.selector, m.r.IP))
}

fn programs(dir: &Path, suite: &str) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir.join(suite).join("bin")) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "com") {
            let name = p.file_stem().unwrap().to_string_lossy().into_owned();
            out.push((format!("{suite}/{name}"), std::fs::read(&p).unwrap()));
        }
    }
    out.sort();
    out
}

/// Runs a suite and returns the names that did not pass.
fn suite(name: &str) -> (usize, Vec<String>) {
    let Some(dir) = suite_dir() else {
        eprintln!("simulators/test/i286 not found; skipping");
        return (0, Vec::new());
    };
    let only = std::env::var("PROGRAMS").ok();
    let log = std::env::var("PROGRAMS_LOG").is_ok();
    let mut failed = Vec::new();
    let mut n = 0;
    let mut known = 0;
    for (prog, image) in programs(&dir, name) {
        if let Some(only) = &only {
            if !only.split(',').any(|p| p == prog) {
                continue;
            }
        }
        let end = run(&image);
        if let Some((_, why)) = KNOWN.iter().find(|(p, _)| *p == prog) {
            println!("{prog}: {end:?} (known: {why})");
            known += 1;
            continue;
        }
        n += 1;
        if end != End::Passed {
            if log {
                println!("{prog}: {end:?}");
            }
            failed.push(prog);
        }
    }
    println!("{name}: {} of {n} passed ({known} known divergence(s) not counted)", n - failed.len());
    (n, failed)
}

#[test]
fn simple_programs_pass() {
    let (_, failed) = suite("simple");
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
fn complex_programs_pass() {
    let (_, failed) = suite("complex");
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
fn protected_programs_pass() {
    let (_, failed) = suite("protected");
    assert!(failed.is_empty(), "{failed:?}");
}
