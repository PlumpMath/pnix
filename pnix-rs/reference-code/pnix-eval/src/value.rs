//! pnix-eval 값 타입.
//!
//! PnixExpr (pnix-core AST) 를 evaluate 한 결과.
//! pnix-runtime-legacy 의 LangValue 와 호환되지만 독립 구현.

use crate::fx_map::FxHashMap;
use anyhow::Result;
use pnix_core::lang::pnix::syntax::PnixParamPattern;
use pnix_core::lang::pnix::PnixExpr;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};
use std::sync::Arc;

const JSON_HEX_LOWER: &[u8; 16] = b"0123456789abcdef";

/// pnix expression 의 평가 결과 값.
#[derive(Debug, Clone)]
pub enum Value {
  Null,
  Bool(bool),
  Int(i64),
  Float(f64),
  String(String),
  /// String value carrying *string context* — provenance markers that
  /// flow through string concatenation, interpolation, and select
  /// builtins. Mirrors Nix's `StringWithContext` mechanism, but used as
  /// general-purpose provenance metadata in pnix (not nix-store path
  /// dependencies). Only non-empty contexts surface as this variant;
  /// plain text without context stays in `Value::String(_)` so existing
  /// pattern-match sites need no change.
  StringContext {
    text: String,
    context: BTreeSet<String>,
  },
  Path(std::path::PathBuf),
  /// V-3 amendment (2026-06-11): Arc-shared for the same reason as
  /// `AttrSet` below — list VALUE clones (cached-thunk forces of the
  /// math row lists, env-lookup copies, by-value boundaries) become
  /// refcount bumps; mutation sites go through `Arc::make_mut`
  /// (deep only when shared — never worse than the unconditional
  /// deep clone the by-value form paid).
  List(Arc<Vec<Value>>),
  /// V-2 amendment (2026-06-11): the map is Arc-shared so cloning an
  /// AttrSet VALUE (every cached-thunk force, every env-lookup copy,
  /// every by-value boundary) is a refcount bump instead of a
  /// `BTreeMap::clone_subtree` deep copy + matching deep drop —
  /// post-Ex profile had that pair as the #2 leaf cluster. Values are
  /// immutable semantically; construction/update sites that build a
  /// new map use `Arc::make_mut` (free when uniquely owned, one deep
  /// clone when shared — never worse than the unconditional deep
  /// clone the by-value form paid). BTreeMap stays (sorted keys =
  /// receipt determinism).
  AttrSet(Arc<BTreeMap<String, Value>>),
  /// Lambda closure: full param pattern + body AST + captured env.
  /// `env` is `Arc<Env>` so cloning a `Lambda` (which happens on every
  /// env lookup) only bumps a refcount instead of deep-cloning the
  /// captured `bindings` map. `body` is `Arc<PnixExpr>` for the same
  /// reason: every env lookup that returns a Lambda clones it, and
  /// deep-cloning the body AST on each lookup is O(body size)
  /// per call. With `Arc<PnixExpr>` the clone is a refcount bump,
  /// matching the env-clone discipline.
  Lambda {
    /// Arc (V-1 amendment 2026-06-11, was Box since 2026-05-16):
    /// keeps the 8-byte inline payload the Box bought (Value enum
    /// stays ~56 bytes), and additionally makes Lambda VALUE clones
    /// a refcount bump — the Box form deep-cloned the 56-byte
    /// pattern (plus a heap alloc) on every env lookup that returns
    /// a lambda and every builtin callback pass-around. Patterns are
    /// immutable after parse, so sharing is unobservable.
    param: Arc<PnixParamPattern>,
    body: Arc<pnix_core::lang::pnix::PnixExpr>,
    env: Arc<Env>,
  },
  /// Partial application: builtin waiting for more args
  BuiltinPartial {
    name: Arc<str>,
    args: Vec<Value>,
  },
  /// Lazy unevaluated expression. Forced on demand. Lives only inside
  /// `Value::List` element slots, `Value::AttrSet` field slots, and the
  /// `RecursiveBindings` lookup transient. `interpret::eval()` never
  /// returns a `Thunk` to its caller — it always forces the top-level
  /// result before returning.
  ///
  /// `env` is `Arc<Env>` for the same reason as `Lambda.env` — cloning a
  /// thunk during env propagation must not deep-clone its captured
  /// environment, otherwise tail-recursive code (`f n: f (n+1)`) blows
  /// up to O(N²) memory because each iteration's arg-thunk would
  /// duplicate the previous iteration's env chain. `expr` is
  /// `Arc<PnixExpr>` for the same reason; deep-cloning a thunk body
  /// on every env lookup was the dominant cost on the substrate-
  /// wide ratchets.
  ///
  /// `attr_pos` carries the source location of the *binding site* (e.g.
  /// the attribute name in `{ foo = expr; }`). It is read by
  /// `builtins.unsafeGetAttrPos` without forcing the thunk and survives
  /// caching, since the Thunk wrapper itself remains in the AttrSet
  /// even after its underlying value is computed.
  Thunk {
    expr: Arc<pnix_core::lang::pnix::PnixExpr>,
    env: Arc<Env>,
    cache: Arc<std::sync::OnceLock<Value>>,
    attr_pos: Option<AttrSourcePos>,
  },
}

/// Source position of an attribute *binding* — used by
/// `builtins.unsafeGetAttrPos`. Line / column are pre-computed at parse
/// time so eval doesn't need to re-read the source string.
#[derive(Debug, Clone)]
pub struct AttrSourcePos {
  pub byte_pos: usize,
  pub line: u32,
  pub column: u32,
  pub file: Arc<String>,
}

impl fmt::Display for Value {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Value::Null => f.write_str("null"),
      Value::Bool(b) => f.write_str(if *b { "true" } else { "false" }),
      Value::Int(i) => write!(f, "{}", i),
      Value::Float(v) => {
        if v.fract() == 0.0 && v.abs() < 1e15 {
          write!(f, "{:.1}", v)
        } else {
          write!(f, "{}", v)
        }
      }
      Value::String(s) => {
        f.write_str("\"")?;
        f.write_str(s)?;
        f.write_str("\"")
      }
      Value::StringContext { text, .. } => {
        f.write_str("\"")?;
        f.write_str(text)?;
        f.write_str("\"")
      }
      Value::Path(p) => write!(f, "{}", p.display()),
      Value::List(items) => {
        f.write_str("[")?;
        for (i, item) in items.iter().enumerate() {
          if i > 0 {
            f.write_str(",")?;
          }
          fmt::Display::fmt(item, f)?;
        }
        f.write_str("]")
      }
      Value::AttrSet(map) => {
        f.write_str("{")?;
        for (i, (k, v)) in map.iter().enumerate() {
          if i > 0 {
            f.write_str(",")?;
          }
          f.write_str("\"")?;
          f.write_str(k)?;
          f.write_str("\":")?;
          fmt::Display::fmt(v, f)?;
        }
        f.write_str("}")
      }
      Value::Lambda { param, .. } => {
        f.write_str("<lambda:")?;
        f.write_str(lambda_param_label(param))?;
        f.write_str(">")
      }
      Value::BuiltinPartial { name, .. } => {
        f.write_str("<builtin:")?;
        f.write_str(name)?;
        f.write_str(">")
      }
      Value::Thunk { cache, .. } => match cache.get() {
        Some(forced) => fmt::Display::fmt(forced, f),
        None => f.write_str("<thunk>"),
      },
    }
  }
}

fn lambda_param_label(pattern: &PnixParamPattern) -> &str {
  match pattern {
    PnixParamPattern::Ident(name) => name,
    PnixParamPattern::AttrSet { .. } => "{...}",
    PnixParamPattern::AttrSetWithBind { bind_name, .. } => bind_name,
    PnixParamPattern::List(_) => "[...]",
  }
}

impl Value {
  /// JSON 문자열로 변환. Cycle-safe: a self-referential structure such as
  /// `let x = [x]; in x` produces a finite output where the repeated
  /// thunk is rendered as `"<cycle>"`, matching Nix's `«repeated»`
  /// behavior in spirit.
  ///
  /// The cycle-stack holds `Rc` clones of the cache cells (not raw
  /// pointer addresses) so that the allocator can't reuse a freed
  /// address mid-walk and trigger a false-positive cycle. See
  /// `interpret::deep_force` for the same invariant on the eval side.
  pub fn to_json(&self) -> String {
    let mut out = String::with_capacity(json_initial_capacity(self));
    if self.write_json_leaf(&mut out) {
      return out;
    }
    let mut path: Vec<Arc<std::sync::OnceLock<Value>>> = Vec::with_capacity(8);
    self.write_json_inner(&mut path, &mut out);
    out
  }

  fn write_json_leaf(&self, out: &mut String) -> bool {
    match self {
      Value::Null => out.push_str("null"),
      Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
      Value::Int(value) => {
        let _ = write!(out, "{value}");
      }
      Value::Float(value) => match serde_json::Number::from_f64(*value) {
        Some(number) => {
          let _ = write!(out, "{number}");
        }
        None => out.push_str("null"),
      },
      Value::String(value) => push_json_string(out, value),
      Value::StringContext { text, .. } => push_json_string(out, text),
      Value::Path(p) => push_json_string(out, p.to_string_lossy().as_ref()),
      Value::Lambda { param, .. } => {
        push_json_marker(out, "<lambda:", lambda_param_label(param), ">");
      }
      Value::BuiltinPartial { name, .. } => push_json_marker(out, "<builtin:", name, ">"),
      Value::List(_) | Value::AttrSet(_) | Value::Thunk { .. } => return false,
    }
    true
  }

  fn write_json_inner(&self, path: &mut Vec<Arc<std::sync::OnceLock<Value>>>, out: &mut String) {
    if self.write_json_leaf(out) {
      return;
    }
    match self {
      Value::List(items) => {
        out.push('[');
        for (idx, value) in items.iter().enumerate() {
          if idx > 0 {
            out.push(',');
          }
          value.write_json_inner(path, out);
        }
        out.push(']');
      }
      Value::AttrSet(map) => {
        out.push('{');
        for (idx, (key, value)) in map.iter().enumerate() {
          if idx > 0 {
            out.push(',');
          }
          push_json_string(out, key);
          out.push(':');
          value.write_json_inner(path, out);
        }
        out.push('}');
      }
      Value::Thunk { cache, .. } => {
        if path.iter().any(|c| Arc::ptr_eq(c, cache)) {
          push_json_string(out, "<cycle>");
          return;
        }
        path.push(cache.clone());
        match cache.get() {
          Some(forced) => forced.write_json_inner(path, out),
          None => push_json_string(out, "<thunk>"),
        }
        path.pop();
      }
      _ => unreachable!("JSON leaf values are handled before recursive writer dispatch"),
    }
  }

  pub fn is_true(&self) -> bool {
    match self {
      Value::Bool(b) => *b,
      Value::Int(i) => *i != 0,
      Value::Null => false,
      _ => true,
    }
  }

  pub fn as_str(&self) -> Option<&str> {
    match self {
      Value::String(s) => Some(s),
      Value::StringContext { text, .. } => Some(text),
      _ => None,
    }
  }

  pub fn as_f64(&self) -> Option<f64> {
    match self {
      Value::Int(i) => Some(*i as f64),
      Value::Float(f) => Some(*f),
      _ => None,
    }
  }

  /// Get the string-context entries if this value is a context-bearing
  /// string. Returns `None` for plain strings or non-strings.
  pub fn string_context(&self) -> Option<&BTreeSet<String>> {
    match self {
      Value::StringContext { context, .. } => Some(context),
      _ => None,
    }
  }

  /// Construct a string with a context attached. If `context` is empty,
  /// returns a plain `Value::String` so existing pattern-match sites
  /// continue to see the simple variant.
  pub fn string_with_context(text: String, context: BTreeSet<String>) -> Self {
    if context.is_empty() {
      Value::String(text)
    } else {
      Value::StringContext { text, context }
    }
  }

  pub fn string_with_context_ref(text: String, context: &BTreeSet<String>) -> Self {
    if context.is_empty() {
      Value::String(text)
    } else {
      Value::StringContext {
        text,
        context: context.clone(),
      }
    }
  }

  pub fn string_with_optional_context_ref(
    text: String,
    context: Option<&BTreeSet<String>>,
  ) -> Self {
    match context {
      Some(context) if !context.is_empty() => Value::StringContext {
        text,
        context: context.clone(),
      },
      _ => Value::String(text),
    }
  }
}

fn json_initial_capacity(value: &Value) -> usize {
  match value {
    Value::Null => 4,
    Value::Bool(true) => 4,
    Value::Bool(false) => 5,
    Value::Int(_) => 20,
    Value::Float(_) => 24,
    Value::String(value) => value.len().saturating_add(2),
    Value::StringContext { text, .. } => text.len().saturating_add(2),
    Value::Path(path) => path.as_os_str().len().saturating_add(2),
    Value::List(items) => 2usize.saturating_add(items.len()),
    Value::AttrSet(map) => map.iter().fold(2usize, |capacity, (key, _)| {
      capacity.saturating_add(key.len()).saturating_add(4)
    }),
    Value::Lambda { param, .. } => lambda_param_label(param).len().saturating_add(11),
    Value::BuiltinPartial { name, .. } => name.len().saturating_add(12),
    Value::Thunk { .. } => 9,
  }
}

fn push_json_string(out: &mut String, value: &str) {
  out.push('"');
  push_json_string_fragment(out, value);
  out.push('"');
}

fn push_json_marker(out: &mut String, prefix: &str, value: &str, suffix: &str) {
  out.push('"');
  out.push_str(prefix);
  push_json_string_fragment(out, value);
  out.push_str(suffix);
  out.push('"');
}

fn push_json_u_escape(out: &mut String, codepoint: u32) {
  out.push_str("\\u");
  out.push(JSON_HEX_LOWER[((codepoint >> 12) & 0x0f) as usize] as char);
  out.push(JSON_HEX_LOWER[((codepoint >> 8) & 0x0f) as usize] as char);
  out.push(JSON_HEX_LOWER[((codepoint >> 4) & 0x0f) as usize] as char);
  out.push(JSON_HEX_LOWER[(codepoint & 0x0f) as usize] as char);
}

fn push_json_string_fragment(out: &mut String, value: &str) {
  let Some(first_escape_idx) = value.bytes().position(json_byte_requires_escape) else {
    out.push_str(value);
    return;
  };

  out.push_str(&value[..first_escape_idx]);
  let mut chunk_start = first_escape_idx;
  for (relative_idx, ch) in value[first_escape_idx..].char_indices() {
    let idx = first_escape_idx + relative_idx;
    let escaped = match ch {
      '"' => Some("\\\""),
      '\\' => Some("\\\\"),
      '\u{08}' => Some("\\b"),
      '\u{0c}' => Some("\\f"),
      '\n' => Some("\\n"),
      '\r' => Some("\\r"),
      '\t' => Some("\\t"),
      ch if ch <= '\u{1f}' => {
        out.push_str(&value[chunk_start..idx]);
        push_json_u_escape(out, ch as u32);
        chunk_start = idx + ch.len_utf8();
        continue;
      }
      _ => None,
    };
    if let Some(escaped) = escaped {
      out.push_str(&value[chunk_start..idx]);
      out.push_str(escaped);
      chunk_start = idx + ch.len_utf8();
    }
  }
  out.push_str(&value[chunk_start..]);
}

fn json_byte_requires_escape(byte: u8) -> bool {
  byte < 0x20 || byte == b'"' || byte == b'\\'
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn display_writer_preserves_nested_shape() {
    let mut nested = BTreeMap::new();
    nested.insert("b".to_string(), Value::Bool(true));

    let mut root = BTreeMap::new();
    root.insert(
      "a".to_string(),
      Value::List(Arc::new(vec![
        Value::Int(1),
        Value::String("x".to_string()),
      ])),
    );
    root.insert("nested".to_string(), Value::AttrSet(Arc::new(nested)));

    assert_eq!(
      Value::AttrSet(Arc::new(root)).to_string(),
      "{\"a\":[1,\"x\"],\"nested\":{\"b\":true}}"
    );
  }

  #[test]
  fn to_json_direct_writer_preserves_nested_shape_and_escaping() {
    let mut nested = BTreeMap::new();
    nested.insert(
      "newline".to_string(),
      Value::String("line\nnext".to_string()),
    );

    let mut root = BTreeMap::new();
    root.insert(
      "a".to_string(),
      Value::List(Arc::new(vec![
        Value::Int(1),
        Value::String("x\"y".to_string()),
        Value::Null,
      ])),
    );
    root.insert("b".to_string(), Value::AttrSet(Arc::new(nested)));

    assert_eq!(
      Value::AttrSet(Arc::new(root)).to_json(),
      "{\"a\":[1,\"x\\\"y\",null],\"b\":{\"newline\":\"line\\nnext\"}}"
    );
  }

  #[test]
  fn to_json_leaf_fast_path_preserves_scalar_surface() {
    assert_eq!(Value::Null.to_json(), "null");
    assert_eq!(Value::Bool(true).to_json(), "true");
    assert_eq!(Value::Int(42).to_json(), "42");
    assert_eq!(
      Value::String("line\nnext".to_string()).to_json(),
      serde_json::to_string("line\nnext").expect("serde json")
    );
  }

  #[test]
  fn to_json_direct_writer_preserves_non_finite_float_null() {
    assert_eq!(Value::Float(f64::NAN).to_json(), "null");
  }

  #[test]
  fn to_json_path_uses_lossy_path_surface_with_json_escaping() {
    let value = Value::Path(std::path::PathBuf::from("/tmp/path\"segment"));
    assert_eq!(
      value.to_json(),
      serde_json::to_string("/tmp/path\"segment").expect("serde json")
    );
  }

  #[test]
  fn push_json_string_matches_serde_json_escape_surface() {
    for input in [
      "",
      "plain",
      "x\"y",
      "slash\\path",
      "line\nnext",
      "carriage\rreturn",
      "tab\tvalue",
      "back\u{08}space",
      "form\u{0c}feed",
      "nul\u{00}unit",
      "unit\u{1f}separator",
      "한국어",
      "\u{2028}",
    ] {
      let mut out = String::new();
      push_json_string(&mut out, input);
      assert_eq!(out, serde_json::to_string(input).expect("serde json"));
    }
  }

  #[test]
  fn push_json_marker_escapes_dynamic_segment_without_format_allocation() {
    let mut out = String::new();
    push_json_marker(&mut out, "<builtin:", "name\"\nsegment", ">");
    assert_eq!(
      out,
      serde_json::to_string("<builtin:name\"\nsegment>").expect("serde json")
    );
  }
}

#[derive(Debug)]
struct RecursiveBindings {
  // A2 perf slice (2026-06-11): expr and value cache merged into ONE
  // map entry per binding, with a per-binding `OnceLock` replacing the
  // S7c `Mutex<HashMap<String, Value>>`. Effects:
  //   - cache READ (every let-bound Var lookup after first force) is
  //     one hash probe + one atomic load — no pthread mutex, which was
  //     a visible leaf cost (~1.5%) in the substrate-wide profile;
  //   - construction allocates ONE map instead of two (the key Strings
  //     were already owned by the exprs map — merging adds zero String
  //     clones, just an inline OnceLock per entry);
  //   - miss/exprs-membership checks (Env::lookup pass 1 and the
  //     R5/R7/R9 borrow walkers) collapse from two probes (cache lock
  //     + exprs) to one.
  // Concurrency contract is unchanged: cross-thread sharing via
  // Lambda envs needs Send+Sync (OnceLock<Value> provides it exactly
  // as Mutex did); concurrent first-forces both evaluate and one
  // `set` wins — same race outcome as the pre-A2 shape, where
  // evaluation also happened outside the lock and the second insert
  // overwrote an identical deterministic value. Cycle detection stays
  // in the thread-local IN_PROGRESS stack.
  bindings: FxBrownMap<String, RecursiveBinding>,
  outer_env: Env,
}

#[derive(Debug)]
struct RecursiveBinding {
  expr: Arc<PnixExpr>,
  cache: std::sync::OnceLock<Value>,
}

// Per-thread in-progress stack. (2026-05-16 Arc-share): when the same
// `RecursiveBindings` Arc is referenced by Value::Lambda envs that get
// shared across worker threads, each thread's evaluation must have its
// OWN cycle-detection stack. A shared `Mutex<Vec<String>>` would treat
// concurrent evaluations of the same name from different threads as a
// false cycle. We key by `(Arc::as_ptr(&scope), name)` so each
// RecursiveBindings instance's stack stays scoped.
thread_local! {
  static IN_PROGRESS: RefCell<Vec<(usize, String)>> =
    const { RefCell::new(Vec::new()) };
}

impl RecursiveBindings {
  /// Probe on the merged map (see `BindingsRepr::get`).
  #[inline]
  fn probe(&self, name: &str) -> Option<&RecursiveBinding> {
    self.bindings.get(name)
  }

  fn lookup(scope: &Arc<Self>, name: &str) -> Result<Option<Value>> {
    let Some(binding) = scope.probe(name) else {
      return Ok(None);
    };
    if let Some(value) = binding.cache.get() {
      return Ok(Some(value.clone()));
    }
    let expr = binding.expr.clone();

    let scope_key = Arc::as_ptr(scope) as usize;
    IN_PROGRESS.with(|stack| -> Result<()> {
      let stack = stack.borrow();
      if let Some(pos) = stack.iter().position(|(k, n)| *k == scope_key && n == name) {
        let mut cycle = String::new();
        for (_, item) in &stack[pos..] {
          if !cycle.is_empty() {
            cycle.push_str(" -> ");
          }
          cycle.push_str(item);
        }
        if !cycle.is_empty() {
          cycle.push_str(" -> ");
        }
        cycle.push_str(name);
        anyhow::bail!("recursive attrset cycle: {}", cycle);
      }
      Ok(())
    })?;

    IN_PROGRESS.with(|stack| stack.borrow_mut().push((scope_key, name.to_string())));
    let eval_env = Env {
      bindings: shared_empty_bindings(),
      parent: Some(Arc::new(scope.outer_env.clone())),
      recursive: Some(scope.clone()),
      // Recursive-binding evaluation runs against the captured outer
      // env; `with` frames from the outer chain stay reachable
      // because they are pinned on `outer_env.with_chain`. We do
      // not push a fresh frame here — the `rec`-set itself is not a
      // `with`.
      with_chain: scope.outer_env.with_chain.clone(),
    };
    // A7: the binding's expr is already an Arc — hand it through.
    let value = crate::interpret::eval_arc(expr, &eval_env);
    IN_PROGRESS.with(|stack| {
      stack.borrow_mut().pop();
    });

    let value = value?;
    // OnceLock::set returns Err if another thread won the race; the
    // cache then holds that thread's (deterministically identical)
    // value. Same outcome as the pre-A2 second-insert-overwrites.
    let _ = binding.cache.set(value.clone());
    Ok(Some(value))
  }
}

/// One frame of `with` attrs in the current scope. `prev` points at
/// the next-outer `with` frame (if any). The chain is inner-first, so
/// `with X; with Y; e` builds `Frame { Y, prev: Frame { X, prev: None
/// } }` and a lookup walks `Y` before `X`. The chain is consulted
/// only after every lexical (bindings + recursive + parent) walk has
/// failed — that is what gives `let x = …; in with attrs; x` its
/// Nix-correct "let wins" semantics.
///
/// The `with` attrset itself is **lazy**: `with throw "boom"; 1` must
/// return `1` (Nix-correct). The frame stores the unevaluated source
/// expression and the env it was declared in, plus a memoized cache.
/// The first lookup that has to consult this frame (i.e. every
/// lexical scope failed for this name) forces the source; subsequent
/// lookups reuse the cached attrset. If the body never falls through
/// to this frame, the source is never evaluated.
#[derive(Debug)]
pub(crate) struct WithFrame {
  pub(crate) source: Arc<PnixExpr>,
  pub(crate) source_env: Env,
  pub(crate) cache: std::sync::Mutex<Option<Arc<BTreeMap<String, Value>>>>,
  pub(crate) prev: Option<Arc<WithFrame>>,
}

/// 평가 환경 (변수 바인딩).
///
/// `bindings` 는 `Arc<HashMap<...>>` 로 wrap 되어 있다. 이유:
/// 매 thunk 생성마다 `Arc::new(env.clone())` 가 호출되는데, 그 안의
/// `bindings.clone()` 이 O(N) 이다. read-only env (대다수의 thunk env)
/// 는 Rc 공유로 충분하고, `bind()` 가 호출될 때만 `Arc::make_mut` 가
/// clone-on-write 한다. 같은 frame 의 여러 thunk 가 같은 bindings
/// Rc 를 공유 → 하나만 alloc, 나머지는 refcount bump 만.
/// A4 perf slice (2026-06-11): `bindings` payload is a small-map enum
/// instead of a bare map. The overwhelmingly common frame is a
/// single lambda-param binding created once per function application;
/// the hashmap shape paid one table allocation per frame and a hash on
/// every insert/probe. `One` keeps that
/// frame inline in the Arc allocation (lookup = one string compare),
/// `Empty` frames share ONE static Arc (zero alloc for let/with/eval
/// scaffolding frames that never bind), and ≥2-entry frames promote to
/// the map exactly as before. The field is private and every access
/// lives in this file, so the change is invisible outside (Rule 2).
#[derive(Debug, Clone)]
pub(crate) enum BindingsRepr {
  Empty,
  One(String, Value),
  Map(FxBrownMap<String, Value>),
}

pub(crate) type FxBrownMap<K, V> = crate::fx_map::FxHashMap<K, V>;

impl BindingsRepr {
  #[inline]
  fn get(&self, name: &str) -> Option<&Value> {
    match self {
      BindingsRepr::Empty => None,
      BindingsRepr::One(key, value) => (key == name).then_some(value),
      BindingsRepr::Map(map) => map.get(name),
    }
  }

  fn insert(&mut self, name: String, value: Value) {
    match self {
      BindingsRepr::Empty => *self = BindingsRepr::One(name, value),
      BindingsRepr::One(key, slot) => {
        if *key == name {
          // Same overwrite semantics as HashMap::insert.
          *slot = value;
        } else {
          let old = std::mem::replace(self, BindingsRepr::Empty);
          let BindingsRepr::One(old_key, old_value) = old else {
            unreachable!("matched One above");
          };
          let mut map = crate::fx_map::fx_hashmap_with_capacity(4);
          map.insert(old_key, old_value);
          map.insert(name, value);
          *self = BindingsRepr::Map(map);
        }
      }
      BindingsRepr::Map(map) => {
        map.insert(name, value);
      }
    }
  }
}

/// Shared zero-allocation empty bindings. Frames that typically never
/// bind (let/recursive scaffolding, `with` frames, recursive-eval
/// envs) share this one Arc; a later `bind` COWs off it via
/// `Arc::make_mut` (cloning `Empty` is free).
fn shared_empty_bindings() -> Arc<BindingsRepr> {
  static EMPTY: std::sync::OnceLock<Arc<BindingsRepr>> = std::sync::OnceLock::new();
  EMPTY.get_or_init(|| Arc::new(BindingsRepr::Empty)).clone()
}

/// Bindings storage sized for an expected entry count: `Empty` for 0
/// (shared) and 1 (fresh, promoted in place on first bind), a real map
/// only when a destructuring frame will bind several names at once.
fn bindings_with_capacity(capacity: usize) -> Arc<BindingsRepr> {
  match capacity {
    0 => shared_empty_bindings(),
    1 => Arc::new(BindingsRepr::Empty),
    n => Arc::new(BindingsRepr::Map(crate::fx_map::fx_hashmap_with_capacity(n))),
  }
}

#[derive(Debug, Clone)]
pub struct Env {
  bindings: Arc<BindingsRepr>,
  // `Rc` (not `Box`) so that `Env::clone()` — performed on every
  // `Value::Thunk` construction (Apply lazy-arg, AttrSet field thunks,
  // etc.) — is O(1) instead of recursively duplicating the parent
  // chain. Without this, recursive lib.nix-style code (`flatten`,
  // `fold`) blows up to seconds even on small inputs because each
  // level allocates a fresh O(n) parent-chain copy.
  parent: Option<Arc<Env>>,
  recursive: Option<Arc<RecursiveBindings>>,
  /// Inner-first chain of enclosing `with` frames. The chain itself
  /// is shared via `Rc`, and each `Env::with_with_attrs` constructor
  /// pushes a new frame whose `prev` points at the parent's chain
  /// (so outer `with` frames remain reachable through the same
  /// pointer). Lookup is two-pass: first all lexical scopes, then
  /// this chain inner-first.
  with_chain: Option<Arc<WithFrame>>,
}

impl Default for Env {
  fn default() -> Self {
    Self {
      bindings: shared_empty_bindings(),
      parent: None,
      recursive: None,
      with_chain: None,
    }
  }
}

// Iterative drop to break long recursive drop chains. Each tail-call
// `f n: f (n+1)` iteration creates an arg-thunk whose env captures the
// previous iteration's env, which itself has an arg-thunk that captures
// the iteration before that — a list of N thunks linked through their
// `env` field. Letting the default field-by-field Drop walk this chain
// recursively blows the Rust call stack at ~100k depth even with a 32MB
// thread stack. The custom impl below pulls every transitively-owned
// value into a flat work list and drops it iteratively.
impl Drop for Env {
  fn drop(&mut self) {
    // R2 perf slice (2026-06-10): start both work stacks EMPTY
    // (`Vec::new` does not allocate). The overwhelming majority of Env
    // drops are non-owning — their bindings / parent Arcs are still
    // shared with live thunks/lambdas — so `drain_env_into` pushes
    // nothing and the pre-fix `with_capacity` calls were two
    // guaranteed heap allocations (plus frees) on every single Env
    // drop. Real teardown (sole-owner drops) grows the stacks lazily
    // on first push instead.
    let mut value_stack: Vec<Value> = Vec::new();
    let mut env_stack: Vec<Env> = Vec::new();
    drain_env_into(self, &mut value_stack, &mut env_stack);
    loop {
      // Process any pending Envs first so their bindings join the
      // value stack before we tear values down.
      if let Some(mut next) = env_stack.pop() {
        drain_env_into(&mut next, &mut value_stack, &mut env_stack);
        continue;
      }
      if let Some(v) = value_stack.pop() {
        drain_value_into(v, &mut value_stack, &mut env_stack);
        continue;
      }
      break;
    }
  }
}

fn drain_env_into(env: &mut Env, value_stack: &mut Vec<Value>, env_stack: &mut Vec<Env>) {
  // Bindings is `Arc<HashMap>`. If this Env is the sole holder, drain
  // the values iteratively into `value_stack` so their Drops don't
  // recurse. If the Rc is shared, leave it -- the natural Drop will run
  // when the last holder is processed (also via this same function in
  // its own iterative pass).
  //
  // R2 perf slice (2026-06-10): the pre-fix shape
  // (`std::mem::take(&mut env.bindings)` + `Arc::try_unwrap`) allocated
  // a brand-new `Arc<empty map>` (Arc's `Default`) on EVERY Env drop —
  // one guaranteed heap alloc + free per drop, visible as a top
  // allocation site in the eval profile. `Arc::get_mut` reaches the
  // same sole-owner decision without allocating: sole owner → drain the
  // inner map in place (`mem::take` on the map itself is allocation-
  // free); shared → leave the field for its natural drop (a pure
  // refcount decrement), exactly like the pre-fix `try_unwrap` Err arm.
  // A4: same sole-owner decision over the small-map enum. The shared
  // static Empty Arc always has refcount > 1, so `get_mut` skips it.
  if let Some(repr) = Arc::get_mut(&mut env.bindings) {
    match std::mem::replace(repr, BindingsRepr::Empty) {
      BindingsRepr::Empty => {}
      BindingsRepr::One(_, v) => value_stack.push(v),
      BindingsRepr::Map(map) => {
        for (_, v) in map {
          value_stack.push(v);
        }
      }
    }
  }
  // Iteratively walk the parent chain via env_stack, never via
  // recursive self-call. Long `let`/`Apply` chains (100k+ frames)
  // would otherwise overflow the Rust stack at function-return time.
  if let Some(parent) = env.parent.take() {
    if let Ok(inner) = Arc::try_unwrap(parent) {
      env_stack.push(inner);
    }
  }
  // `recursive` holds an Arc<RecursiveBindings>; the expr map inside is
  // cached values, but RecursiveBindings owns them via RefCell. We let
  // the Rc auto-drop — `RecursiveBindings::cache` is bounded by the
  // number of bindings in the original `let`/`rec`, not the runtime
  // recursion depth, so its drop chain can't blow the stack.
  env.recursive.take();
  // `with_chain` is `Arc<WithFrame>`. Each frame holds a finite map of
  // attrset values plus an optional `prev` Rc that points at the
  // next-outer frame. `with` doesn't appear in the tail-recursion
  // hot path (it doesn't get cloned per call frame the way `Apply`
  // arg-thunks do), so the chain length is bounded by lexical
  // nesting and the default Rc auto-drop is safe here.
  env.with_chain.take();
}

fn drain_value_into(value: Value, value_stack: &mut Vec<Value>, env_stack: &mut Vec<Env>) {
  match value {
    // V-2/V-3: a SHARED container must not be cloned just to walk for
    // drop — dropping the Arc is a refcount decrement and the survivor
    // owns the children. Unique containers drain iteratively as before
    // (same shape as the Thunk/Lambda arms below).
    Value::List(items) => {
      if let Ok(items) = Arc::try_unwrap(items) {
        for item in items {
          value_stack.push(item);
        }
      }
    }
    Value::AttrSet(map) => {
      if let Ok(map) = Arc::try_unwrap(map) {
        for (_, v) in map {
          value_stack.push(v);
        }
      }
    }
    Value::Thunk {
      expr: _,
      env,
      cache,
      attr_pos: _,
    } => {
      if let Ok(inner) = Arc::try_unwrap(env) {
        env_stack.push(inner);
      }
      if let Ok(cell) = Arc::try_unwrap(cache) {
        if let Some(cached) = cell.into_inner() {
          value_stack.push(cached);
        }
      }
    }
    Value::Lambda { env, .. } => {
      if let Ok(inner) = Arc::try_unwrap(env) {
        env_stack.push(inner);
      }
    }
    Value::BuiltinPartial { args, .. } => {
      for arg in args {
        value_stack.push(arg);
      }
    }
    _ => {}
  }
}

impl Env {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      bindings: bindings_with_capacity(capacity),
      parent: None,
      recursive: None,
      with_chain: None,
    }
  }

  pub fn with_parent(parent: &Env) -> Self {
    Self::with_parent_capacity(parent, 0)
  }

  pub fn with_parent_capacity(parent: &Env, capacity: usize) -> Self {
    Self {
      bindings: bindings_with_capacity(capacity),
      parent: Some(Arc::new(parent.clone())),
      recursive: None,
      with_chain: parent.with_chain.clone(),
    }
  }

  /// R3 perf slice (2026-06-10): like `with_parent_capacity`, but takes
  /// the parent as an already-shared `Arc<Env>` — a pure refcount bump
  /// instead of cloning the parent Env struct and re-wrapping it in a
  /// fresh Arc allocation. Used by the function-application hot paths,
  /// which destructure `Lambda { env: Arc<Env>, .. }` and previously
  /// paid one Env clone + one Arc allocation per call for a parent that
  /// was already sitting in an Arc.
  pub fn with_parent_arc_capacity(parent: Arc<Env>, capacity: usize) -> Self {
    let with_chain = parent.with_chain.clone();
    Self {
      bindings: bindings_with_capacity(capacity),
      parent: Some(parent),
      recursive: None,
      with_chain,
    }
  }

  pub fn with_recursive_bindings(parent: &Env, exprs: FxHashMap<String, Arc<PnixExpr>>) -> Self {
    let outer_env = parent.clone();
    // A2: keys and expr Arcs MOVE into the merged map — zero clones,
    // one inline OnceLock per binding replaces the old Mutex'd second
    // map.
    let mut recursive_bindings = crate::fx_map::fx_hashmap_with_capacity(exprs.len());
    for (name, expr) in exprs {
      recursive_bindings.insert(
        name,
        RecursiveBinding {
          expr,
          cache: std::sync::OnceLock::new(),
        },
      );
    }
    Self {
      bindings: shared_empty_bindings(),
      parent: Some(Arc::new(parent.clone())),
      recursive: Some(Arc::new(RecursiveBindings {
        bindings: recursive_bindings,
        outer_env,
      })),
      with_chain: parent.with_chain.clone(),
    }
  }

  /// Push a lazy `with` frame. `source` is evaluated only when the
  /// body falls through to this frame for an unresolved name; the
  /// result is memoized in the frame's `cache`. `source_env` is the
  /// env in which `source` should evaluate (the caller's env, not
  /// the with body's env — `with X; e` evaluates `X` outside `e`).
  pub fn with_with_attrs(parent: &Env, source: Arc<PnixExpr>, source_env: Env) -> Self {
    let frame = WithFrame {
      source,
      source_env,
      cache: std::sync::Mutex::new(None),
      prev: parent.with_chain.clone(),
    };
    Self {
      bindings: shared_empty_bindings(),
      parent: Some(Arc::new(parent.clone())),
      recursive: None,
      with_chain: Some(Arc::new(frame)),
    }
  }

  pub fn bind(&mut self, name: String, value: Value) {
    // Clone-on-write: if this Env's bindings Rc is shared with other
    // Envs (typical for thunks captured from this frame), `make_mut`
    // clones into a fresh allocation so we don't mutate shared state.
    // If we're the sole holder, mutation happens in place.
    Arc::make_mut(&mut self.bindings).insert(name, value);
  }

  /// Two-pass iterative lookup. Pass 1 walks every lexical scope
  /// (bindings + recursive + parent) — Nix says `let`/lambda/`rec`
  /// bindings always win over `with`. Pass 2 walks the `with_chain`
  /// inner-first, which is itself shared via the parent's chain so a
  /// single walk covers every enclosing `with`. The per-Env hop is a
  /// loop, not a self-call, so 100k-deep `let` / `Apply` chains don't
  /// blow the Rust call stack at lookup time.
  pub fn lookup(&self, name: &str) -> Result<Option<Value>> {
    let mut current: &Env = self;
    loop {
      if let Some(value) = current.bindings.get(name).cloned() {
        return Ok(Some(value));
      }
      if let Some(scope) = &current.recursive {
        if let Some(value) = RecursiveBindings::lookup(scope, name)? {
          return Ok(Some(value));
        }
      }
      match &current.parent {
        Some(parent) => current = parent.as_ref(),
        None => break,
      }
    }
    // Lexical lookup failed — fall through to `with` attrs, inner
    // first. Each frame is lazy: force its source expression only
    // when we have to ask it for a name. If forcing the source
    // throws (e.g. `with throw "boom"; …`), the error surfaces
    // here — exactly the same point where real Nix raises it.
    let mut frame = self.with_chain.as_ref();
    while let Some(f) = frame {
      let attrs = WithFrame::force(f)?;
      if let Some(value) = attrs.get(name).cloned() {
        return Ok(Some(value));
      }
      frame = f.prev.as_ref();
    }
    Ok(None)
  }

  /// R5 perf slice (2026-06-10): borrow-walk the lexical scope for
  /// `name` and, when the bound value is an already-forced attrset
  /// (directly, or through a thunk whose cache is populated), clone
  /// ONLY the requested `attr` field instead of the whole container.
  /// `m.field` on a module-sized binding previously cloned the entire
  /// attrset (O(keys) String + Value clones) just to keep one field.
  ///
  /// Walk order mirrors `lookup` pass 1 exactly: bindings → recursive
  /// cache → parent. Any case that would need evaluation or pass-2
  /// `with` fallthrough returns `None` so the caller takes the generic
  /// eval path unchanged (same side effects, same error provenance):
  ///   - name bound to a thunk whose cache is empty (or holds a thunk)
  ///   - name present in a recursive scope's exprs but not yet cached
  ///   - lexical walk exhausted (with-chain / global alias / undefined)
  pub(crate) fn select_attr_borrowed(&self, name: &str, attr: &str) -> Option<SelectBorrowed> {
    fn project(value: &Value, attr: &str) -> Option<SelectBorrowed> {
      match value {
        Value::AttrSet(map) => Some(SelectBorrowed::Attr(map.get(attr).cloned())),
        Value::Thunk { cache, .. } => match cache.get() {
          Some(cached) if !matches!(cached, Value::Thunk { .. }) => project(cached, attr),
          _ => None,
        },
        _ => Some(SelectBorrowed::NonAttrset),
      }
    }
    let mut current: &Env = self;
    loop {
      if let Some(value) = current.bindings.get(name) {
        return project(value, attr);
      }
      if let Some(scope) = &current.recursive {
        // A2: one probe on the merged map; the per-binding OnceLock
        // read is lock-free. Cached → clone only the selected field
        // (the generic RecursiveBindings::lookup clones the whole
        // container before selecting). Uncached → needs evaluation
        // (cycle guard, caching, error provenance) — generic path.
        if let Some(binding) = scope.probe(name) {
          return match binding.cache.get() {
            Some(value) => project(value, attr),
            None => None,
          };
        }
      }
      match &current.parent {
        Some(parent) => current = parent.as_ref(),
        None => return None,
      }
    }
  }

  /// R9 perf slice (2026-06-10): `f x` callee fast path. When the
  /// `Apply` function position is a plain `Var` whose lexical binding
  /// is (or caches) an Ident-pattern lambda, extract just the call
  /// parts — param name (cloned once, used directly as the binding
  /// key the generic path would clone it into anyway), body Arc, and
  /// captured-env Arc — instead of evaluating `Var(f)`, which clones
  /// the whole `Value::Lambda` (a fresh `Box<PnixParamPattern>` heap
  /// allocation + pattern clone) on every single application.
  ///
  /// Same conservative walk as [`Env::var_thunk_passthrough`]:
  /// bindings / recursive-CACHE hits only, populated thunk caches
  /// only, lexical-walk exhaustion (with-chain / global alias /
  /// undefined) falls back to the generic path. Destructure-pattern
  /// lambdas and non-lambda values also fall back. The projection
  /// inside the recursive-cache lock performs only field clones
  /// (String + two Arc bumps) — it never runs user code, so it
  /// cannot re-enter the same `RecursiveBindings` Mutex.
  pub(crate) fn var_lambda_ident_call_parts(
    &self,
    name: &str,
  ) -> Option<(String, Arc<PnixExpr>, Arc<Env>)> {
    fn project(value: &Value) -> Option<(String, Arc<PnixExpr>, Arc<Env>)> {
      match value {
        Value::Lambda { param, body, env } => match param.as_ref() {
          PnixParamPattern::Ident(param_name) => {
            Some((param_name.clone(), body.clone(), env.clone()))
          }
          _ => None,
        },
        Value::Thunk { cache, .. } => match cache.get() {
          Some(cached) if !matches!(cached, Value::Thunk { .. }) => project(cached),
          _ => None,
        },
        _ => None,
      }
    }
    let mut current: &Env = self;
    loop {
      if let Some(value) = current.bindings.get(name) {
        return project(value);
      }
      if let Some(scope) = &current.recursive {
        // A2: lock-free merged-map probe (see select_attr_borrowed).
        if let Some(binding) = scope.probe(name) {
          return match binding.cache.get() {
            Some(value) => project(value),
            None => None,
          };
        }
      }
      match &current.parent {
        Some(parent) => current = parent.as_ref(),
        None => return None,
      }
    }
  }

  /// R7 perf slice (2026-06-10): `f x` argument pass-through probe.
  /// When an `Apply` argument is a plain `Var` whose lexical binding is
  /// an existing thunk, the caller can hand that thunk straight through
  /// as the argument instead of wrapping `Var(x)` in a fresh thunk.
  /// The wrapper costs one Arc<OnceLock> allocation per application
  /// AND — the expensive half — builds a SECOND cache layer: forcing
  /// the wrapper forces the inner thunk, then deep-clones the result
  /// into the wrapper's own cache, so container values end up cached
  /// (and re-cloned per access) twice.
  ///
  /// Strictly non-evaluating and strictly conservative:
  ///   - only `bindings` / recursive-CACHE hits qualify (a recursive
  ///     name that exists in `exprs` but is uncached would need
  ///     evaluation — fall back so laziness of `let` bindings holds)
  ///   - only `Value::Thunk` passes through (Arc-cheap clone; passing
  ///     a forced container would pay an eager deep clone the lazy
  ///     wrapper might never pay)
  ///   - only thunks with `attr_pos: None` (an attrset-field thunk
  ///     carries a source position observable via
  ///     `unsafeGetAttrPos` if user code re-stores the value; the
  ///     wrapper would have reported None)
  ///   - lexical-walk exhaustion (with-chain / global alias /
  ///     undefined) falls back, preserving lazy undefined-variable
  ///     semantics (`(a: 1) undefined` must still return 1).
  pub(crate) fn var_thunk_passthrough(&self, name: &str) -> Option<Value> {
    fn qualifies(value: &Value) -> bool {
      matches!(value, Value::Thunk { attr_pos: None, .. })
    }
    let mut current: &Env = self;
    loop {
      if let Some(value) = current.bindings.get(name) {
        return if qualifies(value) {
          Some(value.clone())
        } else {
          None
        };
      }
      if let Some(scope) = &current.recursive {
        // A2: lock-free merged-map probe (see select_attr_borrowed).
        if let Some(binding) = scope.probe(name) {
          return match binding.cache.get() {
            Some(value) if qualifies(value) => Some(value.clone()),
            _ => None,
          };
        }
      }
      match &current.parent {
        Some(parent) => current = parent.as_ref(),
        None => return None,
      }
    }
  }
}

/// Result of [`Env::select_attr_borrowed`]'s field projection. The
/// caller maps these onto the exact same outcomes the generic
/// `Select` / `SelectOrDefault` arms produce.
pub(crate) enum SelectBorrowed {
  /// Bound value is a forced attrset; `Some(field)` when the attr
  /// exists, `None` when it is missing.
  Attr(Option<Value>),
  /// Bound value is forced and is not an attrset.
  NonAttrset,
}

impl WithFrame {
  /// Force the lazy `with` source if it hasn't been evaluated yet,
  /// memoize, and return a shared reference-counted attrset map.
  ///
  /// 2026-05-05: previously, a non-attrset source resolved to an
  /// empty map and the lookup fell through to the next outer
  /// frame (or to the global "undefined variable" path). That
  /// silently masked a real type error: an author writing
  /// `with maybeAttrset; foo` where `maybeAttrset` came back
  /// from a misconfigured loader as `null` / `42` / `[ ... ]`
  /// would see "undefined variable: foo" and waste time
  /// looking for a missing `foo` definition, when the actual
  /// bug is the `with` source's type. Real Nix errors at the
  /// force point with "value is a <type> while a set was
  /// expected" — fail-loud at the type boundary, not at the
  /// incidental name-lookup tail. Match that shape: if the
  /// with-source forces to a non-attrset, return a typed error
  /// here. The "lazy" part of the contract is unchanged: the
  /// source is still only forced when a body name lookup
  /// actually has to consult this frame, so `with 42; 1` still
  /// returns `1` without ever evaluating the `42`.
  fn force(this: &Arc<WithFrame>) -> Result<Arc<BTreeMap<String, Value>>> {
    {
      let guard = this.cache.lock().unwrap();
      if let Some(cached) = guard.as_ref() {
        return Ok(cached.clone());
      }
    }
    // A7: the frame source is already an Arc — refcount bump only.
    let value = crate::interpret::eval_arc(this.source.clone(), &this.source_env)?;
    let map = match value {
      // V-2: the attrset already carries its Arc — share it directly
      // (pre-V-2 this wrapped a fresh Arc around the owned map).
      Value::AttrSet(m) => m,
      other => {
        return Err(anyhow::anyhow!(
          "with: argument must be attrset, got {}",
          crate::interpret::type_name(&other)
        ));
      }
    };
    let mut guard = this.cache.lock().unwrap();
    if guard.is_none() {
      *guard = Some(map.clone());
    }
    Ok(map)
  }
}
