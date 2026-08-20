//! Regression cover for `builtins.toString` (and the `toString` global
//! alias). Coercion follows the Nix manual:
//!
//!   - string             → unchanged
//!   - path               → display form (NOT a fake store path —
//!     the fake-store-path shape is for `${./p}` interpolation only)
//!   - integer / float    → text representation
//!   - boolean true       → "1"; boolean false → "" (yes, surprising)
//!   - null               → ""
//!   - list               → element-wise toString joined with spaces
//!   - attrset __toString → invoke `(self: ...)` and re-coerce
//!   - attrset outPath    → re-coerce the outPath value
//!   - attrset without either → error
//!   - lambda / function  → error
//!
//! Previous implementation routed through `format!("{}", value)`,
//! which used the `Display` impl meant for debug-style JSON-shape
//! output: `toString "hello"` returned `"\"hello\""`, `toString
//! [1 2 3]` returned `"[1,2,3]"`, etc. Fixed by adding a dedicated
//! `coerce_to_string_for_to_string` helper.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

#[test]
fn ts_string_unchanged() {
  assert_eq!(json(r#"toString "hello""#), r#""hello""#);
  assert_eq!(
    json(r#"toString "with \"quotes\"""#),
    r#""with \"quotes\"""#
  );
}

#[test]
fn ts_int() {
  assert_eq!(json(r#"toString 42"#), r#""42""#);
  assert_eq!(json(r#"toString 0"#), r#""0""#);
  assert_eq!(json(r#"toString (-7)"#), r#""-7""#);
}

#[test]
fn ts_float() {
  // Real Nix prints `3.5` as `"3.500000"` in some versions; pnix
  // uses Rust's default Display which gives `"3.5"`. Both are valid
  // numeric round-trips — pin pnix's choice here so future codegen
  // can rely on it.
  assert_eq!(json(r#"toString 3.5"#), r#""3.5""#);
}

#[test]
fn ts_bool_surprising_but_correct() {
  // The Nix-manual shape that surprises everyone.
  assert_eq!(json(r#"toString true"#), r#""1""#);
  assert_eq!(json(r#"toString false"#), r#""""#);
}

#[test]
fn ts_null() {
  assert_eq!(json(r#"toString null"#), r#""""#);
}

#[test]
fn ts_list_space_joined() {
  assert_eq!(json(r#"toString [ 1 2 3 ]"#), r#""1 2 3""#);
}

#[test]
fn ts_list_mixed_types() {
  // Each element is coerced with the same toString rules — booleans
  // collapse, etc.
  assert_eq!(json(r#"toString [ 1 "x" true ]"#), r#""1 x 1""#);
}

#[test]
fn ts_path_is_display_not_fake_store() {
  // `toString ./foo` prints the path as-is. This is distinct from
  // `${./foo}` interpolation which produces a `/nix/store/<hash>-…`
  // shape.
  let v = eval_expr("toString /a/b/c").unwrap();
  assert_eq!(v.to_json(), r#""/a/b/c""#);
}

#[test]
fn ts_attrset_to_string_invokes_self() {
  assert_eq!(
    json(r#"toString { __toString = self: "hi-" + self.label; label = "x"; }"#),
    r#""hi-x""#
  );
}

#[test]
fn ts_attrset_outpath() {
  assert_eq!(
    json(r#"toString { outPath = "/nix/store/x"; }"#),
    r#""/nix/store/x""#
  );
}

#[test]
fn ts_to_string_takes_priority_over_outpath() {
  // If both __toString and outPath are present, __toString wins.
  assert_eq!(
    json(r#"toString { __toString = _: "from-toString"; outPath = "from-outPath"; }"#),
    r#""from-toString""#
  );
}

#[test]
fn ts_attrset_neither_errors() {
  let r = eval_expr(r#"toString { a = 1; }"#);
  let msg = format!("{}", r.expect_err("expected coercion error"));
  assert!(
    msg.contains("__toString") || msg.contains("outPath"),
    "got: {msg}"
  );
}

#[test]
fn ts_lambda_errors() {
  let r = eval_expr(r#"toString (x: x)"#);
  assert!(r.is_err());
}

#[test]
fn ts_via_global_alias() {
  // The bare `toString` global alias must call the same builtin —
  // tested separately because the alias is registered in
  // `global_builtin_alias` rather than via `builtins.foo` path.
  assert_eq!(json(r#"toString 42"#), r#""42""#);
  assert_eq!(json(r#"builtins.toString 42"#), r#""42""#);
}
