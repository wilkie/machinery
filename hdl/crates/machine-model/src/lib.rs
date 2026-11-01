//! `machine-model`: generates a Rust interpreter from an isa.
//!
//! Input is the typed HIR of a checked program; output is Rust source for a
//! crate that depends on `machine-runtime`. The generated crate contains:
//!
//! - every enum (as a module of constants), bundle (as a struct) and unit
//!   instance (as a function) the isa reaches;
//! - the architectural state (`Regs`, `mode`, one `Memory` per address
//!   space), faults, operand structs with their tails, operand-kind
//!   functions, isa `fn`s, one `exec_*` method per instruction, a decoder,
//!   and `step()` with fault rollback;
//! - one `#[test]` per `test ... on <Isa>` block.

mod gen;

use std::fmt;
use std::path::Path;

use machine_sema::program::{DefId, ItemKind, Program};
use machine_sema::Checked;

pub use gen::GenError;

/// Generate the interpreter crate source for the isa named `isa_name`.
pub fn generate_isa(checked: &Checked, isa_name: &str) -> Result<String, GenError> {
    let prog = &checked.program;
    let isa = find_isa(prog, isa_name)?;
    gen::generate(prog, &checked.hir, isa)
}

fn find_isa(prog: &Program, name: &str) -> Result<DefId, GenError> {
    let mut found = None;
    for id in prog.items_of(ItemKind::Isa) {
        if prog.item_name(id) == name {
            found = Some(id);
        }
    }
    found.ok_or_else(|| {
        let known: Vec<String> = prog
            .items_of(ItemKind::Isa)
            .map(|i| prog.item_name(i))
            .collect();
        GenError::new(format!(
            "no isa named `{name}` (known: {})",
            known.join(", ")
        ))
    })
}

/// Files of a generated crate.
#[derive(Debug, Clone)]
pub struct GeneratedCrate {
    pub cargo_toml: String,
    pub lib_rs: String,
}

/// Assemble a standalone crate around the generated source. `runtime_path`
/// is the path to the `machine-runtime` crate the generated code links
/// against.
pub fn package(name: &str, lib_rs: String, runtime_path: &Path) -> GeneratedCrate {
    let cargo_toml = format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[lib]\npath = \"src/lib.rs\"\n\n[dependencies]\nmachine-runtime = {{ path = {:?} }}\n\n[workspace]\n",
        runtime_path.display().to_string()
    );
    GeneratedCrate { cargo_toml, lib_rs }
}

/// Write a generated crate to `dir` (created if needed).
pub fn write_crate(dir: &Path, crate_: &GeneratedCrate) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::write(dir.join("Cargo.toml"), &crate_.cargo_toml)?;
    std::fs::write(dir.join("src").join("lib.rs"), &crate_.lib_rs)?;
    Ok(())
}

/// A crate name from an isa name: lower-case, `_`-separated.
pub fn crate_name(isa: &str) -> String {
    let mut out = String::new();
    for (i, c) in isa.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else if c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    format!("machine_{out}")
}

impl fmt::Display for GeneratedCrate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.lib_rs)
    }
}
