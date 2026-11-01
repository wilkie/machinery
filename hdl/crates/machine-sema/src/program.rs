//! Modules, items and scopes: the first pass over a set of parsed files.
//!
//! A module is one source file, named by its file stem. Every top-level
//! declaration becomes an item with a [`DefId`]; module scopes map names to
//! items, and `use` declarations add imports. An `isa` and a `bus` are also
//! namespaces: their nested declarations are items too, visible by bare name
//! inside and as `Ns::Name` outside. No bodies are examined here.

use std::collections::HashMap;
use std::path::Path;

use machine_syntax::ast::{self, BusItem, Decl, IsaItem};
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
    // Nested in an `isa`:
    /// `mode Name : u1 { ... }`: an enum that is also the isa's `mode`
    /// register.
    Mode,
    Space,
    Register,
    Fault,
    Fn,
    Operand,
    OperandKind,
    Instruction,
    Latch,
    // Nested in a `bus`:
    Transaction,
    Signal,
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
            ItemKind::Mode => "mode",
            ItemKind::Space => "address space",
            ItemKind::Register => "register",
            ItemKind::Fault => "fault",
            ItemKind::Fn => "function",
            ItemKind::Operand => "operand",
            ItemKind::OperandKind => "operand kind",
            ItemKind::Instruction => "instruction",
            ItemKind::Latch => "latch",
            ItemKind::Transaction => "transaction",
            ItemKind::Signal => "signal",
        }
    }

    /// Items usable as types.
    pub fn is_type(self) -> bool {
        matches!(
            self,
            ItemKind::Enum
                | ItemKind::Bundle
                | ItemKind::Union
                | ItemKind::Mode
                | ItemKind::Operand
        )
    }

    /// Enum-like items (`enum` and `mode`).
    pub fn is_enum(self) -> bool {
        matches!(self, ItemKind::Enum | ItemKind::Mode)
    }
}

/// Where an item's syntax lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemDecl {
    /// Index into the module's `ast.decls`.
    Top(usize),
    /// Index into the `items` of the enclosing `isa` declaration.
    Isa { isa: DefId, index: usize },
    /// Index into the `items` of the enclosing `bus` declaration.
    Bus { bus: DefId, index: usize },
}

#[derive(Clone, Debug)]
pub struct Item {
    pub module: ModuleId,
    pub decl: ItemDecl,
    pub name: String,
    pub kind: ItemKind,
    pub span: Span,
}

/// What a name inside a namespace (isa or bus) scope denotes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeEntry {
    Item(DefId),
    /// A register field, visible by bare name (`AL`) and as `AX.AL`.
    RegField {
        reg: DefId,
        index: u32,
    },
    /// A register view, visible as `FLAGS.arith` (and by bare name).
    RegView {
        reg: DefId,
        index: u32,
    },
    /// The isa's `mode` register.
    Mode(DefId),
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
    /// Scope of each namespace item (`isa` or `bus`).
    pub ns_scopes: HashMap<DefId, HashMap<String, ScopeEntry>>,
}

fn isa_item_meta(item: &IsaItem) -> Option<(String, ItemKind, Span)> {
    Some(match item {
        IsaItem::Space(d) => (d.name.name.clone(), ItemKind::Space, d.name.span),
        IsaItem::Register(d) => (d.name.name.clone(), ItemKind::Register, d.name.span),
        IsaItem::Mode(d) => (d.name.name.clone(), ItemKind::Mode, d.name.span),
        IsaItem::Fault(d) => (d.name.name.clone(), ItemKind::Fault, d.name.span),
        IsaItem::Fn(d) => (d.name.name.clone(), ItemKind::Fn, d.name.span),
        IsaItem::Operand(d) => (d.name.name.clone(), ItemKind::Operand, d.name.span),
        IsaItem::OperandKind(d) => (d.name.name.clone(), ItemKind::OperandKind, d.name.span),
        IsaItem::Instruction(d) => (d.name.name.clone(), ItemKind::Instruction, d.name.span),
        IsaItem::Enum(d) => (d.name.name.clone(), ItemKind::Enum, d.name.span),
        IsaItem::Bundle(d) => (d.name.name.clone(), ItemKind::Bundle, d.name.span),
        IsaItem::Const(d) => (d.name.name.clone(), ItemKind::Const, d.name.span),
        IsaItem::Latch(d) => (d.name.name.clone(), ItemKind::Latch, d.name.span),
        IsaItem::Istream(_) => return None,
    })
}

/// Bus items that become named items (wires, roles, clock and params are
/// structural and live in the bus's own info).
fn bus_item_meta(item: &BusItem) -> Option<(String, ItemKind, Span)> {
    match item {
        BusItem::Enum(d) => Some((d.name.name.clone(), ItemKind::Enum, d.name.span)),
        BusItem::Transaction(d) => Some((d.name.name.clone(), ItemKind::Transaction, d.name.span)),
        BusItem::Signal(d) => Some((d.name.name.clone(), ItemKind::Signal, d.name.span)),
        _ => None,
    }
}

/// Insert a name into a namespace scope, reporting duplicates.
#[allow(clippy::too_many_arguments)]
fn define(
    scope: &mut HashMap<String, ScopeEntry>,
    ns_kind: &str,
    ns_name: &str,
    name: &str,
    entry: ScopeEntry,
    span: Span,
    items: &[Item],
    diags: &mut Diagnostics,
) {
    if let Some(prev) = scope.insert(name.to_string(), entry) {
        let mut d = Diagnostic::error(
            "E1001",
            span,
            format!("`{name}` is defined more than once in {ns_kind} `{ns_name}`"),
        );
        if let ScopeEntry::Item(p) = prev {
            d = d.with_label(items[p.0 as usize].span, "first definition here");
        }
        diags.push(d);
    }
}

impl Program {
    /// Parse the files, collect items and resolve imports. Returns the
    /// program and one diagnostics sink per module (parse diagnostics
    /// included).
    pub fn collect(files: Vec<SourceFile>) -> (Program, Vec<Diagnostics>) {
        let mut modules = Vec::new();
        let mut diagnostics = Vec::new();
        let mut items: Vec<Item> = Vec::new();
        let mut ns_scopes: HashMap<DefId, HashMap<String, ScopeEntry>> = HashMap::new();

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

        // Pass 1: items and scopes.
        for m in 0..modules.len() {
            let mut module_scope: HashMap<String, DefId> = HashMap::new();
            let decls = &modules[m].ast.decls;
            for (i, decl) in decls.iter().enumerate() {
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
                    decl: ItemDecl::Top(i),
                    name: name.clone(),
                    kind,
                    span,
                });
                // Tests, vectors and conformance blocks are anonymous; they
                // are items but not names.
                if !matches!(
                    kind,
                    ItemKind::Test | ItemKind::Vectors | ItemKind::Conformance
                ) {
                    if let Some(prev) = module_scope.insert(name.clone(), id) {
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

                // Nested isa items.
                if let Decl::Isa(isa) = decl {
                    let mut scope: HashMap<String, ScopeEntry> = HashMap::new();
                    for (j, item) in isa.items.iter().enumerate() {
                        let Some((iname, ikind, ispan)) = isa_item_meta(item) else {
                            continue;
                        };
                        let nested_id = DefId(items.len() as u32);
                        items.push(Item {
                            module: m,
                            decl: ItemDecl::Isa { isa: id, index: j },
                            name: iname.clone(),
                            kind: ikind,
                            span: ispan,
                        });
                        define(
                            &mut scope,
                            "isa",
                            &isa.name.name,
                            &iname,
                            ScopeEntry::Item(nested_id),
                            ispan,
                            &items,
                            &mut diagnostics[m],
                        );
                        match item {
                            IsaItem::Register(r) => {
                                for (fi, f) in r.fields.iter().enumerate() {
                                    let e = ScopeEntry::RegField {
                                        reg: nested_id,
                                        index: fi as u32,
                                    };
                                    define(
                                        &mut scope,
                                        "isa",
                                        &isa.name.name,
                                        &f.name.name,
                                        e,
                                        f.name.span,
                                        &items,
                                        &mut diagnostics[m],
                                    );
                                }
                                for (vi, v) in r.views.iter().enumerate() {
                                    let e = ScopeEntry::RegView {
                                        reg: nested_id,
                                        index: vi as u32,
                                    };
                                    define(
                                        &mut scope,
                                        "isa",
                                        &isa.name.name,
                                        &v.name.name,
                                        e,
                                        v.name.span,
                                        &items,
                                        &mut diagnostics[m],
                                    );
                                }
                            }
                            IsaItem::Mode(md) => {
                                define(
                                    &mut scope,
                                    "isa",
                                    &isa.name.name,
                                    "mode",
                                    ScopeEntry::Mode(id),
                                    md.name.span,
                                    &items,
                                    &mut diagnostics[m],
                                );
                            }
                            _ => {}
                        }
                    }
                    ns_scopes.insert(id, scope);
                }

                // Nested bus items.
                if let Decl::Bus(bus) = decl {
                    let mut scope: HashMap<String, ScopeEntry> = HashMap::new();
                    for (j, item) in bus.items.iter().enumerate() {
                        let Some((iname, ikind, ispan)) = bus_item_meta(item) else {
                            continue;
                        };
                        let nested_id = DefId(items.len() as u32);
                        items.push(Item {
                            module: m,
                            decl: ItemDecl::Bus { bus: id, index: j },
                            name: iname.clone(),
                            kind: ikind,
                            span: ispan,
                        });
                        define(
                            &mut scope,
                            "bus",
                            &bus.name.name,
                            &iname,
                            ScopeEntry::Item(nested_id),
                            ispan,
                            &items,
                            &mut diagnostics[m],
                        );
                    }
                    ns_scopes.insert(id, scope);
                }
            }
            modules[m].scope = module_scope;
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

        (
            Program {
                modules,
                items,
                ns_scopes,
            },
            diagnostics,
        )
    }

    pub fn item(&self, id: DefId) -> &Item {
        &self.items[id.0 as usize]
    }

    /// The enclosing `isa` of a nested item.
    pub fn parent_isa(&self, id: DefId) -> Option<DefId> {
        match self.item(id).decl {
            ItemDecl::Isa { isa, .. } => Some(isa),
            _ => None,
        }
    }

    /// The enclosing `bus` of a nested item.
    pub fn parent_bus(&self, id: DefId) -> Option<DefId> {
        match self.item(id).decl {
            ItemDecl::Bus { bus, .. } => Some(bus),
            _ => None,
        }
    }

    /// The top-level declaration of an item. Panics for nested items; use
    /// the typed accessors for those.
    pub fn decl(&self, id: DefId) -> &Decl {
        let item = self.item(id);
        match item.decl {
            ItemDecl::Top(i) => &self.modules[item.module].ast.decls[i],
            _ => panic!("item `{}` is nested in a namespace", item.name),
        }
    }

    pub fn isa_decl(&self, id: DefId) -> &ast::IsaDecl {
        match self.decl(id) {
            Decl::Isa(d) => d,
            _ => panic!("not an isa"),
        }
    }

    pub fn bus_decl(&self, id: DefId) -> &ast::BusDecl {
        match self.decl(id) {
            Decl::Bus(d) => d,
            _ => panic!("not a bus"),
        }
    }

    pub fn component_decl(&self, id: DefId) -> &ast::ComponentDecl {
        match self.decl(id) {
            Decl::Component(d) => d,
            _ => panic!("not a component"),
        }
    }

    /// The syntax of a nested isa item.
    pub fn isa_item(&self, id: DefId) -> Option<&IsaItem> {
        match self.item(id).decl {
            ItemDecl::Isa { isa, index } => Some(&self.isa_decl(isa).items[index]),
            _ => None,
        }
    }

    /// The syntax of a nested bus item.
    pub fn bus_item(&self, id: DefId) -> Option<&BusItem> {
        match self.item(id).decl {
            ItemDecl::Bus { bus, index } => Some(&self.bus_decl(bus).items[index]),
            _ => None,
        }
    }

    pub fn enum_decl(&self, id: DefId) -> &ast::EnumDecl {
        match self.item(id).decl {
            ItemDecl::Top(_) => match self.decl(id) {
                Decl::Enum(d) => d,
                _ => panic!("not an enum"),
            },
            ItemDecl::Isa { .. } => match self.isa_item(id).unwrap() {
                IsaItem::Enum(d) | IsaItem::Mode(d) => d,
                _ => panic!("not an enum"),
            },
            ItemDecl::Bus { .. } => match self.bus_item(id).unwrap() {
                BusItem::Enum(d) => d,
                _ => panic!("not an enum"),
            },
        }
    }

    pub fn bundle_decl(&self, id: DefId) -> &ast::BundleDecl {
        match self.item(id).decl {
            ItemDecl::Top(_) => match self.decl(id) {
                Decl::Bundle(d) => d,
                _ => panic!("not a bundle"),
            },
            ItemDecl::Isa { .. } => match self.isa_item(id).unwrap() {
                IsaItem::Bundle(d) => d,
                _ => panic!("not a bundle"),
            },
            ItemDecl::Bus { .. } => panic!("buses have no nested bundles"),
        }
    }

    pub fn const_decl(&self, id: DefId) -> &ast::ConstDecl {
        match self.item(id).decl {
            ItemDecl::Top(_) => match self.decl(id) {
                Decl::Const(d) => d,
                _ => panic!("not a const"),
            },
            ItemDecl::Isa { .. } => match self.isa_item(id).unwrap() {
                IsaItem::Const(d) => d,
                _ => panic!("not a const"),
            },
            ItemDecl::Bus { .. } => panic!("buses have no nested consts"),
        }
    }

    pub fn space_decl(&self, id: DefId) -> &ast::SpaceDecl {
        match self.isa_item(id) {
            Some(IsaItem::Space(d)) => d,
            _ => panic!("not a space"),
        }
    }

    pub fn register_decl(&self, id: DefId) -> &ast::RegisterDecl {
        match self.isa_item(id) {
            Some(IsaItem::Register(d)) => d,
            _ => panic!("not a register"),
        }
    }

    pub fn fault_decl(&self, id: DefId) -> &ast::FaultDecl {
        match self.isa_item(id) {
            Some(IsaItem::Fault(d)) => d,
            _ => panic!("not a fault"),
        }
    }

    pub fn fn_decl(&self, id: DefId) -> &ast::FnDecl {
        match self.isa_item(id) {
            Some(IsaItem::Fn(d)) => d,
            _ => panic!("not a fn"),
        }
    }

    pub fn operand_decl(&self, id: DefId) -> &ast::OperandDecl {
        match self.isa_item(id) {
            Some(IsaItem::Operand(d)) => d,
            _ => panic!("not an operand"),
        }
    }

    pub fn operand_kind_decl(&self, id: DefId) -> &ast::OperandKindDecl {
        match self.isa_item(id) {
            Some(IsaItem::OperandKind(d)) => d,
            _ => panic!("not an operand kind"),
        }
    }

    pub fn instruction_decl(&self, id: DefId) -> &ast::InstructionDecl {
        match self.isa_item(id) {
            Some(IsaItem::Instruction(d)) => d,
            _ => panic!("not an instruction"),
        }
    }

    pub fn latch_decl(&self, id: DefId) -> &ast::LatchDecl {
        match self.isa_item(id) {
            Some(IsaItem::Latch(d)) => d,
            _ => panic!("not a latch"),
        }
    }

    pub fn transaction_decl(&self, id: DefId) -> &ast::TransactionDecl {
        match self.bus_item(id) {
            Some(BusItem::Transaction(d)) => d,
            _ => panic!("not a transaction"),
        }
    }

    pub fn signal_decl(&self, id: DefId) -> &ast::SignalDecl {
        match self.bus_item(id) {
            Some(BusItem::Signal(d)) => d,
            _ => panic!("not a signal"),
        }
    }

    pub fn module_of(&self, id: DefId) -> &Module {
        &self.modules[self.item(id).module]
    }

    /// Look a single name up in a module: own items first, then imports.
    pub fn lookup(&self, module: ModuleId, name: &str) -> Option<DefId> {
        let m = &self.modules[module];
        m.scope.get(name).or_else(|| m.imports.get(name)).copied()
    }

    /// Look a name up inside a namespace (isa or bus) scope.
    pub fn lookup_in_ns(&self, ns: DefId, name: &str) -> Option<ScopeEntry> {
        self.ns_scopes.get(&ns).and_then(|s| s.get(name)).copied()
    }

    /// Display name of an item.
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

    /// Nested items of an isa, of one kind, in definition order.
    pub fn isa_items_of(&self, isa: DefId, kind: ItemKind) -> impl Iterator<Item = DefId> + '_ {
        self.items
            .iter()
            .enumerate()
            .filter(move |(_, it)| {
                it.kind == kind && matches!(it.decl, ItemDecl::Isa { isa: p, .. } if p == isa)
            })
            .map(|(i, _)| DefId(i as u32))
    }

    /// Nested items of a bus, of one kind, in definition order.
    pub fn bus_items_of(&self, bus: DefId, kind: ItemKind) -> impl Iterator<Item = DefId> + '_ {
        self.items
            .iter()
            .enumerate()
            .filter(move |(_, it)| {
                it.kind == kind && matches!(it.decl, ItemDecl::Bus { bus: p, .. } if p == bus)
            })
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
