//! Regression cover for duplicate-attribute detection in attrsets,
//! including the `rec { x = 1; inherit x; }` path that previously
//! silent-shadowed instead of erroring.
//!
//! Plain `{ a = 1; a = 2; }` and inherit-with-prior-assign now share
//! the same `attribute '...' already defined at this level` error.

use pnix_eval::eval_expr;

#[test]
fn plain_duplicate_assign_errors() {
  let r = eval_expr(r#"{ a = 1; a = 2; }"#);
  let msg = format!("{}", r.expect_err("expected duplicate error"));
  assert!(msg.contains("already defined"), "got: {msg}");
}

#[test]
fn rec_assign_then_inherit_errors() {
  // `rec { x = 1; inherit x; }` — the assign defines x, then the
  // inherit-no-from would try to pull outer x and shadow. Must error.
  let r = eval_expr(r#"let x = 99; in (rec { x = 1; inherit x; }).x"#);
  let msg = format!("{}", r.expect_err("expected duplicate error"));
  assert!(msg.contains("already defined"), "got: {msg}");
}

#[test]
fn inherit_then_assign_errors() {
  // The reverse order: inherit first, then assign the same name.
  let r = eval_expr(r#"let x = 99; in ({ inherit x; x = 1; }).x"#);
  let msg = format!("{}", r.expect_err("expected duplicate error"));
  assert!(msg.contains("already defined"), "got: {msg}");
}

#[test]
fn inherit_from_then_assign_errors() {
  // `{ inherit (s) a; a = 99; }` — same duplicate kind.
  let r = eval_expr(r#"let s = { a = 1; }; in ({ inherit (s) a; a = 99; }).a"#);
  let msg = format!("{}", r.expect_err("expected duplicate error"));
  assert!(msg.contains("already defined"), "got: {msg}");
}

#[test]
fn assign_then_inherit_from_errors() {
  // The reverse order: assign first, then inherit-from collides.
  let r = eval_expr(r#"let s = { a = 1; }; in ({ a = 99; inherit (s) a; }).a"#);
  let msg = format!("{}", r.expect_err("expected duplicate error"));
  assert!(msg.contains("already defined"), "got: {msg}");
}

#[test]
fn distinct_names_in_inherit_clause_pass() {
  // Sanity: multiple unrelated names in one inherit clause do not
  // self-collide.
  let v = eval_expr(r#"let s = { a = 1; b = 2; }; in ({ inherit (s) a b; }).b"#).unwrap();
  assert_eq!(v.to_json(), "2");
}

#[test]
fn rec_with_no_dup_passes() {
  // The non-duplicate case (the audit's earlier baseline): outer x
  // exists, rec inherits it lazily, sibling uses it. No duplicate.
  let v = eval_expr(r#"let x = 5; in (rec { inherit x; y = x + 1; }).y"#).unwrap();
  assert_eq!(v.to_json(), "6");
}
