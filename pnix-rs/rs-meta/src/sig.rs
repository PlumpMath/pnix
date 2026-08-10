//! Canonical AST serialization — a stable, hand-written rendering of the
//! parsed program (every AST variant covered).
//!
//! Two consumers:
//!   - the `ast-canonical` CLI command: a machine-parseable, stability-
//!     guaranteed alternative to `ast` (which prints rustc-derive Debug with
//!     no stability promise);
//!   - the stage3 mirror/fixed-point proofs: this exact source is appended to
//!     the evaluator-core bundle so the same serializer runs at stage1
//!     (rustc-native), stage2, and stage2', and must emit byte-identical
//!     output (see `check::stage3_mirror_check`).
//!
//! Derived `Debug` is deliberately not used: interpreter debug rendering is
//! not byte-faithful to rustc's derive. This file stays inside the evaluated
//! Rust subset for the self-host bundles.

use crate::ast::*;

fn sig_esc(s: &str) -> String {
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
        } else {
            out.push(c);
        }
    }
    out
}

fn sig_type(t: &Type) -> String {
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
        Type::Unit => String::from("unit"),
        Type::Never => String::from("never"),
        Type::Named(n) => format!("N({})", n),
        Type::Generic { name, args } => format!("G({},[{}])", name, sig_types(args)),
        Type::ImplTrait(n) => format!("IT({})", n),
        Type::Tuple(ts) => format!("Tu([{}])", sig_types(ts)),
        Type::Slice(inner) => format!("Sl({})", sig_type(inner)),
        Type::Array(inner, size) => format!("Ar({},{})", sig_type(inner), size),
        Type::Ref { mutable, inner } => {
            let m = if *mutable { "mut" } else { "imm" };
            format!("R({},{})", m, sig_type(inner))
        }
        Type::RefLt { mutable, inner, .. } => {
            let m = if *mutable { "mut" } else { "imm" };
            format!("R({},{})", m, sig_type(inner))
        }
        Type::Closure { params, ret } => format!("Cl([{}],{})", sig_types(params), sig_type(ret)),
    }
}

fn sig_types(ts: &Vec<Type>) -> String {
    let mut parts = Vec::new();
    for t in ts {
        // Lifetime arguments (Named("'a")) are emission-only carriers; the
        // canonical serialization stays lifetime-erased (stability).
        if let Type::Named(n) = t {
            if n.starts_with("'") {
                continue;
            }
        }
        parts.push(sig_type(t));
    }
    parts.join(",")
}

fn sig_opt_type(t: &Option<Type>) -> String {
    match t {
        Some(t2) => sig_type(t2),
        None => String::from("_"),
    }
}

fn sig_params(ps: &Vec<Param>) -> String {
    let mut parts = Vec::new();
    for p in ps {
        parts.push(format!("{}:{}", p.name, sig_type(&p.ty)));
    }
    parts.join(",")
}

fn sig_block(b: &Block) -> String {
    let mut parts = Vec::new();
    for s in &b.stmts {
        parts.push(sig_stmt(s));
    }
    let tail = match &b.tail {
        Some(e) => sig_expr(e),
        None => String::from("_"),
    };
    format!("{{{}|{}}}", parts.join(";"), tail)
}

fn sig_stmt(s: &Stmt) -> String {
    match s {
        Stmt::Let { name, mutable, ty, init } => {
            let m = if *mutable { "mut " } else { "" };
            format!("let {}{}:{}={}", m, name, sig_opt_type(ty), sig_expr(init))
        }
        Stmt::LetPat { pat, init } => format!("letp {}={}", sig_pat(pat), sig_expr(init)),
        Stmt::LetElse { pat, init, else_blk } => format!(
            "lete {}={} else {}",
            sig_pat(pat),
            sig_expr(init),
            sig_block(else_blk)
        ),
        Stmt::Assign { target, value } => format!("asg {}={}", sig_expr(target), sig_expr(value)),
        Stmt::Expr(e) => format!("ex {}", sig_expr(e)),
        Stmt::Return(opt) => match opt {
            Some(e) => format!("ret {}", sig_expr(e)),
            None => String::from("ret _"),
        },
    }
}

fn sig_unop(op: &UnOp) -> String {
    match op {
        UnOp::Neg => String::from("neg"),
        UnOp::Not => String::from("not"),
        UnOp::Deref => String::from("deref"),
    }
}

fn sig_binop(op: &BinOp) -> String {
    match op {
        BinOp::Add => String::from("add"),
        BinOp::Sub => String::from("sub"),
        BinOp::Mul => String::from("mul"),
        BinOp::Div => String::from("div"),
        BinOp::Rem => String::from("rem"),
        BinOp::BitXor => String::from("xor"),
        BinOp::BitAnd => String::from("band"),
        BinOp::BitOr => String::from("bor"),
        BinOp::Shl => String::from("shl"),
        BinOp::Shr => String::from("shr"),
        BinOp::Eq => String::from("eq"),
        BinOp::Ne => String::from("ne"),
        BinOp::Lt => String::from("lt"),
        BinOp::Le => String::from("le"),
        BinOp::Gt => String::from("gt"),
        BinOp::Ge => String::from("ge"),
        BinOp::And => String::from("and"),
        BinOp::Or => String::from("or"),
    }
}

fn sig_exprs(es: &Vec<Expr>) -> String {
    let mut parts = Vec::new();
    for e in es {
        parts.push(sig_expr(e));
    }
    parts.join(",")
}

fn sig_field_inits(fs: &Vec<(String, Expr)>) -> String {
    let mut parts = Vec::new();
    for (n, e) in fs {
        parts.push(format!("{}:{}", n, sig_expr(e)));
    }
    parts.join(",")
}

fn sig_opt_boxed_expr(e: &Option<Box<Expr>>) -> String {
    match e {
        Some(x) => sig_expr(x),
        None => String::from("_"),
    }
}

fn sig_arm(a: &Arm) -> String {
    let g = match &a.guard {
        Some(e) => format!(" if {}", sig_expr(e)),
        None => String::new(),
    };
    format!("{}{}=>{}", sig_pat(&a.pat), g, sig_expr(&a.body))
}

fn sig_arms(arms: &Vec<Arm>) -> String {
    let mut parts = Vec::new();
    for a in arms {
        parts.push(sig_arm(a));
    }
    parts.join(",")
}

fn sig_closure_param(p: &ClosureParam) -> String {
    format!("{}:{}", sig_pat(&p.pat), sig_opt_type(&p.ty))
}

fn sig_expr(e: &Expr) -> String {
    match e {
        Expr::Int(n) => format!("int({})", n),
        Expr::IntHex(n, _text) => format!("int({})", n),
        Expr::Float(text) => format!("float({})", text),
        Expr::Char(c) => format!("char({})", c),
        Expr::Str(s) => format!("str(\"{}\")", sig_esc(s)),
        Expr::Bool(b) => format!("bool({})", b),
        Expr::Var(n) => format!("var({})", n),
        Expr::Ref { mutable, expr } => {
            let m = if *mutable { "mut" } else { "imm" };
            format!("ref({},{})", m, sig_expr(expr))
        }
        Expr::Unary { op, rhs } => format!("un({},{})", sig_unop(op), sig_expr(rhs)),
        Expr::Binary { op, lhs, rhs } => {
            format!("bin({},{},{})", sig_binop(op), sig_expr(lhs), sig_expr(rhs))
        }
        Expr::Cast { expr, ty } => format!("cast({},{})", sig_expr(expr), sig_type(ty)),
        Expr::Try(inner) => format!("try({})", sig_expr(inner)),
        Expr::Return(opt) => format!("retx({})", sig_opt_boxed_expr(opt)),
        Expr::Assign { target, value } => {
            format!("assign({},{})", sig_expr(target), sig_expr(value))
        }
        Expr::Closure { params, ret, body } => {
            let mut parts = Vec::new();
            for p in params {
                parts.push(sig_closure_param(p));
            }
            format!(
                "closure([{}],{},{})",
                parts.join(","),
                sig_opt_type(ret),
                sig_expr(body)
            )
        }
        Expr::Call { name, args } => format!("call({},[{}])", name, sig_exprs(args)),
        Expr::CallExpr { callee, args } => {
            format!("callx({},[{}])", sig_expr(callee), sig_exprs(args))
        }
        Expr::PathCall { type_name, item, args } => {
            format!("pcall({},{},[{}])", type_name, item, sig_exprs(args))
        }
        Expr::MethodCall { receiver, name, type_args, args } => format!(
            "mcall({},{},[{}],[{}])",
            sig_expr(receiver),
            name,
            sig_types(type_args),
            sig_exprs(args)
        ),
        Expr::If { cond, then_blk, else_blk } => {
            let e2 = match else_blk {
                Some(b) => sig_block(b),
                None => String::from("_"),
            };
            format!("if({},{},{})", sig_expr(cond), sig_block(then_blk), e2)
        }
        Expr::Block(b) => format!("blk({})", sig_block(b)),
        Expr::Println { fmt, args } => {
            format!("println(\"{}\",[{}])", sig_esc(fmt), sig_exprs(args))
        }
        Expr::Print { fmt, args } => format!("print(\"{}\",[{}])", sig_esc(fmt), sig_exprs(args)),
        Expr::Eprintln { fmt, args } => {
            format!("eprintln(\"{}\",[{}])", sig_esc(fmt), sig_exprs(args))
        }
        Expr::Format { fmt, args } => format!("format(\"{}\",[{}])", sig_esc(fmt), sig_exprs(args)),
        Expr::Write { newline, target, fmt, args } => {
            let nl = if *newline { "ln" } else { "" };
            format!(
                "write{}({},\"{}\",[{}])",
                nl,
                sig_expr(target),
                sig_esc(fmt),
                sig_exprs(args)
            )
        }
        Expr::Panic { name } => format!("panic({})", name),
        Expr::Assert { cond } => format!("assert({})", sig_expr(cond)),
        Expr::AssertEq { left, right } => {
            format!("asserteq({},{})", sig_expr(left), sig_expr(right))
        }
        Expr::Cfg { name } => format!("cfg({})", name),
        Expr::Matches { expr, pat, guard } => {
            let g = match guard {
                Some(e2) => sig_expr(e2),
                None => String::from("_"),
            };
            format!("matches({},{},{})", sig_expr(expr), sig_pat(pat), g)
        }
        Expr::TupleLit(es) => format!("tuple([{}])", sig_exprs(es)),
        Expr::VecLit(es) => format!("vec([{}])", sig_exprs(es)),
        Expr::VecRepeat { elem, count } => {
            format!("vecrep({},{})", sig_expr(elem), sig_expr(count))
        }
        Expr::StructLit { name, fields } => {
            format!("slit({},[{}])", name, sig_field_inits(fields))
        }
        Expr::EnumCtor { enum_name, variant } => format!("ector({},{})", enum_name, variant),
        Expr::EnumStructLit { enum_name, variant, fields } => format!(
            "eslit({},{},[{}])",
            enum_name,
            variant,
            sig_field_inits(fields)
        ),
        Expr::Field { base, name } => format!("field({},{})", sig_expr(base), name),
        Expr::TupleIndex { base, index } => format!("tidx({},{})", sig_expr(base), index),
        Expr::Index { base, index } => format!("idx({},{})", sig_expr(base), sig_expr(index)),
        Expr::Slice { base, start, end, inclusive } => {
            let inc = if *inclusive { "inc" } else { "exc" };
            format!(
                "slice({},{},{},{})",
                sig_expr(base),
                sig_opt_boxed_expr(start),
                sig_opt_boxed_expr(end),
                inc
            )
        }
        Expr::Range { start, end, inclusive } => {
            let inc = if *inclusive { "inc" } else { "exc" };
            format!("range({},{},{})", sig_expr(start), sig_expr(end), inc)
        }
        Expr::Match { scrut, arms } => format!("match({},[{}])", sig_expr(scrut), sig_arms(arms)),
        Expr::While { cond, body } => format!("while({},{})", sig_expr(cond), sig_block(body)),
        Expr::WhileLet { pat, expr, body } => format!(
            "whilelet({},{},{})",
            sig_pat(pat),
            sig_expr(expr),
            sig_block(body)
        ),
        Expr::Loop { body } => format!("loop({})", sig_block(body)),
        Expr::For { var, start, end, inclusive, body } => {
            let inc = if *inclusive { "inc" } else { "exc" };
            format!(
                "for({},{},{},{},{})",
                var,
                sig_expr(start),
                sig_expr(end),
                inc,
                sig_block(body)
            )
        }
        Expr::ForEach { pat, iter, body } => format!(
            "foreach({},{},{})",
            sig_pat(pat),
            sig_expr(iter),
            sig_block(body)
        ),
        Expr::Break { label, value } => {
            let l = match label {
                Some(x) => x.clone(),
                None => String::from("_"),
            };
            format!("break[{}]({})", l, sig_opt_boxed_expr(value))
        }
        Expr::Continue => String::from("continue"),
        Expr::Labeled { label, body } => format!("labeled({},{})", label, sig_expr(body)),
    }
}

fn sig_field_pats(fs: &Vec<(String, Pattern)>) -> String {
    let mut parts = Vec::new();
    for (n, p) in fs {
        parts.push(format!("{}:{}", n, sig_pat(p)));
    }
    parts.join(",")
}

fn sig_pats(ps: &Vec<Pattern>) -> String {
    let mut parts = Vec::new();
    for p in ps {
        parts.push(sig_pat(p));
    }
    parts.join(",")
}

fn sig_pat(p: &Pattern) -> String {
    match p {
        Pattern::Wild => String::from("wild"),
        Pattern::Bind(n) => format!("bind({})", n),
        Pattern::Int(n) => format!("pint({})", n),
        Pattern::Char(c) => format!("pchar({})", c),
        Pattern::Str(s) => format!("pstr(\"{}\")", sig_esc(s)),
        Pattern::Bool(b) => format!("pbool({})", b),
        Pattern::IntRange { start, end } => format!("pirange({},{})", start, end),
        Pattern::CharRange { start, end } => format!("pcrange({},{})", start, end),
        Pattern::BindAt { name, sub } => format!("bindat({},{})", name, sig_pat(sub)),
        Pattern::BindRef { name, mutable } => {
            let m = if *mutable { "mut" } else { "imm" };
            format!("bindref({},{})", name, m)
        }
        Pattern::Tuple(ps) => format!("ptuple([{}])", sig_pats(ps)),
        Pattern::Slice {
            prefix,
            rest,
            suffix,
        } => {
            let r = match rest {
                None => "none".to_string(),
                Some(None) => "rest".to_string(),
                Some(Some(name)) => format!("rest({})", name),
            };
            format!(
                "pslice([{}],{},[{}])",
                sig_pats(prefix),
                r,
                sig_pats(suffix)
            )
        }
        Pattern::Or(ps) => format!("por([{}])", sig_pats(ps)),
        Pattern::Ref { mutable, sub } => {
            let m = if *mutable { "mut" } else { "imm" };
            format!("pref({},{})", m, sig_pat(sub))
        }
        Pattern::Struct { name, fields, rest } => {
            let r = if *rest { "rest" } else { "norest" };
            format!("pstruct({},[{}],{})", name, sig_field_pats(fields), r)
        }
        Pattern::Enum { enum_name, variant, sub } => {
            format!("penum({},{},[{}])", enum_name, variant, sig_pats(sub))
        }
        Pattern::EnumStruct { enum_name, variant, fields, rest } => {
            let r = if *rest { "rest" } else { "norest" };
            format!(
                "penumstruct({},{},[{}],{})",
                enum_name,
                variant,
                sig_field_pats(fields),
                r
            )
        }
    }
}

/// Generic parameter list `<T,U>` (empty string when there are none). Kept in
/// the canonical serialization so ast-canonical is FAITHFUL on the generic
/// surface — E1c made emit generic-complete; this makes sig match (distinct
/// generic programs get distinct sigs).
fn sig_generics(generics: &Vec<String>) -> String {
    if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(","))
    }
}

pub fn sig_program(p: &Program) -> String {
    let mut out = String::new();
    for a in &p.aliases {
        out.push_str(&format!("type {}={};", a.name, sig_type(&a.ty)));
    }
    for g in &p.globals {
        out.push_str(&format!(
            "global {}:{}={};",
            g.name,
            sig_type(&g.ty),
            sig_expr(&g.init)
        ));
    }
    for s in &p.structs {
        if s.unit {
            out.push_str(&format!("struct {}{};", s.name, sig_generics(&s.generics)));
            continue;
        }
        if s.tuple {
            let mut tys = Vec::new();
            for (_, t) in &s.fields {
                tys.push(sig_type(t));
            }
            out.push_str(&format!(
                "struct {}{}({});",
                s.name,
                sig_generics(&s.generics),
                tys.join(",")
            ));
            continue;
        }
        let mut fs = Vec::new();
        for (n, t) in &s.fields {
            fs.push(format!("{}:{}", n, sig_type(t)));
        }
        out.push_str(&format!(
            "struct {}{}{{{}}};",
            s.name,
            sig_generics(&s.generics),
            fs.join(",")
        ));
    }
    for e in &p.enums {
        let mut vs = Vec::new();
        for v in &e.variants {
            let mut ns = Vec::new();
            for (n, t) in &v.named_fields {
                ns.push(format!("{}:{}", n, sig_type(t)));
            }
            vs.push(format!("{}({})[{}]", v.name, sig_types(&v.fields), ns.join(",")));
        }
        out.push_str(&format!(
            "enum {}{}{{{}}};",
            e.name,
            sig_generics(&e.generics),
            vs.join(",")
        ));
    }
    for tr in &p.traits {
        let mut tms = Vec::new();
        for d in &tr.decls {
            tms.push(format!(
                "decl {}({})->{}",
                d.name,
                sig_params(&d.params),
                sig_type(&d.ret)
            ));
        }
        for m in &tr.methods {
            tms.push(format!(
                "{}({})->{} {}",
                m.name,
                sig_params(&m.params),
                sig_type(&m.ret),
                sig_block(&m.body)
            ));
        }
        out.push_str(&format!("trait {}[{}];", tr.name, tms.join(",")));
    }
    for i in &p.impls {
        let mut ms = Vec::new();
        for c in &i.consts {
            ms.push(format!("const {}:{}={}", c.name, sig_type(&c.ty), sig_expr(&c.init)));
        }
        for m in &i.methods {
            let recv = match &m.receiver {
                Some(k) => match k {
                    ReceiverKind::Value => "self",
                    ReceiverKind::Ref => "&self",
                    ReceiverKind::RefMut => "&mut self",
                },
                None => "assoc",
            };
            ms.push(format!(
                "{}{}[{}]({})->{} {}",
                m.name,
                sig_generics(&m.generics),
                recv,
                sig_params(&m.params),
                sig_type(&m.ret),
                sig_block(&m.body)
            ));
        }
        let trait_pfx = match &i.trait_name {
            Some(tn) => format!("for({})", tn),
            None => String::new(),
        };
        out.push_str(&format!(
            "impl{} {}{}{{{}}};",
            sig_generics(&i.generics),
            sig_type(&i.target),
            trait_pfx,
            ms.join(",")
        ));
    }
    for f in &p.funcs {
        out.push_str(&format!(
            "fn {}{}({})->{} {};",
            f.name,
            sig_generics(&f.generics),
            sig_params(&f.params),
            sig_type(&f.ret),
            sig_block(&f.body)
        ));
    }
    out
}
