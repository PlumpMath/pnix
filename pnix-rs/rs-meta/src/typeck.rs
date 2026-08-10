//! Light type-checker for the Rust subset.
//!
//! Purpose: make the interpreter reject programs that `rustc` would reject, so
//! the two execution paths share an honest boundary (translation validation on
//! *acceptance*, not just results). Deliberately light — lenient where divergence
//! or exhaustiveness analysis would be needed — so it never false-rejects a valid
//! program in the subset.

use crate::ast::*;
use std::collections::HashMap;

type TypeSubst = HashMap<String, Type>;

#[derive(Clone)]
struct MethodSig {
    receiver: Option<ReceiverKind>,
    params: Vec<Type>,
    ret: Type,
}

pub fn check(prog: &Program) -> Result<(), String> {
    let mut aliases: HashMap<String, Type> = HashMap::new();
    for alias in &prog.aliases {
        if aliases
            .insert(alias.name.clone(), alias.ty.clone())
            .is_some()
        {
            return Err(format!("typeck: duplicate type alias {}", alias.name));
        }
    }
    let mut sigs: HashMap<String, (Vec<Type>, Type)> = HashMap::new();
    for f in &prog.funcs {
        let params = f
            .params
            .iter()
            .map(|p| resolve_aliases(&p.ty, &aliases))
            .collect();
        if sigs
            .insert(f.name.clone(), (params, resolve_aliases(&f.ret, &aliases)))
            .is_some()
        {
            return Err(format!("typeck: duplicate function {}", f.name));
        }
    }
    let mut structs: HashMap<String, Vec<(String, Type)>> = HashMap::new();
    for s in &prog.structs {
        let fields = s
            .fields
            .iter()
            .map(|(name, ty)| (name.clone(), resolve_aliases(ty, &aliases)))
            .collect();
        if structs.insert(s.name.clone(), fields).is_some() {
            return Err(format!("typeck: duplicate struct {}", s.name));
        }
    }
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
    let mut enums: HashMap<String, HashMap<String, Vec<Type>>> = HashMap::new();
    let mut enum_named_fields: HashMap<(String, String), Vec<(String, Type)>> = HashMap::new();
    for e in &prog.enums {
        let mut variants = HashMap::new();
        for v in &e.variants {
            let payload = if v.named_fields.is_empty() {
                v.fields
                    .iter()
                    .map(|ty| resolve_aliases(ty, &aliases))
                    .collect()
            } else {
                let named: Vec<(String, Type)> = v
                    .named_fields
                    .iter()
                    .map(|(name, ty)| (name.clone(), resolve_aliases(ty, &aliases)))
                    .collect();
                enum_named_fields.insert((e.name.clone(), v.name.clone()), named.clone());
                named.into_iter().map(|(_, ty)| ty).collect()
            };
            variants.insert(v.name.clone(), payload);
        }
        if enums.insert(e.name.clone(), variants).is_some() {
            return Err(format!("typeck: duplicate enum {}", e.name));
        }
    }
    // Built-in std::cmp::Ordering (Less/Equal/Greater, all unit variants) so
    // `.cmp()` results and `match o { Ordering::Less => .. }` typecheck.
    if !enums.contains_key("Ordering") {
        let mut ord = HashMap::new();
        ord.insert("Less".to_string(), Vec::new());
        ord.insert("Equal".to_string(), Vec::new());
        ord.insert("Greater".to_string(), Vec::new());
        enums.insert("Ordering".to_string(), ord);
    }
    let mut globals: HashMap<String, Type> = HashMap::new();
    for g in &prog.globals {
        if globals
            .insert(g.name.clone(), resolve_aliases(&g.ty, &aliases))
            .is_some()
        {
            return Err(format!("typeck: duplicate global {}", g.name));
        }
    }
    let mut methods: HashMap<(String, String), MethodSig> = HashMap::new();
    for imp in &prog.impls {
        let target_ty = resolve_aliases(&imp.target, &aliases);
        let target = impl_target_name(&target_ty)?;
        if !structs.contains_key(&target) && !enums.contains_key(&target) {
            return Err(format!(
                "typeck: impl target {} is not a known struct/enum",
                target
            ));
        }
        // Within an impl, `Self` names the target type; resolve it like any
        // other alias so `-> Self` / `other: Self` register concretely.
        let mut impl_aliases = aliases.clone();
        impl_aliases.insert("Self".to_string(), Type::Named(target.clone()));
        for c in &imp.consts {
            globals.insert(
                format!("{}::{}", target, c.name),
                resolve_aliases(&c.ty, &impl_aliases),
            );
        }
        for m in &imp.methods {
            let sig = MethodSig {
                receiver: m.receiver,
                params: m
                    .params
                    .iter()
                    .map(|p| resolve_aliases(&p.ty, &impl_aliases))
                    .collect(),
                ret: resolve_aliases(&m.ret, &impl_aliases),
            };
            if methods
                .insert((target.clone(), m.name.clone()), sig)
                .is_some()
            {
                return Err(format!("typeck: duplicate method {}::{}", target, m.name));
            }
        }
        // Flatten trait default method signatures (not overridden by this impl).
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
                            let mut params = Vec::new();
                            for pr in &dm.params {
                                params.push(resolve_aliases(&pr.ty, &impl_aliases));
                            }
                            let sig = MethodSig {
                                receiver: dm.receiver,
                                params,
                                ret: resolve_aliases(&dm.ret, &impl_aliases),
                            };
                            methods.insert(key, sig);
                        }
                    }
                }
            }
        }
    }

    let mut ck = TypeCk {
        sigs,
        structs,
        unit_structs,
        tuple_structs,
        enums,
        enum_named_fields,
        methods,
        aliases,
        globals,
        ret: Type::Unit,
        scopes: Vec::new(),
        loop_depth: 0,
        loop_break_types: Vec::new(),
        loop_labels: Vec::new(),
        pending_loop_label: None,
    };
    for g in &prog.globals {
        ck.check_global(g)?;
    }
    for f in &prog.funcs {
        ck.check_func(f)?;
    }
    for imp in &prog.impls {
        ck.check_impl(imp)?;
    }
    Ok(())
}

struct TypeCk {
    sigs: HashMap<String, (Vec<Type>, Type)>,
    structs: HashMap<String, Vec<(String, Type)>>,
    unit_structs: Vec<String>,
    tuple_structs: Vec<String>,
    enums: HashMap<String, HashMap<String, Vec<Type>>>,
    enum_named_fields: HashMap<(String, String), Vec<(String, Type)>>,
    methods: HashMap<(String, String), MethodSig>,
    aliases: HashMap<String, Type>,
    globals: HashMap<String, Type>,
    ret: Type,
    /// name -> (type, is_mut)
    scopes: Vec<HashMap<String, (Type, bool)>>,
    loop_depth: usize,
    loop_break_types: Vec<Option<Type>>,
    /// Loop labels parallel to `loop_break_types` (so `break 'outer v` routes v's
    /// type to the labeled loop's slot, not the innermost). Set by `Labeled`,
    /// consumed by `Loop`.
    loop_labels: Vec<Option<String>>,
    pending_loop_label: Option<String>,
}

impl TypeCk {
    fn resolve_type(&self, ty: &Type) -> Type {
        resolve_aliases(ty, &self.aliases)
    }

    fn check_global(&mut self, g: &Global) -> Result<(), String> {
        let ann = self.resolve_type(&g.ty);
        let t = self.type_expr_against(&g.init, &ann, &format!("global {}", g.name))?;
        if !type_compatible(&t, &ann) {
            return Err(format!(
                "typeck: global {}: {:?} initialized with {:?}",
                g.name, ann, t
            ));
        }
        Ok(())
    }

    fn check_func(&mut self, f: &Func) -> Result<(), String> {
        self.ret = self.resolve_type(&f.ret);
        let mut frame = HashMap::new();
        for p in &f.params {
            frame.insert(p.name.clone(), (self.resolve_type(&p.ty), false));
        }
        self.scopes.push(frame);
        let body_ty = match self.type_block(&f.body) {
            Ok(t) => t,
            Err(e) => {
                self.scopes.pop();
                return Err(format!("typeck: in fn {}: {}", f.name, e));
            }
        };
        self.scopes.pop();
        let checks_body = f.body.tail.is_some() || block_falls_through(&f.body);
        if checks_body && !type_compatible(&body_ty, &self.ret) {
            return Err(format!(
                "typeck: {} returns {:?} but body yields {:?}",
                f.name, self.ret, body_ty
            ));
        }
        Ok(())
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

    fn check_impl(&mut self, imp: &ImplBlock) -> Result<(), String> {
        let target = impl_target_name(&imp.target)?;
        for m in &imp.methods {
            self.check_method(&target, m)?;
        }
        Ok(())
    }

    fn check_method(&mut self, target: &str, m: &Method) -> Result<(), String> {
        // Make `Self` resolve to the impl target for the duration of the body
        // (so `-> Self`, `Self { .. }`, `Self::new()` all see the concrete type).
        let saved_self = self
            .aliases
            .insert("Self".to_string(), Type::Named(target.to_string()));
        let result = self.check_method_inner(target, m);
        match saved_self {
            Some(t) => {
                self.aliases.insert("Self".to_string(), t);
            }
            None => {
                self.aliases.remove("Self");
            }
        }
        result
    }

    fn check_method_inner(&mut self, target: &str, m: &Method) -> Result<(), String> {
        self.ret = self.resolve_type(&m.ret);
        let mut frame = HashMap::new();
        if let Some(receiver) = m.receiver {
            let self_ty = match receiver {
                ReceiverKind::Value => Type::Named(target.to_string()),
                ReceiverKind::Ref => Type::Ref {
                    mutable: false,
                    inner: Box::new(Type::Named(target.to_string())),
                },
                ReceiverKind::RefMut => Type::Ref {
                    mutable: true,
                    inner: Box::new(Type::Named(target.to_string())),
                },
            };
            frame.insert(
                "self".to_string(),
                (self_ty, receiver == ReceiverKind::RefMut),
            );
        }
        for p in &m.params {
            frame.insert(p.name.clone(), (self.resolve_type(&p.ty), false));
        }
        self.scopes.push(frame);
        let body_ty = match self.type_block(&m.body) {
            Ok(t) => t,
            Err(e) => {
                self.scopes.pop();
                return Err(format!("typeck: in method {}::{}: {}", target, m.name, e));
            }
        };
        self.scopes.pop();
        let checks_body = m.body.tail.is_some() || block_falls_through(&m.body);
        if checks_body && !type_compatible(&body_ty, &self.ret) {
            return Err(format!(
                "typeck: {}::{} returns {:?} but body yields {:?}",
                target, m.name, self.ret, body_ty
            ));
        }
        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        self.lookup_full(name).map(|(t, _)| t)
    }

    fn lookup_full(&self, name: &str) -> Option<(Type, bool)> {
        for frame in self.scopes.iter().rev() {
            if let Some(entry) = frame.get(name) {
                return Some(entry.clone());
            }
        }
        if let Some(ty) = self.globals.get(name) {
            return Some((ty.clone(), false));
        }
        None
    }

    fn define(&mut self, name: &str, ty: Type, mutable: bool) {
        let ty = self.resolve_type(&ty);
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_string(), (ty, mutable));
    }

    fn refine_var_placeholder(&mut self, name: &str, ty: Type) {
        for frame in self.scopes.iter_mut().rev() {
            if let Some(entry) = frame.get_mut(name) {
                if is_refinable_placeholder(&entry.0) && !is_refinable_placeholder(&ty) {
                    entry.0 = ty;
                }
                return;
            }
        }
    }

    fn refine_vec_placeholder(&mut self, receiver: &Expr, elem: Type) {
        let name = match receiver {
            Expr::Var(name) => name,
            _ => return,
        };
        for frame in self.scopes.iter_mut().rev() {
            if let Some(entry) = frame.get_mut(name) {
                if matches!(&entry.0, Type::Generic { name, args } if name == "Vec" && args.is_empty())
                {
                    entry.0 = Type::Generic {
                        name: "Vec".to_string(),
                        args: vec![elem],
                    };
                }
                return;
            }
        }
    }

    fn type_block(&mut self, block: &Block) -> Result<Type, String> {
        self.scopes.push(HashMap::new());
        let result = (|| -> Result<Type, String> {
            let mut last_stmt = Type::Unit;
            for s in &block.stmts {
                last_stmt = self.check_stmt(s)?;
            }
            match &block.tail {
                Some(e) => self.type_expr(e),
                None if last_stmt == Type::Never => Ok(Type::Never),
                None => Ok(Type::Unit),
            }
        })();
        self.scopes.pop();
        result
    }

    fn check_stmt(&mut self, s: &Stmt) -> Result<Type, String> {
        match s {
            Stmt::Let {
                name,
                mutable,
                ty,
                init,
            } => {
                let bind = if let Some(ann) = ty {
                    let ann = self.resolve_type(ann);
                    let t = self.type_expr_against(init, &ann, &format!("let {}", name))?;
                    if !type_compatible(&t, &ann) {
                        return Err(format!(
                            "typeck: let {}: {:?} initialized with {:?}",
                            name, ann, t
                        ));
                    }
                    ann
                } else {
                    collapse_lit(self.type_expr(init)?)
                };
                self.define(name, bind, *mutable);
                Ok(Type::Unit)
            }
            Stmt::LetPat { pat, init } => {
                let t = self.type_expr(init)?;
                self.check_pattern(pat, &t)?;
                Ok(Type::Unit)
            }
            Stmt::LetElse {
                pat,
                init,
                else_blk,
            } => {
                let t = self.type_expr(init)?;
                self.scopes.push(HashMap::new());
                let pat_result = self.check_pattern(pat, &t);
                let bindings = self.scopes.pop().unwrap();
                pat_result?;
                let else_ty = self.type_block(else_blk)?;
                if else_ty != Type::Never {
                    return Err(format!(
                        "typeck: let-else else block must diverge, got {:?}",
                        else_ty
                    ));
                }
                for (name, binding) in bindings.iter() {
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert(name.clone(), binding.clone());
                }
                Ok(Type::Unit)
            }
            Stmt::Assign { target, value } => {
                let (vt, is_mut) = self.place_type(target)?;
                if !is_mut {
                    return Err(format!(
                        "typeck: cannot assign to immutable place {:?}",
                        target
                    ));
                }
                let rt = self.type_expr(value)?;
                if !type_compatible(&rt, &vt) {
                    return Err(format!(
                        "typeck: assigning {:?} to place of type {:?}",
                        rt, vt
                    ));
                }
                if let Expr::Var(name) = target {
                    self.refine_var_placeholder(name, rt);
                }
                Ok(Type::Unit)
            }
            Stmt::Expr(e) => {
                let t = self.type_expr(e)?;
                if t == Type::Never {
                    Ok(Type::Never)
                } else {
                    Ok(Type::Unit)
                }
            }
            Stmt::Return(opt) => {
                let t = match opt {
                    Some(e) => self.type_expr(e)?,
                    None => Type::Unit,
                };
                if !type_compatible(&t, &self.ret) {
                    return Err(format!(
                        "typeck: return {:?} in fn returning {:?}",
                        t, self.ret
                    ));
                }
                Ok(Type::Never)
            }
        }
    }

    fn type_expr(&mut self, e: &Expr) -> Result<Type, String> {
        match e {
            Expr::Int(_) => Ok(Type::IntLit),
            Expr::IntHex(_, _) => Ok(Type::IntLit),
            Expr::Float(_) => Ok(Type::F64),
            Expr::Char(_) => Ok(Type::Char),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Str(_) => Ok(Type::Ref {
                mutable: false,
                inner: Box::new(Type::Named("str".to_string())),
            }),
            Expr::Var(name) => {
                if name == "None" {
                    return Ok(Type::Generic {
                        name: "Option".to_string(),
                        args: Vec::new(),
                    });
                }
                if let Some(local) = self.lookup(name) {
                    return Ok(local);
                }
                if let Some((params, ret)) = self.sigs.get(name).cloned() {
                    return Ok(Type::Closure {
                        params,
                        ret: Box::new(ret),
                    });
                }
                if self.is_unit_struct(name) {
                    return Ok(Type::Named(name.clone()));
                }
                Err(format!("typeck: unbound variable {}", name))
            }
            Expr::Ref { mutable, expr } => {
                let inner = if *mutable {
                    match self.place_type(expr) {
                        Ok((inner, true)) => inner,
                        Ok((_, false)) => {
                            return Err("typeck: cannot take &mut of immutable place".to_string());
                        }
                        Err(_) => self.type_expr(expr)?,
                    }
                } else {
                    self.type_expr(expr)?
                };
                Ok(Type::Ref {
                    mutable: *mutable,
                    inner: Box::new(inner),
                })
            }
            Expr::Unary { op, rhs } => {
                let t = self.type_expr(rhs)?;
                match op {
                    UnOp::Neg => {
                        if !matches!(t, Type::I64 | Type::I32 | Type::F64 | Type::IntLit) {
                            return Err(format!("typeck: unary - on {:?}", t));
                        }
                        Ok(t)
                    }
                    UnOp::Not => {
                        let t = deref_type(t);
                        if t == Type::Bool {
                            Ok(Type::Bool)
                        } else if is_integer(&t) {
                            Ok(t)
                        } else {
                            Err(format!("typeck: unary ! on {:?}", t))
                        }
                    }
                    UnOp::Deref => match t {
                        Type::Ref { inner, .. } => Ok(*inner),
                        Type::Generic { name, args }
                            if (name == "Box" || name == "Rc") && args.len() == 1 =>
                        {
                            Ok(args[0].clone())
                        }
                        other => Err(format!("typeck: cannot deref {:?}", other)),
                    },
                }
            }
            Expr::Binary { op, lhs, rhs } => {
                let l = self.type_expr(lhs)?;
                let r = self.type_expr(rhs)?;
                self.type_binary(*op, l, r)
            }
            Expr::Cast { expr, ty } => {
                let from = self.type_expr(expr)?;
                if !is_valid_cast(&from, ty) {
                    return Err(format!("typeck: cannot cast {:?} as {:?}", from, ty));
                }
                // rustc's overflowing_literals lint (deny-by-default) rejects a
                // literal cast directly out of the target's range, e.g.
                // `1000 as u8`. The interp's i64 value model would silently
                // truncate. Casting a *variable* (`let x = 1000; x as u8`) is a
                // legal runtime wrapping cast, so only literal operands are linted.
                if let Some(n) = literal_int_value(expr) {
                    let out_of_range = match ty {
                        Type::U8 => n < 0 || n > 255,
                        Type::U32 => n < 0 || n > 4294967295,
                        Type::I32 => n < -2147483648 || n > 2147483647,
                        Type::U64 | Type::Usize => n < 0,
                        _ => false,
                    };
                    if out_of_range {
                        return Err(format!(
                            "typeck: literal {} out of range for {:?} (overflowing_literals)",
                            n, ty
                        ));
                    }
                }
                Ok(ty.clone())
            }
            Expr::Try(expr) => {
                let t = self.type_expr(expr)?;
                match t {
                    Type::Generic { name, args } if name == "Option" && args.len() == 1 => {
                        match &self.ret {
                            Type::Generic { name: ret_name, .. } if ret_name == "Option" => {
                                Ok(args[0].clone())
                            }
                            other => {
                                Err(format!("typeck: ? on Option in fn returning {:?}", other))
                            }
                        }
                    }
                    Type::Generic { name, args } if name == "Result" && args.len() == 2 => {
                        match &self.ret {
                            Type::Generic {
                                name: ret_name,
                                args: ret_args,
                            } if ret_name == "Result"
                                && ret_args.len() == 2
                                && question_error_compatible(&args[1], &ret_args[1]) =>
                            {
                                Ok(args[0].clone())
                            }
                            other => {
                                Err(format!("typeck: ? on Result in fn returning {:?}", other))
                            }
                        }
                    }
                    other => Err(format!("typeck: ? on {:?}", other)),
                }
            }
            Expr::Return(opt) => {
                let t = match opt {
                    Some(e) => self.type_expr(e)?,
                    None => Type::Unit,
                };
                if !type_compatible(&t, &self.ret) {
                    return Err(format!(
                        "typeck: return {:?} in fn returning {:?}",
                        t, self.ret
                    ));
                }
                Ok(Type::Never)
            }
            Expr::Assign { target, value } => {
                let (vt, is_mut) = self.place_type(target)?;
                if !is_mut {
                    return Err(format!(
                        "typeck: cannot assign to immutable place {:?}",
                        target
                    ));
                }
                let rt = self.type_expr(value)?;
                if !type_compatible(&rt, &vt) {
                    return Err(format!(
                        "typeck: assigning {:?} to place of type {:?}",
                        rt, vt
                    ));
                }
                if let Expr::Var(name) = target.as_ref() {
                    self.refine_var_placeholder(name, rt);
                }
                Ok(Type::Unit)
            }
            Expr::Closure { params, ret, body } => {
                let mut param_tys = Vec::with_capacity(params.len());
                self.scopes.push(HashMap::new());
                let saved_ret = self.ret.clone();
                if let Some(ann) = ret {
                    self.ret = ann.clone();
                }
                let body_ty = (|| -> Result<Type, String> {
                    for p in params {
                        let ty = p.ty.clone().ok_or_else(|| {
                            format!("typeck: closure parameter {:?} needs a type", p.pat)
                        })?;
                        param_tys.push(ty.clone());
                        self.check_pattern(&p.pat, &ty)?;
                    }
                    self.type_expr(body)
                })();
                self.ret = saved_ret;
                self.scopes.pop();
                let body_ty = body_ty?;
                let out_ty = match ret {
                    Some(ann) => {
                        if !type_compatible(&body_ty, ann) {
                            return Err(format!(
                                "typeck: closure returns {:?}, expected {:?}",
                                body_ty, ann
                            ));
                        }
                        ann.clone()
                    }
                    None => body_ty,
                };
                Ok(Type::Closure {
                    params: param_tys,
                    ret: Box::new(out_ty),
                })
            }
            Expr::Call { name, args } => {
                match name.as_str() {
                    "Some" => {
                        if args.len() != 1 {
                            return Err(format!("typeck: Some expects 1 arg, got {}", args.len()));
                        }
                        let inner = collapse_lit(self.type_expr(&args[0])?);
                        return Ok(Type::Generic {
                            name: "Option".to_string(),
                            args: vec![inner],
                        });
                    }
                    "Ok" | "Err" => {
                        if args.len() != 1 {
                            return Err(format!(
                                "typeck: {} expects 1 arg, got {}",
                                name,
                                args.len()
                            ));
                        }
                        let t = self.type_expr(&args[0])?;
                        return Ok(if name == "Ok" {
                            Type::Generic {
                                name: "Result".to_string(),
                                args: vec![t, Type::Unit],
                            }
                        } else {
                            Type::Generic {
                                name: "Result".to_string(),
                                args: vec![Type::Unit, t],
                            }
                        });
                    }
                    _ => {}
                }
                if let Some(local_ty) = self.lookup(name) {
                    return match local_ty {
                        Type::Closure { params, ret } => self.check_closure_call(
                            &format!("closure {}", name),
                            &params,
                            &ret,
                            args,
                        ),
                        other => Err(format!("typeck: {:?} is not callable", other)),
                    };
                }
                if self.is_tuple_struct(name) {
                    let fields = match self.structs.get(name) { Some(f) => f.clone(), None => Vec::new() };
                    if args.len() != fields.len() {
                        return Err(format!(
                            "typeck: tuple struct {} expects {} fields, got {}",
                            name,
                            fields.len(),
                            args.len()
                        ));
                    }
                    let mut i = 0;
                    while i < args.len() {
                        let at = self.type_expr(&args[i])?;
                        if !type_compatible(&at, &fields[i].1) {
                            return Err(format!(
                                "typeck: tuple struct {} field {} expects {:?}, got {:?}",
                                name, i, fields[i].1, at
                            ));
                        }
                        i += 1;
                    }
                    return Ok(Type::Named(name.clone()));
                }
                let (params, ret) = self
                    .sigs
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("typeck: call to unknown function {}", name))?;
                if args.len() != params.len() {
                    return Err(format!(
                        "typeck: {} expects {} args, got {}",
                        name,
                        params.len(),
                        args.len()
                    ));
                }
                let subst = self.check_call_args(name, name, args, &params)?;
                Ok(apply_subst(&ret, &subst))
            }
            Expr::CallExpr { callee, args } => match self.type_expr(callee)? {
                Type::Closure { params, ret } => {
                    self.check_closure_call("closure expression", &params, &ret, args)
                }
                other => Err(format!("typeck: {:?} is not callable", other)),
            },
            Expr::PathCall {
                type_name,
                item,
                args,
            } => {
                if type_name == "Vec" {
                    match item.as_str() {
                        "new" => {
                            if !args.is_empty() {
                                return Err("typeck: Vec::new expects 0 args".to_string());
                            }
                            return Ok(Type::Generic {
                                name: "Vec".to_string(),
                                args: Vec::new(),
                            });
                        }
                        "with_capacity" => {
                            if args.len() != 1 {
                                return Err(format!(
                                    "typeck: Vec::with_capacity expects 1 arg, got {}",
                                    args.len()
                                ));
                            }
                            let cap_ty = self.type_expr(&args[0])?;
                            if !is_integer(&cap_ty) {
                                return Err(format!(
                                    "typeck: Vec::with_capacity expects integer capacity, got {:?}",
                                    cap_ty
                                ));
                            }
                            return Ok(Type::Generic {
                                name: "Vec".to_string(),
                                args: Vec::new(),
                            });
                        }
                        _ => {}
                    }
                }
                if type_name == "String" {
                    match item.as_str() {
                        "new" => {
                            if !args.is_empty() {
                                return Err("typeck: String::new expects 0 args".to_string());
                            }
                            return Ok(Type::Named("String".to_string()));
                        }
                        "from" => {
                            if args.len() != 1 {
                                return Err(format!(
                                    "typeck: String::from expects 1 arg, got {}",
                                    args.len()
                                ));
                            }
                            let got = self.type_expr(&args[0])?;
                            expect_str_like(&got, "String::from")?;
                            return Ok(Type::Named("String".to_string()));
                        }
                        "from_utf8_lossy" => {
                            if args.len() != 1 {
                                return Err(format!(
                                    "typeck: String::from_utf8_lossy expects 1 arg, got {}",
                                    args.len()
                                ));
                            }
                            self.type_expr(&args[0])?;
                            return Ok(Type::Named("String".to_string()));
                        }
                        _ => {}
                    }
                }
                if matches!(type_name.as_str(), "Command" | "std::process::Command")
                    && item == "new"
                {
                    if args.len() != 1 {
                        return Err(format!(
                            "typeck: {}::new expects 1 arg, got {}",
                            type_name,
                            args.len()
                        ));
                    }
                    self.type_expr(&args[0])?;
                    return Ok(Type::Named("Command".to_string()));
                }
                if matches!(type_name.as_str(), "env" | "std::env") && item == "args" {
                    if !args.is_empty() {
                        return Err(format!(
                            "typeck: {}::args expects 0 args, got {}",
                            type_name,
                            args.len()
                        ));
                    }
                    return Ok(Type::Generic {
                        name: "Iter".to_string(),
                        args: vec![Type::Named("String".to_string())],
                    });
                }
                if matches!(type_name.as_str(), "env" | "std::env") && item == "current_exe" {
                    if !args.is_empty() {
                        return Err(format!(
                            "typeck: {}::current_exe expects 0 args, got {}",
                            type_name,
                            args.len()
                        ));
                    }
                    return Ok(Type::Generic {
                        name: "Result".to_string(),
                        args: vec![
                            Type::Named("PathBuf".to_string()),
                            Type::Named("String".to_string()),
                        ],
                    });
                }
                if matches!(type_name.as_str(), "env" | "std::env") && item == "var" {
                    if args.len() != 1 {
                        return Err(format!(
                            "typeck: {}::var expects 1 arg, got {}",
                            type_name,
                            args.len()
                        ));
                    }
                    let key = self.type_expr(&args[0])?;
                    expect_str_like(&key, "env::var key")?;
                    return Ok(Type::Generic {
                        name: "Result".to_string(),
                        args: vec![
                            Type::Named("String".to_string()),
                            Type::Named("String".to_string()),
                        ],
                    });
                }
                if type_name == "PathBuf" && item == "from" {
                    if args.len() != 1 {
                        return Err(format!(
                            "typeck: PathBuf::from expects 1 arg, got {}",
                            args.len()
                        ));
                    }
                    self.type_expr(&args[0])?;
                    return Ok(Type::Named("PathBuf".to_string()));
                }
                if type_name == "Path" && item == "new" {
                    if args.len() != 1 {
                        return Err(format!(
                            "typeck: Path::new expects 1 arg, got {}",
                            args.len()
                        ));
                    }
                    let got = self.type_expr(&args[0])?;
                    expect_str_like(&got, "Path::new")?;
                    return Ok(Type::Named("PathBuf".to_string()));
                }
                if type_name == "char" && item == "from_u32" {
                    if args.len() != 1 {
                        return Err(format!(
                            "typeck: char::from_u32 expects 1 arg, got {}",
                            args.len()
                        ));
                    }
                    let arg_ty = self.type_expr(&args[0])?;
                    if !is_integer(&arg_ty) {
                        return Err(format!(
                            "typeck: char::from_u32 arg is {:?}, expected integer",
                            arg_ty
                        ));
                    }
                    return Ok(Type::Generic {
                        name: "Option".to_string(),
                        args: vec![Type::Char],
                    });
                }
                if is_int_target(type_name) && item == "from_str_radix" {
                    if args.len() != 2 {
                        return Err(format!(
                            "typeck: {}::from_str_radix expects 2 args, got {}",
                            type_name,
                            args.len()
                        ));
                    }
                    let text_ty = self.type_expr(&args[0])?;
                    expect_str_like(&text_ty, "from_str_radix input")?;
                    let radix_ty = self.type_expr(&args[1])?;
                    if !is_integer(&radix_ty) {
                        return Err(format!(
                            "typeck: {}::from_str_radix radix is {:?}, expected integer",
                            type_name, radix_ty
                        ));
                    }
                    return Ok(Type::Generic {
                        name: "Result".to_string(),
                        args: vec![
                            int_target_type(type_name)?,
                            Type::Named("String".to_string()),
                        ],
                    });
                }
                if matches!(type_name.as_str(), "fs" | "std::fs") {
                    match item.as_str() {
                        "create_dir_all" => {
                            if args.len() != 1 {
                                return Err(format!(
                                    "typeck: {}::create_dir_all expects 1 arg, got {}",
                                    type_name,
                                    args.len()
                                ));
                            }
                            self.type_expr(&args[0])?;
                            return Ok(Type::Generic {
                                name: "Result".to_string(),
                                args: vec![Type::Unit, Type::Named("String".to_string())],
                            });
                        }
                        "write" => {
                            if args.len() != 2 {
                                return Err(format!(
                                    "typeck: {}::write expects 2 args, got {}",
                                    type_name,
                                    args.len()
                                ));
                            }
                            self.type_expr(&args[0])?;
                            self.type_expr(&args[1])?;
                            return Ok(Type::Generic {
                                name: "Result".to_string(),
                                args: vec![Type::Unit, Type::Named("String".to_string())],
                            });
                        }
                        "read_to_string" => {
                            if args.len() != 1 {
                                return Err(format!(
                                    "typeck: {}::read_to_string expects 1 arg, got {}",
                                    type_name,
                                    args.len()
                                ));
                            }
                            self.type_expr(&args[0])?;
                            return Ok(Type::Generic {
                                name: "Result".to_string(),
                                args: vec![
                                    Type::Named("String".to_string()),
                                    Type::Named("String".to_string()),
                                ],
                            });
                        }
                        "read" => {
                            if args.len() != 1 {
                                return Err(format!(
                                    "typeck: {}::read expects 1 arg, got {}",
                                    type_name,
                                    args.len()
                                ));
                            }
                            self.type_expr(&args[0])?;
                            return Ok(Type::Generic {
                                name: "Result".to_string(),
                                args: vec![
                                    Type::Generic {
                                        name: "Vec".to_string(),
                                        args: vec![Type::U8],
                                    },
                                    Type::Named("String".to_string()),
                                ],
                            });
                        }
                        _ => {}
                    }
                }
                match (type_name.as_str(), item.as_str()) {
                    ("HashMap", "new") => {
                        if !args.is_empty() {
                            return Err(format!(
                                "typeck: HashMap::new expects 0 args, got {}",
                                args.len()
                            ));
                        }
                        return Ok(Type::Generic {
                            name: "HashMap".to_string(),
                            args: Vec::new(),
                        });
                    }
                    ("Box", "new") | ("Rc", "new") | ("RefCell", "new") => {
                        if args.len() != 1 {
                            return Err(format!(
                                "typeck: {}::new expects 1 arg, got {}",
                                type_name,
                                args.len()
                            ));
                        }
                        let inner = self.type_expr(&args[0])?;
                        return Ok(Type::Generic {
                            name: type_name.clone(),
                            args: vec![inner],
                        });
                    }
                    ("Rc", "clone") => {
                        if args.len() != 1 {
                            return Err(format!(
                                "typeck: Rc::clone expects 1 arg, got {}",
                                args.len()
                            ));
                        }
                        let arg_ty = self.type_expr(&args[0])?;
                        let inner = match arg_ty {
                            Type::Ref { inner, .. } => match *inner {
                                Type::Generic { name, args } if name == "Rc" && args.len() == 1 => {
                                    args[0].clone()
                                }
                                other => return Err(format!("typeck: Rc::clone on {:?}", other)),
                            },
                            other => {
                                return Err(format!(
                                    "typeck: Rc::clone expects &Rc<T>, got {:?}",
                                    other
                                ));
                            }
                        };
                        return Ok(Type::Generic {
                            name: "Rc".to_string(),
                            args: vec![inner],
                        });
                    }
                    ("Rc", "ptr_eq") => {
                        if args.len() != 2 {
                            return Err(format!(
                                "typeck: Rc::ptr_eq expects 2 args, got {}",
                                args.len()
                            ));
                        }
                        let mut inners = Vec::new();
                        for arg in args {
                            let arg_ty = self.type_expr(arg)?;
                            let inner = match arg_ty {
                                Type::Ref { inner, .. } => match *inner {
                                    Type::Generic { name, args }
                                        if name == "Rc" && args.len() == 1 =>
                                    {
                                        args[0].clone()
                                    }
                                    other => {
                                        return Err(format!(
                                            "typeck: Rc::ptr_eq on {:?}",
                                            other
                                        ))
                                    }
                                },
                                other => {
                                    return Err(format!(
                                        "typeck: Rc::ptr_eq expects &Rc<T>, got {:?}",
                                        other
                                    ))
                                }
                            };
                            inners.push(inner);
                        }
                        if !same_generic_type(&inners[0], &inners[1]) {
                            return Err(format!(
                                "typeck: Rc::ptr_eq inner types differ: {:?} vs {:?}",
                                inners[0], inners[1]
                            ));
                        }
                        return Ok(Type::Bool);
                    }
                    _ => {}
                }
                if let Some((params, ret)) = self.sigs.get(item).cloned() {
                    let subst = self.check_call_args(type_name, item, args, &params)?;
                    return Ok(apply_subst(&ret, &subst));
                }
                if (type_name == "std::cmp" || type_name == "cmp")
                    && matches!(item.as_str(), "max" | "min")
                {
                    if args.len() != 2 {
                        return Err(format!("typeck: cmp::{} expects 2 args", item));
                    }
                    let a = self.type_expr(&args[0])?;
                    self.type_expr(&args[1])?;
                    return Ok(a);
                }
                if let Ok(fields) = self.enum_variant_fields(type_name, item) {
                    let subst = self.check_call_args(type_name, item, args, &fields)?;
                    return Ok(self.generic_instance_type(type_name, &subst));
                }

                let sig = self
                    .methods
                    .get(&(type_name.clone(), item.clone()))
                    .cloned()
                    .ok_or_else(|| {
                        format!("typeck: unknown associated item {}::{}", type_name, item)
                    })?;
                if sig.receiver.is_some() {
                    return Err(format!("typeck: {}::{} needs a receiver", type_name, item));
                }
                let subst = self.check_call_args(type_name, item, args, &sig.params)?;
                Ok(apply_subst(&sig.ret, &subst))
            }
            Expr::MethodCall {
                receiver,
                name,
                type_args,
                args,
            } => {
                let rt = self.type_expr(receiver)?;
                if deref_type(rt.clone()) == Type::IntLit {
                    return Err(format!(
                        "typeck: cannot call method `{}` on ambiguous numeric type (unsuffixed literal; rustc E0689 -- annotate, e.g. `0i64`)",
                        name
                    ));
                }
                let target = method_target_name(&rt)
                    .ok_or_else(|| format!("typeck: method {} on {:?}", name, rt))?;
                if name == "clone" {
                    if !args.is_empty() {
                        return Err(format!(
                            "typeck: {}::clone expects 0 args, got {}",
                            target,
                            args.len()
                        ));
                    }
                    return Ok(match rt {
                        Type::Ref { inner, .. } if !matches!(inner.as_ref(), Type::Named(name) if name == "str") => {
                            *inner
                        }
                        other => other,
                    });
                }
                if name == "parse" {
                    expect_str_like(&rt, "parse receiver")?;
                    if !args.is_empty() {
                        return Err("typeck: parse expects 0 args".to_string());
                    }
                    let elem = if type_args == &[Type::I64] {
                        Type::I64
                    } else if type_args == &[Type::F64] {
                        Type::F64
                    } else {
                        return Err(format!(
                            "typeck: parse only supports turbofish i64/f64, got {:?}",
                            type_args
                        ));
                    };
                    return Ok(Type::Generic {
                        name: "Result".to_string(),
                        args: vec![elem, Type::Named("String".to_string())],
                    });
                }
                if name == "into" {
                    if !args.is_empty() {
                        return Err(format!("typeck: into expects 0 args, got {}", args.len()));
                    }
                    return match rt {
                        Type::ImplTrait(name)
                            if name == "Into" || name.starts_with("Into<") =>
                        {
                            Ok(Type::Named("String".to_string()))
                        }
                        Type::Named(name) if name == "String" => {
                            Ok(Type::Named("String".to_string()))
                        }
                        Type::Ref { inner, .. } if matches!(inner.as_ref(), Type::Named(name) if name == "str") => {
                            Ok(Type::Named("String".to_string()))
                        }
                        other => Err(format!("typeck: into on {:?}", other)),
                    };
                }
                if target == "Vec" {
                    return self.type_vec_method(name, receiver, &rt, args);
                }
                if target == "slice" {
                    return self.type_slice_method(name, &rt, args);
                }
                if target == "String" || target == "str" {
                    return self.type_string_method(&target, name, receiver, &rt, args);
                }
                if target == "Path" || target == "PathBuf" {
                    return self.type_path_method(&target, name, args);
                }
                if target == "Command" {
                    return self.type_command_method(name, args);
                }
                if target == "ExitStatus" {
                    return self.type_exit_status_method(name, args);
                }
                if is_int_target(&target) {
                    return self.type_int_method(&target, name, &rt, args);
                }
                if target == "bool" {
                    return self.type_bool_method(name, args);
                }
                if target == "char" {
                    return self.type_char_method(name, args);
                }
                if target == "Option" {
                    return self.type_option_method(name, &rt, args);
                }
                if target == "Result" {
                    return self.type_result_method(name, &rt, args);
                }
                if target == "Box" || target == "Rc" {
                    return self.type_boxlike_method(&target, name, &rt, args);
                }
                if target == "RefCell" {
                    return self.type_refcell_method(name, &rt, args);
                }
                if target == "HashMap" {
                    return self.type_hashmap_method(name, receiver, &rt, args);
                }
                if target == "HashEntry" {
                    return self.type_hashentry_method(name, &rt, args);
                }
                if target == "Iter" {
                    return self.type_iter_method(name, receiver, &rt, type_args, args);
                }
                let lookup_target = flattened_method_target(&target);
                let sig = self
                    .methods
                    .get(&(target.clone(), name.clone()))
                    .or_else(|| self.methods.get(&(lookup_target.clone(), name.clone())))
                    .cloned()
                    .ok_or_else(|| format!("typeck: unknown method {}::{}", target, name))?;
                if sig.receiver.is_none() {
                    return Err(format!("typeck: {}::{} is not a method", target, name));
                }
                if sig.receiver == Some(ReceiverKind::RefMut) {
                    match receiver.as_ref() {
                        Expr::Var(var) => {
                            let (var_ty, is_mut) = self
                                .lookup_full(var)
                                .ok_or_else(|| format!("typeck: unbound variable {}", var))?;
                            if !is_mut && !matches!(var_ty, Type::Ref { mutable: true, .. }) {
                                return Err(format!(
                                    "typeck: cannot call &mut self method {}::{} on immutable {}",
                                    target, name, var
                                ));
                            }
                        }
                        _ => {
                            return Err(format!(
                                "typeck: &mut self method {}::{} needs a mutable variable receiver",
                                target, name
                            ));
                        }
                    }
                }
                let mut subst = self.type_subst_from_instance(&target, &rt);
                self.check_call_args_into(&target, name, args, &sig.params, &mut subst)?;
                Ok(apply_subst(&sig.ret, &subst))
            }
            Expr::If {
                cond,
                then_blk,
                else_blk,
            } => {
                let ct = self.type_expr(cond)?;
                expect(&ct, &Type::Bool, "if condition")?;
                let tt = self.type_block(then_blk)?;
                match else_blk {
                    Some(eb) => {
                        let et = self.type_block(eb)?;
                        common_type(&tt, &et).ok_or_else(|| {
                            format!("typeck: if/else branches differ: {:?} vs {:?}", tt, et)
                        })
                    }
                    None => {
                        if !type_compatible(&tt, &Type::Unit) {
                            return Err(format!(
                                "typeck: if without else must yield (), got {:?}",
                                tt
                            ));
                        }
                        Ok(Type::Unit)
                    }
                }
            }
            Expr::Block(b) => self.type_block(b),
            Expr::Println { fmt, args } => {
                self.check_format_args(fmt, args, "println!")?;
                Ok(Type::Unit)
            }
            Expr::Print { fmt, args } => {
                self.check_format_args(fmt, args, "print!")?;
                Ok(Type::Unit)
            }
            Expr::Eprintln { fmt, args } => {
                self.check_format_args(fmt, args, "eprintln!")?;
                Ok(Type::Unit)
            }
            Expr::Format { fmt, args } => {
                self.check_format_args(fmt, args, "format!")?;
                Ok(Type::Named("String".to_string()))
            }
            Expr::Write {
                target, fmt, args, ..
            } => {
                self.check_write_target(target)?;
                self.check_format_args(fmt, args, "write!/writeln!")?;
                Ok(Type::Generic {
                    name: "Result".to_string(),
                    args: vec![Type::Unit, Type::Unit],
                })
            }
            Expr::Panic { .. } => Ok(Type::Never),
            Expr::Assert { cond } => {
                let ct = self.type_expr(cond)?;
                expect(&ct, &Type::Bool, "assert! condition")?;
                Ok(Type::Unit)
            }
            Expr::AssertEq { left, right } => {
                let lt = self.type_expr(left)?;
                let rt = self.type_expr(right)?;
                self.type_binary(BinOp::Eq, lt, rt)?;
                Ok(Type::Unit)
            }
            Expr::Cfg { .. } => Ok(Type::Bool),
            Expr::Matches { expr, pat, guard } => {
                let st = self.type_expr(expr)?;
                self.scopes.push(HashMap::new());
                let result = (|| -> Result<Type, String> {
                    self.check_pattern(pat, &st)?;
                    if let Some(g) = guard {
                        let gt = self.type_expr(g)?;
                        expect(&gt, &Type::Bool, "matches! guard")?;
                    }
                    Ok(Type::Bool)
                })();
                self.scopes.pop();
                result
            }
            Expr::TupleLit(items) => {
                if items.is_empty() {
                    return Ok(Type::Unit);
                }
                let mut tys = Vec::with_capacity(items.len());
                for it in items {
                    tys.push(collapse_lit(self.type_expr(it)?));
                }
                Ok(Type::Tuple(tys))
            }
            Expr::VecLit(items) => {
                if items.is_empty() {
                    return Ok(Type::Generic {
                        name: "Vec".to_string(),
                        args: vec![Type::Unit],
                    });
                }
                let mut first = self.type_expr(&items[0])?;
                for it in &items[1..] {
                    let got = self.type_expr(it)?;
                    if got == first {
                        continue;
                    }
                    // An unsuffixed-literal element joins with any concrete
                    // integer element (the literal coerces).
                    if first == Type::IntLit && is_integer(&got) {
                        first = got;
                        continue;
                    }
                    if got == Type::IntLit && is_integer(&first) {
                        continue;
                    }
                    return Err(format!(
                        "typeck: vec! element {:?} differs from {:?}",
                        got, first
                    ));
                }
                Ok(Type::Generic {
                    name: "Vec".to_string(),
                    args: vec![collapse_lit(first)],
                })
            }
            Expr::VecRepeat { elem, count } => {
                let elem_ty = self.type_expr(elem)?;
                let count_ty = self.type_expr(count)?;
                if !is_integer(&count_ty) {
                    return Err(format!(
                        "typeck: repeat array count expected integer, got {:?}",
                        count_ty
                    ));
                }
                Ok(Type::Generic {
                    name: "Vec".to_string(),
                    args: vec![elem_ty],
                })
            }
            Expr::StructLit { name, fields } => {
                let def = self
                    .structs
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("typeck: unknown struct {}", name))?;
                if fields.len() != def.len() {
                    return Err(format!(
                        "typeck: struct {} expects {} fields, got {}",
                        name,
                        def.len(),
                        fields.len()
                    ));
                }
                let mut subst = HashMap::new();
                for (fname, fexpr) in fields {
                    let want = def
                        .iter()
                        .find(|(k, _)| k == fname)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| format!("typeck: struct {} has no field {}", name, fname))?;
                    let expected = apply_subst(&want, &subst);
                    let got = self.type_expr_against(
                        fexpr,
                        &expected,
                        &format!("struct {} field {}", name, fname),
                    )?;
                    if !unify_expected_type(&want, &got, &mut subst) {
                        return Err(format!(
                            "typeck: struct {} field {} is {:?}, expected {:?}",
                            name, fname, got, want
                        ));
                    }
                }
                Ok(self.generic_instance_type(name, &subst))
            }
            Expr::EnumCtor { enum_name, variant } => {
                if enum_name == "ExitCode" && matches!(variant.as_str(), "SUCCESS" | "FAILURE") {
                    return Ok(Type::Named("ExitCode".to_string()));
                }
                // Associated const `Target::N`?
                if let Some(ty) = self.globals.get(&format!("{}::{}", enum_name, variant)) {
                    return Ok(ty.clone());
                }
                let fields = self.enum_variant_fields(enum_name, variant)?;
                if fields.is_empty() {
                    Ok(Type::Named(enum_name.clone()))
                } else {
                    Ok(Type::Closure {
                        params: fields,
                        ret: Box::new(Type::Named(enum_name.clone())),
                    })
                }
            }
            Expr::EnumStructLit {
                enum_name,
                variant,
                fields,
            } => {
                let def = self
                    .enum_named_fields
                    .get(&(enum_name.clone(), variant.clone()))
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "typeck: {}::{} is not a struct-like enum variant",
                            enum_name, variant
                        )
                    })?;
                if fields.len() != def.len() {
                    return Err(format!(
                        "typeck: {}::{} expects {} fields, got {}",
                        enum_name,
                        variant,
                        def.len(),
                        fields.len()
                    ));
                }
                let mut subst = HashMap::new();
                for (fname, fexpr) in fields {
                    let want = def
                        .iter()
                        .find(|(k, _)| k == fname)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| {
                            format!("typeck: {}::{} has no field {}", enum_name, variant, fname)
                        })?;
                    let expected = apply_subst(&want, &subst);
                    let got = self.type_expr_against(
                        fexpr,
                        &expected,
                        &format!("{}::{} field {}", enum_name, variant, fname),
                    )?;
                    if !unify_expected_type(&want, &got, &mut subst) {
                        return Err(format!(
                            "typeck: {}::{} field {} is {:?}, expected {:?}",
                            enum_name, variant, fname, got, want
                        ));
                    }
                }
                Ok(self.generic_instance_type(enum_name, &subst))
            }
            Expr::Field { base, name } => {
                let bt = self.type_expr(base)?;
                match deref_type(bt) {
                    Type::Named(sname) => {
                        if sname == "Output" {
                            return match name.as_str() {
                                "status" => Ok(Type::Named("ExitStatus".to_string())),
                                "stdout" | "stderr" => Ok(Type::Named("String".to_string())),
                                _ => Err(format!("typeck: Output has no field {}", name)),
                            };
                        }
                        let def = self.structs.get(&sname).ok_or_else(|| {
                            format!("typeck: field access on non-struct {}", sname)
                        })?;
                        def.iter()
                            .find(|(k, _)| k == name)
                            .map(|(_, t)| t.clone())
                            .ok_or_else(|| format!("typeck: {} has no field {}", sname, name))
                    }
                    Type::Generic { name: sname, args } => {
                        if sname == "Output" {
                            return match name.as_str() {
                                "status" => Ok(Type::Named("ExitStatus".to_string())),
                                "stdout" | "stderr" => Ok(Type::Named("String".to_string())),
                                _ => Err(format!("typeck: Output has no field {}", name)),
                            };
                        }
                        let def = self.structs.get(&sname).ok_or_else(|| {
                            format!("typeck: field access on non-struct {}", sname)
                        })?;
                        let raw = def
                            .iter()
                            .find(|(k, _)| k == name)
                            .map(|(_, t)| t.clone())
                            .ok_or_else(|| format!("typeck: {} has no field {}", sname, name))?;
                        let subst = self.type_subst_from_args(&sname, &args);
                        Ok(apply_subst(&raw, &subst))
                    }
                    other => Err(format!("typeck: field access on {:?}", other)),
                }
            }
            Expr::TupleIndex { base, index } => {
                let bt = self.type_expr(base)?;
                match deref_type(bt) {
                    Type::Tuple(tys) => tys
                        .get(*index)
                        .cloned()
                        .ok_or_else(|| format!("typeck: tuple index {} out of range", index)),
                    Type::Named(sname) if self.is_tuple_struct(&sname) => {
                        let def = match self.structs.get(&sname) { Some(f) => f.clone(), None => Vec::new() };
                        def.get(*index).map(|(_, t)| t.clone()).ok_or_else(|| {
                            format!("typeck: tuple struct {} index {} out of range", sname, index)
                        })
                    }
                    other => Err(format!("typeck: tuple index on {:?}", other)),
                }
            }
            Expr::Index { base, index } => {
                let bt = deref_type(self.type_expr(base)?);
                let it = self.type_expr(index)?;
                // HashMap indexing `m[&k]` -> V; the key is a reference to (or
                // value of) the key type.
                if let Type::Generic { name, args } = &bt {
                    if name == "HashMap" && args.len() == 2 {
                        let kt = deref_type(it.clone());
                        if !type_compatible(&kt, &args[0]) && !type_compatible(&args[0], &kt) {
                            return Err(format!(
                                "typeck: HashMap index key is {:?}, expected {:?}",
                                kt, args[0]
                            ));
                        }
                        return Ok(args[1].clone());
                    }
                }
                if !is_integer(&it) {
                    return Err(format!("typeck: index expected integer, got {:?}", it));
                }
                vec_index_elem(&bt).ok_or_else(|| format!("typeck: index on {:?}", bt))
            }
            Expr::Slice {
                base, start, end, ..
            } => {
                let bt = deref_type(self.type_expr(base)?);
                if let Some(e) = start {
                    let t = self.type_expr(e)?;
                    if !is_integer(&t) {
                        return Err(format!("typeck: slice start expected integer, got {:?}", t));
                    }
                }
                if let Some(e) = end {
                    let t = self.type_expr(e)?;
                    if !is_integer(&t) {
                        return Err(format!("typeck: slice end expected integer, got {:?}", t));
                    }
                }
                match bt {
                    Type::Generic { name, args } if name == "Vec" && args.len() == 1 => {
                        Ok(Type::Generic { name, args })
                    }
                    other => Err(format!("typeck: slice on {:?}", other)),
                }
            }
            Expr::Range { start, end, .. } => {
                let st = self.type_expr(start)?;
                if !is_integer(&st) {
                    return Err(format!(
                        "typeck: range start expected integer, got {:?}",
                        st
                    ));
                }
                let et = self.type_expr(end)?;
                if !is_integer(&et) {
                    return Err(format!("typeck: range end expected integer, got {:?}", et));
                }
                let elem = common_type(&st, &et).unwrap_or(st);
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![elem],
                })
            }
            Expr::Match { scrut, arms } => {
                let st = self.type_expr(scrut)?;
                if arms.is_empty() {
                    return Err("typeck: empty match".to_string());
                }
                let mut result: Option<Type> = None;
                for arm in arms {
                    self.scopes.push(HashMap::new());
                    let arm_ty = (|| -> Result<Type, String> {
                        self.check_pattern(&arm.pat, &st)?;
                        if let Some(guard) = &arm.guard {
                            let gt = self.type_expr(guard)?;
                            expect(&gt, &Type::Bool, "match guard")?;
                        }
                        self.type_expr(&arm.body)
                    })();
                    self.scopes.pop();
                    let arm_ty = arm_ty?;
                    match &result {
                        None => result = Some(arm_ty),
                        Some(prev) => {
                            if let Some(common) = common_type(prev, &arm_ty) {
                                result = Some(common);
                            } else {
                                return Err(format!(
                                    "typeck: match arms differ: {:?} vs {:?} at pattern {:?} body {:?}",
                                    prev, arm_ty, arm.pat, arm.body
                                ));
                            }
                        }
                    }
                }
                self.check_match_exhaustive(&st, arms)?;
                Ok(result.unwrap())
            }
            Expr::While { cond, body } => {
                // A label on a while/for can't carry a break value; drop it so it
                // doesn't leak onto an inner `loop`.
                self.pending_loop_label = None;
                let ct = self.type_expr(cond)?;
                expect(&ct, &Type::Bool, "while condition")?;
                self.loop_depth += 1;
                let bt = self.type_block(body);
                self.loop_depth -= 1;
                expect(&bt?, &Type::Unit, "while body")?;
                Ok(Type::Unit)
            }
            Expr::WhileLet { pat, expr, body } => {
                self.pending_loop_label = None;
                let et = self.type_expr(expr)?;
                self.scopes.push(HashMap::new());
                self.check_pattern(pat, &et)?;
                self.loop_depth += 1;
                let bt = self.type_block(body);
                self.loop_depth -= 1;
                self.scopes.pop();
                expect(&bt?, &Type::Unit, "while let body")?;
                Ok(Type::Unit)
            }
            Expr::Loop { body } => {
                self.loop_depth += 1;
                let lbl = self.pending_loop_label.clone();
                self.pending_loop_label = None;
                self.loop_labels.push(lbl);
                self.loop_break_types.push(None);
                let bt = self.type_block(body);
                let break_ty = match self.loop_break_types.pop() {
                    Some(inner) => inner,
                    None => None,
                };
                self.loop_labels.pop();
                self.loop_depth -= 1;
                let bt = bt?;
                if bt != Type::Unit && bt != Type::Never {
                    return Err(format!("typeck: loop body yields {:?}", bt));
                }
                match break_ty {
                    // `loop { break v; }` is an expression of the break value's type.
                    Some(t) => Ok(t),
                    // plain `break;` (no value) -> unit loop expression.
                    None if block_contains_break(body) => Ok(Type::Unit),
                    // no break at all -> the loop diverges.
                    None => Ok(Type::Never),
                }
            }
            Expr::For {
                var,
                start,
                end,
                body,
                ..
            } => {
                self.pending_loop_label = None;
                let st = self.type_expr(start)?;
                if !is_integer(&st) {
                    return Err(format!("typeck: for start expected integer, got {:?}", st));
                }
                let et = self.type_expr(end)?;
                if !is_integer(&et) {
                    return Err(format!("typeck: for end expected integer, got {:?}", et));
                }
                self.scopes.push(HashMap::new());
                self.define(var, st, false);
                self.loop_depth += 1;
                let bt = self.type_block(body);
                self.loop_depth -= 1;
                self.scopes.pop();
                expect(&bt?, &Type::Unit, "for body")?;
                Ok(Type::Unit)
            }
            Expr::ForEach { pat, iter, body } => {
                let iter_ty = self.type_expr(iter)?;
                let elem = match iter_ty {
                    Type::Generic { name, args } if name == "Vec" && args.len() == 1 => {
                        args[0].clone()
                    }
                    Type::Ref {
                        mutable: false,
                        inner,
                    } => match *inner {
                        Type::Generic { name, args } if name == "Vec" && args.len() == 1 => {
                            Type::Ref {
                                mutable: false,
                                inner: Box::new(args[0].clone()),
                            }
                        }
                        Type::Slice(inner) => Type::Ref {
                            mutable: false,
                            inner,
                        },
                        other => return Err(format!("typeck: for over &{:?}", other)),
                    },
                    Type::Ref {
                        mutable: true,
                        inner,
                    } => {
                        return Err(format!("typeck: for over &mut {:?} is held", inner));
                    }
                    Type::Generic { name, args } if name == "Iter" && args.len() == 1 => {
                        args[0].clone()
                    }
                    other => return Err(format!("typeck: for over {:?}", other)),
                };
                self.scopes.push(HashMap::new());
                self.check_pattern(pat, &elem)?;
                self.loop_depth += 1;
                let bt = self.type_block(body);
                self.loop_depth -= 1;
                self.scopes.pop();
                expect(&bt?, &Type::Unit, "for body")?;
                Ok(Type::Unit)
            }
            Expr::Break { label, value: opt } => {
                if self.loop_depth == 0 {
                    return Err("typeck: `break` outside of a loop".to_string());
                }
                if let Some(e) = opt {
                    let vt = self.type_expr(e)?;
                    // Route the break value to the LABELED loop's slot (not the
                    // innermost) so `'o: loop { loop { break 'o v } }` types as v.
                    let idx = match label {
                        Some(l) => self
                            .loop_labels
                            .iter()
                            .rposition(|ll| matches!(ll, Some(x) if x == l)),
                        None => self.loop_break_types.len().checked_sub(1),
                    };
                    if let Some(i) = idx {
                        if let Some(slot) = self.loop_break_types.get_mut(i) {
                            *slot = Some(vt);
                        }
                    }
                }
                Ok(Type::Never)
            }
            Expr::Labeled { label, body } => {
                // Hand the label to the loop this wraps (consumed on its push).
                self.pending_loop_label = Some(label.clone());
                let t = self.type_expr(body);
                self.pending_loop_label = None;
                t
            }
            Expr::Continue => {
                if self.loop_depth == 0 {
                    return Err("typeck: `continue` outside of a loop".to_string());
                }
                Ok(Type::Never)
            }
        }
    }

    fn check_format_args(&mut self, fmt: &str, args: &[Expr], ctx: &str) -> Result<(), String> {
        let kinds = format_placeholder_kinds(fmt, ctx)?;
        if kinds.len() != args.len() {
            return Err(format!(
                "typeck: {} expects {} args, got {}",
                ctx,
                kinds.len(),
                args.len()
            ));
        }
        for (i, (arg, kind)) in args.iter().zip(kinds.iter()).enumerate() {
            let ty = self.type_expr(arg)?;
            // `{}` on a user type with a registered Display `fmt` method.
            if *kind == FormatArgKind::Display {
                if let Type::Named(n) = deref_type(ty.clone()) {
                    if self.methods.get(&(n.clone(), String::from("fmt"))).is_some() {
                        continue;
                    }
                }
            }
            check_format_arg_type(&ty, *kind, &format!("{} arg {}", ctx, i))?;
        }
        Ok(())
    }

    fn check_write_target(&mut self, target: &Expr) -> Result<(), String> {
        match self.type_expr(target)? {
            Type::Ref {
                mutable: true,
                inner,
            } if matches!(inner.as_ref(), Type::Named(name) if name == "String") => Ok(()),
            // `write!(f, ...)` inside a Display impl: the Formatter is modelled
            // as a String buffer at runtime.
            Type::Ref {
                mutable: true,
                inner,
            } if matches!(inner.as_ref(), Type::Named(name) if name == "fmt::Formatter" || name == "Formatter") => {
                Ok(())
            }
            Type::Named(name) if name == "String" => {
                let (_, is_mut) = self.place_type(target)?;
                if is_mut {
                    Ok(())
                } else {
                    Err("typeck: write!/writeln! target String must be mutable".to_string())
                }
            }
            other => Err(format!(
                "typeck: write!/writeln! target must be String or &mut String, got {:?}",
                other
            )),
        }
    }

    fn enum_variant_fields(&self, enum_name: &str, variant: &str) -> Result<Vec<Type>, String> {
        self.enums
            .get(enum_name)
            .ok_or_else(|| format!("typeck: unknown enum {}", enum_name))?
            .get(variant)
            .cloned()
            .ok_or_else(|| format!("typeck: enum {} has no variant {}", enum_name, variant))
    }

    fn generic_vars_for_type_name(&self, name: &str) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(fields) = self.structs.get(name) {
            for (_, ty) in fields {
                collect_generic_vars(ty, &mut out);
            }
        }
        if let Some(variants) = self.enums.get(name) {
            for (_variant, fields) in variants.iter() {
                for ty in fields {
                    collect_generic_vars(ty, &mut out);
                }
            }
        }
        out
    }

    fn type_subst_from_args(&self, name: &str, args: &[Type]) -> TypeSubst {
        let vars = self.generic_vars_for_type_name(name);
        let mut subst = HashMap::new();
        for (var, arg) in vars.iter().zip(args.iter()) {
            subst.insert(var.clone(), arg.clone());
        }
        subst
    }

    fn type_subst_from_instance(&self, name: &str, ty: &Type) -> TypeSubst {
        match deref_type(ty.clone()) {
            Type::Generic {
                name: got_name,
                args,
            } if got_name == name => self.type_subst_from_args(name, &args),
            _ => HashMap::new(),
        }
    }

    fn generic_instance_type(&self, name: &str, subst: &TypeSubst) -> Type {
        let vars = self.generic_vars_for_type_name(name);
        if vars.is_empty() {
            return Type::Named(name.to_string());
        }
        let mut args = Vec::new();
        for var in vars {
            args.push(subst.get(&var).cloned().unwrap_or_else(|| Type::Named(var)));
        }
        Type::Generic {
            name: name.to_string(),
            args,
        }
    }

    fn check_call_args(
        &mut self,
        owner: &str,
        name: &str,
        args: &[Expr],
        params: &[Type],
    ) -> Result<TypeSubst, String> {
        let mut subst = HashMap::new();
        self.check_call_args_into(owner, name, args, params, &mut subst)?;
        Ok(subst)
    }

    fn check_call_args_into(
        &mut self,
        owner: &str,
        name: &str,
        args: &[Expr],
        params: &[Type],
        subst: &mut TypeSubst,
    ) -> Result<(), String> {
        if args.len() != params.len() {
            return Err(format!(
                "typeck: {}::{} expects {} args, got {}",
                owner,
                name,
                params.len(),
                args.len()
            ));
        }
        for (i, (a, pt)) in args.iter().zip(params.iter()).enumerate() {
            let expected = apply_subst(pt, subst);
            let at =
                self.type_expr_against(a, &expected, &format!("{}::{} arg {}", owner, name, i))?;
            if !unify_expected_type(pt, &at, subst) {
                return Err(format!(
                    "typeck: {}::{} arg {} is {:?}, expected {:?}",
                    owner, name, i, at, pt
                ));
            }
        }
        Ok(())
    }

    fn check_closure_call(
        &mut self,
        label: &str,
        params: &[Type],
        ret: &Type,
        args: &[Expr],
    ) -> Result<Type, String> {
        if args.len() != params.len() {
            return Err(format!(
                "typeck: {} expects {} args, got {}",
                label,
                params.len(),
                args.len()
            ));
        }
        for (i, (a, pt)) in args.iter().zip(params.iter()).enumerate() {
            let at = self.type_expr_against(a, pt, &format!("{} arg {}", label, i))?;
            if !type_compatible(&at, pt) {
                return Err(format!(
                    "typeck: {} arg {} is {:?}, expected {:?}",
                    label, i, at, pt
                ));
            }
        }
        Ok(ret.clone())
    }

    fn type_expr_against(
        &mut self,
        expr: &Expr,
        expected: &Type,
        label: &str,
    ) -> Result<Type, String> {
        match (expr, expected) {
            (
                Expr::MethodCall {
                    receiver,
                    name,
                    args,
                    ..
                },
                Type::Generic {
                    name: want_name,
                    args: want_args,
                },
            ) if name == "collect" && want_name == "Vec" && want_args.len() == 1 => {
                let got = self.type_expr(expr)?;
                if type_compatible(&got, expected) {
                    return Ok(got);
                }
                if args.is_empty() {
                    if let Type::Generic {
                        name: iter_name,
                        args: iter_args,
                    } = deref_type(self.type_expr(receiver)?)
                    {
                        if iter_name == "Iter" && iter_args.len() == 1 {
                            let elem = deref_type(iter_args[0].clone());
                            if type_compatible(&elem, &want_args[0]) {
                                return Ok(expected.clone());
                            }
                        }
                    }
                }
                Ok(got)
            }
            // `let x: T = e.into()` -- the target `T` comes from the annotation
            // (a single hop, not full inference). Integer->integer and
            // str/String->String are the convertible shapes in this subset.
            (Expr::MethodCall { receiver, name, args, .. }, _)
                if name == "into" && args.is_empty() =>
            {
                let rt = deref_type(self.type_expr(receiver)?);
                let ok = (is_integer(&rt) && is_integer(expected))
                    || (is_string_like(&rt)
                        && matches!(expected, Type::Named(n) if n == "String"));
                if ok {
                    Ok(expected.clone())
                } else {
                    let got = self.type_expr(expr)?;
                    if type_compatible(&got, expected) {
                        Ok(got)
                    } else {
                        Err(format!(
                            "typeck: {} .into() from {:?} to {:?}",
                            label, rt, expected
                        ))
                    }
                }
            }
            (
                Expr::Closure { params, ret, body },
                Type::Closure {
                    params: want_params,
                    ret: want_ret,
                },
            ) => {
                if params.len() != want_params.len() {
                    return Err(format!(
                        "typeck: {} closure expects {} params, got {}",
                        label,
                        want_params.len(),
                        params.len()
                    ));
                }
                self.scopes.push(HashMap::new());
                let body_ty = (|| -> Result<Type, String> {
                    for (p, want) in params.iter().zip(want_params.iter()) {
                        let bind_ty = match &p.ty {
                            Some(ann) if type_compatible(want, ann) => ann.clone(),
                            Some(ann) => {
                                return Err(format!(
                                    "typeck: {} closure param is {:?}, expected {:?}",
                                    label, ann, want
                                ));
                            }
                            None => want.clone(),
                        };
                        self.check_pattern(&p.pat, &bind_ty)?;
                    }
                    self.type_expr(body)
                })();
                self.scopes.pop();
                let body_ty = body_ty?;
                let out_ty = match ret {
                    Some(ann) => {
                        if !type_compatible(&body_ty, ann) {
                            return Err(format!(
                                "typeck: {} closure returns {:?}, annotated {:?}",
                                label, body_ty, ann
                            ));
                        }
                        ann.clone()
                    }
                    None => body_ty,
                };
                if !type_compatible(&out_ty, want_ret) {
                    return Err(format!(
                        "typeck: {} closure returns {:?}, expected {:?}",
                        label, out_ty, want_ret
                    ));
                }
                Ok(Type::Closure {
                    params: want_params.clone(),
                    ret: Box::new(out_ty),
                })
            }
            _ => self.type_expr(expr),
        }
    }

    fn check_expected_closure(
        &mut self,
        label: &str,
        arg: &Expr,
        param_ty: Type,
        ret_ty: Type,
    ) -> Result<(), String> {
        match arg {
            Expr::Closure { params, ret, body } => {
                if params.len() != 1 {
                    return Err(format!(
                        "typeck: {} closure expects 1 param, got {}",
                        label,
                        params.len()
                    ));
                }
                self.scopes.push(HashMap::new());
                let body_ty = (|| -> Result<Type, String> {
                    let p = &params[0];
                    let bind_ty = match &p.ty {
                        Some(ann) if type_compatible(&param_ty, ann) => ann.clone(),
                        Some(ann) => {
                            return Err(format!(
                                "typeck: {} closure param is {:?}, expected {:?}",
                                label, ann, param_ty
                            ));
                        }
                        None => param_ty.clone(),
                    };
                    self.check_pattern(&p.pat, &bind_ty)?;
                    self.type_expr(body)
                })();
                self.scopes.pop();
                let body_ty = body_ty?;
                if let Some(ann) = ret {
                    if !type_compatible(&body_ty, ann) {
                        return Err(format!(
                            "typeck: {} closure returns {:?}, annotated {:?}",
                            label, body_ty, ann
                        ));
                    }
                }
                if !type_compatible(&body_ty, &ret_ty) {
                    return Err(format!(
                        "typeck: {} closure returns {:?}, expected {:?}",
                        label, body_ty, ret_ty
                    ));
                }
                Ok(())
            }
            other => match self.type_expr(other)? {
                Type::Closure { params, ret } if params.len() == 1 => {
                    if !type_compatible(&param_ty, &params[0]) {
                        return Err(format!(
                            "typeck: {} closure param is {:?}, expected {:?}",
                            label, params[0], param_ty
                        ));
                    }
                    if !type_compatible(&ret, &ret_ty) {
                        return Err(format!(
                            "typeck: {} closure returns {:?}, expected {:?}",
                            label, ret, ret_ty
                        ));
                    }
                    Ok(())
                }
                Type::Closure { params, .. } => Err(format!(
                    "typeck: {} closure expects 1 param, got {}",
                    label,
                    params.len()
                )),
                other => Err(format!(
                    "typeck: {} expected closure, got {:?}",
                    label, other
                )),
            },
        }
    }

    fn infer_expected_closure(
        &mut self,
        label: &str,
        arg: &Expr,
        param_ty: Type,
    ) -> Result<Type, String> {
        match arg {
            Expr::Closure { params, ret, body } => {
                if params.len() != 1 {
                    return Err(format!(
                        "typeck: {} closure expects 1 param, got {}",
                        label,
                        params.len()
                    ));
                }
                self.scopes.push(HashMap::new());
                let body_ty = (|| -> Result<Type, String> {
                    let p = &params[0];
                    let bind_ty = match &p.ty {
                        Some(ann) if type_compatible(&param_ty, ann) => ann.clone(),
                        Some(ann) => {
                            return Err(format!(
                                "typeck: {} closure param is {:?}, expected {:?}",
                                label, ann, param_ty
                            ));
                        }
                        None => param_ty.clone(),
                    };
                    self.check_pattern(&p.pat, &bind_ty)?;
                    self.type_expr(body)
                })();
                self.scopes.pop();
                let body_ty = body_ty?;
                if let Some(ann) = ret {
                    if !type_compatible(&body_ty, ann) {
                        return Err(format!(
                            "typeck: {} closure returns {:?}, annotated {:?}",
                            label, body_ty, ann
                        ));
                    }
                    Ok(ann.clone())
                } else {
                    Ok(body_ty)
                }
            }
            other => match self.type_expr(other)? {
                Type::Closure { params, ret } if params.len() == 1 => {
                    if !type_compatible(&param_ty, &params[0]) {
                        return Err(format!(
                            "typeck: {} closure param is {:?}, expected {:?}",
                            label, params[0], param_ty
                        ));
                    }
                    Ok(*ret)
                }
                Type::Closure { params, .. } => Err(format!(
                    "typeck: {} closure expects 1 param, got {}",
                    label,
                    params.len()
                )),
                other => Err(format!(
                    "typeck: {} expected closure, got {:?}",
                    label, other
                )),
            },
        }
    }

    fn infer_zero_arg_closure(&mut self, label: &str, arg: &Expr) -> Result<Type, String> {
        match arg {
            Expr::Closure { params, ret, body } => {
                if !params.is_empty() {
                    return Err(format!(
                        "typeck: {} closure expects 0 params, got {}",
                        label,
                        params.len()
                    ));
                }
                let body_ty = self.type_expr(body)?;
                if let Some(ann) = ret {
                    if !type_compatible(&body_ty, ann) {
                        return Err(format!(
                            "typeck: {} closure returns {:?}, annotated {:?}",
                            label, body_ty, ann
                        ));
                    }
                    Ok(ann.clone())
                } else {
                    Ok(body_ty)
                }
            }
            other => match self.type_expr(other)? {
                Type::Closure { params, ret } if params.is_empty() => Ok(*ret),
                Type::Closure { params, .. } => Err(format!(
                    "typeck: {} closure expects 0 params, got {}",
                    label,
                    params.len()
                )),
                other => Err(format!(
                    "typeck: {} expected closure, got {:?}",
                    label, other
                )),
            },
        }
    }

    fn type_option_method(
        &mut self,
        name: &str,
        receiver_ty: &Type,
        args: &[Expr],
    ) -> Result<Type, String> {
        let elem = match deref_type(receiver_ty.clone()) {
            Type::Generic { name, args } if name == "Option" && args.len() == 1 => args[0].clone(),
            Type::Generic { name, args } if name == "Option" && args.is_empty() => Type::Unit,
            other => return Err(format!("typeck: Option method on {:?}", other)),
        };
        match name {
            "unwrap" => {
                if !args.is_empty() {
                    return Err("typeck: Option::unwrap expects 0 args".to_string());
                }
                Ok(elem)
            }
            "unwrap_or" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Option::unwrap_or expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let default_ty = self.type_expr(&args[0])?;
                if elem == Type::Unit {
                    Ok(default_ty)
                } else if type_compatible(&default_ty, &elem) {
                    Ok(elem)
                } else {
                    Err(format!(
                        "typeck: Option::unwrap_or default is {:?}, expected {:?}",
                        default_ty, elem
                    ))
                }
            }
            "unwrap_or_else" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Option::unwrap_or_else expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let default_ty = self.infer_zero_arg_closure("Option::unwrap_or_else", &args[0])?;
                if elem == Type::Unit {
                    Ok(default_ty)
                } else if type_compatible(&default_ty, &elem) {
                    Ok(elem)
                } else {
                    Err(format!(
                        "typeck: Option::unwrap_or_else default is {:?}, expected {:?}",
                        default_ty, elem
                    ))
                }
            }
            "is_some" | "is_none" => {
                if !args.is_empty() {
                    return Err(format!("typeck: Option::{} expects 0 args", name));
                }
                Ok(Type::Bool)
            }
            "ok_or_else" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Option::ok_or_else expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let err_ty = self.infer_zero_arg_closure("Option::ok_or_else", &args[0])?;
                Ok(Type::Generic {
                    name: "Result".to_string(),
                    args: vec![elem, err_ty],
                })
            }
            "map_or" => {
                if args.len() != 2 {
                    return Err(format!(
                        "typeck: Option::map_or expects 2 args (default, f), got {}",
                        args.len()
                    ));
                }
                let default_ty = self.type_expr(&args[0])?;
                let mapped =
                    self.infer_expected_closure("Option::map_or", &args[1], elem.clone())?;
                if elem == Type::Unit
                    || type_compatible(&default_ty, &mapped)
                    || type_compatible(&mapped, &default_ty)
                {
                    Ok(mapped)
                } else {
                    Err(format!(
                        "typeck: Option::map_or default {:?} vs closure result {:?}",
                        default_ty, mapped
                    ))
                }
            }
            "ok_or" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Option::ok_or expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let err_ty = self.type_expr(&args[0])?;
                Ok(Type::Generic {
                    name: "Result".to_string(),
                    args: vec![elem, err_ty],
                })
            }
            "is_some_and" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Option::is_some_and expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let _ = self.infer_expected_closure("Option::is_some_and", &args[0], elem.clone())?;
                Ok(Type::Bool)
            }
            "map" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Option::map expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let mapped = self.infer_expected_closure("Option::map", &args[0], elem)?;
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![mapped],
                })
            }
            "and_then" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Option::and_then expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let out = self.infer_expected_closure("Option::and_then", &args[0], elem)?;
                match deref_type(out.clone()) {
                    Type::Generic { name, args } if name == "Option" && args.len() == 1 => Ok(out),
                    other => Err(format!(
                        "typeck: Option::and_then closure returns {:?}",
                        other
                    )),
                }
            }
            "or_else" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Option::or_else expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let out = self.infer_zero_arg_closure("Option::or_else", &args[0])?;
                match deref_type(out.clone()) {
                    Type::Generic { name, args } if name == "Option" && args.len() == 1 => {
                        if elem == Type::Unit || type_compatible(&args[0], &elem) {
                            Ok(Type::Generic {
                                name: "Option".to_string(),
                                args: vec![args[0].clone()],
                            })
                        } else {
                            Err(format!(
                                "typeck: Option::or_else closure returns Option<{:?}>, expected Option<{:?}>",
                                args[0], elem
                            ))
                        }
                    }
                    other => Err(format!(
                        "typeck: Option::or_else closure returns {:?}",
                        other
                    )),
                }
            }
            "as_ref" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: Option::as_ref expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(elem),
                    }],
                })
            }
            "copied" | "cloned" => {
                if !args.is_empty() {
                    return Err(format!("typeck: Option::{} expects 0 args", name));
                }
                match elem {
                    Type::Ref { inner, .. } => Ok(Type::Generic {
                        name: "Option".to_string(),
                        args: vec![*inner],
                    }),
                    other => Err(format!("typeck: Option::{} on Option<{:?}>", name, other)),
                }
            }
            other => Err(format!("typeck: unsupported Option method {}", other)),
        }
    }

    fn type_result_method(
        &mut self,
        name: &str,
        receiver_ty: &Type,
        args: &[Expr],
    ) -> Result<Type, String> {
        let (ok_ty, err_ty) = match deref_type(receiver_ty.clone()) {
            Type::Generic { name, args } if name == "Result" && args.len() == 2 => {
                (args[0].clone(), args[1].clone())
            }
            other => return Err(format!("typeck: Result method on {:?}", other)),
        };
        match name {
            "unwrap" => {
                if !args.is_empty() {
                    return Err("typeck: Result::unwrap expects 0 args".to_string());
                }
                Ok(ok_ty)
            }
            "unwrap_or" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Result::unwrap_or expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let default_ty = self.type_expr(&args[0])?;
                if ok_ty == Type::Unit {
                    Ok(default_ty)
                } else if type_compatible(&default_ty, &ok_ty) {
                    Ok(ok_ty)
                } else {
                    Err(format!(
                        "typeck: Result::unwrap_or default is {:?}, expected {:?}",
                        default_ty, ok_ty
                    ))
                }
            }
            "is_ok" | "is_err" => {
                if !args.is_empty() {
                    return Err(format!("typeck: Result::{} expects 0 args", name));
                }
                Ok(Type::Bool)
            }
            "ok" => {
                if !args.is_empty() {
                    return Err("typeck: Result::ok expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![ok_ty],
                })
            }
            "map_err" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Result::map_err expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let new_err = self.infer_expected_closure("Result::map_err", &args[0], err_ty)?;
                Ok(Type::Generic {
                    name: "Result".to_string(),
                    args: vec![ok_ty, new_err],
                })
            }
            "map" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Result::map expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let new_ok = self.infer_expected_closure("Result::map", &args[0], ok_ty)?;
                Ok(Type::Generic {
                    name: "Result".to_string(),
                    args: vec![new_ok, err_ty],
                })
            }
            other => Err(format!("typeck: unsupported Result method {}", other)),
        }
    }

    fn type_refcell_method(
        &mut self,
        name: &str,
        receiver_ty: &Type,
        args: &[Expr],
    ) -> Result<Type, String> {
        let inner = match deref_type(receiver_ty.clone()) {
            Type::Generic { name, args } if name == "RefCell" && args.len() == 1 => args[0].clone(),
            other => return Err(format!("typeck: RefCell method on {:?}", other)),
        };
        match name {
            "borrow" | "borrow_mut" => {
                if !args.is_empty() {
                    return Err(format!("typeck: RefCell::{} expects 0 args", name));
                }
                Ok(Type::Ref {
                    mutable: name == "borrow_mut",
                    inner: Box::new(inner),
                })
            }
            "into_inner" => {
                if !args.is_empty() {
                    return Err("typeck: RefCell::into_inner expects 0 args".to_string());
                }
                Ok(inner)
            }
            other => Err(format!("typeck: unsupported RefCell method {}", other)),
        }
    }

    fn type_boxlike_method(
        &mut self,
        target: &str,
        name: &str,
        receiver_ty: &Type,
        args: &[Expr],
    ) -> Result<Type, String> {
        match name {
            "to_string" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: {}::to_string expects 0 args, got {}",
                        target,
                        args.len()
                    ));
                }
                match deref_type(receiver_ty.clone()) {
                    Type::Generic {
                        name: container_name,
                        args: outer_args,
                    } if container_name == target
                        && outer_args.len() == 1
                        && is_displayable_type(&outer_args[0]) =>
                    {
                        Ok(Type::Named("String".to_string()))
                    }
                    Type::Generic {
                        name: container_name,
                        args: outer_args,
                    } if container_name == target && outer_args.len() == 1 => Err(format!(
                        "typeck: {}::to_string on non-display {:?}",
                        target, outer_args[0]
                    )),
                    other => Err(format!("typeck: {}::to_string on {:?}", target, other)),
                }
            }
            "as_str" => {
                if !args.is_empty() {
                    return Err(format!("typeck: {}::as_str expects 0 args", target));
                }
                match deref_type(receiver_ty.clone()) {
                    Type::Generic {
                        name: container_name,
                        args: outer_args,
                    } if container_name == target && outer_args.len() == 1 => {
                        if outer_args[0] == Type::Named("String".to_string()) {
                            Ok(Type::Ref {
                                mutable: false,
                                inner: Box::new(Type::Named("str".to_string())),
                            })
                        } else {
                            Err(format!("typeck: {}::as_str on {:?}", target, outer_args[0]))
                        }
                    }
                    other => Err(format!("typeck: {}::as_str on {:?}", target, other)),
                }
            }
            "chars" => {
                if !args.is_empty() {
                    return Err(format!("typeck: {}::chars expects 0 args", target));
                }
                match deref_type(receiver_ty.clone()) {
                    Type::Generic {
                        name: container_name,
                        args: outer_args,
                    } if container_name == target && outer_args.len() == 1 => {
                        if outer_args[0] == Type::Named("String".to_string()) {
                            Ok(Type::Generic {
                                name: "Iter".to_string(),
                                args: vec![Type::Char],
                            })
                        } else {
                            Err(format!("typeck: {}::chars on {:?}", target, outer_args[0]))
                        }
                    }
                    other => Err(format!("typeck: {}::chars on {:?}", target, other)),
                }
            }
            "get" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: {}::get expects 1 arg, got {}",
                        target,
                        args.len()
                    ));
                }
                let index_ty = self.type_expr(&args[0])?;
                if !is_integer(&index_ty) {
                    return Err(format!(
                        "typeck: {}::get index expected integer, got {:?}",
                        target, index_ty
                    ));
                }
                match deref_type(receiver_ty.clone()) {
                    Type::Generic {
                        name: container_name,
                        args: outer_args,
                    } if container_name == target && outer_args.len() == 1 => {
                        match &outer_args[0] {
                            Type::Generic {
                                name: inner_name,
                                args: inner_args,
                            } if inner_name == "Vec" && inner_args.len() == 1 => {
                                Ok(Type::Generic {
                                    name: "Option".to_string(),
                                    args: vec![Type::Ref {
                                        mutable: false,
                                        inner: Box::new(inner_args[0].clone()),
                                    }],
                                })
                            }
                            other => Err(format!("typeck: {}::get on {:?}", target, other)),
                        }
                    }
                    other => Err(format!("typeck: {}::get on {:?}", target, other)),
                }
            }
            "len" | "is_empty" | "iter" => {
                if !args.is_empty() {
                    return Err(format!("typeck: {}::{} expects 0 args", target, name));
                }
                match deref_type(receiver_ty.clone()) {
                    Type::Generic {
                        name: container_name,
                        args: outer_args,
                    } if container_name == target && outer_args.len() == 1 => {
                        match &outer_args[0] {
                            Type::Generic {
                                name: inner_name,
                                args: inner_args,
                            } if inner_name == "Vec" && inner_args.len() == 1 => {
                                if name == "len" {
                                    Ok(Type::I64)
                                } else if name == "iter" {
                                    Ok(Type::Generic {
                                        name: "Iter".to_string(),
                                        args: vec![Type::Ref {
                                            mutable: false,
                                            inner: Box::new(inner_args[0].clone()),
                                        }],
                                    })
                                } else {
                                    Ok(Type::Bool)
                                }
                            }
                            other => Err(format!("typeck: {}::{} on {:?}", target, name, other)),
                        }
                    }
                    other => Err(format!("typeck: {}::{} on {:?}", target, name, other)),
                }
            }
            "borrow" | "borrow_mut" => {
                if !args.is_empty() {
                    return Err(format!("typeck: {}::{} expects 0 args", target, name));
                }
                match deref_type(receiver_ty.clone()) {
                    Type::Generic {
                        name: container_name,
                        args: outer_args,
                    } if container_name == target && outer_args.len() == 1 => {
                        match &outer_args[0] {
                            Type::Generic {
                                name: inner_name,
                                args: inner_args,
                            } if inner_name == "RefCell" && inner_args.len() == 1 => {
                                Ok(Type::Ref {
                                    mutable: name == "borrow_mut",
                                    inner: Box::new(inner_args[0].clone()),
                                })
                            }
                            other => Err(format!("typeck: {}::{} on {:?}", target, name, other)),
                        }
                    }
                    other => Err(format!("typeck: {}::{} on {:?}", target, name, other)),
                }
            }
            "as_ref" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: {}::as_ref expects 0 args, got {}",
                        target,
                        args.len()
                    ));
                }
                match deref_type(receiver_ty.clone()) {
                    Type::Generic { name, args } if name == target && args.len() == 1 => {
                        Ok(Type::Ref {
                            mutable: false,
                            inner: Box::new(args[0].clone()),
                        })
                    }
                    other => Err(format!("typeck: {}::as_ref on {:?}", target, other)),
                }
            }
            other => Err(format!("typeck: unsupported {} method {}", target, other)),
        }
    }

    fn type_hashmap_method(
        &mut self,
        name: &str,
        receiver: &Expr,
        receiver_ty: &Type,
        args: &[Expr],
    ) -> Result<Type, String> {
        let (key_ty, val_ty) = match deref_type(receiver_ty.clone()) {
            Type::Generic { name, args } if name == "HashMap" && args.len() == 2 => {
                (args[0].clone(), args[1].clone())
            }
            Type::Generic { name, args } if name == "HashMap" && args.is_empty() => {
                (Type::Unit, Type::Unit)
            }
            other => return Err(format!("typeck: HashMap method on {:?}", other)),
        };
        match name {
            "insert" => {
                self.require_mut_receiver(receiver, receiver_ty, "HashMap::insert")?;
                if args.len() != 2 {
                    return Err(format!(
                        "typeck: HashMap::insert expects 2 args, got {}",
                        args.len()
                    ));
                }
                let got_key = self.type_expr(&args[0])?;
                let got_val = self.type_expr(&args[1])?;
                if key_ty != Type::Unit && !type_compatible(&got_key, &key_ty) {
                    return Err(format!(
                        "typeck: HashMap::insert key is {:?}, expected {:?}",
                        got_key, key_ty
                    ));
                }
                if val_ty != Type::Unit && !type_compatible(&got_val, &val_ty) {
                    return Err(format!(
                        "typeck: HashMap::insert value is {:?}, expected {:?}",
                        got_val, val_ty
                    ));
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![val_ty],
                })
            }
            "contains_key" => {
                self.check_hash_key_arg(name, args, &key_ty)?;
                Ok(Type::Bool)
            }
            "get" => {
                self.check_hash_key_arg(name, args, &key_ty)?;
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(val_ty),
                    }],
                })
            }
            "get_mut" => {
                self.require_mut_receiver(receiver, receiver_ty, "HashMap::get_mut")?;
                self.check_hash_key_arg(name, args, &key_ty)?;
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Ref {
                        mutable: true,
                        inner: Box::new(val_ty),
                    }],
                })
            }
            "remove" => {
                self.require_mut_receiver(receiver, receiver_ty, "HashMap::remove")?;
                self.check_hash_key_arg(name, args, &key_ty)?;
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![val_ty],
                })
            }
            "len" => {
                if !args.is_empty() {
                    return Err("typeck: HashMap::len expects 0 args".to_string());
                }
                Ok(Type::I64)
            }
            "is_empty" => {
                if !args.is_empty() {
                    return Err("typeck: HashMap::is_empty expects 0 args".to_string());
                }
                Ok(Type::Bool)
            }
            "keys" | "values" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: HashMap::{} expects 0 args, got {}",
                        name,
                        args.len()
                    ));
                }
                let inner = if name == "keys" { key_ty } else { val_ty };
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(inner),
                    }],
                })
            }
            "iter" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: HashMap::iter expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Tuple(vec![
                        Type::Ref {
                            mutable: false,
                            inner: Box::new(key_ty),
                        },
                        Type::Ref {
                            mutable: false,
                            inner: Box::new(val_ty),
                        },
                    ])],
                })
            }
            "entry" => {
                self.require_mut_receiver(receiver, receiver_ty, "HashMap::entry")?;
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: HashMap::entry expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let got_key = self.type_expr(&args[0])?;
                if key_ty != Type::Unit && !type_compatible(&got_key, &key_ty) {
                    return Err(format!(
                        "typeck: HashMap::entry key is {:?}, expected {:?}",
                        got_key, key_ty
                    ));
                }
                Ok(Type::Generic {
                    name: "HashEntry".to_string(),
                    args: vec![key_ty, val_ty],
                })
            }
            other => Err(format!("typeck: unsupported HashMap method {}", other)),
        }
    }

    fn type_hashentry_method(
        &mut self,
        name: &str,
        receiver_ty: &Type,
        args: &[Expr],
    ) -> Result<Type, String> {
        let (_key_ty, val_ty) = match deref_type(receiver_ty.clone()) {
            Type::Generic { name, args } if name == "HashEntry" && args.len() == 2 => {
                (args[0].clone(), args[1].clone())
            }
            other => return Err(format!("typeck: HashEntry method on {:?}", other)),
        };
        match name {
            "or_insert" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: HashEntry::or_insert expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let got = self.type_expr(&args[0])?;
                if !type_compatible(&got, &val_ty) {
                    return Err(format!(
                        "typeck: HashEntry::or_insert value is {:?}, expected {:?}",
                        got, val_ty
                    ));
                }
                Ok(Type::Ref {
                    mutable: true,
                    inner: Box::new(val_ty),
                })
            }
            "or_insert_with" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: HashEntry::or_insert_with expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let got = self.infer_zero_arg_closure("HashEntry::or_insert_with", &args[0])?;
                if !type_compatible(&got, &val_ty) {
                    return Err(format!(
                        "typeck: HashEntry::or_insert_with closure returns {:?}, expected {:?}",
                        got, val_ty
                    ));
                }
                Ok(Type::Ref {
                    mutable: true,
                    inner: Box::new(val_ty),
                })
            }
            "and_modify" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: HashEntry::and_modify expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let expected = Type::Closure {
                    params: vec![Type::Ref {
                        mutable: true,
                        inner: Box::new(val_ty.clone()),
                    }],
                    ret: Box::new(Type::Unit),
                };
                let got =
                    self.type_expr_against(&args[0], &expected, "HashEntry::and_modify closure")?;
                if !type_compatible(&got, &expected) {
                    return Err(format!(
                        "typeck: HashEntry::and_modify closure is {:?}, expected {:?}",
                        got, expected
                    ));
                }
                Ok(receiver_ty.clone())
            }
            other => Err(format!("typeck: unsupported HashEntry method {}", other)),
        }
    }

    fn check_hash_key_arg(
        &mut self,
        name: &str,
        args: &[Expr],
        key_ty: &Type,
    ) -> Result<(), String> {
        if args.len() != 1 {
            return Err(format!(
                "typeck: HashMap::{} expects 1 arg, got {}",
                name,
                args.len()
            ));
        }
        let got = self.type_expr(&args[0])?;
        let want = Type::Ref {
            mutable: false,
            inner: Box::new(key_ty.clone()),
        };
        if key_ty != &Type::Unit
            && !type_compatible(&got, &want)
            && !hashmap_lookup_key_compatible(&got, key_ty)
        {
            return Err(format!(
                "typeck: HashMap::{} key arg is {:?}, expected {:?}",
                name, got, want
            ));
        }
        Ok(())
    }

    fn type_string_method(
        &mut self,
        target: &str,
        name: &str,
        receiver: &Expr,
        receiver_ty: &Type,
        args: &[Expr],
    ) -> Result<Type, String> {
        match name {
            "push_str" if target == "String" => {
                self.require_mut_receiver(receiver, receiver_ty, "String::push_str")?;
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: String::push_str expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let got = self.type_expr(&args[0])?;
                expect_str_like(&got, "String::push_str")?;
                Ok(Type::Unit)
            }
            "push" if target == "String" => {
                self.require_mut_receiver(receiver, receiver_ty, "String::push")?;
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: String::push expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let got = self.type_expr(&args[0])?;
                expect(&got, &Type::Char, "String::push")?;
                Ok(Type::Unit)
            }
            "len" => {
                if !args.is_empty() {
                    return Err("typeck: string len expects 0 args".to_string());
                }
                Ok(Type::I64)
            }
            "is_empty" => {
                if !args.is_empty() {
                    return Err("typeck: string is_empty expects 0 args".to_string());
                }
                Ok(Type::Bool)
            }
            "as_str" if target == "String" => {
                if !args.is_empty() {
                    return Err("typeck: String::as_str expects 0 args".to_string());
                }
                Ok(Type::Ref {
                    mutable: false,
                    inner: Box::new(Type::Named("str".to_string())),
                })
            }
            "trim" => {
                if !args.is_empty() {
                    return Err("typeck: string trim expects 0 args".to_string());
                }
                Ok(Type::Ref {
                    mutable: false,
                    inner: Box::new(Type::Named("str".to_string())),
                })
            }
            "repeat" => {
                if args.len() != 1 {
                    return Err("typeck: str::repeat expects 1 arg".to_string());
                }
                let n = self.type_expr(&args[0])?;
                if !is_integer(&n) {
                    return Err(format!("typeck: str::repeat count is {:?}, expected integer", n));
                }
                Ok(Type::Named("String".to_string()))
            }
            "to_uppercase" | "to_lowercase" => {
                if !args.is_empty() {
                    return Err(format!("typeck: str::{} expects 0 args", name));
                }
                Ok(Type::Named("String".to_string()))
            }
            "to_string" => {
                if !args.is_empty() {
                    return Err("typeck: string to_string expects 0 args".to_string());
                }
                Ok(Type::Named("String".to_string()))
            }
            "chars" => {
                if !args.is_empty() {
                    return Err("typeck: string chars expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Char],
                })
            }
            "bytes" => {
                if !args.is_empty() {
                    return Err("typeck: string bytes expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::U8],
                })
            }
            "as_bytes" => {
                if !args.is_empty() {
                    return Err("typeck: string as_bytes expects 0 args".to_string());
                }
                Ok(Type::Ref {
                    mutable: false,
                    inner: Box::new(Type::Slice(Box::new(Type::U8))),
                })
            }
            "split" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: string split expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let sep = self.type_expr(&args[0])?;
                expect_str_like(&sep, "string split separator")?;
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(Type::Named("str".to_string())),
                    }],
                })
            }
            "contains" | "starts_with" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: string {} expects 1 arg, got {}",
                        name,
                        args.len()
                    ));
                }
                let got = self.type_expr(&args[0])?;
                expect_str_like(&got, name)?;
                Ok(Type::Bool)
            }
            "find" => {
                if args.len() != 1 {
                    return Err("typeck: string find expects 1 arg".to_string());
                }
                let got = self.type_expr(&args[0])?;
                expect_str_like(&got, name)?;
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Usize],
                })
            }
            "split_whitespace" | "lines" => {
                if !args.is_empty() {
                    return Err(format!("typeck: str::{} expects 0 args", name));
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![str_ref_type()],
                })
            }
            "trim_start" | "trim_end" => {
                if !args.is_empty() {
                    return Err(format!("typeck: str::{} expects 0 args", name));
                }
                Ok(str_ref_type())
            }
            "replace" => {
                if args.len() != 2 {
                    return Err("typeck: str::replace expects 2 args".to_string());
                }
                expect_str_like(&self.type_expr(&args[0])?, "str::replace from")?;
                expect_str_like(&self.type_expr(&args[1])?, "str::replace to")?;
                Ok(Type::Named("String".to_string()))
            }
            "splitn" => {
                if args.len() != 2 {
                    return Err("typeck: str::splitn expects 2 args".to_string());
                }
                if !is_integer(&self.type_expr(&args[0])?) {
                    return Err("typeck: str::splitn first arg must be an integer".to_string());
                }
                expect_str_like(&self.type_expr(&args[1])?, "str::splitn separator")?;
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![str_ref_type()],
                })
            }
            "strip_prefix" | "strip_suffix" => {
                if args.len() != 1 {
                    return Err(format!("typeck: str::{} expects 1 arg", name));
                }
                expect_str_like(&self.type_expr(&args[0])?, name)?;
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![str_ref_type()],
                })
            }
            "split_once" => {
                if args.len() != 1 {
                    return Err("typeck: str::split_once expects 1 arg".to_string());
                }
                expect_str_like(&self.type_expr(&args[0])?, "str::split_once separator")?;
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Tuple(vec![str_ref_type(), str_ref_type()])],
                })
            }
            "char_indices" => {
                if !args.is_empty() {
                    return Err("typeck: str::char_indices expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Tuple(vec![Type::Usize, Type::Char])],
                })
            }
            other => Err(format!("typeck: unsupported {} method {}", target, other)),
        }
    }

    fn type_slice_method(
        &mut self,
        name: &str,
        receiver_ty: &Type,
        args: &[Expr],
    ) -> Result<Type, String> {
        let elem = match deref_type(receiver_ty.clone()) {
            Type::Slice(inner) => *inner,
            other => return Err(format!("typeck: slice method on {:?}", other)),
        };
        match name {
            "len" => {
                if !args.is_empty() {
                    return Err("typeck: slice len expects 0 args".to_string());
                }
                Ok(Type::I64)
            }
            "is_empty" => {
                if !args.is_empty() {
                    return Err("typeck: slice is_empty expects 0 args".to_string());
                }
                Ok(Type::Bool)
            }
            "get" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: slice get expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let index_ty = self.type_expr(&args[0])?;
                if !is_integer(&index_ty) {
                    return Err(format!(
                        "typeck: slice get index expected integer, got {:?}",
                        index_ty
                    ));
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(elem),
                    }],
                })
            }
            "first" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: slice first expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(elem),
                    }],
                })
            }
            "iter" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: slice iter expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(elem),
                    }],
                })
            }
            "to_vec" => {
                if !args.is_empty() {
                    return Err("typeck: slice to_vec expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Vec".to_string(),
                    args: vec![elem],
                })
            }
            other => Err(format!("typeck: unsupported slice method {}", other)),
        }
    }

    fn type_char_method(&mut self, name: &str, args: &[Expr]) -> Result<Type, String> {
        if !args.is_empty() {
            return Err(format!("typeck: char::{} expects 0 args", name));
        }
        match name {
            "is_whitespace"
            | "is_ascii_digit"
            | "is_ascii_hexdigit"
            | "is_ascii_alphabetic"
            | "is_ascii_alphanumeric"
            | "is_alphabetic"
            | "is_numeric"
            | "is_alphanumeric"
            | "is_uppercase"
            | "is_lowercase" => Ok(Type::Bool),
            "to_ascii_uppercase" | "to_ascii_lowercase" => Ok(Type::Char),
            "len_utf8" => Ok(Type::Usize),
            "to_string" => Ok(Type::Named("String".to_string())),
            other => Err(format!("typeck: unsupported char method {}", other)),
        }
    }

    fn type_path_method(
        &mut self,
        target: &str,
        name: &str,
        args: &[Expr],
    ) -> Result<Type, String> {
        match name {
            "join" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: {}::join expects 1 arg, got {}",
                        target,
                        args.len()
                    ));
                }
                self.type_expr(&args[0])?;
                Ok(Type::Named("PathBuf".to_string()))
            }
            "display" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: {}::display expects 0 args, got {}",
                        target,
                        args.len()
                    ));
                }
                Ok(Type::Named("String".to_string()))
            }
            "exists" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: {}::exists expects 0 args, got {}",
                        target,
                        args.len()
                    ));
                }
                Ok(Type::Bool)
            }
            other => Err(format!("typeck: unsupported {} method {}", target, other)),
        }
    }

    fn type_command_method(&mut self, name: &str, args: &[Expr]) -> Result<Type, String> {
        match name {
            "arg" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Command::arg expects 1 arg, got {}",
                        args.len()
                    ));
                }
                self.type_expr(&args[0])?;
                Ok(Type::Named("Command".to_string()))
            }
            "env" => {
                if args.len() != 2 {
                    return Err(format!(
                        "typeck: Command::env expects 2 args, got {}",
                        args.len()
                    ));
                }
                let key = self.type_expr(&args[0])?;
                expect_str_like(&key, "Command::env key")?;
                let value = self.type_expr(&args[1])?;
                expect_str_like(&value, "Command::env value")?;
                Ok(Type::Named("Command".to_string()))
            }
            "env_clear" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: Command::env_clear expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Named("Command".to_string()))
            }
            "output" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: Command::output expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Generic {
                    name: "Result".to_string(),
                    args: vec![
                        Type::Named("Output".to_string()),
                        Type::Named("String".to_string()),
                    ],
                })
            }
            other => Err(format!("typeck: unsupported Command method {}", other)),
        }
    }

    fn type_exit_status_method(&mut self, name: &str, args: &[Expr]) -> Result<Type, String> {
        match name {
            "success" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: ExitStatus::success expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Bool)
            }
            other => Err(format!("typeck: unsupported ExitStatus method {}", other)),
        }
    }

    fn type_int_method(
        &mut self,
        target: &str,
        name: &str,
        receiver_ty: &Type,
        args: &[Expr],
    ) -> Result<Type, String> {
        match name {
            "to_string" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: {}::to_string expects 0 args, got {}",
                        target,
                        args.len()
                    ));
                }
                Ok(Type::Named("String".to_string()))
            }
            "wrapping_neg" if target == "i64" || target == "i32" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: {}::wrapping_neg expects 0 args, got {}",
                        target,
                        args.len()
                    ));
                }
                Ok(receiver_ty.clone())
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
                    return Err(format!(
                        "typeck: {}::{} expects 1 arg, got {}",
                        target,
                        name,
                        args.len()
                    ));
                }
                let arg_ty = self.type_expr(&args[0])?;
                if !type_compatible(&arg_ty, receiver_ty) {
                    return Err(format!(
                        "typeck: {}::{} arg is {:?}, expected {:?}",
                        target, name, arg_ty, receiver_ty
                    ));
                }
                Ok(receiver_ty.clone())
            }
            "saturating_sub" if target == "usize" || target == "i64" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: {}::saturating_sub expects 1 arg, got {}",
                        target,
                        args.len()
                    ));
                }
                let arg_ty = self.type_expr(&args[0])?;
                if !type_compatible(&arg_ty, receiver_ty) {
                    return Err(format!(
                        "typeck: {}::saturating_sub arg is {:?}, expected {:?}",
                        target, arg_ty, receiver_ty
                    ));
                }
                Ok(receiver_ty.clone())
            }
            "pow" if is_int_target(target) => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: {}::pow expects 1 arg, got {}",
                        target, args.len()
                    ));
                }
                let arg_ty = self.type_expr(&args[0])?;
                if !is_integer(&arg_ty) {
                    return Err(format!(
                        "typeck: {}::pow exponent is {:?}, expected an integer",
                        target, arg_ty
                    ));
                }
                Ok(receiver_ty.clone())
            }
            "signum" if is_int_target(target) => {
                if !args.is_empty() {
                    return Err(format!("typeck: {}::signum expects 0 args", target));
                }
                Ok(receiver_ty.clone())
            }
            "rem_euclid" if is_int_target(target) => {
                if args.len() != 1 {
                    return Err(format!("typeck: {}::rem_euclid expects 1 arg", target));
                }
                let arg_ty = self.type_expr(&args[0])?;
                if !is_integer(&arg_ty) {
                    return Err(format!("typeck: {}::rem_euclid arg is {:?}", target, arg_ty));
                }
                Ok(receiver_ty.clone())
            }
            // i64-gated (mirrors the interp): 64-bit overflow/bit semantics match
            // rustc only at that width.
            "abs" if target == "i64" => {
                if !args.is_empty() {
                    return Err(format!("typeck: {}::abs expects 0 args", target));
                }
                Ok(Type::I64)
            }
            "cmp" if is_int_target(target) => {
                if args.len() != 1 {
                    return Err(format!("typeck: {}::cmp expects 1 arg", target));
                }
                let arg = deref_type(self.type_expr(&args[0])?);
                if !is_integer(&arg) {
                    return Err(format!("typeck: {}::cmp arg is {:?}", target, arg));
                }
                Ok(Type::Named("Ordering".to_string()))
            }
            "checked_add" | "checked_sub" | "checked_mul" | "checked_div" | "checked_rem"
                if target == "i64" =>
            {
                if args.len() != 1 {
                    return Err(format!("typeck: {}::{} expects 1 arg", target, name));
                }
                let arg_ty = self.type_expr(&args[0])?;
                if !is_integer(&arg_ty) {
                    return Err(format!("typeck: {}::{} arg is {:?}", target, name, arg_ty));
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::I64],
                })
            }
            "saturating_add" if target == "i64" || target == "usize" => {
                if args.len() != 1 {
                    return Err(format!("typeck: {}::saturating_add expects 1 arg", target));
                }
                let arg_ty = self.type_expr(&args[0])?;
                if !is_integer(&arg_ty) {
                    return Err(format!(
                        "typeck: {}::saturating_add arg is {:?}",
                        target, arg_ty
                    ));
                }
                Ok(receiver_ty.clone())
            }
            "count_ones" | "leading_zeros" | "trailing_zeros"
                if target == "i64" || target == "usize" =>
            {
                if !args.is_empty() {
                    return Err(format!("typeck: {}::{} expects 0 args", target, name));
                }
                Ok(Type::U32)
            }
            "max" | "min" if target == "i64" || target == "usize" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: {}::{} expects 1 arg, got {}",
                        target,
                        name,
                        args.len()
                    ));
                }
                let arg_ty = self.type_expr(&args[0])?;
                if !type_compatible(&arg_ty, receiver_ty) {
                    return Err(format!(
                        "typeck: {}::{} arg is {:?}, expected {:?}",
                        target, name, arg_ty, receiver_ty
                    ));
                }
                Ok(receiver_ty.clone())
            }
            other => Err(format!(
                "typeck: unsupported integer method {}::{}",
                target, other
            )),
        }
    }

    fn type_bool_method(&mut self, name: &str, args: &[Expr]) -> Result<Type, String> {
        match name {
            "to_string" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: bool::to_string expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Named("String".to_string()))
            }
            "then" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: bool::then expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let out = self.infer_zero_arg_closure("bool::then", &args[0])?;
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![out],
                })
            }
            other => Err(format!("typeck: unsupported bool method {}", other)),
        }
    }

    fn type_vec_method(
        &mut self,
        name: &str,
        receiver: &Expr,
        receiver_ty: &Type,
        args: &[Expr],
    ) -> Result<Type, String> {
        let (elem, placeholder) = match deref_type(receiver_ty.clone()) {
            Type::Generic { name, args } if name == "Vec" && args.len() == 1 => {
                (args[0].clone(), false)
            }
            Type::Generic { name, args } if name == "Vec" && args.is_empty() => (Type::Unit, true),
            other => return Err(format!("typeck: Vec method on {:?}", other)),
        };
        match name {
            "push" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::push")?;
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Vec::push expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let got = self.type_expr(&args[0])?;
                if placeholder {
                    self.refine_vec_placeholder(receiver, got);
                } else if !type_compatible(&got, &elem) {
                    return Err(format!(
                        "typeck: Vec::push arg is {:?}, expected {:?}",
                        got, elem
                    ));
                }
                Ok(Type::Unit)
            }
            "len" => {
                if !args.is_empty() {
                    return Err("typeck: Vec::len expects 0 args".to_string());
                }
                Ok(Type::I64)
            }
            "is_empty" => {
                if !args.is_empty() {
                    return Err("typeck: Vec::is_empty expects 0 args".to_string());
                }
                Ok(Type::Bool)
            }
            "clear" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::clear")?;
                if !args.is_empty() {
                    return Err("typeck: Vec::clear expects 0 args".to_string());
                }
                Ok(Type::Unit)
            }
            "reverse" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::reverse")?;
                if !args.is_empty() {
                    return Err("typeck: Vec::reverse expects 0 args".to_string());
                }
                Ok(Type::Unit)
            }
            "sort" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::sort")?;
                if !args.is_empty() {
                    return Err("typeck: Vec::sort expects 0 args".to_string());
                }
                Ok(Type::Unit)
            }
            "dedup" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::dedup")?;
                if !args.is_empty() {
                    return Err("typeck: Vec::dedup expects 0 args".to_string());
                }
                Ok(Type::Unit)
            }
            "chunks" => {
                if args.len() != 1 {
                    return Err("typeck: Vec::chunks expects 1 arg".to_string());
                }
                let n_ty = self.type_expr(&args[0])?;
                if !is_integer(&n_ty) {
                    return Err(format!("typeck: Vec::chunks size is {:?}", n_ty));
                }
                let elem = vec_index_elem(receiver_ty)
                    .ok_or_else(|| format!("typeck: Vec::chunks on {:?}", receiver_ty))?;
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Generic {
                        name: "Vec".to_string(),
                        args: vec![elem],
                    }],
                })
            }
            "sort_by_key" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::sort_by_key")?;
                if args.len() != 1 {
                    return Err("typeck: Vec::sort_by_key expects 1 arg".to_string());
                }
                let elem = vec_index_elem(receiver_ty)
                    .ok_or_else(|| format!("typeck: Vec::sort_by_key on {:?}", receiver_ty))?;
                let key_ty = self.infer_expected_closure(
                    "Vec::sort_by_key",
                    &args[0],
                    Type::Ref {
                        mutable: false,
                        inner: Box::new(elem),
                    },
                )?;
                if !is_integer(&deref_type(key_ty.clone())) {
                    return Err(format!(
                        "typeck: Vec::sort_by_key key is {:?}, expected an integer",
                        key_ty
                    ));
                }
                Ok(Type::Unit)
            }
            "retain" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::retain")?;
                if args.len() != 1 {
                    return Err("typeck: Vec::retain expects 1 arg".to_string());
                }
                let elem = vec_index_elem(receiver_ty)
                    .ok_or_else(|| format!("typeck: Vec::retain on {:?}", receiver_ty))?;
                self.check_expected_closure(
                    "Vec::retain",
                    &args[0],
                    Type::Ref {
                        mutable: false,
                        inner: Box::new(elem),
                    },
                    Type::Bool,
                )?;
                Ok(Type::Unit)
            }
            "truncate" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::truncate")?;
                if args.len() != 1 {
                    return Err("typeck: Vec::truncate expects 1 arg".to_string());
                }
                self.type_expr(&args[0])?;
                Ok(Type::Unit)
            }
            "insert" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::insert")?;
                if args.len() != 2 {
                    return Err("typeck: Vec::insert expects 2 args".to_string());
                }
                self.type_expr(&args[0])?;
                self.type_expr(&args[1])?;
                Ok(Type::Unit)
            }
            "extend" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::extend")?;
                if args.len() != 1 {
                    return Err("typeck: Vec::extend expects 1 arg".to_string());
                }
                self.type_expr(&args[0])?;
                Ok(Type::Unit)
            }
            "contains" => {
                if args.len() != 1 {
                    return Err("typeck: Vec::contains expects 1 arg".to_string());
                }
                self.type_expr(&args[0])?;
                Ok(Type::Bool)
            }
            "pop" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::pop")?;
                if !args.is_empty() {
                    return Err("typeck: Vec::pop expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![elem],
                })
            }
            "remove" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::remove")?;
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Vec::remove expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let index_ty = self.type_expr(&args[0])?;
                if !is_integer(&index_ty) {
                    return Err(format!(
                        "typeck: Vec::remove index expected integer, got {:?}",
                        index_ty
                    ));
                }
                Ok(elem)
            }
            "get" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Vec::get expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let index_ty = self.type_expr(&args[0])?;
                if !is_integer(&index_ty) {
                    return Err(format!(
                        "typeck: Vec::get index expected integer, got {:?}",
                        index_ty
                    ));
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(elem),
                    }],
                })
            }
            "get_mut" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::get_mut")?;
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Vec::get_mut expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let index_ty = self.type_expr(&args[0])?;
                if !is_integer(&index_ty) {
                    return Err(format!(
                        "typeck: Vec::get_mut index expected integer, got {:?}",
                        index_ty
                    ));
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Ref {
                        mutable: true,
                        inner: Box::new(elem),
                    }],
                })
            }
            "first" => {
                if !args.is_empty() {
                    return Err("typeck: Vec::first expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(elem),
                    }],
                })
            }
            "last" => {
                if !args.is_empty() {
                    return Err("typeck: Vec::last expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(elem),
                    }],
                })
            }
            "last_mut" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::last_mut")?;
                if !args.is_empty() {
                    return Err("typeck: Vec::last_mut expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Ref {
                        mutable: true,
                        inner: Box::new(elem),
                    }],
                })
            }
            "iter" => {
                if !args.is_empty() {
                    return Err("typeck: Vec::iter expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(elem),
                    }],
                })
            }
            "iter_mut" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::iter_mut")?;
                if !args.is_empty() {
                    return Err("typeck: Vec::iter_mut expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Ref {
                        mutable: true,
                        inner: Box::new(elem),
                    }],
                })
            }
            "into_iter" => {
                if !args.is_empty() {
                    return Err("typeck: Vec::into_iter expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![elem],
                })
            }
            "join" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Vec::join expects 1 arg, got {}",
                        args.len()
                    ));
                }
                if !is_string_like(&elem) {
                    return Err(format!("typeck: Vec::join element is {:?}", elem));
                }
                let sep_ty = self.type_expr(&args[0])?;
                expect_str_like(&sep_ty, "Vec::join separator")?;
                Ok(Type::Named("String".to_string()))
            }
            "collect" => {
                if !args.is_empty() {
                    return Err("typeck: Vec::collect expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Vec".to_string(),
                    args: vec![elem],
                })
            }
            "to_vec" => {
                if !args.is_empty() {
                    return Err("typeck: Vec::to_vec expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Vec".to_string(),
                    args: vec![elem],
                })
            }
            "sort_by" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::sort_by")?;
                if args.len() != 1 {
                    return Err("typeck: Vec::sort_by expects 1 arg".to_string());
                }
                let ref_elem = Type::Ref {
                    mutable: false,
                    inner: Box::new(elem.clone()),
                };
                self.type_expr_against(
                    &args[0],
                    &Type::Closure {
                        params: vec![ref_elem.clone(), ref_elem],
                        ret: Box::new(Type::Named("Ordering".to_string())),
                    },
                    "Vec::sort_by",
                )?;
                Ok(Type::Unit)
            }
            "drain" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::drain")?;
                if args.len() != 1 {
                    return Err("typeck: Vec::drain expects 1 arg".to_string());
                }
                // The arg is a range expression; drain yields an iterator of the
                // removed elements.
                let _ = self.type_expr(&args[0])?;
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![elem.clone()],
                })
            }
            "windows" => {
                if args.len() != 1 {
                    return Err("typeck: Vec::windows expects 1 arg".to_string());
                }
                if !is_integer(&self.type_expr(&args[0])?) {
                    return Err("typeck: Vec::windows size must be an integer".to_string());
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Generic {
                        name: "Vec".to_string(),
                        args: vec![elem.clone()],
                    }],
                })
            }
            "split_at" => {
                if args.len() != 1 {
                    return Err("typeck: Vec::split_at expects 1 arg".to_string());
                }
                if !is_integer(&self.type_expr(&args[0])?) {
                    return Err("typeck: Vec::split_at index must be an integer".to_string());
                }
                let half = Type::Generic {
                    name: "Vec".to_string(),
                    args: vec![elem.clone()],
                };
                Ok(Type::Tuple(vec![half.clone(), half]))
            }
            "concat" => {
                if !args.is_empty() {
                    return Err("typeck: Vec::concat expects 0 args".to_string());
                }
                match deref_type(elem.clone()) {
                    Type::Generic { name, .. } if name == "Vec" => Ok(elem),
                    other => Err(format!(
                        "typeck: Vec::concat needs Vec<Vec<_>>, element is {:?}",
                        other
                    )),
                }
            }
            "binary_search" => {
                if args.len() != 1 {
                    return Err("typeck: Vec::binary_search expects 1 arg".to_string());
                }
                let arg_ty = self.type_expr(&args[0])?;
                if !placeholder && !type_compatible(&deref_type(arg_ty.clone()), &elem) {
                    return Err(format!(
                        "typeck: Vec::binary_search arg is {:?}, expected &{:?}",
                        arg_ty, elem
                    ));
                }
                Ok(Type::Generic {
                    name: "Result".to_string(),
                    args: vec![Type::Usize, Type::Usize],
                })
            }
            "rotate_left" | "rotate_right" => {
                self.require_mut_receiver(receiver, receiver_ty, "Vec::rotate")?;
                if args.len() != 1 {
                    return Err(format!("typeck: Vec::{} expects 1 arg", name));
                }
                if !is_integer(&self.type_expr(&args[0])?) {
                    return Err(format!("typeck: Vec::{} arg must be an integer", name));
                }
                Ok(Type::Unit)
            }
            other => Err(format!("typeck: unsupported Vec method {}", other)),
        }
    }

    fn type_iter_method(
        &mut self,
        name: &str,
        receiver: &Expr,
        receiver_ty: &Type,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<Type, String> {
        let elem = match deref_type(receiver_ty.clone()) {
            Type::Generic { name, args } if name == "Iter" && args.len() == 1 => args[0].clone(),
            other => return Err(format!("typeck: Iter method on {:?}", other)),
        };
        if name != "collect" && !type_args.is_empty() {
            return Err(format!(
                "typeck: Iter::{} does not support turbofish args",
                name
            ));
        }
        match name {
            "next" => {
                self.require_mut_or_temporary_receiver(receiver, receiver_ty, "Iter::next")?;
                if !args.is_empty() {
                    return Err("typeck: Iter::next expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![elem],
                })
            }
            "nth" => {
                self.require_mut_or_temporary_receiver(receiver, receiver_ty, "Iter::nth")?;
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Iter::nth expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let n_ty = self.type_expr(&args[0])?;
                if !is_integer(&n_ty) {
                    return Err(format!(
                        "typeck: Iter::nth index expected integer, got {:?}",
                        n_ty
                    ));
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![elem],
                })
            }
            "last" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: Iter::last expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![elem],
                })
            }
            "filter" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Iter::filter expects 1 arg, got {}",
                        args.len()
                    ));
                }
                self.check_expected_closure(
                    "Iter::filter",
                    &args[0],
                    Type::Ref {
                        mutable: false,
                        inner: Box::new(elem.clone()),
                    },
                    Type::Bool,
                )?;
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![elem],
                })
            }
            "find" => {
                self.require_mut_or_temporary_receiver(receiver, receiver_ty, "Iter::find")?;
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Iter::find expects 1 arg, got {}",
                        args.len()
                    ));
                }
                self.check_expected_closure(
                    "Iter::find",
                    &args[0],
                    Type::Ref {
                        mutable: false,
                        inner: Box::new(elem.clone()),
                    },
                    Type::Bool,
                )?;
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![elem],
                })
            }
            "map" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Iter::map expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let mapped = self.infer_expected_closure("Iter::map", &args[0], elem)?;
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![mapped],
                })
            }
            "flat_map" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Iter::flat_map expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let produced = self.infer_expected_closure("Iter::flat_map", &args[0], elem)?;
                let inner = match &produced {
                    Type::Generic { name: n, args: a }
                        if (n == "Vec" || n == "Iter") && a.len() == 1 =>
                    {
                        a[0].clone()
                    }
                    other => {
                        return Err(format!(
                            "typeck: Iter::flat_map closure must return Vec/Iter, got {:?}",
                            other
                        ))
                    }
                };
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![inner],
                })
            }
            "zip" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Iter::zip expects 1 arg, got {}",
                        args.len()
                    ));
                }
                let other = match deref_type(self.type_expr(&args[0])?) {
                    Type::Generic { name, args } if name == "Iter" && args.len() == 1 => {
                        args[0].clone()
                    }
                    other => return Err(format!("typeck: Iter::zip argument is {:?}", other)),
                };
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Tuple(vec![elem, other])],
                })
            }
            "all" | "any" => {
                self.require_mut_or_temporary_receiver(
                    receiver,
                    receiver_ty,
                    &format!("Iter::{}", name),
                )?;
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Iter::{} expects 1 arg, got {}",
                        name,
                        args.len()
                    ));
                }
                self.check_expected_closure(
                    &format!("Iter::{}", name),
                    &args[0],
                    elem,
                    Type::Bool,
                )?;
                Ok(Type::Bool)
            }
            "position" => {
                self.require_mut_or_temporary_receiver(receiver, receiver_ty, "Iter::position")?;
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Iter::position expects 1 arg, got {}",
                        args.len()
                    ));
                }
                self.check_expected_closure("Iter::position", &args[0], elem, Type::Bool)?;
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Usize],
                })
            }
            "rposition" => {
                self.require_mut_or_temporary_receiver(receiver, receiver_ty, "Iter::rposition")?;
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Iter::rposition expects 1 arg, got {}",
                        args.len()
                    ));
                }
                self.check_expected_closure("Iter::rposition", &args[0], elem, Type::Bool)?;
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Usize],
                })
            }
            "count" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: Iter::count expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Usize)
            }
            "sum" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: Iter::sum expects 0 args, got {}",
                        args.len()
                    ));
                }
                let out = deref_type(elem.clone());
                if !is_integer(&out) {
                    return Err(format!("typeck: Iter::sum item is {:?}", elem));
                }
                Ok(out)
            }
            "product" => {
                if !args.is_empty() {
                    return Err(format!("typeck: Iter::product expects 0 args"));
                }
                let out = deref_type(elem.clone());
                if !is_integer(&out) {
                    return Err(format!("typeck: Iter::product item is {:?}", elem));
                }
                Ok(out)
            }
            "min" | "max" => {
                if !args.is_empty() {
                    return Err(format!("typeck: Iter::{} expects 0 args", name));
                }
                let out = deref_type(elem.clone());
                if !is_integer(&out) {
                    return Err(format!("typeck: Iter::{} item is {:?}", name, elem));
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![out],
                })
            }
            "max_by_key" | "min_by_key" => {
                if args.len() != 1 {
                    return Err(format!("typeck: Iter::{} expects 1 arg", name));
                }
                let key_ty = self.infer_expected_closure(
                    name,
                    &args[0],
                    Type::Ref {
                        mutable: false,
                        inner: Box::new(elem.clone()),
                    },
                )?;
                if !is_integer(&deref_type(key_ty.clone())) {
                    return Err(format!(
                        "typeck: Iter::{} key is {:?}, expected an integer",
                        name, key_ty
                    ));
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![elem],
                })
            }
            "fold" => {
                if args.len() != 2 {
                    return Err(format!(
                        "typeck: Iter::fold expects 2 args, got {}",
                        args.len()
                    ));
                }
                let acc_ty = self.type_expr(&args[0])?;
                self.type_expr_against(
                    &args[1],
                    &Type::Closure {
                        params: vec![acc_ty.clone(), elem],
                        ret: Box::new(acc_ty.clone()),
                    },
                    "Iter::fold",
                )?;
                Ok(acc_ty)
            }
            "take" | "skip" => {
                if args.len() != 1 {
                    return Err(format!(
                        "typeck: Iter::{} expects 1 arg, got {}",
                        name,
                        args.len()
                    ));
                }
                let n_ty = self.type_expr(&args[0])?;
                if !is_integer(&n_ty) {
                    return Err(format!(
                        "typeck: Iter::{} count expected integer, got {:?}",
                        name, n_ty
                    ));
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![elem],
                })
            }
            "step_by" => {
                if args.len() != 1 {
                    return Err(format!("typeck: Iter::step_by expects 1 arg"));
                }
                let n_ty = self.type_expr(&args[0])?;
                if !is_integer(&n_ty) {
                    return Err(format!(
                        "typeck: Iter::step_by expected integer, got {:?}",
                        n_ty
                    ));
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![elem],
                })
            }
            "chain" => {
                if args.len() != 1 {
                    return Err(format!("typeck: Iter::chain expects 1 arg"));
                }
                let other = self.type_expr(&args[0])?;
                match &other {
                    Type::Generic { name: n, .. } if n == "Iter" => {}
                    _ => {
                        return Err(format!(
                            "typeck: Iter::chain expected an iterator, got {:?}",
                            other
                        ))
                    }
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![elem],
                })
            }
            "copied" | "cloned" => {
                if !args.is_empty() {
                    return Err(format!("typeck: Iter::{} expects 0 args", name));
                }
                match elem {
                    Type::Ref { inner, .. } => Ok(Type::Generic {
                        name: "Iter".to_string(),
                        args: vec![*inner],
                    }),
                    other => Err(format!("typeck: Iter::{} on Iter<{:?}>", name, other)),
                }
            }
            "rev" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: Iter::rev expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![elem],
                })
            }
            "enumerate" => {
                if !args.is_empty() {
                    return Err(format!(
                        "typeck: Iter::enumerate expects 0 args, got {}",
                        args.len()
                    ));
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![Type::Tuple(vec![Type::Usize, elem])],
                })
            }
            "collect" => {
                if !args.is_empty() {
                    return Err("typeck: Iter::collect expects 0 args".to_string());
                }
                if type_args.len() == 1 {
                    let out = self.resolve_type(&type_args[0]);
                    return match &out {
                        Type::Named(name) if name == "String" => match elem {
                            Type::Char => Ok(out),
                            Type::Ref { inner, .. } if *inner == Type::Char => Ok(out),
                            other => Err(format!(
                                "typeck: Iter::collect::<String> on Iter<{:?}>",
                                other
                            )),
                        },
                        Type::Generic { name, args } if name == "Vec" && args.len() == 1 => {
                            if type_compatible(&elem, &args[0]) {
                                Ok(out)
                            } else {
                                Err(format!(
                                    "typeck: Iter::collect::<Vec<{:?}>> on Iter<{:?}>",
                                    args[0], elem
                                ))
                            }
                        }
                        other => Err(format!(
                            "typeck: Iter::collect turbofish unsupported for {:?}",
                            other
                        )),
                    };
                }
                if !type_args.is_empty() {
                    return Err(format!(
                        "typeck: Iter::collect expects 0 or 1 turbofish args, got {}",
                        type_args.len()
                    ));
                }
                match elem {
                    Type::Char => Ok(Type::Named("String".to_string())),
                    Type::Ref { inner, .. } if *inner == Type::Char => {
                        Ok(Type::Named("String".to_string()))
                    }
                    other => Ok(Type::Generic {
                        name: "Vec".to_string(),
                        args: vec![other],
                    }),
                }
            }
            "take_while" | "skip_while" => {
                if args.len() != 1 {
                    return Err(format!("typeck: Iter::{} expects 1 arg", name));
                }
                self.check_expected_closure(
                    "Iter::take_while/skip_while",
                    &args[0],
                    Type::Ref {
                        mutable: false,
                        inner: Box::new(elem.clone()),
                    },
                    Type::Bool,
                )?;
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![elem],
                })
            }
            "find_map" => {
                self.require_mut_or_temporary_receiver(receiver, receiver_ty, "Iter::find_map")?;
                if args.len() != 1 {
                    return Err("typeck: Iter::find_map expects 1 arg".to_string());
                }
                let out = self.infer_expected_closure("Iter::find_map", &args[0], elem)?;
                match deref_type(out.clone()) {
                    Type::Generic { name, .. } if name == "Option" => Ok(out),
                    other => Err(format!(
                        "typeck: Iter::find_map closure returns {:?}, expected Option",
                        other
                    )),
                }
            }
            "peekable" => {
                if !args.is_empty() {
                    return Err("typeck: Iter::peekable expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Iter".to_string(),
                    args: vec![elem],
                })
            }
            "peek" => {
                self.require_mut_or_temporary_receiver(receiver, receiver_ty, "Iter::peek")?;
                if !args.is_empty() {
                    return Err("typeck: Iter::peek expects 0 args".to_string());
                }
                Ok(Type::Generic {
                    name: "Option".to_string(),
                    args: vec![Type::Ref {
                        mutable: false,
                        inner: Box::new(elem),
                    }],
                })
            }
            other => Err(format!("typeck: unsupported Iter method {}", other)),
        }
    }

    fn require_mut_receiver(
        &mut self,
        receiver: &Expr,
        receiver_ty: &Type,
        ctx: &str,
    ) -> Result<(), String> {
        match receiver {
            Expr::Var(_)
            | Expr::Field { .. }
            | Expr::Index { .. }
            | Expr::Unary {
                op: UnOp::Deref, ..
            } => {
                let (ty, is_mut) = self.place_type(receiver)?;
                if is_mut || matches!(ty, Type::Ref { mutable: true, .. }) {
                    Ok(())
                } else {
                    Err(format!("typeck: {} needs a mutable receiver", ctx))
                }
            }
            _ => match receiver_ty {
                Type::Ref { mutable: true, .. } => Ok(()),
                _ => Err(format!("typeck: {} needs a mutable receiver", ctx)),
            },
        }
    }

    fn require_mut_or_temporary_receiver(
        &mut self,
        receiver: &Expr,
        receiver_ty: &Type,
        ctx: &str,
    ) -> Result<(), String> {
        match receiver {
            Expr::Var(_) => self.require_mut_receiver(receiver, receiver_ty, ctx),
            Expr::Field { .. }
            | Expr::Index { .. }
            | Expr::Unary {
                op: UnOp::Deref, ..
            } => self.require_mut_receiver(receiver, receiver_ty, ctx),
            _ => Ok(()),
        }
    }

    fn place_type(&mut self, e: &Expr) -> Result<(Type, bool), String> {
        match e {
            Expr::Var(name) => self
                .lookup_full(name)
                .ok_or_else(|| format!("typeck: unbound variable {}", name)),
            Expr::Unary {
                op: UnOp::Deref,
                rhs,
            } => match self.type_expr(rhs)? {
                Type::Ref { mutable, inner } => Ok((*inner, mutable)),
                other => Err(format!("typeck: deref place on {:?}", other)),
            },
            Expr::Index { base, index } => {
                let (base_ty, is_mut) = self.place_type(base)?;
                let index_ty = self.type_expr(index)?;
                if !is_integer(&index_ty) {
                    return Err(format!(
                        "typeck: index assignment expected integer index, got {:?}",
                        index_ty
                    ));
                }
                match deref_type(base_ty) {
                    Type::Generic { name, args } if name == "Vec" && args.len() == 1 => {
                        Ok((args[0].clone(), is_mut))
                    }
                    other => Err(format!("typeck: index assignment on {:?}", other)),
                }
            }
            Expr::TupleIndex { base, index } => {
                let (base_ty, is_mut) = self.place_type(base)?;
                let tuple_mut = is_mut || matches!(base_ty, Type::Ref { mutable: true, .. });
                match deref_type(base_ty) {
                    Type::Tuple(items) => items
                        .get(*index)
                        .cloned()
                        .map(|ty| (ty, tuple_mut))
                        .ok_or_else(|| format!("typeck: tuple index {} out of range", index)),
                    other => Err(format!("typeck: tuple index assignment on {:?}", other)),
                }
            }
            Expr::Field { base, name } => {
                let (base_ty, is_mut) = self.place_type(base)?;
                let field_mut = is_mut || matches!(base_ty, Type::Ref { mutable: true, .. });
                match deref_type(base_ty) {
                    Type::Named(sname) | Type::Generic { name: sname, .. } => {
                        let def = self.structs.get(&sname).ok_or_else(|| {
                            format!("typeck: field place on non-struct {}", sname)
                        })?;
                        def.iter()
                            .find(|(k, _)| k == name)
                            .map(|(_, t)| (t.clone(), field_mut))
                            .ok_or_else(|| format!("typeck: {} has no field {}", sname, name))
                    }
                    other => Err(format!("typeck: field place on {:?}", other)),
                }
            }
            other => Err(format!(
                "typeck: expected assignable/referenceable place, got {:?}",
                other
            )),
        }
    }

    fn check_pattern(&mut self, pat: &Pattern, expected: &Type) -> Result<(), String> {
        self.check_pattern_with_mode(pat, expected, false)
    }

    fn check_pattern_with_mode(
        &mut self,
        pat: &Pattern,
        expected: &Type,
        default_ref_bind: bool,
    ) -> Result<(), String> {
        if let Type::Ref { inner, .. } = expected {
            if !matches!(
                pat,
                Pattern::Wild | Pattern::Bind(_) | Pattern::BindRef { .. } | Pattern::Ref { .. }
            ) {
                return self.check_pattern_with_mode(pat, inner, true);
            }
        }
        match pat {
            Pattern::Wild => Ok(()),
            Pattern::Bind(name) => {
                let bind_ty = if default_ref_bind {
                    Type::Ref {
                        mutable: false,
                        inner: Box::new(expected.clone()),
                    }
                } else {
                    expected.clone()
                };
                self.define(name, bind_ty, false);
                Ok(())
            }
            Pattern::BindRef { name, mutable } => {
                self.define(
                    name,
                    Type::Ref {
                        mutable: *mutable,
                        inner: Box::new(expected.clone()),
                    },
                    false,
                );
                Ok(())
            }
            Pattern::Int(_) => {
                if is_integer(expected) {
                    Ok(())
                } else {
                    Err(format!("typeck: int pattern against {:?}", expected))
                }
            }
            Pattern::IntRange { .. } => {
                if is_integer(expected) {
                    Ok(())
                } else {
                    Err(format!("typeck: int range pattern against {:?}", expected))
                }
            }
            Pattern::Char(_) => expect(expected, &Type::Char, "char pattern"),
            Pattern::CharRange { .. } => expect(expected, &Type::Char, "char range pattern"),
            Pattern::Str(_) => expect_str_like(expected, "string pattern"),
            Pattern::Bool(_) => expect(expected, &Type::Bool, "bool pattern"),
            Pattern::BindAt { name, sub } => {
                let bind_ty = if default_ref_bind {
                    Type::Ref {
                        mutable: false,
                        inner: Box::new(expected.clone()),
                    }
                } else {
                    expected.clone()
                };
                self.define(name, bind_ty, false);
                self.check_pattern_with_mode(sub, expected, default_ref_bind)
            }
            Pattern::Tuple(subs) => match expected {
                Type::Unit if subs.is_empty() => Ok(()),
                Type::Tuple(tys) if tys.len() == subs.len() => {
                    for (p, t) in subs.iter().zip(tys.iter()) {
                        self.check_pattern_with_mode(p, t, default_ref_bind)?;
                    }
                    Ok(())
                }
                other => Err(format!("typeck: tuple pattern against {:?}", other)),
            },
            Pattern::Slice {
                prefix,
                rest,
                suffix,
            } => {
                let elem = match deref_type(expected.clone()) {
                    Type::Generic { name, args } if name == "Vec" && args.len() == 1 => {
                        args[0].clone()
                    }
                    Type::Array(inner, _) => *inner,
                    Type::Slice(inner) => *inner,
                    other => {
                        return Err(format!("typeck: slice pattern against {:?}", other));
                    }
                };
                for p in prefix {
                    self.check_pattern_with_mode(p, &elem, default_ref_bind)?;
                }
                for p in suffix {
                    self.check_pattern_with_mode(p, &elem, default_ref_bind)?;
                }
                if let Some(Some(name)) = rest {
                    self.define(
                        name,
                        Type::Generic {
                            name: "Vec".to_string(),
                            args: vec![elem.clone()],
                        },
                        false,
                    );
                }
                Ok(())
            }
            Pattern::Or(items) => {
                for p in items {
                    self.check_pattern_with_mode(p, expected, default_ref_bind)?;
                }
                Ok(())
            }
            Pattern::Ref { mutable, sub } => match expected {
                Type::Ref { mutable: m, inner } if !*mutable || *m => {
                    self.check_pattern_with_mode(sub, inner, false)
                }
                other => Err(format!("typeck: ref pattern against {:?}", other)),
            },
            Pattern::Struct { name, fields, rest } => {
                let (def, subst) = match expected {
                    Type::Named(got) if got == name => (
                        self.structs
                            .get(name)
                            .cloned()
                            .ok_or_else(|| format!("typeck: unknown struct {}", name))?,
                        HashMap::new(),
                    ),
                    Type::Generic { name: got, args } if got == name => (
                        self.structs
                            .get(name)
                            .cloned()
                            .ok_or_else(|| format!("typeck: unknown struct {}", name))?,
                        self.type_subst_from_args(name, args),
                    ),
                    other => {
                        return Err(format!(
                            "typeck: struct pattern {} against {:?}",
                            name, other
                        ))
                    }
                };
                if !rest && fields.len() != def.len() {
                    return Err(format!(
                        "typeck: struct pattern {} expects {} fields, got {}",
                        name,
                        def.len(),
                        fields.len()
                    ));
                }
                let mut seen = Vec::new();
                for (fname, pat) in fields {
                    if seen.iter().any(|seen| seen == fname) {
                        return Err(format!(
                            "typeck: duplicate field {} in struct pattern {}",
                            fname, name
                        ));
                    }
                    seen.push(fname.clone());
                    let want = def
                        .iter()
                        .find(|(k, _)| k == fname)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| format!("typeck: struct {} has no field {}", name, fname))?;
                    let want = apply_subst(&want, &subst);
                    self.check_pattern_with_mode(pat, &want, default_ref_bind)?;
                }
                Ok(())
            }
            Pattern::Enum {
                enum_name,
                variant,
                sub,
            } => {
                let fields = match expected {
                    Type::Named(name) if name == enum_name => {
                        self.enum_variant_fields(enum_name, variant)?
                    }
                    Type::Generic { name, args }
                        if name == enum_name && self.enums.contains_key(enum_name) =>
                    {
                        let subst = self.type_subst_from_args(enum_name, args);
                        self.enum_variant_fields(enum_name, variant)?
                            .into_iter()
                            .map(|ty| apply_subst(&ty, &subst))
                            .collect()
                    }
                    Type::Generic { name, args } if name == "Option" && enum_name == "Option" => {
                        match variant.as_str() {
                            "Some" if args.len() == 1 => vec![args[0].clone()],
                            "None" => Vec::new(),
                            _ => {
                                return Err(format!(
                                    "typeck: Option has no compatible variant {}",
                                    variant
                                ))
                            }
                        }
                    }
                    Type::Generic { name, args } if name == "Result" && enum_name == "Result" => {
                        match variant.as_str() {
                            "Ok" if args.len() == 2 => vec![args[0].clone()],
                            "Err" if args.len() == 2 => vec![args[1].clone()],
                            _ => {
                                return Err(format!(
                                    "typeck: Result has no compatible variant {}",
                                    variant
                                ))
                            }
                        }
                    }
                    other => {
                        return Err(format!(
                            "typeck: enum pattern {}::{} against {:?}",
                            enum_name, variant, other
                        ))
                    }
                };
                if fields.len() != sub.len() {
                    return Err(format!(
                        "typeck: pattern {}::{} expects {} fields, got {}",
                        enum_name,
                        variant,
                        fields.len(),
                        sub.len()
                    ));
                }
                for (p, t) in sub.iter().zip(fields.iter()) {
                    self.check_pattern_with_mode(p, t, default_ref_bind)?;
                }
                Ok(())
            }
            Pattern::EnumStruct {
                enum_name,
                variant,
                fields,
                rest,
            } => {
                let subst = match expected {
                    Type::Named(name) if name == enum_name => HashMap::new(),
                    Type::Generic { name, args }
                        if name == enum_name && self.enums.contains_key(enum_name) =>
                    {
                        self.type_subst_from_args(enum_name, args)
                    }
                    other => {
                        return Err(format!(
                            "typeck: enum struct pattern {}::{} against {:?}",
                            enum_name, variant, other
                        ))
                    }
                };
                let def = self
                    .enum_named_fields
                    .get(&(enum_name.clone(), variant.clone()))
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "typeck: {}::{} is not a struct-like enum variant",
                            enum_name, variant
                        )
                    })?;
                if !rest && fields.len() != def.len() {
                    return Err(format!(
                        "typeck: pattern {}::{} expects {} fields, got {}",
                        enum_name,
                        variant,
                        def.len(),
                        fields.len()
                    ));
                }
                let mut seen = Vec::new();
                for (fname, pat) in fields {
                    if seen.iter().any(|seen| seen == fname) {
                        return Err(format!(
                            "typeck: duplicate field {} in pattern {}::{}",
                            fname, enum_name, variant
                        ));
                    }
                    seen.push(fname.clone());
                    let want = def
                        .iter()
                        .find(|(k, _)| k == fname)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| {
                            format!("typeck: {}::{} has no field {}", enum_name, variant, fname)
                        })?;
                    let want = apply_subst(&want, &subst);
                    self.check_pattern_with_mode(pat, &want, default_ref_bind)?;
                }
                Ok(())
            }
        }
    }

    fn check_match_exhaustive(&self, scrut: &Type, arms: &[Arm]) -> Result<(), String> {
        for arm in arms {
            if arm.guard.is_none() && pattern_covers_all(&arm.pat) {
                return Ok(());
            }
        }
        match deref_type(scrut.clone()) {
            Type::Bool => {
                let mut has_true = false;
                let mut has_false = false;
                for arm in arms {
                    if arm.guard.is_none() {
                        collect_bool_coverage(&arm.pat, &mut has_true, &mut has_false);
                    }
                }
                if has_true && has_false {
                    Ok(())
                } else {
                    Err("typeck: non-exhaustive bool match".to_string())
                }
            }
            Type::Named(name) | Type::Generic { name, .. } => {
                let variants = match self.enum_variant_names(&name) {
                    Some(v) => v,
                    None => return Ok(()),
                };
                let mut seen = Vec::new();
                for arm in arms {
                    if arm.guard.is_none() {
                        collect_enum_coverage(&arm.pat, &name, &mut seen);
                    }
                }
                for variant in variants {
                    if !seen.iter().any(|got| got == &variant) {
                        return Err(format!(
                            "typeck: non-exhaustive match for {}; missing {}",
                            name, variant
                        ));
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn enum_variant_names(&self, name: &str) -> Option<Vec<String>> {
        if name == "Option" {
            return Some(vec!["Some".to_string(), "None".to_string()]);
        }
        if name == "Result" {
            return Some(vec!["Ok".to_string(), "Err".to_string()]);
        }
        let variants = self.enums.get(name)?;
        let mut out = Vec::new();
        for (variant, _) in variants.iter() {
            out.push(variant.clone());
        }
        Some(out)
    }

    fn type_binary(&self, op: BinOp, l: Type, r: Type) -> Result<Type, String> {
        // Auto-deref references to numbers (rustc has `impl Add for &T`, etc.),
        // so `v.iter().map(|x| x * 2)` where x is `&i64` type-checks.
        let l = deref_num_ref(l);
        let r = deref_num_ref(r);
        match op {
            // Shifts: the result is the LHS integer type; the shift amount may be
            // a DIFFERENT integer type (rustc allows `i64 << u32`), so this does
            // NOT go through the mixed-width arithmetic check.
            BinOp::Shl | BinOp::Shr => {
                if !is_integer(&l) || !is_integer(&r) {
                    return Err(format!("typeck: shift on {:?} and {:?}", l, r));
                }
                Ok(l)
            }
            // Bitwise `&`/`|`/`^` on bools is logical (rustc allows `bool & bool`).
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor
                if l == Type::Bool && r == Type::Bool =>
            {
                Ok(Type::Bool)
            }
            BinOp::Add if l == Type::Named("String".to_string()) => {
                expect_str_like(&r, "String +")?;
                Ok(Type::Named("String".to_string()))
            }
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div
                if l == Type::F64 || r == Type::F64 =>
            {
                if l == Type::F64 && r == Type::F64 {
                    Ok(Type::F64)
                } else {
                    Err(format!("typeck: arithmetic on {:?} and {:?}", l, r))
                }
            }
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::BitXor | BinOp::BitAnd | BinOp::BitOr => {
                if !is_integer(&l) || !is_integer(&r) {
                    return Err(format!("typeck: arithmetic on {:?} and {:?}", l, r));
                }
                // rustc requires both integer operands to share one type. I64 is
                // the flexible literal type (an integer literal is typed I64 and
                // coerces), so it is exempt; two DIFFERENT concrete integer
                // widths (e.g. u32 + u64) are a type error the interpreter must
                // reject to match rustc.
                if l != r
                    && l != Type::I64
                    && r != Type::I64
                    && l != Type::IntLit
                    && r != Type::IntLit
                {
                    return Err(format!(
                        "typeck: arithmetic on mismatched integer types {:?} and {:?}",
                        l, r
                    ));
                }
                if l == Type::IntLit && r == Type::IntLit {
                    Ok(Type::IntLit)
                } else if l == Type::IntLit {
                    Ok(r)
                } else if r == Type::IntLit {
                    Ok(l)
                } else if l == Type::I64 {
                    Ok(r)
                } else {
                    Ok(l)
                }
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                if is_integer(&l) && is_integer(&r) {
                    if l != r
                        && l != Type::I64
                        && r != Type::I64
                        && l != Type::IntLit
                        && r != Type::IntLit
                    {
                        return Err(format!(
                            "typeck: comparison on mismatched integer types {:?} and {:?}",
                            l, r
                        ));
                    }
                    return Ok(Type::Bool);
                }
                if l == Type::F64 && r == Type::F64 {
                    return Ok(Type::Bool);
                }
                if is_string_like(&l) && is_string_like(&r) {
                    return Ok(Type::Bool);
                }
                if is_ordered_scalar(&l) && l == r {
                    return Ok(Type::Bool);
                }
                // Derived Ord on structs/tuples/Vec: same over-acceptance as `==`
                // (the interp can't see #[derive], so it trusts same-typed operands
                // and orders them field-by-field via value_cmp at run time).
                if type_compatible(&l, &r) || type_compatible(&r, &l) {
                    return Ok(Type::Bool);
                }
                Err(format!("typeck: ordered comparison on {:?} and {:?}", l, r))
            }
            BinOp::Eq | BinOp::Ne => {
                if l == Type::Unit && r == Type::Unit {
                    return Err("typeck: cannot compare ()".to_string());
                }
                if is_integer(&l) && is_integer(&r) {
                    if l != r
                        && l != Type::I64
                        && r != Type::I64
                        && l != Type::IntLit
                        && r != Type::IntLit
                    {
                        return Err(format!(
                            "typeck: comparison on mismatched integer types {:?} and {:?}",
                            l, r
                        ));
                    }
                    return Ok(Type::Bool);
                }
                if is_string_like(&l) && is_string_like(&r)
                    || l == Type::F64 && r == Type::F64
                    || type_compatible(&l, &r)
                    || type_compatible(&r, &l)
                {
                    return Ok(Type::Bool);
                }
                return Err(format!("typeck: cannot compare {:?} with {:?}", l, r));
            }
            BinOp::And | BinOp::Or => {
                expect(&l, &Type::Bool, "boolean op")?;
                expect(&r, &Type::Bool, "boolean op")?;
                Ok(Type::Bool)
            }
        }
    }
}

/// True iff a tail-less block can FALL THROUGH to `()` (does not diverge on
/// every path). Conservative toward diverging: control-flow constructs
/// (if/match/loop/while/return/panic) are treated as NOT falling through, so a
/// valid diverging function (e.g. `{ return 5; }`) is never wrongly rejected;
/// this deliberately catches the common gap (`fn f() -> i64 { }`, `{ let x = 3; }`)
/// where rustc rejects a non-unit return type but the interpreter accepted `()`.
fn block_falls_through(block: &Block) -> bool {
    match block.stmts.last() {
        None => true,
        Some(Stmt::Return(_)) => false,
        Some(Stmt::Expr(e)) => match e {
            Expr::Return(_) => false,
            Expr::Panic { .. } => false,
            Expr::Loop { .. } => false,
            Expr::While { .. } => false,
            Expr::If { .. } => false,
            Expr::Match { .. } => false,
            _ => true,
        },
        Some(_) => true,
    }
}

fn block_contains_break(block: &Block) -> bool {
    block.stmts.iter().any(stmt_contains_break)
        || block
            .tail
            .as_ref()
            .map(|expr| expr_contains_break(expr))
            .unwrap_or(false)
}

fn stmt_contains_break(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Let { init, .. } | Stmt::LetPat { init, .. } => expr_contains_break(init),
        Stmt::LetElse { init, else_blk, .. } => {
            expr_contains_break(init) || block_contains_break(else_blk)
        }
        Stmt::Assign { target, value } => expr_contains_break(target) || expr_contains_break(value),
        Stmt::Expr(expr) => expr_contains_break(expr),
        Stmt::Return(expr) => expr.as_ref().map(expr_contains_break).unwrap_or(false),
    }
}

fn expr_contains_break(expr: &Expr) -> bool {
    match expr {
        Expr::Break { .. } => true,
        Expr::Labeled { .. } => false,
        Expr::Ref { expr, .. }
        | Expr::Unary { rhs: expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Try(expr)
        | Expr::Return(Some(expr))
        | Expr::Field { base: expr, .. }
        | Expr::TupleIndex { base: expr, .. } => expr_contains_break(expr),
        Expr::Binary { lhs, rhs, .. }
        | Expr::Assign {
            target: lhs,
            value: rhs,
        } => expr_contains_break(lhs) || expr_contains_break(rhs),
        Expr::Call { args, .. } | Expr::PathCall { args, .. } => {
            args.iter().any(expr_contains_break)
        }
        Expr::CallExpr { callee, args } => {
            expr_contains_break(callee) || args.iter().any(expr_contains_break)
        }
        Expr::MethodCall { receiver, args, .. } => {
            expr_contains_break(receiver) || args.iter().any(expr_contains_break)
        }
        Expr::If {
            cond,
            then_blk,
            else_blk,
        } => {
            expr_contains_break(cond)
                || block_contains_break(then_blk)
                || else_blk.as_ref().map(block_contains_break).unwrap_or(false)
        }
        Expr::Block(block) => block_contains_break(block),
        Expr::Println { args, .. }
        | Expr::Print { args, .. }
        | Expr::Eprintln { args, .. }
        | Expr::Format { args, .. }
        | Expr::TupleLit(args)
        | Expr::VecLit(args) => args.iter().any(expr_contains_break),
        Expr::VecRepeat { elem, count } => expr_contains_break(elem) || expr_contains_break(count),
        Expr::Write { target, args, .. } => {
            expr_contains_break(target) || args.iter().any(expr_contains_break)
        }
        Expr::Matches { expr, guard, .. } => {
            expr_contains_break(expr)
                || guard
                    .as_ref()
                    .map(|guard| expr_contains_break(guard))
                    .unwrap_or(false)
        }
        Expr::Assert { cond } => expr_contains_break(cond),
        Expr::AssertEq { left, right } => expr_contains_break(left) || expr_contains_break(right),
        Expr::StructLit { fields, .. } | Expr::EnumStructLit { fields, .. } => {
            fields.iter().any(|(_, expr)| expr_contains_break(expr))
        }
        Expr::Index { base, index } => expr_contains_break(base) || expr_contains_break(index),
        Expr::Slice {
            base, start, end, ..
        } => {
            expr_contains_break(base)
                || start
                    .as_ref()
                    .map(|expr| expr_contains_break(expr))
                    .unwrap_or(false)
                || end
                    .as_ref()
                    .map(|expr| expr_contains_break(expr))
                    .unwrap_or(false)
        }
        Expr::Range { start, end, .. } => expr_contains_break(start) || expr_contains_break(end),
        Expr::Match { scrut, arms } => {
            expr_contains_break(scrut) || arms.iter().any(|arm| expr_contains_break(&arm.body))
        }
        // Breaks inside nested loops target those loops, not the loop currently
        // being classified.
        Expr::While { .. }
        | Expr::WhileLet { .. }
        | Expr::Loop { .. }
        | Expr::For { .. }
        | Expr::ForEach { .. } => false,
        Expr::Closure { .. } => false,
        Expr::Int(_)
        | Expr::IntHex(_, _)
        | Expr::Float(_)
        | Expr::Char(_)
        | Expr::Str(_)
        | Expr::Bool(_)
        | Expr::Var(_)
        | Expr::Return(None)
        | Expr::EnumCtor { .. }
        | Expr::Panic { .. }
        | Expr::Cfg { .. }
        | Expr::Continue => false,
    }
}

fn expect(got: &Type, want: &Type, ctx: &str) -> Result<(), String> {
    if got == want || *got == Type::Never {
        Ok(())
    } else {
        Err(format!(
            "typeck: {} expected {:?}, got {:?}",
            ctx, want, got
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FormatArgKind {
    Display,
    Debug,
    LowerHex,
    FixedPrecision(usize),
}

fn format_placeholder_kinds(fmt: &str, ctx: &str) -> Result<Vec<FormatArgKind>, String> {
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    let mut kinds = Vec::new();
    while i < chars.len() {
        match chars[i] {
            '{' if chars.get(i + 1) == Some(&'}') => {
                kinds.push(FormatArgKind::Display);
                i += 2;
            }
            '{' if chars.get(i + 1) == Some(&':')
                && chars.get(i + 2) == Some(&'?')
                && chars.get(i + 3) == Some(&'}') =>
            {
                kinds.push(FormatArgKind::Debug);
                i += 4;
            }
            '{' if chars.get(i + 1) == Some(&':')
                && chars.get(i + 2) == Some(&'#')
                && chars.get(i + 3) == Some(&'?')
                && chars.get(i + 4) == Some(&'}') =>
            {
                kinds.push(FormatArgKind::Debug);
                i += 5;
            }
            '{' if chars.get(i + 1) == Some(&':')
                && chars.get(i + 2) == Some(&'0')
                && chars.get(i + 3) == Some(&'1')
                && chars.get(i + 4) == Some(&'6')
                && chars.get(i + 5) == Some(&'x')
                && chars.get(i + 6) == Some(&'}') =>
            {
                kinds.push(FormatArgKind::LowerHex);
                i += 7;
            }
            '{' if chars.get(i + 1) == Some(&':')
                && matches!(
                    chars.get(i + 2),
                    Some('x') | Some('X') | Some('b') | Some('o')
                )
                && chars.get(i + 3) == Some(&'}') =>
            {
                // {:x}/{:X}/{:b}/{:o} all require an integer, like LowerHex.
                kinds.push(FormatArgKind::LowerHex);
                i += 4;
            }
            '{' if typeck_fixed_precision_placeholder(&chars, i).is_some() => {
                let (precision, next_i) =
                    typeck_fixed_precision_placeholder(&chars, i).unwrap();
                kinds.push(FormatArgKind::FixedPrecision(precision));
                i = next_i;
            }
            '{' if typeck_left_align_placeholder(&chars, i).is_some() => {
                let (_, next_i) = typeck_left_align_placeholder(&chars, i).unwrap();
                kinds.push(FormatArgKind::Display);
                i = next_i;
            }
            '{' if typeck_right_align_placeholder(&chars, i).is_some() => {
                let (_, next_i) = typeck_right_align_placeholder(&chars, i).unwrap();
                kinds.push(FormatArgKind::Display);
                i = next_i;
            }
            '{' if chars.get(i + 1) == Some(&'{') => i += 2,
            '}' if chars.get(i + 1) == Some(&'}') => i += 2,
            '{' | '}' => {
                return Err(format!(
                    "typeck: {} only supports {{}}/{{:?}}/{{:#?}}/{{:016x}}/{{:.N}}/{{:<N}}/{{:>N}} placeholders and escaped braces",
                    ctx
                ));
            }
            _ => i += 1,
        }
    }
    Ok(kinds)
}

fn check_format_arg_type(ty: &Type, kind: FormatArgKind, ctx: &str) -> Result<(), String> {
    match kind {
        FormatArgKind::Display => {
            if format_displayable(ty) {
                Ok(())
            } else {
                Err(format!(
                    "typeck: {} {:?} does not implement Display",
                    ctx, ty
                ))
            }
        }
        FormatArgKind::Debug => {
            if format_debuggable(ty) {
                Ok(())
            } else {
                Err(format!("typeck: {} {:?} does not implement Debug", ctx, ty))
            }
        }
        FormatArgKind::LowerHex => {
            if is_integer(&format_deref_type(ty)) {
                Ok(())
            } else {
                Err(format!(
                    "typeck: {} {:?} does not implement LowerHex",
                    ctx, ty
                ))
            }
        }
        FormatArgKind::FixedPrecision(_) => {
            let ty = format_deref_type(ty);
            if ty == Type::F64 || is_integer(&ty) {
                Ok(())
            } else {
                Err(format!(
                    "typeck: {} {:?} does not support fixed precision",
                    ctx, ty
                ))
            }
        }
    }
}

fn format_displayable(ty: &Type) -> bool {
    match format_deref_type(ty) {
        Type::IntLit
        | Type::I64
        | Type::F64
        | Type::I32
        | Type::U32
        | Type::U64
        | Type::U8
        | Type::Usize
        | Type::Char
        | Type::Bool => true,
        Type::Named(name) => matches!(name.as_str(), "String" | "str" | "ExitStatus" | "ExitCode"),
        Type::Generic { name, args } if (name == "Box" || name == "Rc") && args.len() == 1 => {
            format_displayable(&args[0])
        }
        Type::Never => true,
        _ => false,
    }
}

fn format_debuggable(ty: &Type) -> bool {
    match format_deref_type(ty) {
        Type::Closure { .. } | Type::ImplTrait(_) | Type::Never => false,
        _ => true,
    }
}

fn format_deref_type(ty: &Type) -> Type {
    match ty {
        Type::Ref { inner, .. } => format_deref_type(inner),
        other => other.clone(),
    }
}

fn typeck_left_align_placeholder(chars: &[char], i: usize) -> Option<(usize, usize)> {
    typeck_align_placeholder(chars, i, '<')
}

fn typeck_right_align_placeholder(chars: &[char], i: usize) -> Option<(usize, usize)> {
    typeck_align_placeholder(chars, i, '>')
}

fn typeck_fixed_precision_placeholder(chars: &[char], i: usize) -> Option<(usize, usize)> {
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

fn typeck_align_placeholder(chars: &[char], i: usize, align: char) -> Option<(usize, usize)> {
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

/// The `&str` type, produced by str-slicing methods (split parts, trim, etc.).
fn str_ref_type() -> Type {
    Type::Ref {
        mutable: false,
        inner: Box::new(Type::Named("str".to_string())),
    }
}

fn expect_str_like(got: &Type, ctx: &str) -> Result<(), String> {
    match got {
        Type::Named(name) if name == "String" || name == "str" => Ok(()),
        Type::Ref { inner, .. } if matches!(inner.as_ref(), Type::Named(name) if name == "String" || name == "str") => {
            Ok(())
        }
        other => Err(format!(
            "typeck: {} expected string-like, got {:?}",
            ctx, other
        )),
    }
}

fn is_string_like(t: &Type) -> bool {
    matches!(t, Type::Named(name) if name == "String" || name == "str")
        || matches!(t, Type::Ref { inner, .. } if matches!(inner.as_ref(), Type::Named(name) if name == "String" || name == "str"))
}

fn question_error_compatible(got: &Type, want: &Type) -> bool {
    type_compatible(got, want) || str_to_string_error(got, want)
}

fn str_to_string_error(got: &Type, want: &Type) -> bool {
    matches!(deref_type(want.clone()), Type::Named(name) if name == "String")
        && matches!(deref_type(got.clone()), Type::Named(name) if name == "str")
}

fn is_int_target(target: &str) -> bool {
    matches!(target, "i64" | "i32" | "u32" | "u64" | "u8" | "usize")
}

fn int_target_type(target: &str) -> Result<Type, String> {
    match target {
        "i64" => Ok(Type::I64),
        "i32" => Ok(Type::I32),
        "u32" => Ok(Type::U32),
        "u64" => Ok(Type::U64),
        "u8" => Ok(Type::U8),
        "usize" => Ok(Type::Usize),
        other => Err(format!("typeck: unsupported integer target {}", other)),
    }
}

/// Collapse the literal type at a binding/container boundary: an unconstrained
/// integer literal defaults to i64 in this subset.
fn collapse_lit(t: Type) -> Type {
    if t == Type::IntLit {
        Type::I64
    } else {
        t
    }
}

fn is_integer(t: &Type) -> bool {
    matches!(
        t,
        Type::I64 | Type::I32 | Type::U32 | Type::U64 | Type::U8 | Type::Usize | Type::IntLit
    )
}

fn is_ordered_scalar(t: &Type) -> bool {
    is_integer(t) || *t == Type::Char
}

fn is_displayable_type(t: &Type) -> bool {
    if is_integer(t) || *t == Type::Bool || *t == Type::Char {
        return true;
    }
    match t {
        Type::Named(name) => name == "String" || name == "str",
        Type::Ref { inner, .. } => matches!(inner.as_ref(), Type::Named(name) if name == "str"),
        _ => false,
    }
}

/// The constant value of an integer-literal expression (decimal, hex, or a
/// negated literal), used to lint out-of-range literal casts. `None` for any
/// non-literal operand (variables cast at runtime are never linted).
fn literal_int_value(e: &Expr) -> Option<i64> {
    match e {
        Expr::Int(n) => Some(*n),
        Expr::IntHex(n, _) => Some(*n),
        Expr::Unary {
            op: UnOp::Neg,
            rhs,
        } => literal_int_value(rhs).map(|v| -v),
        _ => None,
    }
}

fn is_valid_cast(from: &Type, to: &Type) -> bool {
    if *from == Type::F64 {
        return is_integer(to) || *to == Type::F64;
    }
    if *to == Type::F64 {
        return is_integer(from);
    }
    (is_integer(from) || *from == Type::Char || *from == Type::Bool)
        && (is_integer(to) || *to == Type::Char)
}

fn common_type(a: &Type, b: &Type) -> Option<Type> {
    if a == b {
        return Some(a.clone());
    }
    if *a == Type::Never {
        return Some(b.clone());
    }
    if *b == Type::Never {
        return Some(a.clone());
    }
    if is_integer(a) && is_integer(b) {
        return Some(if *a == Type::I64 {
            b.clone()
        } else {
            a.clone()
        });
    }
    match (a, b) {
        (Type::Generic { name: an, args: aa }, Type::Generic { name: bn, args: ba })
            if an == "Option" && bn == "Option" =>
        {
            if aa.is_empty() {
                return Some(b.clone());
            }
            if ba.is_empty() {
                return Some(a.clone());
            }
            if aa.len() == 1 && ba.len() == 1 {
                return common_type(&aa[0], &ba[0]).map(|inner| Type::Generic {
                    name: "Option".to_string(),
                    args: vec![inner],
                });
            }
            None
        }
        (Type::Generic { name: an, args: aa }, Type::Generic { name: bn, args: ba })
            if an == "Result" && bn == "Result" && aa.len() == 2 && ba.len() == 2 =>
        {
            let ok = if aa[0] == Type::Unit {
                Some(ba[0].clone())
            } else if ba[0] == Type::Unit {
                Some(aa[0].clone())
            } else {
                common_type(&aa[0], &ba[0])
            }?;
            let err = if aa[1] == Type::Unit {
                Some(ba[1].clone())
            } else if ba[1] == Type::Unit {
                Some(aa[1].clone())
            } else {
                common_type(&aa[1], &ba[1])
            }?;
            Some(Type::Generic {
                name: "Result".to_string(),
                args: vec![ok, err],
            })
        }
        _ if type_compatible(a, b) => Some(b.clone()),
        _ if type_compatible(b, a) => Some(a.clone()),
        _ => None,
    }
}

fn impl_target_name(ty: &Type) -> Result<String, String> {
    match ty {
        Type::Named(name) => Ok(name.clone()),
        Type::Generic { name, .. } => Ok(name.clone()),
        other => Err(format!(
            "typeck: impl target {:?} is not supported yet",
            other
        )),
    }
}

fn method_target_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Named(name) => Some(name.clone()),
        Type::Generic { name, .. } => Some(name.clone()),
        Type::Slice(_) => Some("slice".to_string()),
        Type::Ref { inner, .. } => method_target_name(inner),
        Type::Tuple(_) => Some("tuple".to_string()),
        Type::Char => Some("char".to_string()),
        Type::Bool => Some("bool".to_string()),
        Type::Unit => Some("unit".to_string()),
        Type::I64 => Some("i64".to_string()),
        Type::I32 => Some("i32".to_string()),
        Type::U32 => Some("u32".to_string()),
        Type::U64 => Some("u64".to_string()),
        Type::U8 => Some("u8".to_string()),
        Type::Usize => Some("usize".to_string()),
        Type::ImplTrait(name) => Some(name.clone()),
        _ => None,
    }
}

fn resolve_aliases(ty: &Type, aliases: &HashMap<String, Type>) -> Type {
    resolve_aliases_inner(ty, aliases, 0)
}

fn resolve_aliases_inner(ty: &Type, aliases: &HashMap<String, Type>, depth: usize) -> Type {
    if depth > 32 {
        return ty.clone();
    }
    match ty {
        Type::Named(name) => aliases
            .get(name)
            .map(|aliased| resolve_aliases_inner(aliased, aliases, depth + 1))
            .unwrap_or_else(|| ty.clone()),
        Type::Generic { name, args } => {
            // E1d: lifetime arguments (Named("'a")) are emission-only.
            let real: Vec<Type> = args
                .iter()
                .filter(|arg| !matches!(arg, Type::Named(n) if n.starts_with("'")))
                .map(|arg| resolve_aliases_inner(arg, aliases, depth + 1))
                .collect();
            if real.is_empty() && !args.is_empty() {
                // All arguments were lifetimes: the erased view is the bare
                // name. (A genuinely empty Generic is the inference
                // placeholder and must stay Generic.)
                Type::Named(name.clone())
            } else {
                Type::Generic {
                    name: name.clone(),
                    args: real,
                }
            }
        }
        Type::Tuple(items) => Type::Tuple(
            items
                .iter()
                .map(|item| resolve_aliases_inner(item, aliases, depth + 1))
                .collect(),
        ),
        // E1d: explicit lifetimes exist only for emission — typeck sees the
        // stripped Ref (all comparisons stay on the derived PartialEq).
        Type::RefLt { mutable, inner, .. } => Type::Ref {
            mutable: *mutable,
            inner: Box::new(resolve_aliases_inner(inner, aliases, depth + 1)),
        },
        Type::Slice(inner) => {
            Type::Slice(Box::new(resolve_aliases_inner(inner, aliases, depth + 1)))
        }
        Type::Ref { mutable, inner } => Type::Ref {
            mutable: *mutable,
            inner: Box::new(resolve_aliases_inner(inner, aliases, depth + 1)),
        },
        Type::Closure { params, ret } => Type::Closure {
            params: params
                .iter()
                .map(|param| resolve_aliases_inner(param, aliases, depth + 1))
                .collect(),
            ret: Box::new(resolve_aliases_inner(ret, aliases, depth + 1)),
        },
        other => other.clone(),
    }
}

fn deref_type(ty: Type) -> Type {
    match ty {
        Type::Ref { inner, .. } => *inner,
        other => other,
    }
}

fn deref_num_ref(ty: Type) -> Type {
    match ty {
        Type::Ref { inner, .. } if is_integer(&inner) || *inner == Type::F64 => *inner,
        other => other,
    }
}

fn vec_index_elem(ty: &Type) -> Option<Type> {
    match ty {
        Type::Generic { name, args } if name == "Vec" && args.len() == 1 => Some(args[0].clone()),
        Type::Slice(inner) => Some(*inner.clone()),
        Type::Array(inner, _) => Some(*inner.clone()),
        Type::Generic { name, args } if name == "Rc" && args.len() == 1 => vec_index_elem(&args[0]),
        _ => None,
    }
}

fn ref_deref_compatible(got_inner: &Type, want_inner: &Type) -> bool {
    match got_inner {
        Type::Named(got)
            if got == "String" && matches!(want_inner, Type::Named(want) if want == "str") =>
        {
            true
        }
        Type::Named(got)
            if got == "PathBuf" && matches!(want_inner, Type::Named(want) if want == "Path") =>
        {
            true
        }
        Type::Generic { name, args } if (name == "Box" || name == "Rc") && args.len() == 1 => {
            type_compatible(&args[0], want_inner) || ref_deref_compatible(&args[0], want_inner)
        }
        Type::Ref { inner, .. } => {
            type_compatible(inner, want_inner) || ref_deref_compatible(inner, want_inner)
        }
        _ => false,
    }
}

fn hashmap_lookup_key_compatible(got: &Type, key_ty: &Type) -> bool {
    matches!(key_ty, Type::Named(name) if name == "String")
        && matches!(got, Type::Ref { inner, .. } if matches!(inner.as_ref(), Type::Named(name) if name == "str"))
}

fn same_generic_type(a: &Type, b: &Type) -> bool {
    if a == b {
        return true;
    }
    if (*a == Type::IntLit && is_integer(b)) || (*b == Type::IntLit && is_integer(a)) {
        return true;
    }
    match (a, b) {
        (
            Type::Generic { name: an, args: aa },
            Type::Generic { name: bn, args: ba },
        ) => {
            if an != bn || aa.len() != ba.len() {
                return false;
            }
            let mut i = 0usize;
            while i < aa.len() {
                if !same_generic_type(&aa[i], &ba[i]) {
                    return false;
                }
                i += 1;
            }
            true
        }
        (Type::Slice(ai), Type::Slice(bi)) => same_generic_type(ai, bi),
        (Type::Array(ai, al), Type::Array(bi, bl)) => {
            al == bl && same_generic_type(ai, bi)
        }
        (
            Type::Ref { mutable: am, inner: ai },
            Type::Ref { mutable: bm, inner: bi },
        ) => am == bm && same_generic_type(ai, bi),
        _ => false,
    }
}

fn type_compatible(got: &Type, want: &Type) -> bool {
    if got == want {
        return true;
    }
    // `fmt::Result` is `Result<(), fmt::Error>`; write!/writeln! types as
    // Result<(), ()> in this subset -- treat them as interchangeable.
    let fmt_result = |t: &Type| matches!(t, Type::Named(n) if n == "fmt::Result");
    let unit_result = |t: &Type| {
        matches!(t, Type::Generic { name, args } if name == "Result" && args.len() == 2 && args[0] == Type::Unit)
    };
    if (fmt_result(got) && unit_result(want)) || (unit_result(got) && fmt_result(want)) {
        return true;
    }
    if *got == Type::Never {
        return true;
    }
    if matches!(want, Type::ImplTrait(_)) {
        return true;
    }
    if is_integer(got) && is_integer(want) {
        return true;
    }
    match (got, want) {
        (Type::Named(got_name), Type::Named(want_name))
            if named_path_compatible(got_name, want_name) =>
        {
            true
        }
        (
            Type::Generic {
                name: got_name,
                args: got_args,
            },
            Type::Named(want_name),
        ) if got_args.is_empty() && got_name == want_name => true,
        (
            Type::Named(got_name),
            Type::Generic {
                name: want_name,
                args: want_args,
            },
        ) if want_args.is_empty() && got_name == want_name => true,
        (
            Type::Generic {
                name: got_name,
                args: got_args,
            },
            Type::Slice(want_inner),
        ) if got_name == "Vec" && got_args.len() == 1 => type_compatible(&got_args[0], want_inner),
        // Arrays `[T; N]` are modelled as vecs by the interpreter, so they are
        // interchangeable with `Vec<T>` and `[T]` at the type level (the length
        // is not tracked).
        (
            Type::Generic {
                name: got_name,
                args: got_args,
            },
            Type::Array(want_inner, _),
        ) if got_name == "Vec" && got_args.len() == 1 => type_compatible(&got_args[0], want_inner),
        (
            Type::Array(got_inner, _),
            Type::Generic {
                name: want_name,
                args: want_args,
            },
        ) if want_name == "Vec" && want_args.len() == 1 => type_compatible(got_inner, &want_args[0]),
        (Type::Array(got_inner, _), Type::Slice(want_inner)) => {
            type_compatible(got_inner, want_inner)
        }
        (Type::Slice(got_inner), Type::Array(want_inner, _)) => {
            type_compatible(got_inner, want_inner)
        }
        (Type::Array(got_inner, _), Type::Array(want_inner, _)) => {
            type_compatible(got_inner, want_inner)
        }
        (
            Type::Ref {
                inner: got_inner, ..
            },
            Type::Generic {
                name: want_name,
                args: want_args,
            },
        ) if matches!(got_inner.as_ref(), Type::Slice(_))
            && want_name == "Vec"
            && want_args.len() == 1 =>
        {
            match got_inner.as_ref() {
                Type::Slice(got_elem) => type_compatible(got_elem, &want_args[0]),
                _ => false,
            }
        }
        (
            Type::Generic {
                name: got_name,
                args: got_args,
            },
            Type::Ref {
                inner: want_inner, ..
            },
        ) if got_name == "Vec"
            && got_args.len() == 1
            && matches!(want_inner.as_ref(), Type::Slice(_)) =>
        {
            match want_inner.as_ref() {
                Type::Slice(want_elem) => type_compatible(&got_args[0], want_elem),
                _ => false,
            }
        }
        (
            Type::Ref {
                mutable: got_mut,
                inner: got_inner,
            },
            Type::Ref {
                mutable: want_mut,
                inner: want_inner,
            },
        ) if !*want_mut || *got_mut => {
            type_compatible(got_inner, want_inner)
                || ref_deref_compatible(got_inner, want_inner)
                || is_placeholder_type(got_inner)
                || is_placeholder_type(want_inner)
        }
        (
            Type::Generic {
                name: got_name,
                args: got_args,
            },
            Type::Generic {
                name: want_name, ..
            },
        ) if got_name == "Vec" && want_name == "Vec" && got_args.is_empty() => true,
        (
            Type::Generic {
                name: got_name,
                args: got_args,
            },
            Type::Generic {
                name: want_name, ..
            },
        ) if got_name == "HashMap" && want_name == "HashMap" && got_args.is_empty() => true,
        (
            Type::Generic {
                name: got_name,
                args: got_args,
            },
            Type::Generic {
                name: want_name,
                args: want_args,
            },
        ) if got_name == "Option" && want_name == "Option" => {
            got_args.is_empty()
                || want_args.is_empty()
                || (got_args.len() == 1
                    && want_args.len() == 1
                    && placeholder_compatible(&got_args[0], &want_args[0]))
        }
        (
            Type::Generic {
                name: got_name,
                args: got_args,
            },
            Type::Generic {
                name: want_name,
                args: want_args,
            },
        ) if got_name == "Result"
            && want_name == "Result"
            && got_args.len() == 2
            && want_args.len() == 2 =>
        {
            (got_args[0] == Type::Unit || type_compatible(&got_args[0], &want_args[0]))
                && (got_args[1] == Type::Unit || type_compatible(&got_args[1], &want_args[1]))
        }
        (
            Type::Generic {
                name: got_name,
                args: got_args,
            },
            Type::Generic {
                name: want_name,
                args: want_args,
            },
        ) if got_name == want_name && got_args.len() == want_args.len() => got_args
            .iter()
            .zip(want_args.iter())
            .all(|(g, w)| placeholder_compatible(g, w)),
        (Type::Tuple(got_items), Type::Tuple(want_items)) => {
            got_items.len() == want_items.len()
                && got_items
                    .iter()
                    .zip(want_items.iter())
                    .all(|(g, w)| type_compatible(g, w))
        }
        (
            Type::Closure {
                params: got_params,
                ret: got_ret,
            },
            Type::Closure {
                params: want_params,
                ret: want_ret,
            },
        ) => {
            got_params.len() == want_params.len()
                && got_params
                    .iter()
                    .zip(want_params.iter())
                    .all(|(g, w)| type_compatible(g, w))
                && type_compatible(got_ret, want_ret)
        }
        _ => false,
    }
}

fn named_path_compatible(got: &str, want: &str) -> bool {
    got == want
        || (got == "Report" && want == "check::Report")
        || (got == "check::Report" && want == "Report")
}

fn flattened_method_target(target: &str) -> String {
    if target == "check::Report" {
        "Report".to_string()
    } else {
        target.to_string()
    }
}

fn placeholder_compatible(got: &Type, want: &Type) -> bool {
    type_compatible(got, want) || is_placeholder_type(got) || is_placeholder_type(want)
}

fn is_generic_var_name(name: &str) -> bool {
    matches!(name, "T" | "U" | "V" | "E" | "K" | "A" | "B" | "C" | "R")
}

fn collect_generic_vars(ty: &Type, out: &mut Vec<String>) {
    match ty {
        Type::Named(name) if is_generic_var_name(name) => {
            if !out.iter().any(|seen| seen == name) {
                out.push(name.clone());
            }
        }
        Type::Generic { args, .. } | Type::Tuple(args) => {
            for arg in args {
                collect_generic_vars(arg, out);
            }
        }
        Type::Slice(inner) => collect_generic_vars(inner, out),
        Type::Ref { inner, .. } => collect_generic_vars(inner, out),
        Type::Closure { params, ret } => {
            for param in params {
                collect_generic_vars(param, out);
            }
            collect_generic_vars(ret, out);
        }
        _ => {}
    }
}

fn pattern_covers_all(pat: &Pattern) -> bool {
    match pat {
        Pattern::Wild | Pattern::Bind(_) | Pattern::BindRef { .. } => true,
        Pattern::BindAt { sub, .. } => pattern_covers_all(sub),
        Pattern::Or(items) => items.iter().any(pattern_covers_all),
        Pattern::Ref { sub, .. } => pattern_covers_all(sub),
        _ => false,
    }
}

fn collect_bool_coverage(pat: &Pattern, has_true: &mut bool, has_false: &mut bool) {
    match pat {
        Pattern::Bool(true) => *has_true = true,
        Pattern::Bool(false) => *has_false = true,
        Pattern::BindAt { sub, .. } | Pattern::Ref { sub, .. } => {
            collect_bool_coverage(sub, has_true, has_false);
        }
        Pattern::Or(items) => {
            for item in items {
                collect_bool_coverage(item, has_true, has_false);
            }
        }
        _ => {}
    }
}

fn collect_enum_coverage(pat: &Pattern, enum_name: &str, seen: &mut Vec<String>) {
    match pat {
        Pattern::Enum {
            enum_name: got,
            variant,
            ..
        }
        | Pattern::EnumStruct {
            enum_name: got,
            variant,
            ..
        } if got == enum_name => {
            if !seen.iter().any(|v| v == variant) {
                seen.push(variant.clone());
            }
        }
        Pattern::BindAt { sub, .. } | Pattern::Ref { sub, .. } => {
            collect_enum_coverage(sub, enum_name, seen);
        }
        Pattern::Or(items) => {
            for item in items {
                collect_enum_coverage(item, enum_name, seen);
            }
        }
        _ => {}
    }
}

fn apply_subst(ty: &Type, subst: &TypeSubst) -> Type {
    match ty {
        Type::Named(name) if is_generic_var_name(name) => {
            subst.get(name).cloned().unwrap_or_else(|| ty.clone())
        }
        Type::Generic { name, args } => Type::Generic {
            name: name.clone(),
            args: args.iter().map(|arg| apply_subst(arg, subst)).collect(),
        },
        Type::Tuple(items) => {
            Type::Tuple(items.iter().map(|item| apply_subst(item, subst)).collect())
        }
        Type::Slice(inner) => Type::Slice(Box::new(apply_subst(inner, subst))),
        Type::Ref { mutable, inner } => Type::Ref {
            mutable: *mutable,
            inner: Box::new(apply_subst(inner, subst)),
        },
        Type::Closure { params, ret } => Type::Closure {
            params: params
                .iter()
                .map(|param| apply_subst(param, subst))
                .collect(),
            ret: Box::new(apply_subst(ret, subst)),
        },
        other => other.clone(),
    }
}

fn unify_expected_type(expected: &Type, got: &Type, subst: &mut TypeSubst) -> bool {
    if type_compatible(got, expected) {
        return true;
    }
    match expected {
        Type::Named(name) if is_generic_var_name(name) => match subst.get(name).cloned() {
            Some(prev) => type_compatible(got, &prev) && type_compatible(&prev, got),
            None => {
                subst.insert(name.clone(), got.clone());
                true
            }
        },
        Type::Generic {
            name: want_name,
            args: want_args,
        } => match got {
            Type::Generic {
                name: got_name,
                args: got_args,
            } if got_name == want_name && got_args.len() == want_args.len() => want_args
                .iter()
                .zip(got_args.iter())
                .all(|(want, got)| unify_expected_type(want, got, subst)),
            Type::Generic {
                name: got_name,
                args: got_args,
            } if got_name == want_name && got_args.is_empty() => true,
            Type::Named(got_name) if got_name == want_name && want_args.is_empty() => true,
            _ => type_compatible(got, expected),
        },
        Type::Tuple(want_items) => match got {
            Type::Tuple(got_items) if got_items.len() == want_items.len() => want_items
                .iter()
                .zip(got_items.iter())
                .all(|(want, got)| unify_expected_type(want, got, subst)),
            _ => type_compatible(got, expected),
        },
        Type::Slice(want_inner) => match got {
            Type::Slice(got_inner) => unify_expected_type(want_inner, got_inner, subst),
            _ => type_compatible(got, expected),
        },
        Type::Ref {
            mutable: want_mut,
            inner: want_inner,
        } => match got {
            Type::Ref {
                mutable: got_mut,
                inner: got_inner,
            } if !*want_mut || *got_mut => {
                unify_expected_type(want_inner, got_inner, subst) || type_compatible(got, expected)
            }
            _ => type_compatible(got, expected),
        },
        Type::Closure {
            params: want_params,
            ret: want_ret,
        } => match got {
            Type::Closure {
                params: got_params,
                ret: got_ret,
            } if got_params.len() == want_params.len() => {
                want_params
                    .iter()
                    .zip(got_params.iter())
                    .all(|(want, got)| unify_expected_type(want, got, subst))
                    && unify_expected_type(want_ret, got_ret, subst)
            }
            _ => type_compatible(got, expected),
        },
        _ => type_compatible(got, expected),
    }
}

fn is_placeholder_type(ty: &Type) -> bool {
    match ty {
        Type::Unit => true,
        Type::Ref { inner, .. } => is_placeholder_type(inner),
        Type::Generic { name, args } if (name == "Vec" || name == "HashMap") && args.is_empty() => {
            true
        }
        _ => false,
    }
}

fn is_refinable_placeholder(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Generic { name, args }
            if (name == "Option" || name == "Vec" || name == "HashMap") && args.is_empty()
    )
}
