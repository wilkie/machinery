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

/// Boots into `code` at 0000:0100 in RAM (no wait states): the ROM holds
/// only the reset vector's far jump, three word fetches with a wait state
/// each. Runs until HLT.
fn run_from_ram(code: &[u8]) -> System {
    let mut r = vec![0xF4u8; 0x10000];
    r[0xFFF0..0xFFF5].copy_from_slice(&[0xEA, 0x00, 0x01, 0x00, 0x00]); // jmp far 0000:0100
    let mut sys = boot(r);
    sys.board().ram.cells[0x100..0x100 + code.len()].copy_from_slice(code);
    for _ in 0..100 {
        if sys.cpu.halted {
            break;
        }
        sys.step().unwrap();
    }
    assert!(sys.cpu.halted);
    sys
}

/// The processor model owns its timing (`on retire`): the board's time is
/// the pipeline's clocks plus the wait states of the processor's own
/// transactions (the ROM's, for the reset jump), and the pipeline's last
/// bus cycle is the HALT.
#[test]
fn time_is_the_pipelines_clocks() {
    // mov ax, 3; mov bx, 5; mul bx; hlt
    let mut sys = run_from_ram(&[0xB8, 0x03, 0x00, 0xBB, 0x05, 0x00, 0xF7, 0xE3, 0xF4]);
    assert_eq!(sys.cpu.r.AX, 15);
    let cpu = &sys.board().cpu;
    let clocks = cpu.last_done;
    assert_eq!(cpu.trace_kind[cpu.trace_n as usize - 1], 5);
    assert_eq!(sys.now(), (3 + clocks) * clock::C1);
}

/// MUL r16 works 21 clocks to a register move's 2: with the queue full
/// behind it, each starts as the one before finishes, so the HALT comes 19
/// clocks later.
#[test]
fn mul_works_its_schedule() {
    // mov bx, 5; mul bx (fills the queue); then `x`; hlt
    let halt = |x: [u8; 2]| {
        let mut sys = run_from_ram(&[0xBB, 0x05, 0x00, 0xF7, 0xE3, x[0], x[1], 0xF4]);
        let cpu = &sys.board().cpu;
        cpu.trace_at[cpu.trace_n as usize - 1]
    };
    assert_eq!(halt([0xF7, 0xE3]) - halt([0x8B, 0xCA]), 19);
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
    // The host reaches a model fn through the system with a live context.
    sys.with_console(|console, ctx| console.fn_clear(ctx));
    assert_eq!(sys.board().console.len, 0);
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
    assert_eq!(b.pic1.isr, 0);
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
    assert_eq!(sys.board().pic1.irr & 1, 1, "the request is pending");
}

/// A device model driven directly: what it emits goes to this context.
#[derive(Default)]
struct Probe {
    edges: Vec<bool>,
}

impl Pit8254Ports for Probe {
    fn emit_irq_request(&mut self, line: u8, value: bool) {
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
    let lo = pit.on_port_read(&mut p, io, 0x40, b).unwrap().0;
    let hi = pit.on_port_read(&mut p, io, 0x40, b).unwrap().0;
    assert_eq!((hi << 8) | lo, 70);
    // Unlatched reads follow the counter.
    let lo = pit.on_port_read(&mut p, io, 0x40, b).unwrap().0;
    let hi = pit.on_port_read(&mut p, io, 0x40, b).unwrap().0;
    assert_eq!((hi << 8) | lo, 65);
    // Crossing the end of the count reloads it and pulses OUT once.
    p.edges.clear();
    pit.on_tick(&mut p, 65 + 250);
    assert_eq!(p.edges, [false, true]);
    assert_eq!(pit.ch[0].count, 100 - 50);
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

/// The host pushes six bytes; the guest has the 8237 write them to RAM.
#[test]
fn dma_writes_host_bytes_to_memory() {
    // Single transfer, write to memory, channel 1, six bytes at 0:0700.
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
    // Without autoinitialise the channel masks itself at terminal count.
    assert!(b.dma.ch[1].masked);
    assert_eq!(b.dma.ch[1].addr, 0x0706);
    assert_eq!(b.dma.ch[1].count, 0xFFFF);
    // Nothing is left requesting.
    assert!(!b.sig.isa_dreq[1]);
}

/// The host expects eleven bytes; the guest has the 8237 read them out of
/// the ROM (F000:0040, physical 0F0040) into the link.
#[test]
fn dma_reads_memory_out_to_the_host() {
    let mut image = rom(&dma_program(0x49, 0x0F, 0x0040, 10), &[]);
    image[0x40..0x4B].copy_from_slice(b"Hello, 286!");
    let mut sys = boot(image);
    sys.with_link(|link, ctx| link.fn_expect(ctx, 11));
    let n = sys.run(100_000).unwrap();
    assert!(sys.cpu.halted, "halted after {n} steps");
    let b = sys.board();
    assert_eq!(&b.link.outbox[..b.link.out_len as usize], b"Hello, 286!");
    assert_eq!(b.link.wanted, 0);
}

/// Acknowledges for another channel, or with nothing queued, are declined:
/// the controller then reads open bus.
#[test]
fn host_link_declines_other_channels() {
    use machine_board286::{HostLink, HostLinkPorts};
    struct Nothing;
    impl HostLinkPorts for Nothing {}
    let mut link = HostLink::new();
    let mut p = Nothing;
    assert_eq!(link.on_port_dack_read(&mut p, 1), None, "nothing queued");
    link.fn_push(&mut p, 0x42);
    assert_eq!(link.on_port_dack_read(&mut p, 2), None, "not its channel");
    assert_eq!(link.on_port_dack_read(&mut p, 1), Some((0x42, 0)));
}

/// The AT cascade: the host raises IRQ 12 (the slave controller's IR4). The
/// slave drives its INT onto the master's IR2; the master grants IR2, names
/// the slave on CAS and declines the acknowledge; the slave answers with
/// vector 0x74. The handler counts the interrupt and ends it on both chips.
#[test]
fn slave_controller_interrupts_through_the_master() {
    let mut code = SETUP.to_vec();
    code.extend_from_slice(&[
        0xC7, 0x06, 0xD0, 0x01, 0x00, 0x02, // mov word [0x01D0], 0x0200 (IVT 0x74)
        0xC7, 0x06, 0xD2, 0x01, 0x00, 0xF0, // mov word [0x01D2], 0xF000
        0xB0, 0x11, 0xE6, 0x20, // master ICW1: edge, cascade, ICW4
        0xB0, 0x08, 0xE6, 0x21, // ICW2: vectors 08-0F
        0xB0, 0x04, 0xE6, 0x21, // ICW3: a slave on IR2
        0xB0, 0x01, 0xE6, 0x21, // ICW4: 8086 mode
        0xB0, 0x11, 0xE6, 0xA0, // slave ICW1
        0xB0, 0x70, 0xE6, 0xA1, // ICW2: vectors 70-77
        0xB0, 0x02, 0xE6, 0xA1, // ICW3: ID 2
        0xB0, 0x01, 0xE6, 0xA1, // ICW4
        0xB0, 0xFB, 0xE6, 0x21, // master mask: only IR2
        0xB0, 0xEF, 0xE6, 0xA1, // slave mask: only IR4 (IRQ 12)
        0xFB, // sti
        0xF4, // hlt
        0x80, 0x3E, 0x01, 0x06, 0x01, // cmp byte [0x0601], 1
        0x72, 0xF8, // jb hlt
        0xFA, // cli
        0xF4, // hlt
    ]);
    let mut image = rom(&code, &[]);
    image[0x200..0x20D].copy_from_slice(&[
        0x50, // push ax
        0xFE, 0x06, 0x01, 0x06, // inc byte [0x0601]
        0xB0, 0x20, 0xE6, 0xA0, // EOI to the slave
        0xE6, 0x20, // EOI to the master
        0x58, // pop ax
        0xCF, // iret
    ]);
    let mut sys = boot(image);
    // Run to the first HLT: interrupts are on and nothing is pending.
    sys.run_until(10_000, |s| s.cpu.halted).unwrap();
    assert!(sys.cpu.halted);
    assert_eq!(sys.board().ram.cells[0x601], 0);
    // The host raises IRQ 12.
    sys.with_link(|link, ctx| link.fn_interrupt(ctx, true));
    let n = sys
        .run_until(10_000, |s| s.cpu.halted && s.cpu.r.FLAGS & 0x0200 == 0)
        .unwrap();
    let b = sys.board();
    assert_eq!(b.ram.cells[0x601], 1, "the handler ran once ({n} steps)");
    // The slave's ID was on CAS during the acknowledge.
    assert_eq!(b.sig.w_pic1_cas_out, 2);
    assert_eq!((b.pic1.isr, b.pic2.isr), (0, 0), "both chips ended the interrupt");
    // The request was taken: nothing is pending on either chip.
    assert_eq!(b.pic2.irr & 0x10, 0);
    assert_eq!(b.pic1.irr & 0x04, 0);
}

// --- CGA ---

/// `mov dx, port; mov al, v; out dx, al`
fn out8(code: &mut Vec<u8>, port: u16, v: u8) {
    code.extend_from_slice(&[0xBA, port as u8, (port >> 8) as u8, 0xB0, v, 0xEE]);
}

/// Program the 6845's registers from R0.
fn crtc(code: &mut Vec<u8>, regs: &[u8]) {
    for (i, v) in regs.iter().enumerate() {
        out8(code, 0x3D4, i as u8);
        out8(code, 0x3D5, *v);
    }
}

/// Wait for the start of vertical retrace (status bit 3).
fn wait_vsync(code: &mut Vec<u8>) {
    code.extend_from_slice(&[
        0xBA, 0xDA, 0x03, // mov dx, 0x3DA
        0xEC, 0xA8, 0x08, 0x75, 0xFB, // in al, dx; test al, 8; jnz (still in retrace)
        0xEC, 0xA8, 0x08, 0x74, 0xFB, // in al, dx; test al, 8; jz (until retrace)
    ]);
}

/// Program the CRTC and mode, run `video` (with DS = B800), wait two
/// retraces so a whole frame is drawn in between, halt.
fn cga_program(regs: &[u8], mode: u8, color: u8, video: &[u8]) -> Vec<u8> {
    let mut code = SETUP.to_vec();
    crtc(&mut code, regs);
    out8(&mut code, 0x3D8, mode);
    out8(&mut code, 0x3D9, color);
    code.extend_from_slice(&[0xB8, 0x00, 0xB8, 0x8E, 0xD8]); // mov ax, 0xB800; mov ds, ax
    code.extend_from_slice(video);
    wait_vsync(&mut code);
    wait_vsync(&mut code);
    code.push(0xF4);
    code
}

/// 80x25 text with the cursor off (R10 = 0x20).
const TEXT_80X25: [u8; 16] = [
    0x71, 0x50, 0x5A, 0x0A, 0x1F, 0x06, 0x19, 0x1C, 0x02, 0x07, 0x20, 0x07, 0, 0, 0, 0,
];
/// 320x200 graphics.
const GRAPHICS_320X200: [u8; 16] = [
    0x38, 0x28, 0x2D, 0x0A, 0x7F, 0x06, 0x64, 0x70, 0x02, 0x01, 0x06, 0x07, 0, 0, 0, 0,
];

fn boot_cga(code: &[u8]) -> System {
    let mut sys = boot(rom(code, &[]));
    let font = &mut sys.board().cga.font;
    font[0x41 * 8..0x41 * 8 + 8].copy_from_slice(&[0xFF, 0x81, 0, 0, 0, 0, 0, 0x18]); // 'A'
    font[0x42 * 8..0x42 * 8 + 8].copy_from_slice(&[0xF0; 8]); // 'B'
    sys
}

#[test]
fn cga_renders_text() {
    let code = cga_program(
        &TEXT_80X25,
        0x09, // 80 columns, video on
        0x00,
        &[
            0xC7, 0x06, 0x00, 0x00, 0x41, 0x1F, // 'A' white on blue at row 0, column 0
            0xC7, 0x06, 0xA0, 0x00, 0x42, 0x4E, // 'B' yellow on red at row 1, column 0
        ],
    );
    let mut sys = boot_cga(&code);
    let n = sys.run(200_000).unwrap();
    assert!(sys.cpu.halted, "halted after {n} steps");
    let cga = &sys.board().cga;
    assert!(cga.frames >= 1);
    assert_eq!((cga.frame_width, cga.frame_height), (640, 200));
    let f = &cga.frame;
    // 'A', raster 0: all foreground; raster 1: the two edges.
    assert_eq!(&f[0..8], &[15; 8]);
    assert_eq!(&f[640..648], &[15, 1, 1, 1, 1, 1, 1, 15]);
    // Raster 7: the middle two dots.
    assert_eq!(&f[7 * 640..7 * 640 + 8], &[1, 1, 1, 15, 15, 1, 1, 1]);
    // 'B' on the next character row (scanline 8).
    assert_eq!(&f[8 * 640..8 * 640 + 8], &[14, 14, 14, 14, 4, 4, 4, 4]);
    // An empty cell is black.
    assert_eq!(&f[8..16], &[0; 8]);
}

#[test]
fn cga_renders_320x200_graphics() {
    let code = cga_program(
        &GRAPHICS_320X200,
        0x0A, // graphics, video on
        0x21, // palette 1 (cyan, magenta, white), background blue
        &[
            0xC6, 0x06, 0x00, 0x00, 0x1B, // the first four pixels of line 0: 0, 1, 2, 3
            0xC6, 0x06, 0x00, 0x20, 0xFF, // line 1 (the odd bank): colour 3
        ],
    );
    let mut sys = boot_cga(&code);
    sys.run(200_000).unwrap();
    assert!(sys.cpu.halted);
    let cga = &sys.board().cga;
    assert_eq!((cga.frame_width, cga.frame_height), (640, 200));
    let f = &cga.frame;
    // Each low-resolution pixel is two dots.
    assert_eq!(&f[0..8], &[1, 1, 3, 3, 5, 5, 7, 7]);
    assert_eq!(&f[640..648], &[7; 8]);
    // The rest of line 0 is background.
    assert_eq!(&f[8..16], &[1; 8]);
}

/// A frame is 912 dots by 262 lines of the 14.318 MHz clock: 59.92 Hz.
#[test]
fn cga_frame_rate() {
    let code = cga_program(&TEXT_80X25, 0x09, 0x00, &[]);
    let mut sys = boot_cga(&code);
    // Until the CRTC is programmed its registers are zero and a "frame" is
    // one character: measure once the program has set the mode and halted.
    sys.run(1_000_000).unwrap();
    assert!(sys.cpu.halted);
    let f0 = sys.board().cga.frames;
    sys.run_until(1_000_000, |s| s.board().cga.frames == f0 + 1).unwrap();
    let t1 = sys.seconds();
    sys.run_until(1_000_000, |s| s.board().cga.frames == f0 + 2).unwrap();
    let t2 = sys.seconds();
    let expected = 912.0 * 262.0 / 14_318_180.0;
    assert!(((t2 - t1) - expected).abs() < 20e-6, "frame took {} s, expected {expected}", t2 - t1);
}

// --- emulation speed (`cargo test --release --test board -- --ignored --nocapture`) ---

/// A context for driving the CGA model directly: no signals to emit.
struct Quiet;
impl machine_board286::CgaPorts for Quiet {}

/// Frames a second the CGA model renders in a mode, driven directly, with
/// video memory filled with a pattern.
fn bench_cga(name: &str, regs: &[u8], mode: u8) {
    use machine_board286::Cga;
    let mut cga = Cga::new();
    let mut c = Quiet;
    for (i, &v) in regs.iter().enumerate() {
        cga.on_port_write(&mut c, 1, 0x3D4, 0, i as u16);
        cga.on_port_write(&mut c, 1, 0x3D5, 0, v as u16);
    }
    cga.on_port_write(&mut c, 1, 0x3D8, 0, mode as u16);
    let mut x: u32 = 12345;
    for b in cga.vram.iter_mut().chain(cga.font.iter_mut()) {
        x = x.wrapping_mul(1103515245).wrapping_add(12345);
        *b = (x >> 16) as u8;
    }
    let n = 600;
    let t = std::time::Instant::now();
    let f0 = cga.frames;
    while cga.frames < f0 + n {
        cga.on_tick(&mut c, 1_000_000);
    }
    let s = t.elapsed().as_secs_f64();
    eprintln!("CGA {name}: {:.0} frames/s ({:.1} us a frame)", n as f64 / s, s * 1e6 / n as f64);
}

#[test]
#[ignore]
fn bench_cga_modes() {
    bench_cga("80x25 text", &TEXT_80X25, 0x09);
    bench_cga("320x200 graphics", &GRAPHICS_320X200, 0x0A);
}

// --- Hercules ---

/// Program the 6845 at 3B4/3B5 from R0.
fn herc_crtc(code: &mut Vec<u8>, regs: &[u8]) {
    for (i, v) in regs.iter().enumerate() {
        out8(code, 0x3B4, i as u8);
        out8(code, 0x3B5, *v);
    }
}

/// The MDA's 80x25 text timing, the cursor off (R10 bits 6-5 = 01).
const HERC_TEXT: [u8; 16] = [
    0x61, 0x50, 0x52, 0x0F, 0x19, 0x06, 0x19, 0x19, 0x02, 0x0D, 0x2B, 0x0C, 0, 0, 0, 0,
];
/// The Hercules's 720x348 graphics timing: four scanlines a "row".
const HERC_GRAPHICS: [u8; 16] = [
    0x35, 0x2D, 0x2E, 0x07, 0x5B, 0x02, 0x57, 0x57, 0x02, 0x03, 0x00, 0x00, 0, 0, 0, 0,
];

/// Runs the program to its HLT, then until the Hercules has finished `n`
/// more frames.
fn herc_frames(sys: &mut System, n: u32) {
    sys.run_until(200_000, |s| s.cpu.halted).unwrap();
    assert!(sys.cpu.halted, "the program ran to its HLT");
    sys.catch_up_all();
    let want = sys.board().herc.frames + n;
    sys.run_until(10_000_000, |s| {
        s.catch_up_all();
        s.board().herc.frames >= want
    })
    .unwrap();
}

/// MDA text: one character in five attributes, normal (07), reverse (70),
/// underlined (01), intense (0F) and invisible (00).
#[test]
fn hercules_renders_mda_text() {
    let mut code = SETUP.to_vec();
    herc_crtc(&mut code, &HERC_TEXT);
    out8(&mut code, 0x3B8, 0x08); // text, video on, no blinking
    code.extend_from_slice(&[0xB8, 0x00, 0xB0, 0x8E, 0xD8]); // mov ax, 0xB000; mov ds, ax
    for (i, attr) in [0x07u8, 0x70, 0x01, 0x0F, 0x00].iter().enumerate() {
        let off = 2 + 2 * i as u16;
        code.extend_from_slice(&[0xC7, 0x06, off as u8, (off >> 8) as u8, b'A', *attr]); // mov word [off], attr:'A'
    }
    code.push(0xF4);
    let mut sys = boot(rom(&code, &[]));
    sys.board().herc.font[b'A' as usize * 16] = 0x81;
    herc_frames(&mut sys, 2);
    let h = &sys.board().herc;
    assert_eq!((h.frame_width, h.frame_height), (720, 350));
    let cell = |line: usize, col: usize| h.frame[line * 720 + col * 9..line * 720 + col * 9 + 9].to_vec();
    assert_eq!(cell(0, 1), [1, 0, 0, 0, 0, 0, 0, 1, 0], "normal");
    assert_eq!(cell(0, 2), [0, 1, 1, 1, 1, 1, 1, 0, 1], "reverse");
    assert_eq!(cell(0, 3), [1, 0, 0, 0, 0, 0, 0, 1, 0], "underlined, row 0");
    assert_eq!(cell(12, 3), [1; 9], "underlined, row 12: all nine dots");
    assert_eq!(cell(12, 1), [0; 9], "normal, row 12");
    assert_eq!(cell(0, 4), [2, 0, 0, 0, 0, 0, 0, 2, 0], "intense");
    assert_eq!(cell(0, 5), [0; 9], "invisible");
}

/// 720x348 graphics: the configuration switch allows it, each dot row is in
/// one of four banks, and a frame is 864 dots by 370 lines of the
/// 16.257 MHz clock.
#[test]
fn hercules_renders_720x348_graphics() {
    let mut code = SETUP.to_vec();
    out8(&mut code, 0x3BF, 0x01); // allow graphics
    herc_crtc(&mut code, &HERC_GRAPHICS);
    out8(&mut code, 0x3B8, 0x0A); // graphics, video on
    code.extend_from_slice(&[0xB8, 0x00, 0xB0, 0x8E, 0xD8]); // mov ax, 0xB000; mov ds, ax
    code.extend_from_slice(&[0xC6, 0x06, 0x00, 0x00, 0xF0]); // mov byte [0x0000], 0xF0: line 0
    code.extend_from_slice(&[0xC6, 0x06, 0x00, 0x20, 0x0F]); // mov byte [0x2000], 0x0F: line 1
    code.extend_from_slice(&[0xC6, 0x06, 0x5A, 0x00, 0x81]); // mov byte [0x005A], 0x81: line 4
    code.push(0xF4);
    let mut sys = boot(rom(&code, &[]));
    herc_frames(&mut sys, 1);
    let t0 = sys.seconds();
    herc_frames(&mut sys, 1);
    let period = sys.seconds() - t0;
    let h = &sys.board().herc;
    assert_eq!((h.frame_width, h.frame_height), (720, 348));
    assert_eq!(&h.frame[0..8], &[1, 1, 1, 1, 0, 0, 0, 0]);
    assert_eq!(&h.frame[720..728], &[0, 0, 0, 0, 1, 1, 1, 1]);
    assert_eq!(&h.frame[4 * 720..4 * 720 + 8], &[1, 0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(&h.frame[2 * 720..2 * 720 + 8], &[0; 8]);
    let expect = 864.0 * 370.0 / 16.257e6;
    assert!((period - expect).abs() < 1e-4, "a frame took {period} s, expected {expect}");
}

/// A context for driving the Hercules model directly.
struct QuietHerc;
impl machine_board286::HerculesPorts for QuietHerc {}

/// The configuration switch: graphics and the second page can be selected
/// only once it allows them, and the second page is answered only then.
#[test]
fn hercules_configuration_switch() {
    use machine_board286::Hercules;
    let mut h = Hercules::new();
    let mut c = QuietHerc;
    h.on_port_write(&mut c, 1, 0x3B8, 0, 0x8A);
    assert_eq!(h.mode, 0x08, "graphics and page 1 masked");
    assert_eq!(h.on_port_write(&mut c, 0, 0xB8000, 0, 0x55), None, "second page not answered");
    h.on_port_write(&mut c, 1, 0x3BF, 0, 0x03);
    h.on_port_write(&mut c, 1, 0x3B8, 0, 0x8A);
    assert_eq!(h.mode, 0x8A);
    assert!(h.on_port_write(&mut c, 0, 0xB8000, 0, 0x55).is_some());
    assert_eq!(h.vram[0x8000], 0x55);
    assert_eq!(h.on_port_read(&mut c, 0, 0xB8000, 0).map(|r| r.0), Some(0x55));
}

/// Frames a second the Hercules model renders, driven directly.
fn bench_herc(name: &str, regs: &[u8], config: u8, mode: u8) {
    use machine_board286::Hercules;
    let mut h = Hercules::new();
    let mut c = QuietHerc;
    h.on_port_write(&mut c, 1, 0x3BF, 0, config as u16);
    for (i, &v) in regs.iter().enumerate() {
        h.on_port_write(&mut c, 1, 0x3B4, 0, i as u16);
        h.on_port_write(&mut c, 1, 0x3B5, 0, v as u16);
    }
    h.on_port_write(&mut c, 1, 0x3B8, 0, mode as u16);
    let mut x: u32 = 12345;
    for b in h.vram.iter_mut().chain(h.font.iter_mut()) {
        x = x.wrapping_mul(1103515245).wrapping_add(12345);
        *b = (x >> 16) as u8;
    }
    let n = 600;
    let t = std::time::Instant::now();
    let f0 = h.frames;
    while h.frames < f0 + n {
        h.on_tick(&mut c, 1_000_000);
    }
    let s = t.elapsed().as_secs_f64();
    eprintln!("Hercules {name}: {:.0} frames/s ({:.1} us a frame)", n as f64 / s, s * 1e6 / n as f64);
}

#[test]
#[ignore]
fn bench_hercules_modes() {
    bench_herc("80x25 text", &HERC_TEXT, 0x00, 0x08);
    bench_herc("720x348 graphics", &HERC_GRAPHICS, 0x01, 0x0A);
}
