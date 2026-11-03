//! Drives the crate generated from `board.machine`'s `At286`: the IBM PC/AT
//! (5170, the 8 MHz type 3 board) booting IBM's own BIOS (15NOV85, from
//! `~/roms` or `PC_ROMS`; not in the repository), and its system-board
//! devices driven directly.

use machine_at286::system::System;
use machine_at286::{Kbc8042, Mc146818, Pic8259};

fn roms() -> Option<std::path::PathBuf> {
    match std::env::var("PC_ROMS") {
        Ok(d) => Some(std::path::PathBuf::from(d)),
        Err(_) => Some(std::path::PathBuf::from(std::env::var("HOME").ok()?).join("roms")),
    }
}

/// The BIOS from its two chips: U27 holds the even bytes, U47 the odd.
fn bios() -> Option<Vec<u8>> {
    let dir = roms()?;
    let even = std::fs::read(dir.join("BIOS_5170_15NOV85_U27_62X0820_27256.BIN")).ok()?;
    let odd = std::fs::read(dir.join("BIOS_5170_15NOV85_U47_62X0821_27256.BIN")).ok()?;
    if even.len() != 0x8000 || odd.len() != 0x8000 {
        return None;
    }
    let mut img = vec![0u8; 0x10000];
    for i in 0..0x8000 {
        img[2 * i] = even[i];
        img[2 * i + 1] = odd[i];
    }
    Some(img)
}

/// The battery-backed CMOS of a configured AT: the time, register A as the
/// BIOS leaves it (32.768 kHz divider, 1024 Hz periodic rate), B (24-hour,
/// BCD), the diagnostic and shutdown bytes clear, a 1.2 MB diskette drive A
/// (type 2) and no hard disk, equipment: one diskette drive and an
/// 80-column colour display, 640 KB base memory, and the checksum over
/// 10h-2Dh in 2Eh-2Fh. (An AT without a diskette drive is a configuration
/// error to its BIOS.)
fn cmos() -> [u8; 64] {
    let mut c = [0u8; 64];
    c[0..10].copy_from_slice(&[0x00, 0, 0x30, 0, 0x12, 0, 0x03, 0x02, 0x11, 0x26]);
    c[10] = 0x26;
    c[11] = 0x02;
    c[13] = 0x80;
    c[0x10] = 0x20;
    c[0x14] = 0x21;
    c[0x15] = 0x80;
    c[0x16] = 0x02;
    let sum: u16 = c[0x10..=0x2D].iter().map(|&b| b as u16).sum();
    c[0x2E] = (sum >> 8) as u8;
    c[0x2F] = sum as u8;
    c[0x32] = 0x19;
    c
}

/// The CGA's text page, a line per row (blank rows left out).
fn screen(sys: &mut System) -> Vec<String> {
    let v = &sys.board().cga.vram;
    (0..25)
        .map(|row| {
            (0..80)
                .map(|c| match v[(row * 80 + c) * 2] {
                    0 => ' ',
                    b if (0x20..0x7F).contains(&b) => b as char,
                    _ => '.',
                })
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .filter(|l| !l.is_empty())
        .collect()
}

/// IBM's BIOS runs its power-on self test on the model up to its prompt
/// for F1: the processor, the CMOS, both DMA controllers and the page
/// registers, the timer and the refresh flip-flop, 640 KB of memory
/// (tested from protected mode, and back through the keyboard
/// controller's processor reset), the keyboard controller and the
/// keyboard. With no diskette controller yet, it reports that (601), and
/// so the configuration (162).
#[test]
fn ibm_at_bios_posts() {
    let Some(image) = bios() else {
        eprintln!("the IBM 5170 BIOS images are not in ~/roms; skipping");
        return;
    };
    let mut sys = System::new();
    sys.board().rom.cells = image;
    sys.board().rtc.cmos.copy_from_slice(&cmos());
    sys.reset();
    sys.run_until(60_000_000, |s| {
        s.board().post.last == 0x3F && s.board().cga.vram.windows(2).any(|w| w == b"F\x07")
    })
    .unwrap();
    let post = sys.board().post.clone();
    let codes = &post.codes[..(post.count as usize).min(4096)];
    assert_eq!(post.last, 0x3F, "POST reached its prompt");
    // The memory test in protected mode (81h, 85h), and POST going on
    // after the keyboard controller's reset brought the processor back.
    assert!(codes.windows(3).any(|w| w == [0x81, 0x85, 0x1A]), "{codes:02X?}");
    let text = screen(&mut sys);
    assert_eq!(text[0], "00640 KB OK", "{text:#?}");
    let errors: Vec<&str> = text[1..].iter().map(|l| l.trim()).collect();
    assert_eq!(
        errors,
        [
            "601-Diskette Error",
            "601-Diskette Error",
            "162-System Options Not Set-(Run SETUP)",
            "(RESUME = \"F1\" KEY)"
        ],
        "only the diskette controller is missing"
    );
}

/// A context for driving a device directly: its emits go nowhere.
struct Quiet;
impl machine_at286::Mc146818Ports for Quiet {}
impl machine_at286::Kbc8042Ports for Quiet {}
impl machine_at286::Pic8259Ports for Quiet {}

fn rtc_out(r: &mut Mc146818, reg: u8, v: u8) {
    r.on_port_write(&mut Quiet, 1, 0x70, 0, reg as u16);
    r.on_port_write(&mut Quiet, 1, 0x71, 0, v as u16);
}

fn rtc_in(r: &mut Mc146818, reg: u8) -> u8 {
    r.on_port_write(&mut Quiet, 1, 0x70, 0, reg as u16);
    r.on_port_read(&mut Quiet, 1, 0x71, 0).unwrap().0 as u8
}

/// The MC146818 counts in BCD across a year's end; its update sets the
/// update flag, which reading register C clears; update-in-progress shows
/// for the 244 us before an update; the periodic flag comes at register
/// A's rate (6: 1024 Hz, 32 ticks of 32.768 kHz).
#[test]
fn rtc_keeps_time() {
    let mut r = Mc146818::new();
    for (reg, v) in [(0u8, 0x59u8), (2, 0x59), (4, 0x23), (6, 0x07), (7, 0x31), (8, 0x12), (9, 0x99), (10, 0x26), (11, 0x02)] {
        rtc_out(&mut r, reg, v);
    }
    let _ = rtc_in(&mut r, 12);
    r.on_tick(&mut Quiet, 32768 - 4);
    assert_eq!(rtc_in(&mut r, 10) & 0x80, 0x80, "update in progress");
    assert_eq!(rtc_in(&mut r, 12) & 0x40, 0x40, "periodic flags meanwhile");
    r.on_tick(&mut Quiet, 4);
    let t: Vec<u8> = [0u8, 2, 4, 6, 7, 8, 9].iter().map(|&i| rtc_in(&mut r, i)).collect();
    assert_eq!(t, [0x00, 0x00, 0x00, 0x01, 0x01, 0x01, 0x00], "00:00:00, Sunday 1 January (19)00");
    assert_eq!(rtc_in(&mut r, 12) & 0x10, 0x10, "the update flag");
    assert_eq!(rtc_in(&mut r, 12), 0, "cleared by reading");
    r.on_tick(&mut Quiet, 31);
    assert_eq!(rtc_in(&mut r, 12) & 0x40, 0);
    r.on_tick(&mut Quiet, 1);
    assert_eq!(rtc_in(&mut r, 12) & 0x40, 0x40, "1024 Hz");
    assert_eq!(rtc_in(&mut r, 13), 0x80, "the battery is good");
}

/// Records the 8042's reset line.
#[derive(Default)]
struct Probe {
    reset: bool,
}
impl machine_at286::Kbc8042Ports for Probe {
    fn write_cpu_reset(&mut self, value: bool) {
        self.reset = value;
    }
}

/// The 8042: its self-test answers 55h at once and sets the system flag;
/// the keyboard acknowledges a reset and passes its own self-test (AAh), a
/// byte about a millisecond apart, the output buffer emptying between
/// them; the command byte reads back; FEh pulses the processor's reset.
#[test]
fn kbc_and_keyboard() {
    let mut k = Kbc8042::new();
    k.on_reset(&mut Quiet);
    let status = |k: &mut Kbc8042| k.on_port_read(&mut Quiet, 1, 0x64, 0).unwrap().0 as u8;
    let data = |k: &mut Kbc8042| k.on_port_read(&mut Quiet, 1, 0x60, 0).unwrap().0 as u8;
    assert_eq!(status(&mut k) & 0x05, 0, "cold: no system flag");
    k.on_port_write(&mut Quiet, 1, 0x64, 0, 0xAA);
    assert_eq!(status(&mut k) & 0x05, 0x05);
    assert_eq!(data(&mut k), 0x55);
    k.on_port_write(&mut Quiet, 1, 0x64, 0, 0x60);
    k.on_port_write(&mut Quiet, 1, 0x60, 0, 0x45);
    k.on_port_write(&mut Quiet, 1, 0x64, 0, 0x20);
    assert_eq!(data(&mut k), 0x45);
    k.on_port_write(&mut Quiet, 1, 0x60, 0, 0xFF);
    assert_eq!(status(&mut k) & 1, 0, "not yet");
    k.on_tick(&mut Quiet, 1000);
    assert_eq!(status(&mut k) & 1, 1);
    assert_eq!(data(&mut k), 0xFA);
    assert_eq!(status(&mut k) & 1, 0, "empty between bytes");
    k.on_tick(&mut Quiet, 1000);
    assert_eq!(data(&mut k), 0xAA);
    let mut probe = Probe::default();
    k.on_port_write(&mut probe, 1, 0x64, 0, 0xFE);
    assert!(probe.reset);
    k.on_tick(&mut probe, 7);
    assert!(!probe.reset, "a pulse");
}

/// An 8259 request needs its line held until it is acknowledged: one
/// whose line falls first is withdrawn.
#[test]
fn pic_withdraws_a_dropped_request() {
    let mut p = Pic8259::new();
    p.on_ir_request(&mut Quiet, 1, true);
    assert_eq!(p.irr & 2, 2);
    p.on_ir_request(&mut Quiet, 1, false);
    assert_eq!(p.irr & 2, 0);
}
