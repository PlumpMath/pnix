//! Regression cover for `builtins.derivation` — closes the
//! missing-builtin gap, extends slice #68 / #69 / #70 / #77
//! family.
//!
//! 2026-05-06 audit findings (slice #80):
//!
//!   `builtins.derivation { name; system; builder; }` errored
//!   with `attribute 'derivation' not found`. Pre-fix shape:
//!   pnix had `derivationStrict` (the low-level variant) since
//!   long ago, but the high-level `derivation` was missing. Real
//!   Nix exposes BOTH:
//!     - `derivation`        — high-level, returns a derivation
//!                             value with standard fields
//!                             (`outPath` / `drvPath` /
//!                             `type = "derivation"`) wired in
//!     - `derivationStrict`  — low-level, returns the strict
//!                             attrs directly
//!   Most nixpkgs-style code calls `builtins.derivation`
//!   (directly or via `mkDerivation` / `stdenv.mkDerivation`
//!   wrappers). The missing-builtin gap blocked any `.px` author
//!   trying to use the canonical Nix idiom.
//!
//!   Production-relevant: `mkDerivation` (the most common entry
//!   point in nixpkgs-style code) eventually compiles to a
//!   `builtins.derivation` call. Without it, every derivation-
//!   producing `.px` had to use `derivationStrict` directly,
//!   which is not what nixpkgs-style code does.
//!
//!   Note: `isDerivation` is intentionally NOT a pnix builtin —
//!   real Nix has it in `lib.isDerivation`, not
//!   `builtins.isDerivation`. pnix matches.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin` —
//!   new `"derivation"` arm. Mirrors the structure of the
//!   `"derivationStrict"` arm: clones the input attrs, then
//!   layers `outPath` / `drvPath` / `type = "derivation"` on
//!   top via `or_insert` so user-provided overrides win. The
//!   result attrset round-trips through both call sites
//!   uniformly.
//! - `crates/pnix-eval/src/interpret.rs` BUILTIN registration
//!   list — added `"derivation"` alongside `"derivationStrict"`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── Basic shape ───────────────────────────────────────────────

#[test]
fn derivation_simple_returns_attrset_with_standard_fields() {
  let out = json(
    r#"
    builtins.derivation {
      name = "hello";
      system = "x86_64-linux";
      builder = "/bin/sh";
    }
  "#,
  );
  // Standard fields: name, system, builder, drvPath, outPath, type
  assert!(out.contains(r#""name":"hello""#), "got: {out}");
  assert!(out.contains(r#""system":"x86_64-linux""#), "got: {out}");
  assert!(out.contains(r#""builder":"/bin/sh""#), "got: {out}");
  assert!(out.contains(r#""drvPath""#), "got: {out}");
  assert!(out.contains(r#""outPath""#), "got: {out}");
  assert!(out.contains(r#""type":"derivation""#), "got: {out}");
}

#[test]
fn derivation_type_field_is_derivation_string() {
  assert_eq!(
    json(
      r#"
      (builtins.derivation { name = "x"; system = "x"; builder = "x"; }).type
    "#
    ),
    r#""derivation""#
  );
}

#[test]
fn derivation_name_passes_through() {
  assert_eq!(
    json(
      r#"
      (builtins.derivation { name = "myname"; system = "x"; builder = "x"; }).name
    "#
    ),
    r#""myname""#
  );
}

#[test]
fn derivation_outpath_is_path_shaped_string() {
  // outPath should be a string referencing the derivation's
  // placeholder path.
  let out = json(
    r#"
    (builtins.derivation { name = "abc"; system = "x"; builder = "x"; }).outPath
  "#,
  );
  assert!(out.contains("abc"), "got: {out}");
  assert!(out.contains("derivation"), "got: {out}");
}

#[test]
fn derivation_drvpath_is_path_shaped_string() {
  let out = json(
    r#"
    (builtins.derivation { name = "xyz"; system = "x"; builder = "x"; }).drvPath
  "#,
  );
  assert!(out.contains("xyz"), "got: {out}");
}

// ── Match derivationStrict semantics ──────────────────────────

#[test]
fn derivation_and_derivation_strict_produce_same_shape() {
  // Both should produce the same standard fields. The arms are
  // deliberately structured identically.
  let attrs = r#"{ name = "x"; system = "x86_64-linux"; builder = "/bin/sh"; }"#;
  let d_out = json(&format!(
    "builtins.attrNames (builtins.derivation {})",
    attrs
  ));
  let ds_out = json(&format!(
    "builtins.attrNames (builtins.derivationStrict {})",
    attrs
  ));
  assert_eq!(d_out, ds_out);
}

// ── User-provided fields pass through ─────────────────────────

#[test]
fn derivation_user_fields_pass_through() {
  // User-provided custom fields should appear in the result.
  let out = json(
    r#"
    (builtins.derivation {
      name = "x"; system = "x"; builder = "x";
      myCustomField = 42;
      tags = [ "a" "b" ];
    }).myCustomField
  "#,
  );
  assert_eq!(out, "42");
}

#[test]
fn derivation_user_outpath_overrides_default() {
  // If the user provides an outPath, it takes precedence over
  // the auto-generated placeholder.
  let out = json(
    r#"
    (builtins.derivation {
      name = "x"; system = "x"; builder = "x";
      outPath = "/user/specified";
    }).outPath
  "#,
  );
  assert_eq!(out, r#""/user/specified""#);
}

#[test]
fn derivation_user_type_overrides_default() {
  // Similarly, if the user provides type, it takes precedence
  // (real Nix behaviour — derivation does not force `type`).
  let out = json(
    r#"
    (builtins.derivation {
      name = "x"; system = "x"; builder = "x";
      type = "custom";
    }).type
  "#,
  );
  assert_eq!(out, r#""custom""#);
}

// ── Default-name fallback ─────────────────────────────────────

#[test]
fn derivation_unnamed_uses_unnamed_fallback() {
  // No name → "unnamed" placeholder. Match derivationStrict.
  let out = json(
    r#"
    (builtins.derivation { system = "x"; builder = "x"; }).outPath
  "#,
  );
  assert!(out.contains("unnamed"), "got: {out}");
}

// ── Error path ────────────────────────────────────────────────

#[test]
fn derivation_non_attrset_arg_errors() {
  let e = err(r#"builtins.derivation 42"#);
  assert!(
    e.contains("derivation") && e.contains("attrset"),
    "got: {e}"
  );
}

#[test]
fn derivation_non_attrset_string_errors() {
  let e = err(r#"builtins.derivation "name""#);
  assert!(e.contains("derivation"), "got: {e}");
}

// ── String-context propagation ────────────────────────────────

#[test]
fn derivation_outpath_carries_output_dependency_context() {
  // The outPath has context `!out!<name>` so downstream string
  // operations can detect the output dependency. Slice #56
  // family — derivation values carry their build dependency
  // metadata.
  let ctx = json(
    r#"
    builtins.attrNames (
      builtins.getContext (
        (builtins.derivation { name = "x"; system = "x"; builder = "x"; }).outPath
      )
    )
  "#,
  );
  assert!(ctx.contains("!out!x"), "got: {ctx}");
}

#[test]
fn derivation_drvpath_carries_output_dependency_context() {
  let ctx = json(
    r#"
    builtins.attrNames (
      builtins.getContext (
        (builtins.derivation { name = "y"; system = "x"; builder = "x"; }).drvPath
      )
    )
  "#,
  );
  assert!(ctx.contains("!out!y"), "got: {ctx}");
}

// ── Round-trip via toJSON ─────────────────────────────────────

#[test]
fn derivation_serializes_via_to_json() {
  // toJSON of a derivation should serialize all its fields.
  let out = json(
    r#"
    builtins.toJSON (
      builtins.derivation { name = "n"; system = "s"; builder = "b"; }
    )
  "#,
  );
  // The result is a JSON-string-of-JSON; check standard fields appear.
  assert!(out.contains(r#"\"type\":\"derivation\""#), "got: {out}");
  assert!(out.contains(r#"\"name\":\"n\""#), "got: {out}");
}

// ── isDerivation NOT a builtin (sanity) ───────────────────────

#[test]
fn is_derivation_is_not_a_pnix_builtin() {
  // Real Nix has `lib.isDerivation`, not `builtins.isDerivation`.
  // Verify pnix matches: `builtins.isDerivation` errors as not
  // found. `.px` authors should use a manual `(x.type or null) ==
  // "derivation"` check, OR access via stdlib.
  let e = err(r#"builtins.isDerivation { type = "derivation"; }"#);
  assert!(e.contains("isDerivation"), "got: {e}");
  assert!(e.contains("not found"), "got: {e}");
}

// ── Common idiom: pattern-matching derivation type ───────────

#[test]
fn derivation_type_check_via_dot_access() {
  // The canonical isDerivation-style check: `x.type or null ==
  // "derivation"`. This is how nixpkgs `lib.isDerivation` is
  // defined.
  let out = json(
    r#"
    let
      d = builtins.derivation { name = "x"; system = "x"; builder = "x"; };
    in (d.type or null) == "derivation"
  "#,
  );
  assert_eq!(out, "true");
}

#[test]
fn non_derivation_attrset_type_check_returns_false() {
  let out = json(
    r#"
    let
      x = { a = 1; };
    in (x.type or null) == "derivation"
  "#,
  );
  assert_eq!(out, "false");
}
