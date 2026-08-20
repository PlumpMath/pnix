//! Regression cover for `let`-binding duplicate-name and
//! lambda-formals duplicate-name silent-pass shapes.
//!
//! 2026-05-05 audit findings (slice #39):
//!   - **`let x = 1; x = 2; in x`** silently returned `2` because
//!     `recursive_let_bindings` used `BTreeMap::insert`, which
//!     overwrites duplicates. The same family of silent
//!     overwrites covered:
//!     - `let x = 1; y = 2; x = 3; in x + y` → `5` (uses `x = 3`)
//!     - `let x = 1; x = 2; x = 3; in x` → `3`
//!     - `let inherit ({a=99;}) a; a = 1; in a` → `99` (inherit
//!       wins over later explicit binding because of binding
//!       order at the eval site, not because of any specified
//!       priority)
//!     - `let inherit ({a=1;}) a; inherit ({a=2;}) a; in a` → `2`
//!   - **`({ a, a }: a) { a = 1; }`** silently returned `1`
//!     because pass-1 of `bind_attrset_pattern` binds the field
//!     once and the duplicate field is a no-op rebind to the
//!     same value. The `(args@{ args }: args)` shape (where the
//!     at-binding name shadows / is shadowed by a field of the
//!     same name) had the same silent shape.
//!
//! Real Nix rejects all of these:
//!   - "attribute '<name>' already defined" for `let` bodies
//!     (the same error attrset literal `{ a = 1; a = 2; }` has
//!     used since slice #5)
//!   - "duplicate formal function argument '<name>'" for lambda
//!     formals.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` — new
//!   `check_let_no_duplicates(bindings)` helper called at the
//!   top of both `PnixExpr::Let` arms (the tail-loop entry and
//!   the `eval_inner` entry). Walks `Ident` / `AttrSet` /
//!   `AttrSetWithBind` / `Inherit` shapes; `List` patterns are
//!   not currently walked (rare in `let`).
//! - `crates/pnix-eval/src/interpret.rs` `bind_attrset_pattern`
//!   — pre-pass `HashSet` walk over `fields[*].name` (and the
//!   at-binding `bind_name` if present) reporting the first
//!   duplicate before the argument value is forced.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── let-binding duplicates ─────────────────────────────────────

#[test]
fn let_clean_binding_works() {
  // Sanity: no duplicates → original behaviour preserved.
  assert_eq!(json("let x = 1; y = 2; in x + y"), "3");
}

#[test]
fn let_simple_dup_errors() {
  let m = err_msg("let x = 1; x = 2; in x");
  assert!(m.contains("let:"), "got: {m}");
  assert!(m.contains("'x'"), "got: {m}");
  assert!(m.contains("more than once"), "got: {m}");
}

#[test]
fn let_dup_pinning_offender() {
  // Even when the duplicated name isn't the only binding, the
  // error names the offender, not just "duplicate".
  let m = err_msg("let x = 1; y = 2; x = 3; in x + y");
  assert!(m.contains("'x'"), "got: {m}");
}

#[test]
fn let_triple_dup_errors() {
  let m = err_msg("let x = 1; x = 2; x = 3; in x");
  assert!(m.contains("'x'"), "got: {m}");
  assert!(m.contains("more than once"), "got: {m}");
}

#[test]
fn let_inherit_then_binding_dup_errors() {
  // The most production-relevant shape: someone refactors a
  // chunk of bindings into an `inherit (...)` and forgets to
  // remove the original explicit `name = ...;` form. Was
  // silent (the binding orderings produced different "wins";
  // none erred).
  let m = err_msg("let inherit ({a=99;}) a; a = 1; in a");
  assert!(m.contains("'a'"), "got: {m}");
  assert!(m.contains("more than once"), "got: {m}");
}

#[test]
fn let_inherit_twice_dup_errors() {
  let m = err_msg("let inherit ({a=1;}) a; inherit ({a=2;}) a; in a");
  assert!(m.contains("'a'"), "got: {m}");
}

#[test]
fn let_nested_shadow_does_not_trigger() {
  // Inner `let` is a separate scope — shadowing across nested
  // `let`s is not a duplicate. The duplicate check is per-form,
  // not transitive.
  assert_eq!(json("let x = 1; in let x = 2; in x"), "2");
}

#[test]
fn let_lambda_pattern_shadow_does_not_trigger() {
  // The lambda's `x` parameter and the `let x = ...; in x`
  // inside the body are different scopes. The let inside the
  // lambda body shadows the parameter, not duplicates it.
  assert_eq!(json("let f = x: let x = 99; in x; in f 1"), "99");
}

// ── lambda formals duplicates ──────────────────────────────────

#[test]
fn lambda_formal_clean_works() {
  // Sanity: no duplicates → original behaviour preserved.
  assert_eq!(json("({ a, b }: a + b) { a = 1; b = 2; }"), "3");
}

#[test]
fn lambda_formal_dup_errors() {
  let m = err_msg("({ a, a }: a) { a = 1; }");
  assert!(m.contains("duplicate formal"), "got: {m}");
  assert!(m.contains("'a'"), "got: {m}");
}

#[test]
fn lambda_formal_dup_with_default_errors() {
  // Default on the second occurrence doesn't disambiguate.
  let m = err_msg("({ a, a ? 1 }: a) { a = 9; }");
  assert!(m.contains("duplicate formal"), "got: {m}");
}

#[test]
fn lambda_at_binding_field_dup_errors() {
  // `args@{ args }` — the at-binding name and a field name
  // collide. Was: silent (one shadowed the other depending on
  // pass order). Now: typed error pinning the offender.
  let m = err_msg("(args@{ args }: args) { args = 1; }");
  assert!(m.contains("duplicate formal"), "got: {m}");
  assert!(m.contains("'args'"), "got: {m}");
}

#[test]
fn lambda_at_binding_clean_works() {
  // Sanity: at-binding without collision works.
  assert_eq!(
    json("(args@{ a, b }: args.a + args.b) { a = 1; b = 2; }"),
    "3"
  );
}

#[test]
fn lambda_formal_dup_fires_before_argument_evaluated() {
  // The duplicate check runs before the argument value is
  // touched, so a `throw` in the argument never fires when
  // the pattern itself is malformed. This pins the order of
  // checks: pattern validity first, then argument evaluation.
  let m = err_msg(r#"({ a, a }: a) (throw "should-not-fire")"#);
  assert!(m.contains("duplicate formal"), "got: {m}");
  assert!(!m.contains("should-not-fire"), "got: {m}");
}

#[test]
fn rec_attrset_dup_still_caught_by_attrset_path() {
  // `rec` attrsets reuse the existing attrset literal duplicate
  // check (slice #5). Confirm the error message family is the
  // attrset shape, not the let shape — they go through different
  // code paths.
  let m = err_msg("rec { x = 1; x = 2; }");
  assert!(m.contains("already defined"), "got: {m}");
}

#[test]
fn attrset_literal_dup_unchanged() {
  // The attrset-literal path is untouched by this slice.
  let m = err_msg("{ a = 1; a = 2; }");
  assert!(m.contains("already defined"), "got: {m}");
}
