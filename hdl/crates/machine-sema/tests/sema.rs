//! Semantic checks: the samples check clean (or are skipped where sema is
//! not implemented yet), and each strictness rule produces its diagnostic.

use std::path::PathBuf;

use machine_sema::{check_files, Checked, Ty};
use machine_syntax::SourceFile;

fn sample(name: &str) -> SourceFile {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "samples", name]
        .iter()
        .collect();
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    SourceFile::new(name, text)
}

fn render(c: &Checked) -> String {
    let mut out = String::new();
    for (m, diags) in c.diagnostics.iter().enumerate() {
        for d in diags.iter() {
            out.push_str(&d.render_text(&c.program.modules[m].file));
        }
    }
    out
}

fn check_src(src: &str) -> Checked {
    check_files(vec![SourceFile::new("t.machine", src)])
}

/// Codes of all diagnostics, in order.
fn codes(c: &Checked) -> Vec<&'static str> {
    c.diagnostics
        .iter()
        .flat_map(|d| d.iter().map(|x| x.code))
        .collect()
}

#[track_caller]
fn assert_clean(c: &Checked) {
    if !c.diagnostics.iter().all(|d| d.is_empty()) {
        panic!("unexpected diagnostics:\n{}", render(c));
    }
}

#[track_caller]
fn assert_codes(src: &str, expected: &[&str]) {
    let c = check_src(src);
    let got = codes(&c);
    assert_eq!(got, expected, "diagnostics:\n{}", render(&c));
}

#[test]
fn alu_sample_checks_clean() {
    let c = check_files(vec![sample("alu.machine")]);
    assert_clean(&c);
    assert!(c.skipped.is_empty());
    // Parity8 (non-generic) plus Alu<8>, Alu<16>, AluFlags<8>, AluFlags<16>.
    let mut names: Vec<String> = c
        .hir
        .units
        .iter()
        .map(|u| {
            let args: Vec<String> = u.args.iter().map(|a| a.to_string()).collect();
            format!("{}<{}>", c.program.item_name(u.def), args.join(","))
        })
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "Alu<16>",
            "Alu<8>",
            "AluFlags<16>",
            "AluFlags<8>",
            "Parity8<>"
        ]
    );
    assert_eq!(c.hir.tests.len(), 4);
    // The 16-bit ALU's raw output is 17 bits wide.
    let alu16 = c
        .hir
        .units
        .iter()
        .find(|u| u.args == vec![16] && c.program.item_name(u.def) == "Alu")
        .unwrap();
    match &alu16.ret {
        Ty::Tuple(items) => {
            assert_eq!(items[0].1, Ty::UInt(16));
            assert_eq!(items[1].1, Ty::UInt(17));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn other_samples_resolve_and_skip_unimplemented_bodies() {
    let c = check_files(vec![
        sample("alu.machine"),
        sample("counter_bus.machine"),
        sample("cpu_slice.machine"),
    ]);
    assert_clean(&c);
    let kinds: Vec<&str> = c.skipped.iter().map(|s| s.kind).collect();
    assert!(kinds.contains(&"component"));
    assert!(kinds.contains(&"bus"));
    assert!(kinds.contains(&"isa"));
    assert!(kinds.contains(&"system"));
    assert!(kinds.contains(&"conformance"));
    assert!(kinds.contains(&"vectors"));
    assert!(
        kinds.contains(&"test"),
        "targeted tests are skipped: {kinds:?}"
    );
}

#[test]
fn missing_module_for_use() {
    let c = check_files(vec![sample("cpu_slice.machine")]);
    assert!(codes(&c).contains(&"E1002"), "{}", render(&c));
}

#[test]
fn widths_must_match_exactly() {
    assert_codes("unit F(a: u8, b: u16) -> u16 { return a + b; }", &["E1005"]);
    assert_codes("unit F(a: u8) -> u16 { return a; }", &["E1005"]);
    assert_codes("unit F(a: u8) -> u16 { return a as u16; }", &[]);
    assert_codes("unit F(a: u8, b: i8) -> u8 { return a + b; }", &["E1005"]);
}

#[test]
fn literals_take_their_width_from_context() {
    assert_codes("unit F(a: u8) -> u8 { return a + 1; }", &[]);
    assert_codes("unit F(a: u8) -> u8 { return 1 + a; }", &[]);
    assert_codes("unit F(a: u8) -> u8 { return a + 256; }", &["E1006"]);
    assert_codes("unit F() -> u8 { return 255; }", &[]);
    assert_codes("unit F() -> u8 { return 256; }", &["E1006"]);
    assert_codes("unit F() -> i8 { return -128; }", &[]);
    assert_codes("unit F() -> i8 { return -129; }", &["E1006"]);
    assert_codes("unit F() -> u8 { return 0xFF_u8; }", &[]);
    assert_codes("unit F() -> u8 { return 0xFF_u16; }", &["E1005"]);
    assert_codes("unit F() -> u8 { let x = 5; return x + 1; }", &[]);
}

#[test]
fn bool_and_u1_are_distinct() {
    assert_codes("unit F(a: bool) -> u1 { return a; }", &["E1005"]);
    assert_codes("unit F(a: bool) -> u1 { return a as u1; }", &[]);
    assert_codes("unit F(a: u1) -> bool { return a as bool; }", &[]);
    assert_codes("unit F(a: u8) -> bool { return a as bool; }", &["E1011"]);
    assert_codes("unit F(a: u8) -> bool { return a; }", &["E1005"]);
    assert_codes(
        "unit F(a: u8) -> u8 { if a { return 1; } return 0; }",
        &["E1005"],
    );
    assert_codes("unit F(a: u8, b: u8) -> bool { return a == b; }", &[]);
    assert_codes("unit F(a: u8, b: u8) -> u1 { return (a == b) as u1; }", &[]);
}

#[test]
fn operators_check_their_operand_types() {
    assert_codes("unit F(a: u8) -> u8 { return -a; }", &["E1012"]);
    assert_codes("unit F(a: i8) -> i8 { return -a; }", &[]);
    assert_codes("unit F(a: bool) -> bool { return !a; }", &[]);
    assert_codes("unit F(a: u8) -> u8 { return !a; }", &["E1005"]);
    assert_codes("unit F(a: u8) -> u8 { return ~a; }", &[]);
    assert_codes("unit F(a: bool, b: bool) -> bool { return a && b; }", &[]);
    assert_codes(
        "unit F(a: u8, b: u8) -> bool { return a && b; }",
        &["E1005", "E1005"],
    );
    assert_codes("unit F(a: u8, b: u8) -> u8 { return a & b; }", &[]);
    assert_codes("unit F(a: u8, b: u4) -> u8 { return a << b; }", &[]);
    assert_codes("unit F(a: u8, b: i4) -> u8 { return a << b; }", &["E1012"]);
    assert_codes("unit F(a: u8) -> u8 { return a << 3; }", &[]);
    assert_codes("unit F(a: u8, b: u8) -> u8 { return a < b; }", &["E1005"]);
}

#[test]
fn bit_indexing_and_slicing() {
    assert_codes("unit F(a: u8) -> u1 { return a[7]; }", &[]);
    assert_codes("unit F(a: u8) -> u1 { return a[8]; }", &["E1017"]);
    assert_codes("unit F(a: u8) -> u4 { return a[7:4]; }", &[]);
    assert_codes("unit F(a: u8) -> u4 { return a[3:0]; }", &[]);
    assert_codes("unit F(a: u8) -> u4 { return a[4:0]; }", &["E1005"]);
    assert_codes("unit F(a: u8) -> u4 { return a[0:3]; }", &["E1017"]);
    assert_codes("unit F(a: u8, i: u3) -> u1 { return a[i]; }", &[]);
    assert_codes("unit F(a: [u8; 4], i: u2) -> u8 { return a[i]; }", &[]);
    assert_codes("unit F(a: [u8; 4]) -> u8 { return a[4]; }", &["E1017"]);
    assert_codes("unit F<const W: width>(a: u<W>) -> u1 { let msb = W - 1; return a[msb]; }\nunit G(x: u8) -> u1 { return F<8>(x); }", &[]);
}

#[test]
fn casts() {
    assert_codes(
        "enum E : u2 { A, B } unit F(e: E) -> u2 { return e as u2; }",
        &[],
    );
    assert_codes(
        "enum E : u2 { A, B } unit F(e: E) -> u3 { return e as u3; }",
        &["E1011"],
    );
    assert_codes(
        "enum E : u2 { A, B } unit F(x: u2) -> E { return x as E; }",
        &[],
    );
    assert_codes("unit F(a: u16) -> u8 { return a as u8; }", &[]);
    assert_codes("unit F(a: i8) -> u16 { return a as u16; }", &[]);
    assert_codes("unit F() -> i8 { return 0xFF as i8; }", &[]);
}

#[test]
fn match_exhaustiveness() {
    let e = "enum Op : u2 { A, B, C }\n";
    assert_codes(
        &format!(
            "{e}unit F(o: Op) -> u8 {{ return match o {{ Op::A => 1, Op::B => 2, Op::C => 3 }}; }}"
        ),
        &[],
    );
    assert_codes(
        &format!(
            "{e}unit F(o: Op) -> u8 {{ return match o {{ Op::A | Op::B => 1, Op::C => 3 }}; }}"
        ),
        &[],
    );
    assert_codes(
        &format!("{e}unit F(o: Op) -> u8 {{ return match o {{ Op::A => 1, Op::B => 2 }}; }}"),
        &["E1010"],
    );
    assert_codes(
        &format!("{e}unit F(o: Op) -> u8 {{ return match o {{ Op::A => 1, _ => 2 }}; }}"),
        &[],
    );
    assert_codes(
        "unit F(x: u2) -> u8 { return match x { 0 => 1, 1 => 2, 2 => 3, 3 => 4 }; }",
        &[],
    );
    assert_codes(
        "unit F(x: u2) -> u8 { return match x { 0..=2 => 1, 3 => 4 }; }",
        &[],
    );
    assert_codes(
        "unit F(x: u2) -> u8 { return match x { 0 => 1, 1 => 2, 3 => 4 }; }",
        &["E1010"],
    );
    assert_codes(
        "unit F(x: u2) -> u8 { return match x { 0 => 1, 1 => 2, 2 => 3 }; }",
        &["E1010"],
    );
    assert_codes(
        "unit F(x: u8) -> u8 { return match x { 0 => 1, _ => 2 }; }",
        &[],
    );
    assert_codes(
        "unit F(x: bool) -> u8 { return match x { true => 1, false => 2 }; }",
        &[],
    );
    assert_codes(
        "unit F(x: bool) -> u8 { return match x { true => 1 }; }",
        &["E1010"],
    );
    assert_codes(
        "unit F(x: u2) -> u8 { return match x { 4 => 1, _ => 2 }; }",
        &["E1006"],
    );
    assert_codes(
        "unit F(x: u8) -> u8 { return match x { 0 => 1, _ => unreachable(\"never\") }; }",
        &[],
    );
}

#[test]
fn match_arms_must_agree() {
    assert_codes(
        "unit F(x: bool) -> u8 { return match x { true => 1, false => 2 }; }",
        &[],
    );
    assert_codes(
        "unit F(x: bool, a: u8, b: u16) -> u8 { return match x { true => a, false => b }; }",
        &["E1005"],
    );
    assert_codes(
        "unit F(x: bool, a: u8, b: u16) -> u8 { return x ? a : b; }",
        &["E1005"],
    );
}

#[test]
fn every_path_must_return() {
    assert_codes("unit F(x: bool) -> u8 { if x { return 1; } }", &["E1015"]);
    assert_codes(
        "unit F(x: bool) -> u8 { if x { return 1; } else { return 2; } }",
        &[],
    );
    assert_codes(
        "unit F(x: bool) -> u8 { if x { return 1; } return 2; }",
        &[],
    );
    assert_codes(
        "unit F(x: bool) -> u8 { match x { true => { return 1; } false => { return 2; } } }",
        &[],
    );
    assert_codes("unit F(x: bool) -> u8 { unreachable(\"todo\"); }", &[]);
    assert_codes("unit F(x: bool) -> u8 { return 1; return 2; }", &["W1001"]);
}

#[test]
fn names_and_scopes() {
    assert_codes("unit F() -> u8 { return y; }", &["E1000"]);
    assert_codes(
        "unit F() -> u8 { let a = 1; { let b = 2; } return a; }",
        &[],
    );
    assert_codes(
        "unit F() -> u8 { if true { let b = 2_u8; } return b; }",
        &["E1000"],
    );
    assert_codes("unit F() -> u8 { return G(); }", &["E1000"]);
    assert_codes("unit F() -> u8 { return F(); }", &["E1014"]);
    assert_codes(
        "unit F() -> u8 { return G(); } unit G() -> u8 { return F(); }",
        &["E1014"],
    );
    assert_codes(
        "unit F(a: u8) -> u8 { return a; } unit F(b: u8) -> u8 { return b; }",
        &["E1001"],
    );
    assert_codes("enum E : u1 { A } unit F() -> u8 { return E; }", &["E1000"]);
    assert_codes("unit F() -> u8 { return parity; }", &["E1000"]);
}

#[test]
fn unit_calls() {
    let g = "unit G(a: u8, b: u8) -> u8 { return a + b; }\n";
    assert_codes(&format!("{g}unit F() -> u8 {{ return G(1, 2); }}"), &[]);
    assert_codes(
        &format!("{g}unit F() -> u8 {{ return G(b: 1, a: 2); }}"),
        &[],
    );
    assert_codes(&format!("{g}unit F() -> u8 {{ return G(1); }}"), &["E1007"]);
    assert_codes(
        &format!("{g}unit F() -> u8 {{ return G(1, c: 2); }}"),
        &["E1007"],
    );
    assert_codes(
        &format!("{g}unit F() -> u8 {{ return G(a: 1, a: 2); }}"),
        &["E1007"],
    );
    assert_codes(
        &format!("{g}unit F(x: u16) -> u8 {{ return G(x, 2); }}"),
        &["E1005"],
    );
    assert_codes(
        &format!("{g}unit F() -> u8 {{ return G<8>(1, 2); }}"),
        &["E1016"],
    );
    assert_codes("unit G<const W: width>(a: u<W>) -> u<W> { return a; } unit F(x: u8) -> u8 { return G(x); }", &["E1016"]);
    assert_codes("unit G<const W: width>(a: u<W>) -> u<W> { return a; } unit F(x: u8) -> u8 { return G<200>(x); }", &["E1004"]);
    assert_codes(
        "unit G<const W: width>(a: u<W>) -> u<W> { return a; }",
        &["W1000"],
    );
}

#[test]
fn bundles_tuples_and_arrays() {
    let b = "bundle P { a: u8, b: bool, c: u4 = 3 }\n";
    assert_codes(
        &format!("{b}unit F() -> P {{ return P {{ a: 1, b: true, c: 2 }}; }}"),
        &[],
    );
    assert_codes(
        &format!("{b}unit F() -> P {{ return P {{ a: 1, b: true, ..default }}; }}"),
        &[],
    );
    assert_codes(
        &format!("{b}unit F() -> P {{ return P {{ a: 1, ..default }}; }}"),
        &["E1009"],
    );
    assert_codes(
        &format!("{b}unit F() -> P {{ return P {{ a: 1, b: true, c: 2, d: 0 }}; }}"),
        &["E1008"],
    );
    assert_codes(
        &format!("{b}unit F(a: u8) -> P {{ return P {{ a, b: false, c: 1 }}; }}"),
        &[],
    );
    assert_codes(&format!("{b}unit F(p: P) -> u8 {{ return p.a; }}"), &[]);
    assert_codes(
        &format!("{b}unit F(p: P) -> u8 {{ return p.z; }}"),
        &["E1008"],
    );
    assert_codes(
        "unit F() -> (x: u8, y: bool) { return (x: 1, y: true); }",
        &[],
    );
    assert_codes(
        "unit F() -> (x: u8, y: bool) { return (y: true, x: 1); }",
        &[],
    );
    assert_codes("unit F() -> (x: u8, y: bool) { return (1, true); }", &[]);
    assert_codes(
        "unit F() -> (x: u8, y: bool) { return (x: 1, z: true); }",
        &["E1008"],
    );
    assert_codes("unit G() -> (x: u8, y: bool) { return (1, true); } unit F() -> u8 { let (a, _) = G(); return a; }", &[]);
    assert_codes(
        "unit G() -> (x: u8, y: bool) { return (1, true); } unit F() -> u8 { return G().x; }",
        &[],
    );
    assert_codes("unit G() -> (x: u8, y: bool) { return (1, true); } unit F() -> u8 { let (a, b, c) = G(); return a; }", &["E1005"]);
    assert_codes("unit F() -> [u8; 3] { return [1, 2, 3]; }", &[]);
    assert_codes("unit F() -> [u8; 3] { return [1, 2]; }", &["E1005"]);
    assert_codes("unit F() -> [u8; 3] { return [0; 3]; }", &[]);
    assert_codes(
        "unit F() -> u8 { let a = [1, 2, 3]; return a[0]; }",
        &["E1018"],
    );
    assert_codes("unit F() -> u8 { let a = [1_u8, 2, 3]; return a[0]; }", &[]);
    assert_codes(
        "unit F() -> u8 { let a: [u8; 3] = [1, 2, 3]; return a[0]; }",
        &[],
    );
}

#[test]
fn enums_and_consts() {
    assert_codes("enum E : u1 { A, B, C }", &["E1023"]);
    assert_codes("enum E : u2 { A = 1, B = 1 }", &["E1023"]);
    assert_codes("enum E : u2 { A, A }", &["E1001"]);
    assert_codes("enum E : bool { A }", &["E1003"]);
    assert_codes("enum E : u2 { A = 3, B }", &["E1023"]);
    assert_codes("const N: u8 = 300;", &["E1006"]);
    assert_codes("const N: u8 = 7; unit F() -> u8 { return N + 1; }", &[]);
    assert_codes("const N: u8 = 7; unit F() -> u<N> { return 1; }", &[]);
    assert_codes("const A: u8 = B; const B: u8 = A;", &["E1013"]);
    assert_codes(
        "const W: u8 = 200; unit F() -> u<W> { return 1; }",
        &["E1004"],
    );
}

#[test]
fn builtins() {
    assert_codes("unit F(a: u8) -> u1 { return parity(a); }", &[]);
    assert_codes("unit F(a: u8) -> u4 { return popcount(a); }", &[]);
    assert_codes("unit F(a: u8) -> u3 { return popcount(a); }", &["E1005"]);
    assert_codes("unit F(a: u8, b: u4) -> u12 { return concat(a, b); }", &[]);
    assert_codes("unit F(a: u8) -> u12 { return concat(a, 0); }", &["E1012"]);
    assert_codes("unit F(a: u8) -> u12 { return concat(a, 0_u4); }", &[]);
    assert_codes("unit F(a: u4) -> u12 { return replicate(a, 3); }", &[]);
    assert_codes(
        "unit F(a: bool) -> u8 { assert(a, \"must hold\"); return 1; }",
        &[],
    );
    assert_codes("unit F(a: u8) -> u8 { assert(a); return 1; }", &["E1005"]);
    assert_codes("unit F(a: u8) -> u8 { return sync(a); }", &["E1020"]);
}

#[test]
fn units_are_pure() {
    assert_codes("unit F(a: u8) -> u8 { a = 1; return a; }", &["E1020"]);
    assert_codes("unit F(a: u8) -> u8 { a <- 1; return a; }", &["E1020"]);
    assert_codes("unit F(a: u8) -> u8 { loop { } }", &["E1020", "E1015"]);
    assert_codes(
        "unit F(a: u8) -> u8 { for i in 0..4 { assert(a[i] == 0); } return a; }",
        &[],
    );
    assert_codes("unit F(a: u8) -> u8 { cycle { } return a; }", &["E1020"]);
}

#[test]
fn tests_without_target() {
    assert_codes(
        "unit G() -> u8 { return 3; } test \"t\" { let x = G(); expect { x == 3; x != 4; } }",
        &[],
    );
    assert_codes("test \"t\" { expect { 1; } }", &["E1005"]);
    assert_codes("test \"t\" { return 1; }", &["E1020"]);
    assert_codes("test \"t\" { given { x = 1; } }", &["E1020"]);
}

#[test]
fn imports() {
    let a = SourceFile::new(
        "a.machine",
        "enum E : u1 { X, Y } unit Twice(v: u8) -> u8 { return v + v; }",
    );
    let b = SourceFile::new(
        "b.machine",
        "use a::{E, Twice}; unit F(e: E) -> u8 { return match e { E::X => Twice(1), E::Y => 0 }; }",
    );
    let c = check_files(vec![a, b]);
    assert_clean(&c);
    let b2 = SourceFile::new("b.machine", "use a::Nope; use zzz::Q;");
    let a2 = SourceFile::new("a.machine", "enum E : u1 { X }");
    let c = check_files(vec![a2, b2]);
    assert_eq!(codes(&c), vec!["E1002", "E1002"]);
}
