//! Drives the crate generated from `board.machine`
//! (`machine build --system Board286`): boots a small ROM program and
//! checks what the devices saw and how much time passed.

use machine_board286::system::{clock, System};

/// A boot ROM: print a message on the console port, start the timer and
/// poll it down to zero, store a word in RAM, halt.
fn rom() -> Vec<u8> {
    let mut r = vec![0xF4u8; 0x10000];
    let code: &[u8] = &[
        0xB8, 0x00, 0x00, // mov ax, 0
        0x8E, 0xD8, // mov ds, ax
        0x8E, 0xD0, // mov ss, ax
        0xBC, 0x00, 0x04, // mov sp, 0x0400
        0xBE, 0x40, 0x00, // mov si, 0x0040
        0x2E, 0xAC, // cs lodsb
        0x84, 0xC0, // test al, al
        0x74, 0x04, // jz +4
        0xE6, 0xE9, // out 0xE9, al
        0xEB, 0xF6, // jmp -10
        0xB0, 0x05, // mov al, 5
        0xE6, 0x40, // out 0x40, al      (count = 5)
        0xB0, 0x03, // mov al, 3
        0xE6, 0x42, // out 0x42, al      (enable, interrupt)
        0xE4, 0x41, // in al, 0x41
        0x84, 0xC0, // test al, al
        0x75, 0xFA, // jnz -6
        0xC7, 0x06, 0x00, 0x05, 0x34, 0x12, // mov word [0x0500], 0x1234
        0xF4, // hlt
    ];
    r[..code.len()].copy_from_slice(code);
    let msg = b"Hello, 286!\0";
    r[0x40..0x40 + msg.len()].copy_from_slice(msg);
    r[0xFFF0..0xFFF5].copy_from_slice(&[0xEA, 0x00, 0x00, 0x00, 0xF0]); // jmp far F000:0000
    r
}

/// The first instruction after reset is the far jump at FFFFF0: five byte
/// fetches from the ROM, each two bus cycles plus the ROM's wait state.
#[test]
fn reset_jump_costs_its_rom_fetches() {
    let mut sys = System::new();
    sys.board().rom.cells = rom();
    sys.reset();
    sys.step().unwrap();
    assert_eq!(sys.cpu.r.IP, 0x0000);
    assert_eq!(sys.cpu.r.CS.selector, 0xF000);
    assert_eq!(sys.now(), 5 * (2 + 1) * clock::C0);
}

#[test]
fn boots_prints_and_polls_the_timer() {
    let mut sys = System::new();
    sys.board().rom.cells = rom();
    sys.reset();
    let n = sys.run(10_000).expect("no undeliverable fault");
    assert!(sys.cpu.halted, "halted after {n} instructions");
    let b = sys.board();
    let text = String::from_utf8_lossy(&b.console.text[..b.console.len as usize]).into_owned();
    assert_eq!(text, "Hello, 286!");
    assert_eq!(&b.ram.cells[0x500..0x502], &[0x34, 0x12]);
    assert_eq!(b.timer.count, 0);
    assert!(b.timer.pending);
    assert!(b.sig.local_interrupt[0]);
    // The poll loop can only end after the timer's five cycles elapsed.
    assert!(sys.now() >= 5 * clock::C1, "now = {}", sys.now());
    eprintln!("{n} instructions, {} time units, {:.3} us", sys.now(), sys.seconds() * 1e6);
}
