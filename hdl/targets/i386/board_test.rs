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

/// `out port, value` for each pair: mov dx, port; mov al, value; out dx, al.
fn outs(regs: &[(u16, u8)]) -> Vec<u8> {
    let mut code = Vec::new();
    for &(port, v) in regs {
        code.extend_from_slice(&[0xBA, port as u8, (port >> 8) as u8, 0xB0, v, 0xEE]);
    }
    code
}

/// An EGA mode's registers: Miscellaneous Output, then the sequencer, CRTC,
/// graphics controller and attribute controller from index 0.
fn ega_mode(misc: u8, seq: &[u8], crtc: &[u8], gc: &[u8], attr: &[u8]) -> Vec<u8> {
    let mut r = vec![(0x3C2, misc)];
    for (i, &v) in seq.iter().enumerate() {
        r.push((0x3C4, i as u8));
        r.push((0x3C5, v));
    }
    for (i, &v) in crtc.iter().enumerate() {
        r.push((0x3D4, i as u8));
        r.push((0x3D5, v));
    }
    for (i, &v) in gc.iter().enumerate() {
        r.push((0x3CE, i as u8));
        r.push((0x3CF, v));
    }
    // 3DA resets the attribute flip-flop; then index, data pairs.
    let mut code = vec![0xBA, 0xDA, 0x03, 0xEC]; // mov dx, 0x3DA; in al, dx
    code.extend(outs(&r));
    let mut a = Vec::new();
    for (i, &v) in attr.iter().enumerate() {
        a.push((0x3C0, i as u8));
        a.push((0x3C0, v));
    }
    a.push((0x3C0, 0x20)); // palette address source: the display has it
    code.extend(outs(&a));
    code
}

/// The palette of the EGA's 200-line modes: the CGA's sixteen colours.
const PAL200: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17];

/// Runs the program to its HLT, then until the EGA has finished `n` more
/// frames (until the registers are programmed a "frame" can be a few dots).
fn frames(sys: &mut System, n: u32) {
    sys.run_until(100_000, |s| s.cpu.halted).unwrap();
    assert!(sys.cpu.halted, "the program ran to its HLT");
    sys.catch_up_all();
    let want = sys.board().ega.frames + n;
    sys.run_until(5_000_000, |s| {
        s.catch_up_all();
        s.board().ega.frames >= want
    })
    .unwrap();
}

/// 80x25 text on 200-line timing (the BIOS's mode 3 for a colour monitor):
/// the character from plane 0, its attribute from plane 1 (odd/even mode
/// puts B8000 and B8001 there), the glyph row from plane 2, the colours
/// through the attribute palette.
#[test]
fn ega_text_mode_draws_characters() {
    let mut attr = PAL200.to_vec();
    attr.extend_from_slice(&[0x08, 0x00, 0x0F, 0x00]);
    let mut code = ega_mode(
        0x23,
        &[0x03, 0x01, 0x03, 0x00, 0x03],
        &[
            0x70, 0x4F, 0x5C, 0x2F, 0x5F, 0x07, 0x04, 0x11, 0x00, 0x07, 0x06, 0x07, 0x00, 0x00,
            0x00, 0x00, 0xE1, 0x24, 0xC7, 0x28, 0x08, 0xE0, 0xF0, 0xA3, 0xFF,
        ],
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x0E, 0x00, 0xFF],
        &attr,
    );
    code.extend_from_slice(&[
        0xB8, 0x00, 0xB8, // mov ax, 0xB800
        0x8E, 0xC0, // mov es, ax
        0x26, 0xC7, 0x06, 0x02, 0x00, 0x48, 0x1E, // mov word es:[2], 0x1E48: 'H', yellow on blue
        0xF4, // hlt
    ]);
    let mut sys = boot(rom(&code, &[]));
    // The font, as the BIOS would load it: 'H' row 0 is 11000011.
    sys.board().ega.planes[0x20000 + b'H' as usize * 32] = 0xC3;
    frames(&mut sys, 2);
    let b = sys.board();
    assert_eq!((b.ega.frame_width, b.ega.frame_height), (640, 200));
    // Odd/even: the character in plane 0, the attribute in plane 1.
    assert_eq!((b.ega.planes[2], b.ega.planes[0x10002]), (b'H', 0x1E));
    // The second character cell, scanline 0.
    let fg = 0x16; // palette[14]
    let bg = 0x01; // palette[1]
    assert_eq!(&b.ega.frame[8..16], &[fg, fg, bg, bg, bg, bg, fg, fg]);
}

/// 320x200 in 16 colours (the BIOS's mode 0Dh): writes through each write
/// mode, Set/Reset, the XOR function and the latches, a colour compare
/// (read mode 1), and the dots that come out.
#[test]
fn ega_planar_graphics() {
    let mut attr = PAL200.to_vec();
    attr.extend_from_slice(&[0x01, 0x00, 0x0F, 0x00]);
    let mut code = ega_mode(
        0x23,
        &[0x03, 0x0B, 0x0F, 0x00, 0x06],
        &[
            0x37, 0x27, 0x2D, 0x37, 0x30, 0x14, 0x04, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xE1, 0x24, 0xC7, 0x14, 0x00, 0xE0, 0xF0, 0xE3, 0xFF,
        ],
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0F, 0xFF],
        &attr,
    );
    let gc = |i: u8, v: u8| outs(&[(0x3CE, i), (0x3CF, v)]);
    code.extend_from_slice(&[
        0xB8, 0x00, 0xA0, // mov ax, 0xA000
        0x8E, 0xC0, // mov es, ax
        0x31, 0xC0, // xor ax, ax
        0x8E, 0xD8, // mov ds, ax
    ]);
    // Write mode 2: dot 0 colour 12, then (latches loaded) dot 1 colour 9.
    code.extend(gc(5, 0x02));
    code.extend(gc(8, 0x80));
    code.extend_from_slice(&[0x26, 0xC6, 0x06, 0x00, 0x00, 0x0C]); // mov byte es:[0], 12
    code.extend(gc(8, 0x40));
    code.extend_from_slice(&[0x26, 0xA0, 0x00, 0x00]); // mov al, es:[0] (latches)
    code.extend_from_slice(&[0x26, 0xC6, 0x06, 0x00, 0x00, 0x09]); // mov byte es:[0], 9
    // Write mode 0 with Set/Reset: dots 8-15 colour 5.
    code.extend(gc(5, 0x00));
    code.extend(gc(8, 0xFF));
    code.extend(gc(1, 0x0F));
    code.extend(gc(0, 0x05));
    code.extend_from_slice(&[0x26, 0xC6, 0x06, 0x01, 0x00, 0x00]); // mov byte es:[1], 0
    // Write mode 1: copy the latches of byte 1 to byte 2 (dots 16-23).
    code.extend(gc(5, 0x01));
    code.extend_from_slice(&[0x26, 0xA0, 0x01, 0x00]); // mov al, es:[1]
    code.extend_from_slice(&[0x26, 0xA2, 0x02, 0x00]); // mov es:[2], al
    // Write mode 0, XOR with the latches: dots 12-15 of byte 1 become 5 ^ 15.
    code.extend(gc(5, 0x00));
    code.extend(gc(1, 0x00));
    code.extend(gc(3, 0x18));
    code.extend_from_slice(&[0x26, 0xA0, 0x01, 0x00]); // mov al, es:[1]
    code.extend_from_slice(&[0x26, 0xC6, 0x06, 0x01, 0x00, 0x0F]); // mov byte es:[1], 0x0F
    code.extend(gc(3, 0x00));
    // Read mode 1: which dots of byte 0 are colour 12, stored at 0:0500.
    code.extend(gc(5, 0x08));
    code.extend(gc(2, 0x0C));
    code.extend(gc(7, 0x0F));
    code.extend_from_slice(&[0x26, 0xA0, 0x00, 0x00]); // mov al, es:[0]
    code.extend_from_slice(&[0xA2, 0x00, 0x05]); // mov [0x0500], al
    code.push(0xF4); // hlt
    let mut sys = boot(rom(&code, &[]));
    frames(&mut sys, 2);
    let b = sys.board();
    assert_eq!(b.ram.cells[0x500], 0x80, "read mode 1 found colour 12 at dot 0");
    assert_eq!((b.ega.frame_width, b.ega.frame_height), (640, 200));
    // Each dot of 320 is two of the frame's 640.
    let dot = |x: usize| b.ega.frame[2 * x];
    let pal = |c: usize| PAL200[c];
    assert_eq!((dot(0), dot(1), dot(2)), (pal(12), pal(9), pal(0)));
    for x in 8..12 {
        assert_eq!(dot(x), pal(5), "dot {x}");
    }
    for x in 12..16 {
        assert_eq!(dot(x), pal(10), "dot {x}");
    }
    for x in 16..24 {
        assert_eq!(dot(x), pal(5), "dot {x}");
    }
}

/// 640x350 in 16 of 64 colours (the BIOS's mode 10h): the 16.257 MHz dot
/// clock (744 dots a line, 365 lines, so about 59.9 frames a second), and
/// the 64-colour palette.
#[test]
fn ega_350_line_mode_uses_the_fast_clock() {
    let pal: [u8; 16] = [0, 1, 2, 3, 4, 5, 0x14, 7, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F];
    let mut attr = pal.to_vec();
    attr.extend_from_slice(&[0x01, 0x00, 0x0F, 0x00]);
    let mut code = ega_mode(
        0xA7,
        &[0x03, 0x01, 0x0F, 0x00, 0x06],
        &[
            0x5B, 0x4F, 0x53, 0x37, 0x52, 0x00, 0x6C, 0x1F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x5E, 0x2B, 0x5D, 0x28, 0x0F, 0x5F, 0x0A, 0xE3, 0xFF,
        ],
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0F, 0xFF],
        &attr,
    );
    // Set/Reset colour 14 into the first byte of line 349 (offset 349 * 80).
    code.extend(outs(&[(0x3CE, 1), (0x3CF, 0x0F), (0x3CE, 0), (0x3CF, 0x0E)]));
    code.extend_from_slice(&[
        0xB8, 0x00, 0xA0, // mov ax, 0xA000
        0x8E, 0xC0, // mov es, ax
        0x26, 0xC6, 0x06, 0x10, 0x6D, 0x00, // mov byte es:[0x6D10], 0
        0xF4, // hlt
    ]);
    let mut sys = boot(rom(&code, &[]));
    frames(&mut sys, 1);
    let t0 = sys.seconds();
    frames(&mut sys, 1);
    let period = sys.seconds() - t0;
    let b = sys.board();
    assert_eq!((b.ega.frame_width, b.ega.frame_height), (640, 350));
    assert_eq!(&b.ega.frame[349 * 720..349 * 720 + 9], &[0x3E, 0x3E, 0x3E, 0x3E, 0x3E, 0x3E, 0x3E, 0x3E, 0]);
    let expect = 744.0 * 365.0 / 16.257e6;
    assert!((period - expect).abs() < 1e-4, "a frame took {period} s, expected {expect}");
}

// --- emulation speed (`cargo test --release --test board -- --ignored --nocapture`) ---

/// A context for driving the EGA model directly: no signals to emit.
struct Quiet;
impl machine_board386::EgaPorts for Quiet {}

/// Frames a second the EGA model renders in a mode, driven directly (the
/// processor and the rest of the board take no part), with every plane
/// filled with a pattern so text and graphics both have work.
fn bench_ega(name: &str, misc: u8, seq: &[u8], crtc: &[u8], gc: &[u8], attr: &[u8]) {
    use machine_board386::Ega;
    let mut ega = Ega::new();
    let mut c = Quiet;
    let mut out = |ega: &mut Ega, port: u32, v: u8| {
        ega.on_port_write(&mut c, 1, port, 0, v as u16);
    };
    out(&mut ega, 0x3C2, misc);
    for (i, &v) in seq.iter().enumerate() {
        out(&mut ega, 0x3C4, i as u8);
        out(&mut ega, 0x3C5, v);
    }
    for (i, &v) in crtc.iter().enumerate() {
        out(&mut ega, 0x3D4, i as u8);
        out(&mut ega, 0x3D5, v);
    }
    for (i, &v) in gc.iter().enumerate() {
        out(&mut ega, 0x3CE, i as u8);
        out(&mut ega, 0x3CF, v);
    }
    for (i, &v) in attr.iter().enumerate() {
        out(&mut ega, 0x3C0, i as u8);
        out(&mut ega, 0x3C0, v);
    }
    let mut x: u32 = 12345;
    for b in ega.planes.iter_mut() {
        x = x.wrapping_mul(1103515245).wrapping_add(12345);
        *b = (x >> 16) as u8;
    }
    let mut c = Quiet;
    let n = 600;
    let t = std::time::Instant::now();
    let f0 = ega.frames;
    while ega.frames < f0 + n {
        let left = ega.on_tick_dot14(&mut c, 1_000_000);
        let _ = ega.on_tick_dot16(&mut c, 1_000_000.min(left));
    }
    let s = t.elapsed().as_secs_f64();
    eprintln!("EGA {name}: {:.0} frames/s ({:.1} us a frame)", n as f64 / s, s * 1e6 / n as f64);
}

#[test]
#[ignore]
fn bench_ega_modes() {
    let mut text = PAL200.to_vec();
    text.extend_from_slice(&[0x08, 0x00, 0x0F, 0x00]);
    bench_ega(
        "80x25 text",
        0x23,
        &[0x03, 0x01, 0x03, 0x00, 0x03],
        &[
            0x70, 0x4F, 0x5C, 0x2F, 0x5F, 0x07, 0x04, 0x11, 0x00, 0x07, 0x06, 0x07, 0x00, 0x00,
            0x00, 0x00, 0xE1, 0x24, 0xC7, 0x28, 0x08, 0xE0, 0xF0, 0xA3, 0xFF,
        ],
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x0E, 0x00, 0xFF],
        &text,
    );
    let mut graphics = PAL200.to_vec();
    graphics.extend_from_slice(&[0x01, 0x00, 0x0F, 0x00]);
    bench_ega(
        "320x200x16",
        0x23,
        &[0x03, 0x0B, 0x0F, 0x00, 0x06],
        &[
            0x37, 0x27, 0x2D, 0x37, 0x30, 0x14, 0x04, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xE1, 0x24, 0xC7, 0x14, 0x00, 0xE0, 0xF0, 0xE3, 0xFF,
        ],
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x0F, 0xFF],
        &graphics,
    );
}
