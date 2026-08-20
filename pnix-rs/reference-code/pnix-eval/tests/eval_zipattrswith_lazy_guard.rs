//! Regression cover for `builtins.zipAttrsWith` value-laziness
//! — extends slice #62's list-producing-builtin laziness
//! family to the attrset-producing `zipAttrsWith`.
//!
//! 2026-05-05 audit findings (slice #63):
//!
//!   `builtins.zipAttrsWith (k: vs: throw "x") [...]` errored
//!   immediately at construction time. The pre-fix impl applied
//!   the function eagerly via two `apply_value` calls, so any
//!   throw in the function body fired before the result was
//!   accessed. Real Nix's `zipAttrsWith` is lazy in the
//!   resulting attrset values — each output value is a thunk
//!   for `f key valueList`, only forced when the field is
//!   accessed.
//!
//!   Pre-fix bugs:
//!     - `length (attrNames (zipAttrsWith throw [...]))`
//!       errored instead of returning the number of unique
//!       keys.
//!     - `r ? a` (which only checks key presence, not value)
//!       errored.
//!     - `let r = zipAttrsWith ... in if needed then r else
//!       fallback` ALWAYS forced every output value even if r
//!       was never used.
//!
//!   Production-relevant: extends the slice #62 contract to the
//!   attrset family. After this slice, both `mapAttrs` (was
//!   already lazy) and `zipAttrsWith` produce attrsets whose
//!   values are thunks. Same shape as `map` / `genList` (slice
//!   #62) and `mapAttrs` (predates audit) — every list-/attrset-
//!   producing builtin in pnix is now lazy in the result
//!   elements / values.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"zipAttrsWith"` arm — replaced eager
//!   `apply_value(partial, Value::List(values))` with
//!   `deferred_apply2(func, Value::String(key), Value::List(values))`.
//!   Each output value is now a thunk for `f key valueList`.
//!   Also tightened error messages on type-guard failures
//!   (now name the actual offending type).

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── zipAttrsWith: lazy in result values ───────────────────────

#[test]
fn zip_attrs_with_length_does_not_force_function() {
  // Pre-fix: errored. Now: returns number of unique keys.
  assert_eq!(
    json(
      r#"
      builtins.length (
        builtins.attrNames (
          builtins.zipAttrsWith (k: vs: throw "x") [ { a = 1; } { a = 2; b = 3; } ]
        )
      )
    "#
    ),
    "2"
  );
}

#[test]
fn zip_attrs_with_hasattr_does_not_force_function() {
  // r ? a only checks key presence; should not force value.
  assert_eq!(
    json(
      r#"
      let r = builtins.zipAttrsWith
        (k: vs: throw "x")
        [ { a = 1; } { a = 2; } ];
      in r ? a
    "#
    ),
    "true"
  );
}

#[test]
fn zip_attrs_with_force_throws_when_accessed() {
  // Sanity: forcing the throwing value does fire.
  let e = err(
    r#"
    let r = builtins.zipAttrsWith
      (k: vs: throw "x")
      [ { a = 1; } { a = 2; } ];
    in r.a
  "#,
  );
  assert!(e.contains("x"), "got: {e}");
}

#[test]
fn zip_attrs_with_partial_force_only_accessed_key() {
  // Force one key, leave another lazy.
  assert_eq!(
    json(
      r#"
      let r = builtins.zipAttrsWith
        (k: vs:
          if k == "a"
          then builtins.head vs
          else throw "x")
        [ { a = 1; b = 99; } ];
      in r.a
    "#
    ),
    "1"
  );
}

#[test]
fn zip_attrs_with_unused_in_let_does_not_throw() {
  assert_eq!(
    json(
      r#"
      let r = builtins.zipAttrsWith
        (k: vs: throw "x")
        [ { a = 1; } { b = 2; } ];
      in 99
    "#
    ),
    "99"
  );
}

#[test]
fn zip_attrs_with_happy_path_unchanged() {
  // Sanity.
  assert_eq!(
    json(
      r#"
      builtins.zipAttrsWith (k: vs: vs) [ { a = 1; } { a = 2; b = 3; } ]
    "#
    ),
    r#"{"a":[1,2],"b":[3]}"#
  );
}

#[test]
fn zip_attrs_with_function_receives_correct_args() {
  // Function gets (k, vs) — verify when forced.
  assert_eq!(
    json(
      r#"
      let r = builtins.zipAttrsWith
        (k: vs: { key = k; values = vs; })
        [ { a = 1; } { a = 2; } ];
      in r.a.key
    "#
    ),
    r#""a""#
  );
}

#[test]
fn zip_attrs_with_function_receives_value_list() {
  assert_eq!(
    json(
      r#"
      let r = builtins.zipAttrsWith
        (k: vs: vs)
        [ { a = 1; } { a = 2; } { a = 3; } ];
      in builtins.length r.a
    "#
    ),
    "3"
  );
}

// ── existing argument-type guards still work ──────────────────

#[test]
fn zip_attrs_with_non_list_second_arg_errors() {
  let e = err(r#"builtins.zipAttrsWith (k: vs: vs) 42"#);
  assert!(e.contains("zipAttrsWith"), "got: {e}");
  assert!(e.contains("must be list"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn zip_attrs_with_non_attrset_list_element_errors() {
  let e = err(r#"builtins.zipAttrsWith (k: vs: vs) [ { a = 1; } 42 ]"#);
  assert!(e.contains("zipAttrsWith"), "got: {e}");
  assert!(e.contains("attrset"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

// ── parity with mapAttrs / map / genList laziness ─────────────

#[test]
fn map_attrs_remains_lazy_unchanged() {
  // Sanity: slice #62 + earlier slices.
  assert_eq!(
    json(
      r#"
      builtins.length (
        builtins.attrNames (
          builtins.mapAttrs (k: v: throw "x") { a = 1; b = 2; }
        )
      )
    "#
    ),
    "2"
  );
}

#[test]
fn map_remains_lazy_unchanged() {
  // Sanity: slice #62.
  assert_eq!(
    json(r#"builtins.length (builtins.map (x: throw "x") [ 1 2 3 ])"#),
    "3"
  );
}

// ═══════════════════════════════════════════════════════════════
// slice #75 — attrValues / keys / values / mapKeys / mapValues
// laziness — extends slice #62 / #63 list-builtin lazy contract
// to attrset-list extractors. Pre-fix: these names were missing
// from `apply_builtin`'s `lazy_in_elements` set, so the boundary
// `deep_force`d every input attrset value before passing to the
// impl. After fix: shallow-force only.
//
// 2026-05-06: merged from `eval_attrvalues_keys_lazy.rs`. The
// per-slice file was a duplicate of the lazy-boundary owner —
// `eval_zipattrswith_lazy_guard.rs` (slice #63) is the canonical
// owner since slice #75 extends the same `lazy_in_elements`
// surface.
// ═══════════════════════════════════════════════════════════════

// ── attrValues: lazy length / head / elemAt ───────────────────

#[test]
fn attr_values_length_does_not_force_throw() {
  // Pre-fix: errored. Now: returns 2.
  assert_eq!(
    json(
      r#"
      builtins.length (
        builtins.attrValues { a = 1; b = throw "x"; }
      )
    "#
    ),
    "2"
  );
}

#[test]
fn attr_values_head_only_forces_first() {
  // First value (sorted by key) is `a` = 1. throw is at `b`.
  // head should return 1 without forcing the throw.
  assert_eq!(
    json(
      r#"
      builtins.head (
        builtins.attrValues { a = 1; b = throw "x"; }
      )
    "#
    ),
    "1"
  );
}

#[test]
fn attr_values_elem_at_zero_only_forces_first() {
  assert_eq!(
    json(
      r#"
      builtins.elemAt (
        builtins.attrValues { a = 1; b = throw "x"; }
      ) 0
    "#
    ),
    "1"
  );
}

#[test]
fn attr_values_force_indexed_throw_propagates() {
  // Sanity: forcing the throw value still fires.
  let e = err(
    r#"
    builtins.elemAt (
      builtins.attrValues { a = 1; b = throw "x"; }
    ) 1
  "#,
  );
  assert!(e.contains("x"), "got: {e}");
}

#[test]
fn attr_values_happy_path_unchanged() {
  // Sanity.
  assert_eq!(
    json(r#"builtins.attrValues { a = 1; b = 2; c = 3; }"#),
    "[1,2,3]"
  );
}

#[test]
fn attr_values_unused_in_let_does_not_throw() {
  // Lazy let-binding: the attrValues never accessed → throw
  // never fires.
  assert_eq!(
    json(
      r#"
      let xs = builtins.attrValues { a = throw "x"; b = throw "y"; };
      in 99
    "#
    ),
    "99"
  );
}

// ── keys / values aliases (same shape) ────────────────────────

#[test]
fn keys_length_does_not_force_throw() {
  assert_eq!(
    json(
      r#"
      builtins.length (
        builtins.keys { a = 1; b = throw "x"; }
      )
    "#
    ),
    "2"
  );
}

#[test]
fn values_length_does_not_force_throw() {
  assert_eq!(
    json(
      r#"
      builtins.length (
        builtins.values { a = 1; b = throw "x"; }
      )
    "#
    ),
    "2"
  );
}

// ── attrNames parity (was already lazy) ──────────────────────

#[test]
fn attr_names_length_lazy_unchanged() {
  // Sanity: attrNames was already lazy. The slice #75 fix
  // doesn't break this.
  assert_eq!(
    json(
      r#"
      builtins.length (
        builtins.attrNames { a = throw "x"; b = throw "y"; }
      )
    "#
    ),
    "2"
  );
}

// ── empty attrset ─────────────────────────────────────────────

#[test]
fn attr_values_empty_returns_empty_list() {
  assert_eq!(json(r#"builtins.attrValues { }"#), "[]");
}

#[test]
fn keys_empty_returns_empty_list() {
  assert_eq!(json(r#"builtins.keys { }"#), "[]");
}

// ── ordering preserved ───────────────────────────────────────

#[test]
fn attr_values_sorted_by_key() {
  // BTreeMap is sorted; attrValues returns values in sorted-
  // key order.
  assert_eq!(
    json(r#"builtins.attrValues { z = 3; a = 1; m = 2; }"#),
    "[1,2,3]"
  );
}

#[test]
fn keys_sorted_lexicographically() {
  assert_eq!(
    json(r#"builtins.keys { z = 3; a = 1; m = 2; }"#),
    r#"["a","m","z"]"#
  );
}

// ── error shapes for non-attrset arg unchanged ───────────────

#[test]
fn attr_values_int_arg_still_errors() {
  let e = err(r#"builtins.attrValues 42"#);
  assert!(e.contains("attrValues"), "got: {e}");
  assert!(e.contains("expected attrset"), "got: {e}");
}

#[test]
fn keys_int_arg_still_errors() {
  let e = err(r#"builtins.keys 42"#);
  assert!(e.contains("keys"), "got: {e}");
}

// ═══════════════════════════════════════════════════════════════
// slice #76 — getAttr / catAttrs laziness — extends slice #75
// lazy-extractor contract to single-attr accessor and list-of-
// attrset extractor. Pre-fix: same boundary issue as slice #75
// (deep_force on input). Plus `catAttrs` impl now per-element
// shallow-forces so thunked list entries resolve to attrsets
// before the AttrSet pattern-match.
//
// 2026-05-06: merged from `eval_getattr_catattrs_lazy.rs`.
// ═══════════════════════════════════════════════════════════════

// ── getAttr: lazy in unused attrs ─────────────────────────────

#[test]
fn get_attr_does_not_force_unrelated_throw() {
  // Pre-fix: errored on b. Now: returns 1.
  assert_eq!(
    json(r#"builtins.getAttr "a" { a = 1; b = throw "x"; }"#),
    "1"
  );
}

#[test]
fn get_attr_only_forces_requested_attr() {
  // The string is forced (it's the requested attr), but b stays.
  assert_eq!(
    json(r#"builtins.getAttr "a" { a = "ok"; b = throw "x"; }"#),
    r#""ok""#
  );
}

#[test]
fn get_attr_forces_requested_throw_propagates() {
  // Sanity: forcing the requested attr's throw still fires.
  let e = err(r#"builtins.getAttr "b" { a = 1; b = throw "x"; }"#);
  assert!(e.contains("x"), "got: {e}");
}

#[test]
fn get_attr_missing_key_error_unchanged() {
  // Sanity: missing attr error not affected.
  let e = err(r#"builtins.getAttr "z" { a = 1; b = 2; }"#);
  assert!(e.contains("getAttr") && e.contains("'z'"), "got: {e}");
}

#[test]
fn get_attr_unused_in_let_does_not_throw() {
  // Lazy let-binding: result never accessed → throw never fires.
  assert_eq!(
    json(
      r#"
      let v = builtins.getAttr "a" { a = throw "x"; b = 1; };
      in 99
    "#
    ),
    "99"
  );
}

// ── catAttrs: lazy in extracted values ────────────────────────

#[test]
fn cat_attrs_length_does_not_force_throw() {
  // Pre-fix: errored. Now: returns 2.
  assert_eq!(
    json(
      r#"
      builtins.length (
        builtins.catAttrs "a" [ { a = 1; } { a = throw "x"; } ]
      )
    "#
    ),
    "2"
  );
}

#[test]
fn cat_attrs_head_only_forces_first() {
  // Both entries have attr `a`. First value is 1, second is throw.
  // head should return 1 without firing throw.
  assert_eq!(
    json(
      r#"
      builtins.head (
        builtins.catAttrs "a" [ { a = 1; } { a = throw "x"; } ]
      )
    "#
    ),
    "1"
  );
}

#[test]
fn cat_attrs_elem_at_zero_only_forces_first() {
  assert_eq!(
    json(
      r#"
      builtins.elemAt (
        builtins.catAttrs "a" [ { a = 1; } { a = throw "x"; } ]
      ) 0
    "#
    ),
    "1"
  );
}

#[test]
fn cat_attrs_force_indexed_throw_propagates() {
  // Sanity: forcing the throw value still fires.
  let e = err(
    r#"
    builtins.elemAt (
      builtins.catAttrs "a" [ { a = 1; } { a = throw "x"; } ]
    ) 1
  "#,
  );
  assert!(e.contains("x"), "got: {e}");
}

#[test]
fn cat_attrs_skips_entries_without_attr() {
  // Entries without attr `a` are skipped; result keeps thunks
  // intact for entries that match.
  assert_eq!(
    json(
      r#"
      builtins.catAttrs "a" [ { a = 1; } { b = 2; } { a = 3; } ]
    "#
    ),
    "[1,3]"
  );
}

#[test]
fn cat_attrs_happy_path_unchanged() {
  // Sanity: standard usage.
  assert_eq!(
    json(r#"builtins.catAttrs "x" [ { x = 1; } { x = 2; } { x = 3; } ]"#),
    "[1,2,3]"
  );
}

#[test]
fn cat_attrs_unused_in_let_does_not_throw() {
  // Lazy let-binding: catAttrs never accessed → no force.
  assert_eq!(
    json(
      r#"
      let r = builtins.catAttrs "a" [ { a = throw "x"; } { a = throw "y"; } ];
      in 99
    "#
    ),
    "99"
  );
}

#[test]
fn cat_attrs_thunked_list_element_must_force_to_inspect() {
  // Sanity: catAttrs MUST force each list element to inspect for
  // the attr. A thrown list element fires — this is correct
  // semantics, not a regression. (Real Nix behaves the same.)
  let e = err(
    r#"
    builtins.length (
      builtins.catAttrs "x" [ { x = 1; } (throw "list-item") ]
    )
  "#,
  );
  assert!(e.contains("list-item"), "got: {e}");
}

#[test]
fn cat_attrs_empty_list() {
  assert_eq!(json(r#"builtins.catAttrs "a" [ ]"#), "[]");
}

#[test]
fn cat_attrs_empty_result_when_no_match() {
  assert_eq!(
    json(r#"builtins.catAttrs "z" [ { a = 1; } { b = 2; } ]"#),
    "[]"
  );
}

// ── error shapes unchanged ────────────────────────────────────

#[test]
fn get_attr_int_arg_still_errors() {
  let e = err(r#"builtins.getAttr "a" 42"#);
  assert!(e.contains("getAttr"), "got: {e}");
}

#[test]
fn cat_attrs_non_list_arg_errors() {
  let e = err(r#"builtins.catAttrs "a" 42"#);
  assert!(e.contains("catAttrs"), "got: {e}");
}

#[test]
fn cat_attrs_non_attrset_element_errors_with_force() {
  // After force_value, a non-attrset list element produces a
  // typed error mentioning catAttrs.
  let e = err(r#"builtins.catAttrs "a" [ { a = 1; } 42 ]"#);
  assert!(e.contains("catAttrs") && e.contains("attrset"), "got: {e}");
}

// ═══════════════════════════════════════════════════════════════
// slice #77 — `attrByPath` / `getAttrs` builtins added —
// closes the missing-builtin gap, extends slice #68/#69/#70
// family. `attrByPath` joined `no_force_at_all` because its
// default arg must stay lazy. `getAttrs` joined
// `lazy_in_elements`.
//
// 2026-05-06: merged from
// `eval_attr_by_path_get_attrs_builtins.rs`. The lazy-default
// `attrByPath` semantics belongs to the same lazy-boundary
// family as #75/#76 in this file.
// ═══════════════════════════════════════════════════════════════

// ── attrByPath: path resolution, default semantics ────────────

#[test]
fn attr_by_path_simple_lookup() {
  assert_eq!(
    json(r#"builtins.attrByPath [ "a" "b" ] 0 { a = { b = 42; }; }"#),
    "42"
  );
}

#[test]
fn attr_by_path_missing_leaf_returns_default() {
  assert_eq!(
    json(r#"builtins.attrByPath [ "a" "z" ] 99 { a = { b = 1; }; }"#),
    "99"
  );
}

#[test]
fn attr_by_path_missing_root_returns_default() {
  assert_eq!(
    json(r#"builtins.attrByPath [ "missing" ] 99 { a = 1; }"#),
    "99"
  );
}

#[test]
fn attr_by_path_empty_path_returns_whole_attrset() {
  // Real Nix: `attrByPath [] _ x` = x.
  assert_eq!(
    json(r#"builtins.attrByPath [ ] 99 { a = 1; b = 2; }"#),
    r#"{"a":1,"b":2}"#
  );
}

#[test]
fn attr_by_path_default_stays_lazy_when_path_resolves() {
  // KEY load-bearing case: default is `throw`, but the path
  // resolves so the throw should never fire. Pre-fix, the
  // boundary's force_value would have forced the default arg
  // and fired the throw even on a successful lookup.
  assert_eq!(
    json(r#"builtins.attrByPath [ "a" ] (throw "default-fired") { a = 42; }"#),
    "42"
  );
}

#[test]
fn attr_by_path_default_fires_on_missing_path() {
  // Sanity: when path is missing, the default IS the result —
  // forcing it fires the throw.
  let e = err(r#"builtins.attrByPath [ "missing" ] (throw "default-x") { a = 1; }"#);
  assert!(e.contains("default-x"), "got: {e}");
}

#[test]
fn attr_by_path_lazy_in_unrelated_attrset_values() {
  // Lookup of `a` should not force `b` (which is a throw).
  assert_eq!(
    json(r#"builtins.attrByPath [ "a" ] 0 { a = 1; b = throw "x"; }"#),
    "1"
  );
}

#[test]
fn attr_by_path_intermediate_non_attrset_returns_default() {
  // Path step at a non-attrset value falls back to default.
  assert_eq!(
    json(r#"builtins.attrByPath [ "a" "b" ] 99 { a = "string"; }"#),
    "99"
  );
}

#[test]
fn attr_by_path_unused_in_let_does_not_throw() {
  // Lazy let-binding: result never accessed → throw never fires.
  assert_eq!(
    json(
      r#"
      let v = builtins.attrByPath [ "a" ] 0 { a = throw "x"; };
      in 99
    "#
    ),
    "99"
  );
}

#[test]
fn attr_by_path_non_list_path_errors() {
  let e = err(r#"builtins.attrByPath "a" 0 { a = 1; }"#);
  assert!(e.contains("attrByPath") && e.contains("list"), "got: {e}");
}

#[test]
fn attr_by_path_non_string_segment_errors() {
  let e = err(r#"builtins.attrByPath [ 1 ] 0 { a = 1; }"#);
  assert!(e.contains("attrByPath") && e.contains("string"), "got: {e}");
}

#[test]
fn attr_by_path_three_levels_deep() {
  assert_eq!(
    json(r#"builtins.attrByPath [ "a" "b" "c" ] 0 { a = { b = { c = 7; }; }; }"#),
    "7"
  );
}

// ── getAttrs: subset extraction, lazy in selected values ──────

#[test]
fn get_attrs_simple_subset() {
  assert_eq!(
    json(r#"builtins.getAttrs [ "a" "b" ] { a = 1; b = 2; c = 3; }"#),
    r#"{"a":1,"b":2}"#
  );
}

#[test]
fn get_attrs_empty_names_returns_empty_attrset() {
  assert_eq!(json(r#"builtins.getAttrs [ ] { a = 1; b = 2; }"#), "{}");
}

#[test]
fn get_attrs_missing_name_errors() {
  // Real Nix: `getAttrs ["x"] {}` errors on missing attr.
  let e = err(r#"builtins.getAttrs [ "z" ] { a = 1; }"#);
  assert!(e.contains("getAttrs") && e.contains("'z'"), "got: {e}");
}

#[test]
fn get_attrs_lazy_in_unrelated_values() {
  // Selecting `a` should not force `b` (which is a throw).
  assert_eq!(
    json(r#"builtins.getAttrs [ "a" ] { a = 1; b = throw "x"; }"#),
    r#"{"a":1}"#
  );
}

#[test]
fn get_attrs_lazy_in_selected_value() {
  // Selecting `a` (which IS a throw) should not force the
  // result attrset's values until accessed.
  assert_eq!(
    json(
      r#"
      let r = builtins.getAttrs [ "a" ] { a = throw "x"; b = 2; };
      in builtins.length (builtins.attrNames r)
    "#
    ),
    "1"
  );
}

#[test]
fn get_attrs_force_selected_throw_propagates() {
  // Sanity: forcing the selected throw value fires.
  let e = err(
    r#"
    let r = builtins.getAttrs [ "a" ] { a = throw "x"; };
    in r.a
  "#,
  );
  assert!(e.contains("x"), "got: {e}");
}

#[test]
fn get_attrs_unused_in_let_does_not_throw() {
  assert_eq!(
    json(
      r#"
      let r = builtins.getAttrs [ "a" "b" ] { a = throw "x"; b = throw "y"; };
      in 99
    "#
    ),
    "99"
  );
}

#[test]
fn get_attrs_non_list_names_errors() {
  let e = err(r#"builtins.getAttrs "a" { a = 1; }"#);
  assert!(e.contains("getAttrs") && e.contains("list"), "got: {e}");
}

#[test]
fn get_attrs_non_attrset_arg_errors() {
  let e = err(r#"builtins.getAttrs [ "a" ] 42"#);
  assert!(e.contains("getAttrs") && e.contains("attrset"), "got: {e}");
}

#[test]
fn get_attrs_non_string_name_errors() {
  let e = err(r#"builtins.getAttrs [ 1 ] { a = 1; }"#);
  assert!(e.contains("getAttrs") && e.contains("string"), "got: {e}");
}

#[test]
fn get_attrs_preserves_value_types() {
  // Selected values keep their original types.
  assert_eq!(
    json(
      r#"
      builtins.getAttrs [ "i" "s" "l" ] {
        i = 42;
        s = "hello";
        l = [ 1 2 3 ];
        extra = throw "should-not-fire";
      }
    "#
    ),
    r#"{"i":42,"l":[1,2,3],"s":"hello"}"#
  );
}

// ── interplay: attrByPath as a getAttrs target path ───────────

#[test]
fn attr_by_path_into_get_attrs_result() {
  // Real-world idiom: getAttrs to filter, then attrByPath to
  // walk the filtered set.
  assert_eq!(
    json(
      r#"
      builtins.attrByPath [ "a" ] 0 (
        builtins.getAttrs [ "a" "b" ] { a = 7; b = 8; c = 9; }
      )
    "#
    ),
    "7"
  );
}
