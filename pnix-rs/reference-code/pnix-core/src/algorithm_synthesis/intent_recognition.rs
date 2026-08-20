//! Intent-recognition deterministic synthesis carrier.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/intent-recognition.px`. Given a
//! polymorphic `SynthesisIntentInput` (any combination of facts,
//! utterance, attached code, repo context, prior turns), produce a
//! ranked-candidate `IntentVerdict`. Ambiguity is preserved as Held
//! (no single-answer collapse — see `ontology.md` §15-6 and §15-4c).
//!
//! Registry-driven, not if-else: the signal-to-intent mapping lives in
//! `INTENT_SIGNALS`, an immutable slice of typed entries. Adding a new
//! metaphor or verb cue = one new entry. The carrier never matches raw
//! strings against the registry — signal extraction is upstream (the
//! caller passes a pre-computed `fired_signals` set), so the law and
//! carrier remain free of regex/pattern-on-text logic.
//!
//! Held / Rejected ladder (mirrors `.px` exactly):
//!
//!   missing-input             : no facts AND no utterance AND no code   (Held)
//!   invalid-intent-hint       : operator hint outside valid set         (Rejected)
//!   no-clear-intent           : all scores below `READY_THRESHOLD`      (Held)
//!   ambiguous-multi-intent    : top two scores within `TIE_EPSILON`     (Held)

use serde::{Deserialize, Serialize};

/// Recognized intent categories. Stays byte-identical to the `.px`
/// `validIntents` list. The owner-carrier sync test compares this
/// const slice to the `.px` declarative list.
pub const VALID_INTENTS: &[&str] = &[
  "refactor",
  "fix-bug",
  "add-feature",
  "cleanup",
  "test",
  "optimize",
  "explain",
];

/// Held / Rejected kinds the classifier may emit. Mirror of `.px`
/// `validHeldKinds`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntentHeldKind {
  MissingInput,
  InvalidIntentHint,
  NoClearIntent,
  AmbiguousMultiIntent,
}

impl IntentHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::MissingInput,
    Self::InvalidIntentHint,
    Self::NoClearIntent,
    Self::AmbiguousMultiIntent,
  ];
  pub fn as_str(self) -> &'static str {
    match self {
      Self::MissingInput => "missing-input",
      Self::InvalidIntentHint => "invalid-intent-hint",
      Self::NoClearIntent => "no-clear-intent",
      Self::AmbiguousMultiIntent => "ambiguous-multi-intent",
    }
  }
}

/// One registry entry — pure data. The Rust slice mirrors the `.px`
/// `intentSignals` registry; the owner-carrier sync test asserts
/// set-equality on the `(cue, intent, weight)` tuples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntentSignalEntry {
  pub cue: &'static str,
  pub intent: &'static str,
  pub weight: f32,
}

/// The single source of truth for which signals map to which intent.
/// Adding a new metaphor or verb cue = one new row. No per-cue branch
/// anywhere in the carrier — `score_for_intent` walks this slice
/// generically.
pub const INTENT_SIGNALS: &[IntentSignalEntry] = &[
  // refactor
  IntentSignalEntry {
    cue: "verb:rename",
    intent: "refactor",
    weight: 0.95,
  },
  IntentSignalEntry {
    cue: "verb:simplify",
    intent: "refactor",
    weight: 0.85,
  },
  IntentSignalEntry {
    cue: "verb:reorganize",
    intent: "refactor",
    weight: 0.85,
  },
  IntentSignalEntry {
    cue: "verb:extract",
    intent: "refactor",
    weight: 0.80,
  },
  IntentSignalEntry {
    cue: "verb:inline",
    intent: "refactor",
    weight: 0.80,
  },
  IntentSignalEntry {
    cue: "verb:move",
    intent: "refactor",
    weight: 0.80,
  },
  IntentSignalEntry {
    cue: "metaphor:cleanliness",
    intent: "refactor",
    weight: 0.70,
  },
  IntentSignalEntry {
    cue: "metaphor:elegance",
    intent: "refactor",
    weight: 0.65,
  },
  IntentSignalEntry {
    cue: "metaphor:tidiness",
    intent: "refactor",
    weight: 0.65,
  },
  // fix-bug
  IntentSignalEntry {
    cue: "verb:fix",
    intent: "fix-bug",
    weight: 0.95,
  },
  IntentSignalEntry {
    cue: "verb:repair",
    intent: "fix-bug",
    weight: 0.90,
  },
  IntentSignalEntry {
    cue: "verb:debug",
    intent: "fix-bug",
    weight: 0.90,
  },
  IntentSignalEntry {
    cue: "verb:patch",
    intent: "fix-bug",
    weight: 0.85,
  },
  IntentSignalEntry {
    cue: "metaphor:wrongness",
    intent: "fix-bug",
    weight: 0.75,
  },
  IntentSignalEntry {
    cue: "metaphor:brokenness",
    intent: "fix-bug",
    weight: 0.80,
  },
  IntentSignalEntry {
    cue: "metaphor:diagnosis",
    intent: "fix-bug",
    weight: 0.70,
  },
  IntentSignalEntry {
    cue: "fact:contradicted",
    intent: "fix-bug",
    weight: 0.60,
  },
  // add-feature
  IntentSignalEntry {
    cue: "verb:add",
    intent: "add-feature",
    weight: 0.75,
  },
  IntentSignalEntry {
    cue: "verb:implement",
    intent: "add-feature",
    weight: 0.90,
  },
  IntentSignalEntry {
    cue: "verb:introduce",
    intent: "add-feature",
    weight: 0.80,
  },
  IntentSignalEntry {
    cue: "verb:support",
    intent: "add-feature",
    weight: 0.70,
  },
  IntentSignalEntry {
    cue: "metaphor:new-capability",
    intent: "add-feature",
    weight: 0.80,
  },
  // cleanup
  IntentSignalEntry {
    cue: "verb:remove",
    intent: "cleanup",
    weight: 0.85,
  },
  IntentSignalEntry {
    cue: "verb:delete",
    intent: "cleanup",
    weight: 0.85,
  },
  IntentSignalEntry {
    cue: "metaphor:deadweight",
    intent: "cleanup",
    weight: 0.80,
  },
  IntentSignalEntry {
    cue: "structural:unused-import",
    intent: "cleanup",
    weight: 0.95,
  },
  // test
  IntentSignalEntry {
    cue: "verb:test",
    intent: "test",
    weight: 0.90,
  },
  IntentSignalEntry {
    cue: "verb:cover",
    intent: "test",
    weight: 0.70,
  },
  IntentSignalEntry {
    cue: "metaphor:coverage",
    intent: "test",
    weight: 0.75,
  },
  IntentSignalEntry {
    cue: "structural:test-file",
    intent: "test",
    weight: 0.85,
  },
  // optimize
  IntentSignalEntry {
    cue: "verb:optimize",
    intent: "optimize",
    weight: 0.95,
  },
  IntentSignalEntry {
    cue: "verb:accelerate",
    intent: "optimize",
    weight: 0.85,
  },
  IntentSignalEntry {
    cue: "metaphor:speed",
    intent: "optimize",
    weight: 0.80,
  },
  IntentSignalEntry {
    cue: "metaphor:efficiency",
    intent: "optimize",
    weight: 0.75,
  },
  IntentSignalEntry {
    cue: "fact:slow-path",
    intent: "optimize",
    weight: 0.85,
  },
  // explain
  IntentSignalEntry {
    cue: "verb:explain",
    intent: "explain",
    weight: 0.95,
  },
  IntentSignalEntry {
    cue: "verb:describe",
    intent: "explain",
    weight: 0.85,
  },
  IntentSignalEntry {
    cue: "verb:document",
    intent: "explain",
    weight: 0.85,
  },
  IntentSignalEntry {
    cue: "verb:comment",
    intent: "explain",
    weight: 0.80,
  },
  IntentSignalEntry {
    cue: "metaphor:understanding",
    intent: "explain",
    weight: 0.70,
  },
  // time:* — implicit temporal meaning extracted from NL.
  // Mirror of `.px` time-cue rows; mood cues come from ko.rs's
  // `detect_sentence_mood`, marker cues from query-classifiers.px's
  // `process-markers` / `causal-markers` / `conditional-markers`.
  // Low weights so a single time cue alone never reaches threshold
  // without a verb / metaphor partner — the time cue's main job is
  // downstream (it tells the synthesis chain what *temporal shape*
  // the generated code should have).
  IntentSignalEntry {
    cue: "time:imperative",
    intent: "refactor",
    weight: 0.20,
  },
  IntentSignalEntry {
    cue: "time:imperative",
    intent: "fix-bug",
    weight: 0.20,
  },
  IntentSignalEntry {
    cue: "time:imperative",
    intent: "add-feature",
    weight: 0.20,
  },
  IntentSignalEntry {
    cue: "time:imperative",
    intent: "cleanup",
    weight: 0.20,
  },
  IntentSignalEntry {
    cue: "time:interrogative",
    intent: "explain",
    weight: 0.35,
  },
  IntentSignalEntry {
    cue: "time:interrogative",
    intent: "fix-bug",
    weight: 0.20,
  },
  IntentSignalEntry {
    cue: "time:propositive",
    intent: "refactor",
    weight: 0.10,
  },
  IntentSignalEntry {
    cue: "time:declarative",
    intent: "explain",
    weight: 0.10,
  },
  IntentSignalEntry {
    cue: "time:process-step",
    intent: "explain",
    weight: 0.30,
  },
  IntentSignalEntry {
    cue: "time:process-step",
    intent: "refactor",
    weight: 0.15,
  },
  IntentSignalEntry {
    cue: "time:process-step",
    intent: "add-feature",
    weight: 0.15,
  },
  IntentSignalEntry {
    cue: "time:causal",
    intent: "fix-bug",
    weight: 0.45,
  },
  IntentSignalEntry {
    cue: "time:causal",
    intent: "explain",
    weight: 0.25,
  },
  IntentSignalEntry {
    cue: "time:conditional",
    intent: "add-feature",
    weight: 0.35,
  },
  IntentSignalEntry {
    cue: "time:conditional",
    intent: "fix-bug",
    weight: 0.15,
  },
  // fact:* — observed predicates from relation-extraction or
  // declarative NL. Already present above for `fact:contradicted`
  // (fix-bug) and `fact:slow-path` (optimize); the additional v0
  // fact cues map as follows:
  //
  //   - `fact:definition` is a soft pointer to `explain` intent.
  //   - `fact:causal-relation` is a strong pointer to `fix-bug` and
  //     a soft one to `explain` — the same asymmetry `time:causal`
  //     uses.
  //   - `fact:missing-case` is a strong `test` pointer (need a test
  //     for the unhandled case) and a soft `add-feature` pointer
  //     (handle the case at all).
  IntentSignalEntry {
    cue: "fact:definition",
    intent: "explain",
    weight: 0.55,
  },
  IntentSignalEntry {
    cue: "fact:causal-relation",
    intent: "fix-bug",
    weight: 0.50,
  },
  IntentSignalEntry {
    cue: "fact:causal-relation",
    intent: "explain",
    weight: 0.25,
  },
  IntentSignalEntry {
    cue: "fact:missing-case",
    intent: "test",
    weight: 0.70,
  },
  IntentSignalEntry {
    cue: "fact:missing-case",
    intent: "add-feature",
    weight: 0.30,
  },
  // fact:missing-import — NameError / unresolved-import diagnostic
  // from an upstream lint/compile/LSP source. Strong pointer to
  // fix-bug; soft pointer to cleanup (sometimes the right fix is to
  // remove the dead reference instead of importing).
  IntentSignalEntry {
    cue: "fact:missing-import",
    intent: "fix-bug",
    weight: 0.80,
  },
  IntentSignalEntry {
    cue: "fact:missing-import",
    intent: "cleanup",
    weight: 0.20,
  },
  // Math-domain. `fact:math-question` is a strong pointer to
  // `explain` (user is asking for an equivalent / expansion). High
  // weight because the cue itself requires both a math operator and
  // a question shape — false positives are rare.
  IntentSignalEntry {
    cue: "fact:math-question",
    intent: "explain",
    weight: 0.90,
  },
  // Chemistry-domain — substrate-sharing N=3. Same explain route.
  IntentSignalEntry {
    cue: "fact:chemistry-question",
    intent: "explain",
    weight: 0.90,
  },
  // pnix `.px` — graph mode (algorithm/dataflow specimen, e.g.
  // examples/pnix_algo/completed/**). Detection of `externs/nodes/
  // edges/types/inputs` attrset structure. Editing this kind of `.px`
  // is most often *add-feature* (new builtin call, new node, new edge)
  // but can be *refactor* (renaming externs, restructuring node
  // lists). The cue fires per-shape; the upstream extractor decides
  // which shapes are present.
  IntentSignalEntry {
    cue: "structural:px-extern-decl",
    intent: "add-feature",
    weight: 0.60,
  },
  IntentSignalEntry {
    cue: "structural:px-extern-decl",
    intent: "refactor",
    weight: 0.40,
  },
  IntentSignalEntry {
    cue: "structural:px-node-list",
    intent: "add-feature",
    weight: 0.55,
  },
  IntentSignalEntry {
    cue: "structural:px-node-list",
    intent: "refactor",
    weight: 0.45,
  },
  IntentSignalEntry {
    cue: "structural:px-edge-list",
    intent: "add-feature",
    weight: 0.55,
  },
  IntentSignalEntry {
    cue: "structural:px-edge-list",
    intent: "refactor",
    weight: 0.45,
  },
  IntentSignalEntry {
    cue: "structural:px-types-decl",
    intent: "refactor",
    weight: 0.50,
  },
  // pnix `.px` — expression mode (stdlib/lib/**, gate owners,
  // `let X = ...; in { ... }` shape). Editing this kind of `.px` is
  // most often *refactor* (let restructuring, renaming bindings) or
  // *add-feature* (new helper, new import, new owner-law shape).
  IntentSignalEntry {
    cue: "structural:px-let-binding",
    intent: "refactor",
    weight: 0.70,
  },
  IntentSignalEntry {
    cue: "structural:px-lambda",
    intent: "refactor",
    weight: 0.55,
  },
  IntentSignalEntry {
    cue: "structural:px-lambda",
    intent: "add-feature",
    weight: 0.45,
  },
  IntentSignalEntry {
    cue: "structural:px-import-stmt",
    intent: "add-feature",
    weight: 0.50,
  },
  IntentSignalEntry {
    cue: "structural:px-import-stmt",
    intent: "cleanup",
    weight: 0.30,
  },
  IntentSignalEntry {
    cue: "structural:px-owner-law-shape",
    intent: "add-feature",
    weight: 0.55,
  },
  IntentSignalEntry {
    cue: "structural:px-owner-law-shape",
    intent: "refactor",
    weight: 0.45,
  },
  // verb:create — creating new artifacts (functions, nodes, owner-
  // laws, externs). Strong add-feature pointer. Distinct from
  // verb:add in that the focus is *bringing into existence* rather
  // than appending to a sequence — but they often co-occur and both
  // route to add-feature transforms.
  IntentSignalEntry {
    cue: "verb:create",
    intent: "add-feature",
    weight: 0.85,
  },
  // verb:connect — wiring two existing things (nodes, edges,
  // imports, modules). Primary intent is add-feature (the connection
  // itself is the new artifact), with a softer refactor pointer for
  // re-wiring existing structure.
  IntentSignalEntry {
    cue: "verb:connect",
    intent: "add-feature",
    weight: 0.70,
  },
  IntentSignalEntry {
    cue: "verb:connect",
    intent: "refactor",
    weight: 0.30,
  },
  // ─── Python-domain metaphors — substrate-sharing N=5 ────────────
  //
  // Each Python-idiomatic operation has a primary intent and a soft
  // secondary intent reflecting its dual nature: type hints can be
  // add-feature (introducing types) or refactor (cleaning unhinted
  // signatures); decorators can be add-feature (new @property /
  // @dataclass) or refactor (replacing manual boilerplate); etc.
  //
  // The weights are calibrated against existing cues to avoid
  // tieEpsilon ambiguity when combined with `verb:add` / `verb:rename`
  // utterances. A bare metaphor (no verb) stays below readyThreshold;
  // composition with a verb pushes the typical Python operation above.

  // python-typing — adding/improving type hints
  IntentSignalEntry {
    cue: "metaphor:python-typing",
    intent: "add-feature",
    weight: 0.65,
  },
  IntentSignalEntry {
    cue: "metaphor:python-typing",
    intent: "refactor",
    weight: 0.30,
  },
  // python-fstring — modernizing string formatting
  IntentSignalEntry {
    cue: "metaphor:python-fstring",
    intent: "refactor",
    weight: 0.70,
  },
  IntentSignalEntry {
    cue: "metaphor:python-fstring",
    intent: "cleanup",
    weight: 0.20,
  },
  // python-decorator — applying decorators (add-feature dominant
  // because @dataclass / @property / @cache add behavior; refactor
  // when replacing manual boilerplate)
  IntentSignalEntry {
    cue: "metaphor:python-decorator",
    intent: "add-feature",
    weight: 0.65,
  },
  IntentSignalEntry {
    cue: "metaphor:python-decorator",
    intent: "refactor",
    weight: 0.30,
  },
  // python-comprehension — converting loops to comprehensions
  // (refactor dominant)
  IntentSignalEntry {
    cue: "metaphor:python-comprehension",
    intent: "refactor",
    weight: 0.75,
  },
  // python-async — converting sync to async (typically add-feature
  // because async fundamentally changes function contract)
  IntentSignalEntry {
    cue: "metaphor:python-async",
    intent: "add-feature",
    weight: 0.55,
  },
  IntentSignalEntry {
    cue: "metaphor:python-async",
    intent: "refactor",
    weight: 0.40,
  },
  // python-pytest — test scaffolding
  IntentSignalEntry {
    cue: "metaphor:python-pytest",
    intent: "test",
    weight: 0.85,
  },
  // python-dataclass — applying @dataclass to a class (refactor —
  // replacing manual __init__/__repr__/__eq__ boilerplate)
  IntentSignalEntry {
    cue: "metaphor:python-dataclass",
    intent: "refactor",
    weight: 0.70,
  },
  IntentSignalEntry {
    cue: "metaphor:python-dataclass",
    intent: "add-feature",
    weight: 0.30,
  },
  // python-pythonic — broad "make it more Pythonic" request
  // (refactor)
  IntentSignalEntry {
    cue: "metaphor:python-pythonic",
    intent: "refactor",
    weight: 0.65,
  },
];

/// Scoring thresholds. Stays byte-identical to the `.px` constants.
pub const READY_THRESHOLD: f32 = 0.5;
pub const TIE_EPSILON: f32 = 0.10;

/// Polymorphic input. All fields optional — different input modes
/// populate different subsets:
///
///   - abstract NL : `utterance` + `fired_signals`
///   - detailed NL : `utterance` + `fired_signals` + `facts`
///   - code-only   : `attached_code` + `fired_signals` (structural cues)
///   - mixed       : any combination
///
/// `fired_signals` is the carrier's clean abstraction: callers (or
/// upstream signal extractors) decide which cues fired. The carrier
/// never matches raw text — separation of concerns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SynthesisIntentInput {
  #[serde(default)]
  pub utterance: String,
  /// Pre-lifted relation-extraction facts (subj/pred/obj records as
  /// JSON values). The carrier doesn't unpack these — they exist as
  /// context the caller can use to derive `fact:<pred>` signals.
  #[serde(default)]
  pub facts: Vec<serde_json::Value>,
  #[serde(default)]
  pub attached_code: String,
  #[serde(default)]
  pub target_path: String,
  #[serde(default)]
  pub prior_turn_summaries: Vec<String>,
  /// Pre-computed signal set — what cues fired upstream. The carrier's
  /// classify operates purely on this set.
  #[serde(default)]
  pub fired_signals: Vec<String>,
  /// Operator-supplied intent hint. Empty when the operator gave no
  /// hint. When non-empty AND not in `VALID_INTENTS`, classify rejects.
  #[serde(default)]
  pub intent_hint: String,
}

/// One ranked candidate intent. Multiple intents can coexist in the
/// receipt — pnix preserves ambiguity rather than collapsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankedIntent {
  pub intent: String,
  pub confidence: f32,
  pub signals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum IntentVerdict {
  IntentRecognitionReady {
    ranked_intents: Vec<RankedIntent>,
  },
  IntentRecognitionHeld {
    held_kind: IntentHeldKind,
    reason: String,
    ranked_intents: Vec<RankedIntent>,
  },
  IntentRecognitionRejected {
    held_kind: IntentHeldKind,
    reason: String,
    ranked_intents: Vec<RankedIntent>,
  },
}

fn is_valid_intent(s: &str) -> bool {
  VALID_INTENTS.iter().any(|v| *v == s)
}

/// Generic scorer: for one intent, sum the weights of registry entries
/// whose `intent` matches AND whose `cue` is in `fired_signals`. No
/// per-cue branch.
fn score_for_intent(fired: &[String], intent: &str) -> RankedIntent {
  let mut confidence = 0.0f32;
  let mut signals: Vec<String> = Vec::new();
  for entry in INTENT_SIGNALS {
    if entry.intent == intent && fired.iter().any(|s| s == entry.cue) {
      confidence += entry.weight;
      signals.push(entry.cue.to_string());
    }
  }
  RankedIntent {
    intent: intent.to_string(),
    confidence,
    signals,
  }
}

fn score_all_intents(fired: &[String]) -> Vec<RankedIntent> {
  VALID_INTENTS
    .iter()
    .map(|i| score_for_intent(fired, i))
    .collect()
}

fn rank_intents(mut scored: Vec<RankedIntent>) -> Vec<RankedIntent> {
  scored.retain(|s| s.confidence > 0.0);
  // Descending by confidence; ties broken by intent name (deterministic).
  scored.sort_by(|a, b| {
    b.confidence
      .partial_cmp(&a.confidence)
      .unwrap_or(std::cmp::Ordering::Equal)
      .then_with(|| a.intent.cmp(&b.intent))
  });
  scored
}

/// Classify mirrors the `.px` ladder. First matching rule wins.
pub fn classify_intent_recognition(input: &SynthesisIntentInput) -> IntentVerdict {
  classify_intent_recognition_with_ranked(
    input,
    rank_intents(score_all_intents(&input.fired_signals)),
  )
}

fn classify_intent_recognition_with_ranked(
  input: &SynthesisIntentInput,
  ranked: Vec<RankedIntent>,
) -> IntentVerdict {
  let has_any_input =
    !input.utterance.is_empty() || !input.facts.is_empty() || !input.attached_code.is_empty();

  if !has_any_input {
    return IntentVerdict::IntentRecognitionHeld {
      held_kind: IntentHeldKind::MissingInput,
      reason: "no facts, no utterance, no attached code — synthesis cannot proceed".to_string(),
      ranked_intents: ranked,
    };
  }
  if !input.intent_hint.is_empty() && !is_valid_intent(&input.intent_hint) {
    return IntentVerdict::IntentRecognitionRejected {
      held_kind: IntentHeldKind::InvalidIntentHint,
      reason: format!(
        "operator-supplied intent_hint `{}` is not in valid intent set",
        input.intent_hint
      ),
      ranked_intents: ranked,
    };
  }
  let top = ranked.first().cloned();
  let second = ranked.get(1).cloned();
  let top_ready = top
    .as_ref()
    .map(|t| t.confidence >= READY_THRESHOLD)
    .unwrap_or(false);
  if !top_ready {
    return IntentVerdict::IntentRecognitionHeld {
      held_kind: IntentHeldKind::NoClearIntent,
      reason: format!(
        "all intent scores below readyThreshold ({READY_THRESHOLD}); ask the operator to clarify what kind of change is wanted"
      ),
      ranked_intents: ranked,
    };
  }
  let tie = match (&top, &second) {
    (Some(t), Some(s)) => (t.confidence - s.confidence) < TIE_EPSILON,
    _ => false,
  };
  if tie {
    return IntentVerdict::IntentRecognitionHeld {
      held_kind: IntentHeldKind::AmbiguousMultiIntent,
      reason: format!(
        "top two intents are within tieEpsilon ({TIE_EPSILON}); ambiguity preserved — operator clarification or additional context needed"
      ),
      ranked_intents: ranked,
    };
  }
  IntentVerdict::IntentRecognitionReady {
    ranked_intents: ranked,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn input_with_signals(signals: &[&str]) -> SynthesisIntentInput {
    SynthesisIntentInput {
      utterance: "stub".to_string(),
      fired_signals: signals.iter().map(|s| s.to_string()).collect(),
      ..Default::default()
    }
  }

  #[test]
  fn registry_invariants_hold() {
    // No duplicate (cue, intent) pairs.
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for e in INTENT_SIGNALS {
      let pair = (e.cue, e.intent);
      assert!(!seen.contains(&pair), "duplicate registry entry: {pair:?}");
      seen.push(pair);
      // Weight in [0, 1].
      assert!(
        (0.0..=1.0).contains(&e.weight),
        "weight out of range for {pair:?}: {}",
        e.weight
      );
      // Intent must be in VALID_INTENTS.
      assert!(
        VALID_INTENTS.contains(&e.intent),
        "registry intent `{}` not in VALID_INTENTS",
        e.intent
      );
    }
  }

  #[test]
  fn held_on_missing_input() {
    let v = classify_intent_recognition(&SynthesisIntentInput::default());
    assert!(matches!(
      v,
      IntentVerdict::IntentRecognitionHeld {
        held_kind: IntentHeldKind::MissingInput,
        ..
      }
    ));
  }

  #[test]
  fn rejected_on_invalid_intent_hint() {
    let mut inp = input_with_signals(&["verb:rename"]);
    inp.intent_hint = "remix".to_string(); // not in VALID_INTENTS
    let v = classify_intent_recognition(&inp);
    assert!(matches!(
      v,
      IntentVerdict::IntentRecognitionRejected {
        held_kind: IntentHeldKind::InvalidIntentHint,
        ..
      }
    ));
  }

  #[test]
  fn ready_on_clear_refactor_signal() {
    // Single strong refactor signal (verb:rename = 0.95) far above
    // threshold (0.5), no competing intent.
    let v = classify_intent_recognition(&input_with_signals(&["verb:rename"]));
    match v {
      IntentVerdict::IntentRecognitionReady { ranked_intents } => {
        assert_eq!(ranked_intents[0].intent, "refactor");
        assert!(ranked_intents[0].confidence >= READY_THRESHOLD);
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn held_on_no_clear_intent_when_only_weak_signal() {
    // One weak signal (metaphor:tidiness = 0.65 for refactor) above
    // threshold — should be Ready. But a single 0.45 signal is below.
    // No such single signal exists in registry; build via aggregate of
    // *only* sub-threshold-by-themselves cues. Actually metaphor:elegance
    // alone = 0.65, also above 0.5. Use a contrived but valid case:
    // empty fired_signals + non-empty utterance → no signals fire → Held.
    let inp = SynthesisIntentInput {
      utterance: "blah blah".to_string(),
      ..Default::default()
    };
    let v = classify_intent_recognition(&inp);
    assert!(matches!(
      v,
      IntentVerdict::IntentRecognitionHeld {
        held_kind: IntentHeldKind::NoClearIntent,
        ..
      }
    ));
  }

  #[test]
  fn held_on_ambiguous_multi_intent_when_signals_tie() {
    // verb:rename (refactor 0.95) and verb:fix (fix-bug 0.95) tie
    // exactly. tie - 0 < TIE_EPSILON (0.10) → ambiguous.
    let v = classify_intent_recognition(&input_with_signals(&["verb:rename", "verb:fix"]));
    assert!(matches!(
      v,
      IntentVerdict::IntentRecognitionHeld {
        held_kind: IntentHeldKind::AmbiguousMultiIntent,
        ..
      }
    ));
  }

  #[test]
  fn ready_when_top_intent_has_margin_over_second() {
    // verb:rename (refactor 0.95) + verb:fix (fix-bug 0.95) is tied.
    // Add verb:simplify (refactor +0.85) → refactor total 1.80, fix-bug
    // 0.95. Gap 0.85 > 0.10 → Ready.
    let v = classify_intent_recognition(&input_with_signals(&[
      "verb:rename",
      "verb:simplify",
      "verb:fix",
    ]));
    match v {
      IntentVerdict::IntentRecognitionReady { ranked_intents } => {
        assert_eq!(ranked_intents[0].intent, "refactor");
        assert!(ranked_intents[0].confidence > ranked_intents[1].confidence);
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn code_only_input_with_structural_signals_works() {
    // No utterance, no facts — but attached_code is non-empty AND
    // structural signals fired (e.g. caller saw "unused import"
    // pattern). Should NOT trip missing-input Held.
    let inp = SynthesisIntentInput {
      attached_code: "import os\nimport sys\nprint('hi')\n".to_string(),
      fired_signals: vec!["structural:unused-import".to_string()],
      ..Default::default()
    };
    let v = classify_intent_recognition(&inp);
    match v {
      IntentVerdict::IntentRecognitionReady { ranked_intents } => {
        assert_eq!(ranked_intents[0].intent, "cleanup");
      }
      other => panic!("expected Ready cleanup, got {other:?}"),
    }
  }

  #[test]
  fn detailed_nl_with_multiple_consistent_signals_works() {
    // "X parameter 의 이름을 Y로 바꿔주고, 비슷한 부분도 깔끔하게" →
    // verb:rename + metaphor:cleanliness — both refactor. Total
    // confidence = 0.95 + 0.70 = 1.65. Single dominant intent.
    let v = classify_intent_recognition(&input_with_signals(&[
      "verb:rename",
      "metaphor:cleanliness",
    ]));
    match v {
      IntentVerdict::IntentRecognitionReady { ranked_intents } => {
        assert_eq!(ranked_intents[0].intent, "refactor");
        assert!((ranked_intents[0].confidence - 1.65).abs() < 0.001);
      }
      other => panic!("expected Ready refactor, got {other:?}"),
    }
  }

  #[test]
  fn ranked_intents_preserved_in_held_results() {
    // Held verdict still carries the ranked candidate set for audit —
    // §15-6: ambiguity is preserved, not collapsed.
    let v = classify_intent_recognition(&input_with_signals(&["verb:rename", "verb:fix"]));
    match v {
      IntentVerdict::IntentRecognitionHeld {
        ranked_intents,
        held_kind: IntentHeldKind::AmbiguousMultiIntent,
        ..
      } => {
        // Both intents present, both above threshold, near-tied.
        assert_eq!(ranked_intents.len(), 2);
        let intents: Vec<&str> = ranked_intents.iter().map(|r| r.intent.as_str()).collect();
        assert!(intents.contains(&"refactor"));
        assert!(intents.contains(&"fix-bug"));
      }
      other => panic!("expected AmbiguousMultiIntent Held, got {other:?}"),
    }
  }

  #[test]
  fn classify_is_pure_function_of_fired_signals() {
    // The carrier should not depend on raw utterance text — same
    // fired_signals → same verdict regardless of utterance content.
    // This is a key invariant: signal extraction is upstream.
    let v1 = classify_intent_recognition(&SynthesisIntentInput {
      utterance: "completely different text".to_string(),
      fired_signals: vec!["verb:rename".to_string()],
      ..Default::default()
    });
    let v2 = classify_intent_recognition(&SynthesisIntentInput {
      utterance: "yet another wording".to_string(),
      fired_signals: vec!["verb:rename".to_string()],
      ..Default::default()
    });
    assert_eq!(format!("{v1:?}"), format!("{v2:?}"));
  }

  #[test]
  fn valid_intents_const_matches_registry_intents() {
    // Every intent the registry references must be in VALID_INTENTS.
    for e in INTENT_SIGNALS {
      assert!(
        VALID_INTENTS.contains(&e.intent),
        "registry references intent `{}` not in VALID_INTENTS",
        e.intent
      );
    }
  }
}
