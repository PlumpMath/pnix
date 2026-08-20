//! Regression cover for `builtins.hashFile` — newly added in
//! slice #68 to close the missing-builtin gap.
//!
//! 2026-05-05 audit findings (slice #68):
//!
//!   `builtins.hashFile algo path` was missing — pnix only
//!   implemented `hashString`. Real Nix has both. Production
//!   `.px` code that hashes file content (e.g., for content-
//!   addressed derivations or for verifying file integrity)
//!   would hit `attribute 'hashFile' not found`, which is a
//!   misleading error suggesting a typo or missing attribute
//!   rather than the actual issue (the builtin wasn't
//!   implemented in pnix).
//!
//!   The added implementation mirrors `hashString`'s production-
//!   grade algorithm policy: only SHA-256 and SHA-512
//!   supported; MD5 and SHA-1 explicitly rejected (they're
//!   cryptographically broken and unsuitable for content
//!   addressing in a 2026-era language).
//!
//!   Argument shape:
//!     - `args[0]`: algorithm string ("sha256" or "sha512")
//!     - `args[1]`: path (string or Path) — file to read
//!
//!   Result: hex digest as `Value::string_with_context`. Context
//!   inherits the path argument's context (the hash output
//!   depends on which path was hashed; preserving the path-
//!   context lets downstream consumers track the dependency).
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"hashFile"` arm — new implementation. Routes through
//!   `resolve_value_path` (so context-bearing strings, Path
//!   values, and string paths all work — slice #64 cascade)
//!   and through path normalization (slice #67 cascade).
//! - `crates/pnix-eval/src/interpret.rs` builtins
//!   registration list — `"hashFile"` added next to
//!   `"hashString"`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── basic existence + type ────────────────────────────────────

#[test]
fn hash_file_builtin_exists() {
  // Pre-fix: error "attribute 'hashFile' not found".
  // Post-fix: typeOf returns "lambda" (unevaluated builtin).
  assert_eq!(json(r#"builtins.typeOf builtins.hashFile"#), r#""lambda""#);
}

#[test]
fn hash_file_partial_application_returns_lambda() {
  // hashFile is curried: hashFile algo → partial → applied to path.
  assert_eq!(
    json(r#"builtins.typeOf (builtins.hashFile "sha256")"#),
    r#""lambda""#
  );
}

// ── happy path: sha256 / sha512 ───────────────────────────────

#[test]
fn hash_file_sha256_known_file() {
  // Hash this very test crate's Cargo.toml — must produce a
  // 64-char hex digest.
  let v = json(r#"builtins.hashFile "sha256" ./Cargo.toml"#);
  // Strip surrounding quotes and check length.
  let inner = v.trim_matches('"');
  assert_eq!(inner.len(), 64, "sha256 must be 64 hex chars: {v}");
  assert!(
    inner.chars().all(|c| c.is_ascii_hexdigit()),
    "sha256 must be hex: {v}"
  );
}

#[test]
fn hash_file_sha512_known_file() {
  let v = json(r#"builtins.hashFile "sha512" ./Cargo.toml"#);
  let inner = v.trim_matches('"');
  assert_eq!(inner.len(), 128, "sha512 must be 128 hex chars: {v}");
  assert!(
    inner.chars().all(|c| c.is_ascii_hexdigit()),
    "sha512 must be hex: {v}"
  );
}

#[test]
fn hash_file_deterministic_for_same_path() {
  // Same path, same algo → same hash.
  let v1 = json(r#"builtins.hashFile "sha256" ./Cargo.toml"#);
  let v2 = json(r#"builtins.hashFile "sha256" ./Cargo.toml"#);
  assert_eq!(v1, v2);
}

#[test]
fn hash_file_different_algos_different_results() {
  let v_sha256 = json(r#"builtins.hashFile "sha256" ./Cargo.toml"#);
  let v_sha512 = json(r#"builtins.hashFile "sha512" ./Cargo.toml"#);
  assert_ne!(v_sha256, v_sha512);
}

// ── algorithm policy (matches hashString) ──────────────────────

#[test]
fn hash_file_md5_rejected() {
  let e = err(r#"builtins.hashFile "md5" ./Cargo.toml"#);
  assert!(e.contains("hashFile"), "got: {e}");
  assert!(e.contains("'md5' is not supported"), "got: {e}");
  assert!(e.contains("cryptographically broken"), "got: {e}");
}

#[test]
fn hash_file_sha1_rejected() {
  let e = err(r#"builtins.hashFile "sha1" ./Cargo.toml"#);
  assert!(e.contains("hashFile"), "got: {e}");
  assert!(e.contains("'sha1' is not supported"), "got: {e}");
}

#[test]
fn hash_file_unsupported_algo_errors() {
  let e = err(r#"builtins.hashFile "blake2" ./Cargo.toml"#);
  assert!(e.contains("hashFile"), "got: {e}");
  assert!(e.contains("unsupported algorithm 'blake2'"), "got: {e}");
  assert!(e.contains("'sha256', 'sha512'"), "got: {e}");
}

// ── argument type guards ──────────────────────────────────────

#[test]
fn hash_file_non_string_algo_errors() {
  let e = err(r#"builtins.hashFile 42 ./Cargo.toml"#);
  assert!(e.contains("hashFile"), "got: {e}");
  assert!(e.contains("must be string"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn hash_file_non_path_arg_errors() {
  let e = err(r#"builtins.hashFile "sha256" 42"#);
  assert!(e.contains("hashFile"), "got: {e}");
  assert!(e.contains("expected string or path"), "got: {e}");
}

// ── missing file ──────────────────────────────────────────────

#[test]
fn hash_file_missing_path_errors_with_path() {
  let e = err(r#"builtins.hashFile "sha256" "/non-existent-xyz-test""#);
  assert!(e.contains("hashFile"), "got: {e}");
  assert!(e.contains("failed to read"), "got: {e}");
  assert!(e.contains("/non-existent-xyz-test"), "got: {e}");
}

// ── context inheritance from path argument ────────────────────

#[test]
fn hash_file_context_bearing_path_string_propagates_context() {
  // Path provided as context-bearing string (slice #64
  // cascade — accept), file may not exist but the type-check
  // passes. We verify the call DOESN'T error at the type-
  // check level — only at file-read level.
  let e = err(r#"builtins.hashFile "sha256" "${./not-a-real-file-xyz}""#);
  // Either errors at file-read (fine) or — if path normalize
  // happens to resolve — succeeds. The point: it does NOT
  // error at type-check (no "expected string or path"
  // message).
  assert!(
    !e.contains("expected string or path"),
    "got type error instead of FS error: {e}"
  );
}

// ── parity with hashString ────────────────────────────────────

#[test]
fn hash_string_and_hash_file_consistent_on_same_content() {
  // Hash the literal Cargo.toml content via hashString,
  // hashFile of the path. Both should produce the same SHA256
  // digest if the readFile content matches.
  // We use readFile to bridge: hashFile path == hashString
  // (readFile path).
  let from_file = json(r#"builtins.hashFile "sha256" ./Cargo.toml"#);
  let from_read = json(r#"builtins.hashString "sha256" (builtins.readFile ./Cargo.toml)"#);
  assert_eq!(from_file, from_read);
}
