//! The checker: resolves types, evaluates constants, instantiates units,
//! checks isas, and type-checks bodies into HIR.

mod bus;
mod component;
mod expr;
mod isa;
mod pattern;
mod stmt;

pub use component::{CompEntry, CompScope};

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use machine_syntax::ast::{self, Decl};
use machine_syntax::diagnostics::{Diagnostic, Diagnostics};
use machine_syntax::span::Span;

use crate::consteval::{self, ConstEnv, ConstError, ConstVal};
use crate::hir::{self, LocalId, UnitInstId};
use crate::program::{DefId, ItemKind, ModuleId, Program, ScopeEntry};
use crate::ty::{Ty, TyDisplay};
use crate::Skipped;

/// Resolved facts about an `enum` (or `mode`).
#[derive(Debug)]
pub struct EnumInfo {
    pub width: u32,
    /// `(name, encoding)` in declaration order.
    pub variants: Vec<(String, i128)>,
}

#[derive(Debug)]
pub struct FieldInfo {
    pub name: String,
    pub ty: Ty,
    pub offset: Option<u32>,
    pub default: Option<ConstVal>,
}

#[derive(Debug)]
pub struct BundleInfo {
    pub fields: Vec<FieldInfo>,
}

#[derive(Debug)]
pub struct UnionInfo {
    pub variants: Vec<(String, Option<Ty>)>,
}

#[derive(Debug, Clone)]
pub struct ConstInfo {
    pub ty: Ty,
    pub value: ConstVal,
}

/// Memoisation state for lazily resolved items (cycle detection included).
enum Lazy<T> {
    InProgress,
    Done(Rc<T>),
    Failed,
}

/// The signature of a unit instance, available before its body is checked.
#[derive(Clone, Debug)]
pub struct UnitSig {
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
}

/// The signature of an isa `fn`.
#[derive(Clone, Debug)]
pub struct FnSig {
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
}

/// `(name, kind, parameter types)` of one operand-kind rule.
pub type RuleSig = (String, hir::RuleKind, Vec<(String, Ty)>);

/// The signature part of an operand kind: what callers need before the
/// rule bodies are checked.
#[derive(Clone, Debug)]
pub struct KindSig {
    pub ty: Ty,
    pub source: Option<DefId>,
    pub rules: Vec<RuleSig>,
}

impl KindSig {
    pub fn rule(&self, kind: hir::RuleKind) -> Option<(usize, &RuleSig)> {
        self.rules.iter().enumerate().find(|(_, r)| r.1 == kind)
    }
}

/// The signature part of an instruction, for tests.
#[derive(Clone, Debug)]
pub struct InstrSig {
    pub operands: Vec<(String, DefId)>,
}

/// Where names are looked up: a module, optionally inside an isa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope {
    pub module: ModuleId,
    pub isa: Option<DefId>,
    pub bus: Option<DefId>,
}

/// Per-body checking state.
pub struct FnCtx {
    pub scope: Scope,
    pub module: ModuleId,
    pub generics: HashMap<String, i128>,
    pub locals: Vec<hir::Local>,
    scopes: Vec<HashMap<String, LocalId>>,
    /// `Some` in a unit/fn body; `None` in a test body (no `return`).
    pub ret: Option<Ty>,
    pub in_test: bool,
    /// Instruction operands in scope: `(name, kind)`.
    pub operands: Vec<(String, DefId)>,
    /// The component whose model body this is.
    pub component: Option<DefId>,
    /// Nesting depth of `while`/`loop`, for `break`/`continue`.
    pub loop_depth: u32,
}

impl FnCtx {
    fn new(scope: Scope, generics: HashMap<String, i128>, ret: Option<Ty>) -> FnCtx {
        FnCtx {
            scope,
            module: scope.module,
            generics,
            locals: Vec::new(),
            scopes: vec![HashMap::new()],
            ret,
            in_test: false,
            operands: Vec::new(),
            component: None,
            loop_depth: 0,
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare(&mut self, name: &str, ty: Ty, span: Span) -> LocalId {
        let id = self.locals.len() as LocalId;
        self.locals.push(hir::Local {
            name: name.to_string(),
            ty,
            span,
        });
        self.scopes
            .last_mut()
            .expect("scope")
            .insert(name.to_string(), id);
        id
    }

    pub fn lookup_local(&self, name: &str) -> Option<LocalId> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }

    pub fn lookup_operand(&self, name: &str) -> Option<u32> {
        self.operands
            .iter()
            .position(|(n, _)| n == name)
            .map(|i| i as u32)
    }

    /// Is `name` bound by anything local (local, generic, operand)?
    pub fn shadows(&self, name: &str) -> bool {
        self.lookup_local(name).is_some()
            || self.generics.contains_key(name)
            || self.lookup_operand(name).is_some()
    }
}

pub struct Checker<'p> {
    pub prog: &'p Program,
    diags: &'p mut Vec<Diagnostics>,
    enums: HashMap<DefId, Lazy<EnumInfo>>,
    bundles: HashMap<DefId, Lazy<BundleInfo>>,
    unions: HashMap<DefId, Lazy<UnionInfo>>,
    consts: HashMap<DefId, Lazy<ConstInfo>>,
    unit_ids: HashMap<(DefId, Vec<i128>), UnitInstId>,
    /// Units somebody tried to instantiate (even unsuccessfully).
    unit_attempts: HashSet<DefId>,
    unit_sigs: Vec<UnitSig>,
    /// `None` while the body is being checked (recursion detection).
    unit_insts: Vec<Option<hir::UnitInst>>,
    tests: Vec<hir::TestBody>,
    skipped: Vec<Skipped>,
    // --- isa ---
    pub(crate) spaces: HashMap<DefId, Rc<hir::Space>>,
    pub(crate) registers: HashMap<DefId, Rc<hir::Register>>,
    pub(crate) faults: HashMap<DefId, Rc<hir::Fault>>,
    pub(crate) operands: HashMap<DefId, Rc<hir::Operand>>,
    pub(crate) kinds: HashMap<DefId, Rc<KindSig>>,
    fn_sigs: HashMap<DefId, Lazy<FnSig>>,
    pub(crate) instr_sigs: HashMap<DefId, Rc<InstrSig>>,
    /// The mode enum of each isa.
    pub(crate) isa_modes: HashMap<DefId, DefId>,
    isas: Vec<hir::IsaInfo>,
    // --- buses and components ---
    pub(crate) buses: HashMap<DefId, Rc<hir::BusInfo>>,
    pub(crate) comps: HashMap<DefId, Rc<CompScope>>,
    bus_infos: Vec<hir::BusInfo>,
    components: Vec<hir::ComponentInfo>,
}

/// Check every item of a program.
pub fn check_program(prog: &Program, diags: &mut Vec<Diagnostics>) -> (hir::Hir, Vec<Skipped>) {
    let mut ck = Checker {
        prog,
        diags,
        enums: HashMap::new(),
        bundles: HashMap::new(),
        unions: HashMap::new(),
        consts: HashMap::new(),
        unit_ids: HashMap::new(),
        unit_attempts: HashSet::new(),
        unit_sigs: Vec::new(),
        unit_insts: Vec::new(),
        tests: Vec::new(),
        skipped: Vec::new(),
        spaces: HashMap::new(),
        registers: HashMap::new(),
        faults: HashMap::new(),
        operands: HashMap::new(),
        kinds: HashMap::new(),
        fn_sigs: HashMap::new(),
        instr_sigs: HashMap::new(),
        isa_modes: HashMap::new(),
        isas: Vec::new(),
        buses: HashMap::new(),
        comps: HashMap::new(),
        bus_infos: Vec::new(),
        components: Vec::new(),
    };

    for id in prog
        .items_of(ItemKind::Enum)
        .chain(prog.items_of(ItemKind::Mode))
    {
        ck.enum_info(id);
    }
    for id in prog.items_of(ItemKind::Const) {
        ck.const_info(id);
    }
    for id in prog.items_of(ItemKind::Bundle) {
        ck.bundle_info(id);
    }
    for id in prog.items_of(ItemKind::Union) {
        ck.union_info(id);
    }
    for id in prog.items_of(ItemKind::Bus) {
        if let Some(info) = ck.check_bus(id) {
            let rc = Rc::new(info);
            ck.buses.insert(id, rc.clone());
            ck.bus_infos.push((*rc).clone());
        }
    }
    for id in prog.items_of(ItemKind::Isa) {
        ck.check_isa(id);
    }
    for id in prog.items_of(ItemKind::Component) {
        if let Some(info) = ck.check_component(id) {
            ck.components.push(info);
        }
    }
    for id in prog.items_of(ItemKind::Unit) {
        let Decl::Unit(u) = prog.decl(id) else {
            unreachable!()
        };
        if u.generics.is_empty() {
            ck.instantiate_unit(id, Vec::new(), u.name.span, prog.item(id).module);
        }
    }
    for id in prog.items_of(ItemKind::Test) {
        ck.check_test(id);
    }
    for id in prog.items_of(ItemKind::Unit) {
        let Decl::Unit(u) = prog.decl(id) else {
            unreachable!()
        };
        if !u.generics.is_empty() && !ck.unit_attempts.contains(&id) {
            let module = prog.item(id).module;
            ck.warning(
                module,
                "W1000",
                u.name.span,
                format!(
                    "generic unit `{}` is never instantiated, so its body is unchecked",
                    u.name.name
                ),
            );
        }
    }
    for item in &prog.items {
        let kind = match item.kind {
            ItemKind::System => "system",
            ItemKind::Vectors => "vectors",
            ItemKind::Conformance => "conformance",
            _ => continue,
        };
        ck.skipped.push(Skipped {
            module: item.module,
            kind,
            name: item.name.clone(),
        });
    }

    let units = ck
        .unit_insts
        .into_iter()
        .map(|u| u.expect("every unit instance is finished after checking"))
        .collect();
    let mut enums: Vec<hir::EnumDef> = ck
        .enums
        .iter()
        .filter_map(|(id, l)| match l {
            Lazy::Done(info) => Some(hir::EnumDef {
                def: *id,
                name: prog.item_name(*id),
                width: info.width,
                variants: info.variants.clone(),
            }),
            _ => None,
        })
        .collect();
    enums.sort_by_key(|e| e.def);
    let mut bundles: Vec<hir::BundleDef> = ck
        .bundles
        .iter()
        .filter_map(|(id, l)| match l {
            Lazy::Done(info) => Some(hir::BundleDef {
                def: *id,
                name: prog.item_name(*id),
                fields: info
                    .fields
                    .iter()
                    .map(|f| hir::BundleFieldDef {
                        name: f.name.clone(),
                        ty: f.ty.clone(),
                        offset: f.offset,
                    })
                    .collect(),
            }),
            _ => None,
        })
        .collect();
    bundles.sort_by_key(|b| b.def);
    (
        hir::Hir {
            units,
            tests: ck.tests,
            isas: ck.isas,
            buses: ck.bus_infos,
            components: ck.components,
            enums,
            bundles,
        },
        ck.skipped,
    )
}

impl<'p> Checker<'p> {
    // -- diagnostics ------------------------------------------------------------

    pub fn error(
        &mut self,
        module: ModuleId,
        code: &'static str,
        span: Span,
        msg: impl Into<String>,
    ) {
        self.diags[module].push(Diagnostic::error(code, span, msg));
    }

    pub fn error_with(&mut self, module: ModuleId, d: Diagnostic) {
        self.diags[module].push(d);
    }

    pub fn warning(
        &mut self,
        module: ModuleId,
        code: &'static str,
        span: Span,
        msg: impl Into<String>,
    ) {
        self.diags[module].push(Diagnostic::warning(code, span, msg));
    }

    pub fn display(&self, ty: &Ty) -> String {
        let prog = self.prog;
        let names = move |d: DefId| prog.item_name(d);
        TyDisplay { ty, names: &names }.to_string()
    }

    pub fn mismatch(&mut self, module: ModuleId, span: Span, expected: &Ty, found: &Ty) {
        if expected.is_error() || found.is_error() {
            return;
        }
        let msg = format!(
            "type mismatch: expected `{}`, found `{}`",
            self.display(expected),
            self.display(found)
        );
        let mut d = Diagnostic::error("E1005", span, msg);
        if found.is_integer() && expected.is_integer() {
            d = d.with_note("there is no implicit widening or narrowing; use `as` to convert");
        }
        if matches!(
            (expected, found),
            (Ty::Bool, Ty::UInt(1)) | (Ty::UInt(1), Ty::Bool)
        ) {
            d = d.with_note("`bool` and `u1` are distinct; convert with `as bool` / `as u1`");
        }
        self.error_with(module, d);
    }

    // -- scopes -----------------------------------------------------------------

    /// The scope an item's own declaration is checked in.
    pub fn scope_of(&self, id: DefId) -> Scope {
        Scope {
            module: self.prog.item(id).module,
            isa: self.prog.parent_isa(id),
            bus: self.prog.parent_bus(id),
        }
    }

    /// Resolve a bare name: isa scope first (when inside one), then the
    /// module.
    pub fn lookup(&self, scope: Scope, name: &str) -> Option<ScopeEntry> {
        if let Some(isa) = scope.isa {
            if let Some(e) = self.prog.lookup_in_ns(isa, name) {
                return Some(e);
            }
        }
        if let Some(bus) = scope.bus {
            if let Some(e) = self.prog.lookup_in_ns(bus, name) {
                return Some(e);
            }
        }
        self.prog.lookup(scope.module, name).map(ScopeEntry::Item)
    }

    /// Resolve a bare name to an item (register fields and the like are not
    /// items).
    pub fn lookup_item(&self, scope: Scope, name: &str) -> Option<DefId> {
        match self.lookup(scope, name) {
            Some(ScopeEntry::Item(d)) => Some(d),
            _ => None,
        }
    }

    // -- item resolution --------------------------------------------------------

    pub fn enum_info(&mut self, id: DefId) -> Option<Rc<EnumInfo>> {
        match self.enums.get(&id) {
            Some(Lazy::Done(info)) => return Some(info.clone()),
            Some(Lazy::Failed) => return None,
            Some(Lazy::InProgress) => {
                let item = self.prog.item(id);
                let (module, span, name) = (item.module, item.span, item.name.clone());
                self.error(
                    module,
                    "E1013",
                    span,
                    format!("enum `{name}` depends on itself"),
                );
                return None;
            }
            None => {}
        }
        self.enums.insert(id, Lazy::InProgress);
        let result = self.compute_enum(id);
        match result {
            Some(info) => {
                let rc = Rc::new(info);
                self.enums.insert(id, Lazy::Done(rc.clone()));
                Some(rc)
            }
            None => {
                self.enums.insert(id, Lazy::Failed);
                None
            }
        }
    }

    fn compute_enum(&mut self, id: DefId) -> Option<EnumInfo> {
        let scope = self.scope_of(id);
        let module = scope.module;
        let decl = self.prog.enum_decl(id);
        let empty = HashMap::new();
        let width = match self.resolve_type(scope, &empty, &decl.repr) {
            Ty::UInt(w) => w,
            Ty::Error => return None,
            other => {
                let msg = format!(
                    "enum backing type must be `u<N>`, found `{}`",
                    self.display(&other)
                );
                self.error(module, "E1003", decl.repr.span, msg);
                return None;
            }
        };
        let mut variants: Vec<(String, i128)> = Vec::new();
        let mut next: i128 = 0;
        let mut ok = true;
        for v in &decl.variants {
            let value = match &v.value {
                Some(e) => match self.eval_const(scope, &empty, e) {
                    Some(ConstVal::Int(x)) => x,
                    Some(ConstVal::Bool(_)) => {
                        self.error(
                            module,
                            "E1023",
                            e.span,
                            "enum variant value must be an integer",
                        );
                        ok = false;
                        next
                    }
                    None => {
                        ok = false;
                        next
                    }
                },
                None => next,
            };
            if !Ty::UInt(width).fits(value) {
                self.error(
                    module,
                    "E1023",
                    v.span,
                    format!(
                        "variant `{}` has value {value}, which does not fit in u{width}",
                        v.name.name
                    ),
                );
                ok = false;
            }
            if let Some((prev, _)) = variants.iter().find(|(_, x)| *x == value) {
                self.error(
                    module,
                    "E1023",
                    v.span,
                    format!(
                        "variant `{}` has the same value ({value}) as `{prev}`",
                        v.name.name
                    ),
                );
                ok = false;
            }
            if variants.iter().any(|(n, _)| *n == v.name.name) {
                self.error(
                    module,
                    "E1001",
                    v.name.span,
                    format!("duplicate variant `{}`", v.name.name),
                );
                ok = false;
            }
            variants.push((v.name.name.clone(), value));
            next = value + 1;
        }
        if variants.is_empty() {
            self.error(
                module,
                "E1023",
                decl.span,
                "an enum needs at least one variant",
            );
            ok = false;
        }
        ok.then_some(EnumInfo { width, variants })
    }

    pub fn bundle_info(&mut self, id: DefId) -> Option<Rc<BundleInfo>> {
        match self.bundles.get(&id) {
            Some(Lazy::Done(info)) => return Some(info.clone()),
            Some(Lazy::Failed) => return None,
            Some(Lazy::InProgress) => {
                // A bundle containing itself (directly or through another
                // bundle) has infinite width.
                let item = self.prog.item(id);
                let (module, span, name) = (item.module, item.span, item.name.clone());
                self.error(
                    module,
                    "E1013",
                    span,
                    format!("bundle `{name}` contains itself"),
                );
                return None;
            }
            None => {}
        }
        self.bundles.insert(id, Lazy::InProgress);
        let scope = self.scope_of(id);
        let module = scope.module;
        let decl = self.prog.bundle_decl(id);
        let empty = HashMap::new();
        let mut fields = Vec::new();
        let mut ok = true;
        for f in &decl.fields {
            let ty = self.resolve_type(scope, &empty, &f.ty);
            if ty.is_error() {
                ok = false;
            }
            // Force nested bundles now so self-containment is detected.
            if let Ty::Bundle(inner) = &ty {
                if self.bundle_info(*inner).is_none() {
                    ok = false;
                }
            }
            let offset = match &f.offset {
                Some(e) => match self.eval_const(scope, &empty, e) {
                    Some(ConstVal::Int(v)) if v >= 0 => Some(v as u32),
                    Some(_) => {
                        self.error(
                            module,
                            "E1013",
                            e.span,
                            "field offset must be a non-negative integer",
                        );
                        None
                    }
                    None => None,
                },
                None => None,
            };
            let default = match &f.default {
                Some(e) => self.eval_const(scope, &empty, e),
                None => None,
            };
            if fields.iter().any(|x: &FieldInfo| x.name == f.name.name) {
                self.error(
                    module,
                    "E1001",
                    f.name.span,
                    format!("duplicate field `{}`", f.name.name),
                );
                ok = false;
            }
            fields.push(FieldInfo {
                name: f.name.name.clone(),
                ty,
                offset,
                default,
            });
        }
        if ok {
            let rc = Rc::new(BundleInfo { fields });
            self.bundles.insert(id, Lazy::Done(rc.clone()));
            Some(rc)
        } else {
            self.bundles.insert(id, Lazy::Failed);
            None
        }
    }

    pub fn union_info(&mut self, id: DefId) -> Option<Rc<UnionInfo>> {
        match self.unions.get(&id) {
            Some(Lazy::Done(info)) => return Some(info.clone()),
            Some(Lazy::Failed) => return None,
            Some(Lazy::InProgress) => {
                let item = self.prog.item(id);
                let (module, span, name) = (item.module, item.span, item.name.clone());
                self.error(
                    module,
                    "E1013",
                    span,
                    format!("union `{name}` contains itself"),
                );
                return None;
            }
            None => {}
        }
        self.unions.insert(id, Lazy::InProgress);
        let scope = self.scope_of(id);
        let module = scope.module;
        let Decl::Union(decl) = self.prog.decl(id) else {
            unreachable!()
        };
        let empty = HashMap::new();
        let mut variants = Vec::new();
        let mut ok = true;
        for v in &decl.variants {
            let ty = match &v.payload {
                Some(t) => {
                    let ty = self.resolve_type(scope, &empty, t);
                    if ty.is_error() {
                        ok = false;
                    }
                    Some(ty)
                }
                None => None,
            };
            if variants
                .iter()
                .any(|(n, _): &(String, Option<Ty>)| *n == v.name.name)
            {
                self.error(
                    module,
                    "E1001",
                    v.name.span,
                    format!("duplicate variant `{}`", v.name.name),
                );
                ok = false;
            }
            variants.push((v.name.name.clone(), ty));
        }
        if ok {
            let rc = Rc::new(UnionInfo { variants });
            self.unions.insert(id, Lazy::Done(rc.clone()));
            Some(rc)
        } else {
            self.unions.insert(id, Lazy::Failed);
            None
        }
    }

    pub fn const_info(&mut self, id: DefId) -> Option<Rc<ConstInfo>> {
        match self.consts.get(&id) {
            Some(Lazy::Done(info)) => return Some(info.clone()),
            Some(Lazy::Failed) => return None,
            Some(Lazy::InProgress) => {
                let item = self.prog.item(id);
                let (module, span, name) = (item.module, item.span, item.name.clone());
                self.error(
                    module,
                    "E1013",
                    span,
                    format!("const `{name}` depends on itself"),
                );
                return None;
            }
            None => {}
        }
        self.consts.insert(id, Lazy::InProgress);
        let scope = self.scope_of(id);
        let module = scope.module;
        let decl = self.prog.const_decl(id);
        let empty = HashMap::new();
        let ty = self.resolve_type(scope, &empty, &decl.ty);
        let value = self.eval_const(scope, &empty, &decl.value);
        let result = match (&ty, value) {
            (Ty::Error, _) | (_, None) => None,
            (Ty::UInt(_) | Ty::SInt(_), Some(ConstVal::Int(v))) => {
                if ty.fits(v) {
                    Some(ConstInfo {
                        ty: ty.clone(),
                        value: ConstVal::Int(v),
                    })
                } else {
                    let msg = format!("constant value {v} does not fit in `{}`", self.display(&ty));
                    self.error(module, "E1006", decl.value.span, msg);
                    None
                }
            }
            (Ty::Bool, Some(ConstVal::Bool(b))) => Some(ConstInfo {
                ty: Ty::Bool,
                value: ConstVal::Bool(b),
            }),
            (Ty::Enum(e), Some(ConstVal::Int(v))) => {
                let valid = self
                    .enum_info(*e)
                    .map(|i| i.variants.iter().any(|(_, x)| *x == v))
                    .unwrap_or(false);
                if valid {
                    Some(ConstInfo {
                        ty: ty.clone(),
                        value: ConstVal::Int(v),
                    })
                } else {
                    self.error(
                        module,
                        "E1005",
                        decl.value.span,
                        "value is not a variant of the declared enum",
                    );
                    None
                }
            }
            (_, Some(_)) => {
                let msg = format!(
                    "constant of type `{}` must be an integer, bool or enum",
                    self.display(&ty)
                );
                self.error(module, "E1005", decl.ty.span, msg);
                None
            }
        };
        match result {
            Some(info) => {
                let rc = Rc::new(info);
                self.consts.insert(id, Lazy::Done(rc.clone()));
                Some(rc)
            }
            None => {
                self.consts.insert(id, Lazy::Failed);
                None
            }
        }
    }

    /// Signature of an isa `fn`, resolved on first use.
    pub fn fn_sig(&mut self, id: DefId) -> Option<Rc<FnSig>> {
        match self.fn_sigs.get(&id) {
            Some(Lazy::Done(s)) => return Some(s.clone()),
            Some(Lazy::Failed) => return None,
            Some(Lazy::InProgress) => return None,
            None => {}
        }
        self.fn_sigs.insert(id, Lazy::InProgress);
        let scope = self.scope_of(id);
        let decl = self.prog.fn_decl(id);
        let empty = HashMap::new();
        let mut params = Vec::new();
        let mut ok = true;
        for p in &decl.params {
            let ty = self.resolve_type(scope, &empty, &p.ty);
            ok &= !ty.is_error();
            params.push((p.name.name.clone(), ty));
        }
        let ret = match &decl.ret {
            Some(t) => self.resolve_type(scope, &empty, t),
            None => Ty::Unit,
        };
        ok &= !ret.is_error();
        if ok {
            let rc = Rc::new(FnSig { params, ret });
            self.fn_sigs.insert(id, Lazy::Done(rc.clone()));
            Some(rc)
        } else {
            self.fn_sigs.insert(id, Lazy::Failed);
            None
        }
    }

    // -- types -------------------------------------------------------------------

    /// Resolve a syntactic type in a scope with the given generic bindings.
    pub fn resolve_type(
        &mut self,
        scope: Scope,
        generics: &HashMap<String, i128>,
        ty: &ast::Type,
    ) -> Ty {
        let module = scope.module;
        match &ty.kind {
            ast::TypeKind::Unsigned(w) | ast::TypeKind::Signed(w) => {
                let signed = matches!(ty.kind, ast::TypeKind::Signed(_));
                match self.eval_const(scope, generics, w) {
                    Some(ConstVal::Int(v)) if (1..=128).contains(&v) => {
                        if signed {
                            Ty::SInt(v as u32)
                        } else {
                            Ty::UInt(v as u32)
                        }
                    }
                    Some(ConstVal::Int(v)) => {
                        self.error(
                            module,
                            "E1004",
                            w.span,
                            format!("width must be between 1 and 128, got {v}"),
                        );
                        Ty::Error
                    }
                    Some(ConstVal::Bool(_)) => {
                        self.error(module, "E1004", w.span, "width must be an integer");
                        Ty::Error
                    }
                    None => Ty::Error,
                }
            }
            ast::TypeKind::Bool => Ty::Bool,
            ast::TypeKind::Width => {
                self.error(
                    module,
                    "E1003",
                    ty.span,
                    "`width` is the kind of const parameters, not a value type",
                );
                Ty::Error
            }
            ast::TypeKind::Named(path, args) => {
                if !args.is_empty() {
                    self.error(
                        module,
                        "E1003",
                        ty.span,
                        "types do not take generic arguments",
                    );
                    return Ty::Error;
                }
                let def = match path.segments.len() {
                    1 => {
                        let name = &path.segments[0].name;
                        match self.lookup(scope, name) {
                            Some(ScopeEntry::Item(d)) => d,
                            Some(_) => {
                                self.error(
                                    module,
                                    "E1003",
                                    path.span,
                                    format!("`{name}` is a register field, not a type"),
                                );
                                return Ty::Error;
                            }
                            None => {
                                self.error(
                                    module,
                                    "E1000",
                                    path.span,
                                    format!("unknown type `{name}`"),
                                );
                                return Ty::Error;
                            }
                        }
                    }
                    2 => {
                        // `Isa::Type`
                        let (first, second) = (&path.segments[0], &path.segments[1]);
                        match self.lookup_item(scope, &first.name) {
                            Some(isa) if self.prog.item(isa).kind == ItemKind::Isa => {
                                match self.prog.lookup_in_ns(isa, &second.name) {
                                    Some(ScopeEntry::Item(d)) => d,
                                    _ => {
                                        self.error(
                                            module,
                                            "E1000",
                                            second.span,
                                            format!(
                                                "isa `{}` has no type `{}`",
                                                first.name, second.name
                                            ),
                                        );
                                        return Ty::Error;
                                    }
                                }
                            }
                            _ => {
                                self.error(
                                    module,
                                    "E1003",
                                    path.span,
                                    format!("`{}` is not a type here (namespaced types such as bus roles are not supported in this context)", path.dotted()),
                                );
                                return Ty::Error;
                            }
                        }
                    }
                    _ => {
                        self.error(
                            module,
                            "E1003",
                            path.span,
                            format!("`{}` is not a type", path.dotted()),
                        );
                        return Ty::Error;
                    }
                };
                let name = path.last().name.clone();
                match self.prog.item(def).kind {
                    ItemKind::Enum | ItemKind::Mode => Ty::Enum(def),
                    ItemKind::Bundle => Ty::Bundle(def),
                    ItemKind::Union => Ty::Union(def),
                    ItemKind::Operand => Ty::Operand(def),
                    kind => {
                        self.error(
                            module,
                            "E1003",
                            path.span,
                            format!("`{name}` is a {}, not a type", kind.describe()),
                        );
                        Ty::Error
                    }
                }
            }
            ast::TypeKind::Array(elem, len) => {
                let elem_ty = self.resolve_type(scope, generics, elem);
                match self.eval_const(scope, generics, len) {
                    Some(ConstVal::Int(n)) if n >= 0 => Ty::Array(Box::new(elem_ty), n as u64),
                    Some(_) => {
                        self.error(
                            module,
                            "E1013",
                            len.span,
                            "array length must be a non-negative integer",
                        );
                        Ty::Error
                    }
                    None => Ty::Error,
                }
            }
            ast::TypeKind::Tuple(elems) => {
                let named = elems.iter().filter(|e| e.name.is_some()).count();
                if named != 0 && named != elems.len() {
                    self.error(
                        module,
                        "E1003",
                        ty.span,
                        "a tuple type is either fully named or fully positional",
                    );
                    return Ty::Error;
                }
                let mut out = Vec::new();
                for e in elems {
                    let t = self.resolve_type(scope, generics, &e.ty);
                    if t.is_error() {
                        return Ty::Error;
                    }
                    out.push((e.name.as_ref().map(|n| n.name.clone()), t));
                }
                Ty::Tuple(out)
            }
        }
    }

    /// Width in bits of a value of this type, if it has one.
    pub fn width_of(&mut self, ty: &Ty) -> Option<u32> {
        Some(match ty {
            Ty::UInt(w) | Ty::SInt(w) => *w,
            Ty::Bool => 1,
            Ty::Enum(d) => self.enum_info(*d)?.width,
            Ty::Bundle(d) => {
                let info = self.bundle_info(*d)?;
                let mut total = 0u32;
                let mut max_end = 0u32;
                for f in &info.fields {
                    let w = self.width_of(&f.ty)?;
                    match f.offset {
                        Some(off) => max_end = max_end.max(off + w),
                        None => {
                            total += w;
                            max_end = max_end.max(total);
                        }
                    }
                }
                max_end
            }
            Ty::Union(d) => {
                let info = self.union_info(*d)?;
                let tag = crate::builtins::bits_for(info.variants.len().saturating_sub(1) as u64);
                let mut payload = 0;
                for (_, t) in &info.variants {
                    if let Some(t) = t {
                        payload = payload.max(self.width_of(t)?);
                    }
                }
                tag + payload
            }
            Ty::Array(t, n) => self.width_of(t)? * (*n as u32),
            Ty::Tuple(items) => {
                let mut total = 0;
                for (_, t) in items {
                    total += self.width_of(t)?;
                }
                total
            }
            Ty::Operand(d) => (self.operands.get(d)?.size * 8) as u32,
            Ty::OperandTail(_) | Ty::Const(_) | Ty::Str | Ty::Unit | Ty::Never | Ty::Error => {
                return None
            }
        })
    }

    // -- const evaluation ------------------------------------------------------------

    /// Evaluate a constant expression, reporting failure.
    pub fn eval_const(
        &mut self,
        scope: Scope,
        generics: &HashMap<String, i128>,
        e: &ast::Expr,
    ) -> Option<ConstVal> {
        let mut env = CEnv {
            ck: self,
            scope,
            generics,
        };
        match consteval::eval(e, &mut env) {
            Ok(v) => Some(v),
            Err(err) => {
                if !err.reported {
                    self.error(scope.module, "E1013", err.span, err.message);
                }
                None
            }
        }
    }

    /// Try to evaluate without reporting; `None` means "not a constant".
    pub fn try_eval_const(
        &mut self,
        scope: Scope,
        generics: &HashMap<String, i128>,
        e: &ast::Expr,
    ) -> Option<ConstVal> {
        let mut env = CEnv {
            ck: self,
            scope,
            generics,
        };
        consteval::eval(e, &mut env).ok()
    }

    // -- units -------------------------------------------------------------------------

    /// Instantiate (and, the first time, check) `unit def<args>`. Returns the
    /// instance id and signature, or `None` after reporting.
    pub fn instantiate_unit(
        &mut self,
        def: DefId,
        args: Vec<i128>,
        use_span: Span,
        use_module: ModuleId,
    ) -> Option<(UnitInstId, UnitSig)> {
        let Decl::Unit(decl) = self.prog.decl(def) else {
            unreachable!()
        };
        let def_scope = self.scope_of(def);
        let def_module = def_scope.module;
        self.unit_attempts.insert(def);
        if decl.generics.len() != args.len() {
            let msg = format!(
                "unit `{}` takes {} generic argument(s), {} given",
                decl.name.name,
                decl.generics.len(),
                args.len()
            );
            self.error(use_module, "E1016", use_span, msg);
            return None;
        }
        let key = (def, args.clone());
        if let Some(&id) = self.unit_ids.get(&key) {
            if self.unit_insts[id.0 as usize].is_none() {
                let msg = format!(
                    "recursive call to unit `{}`; units may not recurse",
                    decl.name.name
                );
                self.error(use_module, "E1014", use_span, msg);
                return None;
            }
            return Some((id, self.unit_sigs[id.0 as usize].clone()));
        }

        let mut generics = HashMap::new();
        for (g, v) in decl.generics.iter().zip(&args) {
            match g.kind.kind {
                ast::TypeKind::Width => {
                    if !(1..=128).contains(v) {
                        let msg = format!(
                            "generic argument `{}` must be a width between 1 and 128, got {v}",
                            g.name.name
                        );
                        self.error(use_module, "E1004", use_span, msg);
                        return None;
                    }
                }
                _ => {
                    self.error(
                        def_module,
                        "E1003",
                        g.kind.span,
                        "only `width` const parameters are supported",
                    );
                    return None;
                }
            }
            generics.insert(g.name.name.clone(), *v);
        }

        let mut params = Vec::new();
        for p in &decl.params {
            let ty = self.resolve_type(def_scope, &generics, &p.ty);
            params.push((p.name.name.clone(), ty));
        }
        let ret = self.resolve_type(def_scope, &generics, &decl.ret);
        let sig = UnitSig { params, ret };

        let id = UnitInstId(self.unit_insts.len() as u32);
        self.unit_ids.insert(key, id);
        self.unit_sigs.push(sig.clone());
        self.unit_insts.push(None);

        let mut cx = FnCtx::new(def_scope, generics, Some(sig.ret.clone()));
        let mut param_ids = Vec::new();
        for (p, (name, ty)) in decl.params.iter().zip(&sig.params) {
            if cx.lookup_local(name).is_some() {
                self.error(
                    def_module,
                    "E1001",
                    p.name.span,
                    format!("duplicate parameter `{name}`"),
                );
            }
            param_ids.push(cx.declare(name, ty.clone(), p.name.span));
        }
        let body = self.check_block(&mut cx, &decl.body);
        if !matches!(sig.ret, Ty::Unit | Ty::Error) && !stmt::block_diverges(&body) {
            let msg = format!(
                "unit `{}` does not return a value on every path",
                decl.name.name
            );
            self.error_with(
                def_module,
                Diagnostic::error("E1015", decl.name.span, msg).with_note(
                    "every path through the body must end in `return` or `unreachable(...)`",
                ),
            );
        }
        self.unit_insts[id.0 as usize] = Some(hir::UnitInst {
            def,
            args,
            locals: cx.locals,
            params: param_ids,
            ret: sig.ret.clone(),
            body,
        });
        Some((id, sig))
    }

    // -- tests -----------------------------------------------------------------------------

    fn check_test(&mut self, id: DefId) {
        let module = self.prog.item(id).module;
        let Decl::Test(t) = self.prog.decl(id) else {
            unreachable!()
        };
        let mut isa = None;
        if let Some(target) = &t.target {
            let resolved = if target.segments.len() == 1 {
                self.prog.lookup(module, &target.segments[0].name)
            } else {
                None
            };
            match resolved {
                Some(def) if self.prog.item(def).kind == ItemKind::Isa => isa = Some(def),
                Some(def)
                    if target.segments.len() == 1
                        && self.prog.item(def).kind != ItemKind::Component =>
                {
                    let msg = format!(
                        "`{}` is a {}; tests run on an isa or on `Component.rtl` / `Component.model`",
                        target.dotted(),
                        self.prog.item(def).kind.describe()
                    );
                    self.error(module, "E1000", target.span, msg);
                    return;
                }
                _ => {
                    self.skipped.push(Skipped {
                        module,
                        kind: "test",
                        name: format!("{:?} on {}", t.name.value, target.dotted()),
                    });
                    return;
                }
            }
        }
        let scope = Scope {
            module,
            isa,
            bus: None,
        };
        let mut cx = FnCtx::new(scope, HashMap::new(), None);
        cx.in_test = true;
        let body = self.check_block(&mut cx, &t.body);
        self.tests.push(hir::TestBody {
            def: id,
            name: t.name.value.clone(),
            isa,
            locals: cx.locals,
            body,
        });
    }
}

/// Const-evaluation environment backed by the checker.
struct CEnv<'a, 'p> {
    ck: &'a mut Checker<'p>,
    scope: Scope,
    generics: &'a HashMap<String, i128>,
}

impl ConstEnv for CEnv<'_, '_> {
    fn lookup_name(&mut self, name: &str, span: Span) -> Result<ConstVal, ConstError> {
        if let Some(v) = self.generics.get(name) {
            return Ok(ConstVal::Int(*v));
        }
        match self.ck.lookup(self.scope, name) {
            Some(ScopeEntry::Item(def)) if self.ck.prog.item(def).kind == ItemKind::Const => {
                match self.ck.const_info(def) {
                    Some(info) => Ok(info.value),
                    None => Err(ConstError {
                        span,
                        message: format!("constant `{name}` could not be evaluated"),
                        reported: true,
                    }),
                }
            }
            Some(ScopeEntry::Item(def)) => Err(ConstError {
                span,
                message: format!(
                    "`{name}` is a {}, not a constant",
                    self.ck.prog.item(def).kind.describe()
                ),
                reported: false,
            }),
            Some(_) => Err(ConstError {
                span,
                message: format!("`{name}` is architectural state, not a constant"),
                reported: false,
            }),
            None => Err(ConstError {
                span,
                message: format!("unknown constant `{name}`"),
                reported: false,
            }),
        }
    }

    fn lookup_variant(
        &mut self,
        enum_name: &str,
        variant: &str,
        span: Span,
    ) -> Result<ConstVal, ConstError> {
        let Some(def) = self.ck.lookup_item(self.scope, enum_name) else {
            return Err(ConstError {
                span,
                message: format!("unknown enum `{enum_name}`"),
                reported: false,
            });
        };
        if !self.ck.prog.item(def).kind.is_enum() {
            return Err(ConstError {
                span,
                message: format!("`{enum_name}` is not an enum"),
                reported: false,
            });
        }
        let Some(info) = self.ck.enum_info(def) else {
            return Err(ConstError {
                span,
                message: format!("enum `{enum_name}` could not be resolved"),
                reported: true,
            });
        };
        match info.variants.iter().find(|(n, _)| n == variant) {
            Some((_, v)) => Ok(ConstVal::Int(*v)),
            None => Err(ConstError {
                span,
                message: format!("enum `{enum_name}` has no variant `{variant}`"),
                reported: false,
            }),
        }
    }

    fn cast_width(&mut self, ty: &ast::Type) -> Result<(bool, u32), ConstError> {
        match self.ck.resolve_type(self.scope, self.generics, ty) {
            Ty::UInt(w) => Ok((false, w)),
            Ty::SInt(w) => Ok((true, w)),
            Ty::Error => Err(ConstError {
                span: ty.span,
                message: String::new(),
                reported: true,
            }),
            _ => Err(ConstError {
                span: ty.span,
                message: "constant casts must target `u<N>` or `i<N>`".into(),
                reported: false,
            }),
        }
    }
}
