//! Audit-clean baselines for `builtins.match` / `builtins.split`.
//!
//! The 2026-05-04 regex probe found no semantic bugs — every case
//! matches the Nix manual + the earlier 2026-05-04 split-adjacent-
//! match fix (`crates/pnix-eval/src/interpret.rs::split`). Pinned
//! here so future regex-engine swaps or capture-shape rewrites
//! cannot silently break what `.px` / nixpkgs lib code now relies
//! on.
//!
//! Production guarantees pinned:
//!   - `match` is anchored by default (full-string match only)
//!   - no match → null (not [])
//!   - whole-string match with no captures → [] (not null)
//!   - optional capture group that didn't match → null inside list
//!   - empty pattern matches empty string only ("" / "" → [])
//!   - invalid regex → error with parse-detail message (fail-loud)
//!   - unicode character classes (`[가-힣]+`) work
//!   - `split` returns alternating `[lit, [captures], lit, ...]`,
//!     including empty strings between adjacent matches
//!   - `split` with empty pattern is rejected (avoids infinite-
//!     loop / undefined behaviour)

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── builtins.match ─────────────────────────────────────────────────

#[test]
fn match_no_match_returns_null() {
  assert_eq!(json(r#"builtins.match "abc" "xyz""#), "null");
}

#[test]
fn match_full_string_no_captures_returns_empty_list() {
  // Pattern matches full input, no capture groups → []
  assert_eq!(json(r#"builtins.match "abc" "abc""#), "[]");
}

#[test]
fn match_partial_input_does_not_match() {
  // Anchored: pattern must match entire string.
  assert_eq!(json(r#"builtins.match "ab" "abc""#), "null");
}

#[test]
fn match_with_explicit_dot_star_anchor() {
  // `.*` must be added explicitly when the user wants partial match.
  assert_eq!(json(r#"builtins.match "ab.*" "abc""#), "[]");
}

#[test]
fn match_capture_groups() {
  assert_eq!(
    json(r#"builtins.match "(a+)(b+)" "aaabb""#),
    r#"["aaa","bb"]"#
  );
}

#[test]
fn match_optional_capture_unmatched_is_null_inside_list() {
  // The optional `(a)?` doesn't match, so its position is `null`.
  assert_eq!(json(r#"builtins.match "(a)?(b)" "b""#), r#"[null,"b"]"#);
}

#[test]
fn match_empty_pattern_empty_string() {
  assert_eq!(json(r#"builtins.match "" """#), "[]");
}

#[test]
fn match_invalid_regex_errors_with_parse_detail() {
  let m = err_msg(r#"builtins.match "[invalid" "x""#);
  assert!(m.contains("invalid regex"), "got: {m}");
  assert!(m.contains("unclosed"), "got: {m}");
}

#[test]
fn match_unicode_korean_class() {
  // [가-힣] matches a Hangul syllable.
  assert_eq!(json(r#"builtins.match "[가-힣]+" "안녕""#), "[]");
}

#[test]
fn match_unicode_capture_returns_text() {
  assert_eq!(
    json(r#"builtins.match "([가-힣]+)-(.+)" "안녕-world""#),
    r#"["안녕","world"]"#
  );
}

// ── builtins.split ────────────────────────────────────────────────

#[test]
fn split_no_match_returns_single_element() {
  // Nix: when pattern doesn't appear → list with the input as
  // its only literal segment.
  assert_eq!(json(r#"builtins.split "xyz" "abc""#), r#"["abc"]"#);
}

#[test]
fn split_basic_alternating_shape() {
  // `[ab]` matches both `a` and `b` adjacently. Result interleaves
  // literal segments with capture lists; empty literals fill
  // between adjacent matches (the 2026-05-04 split-adjacency fix).
  assert_eq!(
    json(r#"builtins.split "[ab]" "abc""#),
    r#"["",[],"",[],"c"]"#
  );
}

#[test]
fn split_with_capture_groups() {
  // The captures from each match show up as a list between the
  // surrounding literal segments.
  assert_eq!(
    json(r#"builtins.split "(a)(b)" "ab-ab""#),
    r#"["",["a","b"],"-",["a","b"],""]"#
  );
}

#[test]
fn split_empty_pattern_errors() {
  // Real Nix and pnix both reject empty-pattern split (would
  // otherwise infinite-loop on insertion-between-every-byte).
  let m = err_msg(r#"builtins.split "" "abc""#);
  assert!(m.contains("pattern cannot be empty"), "got: {m}");
}

#[test]
fn split_invalid_regex_errors() {
  let m = err_msg(r#"builtins.split "[invalid" "x""#);
  assert!(m.contains("invalid regex"), "got: {m}");
}
