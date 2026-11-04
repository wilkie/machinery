//! Drives the crate generated from `board.machine`'s `Board386Vga`
//! (`machine build --system Board386Vga`): the 386 board with a VGA, set up
//! by IBM's own VGA BIOS where its image is available (`~/roms`, or
//! `PC_ROMS`; it is not in the repository), and driven directly otherwise.

use machine_board386_vga::system::System;

/// A 64 KB ROM image (filled with HLT): `code` at F000:0000, the reset
/// vector's far jump to F000:0000, and an IRET at F000:F065, where the PC
/// BIOS's video routine is.
fn rom(code: &[u8]) -> Vec<u8> {
    let mut r = vec![0xF4u8; 0x10000];
    r[..code.len()].copy_from_slice(code);
    r[0xF065] = 0xCF;
    r[0xFFF0..0xFFF5].copy_from_slice(&[0xEA, 0x00, 0x00, 0x00, 0xF0]); // jmp far F000:0000
    r
}

fn boot(image: Vec<u8>) -> System {
    let mut sys = System::new();
    sys.board().rom.cells = image;
    sys.reset();
    sys
}

/// A font extracted from the cards' ROMs (`targets/pc/fonts`).
fn font(name: &str) -> Vec<u8> {
    let here = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = here.as_path();
    loop {
        let f = dir.join("targets/pc/fonts").join(name);
        if f.exists() {
            return std::fs::read(f).unwrap();
        }
        dir = dir.parent().expect("targets/pc/fonts not found");
    }
}

/// The IBM VGA's BIOS (a 24 KB option ROM at 2000 in the 32 KB image).
fn vga_bios() -> Option<Vec<u8>> {
    let dir = match std::env::var("PC_ROMS") {
        Ok(d) => std::path::PathBuf::from(d),
        Err(_) => std::path::PathBuf::from(std::env::var("HOME").ok()?).join("roms"),
    };
    let image = std::fs::read(dir.join("ibm_vga.bin")).ok()?;
    let rom = image.get(0x2000..)?.to_vec();
    (rom.len() == 0x6000 && rom[..2] == [0x55, 0xAA]).then_some(rom)
}

/// The stand-in system BIOS's start, at F000:0000: every interrupt vector
/// at F000:F065 (an IRET), the data area's equipment word with no video and
/// 640 KB, and a far call to the option ROM's initialisation at C000:0003.
const HOST_PROLOGUE: &[u8] = &[
    0xFA, // cli
    0x31, 0xC0, // xor ax, ax
    0x8E, 0xD8, // mov ds, ax
    0x8E, 0xC0, // mov es, ax
    0x8E, 0xD0, // mov ss, ax
    0xBC, 0x00, 0x7C, // mov sp, 0x7C00
    0x31, 0xFF, // xor di, di
    0xB9, 0x00, 0x01, // mov cx, 256
    0xC7, 0x05, 0x65, 0xF0, // .vec: mov word [di], 0xF065
    0xC7, 0x45, 0x02, 0x00, 0xF0, // mov word [di + 2], 0xF000
    0x83, 0xC7, 0x04, // add di, 4
    0xE2, 0xF2, // loop .vec
    0xC7, 0x06, 0x10, 0x04, 0x00, 0x00, // mov word [0x410], 0x0000
    0xC7, 0x06, 0x13, 0x04, 0x80, 0x02, // mov word [0x413], 640
    0x9A, 0x03, 0x00, 0x00, 0xC0, // call 0xC000:0x0003
];

/// INT 10h with AX, BX, CX and DX.
fn int10(ax: u16, bx: u16, cx: u16, dx: u16) -> Vec<u8> {
    let mut v = Vec::new();
    for (op, x) in [(0xB8, ax), (0xBB, bx), (0xB9, cx), (0xBA, dx)] {
        v.extend_from_slice(&[op, x as u8, (x >> 8) as u8]);
    }
    v.extend_from_slice(&[0xCD, 0x10]);
    v
}

/// `mov [addr], al`
fn store_al(addr: u16) -> Vec<u8> {
    vec![0xA2, addr as u8, (addr >> 8) as u8]
}

/// Runs the program to its HLT, then until the VGA has finished `n` more
/// frames.
fn frames(sys: &mut System, n: u32) {
    sys.run_until(50_000_000, |s| s.cpu.halted).unwrap();
    assert!(sys.cpu.halted, "the program ran to its HLT");
    sys.catch_up_all();
    let want = sys.board().vga.frames + n;
    sys.run_until(50_000_000, |s| {
        s.catch_up_all();
        s.board().vga.frames >= want
    })
    .unwrap();
}

/// Boots the stand-in BIOS with `calls` after the option ROM's
/// initialisation; None without the VGA BIOS image.
fn run_vga_bios(calls: &[u8]) -> Option<System> {
    let bios = vga_bios()?;
    let mut code = HOST_PROLOGUE.to_vec();
    code.extend_from_slice(calls);
    code.push(0xF4); // hlt
    let mut sys = boot(rom(&code));
    sys.board().vga.bios[..bios.len()].copy_from_slice(&bios);
    frames(&mut sys, 2);
    Some(sys)
}

fn skip() {
    eprintln!("the VGA BIOS image is not in ~/roms; skipping");
}

/// The DAC's colour for dot `x` of line `y`: red, green, blue.
fn rgb(sys: &mut System, x: usize, y: usize) -> [u8; 3] {
    let f = &sys.board().vga.frame;
    let at = (y * 720 + x) * 3;
    [f[at], f[at + 1], f[at + 2]]
}

/// IBM's VGA BIOS on our VGA: its initialisation senses a colour monitor
/// through the DAC's comparator, and INT 10h sets mode 3 (720x400 on the
/// 28 MHz clock), loads its 8x16 font into plane 2, and writes text by
/// teletype.
#[test]
fn vga_bios_sets_text_mode() {
    let mut calls = int10(0x0003, 0, 0, 0);
    calls.extend(int10(0x0E48, 0x0007, 0, 0)); // 'H'
    calls.extend(int10(0x0E69, 0x0007, 0, 0)); // 'i'
    let Some(mut sys) = run_vga_bios(&calls) else { return skip() };
    let b = sys.board();
    let bda = |a: usize| b.ram.cells[a];
    assert_eq!(bda(0x410) & 0x30, 0x20, "80-column colour: a colour monitor sensed");
    assert_eq!((bda(0x449), bda(0x44A), bda(0x485)), (3, 80, 16), "mode 3: 80 columns, 16 rows a character");
    assert_eq!((bda(0x463), bda(0x464)), (0xD4, 0x03), "the CRTC at 3D4");
    assert_eq!(b.vga.misc & 0x0D, 0x05, "the 28 MHz clock, colour addressing");
    assert_eq!((b.vga.frame_width, b.vga.frame_height), (720, 400));
    let font16 = font("vga_8x16.bin");
    for c in [b'H', b'i'] {
        let at = 0x20000 + c as usize * 32;
        assert_eq!(&b.vga.planes[at..at + 16], &font16[c as usize * 16..c as usize * 16 + 16]);
    }
    assert_eq!((b.vga.planes[0], b.vga.planes[2]), (b'H', b'i'));
    // Light grey on black, nine dots a character (the ninth blank for a
    // letter).
    let grey = [0x2A, 0x2A, 0x2A];
    for (col, c) in [b'H', b'i'].iter().enumerate() {
        for r in 0..16 {
            let row = font16[*c as usize * 16 + r];
            for d in 0..9 {
                let on = d < 8 && row >> (7 - d) & 1 == 1;
                let want = if on { grey } else { [0, 0, 0] };
                assert_eq!(rgb(&mut sys, col * 9 + d, r), want, "{} row {r} dot {d}", *c as char);
            }
        }
    }
}

/// Mode 13h (320x200 in 256 colours, chain-4) by the BIOS: a DAC entry set
/// through INT 10h, a dot written by INT 10h and one by the processor
/// straight into A000, each 2x2 in the 640x400 frame (doubled dots, double
/// scan), and read-pixel back.
#[test]
fn vga_bios_mode_13h() {
    let mut calls = int10(0x0013, 0, 0, 0);
    calls.extend(int10(0x1010, 0x0080, 0x0020, 0x3F00)); // DAC 80: red 3F, green 00, blue 20
    calls.extend(int10(0x0C80, 0, 5, 3)); // colour 80 at (5, 3)
    calls.extend_from_slice(&[0xB8, 0x00, 0xA0, 0x8E, 0xC0]); // mov ax, 0xA000; mov es, ax
    calls.extend_from_slice(&[0x26, 0xC6, 0x06, 0x3F, 0x01, 0x80]); // mov byte es:[319], 0x80: (319, 0)
    calls.extend(int10(0x0D00, 0, 5, 3));
    calls.extend(store_al(0x0500));
    let Some(mut sys) = run_vga_bios(&calls) else { return skip() };
    let b = sys.board();
    assert_eq!(b.ram.cells[0x449], 0x13);
    assert_eq!(b.ram.cells[0x500], 0x80, "read back");
    assert_eq!((b.vga.frame_width, b.vga.frame_height), (640, 400));
    // Chain-4: byte 319 of the window is plane 3's byte at 316.
    assert_eq!(b.vga.planes[0x30000 + 316], 0x80);
    let magenta = [0x3F, 0x00, 0x20];
    for (x, y) in [(5, 3), (319, 0)] {
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            assert_eq!(rgb(&mut sys, 2 * x + dx, 2 * y + dy), magenta, "({x}, {y})");
        }
    }
    assert_ne!(rgb(&mut sys, 2 * 4, 2 * 3), magenta);
}

/// Mode 12h (640x480 in 16 colours) by the BIOS: the far corner written and
/// read back, through the attribute palette and the DAC.
#[test]
fn vga_bios_mode_12h() {
    let mut calls = int10(0x0012, 0, 0, 0);
    calls.extend(int10(0x0C0C, 0, 639, 479)); // colour 12 at (639, 479)
    calls.extend(int10(0x0D00, 0, 639, 479));
    calls.extend(store_al(0x0500));
    let Some(mut sys) = run_vga_bios(&calls) else { return skip() };
    let b = sys.board();
    assert_eq!(b.ram.cells[0x449], 0x12);
    assert_eq!(b.ram.cells[0x500], 12, "read back");
    assert_eq!((b.vga.frame_width, b.vga.frame_height), (640, 480));
    let index = (b.vga.attr[12] & 0x3F) as usize;
    let want = [b.vga.dac[index * 3], b.vga.dac[index * 3 + 1], b.vga.dac[index * 3 + 2]];
    assert_ne!(want, [0, 0, 0]);
    assert_eq!(rgb(&mut sys, 639, 479), want);
    assert_eq!(rgb(&mut sys, 638, 479), [0, 0, 0]);
}

// --- driven directly ---

struct Quiet;
impl machine_board386_vga::VgaPorts for Quiet {}

fn out(v: &mut machine_board386_vga::Vga, port: u32, d: u8) {
    v.on_port_write(&mut Quiet, 1, port, 0, d as u16);
}

fn inp(v: &mut machine_board386_vga::Vga, port: u32) -> u8 {
    v.on_port_read(&mut Quiet, 1, port, 0).unwrap().0 as u8
}

/// The DAC: three writes an entry from the write index, auto-incrementing;
/// reads likewise from the read index; 3C7 tells which was set last.
#[test]
fn vga_dac_ports() {
    let mut v = machine_board386_vga::Vga::new();
    out(&mut v, 0x3C8, 0x10);
    for c in [1, 2, 3, 4, 5, 0x7F] {
        out(&mut v, 0x3C9, c);
    }
    assert_eq!(&v.dac[0x30..0x36], &[1, 2, 3, 4, 5, 0x3F], "six bits a component");
    assert_eq!(inp(&mut v, 0x3C8), 0x12);
    assert_eq!(inp(&mut v, 0x3C7), 0, "write mode");
    out(&mut v, 0x3C7, 0x11);
    assert_eq!(inp(&mut v, 0x3C7), 3, "read mode");
    let got: Vec<u8> = (0..3).map(|_| inp(&mut v, 0x3C9)).collect();
    assert_eq!(got, [4, 5, 0x3F]);
}

/// Registers read back; the CRTC's R0-R7 ignore writes while R11h bit 7 is
/// set (but for R7's line compare bit).
#[test]
fn vga_registers_read_back_and_crtc_protect() {
    let mut v = machine_board386_vga::Vga::new();
    out(&mut v, 0x3C2, 0x01); // colour addressing
    assert_eq!(inp(&mut v, 0x3CC), 0x01);
    out(&mut v, 0x3D4, 0x00);
    out(&mut v, 0x3D5, 0x5F);
    out(&mut v, 0x3D4, 0x11);
    out(&mut v, 0x3D5, 0x80);
    out(&mut v, 0x3D4, 0x00);
    out(&mut v, 0x3D5, 0x12);
    assert_eq!(inp(&mut v, 0x3D5), 0x5F, "protected");
    out(&mut v, 0x3D4, 0x07);
    out(&mut v, 0x3D5, 0xFF);
    assert_eq!(inp(&mut v, 0x3D5), 0x10, "only the line compare bit");
    out(&mut v, 0x3CE, 0x05);
    out(&mut v, 0x3CF, 0x40);
    assert_eq!(inp(&mut v, 0x3CF), 0x40);
    // 3Bx is another card's while colour addressing is selected.
    assert!(v.on_port_read(&mut Quiet, 1, 0x3B5, 0).is_none());
}

/// Chain-4 puts byte n in plane n mod 4; write mode 3 masks the Set/Reset
/// colour by the rotated byte and the Bit Mask.
#[test]
fn vga_chain4_and_write_mode_3() {
    let mut v = machine_board386_vga::Vga::new();
    out(&mut v, 0x3C2, 0x03); // RAM enabled
    out(&mut v, 0x3C4, 0x02);
    out(&mut v, 0x3C5, 0x0F); // map mask
    out(&mut v, 0x3C4, 0x04);
    out(&mut v, 0x3C5, 0x0E); // chain-4
    out(&mut v, 0x3CE, 0x06);
    out(&mut v, 0x3CF, 0x05); // A0000, 64 KB
    out(&mut v, 0x3CE, 0x08);
    out(&mut v, 0x3CF, 0xFF); // bit mask
    for (i, b) in [0x11u8, 0x22, 0x33, 0x44, 0x55].iter().enumerate() {
        v.on_port_write(&mut Quiet, 0, 0xA0000 + i as u32, 0, *b as u16);
    }
    assert_eq!((v.planes[0], v.planes[0x10000], v.planes[0x20000], v.planes[0x30000]), (0x11, 0x22, 0x33, 0x44));
    assert_eq!(v.planes[4], 0x55, "byte 4: plane 0 at 4");
    // Unchained, write mode 3: Set/Reset colour 5, rotated byte F0 ANDed
    // with the bit mask 3C.
    out(&mut v, 0x3C4, 0x04);
    out(&mut v, 0x3C5, 0x06);
    out(&mut v, 0x3CE, 0x00);
    out(&mut v, 0x3CF, 0x05);
    out(&mut v, 0x3CE, 0x05);
    out(&mut v, 0x3CF, 0x03);
    out(&mut v, 0x3CE, 0x08);
    out(&mut v, 0x3CF, 0x3C);
    let _ = v.on_port_read(&mut Quiet, 0, 0xA0100, 0); // latches: zero
    v.on_port_write(&mut Quiet, 0, 0xA0100, 0, 0xF0);
    let p = |n: u32| v.planes[(n * 0x10000 + 0x100) as usize];
    assert_eq!((p(0), p(1), p(2), p(3)), (0x30, 0x00, 0x30, 0x00));
}

/// `render` is a `memo fn` (SPEC 8.3b): a line whose reads are unchanged
/// keeps its dots. Two copies of the card, set up by the BIOS, take the
/// same random writes (memory at random points within frames, the DAC, the
/// attribute palette, the CRTC's start address); one forgets its memo
/// before every tick, so every line it draws is drawn afresh. After every
/// frame both show the same dots, in every mode tried.
#[test]
fn vga_memo_matches_drawing_every_line() {
    let modes: &[(u16, u32)] = &[(0x03, 20), (0x01, 3), (0x13, 3), (0x12, 3), (0x04, 3), (0x06, 3), (0x0D, 3), (0x10, 3)];
    for &(mode, frames) in modes {
        let Some(mut sys) = run_vga_bios(&int10(mode, 0, 0, 0)) else { return skip() };
        let mut a = sys.board().vga.clone();
        a.memo_clear();
        let mut b = a.clone();
        let mut x: u32 = 0x1234_5678 ^ mode as u32;
        let mut rnd = move || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x
        };
        let f0 = a.frames;
        while a.frames < f0 + frames {
            let r = rnd();
            match r % 16 {
                0 => {
                    let (i, v) = ((rnd() % 16) as u8, rnd() as u8 & 0x3F);
                    for v2 in [&mut a, &mut b] {
                        out(v2, 0x3C8, i);
                        for _ in 0..3 {
                            out(v2, 0x3C9, v);
                        }
                    }
                }
                1 => {
                    let (i, v) = ((rnd() % 16) as u8, rnd() as u8 & 0x3F);
                    for v2 in [&mut a, &mut b] {
                        let _ = inp(v2, 0x3DA);
                        out(v2, 0x3C0, i | 0x20);
                        out(v2, 0x3C0, v);
                    }
                }
                2 if rnd() % 8 == 0 => {
                    let v = (rnd() % 4) as u8;
                    for v2 in [&mut a, &mut b] {
                        out(v2, 0x3D4, 0x0C);
                        out(v2, 0x3D5, v);
                    }
                }
                _ => {
                    let addr = if r & 0x100 == 0 { 0xA0000 + rnd() % 0x10000 } else { 0xB8000 + rnd() % 0x8000 };
                    let d = rnd() as u16 & 0xFF;
                    for v2 in [&mut a, &mut b] {
                        v2.on_port_write(&mut Quiet, 0, addr, 0, d);
                    }
                }
            }
            let n = 1 + rnd() % 4000;
            let _ = a.on_tick_dot25(&mut Quiet, n);
            let _ = a.on_tick_dot28(&mut Quiet, n);
            b.memo_clear();
            let _ = b.on_tick_dot25(&mut Quiet, n);
            let _ = b.on_tick_dot28(&mut Quiet, n);
            if a.frames != b.frames {
                panic!("mode {mode:#x}: the copies' frames differ");
            }
            if a.pos < n {
                assert!(a.frame == b.frame && a.dots == b.dots, "mode {mode:#x}, frame {}: the memo changed the picture", a.frames);
            }
        }
        assert!(a.frame == b.frame && a.dots == b.dots, "mode {mode:#x}: the memo changed the picture");
    }
}

/// Frames a second the VGA model renders in modes the BIOS sets, driven
/// directly once the BIOS has programmed it.
#[test]
#[ignore]
fn bench_vga_modes() {
    for (name, mode) in [
        ("80x25 text (720x400)", 0x03u16),
        ("80x25 text, nothing blinking", 0x03),
        ("320x200x256 (mode 13h)", 0x13),
        ("640x480x16 (mode 12h)", 0x12),
    ] {
        let Some(mut sys) = run_vga_bios(&int10(mode, 0, 0, 0)) else { return skip() };
        let mut v = sys.board().vga.clone();
        let mut x: u32 = 12345;
        for b in v.planes.iter_mut() {
            x = x.wrapping_mul(1103515245).wrapping_add(12345);
            *b = (x >> 16) as u8;
        }
        if name.contains("blinking") {
            // Attributes (plane 1) without the blink bit.
            for b in &mut v.planes[0x10000..0x20000] {
                *b &= 0x7F;
            }
        }
        // Written directly, not through the card: its memo is forgotten.
        v.memo_clear();
        for (how, redraw) in [("every line drawn", true), ("static screen", false)] {
            let mut v = v.clone();
            let n = 300;
            let t = std::time::Instant::now();
            let f0 = v.frames;
            while v.frames < f0 + n {
                if redraw {
                    v.memo_clear();
                }
                let left = v.on_tick_dot25(&mut Quiet, 1_000_000);
                let _ = v.on_tick_dot28(&mut Quiet, 1_000_000.min(left));
            }
            let s = t.elapsed().as_secs_f64();
            eprintln!("VGA {name}, {how}: {:.0} frames/s ({:.1} us a frame)", n as f64 / s, s * 1e6 / n as f64);
        }
    }
}

// --- the whole system ---

/// RAM only (`compute.asm`): a checksum over 32 KB, 40 passes, with 32-bit
/// arithmetic, a call a word and a string move a pass.
const COMPUTE: &[u8] = &[
    0x31, 0xc0, 0x8e, 0xd8, 0x8e, 0xc0, 0xbd, 0x28, 0x00, 0xbe, 0x00, 0x80, 0xb9, 0x00, 0x40, 0x66,
    0x31, 0xd2, 0xad, 0x66, 0x0f, 0xb7, 0xc0, 0x66, 0x01, 0xc2, 0x66, 0xc1, 0xc2, 0x03, 0xe8, 0x16,
    0x00, 0xe2, 0xef, 0x66, 0x89, 0x16, 0x00, 0x06, 0xbe, 0x00, 0x80, 0xbf, 0x00, 0x90, 0xb9, 0x00,
    0x08, 0xf3, 0xa5, 0x4d, 0x75, 0xd3, 0xf4, 0x81, 0xf2, 0x5a, 0x5a, 0xc3,
];

/// Mode 13h frames (`draw.asm`), 20 of them: the screen filled a pixel at a
/// time over the ISA bus, a little 32-bit arithmetic in RAM, then a wait on
/// Input Status 1 for the start of vertical retrace, as a game loop does.
const DRAW: &[u8] = &[
    0xb8, 0x00, 0xa0, 0x8e, 0xc0, 0xbd, 0x14, 0x00, 0x31, 0xdb, 0x31, 0xff, 0xba, 0xc8, 0x00, 0xb9,
    0x40, 0x01, 0x88, 0xd0, 0x00, 0xd8, 0xaa, 0xfe, 0xc0, 0xe2, 0xfb, 0x4a, 0x75, 0xf1, 0xb9, 0xd0,
    0x07, 0xbe, 0x00, 0x10, 0x66, 0x8b, 0x04, 0x66, 0x69, 0xc0, 0xfd, 0x43, 0x03, 0x00, 0x66, 0x05,
    0xc3, 0x9e, 0x26, 0x00, 0x66, 0x89, 0x04, 0xe2, 0xeb, 0xba, 0xda, 0x03, 0xec, 0xa8, 0x08, 0x75,
    0xfb, 0xec, 0xa8, 0x08, 0x74, 0xfb, 0xfe, 0xc3, 0x4d, 0x75, 0xbf, 0xf4,
];

/// The checksum of `COMPUTE` in 32-bit protected mode with paging on (the
/// first 4 MB identity-mapped; `paged.asm`), loaded at 0000:0100: every
/// access is translated through the TLB.
const PAGED: &[u8] = &[
    0xfa, 0x31, 0xc0, 0x8e, 0xd8, 0x0f, 0x01, 0x16, 0xc0, 0x01, 0x0f, 0x20, 0xc0, 0x0c, 0x01, 0x0f,
    0x22, 0xc0, 0x66, 0xea, 0x1a, 0x01, 0x00, 0x00, 0x08, 0x00, 0x66, 0xb8, 0x10, 0x00, 0x8e, 0xd8,
    0x8e, 0xc0, 0x8e, 0xd0, 0xbc, 0x00, 0x70, 0x00, 0x00, 0xbf, 0x00, 0x10, 0x02, 0x00, 0xb8, 0x07,
    0x00, 0x00, 0x00, 0xb9, 0x00, 0x04, 0x00, 0x00, 0x89, 0x07, 0x05, 0x00, 0x10, 0x00, 0x00, 0x83,
    0xc7, 0x04, 0xe2, 0xf4, 0xc7, 0x05, 0x00, 0x00, 0x02, 0x00, 0x07, 0x10, 0x02, 0x00, 0xb8, 0x00,
    0x00, 0x02, 0x00, 0x0f, 0x22, 0xd8, 0x0f, 0x20, 0xc0, 0x0d, 0x00, 0x00, 0x00, 0x80, 0x0f, 0x22,
    0xc0, 0xeb, 0x00, 0xbd, 0x28, 0x00, 0x00, 0x00, 0xbe, 0x00, 0x80, 0x00, 0x00, 0xb9, 0x00, 0x40,
    0x00, 0x00, 0x31, 0xd2, 0x66, 0xad, 0x0f, 0xb7, 0xc0, 0x01, 0xc2, 0xc1, 0xc2, 0x03, 0xe8, 0x1e,
    0x00, 0x00, 0x00, 0xe2, 0xef, 0x89, 0x15, 0x00, 0x06, 0x00, 0x00, 0xbe, 0x00, 0x80, 0x00, 0x00,
    0xbf, 0x00, 0x90, 0x00, 0x00, 0xb9, 0x00, 0x08, 0x00, 0x00, 0xf3, 0x66, 0xa5, 0x4d, 0x75, 0xc8,
    0xf4, 0x66, 0x81, 0xf2, 0x5a, 0x5a, 0xc3, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xff, 0xff, 0x00, 0x00, 0x00, 0x9a, 0xcf, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x92, 0xcf, 0x00,
    0x17, 0x00, 0xa8, 0x01, 0x00, 0x00,
];

/// Runs to the HLT and reports the rate: instructions, emulated time
/// against wall time.
fn measure(name: &str, sys: &mut System) {
    let (t0, f0) = (sys.seconds(), sys.board().vga.frames);
    let wall = std::time::Instant::now();
    let n = sys.run(2_000_000_000).unwrap();
    let w = wall.elapsed().as_secs_f64();
    assert!(sys.cpu.halted);
    sys.catch_up_all();
    let (t, f) = (sys.seconds() - t0, sys.board().vga.frames - f0);
    eprintln!(
        "{name}: {n} instructions, {:.1} M/s; {:.3} s emulated in {:.3} s ({:.1}x real time); {f} frames shown",
        n as f64 / w / 1e6,
        t,
        w,
        t / w
    );
}

/// The whole system's speed: the 386 interpreter with every device caught
/// up as it is today (per instruction and before each transaction).
#[test]
#[ignore]
fn bench_system() {
    let only = std::env::var("BENCH_ONLY").unwrap_or_default();
    if only != "draw" {
        let mut sys = boot(rom(COMPUTE));
        measure("compute (RAM only)", &mut sys);
    }
    if only.is_empty() || only == "paged" {
        // The ROM jumps to 0000:0100, where the program is.
        let mut sys = boot(rom(&[0xEA, 0x00, 0x01, 0x00, 0x00]));
        sys.board().ram.cells[0x100..0x100 + PAGED.len()].copy_from_slice(PAGED);
        measure("paged (32-bit, paging on)", &mut sys);
        assert_eq!(sys.cpu.r.CR0 >> 31, 1, "paging was on");
    }
    if only == "compute" || only == "paged" {
        return;
    }
    let Some(bios) = vga_bios() else { return skip() };
    let mut code = HOST_PROLOGUE.to_vec();
    code.extend(int10(0x0013, 0, 0, 0));
    let start = code.len() as u32;
    code.extend_from_slice(DRAW);
    let mut sys = boot(rom(&code));
    sys.board().vga.bios[..bios.len()].copy_from_slice(&bios);
    sys.run_until(50_000_000, |s| s.cpu.r.EIP == start).unwrap();
    assert_eq!(sys.cpu.r.EIP, start, "the BIOS set mode 13h");
    measure("mode 13h frames (ISA writes, retrace polling)", &mut sys);
}

/// xorshift64*: the random programs' source.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// Instructions that leave the straight line (or stop): not in a block.
const BRANCHES: &[&str] = &[
    "Jcc", "Jcc_near<16>", "Jcc_near<32>", "Jmp_rel8", "Jmp_rel<16>", "Jmp_rel<32>", "Call_rel<16>", "Call_rel<32>", "Ret<16>", "Ret<32>",
    "Ret_imm<16>", "Ret_imm<32>", "Call_rm<16>", "Call_rm<32>", "Jmp_rm<16>", "Jmp_rm<32>", "Loopne", "Loope", "Loop", "Jcxz", "Call_far<16>",
    "Call_far<32>", "Jmp_far<16>", "Jmp_far<32>", "Call_far_mem<16>", "Call_far_mem<32>", "Jmp_far_mem<16>", "Jmp_far_mem<32>", "Retf<16>",
    "Retf<32>", "Retf_imm<16>", "Retf_imm<32>", "Iret<16>", "Iret<32>", "Int3", "Int_imm8", "Into", "Hlt",
];

/// The ROM's jump to 1000:0000, where the random programs run.
const TO_RAM: &[u8] = &[0xEA, 0x00, 0x00, 0x00, 0x10];

/// `n` random instructions at 1000:0000 for machine `m` (at that address),
/// each tried on a copy of it in the state the ones before leave: kept when
/// it decodes, is not a branch or HLT, and runs without a fault. Then a
/// jump back to their start. The program is written into `m`.
fn random_program(rng: &mut Rng, m: &mut System, n: usize) -> Vec<u8> {
    let mut code = Vec::new();
    let mut probe = m.clone();
    let mut tries = 0;
    while code.len() < 4 * n && tries < 20 * n {
        tries += 1;
        let bytes: Vec<u8> = (0..12).map(|_| rng.next() as u8).collect();
        let mut t = probe.clone();
        let at = 0x10000 + code.len();
        t.board().ram.cells[at..at + 12].copy_from_slice(&bytes);
        t.code_changed();
        if t.cpu.step().is_err() || t.cpu.last_fault.is_some() || t.cpu.halted {
            continue;
        }
        let Some(i) = t.cpu.last else { continue };
        let name = machine_board386_vga::I386::<machine_board386_vga::system::Board>::instr_name(&i);
        if BRANCHES.contains(&name) || t.cpu.r.EIP as usize != code.len() + t.cpu.fetched as usize {
            continue;
        }
        code.extend_from_slice(&bytes[..t.cpu.fetched as usize]);
        probe = t;
    }
    // jmp near back to 0 (16-bit code): E9 rel16.
    let back = -(code.len() as i32 + 3);
    code.extend_from_slice(&[0xE9, back as u8, (back >> 8) as u8]);
    m.board().ram.cells[0x10000..0x10000 + code.len()].copy_from_slice(&code);
    m.code_changed();
    code
}

/// Random straight-line 386 code (its operands, registers and the memory
/// it touches random; port I/O included; a fault returns to the faulting
/// instruction through the ROM's IRET) in a loop, made by `run` (on the JIT
/// build: compiled, once hot) and by `step`: the same registers, memory and
/// time.
#[test]
fn random_programs_agree_with_stepping() {
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let mut compiled = 0;
    let mut made = 0u64;
    let mut bytes = 0;
    for seed in 0..200 {
        let mut a = boot(rom(TO_RAM));
        let b = a.board();
        for c in b.ram.cells[0x400..0x20000].iter_mut() {
            *c = rng.next() as u8;
        }
        // Every vector: the IRET at F000:F065.
        for v in 0..256 {
            b.ram.cells[4 * v..4 * v + 4].copy_from_slice(&[0x65, 0xF0, 0x00, 0xF0]);
        }
        a.code_changed();
        // The reset vector's jump to F000:0000, then the ROM's to 1000:0000.
        a.step().unwrap();
        a.step().unwrap();
        assert_eq!((a.cpu.r.CS.selector, a.cpu.r.EIP), (0x1000, 0));
        let r = &mut a.cpu.r;
        for p in [&mut r.EAX, &mut r.EBX, &mut r.ECX, &mut r.EDX, &mut r.ESI, &mut r.EDI, &mut r.EBP] {
            *p = rng.next() as u32;
        }
        r.ESP = 0x8000 + (rng.next() as u32 & 0x3FFE);
        let program = random_program(&mut rng, &mut a, 16);
        bytes += program.len();
        let mut b = a.clone();
        let steps = 20_000;
        let mut na = 0;
        while na < steps && !a.cpu.halted && !a.cpu.shutdown {
            a.step().unwrap();
            na += 1;
        }
        let nb = b.run_until(steps, |s| s.cpu.halted || s.cpu.shutdown).unwrap();
        compiled += usize::from(b.cpu.compiled_runs() > 0);
        assert_eq!(a.cpu.r, b.cpu.r, "program {seed}: registers after {na} / {nb} steps ({program:02x?})");
        assert_eq!(na, nb, "program {seed}: steps made");
        made += na;
        assert_eq!(a.now(), b.now(), "program {seed}: time");
        let (ra, rb) = (&a.board().ram.cells.clone(), &b.board().ram.cells);
        if let Some(k) = (0..ra.len()).find(|&k| ra[k] != rb[k]) {
            panic!("program {seed}: RAM differs at {k:05x}: {:02x} stepped, {:02x} run ({program:02x?})", ra[k], rb[k]);
        }
    }
    eprintln!("{} ({}): {compiled} of 200 programs compiled, {made} steps, {bytes} bytes of code", machine_board386_vga::system::PROFILE, machine_board386_vga::system::CORE);
}

/// Speed in steady state: a real-mode checksum loop over 32 KB of RAM
/// (`xor ax,ax; xor bx,bx; mov si,0; mov cx,4000h; l: mov dx,[si]; add
/// ax,dx; xor bx,ax; rol bx,1; add si,2; dec cx; jnz l; jmp 0`), timed while
/// it runs. Run with `cargo test --release -- --ignored --nocapture
/// bench_hot_loop`.
#[test]
#[ignore]
fn bench_hot_loop() {
    const LOOP: &[u8] = &[
        0x31, 0xC0, 0x31, 0xDB, 0xBE, 0x00, 0x00, 0xB9, 0x00, 0x40, 0x8B, 0x14, 0x01, 0xD0, 0x31, 0xC3, 0xD1, 0xC3, 0x83, 0xC6, 0x02, 0x49,
        0x75, 0xF2, 0xEB, 0xE6,
    ];
    for _ in 0..3 {
        let mut a = boot(rom(TO_RAM));
        a.board().ram.cells[0x10000..0x10000 + LOOP.len()].copy_from_slice(LOOP);
        a.code_changed();
        let wall = std::time::Instant::now();
        let n = a.run_until(20_000_000, |s| s.cpu.halted || s.cpu.shutdown).unwrap();
        let w = wall.elapsed().as_secs_f64();
        assert_eq!(n, 20_000_000, "the loop ran");
        eprintln!("{} ({}): {n} instructions, {:.1} M/s", machine_board386_vga::system::PROFILE, machine_board386_vga::system::CORE, n as f64 / w / 1e6);
    }
}
