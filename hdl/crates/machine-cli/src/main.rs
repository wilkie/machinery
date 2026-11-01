//! `machine`: the Machine language command-line driver.
//!
//! ```text
//! machine check [--message-format=text|json] <file>...
//! machine dump-ast <file>
//! ```
//!
//! Only the syntax phase exists yet; `check` parses and reports diagnostics.

use std::process::ExitCode;

use machine_syntax::{parse, SourceFile};

const USAGE: &str = "\
Usage:
  machine check [--message-format=text|json] <file.machine>...
  machine dump-ast <file.machine>

Commands:
  check      Parse the given files and report diagnostics.
  dump-ast   Parse one file and print its syntax tree (debug form).
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
    let mut files = Vec::new();
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
        } else if a.starts_with('-') {
            eprintln!("error: unknown option `{a}`");
            return ExitCode::from(2);
        } else {
            files.push(a.clone());
        }
    }
    if files.is_empty() {
        eprintln!("error: no input files\n");
        eprint!("{USAGE}");
        return ExitCode::from(2);
    }

    let mut errors = 0usize;
    let mut warnings = 0usize;
    for path in &files {
        let Some(file) = load(path) else {
            errors += 1;
            continue;
        };
        let parsed = parse(&file.text);
        for d in parsed.diagnostics.iter() {
            match d.severity {
                machine_syntax::Severity::Error => errors += 1,
                machine_syntax::Severity::Warning => warnings += 1,
            }
            match format {
                Format::Text => eprintln!("{}", d.render_text(&file)),
                Format::Json => println!("{}", d.render_json(&file)),
            }
        }
        if format == Format::Text && parsed.diagnostics.is_empty() {
            eprintln!("{path}: ok ({} declarations)", parsed.file.decls.len());
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
