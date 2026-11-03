//! Drives the crate generated from `board.machine`'s `At286`: the IBM PC/AT
//! (5170, the 8 MHz type 3 board) booting IBM's own BIOS (15NOV85, from
//! `~/roms` or `PC_ROMS`; not in the repository) and DOS from disk images
//! there (MS-DOS 3.30 and 4.00, FreeDOS 1.4), and its system-board
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
    c[0x32] = 0x20;
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

/// An AT with IBM's BIOS and a configured CMOS, after reset; None without
/// the ROM images.
fn at() -> Option<System> {
    let image = bios()?;
    let mut sys = System::new();
    sys.board().rom.cells = image;
    sys.board().rtc.cmos.copy_from_slice(&cmos());
    sys.reset();
    Some(sys)
}

fn skip() {
    eprintln!("the IBM 5170 BIOS images are not in ~/roms; skipping");
}

/// Runs until the screen shows `text` (looking every 10,000
/// instructions), or `steps` instructions.
fn until_screen(sys: &mut System, text: &str, steps: u64) {
    let probe = text.as_bytes().to_vec();
    let mut n = 0u32;
    sys.run_until(steps, |s| {
        n += 1;
        if !n.is_multiple_of(10_000) {
            return false;
        }
        let chars: Vec<u8> = s.board().cga.vram.iter().step_by(2).take(2000).copied().collect();
        chars.windows(probe.len()).any(|w| w == probe.as_slice())
    })
    .unwrap();
    assert!(screen(sys).iter().any(|l| l.contains(text)), "no {text:?} on the screen: {:#?}", screen(sys));
}

/// Presses and releases a key (set-1 make code), and gives the BIOS time
/// to take both bytes.
fn press(sys: &mut System, code: u8) {
    for c in [code, code | 0x80] {
        sys.with_kbc(|k, ctx| k.fn_key(ctx, c));
        let t = sys.seconds() + 0.005;
        sys.run_until(1_000_000, |s| s.seconds() >= t).unwrap();
    }
}

/// IBM's BIOS runs its power-on self test on the model: the processor,
/// the CMOS, both DMA controllers and the page registers, the timer and the
/// refresh flip-flop, 640 KB of memory (tested from protected mode, and
/// back through the keyboard controller's processor reset), the keyboard
/// controller and keyboard, and the diskette controller and drive, with no
/// errors. With no disk to boot, it starts its ROM BASIC, which takes
/// typing through the keyboard controller.
#[test]
fn ibm_at_bios_posts_and_starts_basic() {
    let Some(mut sys) = at() else { return skip() };
    until_screen(&mut sys, "Bytes free", 150_000_000);
    let post = sys.board().post.clone();
    let codes = &post.codes[..(post.count as usize).min(4096)];
    // The memory test in protected mode (81h, 85h), and POST going on
    // after the keyboard controller's reset brought the processor back.
    assert!(codes.windows(3).any(|w| w == [0x81, 0x85, 0x1A]), "{codes:02X?}");
    assert_eq!(sys.board().ram.cells[0x416], 0, "POST's error flags");
    let text = screen(&mut sys);
    assert!(text[0].starts_with("The IBM Personal Computer Basic"), "{text:#?}");
    // PRINT 6*7
    for k in [0x19, 0x13, 0x17, 0x31, 0x14, 0x39, 0x07] {
        press(&mut sys, k);
    }
    sys.with_kbc(|k, ctx| k.fn_key(ctx, 0x2A)); // shift
    press(&mut sys, 0x09); // *
    sys.with_kbc(|k, ctx| k.fn_key(ctx, 0xAA));
    press(&mut sys, 0x08);
    press(&mut sys, 0x1C); // enter
    until_screen(&mut sys, " 42", 20_000_000);
    assert!(screen(&mut sys).iter().any(|l| l == "print 6*7"), "{:#?}", screen(&mut sys));
}

/// A boot sector: prints "Booted from A:", reads sectors 2 and 3 with INT
/// 13h (DMA through the diskette controller) and prints the message that
/// straddles them, writes itself to sector 5, and halts.
const BOOT: &[u8] = &[
    0x31, 0xc0, 0x8e, 0xd8, 0x8e, 0xc0, 0x8e, 0xd0, 0xbc, 0x00, 0x7c, 0xbe, 0x52, 0x7c, 0xe8, 0x32,
    0x00, 0xb8, 0x02, 0x02, 0xb9, 0x02, 0x00, 0x31, 0xd2, 0xbb, 0x00, 0x80, 0xcd, 0x13, 0x72, 0x1c,
    0xbe, 0xf4, 0x81, 0xe8, 0x1d, 0x00, 0xb8, 0x01, 0x03, 0xb9, 0x05, 0x00, 0x31, 0xd2, 0xbb, 0x00,
    0x7c, 0xcd, 0x13, 0x72, 0x07, 0xbe, 0x63, 0x7c, 0xe8, 0x08, 0x00, 0xf4, 0xbe, 0x74, 0x7c, 0xe8,
    0x01, 0x00, 0xf4, 0xac, 0x08, 0xc0, 0x74, 0x09, 0xb4, 0x0e, 0xbb, 0x07, 0x00, 0xcd, 0x10, 0xeb,
    0xf2, 0xc3, 0x42, 0x6f, 0x6f, 0x74, 0x65, 0x64, 0x20, 0x66, 0x72, 0x6f, 0x6d, 0x20, 0x41, 0x3a,
    0x0d, 0x0a, 0x00, 0x0d, 0x0a, 0x57, 0x72, 0x6f, 0x74, 0x65, 0x20, 0x73, 0x65, 0x63, 0x74, 0x6f,
    0x72, 0x20, 0x35, 0x00, 0x44, 0x69, 0x73, 0x6b, 0x20, 0x65, 0x72, 0x72, 0x6f, 0x72,
];

/// The AT boots from a 1.2 MB disk in drive A: its BIOS reads the boot
/// sector through the diskette controller and DMA and runs it, and the
/// boot sector's own reads and write go through them too.
#[test]
fn ibm_at_boots_from_a_diskette() {
    let Some(mut sys) = at() else { return skip() };
    let mut disk = vec![0u8; 80 * 2 * 15 * 512];
    disk[..BOOT.len()].copy_from_slice(BOOT);
    disk[510] = 0x55;
    disk[511] = 0xAA;
    disk[512..1024].fill(b'-');
    let msg = b"read across two sectors";
    disk[512 + 500..512 + 500 + msg.len()].copy_from_slice(msg);
    disk[512 + 500 + msg.len()] = 0;
    {
        let f = &mut sys.board().fdc;
        f.image[..disk.len()].copy_from_slice(&disk);
        f.present = true;
        f.cylinders = 80;
        f.heads = 2;
        f.sectors = 15;
        f.rate = 0;
    }
    until_screen(&mut sys, "Wrote sector 5", 100_000_000);
    let text = screen(&mut sys);
    assert!(text.iter().any(|l| l == "Booted from A:"), "{text:#?}");
    assert!(text.iter().any(|l| l == "read across two sectors"), "{text:#?}");
    let img = &sys.board().fdc.image;
    assert_eq!(&img[4 * 512..5 * 512], &img[..512], "the boot sector, written to sector 5");
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
    a20: Option<bool>,
}
impl machine_at286::Kbc8042Ports for Probe {
    fn write_cpu_reset(&mut self, value: bool) {
        self.reset = value;
    }
    fn write_a20(&mut self, value: bool) {
        self.a20 = Some(value);
    }
}

/// The 8042: its self-test answers 55h after about a millisecond and sets
/// the system flag;
/// the keyboard acknowledges a reset and passes its own self-test (AAh), a
/// byte about a millisecond apart, the output buffer emptying between
/// them; the command byte reads back; FEh pulses the processor's reset;
/// D1h writes the output port (A20), and a self-test opens A20 again.
#[test]
fn kbc_and_keyboard() {
    let mut k = Kbc8042::new();
    k.on_reset(&mut Quiet);
    let status = |k: &mut Kbc8042| k.on_port_read(&mut Quiet, 1, 0x64, 0).unwrap().0 as u8;
    let data = |k: &mut Kbc8042| k.on_port_read(&mut Quiet, 1, 0x60, 0).unwrap().0 as u8;
    assert_eq!(status(&mut k) & 0x05, 0, "cold: no system flag");
    k.on_port_write(&mut Quiet, 1, 0x64, 0, 0xAA);
    assert_eq!(status(&mut k) & 0x05, 0x04, "the self-test takes a while");
    k.on_tick(&mut Quiet, 1193);
    assert_eq!(status(&mut k) & 0x05, 0x05);
    assert_eq!(data(&mut k), 0x55);
    k.on_port_write(&mut Quiet, 1, 0x64, 0, 0x60);
    k.on_port_write(&mut Quiet, 1, 0x60, 0, 0x45);
    k.on_port_write(&mut Quiet, 1, 0x64, 0, 0x20);
    k.on_tick(&mut Quiet, 30);
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
    k.on_port_write(&mut probe, 1, 0x64, 0, 0xD1);
    k.on_port_write(&mut probe, 1, 0x60, 0, 0xDD);
    assert_eq!(probe.a20, Some(false), "A20 closed");
    assert!(!probe.reset);
    k.on_port_write(&mut probe, 1, 0x64, 0, 0xAA);
    assert_eq!(probe.a20, Some(true), "the self-test opens it");
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

impl machine_at286::Fdc765Ports for Quiet {}

fn fdc_cmd(f: &mut machine_at286::Fdc765, bytes: &[u8]) {
    for &b in bytes {
        f.on_port_write(&mut Quiet, 1, 0x3F5, 0, b as u16);
    }
}

fn fdc_result(f: &mut machine_at286::Fdc765) -> Vec<u8> {
    let mut r = Vec::new();
    while f.on_port_read(&mut Quiet, 1, 0x3F4, 0).unwrap().0 & 0xC0 == 0xC0 {
        r.push(f.on_port_read(&mut Quiet, 1, 0x3F5, 0).unwrap().0 as u8);
    }
    r
}

/// The diskette controller directly: a reset reports each drive's ready
/// change; recalibrating from beyond 77 tracks stops short (equipment
/// check) and a second gets there; the drive's track-0 sensor tells where
/// its head is; Read ID at the wrong data rate finds no address mark.
#[test]
fn fdc_seeks_and_senses() {
    let mut f = machine_at286::Fdc765::new();
    f.on_port_write(&mut Quiet, 1, 0x3F2, 0, 0x00);
    f.on_port_write(&mut Quiet, 1, 0x3F2, 0, 0x1C); // out of reset, drive A's motor
    for d in 0..4u8 {
        fdc_cmd(&mut f, &[0x08]);
        assert_eq!(fdc_result(&mut f), [0xC0 | d, 0]);
    }
    fdc_cmd(&mut f, &[0x0F, 0x00, 80]); // seek to 80
    f.on_tick(&mut Quiet, 100 * 1193);
    fdc_cmd(&mut f, &[0x08]);
    assert_eq!(fdc_result(&mut f), [0x20, 80]);
    fdc_cmd(&mut f, &[0x07, 0x00]); // recalibrate: 77 steps
    f.on_tick(&mut Quiet, 100 * 1193);
    fdc_cmd(&mut f, &[0x08]);
    assert_eq!(fdc_result(&mut f), [0x70, 0], "equipment check");
    fdc_cmd(&mut f, &[0x04, 0x00]);
    assert_eq!(fdc_result(&mut f)[0] & 0x10, 0, "not at track 0");
    fdc_cmd(&mut f, &[0x07, 0x00]);
    f.on_tick(&mut Quiet, 100 * 1193);
    fdc_cmd(&mut f, &[0x08]);
    assert_eq!(fdc_result(&mut f), [0x20, 0]);
    fdc_cmd(&mut f, &[0x04, 0x00]);
    assert_eq!(fdc_result(&mut f)[0] & 0x10, 0x10, "track 0");
    // A 1.2 MB disk (500 kbit/s) read at 250 kbit/s.
    f.present = true;
    f.rate = 0;
    f.on_port_write(&mut Quiet, 1, 0x3F7, 0, 2);
    fdc_cmd(&mut f, &[0x4A, 0x00]);
    assert_eq!(&fdc_result(&mut f)[..3], &[0x40, 0x01, 0x00], "missing address mark");
    f.on_port_write(&mut Quiet, 1, 0x3F7, 0, 0);
    fdc_cmd(&mut f, &[0x4A, 0x00]);
    assert_eq!(fdc_result(&mut f), [0x00, 0, 0, 0, 0, 1, 2], "cylinder 0, head 0, sector 1, 512 bytes");
}

// --- DOS, from disk images in ~/roms (not in the repository) ---

/// A disk image from the ROM directory, if it is there.
fn disk(name: &str) -> Option<Vec<u8>> {
    std::fs::read(roms()?.join(name)).ok()
}

/// Puts `d` in drive A, its geometry from its boot sector's parameter
/// block (15 sectors a track is high density, 500 kbit/s; fewer, 300
/// kbit/s in the AT's drive), and raises the disk-change line, as opening
/// and closing the door would.
fn insert(sys: &mut System, d: &[u8]) {
    let spt = u16::from_le_bytes([d[24], d[25]]) as u32;
    let heads = u16::from_le_bytes([d[26], d[27]]) as u32;
    let total = u16::from_le_bytes([d[19], d[20]]) as u32;
    let f = &mut sys.board().fdc;
    f.image[..d.len()].copy_from_slice(d);
    f.present = true;
    f.sectors = spt;
    f.heads = heads;
    f.cylinders = total / (spt * heads);
    f.rate = if spt >= 15 { 0 } else { 1 };
    f.changed = true;
}

/// Types keys (set-1 make codes), each pressed and released.
fn type_keys(sys: &mut System, keys: &[u8]) {
    for &k in keys {
        press(sys, k);
    }
}

const ENTER: u8 = 0x1C;

/// MS-DOS 3.30 boots from its 360 KB disk (read double-stepped at 300
/// kbit/s in the AT's 1.2 MB drive): it takes the date and time from the
/// clock, answers VER, and lists the disk.
#[test]
fn ms_dos_330_boots() {
    let (Some(mut sys), Some(d)) = (at(), disk("msdos330/DISK01.IMG")) else { return skip() };
    insert(&mut sys, &d);
    until_screen(&mut sys, "Current date is Mon 11-02-2026", 150_000_000);
    press(&mut sys, ENTER);
    until_screen(&mut sys, "Enter new time", 20_000_000);
    press(&mut sys, ENTER);
    until_screen(&mut sys, "A>", 20_000_000);
    type_keys(&mut sys, &[0x2F, 0x12, 0x13, ENTER]); // VER
    until_screen(&mut sys, "MS-DOS Version 3.30", 20_000_000);
    type_keys(&mut sys, &[0x20, 0x17, 0x13, ENTER]); // DIR
    until_screen(&mut sys, "bytes free", 50_000_000);
    let text = screen(&mut sys);
    assert!(text.iter().any(|l| l.starts_with("FORMAT   COM")), "{text:#?}");
}

/// MS-DOS 4.00's INSTALL disk asks for SELECT; with the disks swapped
/// (the change line raised) SELECT runs its full-screen installer, which
/// asks for INSTALL back.
#[test]
fn ms_dos_400_installs_from_two_disks() {
    let (Some(mut sys), Some(install), Some(select)) = (at(), disk("msdos400/INSTALL.IMG"), disk("msdos400/SELECT.IMG")) else {
        return skip();
    };
    insert(&mut sys, &install);
    until_screen(&mut sys, "Insert SELECT diskette in drive A:", 150_000_000);
    // As a person would: a moment later (INSTALL empties the keyboard
    // buffer before it waits for Enter).
    let t = sys.seconds() + 1.0;
    sys.run_until(100_000_000, |s| s.seconds() >= t).unwrap();
    insert(&mut sys, &select);
    press(&mut sys, ENTER);
    until_screen(&mut sys, "Continuing Installation", 300_000_000);
    until_screen(&mut sys, "Insert INSTALL diskette in drive A", 100_000_000);
}

/// FreeDOS 1.4 boots from its 1.2 MB disk into its installer.
#[test]
fn freedos_boots() {
    let (Some(mut sys), Some(d)) = (at(), disk("freedos/120m/x86BOOT.img")) else { return skip() };
    insert(&mut sys, &d);
    until_screen(&mut sys, "Welcome to the FreeDOS 1.4 installation program", 150_000_000);
}

// --- the fixed disk ---

/// A 20 MB fixed disk in the AT: type 2 in the CMOS (615 cylinders, 4
/// heads, 17 sectors), blank.
fn fixed_disk(sys: &mut System) {
    let c = &mut sys.board().rtc.cmos;
    c[0x12] = 0x20;
    let sum: u16 = c[0x10..=0x2D].iter().map(|&b| b as u16).sum();
    c[0x2E] = (sum >> 8) as u8;
    c[0x2F] = sum as u8;
    let h = &mut sys.board().hdc;
    h.present = true;
    h.cylinders = 615;
    h.heads = 4;
    h.sectors = 17;
}

/// The boot sector of `BOOT` for drive C (80h).
const BOOT_C: &[u8] = &[
    0x31, 0xc0, 0x8e, 0xd8, 0x8e, 0xc0, 0x8e, 0xd0, 0xbc, 0x00, 0x7c, 0xbe, 0x54, 0x7c, 0xe8, 0x34,
    0x00, 0xb8, 0x02, 0x02, 0xb9, 0x02, 0x00, 0xba, 0x80, 0x00, 0xbb, 0x00, 0x80, 0xcd, 0x13, 0x72,
    0x1d, 0xbe, 0xf4, 0x81, 0xe8, 0x1e, 0x00, 0xb8, 0x01, 0x03, 0xb9, 0x05, 0x00, 0xba, 0x80, 0x00,
    0xbb, 0x00, 0x7c, 0xcd, 0x13, 0x72, 0x07, 0xbe, 0x65, 0x7c, 0xe8, 0x08, 0x00, 0xf4, 0xbe, 0x76,
    0x7c, 0xe8, 0x01, 0x00, 0xf4, 0xac, 0x08, 0xc0, 0x74, 0x09, 0xb4, 0x0e, 0xbb, 0x07, 0x00, 0xcd,
    0x10, 0xeb, 0xf2, 0xc3, 0x42, 0x6f, 0x6f, 0x74, 0x65, 0x64, 0x20, 0x66, 0x72, 0x6f, 0x6d, 0x20,
    0x43, 0x3a, 0x0d, 0x0a, 0x00, 0x0d, 0x0a, 0x57, 0x72, 0x6f, 0x74, 0x65, 0x20, 0x73, 0x65, 0x63,
    0x74, 0x6f, 0x72, 0x20, 0x35, 0x00, 0x44, 0x69, 0x73, 0x6b, 0x20, 0x65, 0x72, 0x72, 0x6f, 0x72,
];

/// The AT boots from its fixed disk when drive A is empty: the BIOS reads
/// the boot sector through the WD1003, 16 bits a word, and the boot
/// sector's own reads (across two sectors) and write go through it too.
#[test]
fn ibm_at_boots_from_the_fixed_disk() {
    let Some(mut sys) = at() else { return skip() };
    fixed_disk(&mut sys);
    {
        let h = &mut sys.board().hdc;
        h.image[..BOOT_C.len()].copy_from_slice(BOOT_C);
        h.image[510] = 0x55;
        h.image[511] = 0xAA;
        h.image[512..1024].fill(b'-');
        let msg = b"read across two sectors";
        h.image[512 + 500..512 + 500 + msg.len()].copy_from_slice(msg);
        h.image[512 + 500 + msg.len()] = 0;
    }
    until_screen(&mut sys, "Wrote sector 5", 200_000_000);
    let text = screen(&mut sys);
    assert!(text.iter().any(|l| l == "Booted from C:"), "{text:#?}");
    assert!(text.iter().any(|l| l == "read across two sectors"), "{text:#?}");
    assert_eq!(sys.board().ram.cells[0x416], 0, "POST's error flags");
    assert_eq!(sys.board().ram.cells[0x475], 1, "one fixed disk");
    let img = &sys.board().hdc.image;
    assert_eq!(&img[4 * 512..5 * 512], &img[..512], "the boot sector, written to sector 5");
}

/// Waits for `prompt`, then a moment, then types `keys`.
fn answer(sys: &mut System, prompt: &str, keys: &[u8], steps: u64) {
    until_screen(sys, prompt, steps);
    let t = sys.seconds() + 0.5;
    sys.run_until(100_000_000, |s| s.seconds() >= t).unwrap();
    type_keys(sys, keys);
}

/// MS-DOS 3.30 installs itself on the fixed disk and boots from it: FDISK
/// makes the DOS partition and restarts the AT, FORMAT C:/S formats it
/// (20 MB) and copies the system, and with drive A empty Ctrl-Alt-Del
/// brings up MS-DOS from C:.
#[test]
fn ms_dos_330_installs_on_the_fixed_disk() {
    let (Some(mut sys), Some(d)) = (at(), disk("msdos330/DISK01.IMG")) else { return skip() };
    fixed_disk(&mut sys);
    insert(&mut sys, &d);
    answer(&mut sys, "Enter new date", &[ENTER], 150_000_000);
    answer(&mut sys, "Enter new time", &[ENTER], 20_000_000);
    // FDISK: create the DOS partition, the primary one, at the largest
    // size and active.
    answer(&mut sys, "A>", &[0x21, 0x20, 0x17, 0x1F, 0x25, ENTER], 20_000_000);
    answer(&mut sys, "Enter choice:", &[ENTER], 50_000_000);
    answer(&mut sys, "Create Primary DOS partition", &[ENTER], 50_000_000);
    answer(&mut sys, "(Y/N)", &[ENTER], 50_000_000);
    answer(&mut sys, "Press any key when ready", &[ENTER], 100_000_000);
    // Restarted: from the diskette again.
    answer(&mut sys, "Enter new date", &[ENTER], 200_000_000);
    answer(&mut sys, "Enter new time", &[ENTER], 20_000_000);
    // FORMAT C:/S
    sys.with_kbc(|k, ctx| k.fn_key(ctx, 0x2A));
    answer(&mut sys, "A>", &[0x21, 0x18, 0x13, 0x32, 0x1E, 0x14, 0x39, 0x2E, 0x27], 20_000_000);
    sys.with_kbc(|k, ctx| k.fn_key(ctx, 0xAA));
    type_keys(&mut sys, &[0x35, 0x1F, ENTER]);
    answer(&mut sys, "Proceed with Format (Y/N)?", &[0x15, ENTER], 50_000_000);
    until_screen(&mut sys, "bytes available on disk", 1_500_000_000);
    let text = screen(&mut sys);
    assert!(text.iter().any(|l| l == "System transferred"), "{text:#?}");
    assert!(text.iter().any(|l| l.trim() == "21309440 bytes total disk space"), "{text:#?}");
    // Drive A empty, Ctrl-Alt-Del.
    sys.board().fdc.present = false;
    sys.board().fdc.changed = true;
    for k in [0x1D, 0x38, 0x53, 0xD3, 0xB8, 0x9D] {
        sys.with_kbc(|kb, ctx| kb.fn_key(ctx, k));
        let t = sys.seconds() + 0.01;
        sys.run_until(1_000_000, |s| s.seconds() >= t).unwrap();
    }
    // Until the restart has cleared the screen of FORMAT's report.
    let mut n = 0u32;
    sys.run_until(200_000_000, |s| {
        n += 1;
        n.is_multiple_of(10_000) && !screen(s).iter().any(|l| l.contains("bytes available"))
    })
    .unwrap();
    answer(&mut sys, "Enter new date", &[ENTER], 200_000_000);
    answer(&mut sys, "Enter new time", &[ENTER], 20_000_000);
    answer(&mut sys, "C>", &[0x2F, 0x12, 0x13, ENTER], 20_000_000);
    until_screen(&mut sys, "MS-DOS Version 3.30", 20_000_000);
}
