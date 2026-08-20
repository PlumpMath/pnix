//! Operation candidate mapping carrier.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/operation-candidate-mapping.px`.
//! Given a recognized intent + fired cues, emit a *ranked candidate
//! set* of code-transform operations. This is the bridge from intent
//! recognition (`refactor` / `cleanup` / ...) to concrete transform
//! names (`rename-symbol` / `remove-unused-import` / ...).
//!
//! What this carrier does NOT do:
//!   - fill code-transform request fields (those need parameter
//!     resolution, a future synthesis owner)
//!   - collapse to a single operation (ambiguity is preserved as
//!     ranked candidate set — §15-6 weaponization defense applied to
//!     operation choice)
//!   - emit anything for the `explain` intent (no transform needed)
//!
//! Registry-driven: adding a new (intent, cues, transform) mapping =
//! one new row in `OPERATION_MAP`, no per-cue branch.

use serde::{Deserialize, Serialize};

/// Code-transform names this carrier may emit. Stays byte-identical
/// to the `.px` `validTransforms` list and to the existing code-
/// transform owner law family.
pub const VALID_TRANSFORMS: &[&str] = &[
  "rename-symbol",
  "remove-unused-import",
  "add-test-stub",
  "add-import",
  "extract-function",
  "inline-function",
  "move-symbol",
  "change-signature",
  // Math-domain transform — substrate-sharing.
  "lookup-algebraic-equivalent",
  // Chemistry-domain transform — substrate-sharing N=3.
  "lookup-chemical-reaction",
];

/// Held kinds the carrier may emit when no operation matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationMappingHeldKind {
  NoMatchingOperation,
  IntentWithoutMapping,
  AmbiguousOperationTie,
}

impl OperationMappingHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::NoMatchingOperation,
    Self::IntentWithoutMapping,
    Self::AmbiguousOperationTie,
  ];
  pub fn as_str(self) -> &'static str {
    match self {
      Self::NoMatchingOperation => "no-matching-operation",
      Self::IntentWithoutMapping => "intent-without-mapping",
      Self::AmbiguousOperationTie => "ambiguous-operation-tie",
    }
  }
}

/// One operation-map row — pure data. Mirror of `.px` `operationMap`.
#[derive(Debug, Clone, Copy)]
pub struct OperationMapEntry {
  pub intent: &'static str,
  pub cues: &'static [&'static str],
  pub transform: &'static str,
  pub weight: f32,
}

/// Single source of truth (Rust side). Sync test asserts set parity
/// against the `.px` `operationMap` rows.
pub const OPERATION_MAP: &[OperationMapEntry] = &[
  // ── refactor ──
  OperationMapEntry {
    intent: "refactor",
    cues: &["verb:rename"],
    transform: "rename-symbol",
    weight: 0.95,
  },
  OperationMapEntry {
    intent: "refactor",
    cues: &["verb:extract"],
    transform: "extract-function",
    weight: 0.95,
  },
  OperationMapEntry {
    intent: "refactor",
    cues: &["verb:inline"],
    transform: "inline-function",
    weight: 0.95,
  },
  OperationMapEntry {
    intent: "refactor",
    cues: &["verb:move"],
    transform: "move-symbol",
    weight: 0.95,
  },
  OperationMapEntry {
    intent: "refactor",
    cues: &["verb:simplify"],
    transform: "rename-symbol",
    weight: 0.45,
  },
  OperationMapEntry {
    intent: "refactor",
    cues: &["verb:simplify"],
    transform: "extract-function",
    weight: 0.45,
  },
  OperationMapEntry {
    intent: "refactor",
    cues: &["verb:reorganize"],
    transform: "move-symbol",
    weight: 0.55,
  },
  OperationMapEntry {
    intent: "refactor",
    cues: &["metaphor:cleanliness"],
    transform: "remove-unused-import",
    weight: 0.45,
  },
  OperationMapEntry {
    intent: "refactor",
    cues: &["metaphor:cleanliness"],
    transform: "rename-symbol",
    weight: 0.30,
  },
  // ── cleanup ──
  OperationMapEntry {
    intent: "cleanup",
    cues: &["structural:unused-import"],
    transform: "remove-unused-import",
    weight: 0.95,
  },
  OperationMapEntry {
    intent: "cleanup",
    cues: &["verb:remove", "metaphor:deadweight"],
    transform: "remove-unused-import",
    weight: 0.90,
  },
  OperationMapEntry {
    intent: "cleanup",
    cues: &["verb:delete", "metaphor:deadweight"],
    transform: "remove-unused-import",
    weight: 0.90,
  },
  OperationMapEntry {
    intent: "cleanup",
    cues: &["verb:remove"],
    transform: "remove-unused-import",
    weight: 0.55,
  },
  OperationMapEntry {
    intent: "cleanup",
    cues: &["metaphor:deadweight"],
    transform: "remove-unused-import",
    weight: 0.65,
  },
  // ── add-feature ──
  OperationMapEntry {
    intent: "add-feature",
    cues: &["verb:add", "metaphor:new-capability"],
    transform: "add-import",
    weight: 0.50,
  },
  OperationMapEntry {
    intent: "add-feature",
    cues: &["verb:implement"],
    transform: "add-import",
    weight: 0.45,
  },
  OperationMapEntry {
    intent: "add-feature",
    cues: &["verb:introduce"],
    transform: "add-import",
    weight: 0.40,
  },
  OperationMapEntry {
    intent: "add-feature",
    cues: &["verb:add", "time:conditional"],
    transform: "change-signature",
    weight: 0.45,
  },
  OperationMapEntry {
    intent: "add-feature",
    cues: &["verb:add"],
    transform: "add-import",
    weight: 0.35,
  },
  // ── test ──
  OperationMapEntry {
    intent: "test",
    cues: &["verb:test"],
    transform: "add-test-stub",
    weight: 0.95,
  },
  OperationMapEntry {
    intent: "test",
    cues: &["verb:cover"],
    transform: "add-test-stub",
    weight: 0.80,
  },
  OperationMapEntry {
    intent: "test",
    cues: &["metaphor:coverage"],
    transform: "add-test-stub",
    weight: 0.80,
  },
  OperationMapEntry {
    intent: "test",
    cues: &["structural:test-file"],
    transform: "add-test-stub",
    weight: 0.70,
  },
  // ── fix-bug ──
  // First first-class fix-bug → typed-transform mapping:
  // missing-import diagnostic → add-import.
  OperationMapEntry {
    intent: "fix-bug",
    cues: &["fact:missing-import"],
    transform: "add-import",
    weight: 0.95,
  },
  OperationMapEntry {
    intent: "fix-bug",
    cues: &["time:causal"],
    transform: "rename-symbol",
    weight: 0.40,
  },
  OperationMapEntry {
    intent: "fix-bug",
    cues: &["time:causal"],
    transform: "change-signature",
    weight: 0.40,
  },
  OperationMapEntry {
    intent: "fix-bug",
    cues: &["metaphor:brokenness"],
    transform: "change-signature",
    weight: 0.40,
  },
  OperationMapEntry {
    intent: "fix-bug",
    cues: &["metaphor:wrongness"],
    transform: "rename-symbol",
    weight: 0.35,
  },
  // ── optimize ──
  OperationMapEntry {
    intent: "optimize",
    cues: &["metaphor:speed"],
    transform: "inline-function",
    weight: 0.45,
  },
  OperationMapEntry {
    intent: "optimize",
    cues: &["metaphor:speed"],
    transform: "extract-function",
    weight: 0.35,
  },
  OperationMapEntry {
    intent: "optimize",
    cues: &["verb:optimize"],
    transform: "inline-function",
    weight: 0.45,
  },
  OperationMapEntry {
    intent: "optimize",
    cues: &["verb:optimize"],
    transform: "extract-function",
    weight: 0.35,
  },
  // ── explain (math) ──
  // First explain-intent operation mapping: math-question cue →
  // `lookup-algebraic-equivalent`. Substrate-sharing proof — same
  // operation-map mechanism that handles refactor/cleanup/fix-bug
  // routes a non-coding intent to a non-coding transform.
  OperationMapEntry {
    intent: "explain",
    cues: &["fact:math-question"],
    transform: "lookup-algebraic-equivalent",
    weight: 0.95,
  },
  // ── explain (chemistry) — substrate-sharing N=3.
  OperationMapEntry {
    intent: "explain",
    cues: &["fact:chemistry-question"],
    transform: "lookup-chemical-reaction",
    weight: 0.95,
  },
];

/// One ranked operation candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationCandidate {
  /// Code-transform name (member of `VALID_TRANSFORMS`).
  pub transform: String,
  /// Confidence — sum of `weight`s for all matching rows merged
  /// by `(transform)`.
  pub confidence: f32,
  /// The cue lists that contributed to this candidate's confidence.
  /// Provenance: every entry is one of the `cues` arrays from a
  /// matching `OPERATION_MAP` row.
  pub firing_cue_sets: Vec<Vec<String>>,
  /// Source intent this candidate was derived from. For multi-intent
  /// inputs the caller invokes `classify_operation_candidates` once
  /// per ranked intent and merges results.
  pub source_intent: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum OperationMappingVerdict {
  OperationMappingReady {
    ranked_operations: Vec<OperationCandidate>,
  },
  OperationMappingHeld {
    held_kind: OperationMappingHeldKind,
    reason: String,
    ranked_operations: Vec<OperationCandidate>,
  },
}

fn row_fires(fired: &[String], row_cues: &[&str]) -> bool {
  row_cues.iter().all(|c| fired.iter().any(|f| f == c))
}

/// Classify operation candidates for a single intent + fired cues.
///
/// Behavior:
///   - For each `OPERATION_MAP` row matching `(intent, fired_cues
///     subset)`, contribute its `weight` to the candidate transform.
///   - Multiple matching rows merge their weights into one candidate
///     per `transform`, preserving every matching `cues` array as
///     provenance in `firing_cue_sets`.
///   - Held if zero candidates (no-matching-operation when at least
///     one cue fired, intent-without-mapping if no rows even exist
///     for the intent).
///   - Ambiguous-operation-tie when top two candidates are within
///     `TIE_EPSILON` (matching `intent-recognition`'s threshold).
pub const TIE_EPSILON: f32 = 0.10;

pub fn classify_operation_candidates(
  intent: &str,
  fired_signals: &[String],
) -> OperationMappingVerdict {
  // Aggregate by transform.
  let mut by_transform: std::collections::BTreeMap<String, OperationCandidate> =
    std::collections::BTreeMap::new();
  let mut intent_had_rows = false;
  for row in OPERATION_MAP {
    if row.intent != intent {
      continue;
    }
    intent_had_rows = true;
    if !row_fires(fired_signals, row.cues) {
      continue;
    }
    let cue_set: Vec<String> = row.cues.iter().map(|c| c.to_string()).collect();
    by_transform
      .entry(row.transform.to_string())
      .and_modify(|c| {
        c.confidence += row.weight;
        c.firing_cue_sets.push(cue_set.clone());
      })
      .or_insert(OperationCandidate {
        transform: row.transform.to_string(),
        confidence: row.weight,
        firing_cue_sets: vec![cue_set],
        source_intent: intent.to_string(),
      });
  }

  let mut ranked: Vec<OperationCandidate> = by_transform.into_values().collect();
  ranked.sort_by(|a, b| {
    b.confidence
      .partial_cmp(&a.confidence)
      .unwrap_or(std::cmp::Ordering::Equal)
      .then_with(|| a.transform.cmp(&b.transform))
  });

  if ranked.is_empty() {
    let (kind, reason) = if !intent_had_rows {
      (
        OperationMappingHeldKind::IntentWithoutMapping,
        format!(
          "intent `{intent}` has no rows in operation-candidate-mapping (e.g. `explain` emits no code transform)"
        ),
      )
    } else {
      (
        OperationMappingHeldKind::NoMatchingOperation,
        format!(
          "intent `{intent}` had rows but no fired cue combination matched any; operator can resubmit with more specific cues"
        ),
      )
    };
    return OperationMappingVerdict::OperationMappingHeld {
      held_kind: kind,
      reason,
      ranked_operations: ranked,
    };
  }
  // Ambiguous tie at the top?
  if ranked.len() >= 2 && (ranked[0].confidence - ranked[1].confidence) < TIE_EPSILON {
    return OperationMappingVerdict::OperationMappingHeld {
      held_kind: OperationMappingHeldKind::AmbiguousOperationTie,
      reason: format!(
        "top two operation candidates within tieEpsilon ({TIE_EPSILON}); ambiguity preserved — operator clarification needed"
      ),
      ranked_operations: ranked,
    };
  }
  OperationMappingVerdict::OperationMappingReady {
    ranked_operations: ranked,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn fired(cues: &[&str]) -> Vec<String> {
    cues.iter().map(|s| s.to_string()).collect()
  }

  #[test]
  fn registry_invariants_hold() {
    for row in OPERATION_MAP {
      assert!(
        VALID_TRANSFORMS.contains(&row.transform),
        "transform `{}` not in VALID_TRANSFORMS",
        row.transform
      );
      assert!(
        (0.0..=1.0).contains(&row.weight),
        "weight out of range: {}",
        row.weight
      );
      assert!(!row.intent.is_empty());
    }
  }

  #[test]
  fn refactor_with_rename_cue_picks_rename_symbol() {
    let v = classify_operation_candidates("refactor", &fired(&["verb:rename"]));
    match v {
      OperationMappingVerdict::OperationMappingReady { ranked_operations } => {
        assert_eq!(ranked_operations[0].transform, "rename-symbol");
        assert!(ranked_operations[0].confidence >= 0.9);
      }
      other => panic!("expected Ready rename-symbol, got {other:?}"),
    }
  }

  #[test]
  fn cleanup_with_unused_import_structural_picks_remove_unused_import() {
    let v = classify_operation_candidates("cleanup", &fired(&["structural:unused-import"]));
    match v {
      OperationMappingVerdict::OperationMappingReady { ranked_operations } => {
        assert_eq!(ranked_operations[0].transform, "remove-unused-import");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn cleanup_with_remove_and_deadweight_picks_remove_unused_import_high_conf() {
    let v =
      classify_operation_candidates("cleanup", &fired(&["verb:remove", "metaphor:deadweight"]));
    match v {
      OperationMappingVerdict::OperationMappingReady { ranked_operations } => {
        // 0.90 (joint row) + 0.55 (verb:remove alone row) + 0.65
        // (metaphor:deadweight alone) = 2.10. Single transform.
        assert_eq!(ranked_operations[0].transform, "remove-unused-import");
        assert!(ranked_operations[0].confidence > 1.5);
        // Three rows contributed; firing_cue_sets has 3 entries.
        assert_eq!(ranked_operations[0].firing_cue_sets.len(), 3);
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn test_intent_with_verb_test_picks_add_test_stub() {
    let v = classify_operation_candidates("test", &fired(&["verb:test"]));
    match v {
      OperationMappingVerdict::OperationMappingReady { ranked_operations } => {
        assert_eq!(ranked_operations[0].transform, "add-test-stub");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn optimize_intent_with_speed_holds_on_inline_vs_extract_tie() {
    // optimize+metaphor:speed fires both inline-function (0.45) and
    // extract-function (0.35). Gap is ~0.10 (float-precision-wise
    // slightly under TIE_EPSILON), so the tie-guard fires. This is
    // *correct* — both are reasonable optimization candidates and
    // the ambiguity should be surfaced (the operator picks).
    let v = classify_operation_candidates("optimize", &fired(&["metaphor:speed"]));
    match v {
      OperationMappingVerdict::OperationMappingHeld {
        held_kind: OperationMappingHeldKind::AmbiguousOperationTie,
        ranked_operations,
        ..
      } => {
        // Both candidates are surfaced even when Held.
        let names: Vec<&str> = ranked_operations
          .iter()
          .map(|r| r.transform.as_str())
          .collect();
        assert!(names.contains(&"inline-function"));
        assert!(names.contains(&"extract-function"));
      }
      other => panic!("expected AmbiguousOperationTie Held, got {other:?}"),
    }
  }

  #[test]
  fn explain_intent_with_non_math_cues_gets_no_matching_operation() {
    // Updated 2026-05-12: explain intent now has one mapping row
    // (`fact:math-question` → `lookup-algebraic-equivalent`).
    // Generic explain cues (verb:explain / time:interrogative) still
    // produce a Held, but it's `NoMatchingOperation` (the cue set
    // doesn't satisfy any of explain's rows), not
    // `IntentWithoutMapping` (no rows at all for the intent).
    let v =
      classify_operation_candidates("explain", &fired(&["verb:explain", "time:interrogative"]));
    assert!(matches!(
      v,
      OperationMappingVerdict::OperationMappingHeld {
        held_kind: OperationMappingHeldKind::NoMatchingOperation,
        ..
      }
    ));
  }

  #[test]
  fn explain_intent_with_math_question_cue_routes_to_lookup_algebraic_equivalent() {
    // Substrate-sharing closure: the same classifier that routes
    // refactor/cleanup/fix-bug intents to their code-transforms
    // routes explain-intent + math-question cue to the math
    // transform.
    let v = classify_operation_candidates("explain", &fired(&["fact:math-question"]));
    match v {
      OperationMappingVerdict::OperationMappingReady { ranked_operations } => {
        assert!(
          ranked_operations
            .iter()
            .any(|c| c.transform == "lookup-algebraic-equivalent"),
          "expected lookup-algebraic-equivalent in ranked operations, got {:?}",
          ranked_operations
            .iter()
            .map(|c| &c.transform)
            .collect::<Vec<_>>()
        );
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn unmatched_cues_for_known_intent_gets_no_matching_operation() {
    // `cleanup` is in the registry but only one of its rows matches
    // on a cue we didn't fire.
    let v = classify_operation_candidates("cleanup", &fired(&["verb:rename"]));
    assert!(matches!(
      v,
      OperationMappingVerdict::OperationMappingHeld {
        held_kind: OperationMappingHeldKind::NoMatchingOperation,
        ..
      }
    ));
  }

  #[test]
  fn cue_subset_match_only_requires_all_row_cues_present() {
    // Row needs ["verb:remove", "metaphor:deadweight"]. Firing more
    // cues (extras) should still match.
    let v = classify_operation_candidates(
      "cleanup",
      &fired(&[
        "verb:remove",
        "metaphor:deadweight",
        "time:imperative", // extra
      ]),
    );
    assert!(matches!(
      v,
      OperationMappingVerdict::OperationMappingReady { .. }
    ));
  }

  #[test]
  fn ranked_operations_carry_source_intent_for_audit() {
    let v = classify_operation_candidates("refactor", &fired(&["verb:rename"]));
    match v {
      OperationMappingVerdict::OperationMappingReady { ranked_operations } => {
        assert_eq!(ranked_operations[0].source_intent, "refactor");
      }
      _ => panic!("expected Ready"),
    }
  }

  #[test]
  fn add_feature_with_add_and_conditional_holds_change_signature_vs_add_import_tie() {
    // verb:add + time:conditional jointly fire change-signature row
    // (0.45). verb:add alone fires add-import row (0.35). Gap ~0.10
    // → AmbiguousOperationTie Held. Both are legitimate candidates
    // for "add a parameter under condition" requests; operator picks.
    let v = classify_operation_candidates("add-feature", &fired(&["verb:add", "time:conditional"]));
    match v {
      OperationMappingVerdict::OperationMappingHeld {
        held_kind: OperationMappingHeldKind::AmbiguousOperationTie,
        ranked_operations,
        ..
      } => {
        let names: Vec<&str> = ranked_operations
          .iter()
          .map(|r| r.transform.as_str())
          .collect();
        assert!(names.contains(&"change-signature"));
        assert!(names.contains(&"add-import"));
      }
      other => panic!("expected AmbiguousOperationTie Held, got {other:?}"),
    }
  }
}
