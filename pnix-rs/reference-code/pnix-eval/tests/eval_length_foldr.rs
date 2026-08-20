//! Regression cover for `builtins.length` non-list/non-string guard,
//! `builtins.foldr` (newly added), and `builtins.foldl'` non-list
//! guard.
//!
//! 2026-05-04 audit findings:
//!   - **`builtins.length` silent-zero on non-list/non-string**:
//!     `length 42`, `length null`, `length true`, `length 1.5`,
//!     `length { a = 1; }` all returned `0` instead of erroring.
//!     A `.px` author guarding on `if builtins.length x == 0 then …
//!     else …` would silently take the empty branch when `x` is the
//!     wrong type. Now errors with `expected list or string, got
//!     <type>`.
//!   - **`builtins.foldr` was completely missing**: `attribute
//!     'foldr' not found`. Real nixpkgs uses both `foldl'` and
//!     `foldr`; fold-right is a normal Nix idiom (e.g. building a
//!     list with `foldr (x: acc: [x] ++ acc)`). Now registered, with
//!     same `(item, acc)` arg order Nix uses (note: opposite of
//!     `foldl'`'s `(acc, item)` order) and same lazy-init semantics.
//!   - **`builtins.foldl'` silent-init on non-list third arg**:
//!     `foldl' op init 42` previously returned `init` silently. Now
//!     errors. (Same shape as the foldr fix above.)

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── builtins.length non-list/non-string guard ─────────────────

#[test]
fn length_on_list_works() {
  assert_eq!(json("builtins.length [ 1 2 3 ]"), "3");
  assert_eq!(json("builtins.length []"), "0");
}

#[test]
fn length_on_string_byte_count() {
  // `length "abc"` is 3 bytes. Multibyte safe (Nix-byte semantics).
  assert_eq!(json(r#"builtins.length "abc""#), "3");
  // "héllo" — é is 2 bytes (UTF-8) → 6 bytes total.
  assert_eq!(json(r#"builtins.length "héllo""#), "6");
}

#[test]
fn length_on_int_errors() {
  let m = err_msg("builtins.length 42");
  assert!(m.contains("expected list or string"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn length_on_null_errors() {
  let m = err_msg("builtins.length null");
  assert!(m.contains("expected list or string"), "got: {m}");
  assert!(m.contains("got null"), "got: {m}");
}

#[test]
fn length_on_bool_errors() {
  let m = err_msg("builtins.length true");
  assert!(m.contains("expected list or string"), "got: {m}");
  assert!(m.contains("got bool"), "got: {m}");
}

#[test]
fn length_on_float_errors() {
  let m = err_msg("builtins.length 1.5");
  assert!(m.contains("expected list or string"), "got: {m}");
}

#[test]
fn length_on_attrset_errors() {
  // attrNames/attrValues are the right tools for set "size".
  let m = err_msg("builtins.length { a = 1; b = 2; }");
  assert!(m.contains("expected list or string"), "got: {m}");
  assert!(m.contains("got set"), "got: {m}");
}

// ── builtins.foldr (item-first arg order) ─────────────────────

#[test]
fn foldr_empty_list_returns_initial() {
  assert_eq!(json("builtins.foldr (a: b: a + b) 0 []"), "0");
}

#[test]
fn foldr_basic_sum_matches_foldl() {
  // For commutative `+`, foldr and foldl' agree.
  assert_eq!(json("builtins.foldr (a: b: a + b) 0 [ 1 2 3 ]"), "6");
}

#[test]
fn foldr_non_commutative_distinguishes_from_foldl() {
  // `foldr (-) 0 [1,2,3,4]` = 1 - (2 - (3 - (4 - 0))) = -2.
  // `foldl' (-) 0 [1,2,3,4]` = (((0 - 1) - 2) - 3) - 4 = -10.
  assert_eq!(json("builtins.foldr (a: b: a - b) 0 [ 1 2 3 4 ]"), "-2");
  assert_eq!(json("builtins.foldl' (a: b: a - b) 0 [ 1 2 3 4 ]"), "-10");
}

#[test]
fn foldr_string_concat() {
  assert_eq!(
    json(r#"builtins.foldr (a: b: a + b) "" [ "a" "b" "c" ]"#),
    r#""abc""#
  );
}

#[test]
fn foldr_builds_list_idiomatic() {
  // Classic foldr-builds-list idiom: cons each item onto the acc.
  // Order is preserved because right-fold visits from rightmost.
  assert_eq!(
    json("builtins.foldr (x: acc: [ x ] ++ acc) [] [ 1 2 3 ]"),
    "[1,2,3]"
  );
}

#[test]
fn foldr_lazy_initial_when_list_empty() {
  // Like foldl': the initial accumulator is not forced when the
  // list is empty. Production code that conditionally throws as
  // an "unreachable default" relies on this.
  assert_eq!(
    json(r#"let r = builtins.foldr (a: b: a + b) (throw "x") []; in 42"#),
    "42"
  );
}

#[test]
fn foldr_throw_in_step_propagates() {
  let m = err_msg(r#"builtins.foldr (a: b: throw "step") 0 [ 1 ]"#);
  assert!(m.contains("step"), "got: {m}");
}

#[test]
fn foldr_non_list_third_arg_errors() {
  let m = err_msg("builtins.foldr (a: b: a) 0 42");
  assert!(m.contains("third arg must be list"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn foldr_string_as_list_errors() {
  // Was NOT a bug before (foldr didn't exist), pinning the new
  // contract: foldr only accepts a list.
  let m = err_msg(r#"builtins.foldr (a: b: a) 0 "abc""#);
  assert!(m.contains("third arg must be list"), "got: {m}");
}

// ── builtins.foldl' non-list guard (was silent-init) ──────────

#[test]
fn foldl_non_list_third_arg_errors() {
  // Was: silently returned init. Now: type error.
  let m = err_msg("builtins.foldl' (a: b: a) 0 42");
  assert!(m.contains("third arg must be list"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn foldl_lazy_initial_still_works_after_guard() {
  // Make sure the lazy-init path still works for the empty-list case.
  assert_eq!(
    json(r#"let r = builtins.foldl' (a: b: a + b) (throw "x") []; in 42"#),
    "42"
  );
}

// ── foldr arg-order pin (item-first, opposite of foldl') ──────

#[test]
fn foldr_arg_order_is_item_first() {
  // foldr passes the LIST element as the first arg to the step
  // function; the accumulator is second.
  // `foldr (item: acc: { i = item; a = acc; }) "Z" [ "X" ]`
  //  = step "X" "Z" = { i = "X"; a = "Z"; }
  assert_eq!(
    json(r#"builtins.foldr (item: acc: { i = item; a = acc; }) "Z" [ "X" ]"#),
    r#"{"a":"Z","i":"X"}"#
  );
}

#[test]
fn foldl_arg_order_is_acc_first() {
  // foldl' passes the ACC as the first arg to the step function;
  // the item is second. (Inverse of foldr.)
  assert_eq!(
    json(r#"builtins.foldl' (acc: item: { a = acc; i = item; }) "Z" [ "X" ]"#),
    r#"{"a":"Z","i":"X"}"#
  );
}
