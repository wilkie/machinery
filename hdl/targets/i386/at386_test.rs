//! Drives the crate generated from `board.machine`'s `At386Vga`: a 25 MHz
//! 386 AT with the PC/AT's system-board devices and IBM's VGA, booting
//! AMI's 1989 AT BIOS (IBM's own 5170 BIOS stops at its speed check: it
//! accepts only an 8 MHz AT) and MS-DOS. The ROM and disk images come from
//! `~/roms` or `PC_ROMS`; they are not in the repository, and the tests
//! skip without them.

use machine_at386_vga::system::System;

fn roms() -> Option<std::path::PathBuf> {
    match std::env::var("PC_ROMS") {
        Ok(d) => Some(std::path::PathBuf::from(d)),
        Err(_) => Some(std::path::PathBuf::from(std::env::var("HOME").ok()?).join("roms")),
    }
}

fn read(name: &str) -> Option<Vec<u8>> {
    std::fs::read(roms()?.join(name)).ok()
}

/// AMI's BIOS from its two chips (U27 even bytes, U47 odd).
fn ami_bios() -> Option<Vec<u8>> {
    let even = read("BIOS_5170_30APR89_U27_AMI_27256.BIN")?;
    let odd = read("BIOS_5170_30APR89_U47_AMI_27256.BIN")?;
    let mut img = vec![0u8; 0x10000];
    for i in 0..0x8000 {
        img[2 * i] = even[i];
        img[2 * i + 1] = odd[i];
    }
    Some(img)
}

/// The CMOS of a configured 386 AT: 2 November 2026, a 1.2 MB drive A, an
/// adapter with its own BIOS for display (the VGA), 640 KB and 3 MB of
/// extended memory, and the fixed disk type given (0: none).
fn cmos(disk_type: u8) -> [u8; 64] {
    let mut c = [0u8; 64];
    c[0..10].copy_from_slice(&[0x00, 0, 0x30, 0, 0x12, 0, 0x03, 0x02, 0x11, 0x26]);
    c[10] = 0x26;
    c[11] = 0x02;
    c[13] = 0x80;
    c[0x10] = 0x20;
    c[0x12] = disk_type << 4;
    c[0x14] = 0x01;
    c[0x15] = 0x80;
    c[0x16] = 0x02;
    c[0x17] = 0x00;
    c[0x18] = 0x0C;
    let sum: u16 = c[0x10..=0x2D].iter().map(|&b| b as u16).sum();
    c[0x2E] = (sum >> 8) as u8;
    c[0x2F] = sum as u8;
    c[0x32] = 0x20;
    c
}

fn skip() {
    eprintln!("the AMI BIOS, VGA BIOS or disk images are not in ~/roms; skipping");
}

/// A 386 AT with AMI's BIOS, IBM's VGA BIOS and a configured CMOS, after
/// reset; None without the ROM images.
fn at(disk_type: u8) -> Option<System> {
    let bios = ami_bios()?;
    let vga = read("ibm_vga.bin")?;
    let mut sys = System::new();
    sys.board().rom.cells = bios;
    sys.board().vga.bios[..0x6000].copy_from_slice(vga.get(0x2000..0x8000)?);
    sys.board().rtc.cmos.copy_from_slice(&cmos(disk_type));
    if disk_type == 2 {
        let h = &mut sys.board().hdc;
        h.present = true;
        h.cylinders = 615;
        h.heads = 4;
        h.sectors = 17;
    }
    sys.reset();
    Some(sys)
}

/// The VGA's text page (characters at the even bytes of plane 0), a line
/// per row (blank rows left out).
fn screen(sys: &mut System) -> Vec<String> {
    let p = &sys.board().vga.planes;
    (0..25)
        .map(|r| {
            (0..80)
                .map(|c| match p[(r * 80 + c) * 2] {
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

/// Runs until the screen shows `text` (looking every 10,000
/// instructions), or `steps` instructions.
fn until_screen(sys: &mut System, text: &str, steps: u64) {
    let mut n = 0u32;
    sys.run_until(steps, |s| {
        n += 1;
        n.is_multiple_of(10_000) && screen(s).iter().any(|l| l.contains(text))
    })
    .unwrap();
    assert!(screen(sys).iter().any(|l| l.contains(text)), "no {text:?} on the screen: {:#?}", screen(sys));
}

fn wait(sys: &mut System, seconds: f64) {
    let t = sys.seconds() + seconds;
    sys.run_until(400_000_000, |s| s.seconds() >= t).unwrap();
}

/// Presses and releases keys (set-1 make codes).
fn type_keys(sys: &mut System, keys: &[u8]) {
    for &k in keys {
        for c in [k, k | 0x80] {
            sys.with_kbc(|kb, ctx| kb.fn_key(ctx, c));
            wait(sys, 0.005);
        }
    }
}

/// Waits for `prompt`, then a moment, then types `keys`.
fn answer(sys: &mut System, prompt: &str, keys: &[u8], steps: u64) {
    until_screen(sys, prompt, steps);
    wait(sys, 0.5);
    type_keys(sys, keys);
}

const ENTER: u8 = 0x1C;

/// Puts `d` in drive A, its geometry from its parameter block, and raises
/// the change line.
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

/// AMI's BIOS on the 386 with the VGA: POST finds the processor (and no
/// coprocessor: its FNSTSW reads the floating bus), the memory, drive A
/// and the VGA, whose own BIOS it has run; MS-DOS 3.30 boots from drive A
/// in VGA text mode and answers VER.
#[test]
fn ami_bios_boots_ms_dos_on_the_386() {
    let (Some(mut sys), Some(d)) = (at(0), read("msdos330/DISK01.IMG")) else { return skip() };
    insert(&mut sys, &d);
    until_screen(&mut sys, "Current date is Mon 11-02-2026", 400_000_000);
    let text = screen(&mut sys);
    for want in ["Main Processor     : 80386", "Numeric Processor  : None", "Ext. Memory Size   : 3072 KB", "Display Type       : VGA or EGA"] {
        assert!(text.iter().any(|l| l.contains(want)), "{want:?}: {text:#?}");
    }
    assert_eq!(sys.board().ram.cells[0x449], 3, "VGA text mode 3");
    assert_eq!(sys.board().vga.frame_width, 720, "on the 28 MHz clock");
    answer(&mut sys, "Enter new date", &[ENTER], 20_000_000);
    answer(&mut sys, "Enter new time", &[ENTER], 20_000_000);
    answer(&mut sys, "A>", &[0x2F, 0x12, 0x13, ENTER], 20_000_000);
    until_screen(&mut sys, "MS-DOS Version 3.30", 20_000_000);
}

/// MS-DOS 3.30 installs on the 386's 20 MB fixed disk (FDISK, which
/// restarts the machine; FORMAT C:/S) and, with drive A empty, boots from
/// it after Ctrl-Alt-Del.
#[test]
fn ms_dos_installs_on_the_386s_fixed_disk() {
    let (Some(mut sys), Some(d)) = (at(2), read("msdos330/DISK01.IMG")) else { return skip() };
    insert(&mut sys, &d);
    answer(&mut sys, "Enter new date", &[ENTER], 400_000_000);
    answer(&mut sys, "Enter new time", &[ENTER], 20_000_000);
    answer(&mut sys, "A>", &[0x21, 0x20, 0x17, 0x1F, 0x25, ENTER], 20_000_000); // FDISK
    answer(&mut sys, "Enter choice:", &[ENTER], 50_000_000);
    answer(&mut sys, "Create Primary DOS partition", &[ENTER], 50_000_000);
    answer(&mut sys, "(Y/N)", &[ENTER], 50_000_000);
    answer(&mut sys, "Press any key when ready", &[ENTER], 100_000_000);
    answer(&mut sys, "Enter new date", &[ENTER], 400_000_000);
    answer(&mut sys, "Enter new time", &[ENTER], 20_000_000);
    // FORMAT C:/S
    until_screen(&mut sys, "A>", 20_000_000);
    wait(&mut sys, 0.5);
    type_keys(&mut sys, &[0x21, 0x18, 0x13, 0x32, 0x1E, 0x14, 0x39, 0x2E]);
    sys.with_kbc(|k, ctx| k.fn_key(ctx, 0x2A));
    type_keys(&mut sys, &[0x27]);
    sys.with_kbc(|k, ctx| k.fn_key(ctx, 0xAA));
    type_keys(&mut sys, &[0x35, 0x1F, ENTER]);
    answer(&mut sys, "Proceed with Format (Y/N)?", &[0x15, ENTER], 50_000_000);
    until_screen(&mut sys, "bytes available on disk", 3_000_000_000);
    assert!(screen(&mut sys).iter().any(|l| l == "System transferred"));
    sys.board().fdc.present = false;
    sys.board().fdc.changed = true;
    for k in [0x1D, 0x38, 0x53, 0xD3, 0xB8, 0x9D] {
        sys.with_kbc(|kb, ctx| kb.fn_key(ctx, k));
        wait(&mut sys, 0.01);
    }
    let mut n = 0u32;
    sys.run_until(400_000_000, |s| {
        n += 1;
        n.is_multiple_of(10_000) && !screen(s).iter().any(|l| l.contains("bytes available"))
    })
    .unwrap();
    answer(&mut sys, "Enter new date", &[ENTER], 400_000_000);
    answer(&mut sys, "Enter new time", &[ENTER], 20_000_000);
    answer(&mut sys, "C>", &[0x2F, 0x12, 0x13, ENTER], 20_000_000);
    until_screen(&mut sys, "MS-DOS Version 3.30", 20_000_000);
}

/// `A20.asm` from reset: memory at FFFF:0010 with the A20 gate open (it
/// comes up open, as the 8042's output port pins come up high), closed
/// through the 8042 (D1h, DDh), and open again (D1h, DFh); the readings at
/// 0:0500-0502.
const A20: &[u8] = &[
    0x31, 0xc0, 0x8e, 0xc0, 0x26, 0xc6, 0x06, 0x00, 0x00, 0x11, 0xb8, 0xff, 0xff, 0x8e, 0xd8, 0xc6,
    0x06, 0x10, 0x00, 0x22, 0xa0, 0x10, 0x00, 0x26, 0xa2, 0x00, 0x05, 0xb0, 0xd1, 0xe6, 0x64, 0xb0,
    0xdd, 0xe6, 0x60, 0xa0, 0x10, 0x00, 0x26, 0xa2, 0x01, 0x05, 0xc6, 0x06, 0x20, 0x00, 0x33, 0xb0,
    0xd1, 0xe6, 0x64, 0xb0, 0xdf, 0xe6, 0x60, 0xa0, 0x10, 0x00, 0x26, 0xa2, 0x02, 0x05, 0xf4,
];

/// The A20 gate: with it open FFFF:0010 is the first byte above 1 MB; the
/// 8042 closes it and the address wraps to 0, as on an 8086 (a write there
/// lands in low memory); opened again, the memory above 1 MB is back. No
/// ROM images needed: the program is the boot ROM.
#[test]
fn a20_gate_wraps_memory_at_1_mb() {
    // Direct memory bypasses the gate (SPEC 12): an emulator built so never
    // wraps.
    if machine_at386_vga::system::DIRECT_MEMORY {
        return;
    }
    let mut sys = System::new();
    let mut rom = vec![0xF4u8; 0x10000];
    rom[..A20.len()].copy_from_slice(A20);
    rom[0xFFF0..0xFFF5].copy_from_slice(&[0xEA, 0x00, 0x00, 0x00, 0xF0]); // jmp far F000:0000
    sys.board().rom.cells = rom;
    sys.reset();
    sys.run(10_000).unwrap();
    assert!(sys.cpu.halted);
    let r = &sys.board().ram.cells;
    assert_eq!(&r[0x500..0x503], &[0x22, 0x11, 0x22], "open, closed (wraps to 0:0000), open");
    assert_eq!((r[0x100000], r[0x010], r[0x100010]), (0x22, 0x33, 0), "the closed write wrapped to 0:0010");
}

/// Runs until `stop` (or `steps` instructions) and reports the stretch:
/// emulated time against wall time, and instructions.
fn timed(sys: &mut System, what: &str, steps: u64, stop: impl FnMut(&mut System) -> bool) {
    let t0 = sys.seconds();
    let wall = std::time::Instant::now();
    let n = sys.run_until(steps, stop).unwrap();
    let w = wall.elapsed().as_secs_f64();
    sys.catch_up_all();
    let t = sys.seconds() - t0;
    eprintln!("{what:>12}: {t:6.2} s emulated in {w:6.2} s ({:5.1}x real time); {n:>11} instructions, {:5.1} M/s", t / w, n as f64 / w / 1e6);
}

/// Has the screen got `text`? Looks every 10,000 instructions.
fn shows(text: &'static str) -> impl FnMut(&mut System) -> bool {
    let mut n = 0u32;
    move |s| {
        n += 1;
        n.is_multiple_of(10_000) && screen(s).iter().any(|l| l.contains(text))
    }
}

/// How fast a DOS boot runs (AMI's BIOS on the 25 MHz 386 AT with the VGA): POST up to the
/// boot sector, MS-DOS 3.30 loading from drive A up to its date prompt,
/// and two seconds idle at the prompt (DOS polling the keyboard). Run with
/// `cargo test --release -- --ignored --nocapture bench_dos_boot`.
#[test]
#[ignore]
fn bench_dos_boot() {
    let (Some(mut sys), Some(d)) = (at(0), read("msdos330/DISK01.IMG")) else { return skip() };
    insert(&mut sys, &d);
    timed(&mut sys, "POST", 400_000_000, |s| s.cpu.r.CS.base + s.cpu.r.EIP == 0x7C00);
    timed(&mut sys, "DOS loading", 400_000_000, shows("Enter new date"));
    let t = sys.seconds() + 2.0;
    timed(&mut sys, "idle", 400_000_000, |s| s.seconds() >= t);
}

/// For comparing with other emulators: boots the floppy image in
/// `BENCH_DISK` (MS-DOS with an AUTOEXEC.BAT running a benchmark program),
/// and reports the instructions and wall time between the program's
/// markers at 0000:04F0 (0BEE0h at its start, 0BEEFh at its end), and from
/// reset to the end. Run with `BENCH_DISK=... cargo test --release --
/// --ignored --nocapture bench_compare`.
#[test]
#[ignore]
fn bench_compare() {
    let Some(path) = std::env::var_os("BENCH_DISK") else { return eprintln!("BENCH_DISK not set; skipping") };
    let (Some(mut sys), Ok(d)) = (at(0), std::fs::read(path)) else { return skip() };
    insert(&mut sys, &d);
    let mark = |s: &mut System| u16::from_le_bytes([s.board().ram.cells[0x4F0], s.board().ram.cells[0x4F1]]);
    let wall = std::time::Instant::now();
    let n0 = sys.run_until(4_000_000_000, |s| mark(s) == 0xBEE0).unwrap();
    let w0 = wall.elapsed().as_secs_f64();
    let n1 = sys.run_until(40_000_000_000, |s| mark(s) == 0xBEEF).unwrap();
    let w1 = wall.elapsed().as_secs_f64();
    assert_eq!(mark(&mut sys), 0xBEEF, "the program finished");
    println!(
        "BENCH {} {} boot_instructions {n0} boot_seconds {w0:.3} run_instructions {n1} run_seconds {:.3} mips {:.1} emulated_seconds {:.3}",
        machine_at386_vga::system::PROFILE,
        machine_at386_vga::system::CORE,
        w1 - w0,
        n1 as f64 / (w1 - w0) / 1e6,
        sys.seconds()
    );
}
