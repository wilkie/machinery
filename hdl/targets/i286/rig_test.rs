//! Conformance of the 80286 processor model's pipeline (SPEC 11.2c): drives
//! the crate generated from `rig.machine` (`machine build --system Rig286`)
//! through SingleStepTests vectors and compares the bus cycles the pipeline
//! records with each vector's hardware trace, cycle for cycle.
//!
//! `RIG_FILES` (comma-separated opcode files, `89,F7.4`) and `RIG_LIMIT`
//! (vectors per file) choose the sample; `RIG_MIN` is the share that must
//! match exactly; `RIG_SHOW` prints the first mismatches of each file.

use std::path::PathBuf;

use machine_rig286::system::System;
use machine_rig286::Segment;
use machine_runtime::vectors::{read_moo, BusKind, MooTest};

fn corpus() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = here.as_path();
    // The generated crate lives under hdl/target; the corpus beside hdl.
    loop {
        let c = dir.join("simulators/test/i286/SingleStepTests/v1_real_mode");
        if c.exists() {
            return c;
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => return PathBuf::from(std::env::var("RIG_CORPUS").unwrap_or_default()),
        }
    }
}

fn kind(k: u8) -> BusKind {
    match k {
        0 => BusKind::Code,
        1 => BusKind::MemRead,
        2 => BusKind::MemWrite,
        3 => BusKind::IoRead,
        4 => BusKind::IoWrite,
        5 => BusKind::Halt,
        _ => BusKind::Other,
    }
}

fn seg(v: u16) -> Segment {
    Segment { selector: v, base: (v as u32) << 4 }
}

/// Runs one vector; returns the pipeline's bus cycles.
fn run(sys: &mut System, t: &MooTest) -> Vec<(u32, BusKind)> {
    {
        let r = &mut sys.cpu.r;
        for (name, v) in &t.initial.regs {
            let v = *v;
            match *name {
                "ax" => r.AX = v,
                "bx" => r.BX = v,
                "cx" => r.CX = v,
                "dx" => r.DX = v,
                "sp" => r.SP = v,
                "bp" => r.BP = v,
                "si" => r.SI = v,
                "di" => r.DI = v,
                "ip" => r.IP = v,
                "flags" => r.FLAGS = v,
                "cs" => r.CS = seg(v),
                "ds" => r.DS = seg(v),
                "es" => r.ES = seg(v),
                "ss" => r.SS = seg(v),
                _ => {}
            }
        }
    }
    sys.cpu.halted = false;
    for (a, b) in &t.initial.ram {
        sys.board().space.cells[*a as usize & 0xFF_FFFF] = *b;
    }
    sys.board().cpu.code_valid = false;
    sys.with_cpu(|c, ctx| c.fn_flush(ctx));
    for _ in 0..16 {
        if sys.cpu.halted || sys.step().is_err() {
            break;
        }
    }
    let c = &sys.board().cpu;
    (0..c.trace_n as usize)
        .map(|i| (c.trace_at[i], kind(c.trace_kind[i])))
        .collect()
}

#[test]
fn pipeline_reproduces_the_bus_traces() {
    let dir = corpus();
    if !dir.join("00.MOO.gz").exists() {
        eprintln!("SingleStepTests corpus not found; skipping");
        return;
    }
    let files = std::env::var("RIG_FILES").unwrap_or_else(|_| {
        "00,01,03,05,40,50,58,74,80.0,83.0,88,8B,8D,9C,A1,B8,C3,C6,CF,D1.4,D3.4,E2,E5,E8,EB,F6.4,F7.6,FF.2,60,9A".into()
    });
    let limit: usize = std::env::var("RIG_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(400);
    let min: f64 = std::env::var("RIG_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let show: usize = std::env::var("RIG_SHOW").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut sys = System::new();
    sys.reset();
    let (mut total, mut good) = (0usize, 0usize);
    for f in files.split(',').filter(|f| !f.is_empty()) {
        let tests = read_moo(&dir.join(format!("{f}.MOO.gz"))).unwrap_or_else(|e| panic!("{e}"));
        let (mut n, mut ok, mut shown) = (0usize, 0usize, 0usize);
        for t in tests.iter().filter(|t| t.exception.is_none()).take(limit) {
            let Some(timing) = &t.timing else { continue };
            let want: Vec<(u32, BusKind)> = timing
                .bus
                .iter()
                .copied()
                .filter(|(_, k)| *k != BusKind::Other)
                .collect();
            let got = run(&mut sys, t);
            n += 1;
            if got == want {
                ok += 1;
            } else if shown < show {
                shown += 1;
                println!("{f} #{} {:02X?}\n  want {want:?}\n  got  {got:?}", t.index, t.bytes);
            }
        }
        println!("{f:6} {ok:4}/{n:<4} {:5.1}%", 100.0 * ok as f64 / n.max(1) as f64);
        total += n;
        good += ok;
    }
    let pct = 100.0 * good as f64 / total.max(1) as f64;
    println!("rig: {good}/{total} = {pct:.1}% exact");
    assert!(pct >= min, "rig: {pct:.1}% exact, below {min}%");
}
