//! Regression cover for `force_value`'s self-cycle guard.
//!
//! `RecursiveBindings::lookup` already catches per-name cycles in
//! `let` / `rec` (e.g. `let x = x; in x` → "recursive attrset
//! cycle"), but indirect cycles that go through a thunk's value
//! produce a self-referential thunk whose `cache` re-enters
//! `force_value` before populating itself — without the guard, that
//! stack-overflows.
//!
//! Example previously overflowing on a 2 MB test thread:
//!   `let s = { x = s.x; }; in s.x`
//!
//! Fix: `force_value` keeps a thread-local
//! `FORCING_THUNKS: Vec<Rc<RefCell<Option<Value>>>>` and refuses to
//! recurse into a thunk whose cache pointer is already on the stack.
//! Returns a clear `infinite recursion encountered` error instead.

use pnix_eval::eval_expr;

#[test]
fn indirect_self_cycle_errors_not_overflow() {
  let r = eval_expr("let s = { x = s.x; }; in s.x");
  let err = r.expect_err("expected infinite recursion error");
  let msg = format!("{err}");
  assert!(
    msg.contains("infinite recursion"),
    "expected `infinite recursion` in error, got: {msg}"
  );
}

#[test]
fn deep_attrset_cycle_errors() {
  // `(rec { a = b; b = a; }).a` — caught by RecursiveBindings as
  // "recursive attrset cycle"; either error message is fine, the
  // point is no overflow.
  let r = eval_expr("(rec { a = b; b = a; }).a");
  assert!(r.is_err());
}

#[test]
fn rec_self_cycle_errors() {
  // Direct self in rec: `rec { x = x; }`.
  let r = eval_expr("(rec { x = x; }).x");
  assert!(r.is_err());
}

#[test]
fn unforced_self_reference_passes() {
  // `let x = x; in 1` — x is never forced, so no cycle to detect.
  let v = eval_expr("let x = x; in 1").unwrap();
  assert_eq!(v.to_json(), "1");
}

#[test]
fn legit_recursion_via_lambda_works() {
  // Bounded recursion through a lambda must not be flagged as a
  // cycle. Each `f (n - 1)` builds a fresh thunk with a distinct
  // cache, so `Rc::ptr_eq` returns false for previously-popped
  // frames. Depth fits within default 2 MB thread stack.
  let v = eval_expr("let f = n: if n == 0 then 0 else f (n - 1) + 1; in f 30").unwrap();
  assert_eq!(v.to_json(), "30");
}

#[test]
fn cycle_guard_pops_on_success() {
  // After a successful force, the same thunk path can be reused.
  // `let x = 1; in x + x` evaluates `x` twice without a false
  // positive.
  let v = eval_expr("let x = 1; in x + x").unwrap();
  assert_eq!(v.to_json(), "2");
}
