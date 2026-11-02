//! Runs self-checking test programs through the interpreter generated from
//! `i386.machine` (`machine build --isa I386`): the 286's programs
//! (`simulators/test/i286/{simple,complex,protected}`), which the 386 runs
//! as a 286 would, and the 386's own (`simulators/test/i386/protected`:
//! 32-bit segments, gates and tasks, paging, virtual-8086 mode). Each
//! suite is assembled to `.com` files by its `assemble-tests.sh`.
//!
//! The harness, as the 286's: loaded at 0000:0100; `INT 0x23` asserts
//! AX == BX, `INT 0x22` AL == AH, and (for the 386's programs) `INT 0x24`
//! EAX == EBX; `INT 0x21` with AH = 0x25 sets a real-mode interrupt vector
//! and with AH = 0x4C (or 0) ends the program. Those are handled here in
//! place of the instruction, in any mode; every other one runs as the
//! processor would. The instruction is found through CS's base and EIP
//! directly, so a program that pages must map its code at the same
//! physical address.
//!
//! The programs start with CR0 = 0, as the 286's expect of the MSW.
//!
//! `KNOWN` lists the programs whose expectations differ from the 386, with
//! the reason; they are run and reported but not required to pass.
//!
//! `PROGRAMS` (comma-separated `suite/name`) chooses a subset;
//! `PROGRAMS_LOG` prints each failure's details; `PROGRAMS_TRACE` prints
//! the steps (CS:EIP, ESP, and any fault delivered) leading to the first
//! fault, and the last steps of a stuck program; `PROGRAMS_DIR` names the
//! `simulators/test` directory when the crate is not under the repository.

use std::path::{Path, PathBuf};

use machine_i386::{Segment, I386};

fn test_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("PROGRAMS_DIR") {
        return Some(PathBuf::from(d));
    }
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = here.as_path();
    loop {
        let c = dir.join("simulators/test");
        if c.join("i286").join("simple").exists() {
            return Some(c);
        }
        dir = dir.parent()?;
    }
}

/// Programs that expect what the 386 does not do.
const KNOWN: &[(&str, &str)] = &[(
    "i286/protected/iopl",
    "its #GP handler loads DS with a DPL 0 selector and IRETs to ring 3, where the processor nulls DS (a segment the outer level may not use); the next store through DS faults",
)];

/// How a program ended.
#[derive(Debug, PartialEq)]
enum End {
    Passed,
    /// An assertion failed at this CS:EIP.
    Assertion(u16, u32, String),
    /// The processor faulted out of the interpreter, or ran too long.
    Stuck(String),
}

fn run(image: &[u8]) -> End {
    let mut m = I386::new();
    m.reset();
    m.r.CR0 = 0;
    m.r.CS = Segment { selector: 0, base: 0, ..m.r.CS };
    m.r.DS = Segment { selector: 0, base: 0, ..m.r.DS };
    m.r.ES = Segment { selector: 0, base: 0, ..m.r.ES };
    m.r.SS = Segment { selector: 0, base: 0, ..m.r.SS };
    m.r.FS = Segment { selector: 0, base: 0, ..m.r.FS };
    m.r.GS = Segment { selector: 0, base: 0, ..m.r.GS };
    m.r.EIP = 0x100;
    for (i, b) in image.iter().enumerate() {
        m.bus.mem.write_cell(0x100 + i as u64, *b as u128);
    }
    let trace = std::env::var("PROGRAMS_TRACE").is_ok();
    let mut recent: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    let mut first_fault = true;
    let stuck = |why: String, recent: &std::collections::VecDeque<String>| {
        if trace {
            for l in recent {
                println!("  {l}");
            }
        }
        End::Stuck(why)
    };
    for _ in 0..2_000_000 {
        let at = m.r.CS.base.wrapping_add(m.r.EIP) as u64;
        let op = m.bus.mem.read_cell(at) as u8;
        let n = m.bus.mem.read_cell(at + 1) as u8;
        if op == 0xCD && (0x21..=0x24).contains(&n) {
            let (eax, ebx) = (m.r.EAX, m.r.EBX);
            let (ax, bx) = (eax as u16, ebx as u16);
            let here = (m.r.CS.selector, m.r.EIP);
            match n {
                0x24 if eax != ebx => return End::Assertion(here.0, here.1, format!("EAX={eax:08x} EBX={ebx:08x}")),
                0x23 if ax != bx => return End::Assertion(here.0, here.1, format!("AX={ax:04x} BX={bx:04x}")),
                0x22 if ax as u8 != (ax >> 8) as u8 => {
                    return End::Assertion(here.0, here.1, format!("AL={:02x} AH={:02x}", ax as u8, ax >> 8))
                }
                0x21 => match (ax >> 8) as u8 {
                    0x25 => {
                        let entry = (ax & 0xFF) as u64 * 4;
                        let (dx, ds) = (m.r.EDX as u16, m.r.DS.selector);
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
            m.r.EIP = m.r.EIP.wrapping_add(2);
            continue;
        }
        if m.halted {
            return stuck(format!("halted at {:04x}:{:08x}", m.r.CS.selector, m.r.EIP), &recent);
        }
        if trace {
            m.last_fault = None;
            if recent.len() == 40 {
                recent.pop_front();
            }
            recent.push_back(format!("{:04x}:{:08x} esp={:08x}", m.r.CS.selector, m.r.EIP, m.r.ESP));
        }
        if let Err(f) = m.step() {
            return stuck(format!("{f:?} at {:04x}:{:08x}", m.r.CS.selector, m.r.EIP), &recent);
        }
        if let (true, Some(f)) = (trace, m.last_fault.take()) {
            if let Some(l) = recent.back_mut() {
                l.push_str(&format!(" -> {f:?}"));
            }
            if first_fault {
                first_fault = false;
                println!("  first fault:");
                for l in &recent {
                    println!("    {l}");
                }
            }
        }
    }
    stuck(format!("still running at {:04x}:{:08x}", m.r.CS.selector, m.r.EIP), &recent)
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

/// Runs a suite (`i286/simple`, `i386/protected`) and returns the names
/// that did not pass.
fn suite(name: &str) -> (usize, Vec<String>) {
    let Some(dir) = test_dir() else {
        eprintln!("simulators/test not found; skipping");
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
fn i286_simple_programs_pass() {
    let (_, failed) = suite("i286/simple");
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
fn i286_complex_programs_pass() {
    let (_, failed) = suite("i286/complex");
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
fn i286_protected_programs_pass() {
    let (_, failed) = suite("i286/protected");
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
fn i386_protected_programs_pass() {
    let (_, failed) = suite("i386/protected");
    assert!(failed.is_empty(), "{failed:?}");
}
