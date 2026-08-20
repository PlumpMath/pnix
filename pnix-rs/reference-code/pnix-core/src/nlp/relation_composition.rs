//! Relation composition carrier — host mirror for
//! `stdlib/lib/gate/reasoning/relation-composition.px`.
//!
//! # OWNER-LAW (2026-05-11)
//!
//! `relation-composition.px` is the source of truth for which 2-hop
//! chains compose into a derived predicate. This Rust module is a
//! **carrier**: it mirrors the rule table so the internal-cognition
//! path can promote `TwoHopChain` instances into typed `DerivedTriple`
//! values without round-tripping through evaluator runtime.
//!
//! **Derived facts are always Candidate**, never Accepted. Even when
//! both bridging 1-hop facts are Accepted, composition does not
//! transfer Accepted status — that requires a separate owner-law
//! proof slice. The owner-law file is referenced in provenance so
//! audit trails land on `.px`.

/// Result of composing a 2-hop chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositionVerdict {
  /// A composition rule matched. The derived predicate is supplied.
  Composed { composed_pred: String },
  /// No rule matched for this (pred1, pred2) pair.
  NotApplicable,
  /// Chain was malformed or self-cycle.
  Held { held_kind: &'static str },
}

/// Owner-law reference used in EvidenceFact / DerivedTriple provenance.
pub const OWNER_LAW: &str = "stdlib/lib/gate/reasoning/relation-composition.px";

/// Look up a composition rule. Mirrors `composedFor pred1 pred2` in the
/// `.px` owner — exact-match table plus the equality-collapse rule
/// `<p> + equal → <p>`.
fn composed_for(pred1: &str, pred2: &str) -> Option<String> {
  // Order matches `relation-composition.px::rules`. Keep both lists in
  // sync — when the `.px` owner adds a rule, this table must follow.
  let table: &[(&str, &str, &str)] = &[
    ("has-mass", "less-than", "mass-less-than"),
    ("has-mass", "greater-than", "mass-greater-than"),
    ("has-size", "greater-than", "size-greater-than"),
    ("has-size", "less-than", "size-less-than"),
    ("kind-of", "property", "inherited-property"),
    ("is-defined-as", "property", "definition-derived-property"),
    ("causes", "causes", "causal-chain-candidate"),
  ];
  for (p1, p2, composed) in table {
    if *p1 == pred1 && *p2 == pred2 {
      return Some((*composed).to_string());
    }
  }
  // Equality collapse: `<p> + equal → <p>` (mirrors equalityCollapse in
  // the `.px` owner).
  if pred2 == "equal" {
    return Some(pred1.to_string());
  }
  None
}

/// Compose a 2-hop chain into a derived predicate.
///
/// `subj --pred1--> mid --pred2--> obj2` becomes `subj --composed--> obj2`
/// when a rule matches. Self-cycles (`subj == obj2`) are Held — the
/// derived fact would not contribute new information.
pub fn compose_two_hop(
  subj: &str,
  pred1: &str,
  mid: &str,
  pred2: &str,
  obj2: &str,
) -> CompositionVerdict {
  if subj.is_empty() || pred1.is_empty() || mid.is_empty() || pred2.is_empty() || obj2.is_empty() {
    return CompositionVerdict::Held {
      held_kind: "missing-chain-field",
    };
  }
  if subj == obj2 {
    return CompositionVerdict::Held {
      held_kind: "self-cycle",
    };
  }
  match composed_for(pred1, pred2) {
    Some(composed_pred) => CompositionVerdict::Composed { composed_pred },
    None => CompositionVerdict::NotApplicable,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn composes_mass_less_than() {
    match compose_two_hop("electron", "has-mass", "small", "less-than", "proton") {
      CompositionVerdict::Composed { composed_pred } => {
        assert_eq!(composed_pred, "mass-less-than");
      }
      other => panic!("expected Composed, got {:?}", other),
    }
  }

  #[test]
  fn composes_kind_of_property_to_inherited_property() {
    match compose_two_hop("sparrow", "kind-of", "bird", "property", "can-fly") {
      CompositionVerdict::Composed { composed_pred } => {
        assert_eq!(composed_pred, "inherited-property");
      }
      other => panic!("expected Composed, got {:?}", other),
    }
  }

  #[test]
  fn composes_causes_causes_into_causal_chain_candidate() {
    match compose_two_hop("smoking", "causes", "tar-buildup", "causes", "cancer") {
      CompositionVerdict::Composed { composed_pred } => {
        assert_eq!(composed_pred, "causal-chain-candidate");
      }
      other => panic!("expected Composed, got {:?}", other),
    }
  }

  #[test]
  fn equality_collapse_returns_pred1() {
    // <p> + equal → <p>
    match compose_two_hop("a", "greater-than", "b", "equal", "c") {
      CompositionVerdict::Composed { composed_pred } => {
        assert_eq!(composed_pred, "greater-than");
      }
      other => panic!("expected Composed, got {:?}", other),
    }
  }

  #[test]
  fn no_rule_returns_not_applicable() {
    match compose_two_hop("a", "linked-to", "b", "next-to", "c") {
      CompositionVerdict::NotApplicable => {}
      other => panic!("expected NotApplicable, got {:?}", other),
    }
  }

  #[test]
  fn self_cycle_is_held() {
    match compose_two_hop("a", "has-mass", "small", "less-than", "a") {
      CompositionVerdict::Held { held_kind } => {
        assert_eq!(held_kind, "self-cycle");
      }
      other => panic!("expected Held(self-cycle), got {:?}", other),
    }
  }

  #[test]
  fn missing_field_is_held() {
    match compose_two_hop("", "has-mass", "small", "less-than", "proton") {
      CompositionVerdict::Held { held_kind } => {
        assert_eq!(held_kind, "missing-chain-field");
      }
      other => panic!("expected Held(missing-chain-field), got {:?}", other),
    }
  }

  #[test]
  fn owner_law_path_points_to_px() {
    assert!(OWNER_LAW.ends_with(".px"));
    assert!(OWNER_LAW.contains("relation-composition"));
  }
}
