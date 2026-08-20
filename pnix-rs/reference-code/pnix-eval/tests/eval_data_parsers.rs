//! Regression cover for data-parser / encoder / hash builtins:
//! `fromTOML`, `hashString`, `seq` / `deepSeq`, `toFile`.
//!
//! 2026-05-04 audit findings:
//!   - **two missing builtins added**:
//!     `builtins.fromTOML` (TOML → attrset, structurally identical
//!     shape to `fromJSON`'s output) and
//!     `builtins.hashString` (SHA-256 / SHA-512 only).
//!     Previously calling either errored with `attribute 'X' not
//!     found`, which silently broke any nixpkgs-flavoured `.px`
//!     code that mixes JSON + TOML configs or builds content-
//!     addressed cache keys.
//!   - **production hashing policy**: MD5 and SHA-1 are
//!     explicitly rejected with a message naming them as
//!     "cryptographically broken — use 'sha256' or 'sha512'",
//!     even though older Nix accepts them. pnix is a 2026-era
//!     language; legacy hashes don't belong in content
//!     addressing or cache keys.
//!   - the other behaviours (`seq` / `deepSeq` throw paths,
//!     `toFile` non-string content guard) already match Nix +
//!     production fail-loud; pinned alongside.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── builtins.fromTOML ──────────────────────────────────────────

#[test]
fn from_toml_basic_section_with_keys() {
  let v = json(r#"builtins.fromTOML "[section]\nkey = \"val\"\nnum = 42""#);
  assert!(
    v.contains(r#""section":{"key":"val","num":42}"#),
    "got: {v}"
  );
}

#[test]
fn from_toml_top_level_keys() {
  assert_eq!(
    json(r#"builtins.fromTOML "name = \"pnix\"\nyear = 2026""#),
    r#"{"name":"pnix","year":2026}"#
  );
}

#[test]
fn from_toml_array() {
  assert_eq!(
    json(r#"builtins.fromTOML "vals = [1, 2, 3]""#),
    r#"{"vals":[1,2,3]}"#
  );
}

#[test]
fn from_toml_nested_table() {
  let v = json(r#"builtins.fromTOML "[a.b.c]\nx = 1""#);
  assert_eq!(v, r#"{"a":{"b":{"c":{"x":1}}}}"#);
}

#[test]
fn from_toml_bool_and_float() {
  let v = json(r#"builtins.fromTOML "flag = true\npi = 3.14""#);
  assert!(v.contains(r#""flag":true"#));
  assert!(v.contains(r#""pi":3.14"#));
}

#[test]
fn from_toml_invalid_errors() {
  let m = err_msg(r#"builtins.fromTOML "not [valid TOML""#);
  assert!(m.contains("parse error"), "got: {m}");
}

#[test]
fn from_toml_non_string_errors() {
  let m = err_msg(r#"builtins.fromTOML 42"#);
  assert!(m.contains("expected string"), "got: {m}");
}

#[test]
fn from_toml_empty_string_returns_empty_attrset() {
  // An empty TOML document is a valid empty top-level table.
  assert_eq!(json(r#"builtins.fromTOML """#), "{}");
}

// ── builtins.hashString ────────────────────────────────────────

#[test]
fn hash_string_sha256_hello() {
  // SHA-256 of "hello" — golden hex digest.
  assert_eq!(
    json(r#"builtins.hashString "sha256" "hello""#),
    r#""2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824""#
  );
}

#[test]
fn hash_string_sha256_empty() {
  // SHA-256 of empty string — also a known golden value.
  assert_eq!(
    json(r#"builtins.hashString "sha256" """#),
    r#""e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855""#
  );
}

#[test]
fn hash_string_sha512_length() {
  // SHA-512 hex digest is 128 chars (512 bits / 4 bits per hex).
  let v = json(r#"builtins.hashString "sha512" "hello""#);
  // v includes the surrounding `"…"` JSON quotes.
  assert_eq!(v.len(), 130, "got: {v}");
}

#[test]
fn hash_string_md5_explicitly_rejected() {
  // Production policy: MD5 is cryptographically broken.
  let m = err_msg(r#"builtins.hashString "md5" "hello""#);
  assert!(m.contains("cryptographically broken"), "got: {m}");
  assert!(m.contains("sha256") || m.contains("sha512"), "got: {m}");
}

#[test]
fn hash_string_sha1_explicitly_rejected() {
  // Same: SHA-1 is also explicitly rejected.
  let m = err_msg(r#"builtins.hashString "sha1" "hello""#);
  assert!(m.contains("cryptographically broken"), "got: {m}");
}

#[test]
fn hash_string_unknown_algo_errors() {
  let m = err_msg(r#"builtins.hashString "blake42" "hello""#);
  assert!(m.contains("unsupported algorithm"), "got: {m}");
  assert!(m.contains("'sha256', 'sha512'"), "got: {m}");
}

#[test]
fn hash_string_non_string_data_errors() {
  let m = err_msg(r#"builtins.hashString "sha256" 42"#);
  assert!(
    m.contains("data") && m.contains("must be string"),
    "got: {m}"
  );
}

#[test]
fn hash_string_non_string_algo_errors() {
  let m = err_msg(r#"builtins.hashString 42 "hello""#);
  assert!(
    m.contains("algo") && m.contains("must be string"),
    "got: {m}"
  );
}

// ── seq / deepSeq throw paths ──────────────────────────────────

#[test]
fn seq_returns_second_when_first_evaluates() {
  assert_eq!(json("builtins.seq 1 42"), "42");
}

#[test]
fn seq_propagates_first_throw() {
  let m = err_msg(r#"builtins.seq (throw "boom") 42"#);
  assert!(m.contains("boom"), "got: {m}");
}

#[test]
fn deep_seq_forces_inside_lists() {
  // deepSeq is recursive — a thrown value buried inside a list
  // must surface (the production guarantee for pre-flight
  // validation).
  let m = err_msg(r#"builtins.deepSeq [ (throw "deep") ] 42"#);
  assert!(m.contains("deep"), "got: {m}");
}

#[test]
fn deep_seq_returns_second_when_no_throw() {
  assert_eq!(json(r#"builtins.deepSeq [ 1 2 [ 3 4 ] ] 99"#), "99");
}

// ── toFile ─────────────────────────────────────────────────────

#[test]
fn to_file_returns_fake_store_path() {
  // pnix has no nix-store, so toFile returns a deterministic
  // fake-store path in /tmp/pnix-nix-store/. The shape is the
  // contract; the prefix stays inside `/tmp/pnix-nix-store/`.
  let v = json(r#"builtins.toFile "test.txt" "hello""#);
  assert!(v.contains("pnix-nix-store"), "got: {v}");
  assert!(v.contains("test.txt"), "got: {v}");
}

#[test]
fn to_file_non_string_content_errors() {
  let m = err_msg(r#"builtins.toFile "test.txt" 42"#);
  assert!(m.contains("must be string"), "got: {m}");
}
