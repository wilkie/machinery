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
        "fn t0_add_al__bl()",
        "fn t1_add__bx_si___cl_through_memory()",
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
        stdout.contains("2 passed") || stderr.contains("2 passed"),
        "expected 2 generated tests to pass:\n{stdout}\n{stderr}"
    );
}
