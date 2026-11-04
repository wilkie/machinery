//! Drives the crate generated from `board.machine`'s `BoardZ80` (any of its
//! profiles: the decode, trace and JIT cores must agree): a checksum
//! program, checked against the same computation in Rust, and its time
//! against the T-states the Z80's datasheet gives its instructions.

use machine_board_z80::system::System;

/// `checksum.asm`: fill 4 KB at 1000h with a pattern; 1024 passes, each
/// summing 256 pairs of it through IX (RLC, XOR (IX+1), ADD HL,DE) with a
/// CALL a pair; LDIR the 4 KB to 2000h; HALT. The last pass's sum is left
/// at E002h.
const CHECKSUM: &[u8] = &[
    0x31, 0x00, 0xf0, 0x21, 0x00, 0x10, 0x01, 0x00, 0x10, 0x3e, 0x5a, 0x77, 0x07, 0xc6, 0x13, 0x23, 0x0b, 0x57, 0x78, 0xb1,
    0x7a, 0x20, 0xf4, 0x21, 0x00, 0x04, 0x22, 0x00, 0xe0, 0xdd, 0x21, 0x00, 0x10, 0x21, 0x00, 0x00, 0x06, 0x00, 0xdd, 0x7e,
    0x00, 0xcb, 0x07, 0xdd, 0xae, 0x01, 0x5f, 0x16, 0x00, 0x19, 0xdd, 0x23, 0xdd, 0x23, 0xcd, 0x55, 0x00, 0x10, 0xeb, 0x22,
    0x02, 0xe0, 0x2a, 0x00, 0xe0, 0x2b, 0x22, 0x00, 0xe0, 0x7c, 0xb5, 0x20, 0xd4, 0x21, 0x00, 0x10, 0x11, 0x00, 0x20, 0x01,
    0x00, 0x10, 0xed, 0xb0, 0x76, 0x7c, 0xad, 0x6f, 0xc9,
];

fn boot(program: &[u8]) -> System {
    let mut sys = System::new();
    sys.board().ram.cells[..program.len()].copy_from_slice(program);
    sys.reset();
    sys
}

/// The program's T-states, from the datasheet's instruction timings (a
/// conditional jump's taken / not-taken pair as `12/7`): the setup; the
/// fill loop, 4096 times (LD (HL),A 7, RLCA 4, ADD A,n 7, INC HL 6, DEC BC
/// 6, four 4-state loads and OR, JR NZ 12/7); LD HL,nn and LD (nn),HL; 1024
/// passes, each LD IX,nn 14, LD HL,nn 10, LD B,n 7, 256 pairs (LD A,(IX+d)
/// 19, RLC A 8, XOR (IX+d) 19, LD E,A 4, LD D,n 7, ADD HL,DE 11, INC IX 10
/// twice, CALL 17 to LD A,H, XOR L, LD L,A 4 each and RET 10, DJNZ 13/8),
/// then LD (nn),HL 16, LD HL,(nn) 16, DEC HL 6, LD (nn),HL 16, LD A,H 4,
/// OR L 4, JR NZ 12/7; three LD rr,nn 10; LDIR over 4096 bytes (21 an
/// element, 16 the last); HALT 4.
const T_STATES: u64 = {
    let setup = 10 + 10 + 10 + 7;
    let fill = 4096 * (7 + 4 + 7 + 6 + 6 + 4 * 4 + 12) - 5;
    let pair = 19 + 8 + 19 + 4 + 7 + 11 + 10 + 10 + 17 + (4 + 4 + 4 + 10) + 13;
    let pass = 14 + 10 + 7 + (256 * pair - 5) + 16 + 16 + 6 + 16 + 4 + 4 + 12;
    let copy = 3 * 10 + 4095 * 21 + 16;
    setup + fill + 10 + 16 + (1024 * pass - 5) + copy + 4
};

/// The program's computation, in Rust: the pattern, and one pass's sum.
fn expected() -> (Vec<u8>, u16) {
    let mut mem = vec![0u8; 0x1000];
    let mut a: u8 = 0x5A;
    for m in mem.iter_mut() {
        *m = a;
        a = a.rotate_left(1).wrapping_add(0x13);
    }
    let mut hl: u16 = 0;
    for k in 0..256 {
        let v = mem[2 * k].rotate_left(1) ^ mem[2 * k + 1];
        hl = hl.wrapping_add(v as u16);
        let (h, l) = ((hl >> 8) as u8, hl as u8);
        hl = ((h as u16) << 8) | (h ^ l) as u16;
    }
    (mem, hl)
}

#[test]
fn checksum_program_runs() {
    let mut sys = boot(CHECKSUM);
    let n = sys.run(20_000_000).unwrap();
    assert!(sys.cpu.halted, "halted after {n} instructions");
    let (pattern, sum) = expected();
    let ram = &sys.board().ram.cells;
    assert_eq!(&ram[0x1000..0x2000], &pattern[..], "the pattern");
    assert_eq!(&ram[0x2000..0x3000], &pattern[..], "copied by LDIR");
    assert_eq!(u16::from_le_bytes([ram[0xE002], ram[0xE003]]), sum, "the sum");
    assert_eq!((ram[0xE000], ram[0xE001]), (0, 0), "every pass made");
    // The board's clock is 4 MHz.
    assert_eq!((sys.seconds() * 4e6).round() as u64, T_STATES, "T-states");
    eprintln!("{} ({}): {n} instructions, {:.4} s emulated", machine_board_z80::system::PROFILE, machine_board_z80::system::CORE, sys.seconds());
}

/// The cores' speed on the checksum program. Run with `cargo test --release
/// -- --ignored --nocapture bench_checksum`.
#[test]
#[ignore]
fn bench_checksum() {
    for _ in 0..3 {
        let mut sys = boot(CHECKSUM);
        let wall = std::time::Instant::now();
        let n = sys.run(20_000_000).unwrap();
        let w = wall.elapsed().as_secs_f64();
        assert!(sys.cpu.halted);
        let t = sys.seconds();
        eprintln!(
            "{} ({}): {n} instructions, {:.1} M/s; {t:.3} s emulated in {w:.3} s ({:.1}x real time)",
            machine_board_z80::system::PROFILE,
            machine_board_z80::system::CORE,
            n as f64 / w / 1e6,
            t / w
        );
    }
}
