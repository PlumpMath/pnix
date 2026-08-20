//! Relation-extraction classifier — host carrier for the `.px` relation
//! owner laws.
//!
//! # OWNER-LAW (2026-05-11)
//!
//! Source of truth for relation kinds lives in `.px`:
//!
//! - `stdlib/lib/gate/relation-extraction/definition.px`
//! - `stdlib/lib/gate/relation-extraction/comparison.px`
//! - `stdlib/lib/gate/relation-extraction/formula.px`
//! - `stdlib/lib/gate/relation-extraction/negation.px`
//! - `stdlib/lib/gate/relation-extraction/causality.px`
//!
//! This Rust module is *carrier only*. It mirrors the deterministic marker
//! tables those `.px` owners use so the `evidence_facts_from_passage_extracted`
//! lowering path can tag pred / polarity on an `EvidenceFact` without
//! evaluating `.px` at runtime. **It does not invent new relation kinds.**
//!
//! Each function returns the matched marker (so callers can include it in
//! `provenance_refs` for replay) plus a reference back to the `.px` owner
//! so audit trails can land on the canonical law.
//!
//! ## What this module is NOT
//!
//! - It is **not** a separate semantic owner.
//! - It does **not** decide promotion / accepted memory.
//! - It does **not** add comparison kinds the `.px` owner hasn't already named.
//! - It does **not** parse formulas (formula owner-law defers dimension check
//!   to host, which is out of scope for this carrier).
//!
//! When the `.px` owner gains a new marker, this carrier must be updated in
//! the same patch (or it stays stale and behaves more conservatively). The
//! `OWNER_LAW_FILE` constants below name the canonical owner so reviewers
//! know where to look.

/// Comparison kinds named by `stdlib/lib/gate/relation-extraction/comparison.px`.
/// The string form matches the predicate the owner law emits, so callers
/// can use it directly as `EvidenceFact::pred`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonKind {
  GreaterThan,
  LessThan,
  Equal,
  GreaterOrEqual,
  LessOrEqual,
}

impl ComparisonKind {
  pub fn predicate(self) -> &'static str {
    match self {
      Self::GreaterThan => "greater-than",
      Self::LessThan => "less-than",
      Self::Equal => "equal",
      Self::GreaterOrEqual => "greater-or-equal",
      Self::LessOrEqual => "less-or-equal",
    }
  }
}

/// Result of a relation-classifier hit. Carries the matched marker for
/// provenance and a reference to the canonical owner-law file.
#[derive(Debug, Clone)]
pub struct RelationHit {
  pub matched_marker: String,
  pub owner_law: &'static str,
}

/// Negation classifier — mirrors `negation.px`.
///
/// Returns `Some(hit)` when the passage contains a deterministic negation
/// marker. Caller should flip `Polarity::Supports → Polarity::Contradicts`
/// on the resulting EvidenceFact.
///
/// Markers are kept in sync with the owner law. Korean markers come first
/// because the substrate's primary lane is Korean.
pub fn classify_negation(passage: &str) -> Option<RelationHit> {
  const OWNER_LAW: &str = "stdlib/lib/gate/relation-extraction/negation.px";
  let p = passage.to_lowercase();

  // Korean negation markers (from negation.px)
  for marker in &[
    "아니다",
    "아닙니다",
    "아니야",
    "결코 아니",
    "절대 아니",
    "않다",
    "않습니다",
  ] {
    if passage.contains(marker) {
      return Some(RelationHit {
        matched_marker: (*marker).to_string(),
        owner_law: OWNER_LAW,
      });
    }
  }

  // English / symbolic negation markers
  for marker in &[
    " is not ",
    " are not ",
    " was not ",
    " were not ",
    " never ",
    " != ",
  ] {
    if p.contains(marker) {
      return Some(RelationHit {
        matched_marker: marker.trim().to_string(),
        owner_law: OWNER_LAW,
      });
    }
  }

  None
}

/// Comparison classifier — mirrors `comparison.px`.
///
/// Returns the comparison kind plus the matched marker. Order matters:
/// longer / more-specific markers come first so `"greater than or equal"`
/// is not shadowed by `"greater than"`.
pub fn classify_comparison(passage: &str) -> Option<(ComparisonKind, RelationHit)> {
  const OWNER_LAW: &str = "stdlib/lib/gate/relation-extraction/comparison.px";
  let p = passage.to_lowercase();

  // longest-match first to avoid prefix shadowing
  let table: &[(&str, ComparisonKind)] = &[
    ("greater than or equal", ComparisonKind::GreaterOrEqual),
    ("less than or equal", ComparisonKind::LessOrEqual),
    ("greater than", ComparisonKind::GreaterThan),
    ("less than", ComparisonKind::LessThan),
    (" >= ", ComparisonKind::GreaterOrEqual),
    (" <= ", ComparisonKind::LessOrEqual),
    (" > ", ComparisonKind::GreaterThan),
    (" < ", ComparisonKind::LessThan),
    (" == ", ComparisonKind::Equal),
  ];
  for (marker, kind) in table {
    if p.contains(marker) {
      return Some((
        *kind,
        RelationHit {
          matched_marker: marker.trim().to_string(),
          owner_law: OWNER_LAW,
        },
      ));
    }
  }

  // Korean markers — exact substring on the raw passage (not lowercased).
  let ko_table: &[(&str, ComparisonKind)] = &[
    ("이상", ComparisonKind::GreaterOrEqual),
    ("이하", ComparisonKind::LessOrEqual),
    ("같다", ComparisonKind::Equal),
    ("같음", ComparisonKind::Equal),
    ("크다", ComparisonKind::GreaterThan),
    ("작다", ComparisonKind::LessThan),
  ];
  for (marker, kind) in ko_table {
    if passage.contains(marker) {
      return Some((
        *kind,
        RelationHit {
          matched_marker: (*marker).to_string(),
          owner_law: OWNER_LAW,
        },
      ));
    }
  }

  None
}

/// Formula classifier — mirrors `formula.px`.
///
/// Returns `Some(hit)` when the passage contains an equation/formula
/// marker (`X = Y`, `≡`, `:=`, LaTeX `\frac{..}`, `\sqrt{..}` etc.).
/// Caller can set `pred = "formula"` and tag the EvidenceFact for
/// downstream host dimension check (the owner law's `dimension_check_required`
/// flag — surfaced in provenance as `formula-dimension-check-required`).
///
/// The `.px` owner emits subj = lhs, obj = rhs after host extraction;
/// this carrier only flags *presence*, not the lhs/rhs split.
pub fn classify_formula(passage: &str) -> Option<RelationHit> {
  const OWNER_LAW: &str = "stdlib/lib/gate/relation-extraction/formula.px";

  // `=` between alphanumeric / `)` / `]` on the left and alphanumeric /
  // `(` / `[` on the right — mirrors the `.px` regex.
  if passage_has_equation_marker(passage) {
    return Some(RelationHit {
      matched_marker: "=".to_string(),
      owner_law: OWNER_LAW,
    });
  }
  for marker in &[
    "≡", ":=", "\\frac{", "\\sqrt{", "\\sum{", "\\int{", "\\prod{",
  ] {
    if passage.contains(marker) {
      return Some(RelationHit {
        matched_marker: (*marker).to_string(),
        owner_law: OWNER_LAW,
      });
    }
  }
  None
}

/// Inline scan for an equation `=` flanked by content on both sides.
/// Mirrors the `.px` regex `.*[a-zA-Z0-9)\]] *= *[a-zA-Z0-9(\[].*`.
fn passage_has_equation_marker(p: &str) -> bool {
  let bytes = p.as_bytes();
  for (i, b) in bytes.iter().enumerate() {
    if *b != b'=' {
      continue;
    }
    // Avoid matching `==`, `!=`, `>=`, `<=`, `:=` (handled by other markers
    // or by negation/comparison classifiers).
    let prev = if i > 0 { bytes[i - 1] } else { b' ' };
    let next = bytes.get(i + 1).copied().unwrap_or(b' ');
    if next == b'=' || prev == b'=' || prev == b'!' || prev == b'>' || prev == b'<' || prev == b':'
    {
      continue;
    }
    let left = nearest_non_space_left(bytes, i);
    let right = nearest_non_space_right(bytes, i);
    if let (Some(l), Some(r)) = (left, right) {
      let l_ok = l.is_ascii_alphanumeric() || l == b')' || l == b']';
      let r_ok = r.is_ascii_alphanumeric() || r == b'(' || r == b'[';
      if l_ok && r_ok {
        return true;
      }
    }
  }
  false
}

fn nearest_non_space_left(bytes: &[u8], from: usize) -> Option<u8> {
  let mut i = from;
  while i > 0 {
    i -= 1;
    if bytes[i] != b' ' {
      return Some(bytes[i]);
    }
  }
  None
}

fn nearest_non_space_right(bytes: &[u8], from: usize) -> Option<u8> {
  let mut i = from + 1;
  while i < bytes.len() {
    if bytes[i] != b' ' {
      return Some(bytes[i]);
    }
    i += 1;
  }
  None
}

/// Causality classifier — mirrors `causality.px`.
///
/// Returns `Some(hit)` when the passage contains a causality marker
/// (`causes`, `because`, `때문에`, `→`, `⇒`, etc.). Caller can set
/// `pred = "causes"` and mark the EvidenceFact `chain-eligible:true` so
/// the future 2-hop / composed-fact reasoner can walk these.
///
/// Order matters: Korean markers are listed first because they would be
/// stripped by `.to_lowercase()` only by ASCII-fold. We test on the raw
/// passage to keep Korean intact.
pub fn classify_causality(passage: &str) -> Option<RelationHit> {
  const OWNER_LAW: &str = "stdlib/lib/gate/relation-extraction/causality.px";
  let p = passage.to_lowercase();

  // English markers (lowercased match).
  for marker in &[
    " causes ",
    " caused by ",
    " leads to ",
    " results in ",
    " due to ",
    " because ",
    " therefore ",
    " hence ",
    " thus ",
    " so that ",
    " => ",
  ] {
    if p.contains(marker) {
      return Some(RelationHit {
        matched_marker: marker.trim().to_string(),
        owner_law: OWNER_LAW,
      });
    }
  }

  // Korean + symbolic markers (raw passage).
  for marker in &[
    "때문에",
    "야기",
    "초래",
    "결과로",
    "따라서",
    "그러므로",
    "그래서",
    "→",
    "⇒",
  ] {
    if passage.contains(marker) {
      return Some(RelationHit {
        matched_marker: (*marker).to_string(),
        owner_law: OWNER_LAW,
      });
    }
  }

  None
}

/// Definition classifier — mirrors `definition.px`.
///
/// Returns `Some(hit)` when the passage contains a definition marker
/// ("X is Y", "X = Y", "X means Y", "X는 Y이다"). Caller can set
/// `pred = "is-defined-as"` and treat the passage as a definition fact.
pub fn classify_definition(passage: &str) -> Option<RelationHit> {
  const OWNER_LAW: &str = "stdlib/lib/gate/relation-extraction/definition.px";
  let p = passage.to_lowercase();

  // English / symbolic definition markers
  for marker in &[" is defined as ", " means ", " refers to ", " := ", " ::= "] {
    if p.contains(marker) {
      return Some(RelationHit {
        matched_marker: marker.trim().to_string(),
        owner_law: OWNER_LAW,
      });
    }
  }

  // Korean definition markers — exact substring on raw passage.
  for marker in &[
    "라고 한다",
    "라고 부른다",
    "는 다음과 같이 정의된다",
    "의 정의는",
  ] {
    if passage.contains(marker) {
      return Some(RelationHit {
        matched_marker: (*marker).to_string(),
        owner_law: OWNER_LAW,
      });
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn classifies_korean_negation() {
    let hit = classify_negation("빛은 입자가 아니다").expect("negation");
    assert_eq!(hit.matched_marker, "아니다");
    assert!(hit.owner_law.ends_with("negation.px"));
  }

  #[test]
  fn classifies_english_negation() {
    let hit = classify_negation("Light is not a wave alone.").expect("negation");
    assert_eq!(hit.matched_marker, "is not");
  }

  #[test]
  fn classifies_inequality_symbol() {
    let hit = classify_negation("x != y").expect("symbolic negation");
    assert_eq!(hit.matched_marker, "!=");
  }

  #[test]
  fn negation_none_on_plain_sentence() {
    assert!(classify_negation("electron has mass small").is_none());
  }

  #[test]
  fn classifies_greater_than() {
    let (kind, hit) =
      classify_comparison("proton mass is greater than electron mass").expect("comparison");
    assert_eq!(kind, ComparisonKind::GreaterThan);
    assert_eq!(kind.predicate(), "greater-than");
    assert_eq!(hit.matched_marker, "greater than");
  }

  #[test]
  fn comparison_longest_match_wins() {
    // "greater than or equal" must not be shadowed by "greater than".
    let (kind, _) = classify_comparison("X is greater than or equal Y").expect("comparison");
    assert_eq!(kind, ComparisonKind::GreaterOrEqual);
  }

  #[test]
  fn classifies_korean_comparison() {
    let (kind, hit) = classify_comparison("이 값은 100 이상").expect("comparison");
    assert_eq!(kind, ComparisonKind::GreaterOrEqual);
    assert_eq!(hit.matched_marker, "이상");
  }

  #[test]
  fn classifies_symbol_lt() {
    let (kind, _) = classify_comparison("a < b").expect("comparison");
    assert_eq!(kind, ComparisonKind::LessThan);
  }

  #[test]
  fn classifies_definition_means() {
    let hit = classify_definition("ontology means the science of being").expect("def");
    assert_eq!(hit.matched_marker, "means");
  }

  #[test]
  fn classifies_korean_definition() {
    let hit = classify_definition("이것을 빛이라고 한다").expect("def");
    assert_eq!(hit.matched_marker, "라고 한다");
  }

  #[test]
  fn definition_none_on_plain_sentence() {
    assert!(classify_definition("the sky is blue today").is_none());
  }

  #[test]
  fn classifies_formula_with_equals() {
    let hit = classify_formula("F = ma").expect("formula");
    assert_eq!(hit.matched_marker, "=");
    assert!(hit.owner_law.ends_with("formula.px"));
  }

  #[test]
  fn classifies_formula_with_assignment() {
    let hit = classify_formula("v := u + at").expect("formula");
    assert_eq!(hit.matched_marker, ":=");
  }

  #[test]
  fn classifies_formula_with_latex() {
    // `=` is followed by ` \` here, which is not a valid right operand for
    // the equation regex (`\` is not alphanumeric / `(` / `[`). So the
    // LaTeX `\frac{` marker is what actually matches.
    let hit = classify_formula("kinetic energy = \\frac{1}{2}mv^2").expect("formula");
    assert_eq!(hit.matched_marker, "\\frac{");
  }

  #[test]
  fn classifies_formula_with_pure_latex() {
    let hit = classify_formula("Area is \\frac{1}{2}bh in geometry.").expect("formula");
    assert_eq!(hit.matched_marker, "\\frac{");
  }

  #[test]
  fn classifies_formula_with_identity() {
    let hit = classify_formula("a ≡ b mod n").expect("formula");
    assert_eq!(hit.matched_marker, "≡");
  }

  #[test]
  fn formula_does_not_match_inequality() {
    // == and != must NOT register as formula (comparison/negation owns those)
    assert!(
      classify_formula("a == b").is_none(),
      "equality op != formula"
    );
    assert!(
      classify_formula("a != b").is_none(),
      "inequality != formula"
    );
    assert!(classify_formula("a >= b").is_none(), ">= != formula");
    assert!(classify_formula("a <= b").is_none(), "<= != formula");
  }

  #[test]
  fn formula_none_on_plain_sentence() {
    assert!(classify_formula("the sky is blue today").is_none());
  }

  #[test]
  fn classifies_english_causality_causes() {
    let hit = classify_causality("Lower mass causes higher acceleration.").expect("cause");
    assert_eq!(hit.matched_marker, "causes");
    assert!(hit.owner_law.ends_with("causality.px"));
  }

  #[test]
  fn classifies_english_causality_because() {
    let hit = classify_causality("It floats because density is lower.").expect("cause");
    assert_eq!(hit.matched_marker, "because");
  }

  #[test]
  fn classifies_korean_causality() {
    let hit = classify_causality("질량이 작기 때문에 가속도가 크다").expect("cause");
    assert_eq!(hit.matched_marker, "때문에");
  }

  #[test]
  fn classifies_symbolic_arrow_causality() {
    let hit = classify_causality("A → B").expect("cause");
    assert_eq!(hit.matched_marker, "→");
  }

  #[test]
  fn causality_none_on_plain_sentence() {
    assert!(classify_causality("the table is brown").is_none());
  }

  #[test]
  fn comparison_kind_predicate_strings_match_owner_law() {
    // These strings are pinned by `comparison.px` — if the owner law
    // changes them, both sides must move together.
    assert_eq!(ComparisonKind::GreaterThan.predicate(), "greater-than");
    assert_eq!(ComparisonKind::LessThan.predicate(), "less-than");
    assert_eq!(ComparisonKind::Equal.predicate(), "equal");
    assert_eq!(
      ComparisonKind::GreaterOrEqual.predicate(),
      "greater-or-equal"
    );
    assert_eq!(ComparisonKind::LessOrEqual.predicate(), "less-or-equal");
  }
}
