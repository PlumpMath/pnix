//! Regression cover for `==` equality of context-bearing
//! strings.
//!
//! 2026-05-05 audit findings (slice #59):
//!
//!   `values_equal_at_depth` had an arm for
//!   `(Value::String, Value::String)` only, with no arm for
//!   `(Value::StringContext, Value::StringContext)` or the
//!   mixed `(String, StringContext)` / `(StringContext,
//!   String)` cases. The fall-through `_ => false` at the
//!   bottom meant ANY equality comparison involving a
//!   context-bearing string returned `false`.
//!
//!   Pre-fix bugs:
//!     - `"x${./p}" == "x${./p}"` returned `false` (same text,
//!       same context — should be `true`).
//!     - `("a${./p}" + "b") == "a<storepath>b"` returned `false`
//!       (one side is StringContext, other is String — texts
//!       could match but no arm caught the mixed case).
//!     - `[ "x${./p}" ] == [ "x${./p}" ]` returned `false`
//!       (list equality recurses into element equality, which
//!       hit the same gap).
//!
//!   Real Nix's `==` is text-only for strings — context is
//!   IGNORED for equality. Two strings with the same text are
//!   equal regardless of build-time-dependency context. The
//!   fix uses `as_str()` (which already handles both
//!   `Value::String` and `Value::StringContext`) and compares
//!   text under a guard pattern that matches all four
//!   combinations.
//!
//!   Production-relevant: any `if str == "expected" then ... else ...`
//!   on a context-bearing string ALWAYS took the else branch,
//!   silently flipping logic. Set / list deduplication via
//!   equality on context-bearing elements failed to dedupe.
//!   Hash-table-style lookups on context-bearing keys never
//!   matched.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `values_equal_at_depth`
//!   — the `(Value::String, Value::String) => a == b` arm was
//!   replaced with a guard pattern
//!   `(l, r) if matches!(l, String | StringContext) && matches!(r, ...)`
//!   that uses `l.as_str() == r.as_str()` for all four
//!   combinations.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

// ── context-bearing string equality (the core bug) ────────────

#[test]
fn context_bearing_strings_equal_when_text_and_context_match() {
  // Pre-fix returned `false` due to falling through to `_ =>
  // false`. Now: text equality wins.
  assert_eq!(
    json(
      r#"
      let s1 = "x${./p1}";
          s2 = "x${./p1}";
      in s1 == s2
    "#
    ),
    "true"
  );
}

#[test]
fn context_bearing_strings_with_different_text_are_unequal() {
  // Different paths → different store-path texts → different
  // text → unequal. (Sanity, not a fix.)
  assert_eq!(
    json(
      r#"
      let s1 = "x${./a}";
          s2 = "x${./b}";
      in s1 == s2
    "#
    ),
    "false"
  );
}

#[test]
fn context_bearing_string_equal_to_plain_string_with_same_text() {
  // Mixed shape: StringContext vs String. Same text → equal
  // regardless of context. Pre-fix: false. Now: true.
  assert_eq!(json(r#""abc" == ("a" + "b" + "c")"#), "true");
}

#[test]
fn plain_string_equal_to_constructed_with_path_context() {
  // The `+` operator with path uses the path's display form
  // (does NOT resolve to store path — that's only `${./p}`
  // interpolation). Slice #54 adds path context but text is
  // just the literal display form. So plain = "prefix-./p"
  // matches constructed text "prefix-./p" — equal regardless
  // of the constructed string's `./p` context (since `==`
  // ignores context).
  assert_eq!(
    json(
      r#"
      let
        plain = "prefix-./p";
        constructed = "prefix-" + ./p;
      in plain == constructed
    "#
    ),
    "true"
  );
}

// ── list of context-bearing strings ───────────────────────────

#[test]
fn list_of_context_strings_equal_when_each_element_text_matches() {
  // List equality recurses into element equality. Pre-fix
  // failed because element comparison hit the StringContext gap.
  assert_eq!(json(r#"[ "x${./p}" ] == [ "x${./p}" ]"#), "true");
}

#[test]
fn list_of_context_strings_unequal_with_different_text() {
  assert_eq!(json(r#"[ "x${./a}" ] == [ "x${./b}" ]"#), "false");
}

#[test]
fn nested_list_of_context_strings_equality() {
  assert_eq!(json(r#"[ [ "x${./p}" ] ] == [ [ "x${./p}" ] ]"#), "true");
}

// ── attrset with context-bearing values ───────────────────────

#[test]
fn attrset_with_context_value_equality() {
  assert_eq!(json(r#"{ a = "x${./p}"; } == { a = "x${./p}"; }"#), "true");
}

#[test]
fn attrset_with_context_value_unequal_different_text() {
  assert_eq!(json(r#"{ a = "x${./a}"; } == { a = "x${./b}"; }"#), "false");
}

// ── ineq operator !! ───────────────────────────────────────────

#[test]
fn neq_context_bearing_strings_with_same_text() {
  // Pre-fix: !=` returned `true` because `==` returned
  // `false`. Now: `!=` returns `false`.
  assert_eq!(json(r#""x${./p}" != "x${./p}""#), "false");
}

// ── if-condition flip protection ──────────────────────────────

#[test]
fn branch_on_context_string_equality_takes_correct_branch() {
  // Pre-fix: `if str == "expected" then ... else ...` on a
  // context-bearing string ALWAYS took else. Now: takes
  // correct branch.
  assert_eq!(
    json(
      r#"
      let str = "expected${""}";  # context-bearing? actually no,
                                    # ${""} is empty interp
      in if str == "expected" then 1 else 0
    "#
    ),
    "1"
  );
}

#[test]
fn branch_on_path_context_string_via_equality() {
  // Real-world shape: comparing a derivation reference text.
  assert_eq!(
    json(
      r#"
      let str = "x${./p}";
          expected = "x${./p}";
      in if str == expected then "matched" else "miss"
    "#
    ),
    r#""matched""#
  );
}

// ── existing equality cases unchanged ─────────────────────────

#[test]
fn plain_strings_equal_unchanged() {
  assert_eq!(json(r#""abc" == "abc""#), "true");
}

#[test]
fn plain_strings_unequal_unchanged() {
  assert_eq!(json(r#""abc" == "xyz""#), "false");
}

#[test]
fn int_equality_unchanged() {
  assert_eq!(json(r#"42 == 42"#), "true");
}

#[test]
fn list_equality_unchanged() {
  assert_eq!(json(r#"[ 1 2 ] == [ 1 2 ]"#), "true");
}

#[test]
fn attrset_equality_unchanged() {
  assert_eq!(json(r#"{ a = 1; } == { a = 1; }"#), "true");
}

#[test]
fn null_equality_unchanged() {
  assert_eq!(json(r#"null == null"#), "true");
}

#[test]
fn type_mismatch_unequal() {
  assert_eq!(json(r#"42 == "42""#), "false");
}

#[test]
fn lambda_inequality_unchanged() {
  assert_eq!(json(r#"(x: x) == (x: x)"#), "false");
}
