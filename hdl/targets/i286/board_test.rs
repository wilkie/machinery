//! Drives the crate generated from `board.machine`
//! (`machine build --system Board286`): boots small ROM programs and checks
//! what the devices saw, which interrupts were taken, and how much time
//! passed.

use machine_board286::system::{clock, System};
use machine_board286::{Pit8254, Pit8254Ports};

/// A 64 KB ROM image (filled with HLT): `code` at F000:0000, `irq0` at
/// F000:0100, and the reset vector's far jump to F000:0000.
fn rom(code: &[u8], irq0: &[u8]) -> Vec<u8> {
    let mut r = vec![0xF4u8; 0x10000];
    r[..code.len()].copy_from_slice(code);
    r[0x100..0x100 + irq0.len()].copy_from_slice(irq0);
    r[0xFFF0..0xFFF5].copy_from_slice(&[0xEA, 0x00, 0x00, 0x00, 0xF0]); // jmp far F000:0000
    r
}

fn boot(image: Vec<u8>) -> System {
    let mut sys = System::new();
    sys.board().rom.cells = image;
    sys.reset();
    sys
}

/// Segments, the stack, and IVT entry 8 (IRQ 0 at vector base 8) pointing
/// at F000:0100.
const SETUP: &[u8] = &[
    0xB8, 0x00, 0x00, // mov ax, 0
    0x8E, 0xD8, // mov ds, ax
    0x8E, 0xD0, // mov ss, ax
    0xBC, 0x00, 0x04, // mov sp, 0x0400
    0xC7, 0x06, 0x20, 0x00, 0x00, 0x01, // mov word [0x0020], 0x0100
    0xC7, 0x06, 0x22, 0x00, 0x00, 0xF0, // mov word [0x0022], 0xF000
];

/// 8259: ICW1 (edge, single, ICW4), ICW2 (base 8), ICW4 (8086 mode), then
/// the mask `imr`. 8254: channel 0, low then high byte, mode 2, count 1000.
fn init(imr: u8) -> Vec<u8> {
    vec![
        0xB0, 0x13, 0xE6, 0x20, // ICW1
        0xB0, 0x08, 0xE6, 0x21, // ICW2
        0xB0, 0x01, 0xE6, 0x21, // ICW4
        0xB0, imr, 0xE6, 0x21, // OCW1
        0xB0, 0x34, 0xE6, 0x43, // channel 0, lo/hi, mode 2
        0xB0, 0xE8, 0xE6, 0x40, // count low
        0xB0, 0x03, 0xE6, 0x40, // count high: 1000
    ]
}

/// The IRQ 0 handler: count the tick at 0:0600, EOI, return.
const TICK_HANDLER: &[u8] = &[
    0x50, // push ax
    0xFE, 0x06, 0x00, 0x06, // inc byte [0x0600]
    0xB0, 0x20, 0xE6, 0x20, // mov al, 0x20; out 0x20, al (non-specific EOI)
    0x58, // pop ax
    0xCF, // iret
];

/// The first instruction after reset is the far jump at FFFFF0: five byte
/// fetches from the ROM, each two bus cycles plus the ROM's wait state.
#[test]
fn reset_jump_costs_its_rom_fetches() {
    let mut sys = boot(rom(&[], &[]));
    sys.step().unwrap();
    assert_eq!(sys.cpu.r.IP, 0x0000);
    assert_eq!(sys.cpu.r.CS.selector, 0xF000);
    assert_eq!(sys.now(), 5 * (2 + 1) * clock::C1);
}

#[test]
fn boots_prints_and_stores() {
    let mut code = SETUP.to_vec();
    code.extend_from_slice(&[
        0xBE, 0x40, 0x00, // mov si, 0x0040
        0x2E, 0xAC, // cs lodsb
        0x84, 0xC0, // test al, al
        0x74, 0x04, // jz +4
        0xE6, 0xE9, // out 0xE9, al
        0xEB, 0xF6, // jmp -10
        0xC7, 0x06, 0x00, 0x05, 0x34, 0x12, // mov word [0x0500], 0x1234
        0xF4, // hlt
    ]);
    let mut image = rom(&code, &[]);
    image[0x40..0x4C].copy_from_slice(b"Hello, 286!\0");
    let mut sys = boot(image);
    let n = sys.run(10_000).expect("no undeliverable fault");
    assert!(sys.cpu.halted, "halted after {n} steps");
    let b = sys.board();
    let text = String::from_utf8_lossy(&b.console.text[..b.console.len as usize]).into_owned();
    assert_eq!(text, "Hello, 286!");
    assert_eq!(&b.ram.cells[0x500..0x502], &[0x34, 0x12]);
}

/// Program the 8259 and the 8254, enable interrupts and wait in HLT until
/// three timer interrupts have been counted; then print `!` and stop.
fn tick_program(imr: u8) -> Vec<u8> {
    let mut code = SETUP.to_vec();
    code.extend(init(imr));
    let lp = code.len() as u8 + 1; // after the STI
    code.extend_from_slice(&[
        0xFB, // sti
        0xF4, // hlt
        0x80, 0x3E, 0x00, 0x06, 0x03, // cmp byte [0x0600], 3
        0x72, 0xF8, // jb hlt
        0xFA, // cli
        0xB0, b'!', 0xE6, 0xE9, // out 0xE9, '!'
        0xF4, // hlt
    ]);
    assert_eq!(code[lp as usize], 0xF4);
    rom(&code, TICK_HANDLER)
}

#[test]
fn timer_interrupts_wake_halt() {
    let mut sys = boot(tick_program(0xFE));
    let n = sys
        .run_until(1_000_000, |s| s.cpu.halted && s.cpu.r.FLAGS & 0x0200 == 0)
        .unwrap();
    let pit = clock::C2;
    let now = sys.now();
    let b = sys.board();
    assert_eq!(b.ram.cells[0x600], 3, "ticks counted after {n} steps");
    assert_eq!(b.console.text[0], b'!');
    // Every interrupt was acknowledged and ended.
    assert_eq!(b.pic.isr, 0);
    // Writing the mode-2 control word raises OUT, an edge the 8259
    // records, so the first interrupt comes as soon as STI allows; the next
    // two come 1000 timer clocks apart. Setup and the handlers' latency add
    // a few dozen clocks.
    assert!(now >= 2000 * pit, "now {now} = {} timer clocks", now / pit);
    assert!(now < 2100 * pit, "now {now} = {} timer clocks", now / pit);
    eprintln!("{n} steps, {:.1} us", sys.seconds() * 1e6);
}

/// With IRQ 0 masked the request is recorded but never taken.
#[test]
fn masked_timer_does_not_interrupt() {
    let mut sys = boot(tick_program(0xFF));
    let pit = clock::C2;
    sys.run_until(1_000_000, |s| s.now() > 5000 * pit).unwrap();
    let b = sys.board();
    assert_eq!(b.ram.cells[0x600], 0);
    assert!(sys.cpu.halted);
    assert_eq!(sys.board().pic.irr & 1, 1, "the request is pending");
}

/// A device model driven directly: what it emits goes to this context.
#[derive(Default)]
struct Probe {
    edges: Vec<bool>,
}

impl Pit8254Ports for Probe {
    fn emit_port_interrupt(&mut self, line: u8, value: bool) {
        assert_eq!(line, 0);
        self.edges.push(value);
    }
}

#[test]
fn pit_counter_latch_and_rate_generator() {
    use machine_board286::{BusSize, Space};
    let mut pit = Pit8254::new();
    let mut p = Probe::default();
    let io = Space::Io;
    let b = BusSize::Byte;
    pit.on_port_write(&mut p, io, 0x43, b, 0x34); // ch 0, lo/hi, mode 2
    pit.on_port_write(&mut p, io, 0x40, b, 100);
    pit.on_port_write(&mut p, io, 0x40, b, 0);
    pit.on_tick(&mut p, 30);
    pit.on_port_write(&mut p, io, 0x43, b, 0x00); // latch channel 0
    pit.on_tick(&mut p, 5); // the latch holds the old value
    let lo = pit.on_port_read(&mut p, io, 0x40, b).0;
    let hi = pit.on_port_read(&mut p, io, 0x40, b).0;
    assert_eq!((hi << 8) | lo, 70);
    // Unlatched reads follow the counter.
    let lo = pit.on_port_read(&mut p, io, 0x40, b).0;
    let hi = pit.on_port_read(&mut p, io, 0x40, b).0;
    assert_eq!((hi << 8) | lo, 65);
    // Crossing the end of the count reloads it and pulses OUT once.
    p.edges.clear();
    pit.on_tick(&mut p, 65 + 250);
    assert_eq!(p.edges, [false, true]);
    assert_eq!(pit.count[0], 100 - 50);
}
