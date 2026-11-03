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
//!
//! For a `system`, the crate also holds one struct per component model and
//! `system::System`, which runs the processor over a board of device models
//! (see `gen/system.rs`).

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

/// Generate the crate source for the system named `system_name`: its
/// processor's interpreter, every component model, and the system. With a
/// `profile`, check that it selects the model level everywhere.
pub fn generate_system(
    checked: &Checked,
    system_name: &str,
    profile: Option<&str>,
) -> Result<String, GenError> {
    let prog = &checked.program;
    let found = prog
        .items_of(ItemKind::System)
        .find(|i| prog.item_name(*i) == system_name);
    let Some(sys) = found else {
        let known: Vec<String> = prog
            .items_of(ItemKind::System)
            .map(|i| prog.item_name(i))
            .collect();
        return Err(GenError::new(format!(
            "no system named `{system_name}` (known: {})",
            known.join(", ")
        )));
    };
    gen::generate_system(prog, &checked.hir, sys, profile)
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
    /// `src/bin/vectors.rs`, when the isa has `vectors`.
    pub vectors_bin: Option<String>,
}

/// Assemble a standalone crate around the generated source. `runtime_path`
/// is the path to the `machine-runtime` crate the generated code links
/// against.
pub fn package(name: &str, lib_rs: String, runtime_path: &Path) -> GeneratedCrate {
    let has_vectors = lib_rs.contains("pub mod vectors {");
    let mut cargo_toml = format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[lib]\npath = \"src/lib.rs\"\n\n[dependencies]\nmachine-runtime = {{ path = {:?} }}\n\n[features]\nvectors = [\"machine-runtime/vectors\"]\n",
        runtime_path.display().to_string()
    );
    let vectors_bin = if has_vectors {
        cargo_toml.push_str("\n[[bin]]\nname = \"vectors\"\npath = \"src/bin/vectors.rs\"\nrequired-features = [\"vectors\"]\n");
        Some(format!("fn main() {{\n    std::process::exit(if {name}::vectors::main_entry() {{ 0 }} else {{ 1 }});\n}}\n"))
    } else {
        None
    };
    // Link-time optimisation: the processor (generic over its bus) is
    // compiled where it is used, the board's bus and device models here;
    // without it no bus access from the processor can be inlined.
    cargo_toml.push_str("\n[profile.release]\ndebug = 1\nlto = \"fat\"\ncodegen-units = 1\n\n[workspace]\n");
    GeneratedCrate {
        cargo_toml,
        lib_rs,
        vectors_bin,
    }
}

/// Write a generated crate to `dir` (created if needed).
pub fn write_crate(dir: &Path, crate_: &GeneratedCrate) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::write(dir.join("Cargo.toml"), &crate_.cargo_toml)?;
    std::fs::write(dir.join("src").join("lib.rs"), &crate_.lib_rs)?;
    if let Some(bin) = &crate_.vectors_bin {
        std::fs::create_dir_all(dir.join("src").join("bin"))?;
        std::fs::write(dir.join("src").join("bin").join("vectors.rs"), bin)?;
    }
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
