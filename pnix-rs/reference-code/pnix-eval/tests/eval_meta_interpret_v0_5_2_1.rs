//! minimal-ontology-tesseract-v0.5.2.1 — micro-hygiene patch on
//! top of v0.5.2's first interaction proof. Codex 2026-05-06
//! v0.5.2 review directive: tighten the "no re-skin" claim with
//! a direct builder equality test. v0.5.2's invariant 359
//! verifies output-turn shape against the buildAttachTurn
//! contract; v0.5.2.1's invariant 363 verifies BYTE-EQUALITY
//! against a freshly-evaluated direct call to the same builder
//! INSIDE pnix.
//!
//! Truth owner: project-wiki/maps/minimal-ontology-tesseract-v0-map.md
//!              §"v0.5.2.1 micro-hygiene — direct builder
//!               equality"
//! Active scope: project-wiki/maps/active-domain-constitution.md
//!               Art. 6, Art. 7
//!
//! Test count: 1 invariant, index 363.

use pnix_eval::{eval_expr, eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn v0_5_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/meta-interpret-v0_5")
}

fn run_path() -> PathBuf {
  v0_5_root().join("v0_5_2_run.px")
}

fn as_attrs(v: &Value) -> &BTreeMap<String, Value> {
  match v {
    Value::AttrSet(m) => m,
    other => panic!("expected attrset, got {:?}", other),
  }
}

fn get<'a>(v: &'a Value, key: &str) -> &'a Value {
  let attrs = as_attrs(v);
  attrs.get(key).unwrap_or_else(|| {
    panic!(
      "missing key `{}`; available: {:?}",
      key,
      attrs.keys().collect::<Vec<_>>()
    )
  })
}

#[test]
fn v0_5_2_1_direct_builder_equality_load_bearing() {
  // Invariant 363.
  //
  // Codex 2026-05-06 v0.5.2 review point A:
  //
  //   v0.5.2's invariant 359 checks each field of
  //   `cross-A.output-turn` individually against the
  //   buildAttachTurn contract. That is shape verification.
  //   The stronger claim — "v0.5.2 routes through the SAME
  //   owner-law lambda the coexistence proof exposed; it is
  //   NOT a re-skin or parallel implementation" — needs a
  //   direct equality test.
  //
  // This test loads v0_5_2_run.px once to obtain
  // `specialized-interpreter` and `cross-A`. It then evaluates
  // `si.rule-functions.buildAttachTurn 0 cross-A.derived-lens
  // []` INSIDE pnix and compares the result to
  // `cross-A.output-turn` via `Value::to_json()`. Byte-equality
  // closes the no-re-skin claim definitively: cross-application
  // and direct invocation produce the SAME Turn record.
  //
  // The eval_expr inline source pulls the SI through the
  // runner so the SAME `rule-functions.buildAttachTurn` lambda
  // is the one being called both implicitly (inside cross-A)
  // and explicitly (in this test).

  let pos = eval_file(&run_path()).unwrap();
  let cross_a = get(&pos, "cross-A");
  let cross_output_turn = get(cross_a, "output-turn");

  // Directly invoke the same buildAttachTurn through the SI
  // exposed by v0_5_2_run.px. Use cross-A.derived-lens as the
  // lens argument so all three positional args (turnId=0,
  // lens=derived-lens, prev=[]) match what the runner used.
  let source = format!(
    "let r = import {run:?}; \
       lens = r.cross-A.derived-lens; \
       in r.specialized-interpreter.rule-functions.buildAttachTurn 0 lens [ ]",
    run = run_path(),
  );
  let direct = eval_expr(&source).expect("direct buildAttachTurn invocation must evaluate");

  assert_eq!(
    cross_output_turn.to_json(),
    direct.to_json(),
    "v0.5.2 cross-A.output-turn MUST be byte-equal (Value::to_json) to applying SI.rule-functions.buildAttachTurn directly to (0, cross-A.derived-lens, []). This closes the \"no re-skin / no parallel implementation\" claim: cross-application routes through the SAME owner-law lambda, not a copy. Codex 2026-05-06 v0.5.2 review point A."
  );
}
