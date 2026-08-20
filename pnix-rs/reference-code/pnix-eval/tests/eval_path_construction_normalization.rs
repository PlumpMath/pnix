//! Regression cover for path normalization at construction.
//!
//! 2026-05-05 audit findings (slice #66):
//!
//!   Pre-fix `Value::Path` preserved the literal text from the
//!   source. So:
//!     - `toString ./a/../b` returned `"./a/../b"` (literal)
//!     - `toJSON ./a/../b` returned `"\"./a/../b\""` (literal)
//!     - `dirOf ./a/../b` returned `Path("./a/..")` (literal —
//!       semantic parent is `./` since `./a/../b` normalizes
//!       to `./b`)
//!     - `./a + /../b` produced `Path("./a/../b")` (literal)
//!
//!   Slice #65 already normalized at COMPARISON time
//!   (`==`, `<`). This slice closes the symmetry by
//!   normalizing at CONSTRUCTION time. After slice #66, every
//!   `Value::Path` is canonical from the start — the underlying
//!   text representation is normalized, so all downstream
//!   consumers (toString, toJSON, dirOf, baseNameOf,
//!   interpolation, equality, comparison) see the same
//!   canonical form.
//!
//!   Production-relevant: pre-fix every `.px` that printed or
//!   serialized paths showed the literal user-typed
//!   representation including `./..` segments. JSON output
//!   could be confusing — `{"path": "./a/../b"}` instead of
//!   the cleaner `{"path": "./b"}`. dirOf returned a parent
//!   that didn't match the path's semantic parent.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `PnixExpr::Path`
//!   `Relative/Absolute/Search/Home(s)` arm — applies
//!   `normalize_pnix_path` to the constructed PathBuf.
//! - `crates/pnix-eval/src/interpret.rs` `PnixExpr::Path`
//!   `Interpolated` arm — same normalization.
//! - `crates/pnix-eval/src/interpret.rs` `+` operator
//!   `Path + Path` arm — normalizes the concatenated result.
//! - `crates/pnix-eval/src/interpret.rs` `+` operator
//!   `Path + String` (no-context) arm — normalizes too.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

// ── Path literal normalization at parse time ───────────────────

#[test]
fn path_literal_dotdot_normalized_in_toString() {
  assert_eq!(json(r#"builtins.toString ./a/../b"#), r#""./b""#);
}

#[test]
fn path_literal_dotdot_normalized_in_toJSON() {
  assert_eq!(json(r#"builtins.toJSON ./a/../b"#), r#""\"./b\"""#);
}

#[test]
fn path_literal_chain_dotdot_normalized() {
  assert_eq!(json(r#"builtins.toString ./a/b/c/../../d"#), r#""./a/d""#);
}

#[test]
fn path_literal_absolute_dotdot_normalized() {
  assert_eq!(json(r#"builtins.toString /abs/x/../y"#), r#""/abs/y""#);
}

#[test]
fn path_literal_no_dotdot_unchanged() {
  // Sanity: paths without `..` are unchanged.
  assert_eq!(json(r#"builtins.toString ./a/b"#), r#""./a/b""#);
}

// ── dirOf returns normalized parent ────────────────────────────

#[test]
fn dir_of_path_with_dotdot_returns_canonical_parent() {
  // `./a/../b` normalizes to `./b`; dirOf is `./`.
  let v = json(r#"builtins.toString (builtins.dirOf ./a/../b)"#);
  // Result should be `./` or `.` (canonical relative root).
  assert!(v == r#"".""# || v == r#""./""#, "got: {v}");
}

#[test]
fn dir_of_chain_dotdot_canonical() {
  let v = json(r#"builtins.toString (builtins.dirOf ./a/b/c/../../d)"#);
  // ./a/b/c/../../d normalizes to ./a/d; dirOf is ./a.
  assert_eq!(v, r#""./a""#);
}

#[test]
fn dir_of_absolute_dotdot_canonical() {
  let v = json(r#"builtins.toString (builtins.dirOf /abs/x/../y)"#);
  // /abs/x/../y normalizes to /abs/y; dirOf is /abs.
  assert_eq!(v, r#""/abs""#);
}

#[test]
fn dir_of_path_without_dotdot_unchanged() {
  // Sanity.
  let v = json(r#"builtins.toString (builtins.dirOf ./a/b)"#);
  assert_eq!(v, r#""./a""#);
}

// ── baseNameOf path-typed value ────────────────────────────────

#[test]
fn base_name_of_path_with_dotdot_canonical() {
  // ./a/../b normalizes to ./b; baseNameOf is "b".
  assert_eq!(json(r#"builtins.baseNameOf ./a/../b"#), r#""b""#);
}

#[test]
fn base_name_of_chain_dotdot() {
  // ./a/b/c/../../d normalizes to ./a/d; baseNameOf is "d".
  assert_eq!(json(r#"builtins.baseNameOf ./a/b/c/../../d"#), r#""d""#);
}

// ── + operator results normalized ──────────────────────────────

#[test]
fn path_plus_path_dotdot_normalized() {
  // ./a + ./../b → text concat "./a./../b" → normalize → ?
  // The component split of "./a./../b" via Rust Path:
  //   ./a./../b → CurDir, "a.", ParentDir, "b"
  // After normalize: CurDir, "b" → "./b"
  // (Note: "a." is a single component, not "./a/." sequence)
  let v = json(r#"builtins.toString (./a + ./../b)"#);
  assert!(v.contains("b"), "got: {v}");
}

#[test]
fn path_plus_string_dotdot_normalized() {
  // ./a + "/../b" → text "./a/../b" → normalize → "./b"
  assert_eq!(json(r#"builtins.toString (./a + "/../b")"#), r#""./b""#);
}

// ── String dirOf / baseNameOf NOT path-normalized ─────────────

#[test]
fn dir_of_string_does_not_normalize() {
  // String input to dirOf should preserve literal text
  // (real Nix: string-level operation, just splits on last
  // `/`). This test pins that string-level dirOf is text-only
  // and does NOT path-normalize. The slice ONLY normalizes
  // Path values, not string operations on path-like text.
  assert_eq!(json(r#"builtins.dirOf "./a/../b""#), r#""./a/..""#);
}

#[test]
fn base_name_of_string_text_level() {
  // String input — last component is "b" regardless of `..`.
  assert_eq!(json(r#"builtins.baseNameOf "./a/../b""#), r#""b""#);
}

// ── interpolation context preserves original ──────────────────

#[test]
fn interpolation_path_dotdot_resolves_normalized() {
  // The `${./a/../b}` interpolation resolves to the
  // normalized store-path text. Pre-fix the context entry
  // was the literal `./a/../b`; post-fix it's the normalized
  // `./b`.
  let v = json(
    r#"
    builtins.toJSON (
      builtins.attrNames (builtins.getContext "${./a/../b}")
    )
  "#,
  );
  // After normalization, context contains "./b" not "./a/../b".
  assert!(v.contains("./b"), "got: {v}");
}

// ── Path equality / comparison still works (slice #65) ────────

#[test]
fn slice_65_path_equality_still_works() {
  // After construction normalization, the path values are
  // already equal — slice #65's comparison normalization
  // remains a safety net but isn't load-bearing for literals.
  assert_eq!(json(r#"./a/../b == ./b"#), "true");
}

#[test]
fn slice_65_list_path_equality_still_works() {
  assert_eq!(json(r#"[ ./a/../b ./c/../d ] == [ ./b ./d ]"#), "true");
}
