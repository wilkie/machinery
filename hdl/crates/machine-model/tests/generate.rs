//! End to end: generate the interpreter for the `cpu_slice` sample, compile
//! it with cargo, and run its tests (the two ADD tests from the sample).

use std::path::PathBuf;
use std::process::Command;

use machine_model::{bulk_report, crate_name, generate_isa, generate_system, package, write_crate, BulkReport};
use machine_sema::check_files;
use machine_syntax::SourceFile;

fn hdl_root() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "..", ".."]
        .iter()
        .collect::<PathBuf>()
        .canonicalize()
        .unwrap()
}

fn sample(name: &str) -> SourceFile {
    let path = hdl_root().join("samples").join(name);
    SourceFile::new(name, std::fs::read_to_string(&path).unwrap())
}

#[test]
fn cpu_slice_generates_expected_items() {
    let checked = check_files(vec![sample("alu.machine"), sample("cpu_slice.machine")]);
    assert!(!checked.has_errors());
    let src = generate_isa(&checked, "I286Isa").unwrap_or_else(|e| panic!("{e}"));
    for needle in [
        "pub fn unit_Alu_8(",
        "pub fn unit_AluFlags_16(",
        "pub struct ArithFlags",
        "pub mod AluOp",
        "pub enum Fault",
        "pub struct ModRM",
        "pub enum ModRM_Tail",
        "pub fn kind_r8_select_read(",
        "pub fn kind_rm8_read(",
        "pub fn kind_rm8_place_of(",
        "pub fn fn_translate(",
        "pub fn exec_Add_rm8_r8(",
        "pub fn decode(&mut self)",
        "pub fn step(&mut self)",
        "fn t5_add_al__bl()",
        "fn t6_add__bx_si___cl_through_memory()",
        "fn t3_sub_borrow_is_bit_w_of_the_wide_result()",
        "fn t0_lowest_set_bit()",
        "let mut l1_found = 8u8;",
        "pub mod vectors {",
        "pub fn undefined_mask(",
    ] {
        assert!(
            src.contains(needle),
            "generated source lacks `{needle}`:\n{src}"
        );
    }
}

/// The sample isa with these items added.
fn cpu_slice_with(items: &str) -> String {
    let text = std::fs::read_to_string(hdl_root().join("samples").join("cpu_slice.machine")).unwrap();
    // The isa's own closing brace: the first at the start of a line.
    let at = text.find("\n}\n").unwrap();
    format!("{}\n{items}\n{}", &text[..at], &text[at..])
}

#[test]
fn a_bulk_repeat_is_checked() {
    let fill = r#"
    fn more() -> bool { CX = CX - 1; return CX != 0; }
    fn fill_one() { mem.write_u8(DI as u24, AL); DI = DI + 1; }
    fn chase() { let p = mem.read_u8(SI as u24); SI = SI + (p as u16); }
    instruction BulkFill { mnemonic "bf"; encoding { opcode 0xD6; } repeat when true skip CX == 0 next more() interruptible bulk; semantics { fill_one(); } }
"#;
    let checked = check_files(vec![sample("alu.machine"), SourceFile::new("cpu_slice.machine", cpu_slice_with(fill))]);
    assert!(!checked.has_errors(), "{:?}", checked.diagnostics);
    generate_isa(&checked, "I286Isa").unwrap_or_else(|e| panic!("a fill is a stream: {e}"));
    // A pointer that moves by what the element loaded is not a stream.
    let chase = fill.replace("semantics { fill_one(); }", "semantics { chase(); }");
    let checked = check_files(vec![sample("alu.machine"), SourceFile::new("cpu_slice.machine", cpu_slice_with(&chase))]);
    assert!(!checked.has_errors(), "{:?}", checked.diagnostics);
    let err = generate_isa(&checked, "I286Isa").err().expect("chasing a pointer is not a stream").to_string();
    assert!(err.contains("BulkFill") && err.contains("does not move with the element"), "{err}");
}

/// The `bulk` plan of the sample isa with `items` added (one `bulk`
/// instruction, `B`, opcode D6), and the added source.
fn bulk_plan(items: &str) -> (BulkReport, String) {
    let text = cpu_slice_with(&format!("{items}\n    instruction B {{ mnemonic \"b\"; encoding {{ opcode 0xD6; }} repeat when true skip CX == 0 next more() interruptible bulk; semantics {{ element(); }} }}\n    fn more() -> bool {{ CX = CX - 1; return CX != 0; }}"));
    let checked = check_files(vec![sample("alu.machine"), SourceFile::new("cpu_slice.machine", text.clone())]);
    assert!(!checked.has_errors(), "{:?}", checked.diagnostics);
    let mut r = bulk_report(&checked, "I286Isa").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.len(), 1);
    (r.remove(0), text)
}

#[test]
fn bulk_kernels_are_planned() {
    // A fill: a kernel, its dry code stopping nowhere.
    let (r, _) = bulk_plan("    fn element() { mem.write_u8(DI as u24, AL); DI = DI + 1; }");
    assert_eq!(r.shape, "Fill");
    assert!(r.kernel.as_ref().is_ok_and(|k| k.starts_with("Fill of 8 bits, moving 1")), "{r:?}");
    assert!(r.stops.is_empty(), "{r:?}");
    // Where one case (BX's top bit) branches on where the element is, the
    // dry code stops at what the branch tests, which the other case does
    // not evaluate: a kernel still.
    let (r, text) = bulk_plan(
        "    fn ok(a: u16) -> bool { return BX[15] == 0 || (a & 0xFF) != 0xFF; }\n    fn element() { if ok(DI) { mem.write_u8(DI as u24, AL); } else { mem.write_u8(0, AL); } DI = DI + 1; }",
    );
    assert!(r.kernel.is_ok(), "{r:?}");
    let stops: Vec<&str> = r.stops.iter().map(|s| &text[s.start as usize..s.end as usize]).collect();
    assert_eq!(stops, ["(a & 0xFF) != 0xFF"]);
    // A fault check that need not hold between the first and the last
    // element (DI odd), met on every path: no kernel.
    let (r, _) = bulk_plan("    fn element() { if (DI & 1) == 1 { raise GP(0); } mem.write_u8(DI as u24, AL); DI = DI + 1; }");
    assert_eq!(r.shape, "Fill");
    assert!(r.kernel.is_err(), "{r:?}");
    // A count the element writes too: no kernel.
    let (r, _) = bulk_plan("    fn element() { mem.write_u8(DI as u24, AL); DI = DI + 1; CX = CX + 0; }");
    assert!(r.kernel.as_ref().is_err_and(|w| w.contains("count")), "{r:?}");
    // A scan whose `while` depends on what it loaded (REPNE): a kernel
    // testing it on the loaded value.
    let scan = "    fn element() { let b = mem.read_u8(DI as u24); let (_, raw) = Alu<8>(AluOp::Cmp, AL, b, FLAGS.CF); FLAGS.arith = AluFlags<8>(AluOp::Cmp, AL, b, raw); DI = DI + 1; }";
    let text = cpu_slice_with(&format!("{scan}\n    instruction B {{ mnemonic \"b\"; encoding {{ opcode 0xD6; }} repeat when true skip CX == 0 next more() while FLAGS.ZF == 0 interruptible bulk; semantics {{ element(); }} }}\n    fn more() -> bool {{ CX = CX - 1; return CX != 0; }}"));
    let checked = check_files(vec![sample("alu.machine"), SourceFile::new("cpu_slice.machine", text)]);
    assert!(!checked.has_errors(), "{:?}", checked.diagnostics);
    let r = bulk_report(&checked, "I286Isa").unwrap_or_else(|e| panic!("{e}")).remove(0);
    assert_eq!(r.shape, "Scan");
    let k = r.kernel.unwrap_or_else(|w| panic!("a scan has a kernel: {w}"));
    assert!(k.contains("unit_AluFlags_8(AluOp::Cmp,") && k.contains("l0_ld1") && k.ends_with("== (0u8))"), "{k}");
    // Its flags from a carry it is given, not just what it loaded (an ADC):
    // its `while` depends on the element before it. No kernel.
    let adc = scan.replace("AluOp::Cmp", "AluOp::Adc");
    let text = cpu_slice_with(&format!("{adc}\n    instruction B {{ mnemonic \"b\"; encoding {{ opcode 0xD6; }} repeat when true skip CX == 0 next more() while FLAGS.ZF == 0 interruptible bulk; semantics {{ element(); }} }}\n    fn more() -> bool {{ CX = CX - 1; return CX != 0; }}"));
    let checked = check_files(vec![sample("alu.machine"), SourceFile::new("cpu_slice.machine", text)]);
    let r = bulk_report(&checked, "I286Isa").unwrap_or_else(|e| panic!("{e}")).remove(0);
    assert!(r.kernel.is_err(), "{r:?}");
}

#[test]
fn cpu_slice_interpreter_compiles_and_passes_its_tests() {
    let checked = check_files(vec![sample("alu.machine"), sample("cpu_slice.machine")]);
    assert!(!checked.has_errors());
    let src = generate_isa(&checked, "I286Isa").unwrap_or_else(|e| panic!("{e}"));
    let root = hdl_root();
    let runtime = root.join("crates").join("machine-runtime");
    let crate_ = package(&crate_name("I286Isa"), src, &runtime);
    let dir = root.join("target").join("gen-tests").join("cpu_slice");
    write_crate(&dir, &crate_).unwrap();

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .arg("test")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target").join("gen-target"))
        .output()
        .expect("run cargo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "generated crate failed:\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}\n--- source ---\n{}",
        crate_.lib_rs
    );
    assert!(
        stdout.contains("7 passed") || stderr.contains("7 passed"),
        "expected 7 generated tests to pass:\n{stdout}\n{stderr}"
    );
}

/// Instruction families and decode conditions (SPEC 11.2d): the
/// `families` sample's tests run its 16- and 32-bit instances and its
/// 32-bit addressing through a SIB.
#[test]
fn families_sample_passes_its_tests() {
    let checked = check_files(vec![sample("alu.machine"), sample("families.machine")]);
    assert!(!checked.has_errors());
    let src = generate_isa(&checked, "T").unwrap_or_else(|e| panic!("{e}"));
    let root = hdl_root();
    let crate_ = package(
        &crate_name("T"),
        src,
        &root.join("crates").join("machine-runtime"),
    );
    let dir = root.join("target").join("gen-tests").join("families");
    write_crate(&dir, &crate_).unwrap();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .args(["test", "--quiet", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target").join("gen-target"))
        .output()
        .expect("run cargo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "families failed:\n{stdout}\n{stderr}"
    );
    assert!(stdout.contains("10 passed"), "expected 10 tests:\n{stdout}");
}

fn target(name: &str) -> SourceFile {
    let path = hdl_root().join("targets").join("i286").join(name);
    SourceFile::new(name, std::fs::read_to_string(&path).unwrap())
}

/// A file of the i386 target (`targets/i386`).
fn target386(name: &str) -> SourceFile {
    let path = hdl_root().join("targets").join("i386").join(name);
    SourceFile::new(name, std::fs::read_to_string(&path).unwrap())
}

/// A file of the PC library (`targets/pc`): the ISA bus and its devices.
fn pc(name: &str) -> SourceFile {
    let path = hdl_root().join("targets").join("pc").join(name);
    SourceFile::new(name, std::fs::read_to_string(&path).unwrap())
}

/// The real i286 description: its own tests pass, and, when the
/// SingleStepTests corpus is present, a representative subset of hardware
/// vectors passes too (prefixed ALU forms, the immediate group, a
/// conditional jump, a constrained group opcode, HLT).
#[test]
fn i286_target_passes_its_tests_and_hardware_vectors() {
    let checked = check_files(vec![
        target("alu.machine"),
        target("i286.machine"),
        target("timing.machine"),
        target("schedules.machine"),
    ]);
    assert!(!checked.has_errors());
    let src = generate_isa(&checked, "I286").unwrap_or_else(|e| panic!("{e}"));
    let root = hdl_root();
    let crate_ = package(
        &crate_name("I286"),
        src,
        &root.join("crates").join("machine-runtime"),
    );
    assert!(crate_.vectors_bin.is_some());
    let dir = root.join("target").join("gen-tests").join("i286");
    write_crate(&dir, &crate_).unwrap();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let target_dir = root.join("target").join("gen-target");

    let output = Command::new(&cargo)
        .args(["test", "--quiet", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .expect("run cargo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "i286 tests failed:\n{stdout}\n{stderr}"
    );
    assert!(stdout.contains("18 passed"), "expected 18 tests:\n{stdout}");

    // The self-checking test programs (`simulators/test/i286`, assembled
    // with `assemble-tests.sh`): real mode and protected mode. Skipped
    // without them.
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::copy(
        root.join("targets").join("i286").join("programs_test.rs"),
        dir.join("tests").join("programs.rs"),
    )
    .unwrap();
    let output = Command::new(&cargo)
        .args([
            "test",
            "--quiet",
            "--release",
            "--test",
            "programs",
            "--manifest-path",
        ])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("--")
        .arg("--nocapture")
        .output()
        .expect("run programs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "programs failed:\n{stdout}\n{stderr}"
    );

    let vectors = root
        .join("..")
        .join("simulators")
        .join("test")
        .join("i286")
        .join("SingleStepTests")
        .join("v1_real_mode");
    if !vectors.join("00.MOO.gz").exists() {
        eprintln!(
            "SingleStepTests corpus not found at {}; skipping hardware vectors",
            vectors.display()
        );
        return;
    }
    let output = Command::new(&cargo)
        .args([
            "run",
            "--quiet",
            "--features",
            "vectors",
            "--bin",
            "vectors",
            "--manifest-path",
        ])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("--")
        .arg("--dir")
        .arg(&vectors)
        .args([
            "--filter",
            "00,05,26,2B,80.4,83.5,74,C6,F4,CC,CE,62,CF,9D,50,E8,81.0,A4,A5,A7,AB,AD,AE,D3.5,C0.2,F7.4,F6.6,87,8D,C8,60,9A,E5,FF.3,D4,2F,61",
            "--max-failures",
            "5",
        ])
        .output()
        .expect("run vectors");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "hardware vectors failed:\n{stdout}\n{stderr}"
    );
    assert!(stdout.contains(" 0 failed"), "{stdout}");

    // The timing table against the same vectors' cycle traces: most match
    // exactly and nearly all within a cycle (the rest is the queue's
    // overlap, which tier 1 does not model).
    let output = Command::new(&cargo)
        .args([
            "run",
            "--quiet",
            "--features",
            "vectors",
            "--bin",
            "vectors",
            "--manifest-path",
        ])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("--")
        .arg("--dir")
        .arg(&vectors)
        .args([
            "--timing",
            "--quiet",
            "--filter",
            "00,05,74,F4,CC,9D,50,E8,A4,AB,AD,D3.5,C0.2,F7.4,F6.6,87,C8,60",
        ])
        .output()
        .expect("run timing");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|l| l.starts_with("timing:"))
        .unwrap_or_else(|| panic!("{stdout}"));
    let pct = |label: &str| -> f64 {
        let i = line.find(label).unwrap_or_else(|| panic!("{line}"));
        let head = line[..i].trim_end_matches('%').trim_end();
        let head = head.trim_end_matches('%');
        head.rsplit(' ')
            .next()
            .unwrap()
            .trim_end_matches('%')
            .parse()
            .unwrap()
    };
    assert!(pct("exact") >= 70.0, "{line}");
    assert!(pct("within 1") >= 90.0, "{line}");
}

/// The i386 description: it generates and compiles; the self-checking
/// programs pass (the 286's, and the 386's protected-mode, paging and
/// virtual-8086 ones); and, when the SingleStepTests 80386 corpus is
/// present, a subset of real-mode hardware vectors across the operand- and
/// address-size prefixes passes.
#[test]
fn i386_target_passes_hardware_vectors() {
    let root = hdl_root();
    let read = |dir: &str, name: &str| {
        let path = root.join("targets").join(dir).join(name);
        SourceFile::new(name, std::fs::read_to_string(&path).unwrap())
    };
    let checked = check_files(vec![
        read("i286", "alu.machine"),
        read("i386", "i386.machine"),
        read("i386", "timing.machine"),
    ]);
    assert!(!checked.has_errors());
    let src = generate_isa(&checked, "I386").unwrap_or_else(|e| panic!("{e}"));
    let crate_ = package(
        &crate_name("I386"),
        src,
        &root.join("crates").join("machine-runtime"),
    );
    let dir = root.join("target").join("gen-tests").join("i386");
    write_crate(&dir, &crate_).unwrap();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let target_dir = root.join("target").join("gen-target");
    let output = Command::new(&cargo)
        .args(["test", "--quiet", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .expect("run cargo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "i386 tests failed:\n{stdout}\n{stderr}"
    );

    // The self-checking programs: the 286's suites, and the 386's own
    // (protected mode, paging, virtual-8086 mode). Each suite is skipped
    // when it has not been assembled.
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::copy(
        root.join("targets").join("i386").join("programs_test.rs"),
        dir.join("tests").join("programs.rs"),
    )
    .unwrap();
    let output = Command::new(&cargo)
        .args([
            "test",
            "--quiet",
            "--release",
            "--test",
            "programs",
            "--manifest-path",
        ])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("--")
        .arg("--nocapture")
        .output()
        .expect("run programs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "programs failed:\n{stdout}\n{stderr}"
    );

    let vectors = root
        .join("..")
        .join("simulators")
        .join("test")
        .join("i386")
        .join("SingleStepTests")
        .join("v1_ex_real_mode");
    if !vectors.join("00.MOO.gz").exists() {
        eprintln!(
            "SingleStepTests 80386 corpus not found at {}; skipping hardware vectors",
            vectors.display()
        );
        return;
    }
    let output = Command::new(&cargo)
        .args([
            "run",
            "--quiet",
            "--features",
            "vectors",
            "--bin",
            "vectors",
            "--manifest-path",
        ])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("--")
        .arg("--dir")
        .arg(&vectors)
        .args([
            "--filter",
            "00,6601,6703,676629,80.4,6683.5,0F8E,660F85,E8,66E8,C3,66CF,9A,66EA,FF.3,668F,6660,6661,C8,66C8,A4,66A5,67AD,67AE,D3.5,66C1.7,F7.4,66F7.6,6687,8D,678D,0FA3,660FBA.5,0FBC,0FA4,660FAD,0FB6,660FBF,0F94,62,CC,CF,E4,66ED,27,D5,9C,8E,0FA1,660FB2,F4",
            "--max-failures",
            "5",
        ])
        .output()
        .expect("run vectors");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "hardware vectors failed:\n{stdout}\n{stderr}"
    );
    assert!(stdout.contains(" 0 failed"), "{stdout}");

    // The timing table against the same vectors' cycle traces. Tier 1
    // cannot see the clock that depends on where the next instruction
    // falls in the decoder, nor how misaligned accesses overlap in
    // pipelined sequences, so about half match exactly and most within a
    // clock (`targets/i386/timing.machine`).
    let output = Command::new(&cargo)
        .args(["run", "--quiet", "--features", "vectors", "--bin", "vectors", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .arg("--")
        .arg("--dir")
        .arg(&vectors)
        .args([
            "--timing",
            "--quiet",
            "--filter",
            "00,6601,6703,80.4,6683.5,0F8E,E8,66E8,C3,66CF,9A,FF.3,668F,6660,6661,C8,A4,66A5,67AD,D3.5,66C1.7,F7.4,66F7.6,6687,8D,0FA3,0FBC,0FA4,0FB6,62,CC,CF,27,D5,9C,8E,0FA1,F4,74,E2,F3",
        ])
        .output()
        .expect("run timing");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|l| l.starts_with("timing:"))
        .unwrap_or_else(|| panic!("{stdout}"));
    let pct = |label: &str| -> f64 {
        let i = line.find(label).unwrap_or_else(|| panic!("{line}"));
        line[..i]
            .trim_end()
            .trim_end_matches('%')
            .rsplit(' ')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    assert!(pct("% exact") >= 45.0, "{line}");
    assert!(pct("% within 1") >= 75.0, "{line}");
}

/// The model backend: the i286 board system generates, compiles, boots a
/// ROM program (`targets/i286/board_test.rs`) and charges bus time.
#[test]
fn i286_board_system_boots() {
    let checked = check_files(vec![
        target("alu.machine"),
        target("i286.machine"),
        target("timing.machine"),
        target("schedules.machine"),
        pc("isabus.machine"),
        pc("crtc.machine"),
        pc("pc.machine"),
        target("board.machine"),
    ]);
    assert!(!checked.has_errors());
    let src = generate_system(&checked, "Board286", Some("fast")).unwrap_or_else(|e| panic!("{e}"));
    let root = hdl_root();
    let crate_ = package(
        &crate_name("Board286"),
        src,
        &root.join("crates").join("machine-runtime"),
    );
    let dir = root.join("target").join("gen-tests").join("board286");
    write_crate(&dir, &crate_).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::copy(
        root.join("targets").join("i286").join("board_test.rs"),
        dir.join("tests").join("board.rs"),
    )
    .unwrap();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(&cargo)
        .args(["test", "--quiet", "--test", "board", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target").join("gen-target"))
        .output()
        .expect("run cargo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "board tests failed:\n{stdout}\n{stderr}"
    );
    assert!(stdout.contains("22 passed"), "expected 22 tests:\n{stdout}");
}

/// The IBM PC/AT (`At286` in `targets/i286/board.machine`): the 286 board
/// with the AT's system-board devices (the MC146818 clock, the 8042
/// keyboard controller, the second DMA controller, port B), booting IBM's
/// own BIOS through its power-on self test when the ROM images are present
/// (`targets/i286/at_test.rs`).
#[test]
fn i286_at_boots_its_bios() {
    let checked = check_files(vec![
        target("alu.machine"),
        target("i286.machine"),
        target("timing.machine"),
        target("schedules.machine"),
        pc("isabus.machine"),
        pc("crtc.machine"),
        pc("pc.machine"),
        target("board.machine"),
    ]);
    assert!(!checked.has_errors());
    let src = generate_system(&checked, "At286", Some("fast")).unwrap_or_else(|e| panic!("{e}"));
    let root = hdl_root();
    let crate_ = package(&crate_name("At286"), src, &root.join("crates").join("machine-runtime"));
    let dir = root.join("target").join("gen-tests").join("at286");
    write_crate(&dir, &crate_).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::copy(root.join("targets").join("i286").join("at_test.rs"), dir.join("tests").join("at.rs")).unwrap();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(&cargo)
        // Release: the BIOS runs a minute of the AT's time.
        .args(["test", "--release", "--quiet", "--test", "at", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target").join("gen-target"))
        .output()
        .expect("run cargo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "AT tests failed:\n{stdout}\n{stderr}");
}

/// A 386 AT with a VGA (`At386Vga` in `targets/i386/board.machine`): the
/// AT's system-board devices on the 386 board, booting AMI's AT BIOS and
/// MS-DOS from disk images when they are present
/// (`targets/i386/at386_test.rs`).
#[test]
fn i386_at_boots_dos() {
    let checked = check_files(vec![
        pc("isabus.machine"),
        pc("crtc.machine"),
        pc("pc.machine"),
        pc("vga.machine"),
        pc("sb.machine"),
        pc("opl.machine"),
        target("alu.machine"),
        target386("i386.machine"),
        target386("timing.machine"),
        target386("board.machine"),
    ]);
    assert!(!checked.has_errors());
    // The exact build, and the emulator with the trace core: the same DOS
    // boots and installs on both.
    for (profile, dir) in [("fast", "at386vga"), ("trace", "at386vga-trace"), ("jit", "at386vga-jit"), ("approx", "at386vga-approx"), ("blocks", "at386vga-blocks"), ("quantum", "at386vga-quantum")] {
        let src = generate_system(&checked, "At386Vga", Some(profile)).unwrap_or_else(|e| panic!("{e}"));
        // `repeat: bulk(n)`: the memory streams run through kernels; the
        // port streams do not.
        for k in ["Movs", "Stos", "Lods", "Scas", "Cmps"] {
            for w in ["b", "_16", "_32"] {
                assert_eq!(src.contains(&format!("pub fn bulk_{k}{w}(")), matches!(profile, "blocks" | "quantum"), "{profile}: bulk_{k}{w}");
            }
        }
        assert!(!src.contains("pub fn bulk_Insb(") && !src.contains("pub fn bulk_Outsb("), "a port stream has no kernel");
        let root = hdl_root();
        let crate_ = package(&crate_name("At386Vga"), src, &root.join("crates").join("machine-runtime"));
        let dir = root.join("target").join("gen-tests").join(dir);
        write_crate(&dir, &crate_).unwrap();
        std::fs::create_dir_all(dir.join("tests")).unwrap();
        std::fs::copy(root.join("targets").join("i386").join("at386_test.rs"), dir.join("tests").join("at386.rs")).unwrap();
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let output = Command::new(&cargo)
            .args(["test", "--release", "--quiet", "--test", "at386", "--manifest-path"])
            .arg(dir.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", root.join("target").join("gen-target"))
            // The JIT compiles a run entered 64 times here, and again
            // optimised past 1000 steps a step (its defaults wait longer):
            // the tests' short loops run compiled, at both tiers.
            .env("MACHINE_JIT_HOT", "64")
            .env("MACHINE_JIT_HOT2", "1000")
            .output()
            .expect("run cargo");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "386 AT tests ({profile}) failed:\n{stdout}\n{stderr}");
    }
}

/// The 386 board: the i386 on a 32-bit local bus with RAM and a ROM, and
/// the PC's devices on the ISA bus behind the AT bus controller (a bridge:
/// its handlers run ISA cycles). Generates, compiles, boots ROM programs
/// (`targets/i386/board_test.rs`) and charges ISA time.
#[test]
fn i386_board_system_boots() {
    let root = hdl_root();
    let read = |dir: &str, name: &str| {
        let path = root.join("targets").join(dir).join(name);
        SourceFile::new(name, std::fs::read_to_string(&path).unwrap())
    };
    let checked = check_files(vec![
        pc("isabus.machine"),
        pc("crtc.machine"),
        pc("pc.machine"),
        pc("vga.machine"),
        pc("sb.machine"),
        pc("opl.machine"),
        read("i286", "alu.machine"),
        read("i386", "i386.machine"),
        read("i386", "timing.machine"),
        read("i386", "board.machine"),
    ]);
    assert!(!checked.has_errors());
    let src = generate_system(&checked, "Board386", Some("fast")).unwrap_or_else(|e| panic!("{e}"));
    let crate_ = package(
        &crate_name("Board386"),
        src,
        &root.join("crates").join("machine-runtime"),
    );
    let dir = root.join("target").join("gen-tests").join("board386");
    write_crate(&dir, &crate_).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::copy(
        root.join("targets").join("i386").join("board_test.rs"),
        dir.join("tests").join("board.rs"),
    )
    .unwrap();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(&cargo)
        .args(["test", "--quiet", "--test", "board", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target").join("gen-target"))
        .output()
        .expect("run cargo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "board tests failed:\n{stdout}\n{stderr}"
    );
    assert!(stdout.contains("12 passed"), "expected 12 tests:\n{stdout}");
}

/// The 386 board with a VGA (`Board386Vga`): generates, compiles, and runs
/// `targets/i386/vga_test.rs`, which sets the card up with IBM's VGA BIOS
/// when its image is in `~/roms` and drives it directly otherwise.
#[test]
fn i386_vga_board_system_boots() {
    let root = hdl_root();
    let read = |dir: &str, name: &str| {
        let path = root.join("targets").join(dir).join(name);
        SourceFile::new(name, std::fs::read_to_string(&path).unwrap())
    };
    let checked = check_files(vec![
        pc("isabus.machine"),
        pc("crtc.machine"),
        pc("pc.machine"),
        pc("vga.machine"),
        pc("sb.machine"),
        pc("opl.machine"),
        read("i286", "alu.machine"),
        read("i386", "i386.machine"),
        read("i386", "timing.machine"),
        read("i386", "board.machine"),
    ]);
    assert!(!checked.has_errors());
    // Both builds: the exact one, and the emulator (`memory: direct`).
    for (profile, dir) in [("fast", "board386vga"), ("emulate", "board386vga-emulate"), ("trace", "board386vga-trace"), ("jit", "board386vga-jit")] {
        let src = generate_system(&checked, "Board386Vga", Some(profile)).unwrap_or_else(|e| panic!("{e}"));
        let crate_ = package(
            &crate_name("Board386Vga"),
            src,
            &root.join("crates").join("machine-runtime"),
        );
        let dir = root.join("target").join("gen-tests").join(dir);
        write_crate(&dir, &crate_).unwrap();
        std::fs::create_dir_all(dir.join("tests")).unwrap();
        std::fs::copy(
            root.join("targets").join("i386").join("vga_test.rs"),
            dir.join("tests").join("vga.rs"),
        )
        .unwrap();
        std::fs::copy(root.join("targets").join("i386").join("sb_test.rs"), dir.join("tests").join("sb.rs")).unwrap();
        std::fs::copy(root.join("targets").join("i386").join("opl_test.rs"), dir.join("tests").join("opl.rs")).unwrap();
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let output = Command::new(&cargo)
            .args(["test", "--quiet", "--test", "vga", "--test", "sb", "--test", "opl", "--manifest-path"])
            .arg(dir.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", root.join("target").join("gen-target"))
            // The JIT compiles a run entered 64 times here, and again
            // optimised past 1000 steps a step (its defaults wait longer):
            // the tests' short loops run compiled, at both tiers.
            .env("MACHINE_JIT_HOT", "64")
            .env("MACHINE_JIT_HOT2", "1000")
            .output()
            .expect("run cargo");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "VGA board tests, the Sound Blaster's and the OPL2's too ({profile}) failed:\n{stdout}\n{stderr}");
    }
}

/// The processor model's pipeline (SPEC 11.2c): the conformance rig
/// (`targets/i286/rig.machine`) runs SingleStepTests vectors through
/// `Cpu286` and compares its bus cycles with the hardware traces
/// (`targets/i286/rig_test.rs`); nearly all match cycle for cycle. Skipped
/// without the corpus.
#[test]
fn i286_pipeline_reproduces_bus_traces() {
    let checked = check_files(vec![
        target("alu.machine"),
        target("i286.machine"),
        target("timing.machine"),
        target("schedules.machine"),
        pc("isabus.machine"),
        pc("crtc.machine"),
        pc("pc.machine"),
        target("board.machine"),
        target("rig.machine"),
    ]);
    assert!(!checked.has_errors());
    let src = generate_system(&checked, "Rig286", Some("fast")).unwrap_or_else(|e| panic!("{e}"));
    let root = hdl_root();
    let crate_ = package(
        &crate_name("Rig286"),
        src,
        &root.join("crates").join("machine-runtime"),
    );
    let dir = root.join("target").join("gen-tests").join("rig286");
    write_crate(&dir, &crate_).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::copy(
        root.join("targets").join("i286").join("rig_test.rs"),
        dir.join("tests").join("rig.rs"),
    )
    .unwrap();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(&cargo)
        .args([
            "test",
            "--quiet",
            "--release",
            "--features",
            "vectors",
            "--test",
            "rig",
            "--manifest-path",
        ])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target").join("gen-target"))
        .env("RIG_MIN", "97")
        .arg("--")
        .arg("--nocapture")
        .output()
        .expect("run cargo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "rig failed:\n{stdout}\n{stderr}");
}

/// The sample timer board has no processor (its master is an rtl script),
/// which the model backend reports instead of generating.
#[test]
fn system_without_processor_is_reported() {
    let checked = check_files(vec![sample("counter_bus.machine")]);
    assert!(!checked.has_errors());
    let err = generate_system(&checked, "TimerBoard", None).unwrap_err();
    assert!(err.to_string().contains("no processor"), "{err}");
}

fn target_z80(name: &str) -> SourceFile {
    let path = hdl_root().join("targets").join("z80").join(name);
    SourceFile::new(name, std::fs::read_to_string(&path).unwrap())
}

/// The Z80 isa through generated code against the SingleStepTests Z80
/// vectors: a file from each page and prefix (the whole corpus, 1.6
/// million vectors, is `machine vectors --isa Z80 targets/z80/z80.machine`).
/// Skipped without the corpus.
/// A file from each page and prefix, and the instructions whose timing
/// depends on data (DJNZ, JR cc, CALL cc, the CB page on (HL), the
/// repeating block instructions).
const Z80_FILTER: &str = "00,0a,10,27,28,37,3f,76,86,c3,cd,d4,db,e3,cb 06,cb 46,cb 7e,dd 21,dd 34,dd 7e,dd e3,dd cb __ 06,fd cb __ 46,ed 42,ed 4a,ed 57,ed 67,ed a1,ed b0,ed b1,ed b2,ed bb";

#[test]
fn z80_target_passes_hardware_vectors() {
    let root = hdl_root();
    let vectors = root.join("..").join("simulators").join("test").join("z80").join("SingleStepTests").join("v1");
    if !vectors.join("00.json").exists() {
        eprintln!("SingleStepTests Z80 corpus not found at {}; skipping hardware vectors", vectors.display());
        return;
    }
    let checked = check_files(vec![target_z80("z80.machine"), target_z80("timing.machine")]);
    assert!(!checked.has_errors());
    let src = generate_isa(&checked, "Z80").unwrap_or_else(|e| panic!("{e}"));
    let crate_ = package(&crate_name("Z80"), src, &root.join("crates").join("machine-runtime"));
    let dir = root.join("target").join("gen-tests").join("z80");
    write_crate(&dir, &crate_).unwrap();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(&cargo)
        .args(["run", "--quiet", "--release", "--features", "vectors", "--bin", "vectors", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target").join("gen-target"))
        .arg("--")
        .arg("--dir")
        .arg(&vectors)
        .args([
            "--filter",
            Z80_FILTER,
            "--max-failures",
            "5",
        ])
        .output()
        .expect("run vectors");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "Z80 vectors failed:\n{stdout}\n{stderr}");
    assert!(stdout.contains(" 0 failed"), "{stdout}");
    // And the timing table, to the T-state.
    let output = Command::new(&cargo)
        .args(["run", "--quiet", "--release", "--features", "vectors", "--bin", "vectors", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target").join("gen-target"))
        .arg("--")
        .arg("--dir")
        .arg(&vectors)
        .args(["--timing", "--quiet", "--filter", Z80_FILTER])
        .output()
        .expect("run vector timing");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let total = stdout.lines().find(|l| l.starts_with("timing:")).unwrap_or_default();
    assert!(total.contains(" 100.0% exact"), "Z80 timing:\n{stdout}");
}

/// The Z80 board on every core (decode, emulate, trace, JIT): the checksum
/// program, checked against Rust (`targets/z80/board_test.rs`).
#[test]
fn z80_board_runs_on_every_core() {
    let root = hdl_root();
    let checked = check_files(vec![target_z80("z80.machine"), target_z80("timing.machine"), target_z80("board.machine")]);
    assert!(!checked.has_errors());
    for profile in ["fast", "emulate", "trace", "jit", "approx"] {
        let src = generate_system(&checked, "BoardZ80", Some(profile)).unwrap_or_else(|e| panic!("{e}"));
        let crate_ = package(&crate_name("BoardZ80"), src, &root.join("crates").join("machine-runtime"));
        let dir = root.join("target").join("gen-tests").join(format!("boardz80-{profile}"));
        write_crate(&dir, &crate_).unwrap();
        std::fs::create_dir_all(dir.join("tests")).unwrap();
        std::fs::copy(root.join("targets").join("z80").join("board_test.rs"), dir.join("tests").join("board.rs")).unwrap();
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let output = Command::new(&cargo)
            .args(["test", "--release", "--quiet", "--test", "board", "--manifest-path"])
            .arg(dir.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", root.join("target").join("gen-target"))
            // The JIT compiles a run entered 64 times here, and again
            // optimised past 1000 steps a step (its defaults wait longer):
            // the tests' short loops run compiled, at both tiers.
            .env("MACHINE_JIT_HOT", "64")
            .env("MACHINE_JIT_HOT2", "1000")
            .output()
            .expect("run cargo");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "Z80 board ({profile}) failed:\n{stdout}\n{stderr}");
    }
}
