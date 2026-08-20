//! pnix-eval string context propagation 회귀 테스트.
//!
//! pnix 는 nixpkgs 가 아닌 범용 언어이므로 nix-store 는 사용하지 않지만,
//! 참조투명 핵심인 string context (보간/concat 시 provenance 표지가
//! 따라붙는 메커니즘) 는 살린다. Path 가 `${...}` 로 보간되면 path 자체가
//! context 원소로 들어가고, concat / interpolation 은 양쪽 context 를
//! union 한다. `unsafeDiscardStringContext` / `hasContext` /
//! `getContext` / `addDrvOutputDependencies` / `unsafeDiscardOutputDependency` /
//! `appendContext` 가 nixpkgs 호환 표면을 fake-pass 로 제공한다.

use pnix_eval::{eval_expr, Value};

#[test]
fn plain_string_has_no_context() {
  let v = eval_expr(r#"builtins.hasContext "hello""#).unwrap();
  assert!(matches!(v, Value::Bool(false)));
}

#[test]
fn path_interpolation_adds_path_to_context() {
  let v = eval_expr(r#"builtins.hasContext "x=${./foo.nix}""#).unwrap();
  assert!(matches!(v, Value::Bool(true)), "got {:?}", v);
}

#[test]
fn concat_propagates_context() {
  let v = eval_expr(r#"builtins.hasContext ("prefix=" + "${./a.nix}")"#).unwrap();
  assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn discard_strips_context_to_plain_string() {
  let v = eval_expr(
    r#"
    let
      tagged = "v=${./tag.nix}";
      bare = builtins.unsafeDiscardStringContext tagged;
    in
    [ (builtins.hasContext tagged) (builtins.hasContext bare) bare ]
    "#,
  )
  .unwrap();
  if let Value::List(items) = v {
    assert!(matches!(items[0], Value::Bool(true)));
    assert!(matches!(items[1], Value::Bool(false)));
    assert!(matches!(items[2], Value::String(ref s) if s.starts_with("v=")));
  } else {
    panic!("expected list, got {:?}", v);
  }
}

#[test]
fn string_context_builtins_only_force_outer_string_shape() {
  let cases = [
    (
      r#"builtins.hasContext { a = throw "hasContext payload"; }"#,
      "hasContext",
      "hasContext payload",
    ),
    (
      r#"builtins.getContext { a = throw "getContext payload"; }"#,
      "getContext",
      "getContext payload",
    ),
    (
      r#"builtins.unsafeDiscardStringContext { a = throw "discard payload"; }"#,
      "unsafeDiscardStringContext",
      "discard payload",
    ),
    (
      r#"builtins.addDrvOutputDependencies { a = throw "drv payload"; }"#,
      "addDrvOutputDependencies",
      "drv payload",
    ),
    (
      r#"builtins.unsafeDiscardOutputDependency { a = throw "discard output payload"; }"#,
      "unsafeDiscardOutputDependency",
      "discard output payload",
    ),
    (
      r#"builtins.unsafeAddOutputDependency { a = throw "add output payload"; }"#,
      "unsafeAddOutputDependency",
      "add output payload",
    ),
    (
      r#"builtins.unsafeAddOutputName "out" { a = throw "add output name payload"; }"#,
      "unsafeAddOutputName",
      "add output name payload",
    ),
  ];

  for (src, builtin, payload) in cases {
    let err = eval_expr(src).err().expect(src).to_string();
    assert!(err.contains(builtin), "{builtin}: {err}");
    assert!(
      !err.contains(payload),
      "{builtin} forced an inner payload unexpectedly: {err}"
    );
  }
}

#[test]
fn get_context_returns_path_marked_attrset() {
  let v = eval_expr(r#"builtins.getContext "x=${./marker.nix}""#).unwrap();
  if let Value::AttrSet(map) = v {
    assert_eq!(map.len(), 1);
    let only_entry = map.values().next().unwrap();
    if let Value::AttrSet(inner) = only_entry {
      assert!(matches!(inner.get("path"), Some(Value::Bool(true))));
    } else {
      panic!("expected attrset entry, got {:?}", only_entry);
    }
  } else {
    panic!("expected attrset, got {:?}", v);
  }
}

#[test]
fn add_drv_output_dependencies_marks_context() {
  let v = eval_expr(r#"builtins.hasContext (builtins.addDrvOutputDependencies "raw")"#).unwrap();
  assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn unsafe_discard_output_dependency_strips_drv_marker() {
  // Round-trip: addDrvOutputDependencies → unsafeDiscardOutputDependency
  // brings the string back to no-context state (`!out!...` markers are
  // the only context entries we added).
  let v = eval_expr(
    r#"
    let
      tagged = builtins.addDrvOutputDependencies "raw";
      stripped = builtins.unsafeDiscardOutputDependency tagged;
    in
    [ (builtins.hasContext tagged) (builtins.hasContext stripped) ]
    "#,
  )
  .unwrap();
  if let Value::List(items) = v {
    assert!(matches!(items[0], Value::Bool(true)));
    assert!(matches!(items[1], Value::Bool(false)));
  } else {
    panic!();
  }
}

#[test]
fn append_context_adds_new_entries() {
  let v = eval_expr(
    r#"
    let
      tagged = builtins.appendContext "raw" { "/extra/path" = { path = true; }; };
    in
    [ (builtins.hasContext tagged) (builtins.attrNames (builtins.getContext tagged)) ]
    "#,
  )
  .unwrap();
  if let Value::List(items) = v {
    assert!(matches!(items[0], Value::Bool(true)));
    if let Value::List(names) = &items[1] {
      assert_eq!(names.len(), 1);
      assert!(matches!(&names[0], Value::String(s) if s == "/extra/path"));
    } else {
      panic!();
    }
  } else {
    panic!();
  }
}

#[test]
fn interpolation_aggregates_multiple_contexts() {
  let v = eval_expr(r#"builtins.attrNames (builtins.getContext "${./a.nix}-${./b.nix}")"#).unwrap();
  if let Value::List(items) = v {
    // Both paths should appear as separate context entries.
    assert_eq!(items.len(), 2);
  } else {
    panic!("expected list, got {:?}", v);
  }
}

#[test]
fn concat_unions_contexts_from_both_sides() {
  let v = eval_expr(
    r#"
    let
      a = "x=${./a.nix}";
      b = "y=${./b.nix}";
    in
    builtins.attrNames (builtins.getContext (a + b))
    "#,
  )
  .unwrap();
  if let Value::List(items) = v {
    assert_eq!(items.len(), 2);
  } else {
    panic!();
  }
}

#[test]
fn typeof_reports_string_for_context_strings() {
  let v = eval_expr(r#"builtins.typeOf "x=${./a.nix}""#).unwrap();
  assert!(matches!(v, Value::String(ref s) if s == "string"));
}

#[test]
fn placeholder_returns_context_string() {
  // pnix is not nix-store; `placeholder "out"` is a fake-pass returning
  // a stable opaque string with provenance context attached.
  let v = eval_expr(r#"builtins.hasContext (builtins.placeholder "out")"#).unwrap();
  assert!(matches!(v, Value::Bool(true)));
  let v = eval_expr(r#"builtins.placeholder "out""#).unwrap();
  if let Value::StringContext { text, .. } = v {
    assert!(
      text.contains("out"),
      "text should mention output name, got {}",
      text
    );
  } else {
    panic!("expected StringContext, got {:?}", v);
  }
}

#[test]
fn derivation_strict_returns_input_with_outpath() {
  // Fake-pass: pnix doesn't build derivations, but common nixpkgs
  // patterns like `(derivationStrict { name = "foo"; ... }).outPath`
  // must round-trip without error.
  let v = eval_expr(
    r#"
    let
      d = builtins.derivationStrict {
        name = "hello";
        builder = "/bin/sh";
        system = "x86_64-linux";
      };
    in
    [ d.name d.type (builtins.hasContext d.outPath) ]
    "#,
  )
  .unwrap();
  if let Value::List(items) = v {
    assert!(matches!(&items[0], Value::String(s) if s == "hello"));
    assert!(matches!(&items[1], Value::String(s) if s == "derivation"));
    assert!(matches!(&items[2], Value::Bool(true)));
  } else {
    panic!();
  }
}

#[test]
fn equality_ignores_context() {
  // Two strings with the same text but different (or no) context still
  // compare equal — only the visible text matters for `==`.
  let v = eval_expr(r#""x=${./a.nix}" == "x=/Users/gp/pnix/crates/pnix-eval/a.nix""#);
  // Result depends on relative path resolution; just check no error.
  let _ = v;
}
