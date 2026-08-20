//! Gate test: Type invariants theorem smoke.
//!
//! Goal: graph/type invariants are checked and preservation checks stay stable.

use pnix_core::ct::ctast::verify_preservation;
use pnix_core::ct::{CTMorphismOp, CTNode, CTType, CTAST};
use pnix_core::frp::{CtLawViolation, MorphismGraph, MorphismNodeKind};

#[test]
fn gate_type_invariants_valid_graph_has_no_ct_law_violations() {
  let mut graph = MorphismGraph::new("type_inv_valid");
  let input = graph.add_node("input", MorphismNodeKind::Input);
  let derived = graph.add_node("derived", MorphismNodeKind::Derived);
  graph.add_edge(input, derived);

  let result = graph.validate_ct_laws();
  assert!(result.is_valid, "valid graph should pass ct law validation");
  assert!(
    result.violations.is_empty(),
    "valid graph should not report invariants violations"
  );
}

#[test]
fn gate_type_invariants_detect_missing_input_port_type_violation() {
  let mut graph = MorphismGraph::new("type_inv_missing_port");
  let a = graph.add_node("A", MorphismNodeKind::Derived);
  let b = graph.add_node("B", MorphismNodeKind::Derived);
  let c = graph.add_node("C", MorphismNodeKind::Derived);

  // B has one input port by default. The second incoming edge must trigger a
  // missing input port type invariant violation.
  graph.add_edge(a, b);
  graph.add_edge(c, b);

  let result = graph.validate_ct_laws();
  assert!(
    !result.is_valid,
    "invalid graph must fail ct law validation"
  );
  assert!(result.violations.iter().any(|v| {
    matches!(
      v,
      CtLawViolation::MissingInputPortType {
        node_id,
        port_index
      } if *node_id == b && *port_index == 1
    )
  }));
}

#[test]
fn gate_type_invariants_preservation_check_reports_preserved_for_same_ctast() {
  let mut ctast = CTAST::new();
  ctast.nodes.push(CTNode::new(
    "sin",
    CTMorphismOp::Sin,
    CTType::real(),
    CTType::real(),
    vec![],
  ));

  let preservation = verify_preservation(&ctast, &ctast);
  assert!(
    preservation.is_preserved(),
    "same CTAST should preserve invariants, got: {}",
    preservation.summary()
  );
}
