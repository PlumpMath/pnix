//! Regression cover for `builtins.appendContext` value-shape
//! validation.
//!
//! 2026-05-05 audit findings (slice #56):
//!
//!   `builtins.appendContext "x" { "/a" = ANY-VALUE; }` silently
//!   succeeded for any value type because the pre-fix
//!   implementation iterated `extra.keys()` and inserted each key
//!   into the context set, ignoring the value entirely. So:
//!
//!     - `{ "/a" = "wrong-shape"; }` (string value) silently
//!       accepted
//!     - `{ "/a" = 42; }` (int value) silently accepted
//!     - `{ "/a" = null; }` (null value) silently accepted
//!     - `{ "/a" = { outputs = "wrong"; }; }` (wrong outputs
//!       type) silently accepted
//!     - `{ "/a" = { path = 1; }; }` (wrong path type) silently
//!       accepted
//!
//!   Production-relevant: code that constructs a context attrset
//!   programmatically (often via a `mapAttrs`-style transform that
//!   produced wrong types in some branches) would silently
//!   dispatch into the context set with a malformed shape, and
//!   downstream consumers (real Nix's `getContext` reading the
//!   shape, or a derivation realization step) would only surface
//!   the shape error long after the bad value entered the system.
//!
//!   Real Nix validates the value shape: each value must be an
//!   attrset matching `{ path = bool; outputs = [string];
//!   allOutputs = bool; }` (all fields optional, but each present
//!   field must have the correct type). pnix now validates this
//!   shape as well.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"appendContext"` arm — iterates `extra.iter()` (was:
//!   `extra.keys()`), validates each value is an attrset, and
//!   validates known fields (`path`/`outputs`/`allOutputs`) have
//!   the correct types. Unknown fields are allowed for
//!   forward-compat with future Nix shape additions.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── value shape: must be attrset ───────────────────────────────

#[test]
fn append_context_value_string_errors_loud() {
  let e = err(r#"builtins.appendContext "x" { "/a" = "wrong-shape"; }"#);
  assert!(e.contains("appendContext"), "got: {e}");
  assert!(e.contains("/a"), "got: {e}");
  assert!(e.contains("must be an attrset"), "got: {e}");
  assert!(e.contains("string"), "got: {e}");
}

#[test]
fn append_context_value_int_errors_loud() {
  let e = err(r#"builtins.appendContext "x" { "/a" = 42; }"#);
  assert!(e.contains("appendContext"), "got: {e}");
  assert!(e.contains("/a"), "got: {e}");
  assert!(e.contains("must be an attrset"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn append_context_value_null_errors_loud() {
  let e = err(r#"builtins.appendContext "x" { "/a" = null; }"#);
  assert!(e.contains("appendContext"), "got: {e}");
  assert!(e.contains("/a"), "got: {e}");
  assert!(e.contains("must be an attrset"), "got: {e}");
  assert!(e.contains("null"), "got: {e}");
}

#[test]
fn append_context_value_list_errors_loud() {
  let e = err(r#"builtins.appendContext "x" { "/a" = [ ]; }"#);
  assert!(e.contains("appendContext"), "got: {e}");
  assert!(e.contains("/a"), "got: {e}");
  assert!(e.contains("must be an attrset"), "got: {e}");
  assert!(e.contains("list"), "got: {e}");
}

// ── value field types: outputs ─────────────────────────────────

#[test]
fn append_context_outputs_non_list_errors() {
  let e = err(r#"builtins.appendContext "x" { "/a" = { outputs = "wrong"; }; }"#);
  assert!(e.contains("appendContext"), "got: {e}");
  assert!(e.contains("/a"), "got: {e}");
  assert!(e.contains("outputs"), "got: {e}");
  assert!(e.contains("must be list of strings"), "got: {e}");
  assert!(e.contains("string"), "got: {e}");
}

#[test]
fn append_context_outputs_list_with_int_element_errors() {
  let e = err(r#"builtins.appendContext "x" { "/a" = { outputs = [ "out" 42 ]; }; }"#);
  assert!(e.contains("appendContext"), "got: {e}");
  assert!(e.contains("outputs"), "got: {e}");
  assert!(e.contains("element at index 1"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn append_context_outputs_empty_list_works() {
  let v = json(
    r#"builtins.toJSON (builtins.getContext (builtins.appendContext "x" { "/a" = { outputs = [ ]; }; }))"#,
  );
  assert!(v.contains("/a"), "got: {v}");
}

#[test]
fn append_context_outputs_string_list_works() {
  let v = json(
    r#"builtins.toJSON (builtins.getContext (builtins.appendContext "x" { "/a" = { outputs = [ "out" "dev" ]; }; }))"#,
  );
  assert!(v.contains("/a"), "got: {v}");
}

// ── value field types: path ────────────────────────────────────

#[test]
fn append_context_path_non_bool_errors() {
  let e = err(r#"builtins.appendContext "x" { "/a" = { path = 1; }; }"#);
  assert!(e.contains("appendContext"), "got: {e}");
  assert!(e.contains("path"), "got: {e}");
  assert!(e.contains("must be bool"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn append_context_path_string_errors() {
  let e = err(r#"builtins.appendContext "x" { "/a" = { path = "true"; }; }"#);
  assert!(e.contains("appendContext"), "got: {e}");
  assert!(e.contains("path"), "got: {e}");
  assert!(e.contains("must be bool"), "got: {e}");
}

#[test]
fn append_context_path_bool_works() {
  let v = json(
    r#"builtins.toJSON (builtins.getContext (builtins.appendContext "x" { "/a" = { path = true; }; }))"#,
  );
  assert!(v.contains("/a"), "got: {v}");
}

// ── value field types: allOutputs ──────────────────────────────

#[test]
fn append_context_all_outputs_non_bool_errors() {
  let e = err(r#"builtins.appendContext "x" { "/a" = { allOutputs = 1; }; }"#);
  assert!(e.contains("appendContext"), "got: {e}");
  assert!(e.contains("allOutputs"), "got: {e}");
  assert!(e.contains("must be bool"), "got: {e}");
}

#[test]
fn append_context_all_outputs_bool_works() {
  let v = json(
    r#"builtins.toJSON (builtins.getContext (builtins.appendContext "x" { "/a" = { allOutputs = false; }; }))"#,
  );
  assert!(v.contains("/a"), "got: {v}");
}

// ── canonical shape: all fields together ───────────────────────

#[test]
fn append_context_full_shape_works() {
  let v = json(
    r#"builtins.toJSON (builtins.getContext (builtins.appendContext "x" {
      "/a" = { path = false; outputs = [ "out" ]; allOutputs = false; };
    }))"#,
  );
  assert!(v.contains("/a"), "got: {v}");
}

#[test]
fn append_context_does_not_force_unknown_fields() {
  let v = json(
    r#"
    builtins.hasContext (
      builtins.appendContext "x" {
        "/a" = {
          path = true;
          futureField = throw "appendContext unknown field payload";
        };
      }
    )
    "#,
  );
  assert_eq!(v, "true");
}

#[test]
fn append_context_still_forces_known_field_values() {
  let e = err(
    r#"
    builtins.appendContext "x" {
      "/a" = {
        path = throw "appendContext known field payload";
      };
    }
    "#,
  );
  assert!(e.contains("appendContext known field payload"), "got: {e}");
}

// ── unknown fields: forward-compat ─────────────────────────────

#[test]
fn append_context_unknown_field_allowed() {
  // Forward-compat: future-Nix may add new fields to the
  // shape. Unknown fields are allowed; only the documented
  // fields are type-checked. This avoids breakage when running
  // on `.px` produced for a newer Nix.
  let v = json(
    r#"builtins.toJSON (builtins.getContext (builtins.appendContext "x" {
      "/a" = { path = true; futureField = "anything"; };
    }))"#,
  );
  assert!(v.contains("/a"), "got: {v}");
}

// ── existing argument-type guards still work ───────────────────

#[test]
fn append_context_non_string_first_arg_errors() {
  let e = err(r#"builtins.appendContext 42 { }"#);
  assert!(e.contains("appendContext"), "got: {e}");
  assert!(e.contains("first arg"), "got: {e}");
}

#[test]
fn append_context_non_attrset_second_arg_errors() {
  let e = err(r#"builtins.appendContext "x" 42"#);
  assert!(e.contains("appendContext"), "got: {e}");
  assert!(e.contains("second arg"), "got: {e}");
}

#[test]
fn append_context_empty_attrset_works() {
  // Sanity: empty attrset is a no-op (preserves text and
  // existing context).
  let v = json(r#"builtins.appendContext "x" { }"#);
  assert_eq!(v, r#""x""#);
}
