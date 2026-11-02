//! Drives the crate generated from `board.machine`
//! (`machine build --system Board386`): boots small ROM programs on the
//! 386 board and checks what the devices on the ISA bus saw, which
//! interrupts were taken, and how much time passed.

use machine_board386::system::{clock, System};

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

/// The processor starts at FFFFFFF0, the top of the 4 GB space, where the
/// ROM also answers; the far jump puts it in the first megabyte. It prints
/// through the console port on the ISA bus and stores a dword into RAM.
#[test]
fn boots_prints_and_stores() {
    let mut code = SETUP.to_vec();
    code.extend_from_slice(&[
        0xBE, 0x80, 0x00, // mov si, 0x0080
        0x2E, 0xAC, // cs lodsb
        0x84, 0xC0, // test al, al
        0x74, 0x04, // jz +4
        0xE6, 0xE9, // out 0xE9, al
        0xEB, 0xF6, // jmp -10
        0x66, 0xC7, 0x06, 0x00, 0x05, 0x78, 0x56, 0x34, 0x12, // mov dword [0x0500], 0x12345678
        0xF4, // hlt
    ]);
    let mut image = rom(&code, &[]);
    image[0x80..0x8C].copy_from_slice(b"Hello, 386!\0");
    let mut sys = boot(image);
    let n = sys.run(10_000).expect("no undeliverable fault");
    assert!(sys.cpu.halted, "halted after {n} steps");
    let b = sys.board();
    let text = String::from_utf8_lossy(&b.console.text[..b.console.len as usize]).into_owned();
    assert_eq!(text, "Hello, 386!");
    assert_eq!(&b.ram.cells[0x500..0x504], &[0x78, 0x56, 0x34, 0x12]);
}

/// Extended memory on the local bus: a dword stored at 1 MB + 0x10 in
/// protected mode (a flat 4 GB data segment) lands in the RAM's cells
/// there, and a word that crosses a dword boundary is two transfers that
/// still store both bytes.
#[test]
fn protected_mode_reaches_extended_memory() {
    // GDT at F000:0040 (physical F0040): null, then flat 4 GB data.
    let mut code = vec![
        0x2E, 0x0F, 0x01, 0x16, 0x60, 0x00, // lgdt cs:[0x0060]
        0x0F, 0x20, 0xC0, // mov eax, cr0
        0x0C, 0x01, // or al, 1
        0x0F, 0x22, 0xC0, // mov cr0, eax
        0xB8, 0x08, 0x00, // mov ax, 0x08
        0x8E, 0xD8, // mov ds, ax (flat, 4 GB)
        0x67, 0x66, 0xC7, 0x05, 0x10, 0x00, 0x10, 0x00, 0xEF, 0xBE, 0xAD, 0xDE, // mov dword [0x00100010], 0xDEADBEEF
        0x67, 0xC7, 0x05, 0x13, 0x00, 0x10, 0x00, 0x34, 0x12, // mov word [0x00100013], 0x1234
        0xF4, // hlt
    ];
    code.resize(0x40, 0x90);
    code.extend_from_slice(&[0; 8]); // null descriptor
    code.extend_from_slice(&[0xFF, 0xFF, 0x00, 0x00, 0x00, 0x92, 0xCF, 0x00]); // data, flat
    code.resize(0x60, 0x90);
    code.extend_from_slice(&[0x0F, 0x00, 0x40, 0x00, 0x0F, 0x00]); // limit 15, base 0F0040
    let mut sys = boot(rom(&code, &[]));
    let n = sys.run(10_000).expect("no undeliverable fault");
    assert!(sys.cpu.halted, "halted after {n} steps");
    let b = sys.board();
    assert_eq!(&b.ram.cells[0x100010..0x100015], &[0xEF, 0xBE, 0xAD, 0x34, 0x12]);
}

fn tick_program(imr: u8) -> Vec<u8> {
    let mut code = SETUP.to_vec();
    code.extend(init(imr));
    code.extend_from_slice(&[
        0xFB, // sti
        0xF4, // hlt
        0x80, 0x3E, 0x00, 0x06, 0x03, // cmp byte [0x0600], 3
        0x72, 0xF8, // jb hlt
        0xFA, // cli
        0xB0, b'!', 0xE6, 0xE9, // out 0xE9, '!'
        0xF4, // hlt
    ]);
    rom(&code, TICK_HANDLER)
}

/// The 8254 and 8259 on the ISA bus wake the 386 from HLT three times.
#[test]
fn timer_interrupts_wake_halt() {
    let mut sys = boot(tick_program(0xFE));
    let n = sys
        .run_until(1_000_000, |s| s.cpu.halted && s.cpu.r.EFLAGS & 0x0200 == 0)
        .unwrap();
    let pit = clock::C3;
    let now = sys.now();
    let b = sys.board();
    assert_eq!(b.ram.cells[0x600], 3, "ticks counted after {n} steps");
    assert_eq!(b.console.text[0], b'!');
    assert_eq!(b.pic1.isr, 0);
    // The first interrupt comes as soon as STI allows; the next two 1000
    // timer clocks apart.
    assert!(now >= 2000 * pit, "now {now} = {} timer clocks", now / pit);
    assert!(now < 2100 * pit, "now {now} = {} timer clocks", now / pit);
}

/// An I/O write to an 8-bit ISA device is an ISA cycle: two BCLKs and the
/// AT's four default wait states, at a third of the processor's clock, so
/// an OUT costs at least 18 processor clocks more than a register move.
#[test]
fn isa_cycles_take_isa_time() {
    let time = |op: [u8; 2]| {
        // mov al, 'x'; then `op` eight times; hlt (all fetched from RAM)
        let mut code = vec![0xB0, b'x'];
        for _ in 0..8 {
            code.extend_from_slice(&op);
        }
        code.push(0xF4);
        let mut r = vec![0xF4u8; 0x10000];
        r[0xFFF0..0xFFF5].copy_from_slice(&[0xEA, 0x00, 0x01, 0x00, 0x00]); // jmp far 0000:0100
        let mut sys = boot(r);
        sys.board().ram.cells[0x100..0x100 + code.len()].copy_from_slice(&code);
        sys.run(1000).unwrap();
        assert!(sys.cpu.halted);
        sys.now()
    };
    let out = time([0xE6, 0xE9]); // out 0xE9, al
    let mov = time([0x88, 0xC4]); // mov ah, al
    let per = (out - mov) / 8;
    assert_eq!(per % clock::C1, 0);
    assert!(per >= 6 * clock::C2, "an OUT costs {} processor clocks more", per / clock::C1);
    assert!(per < 6 * clock::C2 + 10 * clock::C1, "an OUT costs {} processor clocks more", per / clock::C1);
}

/// Program 8237 channel 1 (`mode`, page:address, `count` + 1 bytes),
/// unmask it, poll the status register until its terminal count, halt.
fn dma_program(mode: u8, page: u8, addr: u16, count: u16) -> Vec<u8> {
    let mut code = SETUP.to_vec();
    code.extend_from_slice(&[
        0xB0, 0x05, 0xE6, 0x0A, // mask channel 1
        0xE6, 0x0C, // clear the flip-flop
        0xB0, mode, 0xE6, 0x0B, // mode
        0xB0, addr as u8, 0xE6, 0x02, // address low
        0xB0, (addr >> 8) as u8, 0xE6, 0x02, // address high
        0xB0, page, 0xE6, 0x83, // page register for channel 1
        0xB0, count as u8, 0xE6, 0x03, // count low
        0xB0, (count >> 8) as u8, 0xE6, 0x03, // count high
        0xB0, 0x01, 0xE6, 0x0A, // unmask channel 1
        0xE4, 0x08, // poll: in al, 0x08
        0xA8, 0x02, // test al, 0x02 (channel 1 terminal count)
        0x74, 0xFA, // jz poll
        0xF4, // hlt
    ]);
    code
}

/// The 8237 on the ISA bus writes the host's bytes into system RAM on the
/// local bus, through the bus controller.
#[test]
fn dma_writes_host_bytes_to_memory() {
    let mut sys = boot(rom(&dma_program(0x45, 0x00, 0x0700, 5), &[]));
    sys.with_link(|link, ctx| {
        for &b in b"DMA OK" {
            link.fn_push(ctx, b);
        }
    });
    let n = sys.run(100_000).unwrap();
    assert!(sys.cpu.halted, "halted after {n} steps");
    let b = sys.board();
    assert_eq!(&b.ram.cells[0x700..0x706], b"DMA OK");
    assert_eq!(b.link.head, b.link.tail, "every byte delivered");
}

/// The 8237 reads the boot ROM (F000:0080) out to the host.
#[test]
fn dma_reads_memory_out_to_the_host() {
    let mut image = rom(&dma_program(0x49, 0x0F, 0x0080, 10), &[]);
    image[0x80..0x8B].copy_from_slice(b"Hello, 386!");
    let mut sys = boot(image);
    sys.with_link(|link, ctx| link.fn_expect(ctx, 11));
    let n = sys.run(100_000).unwrap();
    assert!(sys.cpu.halted, "halted after {n} steps");
    let b = sys.board();
    assert_eq!(&b.link.outbox[..b.link.out_len as usize], b"Hello, 386!");
}

/// The CGA's video memory is on the ISA bus: a dword store becomes four
/// byte cycles (the CGA is an 8-bit device), and every byte arrives.
#[test]
fn video_memory_through_the_isa_bus() {
    let code = [
        0xB8, 0x00, 0xB8, // mov ax, 0xB800
        0x8E, 0xC0, // mov es, ax
        0x66, 0x26, 0xC7, 0x06, 0x00, 0x00, 0x48, 0x07, 0x69, 0x07, // mov dword es:[0], 0x07690748 ("Hi")
        0xF4, // hlt
    ];
    let mut sys = boot(rom(&code, &[]));
    sys.run(1000).unwrap();
    assert!(sys.cpu.halted);
    assert_eq!(&sys.board().cga.vram[0..4], &[0x48, 0x07, 0x69, 0x07]);
}
