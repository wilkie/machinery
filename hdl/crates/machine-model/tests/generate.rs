//! End to end: generate the interpreter for the `cpu_slice` sample, compile
//! it with cargo, and run its tests (the two ADD tests from the sample).

use std::path::PathBuf;
use std::process::Command;

use machine_model::{crate_name, generate_isa, package, write_crate};
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
        "fn t4_add_al__bl()",
        "fn t5_add__bx_si___cl_through_memory()",
        "fn t2_sub_borrow_is_bit_w_of_the_wide_result()",
        "pub mod vectors {",
        "pub fn undefined_mask(",
    ] {
        assert!(
            src.contains(needle),
            "generated source lacks `{needle}`:\n{src}"
        );
    }
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
        stdout.contains("6 passed") || stderr.contains("6 passed"),
        "expected 6 generated tests to pass:\n{stdout}\n{stderr}"
    );
}

fn target(name: &str) -> SourceFile {
    let path = hdl_root().join("targets").join("i286").join(name);
    SourceFile::new(name, std::fs::read_to_string(&path).unwrap())
}

/// The real i286 description: its own tests pass, and, when the
/// SingleStepTests corpus is present, a representative subset of hardware
/// vectors passes too (prefixed ALU forms, the immediate group, a
/// conditional jump, a constrained group opcode, HLT).
#[test]
fn i286_target_passes_its_tests_and_hardware_vectors() {
    let checked = check_files(vec![target("alu.machine"), target("i286.machine")]);
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
    assert!(stdout.contains("14 passed"), "expected 14 tests:\n{stdout}");

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
            "00,05,26,2B,80.4,83.5,74,C6,F4,CC,CE,62,CF,9D,50,E8,81.0,A4,A5,A7,AB,AD,AE",
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
}
