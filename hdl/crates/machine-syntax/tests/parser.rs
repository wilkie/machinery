//! Parser unit tests: expression shapes, the ambiguous corners of the
//! grammar, error recovery, and termination on garbage.

use machine_syntax::ast::*;
use machine_syntax::{parse, parse_expr_str, SourceFile};

fn expr(src: &str) -> Expr {
    let (e, diags) = parse_expr_str(src);
    if !diags.is_empty() {
        let file = SourceFile::new("expr", src);
        let msg: String = diags.iter().map(|d| d.render_text(&file)).collect();
        panic!("{src}: {msg}");
    }
    e.expect("expression")
}

/// Compact S-expression rendering of an expression, for shape assertions.
fn sexp(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Int { value, suffix } => match suffix {
            Some((k, w)) => format!("{value}{k}{w}"),
            None => value.to_string(),
        },
        ExprKind::Bool(b) => b.to_string(),
        ExprKind::Str(s) => format!("{s:?}"),
        ExprKind::Quantity { text, unit, .. } => format!("{text}{unit:?}"),
        ExprKind::Path(p) => p
            .segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join("::"),
        ExprKind::Unary(op, a) => format!("({op:?} {})", sexp(a)),
        ExprKind::Binary(op, a, b) => format!("({op:?} {} {})", sexp(a), sexp(b)),
        ExprKind::Ternary(c, a, b) => format!("(? {} {} {})", sexp(c), sexp(a), sexp(b)),
        ExprKind::Cast(a, t) => format!("(as {} {})", sexp(a), ty(t)),
        ExprKind::Field(a, f) => format!("{}.{}", sexp(a), f.name),
        ExprKind::Index(a, i) => format!("{}[{}]", sexp(a), sexp(i)),
        ExprKind::Slice(a, h, l) => format!("{}[{}:{}]", sexp(a), sexp(h), sexp(l)),
        ExprKind::Call {
            callee,
            generics,
            args,
        } => {
            let g = if generics.is_empty() {
                String::new()
            } else {
                format!(
                    "<{}>",
                    generics
                        .iter()
                        .map(|g| match g {
                            GenericArg::Type(t) => ty(t),
                            GenericArg::Expr(e) => sexp(e),
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                )
            };
            format!(
                "{}{g}({})",
                sexp(callee),
                args.iter().map(arg).collect::<Vec<_>>().join(",")
            )
        }
        ExprKind::BundleLit {
            ty: t,
            fields,
            rest_default,
        } => format!(
            "{}{{{}{}}}",
            t.last().name,
            fields
                .iter()
                .map(|f| match &f.value {
                    Some(v) => format!("{}:{}", f.name.name, sexp(v)),
                    None => f.name.name.clone(),
                })
                .collect::<Vec<_>>()
                .join(","),
            if *rest_default { ",.." } else { "" }
        ),
        ExprKind::Tuple(items) => format!(
            "tuple({})",
            items.iter().map(arg).collect::<Vec<_>>().join(",")
        ),
        ExprKind::ArrayLit(items) => {
            format!("[{}]", items.iter().map(sexp).collect::<Vec<_>>().join(","))
        }
        ExprKind::ArrayRepeat(a, n) => format!("[{};{}]", sexp(a), sexp(n)),
        ExprKind::Match { scrutinee, arms } => {
            format!("(match {} {} arms)", sexp(scrutinee), arms.len())
        }
        ExprKind::If {
            cond, else_branch, ..
        } => format!("(if {} {})", sexp(cond), else_branch.is_some()),
        ExprKind::Block(b) => format!("(block {} stmts)", b.stmts.len()),
        ExprKind::RoutineCall(c) => format!("(call {})", sexp(c)),
        ExprKind::Fetch(p) => format!("(fetch {})", p.last().name),
        ExprKind::Range { lo, hi, inclusive } => format!(
            "({} {} {})",
            if *inclusive { "..=" } else { ".." },
            sexp(lo),
            sexp(hi)
        ),
        ExprKind::Wildcard => "_".into(),
        ExprKind::Error => "<error>".into(),
    }
}

fn arg(a: &Arg) -> String {
    match &a.name {
        Some(n) => format!("{}:{}", n.name, sexp(&a.value)),
        None => sexp(&a.value),
    }
}

fn ty(t: &Type) -> String {
    match &t.kind {
        TypeKind::Unsigned(w) => format!("u<{}>", sexp(w)),
        TypeKind::Signed(w) => format!("i<{}>", sexp(w)),
        TypeKind::Bool => "bool".into(),
        TypeKind::Width => "width".into(),
        TypeKind::Named(p, g) => {
            let name = p
                .segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join("::");
            if g.is_empty() {
                name
            } else {
                format!(
                    "{name}<{}>",
                    g.iter()
                        .map(|g| match g {
                            GenericArg::Type(t) => ty(t),
                            GenericArg::Expr(e) => sexp(e),
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
        }
        TypeKind::Array(e, n) => format!("[{};{}]", ty(e), sexp(n)),
        TypeKind::Tuple(items) => format!(
            "({})",
            items
                .iter()
                .map(|i| match &i.name {
                    Some(n) => format!("{}:{}", n.name, ty(&i.ty)),
                    None => ty(&i.ty),
                })
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

#[track_caller]
fn check(src: &str, expected: &str) {
    assert_eq!(sexp(&expr(src)), expected, "source: {src}");
}

#[test]
fn precedence_follows_the_spec_table() {
    check("a + b * c", "(Add a (Mul b c))");
    check("a * b + c", "(Add (Mul a b) c)");
    check("a - b - c", "(Sub (Sub a b) c)");
    check("a << 2 + 1", "(Shl a (Add 2 1))");
    check("a & b ^ c | d", "(BitOr (BitXor (BitAnd a b) c) d)");
    check("a == b && c != d || e", "(Or (And (Eq a b) (Ne c d)) e)");
    check("a < b && c > d", "(And (Lt a b) (Gt c d))");
    check("!a && ~b == 0", "(And (Not a) (Eq (BitNot b) 0))");
    check("-a + b", "(Add (Neg a) b)");
}

#[test]
fn ternary_is_right_associative_and_lowest() {
    check("a ? b : c ? d : e", "(? a b (? c d e))");
    check("x == 1 ? a + 1 : b", "(? (Eq x 1) (Add a 1) b)");
}

#[test]
fn casts_bind_tighter_than_binary_operators() {
    check(
        "a as u<W + 1> + b as u<W + 1>",
        "(Add (as a u<(Add W 1)>) (as b u<(Add W 1)>))",
    );
    check("(r == 0) as u1", "(as (Eq r 0) u<1>)");
    check("x as i16 >> 2", "(Shr (as x i<16>) 2)");
    check("v as bool", "(as v bool)");
}

#[test]
fn postfix_forms() {
    check("x[7:0]", "x[7:0]");
    check("x[3]", "x[3]");
    check("a.b.c[i].d", "a.b.c[i].d");
    check("raw[W]", "raw[W]");
    check("f(a, b)(c)", "f(a,b)(c)");
    check("io.respond(data: v, wait: 0)", "io.respond(data:v,wait:0)");
    check(
        "m.tail.Disp16.value as u16",
        "(as m.tail.Disp16.value u<16>)",
    );
}

#[test]
fn generic_call_versus_comparison() {
    check("Alu<8>(op, a, b, 0)", "Alu<8>(op,a,b,0)");
    check("AluFlags<W>(op, a, b, raw)", "AluFlags<W>(op,a,b,raw)");
    check("Foo<u8, W + 1>(x)", "Foo<u<8>,(Add W 1)>(x)");
    check("a < b", "(Lt a b)");
    check("a < b > c", "(Gt (Lt a b) c)");
    check("x < y && (z)", "(And (Lt x y) z)");
}

#[test]
fn paths_literals_and_bundles() {
    check("AluOp::Add", "AluOp::Add");
    check(
        "I286Isa::effective_address.rtl(m)",
        "I286Isa::effective_address.rtl(m)",
    );
    check("0xFF_u8", "255u8");
    check("33.333 MHz", "33.333MHz");
    check("8 MHz", "8MHz");
    check("Flags { cf, pf: 1, ..default }", "Flags{cf,pf:1,..}");
    check("[0_u8; 6]", "[0u8;6]");
    check("[a, b, c]", "[a,b,c]");
    check("(result: x, raw: y)", "tuple(result:x,raw:y)");
    check("(a, b)", "tuple(a,b)");
    check("(a)", "a");
    check("concat(a, b)", "concat(a,b)");
    check("call next_byte()", "(call next_byte())");
    check("fetch ModRM", "(fetch ModRM)");
    check("0x40..=0x43", "(..= 64 67)");
}

#[test]
fn match_arms_patterns_and_bodies() {
    let e = expr("match op { AluOp::Add | AluOp::Adc => (1, 2), 0..=3 => x, _ => { y } }");
    let ExprKind::Match { arms, .. } = &e.kind else {
        panic!()
    };
    assert_eq!(arms.len(), 3);
    assert_eq!(arms[0].patterns.len(), 2);
    assert!(matches!(
        arms[1].patterns[0].kind,
        PatternKind::Range {
            inclusive: true,
            ..
        }
    ));
    assert!(matches!(arms[2].patterns[0].kind, PatternKind::Wildcard));
    assert!(matches!(arms[2].body, ArmBody::Block(_)));
}

#[test]
fn if_heads_do_not_take_bundle_literals() {
    // `Foo {` after `==` in an if head opens the body, not a literal.
    let e = expr("if x == Mode::Protected { a } else { b }");
    let ExprKind::If {
        cond, else_branch, ..
    } = &e.kind
    else {
        panic!()
    };
    assert_eq!(sexp(cond), "(Eq x Mode::Protected)");
    assert!(else_branch.is_some());
    // But inside parentheses or call arguments they are allowed again.
    let e = expr("if f(Flags { cf: 1 }) { a } else { b }");
    let ExprKind::If { cond, .. } = &e.kind else {
        panic!()
    };
    assert_eq!(sexp(cond), "f(Flags{cf:1})");
}

#[test]
fn nested_generics_split_shift_right_token() {
    let parsed = parse("unit F(a: Wrap<Inner<8>>) -> u8 { return 0; }");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let Decl::Unit(u) = &parsed.file.decls[0] else {
        panic!()
    };
    assert_eq!(ty(&u.params[0].ty), "Wrap<Inner<8>>");
}

#[test]
fn array_type_length_may_use_comparison_free_expressions() {
    let parsed = parse("unit F(a: [u<W>; N * 2]) -> [bool; 4] { return a; }");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

fn unit_body(src: &str) -> Block {
    let parsed = parse(src);
    if !parsed.diagnostics.is_empty() {
        let file = SourceFile::new("stmt", src);
        let msg: String = parsed
            .diagnostics
            .iter()
            .map(|d| d.render_text(&file))
            .collect();
        panic!("{msg}");
    }
    match parsed.file.decls.into_iter().next() {
        Some(Decl::Unit(u)) => u.body,
        other => panic!("expected unit, got {other:?}"),
    }
}

#[test]
fn statement_forms() {
    let body = unit_body(
        "unit F() -> bool {
            let (a, _, b): u8 = f();
            x = 1;
            y <- 2 @commit(on_retire);
            z <- 3;
            if a { b = 1; } else if c { } else { d = 2; }
            match m { 0 => { x = 1; } _ => { } }
            for i in 0..4 { q[i] = 0; }
            wait until ready;
            cycle { p = 1; }
            retire;
            fault Isa::UD;
            raise GP(0);
            emit io.interrupt(0) = pending;
            start script();
            dispatch op via I286Isa { otherwise => bad(); }
            default { out = 0; }
            assert(x == 1, \"one\");
            return true;
        }",
    );
    let kinds: Vec<&str> = body
        .stmts
        .iter()
        .map(|s| match &s.kind {
            StmtKind::Let { .. } => "let",
            StmtKind::Assign { .. } => "assign",
            StmtKind::Schedule { annotations, .. } if !annotations.is_empty() => "schedule+ann",
            StmtKind::Schedule { .. } => "schedule",
            StmtKind::If { .. } => "if",
            StmtKind::Match { .. } => "match",
            StmtKind::For { .. } => "for",
            StmtKind::WaitUntil(_) => "wait",
            StmtKind::Cycle(_) => "cycle",
            StmtKind::Retire => "retire",
            StmtKind::Fault(_) => "fault",
            StmtKind::Raise(_) => "raise",
            StmtKind::Emit { .. } => "emit",
            StmtKind::Start(_) => "start",
            StmtKind::Dispatch { .. } => "dispatch",
            StmtKind::Default(_) => "default",
            StmtKind::Expr(_) => "expr",
            StmtKind::Return(_) => "return",
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            "let",
            "assign",
            "schedule+ann",
            "schedule",
            "if",
            "match",
            "for",
            "wait",
            "cycle",
            "retire",
            "fault",
            "raise",
            "emit",
            "start",
            "dispatch",
            "default",
            "expr",
            "return"
        ]
    );
}

#[test]
fn contextual_words_remain_usable_as_names() {
    // `size`, `data`, `mode`, `state`, `wire` as plain identifiers.
    let body = unit_body(
        "unit F(size: u8, data: u8, mode: u8) -> u8 {
            let state = size + data;
            let wire = mode;
            default = 1;
            return state + wire + default;
        }",
    );
    assert_eq!(body.stmts.len(), 4);
}

#[test]
fn block_tail_expression_needs_no_semicolon() {
    let e = expr("if c { let x = 1; x } else { 2 }");
    let ExprKind::If { then_branch, .. } = &e.kind else {
        panic!()
    };
    assert_eq!(then_branch.stmts.len(), 2);
    assert!(matches!(then_branch.stmts[1].kind, StmtKind::Expr(_)));
}

#[test]
fn recovery_keeps_following_declarations() {
    let src = "enum A : u1 { X, Y }\nbundle Broken { a: u8 oops }\nenum B : u1 { Z }\n";
    let parsed = parse(src);
    assert!(parsed.diagnostics.has_errors());
    let kws: Vec<_> = parsed.file.decls.iter().map(|d| d.keyword()).collect();
    assert_eq!(kws[0], "enum");
    assert_eq!(*kws.last().unwrap(), "enum");
    let Decl::Enum(b) = parsed.file.decls.last().unwrap() else {
        panic!()
    };
    assert_eq!(b.name.name, "B");
}

#[test]
fn recovery_keeps_following_statements() {
    let src = "unit F() -> u8 { let a = 1; let b = = 2; let c = 3; return c; }";
    let parsed = parse(src);
    assert_eq!(parsed.diagnostics.error_count(), 1);
    let Decl::Unit(u) = &parsed.file.decls[0] else {
        panic!()
    };
    let kinds: Vec<bool> = u
        .body
        .stmts
        .iter()
        .map(|s| matches!(s.kind, StmtKind::Error))
        .collect();
    assert_eq!(kinds, vec![false, true, false, false]);
}

#[test]
fn stray_closers_terminate_with_bounded_diagnostics() {
    // Each of these once sent the parser into an unbounded loop.
    let inputs = [
        "isa X { operand_kind r8 : u8 { select(n) => n; } }",
        "isa X { ) ) ) }",
        "component C { ports { ] } rtl { ) } }",
        "bus B { wires { a: u8 ) ; } roles { ) } }",
        "system S { bus b: P { ) } }",
        "unit F() -> u8 { ) ] ) }",
        "enum E : u1 { ) }",
        "test \"t\" { run ) cycles; }",
        "unit F() -> u8 { match x { ) => 1 } }",
        "{{{{{{{{",
        "}}}}}}}}",
        "unit F(",
        "component C { rtl { routine r( }",
    ];
    for src in inputs {
        let parsed = parse(src);
        assert!(parsed.diagnostics.has_errors(), "{src}: expected errors");
        assert!(
            parsed.diagnostics.len() < 50,
            "{src}: {} diagnostics",
            parsed.diagnostics.len()
        );
    }
}

#[test]
fn random_token_soup_terminates() {
    // Deterministic pseudo-random sequences of plausible tokens. The parser
    // must finish, and the error cap must keep diagnostics bounded.
    let vocab = [
        "unit",
        "component",
        "isa",
        "bus",
        "system",
        "enum",
        "bundle",
        "rtl",
        "model",
        "ports",
        "routine",
        "cycle",
        "match",
        "if",
        "else",
        "let",
        "return",
        "reg",
        "wire",
        "on",
        "default",
        "{",
        "}",
        "(",
        ")",
        "[",
        "]",
        "<",
        ">",
        ";",
        ",",
        ":",
        "::",
        "=",
        "<-",
        "->",
        "=>",
        "+",
        "*",
        "|",
        "?",
        "@",
        "..=",
        "x",
        "Y",
        "u8",
        "0",
        "1",
        "\"s\"",
        "as",
        "for",
        "in",
        "loop",
        "_",
        "true",
    ];
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    for _ in 0..300 {
        let mut src = String::new();
        let len = 20 + (seed % 60) as usize;
        for _ in 0..len {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let i = ((seed >> 33) as usize) % vocab.len();
            src.push_str(vocab[i]);
            src.push(' ');
        }
        let parsed = parse(&src);
        assert!(
            parsed.diagnostics.len() <= 202,
            "soup produced {} diagnostics: {src}",
            parsed.diagnostics.len()
        );
    }
}
