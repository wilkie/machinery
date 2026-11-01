//! `machine`: the Machine language command-line driver.
//!
//! ```text
//! machine check [--message-format=text|json] [--syntax-only] <file>...
//! machine dump-ast <file>
//! ```

use std::process::ExitCode;

use machine_sema::check_files;
use machine_syntax::{parse, Severity, SourceFile};

const USAGE: &str = "\
Usage:
  machine check [--message-format=text|json] [--syntax-only] <file.machine>...
  machine build --isa <Name> --out <dir> <file.machine>...
  machine test --isa <Name> <file.machine>...
  machine vectors --isa <Name> [--filter 00,80.] [--max-failures N] [--quiet] <file.machine>...
  machine dump-ast <file.machine>

Commands:
  check      Parse and check the given files as one program; report diagnostics.
             Files refer to each other with `use <stem>::{...}`, so pass every
             file of the program.
  build      Check, then generate the Rust interpreter crate for an isa into <dir>.
  test       Check, generate the interpreter into target/machine/<isa>, and run
             its tests (the `test ... on <Isa>` blocks) with cargo.
  vectors    Check, generate, and run the isa's `vectors` (hardware-captured
             test vectors) through the interpreter in release mode.
  dump-ast   Parse one file and print its syntax tree (debug form).

Options:
  --message-format=json   One JSON object per diagnostic on stdout.
  --syntax-only           Stop after parsing.
";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Text,
    Json,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        eprint!("{USAGE}");
        return ExitCode::from(2);
    };
    match command.as_str() {
        "check" => check(&args[1..]),
        "build" => build(&args[1..], false),
        "test" => build(&args[1..], true),
        "vectors" => build(&args[1..], true),
        "dump-ast" => dump_ast(&args[1..]),
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command `{other}`\n");
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn load(path: &str) -> Option<SourceFile> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(SourceFile::new(path, text)),
        Err(e) => {
            eprintln!("error: cannot read `{path}`: {e}");
            None
        }
    }
}

fn check(args: &[String]) -> ExitCode {
    let mut format = Format::Text;
    let mut syntax_only = false;
    let mut paths = Vec::new();
    for a in args {
        if let Some(f) = a.strip_prefix("--message-format=") {
            format = match f {
                "text" => Format::Text,
                "json" => Format::Json,
                _ => {
                    eprintln!("error: unknown message format `{f}` (expected `text` or `json`)");
                    return ExitCode::from(2);
                }
            };
        } else if a == "--syntax-only" {
            syntax_only = true;
        } else if a.starts_with('-') {
            eprintln!("error: unknown option `{a}`");
            return ExitCode::from(2);
        } else {
            paths.push(a.clone());
        }
    }
    if paths.is_empty() {
        eprintln!("error: no input files\n");
        eprint!("{USAGE}");
        return ExitCode::from(2);
    }

    let mut files = Vec::new();
    for p in &paths {
        match load(p) {
            Some(f) => files.push(f),
            None => return ExitCode::FAILURE,
        }
    }

    let mut errors = 0usize;
    let mut warnings = 0usize;
    let mut emit = |file: &SourceFile, d: &machine_syntax::Diagnostic| {
        match d.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
        }
        match format {
            Format::Text => eprintln!("{}", d.render_text(file)),
            Format::Json => println!("{}", d.render_json(file)),
        }
    };

    if syntax_only {
        for file in &files {
            let parsed = parse(&file.text);
            for d in parsed.diagnostics.iter() {
                emit(file, d);
            }
            if format == Format::Text && parsed.diagnostics.is_empty() {
                eprintln!(
                    "{}: syntax ok ({} declarations)",
                    file.name,
                    parsed.file.decls.len()
                );
            }
        }
    } else {
        let checked = check_files(files);
        for (m, diags) in checked.diagnostics.iter().enumerate() {
            let file = &checked.program.modules[m].file;
            for d in diags.iter() {
                emit(file, d);
            }
        }
        if format == Format::Text {
            for (m, module) in checked.program.modules.iter().enumerate() {
                if checked.diagnostics[m].is_empty() {
                    let skipped: Vec<String> = checked
                        .skipped
                        .iter()
                        .filter(|s| s.module == m)
                        .map(|s| format!("{} {}", s.kind, s.name))
                        .collect();
                    if skipped.is_empty() {
                        eprintln!("{}: ok", module.file.name);
                    } else {
                        eprintln!(
                            "{}: ok (not yet checked: {})",
                            module.file.name,
                            skipped.join(", ")
                        );
                    }
                }
            }
            let instructions: usize = checked.hir.isas.iter().map(|i| i.instructions.len()).sum();
            let models = checked
                .hir
                .components
                .iter()
                .filter(|c| c.model.is_some())
                .count();
            eprintln!(
                "checked {} unit instance(s), {} isa(s) with {} instruction(s), {} bus(es), {} component(s) ({} with a model), {} test(s)",
                checked.hir.units.len(),
                checked.hir.isas.len(),
                instructions,
                checked.hir.buses.len(),
                checked.hir.components.len(),
                models,
                checked.hir.tests.len()
            );
        }
    }
    if format == Format::Text && (errors > 0 || warnings > 0) {
        eprintln!("{errors} error(s), {warnings} warning(s)");
    }
    if errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn dump_ast(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("error: dump-ast takes one file");
        return ExitCode::from(2);
    };
    let Some(file) = load(path) else {
        return ExitCode::FAILURE;
    };
    let parsed = parse(&file.text);
    for d in parsed.diagnostics.iter() {
        eprintln!("{}", d.render_text(&file));
    }
    println!("{:#?}", parsed.file);
    if parsed.diagnostics.has_errors() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// `machine build` / `machine test`.
fn build(args: &[String], run_tests: bool) -> ExitCode {
    let vectors_mode = std::env::args().nth(1).as_deref() == Some("vectors");
    let mut isa = None;
    let mut out_dir = None;
    let mut paths = Vec::new();
    let mut passthrough: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--isa" {
            isa = args.get(i + 1).cloned();
            i += 2;
            continue;
        }
        if let Some(v) = a.strip_prefix("--isa=") {
            isa = Some(v.to_string());
        } else if a == "--out" {
            out_dir = args.get(i + 1).cloned();
            i += 2;
            continue;
        } else if let Some(v) = a.strip_prefix("--out=") {
            out_dir = Some(v.to_string());
        } else if vectors_mode && (a == "--filter" || a == "--max-failures" || a == "--dir") {
            passthrough.push(a.clone());
            if let Some(v) = args.get(i + 1) {
                passthrough.push(v.clone());
            }
            i += 2;
            continue;
        } else if vectors_mode && a == "--quiet" {
            passthrough.push(a.clone());
        } else if a.starts_with('-') {
            eprintln!("error: unknown option `{a}`");
            return ExitCode::from(2);
        } else {
            paths.push(a.clone());
        }
        i += 1;
    }
    let Some(isa) = isa else {
        eprintln!("error: `--isa <Name>` is required");
        return ExitCode::from(2);
    };
    if paths.is_empty() {
        eprintln!("error: no input files");
        return ExitCode::from(2);
    }
    let mut files = Vec::new();
    for p in &paths {
        match load(p) {
            Some(f) => files.push(f),
            None => return ExitCode::FAILURE,
        }
    }
    let checked = check_files(files);
    for (m, diags) in checked.diagnostics.iter().enumerate() {
        let file = &checked.program.modules[m].file;
        for d in diags.iter() {
            eprintln!("{}", d.render_text(file));
        }
    }
    if checked.has_errors() {
        eprintln!("not generating: the program has errors");
        return ExitCode::FAILURE;
    }
    let src = match machine_model::generate_isa(&checked, &isa) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    // The runtime crate lives next to this binary's source tree.
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("machine-runtime");
    let runtime = runtime.canonicalize().unwrap_or(runtime);
    let name = machine_model::crate_name(&isa);
    let crate_ = machine_model::package(&name, src, &runtime);
    let dir = match out_dir {
        Some(d) => std::path::PathBuf::from(d),
        None => std::path::PathBuf::from("target")
            .join("machine")
            .join(&name),
    };
    if let Err(e) = machine_model::write_crate(&dir, &crate_) {
        eprintln!("error: cannot write `{}`: {e}", dir.display());
        return ExitCode::FAILURE;
    }
    eprintln!(
        "generated {} ({} lines) into {}",
        name,
        crate_.lib_rs.lines().count(),
        dir.display()
    );
    if !run_tests {
        return ExitCode::SUCCESS;
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = std::process::Command::new(cargo);
    if vectors_mode {
        if crate_.vectors_bin.is_none() {
            eprintln!("error: isa `{isa}` has no `vectors` declaration");
            return ExitCode::FAILURE;
        }
        cmd.args([
            "run",
            "--quiet",
            "--release",
            "--features",
            "vectors",
            "--bin",
            "vectors",
            "--manifest-path",
        ])
        .arg(dir.join("Cargo.toml"))
        .arg("--")
        .args(&passthrough);
    } else {
        cmd.arg("test")
            .arg("--manifest-path")
            .arg(dir.join("Cargo.toml"));
    }
    let status = cmd.status();
    match status {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: cannot run cargo: {e}");
            ExitCode::FAILURE
        }
    }
}
