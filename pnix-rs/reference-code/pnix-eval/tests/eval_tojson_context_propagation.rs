//! Regression cover for `builtins.toJSON` context propagation
//! — same metadata-loss family as slice #73 emit functions.
//!
//! 2026-05-05 audit findings (slice #74):
//!
//!   `builtins.toJSON` returned plain `Value::String` with
//!   empty context, silently dropping any context-bearing
//!   strings buried in the input value tree:
//!     - `toJSON "x${./p}"` → empty context
//!     - `toJSON { a = "x${./p}"; }` → empty context
//!     - `toJSON [ "x${./p1}" "y${./p2}" ]` → empty context
//!     - `toJSON ./p` (path-typed input) → empty context
//!     - Nested attrset / list with context-bearing leaves —
//!       same loss
//!
//!   Production-relevant: `.px` authors who serialize
//!   derivation references via `toJSON` (a common pattern for
//!   producing JSON manifests / config files containing store
//!   paths) silently lost build-time-dependency markers at
//!   the JSON serialization boundary. Same shape as slice #55
//!   `toFile`, slice #73 emit family. The serialized JSON
//!   text VISIBLY contains the resolved store paths, but
//!   downstream consumers (subsequent string operations,
//!   hashing of the JSON output, derivation realization that
//!   reads the JSON content) saw no dependency.
//!
//!   Fix: reuse the slice #73 `collect_value_contexts` helper.
//!   Walk the input tree, union all string contexts, wrap
//!   `toJSON` result with `Value::string_with_context(text,
//!   collected_context)`.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"toJSON"` arm — wraps result with
//!   `Value::string_with_context` using
//!   `collect_value_contexts(&args[0])`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

// ── basic context propagation ────────────────────────────────

#[test]
fn to_json_string_context_propagates() {
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (builtins.toJSON "x${./p}"))
  "#,
  );
  assert!(v.contains("./p"), "got: {v}");
}

#[test]
fn to_json_attrset_with_ctx_value_propagates() {
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (builtins.toJSON { a = "x${./p}"; }))
  "#,
  );
  assert!(v.contains("./p"), "got: {v}");
}

#[test]
fn to_json_list_with_ctx_elements_propagates() {
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (builtins.toJSON [ "x${./p1}" "y${./p2}" ]))
  "#,
  );
  assert!(v.contains("./p1"), "got: {v}");
  assert!(v.contains("./p2"), "got: {v}");
}

#[test]
fn to_json_nested_attrset_with_ctx_propagates() {
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (
      builtins.toJSON {
        a = "x${./p1}";
        b = { c = "y${./p2}"; };
      }
    ))
  "#,
  );
  assert!(v.contains("./p1"), "got: {v}");
  assert!(v.contains("./p2"), "got: {v}");
}

#[test]
fn to_json_path_value_propagates_path() {
  // Path-typed input contributes its display form to context.
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (builtins.toJSON ./p))
  "#,
  );
  // Path display form `./p` should appear in context.
  assert!(v.contains("./p"), "got: {v}");
}

#[test]
fn to_json_no_context_input_no_context_output() {
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (builtins.toJSON [ 1 2 3 ]))
  "#,
  );
  assert_eq!(v, r#""{}""#);
}

// ── result type unchanged ─────────────────────────────────────

#[test]
fn to_json_returns_string_typed() {
  let v = json(
    r#"
    builtins.typeOf (builtins.toJSON [ "x${./p}" ])
  "#,
  );
  assert_eq!(v, r#""string""#);
}

#[test]
fn to_json_text_content_unchanged() {
  // Sanity: the actual JSON text is the same as before.
  let v = json(r#"builtins.toJSON [ 1 2 3 ]"#);
  assert_eq!(v, r#""[1,2,3]""#);
}

// ── chained operations preserve context ──────────────────────

#[test]
fn to_json_chained_with_concat_keeps_context() {
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (
      "prefix:" + (builtins.toJSON [ "x${./p}" ])
    ))
  "#,
  );
  assert!(v.contains("./p"), "got: {v}");
}

#[test]
fn to_json_then_hash_string_propagates() {
  // toJSON output → hashString. The hash result's context
  // should include the original path.
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (
      builtins.hashString "sha256" (builtins.toJSON [ "x${./p}" ])
    ))
  "#,
  );
  assert!(v.contains("./p"), "got: {v}");
}

// ── pre-existing inf/nan rejection unchanged ─────────────────

#[test]
fn to_json_inf_still_errors() {
  // Slice #35: toJSON of inf errors. Sanity check.
  let e = eval_expr(r#"builtins.toJSON (1.0e200 * 1.0e200)"#)
    .err()
    .unwrap()
    .to_string();
  assert!(e.contains("toJSON"), "got: {e}");
}

// ── parity with emit family (slice #73) ──────────────────────

#[test]
fn xml_emit_context_still_propagates() {
  // Sanity: slice #73 xmlEmit context propagation still
  // works.
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (
      builtins.xmlEmit { kind = "text"; value = "x${./p}"; }
    ))
  "#,
  );
  assert!(v.contains("./p"), "got: {v}");
}
