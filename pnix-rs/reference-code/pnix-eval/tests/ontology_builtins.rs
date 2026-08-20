//! batch 264 (2026-04-18): G2.1 / G2.2 ontology builtin real implementation 검증.
//!
//! `builtins.ontologyQuery` — pure query descriptor envelope
//! `builtins.ontologyEmit` — ExpressionProjectionRecord canonical emit

use pnix_eval::{eval_expr, Value};

fn eval(src: &str) -> Value {
  eval_expr(src).unwrap_or_else(|e| panic!("eval error on `{}`: {}", src, e))
}

#[test]
fn ontology_query_wraps_input_with_canonical_kind() {
  let v = eval(
    r#"builtins.ontologyQuery { context = "Physics.Classical"; subject = "Force"; predicate = "depends-on"; }"#,
  );
  match v {
    Value::AttrSet(m) => {
      assert_eq!(
        m.get("query-kind").and_then(|v| v.as_str()),
        Some("ontology-query")
      );
      assert_eq!(
        m.get("context").and_then(|v| v.as_str()),
        Some("Physics.Classical")
      );
      assert_eq!(m.get("subject").and_then(|v| v.as_str()), Some("Force"));
      assert_eq!(
        m.get("predicate").and_then(|v| v.as_str()),
        Some("depends-on")
      );
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn ontology_query_preserves_existing_query_kind() {
  // user-provided query-kind override 시 기존 값 유지.
  let v = eval(r#"builtins.ontologyQuery { query-kind = "custom-kind"; context = "X"; }"#);
  match v {
    Value::AttrSet(m) => {
      assert_eq!(
        m.get("query-kind").and_then(|v| v.as_str()),
        Some("custom-kind")
      );
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn ontology_emit_fills_all_four_surface_forms() {
  let v = eval(r#"builtins.ontologyEmit { projection-family = "expmath"; }"#);
  match v {
    Value::AttrSet(m) => {
      assert_eq!(
        m.get("projection-family").and_then(|v| v.as_str()),
        Some("expmath")
      );
      assert_eq!(
        m.get("emit-kind").and_then(|v| v.as_str()),
        Some("expression-projection")
      );
      // surface-forms 에 4 canonical key 존재.
      let forms = match m.get("surface-forms") {
        Some(Value::AttrSet(m)) => m,
        other => panic!("surface-forms must be AttrSet, got {:?}", other),
      };
      for key in [
        "openmath",
        "mathml-content",
        "canonical-text",
        "freecat-geometry",
      ] {
        assert!(
          forms.contains_key(key),
          "surface-forms missing key `{}`",
          key
        );
      }
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn ontology_emit_preserves_provided_surface_forms() {
  let v = eval(
    r#"builtins.ontologyEmit {
      projection-family = "expmath";
      surface-forms = {
        canonical-text = "2 + 2 = 4";
        mathml-content = "<mfrac><mn>2</mn><mn>2</mn></mfrac>";
      };
    }"#,
  );
  match v {
    Value::AttrSet(m) => {
      let forms = match m.get("surface-forms") {
        Some(Value::AttrSet(m)) => m,
        _ => panic!("surface-forms must be AttrSet"),
      };
      assert_eq!(
        forms.get("canonical-text").and_then(|v| v.as_str()),
        Some("2 + 2 = 4")
      );
      assert!(forms
        .get("mathml-content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .contains("mfrac"),);
      // 누락된 두 form 은 null.
      assert!(matches!(forms.get("openmath"), Some(Value::Null)));
      assert!(matches!(forms.get("freecat-geometry"), Some(Value::Null)));
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn ontology_emit_fills_default_projection_family_when_missing() {
  let v = eval(r#"builtins.ontologyEmit {}"#);
  match v {
    Value::AttrSet(m) => {
      assert_eq!(
        m.get("projection-family").and_then(|v| v.as_str()),
        Some("expmath")
      );
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

// batch 265 (2026-04-18): G2.3 / G2.4 테스트.

#[test]
fn ontology_evaluate_computes_six_axes_from_interpretation_shape() {
  let v = eval(
    r#"builtins.ontologyEvaluate
      { policy = "default"; }
      {
        interpretation-id = "I1";
        status = "candidate";
        facts = [ { subj = "F"; pred = "has"; obj = "m"; } { subj = "F"; pred = "has"; obj = "a"; } { subj = "F"; pred = "has"; obj = "formula"; } ];
        proof_refs = [ "proof-F-ma" ];
      }"#,
  );
  match v {
    Value::AttrSet(m) => {
      // id + non-contradicted → coherence 1.0
      assert_eq!(
        m.get("evaluation-coherence").and_then(|v| v.as_f64()),
        Some(1.0)
      );
      // 3 facts → coverage 1.0
      assert_eq!(
        m.get("evaluation-coverage").and_then(|v| v.as_f64()),
        Some(1.0)
      );
      // loss 0 (no losses)
      assert_eq!(m.get("evaluation-loss").and_then(|v| v.as_f64()), Some(0.0));
      // proof_refs present → replayability 1.0
      assert_eq!(
        m.get("evaluation-replayability").and_then(|v| v.as_f64()),
        Some(1.0)
      );
      // safety 1.0 (not contradicted)
      assert_eq!(
        m.get("evaluation-safety").and_then(|v| v.as_f64()),
        Some(1.0)
      );
      // score computed and present
      assert!(m.get("evaluation-score").and_then(|v| v.as_f64()).is_some());
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn ontology_evaluate_lowers_axes_for_contradicted_status() {
  let v = eval(
    r#"builtins.ontologyEvaluate
      { }
      {
        interpretation-id = "I2";
        status = "contradicted";
        facts = [ ];
      }"#,
  );
  match v {
    Value::AttrSet(m) => {
      // contradicted → safety 0
      assert_eq!(
        m.get("evaluation-safety").and_then(|v| v.as_f64()),
        Some(0.0)
      );
      // 0 facts → coverage 0
      assert_eq!(
        m.get("evaluation-coverage").and_then(|v| v.as_f64()),
        Some(0.0)
      );
      // coherence falls to 0.5 when contradicted
      assert_eq!(
        m.get("evaluation-coherence").and_then(|v| v.as_f64()),
        Some(0.5)
      );
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn ontology_select_picks_highest_score() {
  let v = eval(
    r#"builtins.ontologySelect
      { }
      [
        { interpretation-id = "A"; evaluation-score = 0.4; evaluation-safety = 1.0; evaluation-replayability = 1.0; evaluation-loss = 0.1; evaluation-cost = 1.0; }
        { interpretation-id = "B"; evaluation-score = 0.9; evaluation-safety = 1.0; evaluation-replayability = 1.0; evaluation-loss = 0.0; evaluation-cost = 1.0; }
        { interpretation-id = "C"; evaluation-score = 0.6; evaluation-safety = 1.0; evaluation-replayability = 1.0; evaluation-loss = 0.0; evaluation-cost = 1.0; }
      ]"#,
  );
  match v {
    Value::AttrSet(m) => assert_eq!(
      m.get("interpretation-id").and_then(|v| v.as_str()),
      Some("B")
    ),
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn ontology_select_tie_breaks_by_safety_then_replay_then_loss() {
  // 모두 동일 score, safety 가 tie-break
  let v = eval(
    r#"builtins.ontologySelect
      { }
      [
        { interpretation-id = "X"; evaluation-score = 0.5; evaluation-safety = 0.5; evaluation-replayability = 1.0; evaluation-loss = 0.0; evaluation-cost = 1.0; }
        { interpretation-id = "Y"; evaluation-score = 0.5; evaluation-safety = 1.0; evaluation-replayability = 0.5; evaluation-loss = 0.0; evaluation-cost = 1.0; }
      ]"#,
  );
  match v {
    Value::AttrSet(m) => assert_eq!(
      m.get("interpretation-id").and_then(|v| v.as_str()),
      Some("Y")
    ),
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn ontology_select_falls_back_to_lexical_id_when_all_axes_tied() {
  // 모두 동일 — lexical id 오름차순 (작은 id 가 winner, Reverse 로 내림차순 sort).
  // "A" < "B" 이므로 A 가 winner.
  let v = eval(
    r#"builtins.ontologySelect
      { }
      [
        { interpretation-id = "B"; evaluation-score = 0.5; evaluation-safety = 1.0; evaluation-replayability = 1.0; evaluation-loss = 0.0; evaluation-cost = 1.0; }
        { interpretation-id = "A"; evaluation-score = 0.5; evaluation-safety = 1.0; evaluation-replayability = 1.0; evaluation-loss = 0.0; evaluation-cost = 1.0; }
      ]"#,
  );
  match v {
    Value::AttrSet(m) => assert_eq!(
      m.get("interpretation-id").and_then(|v| v.as_str()),
      Some("A")
    ),
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn ontology_select_returns_null_on_empty_list() {
  let v = eval(r#"builtins.ontologySelect { } []"#);
  assert!(matches!(v, Value::Null));
}

#[test]
fn ontology_select_computes_keys_for_unevaluated_candidates() {
  // evaluation-* 필드가 없으면 compute_evaluation_axes 로 계산.
  // "has-proof" 가 더 나은 replayability 를 받아 winner.
  let v = eval(
    r#"builtins.ontologySelect
      { }
      [
        { interpretation-id = "raw"; status = "candidate"; facts = [ { s = "x"; } ]; }
        { interpretation-id = "has-proof"; status = "candidate"; facts = [ { s = "x"; } ]; proof_refs = [ "p1" ]; }
      ]"#,
  );
  match v {
    Value::AttrSet(m) => assert_eq!(
      m.get("interpretation-id").and_then(|v| v.as_str()),
      Some("has-proof")
    ),
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn ontology_evaluate_and_select_compose_in_deterministic_pipeline() {
  // evaluate → select 결합.
  let v = eval(
    r#"
    let
      candidates = [
        { interpretation-id = "A"; status = "candidate"; facts = [ { s = "a"; } ]; }
        { interpretation-id = "B"; status = "candidate"; facts = [ { s = "b1"; } { s = "b2"; } { s = "b3"; } ]; proof_refs = [ "p" ]; }
      ];
      evaluated = builtins.map (c: builtins.ontologyEvaluate {} c) candidates;
    in builtins.ontologySelect {} evaluated
    "#,
  );
  match v {
    Value::AttrSet(m) => {
      // B 가 facts 3 + proof → higher score → winner.
      assert_eq!(
        m.get("interpretation-id").and_then(|v| v.as_str()),
        Some("B")
      );
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}

#[test]
fn ontology_query_and_emit_compose_in_pipeline() {
  // query → emit 파이프라인이 canonical kind + form shape 을 둘 다 유지.
  let v = eval(
    r#"
    let
      query = builtins.ontologyQuery { context = "Math.Arith"; subject = "2+2"; predicate = "evaluates-to"; };
      emission = builtins.ontologyEmit {
        source-query = query;
        surface-forms = { canonical-text = "2 + 2 = 4"; };
      };
    in emission
    "#,
  );
  match v {
    Value::AttrSet(m) => {
      assert_eq!(
        m.get("emit-kind").and_then(|v| v.as_str()),
        Some("expression-projection")
      );
      let source_query = match m.get("source-query") {
        Some(Value::AttrSet(q)) => q,
        _ => panic!("source-query must be AttrSet"),
      };
      assert_eq!(
        source_query.get("query-kind").and_then(|v| v.as_str()),
        Some("ontology-query")
      );
    }
    other => panic!("expected AttrSet, got {:?}", other),
  }
}
