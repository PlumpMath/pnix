//! Gate test: CT optimizer law-preservation smoke.
//!
//! Goal: ensure CT optimization does not introduce new law violations.

use pnix_core::ct::ctast::verify_preservation;
use pnix_core::ct::{CTMorphismOp, CTNode, CTOptimizer, CTType, CTAST};

fn identity_fixture() -> CTAST {
  let mut ctast = CTAST::new();
  ctast.nodes.push(CTNode::new(
    "id",
    CTMorphismOp::Id,
    CTType::real(),
    CTType::real(),
    vec![],
  ));
  ctast.nodes.push(CTNode::new(
    "sin",
    CTMorphismOp::Sin,
    CTType::real(),
    CTType::real(),
    vec!["id".to_string()],
  ));
  ctast.edges.push(("id".to_string(), "sin".to_string()));
  ctast
}

#[test]
fn gate_ct_optimizer_preserves_laws() {
  let before = identity_fixture();
  let optimizer = CTOptimizer::new();
  let result = optimizer.optimize(before.clone());
  let preservation = verify_preservation(&before, &result.ctast);

  assert!(
    preservation.is_preserved(),
    "CT optimization introduced new law violations: {}",
    preservation.summary()
  );
}

#[test]
fn gate_ct_optimizer_applies_identity_elimination_on_fixture() {
  let before = identity_fixture();
  let optimizer = CTOptimizer::new();
  let result = optimizer.optimize(before);

  assert!(
    result
      .applied
      .iter()
      .any(|pass| pass == "IdentityElimination"),
    "fixture should exercise IdentityElimination pass"
  );
}
