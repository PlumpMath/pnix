//! Seed .px evaluator for the pnix-rs lane.
//!
//! .px is the pnix expression language (Nix-like). Current surface: integers,
//! booleans, strings with `${...}` interpolation, lists `[ a b ]` with `++`,
//! attrsets `{ name = expr; }` with right-biased `//` merge and `.name`
//! selection, `let ... ; ... in body` with *recursive* binding scope, lambdas
//! `param: body`, application by juxtaposition, `if/then/else`,
//! arithmetic/comparison, `#` comments, and a fixed `builtins.*` set
//! (toString, stringLength, concatStringsSep, substring, length, map, filter,
//! foldl', attrNames, hasAttr, sort, head, tail, elemAt, elem).
//!
//! This file is deliberately written inside the rs-meta evaluated Rust subset:
//! the substrate contract (`pnix-rs substrate-check`) has ../rs-meta interpret
//! this exact source and requires its output to match the rustc-compiled
//! behavior. That is the dependency between pnix-rs and rs-meta made
//! falsifiable.
//!
//! Still outside the seed (tracked in todo.md): floats, `+` on strings,
//! boolean `&& || !` operators, `?` has-attr operator, `builtins.toJSON`,
//! `rec` attrsets, `with`, paths.

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PxErrorPhase {
    Parse,
    Eval,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PxErrorClass {
    SyntaxError,
    UnsupportedExpression,
    UnknownVariable,
    AttributeMissing,
    NotCallable,
    NonBooleanCondition,
    TypeError,
    DivisionByZero,
    IntegerOverflow,
    CycleDetected,
    PrimitiveContractViolation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PxError {
    pub phase: PxErrorPhase,
    pub class: PxErrorClass,
    pub diagnostic: String,
}

impl PxError {
    pub fn phase_name(&self) -> &'static str {
        match self.phase {
            PxErrorPhase::Parse => "parse",
            PxErrorPhase::Eval => "eval",
        }
    }

    pub fn class_name(&self) -> &'static str {
        match self.class {
            PxErrorClass::SyntaxError => "syntax-error",
            PxErrorClass::UnsupportedExpression => "unsupported-expression",
            PxErrorClass::UnknownVariable => "unknown-variable",
            PxErrorClass::AttributeMissing => "attribute-missing",
            PxErrorClass::NotCallable => "not-callable",
            PxErrorClass::NonBooleanCondition => "non-boolean-condition",
            PxErrorClass::TypeError => "type-error",
            PxErrorClass::DivisionByZero => "division-by-zero",
            PxErrorClass::IntegerOverflow => "integer-overflow",
            PxErrorClass::CycleDetected => "cycle-detected",
            PxErrorClass::PrimitiveContractViolation => "primitive-contract-violation",
        }
    }
}

// Keep constructors and compatibility projections as free functions.
// rs-meta's interpreted Rust subset treats `Type::function` paths as enum
// constructors; plain function values are understood by both rs-meta and
// rustc while preserving the nominal structured error.
fn px_error_parse(class: PxErrorClass, diagnostic: String) -> PxError {
    PxError { phase: PxErrorPhase::Parse, class, diagnostic }
}

fn px_error_eval(class: PxErrorClass, diagnostic: String) -> PxError {
    PxError { phase: PxErrorPhase::Eval, class, diagnostic }
}

fn px_error_unsupported(diagnostic: String) -> PxError {
    px_error_eval(PxErrorClass::UnsupportedExpression, diagnostic)
}

fn px_error_type(diagnostic: String) -> PxError {
    px_error_eval(PxErrorClass::TypeError, diagnostic)
}

fn px_error_into_diagnostic(error: PxError) -> String {
    error.diagnostic
}

#[derive(Clone, Debug, PartialEq)]
pub enum PxOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Concat,
    Update,
    HasAttr,
}

#[derive(Clone, Debug)]
pub enum PxStrPart {
    Lit(String),
    Sub(PxExpr),
}

#[derive(Clone, Debug)]
pub enum PxExpr {
    Int(i64),
    /// Nix-style float (printed with Rust Debug — matches pnix-hy repr).
    Float(f64),
    Bool(bool),
    Null,
    Str(Vec<PxStrPart>),
    Var(String),
    List(Vec<PxExpr>),
    Select { base: Box<PxExpr>, name: String },
    /// Rc body: closures share the lambda body instead of cloning it
    /// (call-by-name recursive lookup re-creates closures constantly).
    Lambda { param: String, body: Rc<PxExpr> },
    Apply { func: Box<PxExpr>, arg: Box<PxExpr> },
    If { cond: Box<PxExpr>, then_e: Box<PxExpr>, else_e: Box<PxExpr> },
    Binary { op: PxOp, lhs: Box<PxExpr>, rhs: Box<PxExpr> },
    LetIn { bindings: Vec<(String, PxExpr)>, body: Box<PxExpr> },
    /// Nix `with scope; body` — dynamic LOWEST-priority scope (let shadows it).
    With { scope: Box<PxExpr>, body: Box<PxExpr> },
    Attrs(Vec<(String, PxExpr)>),
    /// Internal-only lazy import failure. The parser never constructs this;
    /// import expansion installs it at a failing import site so dead code can
    /// remain lazy without exposing a source identifier that user scope could
    /// capture.
    DeferredError(String),
    /// Internal-only environment reset. The parser never constructs this;
    /// import expansion wraps a substituted module's AST in it so the
    /// module evaluates from a fresh (empty) environment instead of
    /// inheriting whatever let/with/lambda frames were active at the
    /// splice site -- import substitutes the target's AST directly into
    /// the tree (no separate per-module evaluation call), so without this
    /// the module's free variables would accidentally resolve against the
    /// importing site's local scope. `with_scope`, when present
    /// (scopedImport), is evaluated in the OUTER (caller's) environment --
    /// same as any ordinary argument expression -- and its value becomes
    /// the *only* frame `body` starts with, so scope is visible to the
    /// module without also re-admitting the caller's other local bindings.
    Isolated {
        with_scope: Option<Box<PxExpr>>,
        body: Box<PxExpr>,
    },
}

#[derive(Clone, Debug)]
pub enum PxVal {
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    Str(String),
    /// RAW-BYTE strings (owner-selected track, 2026-07-11): a byte-level op
    /// (substring/concat) produced bytes that are NOT valid UTF-8 — Nix
    /// permits such intermediates; Rust String cannot hold them. Only the
    /// oracle-pinned surface operates on Bytes (len/slice/concat/eq/lt);
    /// everything else is HELD fail-closed. A concat whose result is valid
    /// UTF-8 returns to Str.
    Bytes(Vec<u8>),
    /// A path literal (`./x`, `../x`, `/x`) or the result of a path-producing
    /// operation (path+path/path+string concat, `dirOf`, `toPath`). Stored
    /// as a plain `String` (this codebase already prefers `String` over
    /// `std::path::PathBuf` for its other host-side path handling) holding
    /// the LEXICALLY NORMALIZED text (`px_normalize_path`), so equality/
    /// ordering can compare the stored strings directly and never need to
    /// re-normalize. Unlike real Nix, a relative literal is NOT resolved to
    /// an absolute filesystem path against its containing file — it keeps
    /// its normalized relative text (`./a/b`), matching how this seed's
    /// `import` already treats path text as file-relative only at the
    /// point it is actually used (`readFile`/`pathExists`/import
    /// resolution), not at construction.
    Path(String),
    /// Rc payload: lists/attrsets share structure on clone (persistent-value
    /// model, same shape as rs-meta's own Val — tower m3 perf boundary fix).
    List(Rc<Vec<PxVal>>),
    Closure { param: String, body: Rc<PxExpr>, env: Vec<PxFrame> },
    Builtin { name: String, args: Vec<PxVal> },
    Attrs(Rc<Vec<(String, PxVal)>>),
    /// Call-by-need thunk (2026-07-10 laziness landing). Attrset fields and
    /// non-immediate function arguments are stored UNFORCED so construction
    /// and application never force values that are not demanded — this
    /// is what lets Nix-style mutually-recursive / cyclic data
    /// (`rec { a = {..b..}; b = {..a..}; }`) be constructed and have
    /// `attrNames`/`typeOf`/partial access work without chasing the cycle,
    /// exactly like real Nix. The Rc<RefCell<..>> mirrors the existing
    /// PxFrame::Rec cache (rs-meta already self-hosts Rc/RefCell). Containment
    /// invariant: consumers force a Thunk only when they inspect its value;
    /// meta subsystems deep-force finite results at their analysis boundary.
    Thunk(Rc<RefCell<PxThunk>>),
}

/// Thunk state machine (Suspended/Blackhole/Evaluated/Failed, on the
/// Rc<RefCell> the rs-meta subset already permits). `Blackhole` is entered
/// while forcing so a re-entrant self-reference (`let x = x; in x`) is caught
/// as a `<<loop>>` instead of overflowing — matching Nix's "infinite recursion
/// encountered". Productive infinite recursion (`f = n: f (n+1)`) is NOT a
/// blackhole and still hits the stack/depth bound, same as Nix.
#[derive(Clone, Debug)]
pub enum PxThunk {
    /// The captured env is shared behind an `Rc` so cloning the thunk state on
    /// each force is two refcount bumps, not a copy of the whole frame stack.
    Unforced(Rc<PxExpr>, Rc<Vec<PxFrame>>),
    /// A builtin-produced value that applies `func` to `arg` only when the
    /// resulting list element or attrset field is inspected.
    DeferredApply(PxVal, PxVal),
    Blackhole,
    Forced(PxVal),
    /// Evaluation failed after this thunk entered `Blackhole`. Memoize the
    /// original error so every later force observes the same failure instead
    /// of misreporting it as a recursion cycle.
    Failed(PxError),
}

/// Build an unforced thunk value from an expression + its (shared) environment.
pub fn px_thunk(expr: PxExpr, env: Rc<Vec<PxFrame>>) -> PxVal {
    PxVal::Thunk(Rc::new(RefCell::new(PxThunk::Unforced(Rc::new(expr), env))))
}

/// Defer one already-evaluated function/value application behind the same
/// memoizing thunk state machine used by source expressions.
fn px_defer_apply(func: PxVal, arg: PxVal) -> PxVal {
    PxVal::Thunk(Rc::new(RefCell::new(PxThunk::DeferredApply(func, arg))))
}

/// Complete a thunk evaluation, memoizing either its WHNF value or its exact
/// error. The cell is already blackholed when this helper is entered.
fn px_finish_thunk(
    cell: &Rc<RefCell<PxThunk>>,
    result: Result<PxVal, PxError>,
) -> Result<PxVal, PxError> {
    let raw = match result {
        Ok(value) => value,
        Err(error) => {
            *cell.borrow_mut() = PxThunk::Failed(error.clone());
            return Err(error);
        }
    };
    let value = match px_force_outcome(&raw) {
        Ok(value) => value,
        Err(error) => {
            *cell.borrow_mut() = PxThunk::Failed(error.clone());
            return Err(error);
        }
    };
    *cell.borrow_mut() = PxThunk::Forced(value.clone());
    Ok(value)
}

/// Recursively force EVERY thunk in a value (attrset fields + list elements),
/// returning a fully thunk-free value. The meta subsystems (tower / mirror /
/// specialize) analyze fully-evaluated values and their traversal predates
/// laziness, so they call this at their input boundary to get a thunk-free
/// value — cheaper than threading force through every field read there, and
/// correct because those paths operate on already-terminating results. A
/// genuinely cyclic value diverges here, exactly as deep-forcing does in Nix.
pub fn px_force_deep_outcome(v: &PxVal) -> Result<PxVal, PxError> {
    let v = px_force_outcome(v)?;
    match v {
        PxVal::List(items) => {
            let mut out = Vec::new();
            for it in items.iter() {
                out.push(px_force_deep_outcome(it)?);
            }
            Ok(px_list(out))
        }
        PxVal::Attrs(fields) => {
            let mut out = Vec::new();
            for (k, val) in fields.iter() {
                out.push((k.clone(), px_force_deep_outcome(val)?));
            }
            Ok(px_attrs(out))
        }
        other => Ok(other),
    }
}

pub fn px_force_deep(v: &PxVal) -> Result<PxVal, String> {
    px_force_deep_outcome(v).map_err(px_error_into_diagnostic)
}

/// Force a value to weak head normal form: if it is a Thunk, evaluate it
/// (memoizing either its value or its error, and blackholing during evaluation
/// to catch cycles), otherwise return it unchanged. Idempotent and safe to
/// call on any value.
fn px_force_outcome(v: &PxVal) -> Result<PxVal, PxError> {
    let cell = match v {
        PxVal::Thunk(c) => c.clone(),
        other => return Ok(other.clone()),
    };
    // Cloning the state is cheap: `Unforced` holds two Rc handles.
    let state = cell.borrow().clone();
    match state {
        PxThunk::Forced(val) => Ok(val),
        PxThunk::Failed(error) => Err(error),
        PxThunk::Blackhole => {
            Err(px_error_eval(
                PxErrorClass::CycleDetected,
                String::from("px: infinite recursion encountered (<<loop>>)")
            ))
        }
        PxThunk::Unforced(expr, env) => {
            *cell.borrow_mut() = PxThunk::Blackhole;
            // The thunk body may itself yield a thunk (field-of-a-field); force
            // through to WHNF before memoizing so callers never re-observe one.
            px_finish_thunk(&cell, px_eval_outcome(&expr, env.as_ref()))
        }
        PxThunk::DeferredApply(func, arg) => {
            *cell.borrow_mut() = PxThunk::Blackhole;
            px_finish_thunk(&cell, px_apply_outcome(&func, arg))
        }
    }
}

pub fn px_force(v: &PxVal) -> Result<PxVal, String> {
    px_force_outcome(v).map_err(px_error_into_diagnostic)
}

/// Canonical constructors — every List/Attrs value is built here.
pub fn px_list(items: Vec<PxVal>) -> PxVal {
    PxVal::List(Rc::new(items))
}

/// Fields are kept SORTED by name (proposal 0002): the single constructor
/// establishes the invariant, so lookups can binary-search. Inputs never
/// carry duplicate names (parser / listToAttrs / merge dedup first).
pub fn px_attrs(fields: Vec<(String, PxVal)>) -> PxVal {
    // Selection sort via remove(min) — the same subset-proven pattern as the
    // sort builtin below (Vec::swap is outside the evaluated subset).
    let mut remaining = fields;
    let mut sorted = Vec::new();
    while !remaining.is_empty() {
        let mut min = 0usize;
        let mut j = 1usize;
        while j < remaining.len() {
            if px_str_lt(&remaining[j].0, &remaining[min].0) {
                min = j;
            }
            j += 1;
        }
        sorted.push(remaining.remove(min));
    }
    PxVal::Attrs(Rc::new(sorted))
}

/// Binary search over the sorted-fields invariant.
fn px_attrs_find<'a>(fields: &'a [(String, PxVal)], name: &str) -> Option<&'a PxVal> {
    if px_is_attr_pos_key(name) {
        return None;
    }
    let mut lo = 0usize;
    let mut hi = fields.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if fields[mid].0 == name {
            return Some(&fields[mid].1);
        }
        if px_str_lt(&fields[mid].0, name) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    None
}

fn px_split_attr_pos(fields: &[(String, PxVal)]) -> (Vec<(String, PxVal)>, Option<PxVal>) {
    let mut user = Vec::new();
    let mut pos = None;
    for (k, v) in fields.iter() {
        if px_is_attr_pos_key(k) {
            pos = Some(v.clone());
        } else {
            user.push((k.clone(), v.clone()));
        }
    }
    (user, pos)
}

fn px_join_attr_pos(user: Vec<(String, PxVal)>, pos: Option<PxVal>) -> PxVal {
    let mut user = user;
    match pos {
        Some(p) => {
            user.push((String::from(PX_ATTR_POS_KEY), p));
            px_attrs(user)
        }
        None => px_attrs(user),
    }
}

// ---- string context (pure simulation of Nix string context) ---------------
//
// A Nix string carries a context: the set of store-path dependencies that
// must be realized before the string may be used. Context-free strings stay
// plain `PxVal::Str` (zero representation change for the overwhelming
// majority of string uses); a string only becomes the tagged shape below
// when its context is non-empty (built by `builtins.appendContext` today, and
// by derivation drvPath/outPath). This mirrors pnix-clj's/pnix-clr's own
// design exactly: a context-bearing string is a `PxVal::Attrs` carrying
// sentinel keys, NOT a new `PxVal` variant (a new variant would force
// exhaustive-match edits across bta.rs/gate.rs/incremental.rs/specialize.rs/
// tower.rs/main.rs; `Attrs` already exists and is already handled
// everywhere). This follows the same precedent already established by
// `PxVal::Bytes` above: a special value shape that only the oracle-pinned
// surface (the fixed `context-aware-builtins` allowlist below) understands —
// everything else is HELD fail-closed rather than silently mangling or
// dropping the context.
const PX_CTX_STRING_TAG: &str = "string-context";

/// Sort + dedup a context list (same normalization as pnix-clj's/pnix-cljs's
/// own `ctx-string` constructor). Manual adjacent-scan dedup after sorting,
/// matching this file's existing `px_attrs`-style explicit-loop idiom rather
/// than `Vec::dedup`.
fn px_ctx_normalize(context: Vec<String>) -> Vec<String> {
    let sorted = px_sort_strings(context);
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < sorted.len() {
        let dup = !out.is_empty() && out[out.len() - 1] == sorted[i];
        if !dup {
            out.push(sorted[i].clone());
        }
        i += 1;
    }
    out
}

/// Build a context-bearing string value. Collapses to a plain `PxVal::Str`
/// when the (deduped) context is empty, so context-free results stay
/// indistinguishable from ordinary strings — the same collapsing rule
/// pnix-clj's `ctx-string` uses.
fn px_ctx_string(content: String, context: Vec<String>) -> PxVal {
    let ctx = px_ctx_normalize(context);
    if ctx.is_empty() {
        return PxVal::Str(content);
    }
    let mut ctx_vals = Vec::new();
    let mut i = 0usize;
    while i < ctx.len() {
        ctx_vals.push(PxVal::Str(ctx[i].clone()));
        i += 1;
    }
    px_attrs(vec![
        (String::from("__pnix_value_kind"), PxVal::Str(String::from(PX_CTX_STRING_TAG))),
        (String::from("string"), PxVal::Str(content)),
        (String::from("context"), px_list(ctx_vals)),
    ])
}

/// True when `v` is a context-bearing string built by `px_ctx_string`. `v`
/// must already be forced (WHNF) — every call site here forces first, the
/// same contract the shallow fail-closed scan below relies on.
fn px_is_ctx_string(v: &PxVal) -> bool {
    match v {
        PxVal::Attrs(fields) => match px_attrs_find(fields.as_ref(), "__pnix_value_kind") {
            Some(PxVal::Str(tag)) => tag == PX_CTX_STRING_TAG,
            _ => false,
        },
        _ => false,
    }
}

/// A real pnix attrset — `PxVal::Attrs` that is NOT the context-string
/// sentinel shape. Every generic "is this an attrset" site (`.` select, `?`,
/// `//`, `isAttrs`) must use this instead of a bare `PxVal::Attrs(_)` match,
/// or a context-bearing string would leak its internal representation
/// through ordinary attrset operations.
fn px_is_real_attrset(v: &PxVal) -> bool {
    matches!(v, PxVal::Attrs(_)) && !px_is_ctx_string(v)
}

/// Character content of a string-like value (plain `Str` or a ctx-string);
/// `None` for anything else (including `Bytes`, tracked separately).
fn px_string_like_content(v: &PxVal) -> Option<String> {
    if let PxVal::Str(s) = v {
        return Some(s.clone());
    }
    if let PxVal::Attrs(fields) = v {
        if px_is_ctx_string(v) {
            if let Some(PxVal::Str(s)) = px_attrs_find(fields.as_ref(), "string") {
                return Some(s.clone());
            }
        }
    }
    None
}

/// `px_string_like_content`, defaulting to the empty string instead of
/// `None` — an explicit match instead of `Option::unwrap_or_default`
/// (unsupported by rs-meta's interpreted-subset typeck; substrate-check
/// caught it — see the `+`-style explicit-match idiom this file already
/// uses throughout). Callers use this only where the value has already been
/// confirmed string-like by the caller's own dispatch guard, so the `None`
/// arm is unreachable in practice; the empty-string default keeps it total.
fn px_string_like_content_or_empty(v: &PxVal) -> String {
    match px_string_like_content(v) {
        Some(s) => s,
        None => String::new(),
    }
}

/// True for anything `px_string_like_content` can extract from (plain `Str`
/// or ctx-string) — NOT `Bytes`, which is tracked by the separate raw-byte
/// surface.
fn px_is_string_like(v: &PxVal) -> bool {
    px_string_like_content(v).is_some()
}

/// The context vector of a string-like value; `[]` for a plain string or
/// anything else (including `Bytes`).
fn px_string_like_context(v: &PxVal) -> Vec<String> {
    if let PxVal::Attrs(fields) = v {
        if px_is_ctx_string(v) {
            if let Some(PxVal::List(items)) = px_attrs_find(fields.as_ref(), "context") {
                let mut out = Vec::new();
                let mut i = 0usize;
                while i < items.len() {
                    if let PxVal::Str(s) = &items[i] {
                        out.push(s.clone());
                    }
                    i += 1;
                }
                return out;
            }
        }
    }
    Vec::new()
}

/// Union two values' contexts (used by `+` concat and every context-unioning
/// builtin below). `px_ctx_string` sorts+dedups, so callers can just
/// concatenate.
fn px_ctx_union(a: &PxVal, b: &PxVal) -> Vec<String> {
    let mut out = px_string_like_context(a);
    out.extend(px_string_like_context(b));
    out
}

/// Every character of `s` after the first (`s` is always one of this file's
/// own ASCII sentinel-prefixed context elements, e.g. `"=<path>"` or
/// `"!<rest>"` — so a char-count drop of 1 is exact). Manual char-vector
/// rebuild rather than byte-range string slicing, matching this file's
/// existing prefix/suffix idiom (`px_str_has_suffix` etc.).
fn px_str_tail(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 1usize;
    while i < chars.len() {
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Split a `getContext` output-dependency encoding "<output>!<path>" (the
/// text AFTER the leading '!' the caller already stripped) at its first
/// remaining '!'. `None` when there is no second '!' (a bare path element
/// that happens to start with '!', falls back to a plain path element).
fn px_split_bang(rest: &str) -> Option<(String, String)> {
    let chars: Vec<char> = rest.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '!' {
            let mut output = String::new();
            let mut j = 0usize;
            while j < i {
                output.push(chars[j]);
                j += 1;
            }
            let mut path = String::new();
            let mut k = i + 1;
            while k < chars.len() {
                path.push(chars[k]);
                k += 1;
            }
            return Some((output, path));
        }
        i += 1;
    }
    None
}

/// Find (by linear scan) or insert a fresh zeroed `(path, has_path,
/// all_outputs, outputs)` record in `getContext`'s per-path accumulator,
/// returning its index.
fn px_getcontext_find_or_insert(
    acc: &mut Vec<(String, bool, bool, Vec<String>)>,
    path: &str,
) -> usize {
    let mut i = 0usize;
    while i < acc.len() {
        if acc[i].0 == path {
            return i;
        }
        i += 1;
    }
    acc.push((String::from(path), false, false, Vec::new()));
    acc.len() - 1
}

/// The fixed, oracle-verified allowlist of builtins permitted to receive a
/// contextful string argument (ported by NAME from pnix-clj's own
/// `context-aware-builtins` / pnix-cljs's port of the same set — not
/// reinvented). Every other builtin is denied by default: a contextful
/// string reaching it fails closed instead of silently dropping or mangling
/// context. Grow this set only builtin-by-builtin, with the builtin itself
/// taught to propagate context correctly.
fn px_context_aware_builtin(name: &str) -> bool {
    name == "hasContext"
        || name == "getContext"
        || name == "hashString"
        || name == "unsafeDiscardStringContext"
        || name == "unsafeDiscardOutputDependency"
        || name == "appendContext"
        || name == "toPath"
        || name == "toString"
        || name == "typeOf"
        || name == "isString"
        || name == "isAttrs"
        || name == "isList"
        || name == "isInt"
        || name == "isFloat"
        || name == "isBool"
        || name == "isNull"
        || name == "isFunction"
        || name == "seq"
        || name == "deepSeq"
        || name == "trace"
        || name == "id"
        || name == "eq"
        || name == "derivation"
        || name == "derivationStrict"
        || name == "placeholder"
        || name == "storePath"
        || name == "stringLength"
        || name == "substring"
        || name == "concatStringsSep"
        || name == "hasPrefix"
        || name == "hasSuffix"
        || name == "hasInfix"
        || name == "toUpper"
        || name == "toLower"
        || name == "replaceStrings"
        || name == "match"
        || name == "split"
        || name == "toJSON"
        || name == "fromJSON"
        || name == "stringToCharacters"
        || name == "splitString"
        || name == "removePrefix"
        || name == "removeSuffix"
        || name == "toInt"
        || name == "concatStrings"
        || name == "concatMapStrings"
        || name == "head"
        || name == "tail"
        || name == "elemAt"
        || name == "last"
        || name == "init"
        || name == "length"
        || name == "elem"
}

/// Shallow scan (top level + one level into list elements) for a contextful
/// string among builtin call args, mirroring pnix-clj's/pnix-cljs's own
/// `ctx-string-in-args?` exactly, INCLUDING its limits: an element still
/// hidden behind an unforced `Thunk` is opaque here (matches the oracle —
/// empirically confirmed there that a context string nested inside an
/// unforced list element passed to e.g. `sort`/`filter` is NOT caught by
/// this scan). Matching that shallow scan rather than a deeper eager-forcing
/// one keeps this host's fail-closed behavior identical to the oracle's
/// instead of stricter than it.
fn px_ctx_string_in_args(args: &Vec<PxVal>) -> bool {
    let mut i = 0usize;
    while i < args.len() {
        if px_is_ctx_string(&args[i]) {
            return true;
        }
        if let PxVal::List(items) = &args[i] {
            let mut j = 0usize;
            while j < items.len() {
                if px_is_ctx_string(&items[j]) {
                    return true;
                }
                j += 1;
            }
        }
        i += 1;
    }
    false
}

/// Environment frame. `Rec` is a recursive let frame: every binding in it can
/// see every other binding in the same frame (pnix let is recursive scope, not
/// sequential). `Bind` is a single evaluated value (lambda parameter).
#[derive(Clone, Debug)]
pub enum PxFrame {
    /// Recursive let frame. The second field memoizes each binding's value
    /// (proposal 0003, call-by-need): px is pure, so the first evaluation's
    /// result is THE result — reuse is observationally equivalent.
    Rec(
        Rc<Vec<(String, PxExpr)>>,
        Rc<RefCell<Vec<Option<Result<PxVal, PxError>>>>>,
    ),
    Bind { name: String, value: PxVal },
    /// `with` scope: consulted only after every static frame + builtins miss
    /// (oracle-pinned: `let a=2; in with {a=1;}; a == 2`); newest wins.
    With(PxVal),
}

// ---- lexer ------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum PxTok {
    Int(i64),
    /// The unsigned magnitude `9223372036854775808` (abs(i64::MIN)) -- one
    /// more than i64::MAX, so it can never tokenize as a positive `Int`.
    /// Only valid when parse_neg's `-` fold consumes it directly; a bare
    /// occurrence falls through to parse_atom's generic "unexpected token"
    /// error.
    IntMinMagnitude,
    Float(f64),
    Ident(String),
    /// Deprecated-but-supported Nix URI literal. Kept distinct from `Str`
    /// because URI tokens are expression atoms, not quoted attribute names.
    Uri(String),
    /// String literal parts: (is_interpolation, text). Interpolation text is
    /// re-lexed/parsed as an expression by the parser.
    Str(Vec<(bool, String)>),
    KwLet,
    KwIn,
    KwWith,
    KwAssert,
    KwIf,
    KwThen,
    KwElse,
    True,
    False,
    Null,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semi,
    Assign,
    Colon,
    Dot,
    Plus,
    PlusPlus,
    Minus,
    Star,
    Slash,
    SlashSlash,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Not,
    AndAnd,
    OrOr,
    Question,
    Comma,
    At,
    Ellipsis,
    DollarBrace,
    PathLit(String),
}

fn px_uri_scheme_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.'
}

fn px_uri_body_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || c == '%'
        || c == '/'
        || c == '?'
        || c == ':'
        || c == '@'
        || c == '&'
        || c == '='
        || c == '+'
        || c == '$'
        || c == ','
        || c == '-'
        || c == '_'
        || c == '.'
        || c == '!'
        || c == '~'
        || c == '*'
        || c == '\''
}

/// Match Nix 2.34.7's lexer rule exactly:
/// `[A-Za-z][A-Za-z0-9+.-]*:[A-Za-z0-9%/?:@&=+$,_.!~*'-]+`.
/// Flex chooses the URI
/// by maximal match over the shorter identifier/colon token sequence.
fn px_uri_end(chars: &Vec<char>, start: usize) -> Option<usize> {
    if start >= chars.len() || !chars[start].is_ascii_alphabetic() {
        return None;
    }
    let mut i = start + 1;
    while i < chars.len() && px_uri_scheme_char(chars[i]) {
        i += 1;
    }
    if i >= chars.len() || chars[i] != ':' {
        return None;
    }
    let body_start = i + 1;
    i = body_start;
    while i < chars.len() && px_uri_body_char(chars[i]) {
        i += 1;
    }
    if i == body_start { None } else { Some(i) }
}

/// Nix indented-string ('') stripping: remove the common leading-SPACE indent
/// (tabs are NOT counted as indentation — nix counts spaces only, oracle-
/// pinned 2026-07-13) and drop a single leading newline. Mirrors the
/// oracle-correct clj/hy algorithm, space-only.
fn px_strip_indented(parts: &Vec<(bool, String)>) -> Vec<(bool, String)> {
    // combined text: literals verbatim, interpolations as a content marker.
    let mut combined = String::new();
    let mut ends_nl = false;
    for (is_sub, text) in parts.iter() {
        if *is_sub {
            combined.push_str("${}");
            ends_nl = false;
        } else {
            combined.push_str(text);
            for ch in text.chars() {
                ends_nl = ch == '\n';
            }
        }
    }
    // lines = split on '\n', drop trailing empty when combined ends with '\n'.
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in combined.chars() {
        if ch == '\n' {
            lines.push(cur.clone());
            cur = String::new();
        } else {
            cur.push(ch);
        }
    }
    lines.push(cur);
    if ends_nl {
        lines.pop();
    }
    // min indent over non-blank lines (leading SPACES only, stop at non-space).
    let mut min_indent: i64 = -1;
    for line in lines.iter() {
        let mut all_ws = true;
        for ch in line.chars() {
            if ch != ' ' && ch != '\t' {
                all_ws = false;
                break;
            }
        }
        if all_ws {
            continue;
        }
        let mut indent: i64 = 0;
        for ch in line.chars() {
            if ch == ' ' {
                indent += 1;
            } else {
                break;
            }
        }
        if min_indent < 0 || indent < min_indent {
            min_indent = indent;
        }
    }
    if min_indent < 0 {
        min_indent = 0;
    }
    // strip pass
    let mut result: Vec<(bool, String)> = Vec::new();
    let mut is_first = true;
    let mut at_line_start = true;
    let mut chars_stripped: i64 = 0;
    for (is_sub, text) in parts.iter() {
        if *is_sub {
            is_first = false;
            at_line_start = false;
            chars_stripped = 0;
            result.push((true, text.clone()));
            continue;
        }
        let mut out = String::new();
        for ch in text.chars() {
            if ch == '\n' {
                if is_first && out.is_empty() {
                    is_first = false;
                    at_line_start = true;
                    chars_stripped = 0;
                    continue;
                }
                is_first = false;
                out.push('\n');
                at_line_start = true;
                chars_stripped = 0;
            } else if at_line_start && ch == ' ' && chars_stripped < min_indent {
                chars_stripped += 1;
            } else {
                is_first = false;
                at_line_start = false;
                out.push(ch);
            }
        }
        if !out.is_empty() || result.is_empty() {
            result.push((false, out));
        }
    }
    result
}

fn px_parse_float_lexeme(text: &str) -> Result<f64, String> {
    let f = match text.parse::<f64>() {
        Ok(v) => v,
        Err(_) => return Err(format!("px: bad float {}", text)),
    };
    let mut mantissa_nonzero = false;
    for ch in text.chars() {
        if ch == 'e' || ch == 'E' {
            break;
        }
        if ch >= '1' && ch <= '9' {
            mantissa_nonzero = true;
        }
    }
    let min_normal = match "2.2250738585072014e-308".parse::<f64>() {
        Ok(v) => v,
        Err(_) => return Err(String::from("px: failed to construct f64 minimum")),
    };
    if f - f != 0.0
        || (f == 0.0 && mantissa_nonzero)
        || (f > 0.0 && f < min_normal)
    {
        return Err(format!("px: bad float {}", text));
    }
    Ok(f)
}

fn px_lex_push(toks: &mut Vec<PxTok>, offs: &mut Vec<usize>, tok: PxTok, off: usize) {
    toks.push(tok);
    offs.push(off);
}

fn px_lex(src: &str) -> Result<(Vec<PxTok>, Vec<usize>), String> {
    let chars = src.chars().collect::<Vec<char>>();
    let mut toks: Vec<PxTok> = Vec::new();
    let mut offs: Vec<usize> = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        let start = i;
        if c == ' ' || c == '\n' || c == '\t' || c == '\r' {
            i += 1;
        } else if c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '\'' && i + 1 < chars.len() && chars[i + 1] == '\'' {
            // Nix INDENTED string ''...''  (harvest floor-raise 2026-07-13,
            // oracle-pinned). Escapes: '' + ' => '' ; '' + $ => $ ;
            // '' + \X => esc(X). Interpolation ${...}. Then strip the common
            // leading-SPACE indent (tabs are NOT indentation in nix) and drop
            // a leading newline. clj/hy already support this; rs was the gap.
            i += 2;
            let mut parts: Vec<(bool, String)> = Vec::new();
            let mut lit = String::new();
            loop {
                if i >= chars.len() {
                    return Err(String::from("px: unterminated indented string"));
                }
                let ch = chars[i];
                if ch == '\'' && i + 1 < chars.len() && chars[i + 1] == '\'' {
                    if i + 2 < chars.len() && chars[i + 2] == '\'' {
                        lit.push('\'');
                        lit.push('\'');
                        i += 3;
                    } else if i + 2 < chars.len() && chars[i + 2] == '$' {
                        lit.push('$');
                        i += 3;
                    } else if i + 2 < chars.len() && chars[i + 2] == '\\' {
                        let e = if i + 3 < chars.len() { chars[i + 3] } else { '\\' };
                        if e == 'n' {
                            lit.push('\n');
                        } else if e == 't' {
                            lit.push('\t');
                        } else if e == 'r' {
                            lit.push('\r');
                        } else {
                            lit.push(e);
                        }
                        i += 4;
                    } else {
                        i += 2;
                        break;
                    }
                } else if ch == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                    parts.push((false, lit.clone()));
                    lit = String::new();
                    i += 2;
                    let mut depth = 1usize;
                    let mut sub = String::new();
                    while depth > 0 {
                        if i >= chars.len() {
                            return Err(String::from("px: unterminated interpolation"));
                        }
                        let sc = chars[i];
                        if sc == '{' {
                            depth += 1;
                            sub.push(sc);
                        } else if sc == '}' {
                            depth -= 1;
                            if depth > 0 {
                                sub.push(sc);
                            }
                        } else {
                            sub.push(sc);
                        }
                        i += 1;
                    }
                    parts.push((true, sub));
                } else {
                    lit.push(ch);
                    i += 1;
                }
            }
            parts.push((false, lit));
            let stripped = px_strip_indented(&parts);
            px_lex_push(&mut toks, &mut offs, PxTok::Str(stripped), start);
        } else if c == '"' {
            i += 1;
            let mut parts: Vec<(bool, String)> = Vec::new();
            let mut lit = String::new();
            loop {
                if i >= chars.len() {
                    return Err(String::from("px: unterminated string"));
                }
                let ch = chars[i];
                if ch == '"' {
                    i += 1;
                    break;
                }
                if ch == '\\' {
                    i += 1;
                    if i >= chars.len() {
                        return Err(String::from("px: dangling string escape"));
                    }
                    let esc = chars[i];
                    if esc == 'n' {
                        lit.push('\n');
                    } else if esc == 't' {
                        lit.push('\t');
                    } else if esc == 'r' {
                        // Nix supports \r (oracle: `"\r"` == carriage return);
                        // needed by kernel-prims is_space (K1 floor-raise).
                        lit.push('\r');
                    } else if esc == '\\' {
                        lit.push('\\');
                    } else if esc == '"' {
                        lit.push('"');
                    } else if esc == '$' {
                        lit.push('$');
                    } else {
                        // Nix rule: an UNKNOWN escape `\c` drops the backslash
                        // and keeps the char literally (oracle: `"a\ub"` ==
                        // "aub", `"\1"` == "1", `"\ "` == " "). Only n/t/r/\\/"/$
                        // are special; everything else is the bare char.
                        lit.push(esc);
                    }
                    i += 1;
                } else if ch == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                    parts.push((false, lit.clone()));
                    lit = String::new();
                    i += 2;
                    let mut depth = 1usize;
                    let mut sub = String::new();
                    while depth > 0 {
                        if i >= chars.len() {
                            return Err(String::from("px: unterminated interpolation"));
                        }
                        let sc = chars[i];
                        if sc == '{' {
                            depth += 1;
                            sub.push(sc);
                        } else if sc == '}' {
                            depth -= 1;
                            if depth > 0 {
                                sub.push(sc);
                            }
                        } else {
                            sub.push(sc);
                        }
                        i += 1;
                    }
                    parts.push((true, sub));
                } else {
                    lit.push(ch);
                    i += 1;
                }
            }
            parts.push((false, lit));
            px_lex_push(&mut toks, &mut offs, PxTok::Str(parts), start);
        } else if c == '(' {
            px_lex_push(&mut toks, &mut offs, PxTok::LParen, start);
            i += 1;
        } else if c == ')' {
            px_lex_push(&mut toks, &mut offs, PxTok::RParen, start);
            i += 1;
        } else if c == '{' {
            px_lex_push(&mut toks, &mut offs, PxTok::LBrace, start);
            i += 1;
        } else if c == '}' {
            px_lex_push(&mut toks, &mut offs, PxTok::RBrace, start);
            i += 1;
        } else if c == '[' {
            px_lex_push(&mut toks, &mut offs, PxTok::LBracket, start);
            i += 1;
        } else if c == ']' {
            px_lex_push(&mut toks, &mut offs, PxTok::RBracket, start);
            i += 1;
        } else if c == ';' {
            px_lex_push(&mut toks, &mut offs, PxTok::Semi, start);
            i += 1;
        } else if c == ':' {
            px_lex_push(&mut toks, &mut offs, PxTok::Colon, start);
            i += 1;
        } else if c == '.'
            && ((i + 1 < chars.len() && chars[i + 1] == '/')
                || (i + 2 < chars.len() && chars[i + 1] == '.' && chars[i + 2] == '/'))
        {
            // Relative path literal `./x` / `../x` (Nix). In the seed, paths
            // are only meaningful as `import` arguments; the parser marks
            // them and load-time expansion resolves them (px_eval stays pure).
            let mut p = String::new();
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric()
                    || chars[i] == '_' || chars[i] == '\'' || chars[i] == '.'
                    || chars[i] == '/' || chars[i] == '-')
            {
                p.push(chars[i]);
                i += 1;
            }
            px_lex_push(&mut toks, &mut offs, PxTok::PathLit(p), start);
        } else if c == '/'
            && !matches!(toks.last(), Some(PxTok::Int(_)) | Some(PxTok::Float(_)))
            && i + 1 < chars.len()
            && chars[i + 1] != '/'
            && !chars[i + 1].is_whitespace()
            && !matches!(chars[i + 1], ')' | ']' | '}' | ';')
        {
            // Absolute path literal `/a/b` (Nix) -- same shape as the `./`/`../`
            // case above, just without the leading dot segment. Guarded against
            // following a number (`1/0` stays division), a second `/` (the
            // `//` update operator, tokenized further below as SlashSlash --
            // must not be shadowed here) AND against being followed by
            // whitespace/a closer (`a / b`, `(a/)` stay division
            // / syntax, not a bogus path) -- same two-sided disambiguation
            // pnix-clr's path-start? already uses.
            let mut p = String::new();
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric()
                    || chars[i] == '_' || chars[i] == '\'' || chars[i] == '.'
                    || chars[i] == '/' || chars[i] == '-')
            {
                p.push(chars[i]);
                i += 1;
            }
            px_lex_push(&mut toks, &mut offs, PxTok::PathLit(p), start);
        } else if c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            let mut digits = String::from("0.");
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                digits.push(chars[i]);
                i += 1;
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                digits.push(chars[i]);
                i += 1;
                if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                    digits.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() || !chars[i].is_ascii_digit() {
                    return Err(format!("px: bad float {}", digits));
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    digits.push(chars[i]);
                    i += 1;
                }
            }
            px_lex_push(&mut toks, &mut offs, PxTok::Float(px_parse_float_lexeme(&digits)?), start);
        } else if c == '.' {
            if i + 2 < chars.len() && chars[i + 1] == '.' && chars[i + 2] == '.' {
                // `...` ellipsis (attrset-pattern "accept extra keys")
                px_lex_push(&mut toks, &mut offs, PxTok::Ellipsis, start);
                i += 3;
            } else {
                px_lex_push(&mut toks, &mut offs, PxTok::Dot, start);
                i += 1;
            }
        } else if c == ',' {
            px_lex_push(&mut toks, &mut offs, PxTok::Comma, start);
            i += 1;
        } else if c == '@' {
            px_lex_push(&mut toks, &mut offs, PxTok::At, start);
            i += 1;
        } else if c == '+' {
            if i + 1 < chars.len() && chars[i + 1] == '+' {
                px_lex_push(&mut toks, &mut offs, PxTok::PlusPlus, start);
                i += 2;
            } else {
                px_lex_push(&mut toks, &mut offs, PxTok::Plus, start);
                i += 1;
            }
        } else if c == '-' {
            px_lex_push(&mut toks, &mut offs, PxTok::Minus, start);
            i += 1;
        } else if c == '*' {
            px_lex_push(&mut toks, &mut offs, PxTok::Star, start);
            i += 1;
        } else if c == '/' {
            if i + 1 < chars.len() && chars[i + 1] == '/' {
                px_lex_push(&mut toks, &mut offs, PxTok::SlashSlash, start);
                i += 2;
            } else {
                px_lex_push(&mut toks, &mut offs, PxTok::Slash, start);
                i += 1;
            }
        } else if c == '=' {
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                px_lex_push(&mut toks, &mut offs, PxTok::EqEq, start);
                i += 2;
            } else {
                px_lex_push(&mut toks, &mut offs, PxTok::Assign, start);
                i += 1;
            }
        } else if c == '!' {
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                px_lex_push(&mut toks, &mut offs, PxTok::Ne, start);
                i += 2;
            } else {
                // Logical NOT (Nix `!`, prec 8). Desugared to `if` at parse.
                px_lex_push(&mut toks, &mut offs, PxTok::Not, start);
                i += 1;
            }
        } else if c == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
            // Expression-level `${` — dynamic attr name (Nix `.${e}` select /
            // `{ ${e} = v; }` keys already handled in strings; this is the
            // SELECT position). Closed by the ordinary RBrace token.
            px_lex_push(&mut toks, &mut offs, PxTok::DollarBrace, start);
            i += 2;
        } else if c == '?' {
            // Nix has-attr operator (prec 4, between apply and ++).
            px_lex_push(&mut toks, &mut offs, PxTok::Question, start);
            i += 1;
        } else if c == '&' {
            if i + 1 < chars.len() && chars[i + 1] == '&' {
                px_lex_push(&mut toks, &mut offs, PxTok::AndAnd, start);
                i += 2;
            } else {
                return Err(String::from("px: unexpected '&'"));
            }
        } else if c == '|' {
            if i + 1 < chars.len() && chars[i + 1] == '|' {
                px_lex_push(&mut toks, &mut offs, PxTok::OrOr, start);
                i += 2;
            } else {
                return Err(String::from("px: unexpected '|'"));
            }
        } else if c == '<' {
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                px_lex_push(&mut toks, &mut offs, PxTok::Le, start);
                i += 2;
            } else {
                px_lex_push(&mut toks, &mut offs, PxTok::Lt, start);
                i += 1;
            }
        } else if c == '>' {
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                px_lex_push(&mut toks, &mut offs, PxTok::Ge, start);
                i += 2;
            } else {
                px_lex_push(&mut toks, &mut offs, PxTok::Gt, start);
                i += 1;
            }
        } else if c.is_ascii_digit() {
            let mut digits = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                digits.push(chars[i]);
                i += 1;
            }
            if i < chars.len() && chars[i] == '.' {
                if digits.len() > 1 && digits.starts_with("0") {
                    match digits.parse::<i64>() {
                        Ok(n) => px_lex_push(&mut toks, &mut offs, PxTok::Int(n), start),
                        Err(_) => return Err(format!("px: bad integer {}", digits)),
                    }
                    continue;
                }
                digits.push('.');
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    digits.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                    digits.push(chars[i]);
                    i += 1;
                    if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                        digits.push(chars[i]);
                        i += 1;
                    }
                    if i >= chars.len() || !chars[i].is_ascii_digit() {
                        return Err(format!("px: bad float {}", digits));
                    }
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        digits.push(chars[i]);
                        i += 1;
                    }
                }
                px_lex_push(&mut toks, &mut offs, PxTok::Float(px_parse_float_lexeme(&digits)?), start);
            } else {
                match digits.parse::<i64>() {
                    Ok(n) => px_lex_push(&mut toks, &mut offs, PxTok::Int(n), start),
                    Err(_) if digits == "9223372036854775808" => {
                        px_lex_push(&mut toks, &mut offs, PxTok::IntMinMagnitude, start)
                    }
                    Err(_) => return Err(format!("px: bad integer {}", digits)),
                }
            }
        } else if c.is_ascii_alphabetic() || c == '_' {
            if let Some(end) = px_uri_end(&chars, i) {
                let uri = chars[i..end].iter().collect::<String>();
                px_lex_push(&mut toks, &mut offs, PxTok::Uri(uri), start);
                i = end;
                continue;
            }
            let mut word = String::new();
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '\''
                    // Nix idents CONTINUE through '-' (kebab keys like
                    // `mathml-content`; oracle: `a-b` is one ident,
                    // `a -b` is subtraction — '-' never STARTS an ident)
                    || chars[i] == '-')
            {
                word.push(chars[i]);
                i += 1;
            }
            if word == "let" {
                px_lex_push(&mut toks, &mut offs, PxTok::KwLet, start);
            } else if word == "in" {
                px_lex_push(&mut toks, &mut offs, PxTok::KwIn, start);
            } else if word == "if" {
                px_lex_push(&mut toks, &mut offs, PxTok::KwIf, start);
            } else if word == "then" {
                px_lex_push(&mut toks, &mut offs, PxTok::KwThen, start);
            } else if word == "else" {
                px_lex_push(&mut toks, &mut offs, PxTok::KwElse, start);
            } else if word == "true" {
                px_lex_push(&mut toks, &mut offs, PxTok::True, start);
            } else if word == "false" {
                px_lex_push(&mut toks, &mut offs, PxTok::False, start);
            } else if word == "null" {
                px_lex_push(&mut toks, &mut offs, PxTok::Null, start);
            } else if word == "with" {
                px_lex_push(&mut toks, &mut offs, PxTok::KwWith, start);
            } else if word == "assert" {
                px_lex_push(&mut toks, &mut offs, PxTok::KwAssert, start);
            } else {
                px_lex_push(&mut toks, &mut offs, PxTok::Ident(word), start);
            }
        } else {
            return Err(format!("px: unexpected character {}", c));
        }
    }
    Ok((toks, offs))
}

// ---- parser -----------------------------------------------------------------
//
// expr    := lambda | if | let | eq
// eq      := cmp (('=='|'!=') cmp)?
// cmp     := update (('<'|'<='|'>'|'>=') update)?
// update  := add ('//' update)?              (right-assoc, Nix `//`)
// add     := mul (('+'|'-') mul)*
// mul     := concat (('*'|'/') concat)*
// concat  := apply ('++' concat)?            (right-assoc, Nix `++`)
// apply   := select select*                  (juxtaposition, left-assoc)
// select  := atom ('.' IDENT)*
// atom    := INT | true | false | IDENT | STR | '(' expr ')'
//          | '[' select* ']' | '{' (IDENT '=' expr ';')* '}'

struct PxParser {
    toks: Vec<PxTok>,
    offs: Vec<usize>,
    pos: usize,
    src: String,
}

impl PxParser {
    fn cur_off(&self) -> usize {
        match self.offs.get(self.pos) {
            Some(o) => *o,
            None => self.src.len(),
        }
    }

    fn source_line_column(&self, offset: usize) -> (i64, i64) {
        let chars = self.src.chars().collect::<Vec<char>>();
        let n = chars.len();
        let clamped = if offset > n { n } else { offset };
        let mut line = 1i64;
        let mut last_nl: i64 = -1;
        let mut i = 0usize;
        while i < clamped {
            if chars[i] == '\n' {
                line += 1;
                last_nl = i as i64;
            }
            i += 1;
        }
        let column = if last_nl < 0 {
            clamped as i64 + 1
        } else {
            clamped as i64 - last_nl
        };
        (line, column)
    }

    fn position_expr(&self, offset: usize) -> PxExpr {
        let (line, column) = self.source_line_column(offset);
        PxExpr::Attrs(vec![
            (
                String::from("file"),
                PxExpr::Str(vec![PxStrPart::Lit(String::from("<pnix-px>"))]),
            ),
            (String::from("line"), PxExpr::Int(line)),
            (String::from("column"), PxExpr::Int(column)),
        ])
    }

    fn cur_desc(&self) -> String {
        match self.toks.get(self.pos) {
            Some(t) => format!("{:?}", t),
            None => String::from("<eof>"),
        }
    }

    fn peek_is(&self, t: PxTok) -> bool {
        match self.toks.get(self.pos) {
            Some(cur) => *cur == t,
            None => false,
        }
    }

    fn eat(&mut self, t: PxTok) -> Result<(), String> {
        if self.peek_is(t.clone()) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!(
                "px parse: expected {:?}, found {}",
                t,
                self.cur_desc()
            ))
        }
    }

    fn ident(&mut self) -> Result<String, String> {
        let tok = self.toks.get(self.pos).cloned();
        match tok {
            Some(PxTok::Ident(s)) => {
                self.pos += 1;
                Ok(s)
            }
            _ => Err(format!(
                "px parse: expected identifier, found {}",
                self.cur_desc()
            )),
        }
    }

    /// Find the index of the RBrace matching an LBrace at `open` (depth-
    /// tracked over nested {}/[]/() so attrset defaults nest correctly).
    /// Returns None if unbalanced.
    fn matching_rbrace(&self, open: usize) -> Option<usize> {
        let mut depth = 0i64;
        let mut j = open;
        while j < self.toks.len() {
            match self.toks.get(j) {
                Some(PxTok::LBrace) | Some(PxTok::LBracket) | Some(PxTok::LParen)
                | Some(PxTok::DollarBrace) => depth += 1,
                Some(PxTok::RBrace) => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(j);
                    }
                }
                Some(PxTok::RBracket) | Some(PxTok::RParen) => depth -= 1,
                None => return None,
                _ => {}
            }
            j += 1;
        }
        None
    }

    /// Detect an attrset-pattern lambda at self.pos. Returns
    /// Some((brace_open_idx, capture_name_option)) if it is one.
    /// Forms: `{...}:`, `{...}@ID:`, `ID@{...}:`.
    fn attrset_pattern_here(&self) -> Option<(usize, Option<String>)> {
        // Form: ID @ { ... }
        if let (Some(PxTok::Ident(name)), Some(PxTok::At), Some(PxTok::LBrace)) = (
            self.toks.get(self.pos),
            self.toks.get(self.pos + 1),
            self.toks.get(self.pos + 2),
        ) {
            let open = self.pos + 2;
            if let Some(close) = self.matching_rbrace(open) {
                if let Some(PxTok::Colon) = self.toks.get(close + 1) {
                    return Some((open, Some(name.clone())));
                }
            }
            return None;
        }
        // Form: { ... } (: | @ ID :)
        if let Some(PxTok::LBrace) = self.toks.get(self.pos) {
            let open = self.pos;
            if let Some(close) = self.matching_rbrace(open) {
                match self.toks.get(close + 1) {
                    Some(PxTok::Colon) => return Some((open, None)),
                    Some(PxTok::At) => {
                        if let (Some(PxTok::Ident(name)), Some(PxTok::Colon)) =
                            (self.toks.get(close + 2), self.toks.get(close + 3))
                        {
                            return Some((open, Some(name.clone())));
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Parse and DESUGAR an attrset-pattern lambda into a plain lambda over
    /// the captured (or fresh) argument, with the formals bound via
    /// builtins.getAttr / hasAttr-or-default and (without `...`) a strict
    /// unknown-key guard. Semantics oracle-pinned (nix 2.34): @-capture is
    /// the RAW passed set (excludes defaults); defaults see sibling formals
    /// (rec let); no-ellipsis rejects unknown keys; missing required = error.
    fn parse_pattern_lambda(&mut self) -> Result<PxExpr, String> {
        let (open, capture) = self.attrset_pattern_here().unwrap();
        let arg_name = capture.clone().unwrap_or_else(|| String::from("__pat_arg"));
        // Advance to just inside the brace.
        self.pos = open + 1;
        let mut formals: Vec<(String, Option<PxExpr>)> = Vec::new();
        let mut ellipsis = false;
        loop {
            match self.toks.get(self.pos).cloned() {
                Some(PxTok::RBrace) => {
                    self.pos += 1;
                    break;
                }
                Some(PxTok::Ellipsis) => {
                    ellipsis = true;
                    self.pos += 1;
                    // optional trailing comma then must be RBrace
                    if self.peek_is(PxTok::Comma) {
                        self.pos += 1;
                    }
                }
                Some(PxTok::Comma) => {
                    self.pos += 1;
                }
                Some(PxTok::Ident(fname)) => {
                    self.pos += 1;
                    let default = if self.peek_is(PxTok::Question) {
                        self.pos += 1;
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    formals.push((fname, default));
                }
                other => {
                    return Err(format!(
                        "px parse: bad attrset-pattern formal, found {:?}",
                        other
                    ));
                }
            }
        }
        // Consume the trailing capture/colon.
        if capture.is_some() {
            // Either `ID @ { } :` (capture already consumed at pos start) or
            // `{ } @ ID :` (consume @ ID here).
            if self.peek_is(PxTok::At) {
                self.pos += 1; // @
                let _ = self.ident()?; // ID (already captured)
            }
        }
        self.eat(PxTok::Colon)?;
        let body = self.parse_expr()?;

        // ---- build the desugared body ----
        let argv = || PxExpr::Var(arg_name.clone());
        let bi = |n: &str| PxExpr::Select {
            base: Box::new(PxExpr::Var(String::from("builtins"))),
            name: String::from(n),
        };
        let apply = |f: PxExpr, a: PxExpr| PxExpr::Apply {
            func: Box::new(f),
            arg: Box::new(a),
        };
        let sstr = |t: &str| PxExpr::Str(vec![PxStrPart::Lit(String::from(t))]);

        // formal let-bindings
        let mut bindings: Vec<(String, PxExpr)> = Vec::new();
        for (fname, default) in &formals {
            let get = apply(apply(bi("getAttr"), sstr(fname)), argv());
            let value = match default {
                None => get,
                Some(d) => {
                    let has = apply(apply(bi("hasAttr"), sstr(fname)), argv());
                    PxExpr::If {
                        cond: Box::new(has),
                        then_e: Box::new(get),
                        else_e: Box::new(d.clone()),
                    }
                }
            };
            bindings.push((fname.clone(), value));
        }
        let inner = if bindings.is_empty() {
            body
        } else {
            PxExpr::LetIn {
                bindings,
                body: Box::new(body),
            }
        };

        // Guards (Nix checks arity EAGERLY at call, independent of use):
        //  (1) every REQUIRED formal (no default) must be present — ALWAYS;
        //  (2) no UNKNOWN key present — only WITHOUT `...`.
        // count-based: `length (filter <pred> <names>) == 0`.
        let count_zero = |pred_param: &str, pred_body: PxExpr, list: PxExpr| -> PxExpr {
            let pred = PxExpr::Lambda {
                param: String::from(pred_param),
                body: Rc::new(pred_body),
            };
            let filtered = apply(apply(bi("filter"), pred), list);
            PxExpr::Binary {
                op: PxOp::Eq,
                lhs: Box::new(apply(bi("length"), filtered)),
                rhs: Box::new(PxExpr::Int(0)),
            }
        };
        // (1) missing-required: filter (n: ! hasAttr n arg) [required]  == []
        let required = PxExpr::List(
            formals
                .iter()
                .filter(|(_, d)| d.is_none())
                .map(|(n, _)| sstr(n))
                .collect(),
        );
        let has_n = apply(apply(bi("hasAttr"), PxExpr::Var(String::from("__rq"))), argv());
        let not_has = PxExpr::If {
            cond: Box::new(has_n),
            then_e: Box::new(PxExpr::Bool(false)),
            else_e: Box::new(PxExpr::Bool(true)),
        };
        let missing_ok = count_zero("__rq", not_has, required);
        // (2) unknown-key (no ellipsis only): filter (k: ! elem k known) (attrNames arg) == []
        let unknown_ok = if ellipsis {
            PxExpr::Bool(true)
        } else {
            let known = PxExpr::List(formals.iter().map(|(n, _)| sstr(n)).collect());
            let elem_call =
                apply(apply(bi("elem"), PxExpr::Var(String::from("__k"))), known);
            let not_elem = PxExpr::If {
                cond: Box::new(elem_call),
                then_e: Box::new(PxExpr::Bool(false)),
                else_e: Box::new(PxExpr::Bool(true)),
            };
            count_zero("__k", not_elem, apply(bi("attrNames"), argv()))
        };
        // `missing_ok && unknown_ok` — `&&` is if-desugared on the floor.
        let guard = PxExpr::If {
            cond: Box::new(missing_ok),
            then_e: Box::new(unknown_ok),
            else_e: Box::new(PxExpr::Bool(false)),
        };
        let throw_e = apply(
            PxExpr::Var(String::from("throw")),
            sstr("attrset pattern: argument mismatch (missing required or unexpected key)"),
        );
        let full_body = PxExpr::If {
            cond: Box::new(guard),
            then_e: Box::new(inner),
            else_e: Box::new(throw_e),
        };
        Ok(PxExpr::Lambda {
            param: arg_name,
            body: Rc::new(full_body),
        })
    }

    fn parse_expr(&mut self) -> Result<PxExpr, String> {
        // attrset-pattern lambda: `{formals}:` / `{formals}@id:` / `id@{formals}:`
        if self.attrset_pattern_here().is_some() {
            return self.parse_pattern_lambda();
        }
        // lambda: IDENT ':' expr
        let cur = self.toks.get(self.pos).cloned();
        let next = self.toks.get(self.pos + 1).cloned();
        match (cur, next) {
            (Some(PxTok::Ident(name)), Some(PxTok::Colon)) => {
                self.pos += 2;
                let body = self.parse_expr()?;
                return Ok(PxExpr::Lambda {
                    param: name,
                    body: Rc::new(body),
                });
            }
            _ => {}
        }
        if self.peek_is(PxTok::KwIf) {
            self.pos += 1;
            let cond = self.parse_expr()?;
            self.eat(PxTok::KwThen)?;
            let then_e = self.parse_expr()?;
            self.eat(PxTok::KwElse)?;
            let else_e = self.parse_expr()?;
            return Ok(PxExpr::If {
                cond: Box::new(cond),
                then_e: Box::new(then_e),
                else_e: Box::new(else_e),
            });
        }
        if self.peek_is(PxTok::KwLet) {
            return self.parse_let();
        }
        if self.peek_is(PxTok::KwAssert) {
            // Nix `assert cond; body` — desugared: failure throws (both sides
            // error; message parity is not part of the contract).
            self.pos += 1;
            let cond = self.parse_expr()?;
            self.eat(PxTok::Semi)?;
            let body = self.parse_expr()?;
            return Ok(PxExpr::If {
                cond: Box::new(cond),
                then_e: Box::new(body),
                else_e: Box::new(PxExpr::Apply {
                    func: Box::new(PxExpr::Select {
                        base: Box::new(PxExpr::Var(String::from("builtins"))),
                        name: String::from("throw"),
                    }),
                    arg: Box::new(PxExpr::Str(vec![PxStrPart::Lit(String::from(
                        "assertion failed",
                    ))])),
                }),
            });
        }
        if self.peek_is(PxTok::KwWith) {
            self.pos += 1;
            let scope = self.parse_expr()?;
            self.eat(PxTok::Semi)?;
            let body = self.parse_expr()?;
            return Ok(PxExpr::With {
                scope: Box::new(scope),
                body: Box::new(body),
            });
        }
        self.parse_or()
    }

    // Logical operators (Nix precedence: `||` loosest (13), `&&` (12), then
    // comparison/eq below, and unary `!` (8) tighter than `//`, looser than
    // `+`). Desugared to `if` so short-circuit + bool-strictness come from the
    // existing evalIf (no new PxExpr/PxOp variant; stays in the rs-meta subset).
    fn parse_or(&mut self) -> Result<PxExpr, String> {
        let mut lhs = self.parse_and()?;
        while self.peek_is(PxTok::OrOr) {
            self.pos += 1;
            let rhs = self.parse_and()?;
            // a || b  ==  if a then true else b
            lhs = PxExpr::If {
                cond: Box::new(lhs),
                then_e: Box::new(PxExpr::Bool(true)),
                else_e: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<PxExpr, String> {
        let mut lhs = self.parse_eq()?;
        while self.peek_is(PxTok::AndAnd) {
            self.pos += 1;
            let rhs = self.parse_eq()?;
            // a && b  ==  if a then b else false
            lhs = PxExpr::If {
                cond: Box::new(lhs),
                then_e: Box::new(rhs),
                else_e: Box::new(PxExpr::Bool(false)),
            };
        }
        Ok(lhs)
    }

    fn parse_not(&mut self) -> Result<PxExpr, String> {
        if self.peek_is(PxTok::Not) {
            self.pos += 1;
            let operand = self.parse_not()?; // allow !!x; operand is add-level
            // !x  ==  if x then false else true
            return Ok(PxExpr::If {
                cond: Box::new(operand),
                then_e: Box::new(PxExpr::Bool(false)),
                else_e: Box::new(PxExpr::Bool(true)),
            });
        }
        self.parse_add()
    }

    fn parse_let(&mut self) -> Result<PxExpr, String> {
        self.eat(PxTok::KwLet)?;
        let mut bindings = Vec::new();
        // Same `:`-marked-name trick parse_attrset_literal's `rec { inherit
        // x; }` uses: a plain `inherit x;` must resolve `x` in the scope
        // ENCLOSING this let, not this let's own (mutually recursive)
        // bindings. Capture such names under a `:`-marked binding in a
        // separate outer LetIn wrapper instead of adding evaluator-side
        // scope-resolution logic.
        let mut outer: Vec<(String, PxExpr)> = Vec::new();
        while !self.peek_is(PxTok::KwIn) {
            if let Some(PxTok::Ident(kw)) = self.toks.get(self.pos) {
                if kw == "inherit" && !matches!(self.toks.get(self.pos + 1), Some(PxTok::Assign)) {
                    self.pos += 1;
                    let mut from: Option<PxExpr> = None;
                    if self.peek_is(PxTok::LParen) {
                        self.pos += 1;
                        let e = self.parse_expr()?;
                        self.eat(PxTok::RParen)?;
                        from = Some(e);
                    }
                    let mut got_any = false;
                    while !self.peek_is(PxTok::Semi) {
                        match self.toks.get(self.pos).cloned() {
                            Some(PxTok::Ident(n)) => {
                                self.pos += 1;
                                got_any = true;
                                match &from {
                                    Some(e) => bindings.push((
                                        n.clone(),
                                        PxExpr::Select {
                                            base: Box::new(e.clone()),
                                            name: n.clone(),
                                        },
                                    )),
                                    None => {
                                        let marked = format!(":{}", n);
                                        outer.push((marked.clone(), PxExpr::Var(n.clone())));
                                        bindings.push((n, PxExpr::Var(marked)));
                                    }
                                }
                            }
                            _ => {
                                return Err(format!(
                                    "px parse: inherit expects attribute names, found {}",
                                    self.cur_desc()
                                ))
                            }
                        }
                    }
                    if !got_any {
                        return Err(String::from("px parse: empty inherit"));
                    }
                    self.eat(PxTok::Semi)?;
                    continue;
                }
            }
            // Nested static path `a.b = v` desugars to `a = { b = v; }`,
            // same as attrset literals (parse_attrset_literal / merge_attr_field).
            let mut path = vec![self.ident()?];
            while self.peek_is(PxTok::Dot) {
                if let Some(PxTok::Ident(n2)) = self.toks.get(self.pos + 1).cloned() {
                    self.pos += 1; // Dot
                    self.pos += 1; // Ident
                    path.push(n2);
                } else {
                    break;
                }
            }
            self.eat(PxTok::Assign)?;
            let mut value = self.parse_expr()?;
            self.eat(PxTok::Semi)?;
            let mut i = path.len();
            while i > 1 {
                i -= 1;
                value = PxExpr::Attrs(vec![(path[i].clone(), value)]);
            }
            bindings = merge_let_binding(bindings, path[0].clone(), value);
        }
        self.eat(PxTok::KwIn)?;
        let body = self.parse_expr()?;
        let inner = PxExpr::LetIn {
            bindings,
            body: Box::new(body),
        };
        Ok(if outer.is_empty() {
            inner
        } else {
            PxExpr::LetIn {
                bindings: outer,
                body: Box::new(inner),
            }
        })
    }

    fn parse_eq(&mut self) -> Result<PxExpr, String> {
        let lhs = self.parse_cmp()?;
        let op = match self.toks.get(self.pos) {
            Some(PxTok::EqEq) => Some(PxOp::Eq),
            Some(PxTok::Ne) => Some(PxOp::Ne),
            _ => None,
        };
        match op {
            Some(op) => {
                self.pos += 1;
                let rhs = self.parse_cmp()?;
                Ok(PxExpr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                })
            }
            None => Ok(lhs),
        }
    }

    fn parse_cmp(&mut self) -> Result<PxExpr, String> {
        let lhs = self.parse_update()?;
        let op = match self.toks.get(self.pos) {
            Some(PxTok::Lt) => Some(PxOp::Lt),
            Some(PxTok::Le) => Some(PxOp::Le),
            Some(PxTok::Gt) => Some(PxOp::Gt),
            Some(PxTok::Ge) => Some(PxOp::Ge),
            _ => None,
        };
        match op {
            Some(op) => {
                self.pos += 1;
                let rhs = self.parse_update()?;
                Ok(PxExpr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                })
            }
            None => Ok(lhs),
        }
    }

    fn parse_update(&mut self) -> Result<PxExpr, String> {
        let lhs = self.parse_not()?;
        if self.peek_is(PxTok::SlashSlash) {
            self.pos += 1;
            let rhs = self.parse_update()?;
            return Ok(PxExpr::Binary {
                op: PxOp::Update,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<PxExpr, String> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.toks.get(self.pos) {
                Some(PxTok::Plus) => Some(PxOp::Add),
                Some(PxTok::Minus) => Some(PxOp::Sub),
                _ => None,
            };
            match op {
                Some(op) => {
                    self.pos += 1;
                    let rhs = self.parse_mul()?;
                    lhs = PxExpr::Binary {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }
                None => {
                    return Ok(lhs);
                }
            }
        }
    }

    fn parse_mul(&mut self) -> Result<PxExpr, String> {
        let mut lhs = self.parse_concat()?;
        loop {
            let op = match self.toks.get(self.pos) {
                Some(PxTok::Star) => Some(PxOp::Mul),
                Some(PxTok::Slash) => Some(PxOp::Div),
                _ => None,
            };
            match op {
                Some(op) => {
                    self.pos += 1;
                    let rhs = self.parse_concat()?;
                    lhs = PxExpr::Binary {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }
                None => {
                    return Ok(lhs);
                }
            }
        }
    }

    fn parse_concat(&mut self) -> Result<PxExpr, String> {
        let lhs = self.parse_hasattr()?;
        if self.peek_is(PxTok::PlusPlus) {
            self.pos += 1;
            let rhs = self.parse_concat()?;
            return Ok(PxExpr::Binary {
                op: PxOp::Concat,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });
        }
        Ok(lhs)
    }

    /// Nix has-attr: `e ? attrpath` (prec 4). The path segments are names
    /// (ident / plain string / dynamic `${e}`); true iff the WHOLE path
    /// exists, false when any step is missing or a non-attrset
    /// (oracle-pinned 2026-07-09: `{a=1;} ? a.b` == false). Multi-segment
    /// paths desugar to a `:hp<k>`-marked guard chain (single eval/step).
    fn parse_hasattr(&mut self) -> Result<PxExpr, String> {
        let mut lhs = self.parse_neg()?;
        while self.peek_is(PxTok::Question) {
            self.pos += 1;
            let mut segs: Vec<PxExpr> = Vec::new();
            loop {
                let seg = match self.toks.get(self.pos).cloned() {
                    Some(PxTok::Ident(n)) => {
                        self.pos += 1;
                        PxExpr::Str(vec![PxStrPart::Lit(n)])
                    }
                    Some(PxTok::Str(parts)) => {
                        // literal AND interpolated string segments —
                        // `? "a"` / `? "${e}"` are both Nix core.
                        self.pos += 1;
                        let mut out = Vec::new();
                        for (is_sub, text) in &parts {
                            if *is_sub {
                                out.push(PxStrPart::Sub(px_parse(text)?));
                            } else if !text.is_empty() {
                                out.push(PxStrPart::Lit(text.clone()));
                            }
                        }
                        PxExpr::Str(out)
                    }
                    Some(PxTok::DollarBrace) => {
                        self.pos += 1;
                        let e = self.parse_expr()?;
                        self.eat(PxTok::RBrace)?;
                        e
                    }
                    other => {
                        return Err(format!(
                            "px: `?` expects an attribute name, found {:?}",
                            other
                        ))
                    }
                };
                segs.push(seg);
                if self.peek_is(PxTok::Dot) {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            lhs = build_hasattr_path(lhs, &segs, 0);
        }
        Ok(lhs)
    }

    /// Nix unary minus (prec 3 — looser than application: `f -1` is a
    /// subtraction, `(x: x) -1` errors on both sides; `[ -1 ]` stays a
    /// syntax error like Nix). Desugars to `0 - operand`.
    fn parse_neg(&mut self) -> Result<PxExpr, String> {
        if self.peek_is(PxTok::Minus) {
            self.pos += 1;
            // `-9223372036854775808` is i64::MIN -- its unsigned magnitude
            // alone never fits a positive i64, and desugaring through
            // `0 - operand` would itself overflow, so fold sign+magnitude
            // into one literal here instead.
            if self.peek_is(PxTok::IntMinMagnitude) {
                self.pos += 1;
                // Written as `-9223372036854775807 - 1` rather than the
                // literal magnitude itself: rs-meta's own Rust-subset lexer
                // has the identical "unsigned literal one past i64::MAX"
                // limitation this fix works around at the pnix layer, so
                // this file must stay inside rs-meta's interpretable subset.
                return Ok(PxExpr::Int(-9223372036854775807i64 - 1));
            }
            let operand = self.parse_neg()?;
            return Ok(PxExpr::Binary {
                op: PxOp::Sub,
                lhs: Box::new(PxExpr::Int(0)),
                rhs: Box::new(operand),
            });
        }
        self.parse_apply()
    }

    /// Application by juxtaposition, left-associative: `f x y` = `(f x) y`.
    fn parse_apply(&mut self) -> Result<PxExpr, String> {
        let mut expr = self.parse_select()?;
        while self.at_atom_start() {
            let arg = self.parse_select()?;
            expr = PxExpr::Apply {
                func: Box::new(expr),
                arg: Box::new(arg),
            };
        }
        Ok(expr)
    }

    /// Attrset selection chain: `base.name.name2` (binds tighter than
    /// application, like Nix).
    fn parse_select(&mut self) -> Result<PxExpr, String> {
        let base = self.parse_atom()?;
        let mut names: Vec<String> = Vec::new();
        let mut segs: Vec<SelectSeg> = Vec::new();
        while self.peek_is(PxTok::Dot) {
            self.pos += 1;
            if self.peek_is(PxTok::DollarBrace) {
                // Nix dynamic select `.${e}` — desugared (no new AST variant):
                // plain -> builtins.getAttr e base (errors on missing, like
                // Nix); with `or` -> hasAttr-guarded via :dsel/:dname temps.
                self.pos += 1;
                let name_expr = self.parse_expr()?;
                self.eat(PxTok::RBrace)?;
                segs.push(SelectSeg::Dynamic(name_expr));
            } else if let Some(PxTok::Str(parts)) = self.toks.get(self.pos) {
                // Nix quoted attr select `x."then"` (plain literal only) —
                // needed by the lowering half reading mk_if's "then"/"else".
                if parts.len() == 1 && !parts[0].0 {
                    let n = parts[0].1.clone();
                    self.pos += 1;
                    names.push(n.clone());
                    segs.push(SelectSeg::Static(n));
                } else {
                    // interpolated select key `e."${k}"` / `e."a${k}"` —
                    // Nix core; lower to a dynamic segment.
                    self.pos += 1;
                    let mut out = Vec::new();
                    for (is_sub, text) in parts.iter() {
                        if *is_sub {
                            out.push(PxStrPart::Sub(px_parse(text)?));
                        } else if !text.is_empty() {
                            out.push(PxStrPart::Lit(text.clone()));
                        }
                    }
                    names.push(String::from("${...}"));
                    segs.push(SelectSeg::Dynamic(PxExpr::Str(out)));
                }
            } else if self.peek_is(PxTok::Null) {
                self.pos += 1;
                names.push(String::from("null"));
                segs.push(SelectSeg::Static(String::from("null")));
            } else if self.peek_is(PxTok::True) {
                self.pos += 1;
                names.push(String::from("true"));
                segs.push(SelectSeg::Static(String::from("true")));
            } else if self.peek_is(PxTok::False) {
                self.pos += 1;
                names.push(String::from("false"));
                segs.push(SelectSeg::Static(String::from("false")));
            } else if self.peek_is(PxTok::KwAssert) {
                self.pos += 1;
                names.push(String::from("assert"));
                segs.push(SelectSeg::Static(String::from("assert")));
            } else {
                let n = self.ident()?;
                names.push(n.clone());
                segs.push(SelectSeg::Static(n));
            }
        }
        if segs.is_empty() {
            return Ok(base);
        }
        if segs.iter().any(|g| matches!(g, SelectSeg::Dynamic(_))) {
            if let Some(PxTok::Ident(o)) = self.toks.get(self.pos) {
                if o == "or" {
                    self.pos += 1;
                    let default = self.parse_select()?;
                    return Ok(build_select_or_segs(base, &segs, &default, 0));
                }
            }
            let mut expr = base;
            for seg in segs {
                expr = match seg {
                    SelectSeg::Static(n) => PxExpr::Select { base: Box::new(expr), name: n },
                    SelectSeg::Dynamic(e) => getattr_apply(e, expr),
                };
            }
            return Ok(expr);
        }
        // Nix `e.attrpath or default` — the default fires when ANY step of
        // the path is missing OR a non-attrset (oracle-pinned 2026-07-08:
        // `{}.a.b or 2` == 2, `(1).b or 2` == 2), and it binds TIGHTER than
        // application (`f x.a or y` == `f (x.a or y)`). Desugared to a chain
        // of `:or<k>`-marked lets (single eval per step) + `?` guards, whose
        // false-on-non-attrs already matches the non-set rule.
        if let Some(PxTok::Ident(o)) = self.toks.get(self.pos) {
            if o == "or" {
                self.pos += 1;
                let default = self.parse_select()?;
                return Ok(build_select_or(base, &names, &default, 0));
            }
        }
        let mut expr = base;
        for name in names {
            expr = PxExpr::Select {
                base: Box::new(expr),
                name,
            };
        }
        Ok(expr)
    }

    fn at_atom_start(&self) -> bool {
        match self.toks.get(self.pos) {
            Some(PxTok::Int(_)) => true,
            Some(PxTok::Float(_)) => true,
            Some(PxTok::Str(_)) => true,
            Some(PxTok::Uri(_)) => true,
            Some(PxTok::Ident(_)) => {
                // `name = ...` starts the next binding, not an application arg.
                !matches!(self.toks.get(self.pos + 1), Some(PxTok::Assign))
            }
            Some(PxTok::True) => true,
            Some(PxTok::False) => true,
            Some(PxTok::Null) => true,
            Some(PxTok::PathLit(_)) => true,
            Some(PxTok::LParen) => true,
            Some(PxTok::LBrace) => true,
            Some(PxTok::LBracket) => true,
            _ => false,
        }
    }

    fn parse_atom(&mut self) -> Result<PxExpr, String> {
        let tok = self.toks.get(self.pos).cloned();
        match tok {
            Some(PxTok::Int(n)) => {
                self.pos += 1;
                Ok(PxExpr::Int(n))
            }
            Some(PxTok::Float(f)) => {
                self.pos += 1;
                Ok(PxExpr::Float(f))
            }
            Some(PxTok::True) => {
                self.pos += 1;
                Ok(PxExpr::Bool(true))
            }
            Some(PxTok::False) => {
                self.pos += 1;
                Ok(PxExpr::Bool(false))
            }
            Some(PxTok::Null) => {
                self.pos += 1;
                Ok(PxExpr::Null)
            }
            Some(PxTok::PathLit(p)) => {
                // `:path:`-marked Var; px_expand_imports substitutes it (as
                // an import argument) before eval — any survivor errors there.
                self.pos += 1;
                Ok(PxExpr::Var(format!(":path:{}", p)))
            }
            Some(PxTok::Ident(name)) => {
                self.pos += 1;
                // `rec { a = 1; b = a + 1; }` -- a recursive attrset. Desugars to
                // `let a = 1; b = a + 1; in { a = a; b = b; }`, reusing the LetIn
                // Rec frame (lazy sibling slots, order-independent). Dynamic keys
                // in `rec` stay held (Nix also restricts them).
                if name == "rec" && self.peek_is(PxTok::LBrace) {
                    let inner = self.parse_attrset_literal(true)?;
                    return match inner {
                        PxExpr::Attrs(fields) => {
                            // `rec { inherit x; }`: x binds in the OUTER scope
                            // (Nix), not the rec frame — capture the outer
                            // value under the `:`-marked name BEFORE the
                            // recursive let, then bind the field to it.
                            let mut outer: Vec<(String, PxExpr)> = Vec::new();
                            let mut bindings: Vec<(String, PxExpr)> = Vec::new();
                            let mut body_fields: Vec<(String, PxExpr)> = Vec::new();
                            for (k, v) in fields {
                                body_fields.push((k.clone(), PxExpr::Var(k.clone())));
                                match v {
                                    PxExpr::Var(n) if n.starts_with(":") => {
                                        let real = strip_inherit_mark(&n);
                                        outer.push((n.clone(), PxExpr::Var(real)));
                                        bindings.push((k, PxExpr::Var(n)));
                                    }
                                    other => bindings.push((k, other)),
                                }
                            }
                            let rec_let = PxExpr::LetIn {
                                bindings,
                                body: Box::new(PxExpr::Attrs(body_fields)),
                            };
                            Ok(if outer.is_empty() {
                                rec_let
                            } else {
                                PxExpr::LetIn {
                                    bindings: outer,
                                    body: Box::new(rec_let),
                                }
                            })
                        }
                        _ => Err(String::from(
                            "px parse: rec with dynamic keys is held",
                        )),
                    };
                }
                Ok(PxExpr::Var(name))
            }
            Some(PxTok::Str(parts)) => {
                self.pos += 1;
                let mut out = Vec::new();
                for (is_sub, text) in &parts {
                    if *is_sub {
                        let sub = px_parse(text)?;
                        out.push(PxStrPart::Sub(sub));
                    } else if !text.is_empty() {
                        out.push(PxStrPart::Lit(text.clone()));
                    }
                }
                Ok(PxExpr::Str(out))
            }
            Some(PxTok::Uri(uri)) => {
                self.pos += 1;
                Ok(PxExpr::Str(vec![PxStrPart::Lit(uri)]))
            }
            Some(PxTok::LParen) => {
                self.pos += 1;
                let inner = self.parse_expr()?;
                self.eat(PxTok::RParen)?;
                Ok(inner)
            }
            Some(PxTok::LBracket) => {
                self.pos += 1;
                let mut items = Vec::new();
                while !self.peek_is(PxTok::RBracket) {
                    // List elements are select-level (no bare application),
                    // matching Nix: `[ f x ]` is two elements.
                    items.push(self.parse_select()?);
                }
                self.eat(PxTok::RBracket)?;
                Ok(PxExpr::List(items))
            }
            Some(PxTok::LBrace) => self.parse_attrset_literal(false),
            _ => Err(format!("px parse: unexpected token {}", self.cur_desc())),
        }
    }

    /// Parse ONE segment of a dotted attrset-binding path (`k1.k2...kn = v`,
    /// used inside `parse_attrset_literal`). A segment is a plain ident /
    /// keyword-name (static) or a possibly-interpolated string / bare
    /// `${ expr }` (dynamic) -- exactly the key vocabulary this file's key
    /// match already accepted for a lone (non-dotted) key. D21: reusing it
    /// for EVERY segment of a dotted path (not just a solitary first/only
    /// key) is what lets a dynamic segment appear anywhere in the path
    /// (`a.${x}.c = 1;`), ported from pnix-clj's `parse-attr-path`/`seg`.
    /// Returns `(parts, is_dynamic)`; `is_dynamic` mirrors this file's
    /// existing `any_dynamic` convention -- true for ANY quoted-string or
    /// `${...}` segment, even a non-interpolated string (matches the
    /// pre-existing single-key behavior, unchanged by this feature).
    fn parse_attr_path_segment(&mut self) -> Result<(Vec<PxStrPart>, bool), String> {
        match self.toks.get(self.pos).cloned() {
            Some(PxTok::Ident(name)) => {
                self.pos += 1;
                Ok((vec![PxStrPart::Lit(name)], false))
            }
            Some(PxTok::Str(parts)) => {
                self.pos += 1;
                let mut out: Vec<PxStrPart> = Vec::new();
                for (is_sub, text) in &parts {
                    if *is_sub {
                        out.push(PxStrPart::Sub(px_parse(text)?));
                    } else if !text.is_empty() {
                        out.push(PxStrPart::Lit(text.clone()));
                    }
                }
                Ok((out, true))
            }
            Some(PxTok::Null) => {
                self.pos += 1;
                Ok((vec![PxStrPart::Lit(String::from("null"))], false))
            }
            Some(PxTok::True) => {
                self.pos += 1;
                Ok((vec![PxStrPart::Lit(String::from("true"))], false))
            }
            Some(PxTok::False) => {
                self.pos += 1;
                Ok((vec![PxStrPart::Lit(String::from("false"))], false))
            }
            Some(PxTok::KwAssert) => {
                self.pos += 1;
                Ok((vec![PxStrPart::Lit(String::from("assert"))], false))
            }
            Some(PxTok::DollarBrace) => {
                self.pos += 1;
                let e = self.parse_expr()?;
                self.eat(PxTok::RBrace)?;
                Ok((vec![PxStrPart::Sub(e)], true))
            }
            _ => Err(format!(
                "px parse: expected attrset key, found {}",
                self.cur_desc()
            )),
        }
    }

    /// Does the token at `idx` start a valid attr-path segment? Used to
    /// decide whether a `.` continues a dotted attrpath (and should be
    /// consumed) or belongs to something else entirely.
    fn token_starts_attr_path_segment(&self, idx: usize) -> bool {
        match self.toks.get(idx) {
            Some(PxTok::Ident(_)) => true,
            Some(PxTok::Str(_)) => true,
            Some(PxTok::Null) => true,
            Some(PxTok::True) => true,
            Some(PxTok::False) => true,
            Some(PxTok::KwAssert) => true,
            Some(PxTok::DollarBrace) => true,
            _ => false,
        }
    }

    /// Attrset literal `{ ... }` (current token must be `{`). With
    /// `keep_inherit_marks`, inherit-clause fields keep their `:`-marked
    /// Var values so the `rec` desugar can capture them from the OUTER scope
    /// (Nix: `rec { inherit x; }` binds x outside the rec frame); otherwise
    /// the marks are stripped (plain attrs create no scope).
    fn parse_attrset_literal(&mut self, keep_inherit_marks: bool) -> Result<PxExpr, String> {
        {
            {
                self.pos += 1;
                // Keys are plain identifiers (static) or string literals with
                // interpolation (dynamic, Nix `{ "k${e}" = v; }`). A literal
                // with any dynamic key desugars WHOLE to
                // `builtins.listToAttrs [ { name = <key>; value = <v>; } .. ]`
                // (m3b machinery; runtime first-wins on duplicates — Nix
                // eval-errors instead, recorded divergence).
                let mut entries: Vec<(Vec<PxStrPart>, PxExpr, usize, bool)> = Vec::new();
                let mut any_dynamic = false;
                while !self.peek_is(PxTok::RBrace) {
                    // `inherit a b c;` — the plain Nix inherit clause. Each
                    // name becomes a field whose value is a `:`-marked Var
                    // (px identifiers cannot contain `:`, so it is collision-free); consumers strip or capture.
                    // `inherit (expr) ...` is held; `inherit = v;` stays a
                    // normal key (un-writable in real Nix anyway).
                    if let Some(PxTok::Ident(kw)) = self.toks.get(self.pos) {
                        if kw == "inherit"
                            && !matches!(self.toks.get(self.pos + 1), Some(PxTok::Assign))
                        {
                            self.pos += 1;
                            // `inherit (E) a b;` — E is evaluated in the
                            // ENCLOSING binding scope (in rec: the rec scope,
                            // siblings visible — oracle-pinned), so each name
                            // is an ordinary field (n, Select(E, n)); no
                            // outer-capture marks needed.
                            let mut from: Option<PxExpr> = None;
                            if self.peek_is(PxTok::LParen) {
                                self.pos += 1;
                                let e = self.parse_expr()?;
                                self.eat(PxTok::RParen)?;
                                from = Some(e);
                            }
                            let mut got_any = false;
                            while !self.peek_is(PxTok::Semi) {
                                match self.toks.get(self.pos).cloned() {
                                    Some(PxTok::Ident(n)) => {
                                        self.pos += 1;
                                        got_any = true;
                                        let value = match &from {
                                            Some(e) => PxExpr::Select {
                                                base: Box::new(e.clone()),
                                                name: n.clone(),
                                            },
                                            None => PxExpr::Var(format!(":{}", n)),
                                        };
                                        entries.push((
                                            vec![PxStrPart::Lit(n.clone())],
                                            value,
                                            0,
                                            true,
                                        ));
                                    }
                                    _ => {
                                        return Err(format!(
                                            "px parse: inherit expects attribute names, found {}",
                                            self.cur_desc()
                                        ))
                                    }
                                }
                            }
                            if !got_any {
                                return Err(String::from("px parse: empty inherit"));
                            }
                            self.eat(PxTok::Semi)?;
                            continue;
                        }
                    }
                    // Dotted attrpath `k1.k2...kn = v` desugars to nested
                    // single-entry attrsets (Nix attrpath assignment; a
                    // static-first-segment prefix later re-merges siblings
                    // via merge_attr_field). D21: EVERY segment -- first or
                    // trailing -- may independently be static or dynamic
                    // (`a.${x}.c = 1;`, not just a lone top-level dynamic
                    // key), ported from pnix-clj's parse-attr-path/seg.
                    let first_off = self.cur_off();
                    let (first_parts, first_dynamic) = self.parse_attr_path_segment()?;
                    if first_dynamic {
                        any_dynamic = true;
                    }
                    let mut seg_parts_list: Vec<Vec<PxStrPart>> = Vec::new();
                    let mut seg_dynamic_list: Vec<bool> = Vec::new();
                    let mut seg_offs: Vec<usize> = Vec::new();
                    seg_parts_list.push(first_parts.clone());
                    seg_dynamic_list.push(first_dynamic);
                    seg_offs.push(first_off);
                    while self.peek_is(PxTok::Dot)
                        && self.token_starts_attr_path_segment(self.pos + 1)
                    {
                        self.pos += 1; // Dot
                        let seg_off = self.cur_off();
                        let (seg_parts, seg_dynamic) = self.parse_attr_path_segment()?;
                        seg_parts_list.push(seg_parts);
                        seg_dynamic_list.push(seg_dynamic);
                        seg_offs.push(seg_off);
                    }
                    let key_parts: Vec<PxStrPart> = first_parts;
                    self.eat(PxTok::Assign)?;
                    let value0 = self.parse_expr()?;
                    self.eat(PxTok::Semi)?;
                    // Wrap trailing segments foo.bar = v as foo = { bar = v; }
                    // (static segment) or foo = builtins.listToAttrs
                    // [ { name = bar; value = v; } ] (dynamic segment,
                    // D21: px_wrap_dynamic_attr reuses the same desugar a
                    // lone dynamic key already gets, applied at this
                    // nested position instead of the top level).
                    let mut value = value0;
                    let mut i = seg_parts_list.len();
                    while i > 1 {
                        i -= 1;
                        let seg_parts = seg_parts_list[i].clone();
                        let seg_dynamic = seg_dynamic_list[i];
                        if seg_dynamic {
                            value = px_wrap_dynamic_attr(seg_parts, value);
                        } else {
                            let name = match seg_parts.first() {
                                Some(PxStrPart::Lit(s)) => s.clone(),
                                _ => return Err(String::from("px parse: empty attrset key")),
                            };
                            let pos = self.position_expr(seg_offs[i]);
                            value = px_expr_attrs_with_pos(
                                vec![(name.clone(), value)],
                                vec![(name, pos)],
                            );
                        }
                    }
                    entries.push((key_parts, value, first_off, first_dynamic));
                }
                self.eat(PxTok::RBrace)?;
                if !any_dynamic {
                    let mut fields: Vec<(String, PxExpr)> = Vec::new();
                    let mut top_pos: Vec<(String, PxExpr)> = Vec::new();
                    for (key_parts, value, first_off, first_dynamic) in entries {
                        let name = match key_parts.first() {
                            Some(PxStrPart::Lit(s)) => s.clone(),
                            _ => return Err(String::from("px parse: empty attrset key")),
                        };
                        if !first_dynamic {
                            let mut already = false;
                            for (k, _) in top_pos.iter() {
                                if *k == name {
                                    already = true;
                                }
                            }
                            if !already {
                                let pos = self.position_expr(first_off);
                                top_pos.push((name.clone(), pos));
                            }
                        }
                        fields = merge_attr_field(fields, name, value)?;
                    }
                    if !keep_inherit_marks {
                        fields = fields
                            .into_iter()
                            .map(|(k, v)| match v {
                                PxExpr::Var(n) if n.starts_with(":") => {
                                    (k, PxExpr::Var(strip_inherit_mark(&n)))
                                }
                                other => (k, other),
                            })
                            .collect();
                    }
                    return Ok(px_expr_attrs_with_pos(fields, top_pos));
                }
                let mut pairs = Vec::new();
                for (key_parts, value, _first_off, _first_dynamic) in entries {
                    let value = match value {
                        PxExpr::Var(n) if n.starts_with(":") => {
                            PxExpr::Var(strip_inherit_mark(&n))
                        }
                        other => other,
                    };
                    pairs.push(PxExpr::Attrs(vec![
                        (String::from("name"), PxExpr::Str(key_parts)),
                        (String::from("value"), value),
                    ]));
                }
                Ok(PxExpr::Apply {
                    func: Box::new(PxExpr::Select {
                        base: Box::new(PxExpr::Var(String::from("builtins"))),
                        name: String::from("listToAttrs"),
                    }),
                    arg: Box::new(PxExpr::List(pairs)),
                })
            }
        }
    }
}


/// Numeric coercion for the arithmetic builtins: int or float -> f64.
fn px_num_f64(v: &PxVal) -> Result<f64, String> {
    match v {
        PxVal::Int(n) => Ok(*n as f64),
        PxVal::Float(f) => Ok(*f),
        other => Err(format!("px: expected a number, got {}", px_kind(other))),
    }
}

// ---- pure-arithmetic transcendental math (2026-08-20) ---------------------
//
// rs-meta's interpreted Rust subset (exercised by `substrate-check`, which
// feeds this entire file through ../rs-meta's bootstrap interpreter) has NO
// method dispatch for `f64`: `call_method` in rs-meta/src/interp.rs only
// recognizes `i64`/`i32`/`u32`/`u64`/`u8`/`usize` as numeric method-call
// targets, so `.sin()`/`.sqrt()`/`.exp()`/`.ln()`/`.powf()`/... are all
// "unknown method". The transcendental math builtins below are therefore
// hand-rolled from the same primitive vocabulary this file already uses
// elsewhere for the same reason (`px_bit_op`'s bit-by-bit AND/OR/XOR,
// `px_round_to_int`'s cast-and-adjust ceil/floor): `+ - * /`, comparisons,
// `as` casts, and `while`. They target full f64 precision (~1e-15 relative
// error) for finite, non-extreme inputs; `sin`/`cos`/`tan` on
// extreme-magnitude arguments trade some precision the way any single
// double-precision range reduction against `2*pi` does — a Payne-Hanek-grade
// reduction was judged not worth the complexity for a config-language
// builtin.

/// `2^k` for any i64 `k`, via exponentiation by squaring. The f64
/// accumulator overflows to `inf` / underflows to `0.0` exactly the way a
/// real power-of-two would.
fn px_pow2_i64(k: i64) -> f64 {
    if k == 0 {
        return 1.0;
    }
    let neg = k < 0;
    let mut n = if neg { 0 - k } else { k };
    let mut result = 1.0;
    let mut base = 2.0;
    while n > 0 {
        if n % 2 == 1 {
            result = result * base;
        }
        base = base * base;
        n = n / 2;
    }
    if neg { 1.0 / result } else { result }
}

/// Round-half-away-from-zero, f64 -> i64 (used for exp/sin/cos range
/// reduction). Plain `as i64` truncates toward zero, so a manual +/-0.5
/// shift is required.
fn px_round_f64_to_i64(q: f64) -> i64 {
    if q >= 0.0 {
        (q + 0.5) as i64
    } else {
        (q - 0.5) as i64
    }
}

/// `sqrt` via Newton's method (`g' = (g + x/g) / 2`). Quadratically
/// convergent, so a fixed iteration count reaches full f64 precision from
/// any starting guess in range with room to spare.
fn px_math_sqrt(x: f64) -> f64 {
    if x != x {
        return x; // NaN in, NaN out
    }
    if x < 0.0 {
        return 0.0 / 0.0; // NaN: sqrt of a negative number
    }
    if x == 0.0 {
        return x; // preserves the sign of zero, like libm
    }
    if x - x != 0.0 {
        return x; // +infinity
    }
    let mut g = if x > 1.0 { x } else { 1.0 };
    let mut i = 0;
    while i < 60 {
        g = 0.5 * (g + x / g);
        i += 1;
    }
    g
}

/// `exp` via range reduction to `x = r + k*ln2` (so `exp(x) = exp(r) * 2^k`)
/// followed by a Taylor series for `exp(r)` on the small remainder.
fn px_math_exp(x: f64) -> f64 {
    if x != x {
        return x;
    }
    if x - x != 0.0 {
        if x > 0.0 {
            return x; // +infinity
        }
        return 0.0; // -infinity
    }
    let ln2 = 0.6931471805599453;
    let k = px_round_f64_to_i64(x / ln2);
    let r = x - (k as f64) * ln2;
    let mut term = 1.0;
    let mut sum = 1.0;
    let mut n = 1.0;
    let mut i = 0;
    while i < 25 {
        term = term * r / n;
        sum = sum + term;
        n = n + 1.0;
        i += 1;
    }
    sum * px_pow2_i64(k)
}

/// `ln` via binary range reduction into `[1, 2)` plus the fast-converging
/// `ln(m) = 2*atanh((m-1)/(m+1))` series (`(m-1)/(m+1)` is at most `1/3` on
/// that range). Domain-checked like the pnix-hy reference's `math.log`.
fn px_math_ln(x: f64) -> Result<f64, String> {
    if x != x {
        return Ok(x);
    }
    if x <= 0.0 {
        return Err(String::from("px: ln: argument must be positive"));
    }
    if x - x != 0.0 {
        return Ok(x); // +infinity
    }
    let ln2 = 0.6931471805599453;
    let mut m = x;
    let mut k: i64 = 0;
    while m >= 2.0 {
        m = m / 2.0;
        k += 1;
    }
    while m < 1.0 {
        m = m * 2.0;
        k -= 1;
    }
    let z = (m - 1.0) / (m + 1.0);
    let z2 = z * z;
    let mut term = z;
    let mut sum = z;
    let mut i = 1;
    while i < 30 {
        term = term * z2;
        let denom = ((2 * i) + 1) as f64;
        sum = sum + term / denom;
        i += 1;
    }
    Ok((2.0 * sum) + ((k as f64) * ln2))
}

/// `sin`/`cos` share range reduction into a small remainder against `2*pi`,
/// then a direct Taylor series (fast-converging at this magnitude: `pi`
/// raised to the ~50th power is still dwarfed by `50!`).
fn px_trig_reduce(x: f64) -> f64 {
    let two_pi = 6.283185307179586;
    let k = px_round_f64_to_i64(x / two_pi);
    x - ((k as f64) * two_pi)
}

fn px_math_sin(x: f64) -> f64 {
    if x != x || x - x != 0.0 {
        return 0.0 / 0.0; // NaN: undefined angle (NaN or +/-infinity input)
    }
    let r = px_trig_reduce(x);
    let r2 = r * r;
    let mut term = r;
    let mut sum = r;
    let mut k: i64 = 0;
    while k < 25 {
        let denom = (((2 * k) + 2) * ((2 * k) + 3)) as f64;
        term = (term * (0.0 - r2)) / denom;
        sum = sum + term;
        k += 1;
    }
    sum
}

fn px_math_cos(x: f64) -> f64 {
    if x != x || x - x != 0.0 {
        return 0.0 / 0.0;
    }
    let r = px_trig_reduce(x);
    let r2 = r * r;
    let mut term = 1.0;
    let mut sum = 1.0;
    let mut k: i64 = 0;
    while k < 25 {
        let denom = (((2 * k) + 1) * ((2 * k) + 2)) as f64;
        term = (term * (0.0 - r2)) / denom;
        sum = sum + term;
        k += 1;
    }
    sum
}

fn px_math_tan(x: f64) -> f64 {
    px_math_sin(x) / px_math_cos(x)
}

/// `atan` via reciprocal + half-angle reduction (`atan(x) = 2*atan(x /
/// (1 + sqrt(1+x^2)))`, applied repeatedly) into a range where the Taylor
/// series converges quickly; `atan2` layers quadrant selection on top.
fn px_math_atan(x: f64) -> f64 {
    if x != x {
        return x;
    }
    let pi_half = 1.5707963267948966;
    if x - x != 0.0 {
        if x > 0.0 {
            return pi_half;
        }
        return 0.0 - pi_half;
    }
    let neg = x < 0.0;
    let mut a = if neg { 0.0 - x } else { x };
    let reciprocal = a > 1.0;
    if reciprocal {
        a = 1.0 / a;
    }
    let mut i = 0;
    while i < 4 {
        a = a / (1.0 + px_math_sqrt(1.0 + (a * a)));
        i += 1;
    }
    let a2 = a * a;
    let mut term = a;
    let mut sum = a;
    let mut k: i64 = 0;
    while k < 30 {
        term = term * (0.0 - a2);
        let denom = ((2 * k) + 3) as f64;
        sum = sum + (term / denom);
        k += 1;
    }
    let mut j = 0;
    while j < 4 {
        sum = 2.0 * sum;
        j += 1;
    }
    let result = if reciprocal { pi_half - sum } else { sum };
    if neg { 0.0 - result } else { result }
}

/// `atan2(y, x)`: standard quadrant selection around `atan(y/x)`.
fn px_math_atan2(y: f64, x: f64) -> f64 {
    if y != y || x != x {
        return 0.0 / 0.0;
    }
    let pi = 3.141592653589793;
    let pi_half = 1.5707963267948966;
    if x > 0.0 {
        return px_math_atan(y / x);
    }
    if x < 0.0 {
        if y < 0.0 {
            return px_math_atan(y / x) - pi;
        }
        return px_math_atan(y / x) + pi;
    }
    if y > 0.0 {
        return pi_half;
    }
    if y < 0.0 {
        return 0.0 - pi_half;
    }
    0.0
}

/// `mod`'s float branch: truncated remainder (`fmod`), matching the
/// pnix-hy reference's `math.fmod`. Truncates the quotient toward zero via
/// the same `as i64` cast `px_round_to_int` already relies on for ceil/floor.
fn px_fmod_f64(a: f64, b: f64) -> f64 {
    if a != a || b != b || b == 0.0 {
        return 0.0 / 0.0;
    }
    if a - a != 0.0 {
        return 0.0 / 0.0; // infinite dividend: undefined
    }
    if b - b != 0.0 {
        return a; // finite dividend mod an infinite divisor is the dividend
    }
    let q = a / b;
    let qi = q as i64;
    let qt = qi as f64;
    a - (qt * b)
}

/// `abs`'s float branch (no `.abs()` method dispatch in rs-meta's
/// interpreted subset — see the block comment above `px_pow2_i64`).
fn px_abs_f64(f: f64) -> f64 {
    if f != f {
        return f;
    }
    if f == 0.0 {
        return 0.0; // normalizes -0.0 to 0.0, matching libm fabs
    }
    if f < 0.0 { 0.0 - f } else { f }
}

/// Checked integer exponentiation by squaring (`base^exp`, `exp >= 0`),
/// erroring the same way `px_int_arith` does on i64 overflow.
fn px_checked_ipow(base: i64, exp: i64) -> Result<i64, String> {
    let mut result: i64 = 1;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e % 2 == 1 {
            match result.checked_mul(b) {
                Some(v) => result = v,
                None => return Err(format!("px: integer overflow in pow {} {}", base, exp)),
            }
        }
        e = e / 2;
        if e > 0 {
            match b.checked_mul(b) {
                Some(v) => b = v,
                None => return Err(format!("px: integer overflow in pow {} {}", base, exp)),
            }
        }
    }
    Ok(result)
}

/// `pow`'s float branch: `base^exp = exp(exp * ln(base))` for `base > 0`,
/// with the same negative/zero-base special cases real `pow` implementations
/// carry (integer exponent on a negative base, zero base, zero exponent).
fn px_powf(base: f64, exp: f64) -> Result<f64, String> {
    if exp == 0.0 {
        return Ok(1.0);
    }
    if base == 0.0 {
        if exp > 0.0 {
            return Ok(0.0);
        }
        return Ok(1.0 / 0.0);
    }
    if base < 0.0 {
        let exp_i = exp as i64;
        if (exp_i as f64) == exp {
            let mag = px_math_exp(exp * px_math_ln(0.0 - base)?);
            if exp_i % 2 != 0 {
                Ok(0.0 - mag)
            } else {
                Ok(mag)
            }
        } else {
            Ok(0.0 / 0.0)
        }
    } else {
        let l = px_math_ln(base)?;
        Ok(px_math_exp(exp * l))
    }
}

fn px_int_arith_outcome(name: &str, a: i64, b: i64) -> Result<i64, PxError> {
    if name == "div" && b == 0 {
        return Err(px_error_eval(
            PxErrorClass::DivisionByZero,
            String::from("px: division by zero"),
        ));
    }
    let out = if name == "add" {
        a.checked_add(b)
    } else if name == "sub" {
        a.checked_sub(b)
    } else if name == "mul" {
        a.checked_mul(b)
    } else {
        a.checked_div(b)
    };
    match out {
        Some(n) => Ok(n),
        None => Err(px_error_eval(
            PxErrorClass::IntegerOverflow,
            format!("px: integer overflow in {} {} {}", name, a, b),
        )),
    }
}

fn px_int_arith(name: &str, a: i64, b: i64) -> Result<i64, String> {
    px_int_arith_outcome(name, a, b).map_err(px_error_into_diagnostic)
}

const PX_PRIMITIVE_ABI_VERSION: &str = "pnix.primitive-abi.v1";
const PX_PRIMITIVE_MANIFEST_DIGEST: &str =
    "f133ee0f3a5c6073eabb6855f3abf44bf36366083f26fbe76e9524521a2a5fd6";

pub fn px_checked_i64_kernel(
    abi_version: &str,
    manifest_digest: &str,
    primitive_id: &str,
    operands: &[i64],
) -> Result<i64, &'static str> {
    if abi_version != PX_PRIMITIVE_ABI_VERSION
        || manifest_digest != PX_PRIMITIVE_MANIFEST_DIGEST
        || operands.len() != 2
    {
        return Err("primitive-contract-violation");
    }
    let left = operands[0];
    let right = operands[1];
    let value = if primitive_id == "i64-add-checked" {
        left.checked_add(right)
    } else if primitive_id == "i64-sub-checked" {
        left.checked_sub(right)
    } else if primitive_id == "i64-mul-checked" {
        left.checked_mul(right)
    } else if primitive_id == "i64-div-checked" {
        if right == 0 {
            return Err("division-by-zero");
        }
        left.checked_div(right)
    } else {
        return Err("primitive-contract-violation");
    };
    value.ok_or("integer-overflow")
}

fn px_manifest_int_arith_outcome(name: &str, a: i64, b: i64) -> Result<i64, PxError> {
    let legacy = px_int_arith_outcome(name, a, b);
    let primitive_id = if name == "add" {
        "i64-add-checked"
    } else if name == "sub" {
        "i64-sub-checked"
    } else if name == "mul" {
        "i64-mul-checked"
    } else if name == "div" {
        "i64-div-checked"
    } else {
        return Err(px_error_eval(
            PxErrorClass::PrimitiveContractViolation,
            String::from("px: primitive contract violation"),
        ));
    };
    let routed = px_checked_i64_kernel(
        PX_PRIMITIVE_ABI_VERSION,
        PX_PRIMITIVE_MANIFEST_DIGEST,
        primitive_id,
        &[a, b],
    );
    let agrees = match (&legacy, &routed) {
        (Ok(left), Ok(right)) => left == right,
        (Err(error), Err(class)) => {
            (error.class == PxErrorClass::DivisionByZero && *class == "division-by-zero")
                || (error.class == PxErrorClass::IntegerOverflow
                    && *class == "integer-overflow")
        }
        _ => false,
    };
    if !agrees {
        return Err(px_error_eval(
            PxErrorClass::PrimitiveContractViolation,
            String::from("px: primitive contract violation"),
        ));
    }
    match routed {
        Ok(value) => Ok(value),
        Err("division-by-zero") => Err(px_error_eval(
            PxErrorClass::DivisionByZero,
            String::from("px: division by zero"),
        )),
        Err("integer-overflow") => Err(px_error_eval(
            PxErrorClass::IntegerOverflow,
            format!("px: integer overflow in {} {} {}", name, a, b),
        )),
        Err(_) => Err(px_error_eval(
            PxErrorClass::PrimitiveContractViolation,
            String::from("px: primitive contract violation"),
        )),
    }
}

fn px_float_binary_outcome(op: &PxOp, a: f64, b: f64) -> Result<PxVal, PxError> {
    match op {
        PxOp::Add => Ok(PxVal::Float(a + b)),
        PxOp::Sub => Ok(PxVal::Float(a - b)),
        PxOp::Mul => Ok(PxVal::Float(a * b)),
        PxOp::Div => {
            if b == 0.0 {
                Err(px_error_eval(
                    PxErrorClass::DivisionByZero,
                    String::from("px: division by zero"),
                ))
            } else {
                Ok(PxVal::Float(a / b))
            }
        }
        PxOp::Lt => Ok(PxVal::Bool(a < b)),
        // Nix defines non-strict comparison from strict ordering. This is
        // observable for NaN: both <= and >= are true while < and > are false.
        PxOp::Le => Ok(PxVal::Bool(!(b < a))),
        PxOp::Gt => Ok(PxVal::Bool(b < a)),
        PxOp::Ge => Ok(PxVal::Bool(!(a < b))),
        other => Err(px_error_type(format!("px: op {:?} on floats", other))),
    }
}

fn px_round_to_int(v: &PxVal, upward: bool) -> Result<PxVal, String> {
    let upper = 9223372036854775808.0;
    let lower = -9223372036854775808.0;
    match v {
        PxVal::Int(n) => {
            // Nix 2.34.7 converts through f64 and rejects a lossy conversion
            // (issue #12899), even though mathematically ceil/floor are identity.
            let f = *n as f64;
            if f >= upper || f < lower || (f as i64) != *n {
                return Err(String::from("px: ceil/floor argument loses integer precision"));
            }
            Ok(PxVal::Int(*n))
        }
        PxVal::Float(f) => {
            let x = *f;
            if x - x != 0.0 || x >= upper || x < lower {
                return Err(String::from("px: ceil/floor argument is outside the int range"));
            }
            let mut n = x as i64;
            let exact = (n as f64) == x;
            if upward && x > 0.0 && !exact {
                n = px_int_arith("add", n, 1)?;
            } else if !upward && x < 0.0 && !exact {
                n = px_int_arith("sub", n, 1)?;
            }
            Ok(PxVal::Int(n))
        }
        other => Err(format!(
            "px: ceil/floor expects a number, got {}",
            px_kind(other)
        )),
    }
}

/// Bitwise ops written without `&`/`|`/`^` on i64 — the rs-meta subset has no
/// bit operators, so they are computed from the two's-complement bit pattern
/// one bit at a time (64 iterations, exact for all i64 inputs).
fn px_bit_op(a: i64, b: i64, kind: u8) -> i64 {
    let mut ua = a as u64;
    let mut ub = b as u64;
    let mut out: u64 = 0;
    let mut bit = 0usize;
    while bit < 64 {
        let abit = ua % 2;
        let bbit = ub % 2;
        let r = if kind == 0 {
            if abit == 1 && bbit == 1 { 1 } else { 0 }
        } else if kind == 1 {
            if abit == 1 || bbit == 1 { 1 } else { 0 }
        } else {
            if abit != bbit { 1 } else { 0 }
        };
        if r == 1 {
            out += 1u64 << bit;
        }
        ua /= 2;
        ub /= 2;
        bit += 1;
    }
    out as i64
}

fn px_bit_and(a: i64, b: i64) -> i64 {
    px_bit_op(a, b, 0)
}

fn px_bit_or(a: i64, b: i64) -> i64 {
    px_bit_op(a, b, 1)
}

fn px_bit_xor(a: i64, b: i64) -> i64 {
    px_bit_op(a, b, 2)
}

/// `builtins.splitVersion` (oracle-pinned): components are separated by `.` or
/// `-`, and additionally split at every digit <-> non-digit boundary; empty
/// components are dropped.
fn px_split_version(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_digit = false;
    let mut have = false;
    for c in s.chars() {
        if c == '.' || c == '-' {
            if have && !cur.is_empty() {
                out.push(cur);
                cur = String::new();
            }
            have = false;
            continue;
        }
        let is_digit = c.is_ascii_digit();
        if have && is_digit != cur_digit {
            if !cur.is_empty() {
                out.push(cur);
                cur = String::new();
            }
        }
        cur.push(c);
        cur_digit = is_digit;
        have = true;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Parse a version component as a number, if it is all ASCII digits.
fn px_version_num(c: &str) -> Option<i64> {
    if c.is_empty() {
        return None;
    }
    let mut n: i64 = 0;
    for ch in c.chars() {
        if !ch.is_ascii_digit() {
            return None;
        }
        n = n * 10 + ((ch as i64) - ('0' as i64));
    }
    Some(n)
}

/// Nix `componentsLT`: numbers compare numerically; `""` < a number;
/// `"pre"` sorts before everything (and nothing sorts before `"pre"`);
/// a number sorts before a non-number; otherwise lexicographic.
fn px_component_lt(c1: &str, c2: &str) -> bool {
    let n1 = px_version_num(c1);
    let n2 = px_version_num(c2);
    match (n1, n2) {
        (Some(a), Some(b)) => a < b,
        (None, Some(_)) => {
            if c1.is_empty() {
                true
            } else if c1 == "pre" {
                true
            } else {
                true
            }
        }
        (Some(_), None) => {
            if c2 == "pre" {
                false
            } else {
                false
            }
        }
        (None, None) => {
            if c1 == "pre" && c2 != "pre" {
                true
            } else if c2 == "pre" {
                false
            } else {
                c1 < c2
            }
        }
    }
}

/// `builtins.compareVersions a b` -> -1 | 0 | 1 (component-wise, missing
/// components padded with "").
fn px_compare_versions(a: &str, b: &str) -> i64 {
    let ca = px_split_version(a);
    let cb = px_split_version(b);
    let n = if ca.len() > cb.len() { ca.len() } else { cb.len() };
    let mut i = 0usize;
    while i < n {
        let x: &str = if i < ca.len() { &ca[i] } else { "" };
        let y: &str = if i < cb.len() { &cb[i] } else { "" };
        if px_component_lt(x, y) {
            return -1;
        }
        if px_component_lt(y, x) {
            return 1;
        }
        i += 1;
    }
    0
}

/// Nix parseDrvName: split at the FIRST '-' followed by an ASCII digit.
fn px_parse_drv_name(s: &str) -> (String, String) {
    let chars: Vec<char> = s.chars().collect();
    let mut split: Option<usize> = None;
    let mut i = 0usize;
    while i + 1 < chars.len() {
        if chars[i] == '-' && chars[i + 1].is_ascii_digit() {
            split = Some(i);
            break;
        }
        i += 1;
    }
    match split {
        None => (String::from(s), String::new()),
        Some(at) => {
            let mut name = String::new();
            let mut version = String::new();
            let mut j = 0usize;
            while j < chars.len() {
                if j < at {
                    name.push(chars[j]);
                } else if j > at {
                    version.push(chars[j]);
                }
                j += 1;
            }
            (name, version)
        }
    }
}

/// Nix toPath is lexical normalization of an absolute string. It neither
/// resolves symlinks nor creates a distinct path value.
fn px_to_path_string(s: &str) -> Result<String, String> {
    if !s.starts_with("/") {
        return Err(format!(
            "px: string '{}' doesn't represent an absolute path",
            s
        ));
    }
    let mut parts: Vec<String> = Vec::new();
    for part in s.split("/") {
        if part == "" || part == "." {
        } else if part == ".." {
            parts.pop();
        } else {
            parts.push(String::from(part));
        }
    }
    if parts.is_empty() {
        Ok(String::from("/"))
    } else {
        Ok(format!("/{}", parts.join("/")))
    }
}

/// Is this expression an attrset literal `{ ... }`? Only attrset literals
/// merge on a duplicate key (Nix semantics); anything else is a hard
/// "already defined". `rec { ... }` is a separate variant and is treated as
/// non-mergeable here (rare; matches the conservative side of Nix).
fn px_is_attrs_lit(e: &PxExpr) -> bool {
    match e {
        PxExpr::Attrs(_) => true,
        _ => false,
    }
}

/// Build `builtins.listToAttrs [ <pairs> ]` -- the shape
/// `parse_attrset_literal`'s any-dynamic-key branch already desugars a
/// dynamic-keyed attrset literal to. Shared by that branch and by D21's
/// nested-dynamic-attrpath-segment desugar (`px_wrap_dynamic_attr`) so both
/// produce byte-identical output for the same semantics.
fn px_make_list_to_attrs(pairs: Vec<PxExpr>) -> PxExpr {
    PxExpr::Apply {
        func: Box::new(PxExpr::Select {
            base: Box::new(PxExpr::Var(String::from("builtins"))),
            name: String::from("listToAttrs"),
        }),
        arg: Box::new(PxExpr::List(pairs)),
    }
}

/// Wrap `value` as the sole entry of a single-key dynamic attrset literal
/// keyed by `key_parts` (`builtins.listToAttrs [ { name = <key_parts>;
/// value = <value>; } ]`). D21: this is how a DYNAMIC non-first segment of
/// a dotted attrpath (`a.${x}.c = v` — the `${x}` segment) nests `v`,
/// mirroring exactly how a static non-first segment nests it via a plain
/// `PxExpr::Attrs` literal (ported semantics from pnix-clj's
/// `path->nested`, adapted to this file's listToAttrs-desugar machinery
/// instead of a literal-with-dynamic-key AST node, which this file's
/// `PxExpr::Attrs` cannot represent).
fn px_wrap_dynamic_attr(key_parts: Vec<PxStrPart>, value: PxExpr) -> PxExpr {
    px_make_list_to_attrs(vec![PxExpr::Attrs(vec![
        (String::from("name"), PxExpr::Str(key_parts)),
        (String::from("value"), value),
    ])])
}

/// If `e` is exactly the shape `px_make_list_to_attrs` produces
/// (`builtins.listToAttrs [ .. ]`), return its pair list. Lets
/// `merge_attr_field` recognize a nested-dynamic-segment desugar as
/// mergeable with a sibling instead of hard-erroring "duplicate attrset
/// key" -- D21's trickiest interaction: a STATIC-first-segment sibling
/// (`a.b = 1;`) must still merge with a binding whose value is a
/// dynamic-segment desugar (`a.${x}.c = 2;`) under the same static key `a`,
/// exactly like pnix-clj's merge-attr-bindings (the static prefix merges at
/// parse time; the dynamic segment itself passes through unresolved to
/// runtime, where this file's existing listToAttrs first-wins divergence
/// applies on an actual collision -- see docs/BUGS.md §3).
fn px_dynamic_pairs(e: &PxExpr) -> Option<Vec<PxExpr>> {
    match e {
        PxExpr::Apply { func, arg } => match func.as_ref() {
            PxExpr::Select { base, name } if name == "listToAttrs" => match base.as_ref() {
                PxExpr::Var(v) if v == "builtins" => match arg.as_ref() {
                    PxExpr::List(items) => Some(items.clone()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// Convert a literal attrset's static fields into `{name=..;value=..;}`
/// pairs, the element shape `builtins.listToAttrs` expects. Used by
/// `merge_attr_field`'s D21 fallback to fold a literal-Attrs side into the
/// same pair-list shape as the dynamic side before concatenating.
fn px_attrs_to_dynamic_pairs(fields: &Vec<(String, PxExpr)>) -> Vec<PxExpr> {
    let mut out: Vec<PxExpr> = Vec::new();
    for (k, v) in fields {
        out.push(PxExpr::Attrs(vec![
            (String::from("name"), PxExpr::Str(vec![PxStrPart::Lit(k.clone())])),
            (String::from("value"), v.clone()),
        ]));
    }
    out
}

/// Add `(name, value)` to `fields`, implementing Nix's duplicate-key rule:
/// if the key already exists AND both the existing and the new value are
/// attrset literals, RECURSIVELY MERGE their bindings (so `a = {x=1;}; a =
/// {y=2;};` becomes `a = {x=1; y=2;}`, and a nested key collision recurses
/// down to `attribute 'a.x' already defined`); otherwise the duplicate is a
/// hard error, exactly as `nix-instantiate` reports. rs-meta subset: index
/// loop, no tuple-match, no rest-patterns.
const PX_ATTR_POS_KEY: &str = "__pnix_attr_pos";

fn px_is_attr_pos_key(name: &str) -> bool {
    name == PX_ATTR_POS_KEY
}

fn px_expr_attrs_with_pos(
    fields: Vec<(String, PxExpr)>,
    pos_fields: Vec<(String, PxExpr)>,
) -> PxExpr {
    let mut fields = fields;
    if pos_fields.is_empty() {
        PxExpr::Attrs(fields)
    } else {
        fields.push((String::from(PX_ATTR_POS_KEY), PxExpr::Attrs(pos_fields)));
        PxExpr::Attrs(fields)
    }
}

fn merge_attr_field(
    fields: Vec<(String, PxExpr)>,
    name: String,
    value: PxExpr,
) -> Result<Vec<(String, PxExpr)>, String> {
    let mut out: Vec<(String, PxExpr)> = Vec::new();
    let mut merged_flag = false;
    for (k, v) in fields {
        if k == name && !merged_flag {
            if px_is_attrs_lit(&v) && px_is_attrs_lit(&value) {
                let mut merged = match v {
                    PxExpr::Attrs(x) => x,
                    _ => Vec::new(),
                };
                let newf = match &value {
                    PxExpr::Attrs(x) => x.clone(),
                    _ => Vec::new(),
                };
                for (nk, nv) in newf {
                    merged = merge_attr_field(merged, nk, nv)?;
                }
                out.push((k, PxExpr::Attrs(merged)));
                merged_flag = true;
            } else {
                // D21: not a literal<->literal merge, but one (or both)
                // sides may still be a nested-dynamic-attrpath-segment
                // desugar (builtins.listToAttrs [...]) -- concatenate pair
                // lists instead of hard-erroring (earlier-defined side's
                // pairs first, so a real collision resolves via this
                // file's existing first-wins divergence, docs/BUGS.md §3).
                let v_pairs = match &v {
                    PxExpr::Attrs(x) => Some(px_attrs_to_dynamic_pairs(x)),
                    _ => px_dynamic_pairs(&v),
                };
                let value_pairs = match &value {
                    PxExpr::Attrs(x) => Some(px_attrs_to_dynamic_pairs(x)),
                    _ => px_dynamic_pairs(&value),
                };
                match (v_pairs, value_pairs) {
                    (Some(vp0), Some(np)) => {
                        let mut vp = vp0;
                        for p in np {
                            vp.push(p);
                        }
                        out.push((k, px_make_list_to_attrs(vp)));
                        merged_flag = true;
                    }
                    _ => {
                        return Err(format!("px parse: duplicate attrset key {}", name));
                    }
                }
            }
        } else {
            out.push((k, v));
        }
    }
    if !merged_flag {
        out.push((name, value));
    }
    Ok(out)
}

/// Same attrset-merge recursion as merge_attr_field, for `let a.b = 1; a.c =
/// 2; in ...`, but `let` bindings intentionally SHADOW on a genuine leaf
/// collision instead of erroring (seed_let_shadow.px: `let x = 1; x = 2; in
/// x` => 2, "the later binding shadows the earlier one" -- a pnix-rs `let`
/// design choice that predates and diverges from merge_attr_field's
/// Nix-parity duplicate-key error for `{ }` literals).
fn merge_let_binding(bindings: Vec<(String, PxExpr)>, name: String, value: PxExpr) -> Vec<(String, PxExpr)> {
    let mut out: Vec<(String, PxExpr)> = Vec::new();
    let mut merged_flag = false;
    for (k, v) in bindings {
        if k == name && !merged_flag {
            if px_is_attrs_lit(&v) && px_is_attrs_lit(&value) {
                let mut merged = match v {
                    PxExpr::Attrs(x) => x,
                    _ => Vec::new(),
                };
                let newf = match &value {
                    PxExpr::Attrs(x) => x.clone(),
                    _ => Vec::new(),
                };
                for (nk, nv) in newf {
                    merged = merge_let_binding(merged, nk, nv);
                }
                out.push((k, PxExpr::Attrs(merged)));
            } else {
                out.push((k, value.clone()));
            }
            merged_flag = true;
        } else {
            out.push((k, v));
        }
    }
    if !merged_flag {
        out.push((name, value));
    }
    out
}

/// Strip the leading `:` inherit mark (rs-meta subset: no string slicing).
fn strip_inherit_mark(n: &str) -> String {
    let mut out = String::new();
    let mut first = true;
    for c in n.chars() {
        if first {
            first = false;
        } else {
            out.push(c);
        }
    }
    out
}



enum SelectSeg {
    Static(String),
    Dynamic(PxExpr),
}


/// `base ? s0.s1...` — guard chain over marked temps (see parse_hasattr).
fn build_hasattr_path(base: PxExpr, segs: &Vec<PxExpr>, idx: usize) -> PxExpr {
    let has = PxExpr::Binary {
        op: PxOp::HasAttr,
        lhs: Box::new(base.clone()),
        rhs: Box::new(segs[idx].clone()),
    };
    if idx + 1 == segs.len() {
        return has;
    }
    let tmp = format!(":hp{}", idx);
    PxExpr::LetIn {
        bindings: vec![(tmp.clone(), base)],
        body: Box::new(PxExpr::If {
            cond: Box::new(PxExpr::Binary {
                op: PxOp::HasAttr,
                lhs: Box::new(PxExpr::Var(tmp.clone())),
                rhs: Box::new(segs[idx].clone()),
            }),
            then_e: Box::new(build_hasattr_path(
                getattr_apply(segs[idx].clone(), PxExpr::Var(tmp)),
                segs,
                idx + 1,
            )),
            else_e: Box::new(PxExpr::Bool(false)),
        }),
    }
}

/// `builtins.getAttr <name-expr> <base>` application node.
fn getattr_apply(name_expr: PxExpr, base: PxExpr) -> PxExpr {
    PxExpr::Apply {
        func: Box::new(PxExpr::Apply {
            func: Box::new(PxExpr::Select {
                base: Box::new(PxExpr::Var(String::from("builtins"))),
                name: String::from("getAttr"),
            }),
            arg: Box::new(name_expr),
        }),
        arg: Box::new(base),
    }
}

/// or-default over a mixed static/dynamic segment chain (see parse_select).
fn build_select_or_segs(base: PxExpr, segs: &Vec<SelectSeg>, default: &PxExpr, idx: usize) -> PxExpr {
    let tmp = format!(":or{}", idx);
    let (cond_rhs, selected) = match &segs[idx] {
        SelectSeg::Static(n) => (
            PxExpr::Str(vec![PxStrPart::Lit(n.clone())]),
            PxExpr::Select {
                base: Box::new(PxExpr::Var(tmp.clone())),
                name: n.clone(),
            },
        ),
        SelectSeg::Dynamic(e) => (
            e.clone(),
            getattr_apply(e.clone(), PxExpr::Var(tmp.clone())),
        ),
    };
    let then_e = if idx + 1 == segs.len() {
        selected
    } else {
        build_select_or_segs(selected, segs, default, idx + 1)
    };
    PxExpr::LetIn {
        bindings: vec![(tmp.clone(), base)],
        body: Box::new(PxExpr::If {
            cond: Box::new(PxExpr::Binary {
                op: PxOp::HasAttr,
                lhs: Box::new(PxExpr::Var(tmp)),
                rhs: Box::new(cond_rhs),
            }),
            then_e: Box::new(then_e),
            else_e: Box::new(default.clone()),
        }),
    }
}

/// Desugar one step of `base.n0.n1... or default` (see parse_select). The
/// `:or<depth>` temp names are unlexable as px identifiers (`:`), so they
/// cannot collide with user bindings; nesting shadows lexically, which is
/// correct. `idx` walks `names` (no slice — rs-meta subset).
fn build_select_or(base: PxExpr, names: &Vec<String>, default: &PxExpr, idx: usize) -> PxExpr {
    let tmp = format!(":or{}", idx);
    let name = names[idx].clone();
    let selected = PxExpr::Select {
        base: Box::new(PxExpr::Var(tmp.clone())),
        name: name.clone(),
    };
    let then_e = if idx + 1 == names.len() {
        selected
    } else {
        build_select_or(selected, names, default, idx + 1)
    };
    PxExpr::LetIn {
        bindings: vec![(tmp.clone(), base)],
        body: Box::new(PxExpr::If {
            cond: Box::new(PxExpr::Binary {
                op: PxOp::HasAttr,
                lhs: Box::new(PxExpr::Var(tmp)),
                rhs: Box::new(PxExpr::Str(vec![PxStrPart::Lit(name)])),
            }),
            then_e: Box::new(then_e),
            else_e: Box::new(default.clone()),
        }),
    }
}

pub fn px_parse(src: &str) -> Result<PxExpr, String> {
    let (toks, offs) = px_lex(src)?;
    let mut p = PxParser {
        toks,
        offs,
        pos: 0,
        src: String::from(src),
    };
    let expr = p.parse_expr()?;
    if p.pos != p.toks.len() {
        return Err(format!("px parse: trailing tokens at {}", p.cur_desc()));
    }
    Ok(expr)
}

// ---- evaluator ----------------------------------------------------------------

/// Scalar literals whose evaluation cannot force another value. Lambdas stay
/// lazy even though constructing a closure is pure: Nix exposes their shared
/// thunk identity when they are nested in lists/attrsets. Lambda bodies are
/// already `Rc<PxExpr>`, so thunking them remains an O(1) AST clone.
fn px_expr_is_immediate(expr: &PxExpr) -> bool {
    match expr {
        PxExpr::Int(_) => true,
        PxExpr::Float(_) => true,
        PxExpr::Bool(_) => true,
        PxExpr::Null => true,
        PxExpr::Str(parts) => {
            let mut lit_only = true;
            for part in parts {
                match part {
                    PxStrPart::Lit(_) => {}
                    PxStrPart::Sub(_) => lit_only = false,
                }
            }
            lit_only
        }
        _ => false,
    }
}

pub fn px_eval_outcome(expr: &PxExpr, env: &Vec<PxFrame>) -> Result<PxVal, PxError> {
    match expr {
        PxExpr::DeferredError(message) => Err(px_error_unsupported(message.clone())),
        PxExpr::Isolated { with_scope, body } => {
            let fresh_env: Vec<PxFrame> = match with_scope {
                Some(scope_expr) => {
                    let scope_val = px_eval_outcome(scope_expr, env)?;
                    vec![PxFrame::With(scope_val)]
                }
                None => Vec::new(),
            };
            px_eval_outcome(body, &fresh_env)
        }
        PxExpr::Int(n) => Ok(PxVal::Int(*n)),
        PxExpr::Float(f) => Ok(PxVal::Float(*f)),
        PxExpr::Bool(b) => Ok(PxVal::Bool(*b)),
        PxExpr::Null => Ok(PxVal::Null),
        PxExpr::Str(parts) => {
            let mut out = String::new();
            // A contextful chunk interpolates as itself; the template joiner
            // unions the contexts of every chunk (matches pnix-clj's
            // `eval-string-template`). Context-free templates stay a plain
            // Str, byte-identical to before this feature existed.
            let mut ctx: Vec<String> = Vec::new();
            for part in parts {
                match part {
                    PxStrPart::Lit(s) => out.push_str(s),
                    PxStrPart::Sub(e) => {
                        let v = px_force_outcome(&px_eval_outcome(e, env)?)?;
                        match &v {
                            PxVal::Str(s) => out.push_str(s),
                            // `${./p}` interpolation coerces via a fake
                            // store-path string, not the path's own literal
                            // text (see px_fake_store_path_for) -- a
                            // deliberately different, simpler mechanism than
                            // the context-carrying coercions right below it.
                            PxVal::Path(p) => out.push_str(&px_fake_store_path_for(p)),
                            _ if px_is_ctx_string(&v) => {
                                if let Some(content) = px_string_like_content(&v) {
                                    out.push_str(&content);
                                    ctx.extend(px_string_like_context(&v));
                                }
                            }
                            other => {
                                return Err(px_error_type(format!(
                                    "px: interpolation must be a string, got {} (use builtins.toString)",
                                    px_kind(other)
                                )))
                            }
                        }
                    }
                }
            }
            Ok(px_ctx_string(out, ctx))
        }
        PxExpr::Var(name) => px_lookup_outcome(name, env),
        PxExpr::List(items) => {
            let mut shared: Option<Rc<Vec<PxFrame>>> = None;
            let mut vals = Vec::new();
            for e in items {
                let value = if px_expr_is_immediate(e) {
                    px_eval_outcome(e, env)?
                } else {
                    if shared.is_none() {
                        shared = Some(Rc::new(env.clone()));
                    }
                    let senv = match &shared {
                        Some(value) => value.clone(),
                        None => Rc::new(env.clone()),
                    };
                    px_thunk(e.clone(), senv)
                };
                vals.push(value);
            }
            Ok(px_list(vals))
        }
        PxExpr::Select { base, name } => {
            let b = px_force_outcome(&px_eval_outcome(base, env)?)?;
            match b {
                // A context-bearing string is never selectable (real Nix:
                // strings are never attrsets). NOTE: this deliberately
                // diverges from the pnix-clj oracle, which represents
                // ctx-strings as a plain map too but never guards `.` select
                // with its `attrset-value?` predicate (unlike `?`/`//`,
                // which it DOES guard) — oracle-confirmed to leak the raw
                // representation (`a.string` returns the content). That is a
                // representational accident in one function, not a modeled
                // Nix behavior; the pnix-cljs port already avoids it (it
                // uses a genuinely distinct record type), and pnix-rs
                // follows that cleaner precedent here instead of
                // replicating the leak.
                PxVal::Attrs(_) if px_is_ctx_string(&b) => Err(px_error_type(format!(
                    "px: cannot select from {}",
                    px_kind(&b),
                ))),
                PxVal::Attrs(fields) => match px_attrs_find(fields.as_ref(), name) {
                    // force at extraction: the containment invariant keeps
                    // thunks inside Attrs slots and forces them out here, so a
                    // selected field never escapes to the caller as a Thunk.
                    Some(v) => px_force_outcome(v),
                    None => Err(px_error_eval(
                        PxErrorClass::AttributeMissing,
                        format!("px: attrset has no attribute {}", name),
                    )),
                },
                other => Err(px_error_type(format!(
                    "px: cannot select from {}",
                    px_kind(&other),
                ))),
            }
        }
        PxExpr::Lambda { param, body } => Ok(PxVal::Closure {
            param: param.clone(),
            body: body.clone(),
            env: env.clone(),
        }),
        PxExpr::Apply { func, arg } => {
            let f = px_eval_outcome(func, env)?;
            let a = if px_expr_is_immediate(arg) {
                px_eval_outcome(arg, env)?
            } else {
                px_thunk(arg.as_ref().clone(), Rc::new(env.clone()))
            };
            px_apply_outcome(&f, a)
        }
        PxExpr::If { cond, then_e, else_e } => {
            let c = px_force_outcome(&px_eval_outcome(cond, env)?)?;
            match c {
                PxVal::Bool(true) => px_eval_outcome(then_e, env),
                PxVal::Bool(false) => px_eval_outcome(else_e, env),
                other => Err(px_error_eval(
                    PxErrorClass::NonBooleanCondition,
                    format!(
                    "px: if condition must be bool, got {}",
                    px_kind(&other)
                    ),
                )),
            }
        }
        PxExpr::Binary { op, lhs, rhs } => {
            let l = px_eval_outcome(lhs, env)?;
            let r = px_eval_outcome(rhs, env)?;
            px_binary_outcome(op, &l, &r)
        }
        PxExpr::With { scope, body } => {
            // Nix does not force the scope unless a lookup reaches it
            // (`with 1; 2` == 2). The seed is eager, so the VALUE exists
            // here, but the attrs-check moves to lookup time to match.
            let sv = px_eval_outcome(scope, env)?;
            let mut env2 = env.clone();
            env2.push(PxFrame::With(sv));
            px_eval_outcome(body, &env2)
        }
        PxExpr::LetIn { bindings, body } => {
            let mut let_env = env.clone();
            let mut empty = Vec::new();
            let mut slot = 0usize;
            while slot < bindings.len() {
                empty.push(None);
                slot += 1;
            }
            let_env.push(PxFrame::Rec(
                Rc::new(bindings.clone()),
                Rc::new(RefCell::new(empty)),
            ));
            px_eval_outcome(body, &let_env)
        }
        PxExpr::Attrs(fields) => {
            // LAZY fields: a field whose evaluation could FORCE something
            // becomes an unforced thunk capturing the current env, so building
            // the attrset never forces it. That is what lets Nix-style
            // mutually-recursive / cyclic attrsets be built and have
            // `attrNames`/`typeOf`/partial access work without walking the
            // cycle. px_attrs sorts by NAME only, so unforced values are fine.
            //
            // IMMEDIATE fields are evaluated eagerly instead. Making a thunk
            // costs a DEEP clone of the field's AST subtree (`e.clone()`);
            // for a lambda that is catastrophic, because eager evaluation of a
            // lambda just builds a Closure over an `Rc<PxExpr>` body in O(1) —
            // and these modules are attrsets OF lambdas, re-built on every DI
            // application. Immediate exprs also cannot force anything, so
            // evaluating them here is observationally identical to a thunk that
            // would yield the same value on first force.
            let mut shared: Option<Rc<Vec<PxFrame>>> = None;
            let mut vals = Vec::new();
            for (name, e) in fields {
                let immediate = px_expr_is_immediate(e);
                let v = if immediate {
                    px_eval_outcome(e, env)?
                } else {
                    if shared.is_none() {
                        shared = Some(Rc::new(env.clone()));
                    }
                    let senv = match &shared {
                        Some(r) => r.clone(),
                        None => Rc::new(env.clone()),
                    };
                    px_thunk(e.clone(), senv)
                };
                vals.push((name.clone(), v));
            }
            Ok(px_attrs(vals))
        }
    }
}

pub fn px_eval(expr: &PxExpr, env: &Vec<PxFrame>) -> Result<PxVal, String> {
    px_eval_outcome(expr, env).map_err(px_error_into_diagnostic)
}

/// Apply a function value (closure or curried builtin) to one argument.
fn px_apply_outcome(f: &PxVal, a: PxVal) -> Result<PxVal, PxError> {
    // force the function position (Nix forces the callee); a function pulled
    // out of an attrset field arrives here as a thunk otherwise.
    let f = &px_force_outcome(f)?;
    match f {
        PxVal::Closure { param, body, env: closure_env } => {
            let mut call_env = closure_env.clone();
            call_env.push(PxFrame::Bind {
                name: param.clone(),
                value: a,
            });
            px_eval_outcome(body, &call_env)
        }
        PxVal::Builtin { name, args } if name == "tryEval" && args.is_empty() => {
            match px_force_outcome(&a) {
                Ok(value) => Ok(px_attrs(vec![
                    (String::from("success"), PxVal::Bool(true)),
                    (String::from("value"), value),
                ])),
                Err(error) => {
                    if error.diagnostic.starts_with("px: throw:") {
                        Ok(px_attrs(vec![
                            (String::from("success"), PxVal::Bool(false)),
                            (String::from("value"), PxVal::Bool(false)),
                        ]))
                    } else {
                        Err(error)
                    }
                }
            }
        }
        PxVal::Builtin { name, args } => {
            let mut next_args = args.clone();
            next_args.push(a);
            if next_args.len() == px_builtin_arity(name) {
                px_builtin_exec(name, &next_args).map_err(px_error_unsupported)
            } else {
                Ok(PxVal::Builtin {
                    name: name.clone(),
                    args: next_args,
                })
            }
        }
        other => Err(px_error_eval(
            PxErrorClass::NotCallable,
            format!("px: cannot apply non-lambda {}", px_kind(other)),
        )),
    }
}

fn px_apply(f: &PxVal, a: PxVal) -> Result<PxVal, String> {
    px_apply_outcome(f, a).map_err(px_error_into_diagnostic)
}

/// Name lookup, innermost frame first. A `Rec` let frame is a recursive scope:
/// its bindings are evaluated in an environment that still contains the whole
/// frame, so siblings and self-references resolve (pnix let semantics).
/// `builtins` falls back to the fixed builtin attrset.
fn px_lookup_outcome(name: &str, env: &Vec<PxFrame>) -> Result<PxVal, PxError> {
    let mut i = env.len();
    while i > 0 {
        i -= 1;
        match &env[i] {
            PxFrame::Bind { name: bound, value } => {
                if bound == name {
                    return Ok(value.clone());
                }
            }
            PxFrame::With(_) => {}
            PxFrame::Rec(bindings, cache) => {
                // Later bindings shadow earlier ones within a frame (pnix let
                // semantics, audit A4): scan the frame back-to-front.
                let mut j = bindings.len();
                while j > 0 {
                    j -= 1;
                    let (bound, expr) = &bindings[j];
                    if bound == name {
                        let cached = cache.borrow()[j].clone();
                        match cached {
                            Some(Ok(value)) => return Ok(value),
                            Some(Err(error)) => return Err(error),
                            None => {}
                        }
                        let loop_error = px_error_eval(
                            PxErrorClass::CycleDetected,
                            String::from(
                                "px: infinite recursion encountered (recursive value forced itself)",
                            ),
                        );
                        {
                            let mut cache_entries = cache.borrow_mut();
                            cache_entries[j] = Some(Err(loop_error));
                        }
                        let scope = env[0..i + 1].to_vec();
                        let evaluated = px_eval_outcome(expr, &scope);
                        match evaluated {
                            Ok(value) => {
                                {
                                    let mut cache_entries = cache.borrow_mut();
                                    cache_entries[j] = Some(Ok(value.clone()));
                                }
                                return Ok(value);
                            }
                            Err(error) => {
                                {
                                    let mut cache_entries = cache.borrow_mut();
                                    cache_entries[j] = Some(Err(error.clone()));
                                }
                                return Err(error);
                            }
                        }
                    }
                }
            }
        }
    }
    if name == "builtins" {
        return Ok(px_builtins_attrset());
    }
    if name == "lib" {
        return Ok(px_lib_attrset());
    }
    // Path literals `./x` parse as `:path:./x` vars. An occurrence consumed
    // by `import`/`scopedImport` is spliced away by px_expand_imports before
    // eval ever sees it; every other occurrence (a bare path literal used as
    // an ordinary expression, or ANY path literal at all under `-c` inline
    // mode, which skips expansion entirely) resolves here to a real
    // `PxVal::Path`, normalized the same way every other path construction
    // site is.
    if name.starts_with(":path:") {
        let mut out = String::new();
        let mut i = 0usize;
        for c in name.chars() {
            if i >= 6 {
                out.push(c);
            }
            i += 1;
        }
        return Ok(PxVal::Path(px_normalize_path(&out)));
    }
    // D14 parity: real Nix binds a fixed subset of builtins UNPREFIXED at
    // the top level (shadowable by let, checked before with-scopes).
    if name == "baseNameOf" || name == "dirOf" || name == "map"
        || name == "toString" || name == "isNull" || name == "removeAttrs"
        || name == "throw" || name == "abort"
    {
        return Ok(PxVal::Builtin {
            name: String::from(name),
            args: Vec::new(),
        });
    }
    // with scopes: lowest priority, newest first (oracle-pinned).
    let mut i = env.len();
    while i > 0 {
        i -= 1;
        if let PxFrame::With(sv) = &env[i] {
            match sv {
                PxVal::Attrs(fields) => {
                    for (k, v) in fields.iter() {
                        if k == name {
                            // extraction point: never hand a Thunk to the caller
                            return px_force_outcome(v);
                        }
                    }
                }
                other => {
                    return Err(px_error_type(format!(
                        "px: with expects an attrset, got {}",
                        px_kind(other)
                    )))
                }
            }
        }
    }
    Err(px_error_eval(
        PxErrorClass::UnknownVariable,
        format!("px: unbound variable {}", name),
    ))
}

fn px_binary_outcome(op: &PxOp, l: &PxVal, r: &PxVal) -> Result<PxVal, PxError> {
    // operators are strict: force both operands out of any attrset-field thunk
    let l = &px_force_outcome(l)?;
    let r = &px_force_outcome(r)?;
    // Nix `?` (oracle-pinned 2026-07-08): attrset -> membership; EVERY other
    // value (int/null/list/...) -> false. NOT builtins.hasAttr, which errors
    // on non-sets (both oracle-confirmed) — the two must stay distinct.
    if let PxOp::HasAttr = op {
        let name = match r {
            PxVal::Str(s) => s,
            other => return Err(px_error_type(format!(
                "px: `?` name must be a string, got {:?}",
                other,
            ))),
        };
        return Ok(PxVal::Bool(match l {
            // A context-bearing string is never a real attrset (oracle-
            // confirmed: `?` on a ctx-string is false, unlike `.` select,
            // which the clj oracle itself leaks through — pnix-rs instead
            // follows the cleaner cljs port here and refuses the leak).
            PxVal::Attrs(fields) if px_is_real_attrset(l) => {
                fields.iter().any(|(k, _)| k == name && !px_is_attr_pos_key(k))
            }
            _ => false,
        }));
    }
    // Nix ==/!= never error on a TYPE mismatch — `1 == "a"` and `"x" != null`
    // are plain false/true (oracle-pinned 2026-07-08). Route every equality
    // through the deep structural px_val_eq (cross-type -> false). Numeric
    // leaves promote int to f64, including nested list/attrset equality.
    match op {
        PxOp::Eq => return Ok(PxVal::Bool(
            px_val_eq(l, r).map_err(px_error_unsupported)?,
        )),
        PxOp::Ne => return Ok(PxVal::Bool(
            !px_val_eq(l, r).map_err(px_error_unsupported)?,
        )),
        _ => {}
    }
    match (l, r) {
        // String-context-aware operators: `+` concat unions both operands'
        // contexts (collapsing to a plain Str when the union is empty);
        // ordering compares CONTENT only (context never participates in
        // ordering, same as equality). This is a LANGUAGE operator, so it is
        // always context-aware — never gated by the builtin allowlist (`+`/
        // `<` etc. are not routed through px_builtin_exec). Matches
        // pnix-clj's `binary-value-result` treating string concat/ordering
        // this way unconditionally.
        (PxVal::Attrs(_), _) | (_, PxVal::Attrs(_))
            if px_is_ctx_string(l) || px_is_ctx_string(r) =>
        {
            match (px_string_like_content(l), px_string_like_content(r)) {
                (Some(lc), Some(rc)) => match op {
                    PxOp::Add => {
                        let mut out = lc;
                        out.push_str(&rc);
                        Ok(px_ctx_string(out, px_ctx_union(l, r)))
                    }
                    PxOp::Lt => Ok(PxVal::Bool(lc < rc)),
                    PxOp::Le => Ok(PxVal::Bool(lc <= rc)),
                    PxOp::Gt => Ok(PxVal::Bool(lc > rc)),
                    PxOp::Ge => Ok(PxVal::Bool(lc >= rc)),
                    _ => Err(px_error_type(String::from(
                        "px: unsupported string-context operation",
                    ))),
                },
                _ => Err(px_error_type(format!(
                    "px: unsupported operands {} and {}",
                    px_kind(l),
                    px_kind(r)
                ))),
            }
        }
        (PxVal::Float(a), PxVal::Float(b)) => {
            px_float_binary_outcome(op, *a, *b)
        }
        (PxVal::Int(a), PxVal::Float(b)) => {
            px_float_binary_outcome(op, *a as f64, *b)
        }
        (PxVal::Float(a), PxVal::Int(b)) => {
            px_float_binary_outcome(op, *a, *b as f64)
        }
        (PxVal::Int(a), PxVal::Int(b)) => {
            let a = *a;
            let b = *b;
            match op {
                PxOp::Add => Ok(PxVal::Int(px_manifest_int_arith_outcome("add", a, b)?)),
                PxOp::Sub => Ok(PxVal::Int(px_manifest_int_arith_outcome("sub", a, b)?)),
                PxOp::Mul => Ok(PxVal::Int(px_manifest_int_arith_outcome("mul", a, b)?)),
                PxOp::Div => Ok(PxVal::Int(px_manifest_int_arith_outcome("div", a, b)?)),
                PxOp::Eq => Ok(PxVal::Bool(a == b)),
                PxOp::Ne => Ok(PxVal::Bool(a != b)),
                PxOp::Lt => Ok(PxVal::Bool(a < b)),
                PxOp::Le => Ok(PxVal::Bool(a <= b)),
                PxOp::Gt => Ok(PxVal::Bool(a > b)),
                PxOp::Ge => Ok(PxVal::Bool(a >= b)),
                _ => Err(px_error_type(String::from(
                    "px: unsupported int operation",
                ))),
            }
        }
        (PxVal::Bool(a), PxVal::Bool(b)) => match op {
            PxOp::Eq => Ok(PxVal::Bool(*a == *b)),
            PxOp::Ne => Ok(PxVal::Bool(*a != *b)),
            _ => Err(px_error_type(String::from(
                "px: unsupported bool operation",
            ))),
        },
        // RAW-BYTE track: when either side is Bytes, string binops go
        // byte-wise (concat revalidates back to Str when the result is
        // valid UTF-8 — oracle: substring 0 1 "가" + substring 1 2 "가"
        // == "가").
        (PxVal::Bytes(_), PxVal::Str(_))
        | (PxVal::Str(_), PxVal::Bytes(_))
        | (PxVal::Bytes(_), PxVal::Bytes(_)) => {
            let xa = match px_val_bytes(l) {
                Some(v) => v,
                None => return Err(px_error_type(String::from(
                    "px: bytes op: not string-like",
                ))),
            };
            let xb = match px_val_bytes(r) {
                Some(v) => v,
                None => return Err(px_error_type(String::from(
                    "px: bytes op: not string-like",
                ))),
            };
            match op {
                PxOp::Add => {
                    let mut out = xa.clone();
                    for v in xb.iter() {
                        out.push(*v);
                    }
                    Ok(px_bytes_val(out))
                }
                PxOp::Eq => Ok(PxVal::Bool(xa == xb)),
                PxOp::Ne => Ok(PxVal::Bool(xa != xb)),
                PxOp::Lt => Ok(PxVal::Bool(xa < xb)),
                PxOp::Le => Ok(PxVal::Bool(xa <= xb)),
                PxOp::Gt => Ok(PxVal::Bool(xa > xb)),
                PxOp::Ge => Ok(PxVal::Bool(xa >= xb)),
                _ => Err(px_error_type(String::from(
                    "px: unsupported raw-bytes string operation",
                ))),
            }
        }
        (PxVal::Str(a), PxVal::Str(b)) => match op {
            // Nix string concatenation (oracle-pinned: "a"+"b" == "ab";
            // string+int stays an error on both sides).
            PxOp::Add => {
                let mut out = String::new();
                out.push_str(a);
                out.push_str(b);
                Ok(PxVal::Str(out))
            }
            PxOp::Eq => Ok(PxVal::Bool(a == b)),
            PxOp::Ne => Ok(PxVal::Bool(a != b)),
            // Nix string ordering is byte-lexicographic ("Z" < "a" is true) —
            // Rust &str comparison matches (oracle-pinned 2026-07-08).
            PxOp::Lt => Ok(PxVal::Bool(a < b)),
            PxOp::Le => Ok(PxVal::Bool(a <= b)),
            PxOp::Gt => Ok(PxVal::Bool(a > b)),
            PxOp::Ge => Ok(PxVal::Bool(a >= b)),
            _ => Err(px_error_type(String::from(
                "px: unsupported string operation",
            ))),
        },
        // Path arithmetic (oracle-pinned): `+` concatenates the RAW text of
        // both operands (no `/` inserted -- Nix's own path arithmetic does
        // not insert a separator either, relying on normalization to
        // collapse whatever the literal text produces) and normalizes the
        // result, so `./a + ./../b` collapses to `./b` rather than staying
        // as the literal `./a./../b`. Ordering compares the normalized text.
        (PxVal::Path(a), PxVal::Path(b)) => match op {
            PxOp::Add => {
                let mut combined = String::new();
                combined.push_str(a);
                combined.push_str(b);
                Ok(PxVal::Path(px_normalize_path(&combined)))
            }
            PxOp::Eq => Ok(PxVal::Bool(a == b)),
            PxOp::Ne => Ok(PxVal::Bool(a != b)),
            PxOp::Lt => Ok(PxVal::Bool(a < b)),
            PxOp::Le => Ok(PxVal::Bool(a <= b)),
            PxOp::Gt => Ok(PxVal::Bool(a > b)),
            PxOp::Ge => Ok(PxVal::Bool(a >= b)),
            _ => Err(px_error_type(String::from(
                "px: unsupported path operation",
            ))),
        },
        // path + string -> path (string coerces via its raw content). A
        // context-bearing right-hand side is caught earlier by the
        // ctx-string guard above (it matches on either side being the
        // Attrs-tagged ctx-string shape) and fails closed there, so this
        // arm only ever sees a plain, context-free string.
        (PxVal::Path(p), PxVal::Str(s)) => match op {
            PxOp::Add => {
                let mut combined = String::new();
                combined.push_str(p);
                combined.push_str(s);
                Ok(PxVal::Path(px_normalize_path(&combined)))
            }
            _ => Err(px_error_type(String::from(
                "px: unsupported operands path and string",
            ))),
        },
        // string + path -> string (Nix coerces the path to its text; the
        // left string's own context, if any, is unaffected).
        (PxVal::Str(s), PxVal::Path(p)) => match op {
            PxOp::Add => {
                let mut combined = String::new();
                combined.push_str(s);
                combined.push_str(p);
                Ok(PxVal::Str(combined))
            }
            _ => Err(px_error_type(String::from(
                "px: unsupported operands string and path",
            ))),
        },
        (PxVal::List(a), PxVal::List(b)) => match op {
            PxOp::Eq => Ok(PxVal::Bool(
                px_val_eq(l, r).map_err(px_error_unsupported)?,
            )),
            PxOp::Ne => Ok(PxVal::Bool(
                !px_val_eq(l, r).map_err(px_error_unsupported)?,
            )),
            PxOp::Concat => {
                let mut out = a.as_ref().clone();
                for v in b.iter() {
                    out.push(v.clone());
                }
                Ok(px_list(out))
            }
            PxOp::Lt | PxOp::Le | PxOp::Gt | PxOp::Ge => {
                let yes = match op {
                    PxOp::Lt => px_val_lt(l, r).map_err(px_error_unsupported)?,
                    PxOp::Le => !px_val_lt(r, l).map_err(px_error_unsupported)?,
                    PxOp::Gt => px_val_lt(r, l).map_err(px_error_unsupported)?,
                    _ => !px_val_lt(l, r).map_err(px_error_unsupported)?,
                };
                Ok(PxVal::Bool(yes))
            }
            _ => Err(px_error_type(String::from(
                "px: unsupported list operation",
            ))),
        },
        (PxVal::Attrs(a), PxVal::Attrs(b)) => match op {
            PxOp::Eq => Ok(PxVal::Bool(
                px_val_eq(l, r).map_err(px_error_unsupported)?,
            )),
            PxOp::Ne => Ok(PxVal::Bool(
                !px_val_eq(l, r).map_err(px_error_unsupported)?,
            )),
            PxOp::Update => {
                // Both sides sorted (proposal 0002): single merge pass,
                // right side wins on collisions. Positions merge as their
                // own attrset (right wins per attribute name).
                let (ua, pa) = px_split_attr_pos(a);
                let (ub, pb) = px_split_attr_pos(b);
                let mut out = Vec::new();
                let mut i = 0usize;
                let mut j = 0usize;
                while i < ua.len() && j < ub.len() {
                    if ua[i].0 == ub[j].0 {
                        out.push((ub[j].0.clone(), ub[j].1.clone()));
                        i += 1;
                        j += 1;
                    } else if px_str_lt(&ua[i].0, &ub[j].0) {
                        out.push((ua[i].0.clone(), ua[i].1.clone()));
                        i += 1;
                    } else {
                        out.push((ub[j].0.clone(), ub[j].1.clone()));
                        j += 1;
                    }
                }
                while i < ua.len() {
                    out.push((ua[i].0.clone(), ua[i].1.clone()));
                    i += 1;
                }
                while j < ub.len() {
                    out.push((ub[j].0.clone(), ub[j].1.clone()));
                    j += 1;
                }
                let pos = match (pa, pb) {
                    (Some(lpos), Some(rpos)) => match (lpos, rpos) {
                        (PxVal::Attrs(lf), PxVal::Attrs(rf)) => Some(px_binary_outcome(
                            &PxOp::Update,
                            &PxVal::Attrs(lf),
                            &PxVal::Attrs(rf),
                        )?),
                        (_, rpos) => Some(rpos),
                    },
                    (None, Some(p)) => Some(p),
                    (Some(p), None) => Some(p),
                    (None, None) => None,
                };
                Ok(px_join_attr_pos(out, pos))
            }
            _ => Err(px_error_type(String::from(
                "px: unsupported attrset operation",
            ))),
        },
        _ => Err(px_error_type(format!(
            "px: unsupported operands {} and {}",
            px_kind(l),
            px_kind(r)
        ))),
    }
}

fn px_attrs_has(fields: &[(String, PxVal)], name: &str) -> bool {
    px_attrs_find(fields, name).is_some()
}

// ---- lib / host helpers ------------------------------------------------------

fn px_as_path_str(v: &PxVal) -> Result<String, String> {
    match v {
        PxVal::Str(s) => Ok(s.clone()),
        PxVal::Path(p) => Ok(p.clone()),
        other => Err(format!("px: expected a path/string, got {}", px_kind(other))),
    }
}

fn px_str_has_suffix(s: &str, suf: &str) -> bool {
    let sc: Vec<char> = s.chars().collect();
    let pc: Vec<char> = suf.chars().collect();
    if pc.len() > sc.len() {
        return false;
    }
    let mut i = 0usize;
    while i < pc.len() {
        if sc[sc.len() - pc.len() + i] != pc[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Naive substring search (char-based, mirrors `px_str_has_suffix`'s manual
/// Vec<char> comparison rather than relying on `str::contains`).
fn px_str_has_infix(hay: &str, needle: &str) -> bool {
    let hc: Vec<char> = hay.chars().collect();
    let nc: Vec<char> = needle.chars().collect();
    if nc.is_empty() {
        return true;
    }
    if nc.len() > hc.len() {
        return false;
    }
    let mut i = 0usize;
    while i + nc.len() <= hc.len() {
        let mut matched = true;
        let mut j = 0usize;
        while j < nc.len() {
            if hc[i + j] != nc[j] {
                matched = false;
            }
            j += 1;
        }
        if matched {
            return true;
        }
        i += 1;
    }
    false
}

fn px_flatten_into(v: &PxVal, out: &mut Vec<PxVal>) -> Result<(), String> {
    let v = px_force(v)?;
    match v {
        PxVal::List(items) => {
            for it in items.iter() {
                px_flatten_into(it, out)?;
            }
            Ok(())
        }
        other => {
            out.push(other);
            Ok(())
        }
    }
}

fn px_to_xml(v: &PxVal) -> Result<String, String> {
    let mut out = String::from("<?xml version='1.0' encoding='utf-8'?>\n");
    out.push_str(&px_to_xml_value(v, 0)?);
    out.push('\n');
    Ok(out)
}

fn px_xml_indent(n: usize) -> String {
    let mut s = String::new();
    let mut i = 0usize;
    while i < n {
        s.push_str("  ");
        i += 1;
    }
    s
}

fn px_xml_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c == '&' {
            out.push_str("&amp;");
        } else if c == '<' {
            out.push_str("&lt;");
        } else if c == '>' {
            out.push_str("&gt;");
        } else if c == '"' {
            out.push_str("&quot;");
        } else if c == '\'' {
            out.push_str("&apos;");
        } else {
            out.push(c);
        }
    }
    out
}

fn px_to_xml_value(v: &PxVal, depth: usize) -> Result<String, String> {
    let v = px_force(v)?;
    let ind = px_xml_indent(depth);
    match v {
        PxVal::Int(n) => Ok(format!("{}<int>{}</int>", ind, n)),
        PxVal::Float(f) => Ok(format!("{}<float>{}</float>", ind, f)),
        PxVal::Bool(true) => Ok(format!("{}<bool>true</bool>", ind)),
        PxVal::Bool(false) => Ok(format!("{}<bool>false</bool>", ind)),
        PxVal::Null => Ok(format!("{}<null />", ind)),
        PxVal::Str(s) => Ok(format!("{}<string>{}</string>", ind, px_xml_escape(&s))),
        // toXML has no distinct path tag; serialize its text like a string.
        PxVal::Path(p) => Ok(format!("{}<string>{}</string>", ind, px_xml_escape(&p))),
        PxVal::Bytes(b) => {
            let mut s = String::new();
            for x in b.iter() {
                s.push(*x as u8 as char);
            }
            Ok(format!("{}<string>{}</string>", ind, px_xml_escape(&s)))
        }
        PxVal::List(items) => {
            let mut out = format!("{}<list>\n", ind);
            for it in items.iter() {
                out.push_str(&px_to_xml_value(it, depth + 1)?);
                out.push('\n');
            }
            out.push_str(&format!("{}</list>", ind));
            Ok(out)
        }
        PxVal::Attrs(fields) => {
            let mut out = format!("{}<attrs>\n", ind);
            let mut names = Vec::new();
            for (k, _v) in fields.iter() {
                if !px_is_attr_pos_key(k) {
                    names.push(k.clone());
                }
            }
            let sorted = px_sort_strings(names);
            for n in sorted {
                for (k, val) in fields.iter() {
                    if *k == n {
                        out.push_str(&format!(
                            "{}  <attr name=\"{}\">\n",
                            ind,
                            px_xml_escape(k)
                        ));
                        out.push_str(&px_to_xml_value(val, depth + 2)?);
                        out.push('\n');
                        out.push_str(&format!("{}  </attr>\n", ind));
                    }
                }
            }
            out.push_str(&format!("{}</attrs>", ind));
            Ok(out)
        }
        PxVal::Closure { .. } | PxVal::Builtin { .. } => {
            Err(String::from("px: toXML cannot serialize a function"))
        }
        PxVal::Thunk(_) => {
            // already forced above; unreachable under containment
            Err(String::from("px: toXML internal thunk"))
        }
    }
}

fn px_get_attr_from_path(
    path: &Rc<Vec<PxVal>>,
    set: &PxVal,
    strict: bool,
    default: Option<PxVal>,
) -> Result<PxVal, String> {
    let mut cur = px_force(set)?;
    for seg in path.iter() {
        let key = match px_force(seg)? {
            PxVal::Str(s) => s,
            other => {
                return Err(format!(
                    "px: attr path segment must be string, got {}",
                    px_kind(&other)
                ))
            }
        };
        match &cur {
            PxVal::Attrs(fields) => match px_attrs_find(fields.as_ref(), &key) {
                Some(v) => cur = px_force(v)?,
                None => {
                    if let Some(d) = default {
                        return Ok(d);
                    }
                    if strict {
                        return Err(format!("px: attribute '{}' missing", key));
                    }
                    return Err(format!("px: attribute '{}' missing", key));
                }
            },
            other => {
                if let Some(d) = default {
                    return Ok(d);
                }
                return Err(format!(
                    "px: expected attrset in path, got {}",
                    px_kind(other)
                ));
            }
        }
    }
    Ok(cur)
}

fn px_recursive_update(left: &PxVal, right: &PxVal) -> Result<PxVal, String> {
    let l = px_force(left)?;
    let r = px_force(right)?;
    match (&l, &r) {
        (PxVal::Attrs(a), PxVal::Attrs(b)) => {
            let mut out = Vec::new();
            let mut names = Vec::new();
            for (k, _) in a.iter() {
                names.push(k.clone());
            }
            for (k, _) in b.iter() {
                let mut seen = false;
                for n in names.iter() {
                    if n == k {
                        seen = true;
                    }
                }
                if !seen {
                    names.push(k.clone());
                }
            }
            let sorted = px_sort_strings(names);
            for n in sorted {
                let lv = px_attrs_find(a.as_ref(), &n);
                let rv = px_attrs_find(b.as_ref(), &n);
                match (lv, rv) {
                    (Some(x), Some(y)) => {
                        let xf = px_force(x)?;
                        let yf = px_force(y)?;
                        if matches!(&xf, PxVal::Attrs(_)) && matches!(&yf, PxVal::Attrs(_)) {
                            out.push((n, px_recursive_update(&xf, &yf)?));
                        } else {
                            out.push((n, y.clone()));
                        }
                    }
                    (Some(x), None) => out.push((n, x.clone())),
                    (None, Some(y)) => out.push((n, y.clone())),
                    (None, None) => {}
                }
            }
            Ok(px_attrs(out))
        }
        _ => Ok(r),
    }
}

fn px_filter_attrs_recursive(pred: &PxVal, set: &PxVal) -> Result<PxVal, String> {
    let set = px_force(set)?;
    match set {
        PxVal::Attrs(fields) => {
            let mut out = Vec::new();
            for (k, v) in fields.iter() {
                let vv = px_force(v)?;
                if matches!(&vv, PxVal::Attrs(_)) {
                    let nested = px_filter_attrs_recursive(pred, &vv)?;
                    // keep nested if non-empty or predicate accepts
                    let step = px_apply(pred, PxVal::Str(k.clone()))?;
                    let keep = px_force(&px_apply(&step, vv.clone())?)?;
                    match keep {
                        PxVal::Bool(true) => out.push((k.clone(), nested)),
                        PxVal::Bool(false) => {
                            if let PxVal::Attrs(nf) = &nested {
                                if !nf.is_empty() {
                                    out.push((k.clone(), nested));
                                }
                            }
                        }
                        other => {
                            return Err(format!(
                                "px: filterAttrsRecursive predicate must return bool, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                } else {
                    let step = px_apply(pred, PxVal::Str(k.clone()))?;
                    let keep = px_force(&px_apply(&step, vv.clone())?)?;
                    match keep {
                        PxVal::Bool(true) => out.push((k.clone(), v.clone())),
                        PxVal::Bool(false) => {}
                        other => {
                            return Err(format!(
                                "px: filterAttrsRecursive predicate must return bool, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                }
            }
            Ok(px_attrs(out))
        }
        other => Err(format!(
            "px: filterAttrsRecursive expects attrset, got {}",
            px_kind(&other)
        )),
    }
}

fn px_map_attrs_recursive(
    f: &PxVal,
    set: &PxVal,
    path: &Vec<String>,
) -> Result<PxVal, String> {
    let set = px_force(set)?;
    match set {
        PxVal::Attrs(fields) => {
            let mut out = Vec::new();
            for (k, v) in fields.iter() {
                let mut p2 = path.clone();
                p2.push(k.clone());
                let vv = px_force(v)?;
                if matches!(&vv, PxVal::Attrs(_)) {
                    out.push((k.clone(), px_map_attrs_recursive(f, &vv, &p2)?));
                } else {
                    let mut path_vals = Vec::new();
                    for s in p2.iter() {
                        path_vals.push(PxVal::Str(s.clone()));
                    }
                    let step = px_apply(f, px_list(path_vals))?;
                    out.push((k.clone(), px_force(&px_apply(&step, vv)?)?));
                }
            }
            Ok(px_attrs(out))
        }
        other => Err(format!(
            "px: mapAttrsRecursive expects attrset, got {}",
            px_kind(&other)
        )),
    }
}

fn px_read_dir(path: &str) -> Result<PxVal, String> {
    // Use ls(1) so the code stays inside the rs-meta Command subset (no read_dir).
    let out = match std::process::Command::new("/bin/ls")
        .arg("-1A")
        .arg(path)
        .output()
    {
        Ok(o) => o,
        Err(e) => return Err(format!("px: readDir: {}", e)),
    };
    if !out.status.success() {
        return Err(format!("px: readDir: cannot read {}", path));
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut fields = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let full = format!("{}/{}", path, line);
        let kind = if std::path::Path::new(&full).exists() {
            // classify with test(1) — portable, subset-safe
            let is_dir = match std::process::Command::new("/bin/test")
                .arg("-d")
                .arg(&full)
                .output()
            {
                Ok(o) => o.status.success(),
                Err(_) => false,
            };
            let is_link = match std::process::Command::new("/bin/test")
                .arg("-L")
                .arg(&full)
                .output()
            {
                Ok(o) => o.status.success(),
                Err(_) => false,
            };
            if is_link {
                "symlink"
            } else if is_dir {
                "directory"
            } else {
                "regular"
            }
        } else {
            "unknown"
        };
        fields.push((String::from(line), PxVal::Str(String::from(kind))));
    }
    Ok(px_attrs(fields))
}

fn px_store_write_fetched(url: &str, tag: &str) -> Result<PxVal, String> {
    let hash = px_sha256_hex(sha_utf8_bytes(url));
    let mut short = String::new();
    let mut i = 0usize;
    for c in hash.chars() {
        if i < 32 {
            short.push(c);
        }
        i += 1;
    }
    let store = String::from("/tmp/pnix-nix-store");
    match std::fs::create_dir_all(&store) {
        Ok(()) => {}
        Err(e) => return Err(format!("px: fetch: {}", e)),
    }
    let path = format!("{}/{}-{}", store, short, tag);
    if std::path::Path::new(&path).exists() {
        return Ok(PxVal::Str(path));
    }
    let out = match std::process::Command::new("curl")
        .arg("-fsSL")
        .arg("-o")
        .arg(&path)
        .arg(url)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return Err(format!(
                "px: fetchurl: curl unavailable ({}). Install curl or use a path.",
                e
            ))
        }
    };
    if !out.status.success() {
        let _ = std::fs::write(&path, ""); // ignore
        return Err(format!("px: fetchurl failed for {}", url));
    }
    Ok(PxVal::Str(path))
}

fn px_fetch_url_arg(v: &PxVal) -> Result<PxVal, String> {
    match v {
        PxVal::Str(url) => px_store_write_fetched(url, "src"),
        PxVal::Attrs(fields) => {
            let url = match px_attrs_find(fields.as_ref(), "url") {
                Some(u) => match px_force(u)? {
                    PxVal::Str(s) => s,
                    other => {
                        return Err(format!(
                            "px: fetchurl url must be string, got {}",
                            px_kind(&other)
                        ))
                    }
                },
                None => return Err(String::from("px: fetchurl requires url")),
            };
            px_store_write_fetched(&url, "src")
        }
        other => Err(format!(
            "px: fetchurl expects string or attrs, got {}",
            px_kind(other)
        )),
    }
}

fn px_fetch_tarball_arg(v: &PxVal) -> Result<PxVal, String> {
    // Download archive; leave as file path (no unpack without tar deps complexity).
    match v {
        PxVal::Str(url) => px_store_write_fetched(url, "tarball"),
        PxVal::Attrs(fields) => {
            let url = match px_attrs_find(fields.as_ref(), "url") {
                Some(u) => match px_force(u)? {
                    PxVal::Str(s) => s,
                    other => {
                        return Err(format!(
                            "px: fetchTarball url must be string, got {}",
                            px_kind(&other)
                        ))
                    }
                },
                None => return Err(String::from("px: fetchTarball requires url")),
            };
            px_store_write_fetched(&url, "tarball")
        }
        other => Err(format!(
            "px: fetchTarball expects string or attrs, got {}",
            px_kind(other)
        )),
    }
}

fn px_fetch_git_arg(v: &PxVal) -> Result<PxVal, String> {
    // Return a structured attrset describing the request; clone when possible.
    match v {
        PxVal::Attrs(fields) => {
            let url = match px_attrs_find(fields.as_ref(), "url") {
                Some(u) => match px_force(u)? {
                    PxVal::Str(s) => s,
                    other => {
                        return Err(format!(
                            "px: fetchGit url must be string, got {}",
                            px_kind(&other)
                        ))
                    }
                },
                None => return Err(String::from("px: fetchGit requires url")),
            };
            let rev = match px_attrs_find(fields.as_ref(), "rev") {
                Some(r) => match px_force(r)? {
                    PxVal::Str(s) => s,
                    _ => String::from("HEAD"),
                },
                None => String::from("HEAD"),
            };
            let hash = px_sha256_hex(sha_utf8_bytes(&format!("{}|{}", url, rev)));
            let mut short = String::new();
            let mut i = 0usize;
            for c in hash.chars() {
                if i < 32 {
                    short.push(c);
                }
                i += 1;
            }
            let store = String::from("/tmp/pnix-nix-store");
            match std::fs::create_dir_all(&store) {
                Ok(()) => {}
                Err(e) => return Err(format!("px: fetchGit: {}", e)),
            }
            let path = format!("{}/{}-git", store, short);
            if !std::path::Path::new(&path).exists() {
                let out = std::process::Command::new("git")
                    .arg("clone")
                    .arg("--depth")
                    .arg("1")
                    .arg(&url)
                    .arg(&path)
                    .output();
                match out {
                    Ok(o) if o.status.success() => {}
                    Ok(_) => {
                        // leave a marker so subsequent calls are stable
                        let _ = std::fs::write(
                            format!("{}/.pnix-fetchGit", path),
                            format!("url={}\nrev={}\n", url, rev),
                        );
                        let _ = std::fs::create_dir_all(&path);
                        let _ = std::fs::write(
                            format!("{}/.pnix-fetchGit", path),
                            format!("url={}\nrev={}\n", url, rev),
                        );
                    }
                    Err(e) => {
                        return Err(format!(
                            "px: fetchGit: git unavailable ({}). Returning path stub.",
                            e
                        ));
                    }
                }
            }
            Ok(px_attrs(vec![
                (String::from("outPath"), PxVal::Str(path.clone())),
                (String::from("rev"), PxVal::Str(rev)),
                (String::from("url"), PxVal::Str(url)),
            ]))
        }
        other => Err(format!(
            "px: fetchGit expects attrs, got {}",
            px_kind(other)
        )),
    }
}

// ---- derivations (pure simulation, no builder/store) --------------------------
//
// Deterministic pseudo store paths carrying string context, without any
// builder or on-disk store (Tvix-style separation of derivation VALUES from
// realization) — ported from pnix-clj's/pnix-cljs's `derivation-core` design.
// Paths look like /nix/store/<32-hex>-<name>(.drv) where the hex comes from
// a sha256 of the deep-forced input attrs' canonical JSON projection —
// deterministic within this host, but NOT byte-compatible with real Nix
// store hashing or with any other pnix host's own hash (documented
// simulation scope). Context elements use Nix's encoding: a drvPath depends
// on itself as "=<drvPath>", an output path as "!<output>!<drvPath>".

/// First 32 hex characters of a sha256 digest (the same truncation
/// `placeholder` already uses) — factored out since derivation hashing
/// needs it twice per output plus once for drvPath.
fn px_hex_prefix32(hex: &str) -> String {
    let hex_chars: Vec<char> = hex.chars().collect();
    let mut prefix = String::new();
    let mut i = 0usize;
    while i < 32 && i < hex_chars.len() {
        prefix.push(hex_chars[i]);
        i += 1;
    }
    prefix
}

/// Last path segment (the part after the final `/`), or the whole string
/// when there is no `/` at all -- same split rule `baseNameOf` uses.
fn px_path_basename(p: &str) -> String {
    let mut last: i64 = -1;
    let mut i = 0i64;
    for c in p.chars() {
        if c == '/' {
            last = i;
        }
        i += 1;
    }
    if last < 0 {
        return String::from(p);
    }
    let mut tail = String::new();
    let mut j = 0i64;
    for c in p.chars() {
        if j > last {
            tail.push(c);
        }
        j += 1;
    }
    tail
}

/// A bare path interpolated into a string (`"${./p}"`) does not stay a
/// literal path -- real Nix "copies the path to the store" and splices the
/// resulting store path text in. pnix has no on-disk store, so this
/// fabricates a deterministic-looking `/nix/store/<hash>-<basename>` string
/// the same way `derivation`'s pseudo store paths are built (a sha256 of a
/// tagged canonical string, truncated to 32 hex chars) -- NOT
/// byte-compatible with a real Nix store path, purely a simulation so
/// downstream string operations on the interpolated text have the expected
/// shape. Unlike `toString`/`dirOf`/etc, which keep working on the real
/// normalized path text, this coercion is specific to the `${...}`
/// interpolation surface.
fn px_fake_store_path_for(p: &str) -> String {
    let hex = px_sha256_hex(sha_utf8_bytes(&format!("path:{}", p)));
    format!(
        "/nix/store/{}-{}",
        px_hex_prefix32(&hex),
        px_path_basename(p)
    )
}

/// Walk a deep-forced derivation input and report whether a function value
/// occurs anywhere inside it (functions cannot be part of a derivation
/// attrset). A context-bearing string is a leaf value here, not a real
/// attrset (`px_is_real_attrset` excludes it), so it never recurses into
/// its own tagged fields.
fn px_derivation_uncoercible(v: &PxVal) -> bool {
    match v {
        PxVal::Closure { .. } | PxVal::Builtin { .. } => true,
        PxVal::List(items) => {
            let mut i = 0usize;
            while i < items.len() {
                if px_derivation_uncoercible(&items[i]) {
                    return true;
                }
                i += 1;
            }
            false
        }
        PxVal::Attrs(fields) if px_is_real_attrset(v) => {
            let mut i = 0usize;
            while i < fields.len() {
                if px_derivation_uncoercible(&fields[i].1) {
                    return true;
                }
                i += 1;
            }
            false
        }
        _ => false,
    }
}

/// Deterministic PSEUDO drvPath and per-output store paths for a
/// deep-forced derivation input. Non-"out" outputs get the Nix-style
/// "-<output>" name suffix (oracle: /nix/store/<h>-t vs
/// /nix/store/<h>-t-dev). `px_derivation_uncoercible` has already ruled out
/// functions by the time this runs (see `px_derivation_core`), so
/// `px_to_json`'s function-rejection branch cannot fire here.
fn px_derivation_paths(
    forced: &PxVal,
    name: &str,
    outputs: &Vec<String>,
) -> Result<(String, Vec<(String, String)>), String> {
    let canonical = px_to_json(forced)?;
    let drv_hex = px_sha256_hex(sha_utf8_bytes(&format!("drv:{}", canonical)));
    let drv_path = format!("/nix/store/{}-{}.drv", px_hex_prefix32(&drv_hex), name);
    let mut out_paths = Vec::new();
    let mut i = 0usize;
    while i < outputs.len() {
        let o = &outputs[i];
        let out_hex = px_sha256_hex(sha_utf8_bytes(&format!("out:{}:{}", o, canonical)));
        let suffix = if o == "out" {
            String::new()
        } else {
            format!("-{}", o)
        };
        out_paths.push((
            o.clone(),
            format!("/nix/store/{}-{}{}", px_hex_prefix32(&out_hex), name, suffix),
        ));
        i += 1;
    }
    Ok((drv_path, out_paths))
}

/// Validated + realized derivation input: `(forced_attrs, name, outputs,
/// drv_path, out_paths)`. Validates name/system/builder required, name a
/// plain string, no function anywhere in the (deep-forced) attrs, outputs
/// defaulting to `["out"]` and otherwise a non-empty vector of distinct
/// strings — oracle-pinned, matches pnix-clj's/pnix-cljs's
/// `derivation-core`.
fn px_derivation_core(
    builtin_name: &str,
    attrs: &PxVal,
) -> Result<(PxVal, String, Vec<String>, String, Vec<(String, String)>), String> {
    if !px_is_real_attrset(attrs) {
        return Err(format!(
            "px: {}: argument must be an attrset, got {}",
            builtin_name,
            px_kind(attrs)
        ));
    }
    let forced = px_force_deep(attrs)?;
    let fields = match &forced {
        PxVal::Attrs(f) => f.clone(),
        _ => return Err(format!("px: {}: argument must be an attrset", builtin_name)),
    };
    let required = ["name", "system", "builder"];
    let mut ri = 0usize;
    while ri < required.len() {
        if px_attrs_find(fields.as_ref(), required[ri]).is_none() {
            return Err(format!(
                "px: {}: missing required attribute '{}'",
                builtin_name, required[ri]
            ));
        }
        ri += 1;
    }
    let name = match px_attrs_find(fields.as_ref(), "name") {
        Some(PxVal::Str(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "px: {}: name must be a plain string, got {}",
                builtin_name,
                px_kind(other)
            ))
        }
        None => return Err(format!("px: {}: missing required attribute 'name'", builtin_name)),
    };
    if px_derivation_uncoercible(&forced) {
        return Err(format!(
            "px: {}: derivation attrs cannot contain a function",
            builtin_name
        ));
    }
    let outputs: Vec<String> = match px_attrs_find(fields.as_ref(), "outputs") {
        None => vec![String::from("out")],
        Some(PxVal::List(items)) => {
            let mut out = Vec::new();
            let mut i = 0usize;
            while i < items.len() {
                match &items[i] {
                    PxVal::Str(s) => out.push(s.clone()),
                    other => {
                        return Err(format!(
                            "px: {}: outputs must be strings, got {}",
                            builtin_name,
                            px_kind(other)
                        ))
                    }
                }
                i += 1;
            }
            out
        }
        Some(other) => {
            return Err(format!(
                "px: {}: outputs must be a list, got {}",
                builtin_name,
                px_kind(other)
            ))
        }
    };
    if outputs.is_empty() {
        return Err(format!("px: {}: outputs must be non-empty", builtin_name));
    }
    let mut oi = 0usize;
    while oi < outputs.len() {
        let mut oj = oi + 1;
        while oj < outputs.len() {
            if outputs[oi] == outputs[oj] {
                return Err(format!("px: {}: outputs must be distinct", builtin_name));
            }
            oj += 1;
        }
        oi += 1;
    }
    let (drv_path, out_paths) = px_derivation_paths(&forced, &name, &outputs)?;
    Ok((forced, name, outputs, drv_path, out_paths))
}

/// Set (overwrite if present, else append) a key in a field-list accumulator
/// — Nix `assoc`/`//`-style "last write wins" semantics for the reserved
/// `type`/`drvPath`/`outPath`/`outputName`/`<output-name>` keys `derivation`
/// adds on top of the user's own input attrs (oracle: pnix-clj does this
/// via plain Clojure `assoc`, which silently overwrites; `px_attrs`'s
/// sorted-unique invariant needs the explicit overwrite instead).
fn px_fields_set(fields: &mut Vec<(String, PxVal)>, key: String, value: PxVal) {
    // Rebuild rather than assign through `fields[i].1 = ..` — indexed
    // tuple-field assignment does not typeck in rs-meta's interpreted
    // subset (substrate-check caught it). `px_attrs` re-sorts afterward, so
    // losing the removed entry's original position here is harmless.
    let mut i = 0usize;
    let mut found = false;
    while i < fields.len() {
        if fields[i].0 == key {
            found = true;
            break;
        }
        i += 1;
    }
    if found {
        fields.remove(i);
    }
    fields.push((key, value));
}

/// Look up an output's store path in a `px_derivation_core` result's
/// `out_paths` list (linear scan; there are at most a handful of outputs).
fn px_out_path_for<'a>(out_paths: &'a Vec<(String, String)>, output: &str) -> &'a str {
    let mut i = 0usize;
    while i < out_paths.len() {
        if out_paths[i].0 == output {
            return &out_paths[i].1;
        }
        i += 1;
    }
    ""
}

// ---- builtins -----------------------------------------------------------------

pub fn px_builtin_names() -> Vec<&'static str> {
    vec![
        "toString",
        "stringLength",
        "concatStringsSep",
        "substring",
        "length",
        "map",
        "filter",
        "all",
        "any",
        "isFunction",
        "isNull",
        "isFloat",
        "typeOf",
        "baseNameOf",
        "dirOf",
        "abort",
        "foldl",
        "genList",
        "foldl'",
        "attrNames",
        "hasAttr",
        "sort",
        "head",
        "tail",
        "elemAt",
        "elem",
        "listToAttrs",
        "removeAttrs",
        "replaceStrings",
        "getAttr",
        "isAttrs",
        "isInt",
        "isBool",
        "isString",
        "isList",
        "toJSON",
        "fromJSON",
        "throw",
        "deepSeq",
        "addErrorContext",
        "hashString",
        "concatMap",
        "concatLists",
        "match",
        "split",
        "sin",
        "cos",
        "tan",
        "sqrt",
        "exp",
        "ln",
        "log",
        "abs",
        "ceil",
        "floor",
        "pow",
        "max",
        "min",
        "mod",
        "functionArgs",
        // Runtime-gap closure (2026-07-09): pure Nix builtins pnix-rs lacked
        // while clj/hy had them. Each oracle-pinned against nix 2.34.7 before
        // implementation (see px_builtin_exec).
        "add",
        "sub",
        "mul",
        "div",
        "lessThan",
        "bitAnd",
        "bitOr",
        "bitXor",
        "attrValues",
        "mapAttrs",
        "catAttrs",
        "intersectAttrs",
        "zipAttrsWith",
        "groupBy",
        "partition",
        "seq",
        "splitVersion",
        "compareVersions",
        // Nix-compatible builtin-surface tranche (proposal 0010).
        "break",
        "parseDrvName",
        "toPath",
        "unsafeDiscardOutputDependency",
        "unsafeDiscardStringContext",
        // String-context builtins (proposal 0006 slice): pure-simulation
        // tracking of Nix's string context, ported from pnix-clj/pnix-clr's
        // tagged-Attrs design (see the "string context" section above).
        "hasContext",
        "getContext",
        "appendContext",
        "derivation",
        "derivationStrict",
        "tryEval",
        "isPath",
        // README surface: missing Nix builtins + pure helpers shared with lib
        "trace",
        "toXML",
        "toFile",
        "readFile",
        "readDir",
        "pathExists",
        "fetchurl",
        "fetchTarball",
        "fetchGit",
        "last",
        "init",
        "flatten",
        "foldr",
        // Also on builtins (not only lib) — README tests use builtins.*
        "getAttrFromPath",
        "hasAttrByPath",
        "attrByPath",
        "getAttrFromPathOr",
        "filterAttrs",
        "filterAttrsRecursive",
        "mapAttrsRecursive",
        "concatMapStringsSep",
        "removePrefix",
        "removeSuffix",
        "hasPrefix",
        "hasSuffix",
        "splitString",
        "toLower",
        "toUpper",
        "boolToString",
        "implies",
        "optional",
        "optionals",
        "optionalAttrs",
        "when",
        "id",
        "const",
        "flip",
        "pipe",
        "fix",
        "range",
        "sum",
        "product",
        "recursiveUpdate",
        "updateManyAttrs",
        "getName",
        "getVersion",
        "unique",
        "intersectLists",
        "subtractLists",
        "zipLists",
        "zipListsWith",
        "warn",
        "assertMsg",
        // Cross-host consensus tranche (2026-08-19): builtins present across
        // >=3 of the 4 reference hosts (pnix-clj/pnix-clr/pnix-cljs/pnix-hy)
        // but missing from pnix-rs, oracle-pinned against those hosts before
        // implementation (see px_builtin_exec). `mapAttrsToList`/`zipAttrs`
        // already existed as dispatch targets (reachable only via lib before
        // now); this tranche makes them public builtins.* names too.
        // `pnixMounts` is still skipped (not a Nix builtin; host-local).
        // `unsafeGetAttrPos` is implemented (hy/Nix {file;line;column}).
        "unsafeGetAttrPos",
        "cons",
        "append",
        "drop",
        "take",
        "find",
        "findFirst",
        "reverseList",
        "replicate",
        "zip",
        "zipAttrs",
        "keys",
        "values",
        "mapAttrsToList",
        "merge",
        "genAttrs",
        "foldlAttrs",
        "genericClosure",
        "nameValuePair",
        "concatStrings",
        "concatMapStrings",
        "stringToCharacters",
        "hasInfix",
        "optionalString",
        "imap0",
        "imap1",
        "toInt",
        "placeholder",
        "storePath",
        "getEnv",
        "and",
        "or",
        "not",
        "eq",
        "lt",
        "le",
        "gt",
        "ge",
        "neg",
        "get",
        "set",
        // Held-math un-hold + new math additions (2026-08-20): sin/cos/tan/
        // sqrt/exp/ln/log/abs/pow/mod above were registered names but always
        // errored at call time ("held (B1 numeric model undecided)"); the
        // other 4 hosts (clj/clr/cljs/hy) all had working implementations,
        // so the hold is lifted (pure-arithmetic implementations — see the
        // block comment above `px_pow2_i64` for why rs-meta's interpreted
        // subset rules out std f64 methods). `atan2` and `mapAttrs'` are new
        // additions, oracle-pinned against pnix-hy and pnix-clj respectively.
        "atan2",
        "mapAttrs'",
    ]
}

/// Every key exposed by the public `builtins` attrset. Callable names stay in
/// `px_builtin_names`; this adds value-valued constants and the recursive self
/// field so the static purity walker recognizes the same surface as eval.
/// `storeDir` here is inert evaluator metadata (`"/nix/store"`), not execution
/// of the protocol's separately capability-gated `store-dir` effect request.
pub fn px_builtin_public_names() -> Vec<&'static str> {
    let mut names = px_builtin_names();
    names.push("true");
    names.push("false");
    names.push("null");
    names.push("langVersion");
    names.push("nixVersion");
    names.push("storeDir");
    names.push("builtins");
    names
}

fn px_builtins_attrset() -> PxVal {
    let mut fields = Vec::new();
    for name in px_builtin_names() {
        fields.push((
            String::from(name),
            PxVal::Builtin {
                name: String::from(name),
                args: Vec::new(),
            },
        ));
    }
    // These are VALUES in Nix's builtins attrset, not zero-argument callables.
    fields.push((String::from("true"), PxVal::Bool(true)));
    fields.push((String::from("false"), PxVal::Bool(false)));
    fields.push((String::from("null"), PxVal::Null));
    fields.push((String::from("langVersion"), PxVal::Int(6)));
    fields.push((
        String::from("nixVersion"),
        PxVal::Str(String::from("2.18.0-pnix")),
    ));
    fields.push((
        String::from("storeDir"),
        PxVal::Str(String::from("/nix/store")),
    ));
    // Existing call-by-need machinery gives the self field a finite
    // construction: it is forced only when selected.
    fields.push((
        String::from("builtins"),
        px_thunk(
            PxExpr::Var(String::from("builtins")),
            Rc::new(Vec::new()),
        ),
    ));
    px_attrs(fields)
}

/// Pure `lib` attrset: aliases of pure builtins plus helpers used by the
/// README surface. Nested `lib.attrsets` holds the attrset submodule.
fn px_lib_attrset() -> PxVal {
    let mut fields = Vec::new();
    // Alias pure builtins under lib.*
    let aliases = [
        "attrNames",
        "attrValues",
        "hasAttr",
        "getAttr",
        "mapAttrs",
        "listToAttrs",
        "length",
        "head",
        "tail",
        "elem",
        "concatLists",
        "concatStringsSep",
        "concatMap",
        "foldl",
        "genList",
        "partition",
        "throw",
        "abort",
        "toString",
        "typeOf",
        "isAttrs",
        "isInt",
        "isBool",
        "isString",
        "isList",
        "isFunction",
        "isNull",
        "trace",
        "last",
        "init",
        "flatten",
        "foldr",
        "min",
        "max",
    ];
    for name in aliases.iter() {
        fields.push((
            String::from(*name),
            PxVal::Builtin {
                name: String::from(*name),
                args: Vec::new(),
            },
        ));
    }
    // lib-only helpers (dispatched by name in px_builtin_exec)
    let extras = [
        "getAttrFromPath",
        "filterAttrs",
        "concatMapStringsSep",
        "removePrefix",
        "removeSuffix",
        "hasPrefix",
        "hasSuffix",
        "splitString",
        "toLower",
        "toUpper",
        "boolToString",
        "implies",
        "optional",
        "optionals",
        "optionalAttrs",
        "when",
        "id",
        "const",
        "flip",
        "pipe",
        "fix",
        "range",
        "sum",
        "product",
        "recursiveUpdate",
        "updateManyAttrs",
        "attrByPath",
        "getName",
        "getVersion",
        "getAttrFromPathOr",
        "hasAttrByPath",
        "filterAttrsRecursive",
        "mapAttrsRecursive",
        "unique",
        "intersectLists",
        "subtractLists",
        "zipLists",
        "zipListsWith",
        "warn",
        "assertMsg",
    ];
    for name in extras.iter() {
        fields.push((
            String::from(*name),
            PxVal::Builtin {
                name: String::from(*name),
                args: Vec::new(),
            },
        ));
    }
    // lib.assert — keyword-named attr; select allows KwAssert as "assert"
    fields.push((
        String::from("assert"),
        PxVal::Builtin {
            name: String::from("assertMsg"),
            args: Vec::new(),
        },
    ));
    // lib.attrsets submodule
    let mut aset = Vec::new();
    for name in ["isAttrs", "mapAttrs", "filterAttrs", "attrNames", "attrValues", "hasAttr", "getAttr", "listToAttrs", "catAttrs", "zipAttrsWith", "intersectAttrs"].iter() {
        aset.push((
            String::from(*name),
            PxVal::Builtin {
                name: String::from(*name),
                args: Vec::new(),
            },
        ));
    }
    aset.push((
        String::from("mapAttrsToList"),
        PxVal::Builtin {
            name: String::from("mapAttrsToList"),
            args: Vec::new(),
        },
    ));
    aset.push((
        String::from("zipAttrs"),
        PxVal::Builtin {
            name: String::from("zipAttrs"),
            args: Vec::new(),
        },
    ));
    fields.push((String::from("attrsets"), px_attrs(aset)));
    px_attrs(fields)
}

fn px_builtin_arity(name: &str) -> usize {
    if name == "toString"
        || name == "stringLength"
        || name == "length"
        || name == "attrNames"
        || name == "head"
        || name == "tail"
        || name == "listToAttrs"
        || name == "isAttrs"
        || name == "isInt"
        || name == "isBool"
        || name == "isString"
        || name == "isList"
        || name == "toJSON"
        || name == "fromJSON"
        || name == "attrValues"
        || name == "splitVersion"
        || name == "break"
        || name == "parseDrvName"
        || name == "toPath"
        || name == "unsafeDiscardOutputDependency"
        || name == "unsafeDiscardStringContext"
        || name == "tryEval"
        || name == "isPath"
        || name == "toXML"
        || name == "readFile"
        || name == "readDir"
        || name == "pathExists"
        || name == "fetchurl"
        || name == "fetchTarball"
        || name == "fetchGit"
        || name == "last"
        || name == "init"
        || name == "flatten"
        || name == "id"
        || name == "sum"
        || name == "product"
        || name == "unique"
        || name == "fix"
        || name == "toLower"
        || name == "toUpper"
        || name == "boolToString"
        || name == "getName"
        || name == "getVersion"
        || name == "zipAttrs"
        // Cross-host consensus tranche (2026-08-19), arity 1.
        || name == "reverseList"
        || name == "keys"
        || name == "values"
        || name == "stringToCharacters"
        || name == "concatStrings"
        || name == "toInt"
        || name == "placeholder"
        || name == "storePath"
        || name == "getEnv"
        || name == "not"
        || name == "neg"
        || name == "genericClosure"
        || name == "hasContext"
        || name == "getContext"
        || name == "derivation"
        || name == "derivationStrict"
    {
        1
    } else if name == "concatLists" {
        1
    } else if name == "isNull" || name == "isFloat" || name == "typeOf"
        || name == "baseNameOf" || name == "dirOf" || name == "abort"
    {
        1
    } else if name == "hashString" || name == "concatMap"
        || name == "trace" || name == "toFile" || name == "warn"
        || name == "removePrefix" || name == "removeSuffix"
        || name == "hasPrefix" || name == "hasSuffix"
        || name == "splitString" || name == "implies"
        || name == "optional" || name == "optionals" || name == "optionalAttrs"
        || name == "when" || name == "const" || name == "pipe"
        || name == "range" || name == "recursiveUpdate"
        || name == "intersectLists" || name == "subtractLists"
        || name == "zipLists" || name == "hasAttrByPath"
        || name == "filterAttrs" || name == "getAttrFromPath"
        || name == "filterAttrsRecursive" || name == "mapAttrsRecursive"
        || name == "mapAttrsToList" || name == "assertMsg"
        || name == "updateManyAttrs"
        // Cross-host consensus tranche (2026-08-19), arity 2.
        || name == "cons"
        || name == "append"
        || name == "drop"
        || name == "take"
        || name == "find"
        || name == "zip"
        || name == "merge"
        || name == "genAttrs"
        || name == "nameValuePair"
        || name == "concatMapStrings"
        || name == "hasInfix"
        || name == "optionalString"
        || name == "imap0"
        || name == "imap1"
        || name == "and"
        || name == "or"
        || name == "eq"
        || name == "lt"
        || name == "le"
        || name == "gt"
        || name == "ge"
        || name == "get"
        || name == "appendContext"
        || name == "unsafeGetAttrPos"
    {
        2
    } else if name == "replaceStrings" || name == "foldl" || name == "foldr"
        || name == "substring" || name == "foldl'"
        || name == "concatMapStringsSep" || name == "flip"
        || name == "attrByPath" || name == "getAttrFromPathOr"
        || name == "zipListsWith"
        // Cross-host consensus tranche (2026-08-19), arity 3.
        || name == "findFirst"
        || name == "foldlAttrs"
        || name == "set"
    {
        3
    } else if name == "isFunction" || name == "throw"
        || name == "sin" || name == "cos" || name == "tan" || name == "sqrt"
        || name == "exp" || name == "ln" || name == "log" || name == "abs"
        || name == "ceil" || name == "floor" || name == "functionArgs"
    {
        1
    } else {
        2
    }
}


/// Nix `builtins.toString` coercion (oracle-pinned 2026-07-08): int as-is,
/// string as-is, null == "", true == "1", false == "", list == space-joined
/// coerced elements. Finite floats use Nix's fixed six-decimal rendering;
/// lambda/attrs(no outPath) error like Nix.
fn px_to_string_coerce(v: &PxVal) -> Result<PxVal, String> {
    let v = px_force(v)?;
    match &v {
        PxVal::Int(n) => Ok(PxVal::Str(format!("{}", n))),
        PxVal::Str(s) => Ok(PxVal::Str(s.clone())),
        PxVal::Path(p) => Ok(PxVal::Str(p.clone())),
        PxVal::Null => Ok(PxVal::Str(String::new())),
        PxVal::Bool(b) => {
            if *b {
                Ok(PxVal::Str(String::from("1")))
            } else {
                Ok(PxVal::Str(String::new()))
            }
        }
        PxVal::List(items) => {
            let mut out = String::new();
            let mut first = true;
            for item in items.iter() {
                let sv = px_to_string_coerce(item)?;
                if let PxVal::Str(txt) = sv {
                    if !first {
                        out.push(' ');
                    }
                    out.push_str(&txt);
                    first = false;
                }
            }
            Ok(PxVal::Str(out))
        }
        PxVal::Float(f) => {
            let x = *f;
            if x - x != 0.0 {
                if x != x {
                    return Ok(PxVal::Str(String::from("nan")));
                }
                if x > 0.0 {
                    return Ok(PxVal::Str(String::from("inf")));
                }
                return Ok(PxVal::Str(String::from("-inf")));
            }
            // Keep an IEEE negative zero produced by multiplication/division.
            // Parser-level unary minus is desugared to `0 - operand`, so the
            // literal `-0.0` has already become positive zero, matching Nix.
            Ok(PxVal::Str(format!("{:.6}", x)))
        }
        other => Err(format!("px: toString unsupported for {}", px_kind(other))),
    }
}

/// Context-aware `toString` coercion — the `toString` BUILTIN's own full
/// implementation, distinct from `px_to_string_coerce` above (which stays
/// strict/context-free and backs builtins NOT on the context-aware
/// allowlist, e.g. `concatMapStringsSep`; a ctx-string reaching THOSE stays
/// a hard error, matching pnix-clj's `pnix-to-string` vs `:toString`'s
/// separate `coerce` split). Strings keep their context; list elements'
/// contexts are collected into `ctx` (oracle: `toString [ ctx "b" ]`
/// carries the context).
fn px_to_string_coerce_ctx(v: &PxVal, ctx: &mut Vec<String>) -> Result<String, String> {
    let v = px_force(v)?;
    match &v {
        PxVal::Int(n) => Ok(format!("{}", n)),
        PxVal::Str(s) => Ok(s.clone()),
        // toString on a path returns its own (normalized) text -- unlike
        // `${...}` interpolation, it does NOT fabricate a store path.
        PxVal::Path(p) => Ok(p.clone()),
        _ if px_is_ctx_string(&v) => {
            ctx.extend(px_string_like_context(&v));
            Ok(px_string_like_content_or_empty(&v))
        }
        PxVal::Null => Ok(String::new()),
        PxVal::Bool(b) => {
            if *b {
                Ok(String::from("1"))
            } else {
                Ok(String::new())
            }
        }
        PxVal::List(items) => {
            let mut out = String::new();
            let mut first = true;
            for item in items.iter() {
                let txt = px_to_string_coerce_ctx(item, ctx)?;
                if !first {
                    out.push(' ');
                }
                out.push_str(&txt);
                first = false;
            }
            Ok(out)
        }
        PxVal::Float(f) => {
            let x = *f;
            if x - x != 0.0 {
                if x != x {
                    return Ok(String::from("nan"));
                }
                if x > 0.0 {
                    return Ok(String::from("inf"));
                }
                return Ok(String::from("-inf"));
            }
            Ok(format!("{:.6}", x))
        }
        other => Err(format!("px: toString unsupported for {}", px_kind(other))),
    }
}

// `{a, b ? d}: body` (parse_pattern_lambda) desugars to a plain
// `Closure { param: __pat_arg, body: LetIn { bindings: [(a, getAttr "a"
// __pat_arg), (b, if hasAttr "b" __pat_arg then getAttr "b" __pat_arg else
// d)], .. } }` with no separate marker retained. functionArgs recovers the
// formal/has-default map by recognizing that exact shape on the closure body
// rather than adding a new AST/Value field (keeps the fix inside px.rs's
// rs-meta-evaluable subset and out of the shared Closure representation).
fn px_is_pattern_getattr(expr: &PxExpr, bname: &str, param: &str) -> bool {
    if let PxExpr::Apply { func, arg } = expr {
        if let PxExpr::Var(v) = arg.as_ref() {
            if v == param {
                if let PxExpr::Apply { func: gfunc, arg: garg } = func.as_ref() {
                    if let PxExpr::Select { base, name } = gfunc.as_ref() {
                        if name == "getAttr" {
                            if let PxExpr::Var(b) = base.as_ref() {
                                if b == "builtins" {
                                    if let PxExpr::Str(parts) = garg.as_ref() {
                                        if let Some(PxStrPart::Lit(s)) = parts.first() {
                                            return s == bname;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn px_is_pattern_hasattr(expr: &PxExpr, bname: &str, param: &str) -> bool {
    if let PxExpr::Apply { func, arg } = expr {
        if let PxExpr::Var(v) = arg.as_ref() {
            if v == param {
                if let PxExpr::Apply { func: gfunc, arg: garg } = func.as_ref() {
                    if let PxExpr::Select { base, name } = gfunc.as_ref() {
                        if name == "hasAttr" {
                            if let PxExpr::Var(b) = base.as_ref() {
                                if b == "builtins" {
                                    if let PxExpr::Str(parts) = garg.as_ref() {
                                        if let Some(PxStrPart::Lit(s)) = parts.first() {
                                            return s == bname;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn px_pattern_formal_has_default(bname: &str, value: &PxExpr, param: &str) -> Option<bool> {
    if px_is_pattern_getattr(value, bname, param) {
        return Some(false);
    }
    if let PxExpr::If { cond, then_e, .. } = value {
        if px_is_pattern_hasattr(cond, bname, param) && px_is_pattern_getattr(then_e, bname, param) {
            return Some(true);
        }
    }
    None
}

// parse_pattern_lambda's real desugared body is
// `if <arity guard> then <bindings-let> else throw "attrset pattern:
// argument mismatch (missing required or unexpected key)"` — the throw's
// message is a marker unique to that desugar path, so unwrap through it
// rather than re-deriving the guard expression (which also depends on the
// formal names and would be a second, more fragile shape to match).
fn px_is_pattern_mismatch_throw(expr: &PxExpr) -> bool {
    if let PxExpr::Apply { func, arg } = expr {
        if let PxExpr::Var(v) = func.as_ref() {
            if v == "throw" {
                if let PxExpr::Str(parts) = arg.as_ref() {
                    if let Some(PxStrPart::Lit(s)) = parts.first() {
                        return s
                            == "attrset pattern: argument mismatch (missing required or unexpected key)";
                    }
                }
            }
        }
    }
    false
}

fn px_function_args(param: &str, body: &PxExpr) -> PxVal {
    let inner = match body {
        PxExpr::If { then_e, else_e, .. } if px_is_pattern_mismatch_throw(else_e) => {
            then_e.as_ref()
        }
        _ => return px_attrs(Vec::new()),
    };
    if let PxExpr::LetIn { bindings, .. } = inner {
        if !bindings.is_empty() {
            let mut fields = Vec::new();
            let mut all_matched = true;
            for (bname, bexpr) in bindings {
                match px_pattern_formal_has_default(bname, bexpr, param) {
                    Some(has_default) => fields.push((bname.clone(), PxVal::Bool(has_default))),
                    None => {
                        all_matched = false;
                    }
                }
            }
            if all_matched {
                return px_attrs(fields);
            }
        }
    }
    px_attrs(Vec::new())
}

fn px_builtin_exec(name: &str, args: &Vec<PxVal>) -> Result<PxVal, String> {
    // hashString validates the selector before forcing the payload. Nix's
    // error order is observable when the second argument is a failing thunk.
    if name == "hashString" {
        let algorithm = px_force(&args[0])?;
        let algo = match &algorithm {
            PxVal::Str(s) => s,
            PxVal::Bytes(raw) => {
                return Err(format!(
                    "px: hashString: unsupported raw-byte algorithm ({} bytes)",
                    raw.len()
                ))
            }
            other => {
                return Err(format!(
                    "px: hashString algorithm must be a string, got {}",
                    px_kind(other)
                ))
            }
        };
        if algo != "md5" && algo != "sha1" && algo != "sha256" && algo != "sha512" {
            return Err(format!("px: hashString: unsupported algorithm {}", algo));
        }
        let payload = px_force(&args[1])?;
        let bytes = match &payload {
            PxVal::Str(s) => sha_utf8_bytes(s),
            PxVal::Bytes(raw) => {
                let mut out = Vec::new();
                for byte in raw.iter() {
                    out.push(*byte as u64);
                }
                out
            }
            // Oracle: hashString consumes UTF-8 bytes and returns lowercase
            // hex. A contextful DATA string is accepted but the digest is
            // context-free (the algorithm selector above may not carry
            // context — already rejected).
            _ if px_is_ctx_string(&payload) => sha_utf8_bytes(&px_string_like_content_or_empty(&payload)),
            other => {
                return Err(format!(
                    "px: hashString payload must be string-like, got {}",
                    px_kind(other)
                ))
            }
        };
        if algo == "md5" {
            return Ok(PxVal::Str(px_md5_hex(bytes)));
        }
        if algo == "sha1" {
            return Ok(PxVal::Str(px_sha1_hex(bytes)));
        }
        if algo == "sha256" {
            return Ok(PxVal::Str(px_sha256_hex(bytes)));
        }
        return Ok(PxVal::Str(px_sha512_hex(bytes)));
    }
    // `elem` forces the haystack first and the needle only while comparing an
    // actual element. Keeping the original needle thunk also preserves Nix's
    // nested shared-value identity rule (`let f = x: x; in elem f [ f ]`).
    if name == "elem" {
        let haystack = px_force(&args[1])?;
        return match &haystack {
            PxVal::List(items) => {
                for item in items.iter() {
                    if px_val_eq_nested(&args[0], item)? {
                        return Ok(PxVal::Bool(true));
                    }
                }
                Ok(PxVal::Bool(false))
            }
            other => Err(format!("px: elem expects a list, got {}", px_kind(other))),
        };
    }
    // Builtins are strict in their arguments (Nix forces them before
    // inspecting). A field pulled out of a lazy attrset arrives as a Thunk, so
    // force every argument to WHNF once, here, instead of at ~200 inspection
    // sites. Field values INSIDE a forced attrset stay lazy.
    let forced: Vec<PxVal> = {
        let mut out = Vec::new();
        for a in args.iter() {
            out.push(px_force(a)?);
        }
        out
    };
    let args = &forced;
    // Fail-closed string-context frontier (single chokepoint, right before
    // dispatch): a contextful string reaching a builtin the allowlist does
    // not recognize is a hard error rather than a silently dropped/mangled
    // context. Scanning the FORCED args (top level only — list ELEMENTS
    // inside a forced list arg are still individually lazy, since forcing
    // an arg to WHNF does not force its contents) reproduces the oracle's
    // exact shallow-scan behavior: a top-level scalar contextful argument is
    // always caught, while one nested inside an unforced list element
    // (e.g. passed to `sort`/`filter`) is not — empirically confirmed to
    // match the real pnix-clj oracle, not merely the pnix-cljs port (see
    // `px_ctx_string_in_args`). `hashString`/`elem` above return before this
    // point, but both are allowlisted, so the gate could never reject them
    // anyway.
    if px_ctx_string_in_args(args) && !px_context_aware_builtin(name) {
        return Err(format!(
            "px: {}: string-context-frontier: this builtin does not accept a contextful string argument yet",
            name
        ));
    }
    if name == "break" {
        Ok(args[0].clone())
    } else if name == "parseDrvName" {
        match &args[0] {
            PxVal::Str(s) => {
                let (drv_name, version) = px_parse_drv_name(s);
                Ok(px_attrs(vec![
                    (String::from("name"), PxVal::Str(drv_name)),
                    (String::from("version"), PxVal::Str(version)),
                ]))
            }
            other => Err(format!(
                "px: parseDrvName expects a string, got {}",
                px_kind(other)
            )),
        }
    } else if name == "toPath" {
        match &args[0] {
            // toPath returns a real Path value now (real Nix: `builtins.toPath
            // : string -> path`); context cannot ride on a Path, so a
            // context-bearing argument is rejected rather than silently
            // dropping its dependency marker (same stance the `+` operator
            // takes for path + context-bearing string below).
            PxVal::Str(s) => Ok(PxVal::Path(px_to_path_string(s)?)),
            PxVal::Path(p) => Ok(PxVal::Path(p.clone())),
            _ if px_is_ctx_string(&args[0]) => {
                let context = px_string_like_context(&args[0]);
                if !context.is_empty() {
                    return Err(format!(
                        "px: toPath: string has context, refusing to silently drop it \
                         (use builtins.unsafeDiscardStringContext first)"
                    ));
                }
                let content = px_string_like_content_or_empty(&args[0]);
                Ok(PxVal::Path(px_to_path_string(&content)?))
            }
            other => Err(format!(
                "px: toPath expects a string, got {}",
                px_kind(other)
            )),
        }
    } else if name == "unsafeDiscardStringContext" {
        match &args[0] {
            PxVal::Str(s) => Ok(PxVal::Str(s.clone())),
            _ if px_is_ctx_string(&args[0]) => {
                Ok(PxVal::Str(px_string_like_content_or_empty(&args[0])))
            }
            other => Err(format!(
                "px: {} expects a string, got {}",
                name,
                px_kind(other)
            )),
        }
    } else if name == "unsafeDiscardOutputDependency" {
        match &args[0] {
            PxVal::Str(s) => Ok(PxVal::Str(s.clone())),
            // Keep the content and every PATH-kind context element; drop
            // only output-dependency elements ("!o!<drv>") and the drvPath
            // allOutputs element ("=<drv>") — oracle-pinned to mirror
            // pnix-clj's `:unsafeDiscardOutputDependency`.
            _ if px_is_ctx_string(&args[0]) => {
                let content = px_string_like_content_or_empty(&args[0]);
                let mut kept = Vec::new();
                let ctx = px_string_like_context(&args[0]);
                let mut i = 0usize;
                while i < ctx.len() {
                    let e = &ctx[i];
                    if !e.starts_with("!") && !e.starts_with("=") {
                        kept.push(e.clone());
                    }
                    i += 1;
                }
                Ok(px_ctx_string(content, kept))
            }
            other => Err(format!(
                "px: {} expects a string, got {}",
                name,
                px_kind(other)
            )),
        }
    } else if name == "hasContext" {
        if px_is_string_like(&args[0]) {
            Ok(PxVal::Bool(px_is_ctx_string(&args[0])))
        } else {
            Err(format!(
                "px: hasContext expects a string, got {}",
                px_kind(&args[0])
            ))
        }
    } else if name == "getContext" {
        // Decode Nix-encoded context elements back into per-path info
        // attrsets, merging kinds on the same path (oracle: nix-instantiate
        // 2.34.7 — plain "<p>" -> { path = true; }, "!o!<drv>" -> { outputs
        // = [o..]; }, "=<drv>" -> { allOutputs = true; }; mixed kinds merge
        // on one key). Pure-simulation scope: WHICH dependency + which
        // kind, not the fuller real-Nix detail.
        if !px_is_string_like(&args[0]) {
            return Err(format!(
                "px: getContext expects a string, got {}",
                px_kind(&args[0])
            ));
        }
        let ctx = px_string_like_context(&args[0]);
        let mut acc: Vec<(String, bool, bool, Vec<String>)> = Vec::new();
        let mut i = 0usize;
        while i < ctx.len() {
            let e = &ctx[i];
            if e.starts_with("=") {
                let path = px_str_tail(e);
                let idx = px_getcontext_find_or_insert(&mut acc, &path);
                acc[idx].2 = true;
            } else if e.starts_with("!") {
                let rest = px_str_tail(e);
                match px_split_bang(&rest) {
                    Some((output, path)) => {
                        let idx = px_getcontext_find_or_insert(&mut acc, &path);
                        let mut already = false;
                        let mut k = 0usize;
                        while k < acc[idx].3.len() {
                            if acc[idx].3[k] == output {
                                already = true;
                            }
                            k += 1;
                        }
                        if !already {
                            // `.push()` needs a plain local mutable
                            // receiver in rs-meta's interpreted subset —
                            // indexing through `acc[idx].3` directly does
                            // not typeck there (substrate-check caught it)
                            // — so clone out, push, and write back instead.
                            let mut updated = acc[idx].3.clone();
                            updated.push(output);
                            acc[idx].3 = updated;
                        }
                    }
                    None => {
                        let idx = px_getcontext_find_or_insert(&mut acc, e);
                        acc[idx].1 = true;
                    }
                }
            } else {
                let idx = px_getcontext_find_or_insert(&mut acc, e);
                acc[idx].1 = true;
            }
            i += 1;
        }
        let mut result = Vec::new();
        let mut i = 0usize;
        while i < acc.len() {
            let (path, has_path, all_outputs, outputs) = &acc[i];
            let mut info = Vec::new();
            if *has_path {
                info.push((String::from("path"), PxVal::Bool(true)));
            }
            if *all_outputs {
                info.push((String::from("allOutputs"), PxVal::Bool(true)));
            }
            if !outputs.is_empty() {
                let sorted_outs = px_sort_strings(outputs.clone());
                let mut ov = Vec::new();
                let mut k = 0usize;
                while k < sorted_outs.len() {
                    ov.push(PxVal::Str(sorted_outs[k].clone()));
                    k += 1;
                }
                info.push((String::from("outputs"), px_list(ov)));
            }
            result.push((path.clone(), px_attrs(info)));
            i += 1;
        }
        Ok(px_attrs(result))
    } else if name == "appendContext" {
        // appendContext s ctxAttrs: interpret each key's info attrset into
        // Nix-encoded context elements — path=true -> "<p>", allOutputs=true
        // -> "=<p>", outputs=[o..] -> "!o!<p>". An EMPTY info attrset
        // contributes NOTHING (oracle-confirmed against real nix-instantiate:
        // hasContext (appendContext s { p = {}; }) is false). Real arg
        // order is (string, ctxAttrs) — string FIRST.
        if !px_is_string_like(&args[0]) {
            return Err(format!(
                "px: appendContext expects a string, got {}",
                px_kind(&args[0])
            ));
        }
        let ctx_attrs = px_force(&args[1])?;
        let info_map = match &ctx_attrs {
            PxVal::Attrs(f) if px_is_real_attrset(&ctx_attrs) => f.clone(),
            other => {
                return Err(format!(
                    "px: appendContext expects an attrset context, got {}",
                    px_kind(other)
                ))
            }
        };
        let mut extra: Vec<String> = Vec::new();
        let mut i = 0usize;
        while i < info_map.len() {
            let key = &info_map[i].0;
            let info = px_force(&info_map[i].1)?;
            let info_fields = match &info {
                PxVal::Attrs(f) if px_is_real_attrset(&info) => f.clone(),
                other => {
                    return Err(format!(
                        "px: appendContext: context info for '{}' must be an attrset, got {}",
                        key,
                        px_kind(other)
                    ))
                }
            };
            if let Some(v) = px_attrs_find(info_fields.as_ref(), "path") {
                if let PxVal::Bool(true) = px_force(v)? {
                    extra.push(key.clone());
                }
            }
            if let Some(v) = px_attrs_find(info_fields.as_ref(), "allOutputs") {
                if let PxVal::Bool(true) = px_force(v)? {
                    extra.push(format!("={}", key));
                }
            }
            if let Some(v) = px_attrs_find(info_fields.as_ref(), "outputs") {
                if let PxVal::List(outs) = px_force(v)? {
                    let mut j = 0usize;
                    while j < outs.len() {
                        if let PxVal::Str(o) = px_force(&outs[j])? {
                            extra.push(format!("!{}!{}", o, key));
                        }
                        j += 1;
                    }
                }
            }
            i += 1;
        }
        let content = px_string_like_content_or_empty(&args[0]);
        let mut full_ctx = px_string_like_context(&args[0]);
        full_ctx.extend(extra);
        Ok(px_ctx_string(content, full_ctx))
    } else if name == "isPath" {
        Ok(PxVal::Bool(matches!(&args[0], PxVal::Path(_))))
    } else if name == "toString" {
        let mut ctx: Vec<String> = Vec::new();
        let content = px_to_string_coerce_ctx(&args[0], &mut ctx)?;
        Ok(px_ctx_string(content, ctx))
    } else if name == "stringLength" {
        match &args[0] {
            // ★B4 DECIDED (owner 2026-07-09): the BYTE model — Nix counts
            // bytes ("å" == 2), by design (manual, Dolstra #770). Was
            // chars().count() (a real divergence the gate exposed).
            PxVal::Str(s) => Ok(PxVal::Int(s.len() as i64)),
            PxVal::Bytes(b) => Ok(PxVal::Int(b.len() as i64)),
            _ if px_is_ctx_string(&args[0]) => {
                Ok(PxVal::Int(px_string_like_content_or_empty(&args[0]).len() as i64))
            }
            other => Err(format!("px: stringLength expects a string, got {}", px_kind(other))),
        }
    } else if name == "concatStringsSep" {
        // Context-aware: contents join at the byte level exactly as before
        // (RAW-BYTE aware: per-byte fragments from e.g. a substring cut
        // reassemble; revalidate -> Str when valid UTF-8); the contexts of
        // the separator and every element union onto the result. A
        // context-free result stays a plain Str/Bytes as before.
        match px_item_bytes_and_ctx(&args[0]) {
            Some((sep_bytes, sep_ctx)) => {
              let mut ctx = sep_ctx;
              match &args[1] {
                PxVal::List(items) => {
                    let mut out: Vec<u8> = Vec::new();
                    let mut first = true;
                    for item in items.iter() {
                        let item = px_force(item)?;
                        match px_item_bytes_and_ctx(&item) {
                            Some((b, ic)) => {
                                if !first {
                                    for x in sep_bytes.iter() {
                                        out.push(*x);
                                    }
                                }
                                for x in b.iter() {
                                    out.push(*x);
                                }
                                ctx.extend(ic);
                                first = false;
                            }
                            None => {
                                return Err(format!(
                                    "px: concatStringsSep expects strings, got {}",
                                    px_kind(&item)
                                ))
                            }
                        }
                    }
                    match px_bytes_val(out) {
                        PxVal::Str(s) => Ok(px_ctx_string(s, ctx)),
                        other if ctx.is_empty() => Ok(other),
                        _ => Err(String::from(
                            "px: concatStringsSep: raw-byte result cannot carry string context",
                        )),
                    }
                }
                _ => Err(String::from("px: concatStringsSep expects (string, list)")),
              }
            }
            None => Err(String::from("px: concatStringsSep expects (string, list)")),
        }
    } else if name == "substring" {
        // Context-aware: Nix keeps the ENTIRE original context on a
        // substring (context is not sliced along with the content) —
        // oracle-pinned, matches pnix-clj's `:substring`.
        if px_is_ctx_string(&args[2]) {
            return match (&args[0], &args[1]) {
                (PxVal::Int(start), PxVal::Int(len)) => {
                    if *start < 0 {
                        return Err(String::from("px: substring: negative start"));
                    }
                    let orig_ctx = px_string_like_context(&args[2]);
                    let content = px_string_like_content_or_empty(&args[2]);
                    let bytes = px_str_bytes(&content);
                    let a = *start as usize;
                    if a >= bytes.len() {
                        return Ok(px_ctx_string(String::new(), orig_ctx));
                    }
                    let b0 = if *len < 0 { bytes.len() } else { a + (*len as usize) };
                    let b = if b0 > bytes.len() { bytes.len() } else { b0 };
                    let mut out: Vec<u8> = Vec::new();
                    let mut i = a;
                    while i < b {
                        out.push(bytes[i]);
                        i += 1;
                    }
                    match px_bytes_val(out) {
                        PxVal::Str(s) => Ok(px_ctx_string(s, orig_ctx)),
                        // A raw-byte cut of a CONTEXTFUL string: the byte
                        // value cannot carry context — held (matches
                        // pnix-clj's `:substring-raw-bytes-with-context`).
                        _ => Err(String::from(
                            "px: substring: raw-byte cut of a contextful string is not supported",
                        )),
                    }
                }
                _ => Err(String::from("px: substring expects (int, int, string)")),
            };
        }
        match (&args[0], &args[1], &args[2]) {
            (PxVal::Int(start), PxVal::Int(len), PxVal::Str(s)) => {
                if *start < 0 {
                    return Err(String::from("px: substring: negative start"));
                }
                // ★B4 BYTE model: byte offsets like Nix. A cut off a UTF-8
                // char boundary would need invalid-UTF-8 strings (Nix allows
                // them; Rust String cannot) — HELD fail-closed until/unless
                // the value model moves to raw bytes. Boundary-aligned cuts
                // (incl. whole hangul syllables) are exact.
                // RAW-BYTE track (2026-07-11): slice the UTF-8 bytes at any
                // offset; a cut off a char boundary yields a Bytes value
                // (Nix-permitted intermediate), revalidated on construction.
                let bytes = px_str_bytes(s);
                let a = *start as usize;
                if a >= bytes.len() {
                    return Ok(PxVal::Str(String::new()));
                }
                // A negative length means "to the end of the string" (same
                // clamp as the Bytes arm below), not an error.
                let b0 = if *len < 0 { bytes.len() } else { a + (*len as usize) };
                let b = if b0 > bytes.len() { bytes.len() } else { b0 };
                let mut out: Vec<u8> = Vec::new();
                let mut i = a;
                while i < b {
                    out.push(bytes[i]);
                    i += 1;
                }
                Ok(px_bytes_val(out))
            }
            (PxVal::Int(start), PxVal::Int(len), PxVal::Bytes(bs)) => {
                if *start < 0 {
                    return Err(String::from("px: substring: negative start"));
                }
                let a = *start as usize;
                if a >= bs.len() {
                    return Ok(PxVal::Str(String::new()));
                }
                let b0 = if *len < 0 { bs.len() } else { a + (*len as usize) };
                let b = if b0 > bs.len() { bs.len() } else { b0 };
                let mut out: Vec<u8> = Vec::new();
                let mut i = a;
                while i < b {
                    out.push(bs[i]);
                    i += 1;
                }
                Ok(px_bytes_val(out))
            }
            _ => Err(String::from("px: substring expects (int, int, string)")),
        }
    } else if name == "length" {
        match &args[0] {
            PxVal::List(items) => Ok(PxVal::Int(items.len() as i64)),
            other => Err(format!("px: length expects a list, got {}", px_kind(other))),
        }
    } else if name == "all" {
        // Nix builtins.all pred list — true iff pred is true for every element.
        match &args[1] {
            PxVal::List(items) => {
                for item in items.iter() {
                    let result = px_apply(&args[0], item.clone())?;
                    match px_force(&result)? {
                        PxVal::Bool(true) => {}
                        PxVal::Bool(false) => return Ok(PxVal::Bool(false)),
                        other => {
                            return Err(format!(
                                "px: all predicate must return bool, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                }
                Ok(PxVal::Bool(true))
            }
            other => Err(format!("px: all expects a list, got {}", px_kind(other))),
        }
    } else if name == "any" {
        // Nix builtins.any pred list — true iff pred is true for some element.
        match &args[1] {
            PxVal::List(items) => {
                for item in items.iter() {
                    let result = px_apply(&args[0], item.clone())?;
                    match px_force(&result)? {
                        PxVal::Bool(true) => return Ok(PxVal::Bool(true)),
                        PxVal::Bool(false) => {}
                        other => {
                            return Err(format!(
                                "px: any predicate must return bool, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                }
                Ok(PxVal::Bool(false))
            }
            other => Err(format!("px: any expects a list, got {}", px_kind(other))),
        }
    } else if name == "isFunction" {
        Ok(PxVal::Bool(matches!(
            &args[0],
            PxVal::Closure { .. } | PxVal::Builtin { .. }
        )))
    } else if name == "genList" {
        // Nix builtins.genList f n — [ (f 0) ... (f (n-1)) ].
        match &args[1] {
            PxVal::Int(n) => {
                if *n < 0 {
                    return Err(String::from("px: genList length must be non-negative"));
                }
                let mut out = Vec::new();
                let mut i = 0i64;
                while i < *n {
                    out.push(px_defer_apply(args[0].clone(), PxVal::Int(i)));
                    i += 1;
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: genList expects an int, got {}", px_kind(other))),
        }
    } else if name == "map" {
        match &args[1] {
            PxVal::List(items) => {
                let mut out = Vec::new();
                for item in items.iter() {
                    out.push(px_defer_apply(args[0].clone(), item.clone()));
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: map expects a list, got {}", px_kind(other))),
        }
    } else if name == "filter" {
        match &args[1] {
            PxVal::List(items) => {
                let mut out = Vec::new();
                for item in items.iter() {
                    let keep = px_force(&px_apply(&args[0], item.clone())?)?;
                    match keep {
                        PxVal::Bool(true) => out.push(item.clone()),
                        PxVal::Bool(false) => {}
                        other => {
                            return Err(format!(
                                "px: filter predicate must return bool, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: filter expects a list, got {}", px_kind(other))),
        }
    } else if name == "foldl'" {
        match &args[2] {
            PxVal::List(items) => {
                let mut acc = args[1].clone();
                for item in items.iter() {
                    let step = px_apply(&args[0], acc)?;
                    acc = px_force(&px_apply(&step, item.clone())?)?;
                }
                Ok(acc)
            }
            other => Err(format!("px: foldl' expects a list, got {}", px_kind(other))),
        }
    } else if name == "attrNames" {
        match &args[0] {
            PxVal::Attrs(fields) => {
                let mut names = Vec::new();
                for (k, _v) in fields.iter() {
                    if !px_is_attr_pos_key(k) {
                        names.push(k.clone());
                    }
                }
                let sorted = px_sort_strings(names);
                let mut out = Vec::new();
                for n in sorted {
                    out.push(PxVal::Str(n));
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: attrNames expects an attrset, got {}", px_kind(other))),
        }
    } else if name == "hasAttr" {
        match (&args[0], &args[1]) {
            (PxVal::Str(k), PxVal::Attrs(fields)) => Ok(PxVal::Bool(px_attrs_has(fields.as_ref(), k))),
            _ => Err(String::from("px: hasAttr expects (string, attrset)")),
        }
    } else if name == "unsafeGetAttrPos" {
        match (&args[0], &args[1]) {
            (PxVal::Str(attr), PxVal::Attrs(fields)) => {
                let pos = px_split_attr_pos(fields).1;
                match pos {
                    Some(p) => match px_force(&p)? {
                        PxVal::Attrs(pos_fields) => {
                            match px_attrs_find(pos_fields.as_ref(), attr) {
                                Some(v) => px_force(v),
                                None => Ok(PxVal::Null),
                            }
                        }
                        _ => Ok(PxVal::Null),
                    },
                    None => Ok(PxVal::Null),
                }
            }
            (_, PxVal::Attrs(_)) => Err(String::from(
                "px: unsafeGetAttrPos first argument must be a string",
            )),
            (PxVal::Str(_), _) => Err(String::from(
                "px: unsafeGetAttrPos second argument must be an attrset",
            )),
            _ => Err(String::from("px: unsafeGetAttrPos expects (string, attrset)")),
        }

    // ---- runtime-gap closure (2026-07-09), each oracle-pinned first ----
    } else if name == "add" || name == "sub" || name == "mul" || name == "div" {
        // oracle: int⊕int -> int (`div` truncates toward zero: div (-7) 2 == -3);
        // any float operand -> float. Division by zero errors.
        let both_int = match (&args[0], &args[1]) {
            (PxVal::Int(_), PxVal::Int(_)) => true,
            _ => false,
        };
        if both_int {
            let a = match &args[0] { PxVal::Int(v) => *v, _ => 0 };
            let b = match &args[1] { PxVal::Int(v) => *v, _ => 0 };
            return Ok(PxVal::Int(px_int_arith(name, a, b)?));
        }
        let af = px_num_f64(&args[0])?;
        let bf = px_num_f64(&args[1])?;
        if name == "add" {
            Ok(PxVal::Float(af + bf))
        } else if name == "sub" {
            Ok(PxVal::Float(af - bf))
        } else if name == "mul" {
            Ok(PxVal::Float(af * bf))
        } else {
            if bf == 0.0 {
                return Err(String::from("px: division by zero"));
            }
            Ok(PxVal::Float(af / bf))
        }
    } else if name == "lessThan" {
        // oracle: ints, floats (mixed ok) and strings all compare.
        match (&args[0], &args[1]) {
            (PxVal::Int(a), PxVal::Int(b)) => Ok(PxVal::Bool(a < b)),
            (PxVal::Str(a), PxVal::Str(b)) => Ok(PxVal::Bool(a < b)),
            _ => {
                let af = px_num_f64(&args[0])?;
                let bf = px_num_f64(&args[1])?;
                Ok(PxVal::Bool(af < bf))
            }
        }
    } else if name == "bitAnd" || name == "bitOr" || name == "bitXor" {
        match (&args[0], &args[1]) {
            (PxVal::Int(a), PxVal::Int(b)) => {
                if name == "bitAnd" {
                    Ok(PxVal::Int(px_bit_and(*a, *b)))
                } else if name == "bitOr" {
                    Ok(PxVal::Int(px_bit_or(*a, *b)))
                } else {
                    Ok(PxVal::Int(px_bit_xor(*a, *b)))
                }
            }
            _ => Err(format!("px: {} expects two ints", name)),
        }
    } else if name == "attrValues" {
        // oracle: values in KEY-SORTED order.
        match &args[0] {
            PxVal::Attrs(fields) => {
                let mut names = Vec::new();
                for (k, _v) in fields.iter() {
                    if !px_is_attr_pos_key(k) {
                        names.push(k.clone());
                    }
                }
                let sorted = px_sort_strings(names);
                let mut out = Vec::new();
                for n in sorted {
                    for (k, v) in fields.iter() {
                        if *k == n {
                            out.push(v.clone());
                        }
                    }
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: attrValues expects an attrset, got {}", px_kind(other))),
        }
    } else if name == "mapAttrs" {
        match &args[1] {
            PxVal::Attrs(fields) => {
                let mut out = Vec::new();
                for (k, v) in fields.iter() {
                    if px_is_attr_pos_key(k) {
                        continue;
                    }
                    let with_name = px_defer_apply(
                        args[0].clone(),
                        PxVal::Str(k.clone()),
                    );
                    out.push((k.clone(), px_defer_apply(with_name, v.clone())));
                }
                Ok(px_attrs(out))
            }
            other => Err(format!("px: mapAttrs expects an attrset, got {}", px_kind(other))),
        }
    } else if name == "mapAttrs'" {
        // oracle (pnix-clj `:mapAttrs'`): apply `f name value` over each
        // attribute (sorted-key order, matching `mapAttrs`); `f` must return
        // a `{ name; value; }` pair, and results are collected into a NEW
        // attrset keyed by the returned `name` — the first occurrence of a
        // duplicated resulting name wins, same tie-break as `listToAttrs`.
        match &args[1] {
            PxVal::Attrs(fields) => {
                let mut out: Vec<(String, PxVal)> = Vec::new();
                for (k, v) in fields.iter() {
                    if px_is_attr_pos_key(k) {
                        continue;
                    }
                    let with_name = px_defer_apply(args[0].clone(), PxVal::Str(k.clone()));
                    let pair = px_defer_apply(with_name, v.clone());
                    match px_force(&pair)? {
                        PxVal::Attrs(pair_fields) => {
                            let mut new_name: Option<String> = None;
                            let mut new_value: Option<PxVal> = None;
                            for (pk, pv) in pair_fields.iter() {
                                if pk == "name" {
                                    // the name field must be forced to check
                                    // it is a string; the value field stays a
                                    // thunk (kept lazy in the result).
                                    match px_force(pv)? {
                                        PxVal::Str(s) => new_name = Some(s.clone()),
                                        other => {
                                            return Err(format!(
                                                "px: mapAttrs' name must be a string, got {}",
                                                px_kind(&other)
                                            ))
                                        }
                                    }
                                }
                                if pk == "value" {
                                    new_value = Some(pv.clone());
                                }
                            }
                            match (new_name, new_value) {
                                (Some(n), Some(v)) => {
                                    // Accumulator is pre-invariant (unsorted):
                                    // linear first-wins check, same as
                                    // listToAttrs.
                                    let mut seen = false;
                                    let mut i = 0usize;
                                    while i < out.len() {
                                        if out[i].0 == n {
                                            seen = true;
                                        }
                                        i += 1;
                                    }
                                    if !seen {
                                        out.push((n, v));
                                    }
                                }
                                _ => {
                                    return Err(String::from(
                                        "px: mapAttrs' function must return a { name; value; } pair",
                                    ))
                                }
                            }
                        }
                        other => {
                            return Err(format!(
                                "px: mapAttrs' function must return an attrset, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                }
                Ok(px_attrs(out))
            }
            other => Err(format!("px: mapAttrs' expects an attrset, got {}", px_kind(other))),
        }
    } else if name == "catAttrs" {
        match (&args[0], &args[1]) {
            (PxVal::Str(key), PxVal::List(items)) => {
                let mut out = Vec::new();
                for it in items.iter() {
                    let it = px_force(it)?;
                    match &it {
                        PxVal::Attrs(fields) => {
                            for (k, v) in fields.iter() {
                                if k == key {
                                    out.push(v.clone());
                                }
                            }
                        }
                        other => {
                            return Err(format!(
                                "px: catAttrs expects a list of attrsets, got {}",
                                px_kind(other)
                            ))
                        }
                    }
                }
                Ok(px_list(out))
            }
            _ => Err(String::from("px: catAttrs expects (string, list)")),
        }
    } else if name == "intersectAttrs" {
        // oracle: keys of e1 ∩ e2, VALUES TAKEN FROM e2.
        match (&args[0], &args[1]) {
            (PxVal::Attrs(f1), PxVal::Attrs(f2)) => {
                let mut out = Vec::new();
                for (k2, v2) in f2.iter() {
                    if px_attrs_has(f1.as_ref(), k2) {
                        out.push((k2.clone(), v2.clone()));
                    }
                }
                Ok(px_attrs(out))
            }
            _ => Err(String::from("px: intersectAttrs expects two attrsets")),
        }
    } else if name == "zipAttrsWith" {
        // oracle: f key [values-in-list-order] for every key in the union.
        match &args[1] {
            PxVal::List(items) => {
                let mut keys: Vec<String> = Vec::new();
                for it in items.iter() {
                    let it = px_force(it)?;
                    match &it {
                        PxVal::Attrs(fields) => {
                            for (k, _v) in fields.iter() {
                                let mut seen = false;
                                let mut j = 0usize;
                                while j < keys.len() {
                                    if keys[j] == *k {
                                        seen = true;
                                    }
                                    j += 1;
                                }
                                if !seen {
                                    keys.push(k.clone());
                                }
                            }
                        }
                        other => {
                            return Err(format!(
                                "px: zipAttrsWith expects a list of attrsets, got {}",
                                px_kind(other)
                            ))
                        }
                    }
                }
                let sorted = px_sort_strings(keys);
                let mut out = Vec::new();
                for k in sorted {
                    let mut vals = Vec::new();
                    for it in items.iter() {
                        let it = px_force(it)?;
                        match &it {
                            PxVal::Attrs(fields) => {
                                for (fk, fv) in fields.iter() {
                                    if *fk == k {
                                        vals.push(fv.clone());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    let with_name = px_defer_apply(
                        args[0].clone(),
                        PxVal::Str(k.clone()),
                    );
                    out.push((k, px_defer_apply(with_name, px_list(vals))));
                }
                Ok(px_attrs(out))
            }
            other => Err(format!("px: zipAttrsWith expects a list, got {}", px_kind(other))),
        }
    } else if name == "groupBy" {
        match &args[1] {
            PxVal::List(items) => {
                let mut item_keys: Vec<String> = Vec::new();
                for it in items.iter() {
                    let kv = px_force(&px_apply(&args[0], it.clone())?)?;
                    match kv {
                        PxVal::Str(s) => item_keys.push(s),
                        other => {
                            return Err(format!(
                                "px: groupBy function must return a string, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                }
                let mut keys: Vec<String> = Vec::new();
                let mut i = 0usize;
                while i < item_keys.len() {
                    let mut seen = false;
                    let mut j = 0usize;
                    while j < keys.len() {
                        if keys[j] == item_keys[i] {
                            seen = true;
                        }
                        j += 1;
                    }
                    if !seen {
                        keys.push(item_keys[i].clone());
                    }
                    i += 1;
                }
                let sorted = px_sort_strings(keys);
                let mut out = Vec::new();
                for k in sorted {
                    let mut vals = Vec::new();
                    let mut m = 0usize;
                    while m < items.len() {
                        if item_keys[m] == k {
                            vals.push(items[m].clone());
                        }
                        m += 1;
                    }
                    out.push((k, px_list(vals)));
                }
                Ok(px_attrs(out))
            }
            other => Err(format!("px: groupBy expects a list, got {}", px_kind(other))),
        }
    } else if name == "partition" {
        // oracle: { right = [pred true]; wrong = [pred false]; }
        match &args[1] {
            PxVal::List(items) => {
                let mut right = Vec::new();
                let mut wrong = Vec::new();
                for it in items.iter() {
                    let r = px_force(&px_apply(&args[0], it.clone())?)?;
                    match r {
                        PxVal::Bool(true) => right.push(it.clone()),
                        PxVal::Bool(false) => wrong.push(it.clone()),
                        other => {
                            return Err(format!(
                                "px: partition predicate must return bool, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                }
                Ok(px_attrs(vec![
                    (String::from("right"), px_list(right)),
                    (String::from("wrong"), px_list(wrong)),
                ]))
            }
            other => Err(format!("px: partition expects a list, got {}", px_kind(other))),
        }
    } else if name == "seq" {
        // both operands are already forced (eager substrate); return the second.
        Ok(args[1].clone())
    } else if name == "splitVersion" {
        // oracle: split at `.`/`-` AND at every digit<->non-digit boundary;
        // empty components dropped ("1..2" == [ "1" "2" ], "1a2" == ["1" "a" "2"]).
        match &args[0] {
            PxVal::Str(s) => {
                let mut out = Vec::new();
                for c in px_split_version(s) {
                    out.push(PxVal::Str(c));
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: splitVersion expects a string, got {}", px_kind(other))),
        }
    } else if name == "compareVersions" {
        match (&args[0], &args[1]) {
            (PxVal::Str(a), PxVal::Str(b)) => Ok(PxVal::Int(px_compare_versions(a, b))),
            _ => Err(String::from("px: compareVersions expects two strings")),
        }
    } else if name == "sort" {
        match &args[1] {
            PxVal::List(items) => {
                let mut remaining = items.as_ref().clone();
                let mut out = Vec::new();
                while !remaining.is_empty() {
                    let mut min = 0usize;
                    let mut j = 1usize;
                    while j < remaining.len() {
                        let step = px_apply(&args[0], remaining[j].clone())?;
                        let less = px_force(&px_apply(&step, remaining[min].clone())?)?;
                        match less {
                            PxVal::Bool(true) => {
                                min = j;
                            }
                            PxVal::Bool(false) => {}
                            other => {
                                return Err(format!(
                                    "px: sort comparator must return bool, got {}",
                                    px_kind(&other)
                                ))
                            }
                        }
                        j += 1;
                    }
                    out.push(remaining.remove(min));
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: sort expects a list, got {}", px_kind(other))),
        }
    } else if name == "head" {
        match &args[0] {
            PxVal::List(items) => {
                if items.is_empty() {
                    Err(String::from("px: head of empty list"))
                } else {
                    px_force(&items[0])
                }
            }
            other => Err(format!("px: head expects a list, got {}", px_kind(other))),
        }
    } else if name == "tail" {
        match &args[0] {
            PxVal::List(items) => {
                if items.is_empty() {
                    Err(String::from("px: tail of empty list"))
                } else {
                    let inner = items.as_ref();
                    Ok(px_list(inner[1..].to_vec()))
                }
            }
            other => Err(format!("px: tail expects a list, got {}", px_kind(other))),
        }
    } else if name == "elemAt" {
        match (&args[0], &args[1]) {
            (PxVal::List(items), PxVal::Int(i)) => {
                if *i < 0 || *i as usize >= items.len() {
                    Err(String::from("px: elemAt index out of range"))
                } else {
                    px_force(&items[*i as usize])
                }
            }
            _ => Err(String::from("px: elemAt expects (list, int)")),
        }
    } else if name == "listToAttrs" {
        // Nix semantics (matches the pnix-hy runtime): elements are
        // { name = <string>; value = <v>; } attrsets; the FIRST occurrence of
        // a name wins.
        match &args[0] {
            PxVal::List(items) => {
                let mut out: Vec<(String, PxVal)> = Vec::new();
                for item in items.iter() {
                    match px_force(item)? {
                        PxVal::Attrs(entry) => {
                            let mut name: Option<String> = None;
                            let mut value: Option<PxVal> = None;
                            for (k, v) in entry.iter() {
                                if k == "name" {
                                    // the name field must be forced to check it
                                    // is a string; the value field stays a thunk
                                    // (kept lazy in the resulting attrset).
                                    match px_force(v)? {
                                        PxVal::Str(s) => name = Some(s.clone()),
                                        other => {
                                            return Err(format!(
                                                "px: listToAttrs name must be a string, got {}",
                                                px_kind(&other)
                                            ))
                                        }
                                    }
                                }
                                if k == "value" {
                                    value = Some(v.clone());
                                }
                            }
                            match (name, value) {
                                (Some(n), Some(v)) => {
                                    // Accumulator is pre-invariant (unsorted):
                                    // linear first-wins check.
                                    let mut seen = false;
                                    let mut k = 0usize;
                                    while k < out.len() {
                                        if out[k].0 == n {
                                            seen = true;
                                        }
                                        k += 1;
                                    }
                                    if !seen {
                                        out.push((n, v));
                                    }
                                }
                                _ => {
                                    return Err(String::from(
                                        "px: listToAttrs element needs name and value",
                                    ))
                                }
                            }
                        }
                        other => {
                            return Err(format!(
                                "px: listToAttrs expects attrset elements, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                }
                Ok(px_attrs(out))
            }
            other => Err(format!("px: listToAttrs expects a list, got {}", px_kind(other))),
        }
    } else if name == "getAttr" {
        match (&args[0], &args[1]) {
            (PxVal::Str(k), PxVal::Attrs(fields)) => match px_attrs_find(fields.as_ref(), k) {
                Some(value) => Ok(value.clone()),
                None => Err(format!("px: getAttr: attribute '{}' missing", k)),
            },
            _ => Err(String::from("px: getAttr expects (string, attrset)")),
        }
    } else if name == "isAttrs" {
        Ok(PxVal::Bool(px_is_real_attrset(&args[0])))
    } else if name == "isNull" {
        Ok(PxVal::Bool(matches!(&args[0], PxVal::Null)))
    } else if name == "isFloat" {
        Ok(PxVal::Bool(matches!(&args[0], PxVal::Float(_))))
    } else if name == "typeOf" {
        // Nix names (oracle-pinned): attrsets are "set"; a context-bearing
        // string is a "string" (context is metadata, not a type).
        let t = match &args[0] {
            PxVal::Int(_) => "int",
            PxVal::Float(_) => "float",
            PxVal::Bool(_) => "bool",
            PxVal::Null => "null",
            PxVal::Str(_) => "string",
            PxVal::Bytes(_) => "string",
            PxVal::Path(_) => "path",
            PxVal::List(_) => "list",
            PxVal::Attrs(_) if px_is_ctx_string(&args[0]) => "string",
            PxVal::Attrs(_) => "set",
            PxVal::Closure { .. } | PxVal::Builtin { .. } => "lambda",
            // typeOf forces its argument to WHNF (Nix); re-dispatch on the
            // forced value. Under the containment invariant this arm is not
            // normally reached, but forcing here is always correct.
            PxVal::Thunk(_) => return px_builtin_exec(name, &vec![px_force(&args[0])?]),
        };
        Ok(PxVal::Str(String::from(t)))
    } else if name == "baseNameOf" || name == "dirOf" {
        match &args[0] {
            PxVal::Str(p) => {
                // last '/' split (oracle: dirOf "x" == ".", dirOf "/a/b/c" ==
                // "/a/b", baseNameOf keeps the tail). '/' is ASCII-safe.
                let mut last: i64 = -1;
                let mut i = 0i64;
                for c in p.chars() {
                    if c == '/' {
                        last = i;
                    }
                    i += 1;
                }
                let mut head = String::new();
                let mut tail = String::new();
                let mut j = 0i64;
                for c in p.chars() {
                    if j < last {
                        head.push(c);
                    } else if j > last {
                        tail.push(c);
                    }
                    j += 1;
                }
                if name == "baseNameOf" {
                    Ok(PxVal::Str(if last < 0 { p.clone() } else { tail }))
                } else if last < 0 {
                    Ok(PxVal::Str(String::from(".")))
                } else if last == 0 {
                    Ok(PxVal::Str(String::from("/")))
                } else {
                    Ok(PxVal::Str(head))
                }
            }
            // A Path input: baseNameOf still returns a plain string (Nix:
            // `baseNameOf : (path | string) -> string`), but dirOf returns
            // a Path (its "directory of" is still path-shaped).
            PxVal::Path(p) => {
                let mut last: i64 = -1;
                let mut i = 0i64;
                for c in p.chars() {
                    if c == '/' {
                        last = i;
                    }
                    i += 1;
                }
                let mut head = String::new();
                let mut tail = String::new();
                let mut j = 0i64;
                for c in p.chars() {
                    if j < last {
                        head.push(c);
                    } else if j > last {
                        tail.push(c);
                    }
                    j += 1;
                }
                if name == "baseNameOf" {
                    Ok(PxVal::Str(if last < 0 { p.clone() } else { tail }))
                } else if last < 0 {
                    Ok(PxVal::Path(px_normalize_path(".")))
                } else if last == 0 {
                    Ok(PxVal::Path(px_normalize_path("/")))
                } else {
                    Ok(PxVal::Path(px_normalize_path(&head)))
                }
            }
            other => Err(format!("px: {} expects a string, got {}", name, px_kind(other))),
        }
    } else if name == "abort" {
        match &args[0] {
            PxVal::Str(msg) => Err(format!("px: abort: {}", msg)),
            other => Err(format!("px: abort expects a string, got {}", px_kind(other))),
        }
    } else if name == "removeAttrs" {
        match (&args[0], &args[1]) {
            (PxVal::Attrs(fields), PxVal::List(names)) => {
                let mut out = Vec::new();
                for (k, v) in fields.iter() {
                    let mut drop = false;
                    for n in names.iter() {
                        let n = px_force(n)?;
                        if let PxVal::Str(ns) = &n {
                            if ns == k {
                                drop = true;
                            }
                        }
                    }
                    if !drop {
                        out.push((k.clone(), v.clone()));
                    }
                }
                Ok(px_attrs(out))
            }
            _ => Err(String::from("px: removeAttrs expects (attrset, list)")),
        }
    } else if name == "replaceStrings" {
        // Context-aware: needles match on content only; the result carries
        // the subject's own context PLUS the context of every replacement
        // actually USED (an unused `to` pair contributes nothing — exact,
        // not an over-approximation). Content-free everywhere collapses to
        // a plain Str exactly as before (`px_ctx_string` collapses on an
        // empty context), so this is a byte-identical rewrite of the
        // original context-free algorithm, not a behavior change for it.
        match (&args[0], &args[1]) {
            (PxVal::List(from), PxVal::List(to)) if px_is_string_like(&args[2]) => {
                if from.len() != to.len() {
                    return Err(String::from("px: replaceStrings: from/to length mismatch"));
                }
                let content = px_string_like_content_or_empty(&args[2]);
                let chars: Vec<char> = content.chars().collect();
                let n = chars.len();
                let mut out = String::new();
                let mut used_ctx = px_string_like_context(&args[2]);
                // Single left-to-right pass over positions 0..=n (INCLUSIVE
                // of the end): at each position, try each `from` in order
                // and take the first prefix match. An empty `from` matches
                // at every position, including n, so it must not be
                // excluded -- but a zero-width match must still emit the
                // current character (if any) and advance by exactly 1, or
                // the same empty match would fire forever at the same spot.
                let mut i = 0usize;
                while i <= n {
                    let mut matched: Option<(usize, String, Vec<String>)> = None;
                    let mut fi = 0usize;
                    while fi < from.len() && matched.is_none() {
                        let from_item = px_force(&from[fi])?;
                        let to_item = px_force(&to[fi])?;
                        if px_is_string_like(&from_item) && px_is_string_like(&to_item) {
                            let f = px_string_like_content_or_empty(&from_item);
                            let fc: Vec<char> = f.chars().collect();
                            if i + fc.len() <= n {
                                let mut eq = true;
                                let mut k = 0usize;
                                while k < fc.len() {
                                    if chars[i + k] != fc[k] {
                                        eq = false;
                                    }
                                    k += 1;
                                }
                                if eq {
                                    matched = Some((
                                        fc.len(),
                                        px_string_like_content_or_empty(&to_item),
                                        px_string_like_context(&to_item),
                                    ));
                                }
                            }
                        }
                        fi += 1;
                    }
                    match matched {
                        Some((0, replacement, r_ctx)) => {
                            out.push_str(&replacement);
                            used_ctx.extend(r_ctx);
                            if i < n {
                                out.push(chars[i]);
                            }
                            i += 1;
                        }
                        Some((len, replacement, r_ctx)) => {
                            out.push_str(&replacement);
                            used_ctx.extend(r_ctx);
                            i += len;
                        }
                        None => {
                            if i < n {
                                out.push(chars[i]);
                            }
                            i += 1;
                        }
                    }
                }
                Ok(px_ctx_string(out, used_ctx))
            }
            _ => Err(String::from("px: replaceStrings expects (list, list, string)")),
        }
    } else if name == "foldl" {
        // alias of foldl' (reference: pnix-clj).
        let mut acc = args[1].clone();
        match &args[2] {
            PxVal::List(items) => {
                for item in items.iter() {
                    let f1 = px_apply(&args[0], acc)?;
                    acc = px_force(&px_apply(&f1, item.clone())?)?;
                }
                Ok(acc)
            }
            other => Err(format!("px: foldl expects a list, got {}", px_kind(other))),
        }
    } else if name == "concatLists" {
        match &args[0] {
            PxVal::List(items) => {
                let mut out = Vec::new();
                for item in items.iter() {
                    let item = px_force(item)?;
                    match &item {
                        PxVal::List(inner) => {
                            for v in inner.iter() {
                                out.push(v.clone());
                            }
                        }
                        other => {
                            return Err(format!(
                                "px: concatLists: element is {}, expected a list",
                                px_kind(other)
                            ))
                        }
                    }
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: concatLists expects a list, got {}", px_kind(other))),
        }
    } else if name == "concatMap" {
        match &args[1] {
            PxVal::List(items) => {
                let mut out = Vec::new();
                for item in items.iter() {
                    let result = px_apply(&args[0], item.clone())?;
                    match px_force(&result)? {
                        PxVal::List(inner) => {
                            for v in inner.iter() {
                                out.push(v.clone());
                            }
                        }
                        other => {
                            return Err(format!(
                                "px: concatMap: function returned {}, expected a list",
                                px_kind(&other)
                            ))
                        }
                    }
                }
                Ok(PxVal::List(std::rc::Rc::new(out)))
            }
            other => Err(format!("px: concatMap expects a list, got {}", px_kind(other))),
        }
    } else if name == "addErrorContext" {
        // Oracle-pinned (nix 2.34.7, 2026-07-09): pure value passthrough —
        // context only decorates error MESSAGES, which the error-form
        // precedent (test262/wasm/SLT) treats as non-normative; allowed
        // under --pure-eval. Stub = return the value.
        Ok(args[1].clone())
    } else if name == "deepSeq" {
        px_force_deep(&args[0])?;
        Ok(args[1].clone())
    } else if name == "throw" {
        // Nix builtins.throw: raise with the message (oracle: lazy callers
        // that never force it proceed — matches the eager seed only when the
        // call site is actually reached, which is the case in guarded code).
        match &args[0] {
            PxVal::Str(msg) => Err(format!("px: throw: {}", msg)),
            other => Err(format!("px: throw expects a string, got {}", px_kind(other))),
        }
    } else if name == "ceil" || name == "floor" {
        px_round_to_int(&args[0], name == "ceil")
    } else if name == "min" || name == "max" {
        match (&args[0], &args[1]) {
            (PxVal::Int(a), PxVal::Int(b)) => {
                if name == "min" {
                    Ok(PxVal::Int(if *a < *b { *a } else { *b }))
                } else {
                    Ok(PxVal::Int(if *a > *b { *a } else { *b }))
                }
            }
            _ => {
                let af = px_num_f64(&args[0])?;
                let bf = px_num_f64(&args[1])?;
                let r = if name == "min" {
                    if af < bf { af } else { bf }
                } else {
                    if af > bf { af } else { bf }
                };
                Ok(PxVal::Float(r))
            }
        }
    } else if name == "sqrt" {
        Ok(PxVal::Float(px_math_sqrt(px_num_f64(&args[0])?)))
    } else if name == "exp" {
        Ok(PxVal::Float(px_math_exp(px_num_f64(&args[0])?)))
    } else if name == "ln" || name == "log" {
        // oracle (pnix-hy): `log` is natural log, identical to `ln` — not
        // base-10/base-2 — kept as a separate name for historical
        // Nix-adjacent reasons (`"log": lambda value: math.log(value)`).
        Ok(PxVal::Float(px_math_ln(px_num_f64(&args[0])?)?))
    } else if name == "sin" {
        Ok(PxVal::Float(px_math_sin(px_num_f64(&args[0])?)))
    } else if name == "cos" {
        Ok(PxVal::Float(px_math_cos(px_num_f64(&args[0])?)))
    } else if name == "tan" {
        Ok(PxVal::Float(px_math_tan(px_num_f64(&args[0])?)))
    } else if name == "atan2" {
        // oracle (pnix-hy): curried (y)(x), i.e. `atan2 y x` per math
        // convention, not the argument-name-alphabetical order.
        let y = px_num_f64(&args[0])?;
        let x = px_num_f64(&args[1])?;
        Ok(PxVal::Float(px_math_atan2(y, x)))
    } else if name == "abs" {
        // oracle: int -> int (checked, like add/sub/mul/div); any float -> float.
        match &args[0] {
            PxVal::Int(n) => {
                let n = *n;
                if n < 0 {
                    match 0i64.checked_sub(n) {
                        Some(v) => Ok(PxVal::Int(v)),
                        None => Err(format!("px: integer overflow in abs {}", n)),
                    }
                } else {
                    Ok(PxVal::Int(n))
                }
            }
            other => Ok(PxVal::Float(px_abs_f64(px_num_f64(other)?))),
        }
    } else if name == "pow" {
        // oracle (pnix-hy `pow_value`): int^int with a non-negative exponent
        // stays int (checked, errors on i64 overflow like add/sub/mul/div);
        // anything else (negative int exponent or any float operand) -> float.
        match (&args[0], &args[1]) {
            (PxVal::Int(base), PxVal::Int(exp)) if *exp >= 0 => {
                Ok(PxVal::Int(px_checked_ipow(*base, *exp)?))
            }
            _ => {
                let bf = px_num_f64(&args[0])?;
                let ef = px_num_f64(&args[1])?;
                Ok(PxVal::Float(px_powf(bf, ef)?))
            }
        }
    } else if name == "mod" {
        // oracle: int⊕int -> int (checked, truncating remainder, same sign
        // rules as Rust's native `%`); any float operand -> float (fmod).
        match (&args[0], &args[1]) {
            (PxVal::Int(a), PxVal::Int(b)) => {
                let a = *a;
                let b = *b;
                if b == 0 {
                    return Err(String::from("px: division by zero"));
                }
                match a.checked_rem(b) {
                    Some(v) => Ok(PxVal::Int(v)),
                    None => Err(format!("px: integer overflow in mod {} {}", a, b)),
                }
            }
            _ => {
                let af = px_num_f64(&args[0])?;
                let bf = px_num_f64(&args[1])?;
                if bf == 0.0 {
                    return Err(String::from("px: division by zero"));
                }
                Ok(PxVal::Float(px_fmod_f64(af, bf)))
            }
        }
    } else if name == "functionArgs" {
        match &args[0] {
            PxVal::Closure { param, body, .. } => Ok(px_function_args(param, body)),
            other => Err(format!(
                "px: builtins.functionArgs expects a lambda, got {}",
                px_kind(other)
            )),
        }
    } else if name == "trace" || name == "warn" {
        let msg = match &args[0] {
            PxVal::Str(s) => s.clone(),
            other => px_print(other),
        };
        if name == "warn" {
            eprintln!("trace: warning: {}", msg);
        } else {
            eprintln!("trace: {}", msg);
        }
        Ok(args[1].clone())
    } else if name == "toXML" {
        Ok(PxVal::Str(px_to_xml(&args[0])?))
    } else if name == "toFile" {
        match (&args[0], &args[1]) {
            (PxVal::Str(name_s), PxVal::Str(content)) => {
                let hash = px_sha256_hex(sha_utf8_bytes(content));
                let mut short = String::new();
                let mut i = 0usize;
                for c in hash.chars() {
                    if i < 32 {
                        short.push(c);
                    }
                    i += 1;
                }
                let store = String::from("/tmp/pnix-nix-store");
                match std::fs::create_dir_all(&store) {
                    Ok(()) => {}
                    Err(e) => return Err(format!("px: toFile: {}", e)),
                }
                let path = format!("{}/{}-{}", store, short, name_s);
                match std::fs::write(&path, content) {
                    Ok(()) => Ok(PxVal::Str(path)),
                    Err(e) => Err(format!("px: toFile: {}", e)),
                }
            }
            _ => Err(String::from("px: toFile expects (string, string)")),
        }
    } else if name == "readFile" {
        let p = px_as_path_str(&args[0])?;
        match std::fs::read_to_string(&p) {
            Ok(s) => Ok(PxVal::Str(s)),
            Err(e) => Err(format!("px: readFile {}: {}", p, e)),
        }
    } else if name == "pathExists" {
        let p = px_as_path_str(&args[0])?;
        Ok(PxVal::Bool(std::path::Path::new(&p).exists()))
    } else if name == "readDir" {
        px_read_dir(&px_as_path_str(&args[0])?)
    } else if name == "fetchurl" {
        px_fetch_url_arg(&args[0])
    } else if name == "fetchTarball" {
        px_fetch_tarball_arg(&args[0])
    } else if name == "fetchGit" {
        px_fetch_git_arg(&args[0])
    } else if name == "last" {
        match &args[0] {
            PxVal::List(items) => {
                if items.is_empty() {
                    Err(String::from("px: last of empty list"))
                } else {
                    px_force(&items[items.len() - 1])
                }
            }
            other => Err(format!("px: last expects a list, got {}", px_kind(other))),
        }
    } else if name == "init" {
        match &args[0] {
            PxVal::List(items) => {
                if items.is_empty() {
                    Err(String::from("px: init of empty list"))
                } else {
                    let inner = items.as_ref();
                    Ok(px_list(inner[0..inner.len() - 1].to_vec()))
                }
            }
            other => Err(format!("px: init expects a list, got {}", px_kind(other))),
        }
    } else if name == "flatten" {
        let mut out = Vec::new();
        px_flatten_into(&args[0], &mut out)?;
        Ok(px_list(out))
    } else if name == "foldr" {
        match &args[2] {
            PxVal::List(items) => {
                let mut acc = args[1].clone();
                let mut i = items.len();
                while i > 0 {
                    i -= 1;
                    let step = px_apply(&args[0], items[i].clone())?;
                    acc = px_force(&px_apply(&step, acc)?)?;
                }
                Ok(acc)
            }
            other => Err(format!("px: foldr expects a list, got {}", px_kind(other))),
        }
    } else if name == "id" {
        Ok(args[0].clone())
    } else if name == "const" {
        Ok(args[0].clone())
    } else if name == "flip" {
        let step = px_apply(&args[0], args[2].clone())?;
        px_force(&px_apply(&step, args[1].clone())?)
    } else if name == "pipe" {
        match &args[1] {
            PxVal::List(fs) => {
                let mut acc = args[0].clone();
                for f in fs.iter() {
                    let f = px_force(f)?;
                    acc = px_force(&px_apply(&f, acc)?)?;
                }
                Ok(acc)
            }
            other => Err(format!("px: pipe expects a list of functions, got {}", px_kind(other))),
        }
    } else if name == "fix" {
        let cell = std::rc::Rc::new(std::cell::RefCell::new(PxThunk::Blackhole));
        let self_thunk = PxVal::Thunk(cell.clone());
        *cell.borrow_mut() = PxThunk::DeferredApply(args[0].clone(), self_thunk.clone());
        px_force(&self_thunk)
    } else if name == "range" {
        match (&args[0], &args[1]) {
            (PxVal::Int(a), PxVal::Int(b)) => {
                let mut out = Vec::new();
                let mut i = *a;
                while i <= *b {
                    out.push(PxVal::Int(i));
                    i += 1;
                }
                Ok(px_list(out))
            }
            _ => Err(String::from("px: range expects two ints")),
        }
    } else if name == "sum" {
        match &args[0] {
            PxVal::List(items) => {
                let mut acc = 0i64;
                let mut all_int = true;
                let mut facc = 0.0f64;
                for it in items.iter() {
                    let it = px_force(it)?;
                    match it {
                        PxVal::Int(n) => {
                            acc = acc + n;
                            facc = facc + (n as f64);
                        }
                        PxVal::Float(f) => {
                            all_int = false;
                            facc = facc + f;
                        }
                        other => {
                            return Err(format!("px: sum expects numbers, got {}", px_kind(&other)))
                        }
                    }
                }
                if all_int {
                    Ok(PxVal::Int(acc))
                } else {
                    Ok(PxVal::Float(facc))
                }
            }
            other => Err(format!("px: sum expects a list, got {}", px_kind(other))),
        }
    } else if name == "product" {
        match &args[0] {
            PxVal::List(items) => {
                let mut acc = 1i64;
                let mut all_int = true;
                let mut facc = 1.0f64;
                for it in items.iter() {
                    let it = px_force(it)?;
                    match it {
                        PxVal::Int(n) => {
                            acc = acc * n;
                            facc = facc * (n as f64);
                        }
                        PxVal::Float(f) => {
                            all_int = false;
                            facc = facc * f;
                        }
                        other => {
                            return Err(format!(
                                "px: product expects numbers, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                }
                if all_int {
                    Ok(PxVal::Int(acc))
                } else {
                    Ok(PxVal::Float(facc))
                }
            }
            other => Err(format!("px: product expects a list, got {}", px_kind(other))),
        }
    } else if name == "unique" {
        match &args[0] {
            PxVal::List(items) => {
                let mut out = Vec::new();
                for it in items.iter() {
                    let mut seen = false;
                    for prev in out.iter() {
                        if px_val_eq_nested(it, prev)? {
                            seen = true;
                        }
                    }
                    if !seen {
                        out.push(it.clone());
                    }
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: unique expects a list, got {}", px_kind(other))),
        }
    } else if name == "toLower" {
        match &args[0] {
            PxVal::Str(s) => Ok(PxVal::Str(s.to_lowercase())),
            // Case conversion keeps the context (oracle-pinned).
            _ if px_is_ctx_string(&args[0]) => Ok(px_ctx_string(
                px_string_like_content_or_empty(&args[0]).to_lowercase(),
                px_string_like_context(&args[0]),
            )),
            other => Err(format!("px: toLower expects a string, got {}", px_kind(other))),
        }
    } else if name == "toUpper" {
        match &args[0] {
            PxVal::Str(s) => Ok(PxVal::Str(s.to_uppercase())),
            _ if px_is_ctx_string(&args[0]) => Ok(px_ctx_string(
                px_string_like_content_or_empty(&args[0]).to_uppercase(),
                px_string_like_context(&args[0]),
            )),
            other => Err(format!("px: toUpper expects a string, got {}", px_kind(other))),
        }
    } else if name == "boolToString" {
        match &args[0] {
            PxVal::Bool(true) => Ok(PxVal::Str(String::from("true"))),
            PxVal::Bool(false) => Ok(PxVal::Str(String::from("false"))),
            other => Err(format!("px: boolToString expects a bool, got {}", px_kind(other))),
        }
    } else if name == "implies" {
        match (&args[0], &args[1]) {
            (PxVal::Bool(a), PxVal::Bool(b)) => Ok(PxVal::Bool((!*a) || *b)),
            _ => Err(String::from("px: implies expects two bools")),
        }
    } else if name == "optional" {
        match &args[0] {
            PxVal::Bool(true) => Ok(px_list(vec![args[1].clone()])),
            PxVal::Bool(false) => Ok(px_list(Vec::new())),
            other => Err(format!("px: optional expects a bool, got {}", px_kind(other))),
        }
    } else if name == "optionals" {
        match (&args[0], &args[1]) {
            (PxVal::Bool(true), PxVal::List(_)) => Ok(args[1].clone()),
            (PxVal::Bool(false), _) => Ok(px_list(Vec::new())),
            _ => Err(String::from("px: optionals expects (bool, list)")),
        }
    } else if name == "optionalAttrs" {
        match (&args[0], &args[1]) {
            (PxVal::Bool(true), PxVal::Attrs(_)) => Ok(args[1].clone()),
            (PxVal::Bool(false), _) => Ok(px_attrs(Vec::new())),
            _ => Err(String::from("px: optionalAttrs expects (bool, attrset)")),
        }
    } else if name == "when" {
        match &args[0] {
            PxVal::Bool(true) => Ok(args[1].clone()),
            PxVal::Bool(false) => Ok(PxVal::Null),
            other => Err(format!("px: when expects a bool, got {}", px_kind(other))),
        }
    } else if name == "removePrefix" {
        // substring-based lib semantics: the result keeps `s`'s WHOLE
        // context; a contextful prefix argument only affects the
        // comparison (oracle-pinned, matches pnix-clj's `:removePrefix`).
        if px_is_ctx_string(&args[0]) || px_is_ctx_string(&args[1]) {
            return match (px_string_like_content(&args[0]), px_string_like_content(&args[1])) {
                (Some(pre), Some(s)) => {
                    let s_ctx = px_string_like_context(&args[1]);
                    if s.starts_with(pre.as_str()) {
                        let mut out = String::new();
                        let mut skipped = 0usize;
                        let n = pre.chars().count();
                        for c in s.chars() {
                            if skipped < n {
                                skipped += 1;
                            } else {
                                out.push(c);
                            }
                        }
                        Ok(px_ctx_string(out, s_ctx))
                    } else {
                        Ok(px_ctx_string(s, s_ctx))
                    }
                }
                _ => Err(String::from("px: removePrefix expects two strings")),
            };
        }
        match (&args[0], &args[1]) {
            (PxVal::Str(pre), PxVal::Str(s)) => {
                if s.starts_with(pre.as_str()) {
                    let mut out = String::new();
                    let mut skipped = 0usize;
                    let n = pre.chars().count();
                    for c in s.chars() {
                        if skipped < n {
                            skipped += 1;
                        } else {
                            out.push(c);
                        }
                    }
                    Ok(PxVal::Str(out))
                } else {
                    Ok(PxVal::Str(s.clone()))
                }
            }
            _ => Err(String::from("px: removePrefix expects two strings")),
        }
    } else if name == "removeSuffix" {
        // Same context contract as removePrefix.
        if px_is_ctx_string(&args[0]) || px_is_ctx_string(&args[1]) {
            return match (px_string_like_content(&args[0]), px_string_like_content(&args[1])) {
                (Some(suf), Some(s)) => {
                    let s_ctx = px_string_like_context(&args[1]);
                    if px_str_has_suffix(&s, &suf) {
                        let n = s.chars().count();
                        let m = suf.chars().count();
                        let mut out = String::new();
                        let mut i = 0usize;
                        for c in s.chars() {
                            if i < n - m {
                                out.push(c);
                            }
                            i += 1;
                        }
                        Ok(px_ctx_string(out, s_ctx))
                    } else {
                        Ok(px_ctx_string(s, s_ctx))
                    }
                }
                _ => Err(String::from("px: removeSuffix expects two strings")),
            };
        }
        match (&args[0], &args[1]) {
            (PxVal::Str(suf), PxVal::Str(s)) => {
                if px_str_has_suffix(s, suf) {
                    let n = s.chars().count();
                    let m = suf.chars().count();
                    let mut out = String::new();
                    let mut i = 0usize;
                    for c in s.chars() {
                        if i < n - m {
                            out.push(c);
                        }
                        i += 1;
                    }
                    Ok(PxVal::Str(out))
                } else {
                    Ok(PxVal::Str(s.clone()))
                }
            }
            _ => Err(String::from("px: removeSuffix expects two strings")),
        }
    } else if name == "hasPrefix" {
        // Content-based predicate: context does not affect the answer.
        if px_is_ctx_string(&args[0]) || px_is_ctx_string(&args[1]) {
            return match (px_string_like_content(&args[0]), px_string_like_content(&args[1])) {
                (Some(pre), Some(s)) => Ok(PxVal::Bool(s.starts_with(pre.as_str()))),
                _ => Err(String::from("px: hasPrefix expects two strings")),
            };
        }
        match (&args[0], &args[1]) {
            (PxVal::Str(pre), PxVal::Str(s)) => Ok(PxVal::Bool(s.starts_with(pre.as_str()))),
            _ => Err(String::from("px: hasPrefix expects two strings")),
        }
    } else if name == "hasSuffix" {
        if px_is_ctx_string(&args[0]) || px_is_ctx_string(&args[1]) {
            return match (px_string_like_content(&args[0]), px_string_like_content(&args[1])) {
                (Some(suf), Some(s)) => Ok(PxVal::Bool(px_str_has_suffix(&s, &suf))),
                _ => Err(String::from("px: hasSuffix expects two strings")),
            };
        }
        match (&args[0], &args[1]) {
            (PxVal::Str(suf), PxVal::Str(s)) => Ok(PxVal::Bool(px_str_has_suffix(s, suf))),
            _ => Err(String::from("px: hasSuffix expects two strings")),
        }
    } else if name == "splitString" {
        // lib.splitString is builtins.split-based, and split results are
        // context-free (oracle) — pieces come back as plain strings even
        // when the separator/subject carry context.
        if px_is_ctx_string(&args[0]) || px_is_ctx_string(&args[1]) {
            return match (px_string_like_content(&args[0]), px_string_like_content(&args[1])) {
                (Some(sep), Some(s)) => {
                    let mut out = Vec::new();
                    if sep.is_empty() {
                        for c in s.chars() {
                            let mut t = String::new();
                            t.push(c);
                            out.push(PxVal::Str(t));
                        }
                    } else {
                        for part in s.split(sep.as_str()) {
                            out.push(PxVal::Str(String::from(part)));
                        }
                    }
                    Ok(px_list(out))
                }
                _ => Err(String::from("px: splitString expects two strings")),
            };
        }
        match (&args[0], &args[1]) {
            (PxVal::Str(sep), PxVal::Str(s)) => {
                let mut out = Vec::new();
                if sep.is_empty() {
                    for c in s.chars() {
                        let mut t = String::new();
                        t.push(c);
                        out.push(PxVal::Str(t));
                    }
                } else {
                    for part in s.split(sep.as_str()) {
                        out.push(PxVal::Str(String::from(part)));
                    }
                }
                Ok(px_list(out))
            }
            _ => Err(String::from("px: splitString expects two strings")),
        }
    } else if name == "concatMapStringsSep" {
        match &args[2] {
            PxVal::List(items) => {
                let sep = match &args[0] {
                    PxVal::Str(s) => s.clone(),
                    other => {
                        return Err(format!(
                            "px: concatMapStringsSep expects string sep, got {}",
                            px_kind(other)
                        ))
                    }
                };
                let mut parts: Vec<String> = Vec::new();
                for it in items.iter() {
                    let r = px_force(&px_apply(&args[1], it.clone())?)?;
                    match px_to_string_coerce(&r)? {
                        PxVal::Str(s) => parts.push(s),
                        _ => {}
                    }
                }
                Ok(PxVal::Str(parts.join(&sep)))
            }
            other => Err(format!(
                "px: concatMapStringsSep expects a list, got {}",
                px_kind(other)
            )),
        }
    } else if name == "filterAttrs" {
        match &args[1] {
            PxVal::Attrs(fields) => {
                let mut out = Vec::new();
                for (k, v) in fields.iter() {
                    let step = px_apply(&args[0], PxVal::Str(k.clone()))?;
                    let keep = px_force(&px_apply(&step, v.clone())?)?;
                    match keep {
                        PxVal::Bool(true) => out.push((k.clone(), v.clone())),
                        PxVal::Bool(false) => {}
                        other => {
                            return Err(format!(
                                "px: filterAttrs predicate must return bool, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                }
                Ok(px_attrs(out))
            }
            other => Err(format!(
                "px: filterAttrs expects an attrset, got {}",
                px_kind(other)
            )),
        }
    } else if name == "getAttrFromPath" {
        match &args[0] {
            PxVal::List(path) => px_get_attr_from_path(path, &args[1], true, None),
            other => Err(format!(
                "px: getAttrFromPath expects a path list, got {}",
                px_kind(other)
            )),
        }
    } else if name == "hasAttrByPath" {
        match &args[0] {
            PxVal::List(path) => match px_get_attr_from_path(path, &args[1], false, None) {
                Ok(_) => Ok(PxVal::Bool(true)),
                Err(_) => Ok(PxVal::Bool(false)),
            },
            other => Err(format!(
                "px: hasAttrByPath expects a path list, got {}",
                px_kind(other)
            )),
        }
    } else if name == "attrByPath" {
        // attrByPath path default set
        match &args[0] {
            PxVal::List(path) => {
                match px_get_attr_from_path(path, &args[2], false, Some(args[1].clone())) {
                    Ok(v) => Ok(v),
                    Err(_) => Ok(args[1].clone()),
                }
            }
            other => Err(format!(
                "px: attrByPath expects a path list, got {}",
                px_kind(other)
            )),
        }
    } else if name == "getAttrFromPathOr" {
        // README order: set path default
        match &args[1] {
            PxVal::List(path) => {
                match px_get_attr_from_path(path, &args[0], false, Some(args[2].clone())) {
                    Ok(v) => Ok(v),
                    Err(_) => Ok(args[2].clone()),
                }
            }
            other => Err(format!(
                "px: getAttrFromPathOr expects a path list, got {}",
                px_kind(other)
            )),
        }
    } else if name == "recursiveUpdate" {
        px_recursive_update(&args[0], &args[1])
    } else if name == "updateManyAttrs" {
        match &args[0] {
            PxVal::List(items) => {
                let mut acc = args[1].clone();
                for it in items.iter() {
                    let it = px_force(it)?;
                    acc = px_binary_outcome(&PxOp::Update, &acc, &it).map_err(|e| e.diagnostic)?;
                }
                Ok(acc)
            }
            other => Err(format!(
                "px: updateManyAttrs expects a list, got {}",
                px_kind(other)
            )),
        }
    } else if name == "getName" {
        match &args[0] {
            PxVal::Attrs(fields) => match px_attrs_find(fields.as_ref(), "name") {
                Some(v) => match px_force(v)? {
                    PxVal::Str(s) => {
                        let (n, _) = px_parse_drv_name(&s);
                        Ok(PxVal::Str(n))
                    }
                    other => Err(format!("px: getName name must be string, got {}", px_kind(&other))),
                },
                None => Err(String::from("px: getName: attribute 'name' missing")),
            },
            PxVal::Str(s) => {
                let (n, _) = px_parse_drv_name(s);
                Ok(PxVal::Str(n))
            }
            other => Err(format!("px: getName expects attrs or string, got {}", px_kind(other))),
        }
    } else if name == "getVersion" {
        match &args[0] {
            PxVal::Attrs(fields) => {
                if let Some(v) = px_attrs_find(fields.as_ref(), "version") {
                    return px_force(v);
                }
                match px_attrs_find(fields.as_ref(), "name") {
                    Some(v) => match px_force(v)? {
                        PxVal::Str(s) => {
                            let (_, ver) = px_parse_drv_name(&s);
                            Ok(PxVal::Str(ver))
                        }
                        other => Err(format!(
                            "px: getVersion name must be string, got {}",
                            px_kind(&other)
                        )),
                    },
                    None => Err(String::from("px: getVersion: no version or name")),
                }
            }
            PxVal::Str(s) => {
                let (_, ver) = px_parse_drv_name(s);
                Ok(PxVal::Str(ver))
            }
            other => Err(format!(
                "px: getVersion expects attrs or string, got {}",
                px_kind(other)
            )),
        }
    } else if name == "filterAttrsRecursive" {
        px_filter_attrs_recursive(&args[0], &args[1])
    } else if name == "mapAttrsRecursive" {
        px_map_attrs_recursive(&args[0], &args[1], &Vec::new())
    } else if name == "intersectLists" {
        match (&args[0], &args[1]) {
            (PxVal::List(a), PxVal::List(b)) => {
                let mut out = Vec::new();
                for x in a.iter() {
                    let mut in_b = false;
                    for y in b.iter() {
                        if px_val_eq_nested(x, y)? {
                            in_b = true;
                        }
                    }
                    if in_b {
                        let mut already = false;
                        for z in out.iter() {
                            if px_val_eq_nested(x, z)? {
                                already = true;
                            }
                        }
                        if !already {
                            out.push(x.clone());
                        }
                    }
                }
                Ok(px_list(out))
            }
            _ => Err(String::from("px: intersectLists expects two lists")),
        }
    } else if name == "subtractLists" {
        // subtractLists list remove — elements of list not in remove
        match (&args[0], &args[1]) {
            (PxVal::List(a), PxVal::List(b)) => {
                let mut out = Vec::new();
                for x in a.iter() {
                    let mut in_b = false;
                    for y in b.iter() {
                        if px_val_eq_nested(x, y)? {
                            in_b = true;
                        }
                    }
                    if !in_b {
                        out.push(x.clone());
                    }
                }
                Ok(px_list(out))
            }
            _ => Err(String::from("px: subtractLists expects two lists")),
        }
    } else if name == "zipLists" {
        match (&args[0], &args[1]) {
            (PxVal::List(a), PxVal::List(b)) => {
                let n = if a.len() < b.len() { a.len() } else { b.len() };
                let mut out = Vec::new();
                let mut i = 0usize;
                while i < n {
                    out.push(px_attrs(vec![
                        (String::from("fst"), a[i].clone()),
                        (String::from("snd"), b[i].clone()),
                    ]));
                    i += 1;
                }
                Ok(px_list(out))
            }
            _ => Err(String::from("px: zipLists expects two lists")),
        }
    } else if name == "zipListsWith" {
        match (&args[1], &args[2]) {
            (PxVal::List(a), PxVal::List(b)) => {
                let n = if a.len() < b.len() { a.len() } else { b.len() };
                let mut out = Vec::new();
                let mut i = 0usize;
                while i < n {
                    let step = px_apply(&args[0], a[i].clone())?;
                    out.push(px_force(&px_apply(&step, b[i].clone())?)?);
                    i += 1;
                }
                Ok(px_list(out))
            }
            _ => Err(String::from("px: zipListsWith expects (fn, list, list)")),
        }
    } else if name == "mapAttrsToList" {
        match &args[1] {
            PxVal::Attrs(fields) => {
                let mut names = Vec::new();
                for (k, _v) in fields.iter() {
                    names.push(k.clone());
                }
                let sorted = px_sort_strings(names);
                let mut out = Vec::new();
                for n in sorted {
                    for (k, v) in fields.iter() {
                        if *k == n {
                            let step = px_apply(&args[0], PxVal::Str(k.clone()))?;
                            out.push(px_force(&px_apply(&step, v.clone())?)?);
                        }
                    }
                }
                Ok(px_list(out))
            }
            other => Err(format!(
                "px: mapAttrsToList expects an attrset, got {}",
                px_kind(other)
            )),
        }
    } else if name == "zipAttrs" {
        // zipAttrs list = zipAttrsWith (_: values: values) list
        let f = PxVal::Closure {
            param: String::from("name"),
            body: std::rc::Rc::new(PxExpr::Lambda {
                param: String::from("values"),
                body: std::rc::Rc::new(PxExpr::Var(String::from("values"))),
            }),
            env: Vec::new(),
        };
        px_builtin_exec("zipAttrsWith", &vec![f, args[0].clone()])
    } else if name == "assertMsg" {
        match &args[0] {
            PxVal::Bool(true) => Ok(args[1].clone()),
            PxVal::Bool(false) => match &args[1] {
                PxVal::Str(msg) => Err(format!("px: assertion failed: {}", msg)),
                _ => Err(String::from("px: assertion failed")),
            },
            other => Err(format!("px: assert expects a bool, got {}", px_kind(other))),
        }
    } else if name == "match" {
        // Oracle: a contextful REGEX is an error (falls to the catch-all
        // below, since a ctx-string never matches PxVal::Str); a contextful
        // SUBJECT is accepted and its captures come back context-free (Nix
        // collects but drops the subject's context on match results).
        match (&args[0], &args[1]) {
            (PxVal::Str(re), PxVal::Str(sub)) => px_match(re, sub),
            (PxVal::Str(re), _) if px_is_ctx_string(&args[1]) => {
                px_match(re, &px_string_like_content_or_empty(&args[1]))
            }
            _ => Err(String::from("px: match expects (string, string)")),
        }
    } else if name == "split" {
        // Same oracle contract as match.
        match (&args[0], &args[1]) {
            (PxVal::Str(re), PxVal::Str(sub)) => px_split(re, sub),
            (PxVal::Str(re), _) if px_is_ctx_string(&args[1]) => {
                px_split(re, &px_string_like_content_or_empty(&args[1]))
            }
            _ => Err(String::from("px: split expects (string, string)")),
        }
    } else if name == "fromJSON" {
        match &args[0] {
            PxVal::Str(txt) => px_from_json(txt),
            other => Err(format!("px: fromJSON expects a string, got {}", px_kind(other))),
        }
    } else if name == "toJSON" {
        let mut ctx: Vec<String> = Vec::new();
        let json = px_to_json_ctx(&args[0], &mut ctx)?;
        Ok(px_ctx_string(json, ctx))
    } else if name == "isInt" {
        match &args[0] {
            PxVal::Int(_) => Ok(PxVal::Bool(true)),
            _ => Ok(PxVal::Bool(false)),
        }
    } else if name == "isBool" {
        match &args[0] {
            PxVal::Bool(_) => Ok(PxVal::Bool(true)),
            _ => Ok(PxVal::Bool(false)),
        }
    } else if name == "isString" {
        match &args[0] {
            PxVal::Str(_) => Ok(PxVal::Bool(true)),
            PxVal::Attrs(_) if px_is_ctx_string(&args[0]) => Ok(PxVal::Bool(true)),
            _ => Ok(PxVal::Bool(false)),
        }
    } else if name == "isList" {
        match &args[0] {
            PxVal::List(_) => Ok(PxVal::Bool(true)),
            _ => Ok(PxVal::Bool(false)),
        }
    // ---- Cross-host consensus tranche (2026-08-19) --------------------
    // Every builtin below was oracle-pinned against >=3 of the 4 reference
    // hosts (pnix-clj/pnix-clr/pnix-cljs/pnix-hy) before being ported here.
    } else if name == "cons" {
        match &args[1] {
            PxVal::List(items) => {
                let mut out = Vec::new();
                out.push(args[0].clone());
                for it in items.iter() {
                    out.push(it.clone());
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: cons expects a list, got {}", px_kind(other))),
        }
    } else if name == "append" {
        match (&args[0], &args[1]) {
            (PxVal::List(a), PxVal::List(b)) => {
                let mut out = Vec::new();
                for it in a.iter() {
                    out.push(it.clone());
                }
                for it in b.iter() {
                    out.push(it.clone());
                }
                Ok(px_list(out))
            }
            _ => Err(String::from("px: append expects two lists")),
        }
    } else if name == "drop" {
        match (&args[0], &args[1]) {
            (PxVal::Int(n), PxVal::List(items)) => {
                let n = if *n < 0 { 0usize } else { *n as usize };
                let mut out = Vec::new();
                let mut i = 0usize;
                while i < items.len() {
                    if i >= n {
                        out.push(items[i].clone());
                    }
                    i += 1;
                }
                Ok(px_list(out))
            }
            _ => Err(String::from("px: drop expects (int, list)")),
        }
    } else if name == "take" {
        match (&args[0], &args[1]) {
            (PxVal::Int(n), PxVal::List(items)) => {
                let n = if *n < 0 { 0usize } else { *n as usize };
                let mut out = Vec::new();
                let mut i = 0usize;
                while i < items.len() && i < n {
                    out.push(items[i].clone());
                    i += 1;
                }
                Ok(px_list(out))
            }
            _ => Err(String::from("px: take expects (int, list)")),
        }
    } else if name == "find" {
        // find needle list: linear scan for a structurally-equal element;
        // the element itself (not a bool) is returned, or null.
        match &args[1] {
            PxVal::List(items) => {
                for it in items.iter() {
                    if px_val_eq_nested(&args[0], it)? {
                        return px_force(it);
                    }
                }
                Ok(PxVal::Null)
            }
            other => Err(format!("px: find expects a list, got {}", px_kind(other))),
        }
    } else if name == "findFirst" {
        match &args[2] {
            PxVal::List(items) => {
                for it in items.iter() {
                    let keep = px_force(&px_apply(&args[0], it.clone())?)?;
                    match keep {
                        PxVal::Bool(true) => return px_force(it),
                        PxVal::Bool(false) => {}
                        other => {
                            return Err(format!(
                                "px: findFirst predicate must return bool, got {}",
                                px_kind(&other)
                            ))
                        }
                    }
                }
                Ok(args[1].clone())
            }
            other => Err(format!("px: findFirst expects a list, got {}", px_kind(other))),
        }
    } else if name == "reverseList" {
        match &args[0] {
            PxVal::List(items) => {
                let mut out = Vec::new();
                let mut i = items.len();
                while i > 0 {
                    i -= 1;
                    out.push(items[i].clone());
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: reverseList expects a list, got {}", px_kind(other))),
        }
    } else if name == "replicate" {
        match &args[0] {
            PxVal::Int(n) => {
                if *n < 0 {
                    return Err(String::from("px: replicate: negative count"));
                }
                let mut out = Vec::new();
                let mut i = 0i64;
                while i < *n {
                    out.push(args[1].clone());
                    i += 1;
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: replicate expects an int, got {}", px_kind(other))),
        }
    } else if name == "zip" {
        match (&args[0], &args[1]) {
            (PxVal::List(a), PxVal::List(b)) => {
                let n = if a.len() < b.len() { a.len() } else { b.len() };
                let mut out = Vec::new();
                let mut i = 0usize;
                while i < n {
                    out.push(px_list(vec![a[i].clone(), b[i].clone()]));
                    i += 1;
                }
                Ok(px_list(out))
            }
            _ => Err(String::from("px: zip expects two lists")),
        }
    } else if name == "keys" {
        px_builtin_exec("attrNames", &vec![args[0].clone()])
    } else if name == "values" {
        px_builtin_exec("attrValues", &vec![args[0].clone()])
    } else if name == "merge" {
        px_binary_outcome(&PxOp::Update, &args[0], &args[1]).map_err(|e| e.diagnostic)
    } else if name == "genAttrs" {
        match &args[0] {
            PxVal::List(names) => {
                let mut out = Vec::new();
                for n in names.iter() {
                    let n = px_force(n)?;
                    match &n {
                        PxVal::Str(k) => {
                            out.push((k.clone(), px_defer_apply(args[1].clone(), PxVal::Str(k.clone()))));
                        }
                        other => {
                            return Err(format!(
                                "px: genAttrs expects a list of strings, got {}",
                                px_kind(other)
                            ))
                        }
                    }
                }
                Ok(px_attrs(out))
            }
            other => Err(format!("px: genAttrs expects a list, got {}", px_kind(other))),
        }
    } else if name == "foldlAttrs" {
        // foldlAttrs op init attrs — visits keys in sorted order.
        match &args[2] {
            PxVal::Attrs(fields) => {
                let mut names = Vec::new();
                for (k, _v) in fields.iter() {
                    names.push(k.clone());
                }
                let sorted = px_sort_strings(names);
                let mut acc = args[1].clone();
                for n in sorted {
                    for (k, v) in fields.iter() {
                        if *k == n {
                            let step = px_apply(&args[0], acc)?;
                            let step = px_apply(&step, PxVal::Str(k.clone()))?;
                            acc = px_force(&px_apply(&step, v.clone())?)?;
                        }
                    }
                }
                Ok(acc)
            }
            other => Err(format!("px: foldlAttrs expects an attrset, got {}", px_kind(other))),
        }
    } else if name == "genericClosure" {
        match &args[0] {
            PxVal::Attrs(fields) => {
                let operator = match px_attrs_find(fields.as_ref(), "operator") {
                    Some(v) => v.clone(),
                    None => return Err(String::from(
                        "px: genericClosure: missing attribute 'operator'",
                    )),
                };
                let start_set = match px_attrs_find(fields.as_ref(), "startSet") {
                    Some(v) => px_force(v)?,
                    None => return Err(String::from(
                        "px: genericClosure: missing attribute 'startSet'",
                    )),
                };
                let mut worklist: Vec<PxVal> = match &start_set {
                    PxVal::List(items) => {
                        let mut out = Vec::new();
                        for it in items.iter() {
                            out.push(it.clone());
                        }
                        out
                    }
                    other => {
                        return Err(format!(
                            "px: genericClosure: startSet must be a list, got {}",
                            px_kind(other)
                        ))
                    }
                };
                let mut seen: Vec<PxVal> = Vec::new();
                let mut result: Vec<PxVal> = Vec::new();
                let mut i = 0usize;
                while i < worklist.len() {
                    let item = px_force(&worklist[i])?;
                    let item_fields = match &item {
                        PxVal::Attrs(fs) => fs,
                        other => {
                            return Err(format!(
                                "px: genericClosure: item must be an attrset, got {}",
                                px_kind(other)
                            ))
                        }
                    };
                    let key = match px_attrs_find(item_fields.as_ref(), "key") {
                        Some(k) => px_force(k)?,
                        None => return Err(String::from(
                            "px: genericClosure: item missing attribute 'key'",
                        )),
                    };
                    let mut already = false;
                    for s in seen.iter() {
                        if px_val_eq_nested(s, &key)? {
                            already = true;
                        }
                    }
                    if !already {
                        seen.push(key);
                        result.push(item.clone());
                        let next = px_force(&px_apply(&operator, item.clone())?)?;
                        match &next {
                            PxVal::List(next_items) => {
                                for it in next_items.iter() {
                                    worklist.push(it.clone());
                                }
                            }
                            other => {
                                return Err(format!(
                                    "px: genericClosure: operator must return a list, got {}",
                                    px_kind(other)
                                ))
                            }
                        }
                    }
                    i += 1;
                }
                Ok(px_list(result))
            }
            other => Err(format!(
                "px: genericClosure expects an attrset, got {}",
                px_kind(other)
            )),
        }
    } else if name == "nameValuePair" {
        match &args[0] {
            PxVal::Str(_) => Ok(px_attrs(vec![
                (String::from("name"), args[0].clone()),
                (String::from("value"), args[1].clone()),
            ])),
            other => Err(format!(
                "px: nameValuePair expects a string name, got {}",
                px_kind(other)
            )),
        }
    } else if name == "concatStrings" {
        // Context-aware: contents join exactly as before, element contexts
        // union onto the output.
        match &args[0] {
            PxVal::List(items) => {
                let mut out = String::new();
                let mut ctx: Vec<String> = Vec::new();
                for it in items.iter() {
                    let it = px_force(it)?;
                    match &it {
                        PxVal::Str(s) => out.push_str(s),
                        _ if px_is_ctx_string(&it) => {
                            out.push_str(&px_string_like_content_or_empty(&it));
                            ctx.extend(px_string_like_context(&it));
                        }
                        other => {
                            return Err(format!(
                                "px: concatStrings expects strings, got {}",
                                px_kind(other)
                            ))
                        }
                    }
                }
                Ok(px_ctx_string(out, ctx))
            }
            other => Err(format!("px: concatStrings expects a list, got {}", px_kind(other))),
        }
    } else if name == "concatMapStrings" {
        // concatMapStrings f xs = concatStrings (map f xs); context-aware —
        // contexts of the mapped results union onto the output.
        match &args[1] {
            PxVal::List(items) => {
                let mut out = String::new();
                let mut ctx: Vec<String> = Vec::new();
                for it in items.iter() {
                    let r = px_force(&px_apply(&args[0], it.clone())?)?;
                    if px_is_ctx_string(&r) {
                        out.push_str(&px_string_like_content_or_empty(&r));
                        ctx.extend(px_string_like_context(&r));
                    } else if let PxVal::Str(s) = px_to_string_coerce(&r)? {
                        out.push_str(&s);
                    }
                }
                Ok(px_ctx_string(out, ctx))
            }
            other => Err(format!(
                "px: concatMapStrings expects a list, got {}",
                px_kind(other)
            )),
        }
    } else if name == "stringToCharacters" {
        // lib.stringToCharacters is substring-based, and substring keeps
        // the whole context — so each character carries the source's full
        // context.
        match &args[0] {
            PxVal::Str(s) => {
                let mut out = Vec::new();
                for c in s.chars() {
                    let mut t = String::new();
                    t.push(c);
                    out.push(PxVal::Str(t));
                }
                Ok(px_list(out))
            }
            _ if px_is_ctx_string(&args[0]) => {
                let ctx = px_string_like_context(&args[0]);
                let content = px_string_like_content_or_empty(&args[0]);
                let mut out = Vec::new();
                for c in content.chars() {
                    let mut t = String::new();
                    t.push(c);
                    out.push(px_ctx_string(t, ctx.clone()));
                }
                Ok(px_list(out))
            }
            other => Err(format!(
                "px: stringToCharacters expects a string, got {}",
                px_kind(other)
            )),
        }
    } else if name == "hasInfix" {
        // Content-based predicate: context does not affect the answer.
        if px_is_ctx_string(&args[0]) || px_is_ctx_string(&args[1]) {
            return match (px_string_like_content(&args[0]), px_string_like_content(&args[1])) {
                (Some(needle), Some(hay)) => Ok(PxVal::Bool(px_str_has_infix(&hay, &needle))),
                _ => Err(String::from("px: hasInfix expects two strings")),
            };
        }
        match (&args[0], &args[1]) {
            (PxVal::Str(needle), PxVal::Str(hay)) => Ok(PxVal::Bool(px_str_has_infix(hay, needle))),
            _ => Err(String::from("px: hasInfix expects two strings")),
        }
    } else if name == "optionalString" {
        match &args[0] {
            PxVal::Bool(true) => match &args[1] {
                PxVal::Str(s) => Ok(PxVal::Str(s.clone())),
                other => Err(format!(
                    "px: optionalString expects a string, got {}",
                    px_kind(other)
                )),
            },
            PxVal::Bool(false) => Ok(PxVal::Str(String::new())),
            other => Err(format!("px: optionalString expects a bool, got {}", px_kind(other))),
        }
    } else if name == "imap0" {
        match &args[1] {
            PxVal::List(items) => {
                let mut out = Vec::new();
                let mut i = 0i64;
                for it in items.iter() {
                    let step = px_apply(&args[0], PxVal::Int(i))?;
                    out.push(px_defer_apply(step, it.clone()));
                    i += 1;
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: imap0 expects a list, got {}", px_kind(other))),
        }
    } else if name == "imap1" {
        match &args[1] {
            PxVal::List(items) => {
                let mut out = Vec::new();
                let mut i = 1i64;
                for it in items.iter() {
                    let step = px_apply(&args[0], PxVal::Int(i))?;
                    out.push(px_defer_apply(step, it.clone()));
                    i += 1;
                }
                Ok(px_list(out))
            }
            other => Err(format!("px: imap1 expects a list, got {}", px_kind(other))),
        }
    } else if name == "toInt" {
        match &args[0] {
            PxVal::Str(s) => match s.parse::<i64>() {
                Ok(n) => Ok(PxVal::Int(n)),
                Err(_) => Err(format!("px: toInt: not an integer: '{}'", s)),
            },
            // Content parses to the integer; the result carries no context
            // (oracle-pinned).
            _ if px_is_ctx_string(&args[0]) => {
                let content = px_string_like_content_or_empty(&args[0]);
                match content.parse::<i64>() {
                    Ok(n) => Ok(PxVal::Int(n)),
                    Err(_) => Err(format!("px: toInt: not an integer: '{}'", content)),
                }
            }
            other => Err(format!("px: toInt expects a string, got {}", px_kind(other))),
        }
    } else if name == "derivationStrict" {
        // Low-level derivation primitive: drvPath plus one attr per output
        // (oracle: attrNames = ["dev" "drvPath" "out"] for outputs=["out"
        // "dev"]), each carrying its own string context — drvPath
        // allOutputs ("=<drvPath>"), an output path its own output
        // ("!<o>!<drvPath>").
        let (_forced, _name, outputs, drv_path, out_paths) =
            px_derivation_core(name, &args[0])?;
        let mut fields = vec![(
            String::from("drvPath"),
            px_ctx_string(drv_path.clone(), vec![format!("={}", drv_path)]),
        )];
        let mut i = 0usize;
        while i < outputs.len() {
            let o = &outputs[i];
            fields.push((
                o.clone(),
                px_ctx_string(
                    String::from(px_out_path_for(&out_paths, o)),
                    vec![format!("!{}!{}", o, drv_path)],
                ),
            ));
            i += 1;
        }
        Ok(px_attrs(fields))
    } else if name == "derivation" {
        // High-level wrapper (in real Nix a nix-lang wrapper over
        // derivationStrict): the input attrs merged with
        // type/name/drvPath/outPath/outputName, plus one attr per output.
        // outPath/outputName follow the FIRST output (oracle:
        // outputs=["dev" "out"] -> outputName="dev"). Each d.<o> is a
        // NON-cyclic reduced derivation attrset (type/name/drvPath/
        // outPath/outputName only) — real Nix's `d.out == d`
        // self-reference is not representable in this plain-Attrs value
        // model; documented simulation limit (see docs/BUGS.md).
        let (forced, drv_name, outputs, drv_path, out_paths) =
            px_derivation_core(name, &args[0])?;
        let drv_ctx = px_ctx_string(drv_path.clone(), vec![format!("={}", drv_path)]);
        let out_attr = |o: &str| -> PxVal {
            px_ctx_string(
                String::from(px_out_path_for(&out_paths, o)),
                vec![format!("!{}!{}", o, drv_path)],
            )
        };
        let forced_fields = match &forced {
            PxVal::Attrs(f) => f.as_ref().clone(),
            _ => Vec::new(),
        };
        let mut fields = forced_fields;
        let mut i = 0usize;
        while i < outputs.len() {
            let o = &outputs[i];
            let sub_drv = px_attrs(vec![
                (String::from("type"), PxVal::Str(String::from("derivation"))),
                (String::from("name"), PxVal::Str(drv_name.clone())),
                (String::from("drvPath"), drv_ctx.clone()),
                (String::from("outputName"), PxVal::Str(o.clone())),
                (String::from("outPath"), out_attr(o)),
            ]);
            px_fields_set(&mut fields, o.clone(), sub_drv);
            i += 1;
        }
        let first_o = outputs[0].clone();
        px_fields_set(&mut fields, String::from("type"), PxVal::Str(String::from("derivation")));
        px_fields_set(&mut fields, String::from("drvPath"), drv_ctx);
        px_fields_set(&mut fields, String::from("outPath"), out_attr(&first_o));
        px_fields_set(&mut fields, String::from("outputName"), PxVal::Str(first_o));
        Ok(px_attrs(fields))
    } else if name == "placeholder" {
        match &args[0] {
            PxVal::Str(output) => {
                let bytes = sha_utf8_bytes(&format!("pnix-output:{}", output));
                let hex = px_sha256_hex(bytes);
                let hex_chars: Vec<char> = hex.chars().collect();
                let mut prefix = String::new();
                let mut i = 0usize;
                while i < 32 && i < hex_chars.len() {
                    prefix.push(hex_chars[i]);
                    i += 1;
                }
                Ok(PxVal::Str(format!("/{}", prefix)))
            }
            other => Err(format!("px: placeholder expects a string, got {}", px_kind(other))),
        }
    } else if name == "storePath" {
        // Pure evaluator: no store to resolve a store path against (matches
        // the clr/cljs/clj majority: this always fails closed here too).
        Err(String::from("px: storePath: pure evaluator has no store"))
    } else if name == "getEnv" {
        match &args[0] {
            PxVal::Str(k) => match std::env::var(k) {
                Ok(v) => Ok(PxVal::Str(v)),
                Err(_) => Ok(PxVal::Str(String::new())),
            },
            other => Err(format!("px: getEnv expects a string, got {}", px_kind(other))),
        }
    } else if name == "and" {
        match (&args[0], &args[1]) {
            (PxVal::Bool(a), PxVal::Bool(b)) => Ok(PxVal::Bool(*a && *b)),
            _ => Err(String::from("px: and expects two bools")),
        }
    } else if name == "or" {
        match (&args[0], &args[1]) {
            (PxVal::Bool(a), PxVal::Bool(b)) => Ok(PxVal::Bool(*a || *b)),
            _ => Err(String::from("px: or expects two bools")),
        }
    } else if name == "not" {
        match &args[0] {
            PxVal::Bool(a) => Ok(PxVal::Bool(!*a)),
            other => Err(format!("px: not expects a bool, got {}", px_kind(other))),
        }
    } else if name == "eq" {
        px_binary_outcome(&PxOp::Eq, &args[0], &args[1]).map_err(|e| e.diagnostic)
    } else if name == "lt" {
        px_binary_outcome(&PxOp::Lt, &args[0], &args[1]).map_err(|e| e.diagnostic)
    } else if name == "le" {
        px_binary_outcome(&PxOp::Le, &args[0], &args[1]).map_err(|e| e.diagnostic)
    } else if name == "gt" {
        px_binary_outcome(&PxOp::Gt, &args[0], &args[1]).map_err(|e| e.diagnostic)
    } else if name == "ge" {
        px_binary_outcome(&PxOp::Ge, &args[0], &args[1]).map_err(|e| e.diagnostic)
    } else if name == "neg" {
        px_binary_outcome(&PxOp::Sub, &PxVal::Int(0), &args[0]).map_err(|e| e.diagnostic)
    } else if name == "get" {
        match (&args[0], &args[1]) {
            (PxVal::Attrs(fields), PxVal::Str(k)) => match px_attrs_find(fields.as_ref(), k) {
                Some(value) => Ok(value.clone()),
                None => Err(format!("px: get: attribute '{}' missing", k)),
            },
            _ => Err(String::from("px: get expects (attrset, string)")),
        }
    } else if name == "set" {
        match (&args[0], &args[1]) {
            (PxVal::Attrs(fields), PxVal::Str(k)) => {
                let mut out = Vec::new();
                let mut replaced = false;
                for (fk, fv) in fields.iter() {
                    if fk == k {
                        out.push((fk.clone(), args[2].clone()));
                        replaced = true;
                    } else {
                        out.push((fk.clone(), fv.clone()));
                    }
                }
                if !replaced {
                    out.push((k.clone(), args[2].clone()));
                }
                Ok(px_attrs(out))
            }
            _ => Err(String::from("px: set expects (attrset, string, value)")),
        }
    } else {
        Err(format!("px: unknown builtin {}", name))
    }
}

/// Scalar equality for `elem` (ints, bools, strings).
fn px_val_eq(a: &PxVal, b: &PxVal) -> Result<bool, String> {
    px_val_eq_mode(a, b, false)
}

fn px_val_eq_nested(a: &PxVal, b: &PxVal) -> Result<bool, String> {
    px_val_eq_mode(a, b, true)
}

fn px_val_eq_mode(a: &PxVal, b: &PxVal, allow_identity: bool) -> Result<bool, String> {
    // Nix equality short-circuits shared values before descending. This is
    // observable for a shared NaN thunk inside two otherwise distinct
    // containers: the shared leaf compares equal there even though `n == n`
    // as two forced scalar operands remains false.
    let identical = match (a, b) {
        (PxVal::List(x), PxVal::List(y)) if allow_identity => Rc::ptr_eq(x, y),
        (PxVal::Thunk(x), PxVal::Thunk(y)) if allow_identity => {
            Rc::ptr_eq(x, y) || px_same_binding_thunk(x, y)
        }
        (PxVal::Attrs(x), PxVal::Attrs(y)) if allow_identity => Rc::ptr_eq(x, y),
        _ => false,
    };
    if identical {
        // A shared scalar thunk must still be forced: errors propagate, while
        // a successfully forced NaN/function keeps nested identity equality.
        if let PxVal::Thunk(_) = a {
            px_force(a)?;
        }
        return Ok(true);
    }
    // Equality forces both sides to WHNF (Nix); the recursive calls force
    // nested attrset-field / list-element thunks in turn.
    let a = px_force(a)?;
    let b = px_force(b)?;
    match (&a, &b) {
        (PxVal::Int(x), PxVal::Int(y)) => Ok(*x == *y),
        (PxVal::Int(x), PxVal::Float(y)) => Ok((*x as f64) == *y),
        (PxVal::Float(x), PxVal::Int(y)) => Ok(*x == (*y as f64)),
        (PxVal::Float(x), PxVal::Float(y)) => Ok(*x == *y),
        (PxVal::Bool(x), PxVal::Bool(y)) => Ok(*x == *y),
        (PxVal::Null, PxVal::Null) => Ok(true),
        (PxVal::Str(x), PxVal::Str(y)) => Ok(x == y),
        // Every PxVal::Path is normalized at construction, so plain string
        // comparison here already IS normalized-path comparison.
        (PxVal::Path(x), PxVal::Path(y)) => Ok(x == y),
        (PxVal::Bytes(_), PxVal::Str(_))
        | (PxVal::Str(_), PxVal::Bytes(_))
        | (PxVal::Bytes(_), PxVal::Bytes(_)) => {
            match (px_val_bytes(&a), px_val_bytes(&b)) {
                (Some(x), Some(y)) => Ok(x == y),
                _ => Ok(false),
            }
        }
        // Nix string equality compares character content only; context
        // rides along without participating (oracle-confirmed, matches
        // pnix-clj's `nix-equal-result`). Guarded so genuine attrset
        // structural equality below is unaffected.
        (PxVal::Attrs(_), _) | (_, PxVal::Attrs(_))
            if px_is_ctx_string(&a) || px_is_ctx_string(&b) =>
        {
            Ok(px_string_like_content(&a) == px_string_like_content(&b))
        }
        // Nix semantics: deep structural equality; lists elementwise,
        // attrsets by name set + per-name value equality (order-insensitive).
        (PxVal::List(xs), PxVal::List(ys)) => {
            if xs.len() != ys.len() {
                return Ok(false);
            }
            let mut i = 0usize;
            while i < xs.len() {
                if !px_val_eq_nested(&xs[i], &ys[i])? {
                    return Ok(false);
                }
                i += 1;
            }
            Ok(true)
        }
        (PxVal::Attrs(xs), PxVal::Attrs(ys)) => {
            // Sorted invariant (proposal 0002): positional zip compare.
            // Positions are metadata and must not participate in equality.
            let xs = px_split_attr_pos(xs).0;
            let ys = px_split_attr_pos(ys).0;
            if xs.len() != ys.len() {
                return Ok(false);
            }
            let mut i = 0usize;
            while i < xs.len() {
                if xs[i].0 != ys[i].0 {
                    return Ok(false);
                }
                if !px_val_eq_nested(&xs[i].1, &ys[i].1)? {
                    return Ok(false);
                }
                i += 1;
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

enum PxBindingIdentity {
    Bound(PxVal),
    Rec(Rc<RefCell<Vec<Option<Result<PxVal, PxError>>>>>, usize),
}

fn px_var_binding_identity(
    expr: &PxExpr,
    env: &Vec<PxFrame>,
) -> Option<PxBindingIdentity> {
    let name = match expr {
        PxExpr::Var(n) => n,
        _ => return None,
    };
    let mut i = env.len();
    while i > 0 {
        i -= 1;
        match &env[i] {
            PxFrame::Bind { name: bound, value } => {
                if bound == name {
                    return Some(PxBindingIdentity::Bound(value.clone()));
                }
            }
            PxFrame::Rec(bindings, cache) => {
                let mut j = bindings.len();
                while j > 0 {
                    j -= 1;
                    if bindings[j].0 == *name {
                        return Some(PxBindingIdentity::Rec(Rc::clone(cache), j));
                    }
                }
            }
            PxFrame::With(_) => {}
        }
    }
    None
}

fn px_same_binding_thunk(
    x: &Rc<RefCell<PxThunk>>,
    y: &Rc<RefCell<PxThunk>>,
) -> bool {
    let mut left = Rc::clone(x);
    let mut right = Rc::clone(y);
    let mut seen: Vec<(Rc<RefCell<PxThunk>>, Rc<RefCell<PxThunk>>)> = Vec::new();
    loop {
        for (sx, sy) in seen.iter() {
            if Rc::ptr_eq(sx, &left) && Rc::ptr_eq(sy, &right) {
                return false;
            }
        }
        seen.push((Rc::clone(&left), Rc::clone(&right)));
        let xs = left.borrow().clone();
        let ys = right.borrow().clone();
        match (xs, ys) {
            (PxThunk::Unforced(xe, xenv), PxThunk::Unforced(ye, yenv)) => {
                let xb = px_var_binding_identity(xe.as_ref(), xenv.as_ref());
                let yb = px_var_binding_identity(ye.as_ref(), yenv.as_ref());
                match (xb, yb) {
                    (
                        Some(PxBindingIdentity::Rec(xc, xi)),
                        Some(PxBindingIdentity::Rec(yc, yi)),
                    ) => return xi == yi && Rc::ptr_eq(&xc, &yc),
                    (
                        Some(PxBindingIdentity::Bound(xv)),
                        Some(PxBindingIdentity::Bound(yv)),
                    ) => match (&xv, &yv) {
                        (PxVal::Thunk(xc), PxVal::Thunk(yc)) => {
                            if Rc::ptr_eq(xc, yc) {
                                return true;
                            }
                            left = Rc::clone(xc);
                            right = Rc::clone(yc);
                        }
                        (PxVal::List(xc), PxVal::List(yc)) => {
                            return Rc::ptr_eq(xc, yc)
                        }
                        (PxVal::Attrs(xc), PxVal::Attrs(yc)) => {
                            return Rc::ptr_eq(xc, yc)
                        }
                        _ => return false,
                    },
                    _ => return false,
                }
            }
            _ => return false,
        }
    }
}

fn px_val_lt(a: &PxVal, b: &PxVal) -> Result<bool, String> {
    let a = px_force(a)?;
    let b = px_force(b)?;
    match (&a, &b) {
        (PxVal::Int(x), PxVal::Int(y)) => Ok(x < y),
        (PxVal::Int(x), PxVal::Float(y)) => Ok((*x as f64) < *y),
        (PxVal::Float(x), PxVal::Int(y)) => Ok(*x < (*y as f64)),
        (PxVal::Float(x), PxVal::Float(y)) => Ok(x < y),
        (PxVal::Str(x), PxVal::Str(y)) => Ok(x < y),
        (PxVal::Path(x), PxVal::Path(y)) => Ok(x < y),
        // Context-bearing strings order by content only (context never
        // participates), matching pnix-clj's `pnix-less-than-result`.
        (PxVal::Attrs(_), _) | (_, PxVal::Attrs(_))
            if px_is_ctx_string(&a) || px_is_ctx_string(&b) =>
        {
            match (px_string_like_content(&a), px_string_like_content(&b)) {
                (Some(x), Some(y)) => Ok(x < y),
                _ => Err(format!(
                    "px: cannot compare {} and {}",
                    px_kind(&a),
                    px_kind(&b)
                )),
            }
        }
        (PxVal::List(xs), PxVal::List(ys)) => {
            let mut i = 0usize;
            while i < xs.len() && i < ys.len() {
                if px_val_eq_nested(&xs[i], &ys[i])? {
                    i += 1;
                } else {
                    return px_val_lt(&xs[i], &ys[i]);
                }
            }
            Ok(xs.len() < ys.len())
        }
        _ => Err(format!(
            "px: cannot compare {} and {}",
            px_kind(&a),
            px_kind(&b)
        )),
    }
}



// ---- builtins.match / builtins.split (oracle-pinned 2026-07-08) -------------
// POSIX-ERE-flavored engine (backtracking, first-successful-alternative --
// NOT a POSIX-leftmost-longest DFA; live cross-checked against
// `nix-instantiate` and real Nix's own regex backend isn't POSIX-longest
// either, e.g. `builtins.match "(a|ab)(c|bcd)(d*)" "abcd"` picks the FIRST
// alternative that leads to an overall match on both, `[ "a" "bcd" "" ]` --
// so backtracking-first-match is the oracle-observed behavior here, not a
// gap against it). Supports: literals+escapes (backslash always takes the
// next char literally -- no `\d`/`\w`/`\s` shorthand, POSIX ERE has none),
// `.`, bracket expressions incl. negation/ranges/named `[:class:]`, groups
// with capture + alternation, `^`/`$` anchors (true start/end of the whole
// subject, evaluated wherever they sit in the pattern -- inside a group or
// an alternation branch works the same as at the top level), and
// 2026-08-20's addition: `*`/`+`/`?` now apply to ANY operand including a
// parenthesized group (not just a single char/class/`.`), plus
// bounded-repetition intervals `{m}`/`{m,}`/`{m,n}` (desugared at parse
// time into mandatory copies + `?`/`*`, so the matcher itself only ever
// sees Star/Plus/Opt -- see try_parse_interval). `{` is a RESERVED
// character here, same as real Nix's own backend (cross-checked live:
// `a{`, `a{,3}`, `a{x}`, `a{}`, and a bare `{3}` with no operand all raise
// "invalid regular expression" there too) -- there is no literal-`{`
// fallback for anything short of a complete `{m}`/`{m,}`/`{m,n}`. KNOWN,
// DELIBERATELY OUT-OF-SCOPE LIMITATION: when a successful match must back
// off a Group repetition's count after an initial greedy walk, a capture
// nested inside that group keeps whichever repetition the greedy walk
// left behind rather than recomputing it for the backed-off count -- true
// correctness there needs re-running the matched span, out of scope for
// this backtracking (not leftmost-longest) engine. Unsupported syntax
// errors fail-closed. match: full-anchored; null on no match; capture list
// otherwise (null for an unmatched optional/zero-repetition group). split:
// scan; empty match consumes one char into the next piece (oracle: split
// "x*" "ab" == ["" [] "a" [] "b" [] ""]).

#[derive(Clone, Debug)]
enum RxNode {
    Ch(char),
    Any,
    Class { neg: bool, lo: Vec<char>, hi: Vec<char> },
    Star(Box<RxNode>),
    Plus(Box<RxNode>),
    Opt(Box<RxNode>),
    Group { idx: usize, alts: Vec<Vec<RxNode>> },
    GroupEnd(usize),
    Start,
    End,
}

fn rx_single_ok(n: &RxNode) -> bool {
    matches!(n, RxNode::Ch(_) | RxNode::Any | RxNode::Class { .. })
}

fn rx_single_match(n: &RxNode, c: char) -> bool {
    match n {
        RxNode::Ch(x) => *x == c,
        RxNode::Any => true,
        RxNode::Class { neg, lo, hi } => {
            let mut hit = false;
            let mut i = 0usize;
            while i < lo.len() {
                if c >= lo[i] && c <= hi[i] {
                    hit = true;
                }
                i += 1;
            }
            if *neg { !hit } else { hit }
        }
        _ => false,
    }
}

fn rx_class_range(lo: &mut Vec<char>, hi: &mut Vec<char>, a: char, b: char) {
    lo.push(a);
    hi.push(b);
}

/// Expand the standard POSIX named classes using the C-locale ASCII ranges
/// observed from Nix 2.34.7. The outer bracket expression owns negation, so a
/// token such as `[:space:]` contributes only its positive member ranges.
fn rx_add_posix_class(
    name: &str,
    lo: &mut Vec<char>,
    hi: &mut Vec<char>,
) -> Result<(), String> {
    match name {
        "alnum" => {
            rx_class_range(lo, hi, 'A', 'Z');
            rx_class_range(lo, hi, 'a', 'z');
            rx_class_range(lo, hi, '0', '9');
        }
        "alpha" => {
            rx_class_range(lo, hi, 'A', 'Z');
            rx_class_range(lo, hi, 'a', 'z');
        }
        "blank" => {
            rx_class_range(lo, hi, '\t', '\t');
            rx_class_range(lo, hi, ' ', ' ');
        }
        "cntrl" => {
            rx_class_range(
                lo,
                hi,
                '\0',
                char::from_u32(31).unwrap_or('?'),
            );
            rx_class_range(
                lo,
                hi,
                char::from_u32(127).unwrap_or('?'),
                char::from_u32(127).unwrap_or('?'),
            );
        }
        "digit" => rx_class_range(lo, hi, '0', '9'),
        "graph" => rx_class_range(lo, hi, '!', '~'),
        "lower" => rx_class_range(lo, hi, 'a', 'z'),
        "print" => rx_class_range(lo, hi, ' ', '~'),
        "punct" => {
            rx_class_range(lo, hi, '!', '/');
            rx_class_range(lo, hi, ':', '@');
            rx_class_range(lo, hi, '[', '`');
            rx_class_range(lo, hi, '{', '~');
        }
        "space" => {
            rx_class_range(lo, hi, '\t', '\r');
            rx_class_range(lo, hi, ' ', ' ');
        }
        "upper" => rx_class_range(lo, hi, 'A', 'Z'),
        "xdigit" => {
            rx_class_range(lo, hi, '0', '9');
            rx_class_range(lo, hi, 'A', 'F');
            rx_class_range(lo, hi, 'a', 'f');
        }
        _ => {
            return Err(format!(
                "px: regex: unknown POSIX character class {}",
                name
            ))
        }
    }
    Ok(())
}

struct RxParser {
    pat: Vec<char>,
    pos: usize,
    ngroups: usize,
}

impl RxParser {
    #[inline]
    fn char_at(&self, idx: usize) -> Option<char> {
        self.pat.iter().skip(idx).next().copied()
    }

    #[inline]
    fn cur(&self) -> Option<char> {
        self.char_at(self.pos)
    }

    #[inline]
    fn peek(&self, off: usize) -> Option<char> {
        self.char_at(self.pos + off)
    }

    #[inline]
    fn at_end(&self) -> bool {
        self.pos >= self.pat.len()
    }

    /// POSIX ERE bounded-repetition interval: `{m}` / `{m,}` / `{m,n}`,
    /// `self.pos` pointing at the `{`. Returns `(min, max)` (`max == None`
    /// for the unbounded `{m,}` form) and leaves `self.pos` just past the
    /// closing `}` on success. `{` is a RESERVED character in this
    /// dialect, same as real Nix's own regex backend (cross-checked live
    /// against `nix-instantiate`: `a{`, `a{,3}`, `a{x}`, `a{}`, and a bare
    /// `{3}` with no operand ALL raise "invalid regular expression" there,
    /// there is no literal-`{` fallback for a malformed interval) -- so
    /// every non-well-formed shape here is a hard parse error, never a
    /// silent fall-through to treating `{` as an ordinary character. A
    /// literal `{` still works when escaped (`\{`), same as any other
    /// metacharacter.
    fn try_parse_interval(&mut self) -> Result<(usize, Option<usize>), String> {
        let mut j = self.pos + 1;
        let mut min_digits = String::new();
        while matches!(self.char_at(j), Some(d) if d.is_ascii_digit()) {
            min_digits.push(self.char_at(j).ok_or(String::from("px: regex: broken parser state"))?);
            j += 1;
        }
        if min_digits.is_empty() {
            return Err(String::from("px: regex: invalid interval"));
        }
        let min_i64: i64 = match min_digits.parse::<i64>() {
            Ok(v) => v,
            Err(_) => return Err(String::from("px: regex: invalid interval")),
        };
        let max_i64: Option<i64> = if self.char_at(j) == Some(',') {
            j += 1;
            let mut max_digits = String::new();
            while matches!(self.char_at(j), Some(d) if d.is_ascii_digit()) {
                max_digits.push(self.char_at(j).ok_or(String::from("px: regex: broken parser state"))?);
                j += 1;
            }
            if max_digits.is_empty() {
                None
            } else {
                match max_digits.parse::<i64>() {
                    Ok(v) => Some(v),
                    Err(_) => return Err(String::from("px: regex: invalid interval")),
                }
            }
        } else {
            Some(min_i64)
        };
        if self.char_at(j) != Some('}') {
            return Err(String::from("px: regex: invalid interval"));
        }
        // This engine desugars an interval into that many literal AST-node
        // copies (see the `{`-handling call site) rather than a real
        // engine's counted-repeat primitive. Live cross-check against
        // `nix-instantiate`: real Nix accepts `a{1000000}` (matches
        // instantly, `null` against too-short input) but rejects
        // `a{4294967296}` ("invalid regular expression") -- so 1,000,000
        // is set here as a cap that stays inside real Nix's own observed
        // working range while still bounding this engine's materialized
        // copy count well short of the multi-GB territory a 32-bit-plus
        // count would hit.
        let rx_interval_cap: i64 = 1000000;
        if min_i64 > rx_interval_cap || matches!(max_i64, Some(hi) if hi > rx_interval_cap) {
            return Err(String::from(
                "px: regex: interval count too large",
            ));
        }
        if let Some(hi) = max_i64 {
            if hi < min_i64 {
                return Err(String::from(
                    "px: regex: interval min exceeds max",
                ));
            }
        }
        self.pos = j + 1;
        let max: Option<usize> = match max_i64 {
            Some(hi) => Some(hi as usize),
            None => None,
        };
        Ok((min_i64 as usize, max))
    }

    fn parse_alts(&mut self) -> Result<Vec<Vec<RxNode>>, String> {
        let mut alts = Vec::new();
        alts.push(self.parse_seq()?);
        while matches!(self.cur(), Some('|')) {
            self.pos += 1;
            alts.push(self.parse_seq()?);
        }
        Ok(alts)
    }

    fn parse_seq(&mut self) -> Result<Vec<RxNode>, String> {
        let mut out: Vec<RxNode> = Vec::new();
        while let Some(c) = self.cur() {
            if c == '|' || c == ')' {
                break;
            }
            let node = if c == '(' {
                self.pos += 1;
                self.ngroups += 1;
                let idx = self.ngroups;
                let alts = self.parse_alts()?;
                if self.cur() != Some(')') {
                    return Err(String::from("px: regex: unclosed group"));
                }
                self.pos += 1;
                RxNode::Group { idx, alts }
            } else if c == '[' {
                self.pos += 1;
                let mut neg = false;
                if self.cur() == Some('^') {
                    neg = true;
                    self.pos += 1;
                }
                let mut lo = Vec::new();
                let mut hi = Vec::new();
                let mut first = true;
                while !self.at_end() && (self.cur() != Some(']') || first) {
                    if self.cur() == Some('[') && self.peek(1) == Some(':') {

                        let mut j = self.pos + 2;
                        let mut name = String::new();
                        while self.char_at(j).is_some() && j + 1 < self.pat.len()
                            && !(self.char_at(j) == Some(':') && self.char_at(j + 1) == Some(']'))
                        {
                            name.push(self.char_at(j).ok_or(String::from("px: regex: unclosed POSIX character class"))?);
                            j += 1;
                        }
                        if j + 1 >= self.pat.len() {
                            return Err(String::from(
                                "px: regex: unclosed POSIX character class",
                            ));
                        }
                        rx_add_posix_class(&name, &mut lo, &mut hi)?;
                        self.pos = j + 2;
                        first = false;
                        continue;
                    }
                    let a = match self.cur() {
                        Some(ch) => ch,
                        None => return Err(String::from("px: regex: unclosed class")),
                    };
                    self.pos += 1;
                    if self.peek(0) == Some('-') && self.peek(1) != Some(']')
                    {
                        let b = match self.peek(1) {
                            Some(ch) => ch,
                            None => return Err(String::from("px: regex: unclosed class")),
                        };
                        self.pos += 2;
                        lo.push(a);
                        hi.push(b);
                    } else {
                        lo.push(a);
                        hi.push(a);
                    }
                    first = false;
                }
                if self.at_end() {
                    return Err(String::from("px: regex: unclosed class"));
                }
                self.pos += 1;
                RxNode::Class { neg, lo, hi }
            } else if c == '.' {
                self.pos += 1;
                RxNode::Any
            } else if c == '^' {
                self.pos += 1;
                RxNode::Start
            } else if c == '$' {
                self.pos += 1;
                RxNode::End
            } else if c == '\\' {
                self.pos += 1;
                if self.at_end() {
                    return Err(String::from("px: regex: dangling escape"));
                }
                let e = self.cur().ok_or(String::from("px: regex: dangling escape"))?;
                self.pos += 1;
                RxNode::Ch(e)
            } else if c == '*' || c == '+' || c == '?' || c == '{' {
                // `{` is reserved even with no preceding operand (cross-
                // checked live against real Nix: a bare `{3}` errors
                // "invalid regular expression", it is not treated as a
                // literal `{` -- same reserved-metacharacter stance as
                // `*`/`+`/`?` here).
                return Err(String::from("px: regex: quantifier without operand"));
            } else {
                self.pos += 1;
                RxNode::Ch(c)
            };
            let node = if self.pos < self.pat.len() {
                let q = self.cur().ok_or(String::from("px: regex: broken parser state"))?;
                if q == '*' || q == '+' {
                    // `*`/`+` used to be held for anything but a single-char
                    // node (Ch/Any/Class); RxNode::Star/Plus now also cover
                    // a Group operand via rx_repeat_group_try (rx_seq_try),
                    // so any atom (including a parenthesized group) may be
                    // repeated.
                    self.pos += 1;
                    if q == '*' {
                        RxNode::Star(Box::new(node))
                    } else {
                        RxNode::Plus(Box::new(node))
                    }
                } else if q == '?' {
                    self.pos += 1;
                    RxNode::Opt(Box::new(node))
                } else if q == '{' {
                    // Bounded repetition `{m}`/`{m,}`/`{m,n}`: desugar into
                    // `m` mandatory copies plus either `(n-m)` optional
                    // copies (bounded) or a trailing `*` (unbounded `{m,}`)
                    // -- reuses Star/Opt/plain-node repetition rather than
                    // adding a new RxNode variant or a counted-loop
                    // primitive to the matcher. try_parse_interval already
                    // errors (does not fall back to a literal `{`) on
                    // anything short of a well-formed interval.
                    let (min, max) = self.try_parse_interval()?;
                    let mut k = 0usize;
                    while k < min {
                        out.push(node.clone());
                        k += 1;
                    }
                    match max {
                        Some(hi) => {
                            while k < hi {
                                out.push(RxNode::Opt(Box::new(node.clone())));
                                k += 1;
                            }
                        }
                        None => {
                            out.push(RxNode::Star(Box::new(node.clone())));
                        }
                    }
                    continue;
                } else {
                    node
                }
            } else {
                node
            };
            out.push(node);
        }
        Ok(out)
    }
}


/// rs-meta subset: no indexed assignment — rebuild the (tiny) vec instead.
fn ivset(v: &Vec<i64>, i: usize, x: i64) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::new();
    let mut j = 0usize;
    for val in v.iter() {
        if j == i {
            out.push(x);
        } else {
            out.push(*val);
        }
        j += 1;
    }
    out
}

/// Star/Plus of a MULTI-CHAR operand (typically a parenthesized Group; the
/// single-char case -- Ch/Any/Class -- keeps its own faster scan-ahead path
/// in RxNode::Star/Plus below). Greedy: walk `child` forward as many times
/// as it will match, recording each repetition's end offset, then hand the
/// rest of `seq` the LONGEST chain first and back off one repetition at a
/// time (down to `min_reps`) exactly like the single-char case already
/// does, just generalized to a possibly-variable-width child. A repetition
/// that consumes zero characters stops the walk immediately (matching a
/// sub-pattern that can match empty forever would never terminate
/// otherwise). KNOWN LIMITATION: capture groups nested inside `child` keep
/// whichever repetition's captures the initial greedy walk left behind even
/// when the eventual successful match backs off to fewer repetitions --
/// true POSIX capture semantics for that backtracked case would need
/// re-running the matched span, which is out of scope for this
/// backtracking (not leftmost-longest-DFA) engine.
fn rx_repeat_group_try(
    child: &RxNode,
    seq: &Vec<RxNode>,
    i: usize,
    s: &Vec<char>,
    pos: usize,
    gs: &mut Vec<i64>,
    ge: &mut Vec<i64>,
    min_reps: usize,
) -> i64 {
    let single = vec![child.clone()];
    let mut ends: Vec<usize> = Vec::new();
    let mut cur = pos;
    loop {
        let stepped = rx_seq_try(&single, 0, s, cur, gs, ge);
        if stepped < 0 {
            break;
        }
        let stepped_u = stepped as usize;
        ends.push(stepped_u);
        if stepped_u <= cur {
            break;
        }
        cur = stepped_u;
    }
    let mut k = ends.len();
    loop {
        if k < min_reps {
            return -1;
        }
        let end_pos = if k == 0 { pos } else { ends[k - 1] };
        let r = rx_seq_try(seq, i + 1, s, end_pos, gs, ge);
        if r >= 0 {
            return r;
        }
        if k == 0 {
            return -1;
        }
        k -= 1;
    }
}

/// Match `seq[i..]` at `pos`; on success return the end position (>= 0),
/// else -1. Backtracking; captures written into gs/ge (restored by callers
/// around alternative attempts).
fn rx_seq_try(
    seq: &Vec<RxNode>,
    i: usize,
    s: &Vec<char>,
    pos: usize,
    gs: &mut Vec<i64>,
    ge: &mut Vec<i64>,
) -> i64 {
    if i >= seq.len() {
        return pos as i64;
    }
    match &seq[i] {
        RxNode::Start => {
            if pos == 0 {
                rx_seq_try(seq, i + 1, s, pos, gs, ge)
            } else {
                -1
            }
        }
        RxNode::End => {
            if pos == s.len() {
                rx_seq_try(seq, i + 1, s, pos, gs, ge)
            } else {
                -1
            }
        }
        RxNode::GroupEnd(idx) => {
            let ix = *idx;
            let saved = ge[ix];
            *ge = ivset(ge, ix, pos as i64);
            let r = rx_seq_try(seq, i + 1, s, pos, gs, ge);
            if r < 0 {
                *ge = ivset(ge, ix, saved);
            }
            r
        }
        RxNode::Group { idx, alts } => {
            for alt in alts.iter() {
                let sgs = gs.clone();
                let sge = ge.clone();
                let ix = *idx;
                *gs = ivset(gs, ix, pos as i64);
                let mut flat: Vec<RxNode> = Vec::new();
                for n in alt.iter() {
                    flat.push(n.clone());
                }
                flat.push(RxNode::GroupEnd(ix));
                let mut j = i + 1;
                while j < seq.len() {
                    flat.push(seq[j].clone());
                    j += 1;
                }
                let r = rx_seq_try(&flat, 0, s, pos, gs, ge);
                if r >= 0 {
                    return r;
                }
                *gs = sgs;
                *ge = sge;
            }
            -1
        }
        RxNode::Opt(child) => {
            let sgs = gs.clone();
            let sge = ge.clone();
            let mut flat: Vec<RxNode> = Vec::new();
            flat.push((**child).clone());
            let mut j = i + 1;
            while j < seq.len() {
                flat.push(seq[j].clone());
                j += 1;
            }
            let r = rx_seq_try(&flat, 0, s, pos, gs, ge);
            if r >= 0 {
                return r;
            }
            *gs = sgs;
            *ge = sge;
            rx_seq_try(seq, i + 1, s, pos, gs, ge)
        }
        RxNode::Star(child) if rx_single_ok(child) => {
            let mut max = 0usize;
            while pos + max < s.len() && rx_single_match(child, s[pos + max]) {
                max += 1;
            }
            let mut k = max as i64;
            while k >= 0 {
                let r = rx_seq_try(seq, i + 1, s, pos + (k as usize), gs, ge);
                if r >= 0 {
                    return r;
                }
                k -= 1;
            }
            -1
        }
        RxNode::Plus(child) if rx_single_ok(child) => {
            let mut max = 0usize;
            while pos + max < s.len() && rx_single_match(child, s[pos + max]) {
                max += 1;
            }
            let mut k = max as i64;
            while k >= 1 {
                let r = rx_seq_try(seq, i + 1, s, pos + (k as usize), gs, ge);
                if r >= 0 {
                    return r;
                }
                k -= 1;
            }
            -1
        }
        // Star/Plus of a multi-char operand (a Group, e.g. `(ab)*`/`(a|bc)+`):
        // rx_single_match can't test it one char at a time, so this walks the
        // whole child pattern per repetition instead (rx_repeat_group_try).
        RxNode::Star(child) => rx_repeat_group_try(child, seq, i, s, pos, gs, ge, 0),
        RxNode::Plus(child) => rx_repeat_group_try(child, seq, i, s, pos, gs, ge, 1),
        node => {
            if pos < s.len() && rx_single_match(node, s[pos]) {
                rx_seq_try(seq, i + 1, s, pos + 1, gs, ge)
            } else {
                -1
            }
        }
    }
}

struct RxCompiled {
    alts: Vec<Vec<RxNode>>,
    ngroups: usize,
}

fn rx_compile(pattern: &str) -> Result<RxCompiled, String> {
    let mut p = RxParser {
        pat: pattern.chars().collect(),
        pos: 0,
        ngroups: 0,
    };
    let alts = p.parse_alts()?;
    if p.pos != p.pat.len() {
        return Err(String::from("px: regex: trailing pattern input"));
    }
    Ok(RxCompiled { alts, ngroups: p.ngroups })
}

/// Try the compiled regex at `pos`. Returns (end, captures) or end -1.
fn rx_at(
    rx: &RxCompiled,
    s: &Vec<char>,
    pos: usize,
    require_full: bool,
) -> (i64, Vec<PxVal>) {
    for alt in rx.alts.iter() {
        let mut gs: Vec<i64> = Vec::new();
        let mut ge: Vec<i64> = Vec::new();
        let mut g = 0usize;
        while g <= rx.ngroups {
            gs.push(-1);
            ge.push(-1);
            g += 1;
        }
        let r = rx_seq_try(alt, 0, s, pos, &mut gs, &mut ge);
        if r >= 0 && (!require_full || (r as usize) == s.len()) {
            let mut caps = Vec::new();
            let mut gi = 1usize;
            while gi <= rx.ngroups {
                if gs[gi] >= 0 && ge[gi] >= gs[gi] {
                    let mut cap = String::new();
                    let mut ci = gs[gi] as usize;
                    while ci < (ge[gi] as usize) {
                        cap.push(s[ci]);
                        ci += 1;
                    }
                    caps.push(PxVal::Str(cap));
                } else {
                    caps.push(PxVal::Null);
                }
                gi += 1;
            }
            return (r, caps);
        }
    }
    (-1, Vec::new())
}

/// Nix builtins.match: full-anchored; null or the capture list.
pub fn px_match(pattern: &str, subject: &str) -> Result<PxVal, String> {
    let rx = rx_compile(pattern)?;
    let s: Vec<char> = subject.chars().collect();
    let (end, caps) = rx_at(&rx, &s, 0, true);
    if end < 0 {
        return Ok(PxVal::Null);
    }
    Ok(px_list(caps))
}

/// Nix builtins.split: pieces interleaved with capture lists.
pub fn px_split(pattern: &str, subject: &str) -> Result<PxVal, String> {
    let rx = rx_compile(pattern)?;
    let s: Vec<char> = subject.chars().collect();
    let mut out: Vec<PxVal> = Vec::new();
    let mut last = 0usize;
    let mut pos = 0usize;
    while pos <= s.len() {
        let (end, caps) = rx_at(&rx, &s, pos, false);
        if end >= 0 {
            let mlen = (end as usize) - pos;
            let mut piece = String::new();
            let mut ci = last;
            while ci < pos {
                piece.push(s[ci]);
                ci += 1;
            }
            out.push(PxVal::Str(piece));
            out.push(px_list(caps));
            if mlen > 0 {
                last = pos + mlen;
                pos = last;
            } else {
                last = pos;
                pos += 1;
            }
        } else {
            pos += 1;
        }
    }
    let mut tail = String::new();
    let mut ci = last;
    while ci < s.len() {
        tail.push(s[ci]);
        ci += 1;
    }
    out.push(PxVal::Str(tail));
    Ok(px_list(out))
}

// ---- builtins.fromJSON (oracle-pinned 2026-07-08) ---------------------------
// Duplicate object keys: LAST wins; whitespace tolerant; invalid input errors.
// Subset-safe recursive descent (chars vec + index; no slices/maps).
// `\uXXXX` string escapes are HELD (fail-closed error) — not needed by the
// corpus; land with an oracle gate when something needs them.

fn fj_ws(c: &Vec<char>, p: &mut usize) {
    while *p < c.len() && (c[*p] == ' ' || c[*p] == '\n' || c[*p] == '\t' || c[*p] == '\r') {
        *p += 1;
    }
}

fn fj_lit(c: &Vec<char>, p: &mut usize, word: &str, v: PxVal) -> Result<PxVal, String> {
    for w in word.chars() {
        if *p >= c.len() || c[*p] != w {
            return Err(String::from("px: fromJSON: invalid literal"));
        }
        *p += 1;
    }
    Ok(v)
}

fn fj_string(c: &Vec<char>, p: &mut usize) -> Result<String, String> {
    *p += 1;
    let mut out = String::new();
    while *p < c.len() {
        let ch = c[*p];
        if ch == '"' {
            *p += 1;
            return Ok(out);
        }
        if ch == '\\' {
            *p += 1;
            if *p >= c.len() {
                return Err(String::from("px: fromJSON: dangling escape"));
            }
            let e = c[*p];
            if e == '"' {
                out.push('"');
            } else if e == '\\' {
                out.push('\\');
            } else if e == '/' {
                out.push('/');
            } else if e == 'n' {
                out.push('\n');
            } else if e == 't' {
                out.push('\t');
            } else if e == 'r' {
                out.push('\r');
            } else if e == 'u' {
                // \uXXXX (BMP only) — landed 2026-07-09 for the hangul
                // mirror's codeToChar (fromJSON "\uXXXX" writes syllables).
                // Surrogate pairs (U+10000+) stay held.
                let mut code: u32 = 0;
                let mut k = 0usize;
                while k < 4 {
                    *p += 1;
                    if *p >= c.len() {
                        return Err(String::from("px: fromJSON: bad \\u escape"));
                    }
                    let hc = c[*p];
                    let d: u32 = if hc >= '0' && hc <= '9' {
                        (hc as u32) - ('0' as u32)
                    } else if hc >= 'a' && hc <= 'f' {
                        (hc as u32) - ('a' as u32) + 10
                    } else if hc >= 'A' && hc <= 'F' {
                        (hc as u32) - ('A' as u32) + 10
                    } else {
                        return Err(String::from("px: fromJSON: bad \\u hex"));
                    };
                    code = code * 16 + d;
                    k += 1;
                }
                if code >= 0xD800 && code <= 0xDFFF {
                    return Err(String::from(
                        "px: fromJSON: held: surrogate \\u escapes (non-BMP) not in the seed",
                    ));
                }
                match char::from_u32(code) {
                    Some(ch2) => out.push(ch2),
                    None => return Err(String::from("px: fromJSON: bad \\u code")),
                }
            } else if e == 'b' || e == 'f' {
                // held (fail-closed): \b, \f need control chars the rs-meta
                // subset cannot express yet; land oracle-gated when needed.
                return Err(String::from("px: fromJSON: held: \\b/\\f escapes not in the seed"));
            } else {
                return Err(String::from("px: fromJSON: bad string escape"));
            }
            *p += 1;
        } else {
            out.push(ch);
            *p += 1;
        }
    }
    Err(String::from("px: fromJSON: unterminated string"))
}

fn fj_value(c: &Vec<char>, p: &mut usize) -> Result<PxVal, String> {
    fj_ws(c, p);
    if *p >= c.len() {
        return Err(String::from("px: fromJSON: unexpected end"));
    }
    let ch = c[*p];
    if ch == '"' {
        return Ok(PxVal::Str(fj_string(c, p)?));
    }
    if ch == 't' {
        return fj_lit(c, p, "true", PxVal::Bool(true));
    }
    if ch == 'f' {
        return fj_lit(c, p, "false", PxVal::Bool(false));
    }
    if ch == 'n' {
        return fj_lit(c, p, "null", PxVal::Null);
    }
    if ch == '[' {
        *p += 1;
        let mut items = Vec::new();
        fj_ws(c, p);
        if *p < c.len() && c[*p] == ']' {
            *p += 1;
            return Ok(px_list(items));
        }
        loop {
            items.push(fj_value(c, p)?);
            fj_ws(c, p);
            if *p < c.len() && c[*p] == ',' {
                *p += 1;
            } else if *p < c.len() && c[*p] == ']' {
                *p += 1;
                return Ok(px_list(items));
            } else {
                return Err(String::from("px: fromJSON: expected , or ] in array"));
            }
        }
    }
    if ch == '{' {
        *p += 1;
        let mut pairs: Vec<(String, PxVal)> = Vec::new();
        fj_ws(c, p);
        if *p < c.len() && c[*p] == '}' {
            *p += 1;
            return Ok(px_attrs(pairs));
        }
        loop {
            fj_ws(c, p);
            if *p >= c.len() || c[*p] != '"' {
                return Err(String::from("px: fromJSON: expected object key"));
            }
            let key = fj_string(c, p)?;
            fj_ws(c, p);
            if *p >= c.len() || c[*p] != ':' {
                return Err(String::from("px: fromJSON: expected :"));
            }
            *p += 1;
            let val = fj_value(c, p)?;
            // duplicate key: last wins (oracle-pinned)
            let mut replaced = false;
            for kv in pairs.iter_mut() {
                if kv.0 == key {
                    kv.1 = val.clone();
                    replaced = true;
                }
            }
            if !replaced {
                pairs.push((key, val));
            }
            fj_ws(c, p);
            if *p < c.len() && c[*p] == ',' {
                *p += 1;
            } else if *p < c.len() && c[*p] == '}' {
                *p += 1;
                return Ok(px_attrs(pairs));
            } else {
                return Err(String::from("px: fromJSON: expected , or } in object"));
            }
        }
    }
    if ch == '-' || ch.is_ascii_digit() {
        let mut num = String::new();
        let mut is_float = false;
        while *p < c.len()
            && (c[*p].is_ascii_digit()
                || c[*p] == '-' || c[*p] == '+'
                || c[*p] == '.' || c[*p] == 'e' || c[*p] == 'E')
        {
            if c[*p] == '.' || c[*p] == 'e' || c[*p] == 'E' {
                is_float = true;
            }
            num.push(c[*p]);
            *p += 1;
        }
        if is_float {
            return match num.parse::<f64>() {
                Ok(f) => Ok(PxVal::Float(f)),
                Err(_) => Err(String::from("px: fromJSON: bad number")),
            };
        }
        return match num.parse::<i64>() {
            Ok(n) => Ok(PxVal::Int(n)),
            Err(_) => Err(String::from("px: fromJSON: bad number")),
        };
    }
    Err(String::from("px: fromJSON: unexpected character"))
}




/// subset-safe UTF-8 validity walk (the raw-byte track's revalidation gate).
fn px_utf8_valid(bytes: &Vec<u8>) -> bool {
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i] as u64;
        let need = if b < 128 {
            0
        } else if b >= 194 && b < 224 {
            1
        } else if b >= 224 && b < 240 {
            2
        } else if b >= 240 && b < 245 {
            3
        } else {
            return false;
        };
        let mut k = 0usize;
        while k < need {
            i += 1;
            if i >= bytes.len() {
                return false;
            }
            let c = bytes[i] as u64;
            if c < 128 || c >= 192 {
                return false;
            }
            // reject overlongs/surrogates at the boundary bytes
            if k == 0 && b == 224 && c < 160 {
                return false;
            }
            if k == 0 && b == 237 && c >= 160 {
                return false;
            }
            if k == 0 && b == 240 && c < 144 {
                return false;
            }
            if k == 0 && b == 244 && c >= 144 {
                return false;
            }
            k += 1;
        }
        i += 1;
    }
    true
}

/// UTF-8 bytes of a Str (subset-safe manual encode, chars walk).
fn px_str_bytes(s: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for ch in s.chars() {
        let c = ch as u64;
        if c < 128 {
            out.push(c as u8);
        } else if c < 2048 {
            out.push((192 + (c >> 6)) as u8);
            out.push((128 + (c & 63)) as u8);
        } else if c < 65536 {
            out.push((224 + (c >> 12)) as u8);
            out.push((128 + ((c >> 6) & 63)) as u8);
            out.push((128 + (c & 63)) as u8);
        } else {
            out.push((240 + (c >> 18)) as u8);
            out.push((128 + ((c >> 12) & 63)) as u8);
            out.push((128 + ((c >> 6) & 63)) as u8);
            out.push((128 + (c & 63)) as u8);
        }
    }
    out
}

/// subset-safe UTF-8 decode (bytes ALREADY validated by px_utf8_valid).
fn px_bytes_to_string(bytes: &Vec<u8>) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i] as u64;
        let (code, need) = if b < 128 {
            (b, 0usize)
        } else if b < 224 {
            (b & 31, 1usize)
        } else if b < 240 {
            (b & 15, 2usize)
        } else {
            (b & 7, 3usize)
        };
        let mut cp = code;
        let mut k = 0usize;
        while k < need {
            i += 1;
            cp = (cp << 6) | ((bytes[i] as u64) & 63);
            k += 1;
        }
        match char::from_u32(cp as u32) {
            Some(ch) => out.push(ch),
            None => {}
        }
        i += 1;
    }
    out
}

/// the revalidating constructor: valid UTF-8 returns to Str, else Bytes.
fn px_bytes_val(bytes: Vec<u8>) -> PxVal {
    if px_utf8_valid(&bytes) {
        PxVal::Str(px_bytes_to_string(&bytes))
    } else {
        PxVal::Bytes(bytes)
    }
}

/// byte view of a string-like value (Str or Bytes).
fn px_val_bytes(v: &PxVal) -> Option<Vec<u8>> {
    match v {
        PxVal::Str(s) => Some(px_str_bytes(s)),
        PxVal::Bytes(b) => Some(b.clone()),
        _ => None,
    }
}

/// Byte content + context of a builtin-arg element, for the context-aware
/// concat family (`concatStrings`/`concatStringsSep`/`concatMapStrings`):
/// plain `Str`/`Bytes` contribute an empty context; a ctx-string contributes
/// its own. `None` for anything not string-like.
fn px_item_bytes_and_ctx(v: &PxVal) -> Option<(Vec<u8>, Vec<String>)> {
    if px_is_ctx_string(v) {
        let content = px_string_like_content_or_empty(v);
        return Some((px_str_bytes(&content), px_string_like_context(v)));
    }
    match px_val_bytes(v) {
        Some(b) => Some((b, Vec::new())),
        None => None,
    }
}

// ---- subset-safe hashes (RFC 1321, FIPS 180-4) -------------------------------
// rs-meta-INTERPRETABLE implementations living inside px.rs (the substrate
// rejects crate-path calls and the native helpers use arrays/rotate intrinsics
// outside the subset). 32-bit words are modeled as u64 & MASK32. SHA-512 uses
// (hi32, lo32) pairs: rs-meta intentionally does not rely on >i64 u64 literals.

fn sha_mask() -> u64 {
    4294967295
}

fn sha_rotr(x: u64, n: u64) -> u64 {
    ((x >> n) | (x << (32 - n))) & sha_mask()
}

fn sha_rotl(x: u64, n: u64) -> u64 {
    ((x << n) | (x >> (32 - n))) & sha_mask()
}

fn sha_hex_digit(d: u64) -> char {
    if d < 10 {
        (('0' as u8) + (d as u8)) as char
    } else {
        (('a' as u8) + ((d - 10) as u8)) as char
    }
}

fn sha_hex_byte(out: &mut String, b: u64) {
    out.push(sha_hex_digit((b >> 4) & 15));
    out.push(sha_hex_digit(b & 15));
}

fn sha_hex_word_be(out: &mut String, word: u64) {
    let mut nib = 28i64;
    while nib >= 0 {
        out.push(sha_hex_digit((word >> (nib as u64)) & 15));
        nib -= 4;
    }
}

fn sha_k() -> Vec<u64> {
    let mut k: Vec<u64> = Vec::new();
    for v in [
        1116352408u64, 1899447441, 3049323471, 3921009573, 961987163, 1508970993, 2453635748,
        2870763221, 3624381080, 310598401, 607225278, 1426881987, 1925078388, 2162078206,
        2614888103, 3248222580, 3835390401, 4022224774, 264347078, 604807628, 770255983,
        1249150122, 1555081692, 1996064986, 2554220882, 2821834349, 2952996808, 3210313671,
        3336571891, 3584528711, 113926993, 338241895, 666307205, 773529912, 1294757372,
        1396182291, 1695183700, 1986661051, 2177026350, 2456956037, 2730485921, 2820302411,
        3259730800, 3345764771, 3516065817, 3600352804, 4094571909, 275423344, 430227734,
        506948616, 659060556, 883997877, 958139571, 1322822218, 1537002063, 1747873779,
        1955562222, 2024104815, 2227730452, 2361852424, 2428436474, 2756734187, 3204031479,
        3329325298,
    ]
    .iter()
    {
        k.push(*v);
    }
    k
}

/// UTF-8 bytes of a &str as u64s (subset-safe: chars walk + manual encode).
fn sha_utf8_bytes(s: &str) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new();
    for ch in s.chars() {
        let c = ch as u64;
        if c < 128 {
            out.push(c);
        } else if c < 2048 {
            out.push(192 + (c >> 6));
            out.push(128 + (c & 63));
        } else if c < 65536 {
            out.push(224 + (c >> 12));
            out.push(128 + ((c >> 6) & 63));
            out.push(128 + (c & 63));
        } else {
            out.push(240 + (c >> 18));
            out.push(128 + ((c >> 12) & 63));
            out.push(128 + ((c >> 6) & 63));
            out.push(128 + (c & 63));
        }
    }
    out
}

fn md5_k() -> Vec<u64> {
    let mut out = Vec::new();
    for v in [
        3614090360u64, 3905402710, 606105819, 3250441966, 4118548399, 1200080426, 2821735955,
        4249261313, 1770035416, 2336552879, 4294925233, 2304563134, 1804603682, 4254626195,
        2792965006, 1236535329, 4129170786, 3225465664, 643717713, 3921069994, 3593408605,
        38016083, 3634488961, 3889429448, 568446438, 3275163606, 4107603335, 1163531501,
        2850285829, 4243563512, 1735328473, 2368359562, 4294588738, 2272392833, 1839030562,
        4259657740, 2763975236, 1272893353, 4139469664, 3200236656, 681279174, 3936430074,
        3572445317, 76029189, 3654602809, 3873151461, 530742520, 3299628645, 4096336452,
        1126891415, 2878612391, 4237533241, 1700485571, 2399980690, 4293915773, 2240044497,
        1873313359, 4264355552, 2734768916, 1309151649, 4149444226, 3174756917, 718787259,
        3951481745,
    ]
    .iter()
    {
        out.push(*v);
    }
    out
}

fn md5_shifts() -> Vec<u64> {
    let mut out = Vec::new();
    for v in [
        7u64, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16,
        23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ]
    .iter()
    {
        out.push(*v);
    }
    out
}

pub fn px_md5_hex(input: Vec<u64>) -> String {
    let mut msg = input;
    let m = sha_mask();
    let k = md5_k();
    let shifts = md5_shifts();
    let bitlen = (msg.len() as u64) * 8;
    msg.push(128);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    let mut sh = 0u64;
    let mut bi = 0usize;
    while bi < 8 {
        msg.push((bitlen >> sh) & 255);
        sh += 8;
        bi += 1;
    }
    let mut h0 = 1732584193u64;
    let mut h1 = 4023233417u64;
    let mut h2 = 2562383102u64;
    let mut h3 = 271733878u64;
    let mut off = 0usize;
    while off < msg.len() {
        let mut w: Vec<u64> = Vec::new();
        let mut j = 0usize;
        while j < 16 {
            let p = off + 4 * j;
            w.push((msg[p] | (msg[p + 1] << 8) | (msg[p + 2] << 16) | (msg[p + 3] << 24)) & m);
            j += 1;
        }
        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut i = 0usize;
        while i < 64 {
            let fg: (u64, usize) = if i < 16 {
                ((b & c) | ((m - b) & d), i)
            } else if i < 32 {
                ((d & b) | ((m - d) & c), (5 * i + 1) % 16)
            } else if i < 48 {
                (b ^ c ^ d, (3 * i + 5) % 16)
            } else {
                (c ^ (b | (m - d)), (7 * i) % 16)
            };
            let sum = (a + fg.0 + k[i] + w[fg.1]) & m;
            let old_d = d;
            d = c;
            c = b;
            b = (b + sha_rotl(sum, shifts[i])) & m;
            a = old_d;
            i += 1;
        }
        h0 = (h0 + a) & m;
        h1 = (h1 + b) & m;
        h2 = (h2 + c) & m;
        h3 = (h3 + d) & m;
        off += 64;
    }
    let mut out = String::new();
    for word in [h0, h1, h2, h3].iter() {
        let mut b = 0u64;
        while b < 4 {
            sha_hex_byte(&mut out, (*word >> (8 * b)) & 255);
            b += 1;
        }
    }
    out
}

pub fn px_sha1_hex(input: Vec<u64>) -> String {
    let mut msg = input;
    let m = sha_mask();
    let bitlen = (msg.len() as u64) * 8;
    msg.push(128);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    let mut sh = 56u64;
    let mut bi = 0usize;
    while bi < 8 {
        msg.push((bitlen >> sh) & 255);
        if sh >= 8 { sh -= 8; }
        bi += 1;
    }
    let mut h0 = 1732584193u64;
    let mut h1 = 4023233417u64;
    let mut h2 = 2562383102u64;
    let mut h3 = 271733878u64;
    let mut h4 = 3285377520u64;
    let mut off = 0usize;
    while off < msg.len() {
        let mut w: Vec<u64> = Vec::new();
        let mut i = 0usize;
        while i < 16 {
            let p = off + 4 * i;
            w.push(((msg[p] << 24) | (msg[p + 1] << 16) | (msg[p + 2] << 8) | msg[p + 3]) & m);
            i += 1;
        }
        while i < 80 {
            w.push(sha_rotl(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1));
            i += 1;
        }
        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;
        i = 0;
        while i < 80 {
            let fk: (u64, u64) = if i < 20 {
                ((b & c) | ((m - b) & d), 1518500249u64)
            } else if i < 40 {
                (b ^ c ^ d, 1859775393u64)
            } else if i < 60 {
                ((b & c) | (b & d) | (c & d), 2400959708u64)
            } else {
                (b ^ c ^ d, 3395469782u64)
            };
            let temp = (sha_rotl(a, 5) + fk.0 + e + fk.1 + w[i]) & m;
            e = d;
            d = c;
            c = sha_rotl(b, 30);
            b = a;
            a = temp;
            i += 1;
        }
        h0 = (h0 + a) & m;
        h1 = (h1 + b) & m;
        h2 = (h2 + c) & m;
        h3 = (h3 + d) & m;
        h4 = (h4 + e) & m;
        off += 64;
    }
    let mut out = String::new();
    for word in [h0, h1, h2, h3, h4].iter() {
        sha_hex_word_be(&mut out, *word);
    }
    out
}

pub fn px_sha256_hex(input: Vec<u64>) -> String {
    let mut msg = input;
    let m = sha_mask();
    let k = sha_k();
    let bitlen: u64 = (msg.len() as u64) * 8;
    msg.push(128);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    let mut sh = 56u64;
    let mut bi = 0usize;
    while bi < 8 {
        msg.push((bitlen >> sh) & 255);
        if sh >= 8 {
            sh -= 8;
        }
        bi += 1;
    }
    let mut h: Vec<u64> = Vec::new();
    for v in [
        1779033703u64, 3144134277, 1013904242, 2773480762, 1359893119, 2600822924, 528734635,
        1541459225,
    ]
    .iter()
    {
        h.push(*v);
    }
    let mut off = 0usize;
    while off < msg.len() {
        let mut w: Vec<u64> = Vec::new();
        let mut i = 0usize;
        while i < 16 {
            let b0 = msg[off + 4 * i];
            let b1 = msg[off + 4 * i + 1];
            let b2 = msg[off + 4 * i + 2];
            let b3 = msg[off + 4 * i + 3];
            w.push(((b0 << 24) | (b1 << 16) | (b2 << 8) | b3) & m);
            i += 1;
        }
        while i < 64 {
            let s0 = sha_rotr(w[i - 15], 7) ^ sha_rotr(w[i - 15], 18) ^ (w[i - 15] >> 3);
            let s1 = sha_rotr(w[i - 2], 17) ^ sha_rotr(w[i - 2], 19) ^ (w[i - 2] >> 10);
            w.push((w[i - 16] + s0 + w[i - 7] + s1) & m);
            i += 1;
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        let mut r = 0usize;
        while r < 64 {
            let s1 = sha_rotr(e, 6) ^ sha_rotr(e, 11) ^ sha_rotr(e, 25);
            let ch = (e & f) ^ ((sha_mask() - e) & g);
            let t1 = (hh + s1 + ch + k[r] + w[r]) & m;
            let s0 = sha_rotr(a, 2) ^ sha_rotr(a, 13) ^ sha_rotr(a, 22);
            let mj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = (s0 + mj) & m;
            hh = g;
            g = f;
            f = e;
            e = (d + t1) & m;
            d = c;
            c = b;
            b = a;
            a = (t1 + t2) & m;
            r += 1;
        }
        h[0] = (h[0] + a) & m;
        h[1] = (h[1] + b) & m;
        h[2] = (h[2] + c) & m;
        h[3] = (h[3] + d) & m;
        h[4] = (h[4] + e) & m;
        h[5] = (h[5] + f) & m;
        h[6] = (h[6] + g) & m;
        h[7] = (h[7] + hh) & m;
        off += 64;
    }
    let mut out = String::new();
    let mut hi = 0usize;
    while hi < 8 {
        let mut nib = 28i64;
        while nib >= 0 {
            let d = (h[hi] >> (nib as u64)) & 15;
            let cch = if d < 10 {
                (('0' as u8) + (d as u8)) as char
            } else {
                (('a' as u8) + ((d - 10) as u8)) as char
            };
            out.push(cch);
            nib -= 4;
        }
        hi += 1;
    }
    out
}

fn sha64_add(a: (u64, u64), b: (u64, u64)) -> (u64, u64) {
    let m = sha_mask();
    let low = a.1 + b.1;
    ((a.0 + b.0 + (low >> 32)) & m, low & m)
}

fn sha64_add4(
    a: (u64, u64),
    b: (u64, u64),
    c: (u64, u64),
    d: (u64, u64),
) -> (u64, u64) {
    sha64_add(sha64_add(a, b), sha64_add(c, d))
}

fn sha64_add5(
    a: (u64, u64),
    b: (u64, u64),
    c: (u64, u64),
    d: (u64, u64),
    e: (u64, u64),
) -> (u64, u64) {
    sha64_add(sha64_add4(a, b, c, d), e)
}

fn sha64_rotr(x: (u64, u64), n: u64) -> (u64, u64) {
    let m = sha_mask();
    if n == 0 {
        x
    } else if n < 32 {
        (((x.0 >> n) | (x.1 << (32 - n))) & m, ((x.1 >> n) | (x.0 << (32 - n))) & m)
    } else if n == 32 {
        (x.1, x.0)
    } else {
        let s = n - 32;
        (((x.1 >> s) | (x.0 << (32 - s))) & m, ((x.0 >> s) | (x.1 << (32 - s))) & m)
    }
}

fn sha64_shr(x: (u64, u64), n: u64) -> (u64, u64) {
    let m = sha_mask();
    if n == 0 {
        x
    } else if n < 32 {
        (x.0 >> n, ((x.1 >> n) | (x.0 << (32 - n))) & m)
    } else if n == 32 {
        (0, x.0)
    } else if n < 64 {
        (0, x.0 >> (n - 32))
    } else {
        (0, 0)
    }
}

fn sha512_k() -> Vec<(u64, u64)> {
    let mut raw: Vec<u64> = Vec::new();
    for v in [
        1116352408u64, 3609767458, 1899447441, 602891725, 3049323471, 3964484399,
        3921009573, 2173295548, 961987163, 4081628472, 1508970993, 3053834265,
        2453635748, 2937671579, 2870763221, 3664609560, 3624381080, 2734883394,
        310598401, 1164996542, 607225278, 1323610764, 1426881987, 3590304994,
        1925078388, 4068182383, 2162078206, 991336113, 2614888103, 633803317,
        3248222580, 3479774868, 3835390401, 2666613458, 4022224774, 944711139,
        264347078, 2341262773, 604807628, 2007800933, 770255983, 1495990901,
        1249150122, 1856431235, 1555081692, 3175218132, 1996064986, 2198950837,
        2554220882, 3999719339, 2821834349, 766784016, 2952996808, 2566594879,
        3210313671, 3203337956, 3336571891, 1034457026, 3584528711, 2466948901,
        113926993, 3758326383, 338241895, 168717936, 666307205, 1188179964,
        773529912, 1546045734, 1294757372, 1522805485, 1396182291, 2643833823,
        1695183700, 2343527390, 1986661051, 1014477480, 2177026350, 1206759142,
        2456956037, 344077627, 2730485921, 1290863460, 2820302411, 3158454273,
        3259730800, 3505952657, 3345764771, 106217008, 3516065817, 3606008344,
        3600352804, 1432725776, 4094571909, 1467031594, 275423344, 851169720,
        430227734, 3100823752, 506948616, 1363258195, 659060556, 3750685593,
        883997877, 3785050280, 958139571, 3318307427, 1322822218, 3812723403,
        1537002063, 2003034995, 1747873779, 3602036899, 1955562222, 1575990012,
        2024104815, 1125592928, 2227730452, 2716904306, 2361852424, 442776044,
        2428436474, 593698344, 2756734187, 3733110249, 3204031479, 2999351573,
        3329325298, 3815920427, 3391569614, 3928383900, 3515267271, 566280711,
        3940187606, 3454069534, 4118630271, 4000239992, 116418474, 1914138554,
        174292421, 2731055270, 289380356, 3203993006, 460393269, 320620315,
        685471733, 587496836, 852142971, 1086792851, 1017036298, 365543100,
        1126000580, 2618297676, 1288033470, 3409855158, 1501505948, 4234509866,
        1607167915, 987167468, 1816402316, 1246189591,
    ]
    .iter()
    {
        raw.push(*v);
    }
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < raw.len() {
        out.push((raw[i], raw[i + 1]));
        i += 2;
    }
    out
}

pub fn px_sha512_hex(input: Vec<u64>) -> String {
    let mut msg = input;
    let m = sha_mask();
    let k = sha512_k();
    let bitlen = (msg.len() as u64) * 8;
    msg.push(128);
    while msg.len() % 128 != 112 {
        msg.push(0);
    }
    let mut z = 0usize;
    while z < 8 {
        msg.push(0);
        z += 1;
    }
    let mut sh = 56u64;
    let mut bi = 0usize;
    while bi < 8 {
        msg.push((bitlen >> sh) & 255);
        if sh >= 8 { sh -= 8; }
        bi += 1;
    }
    let mut init: Vec<u64> = Vec::new();
    for v in [
        1779033703u64, 4089235720, 3144134277, 2227873595,
        1013904242, 4271175723, 2773480762, 1595750129,
        1359893119, 2917565137, 2600822924, 725511199,
        528734635, 4215389547, 1541459225, 327033209,
    ]
    .iter()
    {
        init.push(*v);
    }
    let mut h: Vec<(u64, u64)> = Vec::new();
    let mut ii = 0usize;
    while ii < init.len() {
        h.push((init[ii], init[ii + 1]));
        ii += 2;
    }
    let mut off = 0usize;
    while off < msg.len() {
        let mut w: Vec<(u64, u64)> = Vec::new();
        let mut i = 0usize;
        while i < 16 {
            let p = off + 8 * i;
            let hi = ((msg[p] << 24) | (msg[p + 1] << 16) | (msg[p + 2] << 8) | msg[p + 3]) & m;
            let lo = ((msg[p + 4] << 24) | (msg[p + 5] << 16) | (msg[p + 6] << 8) | msg[p + 7]) & m;
            w.push((hi, lo));
            i += 1;
        }
        while i < 80 {
            let x0 = w[i - 15];
            let r01 = sha64_rotr(x0, 1);
            let r08 = sha64_rotr(x0, 8);
            let s07 = sha64_shr(x0, 7);
            let s0 = (r01.0 ^ r08.0 ^ s07.0, r01.1 ^ r08.1 ^ s07.1);
            let x1 = w[i - 2];
            let r19 = sha64_rotr(x1, 19);
            let r61 = sha64_rotr(x1, 61);
            let s06 = sha64_shr(x1, 6);
            let s1 = (r19.0 ^ r61.0 ^ s06.0, r19.1 ^ r61.1 ^ s06.1);
            w.push(sha64_add4(w[i - 16], s0, w[i - 7], s1));
            i += 1;
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        let mut r = 0usize;
        while r < 80 {
            let e14 = sha64_rotr(e, 14);
            let e18 = sha64_rotr(e, 18);
            let e41 = sha64_rotr(e, 41);
            let big1 = (e14.0 ^ e18.0 ^ e41.0, e14.1 ^ e18.1 ^ e41.1);
            let ch = ((e.0 & f.0) ^ ((m - e.0) & g.0), (e.1 & f.1) ^ ((m - e.1) & g.1));
            let t1 = sha64_add5(hh, big1, ch, k[r], w[r]);
            let a28 = sha64_rotr(a, 28);
            let a34 = sha64_rotr(a, 34);
            let a39 = sha64_rotr(a, 39);
            let big0 = (a28.0 ^ a34.0 ^ a39.0, a28.1 ^ a34.1 ^ a39.1);
            let maj = (
                (a.0 & b.0) ^ (a.0 & c.0) ^ (b.0 & c.0),
                (a.1 & b.1) ^ (a.1 & c.1) ^ (b.1 & c.1),
            );
            let t2 = sha64_add(big0, maj);
            hh = g;
            g = f;
            f = e;
            e = sha64_add(d, t1);
            d = c;
            c = b;
            b = a;
            a = sha64_add(t1, t2);
            r += 1;
        }
        h[0] = sha64_add(h[0], a);
        h[1] = sha64_add(h[1], b);
        h[2] = sha64_add(h[2], c);
        h[3] = sha64_add(h[3], d);
        h[4] = sha64_add(h[4], e);
        h[5] = sha64_add(h[5], f);
        h[6] = sha64_add(h[6], g);
        h[7] = sha64_add(h[7], hh);
        off += 128;
    }
    let mut out = String::new();
    let mut i = 0usize;
    while i < 8 {
        sha_hex_word_be(&mut out, h[i].0);
        sha_hex_word_be(&mut out, h[i].1);
        i += 1;
    }
    out
}

/// Nix `builtins.fromJSON`.
pub fn px_from_json(s: &str) -> Result<PxVal, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0usize;
    let v = fj_value(&chars, &mut pos)?;
    fj_ws(&chars, &mut pos);
    if pos != chars.len() {
        return Err(String::from("px: fromJSON: trailing input"));
    }
    Ok(v)
}

/// JSON serialization (Nix `builtins.toJSON` semantics, matched against the
/// pnix-hy runtime: sorted keys, compact separators, UTF-8 passthrough).
fn px_json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c == '"' {
            out.push_str("\\\"");
        } else if c == '\\' {
            out.push_str("\\\\");
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

/// A finite float serializes as its own Rust-Debug text. Rust's `{:?}`
/// formatting for `f64` only ever emits characters JSON's own `number`
/// grammar already accepts: an optional leading `-`, digits, an optional
/// `.digits` fraction, and an optional `e`/`E` exponent whose sign (when
/// present at all) is `-` only -- e.g. `1e300`/`1e-300`, never a `+` before
/// the exponent digits. JSON's grammar does not require a fraction before
/// an exponent (`1e300` round-trips as a valid JSON number on its own), so
/// no separate canonicalization pass is needed for the finite case. A
/// non-finite float (NaN / +inf / -inf) has NO JSON number representation
/// at all -- JSON's grammar has no token for it -- so this is a hard,
/// specific error rather than silently emitting `NaN`/`Infinity` (both
/// invalid JSON) or dropping the value.
fn px_json_float_text(f: f64) -> Result<String, String> {
    if f - f == 0.0 {
        return Ok(format!("{:?}", f));
    }
    let kind = if f != f {
        "NaN"
    } else if f > 0.0 {
        "+inf"
    } else {
        "-inf"
    };
    Err(format!("px: cannot serialize float {} as JSON", kind))
}

pub fn px_to_json(v: &PxVal) -> Result<String, String> {
    // Force to WHNF first; the recursive calls on fields/items force nested
    // thunks in turn, giving Nix's deep-forcing toJSON semantics. A genuinely
    // cyclic structure loops here exactly as `nix-instantiate --eval` does.
    let v = px_force(v)?;
    match &v {
        PxVal::Null => Ok(String::from("null")),
        PxVal::Int(n) => Ok(format!("{}", n)),
        PxVal::Float(f) => px_json_float_text(*f),
        PxVal::Bool(b) => Ok(format!("{}", b)),
        PxVal::Str(s) => Ok(format!("\"{}\"", px_json_escape(s))),
        // A path serializes as its own (normalized) text, same as toString
        // -- unlike `${...}` interpolation, toJSON does not fabricate a
        // store path.
        PxVal::Path(p) => Ok(format!("\"{}\"", px_json_escape(p))),
        // Canonical/CLI output boundary: a contextful string serializes as
        // its content only — context has no representation in canonical
        // JSON (matches real Nix's own `--json` output of e.g. a
        // derivation's outPath: content only). The `toJSON` BUILTIN itself
        // uses the separate context-COLLECTING `px_to_json_ctx` below
        // instead of this function, so `builtins.toJSON` still keeps
        // context on its resulting string value.
        PxVal::Attrs(_) if px_is_ctx_string(&v) => {
            Ok(format!("\"{}\"", px_json_escape(&px_string_like_content_or_empty(&v))))
        }
        PxVal::List(items) => {
            let mut parts = Vec::new();
            for item in items.iter() {
                parts.push(px_to_json(item)?);
            }
            Ok(format!("[{}]", parts.join(",")))
        }
        PxVal::Attrs(fields) => {
            let mut remaining = Vec::new();
            for (name, value) in fields.iter() {
                if !px_is_attr_pos_key(name) {
                    remaining.push((name.clone(), px_to_json(value)?));
                }
            }
            let mut parts = Vec::new();
            while !remaining.is_empty() {
                let mut min = 0usize;
                let mut j = 1usize;
                while j < remaining.len() {
                    if px_str_lt(&remaining[j].0, &remaining[min].0) {
                        min = j;
                    }
                    j += 1;
                }
                let (name, rendered) = remaining.remove(min);
                parts.push(format!("\"{}\":{}", px_json_escape(&name), rendered));
            }
            Ok(format!("{{{}}}", parts.join(",")))
        }
        other => Err(format!("px: toJSON unsupported for {}", px_kind(other))),
    }
}

/// The `toJSON` BUILTIN's own context-aware serialization — oracle: toJSON
/// KEEPS context, the resulting JSON-text string carries the union of every
/// embedded contextful string's context. Otherwise identical to
/// `px_to_json` (same escaping/sorting/shape); kept as a separate function
/// rather than a flag on `px_to_json` because every OTHER caller of
/// `px_to_json` (the CLI `--json` boundary, `production_outcome.rs`, the
/// meta-tower round-trip lanes) wants the plain content-only projection.
fn px_to_json_ctx(v: &PxVal, ctx: &mut Vec<String>) -> Result<String, String> {
    let v = px_force(v)?;
    match &v {
        PxVal::Null => Ok(String::from("null")),
        PxVal::Int(n) => Ok(format!("{}", n)),
        PxVal::Float(f) => px_json_float_text(*f),
        PxVal::Bool(b) => Ok(format!("{}", b)),
        PxVal::Str(s) => Ok(format!("\"{}\"", px_json_escape(s))),
        PxVal::Path(p) => Ok(format!("\"{}\"", px_json_escape(p))),
        PxVal::Attrs(_) if px_is_ctx_string(&v) => {
            ctx.extend(px_string_like_context(&v));
            Ok(format!("\"{}\"", px_json_escape(&px_string_like_content_or_empty(&v))))
        }
        PxVal::List(items) => {
            let mut parts = Vec::new();
            for item in items.iter() {
                parts.push(px_to_json_ctx(item, ctx)?);
            }
            Ok(format!("[{}]", parts.join(",")))
        }
        // __toString wins over outPath (oracle: toJSON { __toString = ..;
        // outPath = "/x"; } uses __toString); called with self, result
        // must be string-like, its context (if any) is kept.
        PxVal::Attrs(fields) if px_attrs_find(fields, "__toString").is_some() => {
            let to_string_fn = px_attrs_find(fields, "__toString").unwrap().clone();
            let sv = px_apply(&to_string_fn, v.clone())?;
            let forced = px_force(&sv)?;
            if matches!(forced, PxVal::Str(_)) || px_is_ctx_string(&forced) {
                px_to_json_ctx(&forced, ctx)
            } else {
                Err(String::from("px: toJSON __toString result is not a string"))
            }
        }
        // An attrset with outPath serializes as that path (oracle: toJSON
        // { outPath = "/x"; other = 1; } is "\"/x\""), so derivations
        // become their store path with context kept.
        PxVal::Attrs(fields) if px_attrs_find(fields, "outPath").is_some() => {
            px_to_json_ctx(px_attrs_find(fields, "outPath").unwrap(), ctx)
        }
        PxVal::Attrs(fields) => {
            let mut remaining = Vec::new();
            for (name, value) in fields.iter() {
                if !px_is_attr_pos_key(name) {
                    remaining.push((name.clone(), px_to_json_ctx(value, ctx)?));
                }
            }
            let mut parts = Vec::new();
            while !remaining.is_empty() {
                let mut min = 0usize;
                let mut j = 1usize;
                while j < remaining.len() {
                    if px_str_lt(&remaining[j].0, &remaining[min].0) {
                        min = j;
                    }
                    j += 1;
                }
                let (name, rendered) = remaining.remove(min);
                parts.push(format!("\"{}\":{}", px_json_escape(&name), rendered));
            }
            Ok(format!("{{{}}}", parts.join(",")))
        }
        other => Err(format!("px: toJSON unsupported for {}", px_kind(other))),
    }
}

pub fn px_kind(v: &PxVal) -> String {
    match v {
        PxVal::Int(_) => String::from("int"),
        PxVal::Float(_) => String::from("float"),
        PxVal::Bool(_) => String::from("bool"),
        PxVal::Null => String::from("null"),
        PxVal::Str(_) => String::from("string"),
        PxVal::Bytes(_) => String::from("string"),
        PxVal::Path(_) => String::from("path"),
        PxVal::List(_) => String::from("list"),
        PxVal::Closure { .. } => String::from("lambda"),
        PxVal::Builtin { .. } => String::from("builtin"),
        PxVal::Attrs(_) if px_is_ctx_string(v) => String::from("string"),
        PxVal::Attrs(_) => String::from("attrset"),
        // diagnostic only; force best-effort so the message names the real kind
        PxVal::Thunk(_) => match px_force(v) {
            Ok(f) => px_kind(&f),
            Err(_) => String::from("thunk"),
        },
    }
}

// ---- canonical print ------------------------------------------------------------

fn px_escape_string(s: &str) -> String {
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

/// Canonical rendering: attrset keys sorted (selection sort; the subset has no
/// Vec::sort), Nix-style `{ k = v; }` / `[ a b ]` layout, quoted strings.
pub fn px_print(v: &PxVal) -> String {
    match v {
        PxVal::Int(n) => format!("{}", n),
        PxVal::Float(f) => format!("{:?}", f),
        PxVal::Bool(b) => format!("{}", b),
        PxVal::Null => String::from("null"),
        PxVal::Str(s) => format!("\"{}\"", px_escape_string(s)),
        PxVal::Bytes(b) => format!("<raw-bytes:{}>", b.len()),
        // Nix prints paths unquoted, unlike strings.
        PxVal::Path(p) => p.clone(),
        PxVal::Closure { .. } => String::from("<lambda>"),
        PxVal::Builtin { .. } => String::from("<builtin>"),
        // printing forces (like Nix's `:p`); recurse on the forced value so
        // nested thunk fields render too. If forcing FAILS, show the real
        // reason — collapsing every error to "«cycle»" hid genuine errors
        // (e.g. a missing builtin) behind a cycle report.
        PxVal::Thunk(_) => match px_force(v) {
            Ok(f) => px_print(&f),
            Err(e) => format!("«error: {}»", e),
        },
        PxVal::List(items) => {
            if items.is_empty() {
                return String::from("[ ]");
            }
            let mut out = String::from("[ ");
            for item in items.iter() {
                out.push_str(&px_print(item));
                out.push(' ');
            }
            out.push(']');
            out
        }
        // Canonical output boundary: a contextful string prints as its
        // content only — context has no representation in printed/canonical
        // form (matches real Nix, where context is invisible in `nix
        // repl`/`--eval` output; only `builtins.getContext` observes it).
        PxVal::Attrs(_) if px_is_ctx_string(v) => {
            format!("\"{}\"", px_escape_string(&px_string_like_content_or_empty(v)))
        }
        PxVal::Attrs(fields) => {
            let mut remaining = Vec::new();
            for (name, value) in fields.iter() {
                if !px_is_attr_pos_key(name) {
                    remaining.push((name.clone(), px_print(value)));
                }
            }
            let mut out = String::from("{ ");
            while !remaining.is_empty() {
                let mut min = 0usize;
                let mut j = 1usize;
                while j < remaining.len() {
                    if px_str_lt(&remaining[j].0, &remaining[min].0) {
                        min = j;
                    }
                    j += 1;
                }
                let (name, rendered) = remaining.remove(min);
                out.push_str(&format!("{} = {}; ", name, rendered));
            }
            out.push('}');
            out
        }
    }
}

fn px_sort_strings(items: Vec<String>) -> Vec<String> {
    let mut remaining = items;
    let mut out = Vec::new();
    while !remaining.is_empty() {
        let mut min = 0usize;
        let mut j = 1usize;
        while j < remaining.len() {
            if px_str_lt(&remaining[j], &remaining[min]) {
                min = j;
            }
            j += 1;
        }
        out.push(remaining.remove(min));
    }
    out
}

/// Byte-wise string ordering (the evaluated subset has no `<` on strings).
fn px_str_lt(a: &str, b: &str) -> bool {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let mut i = 0usize;
    while i < ab.len() && i < bb.len() {
        let x = ab[i] as i64;
        let y = bb[i] as i64;
        if x < y {
            return true;
        }
        if x > y {
            return false;
        }
        i += 1;
    }
    ab.len() < bb.len()
}

/// One-call entrypoint: parse + evaluate + canonical print.
/// Parse + evaluate, returning the VALUE (callers needing data, not text).

/// Lexically normalize a path VALUE's text the same way Nix collapses a
/// path's segments at construction: a `.` segment vanishes, a `..` segment
/// cancels the previous real segment when there is one to cancel. When
/// there is nothing left to cancel, a relative path keeps the `..` (it
/// cannot be resolved without knowing what it is relative to), while an
/// absolute path just drops it (it can never climb above `/`). Every
/// `PxVal::Path` is normalized at the moment it is built (bare literal,
/// `+` concat, `toPath`, `dirOf`) so `==`/`<` can compare the stored
/// strings directly without re-normalizing.
fn px_normalize_path(s: &str) -> String {
    let is_abs = s.starts_with("/");
    let mut segs: Vec<String> = Vec::new();
    for seg in s.split("/") {
        if seg == "" || seg == "." {
            // collapses "//" and "./" markers -- contribute nothing
        } else if seg == ".." {
            let can_pop = match segs.last() {
                Some(last) => last != "..",
                None => false,
            };
            if can_pop {
                segs.pop();
            } else if !is_abs {
                segs.push(String::from(".."));
            }
            // absolute + nothing to cancel: can't go above root, drop it
        } else {
            segs.push(String::from(seg));
        }
    }
    if is_abs {
        let mut out = String::from("/");
        let mut i = 0usize;
        while i < segs.len() {
            if i > 0 {
                out.push('/');
            }
            out.push_str(&segs[i]);
            i += 1;
        }
        return out;
    }
    if segs.is_empty() {
        return String::from(".");
    }
    let mut out = String::new();
    if segs[0] != ".." {
        out.push_str("./");
    }
    let mut i = 0usize;
    while i < segs.len() {
        if i > 0 {
            out.push('/');
        }
        out.push_str(&segs[i]);
        i += 1;
    }
    out
}

/// Normalize `dir/rel` (handling `./` and `../`) into a `./a/b.px` key.
fn px_path_join(dir: &str, rel: &str) -> String {
    // An absolute `rel` (matches load_px_module's Rust std Path::join, which
    // already replaces rather than appends for an absolute component) ignores
    // `dir` entirely -- only the ".<absolute-path>" key format is added.
    if rel.starts_with("/") {
        return format!(".{}", rel);
    }
    let mut parts: Vec<String> = Vec::new();
    for seg in dir.split("/") {
        if seg != "" && seg != "." {
            parts.push(String::from(seg));
        }
    }
    for seg in rel.split("/") {
        if seg == "" || seg == "." {
        } else if seg == ".." {
            parts.pop();
        } else {
            parts.push(String::from(seg));
        }
    }
    let mut out = String::from(".");
    for p in parts.iter() {
        out.push('/');
        out.push_str(p);
    }
    out
}


/// Directory part of a `./a/b.px` key (subset-safe: chars loop, no rfind).
fn px_key_dir(key: &str) -> String {
    let mut last = 0usize;
    let mut i = 0usize;
    for c in key.chars() {
        if c == '/' {
            last = i;
        }
        i += 1;
    }
    if last == 0 {
        return String::from(".");
    }
    let mut d = String::new();
    let mut j = 0usize;
    for c in key.chars() {
        if j >= last {
            break;
        }
        d.push(c);
        j += 1;
    }
    d
}

/// Represent an import-resolution failure as an internal AST leaf so it is
/// raised only if evaluation reaches that import expression. A dedicated leaf
/// is required here: lowering to `throw "..."` would let user scope capture
/// the free `throw` name and turn the failure into an ordinary value.
fn px_deferred_import_error(message: String) -> PxExpr {
    PxExpr::DeferredError(message)
}

/// Load-time import expansion (Nix `import ./x.px`): substitute the target
/// module's PARSED AST at the import site, resolving relative to the
/// IMPORTING file's directory (oracle-pinned 2026-07-08). px_eval stays pure;
/// modules is an injected in-memory map keyed `./rel/path.px` (the clj
/// *import-modules* pattern). Missing targets and cycles become deferred error
/// expressions; a path outside `import` still errors during expansion.
pub fn px_expand_imports(
    e: &PxExpr,
    modules: &Vec<(String, String)>,
    cur_dir: &str,
    stack: &mut Vec<String>,
) -> Result<PxExpr, String> {
    match e {
        PxExpr::DeferredError(message) => Ok(PxExpr::DeferredError(message.clone())),
        PxExpr::Isolated { with_scope, body } => Ok(PxExpr::Isolated {
            with_scope: match with_scope {
                Some(ws) => Some(Box::new(px_expand_imports(ws, modules, cur_dir, stack)?)),
                None => None,
            },
            body: Box::new(px_expand_imports(body, modules, cur_dir, stack)?),
        }),
        PxExpr::Apply { func, arg } => {
            let is_import = matches!(func.as_ref(), PxExpr::Var(n) if n == "import");
            if is_import {
                if let PxExpr::Var(marked) = arg.as_ref() {
                    if marked.starts_with(":path:") {
                        let rel: String = marked.chars().skip(6).collect();
                        let key = px_path_join(cur_dir, &rel);
                        if stack.iter().any(|k| *k == key) {
                            return Ok(px_deferred_import_error(format!(
                                "px: import cycle at {}",
                                key
                            )));
                        }
                        let mut src_opt: Option<&String> = None;
                        for (k, v) in modules.iter() {
                            if *k == key {
                                src_opt = Some(v);
                            }
                        }
                        let src = match src_opt {
                            Some(s) => s,
                            None => {
                                return Ok(px_deferred_import_error(format!(
                                    "px: import target not in the module map: {}",
                                    key
                                )));
                            }
                        };
                        let ast = match px_parse(src) {
                            Ok(value) => value,
                            Err(error) => return Ok(px_deferred_import_error(error)),
                        };
                        let tdir = px_key_dir(&key);
                        stack.push(key);
                        let out = px_expand_imports(&ast, modules, &tdir, stack)?;
                        stack.pop();
                        // Isolated: the module evaluates from a fresh (empty)
                        // environment, not whatever let/with/lambda frames
                        // are active at this splice site.
                        return Ok(PxExpr::Isolated {
                            with_scope: None,
                            body: Box::new(out),
                        });
                    }
                }
            }
            // scopedImport scope path: `Apply{ func: Apply{ func: Var("scopedImport"),
            // arg: scope }, arg: :path:-marked }`. Same module-lookup/cycle
            // logic as plain import, but the substituted module AST is
            // wrapped in `with <scope>; <module>` instead of splicing the
            // module AST in bare -- scope's names become available (lowest
            // priority, so the module's own `let`/lambda bindings still
            // shadow them) without needing to thread a runtime scope value
            // through this AST-only expansion pass. scope does NOT propagate
            // into the module's own nested imports (those recurse through
            // this same match arm with scope=nil, i.e. the plain-import
            // branch above).
            if let PxExpr::Apply {
                func: inner_func,
                arg: scope_ast,
            } = func.as_ref()
            {
                let is_scoped_import =
                    matches!(inner_func.as_ref(), PxExpr::Var(n) if n == "scopedImport");
                if is_scoped_import {
                    if let PxExpr::Var(marked) = arg.as_ref() {
                        if marked.starts_with(":path:") {
                            let rel: String = marked.chars().skip(6).collect();
                            let key = px_path_join(cur_dir, &rel);
                            if stack.iter().any(|k| *k == key) {
                                return Ok(px_deferred_import_error(format!(
                                    "px: import cycle at {}",
                                    key
                                )));
                            }
                            let mut src_opt: Option<&String> = None;
                            for (k, v) in modules.iter() {
                                if *k == key {
                                    src_opt = Some(v);
                                }
                            }
                            let src = match src_opt {
                                Some(s) => s,
                                None => {
                                    return Ok(px_deferred_import_error(format!(
                                        "px: import target not in the module map: {}",
                                        key
                                    )));
                                }
                            };
                            let ast = match px_parse(src) {
                                Ok(value) => value,
                                Err(error) => return Ok(px_deferred_import_error(error)),
                            };
                            let tdir = px_key_dir(&key);
                            stack.push(key);
                            let module_out = px_expand_imports(&ast, modules, &tdir, stack)?;
                            stack.pop();
                            // scope_ast expands (nested imports inside the
                            // scope expression itself) but stays otherwise
                            // un-isolated -- it evaluates in the CALLER's
                            // environment when the Isolated node is reached
                            // (see px_eval_outcome), same as any ordinary
                            // scopedImport argument expression.
                            let scope_out =
                                px_expand_imports(scope_ast, modules, cur_dir, stack)?;
                            return Ok(PxExpr::Isolated {
                                with_scope: Some(Box::new(scope_out)),
                                body: Box::new(module_out),
                            });
                        }
                    }
                }
            }
            Ok(PxExpr::Apply {
                func: Box::new(px_expand_imports(func, modules, cur_dir, stack)?),
                arg: Box::new(px_expand_imports(arg, modules, cur_dir, stack)?),
            })
        }
        PxExpr::Var(n) => {
            // A `:path:`-marked var that was NOT consumed as an `import`/
            // `scopedImport` target above is a bare path literal used as an
            // ordinary expression (`./x.px` outside `import`). It stays a
            // `:path:`-marked Var here; px_eval's variable lookup resolves
            // the marker into a real `PxVal::Path` (see the `:path:` arm
            // there) -- the same resolution `-c`/inline mode already needs
            // since it never runs this expansion pass at all.
            Ok(PxExpr::Var(n.clone()))
        }
        PxExpr::Int(v) => Ok(PxExpr::Int(*v)),
        PxExpr::Float(v) => Ok(PxExpr::Float(*v)),
        PxExpr::Bool(v) => Ok(PxExpr::Bool(*v)),
        PxExpr::Null => Ok(PxExpr::Null),
        PxExpr::Str(parts) => {
            let mut out = Vec::new();
            for p in parts.iter() {
                out.push(match p {
                    PxStrPart::Lit(t) => PxStrPart::Lit(t.clone()),
                    PxStrPart::Sub(x) => {
                        PxStrPart::Sub(px_expand_imports(x, modules, cur_dir, stack)?)
                    }
                });
            }
            Ok(PxExpr::Str(out))
        }
        PxExpr::List(items) => {
            let mut out = Vec::new();
            for it in items.iter() {
                out.push(px_expand_imports(it, modules, cur_dir, stack)?);
            }
            Ok(PxExpr::List(out))
        }
        PxExpr::Select { base, name } => Ok(PxExpr::Select {
            base: Box::new(px_expand_imports(base, modules, cur_dir, stack)?),
            name: name.clone(),
        }),
        PxExpr::Lambda { param, body } => Ok(PxExpr::Lambda {
            param: param.clone(),
            body: std::rc::Rc::new(px_expand_imports(body, modules, cur_dir, stack)?),
        }),
        PxExpr::If { cond, then_e, else_e } => Ok(PxExpr::If {
            cond: Box::new(px_expand_imports(cond, modules, cur_dir, stack)?),
            then_e: Box::new(px_expand_imports(then_e, modules, cur_dir, stack)?),
            else_e: Box::new(px_expand_imports(else_e, modules, cur_dir, stack)?),
        }),
        PxExpr::Binary { op, lhs, rhs } => Ok(PxExpr::Binary {
            op: op.clone(),
            lhs: Box::new(px_expand_imports(lhs, modules, cur_dir, stack)?),
            rhs: Box::new(px_expand_imports(rhs, modules, cur_dir, stack)?),
        }),
        PxExpr::LetIn { bindings, body } => {
            let mut out = Vec::new();
            for (k, v) in bindings.iter() {
                out.push((k.clone(), px_expand_imports(v, modules, cur_dir, stack)?));
            }
            Ok(PxExpr::LetIn {
                bindings: out,
                body: Box::new(px_expand_imports(body, modules, cur_dir, stack)?),
            })
        }
        PxExpr::With { scope, body } => Ok(PxExpr::With {
            scope: Box::new(px_expand_imports(scope, modules, cur_dir, stack)?),
            body: Box::new(px_expand_imports(body, modules, cur_dir, stack)?),
        }),
        PxExpr::Attrs(fields) => {
            let mut out = Vec::new();
            for (k, v) in fields.iter() {
                out.push((k.clone(), px_expand_imports(v, modules, cur_dir, stack)?));
            }
            Ok(PxExpr::Attrs(out))
        }
    }
}

/// Evaluate `src` with `import ./...` resolving against the injected module
/// map (keys `./rel/path.px`); `cur_key` is the evaluating file's own key
/// (for relative resolution), e.g. "./corpus/conformance/x.px".
pub fn px_run_value_with_modules(
    src: &str,
    modules: &Vec<(String, String)>,
    cur_key: &str,
) -> Result<PxVal, String> {
    let ast = px_parse(src)?;
    let dir = px_key_dir(cur_key);
    let mut stack: Vec<String> = vec![String::from(cur_key)];
    let expanded = px_expand_imports(&ast, modules, &dir, &mut stack)?;
    let env = Vec::new();
    px_eval(&expanded, &env)
}

pub fn px_run_value_outcome(src: &str) -> Result<PxVal, PxError> {
    let ast = px_parse(src).map_err(|diagnostic| {
        px_error_parse(PxErrorClass::SyntaxError, diagnostic)
    })?;
    let env = Vec::new();
    px_eval_outcome(&ast, &env)
}

pub fn px_run_value(src: &str) -> Result<PxVal, String> {
    px_run_value_outcome(src).map_err(px_error_into_diagnostic)
}

pub fn px_run(src: &str) -> Result<String, String> {
    let expr = px_parse(src)?;
    let env = Vec::new();
    let val = px_eval(&expr, &env)?;
    Ok(px_print(&val))
}

// ---- emitter ------------------------------------------------------------------
//
// Canonical px emission for the mirror lane (P1). The emitter favors explicit
// parentheses over precedence bookkeeping; the mirror invariant is the emit
// fixed point (`px_emit(parse(px_emit(ast))) == px_emit(ast)`) plus value
// equality after reparse, not byte-identity with the original source
// (comments/whitespace are not part of the AST).

/// Token count facet for mirror records.
pub fn px_tokens(src: &str) -> Result<usize, String> {
    let (toks, _offs) = px_lex(src)?;
    Ok(toks.len())
}

fn px_emit_is_atomic(e: &PxExpr) -> bool {
    match e {
        PxExpr::Int(n) => *n >= 0,
        PxExpr::Float(f) => *f >= 0.0,
        PxExpr::Bool(_) => true,
        PxExpr::Str(_) => true,
        PxExpr::Var(_) => true,
        PxExpr::List(_) => true,
        PxExpr::Attrs(_) => true,
        PxExpr::Select { .. } => true,
        _ => false,
    }
}

fn px_emit_atom(e: &PxExpr) -> String {
    if px_emit_is_atomic(e) {
        px_emit(e)
    } else {
        format!("({})", px_emit(e))
    }
}

fn px_emit_string(parts: &Vec<PxStrPart>) -> String {
    let mut out = String::from("\"");
    for part in parts {
        match part {
            PxStrPart::Lit(s) => {
                for c in s.chars() {
                    if c == '\\' {
                        out.push_str("\\\\");
                    } else if c == '"' {
                        out.push_str("\\\"");
                    } else if c == '\n' {
                        out.push_str("\\n");
                    } else if c == '\t' {
                        out.push_str("\\t");
                    } else if c == '$' {
                        out.push_str("\\$");
                    } else {
                        out.push(c);
                    }
                }
            }
            PxStrPart::Sub(e) => {
                out.push_str(&format!("${{{}}}", px_emit(e)));
            }
        }
    }
    out.push('"');
    out
}

fn px_emit_op(op: &PxOp) -> String {
    match op {
        PxOp::Add => String::from("+"),
        PxOp::Sub => String::from("-"),
        PxOp::Mul => String::from("*"),
        PxOp::Div => String::from("/"),
        PxOp::Eq => String::from("=="),
        PxOp::Ne => String::from("!="),
        PxOp::Lt => String::from("<"),
        PxOp::Le => String::from("<="),
        PxOp::Gt => String::from(">"),
        PxOp::Ge => String::from(">="),
        PxOp::Concat => String::from("++"),
        PxOp::Update => String::from("//"),
        PxOp::HasAttr => String::from("?"),
    }
}


/// Emit-safe name: desugar markers (`:or<k>` / `:<inherit-name>`) are
/// unlexable by design; rename deterministically for emitted SOURCE so
/// ir/mirror/specialize output re-parses. All occurrences (binding + Var)
/// rename identically, so semantics are preserved.
fn px_emit_name(n: &str) -> String {
    if n.starts_with(":") {
        let mut out = String::from("__px_");
        let mut first = true;
        for c in n.chars() {
            if first {
                first = false;
            } else {
                out.push(c);
            }
        }
        out
    } else {
        String::from(n)
    }
}

pub fn px_emit(e: &PxExpr) -> String {
    match e {
        // DeferredError is internal-only. This rendering exists for diagnostics;
        // evaluation never round-trips an expanded import through source text.
        PxExpr::DeferredError(message) => format!(
            "builtins.throw {}",
            px_emit(&PxExpr::Str(vec![PxStrPart::Lit(message.clone())]))
        ),
        // Isolated is internal-only (see its doc comment); this rendering
        // exists for diagnostics only, `with` is its closest real-syntax
        // approximation (scope's names visible, nothing else inherited).
        PxExpr::Isolated { with_scope, body } => match with_scope {
            Some(scope) => format!("with ({}); ({})", px_emit(scope), px_emit(body)),
            None => format!("({})", px_emit(body)),
        },
        PxExpr::Int(n) => {
            if *n >= 0 {
                format!("{}", n)
            } else {
                format!("(0 - {})", 0i64.wrapping_sub(*n))
            }
        }
        PxExpr::Float(f) => {
            let x = *f;
            if x >= 0.0 {
                format!("{:?}", f)
            } else {
                let y = 0.0 - x;
                format!("(0.0 - {:?})", y)
            }
        }
        PxExpr::Bool(b) => format!("{}", b),
        PxExpr::Null => String::from("null"),
        PxExpr::Str(parts) => px_emit_string(parts),
        PxExpr::Var(name) => px_emit_name(name),
        PxExpr::List(items) => {
            if items.is_empty() {
                return String::from("[ ]");
            }
            let mut out = String::from("[ ");
            for item in items {
                out.push_str(&px_emit_atom(item));
                out.push(' ');
            }
            out.push(']');
            out
        }
        PxExpr::Select { base, name } => format!("{}.{}", px_emit_atom(base), name),
        PxExpr::Lambda { param, body } => format!("{}: {}", param, px_emit(body)),
        PxExpr::Apply { func, arg } => {
            let f = match func.as_ref() {
                PxExpr::Apply { .. } => px_emit(func),
                PxExpr::Var(_) => px_emit(func),
                PxExpr::Select { .. } => px_emit(func),
                other => format!("({})", px_emit(other)),
            };
            format!("{} {}", f, px_emit_atom(arg))
        }
        PxExpr::If { cond, then_e, else_e } => format!(
            "if {} then {} else {}",
            px_emit(cond),
            px_emit(then_e),
            px_emit(else_e)
        ),
        PxExpr::Binary { op, lhs, rhs } => format!(
            "{} {} {}",
            px_emit_atom(lhs),
            px_emit_op(op),
            px_emit_atom(rhs)
        ),
        PxExpr::With { scope, body } => {
            format!("with {}; {}", px_emit(scope), px_emit(body))
        }
        PxExpr::LetIn { bindings, body } => {
            let mut out = String::from("let ");
            for (name, value) in bindings {
                out.push_str(&format!("{} = {}; ", px_emit_name(name), px_emit(value)));
            }
            out.push_str(&format!("in {}", px_emit(body)));
            out
        }
        PxExpr::Attrs(fields) => {
            if fields.is_empty() {
                return String::from("{ }");
            }
            let mut out = String::from("{ ");
            for (name, value) in fields {
                if !px_is_attr_pos_key(name) {
                    out.push_str(&format!("{} = {}; ", name, px_emit(value)));
                }
            }
            out.push('}');
            out
        }
    }
}

/// True when a value contains a non-data leaf (closure/builtin) — mirror
/// roundtrip comparison over such values is `held`, not claimed lossless.
pub fn px_value_has_opaque(v: &PxVal) -> bool {
    match v {
        PxVal::Int(_) => false,
        PxVal::Float(_) => false,
        PxVal::Bool(_) => false,
        PxVal::Null => false,
        PxVal::Str(_) => false,
        // A path is plain text with a canonical normalized form, same tier
        // as a string -- not opaque.
        PxVal::Path(_) => false,
        // raw bytes: data, but with no canonical text form — mirror
        // roundtrip claims stay held rather than lossless.
        PxVal::Bytes(_) => true,
        PxVal::Closure { .. } => true,
        PxVal::Builtin { .. } => true,
        // force then inspect; a cycle counts as opaque (can't be shown lossless)
        PxVal::Thunk(_) => match px_force(v) {
            Ok(f) => px_value_has_opaque(&f),
            Err(_) => true,
        },
        PxVal::List(items) => {
            for item in items.iter() {
                if px_value_has_opaque(item) {
                    return true;
                }
            }
            false
        }
        PxVal::Attrs(fields) => {
            for (_name, value) in fields.iter() {
                if px_value_has_opaque(value) {
                    return true;
                }
            }
            false
        }
    }
}

// ---- normalization ---------------------------------------------------------------
//
// Structural canonicalization for the runtime stage ladder (P2) and the IR
// lane (P3): adjacent string literal parts merge, empty literals drop, attrset
// fields sort by name, and let bindings sort by name when unique (recursive
// let resolution is name-based, so unique-name reordering is semantics-
// preserving; duplicate-name frames keep their order honestly). Pure px only —
// normalization never changes an evaluated value.

pub fn px_normalize(e: &PxExpr) -> PxExpr {
    match e {
        PxExpr::DeferredError(message) => PxExpr::DeferredError(message.clone()),
        PxExpr::Isolated { with_scope, body } => PxExpr::Isolated {
            with_scope: with_scope.as_ref().map(|ws| Box::new(px_normalize(ws))),
            body: Box::new(px_normalize(body)),
        },
        PxExpr::Int(n) => PxExpr::Int(*n),
        PxExpr::Float(f) => PxExpr::Float(*f),
        PxExpr::Bool(b) => PxExpr::Bool(*b),
        PxExpr::Null => PxExpr::Null,
        PxExpr::With { scope, body } => PxExpr::With {
            scope: Box::new(px_normalize(scope)),
            body: Box::new(px_normalize(body)),
        },
        PxExpr::Var(name) => PxExpr::Var(name.clone()),
        PxExpr::Str(parts) => {
            let mut out: Vec<PxStrPart> = Vec::new();
            let mut pending = String::new();
            for part in parts {
                match part {
                    PxStrPart::Lit(s) => pending.push_str(s),
                    PxStrPart::Sub(sub) => {
                        if !pending.is_empty() {
                            out.push(PxStrPart::Lit(pending.clone()));
                            pending = String::new();
                        }
                        out.push(PxStrPart::Sub(px_normalize(sub)));
                    }
                }
            }
            if !pending.is_empty() {
                out.push(PxStrPart::Lit(pending));
            }
            PxExpr::Str(out)
        }
        PxExpr::List(items) => {
            let mut out = Vec::new();
            for item in items {
                out.push(px_normalize(item));
            }
            PxExpr::List(out)
        }
        PxExpr::Select { base, name } => PxExpr::Select {
            base: Box::new(px_normalize(base)),
            name: name.clone(),
        },
        PxExpr::Lambda { param, body } => PxExpr::Lambda {
            param: param.clone(),
            body: Rc::new(px_normalize(body)),
        },
        PxExpr::Apply { func, arg } => PxExpr::Apply {
            func: Box::new(px_normalize(func)),
            arg: Box::new(px_normalize(arg)),
        },
        PxExpr::If { cond, then_e, else_e } => PxExpr::If {
            cond: Box::new(px_normalize(cond)),
            then_e: Box::new(px_normalize(then_e)),
            else_e: Box::new(px_normalize(else_e)),
        },
        PxExpr::Binary { op, lhs, rhs } => PxExpr::Binary {
            op: op.clone(),
            lhs: Box::new(px_normalize(lhs)),
            rhs: Box::new(px_normalize(rhs)),
        },
        PxExpr::LetIn { bindings, body } => {
            let mut normalized = Vec::new();
            for (name, value) in bindings {
                normalized.push((name.clone(), px_normalize(value)));
            }
            if px_names_unique(&normalized) {
                normalized = px_sort_bindings(normalized);
            }
            PxExpr::LetIn {
                bindings: normalized,
                body: Box::new(px_normalize(body)),
            }
        }
        PxExpr::Attrs(fields) => {
            let mut normalized = Vec::new();
            for (name, value) in fields {
                normalized.push((name.clone(), px_normalize(value)));
            }
            if px_names_unique(&normalized) {
                normalized = px_sort_bindings(normalized);
            }
            PxExpr::Attrs(normalized)
        }
    }
}

fn px_names_unique(bindings: &Vec<(String, PxExpr)>) -> bool {
    let mut i = 0usize;
    while i < bindings.len() {
        let mut j = i + 1;
        while j < bindings.len() {
            if bindings[i].0 == bindings[j].0 {
                return false;
            }
            j += 1;
        }
        i += 1;
    }
    true
}

fn px_sort_bindings(items: Vec<(String, PxExpr)>) -> Vec<(String, PxExpr)> {
    let mut remaining = items;
    let mut out = Vec::new();
    while !remaining.is_empty() {
        let mut min = 0usize;
        let mut j = 1usize;
        while j < remaining.len() {
            if px_str_lt(&remaining[j].0, &remaining[min].0) {
                min = j;
            }
            j += 1;
        }
        out.push(remaining.remove(min));
    }
    out
}
