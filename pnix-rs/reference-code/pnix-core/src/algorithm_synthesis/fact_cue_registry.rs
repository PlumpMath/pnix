//! Fact cue extractor.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/fact-cue-registry.px`.
//! Extracts `fact:*` cues from declarative NL phrases AND typed
//! passthrough from an upstream relation-extraction owner. The cues
//! describe *observed predicates* about the code or runtime — they
//! seed intent recognition without claiming to be proofs.
//!
//! Two cue sources:
//!
//!   1. NL-phrase — substring match on declarative phrases. KO + EN.
//!      e.g. "이 함수 느려" → `fact:slow-path`.
//!   2. Typed passthrough — caller supplies an already-extracted
//!      fact predicate (e.g. from doghouse retrieval) and it
//!      flows into the cue universe verbatim.
//!
//! Crude by design — same discipline as `structural_cue_registry`.
//! Fact cues only *fire intent*. Parameter-resolution still demands
//! real host evidence for actual transform requests.

/// Recognized fact cue names. Sync test asserts set parity against
/// `.px` `validFactCues`.
pub const FACT_CUES: &[&str] = &[
  "fact:contradicted",
  "fact:slow-path",
  "fact:definition",
  "fact:causal-relation",
  "fact:missing-case",
  "fact:missing-import",
  // Math-domain fact cue. Fires when the utterance carries a
  // recognizable math expression AND a "what's the equivalent?"
  // question shape. Used by intent-recognition to route to
  // `lookup-algebraic-equivalent`. Substrate-sharing proof: same
  // cue → intent → operation pipeline that handles coding lanes.
  "fact:math-question",
  // Chemistry-domain fact cue. Fires when the utterance carries
  // chemistry reactant formula tokens AND a reaction-question
  // shape. Substrate-sharing N=3.
  "fact:chemistry-question",
  // Self-extension request. Fires when the utterance asks pnix to
  // learn or extend itself across language / math / coding domains.
  // The first production learned-intent overlay row
  // (`stdlib/lib/gate/algorithm-synthesis/learned-intent-overlay.px`)
  // maps this cue to the `add-feature` intent.
  "fact:language-math-code-learning-request",
];

/// Cues whose source-of-truth is a *structural* extractor (not a
/// phrase-pattern row). The registry-consistency sync test skips
/// these when checking FACT_PHRASE_PATTERNS coverage.
pub const STRUCTURAL_FACT_CUES: &[&str] = &[
  // Math-question detection delegates to
  // `parameter_resolution::extract_math_canonical_form` — the same
  // extractor the lift step uses, so cue firing and resolution stay
  // in lock-step. No phrase pattern row.
  "fact:math-question",
  // Chemistry-question detection delegates to
  // `parameter_resolution::extract_chemistry_canonical_form` — same
  // single-source-of-truth pattern.
  "fact:chemistry-question",
];

/// NL phrase pattern row. `markers` are substring match against
/// lowercased utterance; `cue` is the fact cue that fires when any
/// marker is found.
#[derive(Debug, Clone, Copy)]
pub struct FactPhrasePattern {
  pub cue: &'static str,
  pub markers: &'static [&'static str],
}

pub const FACT_PHRASE_PATTERNS: &[FactPhrasePattern] = &[
  FactPhrasePattern {
    cue: "fact:contradicted",
    markers: &[
      "contradicted",
      "contradict",
      "모순",
      "안 맞",
      "안맞",
      "맞지 않",
    ],
  },
  FactPhrasePattern {
    cue: "fact:slow-path",
    markers: &[
      "느려",
      "느림",
      "느린",
      "병목",
      "지연",
      "slow path",
      "slow-path",
      "is slow",
      "too slow",
    ],
  },
  FactPhrasePattern {
    cue: "fact:definition",
    markers: &[
      "정의",
      "뜻이",
      "이란",
      "이 뭐",
      "가 뭐",
      "definition of",
      "what is ",
      "meaning of",
    ],
  },
  FactPhrasePattern {
    cue: "fact:causal-relation",
    markers: &[
      "때문에",
      "이유로",
      "원인으로",
      "인해",
      "caused by",
      "due to",
      "because of",
    ],
  },
  FactPhrasePattern {
    cue: "fact:missing-case",
    markers: &[
      "빠진 케이스",
      "빠진케이스",
      "안 다루",
      "안다루",
      "edge case 없",
      "missing case",
      "uncovered case",
      "not handled",
    ],
  },
  FactPhrasePattern {
    cue: "fact:missing-import",
    markers: &[
      "is not defined",
      "nameerror",
      "name 'unresolved",
      "cannot find name",
      "unresolved import",
      "could not find",
      "정의되지 않",
      "import 누락",
      "임포트 누락",
      "import 없",
    ],
  },
  FactPhrasePattern {
    cue: "fact:language-math-code-learning-request",
    markers: &[
      // KO — substrate-extension requests in Korean.
      "한국어 영어 수학 코딩",
      "한국어와 영어와 수학과 코딩",
      "언어 수학 코딩",
      "수학과 코딩을 배워",
      "수학과 코딩 배워",
      "언어와 수학과 코딩을 학습",
      "스스로 배우",
      "지능을 확장",
      "기능을 확장",
      // EN — same intent in English.
      "learn languages math and coding",
      "language math and coding",
      "learn language math code",
      "extend yourself",
      "extend your intelligence",
      "self-extend",
      "language-math-code-learning",
    ],
  },
];

/// Extract NL-phrase fact cues. Empty input → empty result. Lowered
/// for case-insensitive English matching (Korean markers are
/// case-insensitive by nature).
///
/// Two pattern families:
///   - `FACT_PHRASE_PATTERNS` — substring-based, the bulk of cues.
///   - `fact:math-question` — structural: fires when the utterance
///     contains a recognizable math expression (delegates to
///     `parameter_resolution::extract_math_canonical_form`). Pure
///     substring matching can't capture "math operator AND question
///     shape" conjunctively, so this cue uses the structural
///     extractor as its single source of truth.
pub fn extract_phrase_signals(utterance: &str) -> Vec<String> {
  if utterance.is_empty() {
    return Vec::new();
  }
  let lowered = utterance.to_lowercase();
  let mut out: Vec<String> = Vec::new();
  for row in FACT_PHRASE_PATTERNS {
    let fired = row.markers.iter().any(|m| {
      // ASCII markers match against lowered; non-ASCII (Korean)
      // markers match against original utterance.
      if m.is_ascii() {
        lowered.contains(m)
      } else {
        utterance.contains(m)
      }
    });
    if fired && !out.iter().any(|x| x == row.cue) {
      out.push(row.cue.to_string());
    }
  }
  // Structural math-question detection — single source of truth is
  // the same extractor that `resolve_lookup_algebraic_equivalent`
  // uses. Keeps cue firing and lift in lock-step.
  if super::parameter_resolution::extract_math_canonical_form(utterance).is_some() {
    let cue = "fact:math-question".to_string();
    if !out.iter().any(|x| x == &cue) {
      out.push(cue);
    }
  }
  // Structural chemistry-question detection — same single-source
  // pattern. Both math and chemistry cues fire structurally so
  // intent classifier routes both lanes off NL alone.
  if super::parameter_resolution::extract_chemistry_canonical_form(utterance).is_some() {
    let cue = "fact:chemistry-question".to_string();
    if !out.iter().any(|x| x == &cue) {
      out.push(cue);
    }
  }
  out
}

/// Typed passthrough: filter a caller-supplied fact-cue list against
/// the registered cue universe. Unknown cues are dropped silently —
/// not an error, just ignored. Order preserved, duplicates removed.
pub fn pass_through_supplied_facts(supplied: &[String]) -> Vec<String> {
  let mut out: Vec<String> = Vec::new();
  for cue in supplied {
    if FACT_CUES.iter().any(|c| c == cue) && !out.iter().any(|x| x == cue) {
      out.push(cue.clone());
    }
  }
  out
}

/// Combine NL extraction + typed passthrough. Order: NL first, then
/// typed; duplicates removed. Used by the bridge as the default fact
/// extraction path.
pub fn extract_fact_signals(utterance: &str, supplied: &[String]) -> Vec<String> {
  let mut out = extract_phrase_signals(utterance);
  for cue in pass_through_supplied_facts(supplied) {
    if !out.iter().any(|x| x == &cue) {
      out.push(cue);
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  // ─── NL phrase extraction ─────────────────────────────────────

  #[test]
  fn empty_utterance_returns_empty() {
    assert!(extract_phrase_signals("").is_empty());
  }

  #[test]
  fn korean_slow_path_fires() {
    let s = extract_phrase_signals("이 함수 느려서 답답해");
    assert!(s.iter().any(|c| c == "fact:slow-path"), "got {s:?}");
  }

  #[test]
  fn english_slow_path_fires() {
    let s = extract_phrase_signals("This loop is slow");
    assert!(s.iter().any(|c| c == "fact:slow-path"), "got {s:?}");
  }

  #[test]
  fn korean_contradicted_fires() {
    let s = extract_phrase_signals("결과가 기대값이랑 안 맞아");
    assert!(s.iter().any(|c| c == "fact:contradicted"), "got {s:?}");
  }

  #[test]
  fn korean_causal_fires() {
    let s = extract_phrase_signals("타임아웃 때문에 실패함");
    assert!(s.iter().any(|c| c == "fact:causal-relation"), "got {s:?}");
  }

  #[test]
  fn english_causal_fires() {
    let s = extract_phrase_signals("Test failed due to race condition");
    assert!(s.iter().any(|c| c == "fact:causal-relation"), "got {s:?}");
  }

  #[test]
  fn korean_definition_fires() {
    let s = extract_phrase_signals("이 함수의 정의가 뭐였지");
    assert!(s.iter().any(|c| c == "fact:definition"), "got {s:?}");
  }

  #[test]
  fn english_definition_fires() {
    let s = extract_phrase_signals("what is a closure");
    assert!(s.iter().any(|c| c == "fact:definition"), "got {s:?}");
  }

  #[test]
  fn korean_missing_case_fires() {
    let s = extract_phrase_signals("빈 입력 케이스 안 다루고 있어");
    assert!(s.iter().any(|c| c == "fact:missing-case"), "got {s:?}");
  }

  #[test]
  fn english_missing_case_fires() {
    let s = extract_phrase_signals("empty input is not handled here");
    assert!(s.iter().any(|c| c == "fact:missing-case"), "got {s:?}");
  }

  #[test]
  fn english_missing_import_fires_from_nameerror() {
    let s = extract_phrase_signals("NameError: name 'os' is not defined");
    assert!(s.iter().any(|c| c == "fact:missing-import"), "got {s:?}");
  }

  #[test]
  fn korean_missing_import_fires() {
    let s = extract_phrase_signals("import 누락된 것 같아");
    assert!(s.iter().any(|c| c == "fact:missing-import"), "got {s:?}");
  }

  #[test]
  fn neutral_utterance_fires_nothing() {
    let s = extract_phrase_signals("그냥 코드 보여줘");
    assert!(s.is_empty(), "got {s:?}");
  }

  #[test]
  fn multiple_facts_in_one_utterance_all_fire() {
    let s = extract_phrase_signals("이 함수 느려서 결과가 안 맞아");
    assert!(s.iter().any(|c| c == "fact:slow-path"));
    assert!(s.iter().any(|c| c == "fact:contradicted"));
  }

  #[test]
  fn phrase_signals_deduplicate_within_one_cue() {
    // Two slow-path markers in one utterance still emit the cue once.
    let s = extract_phrase_signals("느린 path 라서 너무 slow 합니다 느림");
    let count = s.iter().filter(|c| *c == "fact:slow-path").count();
    assert_eq!(count, 1, "got {s:?}");
  }

  // ─── typed passthrough ────────────────────────────────────────

  #[test]
  fn passthrough_keeps_only_registered_cues() {
    let supplied = vec![
      "fact:contradicted".to_string(),
      "fact:not-real".to_string(),
      "verb:rename".to_string(), // wrong family
      "fact:slow-path".to_string(),
    ];
    let out = pass_through_supplied_facts(&supplied);
    assert_eq!(out, vec!["fact:contradicted", "fact:slow-path"]);
  }

  #[test]
  fn passthrough_dedupes() {
    let supplied = vec!["fact:slow-path".to_string(), "fact:slow-path".to_string()];
    let out = pass_through_supplied_facts(&supplied);
    assert_eq!(out.len(), 1);
  }

  #[test]
  fn passthrough_empty_returns_empty() {
    let out = pass_through_supplied_facts(&[]);
    assert!(out.is_empty());
  }

  // ─── combined extraction ──────────────────────────────────────

  #[test]
  fn combined_dedupes_across_sources() {
    // NL fires `fact:slow-path`; supplied also has it. Result has
    // it once.
    let supplied = vec!["fact:slow-path".to_string()];
    let out = extract_fact_signals("이 path is slow", &supplied);
    let count = out.iter().filter(|c| *c == "fact:slow-path").count();
    assert_eq!(count, 1, "got {out:?}");
  }

  #[test]
  fn combined_carries_both_sources_when_different() {
    let supplied = vec!["fact:definition".to_string()];
    let out = extract_fact_signals("이 함수 느려", &supplied);
    assert!(out.iter().any(|c| c == "fact:slow-path"));
    assert!(out.iter().any(|c| c == "fact:definition"));
  }

  // ─── registry consistency ─────────────────────────────────────

  #[test]
  fn every_cue_in_registry_has_at_least_one_phrase_pattern() {
    for cue in FACT_CUES {
      // Structural cues (e.g. `fact:math-question`) are extracted
      // via a structural detector, not a phrase row. Sync test
      // skips them — the structural extractor coverage is asserted
      // separately by the per-cue NL tests.
      if STRUCTURAL_FACT_CUES.iter().any(|c| c == cue) {
        continue;
      }
      let has_row = FACT_PHRASE_PATTERNS.iter().any(|r| &r.cue == cue);
      assert!(has_row, "fact cue `{cue}` has no phrase pattern row");
    }
  }

  #[test]
  fn every_phrase_pattern_cue_is_in_fact_cues() {
    for row in FACT_PHRASE_PATTERNS {
      let registered = FACT_CUES.iter().any(|c| c == &row.cue);
      assert!(registered, "phrase row cue `{}` not in FACT_CUES", row.cue);
    }
  }

  #[test]
  fn no_duplicate_cues_in_phrase_patterns() {
    let mut seen = std::collections::HashSet::new();
    for row in FACT_PHRASE_PATTERNS {
      assert!(seen.insert(row.cue), "duplicate row for cue `{}`", row.cue);
    }
  }

  // ─── structural cue: math-question ────────────────────────────

  #[test]
  fn math_question_cue_fires_on_korean_polynomial_utterance() {
    let cues = extract_phrase_signals("x^2 + 2*x*y + y^2 는 뭐야?");
    assert!(
      cues.iter().any(|c| c == "fact:math-question"),
      "expected fact:math-question, got {cues:?}"
    );
  }

  #[test]
  fn math_question_cue_fires_on_boolean_algebra_utterance() {
    let cues = extract_phrase_signals("(p ∧ q) ∨ (p ∧ r) 는 뭐야?");
    assert!(
      cues.iter().any(|c| c == "fact:math-question"),
      "expected fact:math-question, got {cues:?}"
    );
  }

  #[test]
  fn math_question_cue_does_not_fire_on_pure_prose() {
    let cues = extract_phrase_signals("rename foo to bar in src/a.py");
    assert!(
      !cues.iter().any(|c| c == "fact:math-question"),
      "fact:math-question must NOT fire on prose, got {cues:?}"
    );
  }

  #[test]
  fn math_question_cue_does_not_fire_on_definition_question_without_expression() {
    // "이게 뭐야?" is a definition question, not math.
    let cues = extract_phrase_signals("이게 뭐야?");
    assert!(
      !cues.iter().any(|c| c == "fact:math-question"),
      "fact:math-question must NOT fire on non-math 뭐야, got {cues:?}"
    );
  }

  #[test]
  fn chemistry_question_cue_fires_on_korean_reaction_utterance() {
    let cues = extract_phrase_signals("2 H2 + O2 가 어떻게 반응해?");
    assert!(
      cues.iter().any(|c| c == "fact:chemistry-question"),
      "fact:chemistry-question must fire, got {cues:?}"
    );
  }

  #[test]
  fn chemistry_question_cue_does_not_fire_on_math_utterance() {
    // Math has no chemistry formula tokens → chemistry cue must
    // not fire. (math cue fires, but that's separate.)
    let cues = extract_phrase_signals("x^2 + 2*x*y + y^2 는 뭐야?");
    assert!(
      !cues.iter().any(|c| c == "fact:chemistry-question"),
      "fact:chemistry-question must NOT fire on math, got {cues:?}"
    );
    // Math cue still fires.
    assert!(cues.iter().any(|c| c == "fact:math-question"));
  }

  #[test]
  fn chemistry_question_cue_does_not_fire_on_pure_prose() {
    let cues = extract_phrase_signals("rename foo to bar in src/a.py");
    assert!(!cues.iter().any(|c| c == "fact:chemistry-question"));
  }

  #[test]
  fn structural_fact_cues_are_in_fact_cues() {
    // Sync: every structural cue must appear in FACT_CUES, else the
    // test-skip in `every_cue_in_registry_has_at_least_one_phrase_pattern`
    // would silently mask a missing-registration bug.
    for cue in STRUCTURAL_FACT_CUES {
      assert!(
        FACT_CUES.iter().any(|c| c == cue),
        "structural cue `{cue}` not registered in FACT_CUES"
      );
    }
  }
}
