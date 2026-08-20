//! Regression cover for `builtins.abort` non-string guard +
//! `with`-source non-attrset force-time error.
//!
//! 2026-05-05 audit findings (slice #37):
//!   - **`builtins.abort` was silently coercing non-string args
//!     to their `Display` representation**: `abort 42` produced
//!     `"evaluation aborted: 42"`, `abort [ 1 ]` produced
//!     `"evaluation aborted: [1]"`, `abort { a = 1; }` etc.
//!     `builtins.throw` already errored on non-string with
//!     "argument must be string, got <value>" — `abort` was the
//!     inconsistent one. Real Nix rejects non-string at both.
//!     Now matched: `builtins.abort: argument must be string,
//!     got <value>`. The `evaluation aborted: ` prefix (which
//!     slice #35's `tryEval` abort-propagation marker keys off)
//!     stays load-bearing for valid string arguments — it's
//!     emitted only after the type check passes.
//!   - **`with non-attrset; identifier-not-in-outer-scope` was
//!     hiding the with-source type error.** `with 42; foo`
//!     errored "undefined variable: foo" instead of
//!     "with: argument must be attrset, got int". An author
//!     writing `with myConfig; foo` where `myConfig` came back
//!     from a misconfigured loader as `null` / `42` / `[ ... ]`
//!     would chase a missing `foo` definition for hours when
//!     the actual bug is the `with` source's type. The lazy
//!     contract on `with` is preserved: the source is still
//!     only forced when a body name lookup actually has to
//!     consult the with-frame, so `with 42; 1` still returns
//!     `1` without ever evaluating the `42`.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"abort"` arm — string-only guard mirroring `"throw"`.
//!   `type_name` exported as `pub(crate)` for use from
//!   `value.rs`.
//! - `crates/pnix-eval/src/value.rs` `WithFrame::force` —
//!   non-attrset source returns `Err` instead of falling back
//!   to an empty map.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── builtins.abort string-only ─────────────────────────────────

#[test]
fn abort_string_message_propagates() {
  let m = err_msg(r#"builtins.abort "boom""#);
  assert!(m.contains("evaluation aborted"), "got: {m}");
  assert!(m.contains("boom"), "got: {m}");
}

#[test]
fn abort_int_arg_errors_with_type_message() {
  // Was: silently produced "evaluation aborted: 42".
  let m = err_msg("builtins.abort 42");
  assert!(m.contains("builtins.abort"), "got: {m}");
  assert!(m.contains("must be string"), "got: {m}");
  assert!(m.contains("42"), "got: {m}");
}

#[test]
fn abort_list_arg_errors() {
  let m = err_msg("builtins.abort [ 1 2 ]");
  assert!(m.contains("must be string"), "got: {m}");
}

#[test]
fn abort_null_arg_errors() {
  let m = err_msg("builtins.abort null");
  assert!(m.contains("must be string"), "got: {m}");
}

#[test]
fn abort_attrset_arg_errors() {
  let m = err_msg(r#"builtins.abort { a = 1; }"#);
  assert!(m.contains("must be string"), "got: {m}");
}

#[test]
fn abort_via_global_alias_string_message() {
  // `abort` is a global builtin alias — same shape.
  let m = err_msg(r#"abort "via-global""#);
  assert!(m.contains("evaluation aborted"), "got: {m}");
  assert!(m.contains("via-global"), "got: {m}");
}

#[test]
fn abort_via_global_alias_int_errors() {
  let m = err_msg("abort 42");
  assert!(m.contains("must be string"), "got: {m}");
}

#[test]
fn abort_marker_still_propagates_through_try_eval() {
  // Slice #35 invariant: tryEval re-raises on the
  // "evaluation aborted:" marker. The string-arg type check
  // happens before the marker is emitted, so a valid string
  // abort still propagates out of tryEval.
  let m = err_msg(r#"builtins.tryEval (builtins.abort "stop")"#);
  assert!(m.contains("evaluation aborted"), "got: {m}");
  assert!(m.contains("stop"), "got: {m}");
}

#[test]
fn abort_marker_does_not_propagate_when_type_check_fails() {
  // If abort's argument is non-string, the error is the
  // type-check failure (not "evaluation aborted: ..."). That
  // means tryEval *can* catch this one — it's a regular
  // builtin error, not an abort. This is the right shape: the
  // language is reporting "you called abort wrong" as a
  // recoverable type error, separate from the unrecoverable
  // hard-fail that abort is supposed to express.
  assert_eq!(
    json("(builtins.tryEval (builtins.abort 42)).success"),
    "false"
  );
}

// ── `with` non-attrset force-time error ────────────────────────

#[test]
fn with_lazy_unforced_when_body_doesnt_use_scope() {
  // The `with` source is never forced if the body doesn't fall
  // through to it.
  assert_eq!(json("with 42; 1"), "1");
  assert_eq!(json(r#"with "hi"; 1"#), "1");
  assert_eq!(json("with null; 1"), "1");
  assert_eq!(json("with [ 1 2 ]; 1"), "1");
}

#[test]
fn with_lazy_unforced_when_lexical_lookup_succeeds() {
  // Lexical lookup wins, so `x` is found in the let, not in
  // the with-frame. With-source still never forced.
  assert_eq!(json("with 42; let x = 99; in x"), "99");
}

#[test]
fn with_throw_source_not_forced_until_lookup() {
  // Sanity from earlier slice: a throwing source is still
  // dormant until a name lookup actually consults it.
  assert_eq!(json(r#"with (throw "boom"); 1"#), "1");
}

#[test]
fn with_int_source_errors_when_lookup_falls_through() {
  // Was: silently fell through with-empty-map → "undefined
  // variable: foo". Now: typed error pinning the with-source
  // type.
  let m = err_msg("with 42; foo");
  assert!(m.contains("with: argument must be attrset"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn with_string_source_errors_when_lookup_falls_through() {
  let m = err_msg(r#"with "hello"; foo"#);
  assert!(m.contains("with: argument must be attrset"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn with_list_source_errors_when_lookup_falls_through() {
  let m = err_msg("with [ 1 2 ]; foo");
  assert!(m.contains("with: argument must be attrset"), "got: {m}");
  assert!(m.contains("got list"), "got: {m}");
}

#[test]
fn with_null_source_errors_when_lookup_falls_through() {
  // Most useful real-world case: a misconfigured loader
  // returning `null` from a JSON parse, used in a `with` chain.
  let m = err_msg("with null; foo");
  assert!(m.contains("with: argument must be attrset"), "got: {m}");
  assert!(m.contains("got null"), "got: {m}");
}

#[test]
fn with_throw_source_propagates_when_lookup_falls_through() {
  // The source error wins — if forcing the with-source itself
  // throws, that's the error the user sees, not the
  // with-not-attrset shape.
  let m = err_msg(r#"with (throw "src-boom"); foo"#);
  assert!(m.contains("src-boom"), "got: {m}");
}

#[test]
fn with_attrset_source_resolves_normally() {
  // Sanity: the happy path still works.
  assert_eq!(json("with { a = 1; b = 2; }; a + b"), "3");
}

#[test]
fn with_inner_attrset_wins_over_outer_int() {
  // Stacked `with`: inner is an attrset and provides the name,
  // so the outer `with 42` is never forced. No error.
  assert_eq!(json("with 42; with { x = 99; }; x"), "99");
}

#[test]
fn with_outer_int_errors_when_inner_doesnt_have_name() {
  // Stacked `with`: inner attrset doesn't have the name, so
  // lookup falls through to outer. Outer is `42` → error.
  let m = err_msg("with 42; with { x = 99; }; missing");
  assert!(m.contains("with: argument must be attrset"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn with_lexical_let_bypasses_outer_int() {
  // `let` introduces lexical scope that's checked before
  // any with-frame, so `with 42; let foo = 1; in foo` works
  // without forcing the outer.
  assert_eq!(json("with 42; let foo = 1; in foo"), "1");
}
