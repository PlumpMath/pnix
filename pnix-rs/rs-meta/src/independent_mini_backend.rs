//! Tiny independent Rust-subset interpreter.
//!
//! Trusting-Trust (Diverse Double-Compiling) witness: a hand-written
//! tokenizer/parser/AST/tree-walking interpreter for a small `i64` Rust
//! subset, sharing zero code with `lexer.rs`/`parser.rs`/`ast.rs`/
//! `typeck.rs`/`interp.rs` (rs-meta's own evaluator core, the thing
//! `tv-check` already proves `== rustc`). Cross-checked in `check.rs`
//! against real `rustc` (via `native::native_run`), independently of
//! rs-meta's own interpreter. `rustc` itself remains the trusted oracle
//! here, the same honest role real upstream Hy plays for the Python host's
//! `independent_mini_backend.py` and the self-hosted compiler plays for the
//! ClojureScript host's `independent_mini_backend.js`.
//!
//! It is a frontier witness, not a replacement for rs-meta's own
//! interpreter: it covers a bounded `fn`/`if`/`let`/`while`/assignment/
//! closure/arithmetic/comparison/call fixture set, not the Rust language
//! `interp.rs` targets.

use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
enum MiniTok {
    Ident(String),
    Int(i64),
    Str(String),
    Punct(&'static str),
}

fn tokenize(src: &str) -> Result<Vec<MiniTok>, String> {
    let bytes: Vec<char> = src.chars().collect();
    let mut i = 0usize;
    let mut out = Vec::new();
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '"' {
            let mut j = i + 1;
            let mut s = String::new();
            while j < bytes.len() && bytes[j] != '"' {
                s.push(bytes[j]);
                j += 1;
            }
            if j >= bytes.len() {
                return Err("tiny lexer: unterminated string".to_string());
            }
            out.push(MiniTok::Str(s));
            i = j + 1;
            continue;
        }
        if c.is_ascii_digit() {
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let text: String = bytes[i..j].iter().collect();
            out.push(MiniTok::Int(
                text.parse::<i64>().map_err(|_| "tiny lexer: bad int")?,
            ));
            i = j;
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let mut j = i;
            while j < bytes.len() && (bytes[j].is_alphanumeric() || bytes[j] == '_') {
                j += 1;
            }
            out.push(MiniTok::Ident(bytes[i..j].iter().collect()));
            i = j;
            continue;
        }
        // two-char punctuation first
        if i + 1 < bytes.len() {
            let two: String = bytes[i..i + 2].iter().collect();
            if let Some(p) = match two.as_str() {
                "->" => Some("->"),
                "<=" => Some("<="),
                ">=" => Some(">="),
                "==" => Some("=="),
                _ => None,
            } {
                out.push(MiniTok::Punct(p));
                i += 2;
                continue;
            }
        }
        let one = match c {
            '(' => "(",
            ')' => ")",
            '{' => "{",
            '}' => "}",
            ',' => ",",
            ';' => ";",
            ':' => ":",
            '+' => "+",
            '-' => "-",
            '*' => "*",
            '<' => "<",
            '>' => ">",
            '!' => "!",
            '=' => "=",
            '|' => "|",
            _ => return Err(format!("tiny lexer: unexpected char {:?}", c)),
        };
        out.push(MiniTok::Punct(one));
        i += 1;
    }
    Ok(out)
}

#[derive(Debug, Clone)]
enum MiniBinOp {
    Add,
    Sub,
    Mul,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
}

#[derive(Debug, Clone)]
enum MiniExpr {
    Int(i64),
    Var(String),
    Bin(MiniBinOp, Box<MiniExpr>, Box<MiniExpr>),
    If(Box<MiniExpr>, Box<MiniBlock>, Box<MiniBlock>),
    // The callee is always a bare name, never an arbitrary expression --
    // real Rust also allows calling e.g. a parenthesized closure literal
    // directly, but no fixture needs that, and supporting it would mean
    // widening `Call` to hold a boxed callee expression instead of a
    // `String`. Resolved at eval time: a name bound to a closure value in
    // `env` calls the closure (real Rust shadowing rules: a local
    // shadows a same-named top-level `fn`), otherwise it's a top-level
    // `fn` lookup, matching this node's original (pre-closure) meaning.
    Call(String, Vec<MiniExpr>),
    // `[move] |params| EXPR` -- single-expression body only (no `{ ... }`
    // block form), matching this backend's other closures precedents
    // (single-arity, narrow scope). Real Rust's `move` keyword forces
    // by-value capture instead of by-reference, needed whenever the
    // closure can outlive its creating call (e.g. returned, or otherwise
    // escaping) -- since this interpreter always captures by cloning the
    // whole `env` snapshot at creation time regardless, `move` is parsed
    // and simply ignored; the resulting behavior matches real Rust's
    // `move` semantics for the Copy-like values (`i64`, `Rc`-shared
    // closures) this backend supports.
    Closure(Vec<String>, Box<MiniExpr>),
}

// A runtime value: either a plain `i64`, or a closure -- captured
// parameters, its (single-expression) body, and a snapshot clone of the
// defining `env` at creation time (lexical/creation-time scoping, not
// dynamic scoping: a closure's free variables always resolve against the
// env it closed over, never the caller's env at the point it's invoked).
#[derive(Debug, Clone)]
enum MiniVal {
    Int(i64),
    Closure(Rc<ClosureVal>),
}

#[derive(Debug, Clone)]
struct ClosureVal {
    params: Vec<String>,
    body: MiniExpr,
    env: HashMap<String, MiniVal>,
}

fn expect_int(v: MiniVal, what: &str) -> Result<i64, String> {
    match v {
        MiniVal::Int(n) => Ok(n),
        MiniVal::Closure(_) => Err(format!("tiny interp: expected i64, got a closure ({})", what)),
    }
}

// A `{ let a = ...; let b = ...; TAIL }` block: zero or more `let` statements
// (evaluated for their side effect on `env`, left-to-right) followed by a
// required tail expression that produces the block's value -- matching the
// two places real Rust needs exactly this shape: a `fn` body and each arm of
// an `if`/`else` used as an expression. Deliberately shallow scoping: `let`
// stores directly into the enclosing call's flat `env` rather than a fresh
// child scope, so (unlike real Rust) a name bound inside an `if` branch
// stays visible after the branch ends. No fixture relies on that
// difference (each uses fresh names), so it is left undocumented in code
// but noted here rather than silently risking a future divergence.
#[derive(Debug, Clone)]
struct MiniBlock {
    stmts: Vec<MiniStmt>,
    tail: Box<MiniExpr>,
}

#[derive(Debug, Clone)]
enum MiniStmt {
    Let(String, MiniExpr),
    Assign(String, MiniExpr),
    // A `while` loop's own body is a bare statement sequence with no tail
    // expression -- real Rust's `while { ... }` is unit-typed, and this
    // backend only ever needs it for its mutation side effects (`Assign`
    // onto names declared outside the loop), never its own value.
    While(MiniExpr, Vec<MiniStmt>),
}

#[derive(Debug, Clone)]
struct FnDef {
    name: String,
    params: Vec<String>,
    body: MiniBlock,
}

struct MiniParser {
    toks: Vec<MiniTok>,
    pos: usize,
}

impl MiniParser {
    fn peek(&self) -> Option<&MiniTok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Result<MiniTok, String> {
        let t = self
            .toks
            .get(self.pos)
            .cloned()
            .ok_or("tiny parser: unexpected end of input")?;
        self.pos += 1;
        Ok(t)
    }
    fn expect_punct(&mut self, p: &str) -> Result<(), String> {
        match self.next()? {
            MiniTok::Punct(q) if q == p => Ok(()),
            other => Err(format!("tiny parser: expected {:?}, got {:?}", p, other)),
        }
    }
    fn expect_ident(&mut self) -> Result<String, String> {
        match self.next()? {
            MiniTok::Ident(s) => Ok(s),
            other => Err(format!("tiny parser: expected identifier, got {:?}", other)),
        }
    }
    fn is_punct(&self, p: &str) -> bool {
        matches!(self.peek(), Some(MiniTok::Punct(q)) if *q == p)
    }
    fn is_ident(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(MiniTok::Ident(s)) if s == kw)
    }
    // `IDENT =` (not `IDENT ==`, a separate two-char token) starts an
    // assignment statement -- distinguished from an ordinary `IDENT` var
    // reference or `IDENT(...)` call by this one-token lookahead.
    fn is_assign_stmt(&self) -> bool {
        matches!(self.peek(), Some(MiniTok::Ident(_)))
            && matches!(self.toks.get(self.pos + 1), Some(MiniTok::Punct(q)) if *q == "=")
    }

    // Skip an optional `: i64` type annotation.
    fn skip_type_annotation(&mut self) -> Result<(), String> {
        if self.is_punct(":") {
            self.next()?;
            self.expect_ident()?; // just "i64"
        }
        Ok(())
    }

    fn parse_fn(&mut self) -> Result<FnDef, String> {
        self.expect_ident()?; // "fn" (already checked by caller via is_ident)
        let name = self.expect_ident()?;
        self.expect_punct("(")?;
        let mut params = Vec::new();
        while !self.is_punct(")") {
            if self.is_ident("mut") {
                self.next()?;
            }
            let p = self.expect_ident()?;
            self.skip_type_annotation()?;
            params.push(p);
            if self.is_punct(",") {
                self.next()?;
            }
        }
        self.expect_punct(")")?;
        if self.is_punct("->") {
            self.next()?;
            self.expect_ident()?; // return type, ignored
        }
        let body = self.parse_block()?;
        Ok(FnDef { name, params, body })
    }

    // `{ let ... ; let ... ; TAIL }`
    fn parse_block(&mut self) -> Result<MiniBlock, String> {
        self.expect_punct("{")?;
        let stmts = self.parse_stmt_list()?;
        let tail = Box::new(self.parse_expr()?);
        self.expect_punct("}")?;
        Ok(MiniBlock { stmts, tail })
    }

    fn parse_stmt_list(&mut self) -> Result<Vec<MiniStmt>, String> {
        let mut out = Vec::new();
        loop {
            if self.is_ident("let") {
                out.push(self.parse_let_stmt()?);
            } else if self.is_ident("while") {
                out.push(self.parse_while_stmt()?);
            } else if self.is_assign_stmt() {
                out.push(self.parse_assign_stmt()?);
            } else {
                break;
            }
        }
        Ok(out)
    }

    fn parse_let_stmt(&mut self) -> Result<MiniStmt, String> {
        self.next()?; // "let"
        if self.is_ident("mut") {
            self.next()?;
        }
        let name = self.expect_ident()?;
        self.skip_type_annotation()?;
        self.expect_punct("=")?;
        let init = self.parse_expr()?;
        self.expect_punct(";")?;
        Ok(MiniStmt::Let(name, init))
    }

    fn parse_assign_stmt(&mut self) -> Result<MiniStmt, String> {
        let name = self.expect_ident()?;
        self.expect_punct("=")?;
        let value = self.parse_expr()?;
        self.expect_punct(";")?;
        Ok(MiniStmt::Assign(name, value))
    }

    fn parse_while_stmt(&mut self) -> Result<MiniStmt, String> {
        self.next()?; // "while"
        let cond = self.parse_expr()?;
        self.expect_punct("{")?;
        let body = self.parse_stmt_list()?;
        self.expect_punct("}")?;
        Ok(MiniStmt::While(cond, body))
    }

    fn parse_expr(&mut self) -> Result<MiniExpr, String> {
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> Result<MiniExpr, String> {
        let lhs = self.parse_add()?;
        let op = match self.peek() {
            Some(MiniTok::Punct("<")) => Some(MiniBinOp::Lt),
            Some(MiniTok::Punct(">")) => Some(MiniBinOp::Gt),
            Some(MiniTok::Punct("<=")) => Some(MiniBinOp::Le),
            Some(MiniTok::Punct(">=")) => Some(MiniBinOp::Ge),
            Some(MiniTok::Punct("==")) => Some(MiniBinOp::Eq),
            _ => None,
        };
        if let Some(op) = op {
            self.next()?;
            let rhs = self.parse_add()?;
            Ok(MiniExpr::Bin(op, Box::new(lhs), Box::new(rhs)))
        } else {
            Ok(lhs)
        }
    }

    fn parse_add(&mut self) -> Result<MiniExpr, String> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Some(MiniTok::Punct("+")) => Some(MiniBinOp::Add),
                Some(MiniTok::Punct("-")) => Some(MiniBinOp::Sub),
                _ => None,
            };
            match op {
                Some(op) => {
                    self.next()?;
                    let rhs = self.parse_mul()?;
                    lhs = MiniExpr::Bin(op, Box::new(lhs), Box::new(rhs));
                }
                None => return Ok(lhs),
            }
        }
    }

    fn parse_mul(&mut self) -> Result<MiniExpr, String> {
        let mut lhs = self.parse_atom()?;
        while self.is_punct("*") {
            self.next()?;
            let rhs = self.parse_atom()?;
            lhs = MiniExpr::Bin(MiniBinOp::Mul, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_atom(&mut self) -> Result<MiniExpr, String> {
        if self.is_punct("(") {
            self.next()?;
            let e = self.parse_expr()?;
            self.expect_punct(")")?;
            return Ok(e);
        }
        if self.is_punct("-") {
            self.next()?;
            let e = self.parse_atom()?;
            return Ok(MiniExpr::Bin(MiniBinOp::Sub, Box::new(MiniExpr::Int(0)), Box::new(e)));
        }
        if self.is_ident("if") {
            self.next()?;
            let cond = self.parse_expr()?;
            let then_b = self.parse_block()?;
            self.expect_ident()?; // "else"
            let else_b = self.parse_block()?;
            return Ok(MiniExpr::If(Box::new(cond), Box::new(then_b), Box::new(else_b)));
        }
        if self.is_ident("move") {
            self.next()?; // parsed and ignored, see MiniExpr::Closure's doc comment
        }
        if self.is_punct("|") {
            self.next()?;
            let mut params = Vec::new();
            while !self.is_punct("|") {
                let p = self.expect_ident()?;
                self.skip_type_annotation()?;
                params.push(p);
                if self.is_punct(",") {
                    self.next()?;
                }
            }
            self.expect_punct("|")?;
            let body = self.parse_expr()?;
            return Ok(MiniExpr::Closure(params, Box::new(body)));
        }
        match self.next()? {
            MiniTok::Int(n) => Ok(MiniExpr::Int(n)),
            MiniTok::Ident(name) => {
                if self.is_punct("(") {
                    self.next()?;
                    let mut args = Vec::new();
                    while !self.is_punct(")") {
                        args.push(self.parse_expr()?);
                        if self.is_punct(",") {
                            self.next()?;
                        }
                    }
                    self.expect_punct(")")?;
                    Ok(MiniExpr::Call(name, args))
                } else {
                    Ok(MiniExpr::Var(name))
                }
            }
            other => Err(format!("tiny parser: unexpected token {:?}", other)),
        }
    }
}

/// Parse `fn ...` definitions followed by
/// `fn main() { println!("{}", EXPR); }` and return (helper fns, main's
/// printed expression).
fn parse_mini_program(src: &str) -> Result<(Vec<FnDef>, MiniExpr), String> {
    let toks = tokenize(src)?;
    let mut p = MiniParser { toks, pos: 0 };
    let mut fns = Vec::new();
    let mut main_expr = None;
    while p.peek().is_some() {
        if !p.is_ident("fn") {
            return Err("tiny parser: expected 'fn' at top level".to_string());
        }
        if matches!(&p.toks.get(p.pos + 1), Some(MiniTok::Ident(n)) if n == "main") {
            p.next()?; // fn
            p.next()?; // main
            p.expect_punct("(")?;
            p.expect_punct(")")?;
            p.expect_punct("{")?;
            p.expect_ident()?; // println
            p.expect_punct("!")?;
            p.expect_punct("(")?;
            match p.next()? {
                MiniTok::Str(s) if s == "{}" => {}
                other => return Err(format!("tiny parser: expected \"{{}}\" format string, got {:?}", other)),
            }
            p.expect_punct(",")?;
            let e = p.parse_expr()?;
            p.expect_punct(")")?;
            p.expect_punct(";")?;
            p.expect_punct("}")?;
            main_expr = Some(e);
        } else {
            fns.push(p.parse_fn()?);
        }
    }
    let main_expr = main_expr.ok_or("tiny parser: missing fn main")?;
    Ok((fns, main_expr))
}

fn exec_stmts(
    stmts: &[MiniStmt],
    env: &mut HashMap<String, MiniVal>,
    fns: &HashMap<String, FnDef>,
) -> Result<(), String> {
    for stmt in stmts {
        match stmt {
            MiniStmt::Let(name, init) => {
                let v = eval_expr(init, env, fns)?;
                env.insert(name.clone(), v);
            }
            MiniStmt::Assign(name, value) => {
                if !env.contains_key(name) {
                    return Err(format!("tiny interp: assignment to unknown local {}", name));
                }
                let v = eval_expr(value, env, fns)?;
                env.insert(name.clone(), v);
            }
            MiniStmt::While(cond, body) => {
                while expect_int(eval_expr(cond, env, fns)?, "while condition")? != 0 {
                    exec_stmts(body, env, fns)?;
                }
            }
        }
    }
    Ok(())
}

fn eval_block(
    block: &MiniBlock,
    env: &mut HashMap<String, MiniVal>,
    fns: &HashMap<String, FnDef>,
) -> Result<MiniVal, String> {
    exec_stmts(&block.stmts, env, fns)?;
    eval_expr(&block.tail, env, fns)
}

fn eval_expr(
    e: &MiniExpr,
    env: &mut HashMap<String, MiniVal>,
    fns: &HashMap<String, FnDef>,
) -> Result<MiniVal, String> {
    match e {
        MiniExpr::Int(n) => Ok(MiniVal::Int(*n)),
        MiniExpr::Var(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| format!("tiny interp: unknown local {}", name)),
        MiniExpr::Bin(op, l, r) => {
            let lv = expect_int(eval_expr(l, env, fns)?, "binary op lhs")?;
            let rv = expect_int(eval_expr(r, env, fns)?, "binary op rhs")?;
            Ok(MiniVal::Int(match op {
                MiniBinOp::Add => lv
                    .checked_add(rv)
                    .ok_or("tiny interp: overflow in +")?,
                MiniBinOp::Sub => lv
                    .checked_sub(rv)
                    .ok_or("tiny interp: overflow in -")?,
                MiniBinOp::Mul => lv
                    .checked_mul(rv)
                    .ok_or("tiny interp: overflow in *")?,
                MiniBinOp::Lt => (lv < rv) as i64,
                MiniBinOp::Gt => (lv > rv) as i64,
                MiniBinOp::Le => (lv <= rv) as i64,
                MiniBinOp::Ge => (lv >= rv) as i64,
                MiniBinOp::Eq => (lv == rv) as i64,
            }))
        }
        MiniExpr::If(cond, then_b, else_b) => {
            if expect_int(eval_expr(cond, env, fns)?, "if condition")? != 0 {
                eval_block(then_b, env, fns)
            } else {
                eval_block(else_b, env, fns)
            }
        }
        MiniExpr::Closure(params, body) => Ok(MiniVal::Closure(Rc::new(ClosureVal {
            params: params.clone(),
            body: (**body).clone(),
            env: env.clone(),
        }))),
        MiniExpr::Call(name, args) => {
            if let Some(MiniVal::Closure(closure)) = env.get(name) {
                let closure = closure.clone();
                if closure.params.len() != args.len() {
                    return Err(format!("tiny interp: arity mismatch calling closure {}", name));
                }
                let mut call_env = closure.env.clone();
                for (p, a) in closure.params.iter().zip(args.iter()) {
                    let v = eval_expr(a, env, fns)?;
                    call_env.insert(p.clone(), v);
                }
                return eval_expr(&closure.body, &mut call_env, fns);
            }
            if env.contains_key(name) {
                return Err(format!("tiny interp: {} is not callable", name));
            }
            let f = fns
                .get(name.as_str())
                .ok_or_else(|| format!("tiny interp: unknown fn {}", name))?;
            if f.params.len() != args.len() {
                return Err(format!("tiny interp: arity mismatch calling {}", name));
            }
            let mut call_env = HashMap::new();
            for (p, a) in f.params.iter().zip(args.iter()) {
                let v = eval_expr(a, env, fns)?;
                call_env.insert(p.clone(), v);
            }
            eval_block(&f.body, &mut call_env, fns)
        }
    }
}

/// Parse and evaluate a small `fn`/`if`/`let`/`while`/closure/arithmetic
/// Rust program whose `main` is exactly `println!("{}", EXPR);`, and return
/// the printed value as a string (matching how `native::native_run` returns
/// captured stdout, so the two can be compared directly).
pub fn compile_and_run(src: &str) -> Result<String, String> {
    let (fn_defs, main_expr) = parse_mini_program(src)?;
    let mut fn_map: HashMap<String, FnDef> = HashMap::new();
    for f in fn_defs.into_iter() {
        fn_map.insert(f.name.clone(), f);
    }
    let mut env = HashMap::new();
    let value = expect_int(eval_expr(&main_expr, &mut env, &fn_map)?, "println! argument")?;
    Ok(format!("{}\n", value))
}
