//! Ontology bridge — string context propagation 위에 올라타는 회귀.
//!
//! pnix는 nixpkgs/build system이 아닌 범용 의미 substrate. 문자열 안에
//! `${./file}` 같은 path 가 보간되면 string context 에 path 가 들어가고,
//! `ontologyLift` 가 그것을 자동으로 `provenance_refs` 로 끌어올린다.
//! 이렇게 ontology engine 이 "이 fact 는 어디서 왔는가" 를 별도 plumbing
//! 없이 본다.

use pnix_eval::{eval_expr, Value};

#[test]
fn ontology_lift_seeds_provenance_from_string_context() {
  let v = eval_expr(
    r#"
    let
      tagged = "manifest=${./pkg.nix}";
      fact = builtins.ontologyLift tagged "manifest-context";
    in
    fact
    "#,
  )
  .unwrap();
  if let Value::AttrSet(map) = v {
    assert!(matches!(
      map.get("ontology-context"),
      Some(Value::String(s)) if s == "manifest-context"
    ));
    assert!(matches!(
      map.get("ontology-status"),
      Some(Value::String(s)) if s == "Candidate"
    ));
    if let Some(Value::List(prov)) = map.get("provenance_refs") {
      // The path appears in context, so should appear in provenance.
      assert_eq!(prov.len(), 1);
    } else {
      panic!(
        "expected provenance_refs list, got {:?}",
        map.get("provenance_refs")
      );
    }
    if let Some(Value::String(text)) = map.get("value") {
      assert!(text.starts_with("manifest="));
    } else {
      panic!("expected value=text, got {:?}", map.get("value"));
    }
  } else {
    panic!("expected attrset, got {:?}", v);
  }
}

#[test]
fn ontology_lift_preserves_explicit_attrs() {
  // Existing call shape: when args[0] is already an attrset, ontologyLift
  // keeps the user-supplied fields.
  let v = eval_expr(
    r#"
    builtins.ontologyLift {
      facts = [ "f1" "f2" ];
      cost = 1.5;
    } "demo"
    "#,
  )
  .unwrap();
  if let Value::AttrSet(map) = v {
    assert!(matches!(map.get("facts"), Some(Value::List(items)) if items.len() == 2));
    assert!(matches!(map.get("cost"), Some(Value::Float(f)) if (*f - 1.5).abs() < 1e-9));
    assert!(matches!(
      map.get("ontology-context"),
      Some(Value::String(s)) if s == "demo"
    ));
  } else {
    panic!();
  }
}

#[test]
fn ontology_lift_plain_string_no_provenance() {
  // No interpolation → no context → no provenance_refs added.
  let v = eval_expr(r#"builtins.ontologyLift "raw" "ctx""#).unwrap();
  if let Value::AttrSet(map) = v {
    assert!(matches!(map.get("value"), Some(Value::String(s)) if s == "raw"));
    assert!(map.get("provenance_refs").is_none());
  } else {
    panic!();
  }
}

#[test]
fn string_context_to_provenance_returns_list() {
  let v = eval_expr(r#"builtins.stringContextToProvenance "x=${./a.nix}-${./b.nix}""#).unwrap();
  if let Value::List(items) = v {
    assert_eq!(items.len(), 2);
    for it in items.iter() {
      assert!(matches!(it, Value::String(_)));
    }
  } else {
    panic!();
  }
}

#[test]
fn string_context_to_provenance_empty_for_plain_string() {
  let v = eval_expr(r#"builtins.stringContextToProvenance "no context""#).unwrap();
  if let Value::List(items) = v {
    assert_eq!(items.len(), 0);
  } else {
    panic!();
  }
}

#[test]
fn provenance_chains_through_concat_and_lift() {
  // Two paths get joined, then lifted — both paths must appear as
  // provenance refs (showing string context survives through builtin
  // operations and reaches the ontology layer).
  let v = eval_expr(
    r#"
    let
      a = "aa=${./a.nix}";
      b = "bb=${./b.nix}";
      combined = a + b;
      fact = builtins.ontologyLift combined "join";
    in
    fact.provenance_refs
    "#,
  )
  .unwrap();
  if let Value::List(items) = v {
    assert_eq!(items.len(), 2);
  } else {
    panic!("expected provenance_refs list, got {:?}", v);
  }
}

#[test]
fn provenance_refs_compose_with_evaluation_axes() {
  // After lifting a context-bearing string, ontologyEvaluate should see
  // provenance_refs and bump the replayability axis (1.0 instead of 0.5
  // — see compute_evaluation_axes).
  let v = eval_expr(
    r#"
    let
      lifted = builtins.ontologyLift "x=${./bar.nix}" "ctx";
      evald = builtins.ontologyEvaluate {} lifted;
    in
    evald.replayability
    "#,
  )
  .unwrap();
  if let Value::Float(r) = v {
    assert!(
      (r - 1.0).abs() < 1e-9,
      "expected 1.0 (provenance present), got {}",
      r
    );
  } else {
    panic!("expected float, got {:?}", v);
  }
}
