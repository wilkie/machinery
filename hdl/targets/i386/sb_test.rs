//! The Sound Blaster's DSP (`targets/pc/sb.machine`) on the 386 VGA board
//! (`Board386Vga`, where it is at 220h on IRQ 5 and DMA 3): driven directly,
//! and playing by DMA under a program (`sb_play.asm`).

use machine_board386_vga::system::System;
use machine_board386_vga::{SoundBlaster, SoundBlasterPorts};

/// The DSP's lines, as it drives them.
#[derive(Default)]
struct Lines {
    dreq: [bool; 4],
    irq: [bool; 8],
}

impl SoundBlasterPorts for Lines {
    fn emit_port_dreq(&mut self, channel: u8, value: bool) {
        self.dreq[channel as usize] = value;
    }
    fn level_port_dreq(&mut self, channel: u8) -> bool {
        self.dreq[channel as usize]
    }
    fn emit_irq_request(&mut self, line: u8, value: bool) {
        self.irq[line as usize] = value;
    }
    fn level_irq_request(&mut self, line: u8) -> bool {
        self.irq[line as usize]
    }
    fn read_dma_ch(&mut self) -> u8 {
        1
    }
    fn read_irq_line(&mut self) -> u8 {
        5
    }
}

fn out(sb: &mut SoundBlaster, l: &mut Lines, port: u32, d: u8) {
    sb.on_port_write(l, 1, port, 0, d as u16);
}

fn inp(sb: &mut SoundBlaster, l: &mut Lines, port: u32) -> u8 {
    sb.on_port_read(l, 1, port, 0).unwrap().0 as u8
}

fn reset(sb: &mut SoundBlaster, l: &mut Lines) {
    out(sb, l, 0x226, 1);
    out(sb, l, 0x226, 0);
}

/// Reset gives 0AAh; the version, identification, the speaker's state and
/// the test register answer through port A, port E saying when.
#[test]
fn dsp_resets_and_answers() {
    let (mut sb, mut l) = (SoundBlaster::new(), Lines::default());
    assert_eq!(inp(&mut sb, &mut l, 0x22E) & 0x80, 0, "nothing to read");
    reset(&mut sb, &mut l);
    assert_eq!(inp(&mut sb, &mut l, 0x22E) & 0x80, 0x80);
    assert_eq!(inp(&mut sb, &mut l, 0x22A), 0xAA);
    assert_eq!(inp(&mut sb, &mut l, 0x22E) & 0x80, 0);
    assert_eq!(inp(&mut sb, &mut l, 0x22C) & 0x80, 0, "ready for a command");
    for c in [0xE1, 0xE0, 0x5A, 0xD1, 0xD8, 0xE4, 0x37, 0xE8] {
        out(&mut sb, &mut l, 0x22C, c);
    }
    let got: Vec<u8> = (0..5).map(|_| inp(&mut sb, &mut l, 0x22A)).collect();
    assert_eq!(got, [2, 1, 0xA5, 0xFF, 0x37]);
    assert_eq!(inp(&mut sb, &mut l, 0x22A), 0x37, "the last byte again");
}

/// Direct output plays a sample as it is written.
#[test]
fn dsp_direct_output() {
    let (mut sb, mut l) = (SoundBlaster::new(), Lines::default());
    reset(&mut sb, &mut l);
    for s in [0x10, 0x42, 0x10, 0x99] {
        out(&mut sb, &mut l, 0x22C, s);
    }
    assert_eq!(sb.played, 2);
    assert_eq!(&sb.pcm[..2], &[0x42, 0x99]);
}

/// Single-cycle DMA output: a byte asked for as the command starts and as
/// each sample plays, one a sample period (256 less the time constant, in
/// microseconds); the block's last byte raises the interrupt, which port E
/// acknowledges; a byte missing when its period starts is an underrun.
#[test]
fn dsp_single_cycle_dma() {
    let (mut sb, mut l) = (SoundBlaster::new(), Lines::default());
    reset(&mut sb, &mut l);
    for c in [0x40, 156, 0x14, 2, 0] {
        out(&mut sb, &mut l, 0x22C, c);
    }
    assert!(l.dreq[1], "asks for the first byte");
    assert_eq!(sb.on_port_dack_write(&mut l, 3, 0x11, false), None, "another channel's");
    assert!(sb.on_port_dack_write(&mut l, 1, 0x11, false).is_some());
    assert!(!l.dreq[1]);
    assert_eq!(sb.on_tick(&mut l, 99), 1, "the first period ends in 1 us");
    assert_eq!(sb.played, 0);
    assert_eq!(sb.on_tick(&mut l, 1), 100);
    assert_eq!((sb.played, sb.pcm[0]), (1, 0x11));
    assert!(l.dreq[1], "asks for the next as one plays");
    sb.on_port_dack_write(&mut l, 1, 0x22, false).unwrap();
    // The third byte late: the second plays, then the second again.
    assert_eq!(sb.on_tick(&mut l, 200), 100);
    assert_eq!(&sb.pcm[..3], &[0x11, 0x22, 0x22]);
    assert_eq!(sb.underruns, 1);
    assert!(!l.irq[5]);
    sb.on_port_dack_write(&mut l, 1, 0x33, true).unwrap();
    assert!(l.irq[5], "the block's last byte");
    assert!(!l.dreq[1]);
    assert_eq!(sb.on_tick(&mut l, 100), u32::MAX, "done once it plays");
    assert_eq!((sb.played, sb.pcm[3]), (4, 0x33));
    inp(&mut sb, &mut l, 0x22E);
    assert!(!l.irq[5], "acknowledged");
}

/// Auto-init output: the block (48h) starts over at its end, each end an
/// interrupt, until DAh ends it after the block playing.
#[test]
fn dsp_auto_init_dma() {
    let (mut sb, mut l) = (SoundBlaster::new(), Lines::default());
    reset(&mut sb, &mut l);
    for c in [0x40, 0xF6, 0x48, 1, 0, 0x1C] {
        out(&mut sb, &mut l, 0x22C, c);
    }
    let mut irqs = 0;
    let mut b = 0u8;
    for k in 0..12 {
        if k == 7 {
            out(&mut sb, &mut l, 0x22C, 0xDA);
        }
        if l.dreq[1] {
            sb.on_port_dack_write(&mut l, 1, b, false).unwrap();
            b += 1;
        }
        if l.irq[5] {
            irqs += 1;
            inp(&mut sb, &mut l, 0x22E);
        }
        sb.on_tick(&mut l, 10);
    }
    // Blocks of two bytes; DAh during the fourth block: it is the last.
    assert_eq!(sb.played, 8);
    assert_eq!(&sb.pcm[..8], &[0, 1, 2, 3, 4, 5, 6, 7]);
    assert_eq!((irqs, sb.underruns), (4, 0));
    assert!(!l.dreq[1]);
}

/// A 64 KB ROM image (filled with HLT): `code` at F000:0000 and the reset
/// vector's far jump to it.
fn rom(code: &[u8]) -> Vec<u8> {
    let mut r = vec![0xF4u8; 0x10000];
    r[..code.len()].copy_from_slice(code);
    r[0xFFF0..0xFFF5].copy_from_slice(&[0xEA, 0x00, 0x00, 0x00, 0xF0]); // jmp far F000:0000
    r
}

/// `sb_play.asm` (`nasm -f bin`): 256 bytes 0-255 at 1000h played by DMA 3
/// in auto-init, the DSP at 20 kHz in blocks of 64, IRQ 5 counted at 502h;
/// after the fourth interrupt DAh makes the fifth block the last.
const SB_PLAY: &[u8] = &[
    0xfa, 0x31, 0xc0, 0x8e, 0xd8, 0x8e, 0xd0, 0xbc, 0x00, 0x7c, 0xc7, 0x06, 0x34, 0x00, 0xb8, 0x00, 0xc7, 0x06, 0x36, 0x00,
    0x00, 0xf0, 0xb0, 0x11, 0xe6, 0x20, 0xb0, 0x08, 0xe6, 0x21, 0xb0, 0x04, 0xe6, 0x21, 0xb0, 0x01, 0xe6, 0x21, 0xb0, 0xdf,
    0xe6, 0x21, 0xbf, 0x00, 0x10, 0x30, 0xc0, 0xb9, 0x00, 0x01, 0x88, 0x05, 0xfe, 0xc0, 0x47, 0xe2, 0xf9, 0xb0, 0x07, 0xe6,
    0x0a, 0xe6, 0x0c, 0xb0, 0x5b, 0xe6, 0x0b, 0x30, 0xc0, 0xe6, 0x06, 0xb0, 0x10, 0xe6, 0x06, 0xb0, 0xff, 0xe6, 0x07, 0x30,
    0xc0, 0xe6, 0x07, 0xe6, 0x82, 0xb0, 0x03, 0xe6, 0x0a, 0xba, 0x26, 0x02, 0xb0, 0x01, 0xee, 0x30, 0xc0, 0xee, 0xba, 0x2e,
    0x02, 0xec, 0xa8, 0x80, 0x74, 0xfb, 0xba, 0x2a, 0x02, 0xec, 0xa2, 0x00, 0x05, 0xb0, 0x40, 0xe8, 0x36, 0x00, 0xb0, 0xce,
    0xe8, 0x31, 0x00, 0xb0, 0x48, 0xe8, 0x2c, 0x00, 0xb0, 0x3f, 0xe8, 0x27, 0x00, 0x30, 0xc0, 0xe8, 0x22, 0x00, 0xb0, 0xd1,
    0xe8, 0x1d, 0x00, 0xb0, 0x1c, 0xe8, 0x18, 0x00, 0xfb, 0xf4, 0x83, 0x3e, 0x02, 0x05, 0x04, 0x72, 0xf8, 0xb0, 0xda, 0xe8,
    0x0a, 0x00, 0xf4, 0x83, 0x3e, 0x02, 0x05, 0x05, 0x72, 0xf8, 0xfa, 0xf4, 0xba, 0x2c, 0x02, 0x50, 0xec, 0xa8, 0x80, 0x75,
    0xfb, 0x58, 0xee, 0xc3, 0x50, 0x52, 0xba, 0x2e, 0x02, 0xec, 0xff, 0x06, 0x02, 0x05, 0xb0, 0x20, 0xe6, 0x20, 0x5a, 0x58,
    0xcf,
];

/// The program's samples play in order, on time, none missed: 320 (five
/// blocks of 64, the buffer and its first quarter again) at 50 us each,
/// five interrupts, and then silence.
#[test]
fn dsp_plays_a_dma_buffer_under_a_program() {
    let mut sys = System::new();
    sys.board().rom.cells = rom(SB_PLAY);
    sys.reset();
    let irqs = |s: &mut System| u16::from_le_bytes([s.board().ram.cells[0x502], s.board().ram.cells[0x503]]);
    sys.run_until(50_000_000, |s| irqs(s) == 5).unwrap();
    let t0 = sys.seconds();
    // The last block's last byte, transferred, has yet to play.
    let end = t0 + 0.002;
    sys.run_until(50_000_000, |s| s.seconds() >= end).unwrap();
    let b = sys.board();
    assert_eq!(b.ram.cells[0x500], 0xAA, "reset");
    assert_eq!(irqs(&mut sys), 5);
    let b = sys.board();
    assert_eq!((b.sb.played, b.sb.underruns), (320, 0));
    let want: Vec<u8> = (0..320u32).map(|i| (i % 256) as u8).collect();
    assert_eq!(&b.sb.pcm[..320], &want[..]);
    assert!(b.sb.speaker);
    // The fifth interrupt as the 320th byte arrives, the 319th playing:
    // 319 periods of 50 us, and setting up.
    assert!((0.0155..0.0175).contains(&t0), "{t0}");
}
