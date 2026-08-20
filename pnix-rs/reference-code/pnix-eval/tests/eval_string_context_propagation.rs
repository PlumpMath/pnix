//! Regression cover for string-context propagation through
//! `concatStringsSep` / `substring` / `replaceStrings` /
//! `toString`.
//!
//! 2026-05-05 audit findings (slice #49):
//!   - `builtins.concatStringsSep "-" [ "a${./p1}" "b${./p2}" ]`
//!     silently produced a `Value::String` with empty context.
//!     Real Nix unions the contexts of every input string.
//!     Production-relevant: derivations rely on context
//!     propagation to track build-time dependencies. A user
//!     who joined paths via `concatStringsSep` would silently
//!     lose the dependency markers.
//!   - `builtins.substring 0 5 "x${./p}y"` silently lost
//!     context. The slice IS "from" the original
//!     context-bearing string, so it should inherit the
//!     provenance.
//!   - `builtins.replaceStrings [ "x" ] [ "y" ] "x${./p}"`
//!     lost the haystack's context. And
//!     `builtins.replaceStrings [ "x" ] [ "${./p}" ] "x"` lost
//!     the to-list element's context (when the to-element
//!     actually got used).
//!   - `builtins.toString "x${./p}"` lost context for the
//!     str-in case. (Non-string coercion has no context to
//!     start with, so that case is a no-op.)
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"concatStringsSep"` arm — unions separator + every
//!   element's context.
//! - `crates/pnix-eval/src/interpret.rs` `"substring"` arm —
//!   inherits input string's context unchanged.
//! - `crates/pnix-eval/src/interpret.rs` `"replaceStrings"`
//!   arm — unions haystack context + every `to[i]` context
//!   when the to-element actually gets used. The lazy `to[j]`
//!   for unused `j` keeps its existing slice-#38 unforced
//!   contract: its context is also NOT added if the pattern
//!   never matches.
//! - `crates/pnix-eval/src/interpret.rs` `"toString"` arm —
//!   inherits input string's context for the str-in case.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

// Helper: extract the JSON-encoded context attrset from a
// pnix expression that produces a context-bearing string.
fn ctx_json(expr: &str) -> String {
  json(&format!("builtins.toJSON (builtins.getContext ({}))", expr))
}

// ── concatStringsSep ───────────────────────────────────────────

#[test]
fn concat_strings_sep_unions_all_contexts() {
  // Was: empty context. Now: union of all path contexts.
  let ctx = ctx_json(r#"builtins.concatStringsSep "-" [ "a${./p1}" "b${./p2}" ]"#);
  assert!(ctx.contains("./p1"), "got: {ctx}");
  assert!(ctx.contains("./p2"), "got: {ctx}");
}

#[test]
fn concat_strings_sep_separator_context_propagates() {
  // The separator's context is also unioned.
  let ctx = ctx_json(r#"builtins.concatStringsSep "${./sep}" [ "a" "b" ]"#);
  assert!(ctx.contains("./sep"), "got: {ctx}");
}

#[test]
fn concat_strings_sep_no_context_unchanged() {
  // Sanity: non-context-bearing input produces no context.
  let ctx = ctx_json(r#"builtins.concatStringsSep "-" [ "a" "b" "c" ]"#);
  assert_eq!(ctx, r#""{}""#);
}

#[test]
fn concat_strings_sep_value_unchanged() {
  // The text value is unchanged from the slice-#38 contract.
  assert_eq!(
    json(r#"builtins.concatStringsSep "," [ "a" "b" "c" ]"#),
    r#""a,b,c""#
  );
}

// ── substring ──────────────────────────────────────────────────

#[test]
fn substring_inherits_input_context() {
  let ctx = ctx_json(r#"builtins.substring 0 5 "x${./p}y""#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn substring_no_context_unchanged() {
  let ctx = ctx_json(r#"builtins.substring 0 3 "abc""#);
  assert_eq!(ctx, r#""{}""#);
}

#[test]
fn substring_value_unchanged() {
  assert_eq!(json(r#"builtins.substring 1 2 "hello""#), r#""el""#);
}

// ── replaceStrings ─────────────────────────────────────────────

#[test]
fn replace_strings_haystack_context_propagates() {
  let ctx = ctx_json(r#"builtins.replaceStrings [ "x" ] [ "y" ] "x${./p}""#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn replace_strings_used_to_element_context_propagates() {
  // The `to` element's context is unioned only when the pattern
  // actually matches and the to-string gets used.
  let ctx = ctx_json(r#"builtins.replaceStrings [ "x" ] [ "${./p}" ] "x""#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn replace_strings_unused_to_element_context_not_added() {
  // Slice #38 / #49 invariant: a `to[j]` whose `from[j]` doesn't
  // match in the haystack is NOT forced. Same applies to its
  // context — it's not added if the to-element is never used.
  // (We verify this by checking that the pattern is unmatched
  // and the to-list contains a path-context string. The result
  // should have no context from that unused to-element.)
  let ctx = ctx_json(r#"builtins.replaceStrings [ "z" ] [ "${./p}" ] "abc""#);
  assert_eq!(ctx, r#""{}""#);
}

#[test]
fn replace_strings_haystack_and_to_unioned() {
  // Both haystack context and matched-to context are unioned.
  let ctx = ctx_json(r#"builtins.replaceStrings [ "x" ] [ "${./pTo}" ] "x${./pHay}""#);
  assert!(ctx.contains("./pTo"), "got: {ctx}");
  assert!(ctx.contains("./pHay"), "got: {ctx}");
}

#[test]
fn replace_strings_value_unchanged() {
  assert_eq!(
    json(r#"builtins.replaceStrings [ "a" "b" ] [ "X" "Y" ] "abc""#),
    r#""XYc""#
  );
}

// ── toString ───────────────────────────────────────────────────

#[test]
fn to_string_str_in_propagates_context() {
  let ctx = ctx_json(r#"builtins.toString "x${./p}""#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn to_string_int_in_no_context() {
  // Int has no context to start with.
  let ctx = ctx_json("builtins.toString 42");
  assert_eq!(ctx, r#""{}""#);
}

// ── parity reference: `+` operator already preserved context ──

#[test]
fn plus_operator_context_unchanged() {
  // Pre-existing behaviour, pinned for parity. The slice-#49 fix
  // brings the four other builtins in line with `+`.
  let ctx = ctx_json(r#""a" + "x${./p}""#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}
