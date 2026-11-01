//! Modules, items and scopes: the first pass over a set of parsed files.
//!
//! A module is one source file, named by its file stem. Every top-level
//! declaration becomes an item with a [`DefId`]; module scopes map names to
//! items, and `use` declarations add imports. No bodies are examined here.

use std::collections::HashMap;
use std::path::Path;

use machine_syntax::ast::{self, Decl};
use machine_syntax::diagnostics::{Diagnostic, Diagnostics};
use machine_syntax::span::Span;
use machine_syntax::{parse, SourceFile};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DefId(pub u32);

pub type ModuleId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Enum,
    Bundle,
    Union,
    Const,
    Unit,
    Component,
    Bus,
    Isa,
    System,
    Test,
    Vectors,
    Conformance,
}

impl ItemKind {
    pub fn describe(self) -> &'static str {
        match self {
            ItemKind::Enum => "enum",
            ItemKind::Bundle => "bundle",
            ItemKind::Union => "union",
            ItemKind::Const => "const",
            ItemKind::Unit => "unit",
            ItemKind::Component => "component",
            ItemKind::Bus => "bus",
            ItemKind::Isa => "isa",
            ItemKind::System => "system",
            ItemKind::Test => "test",
            ItemKind::Vectors => "vectors",
            ItemKind::Conformance => "conformance",
        }
    }

    pub fn is_type(self) -> bool {
        matches!(self, ItemKind::Enum | ItemKind::Bundle | ItemKind::Union)
    }
}

#[derive(Clone, Debug)]
pub struct Item {
    pub module: ModuleId,
    /// Index into the module's `ast.decls`.
    pub decl: usize,
    pub name: String,
    pub kind: ItemKind,
    pub span: Span,
}

pub struct Module {
    pub name: String,
    pub file: SourceFile,
    pub ast: ast::File,
    /// Items declared in this module.
    pub scope: HashMap<String, DefId>,
    /// Items imported with `use`.
    pub imports: HashMap<String, DefId>,
}

pub struct Program {
    pub modules: Vec<Module>,
    pub items: Vec<Item>,
}

impl Program {
    /// Parse the files, collect items and resolve imports. Returns the
    /// program and one diagnostics sink per module (parse diagnostics
    /// included).
    pub fn collect(files: Vec<SourceFile>) -> (Program, Vec<Diagnostics>) {
        let mut modules = Vec::new();
        let mut diagnostics = Vec::new();
        let mut items = Vec::new();

        for file in files {
            let parsed = parse(&file.text);
            let name = module_name(&file.name);
            diagnostics.push(parsed.diagnostics);
            modules.push(Module {
                name,
                file,
                ast: parsed.file,
                scope: HashMap::new(),
                imports: HashMap::new(),
            });
        }

        // Pass 1: items and module scopes.
        for (m, module) in modules.iter_mut().enumerate() {
            for (i, decl) in module.ast.decls.iter().enumerate() {
                let (name, kind, span) = match decl {
                    Decl::Enum(d) => (d.name.name.clone(), ItemKind::Enum, d.name.span),
                    Decl::Bundle(d) => (d.name.name.clone(), ItemKind::Bundle, d.name.span),
                    Decl::Union(d) => (d.name.name.clone(), ItemKind::Union, d.name.span),
                    Decl::Const(d) => (d.name.name.clone(), ItemKind::Const, d.name.span),
                    Decl::Unit(d) => (d.name.name.clone(), ItemKind::Unit, d.name.span),
                    Decl::Component(d) => (d.name.name.clone(), ItemKind::Component, d.name.span),
                    Decl::Bus(d) => (d.name.name.clone(), ItemKind::Bus, d.name.span),
                    Decl::Isa(d) => (d.name.name.clone(), ItemKind::Isa, d.name.span),
                    Decl::System(d) => (d.name.name.clone(), ItemKind::System, d.name.span),
                    Decl::Test(d) => (
                        format!("test {:?}", d.name.value),
                        ItemKind::Test,
                        d.name.span,
                    ),
                    Decl::Vectors(d) => (
                        format!("{:?}", d.glob.value),
                        ItemKind::Vectors,
                        d.glob.span,
                    ),
                    Decl::Conformance(d) => {
                        (d.target.dotted(), ItemKind::Conformance, d.target.span)
                    }
                    Decl::Use(_) | Decl::Error(_) => continue,
                };
                let id = DefId(items.len() as u32);
                items.push(Item {
                    module: m,
                    decl: i,
                    name: name.clone(),
                    kind,
                    span,
                });
                // Tests, vectors and conformance blocks are anonymous; they
                // are items but not names.
                if matches!(
                    kind,
                    ItemKind::Test | ItemKind::Vectors | ItemKind::Conformance
                ) {
                    continue;
                }
                if let Some(prev) = module.scope.insert(name.clone(), id) {
                    let prev_span = items[prev.0 as usize].span;
                    diagnostics[m].push(
                        Diagnostic::error(
                            "E1001",
                            span,
                            format!("`{name}` is defined more than once in this module"),
                        )
                        .with_label(prev_span, "first definition here"),
                    );
                }
            }
        }

        // Pass 2: imports.
        let by_name: HashMap<String, ModuleId> = modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.name.clone(), i))
            .collect();
        for m in 0..modules.len() {
            let mut imports: Vec<(String, DefId, Span)> = Vec::new();
            for decl in &modules[m].ast.decls {
                let Decl::Use(u) = decl else { continue };
                let (module_path, single) = if u.items.is_empty() {
                    // `use a::b::Item;` → module `a::b`, item `Item`.
                    let n = u.path.segments.len();
                    if n < 2 {
                        diagnostics[m].push(Diagnostic::error(
                            "E1002",
                            u.span,
                            "`use` needs a module and an item: `use alu::AluOp;` or `use alu::{A, B};`",
                        ));
                        continue;
                    }
                    (&u.path.segments[..n - 1], Some(&u.path.segments[n - 1]))
                } else {
                    (&u.path.segments[..], None)
                };
                if module_path.len() != 1 {
                    diagnostics[m].push(Diagnostic::error(
                        "E1002",
                        u.path.span,
                        "nested module paths are not supported; a module is one file named by its stem",
                    ));
                    continue;
                }
                let mod_name = &module_path[0].name;
                let Some(&target) = by_name.get(mod_name) else {
                    diagnostics[m].push(
                        Diagnostic::error("E1002", module_path[0].span, format!("unknown module `{mod_name}`"))
                            .with_note(format!("modules are files; `{mod_name}.machine` was not among the files being checked")),
                    );
                    continue;
                };
                if target == m {
                    diagnostics[m].push(Diagnostic::error(
                        "E1002",
                        u.span,
                        "a module cannot import itself",
                    ));
                    continue;
                }
                let wanted: Vec<&ast::Ident> = match single {
                    Some(s) => vec![s],
                    None => u.items.iter().collect(),
                };
                for ident in wanted {
                    match modules[target].scope.get(&ident.name) {
                        Some(&def) => imports.push((ident.name.clone(), def, ident.span)),
                        None => diagnostics[m].push(Diagnostic::error(
                            "E1002",
                            ident.span,
                            format!("module `{mod_name}` has no item `{}`", ident.name),
                        )),
                    }
                }
            }
            for (name, def, span) in imports {
                if let Some(&local) = modules[m].scope.get(&name) {
                    let local_span = items[local.0 as usize].span;
                    diagnostics[m].push(
                        Diagnostic::error(
                            "E1001",
                            span,
                            format!("import `{name}` conflicts with a definition in this module"),
                        )
                        .with_label(local_span, "defined here"),
                    );
                    continue;
                }
                modules[m].imports.insert(name, def);
            }
        }

        (Program { modules, items }, diagnostics)
    }

    pub fn item(&self, id: DefId) -> &Item {
        &self.items[id.0 as usize]
    }

    pub fn decl(&self, id: DefId) -> &Decl {
        let item = self.item(id);
        &self.modules[item.module].ast.decls[item.decl]
    }

    pub fn module_of(&self, id: DefId) -> &Module {
        &self.modules[self.item(id).module]
    }

    /// Look a single name up in a module: own items first, then imports.
    pub fn lookup(&self, module: ModuleId, name: &str) -> Option<DefId> {
        let m = &self.modules[module];
        m.scope.get(name).or_else(|| m.imports.get(name)).copied()
    }

    /// `Enum` or `module::Enum`-free display name of an item.
    pub fn item_name(&self, id: DefId) -> String {
        self.item(id).name.clone()
    }

    /// Items of a kind, in definition order.
    pub fn items_of(&self, kind: ItemKind) -> impl Iterator<Item = DefId> + '_ {
        self.items
            .iter()
            .enumerate()
            .filter(move |(_, it)| it.kind == kind)
            .map(|(i, _)| DefId(i as u32))
    }
}

/// `path/to/alu.machine` → `alu`.
pub fn module_name(file_name: &str) -> String {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);
    stem.to_string()
}
