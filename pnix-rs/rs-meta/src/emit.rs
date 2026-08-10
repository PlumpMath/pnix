//! Native Rust emitter: AST -> Rust source text.
//!
//! This is the plan's emit layer: instead of handing the *original* source to
//! rustc, the native tier can regenerate Rust from the parsed AST and compile
//! that. Two proof surfaces follow:
//!   - roundtrip: parse(emit(parse(src))) is structurally identical to
//!     parse(src)  (the AST representation is complete for the subset), and
//!   - emit parity: interp(src) == interp(emit) == rustc(emit) == rustc(src).
//!
//! Emission favors explicit parentheses over precedence bookkeeping: operands
//! of binary/unary/cast expressions and control-flow heads are parenthesized so
//! the output is unambiguous for both rustc and the rs-meta parser. The emitter
//! is part of the self-host source surface, so it stays inside the evaluated
//! subset.

use crate::ast::*;

pub fn emit_program(p: &Program) -> String {
    let mut out = String::new();
    for u in &p.uses {
        out.push_str(u);
        out.push('\n');
    }
    for a in &p.aliases {
        out.push_str(&format!("type {} = {};\n", a.name, emit_type(&a.ty)));
    }
    for g in &p.globals {
        out.push_str(&format!(
            "const {}: {} = {};\n",
            g.name,
            emit_type(&g.ty),
            emit_expr(&g.init)
        ));
    }
    for s in &p.structs {
        out.push_str(&emit_derives(&s.derives));
        if s.unit {
            out.push_str(&format!("struct {}{};\n", s.name, emit_generics(&s.generics)));
            continue;
        }
        if s.tuple {
            let mut tys = Vec::new();
            for (_, t) in &s.fields {
                tys.push(emit_type(t));
            }
            out.push_str(&format!(
                "struct {}{}({});\n",
                s.name,
                emit_generics(&s.generics),
                tys.join(", ")
            ));
            continue;
        }
        let mut fields = Vec::new();
        for (n, t) in &s.fields {
            fields.push(format!("{}: {}", n, emit_type(t)));
        }
        out.push_str(&format!(
            "struct {}{} {{ {} }}\n",
            s.name,
            emit_generics(&s.generics),
            fields.join(", ")
        ));
    }
    for e in &p.enums {
        out.push_str(&emit_derives(&e.derives));
        let mut variants = Vec::new();
        for v in &e.variants {
            variants.push(emit_variant(v));
        }
        out.push_str(&format!(
            "enum {}{} {{ {} }}\n",
            e.name,
            emit_generics(&e.generics),
            variants.join(", ")
        ));
    }
    for tr in &p.traits {
        out.push_str(&format!("trait {} {{\n", tr.name));
        for d in &tr.decls {
            out.push_str(&emit_trait_decl(d));
            out.push('\n');
        }
        for m in &tr.methods {
            out.push_str(&emit_method(m));
            out.push('\n');
        }
        out.push_str("}\n");
    }
    for i in &p.impls {
        let head = match &i.trait_name {
            Some(tn) => {
                let tq = if tn == "fmt::Display" || tn == "Display" {
                    String::from("std::fmt::Display")
                } else {
                    tn.clone()
                };
                format!(
                    "impl{} {} for {} {{\n",
                    emit_generics(&i.generics),
                    tq,
                    emit_type(&i.target)
                )
            }
            None => format!(
                "impl{} {} {{\n",
                emit_generics(&i.generics),
                emit_type(&i.target)
            ),
        };
        out.push_str(&head);
        for c in &i.consts {
            out.push_str(&format!(
                "const {}: {} = {};\n",
                c.name,
                emit_type(&c.ty),
                emit_expr(&c.init)
            ));
        }
        for m in &i.methods {
            out.push_str(&emit_method(m));
            out.push('\n');
        }
        out.push_str("}\n");
    }
    for f in &p.funcs {
        out.push_str(&emit_fn(f));
        out.push('\n');
    }
    out
}

fn emit_derives(derives: &Vec<String>) -> String {
    if derives.is_empty() {
        String::new()
    } else {
        format!("#[derive({})]\n", derives.join(", "))
    }
}

/// Known std types the parser canonicalizes to bare names; emitted call paths
/// are re-qualified so the output compiles without a matching `use` item.
fn qualify_type_name(name: &str) -> String {
    if name == "fmt::Formatter" || name == "Formatter" {
        String::from("std::fmt::Formatter")
    } else if name == "fmt::Result" {
        String::from("std::fmt::Result")
    } else if name == "Rc" {
        String::from("std::rc::Rc")
    } else if name == "HashMap" {
        String::from("std::collections::HashMap")
    } else if name == "PathBuf" {
        String::from("std::path::PathBuf")
    } else if name == "ExitCode" {
        String::from("std::process::ExitCode")
    } else if name == "fs" {
        String::from("std::fs")
    } else if name == "env" {
        String::from("std::env")
    } else {
        String::from(name)
    }
}

fn emit_variant(v: &Variant) -> String {
    if !v.named_fields.is_empty() {
        let mut fields = Vec::new();
        for (n, t) in &v.named_fields {
            fields.push(format!("{}: {}", n, emit_type(t)));
        }
        format!("{} {{ {} }}", v.name, fields.join(", "))
    } else if !v.fields.is_empty() {
        let mut tys = Vec::new();
        for t in &v.fields {
            tys.push(emit_type(t));
        }
        format!("{}({})", v.name, tys.join(", "))
    } else {
        v.name.clone()
    }
}

fn emit_generics(generics: &Vec<String>) -> String {
    if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    }
}

fn emit_fn(f: &Func) -> String {
    format!(
        "fn {}{}({}){} {}",
        f.name,
        emit_generics(&f.generics),
        emit_params(&f.params),
        emit_ret(&f.ret),
        emit_block(&f.body)
    )
}

fn emit_trait_decl(d: &TraitDecl) -> String {
    let mut parts = Vec::new();
    match &d.receiver {
        Some(k) => match k {
            ReceiverKind::Value => parts.push(String::from("self")),
            ReceiverKind::Ref => parts.push(String::from("&self")),
            ReceiverKind::RefMut => parts.push(String::from("&mut self")),
        },
        None => {}
    }
    for p in &d.params {
        parts.push(format!("{}: {}", p.name, emit_type(&p.ty)));
    }
    format!(
        "fn {}{}({}){};",
        d.name,
        emit_generics(&d.generics),
        parts.join(", "),
        emit_ret(&d.ret)
    )
}

fn emit_method(m: &Method) -> String {
    let mut parts = Vec::new();
    match &m.receiver {
        Some(k) => match k {
            ReceiverKind::Value => parts.push(String::from("self")),
            ReceiverKind::Ref => parts.push(String::from("&self")),
            ReceiverKind::RefMut => parts.push(String::from("&mut self")),
        },
        None => {}
    }
    for p in &m.params {
        parts.push(format!("{}: {}", p.name, emit_type(&p.ty)));
    }
    format!(
        "fn {}{}({}){} {}",
        m.name,
        emit_generics(&m.generics),
        parts.join(", "),
        emit_ret(&m.ret),
        emit_block(&m.body)
    )
}

fn emit_params(ps: &Vec<Param>) -> String {
    let mut parts = Vec::new();
    for p in ps {
        parts.push(format!("{}: {}", p.name, emit_type(&p.ty)));
    }
    parts.join(", ")
}

fn emit_ret(t: &Type) -> String {
    match t {
        Type::Unit => String::new(),
        other => format!(" -> {}", emit_type(other)),
    }
}

pub fn emit_type(t: &Type) -> String {
    match t {
        Type::I64 => String::from("i64"),
        Type::IntLit => String::from("i64"),
        Type::F64 => String::from("f64"),
        Type::I32 => String::from("i32"),
        Type::U32 => String::from("u32"),
        Type::U64 => String::from("u64"),
        Type::U8 => String::from("u8"),
        Type::Usize => String::from("usize"),
        Type::Char => String::from("char"),
        Type::Bool => String::from("bool"),
        Type::Unit => String::from("()"),
        Type::Never => String::from("!"),
        // E1b: the parser canonicalizes std paths to bare names; emitted
        // programs are compiled standalone (no `use` from the source), so
        // TYPE positions re-qualify exactly like call paths do.
        Type::Named(n) => qualify_type_name(n),
        Type::Generic { name, args } => {
            if args.is_empty() {
                return qualify_type_name(name);
            }
            let mut parts = Vec::new();
            for a in args {
                parts.push(emit_type(a));
            }
            format!("{}<{}>", qualify_type_name(name), parts.join(", "))
        }
        Type::ImplTrait(n) => format!("impl {}", n),
        Type::Tuple(ts) => {
            let mut parts = Vec::new();
            for a in ts {
                parts.push(emit_type(a));
            }
            format!("({})", parts.join(", "))
        }
        Type::Slice(inner) => format!("[{}]", emit_type(inner)),
        Type::Array(inner, size) => format!("[{}; {}]", emit_type(inner), size),
        Type::Ref { mutable, inner } => {
            if *mutable {
                format!("&mut {}", emit_type(inner))
            } else {
                format!("&{}", emit_type(inner))
            }
        }
        Type::RefLt { lifetime, mutable, inner } => {
            if *mutable {
                format!("&'{} mut {}", lifetime, emit_type(inner))
            } else {
                format!("&'{} {}", lifetime, emit_type(inner))
            }
        }
        Type::Closure { params, ret } => {
            let mut parts = Vec::new();
            for a in params {
                parts.push(emit_type(a));
            }
            format!("impl Fn({}) -> {}", parts.join(", "), emit_type(ret))
        }
    }
}

pub fn emit_block(b: &Block) -> String {
    let mut out = String::from("{ ");
    for s in &b.stmts {
        out.push_str(&emit_stmt(s));
        out.push(' ');
    }
    match &b.tail {
        Some(e) => {
            out.push_str(&emit_expr(e));
            out.push(' ');
        }
        None => {}
    }
    out.push('}');
    out
}

fn emit_stmt(s: &Stmt) -> String {
    match s {
        Stmt::Let { name, mutable, ty, init } => {
            let m = if *mutable { "mut " } else { "" };
            let ann = match ty {
                Some(t) => format!(": {}", emit_type(t)),
                None => String::new(),
            };
            // A `[T; N]` annotation needs an ARRAY initializer (`[..]`), not the
            // `vec![..]` that a bare `VecLit`/`VecRepeat` otherwise emits to
            // (both parse to the same node) -- else rustc sees `[i64;3] = vec![]`.
            let init_str = match ty {
                Some(Type::Array(_, _)) => match init {
                    Expr::VecLit(items) => format!("[{}]", emit_args(items)),
                    Expr::VecRepeat { elem, count } => {
                        format!("[{}; {}]", emit_expr(elem), emit_expr(count))
                    }
                    _ => emit_expr(init),
                },
                _ => emit_expr(init),
            };
            format!("let {}{}{} = {};", m, name, ann, init_str)
        }
        Stmt::LetPat { pat, init } => {
            format!("let {} = {};", emit_pattern(pat), emit_expr(init))
        }
        Stmt::LetElse { pat, init, else_blk } => format!(
            "let {} = {} else {};",
            emit_pattern(pat),
            emit_expr(init),
            emit_block(else_blk)
        ),
        Stmt::Assign { target, value } => {
            format!("{} = {};", emit_expr(target), emit_expr(value))
        }
        Stmt::Expr(e) => format!("{};", emit_expr(e)),
        Stmt::Return(opt) => match opt {
            Some(e) => format!("return {};", emit_expr(e)),
            None => String::from("return;"),
        },
    }
}

/// Emit an expression, parenthesized when used as an operand of another
/// expression so precedence never has to be tracked.
fn emit_operand(e: &Expr) -> String {
    match e {
        Expr::Int(n) => {
            if *n < 0 {
                format!("({})", n)
            } else {
                n.to_string()
            }
        }
        Expr::IntHex(_n, text) => text.clone(),
        Expr::Float(text) => text.clone(),
        Expr::Bool(_) | Expr::Char(_) | Expr::Str(_) | Expr::Var(_) => emit_expr(e),
        Expr::TupleLit(_) | Expr::VecLit(_) | Expr::VecRepeat { .. } | Expr::Block(_) => {
            emit_expr(e)
        }
        Expr::Call { .. }
        | Expr::PathCall { .. }
        | Expr::MethodCall { .. }
        | Expr::CallExpr { .. }
        | Expr::Field { .. }
        | Expr::TupleIndex { .. }
        | Expr::Index { .. }
        | Expr::EnumCtor { .. }
        | Expr::Println { .. }
        | Expr::Print { .. }
        | Expr::Eprintln { .. }
        | Expr::Format { .. }
        | Expr::Matches { .. }
        | Expr::Cfg { .. } => emit_expr(e),
        other => format!("({})", emit_expr(other)),
    }
}

pub fn emit_expr(e: &Expr) -> String {
    match e {
        Expr::Int(n) => n.to_string(),
        Expr::IntHex(_n, text) => text.clone(),
        Expr::Float(text) => text.clone(),
        Expr::Char(c) => format!("'{}'", esc_char(*c)),
        Expr::Str(s) => format!("\"{}\"", esc_str(s)),
        Expr::Bool(b) => b.to_string(),
        Expr::Var(n) => n.clone(),
        Expr::Ref { mutable, expr } => {
            if *mutable {
                format!("&mut {}", emit_operand(expr))
            } else {
                format!("&{}", emit_operand(expr))
            }
        }
        Expr::Unary { op, rhs } => {
            let sym = match op {
                UnOp::Neg => "-",
                UnOp::Not => "!",
                UnOp::Deref => "*",
            };
            format!("{}{}", sym, emit_operand(rhs))
        }
        Expr::Binary { op, lhs, rhs } => format!(
            "{} {} {}",
            emit_operand(lhs),
            emit_binop(op),
            emit_operand(rhs)
        ),
        Expr::Cast { expr, ty } => format!("{} as {}", emit_operand(expr), emit_type(ty)),
        Expr::Try(inner) => format!("{}?", emit_operand(inner)),
        Expr::Return(opt) => match opt {
            Some(v) => format!("return {}", emit_expr(v)),
            None => String::from("return"),
        },
        Expr::Assign { target, value } => {
            format!("{} = {}", emit_expr(target), emit_expr(value))
        }
        Expr::Closure { params, ret, body } => {
            let mut parts = Vec::new();
            for p in params {
                parts.push(emit_closure_param(p));
            }
            match ret {
                Some(t) => {
                    let body_src = match body.as_ref() {
                        Expr::Block(b) => emit_block(b),
                        other => format!("{{ {} }}", emit_expr(other)),
                    };
                    format!("|{}| -> {} {}", parts.join(", "), emit_type(t), body_src)
                }
                None => format!("|{}| {}", parts.join(", "), emit_operand(body)),
            }
        }
        Expr::Call { name, args } => format!("{}({})", name, emit_args(args)),
        Expr::CallExpr { callee, args } => {
            format!("({})({})", emit_expr(callee), emit_args(args))
        }
        Expr::PathCall { type_name, item, args } => {
            format!("{}::{}({})", qualify_type_name(type_name), item, emit_args(args))
        }
        Expr::MethodCall { receiver, name, type_args, args } => {
            let turbofish = if type_args.is_empty() {
                String::new()
            } else {
                let mut parts = Vec::new();
                for t in type_args {
                    parts.push(emit_type(t));
                }
                format!("::<{}>", parts.join(", "))
            };
            format!(
                "{}.{}{}({})",
                emit_receiver(receiver),
                name,
                turbofish,
                emit_args(args)
            )
        }
        Expr::If { cond, then_blk, else_blk } => {
            let head = format!("if {} {}", emit_operand(cond), emit_block(then_blk));
            match else_blk {
                Some(b) => format!("{} else {}", head, emit_block(b)),
                None => head,
            }
        }
        Expr::Block(b) => emit_block(b),
        Expr::Println { fmt, args } => emit_macro("println", fmt, args),
        Expr::Print { fmt, args } => emit_macro("print", fmt, args),
        Expr::Eprintln { fmt, args } => emit_macro("eprintln", fmt, args),
        Expr::Format { fmt, args } => emit_macro("format", fmt, args),
        Expr::Write { newline, target, fmt, args } => {
            let name = if *newline { "writeln" } else { "write" };
            let rest = if args.is_empty() {
                String::new()
            } else {
                format!(", {}", emit_args(args))
            };
            format!(
                "{}!({}, \"{}\"{})",
                name,
                emit_expr(target),
                esc_fmt(fmt),
                rest
            )
        }
        Expr::Panic { name } => format!("{}!()", name),
        Expr::Assert { cond } => format!("assert!({})", emit_expr(cond)),
        Expr::AssertEq { left, right } => {
            format!("assert_eq!({}, {})", emit_expr(left), emit_expr(right))
        }
        Expr::Cfg { name } => format!("cfg!({})", name),
        Expr::Matches { expr, pat, guard } => match guard {
            Some(g) => format!(
                "matches!({}, {} if {})",
                emit_expr(expr),
                emit_pattern(pat),
                emit_expr(g)
            ),
            None => format!("matches!({}, {})", emit_expr(expr), emit_pattern(pat)),
        },
        Expr::TupleLit(items) => {
            if items.is_empty() {
                String::from("()")
            } else if items.len() == 1 {
                format!("({},)", emit_expr(&items[0]))
            } else {
                format!("({})", emit_args(items))
            }
        }
        Expr::VecLit(items) => format!("vec![{}]", emit_args(items)),
        Expr::VecRepeat { elem, count } => {
            format!("vec![{}; {}]", emit_expr(elem), emit_expr(count))
        }
        Expr::StructLit { name, fields } => {
            let mut parts = Vec::new();
            for (n, v) in fields {
                parts.push(format!("{}: {}", n, emit_expr(v)));
            }
            format!("{} {{ {} }}", name, parts.join(", "))
        }
        Expr::EnumCtor { enum_name, variant } => {
            format!("{}::{}", qualify_type_name(enum_name), variant)
        }
        Expr::EnumStructLit { enum_name, variant, fields } => {
            let mut parts = Vec::new();
            for (n, v) in fields {
                parts.push(format!("{}: {}", n, emit_expr(v)));
            }
            format!("{}::{} {{ {} }}", enum_name, variant, parts.join(", "))
        }
        Expr::Field { base, name } => format!("{}.{}", emit_receiver(base), name),
        Expr::TupleIndex { base, index } => format!("{}.{}", emit_receiver(base), index),
        Expr::Index { base, index } => {
            format!("{}[{}]", emit_receiver(base), emit_expr(index))
        }
        Expr::Slice { base, start, end, inclusive } => {
            let s = match start {
                Some(v) => emit_operand(v),
                None => String::new(),
            };
            let t = match end {
                Some(v) => emit_operand(v),
                None => String::new(),
            };
            let dots = if *inclusive { "..=" } else { ".." };
            format!("{}[{}{}{}]", emit_receiver(base), s, dots, t)
        }
        Expr::Range { start, end, inclusive } => {
            let dots = if *inclusive { "..=" } else { ".." };
            format!("{}{}{}", emit_operand(start), dots, emit_operand(end))
        }
        Expr::Match { scrut, arms } => {
            let mut out = format!("match {} {{ ", emit_operand(scrut));
            for a in arms {
                match &a.guard {
                    Some(g) => out.push_str(&format!(
                        "{} if {} => {}, ",
                        emit_pattern(&a.pat),
                        emit_expr(g),
                        emit_expr(&a.body)
                    )),
                    None => out.push_str(&format!(
                        "{} => {}, ",
                        emit_pattern(&a.pat),
                        emit_expr(&a.body)
                    )),
                }
            }
            out.push('}');
            out
        }
        Expr::While { cond, body } => {
            format!("while {} {}", emit_operand(cond), emit_block(body))
        }
        Expr::WhileLet { pat, expr, body } => format!(
            "while let {} = {} {}",
            emit_pattern(pat),
            emit_operand(expr),
            emit_block(body)
        ),
        Expr::Loop { body } => format!("loop {}", emit_block(body)),
        Expr::For { var, start, end, inclusive, body } => {
            let dots = if *inclusive { "..=" } else { ".." };
            format!(
                "for {} in {}{}{} {}",
                var,
                emit_operand(start),
                dots,
                emit_operand(end),
                emit_block(body)
            )
        }
        Expr::ForEach { pat, iter, body } => format!(
            "for {} in {} {}",
            emit_pattern(pat),
            emit_operand(iter),
            emit_block(body)
        ),
        Expr::Break { label, value } => {
            let lbl = match label {
                Some(l) => format!(" '{}", l),
                None => String::new(),
            };
            match value {
                Some(v) => format!("break{} {}", lbl, emit_expr(v)),
                None => format!("break{}", lbl),
            }
        }
        Expr::Continue => String::from("continue"),
        Expr::Labeled { label, body } => format!("'{}: {}", label, emit_expr(body)),
    }
}

/// Receivers of `.field` / `.0` / `[i]` / method calls: postfix-safe forms can
/// stay bare, everything else gets parentheses.
fn emit_receiver(e: &Expr) -> String {
    match e {
        Expr::Var(_)
        | Expr::Call { .. }
        | Expr::PathCall { .. }
        | Expr::MethodCall { .. }
        | Expr::Field { .. }
        | Expr::TupleIndex { .. }
        | Expr::Index { .. }
        | Expr::Str(_)
        | Expr::Format { .. }
        | Expr::EnumCtor { .. }
        | Expr::TupleLit(_)
        | Expr::VecLit(_)
        | Expr::Slice { .. } => emit_expr(e),
        other => format!("({})", emit_expr(other)),
    }
}

fn emit_args(args: &Vec<Expr>) -> String {
    let mut parts = Vec::new();
    for a in args {
        parts.push(emit_expr(a));
    }
    parts.join(", ")
}

fn emit_macro(name: &str, fmt: &str, args: &Vec<Expr>) -> String {
    if args.is_empty() {
        format!("{}!(\"{}\")", name, esc_fmt(fmt))
    } else {
        format!("{}!(\"{}\", {})", name, esc_fmt(fmt), emit_args(args))
    }
}

fn emit_closure_param(p: &ClosureParam) -> String {
    match &p.ty {
        Some(t) => format!("{}: {}", emit_pattern(&p.pat), emit_type(t)),
        None => emit_pattern(&p.pat),
    }
}

fn emit_binop(op: &BinOp) -> String {
    let sym = match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::BitXor => "^",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    };
    String::from(sym)
}

pub fn emit_pattern(p: &Pattern) -> String {
    match p {
        Pattern::Wild => String::from("_"),
        Pattern::Bind(n) => n.clone(),
        Pattern::Int(n) => n.to_string(),
        Pattern::Char(c) => format!("'{}'", esc_char(*c)),
        Pattern::Str(s) => format!("\"{}\"", esc_str(s)),
        Pattern::Bool(b) => b.to_string(),
        Pattern::IntRange { start, end } => format!("{}..={}", start, end),
        Pattern::CharRange { start, end } => {
            format!("'{}'..='{}'", esc_char(*start), esc_char(*end))
        }
        Pattern::BindAt { name, sub } => format!("{} @ {}", name, emit_pattern(sub)),
        Pattern::BindRef { name, mutable } => {
            if *mutable {
                format!("ref mut {}", name)
            } else {
                format!("ref {}", name)
            }
        }
        Pattern::Tuple(ps) => {
            if ps.len() == 1 {
                format!("({},)", emit_pattern(&ps[0]))
            } else {
                format!("({})", emit_patterns(ps))
            }
        }
        Pattern::Slice {
            prefix,
            rest,
            suffix,
        } => {
            let mut parts: Vec<String> = Vec::new();
            for p in prefix {
                parts.push(emit_pattern(p));
            }
            match rest {
                None => {}
                Some(None) => parts.push("..".to_string()),
                Some(Some(name)) => parts.push(format!("{} @ ..", name)),
            }
            for p in suffix {
                parts.push(emit_pattern(p));
            }
            format!("[{}]", parts.join(", "))
        }
        Pattern::Or(ps) => {
            let mut parts = Vec::new();
            for sub in ps {
                parts.push(emit_pattern(sub));
            }
            parts.join(" | ")
        }
        Pattern::Ref { mutable, sub } => {
            if *mutable {
                format!("&mut {}", emit_pattern(sub))
            } else {
                format!("&{}", emit_pattern(sub))
            }
        }
        Pattern::Struct { name, fields, rest } => {
            emit_struct_pattern(name, fields, *rest)
        }
        Pattern::Enum { enum_name, variant, sub } => {
            if sub.is_empty() {
                format!("{}::{}", enum_name, variant)
            } else {
                format!("{}::{}({})", enum_name, variant, emit_patterns(sub))
            }
        }
        Pattern::EnumStruct { enum_name, variant, fields, rest } => {
            let path = format!("{}::{}", enum_name, variant);
            emit_struct_pattern(&path, fields, *rest)
        }
    }
}

fn emit_struct_pattern(path: &str, fields: &Vec<(String, Pattern)>, rest: bool) -> String {
    let mut parts = Vec::new();
    for (n, sub) in fields {
        let sub_src = emit_pattern(sub);
        if sub_src == *n {
            parts.push(n.clone());
        } else {
            parts.push(format!("{}: {}", n, sub_src));
        }
    }
    if rest {
        parts.push(String::from(".."));
    }
    format!("{} {{ {} }}", path, parts.join(", "))
}

fn emit_patterns(ps: &Vec<Pattern>) -> String {
    let mut parts = Vec::new();
    for p in ps {
        parts.push(emit_pattern(p));
    }
    parts.join(", ")
}

fn esc_str(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c == '\\' {
            out.push_str("\\\\");
        } else if c == '"' {
            out.push_str("\\\"");
        } else if c == '\n' {
            out.push_str("\\n");
        } else if c == '\t' {
            out.push_str("\\t");
        } else if c == '\r' {
            out.push_str("\\r");
        } else {
            out.push(c);
        }
    }
    out
}

/// Format strings keep `{}` placeholders and `{{`/`}}` escapes as stored; only
/// string-literal escapes are re-applied.
fn esc_fmt(s: &str) -> String {
    esc_str(s)
}

fn esc_char(c: char) -> String {
    if c == '\\' {
        String::from("\\\\")
    } else if c == '\'' {
        String::from("\\'")
    } else if c == '\n' {
        String::from("\\n")
    } else if c == '\t' {
        String::from("\\t")
    } else if c == '\r' {
        String::from("\\r")
    } else {
        c.to_string()
    }
}
