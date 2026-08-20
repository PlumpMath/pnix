//! Regression cover for `builtins.toString` cross-call cycle
//! protection — extension of the slice #40 InterpDepthGuard
//! family to the `toString` builtin path.
//!
//! 2026-05-05 audit findings (slice #58):
//!
//!   `let r = { __toString = self: builtins.toString self; };
//!    in builtins.toString r`
//!   overflowed the Rust call stack with SIGABRT. The cycle
//!   pattern: `toString r` calls the `__toString` lambda which
//!   calls `toString self` which loops back. The within-call
//!   `depth` parameter in `coerce_to_string_for_to_string_with_
//!   context` reset to 0 each time `apply_builtin "toString"`
//!   was re-entered (every `toString` call constructs a fresh
//!   coerce frame), so the per-call check never fired.
//!
//!   Same shape as slice #40's interpolation cycle guard. Slice
//!   #40 closed it for `${...}` (`coerce_to_string_for_inter-
//!   polation`); slice #58 extends the same `InterpDepthGuard`
//!   to the `toString` builtin path. Cross-call cycles can
//!   alternate between the two paths (a `__toString` returning
//!   a string with `${...}` interpolation), so sharing the
//!   thread-local guard handles every shape.
//!
//!   Pre-fix shape was a real DoS: any `.px` author who
//!   accidentally wrote a self-referential `__toString`
//!   crashed the evaluator process with SIGABRT instead of
//!   getting a typed error. Production-grade evaluators MUST
//!   surface infinite recursion as a typed error rather than
//!   blowing up the process.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"toString"` arm — adds `let _depth_guard =
//!   InterpDepthGuard::enter()?;` at entry, before calling
//!   `coerce_to_string_for_to_string_with_context`. The guard
//!   is exception-safe RAII — its `Drop` impl decrements the
//!   thread-local depth counter regardless of how the function
//!   exits.

use pnix_eval::eval_expr;

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

// ── self-referential __toString cycle ─────────────────────────

#[test]
fn to_string_self_referential_method_errors_without_overflow() {
  // Pre-fix: SIGABRT (Rust call stack overflow). Now: typed
  // error from InterpDepthGuard.
  let e = err(
    r#"
    let r = { __toString = self: builtins.toString self; };
    in builtins.toString r
  "#,
  );
  assert!(
    e.contains("interpolation") || e.contains("toString") || e.contains("cycle"),
    "got: {e}"
  );
}

#[test]
fn to_string_alternating_method_chain_errors_loud() {
  // Two-element cycle: a's __toString returns `toString b`,
  // b's __toString returns `toString a`. Same DoS shape; guard
  // catches it.
  let e = err(
    r#"
    let
      a = { __toString = self: builtins.toString b; };
      b = { __toString = self: builtins.toString a; };
    in builtins.toString a
  "#,
  );
  assert!(
    e.contains("interpolation") || e.contains("toString") || e.contains("cycle"),
    "got: {e}"
  );
}

#[test]
fn to_string_outpath_cycle_errors_loud() {
  // outPath returning self → toString-coerce loop.
  let e = err(
    r#"
    let r = { outPath = builtins.toString r; };
    in builtins.toString r
  "#,
  );
  assert!(
    e.contains("interpolation")
      || e.contains("toString")
      || e.contains("cycle")
      || e.contains("recursion"),
    "got: {e}"
  );
}

// ── cross-path cycle (toString ↔ interpolation) ───────────────

#[test]
fn to_string_then_interp_cycle_errors_loud() {
  // toString r → __toString returns string with ${self}
  // interpolation → recurses into interp coerce → which
  // re-enters via __toString → ... Sharing the guard between
  // the two paths catches this.
  let e = err(
    r#"
    let r = { __toString = self: "wrapped:${self}"; };
    in builtins.toString r
  "#,
  );
  assert!(
    e.contains("interpolation") || e.contains("toString") || e.contains("cycle"),
    "got: {e}"
  );
}

#[test]
fn interp_then_to_string_cycle_errors_loud() {
  // ${r} → __toString returns toString self → toString re-enters
  // into __toString → ... etc.
  let e = err(
    r#"
    let r = { __toString = self: "${builtins.toString self}"; };
    in "${r}"
  "#,
  );
  assert!(
    e.contains("interpolation") || e.contains("toString") || e.contains("cycle"),
    "got: {e}"
  );
}

// ── happy path: legitimate __toString chains still work ───────

#[test]
fn to_string_two_level_method_chain_works() {
  // Legitimate two-level chain: a's __toString returns "from-a:"
  // + (a non-cyclic toString of b). Each call resolves to a
  // string and terminates.
  let v = json(
    r#"
    let
      a = { __toString = self: "wrapped(" + (builtins.toString b) + ")"; };
      b = { __toString = self: "leaf"; };
    in builtins.toString a
  "#,
  );
  assert_eq!(v, r#""wrapped(leaf)""#);
}

#[test]
fn to_string_method_returning_string_works() {
  // Pure non-cyclic case — sanity that the guard doesn't
  // false-positive.
  let v = json(
    r#"
    let r = { __toString = self: "result"; };
    in builtins.toString r
  "#,
  );
  assert_eq!(v, r#""result""#);
}

#[test]
fn to_string_outpath_string_works() {
  // outPath as bare string — not cyclic.
  let v = json(r#"builtins.toString { outPath = "/some/path"; }"#);
  assert_eq!(v, r#""/some/path""#);
}

#[test]
fn to_string_self_field_access_works() {
  // __toString uses `self` to read a field — terminates.
  let v = json(
    r#"
    builtins.toString {
      __toString = self: builtins.toString self.field;
      field = 99;
    }
  "#,
  );
  assert_eq!(v, r#""99""#);
}

// ── interplay with slice #40 (interpolation cycle) ────────────

#[test]
fn interpolation_cycle_still_caught() {
  // Sanity: slice #40 still fires for pure-interpolation cycles.
  let e = err(
    r#"
    let r = { __toString = self: "${self}"; };
    in "${r}"
  "#,
  );
  assert!(
    e.contains("interpolation") || e.contains("cycle") || e.contains("recursion"),
    "got: {e}"
  );
}

#[test]
fn interpolation_pure_chain_works() {
  // Sanity: legitimate interp `__toString` chain.
  let v = json(
    r#"
    let r = { __toString = self: "wrapped"; };
    in "before:${r}:after"
  "#,
  );
  assert_eq!(v, r#""before:wrapped:after""#);
}
