//! The three samples in `hdl/samples/` must parse without diagnostics. They
//! are the executable form of SPEC.md, so a parser change that breaks one is
//! either a bug or a deliberate spec change (in which case update the sample
//! and the spec together).

use std::path::PathBuf;

use machine_syntax::ast::*;
use machine_syntax::{parse, SourceFile};

fn sample(name: &str) -> SourceFile {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "samples", name]
        .iter()
        .collect();
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    SourceFile::new(name, text)
}

fn parse_clean(name: &str) -> File {
    let file = sample(name);
    let parsed = parse(&file.text);
    if !parsed.diagnostics.is_empty() {
        let mut msg = String::new();
        for d in parsed.diagnostics.iter() {
            msg.push_str(&d.render_text(&file));
        }
        panic!("{name}: unexpected diagnostics:\n{msg}");
    }
    assert!(
        !parsed
            .file
            .decls
            .iter()
            .any(|d| matches!(d, Decl::Error(_))),
        "{name}: error declaration without a diagnostic"
    );
    parsed.file
}

fn decl_keywords(file: &File) -> Vec<&'static str> {
    file.decls.iter().map(|d| d.keyword()).collect()
}

#[test]
fn alu_sample_parses() {
    let file = parse_clean("alu.machine");
    assert_eq!(
        decl_keywords(&file),
        vec!["enum", "enum", "bundle", "unit", "unit", "unit", "test", "test", "test", "test"]
    );
    // Doc comments attach to declarations.
    match &file.decls[0] {
        Decl::Enum(e) => {
            assert_eq!(e.name.name, "AluOp");
            assert_eq!(e.variants.len(), 8);
            assert!(e.docs[0].starts_with("ALU operation selector"));
        }
        other => panic!("expected enum, got {}", other.keyword()),
    }
    // Generic unit with a named-tuple return.
    match &file.decls[4] {
        Decl::Unit(u) => {
            assert_eq!(u.name.name, "Alu");
            assert_eq!(u.generics.len(), 1);
            assert_eq!(u.params.len(), 4);
            assert!(
                matches!(u.ret.kind, TypeKind::Tuple(ref elems) if elems.len() == 2 && elems[0].name.is_some())
            );
        }
        other => panic!("expected unit, got {}", other.keyword()),
    }
}

#[test]
fn counter_bus_sample_parses() {
    let file = parse_clean("counter_bus.machine");
    assert_eq!(
        decl_keywords(&file),
        vec![
            "bus",
            "bundle",
            "component",
            "component",
            "system",
            "test",
            "conformance"
        ]
    );
    match &file.decls[0] {
        Decl::Bus(b) => {
            assert_eq!(b.name.name, "PeriphBus");
            let wires = b.items.iter().find_map(|i| match i {
                BusItem::Wires(w) => Some(w),
                _ => None,
            });
            assert_eq!(wires.unwrap().len(), 6);
            let txns = b
                .items
                .iter()
                .filter(|i| matches!(i, BusItem::Transaction(_)))
                .count();
            assert_eq!(txns, 2);
        }
        other => panic!("expected bus, got {}", other.keyword()),
    }
    match &file.decls[2] {
        Decl::Component(c) => {
            assert_eq!(c.name.name, "Timer");
            assert_eq!(c.ports.len(), 3);
            assert!(c.rtl.is_some());
            assert!(c.model.is_some());
            let model = c.model.as_ref().unwrap();
            let handlers = model
                .stmts
                .iter()
                .filter(|s| matches!(s.kind, StmtKind::On { .. }))
                .count();
            assert_eq!(handlers, 3);
        }
        other => panic!("expected component, got {}", other.keyword()),
    }
    match &file.decls[4] {
        Decl::System(s) => {
            let profiles = s
                .items
                .iter()
                .filter(|i| matches!(i, SystemItem::Profile(_)))
                .count();
            assert_eq!(profiles, 2);
            let clocks = s
                .items
                .iter()
                .filter(|i| matches!(i, SystemItem::Clock(_)))
                .count();
            assert_eq!(clocks, 2);
        }
        other => panic!("expected system, got {}", other.keyword()),
    }
}

#[test]
fn cpu_slice_sample_parses() {
    let file = parse_clean("cpu_slice.machine");
    assert_eq!(
        decl_keywords(&file),
        vec![
            "use",
            "bundle",
            "isa",
            "enum",
            "bus",
            "bundle",
            "component",
            "test",
            "test",
            "vectors",
            "conformance"
        ]
    );
    match &file.decls[2] {
        Decl::Isa(isa) => {
            assert_eq!(isa.name.name, "I286Isa");
            let instructions: Vec<_> = isa
                .items
                .iter()
                .filter_map(|i| match i {
                    IsaItem::Instruction(ins) => Some(ins),
                    _ => None,
                })
                .collect();
            assert_eq!(instructions.len(), 2);
            assert_eq!(instructions[0].name.name, "Add_rm8_r8");
            assert!(instructions[0].items.iter().any(
                |i| matches!(i, InstructionItem::Implements(p) if p.last().name == "add_rm_reg8")
            ));
            let registers = isa
                .items
                .iter()
                .filter(|i| matches!(i, IsaItem::Register(_)))
                .count();
            assert_eq!(registers, 14);
            let flags = isa.items.iter().find_map(|i| match i {
                IsaItem::Register(r) if r.name.name == "FLAGS" => Some(r),
                _ => None,
            });
            let flags = flags.unwrap();
            assert_eq!(flags.fields.len(), 9);
            assert_eq!(flags.views.len(), 1);
        }
        other => panic!("expected isa, got {}", other.keyword()),
    }
    match &file.decls[6] {
        Decl::Component(c) => {
            assert_eq!(c.name.name, "I286");
            assert_eq!(c.isa.as_ref().unwrap().last().name, "I286Isa");
            let rtl = c.rtl.as_ref().unwrap();
            let routines: Vec<&RoutineDecl> = rtl
                .stmts
                .iter()
                .filter_map(|s| match &s.kind {
                    StmtKind::Routine(r) => Some(r),
                    _ => None,
                })
                .collect();
            let names: Vec<&str> = routines.iter().map(|r| r.name.name.as_str()).collect();
            assert_eq!(
                names,
                vec![
                    "prefetch_fill",
                    "next_byte",
                    "mem_read8",
                    "mem_write8",
                    "ea_calc",
                    "add_rm_reg8",
                    "cmp_rm_reg8",
                    "undefined_opcode"
                ]
            );
            assert!(routines[1].istream);
            assert!(rtl
                .stmts
                .iter()
                .any(|s| matches!(s.kind, StmtKind::Sequencer(_))));
            let arch = rtl.stmts.iter().find_map(|s| match &s.kind {
                StmtKind::Architectural { mappings, at } => Some((mappings, at)),
                _ => None,
            });
            let (mappings, at) = arch.expect("architectural block");
            assert_eq!(mappings.len(), 14);
            assert_eq!(at.as_ref().unwrap().name, "retire");
        }
        other => panic!("expected component, got {}", other.keyword()),
    }
}
