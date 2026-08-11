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
//! interpreter: it covers a bounded `fn`/`if`/arithmetic/comparison/call
//! fixture set, not the Rust language `interp.rs` targets.

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Int(i64),
    Str(String),
    Punct(&'static str),
}

fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
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
            out.push(Tok::Str(s));
            i = j + 1;
            continue;
        }
        if c.is_ascii_digit() {
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let text: String = bytes[i..j].iter().collect();
            out.push(Tok::Int(text.parse().map_err(|_| "tiny lexer: bad int")?));
            i = j;
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let mut j = i;
            while j < bytes.len() && (bytes[j].is_alphanumeric() || bytes[j] == '_') {
                j += 1;
            }
            out.push(Tok::Ident(bytes[i..j].iter().collect()));
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
                out.push(Tok::Punct(p));
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
            _ => return Err(format!("tiny lexer: unexpected char {:?}", c)),
        };
        out.push(Tok::Punct(one));
        i += 1;
    }
    Ok(out)
}

#[derive(Debug, Clone)]
enum BinOp {
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
enum Expr {
    Int(i64),
    Var(String),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone)]
struct FnDef {
    name: String,
    params: Vec<String>,
    body: Expr,
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Result<Tok, String> {
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
            Tok::Punct(q) if q == p => Ok(()),
            other => Err(format!("tiny parser: expected {:?}, got {:?}", p, other)),
        }
    }
    fn expect_ident(&mut self) -> Result<String, String> {
        match self.next()? {
            Tok::Ident(s) => Ok(s),
            other => Err(format!("tiny parser: expected identifier, got {:?}", other)),
        }
    }
    fn is_punct(&self, p: &str) -> bool {
        matches!(self.peek(), Some(Tok::Punct(q)) if *q == p)
    }
    fn is_ident(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(s)) if s == kw)
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
        self.expect_punct("{")?;
        let body = self.parse_expr()?;
        self.expect_punct("}")?;
        Ok(FnDef { name, params, body })
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_add()?;
        let op = match self.peek() {
            Some(Tok::Punct("<")) => Some(BinOp::Lt),
            Some(Tok::Punct(">")) => Some(BinOp::Gt),
            Some(Tok::Punct("<=")) => Some(BinOp::Le),
            Some(Tok::Punct(">=")) => Some(BinOp::Ge),
            Some(Tok::Punct("==")) => Some(BinOp::Eq),
            _ => None,
        };
        if let Some(op) = op {
            self.next()?;
            let rhs = self.parse_add()?;
            Ok(Expr::Bin(op, Box::new(lhs), Box::new(rhs)))
        } else {
            Ok(lhs)
        }
    }

    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Punct("+")) => Some(BinOp::Add),
                Some(Tok::Punct("-")) => Some(BinOp::Sub),
                _ => None,
            };
            match op {
                Some(op) => {
                    self.next()?;
                    let rhs = self.parse_mul()?;
                    lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
                }
                None => return Ok(lhs),
            }
        }
    }

    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_atom()?;
        while self.is_punct("*") {
            self.next()?;
            let rhs = self.parse_atom()?;
            lhs = Expr::Bin(BinOp::Mul, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_atom(&mut self) -> Result<Expr, String> {
        if self.is_punct("(") {
            self.next()?;
            let e = self.parse_expr()?;
            self.expect_punct(")")?;
            return Ok(e);
        }
        if self.is_punct("-") {
            self.next()?;
            let e = self.parse_atom()?;
            return Ok(Expr::Bin(BinOp::Sub, Box::new(Expr::Int(0)), Box::new(e)));
        }
        if self.is_ident("if") {
            self.next()?;
            let cond = self.parse_expr()?;
            self.expect_punct("{")?;
            let then_e = self.parse_expr()?;
            self.expect_punct("}")?;
            self.expect_ident()?; // "else"
            self.expect_punct("{")?;
            let else_e = self.parse_expr()?;
            self.expect_punct("}")?;
            return Ok(Expr::If(Box::new(cond), Box::new(then_e), Box::new(else_e)));
        }
        match self.next()? {
            Tok::Int(n) => Ok(Expr::Int(n)),
            Tok::Ident(name) => {
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
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            other => Err(format!("tiny parser: unexpected token {:?}", other)),
        }
    }
}

/// Parse `fn ...` definitions followed by
/// `fn main() { println!("{}", EXPR); }` and return (helper fns, main's
/// printed expression).
fn parse_program(src: &str) -> Result<(Vec<FnDef>, Expr), String> {
    let toks = tokenize(src)?;
    let mut p = Parser { toks, pos: 0 };
    let mut fns = Vec::new();
    let mut main_expr = None;
    while p.peek().is_some() {
        if !p.is_ident("fn") {
            return Err("tiny parser: expected 'fn' at top level".to_string());
        }
        if matches!(&p.toks.get(p.pos + 1), Some(Tok::Ident(n)) if n == "main") {
            p.next()?; // fn
            p.next()?; // main
            p.expect_punct("(")?;
            p.expect_punct(")")?;
            p.expect_punct("{")?;
            p.expect_ident()?; // println
            p.expect_punct("!")?;
            p.expect_punct("(")?;
            match p.next()? {
                Tok::Str(s) if s == "{}" => {}
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

fn eval_expr(
    e: &Expr,
    env: &HashMap<String, i64>,
    fns: &HashMap<String, &FnDef>,
) -> Result<i64, String> {
    match e {
        Expr::Int(n) => Ok(*n),
        Expr::Var(name) => env
            .get(name)
            .copied()
            .ok_or_else(|| format!("tiny interp: unknown local {}", name)),
        Expr::Bin(op, l, r) => {
            let lv = eval_expr(l, env, fns)?;
            let rv = eval_expr(r, env, fns)?;
            Ok(match op {
                BinOp::Add => lv
                    .checked_add(rv)
                    .ok_or("tiny interp: overflow in +")?,
                BinOp::Sub => lv
                    .checked_sub(rv)
                    .ok_or("tiny interp: overflow in -")?,
                BinOp::Mul => lv
                    .checked_mul(rv)
                    .ok_or("tiny interp: overflow in *")?,
                BinOp::Lt => (lv < rv) as i64,
                BinOp::Gt => (lv > rv) as i64,
                BinOp::Le => (lv <= rv) as i64,
                BinOp::Ge => (lv >= rv) as i64,
                BinOp::Eq => (lv == rv) as i64,
            })
        }
        Expr::If(cond, then_e, else_e) => {
            if eval_expr(cond, env, fns)? != 0 {
                eval_expr(then_e, env, fns)
            } else {
                eval_expr(else_e, env, fns)
            }
        }
        Expr::Call(name, args) => {
            let f = fns
                .get(name.as_str())
                .ok_or_else(|| format!("tiny interp: unknown fn {}", name))?;
            if f.params.len() != args.len() {
                return Err(format!("tiny interp: arity mismatch calling {}", name));
            }
            let mut call_env = HashMap::new();
            for (p, a) in f.params.iter().zip(args.iter()) {
                call_env.insert(p.clone(), eval_expr(a, env, fns)?);
            }
            eval_expr(&f.body, &call_env, fns)
        }
    }
}

#[derive(Debug)]
pub struct MiniBackendError(pub String);

impl fmt::Display for MiniBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Parse and evaluate a small `fn`/`if`/arithmetic Rust program whose `main`
/// is exactly `println!("{}", EXPR);`, and return the printed value as a
/// string (matching how `native::native_run` returns captured stdout, so the
/// two can be compared directly).
pub fn compile_and_run(src: &str) -> Result<String, MiniBackendError> {
    let (fn_defs, main_expr) = parse_program(src).map_err(MiniBackendError)?;
    let fn_map: HashMap<String, &FnDef> = fn_defs.iter().map(|f| (f.name.clone(), f)).collect();
    let env = HashMap::new();
    let value = eval_expr(&main_expr, &env, &fn_map).map_err(MiniBackendError)?;
    Ok(format!("{}\n", value))
}
