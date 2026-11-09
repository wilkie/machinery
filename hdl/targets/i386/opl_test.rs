//! The OPL2 (`targets/pc/opl.machine`) on the 386 VGA board (`Board386Vga`,
//! at 388h and 228h): driven directly, and found by a program as AdLib
//! drivers find it (`opl_detect.asm`).

use machine_board386_vga::system::System;
use machine_board386_vga::{Opl2, Opl2Ports};

struct Quiet;
impl Opl2Ports for Quiet {}

fn reg(o: &mut Opl2, r: u8, v: u8) {
    o.on_port_write(&mut Quiet, 1, 0x388, 0, r as u16);
    o.on_port_write(&mut Quiet, 1, 0x389, 0, v as u16);
}

/// A chip after reset (its operators silent).
fn chip() -> Opl2 {
    let mut o = Opl2::new();
    o.on_reset(&mut Quiet);
    o
}

fn status(o: &mut Opl2) -> u8 {
    o.on_port_read(&mut Quiet, 1, 0x388, 0).unwrap().0 as u8
}

/// The AdLib's detection: both timers masked and the status cleared reads
/// no flags; timer 1 started from FFh overflows after one tick, 80 us (4
/// sample periods), setting bits 7 and 6. The tick reports when it is due.
#[test]
fn opl_timer1_as_detection_uses_it() {
    let mut o = chip();
    reg(&mut o, 4, 0x60);
    reg(&mut o, 4, 0x80);
    assert_eq!(status(&mut o) & 0xE0, 0);
    reg(&mut o, 2, 0xFF);
    reg(&mut o, 4, 0x21);
    assert_eq!(o.on_tick(&mut Quiet, 0), 4, "due in 4 sample periods");
    assert_eq!(o.on_tick(&mut Quiet, 3), 1);
    assert_eq!(status(&mut o) & 0xE0, 0, "not yet");
    o.on_tick(&mut Quiet, 1);
    assert_eq!(status(&mut o) & 0xE0, 0xC0);
    reg(&mut o, 4, 0x60);
    reg(&mut o, 4, 0x80);
    assert_eq!(status(&mut o) & 0xE0, 0, "cleared");
    assert_eq!(status(&mut o) & 0x1F, 0x06, "the low bits");
}

/// Timer 2 ticks every 320 us (16 sample periods) and starts again from its
/// preset; a masked timer sets no flag; a long catch-up crosses several
/// overflows at once.
#[test]
fn opl_timer2_and_masks() {
    let mut o = chip();
    reg(&mut o, 3, 0xFE);
    reg(&mut o, 4, 0x02);
    assert_eq!(o.on_tick(&mut Quiet, 31), 1);
    assert_eq!(status(&mut o) & 0xE0, 0);
    o.on_tick(&mut Quiet, 1);
    assert_eq!(status(&mut o) & 0xE0, 0xA0, "timer 2");
    reg(&mut o, 4, 0x80);
    // Many periods at once: it overflowed again, from FEh each time.
    assert_eq!(o.on_tick(&mut Quiet, 32 * 10 + 5), 32 - 5);
    assert_eq!(status(&mut o) & 0xE0, 0xA0);
    // Masked (bit 5), it counts but says nothing.
    reg(&mut o, 4, 0x80);
    reg(&mut o, 4, 0x22);
    o.on_tick(&mut Quiet, 64);
    assert_eq!(status(&mut o) & 0xE0, 0);
    // Stopped: never due.
    reg(&mut o, 4, 0x00);
    assert_eq!(o.on_tick(&mut Quiet, 10), u32::MAX);
}

/// A 64 KB ROM image (filled with HLT): `code` at F000:0000 and the reset
/// vector's far jump to it.
fn rom(code: &[u8]) -> Vec<u8> {
    let mut r = vec![0xF4u8; 0x10000];
    r[..code.len()].copy_from_slice(code);
    r[0xFFF0..0xFFF5].copy_from_slice(&[0xEA, 0x00, 0x00, 0x00, 0xF0]); // jmp far F000:0000
    r
}

/// `opl_detect.asm` (`nasm -f bin`): the detection under a program, timed by
/// reads of the status port; the statuses it read at 500h-502h.
const OPL_DETECT: &[u8] = &[
    0xfa, 0x31, 0xc0, 0x8e, 0xd8, 0xba, 0x88, 0x03, 0xb4, 0x04, 0xb0, 0x60, 0xe8, 0x36, 0x00, 0xb4, 0x04, 0xb0, 0x80, 0xe8,
    0x2f, 0x00, 0xec, 0xa2, 0x00, 0x05, 0xb4, 0x02, 0xb0, 0xff, 0xe8, 0x24, 0x00, 0xb4, 0x04, 0xb0, 0x21, 0xe8, 0x1d, 0x00,
    0xb9, 0xc8, 0x00, 0xec, 0xe2, 0xfd, 0xec, 0xa2, 0x01, 0x05, 0xb4, 0x04, 0xb0, 0x60, 0xe8, 0x0c, 0x00, 0xb4, 0x04, 0xb0,
    0x80, 0xe8, 0x05, 0x00, 0xec, 0xa2, 0x02, 0x05, 0xf4, 0x86, 0xc4, 0xee, 0xec, 0xec, 0x42, 0x86, 0xc4, 0xee, 0x4a, 0xb9,
    0x23, 0x00, 0xec, 0xe2, 0xfd, 0xc3,
];

#[test]
fn opl_found_by_a_program() {
    let mut sys = System::new();
    sys.board().rom.cells = rom(OPL_DETECT);
    sys.reset();
    sys.run_until(10_000_000, |s| s.cpu.halted).unwrap();
    let b = sys.board();
    assert_eq!(&b.ram.cells[0x500..0x503], &[0x06, 0xC6, 0x06]);
}

/// The samples made since `from` (two's complement in `pcm`).
fn samples(o: &Opl2, from: u32) -> Vec<i32> {
    (from..o.played).map(|i| o.pcm[(i & 0xFFFF) as usize] as i16 as i32).collect()
}

/// Channel 0 as a plain sine: the modulator silent (total level 63), the
/// carrier at full level, an instant attack, its sustain held at the top;
/// 440 Hz is F-number 580 in block 4 (49716 Hz / 2^16 a step).
fn sine_voice(o: &mut Opl2) {
    for (r, v) in [(0x20, 0x01), (0x40, 0x3F), (0x60, 0xF0), (0x80, 0x00), (0x23, 0x21), (0x43, 0x00), (0x63, 0xF0), (0x83, 0x0F), (0xC0, 0x00)] {
        reg(o, r, v);
    }
    reg(o, 0xA0, (580 & 0xFF) as u8);
}

/// A note: its pitch (zero crossings over a second) and its peak, the
/// tables' largest level (4084); nothing before key on.
#[test]
fn opl_plays_a_sine_note() {
    let mut o = chip();
    sine_voice(&mut o);
    o.on_tick(&mut Quiet, 1000);
    assert!(samples(&o, 0).iter().all(|&s| s == 0), "silent before key on");
    let from = o.played;
    reg(&mut o, 0xB0, 0x20 | (4 << 2) | (580 >> 8) as u8);
    o.on_tick(&mut Quiet, 49716);
    let s = samples(&o, from);
    let rising = s.windows(2).filter(|w| w[0] < 0 && w[1] >= 0).count();
    assert!((438..=442).contains(&rising), "{rising} Hz");
    let peak = s.iter().map(|v| v.abs()).max().unwrap();
    assert!((4080..=4085).contains(&peak), "peak {peak}");
}

/// Key off: the note releases (release rate 15: within a few ms) to
/// silence.
#[test]
fn opl_note_releases() {
    let mut o = chip();
    sine_voice(&mut o);
    reg(&mut o, 0xB0, 0x20 | (4 << 2) | (580 >> 8) as u8);
    o.on_tick(&mut Quiet, 5000);
    reg(&mut o, 0xB0, (4 << 2) | (580 >> 8) as u8);
    let from = o.played;
    o.on_tick(&mut Quiet, 5000);
    let s = samples(&o, from);
    let early = s[..200].iter().map(|v| v.abs()).max().unwrap();
    let late = s[4000..].iter().map(|v| v.abs()).max().unwrap();
    assert!(early > 1000, "still sounding just after key off: {early}");
    // (At full attenuation the sine's negative half is -1, as the chip's.)
    assert!(late <= 1, "silent once released: {late}");
}

/// FM: a modulator at full level, through the connection bit, moves the
/// carrier's phase, and feedback the modulator's own: the wave is no
/// longer a sine (more zero crossings than the pitch).
#[test]
fn opl_modulation_and_feedback() {
    let mut o = chip();
    sine_voice(&mut o);
    reg(&mut o, 0x40, 0x00);
    reg(&mut o, 0xC0, 0x0E);
    reg(&mut o, 0xB0, 0x20 | (4 << 2) | (580 >> 8) as u8);
    let from = o.played;
    o.on_tick(&mut Quiet, 49716);
    let s = samples(&o, from);
    let crossings = s.windows(2).filter(|w| w[0] < 0 && w[1] >= 0).count();
    assert!(crossings > 600, "{crossings}");
}


/// Envelope timing: decay rate 10 with block 4's key scale (2, even with
/// KSR off: rate 42, rate group 10: one step in four moves, six of eight
/// of them by 1) takes the carrier from the top to sustain level 15
/// (attenuation 496) in 2645 sample periods, where it holds.
#[test]
fn opl_decay_takes_its_time() {
    let mut o = chip();
    sine_voice(&mut o);
    reg(&mut o, 0x63, 0xFA);
    reg(&mut o, 0x83, 0xFF);
    reg(&mut o, 0xB0, 0x20 | (4 << 2) | (580 >> 8) as u8);
    o.on_tick(&mut Quiet, 2600);
    assert!(o.op_env[3] < 496 && o.op_stage[3] == 1, "still decaying: {} {}", o.op_env[3], o.op_stage[3]);
    o.on_tick(&mut Quiet, 100);
    assert!(o.op_env[3] >= 496 && o.op_stage[3] == 2, "at the sustain level: {} {}", o.op_env[3], o.op_stage[3]);
    o.on_tick(&mut Quiet, 5000);
    assert_eq!(o.op_env[3], 496, "held there (sustaining)");
}

/// The peaks of each `w`-sample window.
fn peaks(s: &[i32], w: usize) -> Vec<i32> {
    s.chunks(w).map(|c| c.iter().map(|v| v.abs()).max().unwrap()).collect()
}

/// Tremolo (AM): the carrier's level swings by 4.8 dB over 210 steps of 64
/// samples (BDh bit 7), by 1 dB without.
#[test]
fn opl_tremolo() {
    for (bd, lo, hi) in [(0x80u8, 1.6, 1.9), (0x00, 1.08, 1.18)] {
        let mut o = chip();
        sine_voice(&mut o);
        reg(&mut o, 0x23, 0xA1);
        reg(&mut o, 0xBD, bd);
        reg(&mut o, 0xB0, 0x20 | (4 << 2) | (580 >> 8) as u8);
        let from = o.played;
        o.on_tick(&mut Quiet, 13440 * 2);
        let p = peaks(&samples(&o, from), 500);
        let (max, min) = (*p.iter().max().unwrap() as f64, *p.iter().min().unwrap() as f64);
        assert!((lo..hi).contains(&(max / min)), "BDh {bd:#x}: {max} / {min}");
    }
}

/// Vibrato (VIB): the pitch swings by 1/128 of the F-number (4 of 580 at
/// 440 Hz: +-0.7%) over 8 steps of 1024 samples (BDh bit 6).
#[test]
fn opl_vibrato() {
    let mut o = chip();
    sine_voice(&mut o);
    reg(&mut o, 0x23, 0x61);
    reg(&mut o, 0xBD, 0x40);
    reg(&mut o, 0xB0, 0x20 | (4 << 2) | (580 >> 8) as u8);
    let from = o.played;
    o.on_tick(&mut Quiet, 8192 * 2);
    let s = samples(&o, from);
    // Rising zero crossings, to a fraction of a sample.
    let at: Vec<f64> = s.windows(2).enumerate().filter(|(_, w)| w[0] < 0 && w[1] >= 0).map(|(i, w)| i as f64 + (-w[0]) as f64 / (w[1] - w[0]) as f64).collect();
    let hz: Vec<f64> = at.windows(2).map(|w| 49716.0 / (w[1] - w[0])).collect();
    let (min, max) = hz.iter().fold((f64::MAX, 0f64), |(a, b), &h| (a.min(h), b.max(h)));
    assert!(min < 438.0 && max > 442.0 && min > 432.0 && max < 448.0, "{min:.1}..{max:.1} Hz");
}

/// Rhythm mode (BDh bit 5): the bass drum keys channel 6's operators (bit
/// 4) without its channel's key; the hi-hat (bit 0), channel 7's
/// modulator, is noise (zero crossings far from its pitch's); clearing the
/// bits releases them.
#[test]
fn opl_rhythm_mode() {
    let mut o = chip();
    // Channel 6 (operators 12 and 15, offsets 10h and 13h) as channel 0's
    // sine; channel 7's modulator (operator 13, offset 11h) loud and held.
    for (r, v) in [(0x30, 0x01), (0x50, 0x3F), (0x70, 0xF0), (0x90, 0x0F), (0x33, 0x21), (0x53, 0x00), (0x73, 0xF0), (0x93, 0x0F), (0xC6, 0x00)] {
        reg(&mut o, r, v);
    }
    reg(&mut o, 0xA6, (580 & 0xFF) as u8);
    reg(&mut o, 0xB6, (4 << 2) | (580 >> 8) as u8);
    reg(&mut o, 0xBD, 0x30);
    let from = o.played;
    o.on_tick(&mut Quiet, 49716);
    let s = samples(&o, from);
    let crossings = s.windows(2).filter(|w| w[0] < 0 && w[1] >= 0).count();
    assert!((438..=442).contains(&crossings), "bass drum: {crossings} Hz");
    assert!(s.iter().map(|v| v.abs()).max().unwrap() > 8000, "heard doubled");
    // The hi-hat alone.
    reg(&mut o, 0xBD, 0x20);
    o.on_tick(&mut Quiet, 5000);
    for (r, v) in [(0x31, 0x21), (0x51, 0x00), (0x71, 0xF0), (0x91, 0x0F)] {
        reg(&mut o, r, v);
    }
    reg(&mut o, 0xA7, (580 & 0xFF) as u8);
    reg(&mut o, 0xB7, (4 << 2) | (580 >> 8) as u8);
    reg(&mut o, 0xBD, 0x21);
    let from = o.played;
    o.on_tick(&mut Quiet, 49716);
    let s = samples(&o, from);
    let crossings = s.windows(2).filter(|w| w[0] < 0 && w[1] >= 0).count();
    assert!(crossings > 2000, "hi-hat is noise: {crossings}");
    reg(&mut o, 0xBD, 0x20);
    let from = o.played;
    o.on_tick(&mut Quiet, 10000);
    assert!(samples(&o, from)[9000..].iter().all(|v| v.abs() <= 1), "released");
}
