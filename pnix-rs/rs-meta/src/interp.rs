//! Tree-walking interpreter for the Rust subset — the meta-circular oracle.
//!
//! It evaluates Rust by running `fn main`, capturing what `println!` writes to
//! stdout. The native tier (`native.rs`) hands the *same Rust source* to rustc
//! and reads its stdout; translation validation requires the two to agree.

use crate::ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    I64(i64),
    F64(f64),
    Char(char),
    Bool(bool),
    Unit,
    Str(Rc<String>),
    String(Rc<RefCell<String>>),
    PathBuf(Rc<String>),
    Command(Rc<RefCell<Vec<String>>>),
    ExitStatus(bool),
    Tuple(Rc<Vec<Val>>),
    Vec(Rc<RefCell<Vec<Val>>>),
    Box(Slot),
    RcVal(Slot),
    RefCellVal(Slot),
    HashMap(Rc<RefCell<Vec<(Val, Slot)>>>),
    HashEntry {
        map: Rc<RefCell<Vec<(Val, Slot)>>>,
        key: Rc<Val>,
    },
    Iter(Rc<RefCell<IterState>>),
    Struct {
        name: Rc<String>,
        fields: Rc<Vec<(String, Val)>>,
    },
    Enum {
        enum_name: Rc<String>,
        variant: Rc<String>,
        data: Rc<Vec<Val>>,
    },
    EnumCtor {
        enum_name: Rc<String>,
        variant: Rc<String>,
        arity: usize,
    },
    Function(Rc<String>),
    Closure(usize),
    Ref {
        slot: Slot,
        mutable: bool,
    },
    VecElemRef {
        vec: Rc<RefCell<Vec<Val>>>,
        index: usize,
        mutable: bool,
    },
}

type Slot = Rc<RefCell<Val>>;

#[derive(Clone, Debug, PartialEq)]
pub struct IterState {
    items: Vec<Val>,
    pos: usize,
}

impl Val {
    fn as_i64(&self, ctx: &str) -> Result<i64, String> {
        match self {
            Val::I64(n) => Ok(*n),
            other => Err(format!("{}: expected i64, got {}", ctx, other.kind())),
        }
    }
    fn as_bool(&self, ctx: &str) -> Result<bool, String> {
        match self {
            Val::Bool(b) => Ok(*b),
            other => Err(format!("{}: expected bool, got {}", ctx, other.kind())),
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Val::I64(_) => "i64",
            Val::F64(_) => "f64",
            Val::Char(_) => "char",
            Val::Bool(_) => "bool",
            Val::Unit => "()",
            Val::Str(_) => "&str",
            Val::String(_) => "String",
            Val::PathBuf(_) => "PathBuf",
            Val::Command(_) => "Command",
            Val::ExitStatus(_) => "ExitStatus",
            Val::Tuple(_) => "tuple",
            Val::Vec(_) => "Vec",
            Val::Box(_) => "Box",
            Val::RcVal(_) => "Rc",
            Val::RefCellVal(_) => "RefCell",
            Val::HashMap(_) => "HashMap",
            Val::HashEntry { .. } => "HashEntry",
            Val::Iter(_) => "Iter",
            Val::Struct { .. } => "struct",
            Val::Enum { .. } => "enum",
            Val::EnumCtor { .. } => "enum constructor",
            Val::Function(_) => "function",
            Val::Closure(_) => "closure",
            Val::Ref { .. } => "ref",
            Val::VecElemRef { .. } => "vec-element-ref",
        }
    }
    fn display(&self) -> String {
        match self {
            Val::I64(n) => n.to_string(),
            Val::F64(f) => format!("{}", f),
            Val::Char(ch) => ch.to_string(),
            Val::Bool(b) => b.to_string(),
            Val::Unit => "()".to_string(),
            Val::Str(s) => s.to_string(),
            Val::String(s) => s.borrow().clone(),
            Val::PathBuf(s) => s.to_string(),
            Val::Command(_) => "Command".to_string(),
            Val::ExitStatus(ok) => ok.to_string(),
            Val::Tuple(vs) => {
                let parts: Vec<String> = vs.iter().map(|v| v.display()).collect();
                format!("({})", parts.join(", "))
            }
            Val::Vec(vs) => {
                let parts: Vec<String> = vs.borrow().iter().map(|v| v.display()).collect();
                format!("[{}]", parts.join(", "))
            }
            Val::Box(v) => v.borrow().display(),
            Val::RcVal(v) => v.borrow().display(),
            Val::RefCellVal(v) => format!("RefCell({})", v.borrow().display()),
            Val::HashMap(m) => format!("HashMap(len={})", m.borrow().len()),
            Val::HashEntry { .. } => "HashEntry".to_string(),
            Val::Iter(iter) => {
                let iter = iter.borrow();
                format!("Iter(len={}, pos={})", iter.items.len(), iter.pos)
            }
            Val::Struct { name, fields } => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.display()))
                    .collect();
                format!("{} {{ {} }}", name, parts.join(", "))
            }
            Val::Enum {
                enum_name,
                variant,
                data,
            } => {
                if data.is_empty() {
                    format!("{}::{}", enum_name, variant)
                } else {
                    let parts: Vec<String> = data.iter().map(|v| v.display()).collect();
                    format!("{}::{}({})", enum_name, variant, parts.join(", "))
                }
            }
            Val::EnumCtor {
                enum_name, variant, ..
            } => format!("{}::{}", enum_name, variant),
            Val::Function(name) => format!("<fn:{}>", name),
            Val::Closure(id) => format!("<closure:{}>", id),
            Val::Ref { slot, .. } => slot.borrow().display(),
            Val::VecElemRef { vec, index, .. } => vec
                .borrow()
                .get(*index)
                .map(|v| v.display())
                .unwrap_or_else(|| "<dangling-vec-ref>".to_string()),
        }
    }
    fn debug_display(&self, ef: &HashMap<(String, String), Vec<String>>) -> String {
        self.debug_fmt(false, 0, ef)
    }
    /// `{:?}` (pretty=false) and `{:#?}` (pretty=true) rendering, matching
    /// rustc's derive(Debug): strings/chars quoted at every depth, containers
    /// and structs recurse with debug (not display) formatting, tuple structs
    /// render as `Name(a, b)`, struct-variant enums as `Name { k: v }` (field
    /// names looked up in `ef`), and enum variants without the type prefix. In
    /// pretty mode containers break one element per line, 4-space indented,
    /// with a trailing comma — byte-identical to rustc.
    fn debug_fmt(
        &self,
        pretty: bool,
        indent: usize,
        ef: &HashMap<(String, String), Vec<String>>,
    ) -> String {
        match self {
            Val::F64(f) => format!("{:?}", f),
            Val::Char(ch) => format!("{:?}", ch),
            Val::Str(s) => format!("{:?}", s),
            Val::String(s) => format!("{:?}", s.borrow().as_str()),
            Val::Box(slot) | Val::RcVal(slot) => slot.borrow().debug_fmt(pretty, indent, ef),
            Val::Ref { slot, .. } => slot.borrow().debug_fmt(pretty, indent, ef),
            Val::VecElemRef { vec, index, .. } => vec
                .borrow()
                .get(*index)
                .map(|v| v.debug_fmt(pretty, indent, ef))
                .unwrap_or_else(|| "\"<dangling-vec-ref>\"".to_string()),
            Val::Vec(vs) => {
                let items = vs.borrow();
                let parts: Vec<String> = items
                    .iter()
                    .map(|v| v.debug_fmt(pretty, indent + 1, ef))
                    .collect();
                debug_wrap("[", "]", &parts, pretty, indent)
            }
            Val::Tuple(vs) => {
                let parts: Vec<String> = vs
                    .iter()
                    .map(|v| v.debug_fmt(pretty, indent + 1, ef))
                    .collect();
                debug_wrap("(", ")", &parts, pretty, indent)
            }
            Val::Struct { name, fields } => {
                // A tuple struct's synthesized field keys are "0","1",... —
                // render those as `Name(v0, v1)`, others as `Name { k: v }`.
                let mut is_tuple = !fields.is_empty();
                let mut idx = 0;
                while idx < fields.len() {
                    if fields[idx].0 != idx.to_string() {
                        is_tuple = false;
                        break;
                    }
                    idx += 1;
                }
                if is_tuple {
                    let parts: Vec<String> = fields
                        .iter()
                        .map(|(_, v)| v.debug_fmt(pretty, indent + 1, ef))
                        .collect();
                    let open = format!("{}(", name);
                    debug_wrap(&open, ")", &parts, pretty, indent)
                } else {
                    let parts: Vec<String> = fields
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, v.debug_fmt(pretty, indent + 1, ef)))
                        .collect();
                    debug_wrap_fields(name, &parts, pretty, indent)
                }
            }
            // rustc's derive(Debug) prints an enum variant WITHOUT the type
            // prefix (`Some(1)`, not `Option::Some(1)`); a struct-variant renders
            // as `A { x: 1 }` (field names from `ef`), a tuple variant as `A(1)`.
            Val::Enum {
                enum_name,
                variant,
                data,
            } => {
                if data.is_empty() {
                    return variant.to_string();
                }
                let key = ((**enum_name).clone(), (**variant).clone());
                let named = match ef.get(&key) {
                    Some(fs) if !fs.is_empty() && fs.len() == data.len() => Some(fs),
                    _ => None,
                };
                match named {
                    Some(fs) => {
                        let mut parts: Vec<String> = Vec::new();
                        let mut i = 0;
                        while i < data.len() {
                            parts.push(format!(
                                "{}: {}",
                                fs[i],
                                data[i].debug_fmt(pretty, indent + 1, ef)
                            ));
                            i += 1;
                        }
                        let vname = variant.to_string();
                        debug_wrap_fields(&vname, &parts, pretty, indent)
                    }
                    None => {
                        let parts: Vec<String> = data
                            .iter()
                            .map(|v| v.debug_fmt(pretty, indent + 1, ef))
                            .collect();
                        let open = format!("{}(", variant);
                        debug_wrap(&open, ")", &parts, pretty, indent)
                    }
                }
            }
            other => other.display(),
        }
    }
}

fn debug_indent(n: usize) -> String {
    let mut s = String::new();
    let mut k = 0;
    while k < n * 4 {
        s.push(' ');
        k += 1;
    }
    s
}

/// Comma-join string parts. The self-host subset has no slice `join`, so we
/// build it with a while loop (works on both `Vec<String>` and `&[String]`).
fn join_comma(parts: &[String]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < parts.len() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&parts[i]);
        i += 1;
    }
    out
}

/// Wrap sequence-like debug parts in `open`..`close`. Non-pretty: `open a, b close`.
/// Pretty: one part per line, 4-space indented, trailing comma (rustc-exact).
fn debug_wrap(open: &str, close: &str, parts: &[String], pretty: bool, indent: usize) -> String {
    if parts.len() == 0 {
        return format!("{}{}", open, close);
    }
    if !pretty {
        return format!("{}{}{}", open, join_comma(parts), close);
    }
    let mut out = String::new();
    out.push_str(open);
    out.push('\n');
    let inner = debug_indent(indent + 1);
    let mut i = 0;
    while i < parts.len() {
        out.push_str(&inner);
        out.push_str(&parts[i]);
        out.push_str(",\n");
        i += 1;
    }
    out.push_str(&debug_indent(indent));
    out.push_str(close);
    out
}

/// Same as `debug_wrap` but for `Name { k: v, ... }` struct bodies.
fn debug_wrap_fields(name: &str, parts: &[String], pretty: bool, indent: usize) -> String {
    if parts.len() == 0 {
        return name.to_string();
    }
    if !pretty {
        return format!("{} {{ {} }}", name, join_comma(parts));
    }
    let mut out = String::new();
    out.push_str(name);
    out.push_str(" {\n");
    let inner = debug_indent(indent + 1);
    let mut i = 0;
    while i < parts.len() {
        out.push_str(&inner);
        out.push_str(&parts[i]);
        out.push_str(",\n");
        i += 1;
    }
    out.push_str(&debug_indent(indent));
    out.push('}');
    out
}

enum Signal {
    Error(String),
    Return(Val),
    Break(Option<String>, Val),
    Continue,
}

type R = Result<Val, Signal>;

fn err(msg: impl Into<String>) -> Signal {
    Signal::Error(msg.into())
}

fn signal_message(ctx: &str, sig: Signal) -> String {
    match sig {
        Signal::Error(e) => e,
        Signal::Return(_) => format!("interp: return in {}", ctx),
        Signal::Break(_, _) => format!("interp: break in {}", ctx),
        Signal::Continue => format!("interp: continue in {}", ctx),
    }
}

struct Scope<'p> {
    vars: RefCell<HashMap<String, Slot>>,
    parent: Option<&'p Scope<'p>>,
}

impl<'p> Scope<'p> {
    fn new(parent: Option<&'p Scope<'p>>) -> Scope<'p> {
        Scope {
            vars: RefCell::new(HashMap::new()),
            parent,
        }
    }
    fn define(&self, name: &str, v: Val) {
        self.define_slot(name, Rc::new(RefCell::new(v)));
    }
    fn define_slot(&self, name: &str, slot: Slot) {
        self.vars.borrow_mut().insert(name.to_string(), slot);
    }
    fn get(&self, name: &str) -> Option<Val> {
        self.get_slot(name).map(|slot| slot.borrow().clone())
    }
    fn get_slot(&self, name: &str) -> Option<Slot> {
        if let Some(slot) = self.vars.borrow().get(name) {
            return Some(slot.clone());
        }
        self.parent.and_then(|p| p.get_slot(name))
    }
    /// Assign to the nearest existing binding; returns false if undefined.
    fn set(&self, name: &str, v: Val) -> bool {
        if let Some(slot) = self.vars.borrow().get(name).cloned() {
            *slot.borrow_mut() = v;
            return true;
        }
        match self.parent {
            Some(p) => p.set(name, v),
            None => false,
        }
    }

    fn snapshot_values(&self) -> Vec<(String, Val)> {
        let mut out = match self.parent {
            Some(p) => p.snapshot_values(),
            None => Vec::new(),
        };
        for (name, slot) in self.vars.borrow().iter() {
            out.push((name.clone(), slot.borrow().clone()));
        }
        out
    }

    fn local_values(&self) -> Vec<(String, Val)> {
        self.vars
            .borrow()
            .iter()
            .map(|(name, slot)| (name.clone(), slot.borrow().clone()))
            .collect()
    }
}

#[derive(Clone)]
struct ClosureVal {
    params: Vec<Pattern>,
    body: Expr,
    captures: Vec<(String, Val)>,
}

pub struct Interp {
    funcs: HashMap<String, Func>,
    methods: HashMap<(String, String), Method>,
    globals: Vec<(String, Val)>,
    enum_variants: HashMap<(String, String), usize>,
    enum_named_fields: HashMap<(String, String), Vec<String>>,
    closures: RefCell<Vec<ClosureVal>>,
    out: RefCell<String>,
    /// Phase E3 eval trace: OFF by default (zero cost beyond one bool test);
    /// facets = bind, call, match arm, loop iteration, error (at run_main).
    /// Expression-level enter/exit is deliberately held (cost) — see plan.
    trace_on: bool,
    trace: RefCell<Vec<String>>,
    unit_structs: Vec<String>,
    tuple_structs: Vec<String>,
}

impl Interp {
    pub fn new(prog: &Program) -> Result<Interp, String> {
        let mut funcs = HashMap::new();
        for f in &prog.funcs {
            if funcs.insert(f.name.clone(), f.clone()).is_some() {
                return Err(format!("interp: duplicate function {}", f.name));
            }
        }
        let mut methods = HashMap::new();
        for imp in &prog.impls {
            let target = interp_impl_target_name(&imp.target)?;
            for m in &imp.methods {
                if methods
                    .insert((target.clone(), m.name.clone()), m.clone())
                    .is_some()
                {
                    return Err(format!("interp: duplicate method {}::{}", target, m.name));
                }
            }
            // Flatten trait default methods (those not overridden by this impl).
            if let Some(tname) = &imp.trait_name {
                for tr in &prog.traits {
                    if tr.name == *tname {
                        for dm in &tr.methods {
                            let mut overridden = false;
                            for m in &imp.methods {
                                if m.name == dm.name {
                                    overridden = true;
                                }
                            }
                            let key = (target.clone(), dm.name.clone());
                            if !overridden && methods.get(&key).is_none() {
                                methods.insert(key, dm.clone());
                            }
                        }
                    }
                }
            }
        }
        let mut enum_variants = HashMap::new();
        let mut enum_named_fields = HashMap::new();
        for e in &prog.enums {
            for v in &e.variants {
                let arity = if v.named_fields.is_empty() {
                    v.fields.len()
                } else {
                    enum_named_fields.insert(
                        (e.name.clone(), v.name.clone()),
                        v.named_fields
                            .iter()
                            .map(|(name, _)| name.clone())
                            .collect(),
                    );
                    v.named_fields.len()
                };
                enum_variants.insert((e.name.clone(), v.name.clone()), arity);
            }
        }
        // Built-in std::cmp::Ordering unit variants (see typeck).
        enum_variants.insert(("Ordering".to_string(), "Less".to_string()), 0);
        enum_variants.insert(("Ordering".to_string(), "Equal".to_string()), 0);
        enum_variants.insert(("Ordering".to_string(), "Greater".to_string()), 0);
        let mut unit_structs: Vec<String> = Vec::new();
        let mut tuple_structs: Vec<String> = Vec::new();
        for s in &prog.structs {
            if s.unit {
                unit_structs.push(s.name.clone());
            }
            if s.tuple {
                tuple_structs.push(s.name.clone());
            }
        }
        let mut interp = Interp {
            funcs,
            methods,
            globals: Vec::new(),
            enum_variants,
            enum_named_fields,
            closures: RefCell::new(Vec::new()),
            out: RefCell::new(String::new()),
            trace_on: false,
            trace: RefCell::new(Vec::new()),
            unit_structs,
            tuple_structs,
        };
        let scope = Scope::new(None);
        for g in &prog.globals {
            let v = interp
                .eval_expr(&g.init, &scope)
                .map_err(|sig| signal_message("global initializer", sig))?;
            scope.define(&g.name, clone_value(&v));
            interp.globals.push((g.name.clone(), v));
        }
        for imp in &prog.impls {
            let target = interp_impl_target_name(&imp.target)?;
            for c in &imp.consts {
                let v = interp
                    .eval_expr(&c.init, &scope)
                    .map_err(|sig| signal_message("associated const initializer", sig))?;
                interp.globals.push((format!("{}::{}", target, c.name), v));
            }
        }
        Ok(interp)
    }

    pub fn enable_trace(&mut self) {
        self.trace_on = true;
    }

    pub fn take_trace(&self) -> Vec<String> {
        let mut slots = self.trace.borrow_mut();
        let mut out = Vec::new();
        while !slots.is_empty() {
            out.push(slots.remove(0));
        }
        out
    }

    fn tr(&self, line: String) {
        if self.trace_on {
            let mut slots = self.trace.borrow_mut();
            slots.push(line);
        }
    }

    pub fn run_main(&self) -> Result<String, String> {
        let main = self
            .funcs
            .get("main")
            .ok_or_else(|| "interp: no `fn main`".to_string())?;
        let scope = Scope::new(None);
        self.populate_globals(&scope);
        match self.eval_block(&main.body, &scope) {
            Ok(_) | Err(Signal::Return(_)) => Ok(self.out.borrow().clone()),
            Err(Signal::Break(_, _)) => Err("interp: `break` outside of a loop".to_string()),
            Err(Signal::Continue) => Err("interp: `continue` outside of a loop".to_string()),
            Err(Signal::Error(e)) => {
                if self.trace_on {
                    self.tr(format!("error:{}", e));
                }
                Err(e)
            }
        }
    }

    fn call(&self, name: &str, args: Vec<Val>, scope: &Scope) -> R {
        if self.trace_on {
            self.tr(format!("call:{}", name));
        }
        match name {
            "Some" => {
                if args.len() != 1 {
                    return Err(err(format!(
                        "interp: Some expected 1 arg, got {}",
                        args.len()
                    )));
                }
                return Ok(Val::Enum {
                    enum_name: Rc::new("Option".to_string()),
                    variant: Rc::new("Some".to_string()),
                    data: Rc::new(args),
                });
            }
            "Ok" | "Err" => {
                if args.len() != 1 {
                    return Err(err(format!(
                        "interp: {} expected 1 arg, got {}",
                        name,
                        args.len()
                    )));
                }
                return Ok(Val::Enum {
                    enum_name: Rc::new("Result".to_string()),
                    variant: Rc::new(name.to_string()),
                    data: Rc::new(args),
                });
            }
            _ => {}
        }
        if let Some(v) = scope.get(name) {
            return self.call_callable(&v, args);
        }
        let f = self
            .funcs
            .get(name)
            .ok_or_else(|| err(format!("interp: unknown function {}", name)))?;
        if args.len() != f.params.len() {
            return Err(err(format!(
                "interp: {} expected {} args, got {}",
                name,
                f.params.len(),
                args.len()
            )));
        }
        let scope = Scope::new(None);
        self.populate_globals(&scope);
        for (p, a) in f.params.iter().zip(args.into_iter()) {
            scope.define(&p.name, coerce_arg_for_param(a, &p.ty));
        }
        match self.eval_block(&f.body, &scope) {
            Ok(v) => Ok(v),
            Err(Signal::Return(v)) => Ok(v),
            Err(Signal::Break(_, _)) => Err(err("interp: `break` outside of a loop")),
            Err(Signal::Continue) => Err(err("interp: `continue` outside of a loop")),
            Err(e) => Err(e),
        }
    }

    fn call_closure(&self, id: usize, args: Vec<Val>) -> R {
        let closure = self
            .closures
            .borrow()
            .get(id)
            .cloned()
            .ok_or_else(|| err(format!("interp: unknown closure {}", id)))?;
        if args.len() != closure.params.len() {
            return Err(err(format!(
                "interp: closure expected {} args, got {}",
                closure.params.len(),
                args.len()
            )));
        }
        let scope = Scope::new(None);
        for (name, value) in closure.captures {
            scope.define(&name, value);
        }
        for (pat, value) in closure.params.iter().zip(args.into_iter()) {
            if !try_match(pat, &value, &scope, &self.enum_named_fields) {
                return Err(err("interp: closure parameter pattern did not match"));
            }
        }
        match self.eval_expr(&closure.body, &scope) {
            Ok(v) => Ok(v),
            Err(Signal::Return(v)) => Ok(v),
            Err(Signal::Break(_, _)) => Err(err("interp: `break` outside of a loop")),
            Err(Signal::Continue) => Err(err("interp: `continue` outside of a loop")),
            Err(e) => Err(e),
        }
    }

    fn call_callable(&self, callable: &Val, args: Vec<Val>) -> R {
        match callable {
            Val::Closure(id) => self.call_closure(*id, args),
            Val::Function(name) => {
                let root = Scope::new(None);
                self.call(name, args, &root)
            }
            Val::EnumCtor {
                enum_name,
                variant,
                arity,
            } => {
                if args.len() != *arity {
                    return Err(err(format!(
                        "interp: {}::{} expected {} args, got {}",
                        enum_name,
                        variant,
                        arity,
                        args.len()
                    )));
                }
                Ok(Val::Enum {
                    enum_name: enum_name.clone(),
                    variant: variant.clone(),
                    data: Rc::new(args),
                })
            }
            other => Err(err(format!("interp: {} is not callable", other.kind()))),
        }
    }

    fn call_parse_method(&self, receiver: Val, type_args: &[Type], args: &[Expr]) -> R {
        if !args.is_empty() {
            return Err(err("interp: parse expects 0 args"));
        }
        if type_args == [Type::F64] {
            let text = string_content(&receiver)?;
            return match text.parse::<f64>() {
                Ok(f) => Ok(Val::Enum {
                    enum_name: Rc::new("Result".to_string()),
                    variant: Rc::new("Ok".to_string()),
                    data: Rc::new(vec![Val::F64(f)]),
                }),
                Err(_) => Ok(Val::Enum {
                    enum_name: Rc::new("Result".to_string()),
                    variant: Rc::new("Err".to_string()),
                    data: Rc::new(vec![Val::String(Rc::new(RefCell::new(
                        "parse error".to_string(),
                    )))]),
                }),
            };
        }
        if type_args != [Type::I64] {
            return Err(err(format!(
                "interp: parse only supports turbofish i64, got {:?}",
                type_args
            )));
        }
        let text = string_content(&receiver)?;
        match text.parse::<i64>() {
            Ok(n) => Ok(Val::Enum {
                enum_name: Rc::new("Result".to_string()),
                variant: Rc::new("Ok".to_string()),
                data: Rc::new(vec![Val::I64(n)]),
            }),
            Err(_) => Ok(Val::Enum {
                enum_name: Rc::new("Result".to_string()),
                variant: Rc::new("Err".to_string()),
                data: Rc::new(vec![Val::String(Rc::new(RefCell::new(
                    "parse error".to_string(),
                )))]),
            }),
        }
    }

    fn call_method(
        &self,
        target: &str,
        name: &str,
        receiver: Option<Val>,
        type_args: &[Type],
        args: Vec<Val>,
    ) -> R {
        if name == "clone" {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: {}::clone expected 0 args, got {}",
                    target,
                    args.len()
                )));
            }
            let receiver = receiver
                .ok_or_else(|| err(format!("interp: {}::clone needs a receiver", target)))?;
            return Ok(clone_value(&receiver));
        }
        if name == "into" {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: {}::into expected 0 args, got {}",
                    target,
                    args.len()
                )));
            }
            let receiver = receiver
                .ok_or_else(|| err(format!("interp: {}::into needs a receiver", target)))?;
            // Integer widening (u8 -> i64 etc.) is identity in the single-width
            // i64 value model; string-like receivers convert to a String.
            match deref_value(receiver.clone()) {
                Val::I64(n) => return Ok(Val::I64(n)),
                _ => {
                    let s = string_content(&receiver)?;
                    return Ok(Val::String(Rc::new(RefCell::new(s))));
                }
            }
        }
        if target == "Vec" {
            // `retain` needs the interpreter to call the predicate closure, so
            // it is handled here rather than in the free-fn dispatch.
            if name == "sort_by_key" {
                if args.len() != 1 {
                    return Err(err("interp: Vec::sort_by_key expects 1 arg"));
                }
                let recv = receiver
                    .ok_or_else(|| err("interp: Vec::sort_by_key needs a receiver"))?;
                let vec = vec_handle(recv)?;
                let mut items: Vec<Val> = Vec::new();
                {
                    let b = vec.borrow();
                    let n = b.len();
                    let mut k = 0;
                    while k < n {
                        match b.get(k) {
                            Some(v) => items.push(v.clone()),
                            None => {}
                        }
                        k += 1;
                    }
                }
                let mut keys: Vec<i64> = Vec::new();
                let mut k = 0;
                while k < items.len() {
                    let key = self
                        .call_callable(&args[0], vec![items[k].clone()])?
                        .as_i64("Vec::sort_by_key key")
                        .map_err(Signal::Error)?;
                    keys.push(key);
                    k += 1;
                }
                // insertion sort by key (stable), moving items alongside
                let mut i = 1;
                while i < items.len() {
                    let key = keys[i];
                    let item = items[i].clone();
                    let mut j = i;
                    while j > 0 && keys[j - 1] > key {
                        keys[j] = keys[j - 1];
                        items[j] = items[j - 1].clone();
                        j -= 1;
                    }
                    keys[j] = key;
                    items[j] = item;
                    i += 1;
                }
                {
                    let mut b = vec.borrow_mut();
                    b.clear();
                    let mut k = 0;
                    while k < items.len() {
                        b.push(items[k].clone());
                        k += 1;
                    }
                }
                return Ok(Val::Unit);
            }
            if name == "sort_by" {
                if args.len() != 1 {
                    return Err(err("interp: Vec::sort_by expects 1 arg"));
                }
                let recv = receiver.ok_or_else(|| err("interp: Vec::sort_by needs a receiver"))?;
                let vec = vec_handle(recv)?;
                let mut items: Vec<Val> = vec.borrow().clone();
                // Stable insertion sort; the comparator returns Ordering, and a
                // `Greater` for (left, right) means left should sort after right.
                let mut i = 1;
                while i < items.len() {
                    let item = items[i].clone();
                    let mut j = i;
                    while j > 0 {
                        let ord = self
                            .call_callable(&args[0], vec![items[j - 1].clone(), item.clone()])?;
                        if ordering_is_greater(&ord) {
                            items[j] = items[j - 1].clone();
                            j -= 1;
                        } else {
                            break;
                        }
                    }
                    items[j] = item;
                    i += 1;
                }
                {
                    let mut b = vec.borrow_mut();
                    b.clear();
                    let mut k = 0;
                    while k < items.len() {
                        b.push(items[k].clone());
                        k += 1;
                    }
                }
                return Ok(Val::Unit);
            }
            if name == "retain" {
                if args.len() != 1 {
                    return Err(err("interp: Vec::retain expects 1 arg"));
                }
                let recv = receiver
                    .ok_or_else(|| err("interp: Vec::retain needs a receiver"))?;
                let vec = vec_handle(recv)?;
                let mut items: Vec<Val> = Vec::new();
                {
                    let b = vec.borrow();
                    let n = b.len();
                    let mut k = 0;
                    while k < n {
                        match b.get(k) {
                            Some(v) => items.push(v.clone()),
                            None => {}
                        }
                        k += 1;
                    }
                }
                let mut kept: Vec<Val> = Vec::new();
                let mut k = 0;
                while k < items.len() {
                    let keep = self
                        .call_callable(&args[0], vec![items[k].clone()])?
                        .as_bool("Vec::retain closure")
                        .map_err(Signal::Error)?;
                    if keep {
                        kept.push(items[k].clone());
                    }
                    k += 1;
                }
                {
                    let mut b = vec.borrow_mut();
                    b.clear();
                    let mut k = 0;
                    while k < kept.len() {
                        b.push(kept[k].clone());
                        k += 1;
                    }
                }
                return Ok(Val::Unit);
            }
            return call_vec_method(name, receiver, args);
        }
        if target == "String" || target == "str" {
            return call_string_method(target, name, receiver, args);
        }
        if target == "Path" || target == "PathBuf" {
            return call_path_method(target, name, receiver, args);
        }
        if target == "Command" {
            return call_command_method(name, receiver, args);
        }
        if target == "ExitStatus" {
            return call_exit_status_method(name, receiver, args);
        }
        if is_int_runtime_target(target) {
            return call_int_method(target, name, receiver, args);
        }
        if target == "bool" {
            return call_bool_method(self, name, receiver, args);
        }
        if target == "char" {
            return call_char_method(name, receiver, args);
        }
        if target == "Option" {
            return call_option_method(self, name, receiver, args);
        }
        if target == "Result" {
            return call_result_method(self, name, receiver, args);
        }
        if target == "Box" || target == "Rc" {
            return call_boxlike_method(target, name, receiver, args);
        }
        if target == "RefCell" {
            return call_refcell_method(name, receiver, args);
        }
        if target == "HashMap" {
            return call_hashmap_method(name, receiver, args);
        }
        if target == "HashEntry" {
            return call_hashentry_method(self, name, receiver, args);
        }
        if target == "Iter" {
            return call_iter_method(self, name, receiver, type_args, args);
        }
        let m = self
            .methods
            .get(&(target.to_string(), name.to_string()))
            .ok_or_else(|| err(format!("interp: unknown method {}::{}", target, name)))?;
        match (&m.receiver, &receiver) {
            (Some(_), None) => {
                return Err(err(format!(
                    "interp: {}::{} needs a receiver",
                    target, name
                )));
            }
            (None, Some(_)) => {
                return Err(err(format!("interp: {}::{} is not a method", target, name)));
            }
            _ => {}
        }
        if args.len() != m.params.len() {
            return Err(err(format!(
                "interp: {}::{} expected {} args, got {}",
                target,
                name,
                m.params.len(),
                args.len()
            )));
        }
        let scope = Scope::new(None);
        self.populate_globals(&scope);
        if let Some(v) = receiver {
            scope.define("self", v);
        }
        for (p, a) in m.params.iter().zip(args.into_iter()) {
            scope.define(&p.name, coerce_arg_for_param(a, &p.ty));
        }
        match self.eval_block(&m.body, &scope) {
            Ok(v) => Ok(v),
            Err(Signal::Return(v)) => Ok(v),
            Err(Signal::Break(_, _)) => Err(err("interp: `break` outside of a loop")),
            Err(Signal::Continue) => Err(err("interp: `continue` outside of a loop")),
            Err(e) => Err(e),
        }
    }

    fn populate_globals(&self, scope: &Scope) {
        for (name, value) in &self.globals {
            scope.define(name, clone_value(value));
        }
    }

    fn eval_block(&self, block: &Block, parent: &Scope) -> R {
        let scope = Scope::new(Some(parent));
        for s in &block.stmts {
            match s {
                Stmt::Let { name, ty, init, .. } => {
                    if self.trace_on {
                        self.tr(format!("bind:{}", name));
                    }
                    let v = coerce_let_value(self.eval_expr(init, &scope)?, ty);
                    scope.define(name, v);
                }
                Stmt::LetPat { pat, init } => {
                    let v = self.eval_expr(init, &scope)?;
                    if !try_match(pat, &v, &scope, &self.enum_named_fields) {
                        return Err(err("interp: let pattern did not match"));
                    }
                }
                Stmt::LetElse {
                    pat,
                    init,
                    else_blk,
                } => {
                    let v = self.eval_expr(init, &scope)?;
                    let pat_scope = Scope::new(Some(&scope));
                    if try_match(pat, &v, &pat_scope, &self.enum_named_fields) {
                        for (name, val) in pat_scope.local_values() {
                            scope.define(&name, val);
                        }
                    } else {
                        match self.eval_block(else_blk, &scope) {
                            Ok(_) => {
                                return Err(err("interp: let-else else block did not diverge"))
                            }
                            Err(Signal::Return(v)) => return Err(Signal::Return(v)),
                            Err(Signal::Break(l, v)) => return Err(Signal::Break(l, v)),
                            Err(Signal::Continue) => return Err(Signal::Continue),
                            Err(Signal::Error(e)) => return Err(Signal::Error(e)),
                        }
                    }
                }
                Stmt::Assign { target, value } => {
                    let v = self.eval_expr(value, &scope)?;
                    self.assign_place(target, v, &scope)?;
                }
                Stmt::Expr(e) => {
                    self.eval_expr(e, &scope)?;
                }
                Stmt::Return(opt) => {
                    let v = match opt {
                        Some(e) => self.eval_expr(e, &scope)?,
                        None => Val::Unit,
                    };
                    return Err(Signal::Return(v));
                }
            }
        }
        match &block.tail {
            Some(e) => self.eval_expr(e, &scope),
            None => Ok(Val::Unit),
        }
    }

    fn is_unit_struct(&self, name: &String) -> bool {
        for s in &self.unit_structs {
            if s == name {
                return true;
            }
        }
        false
    }

    fn is_tuple_struct(&self, name: &String) -> bool {
        for s in &self.tuple_structs {
            if s == name {
                return true;
            }
        }
        false
    }

    fn eval_expr(&self, e: &Expr, scope: &Scope) -> R {
        match e {
            Expr::Int(n) => Ok(Val::I64(*n)),
            Expr::IntHex(n, _text) => Ok(Val::I64(*n)),
            Expr::Float(text) => match text.parse::<f64>() {
                Ok(f) => Ok(Val::F64(f)),
                Err(_) => Err(err(format!("interp: bad float literal {}", text))),
            },
            Expr::Char(ch) => Ok(Val::Char(*ch)),
            Expr::Bool(b) => Ok(Val::Bool(*b)),
            Expr::Str(s) => Ok(Val::Str(Rc::new(s.clone()))),
            Expr::Var(name) => scope
                .get(name)
                .or_else(|| {
                    if name == "None" {
                        Some(Val::Enum {
                            enum_name: Rc::new("Option".to_string()),
                            variant: Rc::new("None".to_string()),
                            data: Rc::new(Vec::new()),
                        })
                    } else if self.funcs.contains_key(name) {
                        Some(Val::Function(Rc::new(name.clone())))
                    } else if self.is_unit_struct(name) {
                        Some(Val::Struct {
                            name: Rc::new(name.clone()),
                            fields: Rc::new(Vec::new()),
                        })
                    } else {
                        None
                    }
                })
                .ok_or_else(|| err(format!("interp: unbound variable {}", name))),
            Expr::Ref { mutable, expr } => {
                let slot = match place_slot(expr.as_ref(), scope) {
                    Ok(slot) => slot,
                    Err(_) => Rc::new(RefCell::new(self.eval_expr(expr.as_ref(), scope)?)),
                };
                Ok(Val::Ref {
                    slot,
                    mutable: *mutable,
                })
            }
            Expr::Unary { op, rhs } => {
                let v = self.eval_expr(rhs, scope)?;
                match op {
                    UnOp::Neg => {
                        if let Val::F64(f) = &v {
                            let x = *f;
                            return Ok(Val::F64(-x));
                        }
                        Ok(Val::I64(
                            v.as_i64("unary -").map_err(Signal::Error)?.wrapping_neg(),
                        ))
                    }
                    UnOp::Not => match deref_value(v) {
                        Val::Bool(b) => Ok(Val::Bool(!b)),
                        Val::I64(n) => Ok(Val::I64(!n)),
                        other => Err(err(format!(
                            "unary !: expected bool/int, got {}",
                            other.kind()
                        ))),
                    },
                    UnOp::Deref => match v {
                        Val::Ref { slot, .. } => Ok(slot.borrow().clone()),
                        Val::VecElemRef { vec, index, .. } => vec
                            .borrow()
                            .get(index)
                            .cloned()
                            .ok_or_else(|| err("interp: dangling Vec element ref")),
                        Val::Box(slot) | Val::RcVal(slot) => Ok(slot.borrow().clone()),
                        // The value model erases reference layers eagerly (e.g.
                        // closure arguments arrive by value), so a statically
                        // valid `*x` may see an already-plain value: deref is
                        // idempotent. Invalid derefs are still rejected by
                        // typeck, so this accepts nothing rustc rejects.
                        other => Ok(other),
                    },
                }
            }
            Expr::Binary { op, lhs, rhs } => self.eval_binary(*op, lhs, rhs, scope),
            Expr::Cast { expr, ty } => {
                let v = self.eval_expr(expr, scope)?;
                Ok(cast_value(v, ty)?)
            }
            Expr::Try(expr) => {
                let v = self.eval_expr(expr, scope)?;
                match v {
                    Val::Enum {
                        enum_name,
                        variant,
                        data,
                    } if enum_name.as_str() == "Option" => {
                        if variant.as_str() == "Some" {
                            Ok(data[0].clone())
                        } else {
                            Err(Signal::Return(option_none()))
                        }
                    }
                    Val::Enum {
                        enum_name,
                        variant,
                        data,
                    } if enum_name.as_str() == "Result" => {
                        if variant.as_str() == "Ok" {
                            Ok(data[0].clone())
                        } else {
                            Err(Signal::Return(Val::Enum {
                                enum_name: Rc::new("Result".to_string()),
                                variant: Rc::new("Err".to_string()),
                                data: question_error_data(data),
                            }))
                        }
                    }
                    other => Err(err(format!("interp: ? on {}", other.kind()))),
                }
            }
            Expr::Return(opt) => {
                let v = match opt {
                    Some(e) => self.eval_expr(e, scope)?,
                    None => Val::Unit,
                };
                Err(Signal::Return(v))
            }
            Expr::Assign { target, value } => {
                let v = self.eval_expr(value, scope)?;
                self.assign_place(target, v, scope)?;
                Ok(Val::Unit)
            }
            Expr::Closure { params, body, .. } => {
                let id = {
                    let mut closures = self.closures.borrow_mut();
                    let id = closures.len();
                    closures.push(ClosureVal {
                        params: params.iter().map(|p| p.pat.clone()).collect(),
                        body: (**body).clone(),
                        captures: scope.snapshot_values(),
                    });
                    id
                };
                Ok(Val::Closure(id))
            }
            Expr::Call { name, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_expr(a, scope)?);
                }
                if self.is_tuple_struct(name) {
                    let mut fs = Vec::new();
                    let mut i = 0usize;
                    for v in vals.into_iter() {
                        fs.push((format!("{}", i), v));
                        i += 1;
                    }
                    return Ok(Val::Struct {
                        name: Rc::new(name.clone()),
                        fields: Rc::new(fs),
                    });
                }
                self.call(name, vals, scope)
            }
            Expr::CallExpr { callee, args } => {
                let c = self.eval_expr(callee, scope)?;
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_expr(a, scope)?);
                }
                match c {
                    Val::Closure(_) | Val::Function(_) | Val::EnumCtor { .. } => {
                        self.call_callable(&c, vals)
                    }
                    other => Err(err(format!("interp: cannot call {}", other.kind()))),
                }
            }
            Expr::PathCall {
                type_name,
                item,
                args,
            } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_expr(a, scope)?);
                }
                if (type_name == "std::cmp" || type_name == "cmp")
                    && matches!(item.as_str(), "max" | "min")
                {
                    if vals.len() != 2 {
                        return Err(err(format!("interp: cmp::{} expects 2 args", item)));
                    }
                    let a = vals[0].as_i64("cmp").map_err(Signal::Error)?;
                    let b = vals[1].as_i64("cmp").map_err(Signal::Error)?;
                    let r = if item == "max" { a.max(b) } else { a.min(b) };
                    return Ok(Val::I64(r));
                }
                if type_name == "Vec" {
                    match item.as_str() {
                        "new" => {
                            if !vals.is_empty() {
                                return Err(err("interp: Vec::new expects 0 args"));
                            }
                            return Ok(Val::Vec(Rc::new(RefCell::new(Vec::new()))));
                        }
                        "with_capacity" => {
                            if vals.len() != 1 {
                                return Err(err(format!(
                                    "interp: Vec::with_capacity expected 1 arg, got {}",
                                    vals.len()
                                )));
                            }
                            vals[0]
                                .as_i64("Vec::with_capacity")
                                .map_err(Signal::Error)?;
                            return Ok(Val::Vec(Rc::new(RefCell::new(Vec::new()))));
                        }
                        _ => {}
                    }
                }
                if type_name == "String" {
                    match item.as_str() {
                        "new" => {
                            if !vals.is_empty() {
                                return Err(err("interp: String::new expects 0 args"));
                            }
                            return Ok(Val::String(Rc::new(RefCell::new(String::new()))));
                        }
                        "from" => {
                            if vals.len() != 1 {
                                return Err(err(format!(
                                    "interp: String::from expected 1 arg, got {}",
                                    vals.len()
                                )));
                            }
                            let s = string_content(&vals[0])?;
                            return Ok(Val::String(Rc::new(RefCell::new(s))));
                        }
                        "from_utf8_lossy" => {
                            if vals.len() != 1 {
                                return Err(err(format!(
                                    "interp: String::from_utf8_lossy expected 1 arg, got {}",
                                    vals.len()
                                )));
                            }
                            let s = string_content(&vals[0])?;
                            return Ok(Val::String(Rc::new(RefCell::new(s))));
                        }
                        _ => {}
                    }
                }
                if type_name == "Command" && item == "new" {
                    if vals.len() != 1 {
                        return Err(err(format!(
                            "interp: Command::new expected 1 arg, got {}",
                            vals.len()
                        )));
                    }
                    let program = path_content(&vals[0])?;
                    return Ok(Val::Command(Rc::new(RefCell::new(vec![program]))));
                }
                if matches!(type_name.as_str(), "env" | "std::env") && item == "args" {
                    if !vals.is_empty() {
                        return Err(err(format!(
                            "interp: {}::args expected 0 args, got {}",
                            type_name,
                            vals.len()
                        )));
                    }
                    return Ok(iter_value(vec![
                        Val::String(Rc::new(RefCell::new("bootstrap".to_string()))),
                        Val::String(Rc::new(RefCell::new("help".to_string()))),
                    ]));
                }
                if matches!(type_name.as_str(), "env" | "std::env") && item == "current_exe" {
                    if !vals.is_empty() {
                        return Err(err(format!(
                            "interp: {}::current_exe expected 0 args, got {}",
                            type_name,
                            vals.len()
                        )));
                    }
                    return Ok(result_ok(Val::PathBuf(Rc::new("bootstrap".to_string()))));
                }
                if matches!(type_name.as_str(), "env" | "std::env") && item == "var" {
                    if vals.len() != 1 {
                        return Err(err(format!(
                            "interp: {}::var expected 1 arg, got {}",
                            type_name,
                            vals.len()
                        )));
                    }
                    let key = string_content(&vals[0])?;
                    return Ok(match std::env::var(&key) {
                        Ok(value) => result_ok(Val::String(Rc::new(RefCell::new(value)))),
                        Err(e) => result_err(e.to_string()),
                    });
                }
                if type_name == "PathBuf" && item == "from" {
                    if vals.len() != 1 {
                        return Err(err(format!(
                            "interp: PathBuf::from expected 1 arg, got {}",
                            vals.len()
                        )));
                    }
                    let path = path_content(&vals[0])?;
                    return Ok(Val::PathBuf(Rc::new(path)));
                }
                if type_name == "Path" && item == "new" {
                    if vals.len() != 1 {
                        return Err(err(format!(
                            "interp: Path::new expected 1 arg, got {}",
                            vals.len()
                        )));
                    }
                    let path = path_content(&vals[0])?;
                    return Ok(Val::PathBuf(Rc::new(path)));
                }
                if type_name == "char" && item == "from_u32" {
                    if vals.len() != 1 {
                        return Err(err(format!(
                            "interp: char::from_u32 expected 1 arg, got {}",
                            vals.len()
                        )));
                    }
                    let n = vals[0]
                        .as_i64("char::from_u32 arg")
                        .map_err(Signal::Error)?;
                    return Ok(match char::from_u32(n as u32) {
                        Some(ch) => option_some(Val::Char(ch)),
                        None => option_none(),
                    });
                }
                if is_int_runtime_target(type_name) && item == "from_str_radix" {
                    if vals.len() != 2 {
                        return Err(err(format!(
                            "interp: {}::from_str_radix expected 2 args, got {}",
                            type_name,
                            vals.len()
                        )));
                    }
                    let text = string_content(&vals[0])?;
                    let radix = vals[1]
                        .as_i64("from_str_radix radix")
                        .map_err(Signal::Error)?;
                    if radix < 0 {
                        return Err(err(format!(
                            "interp: {}::from_str_radix radix out of range",
                            type_name
                        )));
                    }
                    let radix = radix as u32;
                    let parsed = u64::from_str_radix(&text, radix);
                    return Ok(match parsed {
                        Ok(n) => Val::Enum {
                            enum_name: Rc::new("Result".to_string()),
                            variant: Rc::new("Ok".to_string()),
                            data: Rc::new(vec![Val::I64(n as i64)]),
                        },
                        Err(_) => Val::Enum {
                            enum_name: Rc::new("Result".to_string()),
                            variant: Rc::new("Err".to_string()),
                            data: Rc::new(vec![Val::String(Rc::new(RefCell::new(
                                "parse int error".to_string(),
                            )))]),
                        },
                    });
                }
                if matches!(type_name.as_str(), "fs" | "std::fs") {
                    match item.as_str() {
                        "create_dir_all" => {
                            if vals.len() != 1 {
                                return Err(err(format!(
                                    "interp: {}::create_dir_all expected 1 arg, got {}",
                                    type_name,
                                    vals.len()
                                )));
                            }
                            let path = match path_content(&vals[0]) {
                                Ok(path) => path,
                                Err(_) => return Ok(result_ok(Val::Unit)),
                            };
                            return Ok(match std::fs::create_dir_all(&path) {
                                Ok(()) => result_ok(Val::Unit),
                                Err(e) => result_err(e.to_string()),
                            });
                        }
                        "write" => {
                            if vals.len() != 2 {
                                return Err(err(format!(
                                    "interp: {}::write expected 2 args, got {}",
                                    type_name,
                                    vals.len()
                                )));
                            }
                            let path = path_content(&vals[0])?;
                            let body = string_content(&vals[1])?;
                            return Ok(match std::fs::write(&path, body) {
                                Ok(()) => result_ok(Val::Unit),
                                Err(e) => result_err(e.to_string()),
                            });
                        }
                        "read_to_string" => {
                            if vals.len() != 1 {
                                return Err(err(format!(
                                    "interp: {}::read_to_string expected 1 arg, got {}",
                                    type_name,
                                    vals.len()
                                )));
                            }
                            let path = path_content(&vals[0])?;
                            return Ok(match std::fs::read_to_string(&path) {
                                Ok(text) => result_ok(Val::String(Rc::new(RefCell::new(text)))),
                                Err(e) => result_err(e.to_string()),
                            });
                        }
                        "read" => {
                            if vals.len() != 1 {
                                return Err(err(format!(
                                    "interp: {}::read expected 1 arg, got {}",
                                    type_name,
                                    vals.len()
                                )));
                            }
                            let path = path_content(&vals[0])?;
                            return Ok(match std::fs::read(&path) {
                                Ok(bytes) => result_ok(Val::Vec(Rc::new(RefCell::new(
                                    bytes.into_iter().map(|b| Val::I64(b as i64)).collect(),
                                )))),
                                Err(e) => result_err(e.to_string()),
                            });
                        }
                        _ => {}
                    }
                }
                if self.funcs.contains_key(item) {
                    return self.call(item, vals, scope);
                }
                match (type_name.as_str(), item.as_str()) {
                    ("Box", "new") | ("Rc", "new") | ("RefCell", "new") => {
                        if vals.len() != 1 {
                            return Err(err(format!(
                                "interp: {}::new expected 1 arg, got {}",
                                type_name,
                                vals.len()
                            )));
                        }
                        let slot = Rc::new(RefCell::new(vals[0].clone()));
                        return Ok(match type_name.as_str() {
                            "Box" => Val::Box(slot),
                            "Rc" => Val::RcVal(slot),
                            "RefCell" => Val::RefCellVal(slot),
                            _ => unreachable!(),
                        });
                    }
                    ("HashMap", "new") => {
                        if !vals.is_empty() {
                            return Err(err(format!(
                                "interp: HashMap::new expected 0 args, got {}",
                                vals.len()
                            )));
                        }
                        return Ok(Val::HashMap(Rc::new(RefCell::new(Vec::new()))));
                    }
                    ("Rc", "clone") => {
                        if vals.len() != 1 {
                            return Err(err(format!(
                                "interp: Rc::clone expected 1 arg, got {}",
                                vals.len()
                            )));
                        }
                        let slot = rc_slot(&vals[0])?;
                        return Ok(Val::RcVal(slot));
                    }
                    ("Rc", "ptr_eq") => {
                        if vals.len() != 2 {
                            return Err(err(format!(
                                "interp: Rc::ptr_eq expected 2 args, got {}",
                                vals.len()
                            )));
                        }
                        let a = rc_slot(&vals[0])?;
                        let b = rc_slot(&vals[1])?;
                        return Ok(Val::Bool(Rc::ptr_eq(&a, &b)));
                    }
                    _ => {}
                }
                if let Some(expected) = self
                    .enum_variants
                    .get(&(type_name.clone(), item.clone()))
                    .copied()
                {
                    if vals.len() != expected {
                        return Err(err(format!(
                            "interp: {}::{} expected {} args, got {}",
                            type_name,
                            item,
                            expected,
                            vals.len()
                        )));
                    }
                    return Ok(Val::Enum {
                        enum_name: Rc::new(type_name.clone()),
                        variant: Rc::new(item.clone()),
                        data: Rc::new(vals),
                    });
                }
                let empty_type_args: Vec<Type> = Vec::new();
                self.call_method(type_name, item, None, &empty_type_args, vals)
            }
            Expr::MethodCall {
                receiver,
                name,
                type_args,
                args,
            } => {
                let recv = self.eval_expr(receiver, scope)?;
                if name == "parse" {
                    return self.call_parse_method(recv, type_args, args);
                }
                let target = runtime_type_name(&recv).ok_or_else(|| {
                    err(format!(
                        "interp: method {} on unsupported receiver {}",
                        name,
                        recv.kind()
                    ))
                })?;
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_expr(a, scope)?);
                }
                let receiver_kind = match self.methods.get(&(target.clone(), name.clone())) {
                    Some(m) => m.receiver,
                    None => None,
                };
                let method_recv = match receiver_kind {
                    Some(ReceiverKind::Ref) => {
                        let slot = match place_slot(receiver.as_ref(), scope) {
                            Ok(slot) => slot,
                            Err(_) => Rc::new(RefCell::new(recv.clone())),
                        };
                        Val::Ref {
                            slot,
                            mutable: false,
                        }
                    }
                    Some(ReceiverKind::RefMut) => {
                        let slot = match place_slot(receiver.as_ref(), scope) {
                            Ok(slot) => slot,
                            Err(_) => Rc::new(RefCell::new(recv.clone())),
                        };
                        Val::Ref {
                            slot,
                            mutable: true,
                        }
                    }
                    _ => recv,
                };
                self.call_method(&target, name, Some(method_recv), type_args, vals)
            }
            Expr::If {
                cond,
                then_blk,
                else_blk,
            } => {
                let c = self
                    .eval_expr(cond, scope)?
                    .as_bool("if condition")
                    .map_err(Signal::Error)?;
                if c {
                    self.eval_block(then_blk, scope)
                } else if let Some(eb) = else_blk {
                    self.eval_block(eb, scope)
                } else {
                    Ok(Val::Unit)
                }
            }
            Expr::Block(b) => self.eval_block(b, scope),
            Expr::Println { fmt, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_expr(a, scope)?);
                }
                let line = self.format_println(fmt, &vals).map_err(Signal::Error)?;
                self.out.borrow_mut().push_str(&line);
                self.out.borrow_mut().push('\n');
                Ok(Val::Unit)
            }
            Expr::Print { fmt, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_expr(a, scope)?);
                }
                let text = self.format_println(fmt, &vals).map_err(Signal::Error)?;
                self.out.borrow_mut().push_str(&text);
                Ok(Val::Unit)
            }
            Expr::Eprintln { args, .. } => {
                for a in args {
                    self.eval_expr(a, scope)?;
                }
                Ok(Val::Unit)
            }
            Expr::Format { fmt, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_expr(a, scope)?);
                }
                let text = self.format_println(fmt, &vals).map_err(Signal::Error)?;
                Ok(Val::String(Rc::new(RefCell::new(text))))
            }
            Expr::Write {
                newline,
                target,
                fmt,
                args,
            } => {
                let target = self.eval_expr(target, scope)?;
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_expr(a, scope)?);
                }
                let mut text = self.format_println(fmt, &vals).map_err(Signal::Error)?;
                if *newline {
                    text.push('\n');
                }
                let out = write_string_handle(target)?;
                out.borrow_mut().push_str(&text);
                Ok(result_ok(Val::Unit))
            }
            Expr::Panic { name } => Err(err(format!("interp: {}! invoked", name))),
            Expr::Assert { cond } => {
                let ok = self
                    .eval_expr(cond, scope)?
                    .as_bool("assert! condition")
                    .map_err(Signal::Error)?;
                if ok {
                    Ok(Val::Unit)
                } else {
                    Err(err("interp: assert! failed"))
                }
            }
            Expr::AssertEq { left, right } => {
                let l = self.eval_expr(left, scope)?;
                let r = self.eval_expr(right, scope)?;
                if value_eq(&l, &r) {
                    Ok(Val::Unit)
                } else {
                    Err(err("interp: assert_eq! failed"))
                }
            }
            Expr::Cfg { name } => Ok(Val::Bool(cfg_flag(name))),
            Expr::Matches { expr, pat, guard } => {
                let v = self.eval_expr(expr, scope)?;
                let arm_scope = Scope::new(Some(scope));
                if !try_match(pat, &v, &arm_scope, &self.enum_named_fields) {
                    return Ok(Val::Bool(false));
                }
                if let Some(g) = guard {
                    let ok = self
                        .eval_expr(g, &arm_scope)?
                        .as_bool("matches! guard")
                        .map_err(Signal::Error)?;
                    return Ok(Val::Bool(ok));
                }
                Ok(Val::Bool(true))
            }
            Expr::TupleLit(items) => {
                if items.is_empty() {
                    return Ok(Val::Unit);
                }
                let mut vs = Vec::with_capacity(items.len());
                for it in items {
                    vs.push(self.eval_expr(it, scope)?);
                }
                Ok(Val::Tuple(Rc::new(vs)))
            }
            Expr::VecLit(items) => {
                let mut vs = Vec::with_capacity(items.len());
                for it in items {
                    vs.push(self.eval_expr(it, scope)?);
                }
                Ok(Val::Vec(Rc::new(RefCell::new(vs))))
            }
            Expr::VecRepeat { elem, count } => {
                let value = self.eval_expr(elem, scope)?;
                let n = self
                    .eval_expr(count, scope)?
                    .as_i64("repeat array count")
                    .map_err(Signal::Error)?;
                if n < 0 {
                    return Err(err("interp: repeat array count must be non-negative"));
                }
                let mut vs = Vec::with_capacity(n as usize);
                let mut i = 0;
                while i < n {
                    vs.push(clone_value(&value));
                    i += 1;
                }
                Ok(Val::Vec(Rc::new(RefCell::new(vs))))
            }
            Expr::StructLit { name, fields } => {
                let mut fs = Vec::with_capacity(fields.len());
                for (fname, fexpr) in fields {
                    fs.push((fname.clone(), self.eval_expr(fexpr, scope)?));
                }
                Ok(Val::Struct {
                    name: Rc::new(name.clone()),
                    fields: Rc::new(fs),
                })
            }
            Expr::EnumCtor { enum_name, variant } => {
                if enum_name == "ExitCode" && matches!(variant.as_str(), "SUCCESS" | "FAILURE") {
                    return Ok(Val::Enum {
                        enum_name: Rc::new(enum_name.clone()),
                        variant: Rc::new(variant.clone()),
                        data: Rc::new(Vec::new()),
                    });
                }
                // Associated const `Target::N` (impl-level const), registered as
                // a global under the qualified name.
                let qname = format!("{}::{}", enum_name, variant);
                for (gname, gval) in self.globals.iter() {
                    if *gname == qname {
                        return Ok(clone_value(gval));
                    }
                }
                let arity = self
                    .enum_variants
                    .get(&(enum_name.clone(), variant.clone()))
                    .copied()
                    .ok_or_else(|| {
                        err(format!(
                            "interp: unknown enum variant {}::{}",
                            enum_name, variant
                        ))
                    })?;
                if arity == 0 {
                    Ok(Val::Enum {
                        enum_name: Rc::new(enum_name.clone()),
                        variant: Rc::new(variant.clone()),
                        data: Rc::new(Vec::new()),
                    })
                } else {
                    Ok(Val::EnumCtor {
                        enum_name: Rc::new(enum_name.clone()),
                        variant: Rc::new(variant.clone()),
                        arity,
                    })
                }
            }
            Expr::EnumStructLit {
                enum_name,
                variant,
                fields,
            } => {
                let names = self
                    .enum_named_fields
                    .get(&(enum_name.clone(), variant.clone()))
                    .ok_or_else(|| {
                        err(format!(
                            "interp: {}::{} is not a struct-like enum variant",
                            enum_name, variant
                        ))
                    })?;
                let mut data = Vec::with_capacity(names.len());
                for fname in names {
                    let expr = fields
                        .iter()
                        .find(|(name, _)| name == fname)
                        .map(|(_, expr)| expr)
                        .ok_or_else(|| {
                            err(format!(
                                "interp: {}::{} missing field {}",
                                enum_name, variant, fname
                            ))
                        })?;
                    data.push(self.eval_expr(expr, scope)?);
                }
                Ok(Val::Enum {
                    enum_name: Rc::new(enum_name.clone()),
                    variant: Rc::new(variant.clone()),
                    data: Rc::new(data),
                })
            }
            Expr::Field { base, name } => {
                let v = self.eval_expr(base, scope)?;
                match deref_value(v) {
                    Val::Struct { fields, .. } => fields
                        .iter()
                        .find(|(k, _)| k == name)
                        .map(|(_, val)| val.clone())
                        .ok_or_else(|| err(format!("interp: no field {}", name))),
                    other => Err(err(format!("interp: field access on {}", other.kind()))),
                }
            }
            Expr::TupleIndex { base, index } => {
                let v = self.eval_expr(base, scope)?;
                match deref_value(v) {
                    Val::Tuple(vs) => vs
                        .get(*index)
                        .cloned()
                        .ok_or_else(|| err(format!("interp: tuple index {} out of range", index))),
                    Val::Struct { fields, .. } => {
                        let key = format!("{}", index);
                        let mut result = None;
                        for pair in fields.iter() {
                            if pair.0 == key {
                                result = Some(pair.1.clone());
                            }
                        }
                        match result {
                            Some(val) => Ok(val),
                            None => Err(err(format!(
                                "interp: tuple index {} on non-tuple struct",
                                index
                            ))),
                        }
                    }
                    other => Err(err(format!("interp: tuple index on {}", other.kind()))),
                }
            }
            Expr::Index { base, index } => {
                let v = self.eval_expr(base, scope)?;
                let idx_val = self.eval_expr(index, scope)?;
                // HashMap indexing `m[&k]`: key lookup (rustc panics on a
                // missing key; here that is a runtime error).
                if let Val::HashMap(map) = deref_value(v.clone()) {
                    let key = normalized_key(&idx_val);
                    let entries = map.borrow();
                    if let Some((_, slot)) = entries.iter().find(|(k, _)| *k == key) {
                        return Ok(slot.borrow().clone());
                    }
                    return Err(err("interp: HashMap index: key not found"));
                }
                let i = idx_val.as_i64("index").map_err(Signal::Error)?;
                if i < 0 {
                    return Err(err("interp: negative index"));
                }
                match deref_value(v) {
                    Val::Vec(vs) => vec_index_value(&vs, i as usize),
                    Val::RcVal(slot) => match slot.borrow().clone() {
                        Val::Vec(vs) => vec_index_value(&vs, i as usize),
                        other => Err(err(format!("interp: index on Rc<{}>", other.kind()))),
                    },
                    other => Err(err(format!("interp: index on {}", other.kind()))),
                }
            }
            Expr::Slice {
                base,
                start,
                end,
                inclusive,
            } => {
                let v = self.eval_expr(base, scope)?;
                let start = match start {
                    Some(e) => Some(
                        self.eval_expr(e, scope)?
                            .as_i64("slice start")
                            .map_err(Signal::Error)?,
                    ),
                    None => None,
                };
                let end = match end {
                    Some(e) => Some(
                        self.eval_expr(e, scope)?
                            .as_i64("slice end")
                            .map_err(Signal::Error)?,
                    ),
                    None => None,
                };
                slice_value(v, start, end, *inclusive)
            }
            Expr::Range {
                start,
                end,
                inclusive,
            } => {
                let s = self
                    .eval_expr(start, scope)?
                    .as_i64("range start")
                    .map_err(Signal::Error)?;
                let e = self
                    .eval_expr(end, scope)?
                    .as_i64("range end")
                    .map_err(Signal::Error)?;
                Ok(iter_value(range_items(s, e, *inclusive)))
            }
            Expr::Match { scrut, arms } => {
                let v = self.eval_expr(scrut, scope)?;
                let mut arm_index = 0usize;
                for arm in arms {
                    let arm_scope = Scope::new(Some(scope));
                    if try_match(&arm.pat, &v, &arm_scope, &self.enum_named_fields) {
                        if let Some(guard) = &arm.guard {
                            let ok = self
                                .eval_expr(guard, &arm_scope)?
                                .as_bool("match guard")
                                .map_err(Signal::Error)?;
                            if !ok {
                                arm_index += 1;
                                continue;
                            }
                        }
                        if self.trace_on {
                            self.tr(format!("arm:{}", arm_index));
                        }
                        return self.eval_expr(&arm.body, &arm_scope);
                    }
                    arm_index += 1;
                }
                Err(err(format!(
                    "interp: no match arm matched (non-exhaustive): {} {}",
                    v.kind(),
                    v.debug_display(&self.enum_named_fields)
                )))
            }
            Expr::While { cond, body } => {
                if self.trace_on {
                    self.tr(String::from("loop:while"));
                }
                loop {
                    let c = self
                        .eval_expr(cond, scope)?
                        .as_bool("while condition")
                        .map_err(Signal::Error)?;
                    if !c {
                        break;
                    }
                    match self.eval_block(body, scope) {
                        Ok(_) | Err(Signal::Continue) => {}
                        Err(Signal::Break(None, _)) => break,
                        Err(Signal::Break(Some(l), v)) => return Err(Signal::Break(Some(l), v)),
                        Err(other) => return Err(other),
                    }
                }
                Ok(Val::Unit)
            }
            Expr::WhileLet { pat, expr, body } => {
                loop {
                    let v = self.eval_expr(expr, scope)?;
                    let iter_scope = Scope::new(Some(scope));
                    if !try_match(pat, &v, &iter_scope, &self.enum_named_fields) {
                        break;
                    }
                    match self.eval_block(body, &iter_scope) {
                        Ok(_) | Err(Signal::Continue) => {}
                        Err(Signal::Break(None, _)) => break,
                        Err(Signal::Break(Some(l), v)) => return Err(Signal::Break(Some(l), v)),
                        Err(other) => return Err(other),
                    }
                }
                Ok(Val::Unit)
            }
            Expr::Loop { body } => loop {
                match self.eval_block(body, scope) {
                    Ok(_) | Err(Signal::Continue) => {}
                    Err(Signal::Break(None, v)) => return Ok(v),
                    Err(Signal::Break(Some(l), v)) => return Err(Signal::Break(Some(l), v)),
                    Err(other) => return Err(other),
                }
            },
            Expr::For {
                var,
                start,
                end,
                inclusive,
                body,
            } => {
                let s = self
                    .eval_expr(start, scope)?
                    .as_i64("for start")
                    .map_err(Signal::Error)?;
                let e = self
                    .eval_expr(end, scope)?
                    .as_i64("for end")
                    .map_err(Signal::Error)?;
                let last = if *inclusive { e } else { e - 1 };
                let mut i = s;
                while i <= last {
                    let iter_scope = Scope::new(Some(scope));
                    iter_scope.define(var, Val::I64(i));
                    match self.eval_block(body, &iter_scope) {
                        Ok(_) | Err(Signal::Continue) => {}
                        Err(Signal::Break(None, _)) => break,
                        Err(Signal::Break(Some(l), v)) => return Err(Signal::Break(Some(l), v)),
                        Err(other) => return Err(other),
                    }
                    i += 1;
                }
                Ok(Val::Unit)
            }
            Expr::ForEach { pat, iter, body } => {
                let iterable = self.eval_expr(iter, scope)?;
                let items = foreach_items(iterable)?;
                for item in items {
                    let iter_scope = Scope::new(Some(scope));
                    if !try_match(pat, &item, &iter_scope, &self.enum_named_fields) {
                        return Err(err("interp: for pattern did not match"));
                    }
                    match self.eval_block(body, &iter_scope) {
                        Ok(_) | Err(Signal::Continue) => {}
                        Err(Signal::Break(None, _)) => break,
                        Err(Signal::Break(Some(l), v)) => return Err(Signal::Break(Some(l), v)),
                        Err(other) => return Err(other),
                    }
                }
                Ok(Val::Unit)
            }
            Expr::Break { label, value } => {
                let v = match value {
                    Some(e) => self.eval_expr(e, scope)?,
                    None => Val::Unit,
                };
                Err(Signal::Break(label.clone(), v))
            }
            Expr::Continue => Err(Signal::Continue),
            Expr::Labeled { label, body } => match self.eval_expr(body, scope) {
                Ok(v) => Ok(v),
                Err(Signal::Break(Some(l), v)) if l == *label => Ok(v),
                Err(other) => Err(other),
            },
        }
    }

    fn assign_place(&self, target: &Expr, v: Val, scope: &Scope) -> Result<(), Signal> {
        match target {
            Expr::Var(name) => {
                if !scope.set(name, v) {
                    return Err(err(format!("interp: assignment to unbound {}", name)));
                }
            }
            Expr::Unary {
                op: UnOp::Deref,
                rhs,
            } => {
                let target = self.eval_expr(rhs, scope)?;
                match target {
                    Val::Ref { slot, mutable } => {
                        if !mutable {
                            return Err(err("interp: assignment through immutable ref"));
                        }
                        *slot.borrow_mut() = v;
                    }
                    Val::VecElemRef {
                        vec,
                        index,
                        mutable,
                    } => {
                        if !mutable {
                            return Err(err(
                                "interp: assignment through immutable Vec element ref",
                            ));
                        }
                        let mut items = vec.borrow_mut();
                        let slot = items
                            .get_mut(index)
                            .ok_or_else(|| err("interp: dangling Vec element ref"))?;
                        *slot = v;
                    }
                    other => {
                        return Err(err(format!(
                            "interp: deref assignment target is {}",
                            other.kind()
                        )));
                    }
                }
            }
            Expr::Index { base, index } => {
                let collection = self.eval_expr(base, scope)?;
                let i = self
                    .eval_expr(index, scope)?
                    .as_i64("index assignment")
                    .map_err(Signal::Error)?;
                if i < 0 {
                    return Err(err("interp: negative index assignment"));
                }
                match deref_value(collection) {
                    Val::Vec(vs) => {
                        let mut vec = vs.borrow_mut();
                        let slot = vec
                            .get_mut(i as usize)
                            .ok_or_else(|| err(format!("interp: index {} out of bounds", i)))?;
                        *slot = v;
                    }
                    other => {
                        return Err(err(format!("interp: index assignment on {}", other.kind())));
                    }
                }
            }
            Expr::TupleIndex { base, index } => {
                let slot = place_slot(base, scope)?;
                assign_tuple_index(slot, *index, v)?;
            }
            Expr::Field { base, name } => {
                let slot = place_slot(base, scope)?;
                assign_field(slot, name, v)?;
            }
            _ => return Err(err("interp: unsupported assignment target")),
        }
        Ok(())
    }

    fn eval_binary(&self, op: BinOp, lhs: &Expr, rhs: &Expr, scope: &Scope) -> R {
        if let BinOp::And = op {
            let l = self
                .eval_expr(lhs, scope)?
                .as_bool("&&")
                .map_err(Signal::Error)?;
            if !l {
                return Ok(Val::Bool(false));
            }
            let r = self
                .eval_expr(rhs, scope)?
                .as_bool("&&")
                .map_err(Signal::Error)?;
            return Ok(Val::Bool(r));
        }
        if let BinOp::Or = op {
            let l = self
                .eval_expr(lhs, scope)?
                .as_bool("||")
                .map_err(Signal::Error)?;
            if l {
                return Ok(Val::Bool(true));
            }
            let r = self
                .eval_expr(rhs, scope)?
                .as_bool("||")
                .map_err(Signal::Error)?;
            return Ok(Val::Bool(r));
        }

        let l = self.eval_expr(lhs, scope)?;
        let r = self.eval_expr(rhs, scope)?;
        match op {
            BinOp::Add => {
                if let Val::String(s) = &l {
                    let mut out = s.borrow().clone();
                    out.push_str(&string_content(&r)?);
                    return Ok(Val::String(Rc::new(RefCell::new(out))));
                }
                if let (Val::F64(a), Val::F64(b)) = (&l, &r) {
                    let x = *a;
                    let y = *b;
                    return Ok(Val::F64(x + y));
                }
                let (a, b) = int2(&l, &r, "+")?;
                Ok(Val::I64(a.wrapping_add(b)))
            }
            BinOp::Sub => {
                if let (Val::F64(a), Val::F64(b)) = (&l, &r) {
                    let x = *a;
                    let y = *b;
                    return Ok(Val::F64(x - y));
                }
                let (a, b) = int2(&l, &r, "-")?;
                Ok(Val::I64(a.wrapping_sub(b)))
            }
            BinOp::Mul => {
                if let (Val::F64(a), Val::F64(b)) = (&l, &r) {
                    let x = *a;
                    let y = *b;
                    return Ok(Val::F64(x * y));
                }
                let (a, b) = int2(&l, &r, "*")?;
                Ok(Val::I64(a.wrapping_mul(b)))
            }
            BinOp::Div => {
                if let (Val::F64(a), Val::F64(b)) = (&l, &r) {
                    // rustc semantics: float division by zero yields inf/NaN.
                    let x = *a;
                    let y = *b;
                    return Ok(Val::F64(x / y));
                }
                let (a, b) = int2(&l, &r, "/")?;
                if b == 0 {
                    return Err(err("interp: divide by zero"));
                }
                Ok(Val::I64(a.wrapping_div(b)))
            }
            BinOp::Rem => {
                let (a, b) = int2(&l, &r, "%")?;
                if b == 0 {
                    return Err(err("interp: remainder by zero"));
                }
                Ok(Val::I64(a.wrapping_rem(b)))
            }
            BinOp::BitXor => {
                if let (Val::Bool(a), Val::Bool(b)) = (&l, &r) {
                    Ok(Val::Bool(*a ^ *b))
                } else {
                    let (a, b) = int2(&l, &r, "^")?;
                    Ok(Val::I64(a ^ b))
                }
            }
            BinOp::BitAnd => {
                if let (Val::Bool(a), Val::Bool(b)) = (&l, &r) {
                    Ok(Val::Bool(*a & *b))
                } else {
                    let (a, b) = int2(&l, &r, "&")?;
                    Ok(Val::I64(a & b))
                }
            }
            BinOp::BitOr => {
                if let (Val::Bool(a), Val::Bool(b)) = (&l, &r) {
                    Ok(Val::Bool(*a | *b))
                } else {
                    let (a, b) = int2(&l, &r, "|")?;
                    Ok(Val::I64(a | b))
                }
            }
            BinOp::Shl => {
                let (a, b) = int2(&l, &r, "<<")?;
                Ok(Val::I64(a << b))
            }
            BinOp::Shr => {
                let (a, b) = int2(&l, &r, ">>")?;
                Ok(Val::I64(a >> b))
            }
            BinOp::Eq => Ok(Val::Bool(value_eq(&l, &r))),
            BinOp::Ne => Ok(Val::Bool(!value_eq(&l, &r))),
            BinOp::Lt => cmp(&l, &r, "<", |a, b| a < b),
            BinOp::Le => cmp(&l, &r, "<=", |a, b| a <= b),
            BinOp::Gt => cmp(&l, &r, ">", |a, b| a > b),
            BinOp::Ge => cmp(&l, &r, ">=", |a, b| a >= b),
            BinOp::And | BinOp::Or => unreachable!("handled above"),
        }
    }

    fn format_println(&self, fmt: &str, args: &[Val]) -> Result<String, String> {
        let mut out = String::new();
        let mut arg_i = 0;
        let chars: Vec<char> = fmt.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '{' && chars.get(i + 1) == Some(&'}') {
                let v = args
                    .get(arg_i)
                    .ok_or_else(|| "println!: not enough arguments".to_string())?;
                // `{}` on a struct with a user Display impl: run its `fmt`
                // method into a String buffer (the Formatter model).
                let mut rendered: Option<String> = None;
                if let Val::Struct { name, .. } = deref_value(v.clone()) {
                    if self.methods.get(&((*name).clone(), String::from("fmt"))).is_some() {
                        let buf = Rc::new(RefCell::new(String::new()));
                        let recv = deref_value(v.clone());
                        let no_type_args: Vec<Type> = Vec::new();
                        self.call_method(
                            &name,
                            "fmt",
                            Some(recv),
                            &no_type_args,
                            vec![Val::String(buf.clone())],
                        )
                        .map_err(|sig| signal_message("Display::fmt", sig))?;
                        rendered = Some(buf.borrow().clone());
                    }
                }
                match rendered {
                    Some(s) => out.push_str(&s),
                    None => out.push_str(&v.display()),
                }
                arg_i += 1;
                i += 2;
            } else if c == '{'
                && chars.get(i + 1) == Some(&':')
                && chars.get(i + 2) == Some(&'?')
                && chars.get(i + 3) == Some(&'}')
            {
                let v = args
                    .get(arg_i)
                    .ok_or_else(|| "println!: not enough arguments".to_string())?;
                out.push_str(&v.debug_display(&self.enum_named_fields));
                arg_i += 1;
                i += 4;
            } else if c == '{'
                && chars.get(i + 1) == Some(&':')
                && chars.get(i + 2) == Some(&'#')
                && chars.get(i + 3) == Some(&'?')
                && chars.get(i + 4) == Some(&'}')
            {
                let v = args
                    .get(arg_i)
                    .ok_or_else(|| "println!: not enough arguments".to_string())?;
                out.push_str(&v.debug_fmt(true, 0, &self.enum_named_fields));
                arg_i += 1;
                i += 5;
            } else if c == '{'
                && chars.get(i + 1) == Some(&':')
                && chars.get(i + 2) == Some(&'0')
                && chars.get(i + 3) == Some(&'1')
                && chars.get(i + 4) == Some(&'6')
                && chars.get(i + 5) == Some(&'x')
                && chars.get(i + 6) == Some(&'}')
            {
                let v = args
                    .get(arg_i)
                    .ok_or_else(|| "println!: not enough arguments".to_string())?;
                let n = v.as_i64("{:016x}").map_err(|e| e.to_string())?;
                out.push_str(&lower_hex_16(n));
                arg_i += 1;
                i += 7;
            } else if c == '{'
                && chars.get(i + 1) == Some(&':')
                && matches!(
                    chars.get(i + 2),
                    Some('x') | Some('X') | Some('b') | Some('o')
                )
                && chars.get(i + 3) == Some(&'}')
            {
                let v = args
                    .get(arg_i)
                    .ok_or_else(|| "println!: not enough arguments".to_string())?;
                let n = v.as_i64("radix format").map_err(|e| e.to_string())?;
                let spec = *chars.get(i + 2).unwrap();
                let s = match spec {
                    'x' => to_radix(n, 16, false),
                    'X' => to_radix(n, 16, true),
                    'b' => to_radix(n, 2, false),
                    _ => to_radix(n, 8, false),
                };
                out.push_str(&s);
                arg_i += 1;
                i += 4;
            } else if let Some((precision, next_i)) =
                interp_fixed_precision_placeholder(&chars, i)
            {
                let v = args
                    .get(arg_i)
                    .ok_or_else(|| "println!: not enough arguments".to_string())?;
                match deref_value(v.clone()) {
                    Val::F64(f) => out.push_str(&format!("{:.*}", precision, f)),
                    Val::I64(n) => out.push_str(&format!("{}", n)),
                    other => {
                        return Err(format!(
                            "println!: fixed precision expects a number, got {}",
                            other.kind()
                        ))
                    }
                }
                arg_i += 1;
                i = next_i;
            } else if let Some((width, next_i)) = interp_left_align_placeholder(&chars, i) {
                let v = args
                    .get(arg_i)
                    .ok_or_else(|| "println!: not enough arguments".to_string())?;
                let text = v.display();
                out.push_str(&text);
                let mut pad = text.len();
                while pad < width {
                    out.push(' ');
                    pad += 1;
                }
                arg_i += 1;
                i = next_i;
            } else if let Some((width, next_i)) = interp_right_align_placeholder(&chars, i) {
                let v = args
                    .get(arg_i)
                    .ok_or_else(|| "println!: not enough arguments".to_string())?;
                let text = v.display();
                let mut pad = text.len();
                while pad < width {
                    out.push(' ');
                    pad += 1;
                }
                out.push_str(&text);
                arg_i += 1;
                i = next_i;
            } else if c == '{' && chars.get(i + 1) == Some(&'{') {
                out.push('{');
                i += 2;
            } else if c == '}' && chars.get(i + 1) == Some(&'}') {
                out.push('}');
                i += 2;
            } else {
                out.push(c);
                i += 1;
            }
        }
        if arg_i != args.len() {
            return Err("println!: too many arguments".to_string());
        }
        Ok(out)
    }
}

fn interp_left_align_placeholder(chars: &[char], i: usize) -> Option<(usize, usize)> {
    interp_align_placeholder(chars, i, '<')
}

fn interp_right_align_placeholder(chars: &[char], i: usize) -> Option<(usize, usize)> {
    interp_align_placeholder(chars, i, '>')
}

fn interp_fixed_precision_placeholder(chars: &[char], i: usize) -> Option<(usize, usize)> {
    if chars.get(i) != Some(&'{')
        || chars.get(i + 1) != Some(&':')
        || chars.get(i + 2) != Some(&'.')
    {
        return None;
    }
    let mut j = i + 3;
    let mut precision: usize = 0;
    let mut saw_digit = false;
    while j < chars.len() {
        let ch = chars[j];
        if ch == '}' {
            if saw_digit {
                return Some((precision, j + 1));
            }
            return None;
        }
        if !ch.is_ascii_digit() {
            return None;
        }
        saw_digit = true;
        let digit = ch as usize - '0' as usize;
        if precision > 6553 || (precision == 6553 && digit > 5) {
            return None;
        }
        precision = precision * 10 + digit;
        j += 1;
    }
    None
}

fn interp_align_placeholder(chars: &[char], i: usize, align: char) -> Option<(usize, usize)> {
    if chars.get(i) != Some(&'{')
        || chars.get(i + 1) != Some(&':')
        || chars.get(i + 2) != Some(&align)
    {
        return None;
    }
    let mut j = i + 3;
    let mut width: usize = 0;
    let mut saw_digit = false;
    while j < chars.len() {
        let ch = chars[j];
        if ch == '}' {
            if saw_digit {
                return Some((width, j + 1));
            }
            return None;
        }
        if !ch.is_ascii_digit() {
            return None;
        }
        saw_digit = true;
        width = width * 10 + (ch as usize - '0' as usize);
        j += 1;
    }
    None
}

/// Try to match `pat` against `val`, binding captures into `scope`. Returns
/// whether it matched. (Partial bindings are harmless: a failed arm's scope is
/// discarded by the caller before the next arm is tried.)
fn try_match(
    pat: &Pattern,
    val: &Val,
    scope: &Scope,
    enum_named_fields: &HashMap<(String, String), Vec<String>>,
) -> bool {
    try_match_with_mode(pat, val, scope, enum_named_fields, false)
}

fn try_match_with_mode(
    pat: &Pattern,
    val: &Val,
    scope: &Scope,
    enum_named_fields: &HashMap<(String, String), Vec<String>>,
    default_ref_bind: bool,
) -> bool {
    if let Val::VecElemRef { vec, index, .. } = val {
        if !matches!(
            pat,
            Pattern::Wild | Pattern::Bind(_) | Pattern::BindRef { .. } | Pattern::Ref { .. }
        ) {
            let current = vec.borrow().get(*index).cloned();
            return current
                .map(|v| try_match_with_mode(pat, &v, scope, enum_named_fields, true))
                .unwrap_or(false);
        }
    }
    if let Val::Ref { slot, .. } = val {
        if !matches!(
            pat,
            Pattern::Wild | Pattern::Bind(_) | Pattern::BindRef { .. } | Pattern::Ref { .. }
        ) {
            return try_match_with_mode(pat, &slot.borrow(), scope, enum_named_fields, true);
        }
    }
    match pat {
        Pattern::Wild => true,
        Pattern::Bind(name) => {
            let bound = if default_ref_bind {
                Val::Ref {
                    slot: Rc::new(RefCell::new(val.clone())),
                    mutable: false,
                }
            } else {
                val.clone()
            };
            scope.define(name, bound);
            true
        }
        Pattern::BindRef { name, mutable } => {
            scope.define(
                name,
                Val::Ref {
                    slot: Rc::new(RefCell::new(val.clone())),
                    mutable: *mutable,
                },
            );
            true
        }
        Pattern::Int(n) => matches!(val, Val::I64(m) if m == n),
        Pattern::IntRange { start, end } => {
            matches!(val, Val::I64(m) if *m >= *start && *m <= *end)
        }
        Pattern::Char(ch) => matches!(val, Val::Char(m) if m == ch),
        Pattern::CharRange { start, end } => {
            matches!(val, Val::Char(ch) if *ch >= *start && *ch <= *end)
        }
        Pattern::Str(s) => string_content(val).map(|got| got == *s).unwrap_or(false),
        Pattern::Bool(b) => matches!(val, Val::Bool(m) if m == b),
        Pattern::BindAt { name, sub } => {
            if !try_match_with_mode(sub, val, scope, enum_named_fields, default_ref_bind) {
                return false;
            }
            let bound = if default_ref_bind {
                Val::Ref {
                    slot: Rc::new(RefCell::new(val.clone())),
                    mutable: false,
                }
            } else {
                val.clone()
            };
            scope.define(name, bound);
            true
        }
        Pattern::Tuple(subs) => match val {
            Val::Unit if subs.is_empty() => true,
            Val::Tuple(vs) if vs.len() == subs.len() => subs.iter().zip(vs.iter()).all(|(p, v)| {
                try_match_with_mode(p, v, scope, enum_named_fields, default_ref_bind)
            }),
            _ => false,
        },
        Pattern::Slice {
            prefix,
            rest,
            suffix,
        } => match val {
            Val::Vec(vs) => {
                let items = vs.borrow();
                let n = items.len();
                if rest.is_none() {
                    if n != prefix.len() {
                        return false;
                    }
                } else if n < prefix.len() + suffix.len() {
                    return false;
                }
                let mut i = 0;
                while i < prefix.len() {
                    if !try_match_with_mode(
                        &prefix[i],
                        &items[i],
                        scope,
                        enum_named_fields,
                        default_ref_bind,
                    ) {
                        return false;
                    }
                    i += 1;
                }
                let mut j = 0;
                while j < suffix.len() {
                    let idx = n - suffix.len() + j;
                    if !try_match_with_mode(
                        &suffix[j],
                        &items[idx],
                        scope,
                        enum_named_fields,
                        default_ref_bind,
                    ) {
                        return false;
                    }
                    j += 1;
                }
                if let Some(Some(name)) = rest {
                    let mut mid: Vec<Val> = Vec::new();
                    let mut k = prefix.len();
                    while k < n - suffix.len() {
                        mid.push(items[k].clone());
                        k += 1;
                    }
                    scope.define(name, Val::Vec(Rc::new(RefCell::new(mid))));
                }
                true
            }
            _ => false,
        },
        Pattern::Or(items) => items
            .iter()
            .any(|p| try_match_with_mode(p, val, scope, enum_named_fields, default_ref_bind)),
        Pattern::Ref { mutable, sub } => match val {
            Val::Ref { slot, mutable: m } if !*mutable || *m => {
                try_match_with_mode(sub, &slot.borrow(), scope, enum_named_fields, false)
            }
            Val::VecElemRef {
                vec,
                index,
                mutable: m,
            } if !*mutable || *m => {
                let current = vec.borrow().get(*index).cloned();
                current
                    .map(|v| try_match_with_mode(sub, &v, scope, enum_named_fields, false))
                    .unwrap_or(false)
            }
            _ => false,
        },
        Pattern::Struct { name, fields, .. } => match val {
            Val::Struct {
                name: got,
                fields: fs,
                ..
            } if got.as_str() == name => fields.iter().all(|(fname, pat)| {
                fs.iter()
                    .find(|(k, _)| k == fname)
                    .map(|(_, v)| {
                        try_match_with_mode(pat, v, scope, enum_named_fields, default_ref_bind)
                    })
                    .unwrap_or(false)
            }),
            _ => false,
        },
        Pattern::Enum {
            enum_name,
            variant,
            sub,
        } => match val {
            Val::Enum {
                enum_name: en,
                variant: var,
                data,
            } if en.as_str() == enum_name && var.as_str() == variant && data.len() == sub.len() => {
                sub.iter().zip(data.iter()).all(|(p, v)| {
                    try_match_with_mode(p, v, scope, enum_named_fields, default_ref_bind)
                })
            }
            _ => false,
        },
        Pattern::EnumStruct {
            enum_name,
            variant,
            fields,
            ..
        } => match val {
            Val::Enum {
                enum_name: en,
                variant: var,
                data,
            } if en.as_str() == enum_name && var.as_str() == variant => {
                let names = match enum_named_fields.get(&(enum_name.clone(), variant.clone())) {
                    Some(names) => names,
                    None => return false,
                };
                fields.iter().all(|(fname, pat)| {
                    names
                        .iter()
                        .position(|name| name == fname)
                        .and_then(|idx| data.get(idx))
                        .map(|v| {
                            try_match_with_mode(pat, v, scope, enum_named_fields, default_ref_bind)
                        })
                        .unwrap_or(false)
                })
            }
            _ => false,
        },
    }
}

fn interp_impl_target_name(ty: &Type) -> Result<String, String> {
    match ty {
        Type::Named(name) => Ok(name.clone()),
        Type::Generic { name, .. } => Ok(name.clone()),
        other => Err(format!(
            "interp: impl target {:?} is not supported yet",
            other
        )),
    }
}

fn runtime_type_name(v: &Val) -> Option<String> {
    match v {
        Val::I64(_) => Some("i64".to_string()),
        Val::Bool(_) => Some("bool".to_string()),
        Val::Unit => Some("unit".to_string()),
        Val::Tuple(_) => Some("tuple".to_string()),
        Val::Struct { name, .. } => Some((**name).clone()),
        Val::Enum { enum_name, .. } => Some((**enum_name).clone()),
        Val::Vec(_) => Some("Vec".to_string()),
        Val::String(_) => Some("String".to_string()),
        Val::PathBuf(_) => Some("PathBuf".to_string()),
        Val::Command(_) => Some("Command".to_string()),
        Val::ExitStatus(_) => Some("ExitStatus".to_string()),
        Val::Str(_) => Some("str".to_string()),
        Val::Char(_) => Some("char".to_string()),
        Val::Box(_) => Some("Box".to_string()),
        Val::RcVal(_) => Some("Rc".to_string()),
        Val::RefCellVal(_) => Some("RefCell".to_string()),
        Val::HashMap(_) => Some("HashMap".to_string()),
        Val::HashEntry { .. } => Some("HashEntry".to_string()),
        Val::Iter(_) => Some("Iter".to_string()),
        Val::Ref { slot, .. } => runtime_type_name(&slot.borrow()),
        Val::VecElemRef { vec, index, .. } => vec.borrow().get(*index).and_then(runtime_type_name),
        _ => None,
    }
}

fn is_int_runtime_target(target: &str) -> bool {
    matches!(target, "i64" | "i32" | "u32" | "u64" | "u8" | "usize")
}

fn call_int_method(target: &str, name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: {}::{} needs a receiver", target, name)))?;
    match name {
        "to_string" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: {}::to_string expected 0 args, got {}",
                    target,
                    args.len()
                )));
            }
            Ok(Val::String(Rc::new(RefCell::new(receiver.display()))))
        }
        "wrapping_neg" if target == "i64" || target == "i32" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: {}::wrapping_neg expected 0 args, got {}",
                    target,
                    args.len()
                )));
            }
            let a = receiver
                .as_i64("wrapping_neg receiver")
                .map_err(Signal::Error)?;
            Ok(Val::I64(a.wrapping_neg()))
        }
        "wrapping_add" | "wrapping_sub" | "wrapping_mul" | "wrapping_div" | "wrapping_rem"
            if target == "i64"
                || target == "i32"
                || target == "usize"
                || target == "u32"
                || target == "u64"
                || target == "u8" =>
        {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: {}::{} expected 1 arg, got {}",
                    target,
                    name,
                    args.len()
                )));
            }
            let a = receiver
                .as_i64(&format!("{}::{} receiver", target, name))
                .map_err(Signal::Error)?;
            let b = args[0]
                .as_i64(&format!("{}::{} arg", target, name))
                .map_err(Signal::Error)?;
            if (name == "wrapping_div" || name == "wrapping_rem") && b == 0 {
                return Err(err(format!("interp: {} by zero", name)));
            }
            let out = match name {
                "wrapping_add" => a.wrapping_add(b),
                "wrapping_sub" => a.wrapping_sub(b),
                "wrapping_mul" => a.wrapping_mul(b),
                "wrapping_div" => a.wrapping_div(b),
                "wrapping_rem" => a.wrapping_rem(b),
                _ => unreachable!(),
            };
            Ok(Val::I64(out))
        }
        "saturating_sub" if target == "usize" || target == "i64" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: {}::saturating_sub expected 1 arg, got {}",
                    target,
                    args.len()
                )));
            }
            let a = receiver
                .as_i64("saturating_sub receiver")
                .map_err(Signal::Error)?;
            let b = args[0]
                .as_i64("saturating_sub arg")
                .map_err(Signal::Error)?;
            Ok(Val::I64(a.saturating_sub(b).max(0)))
        }
        "pow" if is_int_runtime_target(target) => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: {}::pow expected 1 arg, got {}",
                    target,
                    args.len()
                )));
            }
            let a = receiver
                .as_i64(&format!("{}::pow receiver", target))
                .map_err(Signal::Error)?;
            let b = args[0]
                .as_i64(&format!("{}::pow arg", target))
                .map_err(Signal::Error)?;
            Ok(Val::I64(a.pow(b as u32)))
        }
        "signum" if is_int_runtime_target(target) => {
            if !args.is_empty() {
                return Err(err(format!("interp: {}::signum expects 0 args", target)));
            }
            let a = receiver
                .as_i64(&format!("{}::signum receiver", target))
                .map_err(Signal::Error)?;
            Ok(Val::I64(a.signum()))
        }
        "rem_euclid" if is_int_runtime_target(target) => {
            if args.len() != 1 {
                return Err(err(format!("interp: {}::rem_euclid expects 1 arg", target)));
            }
            let a = receiver
                .as_i64(&format!("{}::rem_euclid receiver", target))
                .map_err(Signal::Error)?;
            let b = args[0]
                .as_i64(&format!("{}::rem_euclid arg", target))
                .map_err(Signal::Error)?;
            if b == 0 {
                return Err(err("interp: rem_euclid by zero"));
            }
            Ok(Val::I64(a.rem_euclid(b)))
        }
        // i64-gated for value-model fidelity: the interp holds a single-width
        // i64, so overflow/bit methods match rustc only at 64-bit width. Narrow
        // widths fall through to `unsupported` (a feature gap, never a divergence).
        "abs" if target == "i64" => {
            if !args.is_empty() {
                return Err(err(format!("interp: {}::abs expects 0 args", target)));
            }
            let a = receiver
                .as_i64(&format!("{}::abs receiver", target))
                .map_err(Signal::Error)?;
            Ok(Val::I64(a.abs()))
        }
        "cmp" if is_int_runtime_target(target) => {
            if args.len() != 1 {
                return Err(err(format!("interp: {}::cmp expects 1 arg", target)));
            }
            let a = receiver
                .as_i64(&format!("{}::cmp receiver", target))
                .map_err(Signal::Error)?;
            let b = deref_value(args[0].clone())
                .as_i64(&format!("{}::cmp arg", target))
                .map_err(Signal::Error)?;
            Ok(ordering_value(if a < b {
                -1
            } else if a > b {
                1
            } else {
                0
            }))
        }
        "checked_add" | "checked_sub" | "checked_mul" | "checked_div" | "checked_rem"
            if target == "i64" =>
        {
            if args.len() != 1 {
                return Err(err(format!("interp: {}::{} expects 1 arg", target, name)));
            }
            let a = receiver
                .as_i64(&format!("{}::{} receiver", target, name))
                .map_err(Signal::Error)?;
            let b = args[0]
                .as_i64(&format!("{}::{} arg", target, name))
                .map_err(Signal::Error)?;
            let out = match name {
                "checked_add" => a.checked_add(b),
                "checked_sub" => a.checked_sub(b),
                "checked_mul" => a.checked_mul(b),
                "checked_div" => a.checked_div(b),
                _ => a.checked_rem(b),
            };
            match out {
                Some(v) => Ok(option_some(Val::I64(v))),
                None => Ok(option_none()),
            }
        }
        "saturating_add" if target == "i64" || target == "usize" => {
            if args.len() != 1 {
                return Err(err(format!("interp: {}::saturating_add expects 1 arg", target)));
            }
            let a = receiver
                .as_i64(&format!("{}::saturating_add receiver", target))
                .map_err(Signal::Error)?;
            let b = args[0]
                .as_i64(&format!("{}::saturating_add arg", target))
                .map_err(Signal::Error)?;
            Ok(Val::I64(a.saturating_add(b)))
        }
        "count_ones" | "leading_zeros" | "trailing_zeros"
            if target == "i64" || target == "usize" =>
        {
            if !args.is_empty() {
                return Err(err(format!("interp: {}::{} expects 0 args", target, name)));
            }
            let a = receiver
                .as_i64(&format!("{}::{} receiver", target, name))
                .map_err(Signal::Error)?;
            let out = match name {
                "count_ones" => a.count_ones(),
                "leading_zeros" => a.leading_zeros(),
                _ => a.trailing_zeros(),
            };
            Ok(Val::I64(out as i64))
        }
        "max" | "min" if target == "i64" || target == "usize" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: {}::{} expected 1 arg, got {}",
                    target,
                    name,
                    args.len()
                )));
            }
            let a = receiver
                .as_i64(&format!("{}::{} receiver", target, name))
                .map_err(Signal::Error)?;
            let b = args[0]
                .as_i64(&format!("{}::{} arg", target, name))
                .map_err(Signal::Error)?;
            Ok(Val::I64(if name == "max" { a.max(b) } else { a.min(b) }))
        }
        other => Err(err(format!(
            "interp: unsupported integer method {}::{}",
            target, other
        ))),
    }
}

fn call_bool_method(interp: &Interp, name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: bool::{} needs a receiver", name)))?;
    match name {
        "to_string" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: bool::to_string expected 0 args, got {}",
                    args.len()
                )));
            }
            Ok(Val::String(Rc::new(RefCell::new(receiver.display()))))
        }
        "then" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: bool::then expected 1 arg, got {}",
                    args.len()
                )));
            }
            if receiver.as_bool("bool::then").map_err(Signal::Error)? {
                Ok(option_some(interp.call_callable(&args[0], Vec::new())?))
            } else {
                Ok(option_none())
            }
        }
        other => Err(err(format!("interp: unsupported bool method {}", other))),
    }
}

fn call_hashmap_method(name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: HashMap::{} needs a receiver", name)))?;
    let map = hashmap_handle(receiver)?;
    match name {
        "insert" => {
            if args.len() != 2 {
                return Err(err(format!(
                    "interp: HashMap::insert expected 2 args, got {}",
                    args.len()
                )));
            }
            let key = normalized_key(&args[0]);
            let mut entries = map.borrow_mut();
            if let Some((_, slot)) = entries.iter_mut().find(|(k, _)| *k == key) {
                let old = slot.borrow().clone();
                *slot.borrow_mut() = args[1].clone();
                Ok(option_some(old))
            } else {
                entries.push((key, Rc::new(RefCell::new(args[1].clone()))));
                Ok(option_none())
            }
        }
        "contains_key" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: HashMap::contains_key expected 1 arg, got {}",
                    args.len()
                )));
            }
            let key = normalized_key(&args[0]);
            Ok(Val::Bool(map.borrow().iter().any(|(k, _)| *k == key)))
        }
        "get" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: HashMap::get expected 1 arg, got {}",
                    args.len()
                )));
            }
            let key = normalized_key(&args[0]);
            let entries = map.borrow();
            if let Some((_, slot)) = entries.iter().find(|(k, _)| *k == key) {
                Ok(option_some(Val::Ref {
                    slot: slot.clone(),
                    mutable: false,
                }))
            } else {
                Ok(option_none())
            }
        }
        "get_mut" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: HashMap::get_mut expected 1 arg, got {}",
                    args.len()
                )));
            }
            let key = normalized_key(&args[0]);
            let entries = map.borrow();
            if let Some((_, slot)) = entries.iter().find(|(k, _)| *k == key) {
                Ok(option_some(Val::Ref {
                    slot: slot.clone(),
                    mutable: true,
                }))
            } else {
                Ok(option_none())
            }
        }
        "remove" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: HashMap::remove expected 1 arg, got {}",
                    args.len()
                )));
            }
            let key = normalized_key(&args[0]);
            let mut entries = map.borrow_mut();
            if let Some(pos) = entries.iter().position(|(k, _)| *k == key) {
                let (_, slot) = entries.remove(pos);
                let old = slot.borrow().clone();
                Ok(option_some(old))
            } else {
                Ok(option_none())
            }
        }
        "len" => {
            if !args.is_empty() {
                return Err(err("interp: HashMap::len expects 0 args"));
            }
            Ok(Val::I64(map.borrow().len() as i64))
        }
        "is_empty" => {
            if !args.is_empty() {
                return Err(err("interp: HashMap::is_empty expects 0 args"));
            }
            Ok(Val::Bool(map.borrow().is_empty()))
        }
        "keys" | "values" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: HashMap::{} expects 0 args, got {}",
                    name,
                    args.len()
                )));
            }
            let mut items: Vec<Val> = Vec::new();
            {
                let entries = map.borrow();
                for (k, slot) in entries.iter() {
                    if name == "keys" {
                        items.push(Val::Ref {
                            slot: Rc::new(RefCell::new(k.clone())),
                            mutable: false,
                        });
                    } else {
                        items.push(Val::Ref {
                            slot: slot.clone(),
                            mutable: false,
                        });
                    }
                }
            }
            Ok(iter_value(items))
        }
        "iter" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: HashMap::iter expects 0 args, got {}",
                    args.len()
                )));
            }
            let items = map
                .borrow()
                .iter()
                .map(|(k, slot)| {
                    Val::Tuple(Rc::new(vec![
                        Val::Ref {
                            slot: Rc::new(RefCell::new(k.clone())),
                            mutable: false,
                        },
                        Val::Ref {
                            slot: slot.clone(),
                            mutable: false,
                        },
                    ]))
                })
                .collect();
            Ok(iter_value(items))
        }
        "entry" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: HashMap::entry expected 1 arg, got {}",
                    args.len()
                )));
            }
            Ok(Val::HashEntry {
                map,
                key: Rc::new(normalized_key(&args[0])),
            })
        }
        other => Err(err(format!("interp: unsupported HashMap method {}", other))),
    }
}

fn call_hashentry_method(interp: &Interp, name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let (map, key) = match receiver
        .ok_or_else(|| err(format!("interp: HashEntry::{} needs a receiver", name)))?
    {
        Val::HashEntry { map, key } => (map, key),
        other => return Err(err(format!("interp: HashEntry method on {}", other.kind()))),
    };
    match name {
        "or_insert" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: HashEntry::or_insert expected 1 arg, got {}",
                    args.len()
                )));
            }
            let slot = hashmap_or_insert(&map, &key, args[0].clone());
            Ok(Val::Ref {
                slot,
                mutable: true,
            })
        }
        "or_insert_with" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: HashEntry::or_insert_with expected 1 arg, got {}",
                    args.len()
                )));
            }
            let slot = match hashmap_get_slot(&map, &key) {
                Some(slot) => slot,
                None => {
                    let value = interp.call_callable(&args[0], Vec::new())?;
                    hashmap_or_insert(&map, &key, value)
                }
            };
            Ok(Val::Ref {
                slot,
                mutable: true,
            })
        }
        "and_modify" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: HashEntry::and_modify expected 1 arg, got {}",
                    args.len()
                )));
            }
            if let Some(slot) = hashmap_get_slot(&map, &key) {
                interp.call_callable(
                    &args[0],
                    vec![Val::Ref {
                        slot,
                        mutable: true,
                    }],
                )?;
            }
            Ok(Val::HashEntry { map, key })
        }
        other => Err(err(format!(
            "interp: unsupported HashEntry method {}",
            other
        ))),
    }
}

fn hashmap_get_slot(map: &Rc<RefCell<Vec<(Val, Slot)>>>, key: &Val) -> Option<Slot> {
    let entries = map.borrow();
    for (k, slot) in entries.iter() {
        if k == key {
            return Some(slot.clone());
        }
    }
    None
}

fn hashmap_or_insert(map: &Rc<RefCell<Vec<(Val, Slot)>>>, key: &Val, value: Val) -> Slot {
    if let Some(slot) = hashmap_get_slot(map, key) {
        return slot;
    }
    let slot = Rc::new(RefCell::new(value));
    map.borrow_mut().push((key.clone(), slot.clone()));
    slot
}

fn hashmap_handle(v: Val) -> Result<Rc<RefCell<Vec<(Val, Slot)>>>, Signal> {
    match v {
        Val::HashMap(m) => Ok(m),
        Val::Ref { slot, .. } => match slot.borrow().clone() {
            Val::HashMap(m) => Ok(m),
            other => Err(err(format!("interp: HashMap method on {}", other.kind()))),
        },
        Val::VecElemRef { vec, index, .. } => hashmap_handle(vec_elem_value(&vec, index)?),
        other => Err(err(format!("interp: HashMap method on {}", other.kind()))),
    }
}

fn normalized_key(v: &Val) -> Val {
    match v {
        Val::Ref { slot, .. } => normalized_key(&slot.borrow()),
        Val::VecElemRef { vec, index, .. } => vec
            .borrow()
            .get(*index)
            .map(normalized_key)
            .unwrap_or(Val::Unit),
        Val::Str(s) => Val::String(Rc::new(RefCell::new((**s).clone()))),
        Val::String(s) => Val::String(Rc::new(RefCell::new(s.borrow().clone()))),
        other => other.clone(),
    }
}

fn option_some(v: Val) -> Val {
    Val::Enum {
        enum_name: Rc::new("Option".to_string()),
        variant: Rc::new("Some".to_string()),
        data: Rc::new(vec![v]),
    }
}

fn option_none() -> Val {
    Val::Enum {
        enum_name: Rc::new("Option".to_string()),
        variant: Rc::new("None".to_string()),
        data: Rc::new(Vec::new()),
    }
}

fn result_ok(v: Val) -> Val {
    Val::Enum {
        enum_name: Rc::new("Result".to_string()),
        variant: Rc::new("Ok".to_string()),
        data: Rc::new(vec![v]),
    }
}

fn result_err(msg: String) -> Val {
    Val::Enum {
        enum_name: Rc::new("Result".to_string()),
        variant: Rc::new("Err".to_string()),
        data: Rc::new(vec![Val::String(Rc::new(RefCell::new(msg)))]),
    }
}

fn question_error_data(data: Rc<Vec<Val>>) -> Rc<Vec<Val>> {
    if data.len() == 1 {
        if let Val::Str(s) = &data[0] {
            return Rc::new(vec![Val::String(Rc::new(RefCell::new((**s).clone())))]);
        }
    }
    data
}

fn call_refcell_method(name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: RefCell::{} needs a receiver", name)))?;
    let slot = refcell_slot(&receiver)?;
    match name {
        "borrow" | "borrow_mut" => {
            if !args.is_empty() {
                return Err(err(format!("interp: RefCell::{} expects 0 args", name)));
            }
            Ok(Val::Ref {
                slot,
                mutable: name == "borrow_mut",
            })
        }
        "into_inner" => {
            if !args.is_empty() {
                return Err(err("interp: RefCell::into_inner expects 0 args"));
            }
            Ok(slot.borrow().clone())
        }
        other => Err(err(format!("interp: unsupported RefCell method {}", other))),
    }
}

fn refcell_slot(v: &Val) -> Result<Slot, Signal> {
    match v {
        Val::RefCellVal(slot) => Ok(slot.clone()),
        Val::Ref { slot, .. } => match slot.borrow().clone() {
            Val::RefCellVal(inner) => Ok(inner),
            other => Err(err(format!("interp: RefCell method on {}", other.kind()))),
        },
        Val::VecElemRef { vec, index, .. } => refcell_slot(&vec_elem_value(vec, *index)?),
        other => Err(err(format!("interp: RefCell method on {}", other.kind()))),
    }
}

fn rc_slot(v: &Val) -> Result<Slot, Signal> {
    match v {
        Val::RcVal(slot) => Ok(slot.clone()),
        Val::Ref { slot, .. } => match slot.borrow().clone() {
            Val::RcVal(inner) => Ok(inner),
            // A reference passed through a function/pattern parameter may
            // already point at the Rc allocation slot rather than at a local
            // variable containing `Val::RcVal`. Static type checking has
            // established `&Rc<T>` before this runtime path.
            _ => Ok(slot.clone()),
        },
        Val::VecElemRef { vec, index, .. } => rc_slot(&vec_elem_value(vec, *index)?),
        other => Err(err(format!("interp: Rc::clone on {}", other.kind()))),
    }
}

fn boxlike_slot(target: &str, v: &Val) -> Result<Slot, Signal> {
    match (target, v) {
        ("Box", Val::Box(slot)) | ("Rc", Val::RcVal(slot)) => Ok(slot.clone()),
        (_, Val::Ref { slot, .. }) => boxlike_slot(target, &slot.borrow()),
        (_, Val::VecElemRef { vec, index, .. }) => {
            boxlike_slot(target, &vec_elem_value(vec, *index)?)
        }
        _ => Err(err(format!("interp: {} method on {}", target, v.kind()))),
    }
}

fn coerce_arg_for_param(v: Val, ty: &Type) -> Val {
    match ty {
        Type::Generic { name, args }
            if name == "Vec" && args.len() == 1 && is_char_vec_arg(&args[0]) =>
        {
            coerce_string_to_char_vec(v)
        }
        Type::Ref { mutable: false, .. } => match v {
            Val::Ref { slot, .. } => {
                let derefed = {
                    let borrowed = slot.borrow();
                    match &*borrowed {
                        Val::Box(inner) | Val::RcVal(inner) => Some(inner.clone()),
                        Val::Ref { slot: inner, .. } => Some(inner.clone()),
                        _ => None,
                    }
                };
                match derefed {
                    Some(inner) => Val::Ref {
                        slot: inner,
                        mutable: false,
                    },
                    None => Val::Ref {
                        slot,
                        mutable: false,
                    },
                }
            }
            Val::VecElemRef { vec, index, .. } => Val::VecElemRef {
                vec,
                index,
                mutable: false,
            },
            other => other,
        },
        _ => v,
    }
}

fn is_char_vec_arg(ty: &Type) -> bool {
    match ty {
        Type::Char => true,
        Type::Named(name) => name == "Val",
        _ => false,
    }
}

fn coerce_string_to_char_vec(v: Val) -> Val {
    match v {
        Val::String(s) => Val::Vec(Rc::new(RefCell::new(
            s.borrow().chars().map(Val::Char).collect(),
        ))),
        Val::Str(s) => Val::Vec(Rc::new(RefCell::new(s.chars().map(Val::Char).collect()))),
        other => other,
    }
}

fn string_char_items(v: &Val) -> Vec<Val> {
    match v {
        Val::String(s) => s.borrow().chars().map(Val::Char).collect(),
        Val::Str(s) => s.chars().map(Val::Char).collect(),
        _ => Vec::new(),
    }
}

fn call_boxlike_method(target: &str, name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: {}::{} needs a receiver", target, name)))?;
    match name {
        "to_string" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: {}::to_string expected 0 args, got {}",
                    target,
                    args.len()
                )));
            }
            let inner = boxlike_slot(target, &receiver)?.borrow().clone();
            Ok(Val::String(Rc::new(RefCell::new(inner.display()))))
        }
        "as_str" => {
            if !args.is_empty() {
                return Err(err(format!("interp: {}::as_str expects 0 args", target)));
            }
            let outer = boxlike_slot(target, &receiver)?;
            let inner = outer.borrow().clone();
            match inner {
                Val::String(_) | Val::Str(_) => Ok(Val::Str(Rc::new(string_content(&inner)?))),
                other => Err(err(format!(
                    "interp: {}::as_str on {}",
                    target,
                    other.kind()
                ))),
            }
        }
        "chars" => {
            if !args.is_empty() {
                return Err(err(format!("interp: {}::chars expects 0 args", target)));
            }
            let outer = boxlike_slot(target, &receiver)?;
            let inner = outer.borrow().clone();
            match inner {
                Val::String(_) | Val::Str(_) => Ok(iter_value(
                    string_content(&inner)?.chars().map(Val::Char).collect(),
                )),
                other => Err(err(format!(
                    "interp: {}::chars on {}",
                    target,
                    other.kind()
                ))),
            }
        }
        "get" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: {}::get expects 1 arg, got {}",
                    target,
                    args.len()
                )));
            }
            let i = args[0]
                .as_i64(&format!("{}::get index", target))
                .map_err(Signal::Error)?;
            let outer = boxlike_slot(target, &receiver)?;
            let inner = outer.borrow().clone();
            match inner {
                Val::Vec(items) => {
                    if i < 0 {
                        return Ok(option_none());
                    }
                    match items.borrow().get(i as usize).cloned() {
                        Some(v) => Ok(option_some(Val::Ref {
                            slot: Rc::new(RefCell::new(v)),
                            mutable: false,
                        })),
                        None => Ok(option_none()),
                    }
                }
                Val::String(_) | Val::Str(_) => {
                    if i < 0 {
                        return Ok(option_none());
                    }
                    match string_char_items(&inner).get(i as usize).cloned() {
                        Some(v) => Ok(option_some(Val::Ref {
                            slot: Rc::new(RefCell::new(v)),
                            mutable: false,
                        })),
                        None => Ok(option_none()),
                    }
                }
                other => Err(err(format!("interp: {}::get on {}", target, other.kind()))),
            }
        }
        "len" | "is_empty" | "iter" => {
            if !args.is_empty() {
                return Err(err(format!("interp: {}::{} expects 0 args", target, name)));
            }
            let outer = boxlike_slot(target, &receiver)?;
            let inner = outer.borrow().clone();
            match inner {
                Val::Vec(items) => {
                    if name == "len" {
                        let len = items.borrow().len();
                        Ok(Val::I64(len as i64))
                    } else if name == "iter" {
                        let iter_items = items
                            .borrow()
                            .iter()
                            .cloned()
                            .map(|v| Val::Ref {
                                slot: Rc::new(RefCell::new(v)),
                                mutable: false,
                            })
                            .collect();
                        Ok(iter_value(iter_items))
                    } else {
                        let len = items.borrow().len();
                        Ok(Val::Bool(len == 0))
                    }
                }
                Val::String(_) | Val::Str(_) => {
                    let items = string_char_items(&inner);
                    if name == "len" {
                        Ok(Val::I64(items.len() as i64))
                    } else if name == "iter" {
                        Ok(iter_value(
                            items
                                .into_iter()
                                .map(|v| Val::Ref {
                                    slot: Rc::new(RefCell::new(v)),
                                    mutable: false,
                                })
                                .collect(),
                        ))
                    } else {
                        Ok(Val::Bool(items.is_empty()))
                    }
                }
                other => Err(err(format!(
                    "interp: {}::{} on {}",
                    target,
                    name,
                    other.kind()
                ))),
            }
        }
        "borrow" | "borrow_mut" => {
            if !args.is_empty() {
                return Err(err(format!("interp: {}::{} expects 0 args", target, name)));
            }
            let outer = boxlike_slot(target, &receiver)?;
            let inner = outer.borrow().clone();
            match inner {
                Val::RefCellVal(slot) => Ok(Val::Ref {
                    slot,
                    mutable: name == "borrow_mut",
                }),
                other => Err(err(format!(
                    "interp: {}::{} on {}",
                    target,
                    name,
                    other.kind()
                ))),
            }
        }
        "as_ref" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: {}::as_ref expected 0 args, got {}",
                    target,
                    args.len()
                )));
            }
            Ok(Val::Ref {
                slot: boxlike_slot(target, &receiver)?,
                mutable: false,
            })
        }
        other => Err(err(format!(
            "interp: unsupported {} method {}",
            target, other
        ))),
    }
}

fn call_string_method(target: &str, name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: {}::{} needs a receiver", target, name)))?;
    match name {
        "push_str" if target == "String" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: String::push_str expected 1 arg, got {}",
                    args.len()
                )));
            }
            let suffix = string_content(&args[0])?;
            match string_handle(receiver)? {
                Some(s) => {
                    s.borrow_mut().push_str(&suffix);
                    Ok(Val::Unit)
                }
                None => Err(err("interp: push_str on non-owned string")),
            }
        }
        "push" if target == "String" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: String::push expected 1 arg, got {}",
                    args.len()
                )));
            }
            let ch = match &args[0] {
                Val::Char(ch) => *ch,
                other => {
                    return Err(err(format!(
                        "interp: String::push expected char, got {}",
                        other.kind()
                    )))
                }
            };
            match string_handle(receiver)? {
                Some(s) => {
                    s.borrow_mut().push(ch);
                    Ok(Val::Unit)
                }
                None => Err(err("interp: push on non-owned string")),
            }
        }
        "len" => {
            if !args.is_empty() {
                return Err(err("interp: string len expects 0 args"));
            }
            Ok(Val::I64(string_content(&receiver)?.len() as i64))
        }
        "is_empty" => {
            if !args.is_empty() {
                return Err(err("interp: string is_empty expects 0 args"));
            }
            Ok(Val::Bool(string_content(&receiver)?.is_empty()))
        }
        "as_str" if target == "String" => {
            if !args.is_empty() {
                return Err(err("interp: String::as_str expects 0 args"));
            }
            Ok(Val::Str(Rc::new(string_content(&receiver)?)))
        }
        "trim" => {
            if !args.is_empty() {
                return Err(err("interp: string trim expects 0 args"));
            }
            Ok(Val::Str(Rc::new(
                string_content(&receiver)?.trim().to_string(),
            )))
        }
        "repeat" => {
            if args.len() != 1 {
                return Err(err("interp: str::repeat expects 1 arg"));
            }
            let n = args[0].as_i64("repeat").map_err(Signal::Error)? as usize;
            let s = string_content(&receiver)?.repeat(n);
            Ok(Val::String(Rc::new(RefCell::new(s))))
        }
        "to_uppercase" => {
            if !args.is_empty() {
                return Err(err("interp: str::to_uppercase expects 0 args"));
            }
            let s = string_content(&receiver)?.to_uppercase();
            Ok(Val::String(Rc::new(RefCell::new(s))))
        }
        "to_lowercase" => {
            if !args.is_empty() {
                return Err(err("interp: str::to_lowercase expects 0 args"));
            }
            let s = string_content(&receiver)?.to_lowercase();
            Ok(Val::String(Rc::new(RefCell::new(s))))
        }
        "to_string" => {
            if !args.is_empty() {
                return Err(err("interp: string to_string expects 0 args"));
            }
            Ok(Val::String(Rc::new(RefCell::new(string_content(
                &receiver,
            )?))))
        }
        "chars" | "into_iter" => {
            if !args.is_empty() {
                return Err(err(format!("interp: string {} expects 0 args", name)));
            }
            Ok(iter_value(
                string_content(&receiver)?.chars().map(Val::Char).collect(),
            ))
        }
        "iter" => {
            if !args.is_empty() {
                return Err(err("interp: string iter expects 0 args"));
            }
            let items = string_content(&receiver)?
                .chars()
                .map(|ch| Val::Ref {
                    slot: Rc::new(RefCell::new(Val::Char(ch))),
                    mutable: false,
                })
                .collect();
            Ok(iter_value(items))
        }
        "bytes" => {
            if !args.is_empty() {
                return Err(err("interp: string bytes expects 0 args"));
            }
            Ok(iter_value(
                string_content(&receiver)?
                    .bytes()
                    .map(|b| Val::I64(b as i64))
                    .collect(),
            ))
        }
        "as_bytes" => {
            if !args.is_empty() {
                return Err(err("interp: string as_bytes expects 0 args"));
            }
            Ok(Val::Vec(Rc::new(RefCell::new(
                string_content(&receiver)?
                    .bytes()
                    .map(|b| Val::I64(b as i64))
                    .collect(),
            ))))
        }
        "split" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: string split expected 1 arg, got {}",
                    args.len()
                )));
            }
            let text = string_content(&receiver)?;
            let sep = string_content(&args[0])?;
            Ok(iter_value(
                text.split(&sep)
                    .map(|part| Val::Str(Rc::new(part.to_string())))
                    .collect(),
            ))
        }
        "contains" | "starts_with" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: string {} expected 1 arg, got {}",
                    name,
                    args.len()
                )));
            }
            let hay = string_content(&receiver)?;
            let needle = string_content(&args[0])?;
            Ok(Val::Bool(if name == "contains" {
                hay.contains(&needle)
            } else {
                hay.starts_with(&needle)
            }))
        }
        "find" => {
            if args.len() != 1 {
                return Err(err("interp: string find expects 1 arg"));
            }
            let hay = string_content(&receiver)?;
            let needle = string_content(&args[0])?;
            match hay.find(&needle) {
                Some(i) => Ok(option_some(Val::I64(i as i64))),
                None => Ok(option_none()),
            }
        }
        "split_whitespace" => {
            if !args.is_empty() {
                return Err(err("interp: str::split_whitespace expects 0 args"));
            }
            let text = string_content(&receiver)?;
            let mut parts: Vec<Val> = Vec::new();
            for p in text.split_whitespace() {
                parts.push(Val::Str(Rc::new(p.to_string())));
            }
            Ok(iter_value(parts))
        }
        "lines" => {
            if !args.is_empty() {
                return Err(err("interp: str::lines expects 0 args"));
            }
            let text = string_content(&receiver)?;
            let mut parts: Vec<Val> = Vec::new();
            for p in text.lines() {
                parts.push(Val::Str(Rc::new(p.to_string())));
            }
            Ok(iter_value(parts))
        }
        "trim_start" | "trim_end" => {
            if !args.is_empty() {
                return Err(err(format!("interp: str::{} expects 0 args", name)));
            }
            let text = string_content(&receiver)?;
            let out = if name == "trim_start" {
                text.trim_start()
            } else {
                text.trim_end()
            };
            Ok(Val::Str(Rc::new(out.to_string())))
        }
        "replace" => {
            if args.len() != 2 {
                return Err(err("interp: str::replace expects 2 args"));
            }
            let text = string_content(&receiver)?;
            let from = string_content(&args[0])?;
            let to = string_content(&args[1])?;
            Ok(Val::String(Rc::new(RefCell::new(text.replace(&from, &to)))))
        }
        "splitn" => {
            if args.len() != 2 {
                return Err(err("interp: str::splitn expects 2 args"));
            }
            let n = args[0].as_i64("splitn").map_err(Signal::Error)? as usize;
            let text = string_content(&receiver)?;
            let sep = string_content(&args[1])?;
            let mut parts: Vec<Val> = Vec::new();
            for p in text.splitn(n, &sep) {
                parts.push(Val::Str(Rc::new(p.to_string())));
            }
            Ok(iter_value(parts))
        }
        "strip_prefix" | "strip_suffix" => {
            if args.len() != 1 {
                return Err(err(format!("interp: str::{} expects 1 arg", name)));
            }
            let text = string_content(&receiver)?;
            let fix = string_content(&args[0])?;
            let out = if name == "strip_prefix" {
                text.strip_prefix(&fix)
            } else {
                text.strip_suffix(&fix)
            };
            match out {
                Some(s) => Ok(option_some(Val::Str(Rc::new(s.to_string())))),
                None => Ok(option_none()),
            }
        }
        "split_once" => {
            if args.len() != 1 {
                return Err(err("interp: str::split_once expects 1 arg"));
            }
            let text = string_content(&receiver)?;
            let sep = string_content(&args[0])?;
            match text.split_once(&sep) {
                Some((a, b)) => Ok(option_some(Val::Tuple(Rc::new(vec![
                    Val::Str(Rc::new(a.to_string())),
                    Val::Str(Rc::new(b.to_string())),
                ])))),
                None => Ok(option_none()),
            }
        }
        "char_indices" => {
            if !args.is_empty() {
                return Err(err("interp: str::char_indices expects 0 args"));
            }
            let text = string_content(&receiver)?;
            let mut parts: Vec<Val> = Vec::new();
            let mut off: usize = 0;
            for ch in text.chars() {
                parts.push(Val::Tuple(Rc::new(vec![Val::I64(off as i64), Val::Char(ch)])));
                off += ch.len_utf8();
            }
            Ok(iter_value(parts))
        }
        other => Err(err(format!(
            "interp: unsupported {} method {}",
            target, other
        ))),
    }
}

fn call_char_method(name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: char::{} needs a receiver", name)))?;
    if !args.is_empty() {
        return Err(err(format!("interp: char::{} expects 0 args", name)));
    }
    let ch = match deref_value(receiver) {
        Val::Char(ch) => ch,
        other => return Err(err(format!("interp: char method on {}", other.kind()))),
    };
    match name {
        "is_whitespace" => Ok(Val::Bool(ch.is_whitespace())),
        "is_ascii_digit" => Ok(Val::Bool(ch.is_ascii_digit())),
        "is_ascii_hexdigit" => Ok(Val::Bool(ch.is_ascii_hexdigit())),
        "is_ascii_alphabetic" => Ok(Val::Bool(ch.is_ascii_alphabetic())),
        "is_ascii_alphanumeric" => Ok(Val::Bool(ch.is_ascii_alphanumeric())),
        "is_alphabetic" => Ok(Val::Bool(ch.is_alphabetic())),
        "is_numeric" => Ok(Val::Bool(ch.is_numeric())),
        "is_alphanumeric" => Ok(Val::Bool(ch.is_alphanumeric())),
        "is_uppercase" => Ok(Val::Bool(ch.is_uppercase())),
        "is_lowercase" => Ok(Val::Bool(ch.is_lowercase())),
        "to_ascii_uppercase" => Ok(Val::Char(ch.to_ascii_uppercase())),
        "to_ascii_lowercase" => Ok(Val::Char(ch.to_ascii_lowercase())),
        "len_utf8" => Ok(Val::I64(ch.len_utf8() as i64)),
        "to_string" => Ok(Val::String(Rc::new(RefCell::new(ch.to_string())))),
        other => Err(err(format!("interp: unsupported char method {}", other))),
    }
}

fn call_path_method(target: &str, name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: {}::{} needs a receiver", target, name)))?;
    match name {
        "join" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: {}::join expected 1 arg, got {}",
                    target,
                    args.len()
                )));
            }
            let base = path_content(&receiver)?;
            let child = path_content(&args[0])?;
            let joined = if base.is_empty() {
                child
            } else if child.is_empty() {
                base
            } else {
                format!("{}/{}", base, child)
            };
            Ok(Val::PathBuf(Rc::new(joined)))
        }
        "display" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: {}::display expected 0 args, got {}",
                    target,
                    args.len()
                )));
            }
            Ok(Val::String(Rc::new(RefCell::new(path_content(&receiver)?))))
        }
        "exists" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: {}::exists expected 0 args, got {}",
                    target,
                    args.len()
                )));
            }
            Ok(Val::Bool(
                std::path::Path::new(&path_content(&receiver)?).exists(),
            ))
        }
        other => Err(err(format!(
            "interp: unsupported {} method {}",
            target, other
        ))),
    }
}

fn call_command_method(name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: Command::{} needs a receiver", name)))?;
    let command = match deref_value(receiver) {
        Val::Command(command) => command,
        other => return Err(err(format!("interp: Command method on {}", other.kind()))),
    };
    match name {
        "arg" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Command::arg expected 1 arg, got {}",
                    args.len()
                )));
            }
            command.borrow_mut().push(path_content(&args[0])?);
            Ok(Val::Command(command))
        }
        "env" => {
            if args.len() != 2 {
                return Err(err(format!(
                    "interp: Command::env expected 2 args, got {}",
                    args.len()
                )));
            }
            let key = string_content(&args[0])?;
            let value = string_content(&args[1])?;
            command.borrow_mut().push(format!("env:{}={}", key, value));
            Ok(Val::Command(command))
        }
        "env_clear" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: Command::env_clear expected 0 args, got {}",
                    args.len()
                )));
            }
            command.borrow_mut().push("env_clear".to_string());
            Ok(Val::Command(command))
        }
        "output" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: Command::output expected 0 args, got {}",
                    args.len()
                )));
            }
            Ok(result_ok(Val::Struct {
                name: Rc::new("Output".to_string()),
                fields: Rc::new(vec![
                    ("status".to_string(), Val::ExitStatus(true)),
                    (
                        "stdout".to_string(),
                        Val::String(Rc::new(RefCell::new(String::new()))),
                    ),
                    (
                        "stderr".to_string(),
                        Val::String(Rc::new(RefCell::new(String::new()))),
                    ),
                ]),
            }))
        }
        other => Err(err(format!("interp: unsupported Command method {}", other))),
    }
}

fn call_exit_status_method(name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: ExitStatus::{} needs a receiver", name)))?;
    let ok = match deref_value(receiver) {
        Val::ExitStatus(ok) => ok,
        other => {
            return Err(err(format!(
                "interp: ExitStatus method on {}",
                other.kind()
            )))
        }
    };
    match name {
        "success" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: ExitStatus::success expected 0 args, got {}",
                    args.len()
                )));
            }
            Ok(Val::Bool(ok))
        }
        other => Err(err(format!(
            "interp: unsupported ExitStatus method {}",
            other
        ))),
    }
}

fn call_option_method(interp: &Interp, name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: Option::{} needs a receiver", name)))?;
    let (variant, data) = option_parts(receiver)?;
    match name {
        "unwrap" => {
            if !args.is_empty() {
                return Err(err("interp: Option::unwrap expects 0 args"));
            }
            if variant == "Some" {
                Ok(data[0].clone())
            } else {
                Err(err("interp: called unwrap on None"))
            }
        }
        "unwrap_or" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Option::unwrap_or expected 1 arg, got {}",
                    args.len()
                )));
            }
            if variant == "Some" {
                Ok(data[0].clone())
            } else {
                Ok(args[0].clone())
            }
        }
        "unwrap_or_else" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Option::unwrap_or_else expected 1 arg, got {}",
                    args.len()
                )));
            }
            if variant == "Some" {
                Ok(data[0].clone())
            } else {
                interp.call_callable(&args[0], Vec::new())
            }
        }
        "is_some" => {
            if !args.is_empty() {
                return Err(err("interp: Option::is_some expects 0 args"));
            }
            Ok(Val::Bool(variant == "Some"))
        }
        "is_none" => {
            if !args.is_empty() {
                return Err(err("interp: Option::is_none expects 0 args"));
            }
            Ok(Val::Bool(variant == "None"))
        }
        "ok_or_else" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Option::ok_or_else expected 1 arg, got {}",
                    args.len()
                )));
            }
            if variant == "Some" {
                return Ok(Val::Enum {
                    enum_name: Rc::new("Result".to_string()),
                    variant: Rc::new("Ok".to_string()),
                    data,
                });
            }
            let err_val = interp.call_callable(&args[0], Vec::new())?;
            Ok(Val::Enum {
                enum_name: Rc::new("Result".to_string()),
                variant: Rc::new("Err".to_string()),
                data: Rc::new(vec![err_val]),
            })
        }
        "map_or" => {
            if args.len() != 2 {
                return Err(err("interp: Option::map_or expects 2 args (default, f)"));
            }
            if variant == "Some" {
                interp.call_callable(&args[1], vec![data[0].clone()])
            } else {
                Ok(args[0].clone())
            }
        }
        "ok_or" => {
            if args.len() != 1 {
                return Err(err("interp: Option::ok_or expects 1 arg"));
            }
            if variant == "Some" {
                Ok(Val::Enum {
                    enum_name: Rc::new("Result".to_string()),
                    variant: Rc::new("Ok".to_string()),
                    data,
                })
            } else {
                Ok(Val::Enum {
                    enum_name: Rc::new("Result".to_string()),
                    variant: Rc::new("Err".to_string()),
                    data: Rc::new(vec![args[0].clone()]),
                })
            }
        }
        "is_some_and" => {
            if args.len() != 1 {
                return Err(err("interp: Option::is_some_and expects 1 arg"));
            }
            if variant == "Some" {
                interp.call_callable(&args[0], vec![data[0].clone()])
            } else {
                Ok(Val::Bool(false))
            }
        }
        "map" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Option::map expected 1 arg, got {}",
                    args.len()
                )));
            }
            if variant == "Some" {
                Ok(option_some(
                    interp.call_callable(&args[0], vec![data[0].clone()])?,
                ))
            } else {
                Ok(option_none())
            }
        }
        "and_then" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Option::and_then expected 1 arg, got {}",
                    args.len()
                )));
            }
            if variant == "Some" {
                interp.call_callable(&args[0], vec![data[0].clone()])
            } else {
                Ok(option_none())
            }
        }
        "or_else" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Option::or_else expected 1 arg, got {}",
                    args.len()
                )));
            }
            if variant == "Some" {
                Ok(Val::Enum {
                    enum_name: Rc::new("Option".to_string()),
                    variant: Rc::new("Some".to_string()),
                    data,
                })
            } else {
                interp.call_callable(&args[0], Vec::new())
            }
        }
        "as_ref" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: Option::as_ref expected 0 args, got {}",
                    args.len()
                )));
            }
            if variant == "Some" {
                Ok(option_some(Val::Ref {
                    slot: Rc::new(RefCell::new(data[0].clone())),
                    mutable: false,
                }))
            } else {
                Ok(option_none())
            }
        }
        "copied" | "cloned" => {
            if !args.is_empty() {
                return Err(err(format!("interp: Option::{} expects 0 args", name)));
            }
            if variant == "Some" {
                Ok(option_some(deref_value(data[0].clone())))
            } else {
                Ok(option_none())
            }
        }
        other => Err(err(format!("interp: unsupported Option method {}", other))),
    }
}

fn call_result_method(interp: &Interp, name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: Result::{} needs a receiver", name)))?;
    let (variant, data) = result_parts(receiver)?;
    match name {
        "unwrap" => {
            if !args.is_empty() {
                return Err(err("interp: Result::unwrap expects 0 args"));
            }
            if variant == "Ok" {
                Ok(data[0].clone())
            } else {
                Err(err("interp: called unwrap on Err"))
            }
        }
        "unwrap_or" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Result::unwrap_or expected 1 arg, got {}",
                    args.len()
                )));
            }
            if variant == "Ok" {
                Ok(data[0].clone())
            } else {
                Ok(args[0].clone())
            }
        }
        "is_ok" => {
            if !args.is_empty() {
                return Err(err("interp: Result::is_ok expects 0 args"));
            }
            Ok(Val::Bool(variant == "Ok"))
        }
        "is_err" => {
            if !args.is_empty() {
                return Err(err("interp: Result::is_err expects 0 args"));
            }
            Ok(Val::Bool(variant == "Err"))
        }
        "ok" => {
            if !args.is_empty() {
                return Err(err("interp: Result::ok expects 0 args"));
            }
            if variant == "Ok" {
                Ok(Val::Enum {
                    enum_name: Rc::new("Option".to_string()),
                    variant: Rc::new("Some".to_string()),
                    data,
                })
            } else {
                Ok(Val::Enum {
                    enum_name: Rc::new("Option".to_string()),
                    variant: Rc::new("None".to_string()),
                    data: Rc::new(Vec::new()),
                })
            }
        }
        "map_err" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Result::map_err expected 1 arg, got {}",
                    args.len()
                )));
            }
            if variant == "Ok" {
                return Ok(Val::Enum {
                    enum_name: Rc::new("Result".to_string()),
                    variant: Rc::new("Ok".to_string()),
                    data,
                });
            }
            let err_val = data.get(0).cloned().unwrap_or(Val::Unit);
            let mapped = interp.call_callable(&args[0], vec![err_val])?;
            Ok(Val::Enum {
                enum_name: Rc::new("Result".to_string()),
                variant: Rc::new("Err".to_string()),
                data: Rc::new(vec![mapped]),
            })
        }
        "map" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Result::map expected 1 arg, got {}",
                    args.len()
                )));
            }
            if variant == "Err" {
                return Ok(Val::Enum {
                    enum_name: Rc::new("Result".to_string()),
                    variant: Rc::new("Err".to_string()),
                    data,
                });
            }
            let ok_val = data.get(0).cloned().unwrap_or(Val::Unit);
            let mapped = interp.call_callable(&args[0], vec![ok_val])?;
            Ok(Val::Enum {
                enum_name: Rc::new("Result".to_string()),
                variant: Rc::new("Ok".to_string()),
                data: Rc::new(vec![mapped]),
            })
        }
        other => Err(err(format!("interp: unsupported Result method {}", other))),
    }
}

fn option_parts(v: Val) -> Result<(String, Rc<Vec<Val>>), Signal> {
    match v {
        Val::Enum {
            enum_name,
            variant,
            data,
        } if enum_name.as_str() == "Option" => Ok((variant.as_str().to_string(), data)),
        Val::Ref { slot, .. } => option_parts(slot.borrow().clone()),
        Val::VecElemRef { vec, index, .. } => option_parts(vec_elem_value(&vec, index)?),
        other => Err(err(format!("interp: Option method on {}", other.kind()))),
    }
}

fn result_parts(v: Val) -> Result<(String, Rc<Vec<Val>>), Signal> {
    match v {
        Val::Enum {
            enum_name,
            variant,
            data,
        } if enum_name.as_str() == "Result" => Ok((variant.as_str().to_string(), data)),
        Val::Ref { slot, .. } => result_parts(slot.borrow().clone()),
        Val::VecElemRef { vec, index, .. } => result_parts(vec_elem_value(&vec, index)?),
        other => Err(err(format!("interp: Result method on {}", other.kind()))),
    }
}

fn string_handle(v: Val) -> Result<Option<Rc<RefCell<String>>>, Signal> {
    match v {
        Val::String(s) => Ok(Some(s)),
        Val::Ref { slot, .. } => match slot.borrow().clone() {
            Val::String(s) => Ok(Some(s)),
            Val::Str(_) => Ok(None),
            other => Err(err(format!("interp: string method on {}", other.kind()))),
        },
        Val::VecElemRef { vec, index, .. } => string_handle(vec_elem_value(&vec, index)?),
        Val::Str(_) => Ok(None),
        other => Err(err(format!("interp: string method on {}", other.kind()))),
    }
}

fn write_string_handle(v: Val) -> Result<Rc<RefCell<String>>, Signal> {
    match v {
        Val::String(s) => Ok(s),
        Val::Ref {
            slot,
            mutable: true,
        } => match slot.borrow().clone() {
            Val::String(s) => Ok(s),
            other => Err(err(format!(
                "interp: write!/writeln! target must be String, got {}",
                other.kind()
            ))),
        },
        Val::VecElemRef {
            vec,
            index,
            mutable: true,
        } => match vec_elem_value(&vec, index)? {
            Val::String(s) => Ok(s),
            other => Err(err(format!(
                "interp: write!/writeln! target must be String, got {}",
                other.kind()
            ))),
        },
        Val::Ref { mutable: false, .. } => {
            Err(err("interp: write!/writeln! target must be mutable"))
        }
        Val::VecElemRef { mutable: false, .. } => {
            Err(err("interp: write!/writeln! target must be mutable"))
        }
        other => Err(err(format!(
            "interp: write!/writeln! target must be String or &mut String, got {}",
            other.kind()
        ))),
    }
}

fn string_content(v: &Val) -> Result<String, Signal> {
    match v {
        Val::Str(s) => Ok((**s).clone()),
        Val::String(s) => Ok(s.borrow().clone()),
        Val::Ref { slot, .. } => string_content(&slot.borrow()),
        Val::VecElemRef { vec, index, .. } => string_content(&vec_elem_value(vec, *index)?),
        other => Err(err(format!(
            "interp: expected string, got {}",
            other.kind()
        ))),
    }
}

fn path_content(v: &Val) -> Result<String, Signal> {
    match v {
        Val::PathBuf(s) => Ok((**s).clone()),
        Val::Ref { slot, .. } => path_content(&slot.borrow()),
        _ => string_content(v),
    }
}

fn value_eq(l: &Val, r: &Val) -> bool {
    match (string_content(l), string_content(r)) {
        (Ok(a), Ok(b)) => a == b,
        _ => {
            let l = deref_value(l.clone());
            let r = deref_value(r.clone());
            match (&l, &r) {
                (Val::Tuple(a), Val::Tuple(b)) => {
                    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| value_eq(x, y))
                }
                (Val::Vec(a), Val::Vec(b)) => {
                    let a = a.borrow();
                    let b = b.borrow();
                    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| value_eq(x, y))
                }
                (
                    Val::Struct {
                        name: an,
                        fields: af,
                    },
                    Val::Struct {
                        name: bn,
                        fields: bf,
                    },
                ) => {
                    an == bn
                        && af.len() == bf.len()
                        && af
                            .iter()
                            .zip(bf.iter())
                            .all(|((ak, av), (bk, bv))| ak == bk && value_eq(av, bv))
                }
                (
                    Val::Enum {
                        enum_name: an,
                        variant: av,
                        data: ad,
                    },
                    Val::Enum {
                        enum_name: bn,
                        variant: bv,
                        data: bd,
                    },
                ) => {
                    an == bn
                        && av == bv
                        && ad.len() == bd.len()
                        && ad.iter().zip(bd.iter()).all(|(x, y)| value_eq(x, y))
                }
                _ => l == r,
            }
        }
    }
}

/// Build a `std::cmp::Ordering` value from a comparison sign (<0/0/>0).
fn ordering_value(sign: i64) -> Val {
    let variant = if sign < 0 {
        "Less"
    } else if sign > 0 {
        "Greater"
    } else {
        "Equal"
    };
    Val::Enum {
        enum_name: Rc::new("Ordering".to_string()),
        variant: Rc::new(variant.to_string()),
        data: Rc::new(Vec::new()),
    }
}

/// True iff `v` is `Ordering::Greater` (used by Vec::sort_by).
fn ordering_is_greater(v: &Val) -> bool {
    match deref_value(v.clone()) {
        Val::Enum { variant, .. } => variant.as_str() == "Greater",
        _ => false,
    }
}

/// Byte-order comparison of two string slices (<0, 0, >0). The self-host subset
/// has no ordered `<` on `&str`/`String`, so we compare via `as_bytes`.
fn str_cmp(a: &str, b: &str) -> i64 {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let mut i = 0;
    while i < ab.len() && i < bb.len() {
        let x = ab[i] as i64;
        let y = bb[i] as i64;
        if x != y {
            return x - y;
        }
        i += 1;
    }
    (ab.len() as i64) - (bb.len() as i64)
}

/// Ordering counterpart of `value_eq` (returns <0, 0, >0). Mirrors derived
/// `Ord`: numbers/chars/bools numerically, strings by byte order, tuples and
/// structs field-by-field in declaration order, Vec lexicographically then by
/// length. Kept in the self-host subset (as_bytes + while loops, no string `<`
/// or `.cmp()`). Used by `Vec::sort` so structs/tuples sort correctly.
fn value_cmp(l: &Val, r: &Val) -> i64 {
    match (string_content(l), string_content(r)) {
        (Ok(a), Ok(b)) => str_cmp(&a, &b),
        _ => {
            let l = deref_value(l.clone());
            let r = deref_value(r.clone());
            match (&l, &r) {
                (Val::I64(a), Val::I64(b)) => {
                    if *a < *b {
                        -1
                    } else if *a > *b {
                        1
                    } else {
                        0
                    }
                }
                (Val::Char(a), Val::Char(b)) => {
                    let x = *a as i64;
                    let y = *b as i64;
                    if x < y {
                        -1
                    } else if x > y {
                        1
                    } else {
                        0
                    }
                }
                (Val::Bool(a), Val::Bool(b)) => {
                    let x = if *a { 1 } else { 0 };
                    let y = if *b { 1 } else { 0 };
                    x - y
                }
                (Val::F64(a), Val::F64(b)) => {
                    if *a < *b {
                        -1
                    } else if *a > *b {
                        1
                    } else {
                        0
                    }
                }
                (Val::Tuple(a), Val::Tuple(b)) => {
                    let mut i = 0;
                    while i < a.len() && i < b.len() {
                        let c = value_cmp(&a[i], &b[i]);
                        if c != 0 {
                            return c;
                        }
                        i += 1;
                    }
                    (a.len() as i64) - (b.len() as i64)
                }
                (Val::Vec(a), Val::Vec(b)) => {
                    let a = a.borrow();
                    let b = b.borrow();
                    let mut i = 0;
                    while i < a.len() && i < b.len() {
                        let c = value_cmp(&a[i], &b[i]);
                        if c != 0 {
                            return c;
                        }
                        i += 1;
                    }
                    (a.len() as i64) - (b.len() as i64)
                }
                (Val::Struct { fields: af, .. }, Val::Struct { fields: bf, .. }) => {
                    let mut i = 0;
                    while i < af.len() && i < bf.len() {
                        let c = value_cmp(&af[i].1, &bf[i].1);
                        if c != 0 {
                            return c;
                        }
                        i += 1;
                    }
                    (af.len() as i64) - (bf.len() as i64)
                }
                (
                    Val::Enum {
                        variant: av,
                        data: ad,
                        ..
                    },
                    Val::Enum {
                        variant: bv,
                        data: bd,
                        ..
                    },
                ) => {
                    // Variant declaration order isn't carried on the value, so
                    // fall back to the variant name (byte order) then payload.
                    let c = str_cmp(&av.to_string(), &bv.to_string());
                    if c != 0 {
                        return c;
                    }
                    let mut i = 0;
                    while i < ad.len() && i < bd.len() {
                        let c = value_cmp(&ad[i], &bd[i]);
                        if c != 0 {
                            return c;
                        }
                        i += 1;
                    }
                    (ad.len() as i64) - (bd.len() as i64)
                }
                _ => 0,
            }
        }
    }
}

fn call_vec_method(name: &str, receiver: Option<Val>, args: Vec<Val>) -> R {
    let receiver =
        receiver.ok_or_else(|| err(format!("interp: Vec::{} needs a receiver", name)))?;
    let vec = vec_handle(receiver)?;
    match name {
        "push" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Vec::push expected 1 arg, got {}",
                    args.len()
                )));
            }
            vec.borrow_mut().push(args[0].clone());
            Ok(Val::Unit)
        }
        "len" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::len expects 0 args"));
            }
            Ok(Val::I64(vec.borrow().len() as i64))
        }
        "is_empty" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::is_empty expects 0 args"));
            }
            Ok(Val::Bool(vec.borrow().is_empty()))
        }
        "clear" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::clear expects 0 args"));
            }
            vec.borrow_mut().clear();
            Ok(Val::Unit)
        }
        "contains" => {
            if args.len() != 1 {
                return Err(err("interp: Vec::contains expects 1 arg"));
            }
            let needle = deref_value(args[0].clone());
            let v = vec.borrow();
            let n = v.len();
            let mut i = 0;
            let mut found = false;
            while i < n {
                if value_eq(&deref_value(v[i].clone()), &needle) {
                    found = true;
                }
                i += 1;
            }
            Ok(Val::Bool(found))
        }
        "reverse" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::reverse expects 0 args"));
            }
            vec.borrow_mut().reverse();
            Ok(Val::Unit)
        }
        "sort" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::sort expects 0 args"));
            }
            // Copy out (avoid holding the borrow), insertion-sort a local Vec by
            // the i64 key using local index assignment, then write back. The
            // interpreter has no Ord on Val and no Vec::swap, so this stays
            // inside the self-host subset.
            let mut items: Vec<Val> = Vec::new();
            {
                let b = vec.borrow();
                let n = b.len();
                let mut k = 0;
                while k < n {
                    match b.get(k) {
                        Some(v) => items.push(v.clone()),
                        None => {}
                    }
                    k += 1;
                }
            }
            let n = items.len();
            let mut i = 1;
            while i < n {
                let key = items[i].clone();
                let mut j = i;
                while j > 0 {
                    // value_cmp mirrors derived Ord: sorts i64 numerically but
                    // also structs/tuples/strings/Vec correctly (the old
                    // as_i64-only compare silently no-op'd on non-i64 elements).
                    if value_cmp(&items[j - 1], &key) > 0 {
                        items[j] = items[j - 1].clone();
                        j -= 1;
                    } else {
                        break;
                    }
                }
                items[j] = key;
                i += 1;
            }
            {
                let mut b = vec.borrow_mut();
                b.clear();
                let mut k = 0;
                while k < items.len() {
                    b.push(items[k].clone());
                    k += 1;
                }
            }
            Ok(Val::Unit)
        }
        "dedup" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::dedup expects 0 args"));
            }
            let mut items: Vec<Val> = Vec::new();
            {
                let b = vec.borrow();
                let n = b.len();
                let mut k = 0;
                while k < n {
                    match b.get(k) {
                        Some(v) => items.push(v.clone()),
                        None => {}
                    }
                    k += 1;
                }
            }
            let mut result: Vec<Val> = Vec::new();
            let mut k = 0;
            while k < items.len() {
                let keep = if k == 0 {
                    true
                } else {
                    !value_eq(&deref_value(items[k].clone()), &deref_value(items[k - 1].clone()))
                };
                if keep {
                    result.push(items[k].clone());
                }
                k += 1;
            }
            {
                let mut b = vec.borrow_mut();
                b.clear();
                let mut k = 0;
                while k < result.len() {
                    b.push(result[k].clone());
                    k += 1;
                }
            }
            Ok(Val::Unit)
        }
        "chunks" => {
            if args.len() != 1 {
                return Err(err("interp: Vec::chunks expects 1 arg"));
            }
            let n = args[0].as_i64("Vec::chunks size").map_err(Signal::Error)?;
            if n <= 0 {
                return Err(err("interp: Vec::chunks needs a positive size"));
            }
            let n = n as usize;
            let items: Vec<Val> = {
                let b = vec.borrow();
                b.clone()
            };
            let mut out: Vec<Val> = Vec::new();
            let mut k = 0;
            while k < items.len() {
                let mut chunk: Vec<Val> = Vec::new();
                let mut j = k;
                while j < items.len() && j < k + n {
                    chunk.push(items[j].clone());
                    j += 1;
                }
                out.push(Val::Vec(Rc::new(RefCell::new(chunk))));
                k += n;
            }
            Ok(iter_value(out))
        }
        "truncate" => {
            if args.len() != 1 {
                return Err(err("interp: Vec::truncate expects 1 arg"));
            }
            let n = args[0]
                .as_i64("Vec::truncate len")
                .map_err(Signal::Error)? as usize;
            let mut items: Vec<Val> = Vec::new();
            {
                let b = vec.borrow();
                let len = b.len();
                let mut k = 0;
                while k < len && k < n {
                    match b.get(k) {
                        Some(v) => items.push(v.clone()),
                        None => {}
                    }
                    k += 1;
                }
            }
            {
                let mut b = vec.borrow_mut();
                b.clear();
                let mut k = 0;
                while k < items.len() {
                    b.push(items[k].clone());
                    k += 1;
                }
            }
            Ok(Val::Unit)
        }
        "insert" => {
            if args.len() != 2 {
                return Err(err("interp: Vec::insert expects 2 args"));
            }
            let idx = args[0].as_i64("Vec::insert index").map_err(Signal::Error)? as usize;
            let val = args[1].clone();
            let mut items: Vec<Val> = Vec::new();
            {
                let b = vec.borrow();
                let n = b.len();
                let mut k = 0;
                while k < n {
                    match b.get(k) {
                        Some(v) => items.push(v.clone()),
                        None => {}
                    }
                    k += 1;
                }
            }
            let mut result: Vec<Val> = Vec::new();
            let mut k = 0;
            while k < items.len() {
                if k == idx {
                    result.push(val.clone());
                }
                result.push(items[k].clone());
                k += 1;
            }
            if idx >= items.len() {
                result.push(val.clone());
            }
            {
                let mut b = vec.borrow_mut();
                b.clear();
                let mut k = 0;
                while k < result.len() {
                    b.push(result[k].clone());
                    k += 1;
                }
            }
            Ok(Val::Unit)
        }
        "extend" => {
            if args.len() != 1 {
                return Err(err("interp: Vec::extend expects 1 arg"));
            }
            let other = vec_handle(args[0].clone())?;
            let mut items: Vec<Val> = Vec::new();
            {
                let b = other.borrow();
                let n = b.len();
                let mut k = 0;
                while k < n {
                    match b.get(k) {
                        Some(v) => items.push(v.clone()),
                        None => {}
                    }
                    k += 1;
                }
            }
            {
                let mut b = vec.borrow_mut();
                let mut k = 0;
                while k < items.len() {
                    b.push(items[k].clone());
                    k += 1;
                }
            }
            Ok(Val::Unit)
        }
        "pop" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::pop expects 0 args"));
            }
            match vec.borrow_mut().pop() {
                Some(v) => Ok(option_some(v)),
                None => Ok(option_none()),
            }
        }
        "remove" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Vec::remove expected 1 arg, got {}",
                    args.len()
                )));
            }
            let i = args[0].as_i64("Vec::remove index").map_err(Signal::Error)?;
            if i < 0 {
                return Err(err(format!(
                    "interp: Vec::remove index {} out of bounds",
                    i
                )));
            }
            let mut items = vec.borrow_mut();
            if i as usize >= items.len() {
                return Err(err(format!(
                    "interp: Vec::remove index {} out of bounds",
                    i
                )));
            }
            Ok(items.remove(i as usize))
        }
        "get" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Vec::get expected 1 arg, got {}",
                    args.len()
                )));
            }
            let i = args[0].as_i64("Vec::get index").map_err(Signal::Error)?;
            if i < 0 {
                return Ok(option_none());
            }
            let len = vec.borrow().len();
            if (i as usize) < len {
                Ok(option_some(Val::VecElemRef {
                    vec,
                    index: i as usize,
                    mutable: false,
                }))
            } else {
                Ok(option_none())
            }
        }
        "get_mut" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Vec::get_mut expected 1 arg, got {}",
                    args.len()
                )));
            }
            let i = args[0]
                .as_i64("Vec::get_mut index")
                .map_err(Signal::Error)?;
            if i < 0 {
                return Ok(option_none());
            }
            let len = vec.borrow().len();
            if (i as usize) < len {
                Ok(option_some(Val::VecElemRef {
                    vec,
                    index: i as usize,
                    mutable: true,
                }))
            } else {
                Ok(option_none())
            }
        }
        "first" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::first expects 0 args"));
            }
            if vec.borrow().is_empty() {
                Ok(option_none())
            } else {
                Ok(option_some(Val::VecElemRef {
                    vec,
                    index: 0,
                    mutable: false,
                }))
            }
        }
        "last" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::last expects 0 args"));
            }
            let len = vec.borrow().len();
            if len == 0 {
                Ok(option_none())
            } else {
                Ok(option_some(Val::VecElemRef {
                    vec,
                    index: len - 1,
                    mutable: false,
                }))
            }
        }
        "last_mut" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::last_mut expects 0 args"));
            }
            let len = vec.borrow().len();
            if len == 0 {
                Ok(option_none())
            } else {
                Ok(option_some(Val::VecElemRef {
                    vec,
                    index: len - 1,
                    mutable: true,
                }))
            }
        }
        "iter" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::iter expects 0 args"));
            }
            let len = vec.borrow().len();
            let items = (0..len)
                .map(|index| Val::VecElemRef {
                    vec: vec.clone(),
                    index,
                    mutable: false,
                })
                .collect();
            Ok(iter_value(items))
        }
        "iter_mut" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::iter_mut expects 0 args"));
            }
            let len = vec.borrow().len();
            let items = (0..len)
                .map(|index| Val::VecElemRef {
                    vec: vec.clone(),
                    index,
                    mutable: true,
                })
                .collect();
            Ok(iter_value(items))
        }
        "into_iter" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::into_iter expects 0 args"));
            }
            let items = vec.borrow().iter().cloned().collect();
            Ok(iter_value(items))
        }
        "join" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Vec::join expected 1 arg, got {}",
                    args.len()
                )));
            }
            let sep = string_content(&args[0])?;
            let mut parts = Vec::new();
            for item in vec.borrow().iter() {
                parts.push(string_content(item)?);
            }
            Ok(Val::String(Rc::new(RefCell::new(parts.join(&sep)))))
        }
        "collect" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::collect expects 0 args"));
            }
            Ok(Val::Vec(vec))
        }
        "to_vec" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::to_vec expects 0 args"));
            }
            Ok(Val::Vec(Rc::new(RefCell::new(
                vec.borrow().iter().cloned().collect(),
            ))))
        }
        "windows" => {
            if args.len() != 1 {
                return Err(err("interp: Vec::windows expects 1 arg"));
            }
            let n = args[0].as_i64("Vec::windows size").map_err(Signal::Error)?;
            if n <= 0 {
                return Err(err("interp: Vec::windows needs a positive size"));
            }
            let n = n as usize;
            let items: Vec<Val> = vec.borrow().clone();
            let mut out: Vec<Val> = Vec::new();
            let mut k = 0;
            while k + n <= items.len() {
                let mut w: Vec<Val> = Vec::new();
                let mut j = k;
                while j < k + n {
                    w.push(items[j].clone());
                    j += 1;
                }
                out.push(Val::Vec(Rc::new(RefCell::new(w))));
                k += 1;
            }
            Ok(iter_value(out))
        }
        "split_at" => {
            if args.len() != 1 {
                return Err(err("interp: Vec::split_at expects 1 arg"));
            }
            let mid = args[0].as_i64("Vec::split_at").map_err(Signal::Error)? as usize;
            let items: Vec<Val> = vec.borrow().clone();
            let mut left: Vec<Val> = Vec::new();
            let mut right: Vec<Val> = Vec::new();
            let mut k = 0;
            while k < items.len() {
                if k < mid {
                    left.push(items[k].clone());
                } else {
                    right.push(items[k].clone());
                }
                k += 1;
            }
            Ok(Val::Tuple(Rc::new(vec![
                Val::Vec(Rc::new(RefCell::new(left))),
                Val::Vec(Rc::new(RefCell::new(right))),
            ])))
        }
        "concat" => {
            if !args.is_empty() {
                return Err(err("interp: Vec::concat expects 0 args"));
            }
            let items: Vec<Val> = vec.borrow().clone();
            let mut out: Vec<Val> = Vec::new();
            for item in items.iter() {
                match deref_value(item.clone()) {
                    Val::Vec(inner) => {
                        for x in inner.borrow().iter() {
                            out.push(x.clone());
                        }
                    }
                    other => {
                        return Err(err(format!(
                            "interp: Vec::concat on non-Vec element {}",
                            other.kind()
                        )))
                    }
                }
            }
            Ok(Val::Vec(Rc::new(RefCell::new(out))))
        }
        "binary_search" => {
            if args.len() != 1 {
                return Err(err("interp: Vec::binary_search expects 1 arg"));
            }
            let target = deref_value(args[0].clone());
            let items: Vec<Val> = vec.borrow().clone();
            let mut lo: usize = 0;
            let mut hi: usize = items.len();
            let mut found: Option<usize> = None;
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                let c = value_cmp(&items[mid], &target);
                if c == 0 {
                    found = Some(mid);
                    break;
                } else if c < 0 {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            match found {
                Some(i) => Ok(Val::Enum {
                    enum_name: Rc::new("Result".to_string()),
                    variant: Rc::new("Ok".to_string()),
                    data: Rc::new(vec![Val::I64(i as i64)]),
                }),
                None => Ok(Val::Enum {
                    enum_name: Rc::new("Result".to_string()),
                    variant: Rc::new("Err".to_string()),
                    data: Rc::new(vec![Val::I64(lo as i64)]),
                }),
            }
        }
        "rotate_left" | "rotate_right" => {
            if args.len() != 1 {
                return Err(err(format!("interp: Vec::{} expects 1 arg", name)));
            }
            let mut items: Vec<Val> = vec.borrow().clone();
            let len = items.len();
            if len > 0 {
                let raw =
                    args[0].as_i64(&format!("Vec::{}", name)).map_err(Signal::Error)? as usize;
                let k = raw % len;
                let shift = if name == "rotate_left" { k } else { (len - k) % len };
                let mut out: Vec<Val> = Vec::new();
                let mut j = 0;
                while j < len {
                    out.push(items[(j + shift) % len].clone());
                    j += 1;
                }
                items = out;
            }
            {
                let mut b = vec.borrow_mut();
                b.clear();
                let mut k = 0;
                while k < items.len() {
                    b.push(items[k].clone());
                    k += 1;
                }
            }
            Ok(Val::Unit)
        }
        "drain" => {
            if args.len() != 1 {
                return Err(err("interp: Vec::drain expects 1 arg"));
            }
            // The range arg (`a..b`) evaluates to an Iter of the indices to remove.
            let mut indices: Vec<i64> = Vec::new();
            match deref_value(args[0].clone()) {
                Val::Iter(it) => {
                    let it = it.borrow();
                    let mut p = it.pos;
                    while p < it.items.len() {
                        let idx = it.items[p].as_i64("drain index").map_err(Signal::Error)?;
                        indices.push(idx);
                        p += 1;
                    }
                }
                _ => return Err(err("interp: Vec::drain expects a range")),
            }
            let items: Vec<Val> = vec.borrow().clone();
            let n = items.len();
            let mut removed: Vec<Val> = Vec::new();
            let mut remaining: Vec<Val> = Vec::new();
            if indices.is_empty() {
                let mut k = 0;
                while k < n {
                    remaining.push(items[k].clone());
                    k += 1;
                }
            } else {
                let mut lo = indices[0];
                let mut hi = indices[0];
                let mut t = 0;
                while t < indices.len() {
                    if indices[t] < lo {
                        lo = indices[t];
                    }
                    if indices[t] > hi {
                        hi = indices[t];
                    }
                    t += 1;
                }
                let lo = lo as usize;
                let hi = (hi + 1) as usize;
                let mut k = 0;
                while k < n {
                    if k >= lo && k < hi {
                        removed.push(items[k].clone());
                    } else {
                        remaining.push(items[k].clone());
                    }
                    k += 1;
                }
            }
            {
                let mut b = vec.borrow_mut();
                b.clear();
                let mut k = 0;
                while k < remaining.len() {
                    b.push(remaining[k].clone());
                    k += 1;
                }
            }
            Ok(iter_value(removed))
        }
        other => Err(err(format!("interp: unsupported Vec method {}", other))),
    }
}

fn iter_value(items: Vec<Val>) -> Val {
    Val::Iter(Rc::new(RefCell::new(IterState { items, pos: 0 })))
}

fn range_items(start: i64, end: i64, inclusive: bool) -> Vec<Val> {
    let last = if inclusive { end } else { end - 1 };
    let mut out = Vec::new();
    let mut i = start;
    while i <= last {
        out.push(Val::I64(i));
        i += 1;
    }
    out
}

fn vec_index_value(vs: &Rc<RefCell<Vec<Val>>>, index: usize) -> R {
    vs.borrow()
        .get(index)
        .cloned()
        .ok_or_else(|| err(format!("interp: index {} out of bounds", index)))
}

fn call_iter_method(
    interp: &Interp,
    name: &str,
    receiver: Option<Val>,
    type_args: &[Type],
    args: Vec<Val>,
) -> R {
    let iter =
        match receiver.ok_or_else(|| err(format!("interp: Iter::{} needs a receiver", name)))? {
            Val::Iter(iter) => iter,
            other => return Err(err(format!("interp: Iter method on {}", other.kind()))),
        };
    if name != "collect" && !type_args.is_empty() {
        return Err(err(format!(
            "interp: Iter::{} does not support turbofish args",
            name
        )));
    }
    match name {
        "next" => {
            if !args.is_empty() {
                return Err(err("interp: Iter::next expects 0 args"));
            }
            let mut iter = iter.borrow_mut();
            if iter.pos >= iter.items.len() {
                return Ok(option_none());
            }
            let item = iter.items[iter.pos].clone();
            iter.pos += 1;
            Ok(option_some(item))
        }
        "nth" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Iter::nth expects 1 arg, got {}",
                    args.len()
                )));
            }
            let n = args[0].as_i64("Iter::nth index").map_err(Signal::Error)?;
            if n < 0 {
                return Err(err("interp: Iter::nth index must be non-negative"));
            }
            let mut iter = iter.borrow_mut();
            let step = n as usize;
            let remaining = iter.items.len() - iter.pos;
            if step >= remaining {
                iter.pos = iter.items.len();
                return Ok(option_none());
            }
            let idx = iter.pos + step;
            let item = iter.items[idx].clone();
            iter.pos = idx + 1;
            Ok(option_some(item))
        }
        "last" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: Iter::last expects 0 args, got {}",
                    args.len()
                )));
            }
            let mut iter = iter.borrow_mut();
            let item = iter.items[iter.pos..].last().cloned();
            iter.pos = iter.items.len();
            Ok(match item {
                Some(v) => option_some(v),
                None => option_none(),
            })
        }
        "filter" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Iter::filter expects 1 arg, got {}",
                    args.len()
                )));
            }
            let mut out = Vec::new();
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            for item in items.iter() {
                let keep = interp.call_callable(
                    &args[0],
                    vec![Val::Ref {
                        slot: Rc::new(RefCell::new(item.clone())),
                        mutable: false,
                    }],
                )?;
                if keep
                    .as_bool("Iter::filter closure")
                    .map_err(Signal::Error)?
                {
                    out.push(item.clone());
                }
            }
            Ok(iter_value(out))
        }
        "find" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Iter::find expects 1 arg, got {}",
                    args.len()
                )));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            for item in items {
                let keep = interp.call_callable(
                    &args[0],
                    vec![Val::Ref {
                        slot: Rc::new(RefCell::new(item.clone())),
                        mutable: false,
                    }],
                )?;
                if keep.as_bool("Iter::find closure").map_err(Signal::Error)? {
                    return Ok(option_some(item));
                }
            }
            Ok(option_none())
        }
        "map" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Iter::map expects 1 arg, got {}",
                    args.len()
                )));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let mut out = Vec::new();
            for item in items {
                out.push(interp.call_callable(&args[0], vec![item])?);
            }
            Ok(iter_value(out))
        }
        "flat_map" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Iter::flat_map expects 1 arg, got {}",
                    args.len()
                )));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let mut out: Vec<Val> = Vec::new();
            for item in items {
                let produced = interp.call_callable(&args[0], vec![item])?;
                match deref_value(produced) {
                    Val::Vec(vs) => {
                        let b = vs.borrow();
                        let n = b.len();
                        let mut k = 0;
                        while k < n {
                            match b.get(k) {
                                Some(v) => out.push(v.clone()),
                                None => {}
                            }
                            k += 1;
                        }
                    }
                    Val::Iter(inner) => {
                        let b = inner.borrow();
                        let rest = b.items[b.pos..].to_vec();
                        for v in rest {
                            out.push(v);
                        }
                    }
                    other => {
                        return Err(err(format!(
                            "interp: Iter::flat_map closure returned {}",
                            other.kind()
                        )))
                    }
                }
            }
            Ok(iter_value(out))
        }
        "zip" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Iter::zip expects 1 arg, got {}",
                    args.len()
                )));
            }
            let right = match &args[0] {
                Val::Iter(other) => {
                    let other = other.borrow();
                    other.items[other.pos..].to_vec()
                }
                other => {
                    return Err(err(format!(
                        "interp: Iter::zip expected Iter, got {}",
                        other.kind()
                    )))
                }
            };
            let left = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let mut out = Vec::new();
            let len = left.len().min(right.len());
            for i in 0..len {
                out.push(Val::Tuple(Rc::new(vec![left[i].clone(), right[i].clone()])));
            }
            Ok(iter_value(out))
        }
        "all" | "any" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Iter::{} expects 1 arg, got {}",
                    name,
                    args.len()
                )));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            if name == "all" {
                for item in items {
                    let keep = interp
                        .call_callable(&args[0], vec![item])?
                        .as_bool("Iter::all closure")
                        .map_err(Signal::Error)?;
                    if !keep {
                        return Ok(Val::Bool(false));
                    }
                }
                Ok(Val::Bool(true))
            } else {
                for item in items {
                    let keep = interp
                        .call_callable(&args[0], vec![item])?
                        .as_bool("Iter::any closure")
                        .map_err(Signal::Error)?;
                    if keep {
                        return Ok(Val::Bool(true));
                    }
                }
                Ok(Val::Bool(false))
            }
        }
        "position" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Iter::position expects 1 arg, got {}",
                    args.len()
                )));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            for (i, item) in items.into_iter().enumerate() {
                let keep = interp
                    .call_callable(&args[0], vec![item])?
                    .as_bool("Iter::position closure")
                    .map_err(Signal::Error)?;
                if keep {
                    return Ok(option_some(Val::I64(i as i64)));
                }
            }
            Ok(option_none())
        }
        "count" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: Iter::count expects 0 args, got {}",
                    args.len()
                )));
            }
            let iter = iter.borrow();
            Ok(Val::I64((iter.items.len() - iter.pos) as i64))
        }
        "sum" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: Iter::sum expects 0 args, got {}",
                    args.len()
                )));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let mut total = 0i64;
            for item in items {
                total += deref_value(item)
                    .as_i64("Iter::sum item")
                    .map_err(Signal::Error)?;
            }
            Ok(Val::I64(total))
        }
        "product" => {
            if !args.is_empty() {
                return Err(err("interp: Iter::product expects 0 args"));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let mut total = 1i64;
            for item in items {
                total *= deref_value(item)
                    .as_i64("Iter::product item")
                    .map_err(Signal::Error)?;
            }
            Ok(Val::I64(total))
        }
        "min" | "max" => {
            if !args.is_empty() {
                return Err(err(format!("interp: Iter::{} expects 0 args", name)));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            if items.is_empty() {
                return Ok(option_none());
            }
            let mut best = deref_value(items[0].clone())
                .as_i64("Iter::min/max item")
                .map_err(Signal::Error)?;
            let mut k = 1;
            while k < items.len() {
                let cur = deref_value(items[k].clone())
                    .as_i64("Iter::min/max item")
                    .map_err(Signal::Error)?;
                if (name == "max" && cur > best) || (name == "min" && cur < best) {
                    best = cur;
                }
                k += 1;
            }
            Ok(option_some(Val::I64(best)))
        }
        "max_by_key" | "min_by_key" => {
            if args.len() != 1 {
                return Err(err(format!("interp: Iter::{} expects 1 arg", name)));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            if items.is_empty() {
                return Ok(option_none());
            }
            let mut best_item = items[0].clone();
            let mut best_key = interp
                .call_callable(&args[0], vec![items[0].clone()])?
                .as_i64("Iter::by_key key")
                .map_err(Signal::Error)?;
            let mut k = 1;
            while k < items.len() {
                let cur_key = interp
                    .call_callable(&args[0], vec![items[k].clone()])?
                    .as_i64("Iter::by_key key")
                    .map_err(Signal::Error)?;
                // rustc max_by_key returns the LAST max; min_by_key the FIRST min.
                let better = if name == "max_by_key" {
                    cur_key >= best_key
                } else {
                    cur_key < best_key
                };
                if better {
                    best_key = cur_key;
                    best_item = items[k].clone();
                }
                k += 1;
            }
            Ok(option_some(best_item))
        }
        "fold" => {
            if args.len() != 2 {
                return Err(err(format!(
                    "interp: Iter::fold expects 2 args, got {}",
                    args.len()
                )));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let mut acc = args[0].clone();
            for item in items {
                acc = interp.call_callable(&args[1], vec![acc, item])?;
            }
            Ok(acc)
        }
        "take" | "skip" => {
            if args.len() != 1 {
                return Err(err(format!(
                    "interp: Iter::{} expects 1 arg, got {}",
                    name,
                    args.len()
                )));
            }
            let n = args[0].as_i64("Iter count").map_err(Signal::Error)?;
            let n = if n < 0 { 0usize } else { n as usize };
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let out = if name == "take" {
                items.into_iter().take(n).collect()
            } else {
                items.into_iter().skip(n).collect()
            };
            Ok(iter_value(out))
        }
        "chain" => {
            if args.len() != 1 {
                return Err(err("interp: Iter::chain expects 1 arg"));
            }
            let other = match args[0].clone() {
                Val::Iter(o) => o,
                other => return Err(err(format!("interp: Iter::chain on {}", other.kind()))),
            };
            let mut out = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            {
                let o = other.borrow();
                let rest = o.items[o.pos..].to_vec();
                for v in rest {
                    out.push(v);
                }
            }
            Ok(iter_value(out))
        }
        "step_by" => {
            if args.len() != 1 {
                return Err(err("interp: Iter::step_by expects 1 arg"));
            }
            let n = args[0].as_i64("Iter::step_by").map_err(Signal::Error)?;
            if n <= 0 {
                return Err(err("interp: Iter::step_by needs a positive step"));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let mut out: Vec<Val> = Vec::new();
            let mut k = 0;
            while k < items.len() {
                out.push(items[k].clone());
                k += n as usize;
            }
            Ok(iter_value(out))
        }
        "rposition" => {
            if args.len() != 1 {
                return Err(err("interp: Iter::rposition expects 1 arg"));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let n = items.len();
            let mut k = n;
            while k > 0 {
                k -= 1;
                let hit = interp
                    .call_callable(&args[0], vec![items[k].clone()])?
                    .as_bool("Iter::rposition closure")
                    .map_err(Signal::Error)?;
                if hit {
                    return Ok(option_some(Val::I64(k as i64)));
                }
            }
            Ok(option_none())
        }
        "copied" | "cloned" => {
            if !args.is_empty() {
                return Err(err(format!("interp: Iter::{} expects 0 args", name)));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..]
                    .iter()
                    .cloned()
                    .map(deref_value)
                    .collect()
            };
            Ok(iter_value(items))
        }
        "rev" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: Iter::rev expects 0 args, got {}",
                    args.len()
                )));
            }
            let mut items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            items.reverse();
            Ok(iter_value(items))
        }
        "enumerate" => {
            if !args.is_empty() {
                return Err(err(format!(
                    "interp: Iter::enumerate expects 0 args, got {}",
                    args.len()
                )));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let mut out = Vec::new();
            for (i, item) in items.into_iter().enumerate() {
                out.push(Val::Tuple(Rc::new(vec![Val::I64(i as i64), item])));
            }
            Ok(iter_value(out))
        }
        "collect" => {
            if !args.is_empty() {
                return Err(err("interp: Iter::collect expects 0 args"));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            if type_args.len() == 1 {
                return match &type_args[0] {
                    Type::Named(name) if name == "String" => {
                        let mut s = String::new();
                        for item in items.into_iter() {
                            match deref_value(item) {
                                Val::Char(ch) => s.push(ch),
                                other => {
                                    return Err(err(format!(
                                        "interp: Iter::collect::<String> item is {}",
                                        other.kind()
                                    )))
                                }
                            }
                        }
                        Ok(Val::String(Rc::new(RefCell::new(s))))
                    }
                    Type::Generic { name, args } if name == "Vec" && args.len() == 1 => {
                        Ok(Val::Vec(Rc::new(RefCell::new(items))))
                    }
                    other => Err(err(format!(
                        "interp: Iter::collect turbofish unsupported for {:?}",
                        other
                    ))),
                };
            }
            if !type_args.is_empty() {
                return Err(err(format!(
                    "interp: Iter::collect expects 0 or 1 turbofish args, got {}",
                    type_args.len()
                )));
            }
            let mut s = String::new();
            let mut all_chars = true;
            for item in items.iter() {
                match deref_value(item.clone()) {
                    Val::Char(ch) => s.push(ch),
                    _ => {
                        all_chars = false;
                        break;
                    }
                }
            }
            // An empty iterator collects to an empty Vec by default (a String is
            // only inferred when there is at least one char element).
            if all_chars && !items.is_empty() {
                Ok(Val::String(Rc::new(RefCell::new(s))))
            } else {
                Ok(Val::Vec(Rc::new(RefCell::new(items))))
            }
        }
        "take_while" => {
            if args.len() != 1 {
                return Err(err("interp: Iter::take_while expects 1 arg"));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let mut out: Vec<Val> = Vec::new();
            for item in items {
                let keep = interp
                    .call_callable(&args[0], vec![item.clone()])?
                    .as_bool("take_while predicate")
                    .map_err(Signal::Error)?;
                if keep {
                    out.push(item);
                } else {
                    break;
                }
            }
            Ok(iter_value(out))
        }
        "skip_while" => {
            if args.len() != 1 {
                return Err(err("interp: Iter::skip_while expects 1 arg"));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            let mut out: Vec<Val> = Vec::new();
            let mut skipping = true;
            for item in items {
                if skipping {
                    let skip = interp
                        .call_callable(&args[0], vec![item.clone()])?
                        .as_bool("skip_while predicate")
                        .map_err(Signal::Error)?;
                    if skip {
                        continue;
                    }
                    skipping = false;
                }
                out.push(item);
            }
            Ok(iter_value(out))
        }
        "find_map" => {
            if args.len() != 1 {
                return Err(err("interp: Iter::find_map expects 1 arg"));
            }
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            for item in items {
                let mapped = interp.call_callable(&args[0], vec![item])?;
                let (variant, data) = option_parts(mapped)?;
                if variant == "Some" {
                    return Ok(option_some(data[0].clone()));
                }
            }
            Ok(option_none())
        }
        "peekable" => {
            if !args.is_empty() {
                return Err(err("interp: Iter::peekable expects 0 args"));
            }
            // The interp's iterator cursor already supports look-ahead via `pos`,
            // so peekable is an identity wrapper over the remaining items.
            let items = {
                let iter = iter.borrow();
                iter.items[iter.pos..].to_vec()
            };
            Ok(iter_value(items))
        }
        "peek" => {
            if !args.is_empty() {
                return Err(err("interp: Iter::peek expects 0 args"));
            }
            // Look at the next item WITHOUT advancing pos (Option<&T> ~ Option<T>).
            let nxt = {
                let it = iter.borrow();
                it.items.get(it.pos).cloned()
            };
            match nxt {
                Some(v) => Ok(option_some(v)),
                None => Ok(option_none()),
            }
        }
        other => Err(err(format!("interp: unsupported Iter method {}", other))),
    }
}

fn vec_handle(v: Val) -> Result<Rc<RefCell<Vec<Val>>>, Signal> {
    match v {
        Val::Vec(vs) => Ok(vs),
        Val::Ref { slot, .. } => match slot.borrow().clone() {
            Val::Vec(vs) => Ok(vs),
            other => Err(err(format!("interp: Vec method on {}", other.kind()))),
        },
        Val::VecElemRef { vec, index, .. } => vec_handle(vec_elem_value(&vec, index)?),
        other => Err(err(format!("interp: Vec method on {}", other.kind()))),
    }
}

fn vec_elem_value(vec: &Rc<RefCell<Vec<Val>>>, index: usize) -> Result<Val, Signal> {
    vec.borrow()
        .get(index)
        .cloned()
        .ok_or_else(|| err("interp: dangling Vec element ref"))
}

fn slice_value(
    v: Val,
    start: Option<i64>,
    end: Option<i64>,
    inclusive: bool,
) -> Result<Val, Signal> {
    let vec = vec_handle(v)?;
    let vec = vec.borrow();
    let len = vec.len() as i64;
    let lo = start.unwrap_or(0);
    let hi = match end {
        Some(n) if inclusive => n + 1,
        Some(n) => n,
        None => len,
    };
    if lo < 0 || hi < lo || hi > len {
        return Err(err(format!(
            "interp: slice bounds {}..{} out of range",
            lo, hi
        )));
    }
    Ok(Val::Vec(Rc::new(RefCell::new(
        vec[lo as usize..hi as usize].to_vec(),
    ))))
}

fn foreach_items(v: Val) -> Result<Vec<Val>, Signal> {
    match v {
        Val::Vec(vs) => Ok(vs.borrow().clone()),
        Val::Iter(iter) => {
            let iter = iter.borrow();
            Ok(iter.items[iter.pos..].to_vec())
        }
        Val::String(s) => Ok(s.borrow().chars().map(Val::Char).collect()),
        Val::Str(s) => Ok(s.chars().map(Val::Char).collect()),
        Val::Ref { slot, mutable } => match slot.borrow().clone() {
            Val::Vec(vs) if !mutable => {
                let len = vs.borrow().len();
                Ok((0..len)
                    .map(|index| Val::VecElemRef {
                        vec: vs.clone(),
                        index,
                        mutable: false,
                    })
                    .collect())
            }
            Val::String(s) if !mutable => Ok(s
                .borrow()
                .chars()
                .map(|ch| Val::Ref {
                    slot: Rc::new(RefCell::new(Val::Char(ch))),
                    mutable: false,
                })
                .collect()),
            Val::Str(s) if !mutable => Ok(s
                .chars()
                .map(|ch| Val::Ref {
                    slot: Rc::new(RefCell::new(Val::Char(ch))),
                    mutable: false,
                })
                .collect()),
            Val::Vec(_) => Err(err("interp: for over &mut Vec is held")),
            Val::String(_) | Val::Str(_) => Err(err("interp: for over &mut string is held")),
            other => Err(err(format!("interp: for over ref to {}", other.kind()))),
        },
        other => Err(err(format!("interp: for over {}", other.kind()))),
    }
}

fn deref_value(v: Val) -> Val {
    match v {
        Val::Ref { slot, .. } => deref_value(slot.borrow().clone()),
        Val::VecElemRef { vec, index, .. } => vec
            .borrow()
            .get(index)
            .cloned()
            .map(deref_value)
            .unwrap_or(Val::Unit),
        other => other,
    }
}

fn clone_value(v: &Val) -> Val {
    match v {
        Val::String(s) => Val::String(Rc::new(RefCell::new(s.borrow().clone()))),
        Val::PathBuf(s) => Val::PathBuf(Rc::new((**s).clone())),
        Val::Command(args) => Val::Command(Rc::new(RefCell::new(args.borrow().clone()))),
        Val::ExitStatus(ok) => Val::ExitStatus(*ok),
        Val::Tuple(items) => Val::Tuple(Rc::new(items.iter().map(clone_value).collect())),
        Val::Vec(items) => Val::Vec(Rc::new(RefCell::new(
            items.borrow().iter().map(clone_value).collect(),
        ))),
        Val::Box(slot) => Val::Box(Rc::new(RefCell::new(clone_value(&slot.borrow())))),
        Val::RcVal(slot) => Val::RcVal(slot.clone()),
        Val::RefCellVal(slot) => {
            Val::RefCellVal(Rc::new(RefCell::new(clone_value(&slot.borrow()))))
        }
        Val::HashMap(entries) => Val::HashMap(Rc::new(RefCell::new(
            entries
                .borrow()
                .iter()
                .map(|(k, v)| {
                    (
                        clone_value(k),
                        Rc::new(RefCell::new(clone_value(&v.borrow()))),
                    )
                })
                .collect(),
        ))),
        Val::HashEntry { map, key } => Val::HashEntry {
            map: map.clone(),
            key: key.clone(),
        },
        Val::Iter(iter) => {
            let iter = iter.borrow();
            Val::Iter(Rc::new(RefCell::new(IterState {
                items: iter.items.iter().map(clone_value).collect(),
                pos: iter.pos,
            })))
        }
        Val::Struct { name, fields } => Val::Struct {
            name: name.clone(),
            fields: Rc::new(
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), clone_value(v)))
                    .collect(),
            ),
        },
        Val::Enum {
            enum_name,
            variant,
            data,
        } => Val::Enum {
            enum_name: enum_name.clone(),
            variant: variant.clone(),
            data: Rc::new(data.iter().map(clone_value).collect()),
        },
        Val::Ref { slot, .. } => clone_value(&slot.borrow()),
        Val::VecElemRef { vec, index, .. } => vec
            .borrow()
            .get(*index)
            .map(clone_value)
            .unwrap_or(Val::Unit),
        other => other.clone(),
    }
}

fn coerce_let_value(v: Val, ty: &Option<Type>) -> Val {
    match (ty, v) {
        (Some(Type::Generic { name, args }), Val::String(s))
            if name == "Vec" && args.len() == 1 && args[0] == Type::Char =>
        {
            coerce_string_to_char_vec(Val::String(s))
        }
        (Some(Type::Generic { name, args }), Val::Str(s))
            if name == "Vec" && args.len() == 1 && args[0] == Type::Char =>
        {
            coerce_string_to_char_vec(Val::Str(s))
        }
        (_, v) => v,
    }
}

fn assign_field(slot: Slot, field: &str, value: Val) -> Result<(), Signal> {
    let ref_target = {
        let current = slot.borrow();
        match &*current {
            Val::Ref {
                slot: inner,
                mutable,
            } => {
                if !*mutable {
                    return Err(err("interp: field assignment through immutable ref"));
                }
                Some(inner.clone())
            }
            _ => None,
        }
    };
    if let Some(inner) = ref_target {
        return assign_field(inner, field, value);
    }

    let current = slot.borrow().clone();
    match current {
        Val::Struct { name, fields } => {
            let items = fields.as_ref();
            let mut out = Vec::with_capacity(items.len());
            let mut found = false;
            let mut i = 0;
            while i < items.len() {
                let fname = items[i].0.clone();
                if fname == field {
                    out.push((fname, value.clone()));
                    found = true;
                } else {
                    out.push((fname, items[i].1.clone()));
                }
                i += 1;
            }
            if !found {
                return Err(err(format!("interp: no field {}", field)));
            }
            *slot.borrow_mut() = Val::Struct {
                name,
                fields: Rc::new(out),
            };
            Ok(())
        }
        other => Err(err(format!("interp: field assignment on {}", other.kind()))),
    }
}

fn assign_tuple_index(slot: Slot, index: usize, value: Val) -> Result<(), Signal> {
    let ref_target = {
        let current = slot.borrow();
        match &*current {
            Val::Ref {
                slot: inner,
                mutable,
            } => {
                if !*mutable {
                    return Err(err("interp: tuple assignment through immutable ref"));
                }
                Some(inner.clone())
            }
            _ => None,
        }
    };
    if let Some(inner) = ref_target {
        return assign_tuple_index(inner, index, value);
    }

    let current = slot.borrow().clone();
    match current {
        Val::Tuple(items) => {
            if index >= items.len() {
                return Err(err(format!("interp: tuple index {} out of range", index)));
            }
            let mut out = Vec::new();
            let mut i = 0;
            while i < items.len() {
                if i == index {
                    out.push(value.clone());
                } else {
                    out.push(items[i].clone());
                }
                i += 1;
            }
            let mut target = slot.borrow_mut();
            *target = Val::Tuple(Rc::new(out));
            Ok(())
        }
        other => Err(err(format!(
            "interp: tuple index assignment on {}",
            other.kind()
        ))),
    }
}

fn place_slot(e: &Expr, scope: &Scope) -> Result<Slot, Signal> {
    match e {
        Expr::Var(name) => scope
            .get_slot(name)
            .ok_or_else(|| err(format!("interp: cannot reference unbound {}", name))),
        Expr::Unary {
            op: UnOp::Deref,
            rhs,
        } => {
            let v = eval_place_expr(rhs, scope)?;
            match v {
                Val::Ref { slot, .. } => Ok(slot),
                other => Err(err(format!(
                    "interp: cannot reference deref of {}",
                    other.kind()
                ))),
            }
        }
        _ => Err(err(
            "interp: references currently require a variable or deref place",
        )),
    }
}

fn eval_place_expr(e: &Expr, scope: &Scope) -> Result<Val, Signal> {
    match e {
        Expr::Var(name) => scope
            .get(name)
            .ok_or_else(|| err(format!("interp: unbound variable {}", name))),
        _ => Err(err("interp: nested reference place is not supported yet")),
    }
}

fn cfg_flag(name: &str) -> bool {
    match name {
        "windows" => cfg!(windows),
        "unix" => cfg!(unix),
        "debug_assertions" => cfg!(debug_assertions),
        _ => false,
    }
}

fn lower_hex_16(n: i64) -> String {
    let mut x = n as u64;
    let mut chars = Vec::new();
    let mut i = 0;
    while i < 16 {
        let digit = (x % 16) as i64;
        chars.push(hex_digit(digit));
        x = x / 16;
        i += 1;
    }
    chars.reverse();
    chars.iter().collect()
}

/// `{:x}`/`{:X}`/`{:b}`/`{:o}` rendering: value in the given radix, no leading
/// zeros (like rustc). Non-negative is exact; negatives use the u64 two's-
/// complement bit pattern (same i64-width limitation as `lower_hex_16`).
fn to_radix(n: i64, radix: i64, upper: bool) -> String {
    let mut x = n as u64;
    if x == 0 {
        return "0".to_string();
    }
    let r = radix as u64;
    let mut chars: Vec<char> = Vec::new();
    while x > 0 {
        let digit = (x % r) as i64;
        let ch = if upper {
            hex_digit_upper(digit)
        } else {
            hex_digit(digit)
        };
        chars.push(ch);
        x = x / r;
    }
    chars.reverse();
    chars.iter().collect()
}

fn hex_digit_upper(n: i64) -> char {
    match n {
        10 => 'A',
        11 => 'B',
        12 => 'C',
        13 => 'D',
        14 => 'E',
        15 => 'F',
        _ => hex_digit(n),
    }
}

fn hex_digit(n: i64) -> char {
    match n {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        9 => '9',
        10 => 'a',
        11 => 'b',
        12 => 'c',
        13 => 'd',
        14 => 'e',
        _ => 'f',
    }
}

fn cast_value(v: Val, ty: &Type) -> Result<Val, Signal> {
    let int = |n: i64| match ty {
        Type::I64 => n,
        Type::I32 => n as i32 as i64,
        Type::U32 => n as u32 as i64,
        Type::U64 => n as u64 as i64,
        Type::U8 => n as u8 as i64,
        Type::Usize => n as usize as i64,
        _ => n,
    };
    match ty {
        Type::F64 => match v {
            Val::I64(n) => Ok(Val::F64(n as f64)),
            Val::F64(f) => Ok(Val::F64(f)),
            other => Err(err(format!("interp: cannot cast {} to f64", other.kind()))),
        },
        Type::I64 | Type::I32 | Type::U32 | Type::U64 | Type::U8 | Type::Usize => match v {
            Val::I64(n) => Ok(Val::I64(int(n))),
            Val::F64(f) => Ok(Val::I64(int(f as i64))),
            Val::Char(ch) => Ok(Val::I64(int(ch as u32 as i64))),
            Val::Bool(b) => Ok(Val::I64(int(if b { 1 } else { 0 }))),
            other => Err(err(format!(
                "interp: cannot cast {} to integer",
                other.kind()
            ))),
        },
        Type::Char => match v {
            Val::I64(n) => char::from_u32(n as u32)
                .map(Val::Char)
                .ok_or_else(|| err("interp: invalid char cast")),
            Val::Char(ch) => Ok(Val::Char(ch)),
            other => Err(err(format!("interp: cannot cast {} to char", other.kind()))),
        },
        other => Err(err(format!("interp: cast to {:?} is unsupported", other))),
    }
}

fn int2(l: &Val, r: &Val, ctx: &str) -> Result<(i64, i64), Signal> {
    // Auto-deref number references (`&i64`, vec-element refs from .iter()) so
    // `v.iter().map(|x| x * 2)` and `&n + 1` evaluate.
    Ok((
        deref_value(l.clone()).as_i64(ctx).map_err(Signal::Error)?,
        deref_value(r.clone()).as_i64(ctx).map_err(Signal::Error)?,
    ))
}

fn cmp(l: &Val, r: &Val, ctx: &str, f: impl Fn(i64, i64) -> bool) -> R {
    match (l, r) {
        (Val::F64(a), Val::F64(b)) => {
            let x = *a;
            let y = *b;
            let result = if ctx == "<" {
                x < y
            } else if ctx == "<=" {
                x <= y
            } else if ctx == ">" {
                x > y
            } else {
                x >= y
            };
            Ok(Val::Bool(result))
        }
        (Val::Char(a), Val::Char(b)) => Ok(Val::Bool(f(*a as u32 as i64, *b as u32 as i64))),
        _ => {
            let dl = deref_value(l.clone());
            let dr = deref_value(r.clone());
            match (&dl, &dr) {
                (Val::I64(a), Val::I64(b)) => Ok(Val::Bool(f(*a, *b))),
                // Derived Ord on strings/structs/tuples/Vec: order via value_cmp
                // (mirrors the typeck acceptance of `<`/`<=`/`>`/`>=` on these).
                (Val::Str(_), _)
                | (Val::String(_), _)
                | (Val::Struct { .. }, _)
                | (Val::Tuple(_), _)
                | (Val::Vec(_), _)
                | (_, Val::Str(_))
                | (_, Val::String(_))
                | (_, Val::Struct { .. })
                | (_, Val::Tuple(_))
                | (_, Val::Vec(_)) => {
                    let c = value_cmp(&dl, &dr);
                    Ok(Val::Bool(f(c, 0)))
                }
                _ => {
                    let (a, b) = int2(l, r, ctx)?;
                    Ok(Val::Bool(f(a, b)))
                }
            }
        }
    }
}
