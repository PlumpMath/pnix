//! Regression cover for string-op edge cases.
//!
//! Three real bugs in this audit slice:
//!
//!   - `builtins.substring (-1) 3 "hello"` returned `"hel"`,
//!     silently treating the negative start as `0`. Real Nix
//!     errors with "negative start position".
//!   - `builtins.stringLength "안녕요"` returned `3` (codepoint
//!     count). Real Nix returns the *byte* length (`9` for three
//!     UTF-8 syllables × 3 bytes each). nixpkgs and the upstream
//!     test corpus both rely on byte semantics.
//!   - `builtins.concatStringsSep "," [ "a" 1 "c" ]` silently
//!     `to_json()`-ed the int element to `"1"`. Real Nix errors
//!     with "list element ... is not a string".
//!
//! Behaviours that already matched Nix and stayed unchanged
//! (negative `length` means "to end", `start > len` returns `""`,
//! `replaceStrings` empty pattern, basic `substring` slicing) are
//! pinned for cross-reference.
//!
//! UTF-8 boundary safety: `substring` snaps the byte end-index
//! down to the nearest char boundary so the slice is always
//! valid UTF-8, even if the requested length lands mid-codepoint.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── substring ───────────────────────────────────────────────────────

#[test]
fn substring_basic() {
  assert_eq!(json(r#"builtins.substring 0 3 "hello""#), r#""hel""#);
  assert_eq!(json(r#"builtins.substring 1 3 "hello""#), r#""ell""#);
}

#[test]
fn substring_zero_length_returns_empty() {
  assert_eq!(json(r#"builtins.substring 0 0 "hello""#), r#""""#);
}

#[test]
fn substring_negative_length_means_to_end() {
  assert_eq!(json(r#"builtins.substring 1 (-1) "hello""#), r#""ello""#);
}

#[test]
fn substring_length_past_end_clamps() {
  assert_eq!(json(r#"builtins.substring 2 100 "hello""#), r#""llo""#);
}

#[test]
fn substring_start_past_end_returns_empty() {
  assert_eq!(json(r#"builtins.substring 100 3 "hello""#), r#""""#);
}

#[test]
fn substring_negative_start_errors() {
  let m = err_msg(r#"builtins.substring (-1) 3 "hello""#);
  assert!(
    m.contains("negative start position"),
    "expected `negative start position`, got: {m}"
  );
}

#[test]
fn substring_byte_indices_with_utf8_safety() {
  // "héllo" — 'é' is 2 UTF-8 bytes (0xC3 0xA9). Byte indices 0..1
  // would split mid-char; the implementation snaps the end down
  // to the nearest char boundary, so substring 0 1 returns "h"
  // (1 byte before é).
  assert_eq!(json(r#"builtins.substring 0 1 "héllo""#), r#""h""#);
  // Byte indices 0..3 cover 'h' + 'é' (1 + 2 bytes). End at 3 is
  // a valid char boundary.
  assert_eq!(json(r#"builtins.substring 0 3 "héllo""#), r#""hé""#);
}

#[test]
fn substring_korean_byte_indexed() {
  // 빛 / 은 / space / 뭐 / 야 / ? — each Hangul syllable is 3
  // UTF-8 bytes, so "빛은" is bytes 0..6.
  assert_eq!(json(r#"builtins.substring 0 6 "빛은 뭐야?""#), r#""빛은""#);
}

// ── stringLength (byte semantics) ──────────────────────────────────

#[test]
fn string_length_ascii() {
  assert_eq!(json(r#"builtins.stringLength "hello""#), "5");
}

#[test]
fn string_length_utf8_counts_bytes() {
  // 'é' is 2 UTF-8 bytes — total 6 not 5.
  assert_eq!(json(r#"builtins.stringLength "héllo""#), "6");
}

#[test]
fn string_length_korean_counts_bytes() {
  // "안녕요" — 3 syllables × 3 bytes each = 9 bytes.
  assert_eq!(json(r#"builtins.stringLength "안녕요""#), "9");
}

#[test]
fn string_length_empty() {
  assert_eq!(json(r#"builtins.stringLength """#), "0");
}

// ── concatStringsSep ───────────────────────────────────────────────

#[test]
fn concat_strings_sep_basic() {
  assert_eq!(
    json(r#"builtins.concatStringsSep ", " [ "a" "b" "c" ]"#),
    r#""a, b, c""#
  );
}

#[test]
fn concat_strings_sep_empty_list() {
  assert_eq!(json(r#"builtins.concatStringsSep "," []"#), r#""""#);
}

#[test]
fn concat_strings_sep_single_element() {
  assert_eq!(json(r#"builtins.concatStringsSep "," [ "x" ]"#), r#""x""#);
}

#[test]
fn concat_strings_sep_non_string_element_errors() {
  let m = err_msg(r#"builtins.concatStringsSep "," [ "a" 1 "c" ]"#);
  assert!(
    m.contains("not a string") && m.contains("index 1"),
    "expected `not a string` mentioning index 1, got: {m}"
  );
}

#[test]
fn concat_strings_sep_non_string_separator_errors() {
  let m = err_msg(r#"builtins.concatStringsSep 42 [ "a" "b" ]"#);
  assert!(
    m.contains("separator must be string"),
    "expected `separator must be string`, got: {m}"
  );
}

// ── replaceStrings ─────────────────────────────────────────────────

#[test]
fn replace_strings_basic() {
  assert_eq!(
    json(r#"builtins.replaceStrings [ "a" "c" ] [ "X" "Z" ] "abc""#),
    r#""XbZ""#
  );
}

#[test]
fn replace_strings_preserves_utf8_when_copying_unmatched_text() {
  assert_eq!(
    json(r#"builtins.replaceStrings [ "$" ] [ "\$" ] "한국어 $HOME""#),
    r#""한국어 $HOME""#
  );
}

#[test]
fn replace_strings_empty_pattern_preserves_utf8_boundaries() {
  assert_eq!(
    json(r#"builtins.replaceStrings [""] ["|"] "한글""#),
    r#""|한|글|""#
  );
}

#[test]
fn replace_strings_no_match() {
  assert_eq!(
    json(r#"builtins.replaceStrings [ "x" ] [ "Y" ] "abc""#),
    r#""abc""#
  );
}

#[test]
fn replace_strings_empty_pattern_inserts_at_each_position() {
  // Real Nix: empty pattern → match before every char + at end.
  assert_eq!(
    json(r#"builtins.replaceStrings [""] ["X"] "abc""#),
    r#""XaXbXcX""#
  );
}
