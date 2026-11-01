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
        &["E1030"],
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
    assert_codes(
        &isa("operand_kind bad : u8 from m: ModRM { read => 1; frob => 2; }"),
        &["E1037"],
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
    assert_codes(&master("on tick(n: u32) { io.interrupt(0); }"), &["E1044"]);
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
    let pattern_overlap = r#"
        instruction A { mnemonic "a"; encoding { opcode "0100oo00" { o: op } } operands (o: op); }
        instruction B { mnemonic "b"; encoding { opcode 0x48; } }"#;
    assert_codes(&dec(pattern_overlap), &["E1045"]);
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
    assert_codes(&v("format json; memory mem;"), &["E1049"]);
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
    assert_eq!(isa.instructions.iter().filter(|i| !i.prefix).count(), 144);
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
    assert_eq!(bus.master, Some((0, 0)));
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
fn transaction_handlers_cannot_start_transactions() {
    let src = format!(
        "{SYS_PRELUDE}\ncomponent Bridge {{ ports {{ bus up: B::S; bus down: B::M; }} model {{
            on up.read(space: Sp, addr: u8) -> (data: u8, wait: u8) {{
                let r = down.read(space: space, addr: addr);
                return (data: r.data, wait: 1);
            }}
        }} }}"
    );
    assert_codes(&src, &["E1048"]);
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
}

#[test]
fn isa_reset_fn_takes_nothing() {
    assert_codes(&isa("fn reset() { AX = 0; }"), &[]);
    assert_codes(&isa("fn reset(v: u16) { AX = v; }"), &["E1046"]);
}

#[test]
fn i286_board_system_checks_clean() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("targets")
        .join("i286");
    let files = ["alu.machine", "i286.machine", "board.machine"]
        .iter()
        .map(|f| SourceFile::new(*f, std::fs::read_to_string(root.join(f)).unwrap()))
        .collect();
    let c = check_files(files);
    assert_clean(&c);
    let sys = &c.hir.systems[0];
    assert_eq!(sys.components.len(), 5);
    assert_eq!(sys.buses[0].slaves.len(), 4);
    // The ROM answers at the top of both the 1 MB and the 16 MB space.
    assert_eq!(sys.buses[0].slaves[1].1.len(), 2);
}
