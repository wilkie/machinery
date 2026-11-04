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
    // Parity8 and LowestSet (non-generic) plus Alu<8>, Alu<16>, AluFlags<8>,
    // AluFlags<16>.
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
            "LowestSet<>",
            "Parity8<>"
        ]
    );
    assert_eq!(c.hir.tests.len(), 5);
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
    assert!(!kinds.contains(&"component"), "components are checked now");
    assert!(!kinds.contains(&"bus"), "bus signatures are checked now");
    assert!(kinds.contains(&"rtl"));
    assert!(kinds.contains(&"bus protocol"));
    assert!(!kinds.contains(&"isa"), "isas are checked now");
    assert!(!kinds.contains(&"system"), "systems are checked now");
    assert!(kinds.contains(&"conformance"));
    // The component test is skipped; the two isa tests are checked.
    let skipped_tests: Vec<&str> = c
        .skipped
        .iter()
        .filter(|s| s.kind == "test")
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(skipped_tests.len(), 1, "{skipped_tests:?}");
    assert!(skipped_tests[0].contains("Timer.rtl"));
    assert_eq!(c.hir.tests.iter().filter(|t| t.isa.is_some()).count(), 2);
}

#[test]
fn cpu_slice_isa_is_fully_checked() {
    let c = check_files(vec![sample("alu.machine"), sample("cpu_slice.machine")]);
    assert_clean(&c);
    assert_eq!(c.hir.isas.len(), 1);
    let isa = &c.hir.isas[0];
    assert_eq!(isa.name, "I286Isa");
    assert_eq!(isa.spaces.len(), 2);
    assert_eq!(isa.registers.len(), 14);
    assert_eq!(isa.faults.len(), 2);
    assert_eq!(isa.fns.len(), 4);
    assert_eq!(isa.operands.len(), 3);
    assert_eq!(isa.kinds.len(), 2);
    assert_eq!(isa.instructions.len(), 2);
    let add = &isa.instructions[0];
    assert_eq!(
        add.mnemonic,
        Some(machine_sema::hir::Mnemonic::Fixed("add".into()))
    );
    assert_eq!(add.operands.len(), 2);
    // dst: rm8 comes from the fetched ModRM (step 1); src: r8 is selected by
    // its `reg` field (field index 1).
    assert!(matches!(
        add.operands[0].source,
        machine_sema::hir::BindSource::From { step: 1 }
    ));
    assert!(matches!(
        add.operands[1].source,
        machine_sema::hir::BindSource::Field { step: 1, field: 1 }
    ));
    assert_eq!(add.flags.modifies.len(), 6);
    assert!(add.semantics.is_some());
    // The ModRM tail can be Disp8 or Disp16.
    let modrm = isa.operands.iter().find(|o| o.name == "ModRM").unwrap();
    assert_eq!(modrm.tail_set.len(), 2);
    assert_eq!(modrm.size, 1);
    // FLAGS has a view onto six fields.
    let flags = isa.registers.iter().find(|r| r.name == "FLAGS").unwrap();
    assert_eq!(flags.views.len(), 1);
    assert_eq!(flags.views[0].mapping.len(), 6);
}

const ISA_PRELUDE: &str = r#"
isa T {
    space mem: addr u16 -> u8, little_endian;
    register AX: u16 { AL: u8 @ 0; AH: u8 @ 8; }
    register BX: u16 { BL: u8 @ 0; BH: u8 @ 8; }
    register F: u8 { CF: u1 @ 0; ZF: u1 @ 6; }
    mode Mode : u1 { Real, Protected }
    fault UD { vector: 6, class: Fault }
    fault GP(code: u16) { vector: 13, class: Fault, pushes_error_code: true }
    operand Imm8 { size 1; fields { value: u8 @ 0; } }
    operand ModRM { size 1; fields { mod: u2 @ 6; reg: u3 @ 3; rm: u3 @ 0; } tail { if mod == 1 { Imm8 } } }
    operand_kind r8 : u8 { select(n: u3) => match n { 0 => AL, 1 => BL, 2 => AH, _ => BH }; }
    operand_kind imm8 : u8 from i: Imm8 { read => i.value; }
    operand_kind rm8 : u8 from m: ModRM {
        is_mem => m.mod != 3;
        read => if m.mod == 3 { r8.select(m.rm) } else { mem.read_u8(m.tail.Imm8.value as u16) };
        write(v: u8) => if m.mod == 3 { r8.select(m.rm) = v; } else { mem.write_u8(m.tail.Imm8.value as u16, v); };
    }
"#;

fn isa(body: &str) -> String {
    format!("{ISA_PRELUDE}\n{body}\n}}")
}

#[test]
fn isa_prelude_is_clean() {
    assert_codes(&isa(""), &[]);
}

#[test]
fn isa_registers_are_places_and_fields_are_typed() {
    assert_codes(&isa("fn f() -> u8 { AL = 1; AX.AH = AL; return AL; }"), &[]);
    assert_codes(&isa("fn f() -> u16 { AX = AL; return AX; }"), &["E1005"]);
    assert_codes(&isa("fn f() -> u16 { AX = AL as u16; return AX; }"), &[]);
    assert_codes(
        &isa("fn f() -> u1 { F.CF = (AX == 0) as u1; return CF; }"),
        &[],
    );
    assert_codes(&isa("fn f() -> u8 { return AX.NOPE; }"), &["E1008"]);
    assert_codes(
        &isa("fn f() -> u8 { (AL + 1) = 2; return AL; }"),
        &["E1030"],
    );
    assert_codes(
        &isa("fn f() -> u8 { let x = AL; x = 2; return x; }"),
        &["E1050"],
    );
    assert_codes(
        &isa("fn f() -> u8 { mode = Mode::Protected; return (mode == Mode::Real) as u8; }"),
        &["E1011"],
    );
    assert_codes(
        &isa("fn f() -> u8 { return (mode == Mode::Real) as u1 as u8; }"),
        &[],
    );
    assert_codes(
        &isa("fn f() -> bool { mode = Mode::Protected; return mode == Mode::Real; }"),
        &[],
    );
    assert_codes(&isa("register AX: u8;"), &["E1001"]);
    assert_codes(&isa("register Q: u8 { X: u8 @ 4; }"), &["E1017"]);
}

#[test]
fn isa_views_map_bundle_fields_to_register_fields() {
    let b = "bundle Two { c: u1, z: u1 }\n";
    assert_codes(&format!("{b}{}", isa("register G: u8 { C: u1 @ 0; Z: u1 @ 1; view two: Two { c: C, z: Z } } fn f() -> Two { G.two = Two { c: 1, z: 0 }; return G.two; }")), &[]);
    assert_codes(
        &format!(
            "{b}{}",
            isa("register G: u8 { C: u1 @ 0; view two: Two { c: C } }")
        ),
        &["E1035"],
    );
    assert_codes(
        &format!(
            "{b}{}",
            isa("register G: u8 { C: u1 @ 0; W: u2 @ 1; view two: Two { c: C, z: W } }")
        ),
        &["E1035"],
    );
    assert_codes(
        &isa("register G: u8 { C: u1 @ 0; view v: u8 { c: C } }"),
        &["E1035"],
    );
}

#[test]
fn isa_spaces() {
    assert_codes(&isa("fn f() -> u8 { return mem.read_u8(0x100); }"), &[]);
    assert_codes(&isa("fn f() -> u16 { return mem.read_u16(AX); }"), &[]);
    assert_codes(
        &isa("fn f() -> u8 { mem.write_u8(0x100, AL); return mem[0x100]; }"),
        &[],
    );
    assert_codes(
        &isa("fn f() -> u8 { mem[0x100] = 5; return mem[AX]; }"),
        &[],
    );
    assert_codes(
        &isa("fn f() -> u8 { return mem.read_u8(0x10000); }"),
        &["E1006"],
    );
    assert_codes(&isa("fn f() -> u8 { return mem.read_u12(0); }"), &["E1031"]);
    assert_codes(&isa("fn f() -> u8 { return mem.peek(0); }"), &["E1031"]);
    assert_codes(&isa("fn f() -> u8 { return mem; }"), &["E1000"]);
    assert_codes(
        &isa("fn f() -> u8 { mem.write_u8(0, AX); return 0; }"),
        &["E1005"],
    );
}

#[test]
fn isa_faults_and_raise() {
    assert_codes(
        &isa("fn f() -> u8 { if AL == 0 { raise UD; } raise GP(0); }"),
        &[],
    );
    assert_codes(&isa("fn f() -> u8 { raise GP; }"), &["E1007"]);
    assert_codes(&isa("fn f() -> u8 { raise GP(1, 2); }"), &["E1007"]);
    assert_codes(&isa("fn f() -> u8 { raise AX; }"), &["E1038"]);
    assert_codes(&isa("fn f() -> u8 { raise Nope; }"), &["E1000"]);
    assert_codes(&isa("fn f() -> u8 { return GP(0); }"), &["E1038"]);
    assert_codes(&isa("fault Q { vector: 1, class: Weird }"), &["E1038"]);
    assert_codes(&isa("fault Q { vector: 1 }"), &["W1003"]);
    assert_codes("unit F() -> u8 { raise X; }", &["E1020"]);
}

#[test]
fn isa_fns() {
    assert_codes(
        &isa("fn g(a: u8) -> u8 { return a + AL; } fn f() -> u8 { return g(1) + g(a: 2); }"),
        &[],
    );
    assert_codes(
        &isa("fn g(a: u8) -> u8 { return a; } fn f() -> u8 { return g(1, 2); }"),
        &["E1007"],
    );
    assert_codes(
        &isa("fn g(a: u8) -> u8 { return a; } fn f() -> u8 { return g(AX); }"),
        &["E1005"],
    );
    assert_codes(&isa("fn g(a: u8) -> (x: u8, y: bool) { return (a, true); } fn f() -> u8 { let (x, _) = g(1); return x + g(2).x; }"), &[]);
    assert_codes(
        &isa("fn g() -> u8 { if AL == 0 { return 1; } }"),
        &["E1015"],
    );
    assert_codes(
        &isa("fn g() { AL = 1; } fn f() -> u8 { g(); return AL; }"),
        &[],
    );
    assert_codes(&isa("fn g() { return 1; }"), &["E1005"]);
    // From outside the isa.
    assert_codes(
        &format!(
            "{}\nunit U(a: u8) -> u8 {{ return a; }}",
            isa("fn g(a: u8) -> u8 { return U(a); }")
        ),
        &[],
    );
    assert_codes(
        &format!(
            "{}\ntest \"t\" on T {{ given {{ AL = 3; }} expect {{ T::g(1) == 4; }} }}",
            isa("fn g(a: u8) -> u8 { return a + AL; }")
        ),
        &[],
    );
}

#[test]
fn isa_operands_tails_and_kinds() {
    assert_codes(&isa("fn f(m: ModRM) -> u2 { return m.mod; }"), &[]);
    assert_codes(
        &isa("fn f(m: ModRM) -> u8 { return m.tail.Imm8.value; }"),
        &[],
    );
    assert_codes(
        &isa("fn f(m: ModRM) -> u8 { return m.tail.ModRM.value; }"),
        &["E1036"],
    );
    assert_codes(
        &isa("fn f(i: Imm8) -> u8 { return i.tail.Imm8.value; }"),
        &["E1036"],
    );
    assert_codes(&isa("fn f(m: ModRM) -> u8 { return m.nope; }"), &["E1008"]);
    assert_codes(
        &isa("operand Bad { size 1; fields { v: u16 @ 0; } }"),
        &["E1017"],
    );
    assert_codes(&isa("operand Bad { fields { v: u8 @ 0; } }"), &["E1036"]);
    assert_codes(&isa("operand Bad { size 1; tail { AX } }"), &["E1036"]);
    assert_codes(
        &isa("operand_kind bad : u8 { read => 1; }"),
        &["E1037", "E1037"],
    );
    assert_codes(
        &isa("operand_kind bad : u8 from m: ModRM { is_mem => false; }"),
        &["E1037"],
    );
    assert_codes(
        &isa("operand_kind bad : u8 from m: ModRM { read => 1; write(v: u16) => { AL = 1; }; }"),
        &["E1037"],
    );
    // Any other rule without parameters is a query; with them it is unknown.
    assert_codes(
        &isa("operand_kind q : u8 from m: ModRM { read => 1; frob => 2; mode => m.mod; }"),
        &[],
    );
    assert_codes(
        &isa("operand_kind bad : u8 from m: ModRM { read => 1; frob(x: u8) => 2; }"),
        &["E1037"],
    );
    // A query is pure, and named once.
    assert_codes(
        &isa("operand_kind bad : u8 from m: ModRM { read => 1; frob => mem.read_u8(0); }"),
        &["E1037"],
    );
    assert_codes(
        &isa("operand_kind bad : u8 from m: ModRM { read => 1; frob => 1; frob => 2; }"),
        &["E1001"],
    );
    assert_codes(
        &isa("operand_kind bad : u8 { select(n: u3) => AL + 1; }"),
        &["E1030"],
    );
    assert_codes(
        &isa("operand_kind bad : u8 { select(n: u3) => AX; }"),
        &["E1005"],
    );
    assert_codes(
        &isa("fn f() -> u8 { r8.select(3) = 7; return r8.select(0); }"),
        &[],
    );
    assert_codes(&isa("fn f() -> u8 { return r8.select(8); }"), &["E1006"]);
    assert_codes(&isa("fn f() -> u8 { return rm8.select(0); }"), &["E1037"]);
}

#[test]
fn isa_instructions() {
    let add = r#"
        instruction Add_rm8_r8 {
            mnemonic "add";
            encoding { opcode 0x00; ModRM { reg: r8 } }
            operands (dst: rm8, src: r8);
            flags { modifies: [CF, ZF]; }
            semantics { let a = dst; dst = a + src; F.ZF = (dst == 0) as u1; }
            timing { cycles: dst.is_mem ? 7 : 2 }
            allowed_in [Mode::Real, Mode::Protected];
        }"#;
    assert_codes(&isa(add), &[]);
    assert_codes(
        &isa(r#"instruction I { encoding { opcode 0x00; } }"#),
        &["E1032"],
    );
    assert_codes(&isa(r#"instruction I { mnemonic "i"; }"#), &["E1032"]);
    assert_codes(
        &isa(r#"instruction I { mnemonic "i"; encoding { ModRM { reg: r8 } } operands (s: r8); }"#),
        &["E1032"],
    );
    assert_codes(
        &isa(r#"instruction I { mnemonic "i"; encoding { opcode 0x100; } }"#),
        &["E1032"],
    );
    assert_codes(
        &isa(r#"instruction I { mnemonic "i"; encoding { opcode 0x00; } operands (d: rm8); }"#),
        &["E1033"],
    );
    assert_codes(
        &isa(
            r#"instruction I { mnemonic "i"; encoding { opcode 0x00; ModRM { reg: r8 } } operands (d: rm8); }"#,
        ),
        &["E1033"],
    );
    assert_codes(
        &isa(
            r#"instruction I { mnemonic "i"; encoding { opcode 0x00; ModRM { reg: r8 } } operands (s: r8, t: r8); }"#,
        ),
        &["E1033"],
    );
    assert_codes(
        &isa(
            r#"instruction I { mnemonic "i"; encoding { opcode 0x00; ModRM { reg: rm8 } } operands (d: rm8); }"#,
        ),
        &["E1033"],
    );
    assert_codes(
        &isa(
            r#"instruction I { mnemonic "i"; encoding { opcode 0x00; ModRM { mod: r8 } } operands (s: r8); }"#,
        ),
        &["E1033"],
    );
    assert_codes(
        &isa(
            r#"instruction I { mnemonic "i"; encoding { opcode 0x00; Imm8 } operands (s: imm8); semantics { s = 1; } }"#,
        ),
        &["E1037"],
    );
    assert_codes(
        &isa(
            r#"instruction I { mnemonic "i"; encoding { opcode 0x00; Imm8 } operands (s: imm8); semantics { AL = s; } }"#,
        ),
        &[],
    );
    assert_codes(
        &isa(
            r#"instruction I { mnemonic "i"; encoding { opcode 0x00; Imm8 } operands (s: imm8); flags { modifies: [AX]; } }"#,
        ),
        &["E1034"],
    );
    assert_codes(
        &isa(
            r#"instruction I { mnemonic "i"; encoding { opcode 0x00; Imm8 } operands (s: imm8); semantics { AL = s.is_mem; } }"#,
        ),
        &["E1037"],
    );
    assert_codes(
        &isa(
            r#"instruction I { mnemonic "i"; encoding { opcode 0x00; Imm8 } operands (s: imm8); allowed_in [AX]; }"#,
        ),
        &["E1005"],
    );
    assert_codes(
        &isa(
            r#"instruction I { mnemonic "i"; encoding { opcode 0x00; Imm8 } operands (s: imm8); semantics { raise GP(s as u16); } }"#,
        ),
        &[],
    );
}

#[test]
fn isa_tests() {
    let t = |body: &str| {
        format!(
            "{}\ntest \"t\" on T {{ {body} }}",
            isa(
                r#"instruction Mov_r8_imm8 { mnemonic "mov"; encoding { opcode 0xB0; ModRM { reg: r8 } Imm8 } operands (dst: r8, src: imm8); semantics { dst = src; } }"#
            )
        )
    };
    assert_codes(&t("given { AL = 1; mem[0x10] = 2; } run instruction Mov_r8_imm8 { dst: BL, src: 5 }; expect { BL == 5; AL == 1; mem[0x10] == 2; }"), &[]);
    assert_codes(
        &t("run bytes [0xB0, 0b11_000_011, 0x05]; expect { BL == 5; }"),
        &[],
    );
    assert_codes(&t("run bytes [0x1B0];"), &["E1006"]);
    assert_codes(&t("run instruction Mov_r8_imm8 { dst: BL };"), &["E1008"]);
    assert_codes(
        &t("run instruction Mov_r8_imm8 { dst: BL, src: 5, x: 1 };"),
        &["E1008"],
    );
    assert_codes(&t("run instruction Nope { };"), &["E1000"]);
    assert_codes(&t("given { AL + 1 = 2; }"), &["E1030"]);
    assert_codes(&t("given { let x = 1; }"), &["E1020"]);
    assert_codes(&t("run 4 cycles;"), &["E1020"]);
    assert_codes(&t("expect { AL; }"), &["E1005"]);
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
    // Parameters are immutable locals.
    assert_codes("unit F(a: u8) -> u8 { a = 1; return a; }", &["E1050"]);
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

// ---------------------------------------------------------------------------
// Buses and component models
// ---------------------------------------------------------------------------

#[test]
fn counter_bus_sample_model_is_checked() {
    use machine_sema::hir::{HandlerKind, WireDriver};
    let c = check_files(vec![sample("counter_bus.machine")]);
    assert_clean(&c);
    assert_eq!(c.hir.buses.len(), 1);
    let bus = &c.hir.buses[0];
    assert_eq!(bus.roles, vec!["Master".to_string(), "Slave".to_string()]);
    assert_eq!(bus.wires.len(), 6);
    assert_eq!(bus.transactions.len(), 2);
    assert_eq!(bus.signals.len(), 1);
    assert_eq!(bus.signals[0].driver, WireDriver::Role(1));
    assert_eq!(c.hir.components.len(), 2);
    let timer = c.hir.components.iter().find(|c| c.name == "Timer").unwrap();
    assert!(timer.has_rtl);
    let model = timer.model.as_ref().unwrap();
    assert_eq!(model.states.len(), 4);
    assert_eq!(model.handlers.len(), 3);
    assert!(matches!(
        model.handlers[0].kind,
        HandlerKind::Transaction { port: 2, .. }
    ));
    assert!(matches!(
        model.handlers[2].kind,
        HandlerKind::Tick { clock: Some(1) }
    ));
    let kinds: Vec<&str> = c.skipped.iter().map(|s| s.kind).collect();
    assert_eq!(kinds.iter().filter(|k| **k == "rtl").count(), 2);
}

const BUS_PRELUDE: &str = r#"
bus B {
    clock clk;
    wires {
        addr: u8 driven_by M;
        data: u8 shared;
        rd: bool driven_by M;
        ready: bool wired_and;
        irq: [bool; 4] driven_by S;
    }
    roles { M; S; }
    transaction read(addr: u8) -> (data: u8) { }
    transaction write(addr: u8, data: u8) { }
    signal interrupt(line: u2) = irq[line];
}
"#;

/// A slave device with a model body.
fn dev(model: &str) -> String {
    format!("{BUS_PRELUDE}\ncomponent D {{ ports {{ clock clk; bus io: B::S; in en: bool; out led: bool; }} model {{ {model} }} }}")
}

/// `port.clocks(n)`: a duration in the component's clock, in the bus's.
#[test]
fn port_clocks_check() {
    assert_codes(&dev("on io.read(addr: u8) -> (data: u8, wait: u8) { return (data: 0, wait: io.clocks(9) as u8); }"), &[]);
    assert_codes(&dev("state x: u8 = 0; fn f() { x = io.clocks(1, 2) as u8; }"), &["E1041"]);
}

/// `memo fn`: no value, integer arguments, no ports, and outputs that it
/// doesn't read and nothing else writes.
#[test]
fn memo_fns_check() {
    let ok = "state x: u8 = 0; mem out: [u8; 16]; memo fn draw(i: u8) { out[i & 15] = x; } fn go() { draw(1); }";
    assert_codes(&dev(ok), &[]);
    assert_codes(&dev("state x: u8 = 0; memo fn f(i: u8) -> u8 { return x; }"), &["E1057"]);
    assert_codes(&dev("state x: u8 = 0; memo fn f(i: u8) { x = x + 1; }"), &["E1057"]);
    assert_codes(&dev("mem out: [u8; 16]; memo fn f(i: u8) { out[i & 15] = if en { 1 } else { 0 }; }"), &["E1057"]);
    assert_codes(
        &dev("state x: u8 = 0; mem out: [u8; 16]; memo fn f(i: u8) { out[i & 15] = x; } on reset { out[0] = 1; }"),
        &["E1057"],
    );
    assert_codes(
        &dev("state x: u8 = 0; mem out: [u8; 16]; fn put(i: u8) { emit io.interrupt(0) = true; } memo fn f(i: u8) { put(i); }"),
        &["E1057"],
    );
}

/// `mem { direct: ... }`: a responding port, `u8` cells, a mask within the
/// memory; `wait` and `read_only` only with `direct`.
#[test]
fn direct_memories_check() {
    assert_codes(&dev("mem m: [u8; 16] { direct: io(addr & 0xF), wait: 1 };"), &[]);
    assert_codes(&dev("mem m: [u8; 16] { direct: io(addr), read_only };"), &[]);
    assert_codes(&dev("mem m: [u8; 16] { direct: io(addr & 0x1F) };"), &["E1043"]);
    assert_codes(&dev("mem m: [u16; 16] { direct: io(addr & 0xF) };"), &["E1043"]);
    assert_codes(&dev("mem m: [u8; 16] { direct: en(addr) };"), &["E1043"]);
    assert_codes(&dev("mem m: [u8; 16] { direct: io(addr + 1) };"), &["E1043"]);
    assert_codes(&dev("mem m: [u8; 16] { wait: 1 };"), &["E1043"]);
    assert_codes(&master("mem m: [u8; 16] { direct: io(addr) };"), &["E1043"]);
}

/// A master device with a model body.
fn master(model: &str) -> String {
    format!("{BUS_PRELUDE}\ncomponent M1 {{ ports {{ clock clk; bus io: B::M; }} model {{ state x: u8 = 0; {model} }} }}")
}

#[test]
fn model_bodies_check() {
    assert_codes(&dev(""), &[]);
    assert_codes(
        &dev(r#"
            state x: u8 = 0;
            mem buf: [u8; 16];
            on io.read(addr: u8) -> (data: u8, wait: u8) { return (data: x + buf[addr[3:0]], wait: 0); }
            on io.write(addr: u8, data: u8) -> (wait: u8) { x = data; buf[0] = data; led = en; emit io.interrupt(0) = true; return (wait: 1); }
            on tick(n: u32) { while x != 0 { x = x - 1; if x == 3 { break; } } }
            on reset { x = 0; }
        "#),
        &[],
    );
    assert_codes(&dev("state x: u8 = true;"), &["E1005"]);
    assert_codes(&dev("state x: u8 = 0; x = 1;"), &["E1043"]);
    assert_codes(&dev("state x: u8 = 0; state x: u8 = 1;"), &["E1001"]);
    assert_codes(&dev("state en: u8 = 0;"), &["E1001"]);
    assert_codes(&dev("on reset { en = true; }"), &["E1030"]);
    assert_codes(&dev("on reset { led = en; }"), &[]);
    assert_codes(&dev("on reset { let c = clk; }"), &["E1041"]);
    assert_codes(&dev("on reset { break; }"), &["E1020"]);
    assert_codes(&dev("on reset { loop { } }"), &[]);
    assert_codes(&dev("on reset { let v = io.read(1); }"), &["E1041"]);
    assert_codes(
        &format!("{BUS_PRELUDE}\ncomponent E {{ ports {{ }} }}"),
        &["W1004"],
    );
}

#[test]
fn model_handler_signatures_follow_the_protocol() {
    assert_codes(
        &dev("on io.read(a: u8) -> (data: u8, wait: u8) { return (data: 0, wait: 0); }"),
        &["E1041"],
    );
    assert_codes(
        &dev("on io.read(addr: u16) -> (data: u8, wait: u8) { return (data: 0, wait: 0); }"),
        &["E1041"],
    );
    assert_codes(
        &dev("on io.read(addr: u8) -> (data: u8) { return (data: 0); }"),
        &["E1041"],
    );
    assert_codes(&dev("on io.read(addr: u8) { }"), &["E1041"]);
    assert_codes(&dev("on io.read(addr: u8) -> (data: u8, wait: u8) { if addr == 0 { return (data: 0, wait: 0); } }"), &["E1015"]);
    assert_codes(&dev("on io.nope(addr: u8) { }"), &["E1041"]);
    assert_codes(&dev("on led.read(addr: u8) { }"), &["E1041"]);
    assert_codes(
        &dev("on io.interrupt(line: u2, value: bool) { }"),
        &["E1041"],
    );
    assert_codes(&dev("on tick(n: u32) -> u8 { return 1; }"), &["E1041"]);
    assert_codes(&dev("on tick(n: u8) { }"), &["E1041"]);
    assert_codes(&dev("on tick @ nope (n: u32) { }"), &["E1041"]);
    assert_codes(&dev("on tick @ clk (n: u32) { }"), &[]);
    assert_codes(&dev("on frob { }"), &["E1041"]);
    assert_codes(&dev("on reset(n: u32) { }"), &["E1041"]);
    // Two clocks: `@` is required.
    assert_codes(
        &format!("{BUS_PRELUDE}\ncomponent T {{ ports {{ clock a; clock b; }} model {{ on tick(n: u32) {{ }} }} }}"),
        &["E1041"],
    );
    assert_codes(
        &format!("{BUS_PRELUDE}\ncomponent T {{ ports {{ clock a; clock b; }} model {{ on tick @ b (n: u32) {{ }} }} }}"),
        &[],
    );
    // No clock port: implicit clock.
    assert_codes(
        &format!("{BUS_PRELUDE}\ncomponent T {{ ports {{ }} model {{ on tick(n: u32) {{ }} }} }}"),
        &[],
    );
}

#[test]
fn model_masters_initiate_and_slaves_respond() {
    assert_codes(
        &master("on tick(n: u32) { let v = io.read(1); x = v.data; io.write(1, x); }"),
        &[],
    );
    assert_codes(
        &master("on tick(n: u32) { let v = io.read(1, 2); }"),
        &["E1007"],
    );
    assert_codes(&master("on tick(n: u32) { x = io.read(1); }"), &["E1005"]);
    assert_codes(
        &master("on io.read(addr: u8) -> (data: u8, wait: u8) { return (data: 0, wait: 0); }"),
        &["E1041"],
    );
    assert_codes(
        &master("on io.interrupt(line: u2, value: bool) { x = line as u8; }"),
        &[],
    );
    assert_codes(
        &master("on tick(n: u32) { emit io.interrupt(0) = true; }"),
        &["E1044"],
    );
    // Calling a signal reads its level (here the block's value).
    assert_codes(&master("on tick(n: u32) { io.interrupt(0); }"), &[]);
    assert_codes(
        &master("on tick(n: u32) { let f = io.fields; }"),
        &["E1041"],
    );
}

#[test]
fn emit_sets_signals_the_role_drives() {
    assert_codes(&dev("on reset { emit io.interrupt(1) = false; }"), &[]);
    assert_codes(&dev("on reset { emit io.interrupt(1); }"), &["E1044"]);
    assert_codes(&dev("on reset { emit io.interrupt(1) = 1; }"), &["E1005"]);
    assert_codes(
        &dev("on reset { emit io.interrupt(4) = true; }"),
        &["E1006"],
    );
    assert_codes(&dev("on reset { emit io.nope(1) = true; }"), &["E1044"]);
    assert_codes(&dev("on reset { emit led(1) = true; }"), &["E1044"]);
    assert_codes("unit F() -> u8 { emit x(1) = true; }", &["E1020", "E1015"]);
}

#[test]
fn bus_declarations() {
    assert_codes("bus B { wires { a: u8 driven_by M; } }", &["E1040"]);
    assert_codes(
        "bus B { roles { M; } wires { a: u8 driven_by Z; } }",
        &["E1040"],
    );
    assert_codes("bus B { roles { M; } wires { a: u8; } }", &["E1040"]);
    assert_codes(
        "bus B { roles { M; } wires { a: u8 shared driven_by M; } }",
        &["E1040"],
    );
    assert_codes(
        "bus B { roles { M; } wires { a: u8 frobnicated; } }",
        &["E1040"],
    );
    assert_codes(
        "bus B { roles { M; } wires { a: [u8; 2] driven_by M; } }",
        &[],
    );
    assert_codes(
        "bus B { roles { M; S; } transaction r(a: u8) -> u8 { } }",
        &["E1040"],
    );
    assert_codes(
        "bus B { roles { M; S; } transaction r(a: u8) -> (v: u8) { Q { } } }",
        &["E1040"],
    );
    assert_codes(
        "bus B { roles { M; S; } transaction r(a: u8) -> (v: u8) { } transaction r(b: u8) { } }",
        &["E1001"],
    );
    assert_codes(
        "bus B { roles { M; S; } wires { x: bool driven_by S; } signal s() = true; }",
        &["E1040"],
    );
    assert_codes(
        "bus B { roles { M; S; } wires { x: u8 driven_by S; } signal s() = x; }",
        &["E1005"],
    );
    assert_codes(
        "bus B { roles { M; S; } wires { x: bool driven_by S; } signal s() = x; }",
        &[],
    );
    assert_codes("bus B { roles { M; S; } enum Sp : u1 { Mem, Io } transaction r(a: u8, s: Sp) -> (v: u8) { } }", &[]);
    assert_codes("bus B { roles { M; M; } }", &["E1001"]);
}

#[test]
fn component_ports() {
    assert_codes(
        &format!("{BUS_PRELUDE}\ncomponent C {{ ports {{ bus io: B::Nope; }} }}"),
        &["E1042"],
    );
    assert_codes(
        &format!("{BUS_PRELUDE}\ncomponent C {{ ports {{ bus io: u8; }} }}"),
        &["E1042"],
    );
    assert_codes(
        &format!("{BUS_PRELUDE}\ncomponent C {{ ports {{ bus io: Nope::M; }} }}"),
        &["E1000"],
    );
    assert_codes(
        &format!("{BUS_PRELUDE}\ncomponent C {{ ports {{ in a: u8; in a: u8; }} }}"),
        &["E1001"],
    );
    assert_codes(
        &format!("{BUS_PRELUDE}\ncomponent C {{ isa Nope; ports {{ }} }}"),
        &["E1000"],
    );
    assert_codes(
        &format!("{BUS_PRELUDE}\ncomponent C<const W: width> {{ ports {{ }} }}"),
        &["E1042"],
    );
}

// ---------------------------------------------------------------------------
// Decoding features: patterns, value kinds, constraints, prefixes, latches
// ---------------------------------------------------------------------------

const DEC_PRELUDE: &str = r#"
isa T {
    space mem: addr u16 -> u8, little_endian;
    register AX: u16 { AL: u8 @ 0; AH: u8 @ 8; }
    register F: u8 { CF: u1 @ 0; AF: u1 @ 4; }
    register IP: u16;
    enum Op : u2 { Add, Or, Sub, Cmp }
    enum Seg : u2 { A, B, C, None }
    latch seg: Seg = Seg::None;
    fault UD { vector: 6, class: Fault }
    fn code_address() -> u16 { return IP; }
    fn next_byte() -> u8 { let b = mem.read_u8(IP); IP = IP + 1; return b; }
    istream { space mem; address code_address; next next_byte; undefined UD; }
    operand ModRM { size 1; fields { mod: u2 @ 6; reg: u3 @ 3; rm: u3 @ 0; } }
    operand Imm8 { size 1; fields { value: u8 @ 0; } }
    operand_kind r8 : u8 { select(n: u3) => match n { 0 => AL, 1 => AH, _ => AL }; }
    operand_kind op : Op { value(n: u2) => n as Op; }
    operand_kind imm8 : u8 from i: Imm8 { read => i.value; }
    fn name(o: Op) -> str { return match o { Op::Add => "add", Op::Or => "or", Op::Sub => "sub", Op::Cmp => "cmp" }; }
"#;

fn dec(body: &str) -> String {
    format!("{DEC_PRELUDE}\n{body}\n}}")
}

#[test]
fn decoding_prelude_is_clean() {
    assert_codes(&dec(""), &[]);
}

#[test]
fn opcode_patterns_and_value_kinds() {
    let ok = r#"instruction AluI { mnemonic name(o); encoding { opcode "0100oo00" { o: op }; Imm8 } operands (o: op, src: imm8);
        flags { modifies: [CF]; undefined: [AF if o == Op::Or]; } semantics { if o != Op::Cmp { AL = AL + src; } } }"#;
    assert_codes(&dec(ok), &[]);
    // Field width must match the kind's parameter.
    assert_codes(
        &dec(
            r#"instruction I { mnemonic "i"; encoding { opcode "0100ooo0" { o: op } } operands (o: op); }"#,
        ),
        &["E1033"],
    );
    // Every field must be bound; `-` is don't-care.
    assert_codes(
        &dec(r#"instruction I { mnemonic "i"; encoding { opcode "0100oo00" } }"#),
        &["E1032"],
    );
    assert_codes(
        &dec(r#"instruction I { mnemonic "i"; encoding { opcode "0100--00" } }"#),
        &[],
    );
    // Patterns are whole bytes, fields contiguous, characters known.
    assert_codes(
        &dec(r#"instruction I { mnemonic "i"; encoding { opcode "0100" } }"#),
        &["E1032"],
    );
    assert_codes(
        &dec(
            r#"instruction I { mnemonic "i"; encoding { opcode "o100oo00" { o: op } } operands (o: op); }"#,
        ),
        &["E1032"],
    );
    assert_codes(
        &dec(r#"instruction I { mnemonic "i"; encoding { opcode "0100?000" } }"#),
        &["E1032"],
    );
    // Value-kind operands are read-only.
    assert_codes(
        &dec(
            r#"instruction I { mnemonic "i"; encoding { opcode "0100oo00" { o: op } } operands (o: op); semantics { o = Op::Add; } }"#,
        ),
        &["E1037"],
    );
    // A kind has one of select/value.
    assert_codes(
        &dec("operand_kind bad : u8 { value(n: u3) => n as u8; select(n: u3) => AL; }"),
        &["E1037"],
    );
    assert_codes(
        &dec("operand_kind bad : u8 { value(n: u8, m: u8) => n; }"),
        &["E1037"],
    );
    assert_codes(
        &dec("operand_kind bad : u8 { value(n: bool) => 1; }"),
        &["E1037"],
    );
    // Mnemonic expressions are strings.
    assert_codes(
        &dec(r#"instruction I { mnemonic 3; encoding { opcode 0x40; } }"#),
        &["E1005"],
    );
}

#[test]
fn field_constraints_and_overlap() {
    let pair = r#"
        instruction A { mnemonic "a"; encoding { opcode 0xC6; ModRM { reg: 0 } } }
        instruction B { mnemonic "b"; encoding { opcode 0xC6; ModRM { reg: 1 } } }"#;
    assert_codes(&dec(pair), &[]);
    let clash = r#"
        instruction A { mnemonic "a"; encoding { opcode 0xC6; ModRM { reg: 0 } } }
        instruction B { mnemonic "b"; encoding { opcode 0xC6; ModRM { reg: 0, mod: 3 } } }"#;
    assert_codes(&dec(clash), &["E1045"]);
    let unconstrained = r#"
        instruction A { mnemonic "a"; encoding { opcode 0xC6; ModRM { reg: 0 } } }
        instruction B { mnemonic "b"; encoding { opcode 0xC6; ModRM } }"#;
    assert_codes(&dec(unconstrained), &["E1045"]);
    // A pattern that fixes more bits wins on the bytes it matches (SPEC
    // 11.2d): 48h refines 0100oo00.
    let refinement = r#"
        instruction A { mnemonic "a"; encoding { opcode "0100oo00" { o: op } } operands (o: op); }
        instruction B { mnemonic "b"; encoding { opcode 0x48; } }"#;
    assert_codes(&dec(refinement), &[]);
    // Neither refines the other where they meet (48h)...
    let partial = r#"
        instruction A { mnemonic "a"; encoding { opcode "0100oo00" { o: op } } operands (o: op); }
        instruction B { mnemonic "b"; encoding { opcode "010010--"; } }"#;
    assert_codes(&dec(partial), &["E1045"]);
    // ...unless an instruction more specific than both covers it.
    let covered = r#"
        instruction A { mnemonic "a"; encoding { opcode "0100oo00" { o: op } } operands (o: op); }
        instruction B { mnemonic "b"; encoding { opcode "010010--"; } }
        instruction C { mnemonic "c"; encoding { opcode 0x48; } }"#;
    assert_codes(&dec(covered), &[]);
    assert_codes(
        &dec(r#"instruction A { mnemonic "a"; encoding { opcode 0xC6; ModRM { reg: 9 } } }"#),
        &["E1006"],
    );
}

#[test]
fn prefixes_latches_and_halt() {
    let ok = r#"
        prefix P { encoding { opcode "001ss110" { s: segk } } operands (s: segk); semantics { seg = s; } }
        operand_kind segk : Seg { value(n: u2) => n as Seg; }
        instruction H { mnemonic "hlt"; encoding { opcode 0xF4; } semantics { if seg == Seg::A { halt; } } }"#;
    assert_codes(&dec(ok), &[]);
    // Prefixes fetch no operands; they need no mnemonic.
    assert_codes(
        &dec(r#"prefix P { encoding { opcode 0xF0; Imm8 } }"#),
        &["E1048"],
    );
    // Latches are initialised at every instruction boundary: they may read
    // state but not call functions or touch memory.
    assert_codes(&dec("latch start: u16 = IP;"), &[]);
    assert_codes(&dec("latch bad: u8 = mem[0];"), &["E1047"]);
    assert_codes(&dec("latch bad: u8 = IP;"), &["E1005"]);
    // Prefixes share the opcode space with instructions.
    assert_codes(
        &dec(
            r#"prefix P { encoding { opcode 0xF0; } } instruction I { mnemonic "i"; encoding { opcode 0xF0; } }"#,
        ),
        &["E1045"],
    );
    // `halt` only in isa code.
    assert_codes("unit F() -> u8 { halt; return 0; }", &["E1020"]);
}

#[test]
fn vectors_declarations() {
    let isa = dec("");
    let v = |body: &str| format!("{isa}\nvectors \"x/*.MOO.gz\" on T {{ {body} }}");
    assert_codes(&v("format moo; memory mem; reg ax = AX; reg ip = IP;"), &[]);
    assert_codes(
        &v("format moo; memory mem; reg ax { load => AX = value + 1; read => AX - 1; }"),
        &[],
    );
    assert_codes(&v("format xml; memory mem;"), &["E1049"]);
    // Clocks a transfer takes: two for MOO traces unless said, 1 to 255.
    let c = check_src(&v("format moo; memory mem;"));
    assert_eq!(c.hir.vectors[0].transfer_cycles, 2);
    let c = check_src(&v("format json; memory mem; transfer_cycles 3;"));
    assert_eq!(c.hir.vectors[0].transfer_cycles, 3);
    assert_codes(&v("format json; memory mem; transfer_cycles 0;"), &["E1049"]);
    // JSON vectors name their registers freely.
    assert_codes(&v("format json; memory mem; reg zz = AX;"), &[]);
    assert_codes(&v("format moo;"), &["E1049"]);
    assert_codes(&v("memory mem;"), &["E1049"]);
    assert_codes(&v("format moo; memory nope;"), &["E1049", "E1049"]);
    assert_codes(&v("format moo; memory mem; reg zz = AX;"), &["E1049"]);
    assert_codes(
        &v("format moo; memory mem; reg ax = AX; reg ax = AX;"),
        &["E1001"],
    );
    assert_codes(&v("format moo; memory mem; reg ax = AL;"), &["E1005"]);
    assert_codes(&format!("{isa}\nvectors \"x\" on T;"), &["E1049"]);
    assert_codes(
        &format!("{isa}\nvectors \"x\" on Nope {{ format moo; }}"),
        &["E1049"],
    );
}

#[test]
fn i286_target_checks_clean() {
    let root: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "targets", "i286"]
        .iter()
        .collect();
    let files = ["alu.machine", "i286.machine"]
        .iter()
        .map(|f| SourceFile::new(*f, std::fs::read_to_string(root.join(f)).unwrap()))
        .collect();
    let c = check_files(files);
    assert_clean(&c);
    let isa = &c.hir.isas[0];
    assert_eq!(isa.instructions.iter().filter(|i| i.prefix).count(), 3);
    assert_eq!(isa.instructions.iter().filter(|i| !i.prefix).count(), 162);
    assert_eq!(isa.latches.len(), 4);
    assert_eq!(c.hir.vectors.len(), 1);
    assert_eq!(c.hir.vectors[0].regs.len(), 14);
}

#[test]
fn istream_fault_delivery_fn() {
    let good = DEC_PRELUDE.replace(
        "undefined UD; }",
        "undefined UD; fault deliver; }\n    fn deliver(v: u8, restart: bool) { IP = v as u16; }",
    );
    assert_codes(&format!("{good}\n}}"), &[]);
    let bad_sig = DEC_PRELUDE.replace(
        "undefined UD; }",
        "undefined UD; fault deliver; }\n    fn deliver(v: u8) { IP = v as u16; }",
    );
    assert_codes(&format!("{bad_sig}\n}}"), &["E1046"]);
    let not_fn = DEC_PRELUDE.replace("undefined UD; }", "undefined UD; fault AX; }");
    assert_codes(&format!("{not_fn}\n}}"), &["E1046"]);
}

#[test]
fn counter_bus_system_resolves_clocks_and_bindings() {
    let c = check_files(vec![sample("counter_bus.machine")]);
    assert_clean(&c);
    let sys = &c.hir.systems[0];
    assert_eq!(sys.name, "TimerBoard");
    // 8 MHz and 2 MHz: the time unit is 125 ns.
    assert_eq!((sys.base_num, sys.base_den), (1, 8_000_000));
    let periods: Vec<u64> = sys.clocks.iter().map(|c| c.period).collect();
    assert_eq!(periods, [1, 4]);
    let timer = &sys.components[1];
    assert_eq!(timer.name, "timer");
    // `@ (bus_clk: bus_clk, tick: tick)`: ports 0 and 1.
    assert_eq!(timer.clocks, [(Some(0), 0), (Some(1), 1)]);
    let bus = &sys.buses[0];
    assert_eq!(bus.masters, [(0, 0)]);
    assert_eq!(bus.slaves.len(), 1);
    assert_eq!((bus.slaves[0].1[0].lo, bus.slaves[0].1[0].hi), (0x40, 0x43));
    assert_eq!(sys.profiles.len(), 2);
}

/// A bus whose transactions have a derivable base cost and an address
/// space, and two devices for systems.
const SYS_PRELUDE: &str = r#"
enum Sp : u1 { Mem, Io }
bus B {
    clock clk;
    wires { addr: u8 driven_by M; data: u8 shared; ready: bool wired_and; irq: [bool; 2] driven_by S; }
    roles { M; S; }
    transaction read(space: Sp, addr: u8) -> (data: u8) { M { cycle { } wait until ready; cycle { } } }
    transaction write(space: Sp, addr: u8, data: u8) { M { cycle { } if true { cycle { } } } }
    signal interrupt(line: u1) = irq[line];
}
component Host { ports { bus io: B::M; } model { } }
component Dev { ports { bus io: B::S; } model {
    state n: u32 = 0;
    on io.read(space: Sp, addr: u8) -> (data: u8, wait: u8) { return (data: addr, wait: 0); }
    on tick(k: u32) { n = n + k; }
} }
"#;

fn system(body: &str) -> String {
    format!("{SYS_PRELUDE}\nsystem S {{ {body} }}")
}

#[test]
fn transaction_base_cycles_count_the_initiator_cycles() {
    let c = check_src(&system(""));
    assert_clean(&c);
    let bus = &c.hir.buses[0];
    assert_eq!(bus.transactions[0].base_cycles, Some(2));
    // A branch makes the count data-dependent.
    assert_eq!(bus.transactions[1].base_cycles, None);
}

#[test]
fn systems_check() {
    let good = "clock a: 4 MHz; clock b = a / 4; component h: Host; component d: Dev @ b;
        bus x: B @ a { master h.io; slave d.io @ mem 0x00..=0x0F, io 0x10..0x20; }
        profile fast { default: model; }";
    let c = check_src(&system(good));
    assert_clean(&c);
    let sys = &c.hir.systems[0];
    assert_eq!(
        sys.clocks.iter().map(|c| c.period).collect::<Vec<_>>(),
        [1, 4]
    );
    let ranges = &sys.buses[0].slaves[0].1;
    assert_eq!(ranges[0].space, Some(0));
    assert_eq!(
        (ranges[1].space, ranges[1].lo, ranges[1].hi),
        (Some(1), 0x10, 0x1F)
    );

    // A 14.31818 MHz crystal and 1/12 of it share an exact time unit.
    let c = check_src(&system(
        "clock osc: 14.31818 MHz; component d: Dev @ (osc / 12);",
    ));
    assert_clean(&c);
    let sys = &c.hir.systems[0];
    assert_eq!(sys.clocks[1].period, 12 * sys.clocks[0].period);

    // A clock used before it is declared.
    assert_codes(&system("clock b = a / 2; clock a: 1 MHz;"), &["E1049"]);
    // A device with `on tick` needs its clock bound.
    assert_codes(&system("component d: Dev;"), &["E1049"]);
    // Overlapping slaves.
    assert_codes(
        &system(
            "clock a: 1 MHz; component d: Dev @ a; component e: Dev @ a;
            bus x: B @ a { slave d.io @ 0..=7; slave e.io @ 4..=9; }",
        ),
        &["E1049"],
    );
    // The same address in different spaces does not overlap.
    assert_codes(
        &system(
            "clock a: 1 MHz; component d: Dev @ a; component e: Dev @ a;
            bus x: B @ a { slave d.io @ mem 0..=7; slave e.io @ io 0..=7; }",
        ),
        &[],
    );
    // A responding port attached as master.
    assert_codes(
        &system("clock a: 1 MHz; component d: Dev @ a; bus x: B @ a { master d.io; }"),
        &["E1049"],
    );
    // An unknown space name.
    assert_codes(
        &system("clock a: 1 MHz; component d: Dev @ a; bus x: B @ a { slave d.io @ rom 0..=7; }"),
        &["E1049"],
    );
    // An unknown level.
    assert_codes(&system("profile p { default: fast; }"), &["E1049"]);
}

#[test]
fn transaction_handlers_may_bridge_to_their_master_ports() {
    // A bridge: the handler for `up` runs a transaction on `down`.
    let src = format!(
        "{SYS_PRELUDE}\ncomponent Bridge {{ ports {{ bus up: B::S; bus down: B::M; }} model {{
            on up.read(space: Sp, addr: u8) -> (data: u8, wait: u8) {{
                let r = down.read(space: space, addr: addr);
                return (data: r.data, wait: 1);
            }}
        }} }}"
    );
    assert_codes(&src, &[]);
}

#[test]
fn processor_models_bind_isa_spaces() {
    let cpu = |model: &str| {
        format!(
            "{}\n{SYS_PRELUDE}\ncomponent Cpu {{ isa T; ports {{ bus io: B::M; }} model {{ {model} }} }}",
            isa("")
        )
    };
    let read = "on mem.read_u8(addr: u16) -> u8 { return io.read(space: Sp::Mem, addr: addr as u8).data; }";
    let write = "on mem.write_u8(addr: u16, data: u8) { io.write(space: Sp::Mem, addr: addr as u8, data: data); }";
    assert_codes(&cpu(&format!("{read} {write}")), &[]);
    // A wider access may have its own handler.
    assert_codes(
        &cpu(&format!(
            "{read} {write} on mem.read_u16(addr: u16) -> u16 {{ return 0; }}"
        )),
        &[],
    );
    // Writes unbound.
    assert_codes(&cpu(read), &["E1041"]);
    // Not a multiple of the cell.
    assert_codes(
        &cpu(&format!(
            "{read} {write} on mem.read_u12(addr: u16) -> u12 {{ return 0; }}"
        )),
        &["E1041"],
    );
    // The wrong address type.
    assert_codes(
        &cpu(&format!(
            "{write} on mem.read_u8(addr: u8) -> u8 {{ return 0; }}"
        )),
        &["E1041", "E1041"],
    );
    // A processor model may own its timing (SPEC 11.2c).
    assert_codes(
        &cpu(&format!(
            "{read} {write} state n: u32 = 0; on retire(s: Schedule) -> u32 {{ n = s.work[0] as u32 + s.length as u32; return n; }}"
        )),
        &[],
    );
    assert_codes(
        &cpu(&format!(
            "{read} {write} on retire @ clk (s: Schedule) -> u32 {{ return 0; }}"
        )),
        &["E1041"],
    );
}

#[test]
fn only_a_processor_model_retires() {
    assert_codes(
        &dev("on retire(s: Schedule) -> u32 { return 0; }"),
        &["E1041"],
    );
}

#[test]
fn isa_reset_fn_takes_nothing() {
    assert_codes(&isa("fn reset() { AX = 0; }"), &[]);
    assert_codes(&isa("fn reset(v: u16) { AX = v; }"), &["E1046"]);
}

#[test]
fn i286_board_system_checks_clean() {
    let targets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("targets");
    let files = [
        ("pc", "isabus.machine"),
        ("pc", "crtc.machine"),
        ("pc", "pc.machine"),
        ("i286", "alu.machine"),
        ("i286", "i286.machine"),
        ("i286", "board.machine"),
    ]
    .iter()
    .map(|(d, f)| SourceFile::new(*f, std::fs::read_to_string(targets.join(d).join(f)).unwrap()))
    .collect();
    let c = check_files(files);
    assert_clean(&c);
    let sys = &c.hir.systems[0];
    assert_eq!(sys.components.len(), 12);
    let local = sys.buses.iter().find(|b| b.name == "local").unwrap();
    let isa = sys.buses.iter().find(|b| b.name == "isa").unwrap();
    // RAM, ROM and the bus controller on the local bus, which the processor
    // and the bus controller (for ISA masters) master.
    assert_eq!(local.slaves.len(), 3);
    assert_eq!(local.masters.len(), 2);
    // The ROM answers at the top of both the 1 MB and the 16 MB space.
    assert_eq!(local.slaves[1].1.len(), 2);
    // On the ISA bus: system memory through the bus controller, both PICs,
    // PIT, the DMA registers and pages, the link, the console, the CGA and
    // the Hercules; mastered by the bus controller and the DMA controller.
    assert_eq!(isa.slaves.len(), 10);
    assert_eq!(isa.masters.len(), 2);
    // SP straps, CAS, INTR, the CGA's font jumper, and A20 tied high.
    assert_eq!(sys.wires.len(), 6);
}

#[test]
fn extern_fns_and_the_boundary_hook() {
    let src = DEC_PRELUDE.replace(
        "undefined UD; }",
        "undefined UD; boundary take; }\n    extern fn line() -> bool;\n    extern fn ack(n: u8) -> u8;\n    fn take() -> bool { if line() { IP = ack(1) as u16; return true; } return false; }",
    );
    let c = check_src(&format!("{src}\n}}"));
    assert_clean(&c);
    let isa = &c.hir.isas[0];
    assert_eq!(isa.fns.iter().filter(|f| f.external).count(), 2);
    assert!(isa.istream.as_ref().unwrap().boundary.is_some());
    // The boundary fn returns whether it took an interrupt.
    let bad = DEC_PRELUDE.replace(
        "undefined UD; }",
        "undefined UD; boundary take; }\n    fn take() { }",
    );
    assert_codes(&format!("{bad}\n}}"), &["E1046"]);
    // The fault hook may take the fault's error code (SPEC 11.1a).
    let src = DEC_PRELUDE.replace(
        "undefined UD; }",
        "undefined UD; fault deliver; }\n    fn deliver(v: u8, restart: bool, has_error: bool, error: u16) { IP = error; }",
    );
    assert_clean(&check_src(&format!("{src}\n}}")));
    let bad = DEC_PRELUDE.replace(
        "undefined UD; }",
        "undefined UD; fault deliver; }\n    fn deliver(v: u8, restart: bool, error: u16) { }",
    );
    assert_codes(&format!("{bad}\n}}"), &["E1046"]);
    // The limit fn says how far the stream reaches (SPEC 11.1a).
    let src = DEC_PRELUDE.replace(
        "undefined UD; }",
        "undefined UD; limit room; }\n    fn room() -> u32 { return 0x10000 - (IP as u32); }",
    );
    let c = check_src(&format!("{src}\n}}"));
    assert_clean(&c);
    assert!(c.hir.isas[0].istream.as_ref().unwrap().limit.is_some());
    let bad = DEC_PRELUDE.replace(
        "undefined UD; }",
        "undefined UD; limit room; }\n    fn room() -> u16 { return 1; }",
    );
    assert_codes(&format!("{bad}\n}}"), &["E1046"]);
    // An extern fn has no body.
    let bad = DEC_PRELUDE.replace(
        "undefined UD; }",
        "undefined UD; }\n    extern fn line() -> bool { return true; }",
    );
    assert_eq!(codes(&check_src(&format!("{bad}\n}}"))), ["E0100"]);
}

#[test]
fn processor_models_supply_extern_fns() {
    let cpu = |model: &str| {
        format!(
            "{}\n{SYS_PRELUDE}\ncomponent Cpu {{ isa T; ports {{ bus io: B::M; }} model {{
                on mem.read_u8(addr: u16) -> u8 {{ return 0; }}
                on mem.write_u8(addr: u16, data: u8) {{ }}
                {model} }} }}",
            isa("extern fn line() -> bool;")
        )
    };
    assert_codes(&cpu("on line() -> bool { return true; }"), &[]);
    // Missing.
    assert_codes(&cpu(""), &["E1041"]);
    // The wrong return type.
    assert_codes(&cpu("on line() -> u8 { return 1; }"), &["E1041", "E1041"]);
}

/// A transaction without `addr` (an interrupt acknowledge) is not decoded,
/// so ranged slaves do not need it to have one.
#[test]
fn transactions_without_addr_are_not_decoded() {
    let src = SYS_PRELUDE.replace(
        "    signal interrupt",
        "    transaction ack() -> (vector: u8) { M { cycle { } } }\n    signal interrupt",
    );
    let c = check_src(&format!(
        "{src}\nsystem S {{ clock a: 1 MHz; component d: Dev @ a; bus x: B @ a {{ slave d.io @ mem 0..=7; }} }}"
    ));
    assert_clean(&c);
}

// --- mutable locals (SPEC 6.1) and places (SPEC 6.2) ---

fn unit_src(body: &str) -> String {
    format!("bundle F {{ cf: u1, zf: u1 }}\nunit U(v: u8, c: bool) -> u8 {{ {body} }}")
}

#[test]
fn mutable_locals_accumulate_in_units() {
    assert_codes(
        &unit_src("let mut n: u8 = 0; for i in 0..8 { if v[i] == 1 { n = n + 1; } } return n;"),
        &[],
    );
    // The type comes from the value when it is not a bare literal.
    assert_codes(&unit_src("let mut n = v; n = n ^ 0xFF; return n;"), &[]);
    // Fields and elements of a mutable local are assignable.
    assert_codes(
        &unit_src("let mut f: F = F { cf: 0, zf: 0 }; f.cf = 1; let mut a: [u8; 4] = [0_u8; 4]; a[v[1:0]] = 3; return a[0] + f.cf as u8;"),
        &[],
    );
    let c = check_src(&unit_src("let mut n: u8 = 0; n = v; return n;"));
    assert_clean(&c);
    let local = c.hir.units[0]
        .locals
        .iter()
        .find(|l| l.name == "n")
        .unwrap();
    assert!(local.mutable);
}

#[test]
fn mutable_local_errors() {
    // Assigning an immutable local.
    assert_codes(&unit_src("let n: u8 = 0; n = 1; return n;"), &["E1050"]);
    // A bare literal gives no type.
    assert_codes(&unit_src("let mut n = 0; return v;"), &["E1004"]);
    // `let mut` binds one name.
    assert_codes(&unit_src("let mut (a, b) = (v, v); return v;"), &["E0108"]);
    // Only a mutable local may omit its value.
    assert!(!codes(&check_src(&unit_src("let n: u8; return v;"))).is_empty());
}

#[test]
fn mutable_locals_are_assigned_before_read() {
    assert_codes(&unit_src("let mut x: u8; return x;"), &["E1051"]);
    // Both branches assign.
    assert_codes(
        &unit_src("let mut x: u8; if c { x = 1; } else { x = 2; } return x;"),
        &[],
    );
    // Only one does.
    assert_codes(
        &unit_src("let mut x: u8; if c { x = 1; } return x;"),
        &["E1051"],
    );
    // A branch that returns does not need to assign.
    assert_codes(
        &unit_src("let mut x: u8; if c { x = 1; } else { return 0; } return x;"),
        &[],
    );
    // Every match arm assigns.
    assert_codes(
        &unit_src("let mut x: u8; match v[1:0] { 0 => { x = 1; } 1 => { x = 2; } _ => { x = 3; } } return x;"),
        &[],
    );
    assert_codes(
        &unit_src("let mut x: u8; match v[1:0] { 0 => { x = 1; } _ => { } } return x;"),
        &["E1051"],
    );
    // A loop body may run zero times.
    assert_codes(
        &unit_src("let mut x: u8; for i in 0..2 { x = 1; } return x;"),
        &["E1051"],
    );
    // `x = x + 1` reads `x` first.
    assert_codes(&unit_src("let mut x: u8; x = x + 1; return x;"), &["E1051"]);
    // In an `if` used as a value.
    assert_codes(
        &unit_src("let mut x: u8; let y: u8 = if c { x = 1; 5 } else { x = 2; 6 }; return x + y;"),
        &[],
    );
}

#[test]
fn locals_may_not_shadow_body_places() {
    // A model state.
    assert_codes(
        &dev("state count: u8 = 0; on reset { let count: u8 = 1; }"),
        &["E1001"],
    );
    // A register, a register field, and a `for` variable named like one.
    assert_codes(&isa("fn f() { let AX: u16 = 1; }"), &["E1001"]);
    assert_codes(&isa("fn f() { let mut AL: u8 = 1; AL = 2; }"), &["E1001"]);
    assert_codes(&isa("fn f() { for BX in 0..2 { } }"), &["E1001"]);
}

#[test]
fn model_state_arrays_are_places() {
    let src = format!(
        "{BUS_PRELUDE}\nbundle Chan {{ count: u16, out: bool }}\ncomponent P {{ ports {{ bus io: B::S; }} model {{
            state ch: [Chan; 3];
            state hits: [u8; 4];
            on io.write(addr: u8, data: u8) -> (wait: u8) {{
                ch[addr[1:0]].count = data as u16;
                ch[0].out = true;
                hits[addr[1:0]] = hits[addr[1:0]] + 1;
                return (wait: 0);
            }}
        }} }}"
    );
    assert_codes(&src, &[]);
}

// --- model fns (SPEC 8.3a) ---

#[test]
fn model_fns_check() {
    let c = check_src(&dev(
        "state n: u8 = 0;
         fn bump(by: u8) -> u8 { n = n + by; return n; }
         fn raise() { emit io.interrupt(0) = bump(1) > 2; }
         on reset { raise(); let v = bump(by: 2); }
         on io.read(addr: u8) -> (data: u8, wait: u8) { raise(); return (data: n, wait: 0); }",
    ));
    assert_clean(&c);
    let model = c.hir.components[0].model.as_ref().unwrap();
    assert_eq!(model.fns.len(), 2);
    assert_eq!(model.fns[0].ret, Ty::UInt(8));
    assert!(!model.fns[0].starts_transaction);
}

#[test]
fn model_fn_errors() {
    // Recursion, direct and mutual.
    assert_codes(&dev("state n: u8 = 0; fn f() { n = 1; f(); }"), &["E1053"]);
    assert_codes(
        &dev("state n: u8 = 0; fn f() { n = 1; g(); } fn g() { f(); }"),
        &["E1053", "E1053"],
    );
    // A value-returning fn must return on every path.
    assert_codes(&dev("state n: u8 = 0; fn f() -> u8 { n = 1; }"), &["E1015"]);
    // The name is already a state.
    assert_codes(&dev("state f: u8 = 0; fn f() { f = 1; }"), &["E1001"]);
    // Not callable from a state initialiser.
    assert_codes(
        &dev("state n: u8 = f(); fn f() -> u8 { return n; }"),
        &["E1020"],
    );
    // A fn that touches nothing could be a unit.
    assert_codes(
        &dev("fn double(v: u8) -> u8 { return v + v; } on reset { let x = double(1); }"),
        &["W1005"],
    );
}

#[test]
fn fns_that_start_transactions_are_marked_and_may_serve_a_handler() {
    let src = format!(
        "{SYS_PRELUDE}\ncomponent Bridge {{ ports {{ bus up: B::S; bus down: B::M; }} model {{
            fn fetch(a: u8) -> u8 {{ return down.read(space: Sp::Mem, addr: a).data; }}
            fn indirect(a: u8) -> u8 {{ return fetch(a); }}
            on up.read(space: Sp, addr: u8) -> (data: u8, wait: u8) {{
                return (data: indirect(addr), wait: 1);
            }}
            on tick(n: u32) {{ let v = indirect(0); }}
        }} }}"
    );
    // A handler may reach a transaction through fns (a bridge), and both
    // fns are marked as starting one.
    let c = check_src(&src);
    assert_clean(&c);
    let model = c
        .hir
        .components
        .iter()
        .find(|c| c.name == "Bridge")
        .unwrap()
        .model
        .as_ref()
        .unwrap();
    assert!(model.fns.iter().all(|f| f.starts_transaction));
}

// --- decline, signal reads, several masters ---

#[test]
fn decline_is_for_transaction_handlers() {
    assert_codes(
        &dev("on io.read(addr: u8) -> (data: u8, wait: u8) { if addr > 4 { decline; } return (data: 0, wait: 0); }"),
        &[],
    );
    // `decline` diverges, so this handler returns on every path.
    assert_codes(
        &dev("on io.read(addr: u8) -> (data: u8, wait: u8) { decline; }"),
        &[],
    );
    assert_codes(&dev("on reset { decline; }"), &["E1054"]);
    assert_codes(
        &dev("state n: u8 = 0; fn f() { n = 1; decline; }"),
        &["E1054"],
    );
}

#[test]
fn models_read_signal_levels() {
    let c = check_src(&dev(
        "state seen: bool = false; on reset { seen = io.interrupt(1); }",
    ));
    assert_clean(&c);
    // A level needs its arguments, and is a `bool`.
    assert_codes(
        &dev("state n: u8 = 0; on reset { n = io.interrupt(1); }"),
        &["E1005"],
    );
}

#[test]
fn a_bus_may_have_several_masters() {
    let src = system(
        "clock a: 1 MHz; component h: Host; component g: Host; component d: Dev @ a;
         bus x: B @ a { master h.io; master g.io; slave d.io @ mem 0..=7; }",
    );
    let c = check_src(&src);
    assert_clean(&c);
    assert_eq!(c.hir.systems[0].buses[0].masters, [(0, 0), (1, 0)]);
}

// --- wires (model-level `in`/`out` ports and system `wire`) ---

const WIRE_PRELUDE: &str = r#"
component Src { ports { out level: bool; out code: u3; } model {
    state n: u8 = 0;
    on reset { level = true; code = 5; n = code as u8; }
} }
component Dst { ports { in level: bool; in code: u3; in strap: bool; } model {
    state seen: u8 = 0;
    on level(value: bool) { if value && strap { seen = code as u8; } }
} }
"#;

fn wired(body: &str) -> String {
    format!("{WIRE_PRELUDE}\nsystem S {{ component s: Src; component d: Dst; {body} }}")
}

#[test]
fn wires_connect_outputs_to_inputs() {
    let c = check_src(&wired(
        "wire d.level = s.level; wire d.code = s.code; wire d.strap = true;",
    ));
    assert_clean(&c);
    let w = &c.hir.systems[0].wires;
    assert_eq!(w.len(), 3);
    assert_eq!(w[0].source, machine_sema::hir::WireSource::Port((0, 0)));
    assert_eq!(w[2].source, machine_sema::hir::WireSource::Const(1));
    let dst = c.hir.components.iter().find(|c| c.name == "Dst").unwrap();
    assert!(matches!(
        dst.model.as_ref().unwrap().handlers[0].kind,
        machine_sema::hir::HandlerKind::Input { port: 0 }
    ));
}

#[test]
fn wire_errors() {
    // An output as the target, an input as the source.
    assert_codes(&wired("wire s.level = d.level;"), &["E1049"]);
    assert_codes(&wired("wire d.level = d.strap;"), &["E1049"]);
    // Types differ.
    assert_codes(&wired("wire d.level = s.code;"), &["E1049"]);
    // Driven twice.
    assert_codes(
        &wired("wire d.strap = true; wire d.strap = false;"),
        &["E1049"],
    );
    // A constant that does not fit.
    assert_codes(&wired("wire d.code = 9;"), &["E1049"]);
    // Inputs are not assignable; outputs are.
    assert_codes(
        "component C { ports { in a: bool; } model { on reset { a = true; } } }",
        &["E1030"],
    );
    // An input handler takes the port's type.
    assert_codes(
        "component C { ports { in a: u4; } model { state n: u4 = 0; on a(value: bool) { } } }",
        &["E1041"],
    );
}

// --- tick scheduling ---

#[test]
fn tick_may_return_its_next_deadline() {
    let c = check_src(&dev(
        "state n: u32 = 0; on tick(k: u32) -> u32 { n = n + k; return 100; }",
    ));
    assert_clean(&c);
    let h = &c.hir.components[0].model.as_ref().unwrap().handlers[0];
    assert_eq!(h.ret, Ty::UInt(32));
    // Any other return type is an error, and a deadline must be returned on
    // every path.
    assert_codes(&dev("on tick(k: u32) -> u8 { return 1; }"), &["E1041"]);
    assert_codes(
        &dev("state n: u32 = 0; on tick(k: u32) -> u32 { n = k; }"),
        &["E1015"],
    );
}

// --- timing tables ---

#[test]
fn timing_tables_check() {
    let table = |entries: &str| {
        let src = DEC_PRELUDE.replace(
            "undefined UD; }",
            "undefined UD; fault deliver; }\n    fn deliver(v: u8, restart: bool) { IP = v as u16; }\n    instruction Nop { mnemonic \"nop\"; encoding { opcode 0x90; } }",
        );
        format!("{src}\n}}\ntiming on T {{ {entries} }}")
    };
    let good = table("Nop => 3; fault => match vector { 0 => 8, _ => 9 }; interrupt => 9;");
    let c = check_src(&good);
    assert_clean(&c);
    let info = &c.hir.isas[0];
    assert!(info
        .instructions
        .iter()
        .find(|i| i.name == "Nop")
        .unwrap()
        .timing
        .is_some());
    assert!(info.fault_cycles.is_some() && info.interrupt_cycles.is_some());
    // An unknown instruction, an entry twice, a non-isa target.
    assert_codes(&table("Nope => 3;"), &["E1055"]);
    assert_codes(&table("Nop => 3; Nop => 4;"), &["E1055"]);
    assert_codes(
        &format!("{}\ntiming on Nothing {{ }}", table("")),
        &["E1055"],
    );
    // `restarted()` belongs to timing expressions.
    assert_codes(&table("Nop => if restarted() { 0 } else { 3 };"), &[]);
    assert_codes(&isa("fn f() -> bool { return restarted(); }"), &["E1020"]);
    // `repeats()` too; and how fetches combine with execution.
    assert_codes(&table("Nop => if repeats() { 8 } else { 3 };"), &[]);
    assert_codes(&isa("fn f() -> bool { return repeats(); }"), &["E1020"]);
    let c = check_src(&table("fetch => serial; Nop => 3;"));
    assert_clean(&c);
    assert!(c.hir.isas[0].serial_fetch);
    assert!(!check_src(&table("fetch => overlapped;")).hir.isas[0].serial_fetch);
    assert_codes(&table("fetch => sometimes;"), &["E1055"]);
    // A schedule belongs in a `schedules` table, as do `decode` entries.
    assert_codes(&table("Nop => schedule(3);"), &["E1055"]);
    assert_codes(&table("decode Imm8 => 1;"), &["E1055"]);
    assert_codes(&table("fault Nop => 1;"), &["E1055"]);
}

#[test]
fn instruction_families_and_decode_conditions() {
    let fam = |enc: &str| {
        let src = DEC_PRELUDE.replace(
            "undefined UD; }",
            &format!("undefined UD; }}\n    latch opsize: u1 = 0;\n    fn osize() -> u6 {{ return if opsize == 1 {{ 32 }} else {{ 16 }}; }}\n    instruction Nop<const W: width> in [16, 32] {{ mnemonic \"nop\"; encoding {{ {enc} opcode 0x90; }} semantics {{ let x: u<W> = 0; }} }}"),
        );
        format!("{src}\n}}")
    };
    // Instances told apart by a `when` on the same expression.
    let c = check_src(&fam("when osize() == W;"));
    assert_clean(&c);
    let names: Vec<&str> = c.hir.isas[0]
        .instructions
        .iter()
        .map(|i| i.name.as_str())
        .collect();
    assert!(
        names.contains(&"Nop<16>") && names.contains(&"Nop<32>"),
        "{names:?}"
    );
    // Without one, they overlap.
    assert_codes(&fam(""), &["E1045"]);
    // Nor with the same constant.
    assert_codes(&fam("when osize() == 16;"), &["E1045"]);
    // A `when` needs a condition.
    assert_codes(&fam("when;"), &["E1032", "E1032"]);
}

#[test]
fn schedule_tables_check() {
    let tables = |schedules: &str| {
        let src = DEC_PRELUDE.replace(
            "undefined UD; }",
            "undefined UD; fault deliver; }\n    fn deliver(v: u8, restart: bool) { IP = v as u16; }\n    instruction Nop { mnemonic \"nop\"; encoding { opcode 0x90; } }",
        );
        format!("{src}\n}}\ntiming on T {{ Nop => 3; }}\nschedules on T {{ {schedules} }}")
    };
    let good = tables(
        "decode Imm8 => 1; Nop => if AL == 0 { schedule(2, access, 1, jump, 0) } else { schedule(4) }; interrupt => schedule(4, access, 0, jump, 0);",
    );
    let c = check_src(&good);
    assert_clean(&c);
    let info = &c.hir.isas[0];
    let nop = info.instructions.iter().find(|i| i.name == "Nop").unwrap();
    assert!(nop.timing.is_some() && nop.schedule.is_some());
    assert!(info.interrupt_schedule.is_some());
    assert_eq!(
        info.operands
            .iter()
            .find(|o| o.name == "Imm8")
            .unwrap()
            .decode,
        1
    );
    // A plain number is `schedule(n)`.
    assert_codes(&tables("Nop => 3; interrupt => 9;"), &[]);
    // A fault's delivery, generally and for one instruction (with its
    // operands and `vector` in scope).
    assert_codes(
        &tables("fault => match vector { 0 => schedule(4, access, 0, branch, 0), _ => schedule(5) }; fault Nop => if vector == 6 { schedule(7) } else { schedule(2) };"),
        &[],
    );
    assert_codes(&tables("fault Nope => 1;"), &["E1055"]);
    assert_codes(&tables("fault Nop => 1; fault Nop => 2;"), &["E1055"]);
    // `decode` names an operand format and a constant.
    assert_codes(&tables("decode Nope => 1;"), &["E1055"]);
    assert_codes(&tables("decode Imm8 => AL;"), &["E1055"]);
    assert_codes(&tables("decode Imm8 => 1; decode Imm8 => 2;"), &["E1055"]);
    // The shape of a schedule: cycles and steps alternate, cycles at both
    // ends, at most 8 steps, no named arguments.
    assert_codes(&tables("Nop => schedule(access, 0);"), &["E1056"]);
    assert_codes(&tables("Nop => schedule(3, access);"), &["E1056"]);
    assert_codes(&tables("Nop => schedule(3, 4);"), &["E1056"]);
    assert_codes(
        &tables("Nop => schedule(3, access, 0, decode: 1);"),
        &["E1056"],
    );
    assert_codes(
        &tables("Nop => schedule(0, access, 0, access, 0, access, 0, access, 0, access, 0, access, 0, access, 0, access, 0, access, 0);"),
        &["E1056"],
    );
    // `access * n`: n accesses with no work between them.
    assert_codes(
        &tables("Nop => schedule(4, access, 4, access * (AL & 3), 4, access, 5);"),
        &[],
    );
    assert_codes(&tables("Nop => schedule(4, access * true, 0);"), &["E1005"]);
    // Only timing expressions build schedules.
    assert_codes(&isa("fn f() { let s = schedule(3); }"), &["E1020"]);
}

#[test]
fn a_trailing_if_or_match_is_the_block_value() {
    assert_codes(
        &unit_src("let x: u8 = if c { if v[0] == 1 { 1 } else { 2 } } else { 3 }; return x;"),
        &[],
    );
    assert_codes(
        &unit_src("let x: u8 = if c { match v[1:0] { 0 => 1, _ => 2 } } else { 3 }; return x;"),
        &[],
    );
}
