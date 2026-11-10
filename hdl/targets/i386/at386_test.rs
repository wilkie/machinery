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

/// The system BIOS: the 64 KB image at `PC_BIOS` (a path: the Bochs
/// project's free "legacy" BIOS, say), else AMI's.
fn system_bios() -> Option<Vec<u8>> {
    match std::env::var_os("PC_BIOS") {
        Some(p) => std::fs::read(p).ok().filter(|b| b.len() == 0x10000),
        None => ami_bios(),
    }
}

/// The VGA's BIOS: the image at `PC_VGABIOS` (a path: the free LGPL
/// VGABIOS, say; up to 64 KB), else IBM's (its 24 KB).
fn video_bios() -> Option<Vec<u8>> {
    match std::env::var_os("PC_VGABIOS") {
        Some(p) => std::fs::read(p).ok().filter(|b| b.len() <= 0x10000),
        None => Some(read("ibm_vga.bin")?.get(0x2000..0x8000)?.to_vec()),
    }
}

/// A 386 AT with AMI's BIOS (or `PC_BIOS`), IBM's VGA BIOS (or
/// `PC_VGABIOS`) and a configured CMOS, after reset; None without the ROM
/// images.
fn at(disk_type: u8) -> Option<System> {
    Some(at_with(system_bios()?, &video_bios()?, disk_type, std::env::var_os("PC_BIOS").is_some()))
}

/// The 386 AT with these ROMs, after reset. `bochs`: the system BIOS is
/// the Bochs project's, which boots the devices CMOS 3Dh lists (the
/// diskette, then the fixed disk) and takes the extended memory from
/// 30h-31h, which AMI's POST measures and writes itself (AMI's runs keep
/// their CMOS).
fn at_with(bios: Vec<u8>, vga: &[u8], disk_type: u8, bochs: bool) -> System {
    let mut sys = System::new();
    sys.board().rom.cells = bios;
    sys.board().vga.bios[..vga.len()].copy_from_slice(vga);
    let mut c = cmos(disk_type);
    if bochs {
        c[0x3D] = 0x21;
        c[0x30] = c[0x17];
        c[0x31] = c[0x18];
    }
    sys.board().rtc.cmos.copy_from_slice(&c);
    if disk_type == 2 {
        let h = &mut sys.board().hdc;
        h.present = true;
        h.cylinders = 615;
        h.heads = 4;
        h.sectors = 17;
    }
    sys.reset();
    sys
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

/// The free ROMs (`bench/i386/build_bios.sh`: the Bochs project's legacy
/// BIOS, patched for an AT, and the LGPL VGABIOS, in `free/`) on the 386
/// with the 20 MB fixed disk: the BIOS finds the drive by IDE's
/// identification, which the WD1003 answers, and FreeDOS's boot floppy
/// (`fetch_games.sh`) boots from drive A. Nothing that is not free.
#[test]
fn free_roms_boot_freedos() {
    let Some(r) = roms() else { return skip() };
    let (Ok(bios), Ok(vga), Some(d)) = (
        std::fs::read(r.join("free").join("BIOS-bochs-legacy-at")),
        std::fs::read(r.join("free").join("VGABIOS-lgpl-latest")),
        read("freedos/120m/x86BOOT.img"),
    ) else {
        return skip();
    };
    let mut sys = at_with(bios, &vga, 2, true);
    insert(&mut sys, &d);
    until_screen(&mut sys, "ata0 master: MACHINERY AT FIXED DISK", 100_000_000);
    until_screen(&mut sys, "FreeDOS kernel", 400_000_000);
    assert_eq!(sys.board().ram.cells[0x449], 3, "VGA text mode 3");
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
/// ROM images needed: the program is the boot ROM. Every profile: direct
/// memory applies the gate through the processor's `on mem.address`.
#[test]
fn a20_gate_wraps_memory_at_1_mb() {
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

/// The A20 gate closed and opened 1200 times over (`a20loop.asm`): each
/// time, the same routine reads and increments FFFF:0010, low memory's
/// first byte with the gate closed, the first above 1 MB with it open,
/// and the value read is added to BL (closed) or BH (open), stored at
/// 0:0500. The JIT core compiles the routine for each state of the gate
/// (the board's address key) and keeps both.
const A20_LOOP: &[u8] = &[
    0xfa, 0x31, 0xc0, 0x8e, 0xd8, 0x8e, 0xd0, 0xbc, 0x00, 0x70, 0xb8, 0xff,
    0xff, 0x8e, 0xc0, 0xb9, 0xb0, 0x04, 0x31, 0xdb, 0xb0, 0xd1, 0xe6, 0x64,
    0xb0, 0xdd, 0xe6, 0x60, 0xe8, 0x16, 0x00, 0x00, 0xc3, 0xb0, 0xd1, 0xe6,
    0x64, 0xb0, 0xdf, 0xe6, 0x60, 0xe8, 0x09, 0x00, 0x00, 0xc7, 0xe2, 0xe4,
    0x89, 0x1e, 0x00, 0x05, 0xf4, 0x26, 0xa0, 0x10, 0x00, 0x26, 0xfe, 0x06,
    0x10, 0x00, 0xc3,
];

#[test]
fn a20_gate_toggled_in_a_loop() {
    let mut sys = System::new();
    let mut rom = vec![0xF4u8; 0x10000];
    rom[..A20_LOOP.len()].copy_from_slice(A20_LOOP);
    rom[0xFFF0..0xFFF5].copy_from_slice(&[0xEA, 0x00, 0x00, 0x00, 0xF0]); // jmp far F000:0000
    sys.board().rom.cells = rom;
    sys.reset();
    sys.run(200_000).unwrap();
    assert!(sys.cpu.halted, "the loop ran to its end");
    let n = 1200u32;
    let sum = ((0..n).sum::<u32>() & 0xFF) as u8;
    let r = &sys.board().ram.cells;
    assert_eq!((r[0x500], r[0x501]), (sum, sum), "the values read, closed and open");
    assert_eq!((r[0x000], r[0x100000]), (n as u8, n as u8), "each byte incremented in its own state");
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
    // The time and the memory exactly: a profile's cores agree to the tick
    // where their concessions allow (`repeat: bulk` kernels change neither).
    let ram = sys.board().ram.cells.iter().fold(0xcbf29ce484222325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100000001b3));
    // The program's own result where it leaves it (mix: EDI; the memory's
    // hash also holds the BIOS's tick count, which moves with interrupt
    // timing).
    let edi = sys.cpu.r.EDI;
    // The Sound Blaster's output (`snd`): the samples played, those that
    // missed their byte, and a hash of the last 64 K.
    let sb = &sys.board().sb;
    let (played, underruns) = (sb.played, sb.underruns);
    let pcm = sb.pcm.iter().fold(0xcbf29ce484222325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100000001b3));
    println!(
        "BENCH {} {} boot_instructions {n0} boot_seconds {w0:.3} run_instructions {n1} run_seconds {:.3} mips {:.1} emulated_seconds {:.3} emulated_now {} ram {ram:016x} edi {edi:08x} sb_played {played} sb_underruns {underruns} pcm {pcm:016x}",
        machine_at386_vga::system::PROFILE,
        machine_at386_vga::system::CORE,
        w1 - w0,
        n1 as f64 / (w1 - w0) / 1e6,
        sys.seconds(),
        sys.now()
    );
}

/// Runs the floppy image in `PLAY_DISK` (DOS with an AUTOEXEC.BAT starting
/// a program, a game say) for `PLAY_SECONDS` of emulated time (30), pressing
/// keys at given times (`PLAY_KEYS`: `seconds:code` pairs, comma-separated,
/// a set-1 make code in hex), and saving the screen every `PLAY_EVERY`
/// seconds (5) as PPM files in `PLAY_SHOTS` (when set; the VGA draws one
/// frame in `PLAY_DRAW`, 1, 0 for none; a fixed disk's image as drive C
/// with `PLAY_HD`; with `PLAY_STEPS`, the FM chip's output in the WAV file
/// `PLAY_WAV`, the DSP's in `PLAY_SBWAV`). Reports the
/// instructions and the time taken. Run with `PLAY_DISK=... cargo test
/// --release -- --ignored --nocapture play_disk`.
#[test]
#[ignore]
fn play_disk() {
    let Some(path) = std::env::var_os("PLAY_DISK") else { return eprintln!("PLAY_DISK not set; skipping") };
    // `PLAY_HD`: a fixed disk image as drive C (type 2: 615 cylinders, 4
    // heads, 17 sectors).
    let hd = std::env::var_os("PLAY_HD").map(|p| std::fs::read(p).expect("PLAY_HD"));
    let (Some(mut sys), Ok(d)) = (at(if hd.is_some() { 2 } else { 0 }), std::fs::read(path)) else { return skip() };
    insert(&mut sys, &d);
    if let Some(h) = &hd {
        sys.board().hdc.image[..h.len()].copy_from_slice(h);
    }
    let var = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    // `PLAY_DRAW`: the VGA draws one frame in so many (0: none); the machine
    // runs the same whatever it is.
    sys.board().vga.draw_every = var("PLAY_DRAW", 1.0) as u32;
    let (seconds, every) = (var("PLAY_SECONDS", 30.0), var("PLAY_EVERY", 5.0));
    let mut keys: Vec<(f64, u8)> = std::env::var("PLAY_KEYS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|p| {
            let (t, c) = p.split_once(':')?;
            Some((t.trim().parse().ok()?, u8::from_str_radix(c.trim(), 16).ok()?))
        })
        .collect();
    keys.sort_by(|a, b| a.0.total_cmp(&b.0));
    let shots = std::env::var_os("PLAY_SHOTS").map(std::path::PathBuf::from);
    let save = |sys: &mut System, t: f64| {
        let Some(dir) = &shots else { return };
        let v = &sys.board().vga;
        let (w, h) = (v.frame_width as usize, v.frame_height as usize);
        let mut out = format!("P6 {w} {h} 255\n").into_bytes();
        for y in 0..h {
            for x in 0..w {
                let at = (y * 720 + x) * 3;
                out.extend(v.frame[at..at + 3].iter().map(|c| (c << 2) | (c >> 4)));
            }
        }
        let _ = std::fs::write(dir.join(format!("shot_{:06.1}.ppm", t)), out);
    };
    let wall = std::time::Instant::now();
    let mut steps = 0u64;
    let mut next_shot = every;
    let mut k = 0;
    // `PLAY_STEPS`: the run, the keys' times and their holds counted in
    // instructions, which every core counts alike (a time is reached at a
    // different instruction by each: a key would land elsewhere).
    // `PLAY_WAV` (with `PLAY_STEPS`): the FM chip's output, gathered as the
    // run goes (its ring holds 1.3 s), written as a WAV file.
    let wav_path = std::env::var_os("PLAY_WAV");
    let mut wav: Vec<i16> = Vec::new();
    let mut heard = 0u32;
    // `PLAY_SBWAV` (with `PLAY_STEPS`): the Sound Blaster DSP's output
    // likewise (8-bit, at the rate its time constant gives at the end).
    let sbwav_path = std::env::var_os("PLAY_SBWAV");
    let mut sbwav: Vec<i16> = Vec::new();
    let mut sb_heard = 0u32;
    if let Some(total) = std::env::var("PLAY_STEPS").ok().and_then(|v| v.parse::<u64>().ok()) {
        let mut run = |sys: &mut System, n: u64| -> u64 {
            let mut made = 0;
            while made < n {
                let chunk = (n - made).min(if wav_path.is_some() || sbwav_path.is_some() { 1_000_000 } else { u64::MAX });
                made += sys.run_until(chunk, |_| false).unwrap();
                if sbwav_path.is_some() {
                    sys.with_sb(|d, _| {
                        assert!(d.played - sb_heard <= 0x10000, "the DSP's ring overran");
                        sbwav.extend((sb_heard..d.played).map(|i| ((d.pcm[(i & 0xFFFF) as usize] as i16) - 128) << 8));
                        sb_heard = d.played;
                    });
                }
                if wav_path.is_some() {
                    sys.with_opl(|o, _| {
                        assert!(o.played - heard <= 0x10000, "the FM chip's ring overran");
                        wav.extend((heard..o.played).map(|i| o.pcm[(i & 0xFFFF) as usize] as i16));
                        heard = o.played;
                    });
                }
            }
            made
        };
        for &(at, c) in &keys {
            let at = at as u64;
            if at > steps {
                steps += run(&mut sys, at - steps);
            }
            sys.with_kbc(|kb, ctx| kb.fn_key(ctx, c));
            steps += run(&mut sys, 300_000);
            sys.with_kbc(|kb, ctx| kb.fn_key(ctx, c | 0x80));
        }
        if total > steps {
            steps += run(&mut sys, total - steps);
        }
        next_shot = f64::MAX;
    }
    let sb_rate = 1_000_000 / (256 - sys.board().sb.time_const as u32);
    for (path, wav, rate) in [(&wav_path, &wav, 49716u32), (&sbwav_path, &sbwav, sb_rate)] {
        let Some(path) = path else { continue };
        let data: Vec<u8> = wav.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&data);
        std::fs::write(path, out).unwrap();
        println!("WAV {} samples at {rate} Hz, peak {}", wav.len(), wav.iter().map(|v| (*v as i32).abs()).max().unwrap_or(0));
    }
    while sys.seconds() < seconds && next_shot != f64::MAX {
        let until = next_shot.min(keys.get(k).map_or(f64::MAX, |p| p.0)).min(seconds);
        steps += sys.run_until(4_000_000_000, |s| s.seconds() >= until).unwrap();
        while k < keys.len() && keys[k].0 <= sys.seconds() {
            // Held a while: a game may look at which keys are down once a
            // frame, not at each press.
            let c = keys[k].1;
            println!("KEY {:.3} {c:02x} at instruction {steps}", sys.seconds());
            sys.with_kbc(|kb, ctx| kb.fn_key(ctx, c));
            wait(&mut sys, 0.15);
            sys.with_kbc(|kb, ctx| kb.fn_key(ctx, c | 0x80));
            k += 1;
        }
        if sys.seconds() >= next_shot {
            save(&mut sys, next_shot);
            next_shot += every;
        }
    }
    let w = wall.elapsed().as_secs_f64();
    let t = sys.seconds();
    save(&mut sys, t);
    let played = sys.board().sb.played;
    let opl_writes = sys.board().opl.writes;
    // What the program shows and holds at the end, to compare profiles by.
    let fnv = |h: u64, b: &u8| (h ^ *b as u64).wrapping_mul(0x100000001b3);
    let frame = sys.board().vga.frame.iter().fold(0xcbf29ce484222325u64, fnv);
    let ram = sys.board().ram.cells.iter().fold(0xcbf29ce484222325u64, fnv);
    println!(
        "PLAY {} {} instructions {steps} seconds {w:.3} mips {:.1} emulated_seconds {:.3} speed {:.2}x sb_played {} opl_writes {opl_writes} frame {frame:016x} ram {ram:016x} now {}",
        machine_at386_vga::system::PROFILE,
        machine_at386_vga::system::CORE,
        steps as f64 / w / 1e6,
        t,
        t / w,
        played,
        sys.now()
    );
}

