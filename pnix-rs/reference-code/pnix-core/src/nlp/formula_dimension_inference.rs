//! Formula dimension inference engine.
//!
//! OWNER-LAW (2026-05-11): the `FormulaDimensionCheckResolution` gate in
//! `ontology.rs` consumes provenance markers
//! (`formula-dimension-check:passed` / `:failed` / `:held`). Something
//! has to *emit* those markers. This module is that something for the
//! deterministic substrate path: it parses lhs / rhs as products of
//! physics symbols raised to integer powers, looks up each symbol's
//! dimension in a fixed table (MLT vector — Mass, Length, Time), and
//! compares the resulting dimension vectors.
//!
//! Scope (deliberately minimal):
//!   - flat products, quotients, integer powers of symbols
//!   - integer literal coefficients (dimensionless)
//!   - parentheses
//!   - the 9-symbol physics-intro table (`F`, `m`, `a`, `v`, `E`, `p`,
//!     `t`, `x`, `c`)
//!
//! Held vs Failed shape:
//!   - parse error on either side → `Held` (uncertainty, not a refutation)
//!   - unknown symbol → `Held` (table gap, not a refutation)
//!   - both sides parse + all symbols known + dimensions agree → `Passed`
//!   - both sides parse + all symbols known + dimensions disagree → `Failed`
//!
//! Not covered (future):
//!   - addition / subtraction (need typecheck for like-dimension operands)
//!   - non-integer exponents (square root would need a richer model)
//!   - vector / tensor / matrix dimensions
//!   - SI base extensions beyond MLT (charge, temperature, etc.)
//!
//! This is the algorithm host the `pnix-core` substrate owns. Other
//! callers (relation classifier, doghouse ingest, freecat-cli memory
//! panel) consume the resolution and propagate the marker into
//! `provenance_refs`, where `resolve_formula_dimension_check` reads it.

use crate::ontology::FormulaDimensionCheckResolution;

/// Dimension as an exponent triple `(mass, length, time)`. Dimensionless
/// quantities are `(0, 0, 0)`. Multiplying two quantities adds the
/// exponents component-wise; dividing subtracts; raising to integer `n`
/// scales by `n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimension {
  pub mass: i32,
  pub length: i32,
  pub time: i32,
}

impl Dimension {
  pub const DIMENSIONLESS: Self = Self {
    mass: 0,
    length: 0,
    time: 0,
  };

  pub fn mul(self, other: Self) -> Self {
    Self {
      mass: self.mass + other.mass,
      length: self.length + other.length,
      time: self.time + other.time,
    }
  }

  pub fn div(self, other: Self) -> Self {
    Self {
      mass: self.mass - other.mass,
      length: self.length - other.length,
      time: self.time - other.time,
    }
  }

  pub fn pow(self, n: i32) -> Self {
    Self {
      mass: self.mass * n,
      length: self.length * n,
      time: self.time * n,
    }
  }
}

/// Look up a physics symbol's dimension. Returns `None` when the
/// symbol is not in the table.
///
/// OWNER-LAW (2026-05-11): kept small and frozen on purpose. Adding a
/// symbol should be a deliberate owner-law slice with its own audit.
pub fn lookup_symbol(name: &str) -> Option<Dimension> {
  Some(match name {
    // Mechanics
    "m" | "M" => Dimension {
      mass: 1,
      length: 0,
      time: 0,
    },
    "x" | "L" => Dimension {
      mass: 0,
      length: 1,
      time: 0,
    },
    "t" => Dimension {
      mass: 0,
      length: 0,
      time: 1,
    },
    "v" => Dimension {
      mass: 0,
      length: 1,
      time: -1,
    },
    "a" => Dimension {
      mass: 0,
      length: 1,
      time: -2,
    },
    "F" => Dimension {
      mass: 1,
      length: 1,
      time: -2,
    },
    "p" => Dimension {
      mass: 1,
      length: 1,
      time: -1,
    },
    "E" => Dimension {
      mass: 1,
      length: 2,
      time: -2,
    },
    "c" => Dimension {
      mass: 0,
      length: 1,
      time: -1,
    },
    _ => return None,
  })
}

/// Inference result for one side of a formula.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SideResult {
  Known(Dimension),
  Held(&'static str),
}

/// Public entry point: given `lhs` and `rhs` strings (already split by
/// the caller around the formula `=`), return the gate verdict. Marker
/// emission is the caller's job — see [`dimension_check_marker`].
///
/// OWNER-LAW (2026-05-11): pure function. Same input → same output.
/// No randomness, no time, no I/O. Replay-stable.
pub fn infer_formula_dimension_check(lhs: &str, rhs: &str) -> FormulaDimensionCheckResolution {
  let lhs_res = parse_side(lhs);
  let rhs_res = parse_side(rhs);
  match (lhs_res, rhs_res) {
    (SideResult::Held(_), _) | (_, SideResult::Held(_)) => {
      // Either side is uncertain → Held, not Failed. Held distinguishes
      // "we don't know" from "we know it's wrong".
      FormulaDimensionCheckResolution::Held
    }
    (SideResult::Known(l), SideResult::Known(r)) => {
      if l == r {
        FormulaDimensionCheckResolution::Passed
      } else {
        FormulaDimensionCheckResolution::Failed
      }
    }
  }
}

/// Split a passage containing an `=` formula marker into `(lhs, rhs)`
/// halves. Returns `None` when the passage has no formula `=` (or only
/// disambiguation markers like `==`, `!=`, `>=`, `<=`, `:=`).
///
/// Splits on the first standalone `=` — i.e. an `=` whose neighbors
/// are not part of a comparison/assignment operator. The match shape
/// mirrors `relation_classifier::passage_has_equation_marker` so this
/// function pairs up with the classifier's detection.
///
/// After splitting, both sides are passed through
/// [`bound_formula_expression`] so a passage like
/// `"F = ma is Newton's second law."` extracts `("F", "ma")` rather
/// than carrying the English suffix through to the parser. The bound
/// is conservative: a word containing any letter not in the symbol
/// table terminates the formula region.
pub fn split_formula_equation(passage: &str) -> Option<(String, String)> {
  let bytes = passage.as_bytes();
  for (i, b) in bytes.iter().enumerate() {
    if *b != b'=' {
      continue;
    }
    let prev = if i > 0 { bytes[i - 1] } else { b' ' };
    let next = bytes.get(i + 1).copied().unwrap_or(b' ');
    // Skip `==`, `!=`, `>=`, `<=`, `:=` — these aren't formula `=`.
    if next == b'=' || prev == b'=' || prev == b'!' || prev == b'>' || prev == b'<' || prev == b':'
    {
      continue;
    }
    let lhs_raw = passage[..i].trim();
    let rhs_raw = passage[i + 1..].trim();
    if lhs_raw.is_empty() || rhs_raw.is_empty() {
      return None;
    }
    // lhs: bound walking *backward* — keep the trailing formula region.
    let lhs = bound_formula_expression_trailing(lhs_raw);
    let rhs = bound_formula_expression(rhs_raw);
    if lhs.is_empty() || rhs.is_empty() {
      return None;
    }
    return Some((lhs.to_string(), rhs.to_string()));
  }
  None
}

/// Bound a string to its leading formula region. Walks whitespace-
/// separated words and keeps each one as long as every alphabetic char
/// is a known single-letter symbol and every non-alphabetic char is a
/// formula operator (`* / ^ ( ) + -`) or digit. Stops at the first word
/// that violates either rule.
///
/// Example: `"ma is Newton's second law."` → `"ma"`.
fn bound_formula_expression(text: &str) -> &str {
  let bytes = text.as_bytes();
  let mut last_good_end: usize = 0;
  let mut i = 0;
  loop {
    // Skip leading whitespace.
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
      i += 1;
    }
    if i >= bytes.len() {
      break;
    }
    // Collect one word: contiguous non-whitespace, formula chars only.
    let mut ok = true;
    while i < bytes.len() && bytes[i] != b' ' && bytes[i] != b'\t' {
      let c = bytes[i];
      if c.is_ascii_alphabetic() {
        let s = std::str::from_utf8(&bytes[i..i + 1]).unwrap_or("?");
        if lookup_symbol(s).is_none() {
          ok = false;
        }
      } else if !(c.is_ascii_digit()
        || c == b'*'
        || c == b'/'
        || c == b'^'
        || c == b'('
        || c == b')'
        || c == b'+'
        || c == b'-')
      {
        ok = false;
      }
      i += 1;
      if !ok {
        break;
      }
    }
    if !ok {
      // This word violates the formula region. Stop before it.
      break;
    }
    last_good_end = i;
  }
  text[..last_good_end].trim_end()
}

/// Like [`bound_formula_expression`] but walks backward from the end —
/// keeps the trailing formula region. Used for lhs (the equation's
/// left side, which is bounded on the right by `=` and on the left by
/// whatever precedes it).
fn bound_formula_expression_trailing(text: &str) -> &str {
  let bytes = text.as_bytes();
  let mut first_good_start: usize = bytes.len();
  let mut i = bytes.len();
  loop {
    // Skip trailing whitespace.
    while i > 0 && (bytes[i - 1] == b' ' || bytes[i - 1] == b'\t') {
      i -= 1;
    }
    if i == 0 {
      break;
    }
    let word_end = i;
    let mut ok = true;
    while i > 0 && bytes[i - 1] != b' ' && bytes[i - 1] != b'\t' {
      let c = bytes[i - 1];
      if c.is_ascii_alphabetic() {
        let s = std::str::from_utf8(&bytes[i - 1..i]).unwrap_or("?");
        if lookup_symbol(s).is_none() {
          ok = false;
        }
      } else if !(c.is_ascii_digit()
        || c == b'*'
        || c == b'/'
        || c == b'^'
        || c == b'('
        || c == b')'
        || c == b'+'
        || c == b'-')
      {
        ok = false;
      }
      i -= 1;
      if !ok {
        break;
      }
    }
    if !ok {
      // Stop after this word — don't include it.
      let _ = word_end;
      break;
    }
    first_good_start = i;
  }
  text[first_good_start..].trim_start()
}

/// Combined entry-point: detect-and-infer for an entire passage. Calls
/// [`split_formula_equation`] to extract `(lhs, rhs)` and then
/// [`infer_formula_dimension_check`].
///
/// Returns `None` when the passage has no formula equation. Returns
/// `Some(resolution)` otherwise — `Held` includes the parse-failure
/// case, so a non-`None` return always carries a resolution caller
/// can act on (push a marker, run the gate, build an audit entry).
pub fn infer_passage_dimension_check(passage: &str) -> Option<FormulaDimensionCheckResolution> {
  let (lhs, rhs) = split_formula_equation(passage)?;
  Some(infer_formula_dimension_check(&lhs, &rhs))
}

/// Map a resolution onto the canonical provenance marker the
/// `resolve_formula_dimension_check` consumer expects. Returns `None`
/// for `NotApplicable` (no marker should be emitted — the gate does
/// not apply to that fact).
///
/// Callers that have computed a resolution via
/// [`infer_formula_dimension_check`] should push the returned string
/// into the `ContextualFact.provenance_refs` and re-evaluate.
pub fn dimension_check_marker(res: FormulaDimensionCheckResolution) -> Option<&'static str> {
  match res {
    FormulaDimensionCheckResolution::NotApplicable => None,
    FormulaDimensionCheckResolution::Passed => Some("formula-dimension-check:passed"),
    FormulaDimensionCheckResolution::Failed => Some("formula-dimension-check:failed"),
    FormulaDimensionCheckResolution::Held => Some("formula-dimension-check:held"),
    FormulaDimensionCheckResolution::Missing => Some("formula-dimension-check:held"),
  }
}

// ─── parser ──────────────────────────────────────────────────────────

fn parse_side(s: &str) -> SideResult {
  let tokens = match tokenize(s) {
    Ok(t) => t,
    Err(reason) => return SideResult::Held(reason),
  };
  if tokens.is_empty() {
    return SideResult::Held("empty side");
  }
  let mut cursor = 0;
  let result = match parse_expr(&tokens, &mut cursor) {
    Ok(d) => d,
    Err(reason) => return SideResult::Held(reason),
  };
  if cursor != tokens.len() {
    return SideResult::Held("trailing tokens after expression");
  }
  SideResult::Known(result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
  Symbol(String),
  Integer(i32),
  LParen,
  RParen,
  Star,
  Slash,
  Caret,
}

fn tokenize(s: &str) -> Result<Vec<Token>, &'static str> {
  let mut tokens = Vec::new();
  let bytes = s.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    let b = bytes[i];
    match b {
      b' ' | b'\t' => {
        i += 1;
      }
      b'(' => {
        tokens.push(Token::LParen);
        i += 1;
      }
      b')' => {
        tokens.push(Token::RParen);
        i += 1;
      }
      b'*' => {
        tokens.push(Token::Star);
        i += 1;
      }
      b'/' => {
        tokens.push(Token::Slash);
        i += 1;
      }
      b'^' => {
        tokens.push(Token::Caret);
        i += 1;
      }
      // Integer literal (no decimal; signed handled by `^-1` pattern below)
      b'0'..=b'9' => {
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
          i += 1;
        }
        let n: i32 = std::str::from_utf8(&bytes[start..i])
          .map_err(|_| "non-utf8 integer")?
          .parse()
          .map_err(|_| "integer literal out of range")?;
        tokens.push(Token::Integer(n));
      }
      // Single-character symbol — physics-intro convention is
      // single-letter symbols (`F`, `m`, `a`, `v`, `E`, `p`, `t`,
      // `x`, `c`). One ASCII alphabetic byte = one Symbol token. This
      // makes implicit multiplication work directly: `ma` →
      // `Symbol("m") Symbol("a")` → parser sees adjacent atoms and
      // multiplies. Multi-character symbol names are out of scope for
      // this slice (extending to `\mathrm{Force}` style would need a
      // separate lexical mode).
      _ if b.is_ascii_alphabetic() => {
        tokens.push(Token::Symbol(
          std::str::from_utf8(&bytes[i..i + 1])
            .map_err(|_| "non-utf8 symbol")?
            .to_string(),
        ));
        i += 1;
      }
      // Unary minus inside a power: `t^-2` should tokenize as
      // `t Caret Integer(-2)`. Detect by context: a `-` directly after
      // a `^` consumes the following digits as a negative integer.
      b'-' if matches!(tokens.last(), Some(Token::Caret)) => {
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
          i += 1;
        }
        if start == i {
          return Err("expected integer after `-` in exponent");
        }
        let n: i32 = std::str::from_utf8(&bytes[start..i])
          .map_err(|_| "non-utf8 negative integer")?
          .parse()
          .map_err(|_| "negative integer literal out of range")?;
        tokens.push(Token::Integer(-n));
      }
      _ => return Err("unexpected character in formula side"),
    }
  }
  Ok(tokens)
}

// Grammar (recursive descent, integer exponent only):
//   expr   = term  (('*' | '/') term)*
//   term   = atom  ('^' Integer)?
//   atom   = Symbol | Integer | '(' expr ')'

fn parse_expr(tokens: &[Token], cursor: &mut usize) -> Result<Dimension, &'static str> {
  let mut acc = parse_term(tokens, cursor)?;
  loop {
    match tokens.get(*cursor) {
      Some(Token::Star) => {
        *cursor += 1;
        let rhs = parse_term(tokens, cursor)?;
        acc = acc.mul(rhs);
      }
      Some(Token::Slash) => {
        *cursor += 1;
        let rhs = parse_term(tokens, cursor)?;
        acc = acc.div(rhs);
      }
      // Implicit multiplication: `ma` parses as `m * a`, `mc^2` parses
      // as `m * c^2`. Standard physics-notation convention. Triggered
      // when a term is followed directly by a Symbol / Integer / `(`
      // without an intervening operator.
      Some(Token::Symbol(_)) | Some(Token::Integer(_)) | Some(Token::LParen) => {
        let rhs = parse_term(tokens, cursor)?;
        acc = acc.mul(rhs);
      }
      _ => break,
    }
  }
  Ok(acc)
}

fn parse_term(tokens: &[Token], cursor: &mut usize) -> Result<Dimension, &'static str> {
  let atom = parse_atom(tokens, cursor)?;
  if matches!(tokens.get(*cursor), Some(Token::Caret)) {
    *cursor += 1;
    match tokens.get(*cursor) {
      Some(Token::Integer(n)) => {
        *cursor += 1;
        Ok(atom.pow(*n))
      }
      _ => Err("expected integer exponent after `^`"),
    }
  } else {
    Ok(atom)
  }
}

fn parse_atom(tokens: &[Token], cursor: &mut usize) -> Result<Dimension, &'static str> {
  match tokens.get(*cursor) {
    Some(Token::Symbol(name)) => {
      let name = name.clone();
      *cursor += 1;
      lookup_symbol(&name).ok_or("unknown symbol")
    }
    Some(Token::Integer(_)) => {
      *cursor += 1;
      Ok(Dimension::DIMENSIONLESS)
    }
    Some(Token::LParen) => {
      *cursor += 1;
      let inner = parse_expr(tokens, cursor)?;
      match tokens.get(*cursor) {
        Some(Token::RParen) => {
          *cursor += 1;
          Ok(inner)
        }
        _ => Err("unmatched `(`"),
      }
    }
    _ => Err("expected symbol, integer, or `(` in formula side"),
  }
}

// ─── tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn newton_second_law_passes() {
    // F = m * a → force = mass × acceleration
    assert_eq!(
      infer_formula_dimension_check("F", "m*a"),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn mass_energy_equivalence_passes() {
    // E = m * c^2 → energy = mass × speed²
    assert_eq!(
      infer_formula_dimension_check("E", "m*c^2"),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn momentum_passes() {
    // p = m * v → momentum = mass × velocity
    assert_eq!(
      infer_formula_dimension_check("p", "m*v"),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn velocity_passes() {
    // v = x / t → velocity = length / time
    assert_eq!(
      infer_formula_dimension_check("v", "x/t"),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn acceleration_passes() {
    // a = v / t → length·time⁻² = length·time⁻¹ / time
    assert_eq!(
      infer_formula_dimension_check("a", "v/t"),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn kinetic_energy_with_dimensionless_half_passes() {
    // E ?= (1/2) * m * v^2 — the (1/2) is dimensionless, so this
    // should still pass on dimensions alone. We model `2` as
    // dimensionless and `1/2` as dimensionless / dimensionless.
    assert_eq!(
      infer_formula_dimension_check("E", "(1/2)*m*v^2"),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn energy_equals_mass_times_velocity_fails() {
    // E = m * v → mass·length²·time⁻² vs mass·length·time⁻¹ (mismatch)
    assert_eq!(
      infer_formula_dimension_check("E", "m*v"),
      FormulaDimensionCheckResolution::Failed
    );
  }

  #[test]
  fn force_equals_mass_times_velocity_fails() {
    // F = m * v → mass·length·time⁻² vs mass·length·time⁻¹
    assert_eq!(
      infer_formula_dimension_check("F", "m*v"),
      FormulaDimensionCheckResolution::Failed
    );
  }

  #[test]
  fn momentum_squared_over_mass_is_energy() {
    // E = p^2 / m → (mass·length·time⁻¹)² / mass = mass·length²·time⁻²
    assert_eq!(
      infer_formula_dimension_check("E", "p^2/m"),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn unknown_symbol_yields_held_not_failed() {
    // `Q` is not in the table → uncertainty, not refutation.
    assert_eq!(
      infer_formula_dimension_check("F", "Q*a"),
      FormulaDimensionCheckResolution::Held
    );
  }

  #[test]
  fn parse_error_yields_held() {
    // Mismatched paren → Held.
    assert_eq!(
      infer_formula_dimension_check("F", "(m*a"),
      FormulaDimensionCheckResolution::Held
    );
    // Empty side → Held.
    assert_eq!(
      infer_formula_dimension_check("", "m*a"),
      FormulaDimensionCheckResolution::Held
    );
  }

  #[test]
  fn negative_exponent_in_power_parses() {
    // t^-1 = 1/t (time⁻¹) — same dimension as velocity / length
    // Test that the tokenizer + parser handle `^-1` correctly.
    assert_eq!(
      infer_formula_dimension_check("v/x", "t^-1"),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn dimension_check_marker_maps_each_variant() {
    assert_eq!(
      dimension_check_marker(FormulaDimensionCheckResolution::NotApplicable),
      None
    );
    assert_eq!(
      dimension_check_marker(FormulaDimensionCheckResolution::Passed),
      Some("formula-dimension-check:passed")
    );
    assert_eq!(
      dimension_check_marker(FormulaDimensionCheckResolution::Failed),
      Some("formula-dimension-check:failed")
    );
    assert_eq!(
      dimension_check_marker(FormulaDimensionCheckResolution::Held),
      Some("formula-dimension-check:held")
    );
    // Missing maps to :held because Missing means "we don't have a
    // result yet" — the resulting marker is the held one, not a
    // separate `:missing` (the consumer's Missing variant arises from
    // the *absence* of any result marker, so emitting `:held` for an
    // engine-side computed Missing is the canonical lowering).
    assert_eq!(
      dimension_check_marker(FormulaDimensionCheckResolution::Missing),
      Some("formula-dimension-check:held")
    );
  }

  #[test]
  fn round_trip_inference_then_resolve_via_provenance() {
    // E = m*c^2 → infer Passed → marker `formula-dimension-check:passed`
    // → push into provenance → `resolve_formula_dimension_check` returns
    // Passed. This is the contract this engine fulfills.
    use crate::ontology::{
      resolve_formula_dimension_check, ContextId, ContextualFact, LayerId, MeaningId, MeaningStatus,
    };
    let res = infer_formula_dimension_check("E", "m*c^2");
    assert_eq!(res, FormulaDimensionCheckResolution::Passed);
    let marker = dimension_check_marker(res).expect("Passed has a marker");
    let f = ContextualFact {
      id: Some(MeaningId::from("fact.round-trip.1".to_string())),
      context: ContextId::from("Physics"),
      layer: LayerId::from("L2"),
      subj: "E".to_string(),
      pred: "formula".to_string(),
      obj: "m*c^2".to_string(),
      status: MeaningStatus::Candidate,
      confidence: 0.9,
      provenance_refs: vec![marker.to_string()],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: None,
      timestamp: None,
    };
    assert_eq!(
      resolve_formula_dimension_check(&f),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn round_trip_failed_inference_clamps_in_resolver() {
    // E = m*v → engine says Failed → marker `:failed` → resolver agrees.
    use crate::ontology::{
      resolve_formula_dimension_check, ContextId, ContextualFact, LayerId, MeaningId, MeaningStatus,
    };
    let res = infer_formula_dimension_check("E", "m*v");
    assert_eq!(res, FormulaDimensionCheckResolution::Failed);
    let marker = dimension_check_marker(res).expect("Failed has a marker");
    let f = ContextualFact {
      id: Some(MeaningId::from("fact.round-trip.2".to_string())),
      context: ContextId::from("Physics"),
      layer: LayerId::from("L2"),
      subj: "E".to_string(),
      pred: "formula".to_string(),
      obj: "m*v".to_string(),
      status: MeaningStatus::Candidate,
      confidence: 0.5,
      provenance_refs: vec![marker.to_string()],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: None,
      timestamp: None,
    };
    assert_eq!(
      resolve_formula_dimension_check(&f),
      FormulaDimensionCheckResolution::Failed
    );
  }

  // ─── implicit multiplication ──────────────────────────────────────

  #[test]
  fn implicit_multiplication_ma_parses_as_m_times_a() {
    // Physics-textbook notation: `F = ma` (no `*` between symbols).
    assert_eq!(
      infer_formula_dimension_check("F", "ma"),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn implicit_multiplication_with_power_mc_squared() {
    // `E = mc^2` — `m` and `c^2` joined without `*`.
    assert_eq!(
      infer_formula_dimension_check("E", "mc^2"),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn implicit_multiplication_three_factors() {
    // `m v t` = `m * v * t` (mass * length/time * time = mass * length)
    let m_v_t = lookup_symbol("m")
      .unwrap()
      .mul(lookup_symbol("v").unwrap())
      .mul(lookup_symbol("t").unwrap());
    // mass * length/time * time = mass * length
    assert_eq!(
      m_v_t,
      Dimension {
        mass: 1,
        length: 1,
        time: 0
      }
    );
    // Same dimension as p*x/... no convenient lhs. Use direct compare:
    // `m v t` vs `m x` (mass * length).
    assert_eq!(
      infer_formula_dimension_check("m x", "m v t"),
      FormulaDimensionCheckResolution::Passed
    );
  }

  // ─── passage split + passage-level inference ──────────────────────

  #[test]
  fn split_formula_equation_basic() {
    assert_eq!(
      split_formula_equation("F = ma"),
      Some(("F".to_string(), "ma".to_string()))
    );
  }

  #[test]
  fn split_formula_equation_trims_whitespace() {
    assert_eq!(
      split_formula_equation("  E    =    m*c^2  "),
      Some(("E".to_string(), "m*c^2".to_string()))
    );
  }

  #[test]
  fn split_formula_equation_skips_double_equal() {
    // `==` is a comparison, not a formula assignment.
    assert_eq!(split_formula_equation("a == b"), None);
  }

  #[test]
  fn split_formula_equation_skips_assignment_operators() {
    // `:=`, `!=`, `>=`, `<=` are all skipped.
    assert_eq!(split_formula_equation("x := 5"), None);
    assert_eq!(split_formula_equation("a != b"), None);
    assert_eq!(split_formula_equation("a >= b"), None);
    assert_eq!(split_formula_equation("a <= b"), None);
  }

  #[test]
  fn split_formula_equation_returns_none_when_no_equals() {
    assert_eq!(split_formula_equation("just some text"), None);
  }

  #[test]
  fn split_formula_equation_returns_none_when_one_side_empty() {
    assert_eq!(split_formula_equation("= ma"), None);
    assert_eq!(split_formula_equation("F ="), None);
  }

  #[test]
  fn infer_passage_dimension_check_passes_newton_second_law_passage() {
    // The host gives the full passage; we extract lhs/rhs and infer.
    assert_eq!(
      infer_passage_dimension_check("F = ma"),
      Some(FormulaDimensionCheckResolution::Passed)
    );
  }

  #[test]
  fn infer_passage_dimension_check_passes_einstein_passage() {
    assert_eq!(
      infer_passage_dimension_check("E = mc^2"),
      Some(FormulaDimensionCheckResolution::Passed)
    );
  }

  #[test]
  fn infer_passage_dimension_check_fails_energy_eq_mv() {
    assert_eq!(
      infer_passage_dimension_check("E = mv"),
      Some(FormulaDimensionCheckResolution::Failed)
    );
  }

  #[test]
  fn infer_passage_dimension_check_returns_none_for_non_formula() {
    // No `=` at all.
    assert_eq!(
      infer_passage_dimension_check("force is mass times accel"),
      None
    );
    // Only `==`.
    assert_eq!(infer_passage_dimension_check("a == b"), None);
  }

  #[test]
  fn bound_formula_expression_keeps_only_formula_region() {
    // English suffix is dropped.
    assert_eq!(bound_formula_expression("ma is Newton's second law."), "ma");
    // Whitespace-separated formula tokens are all kept.
    assert_eq!(bound_formula_expression("m a"), "m a");
    // Power expression with English suffix.
    assert_eq!(
      bound_formula_expression("mc^2 according to Einstein"),
      "mc^2"
    );
    // Pure formula stays unchanged.
    assert_eq!(bound_formula_expression("m*v^2 / 2"), "m*v^2 / 2");
  }

  #[test]
  fn bound_formula_expression_trailing_keeps_only_formula_region() {
    // English prefix is dropped.
    assert_eq!(
      bound_formula_expression_trailing("Newton's second law says F"),
      "F"
    );
    // Pure formula stays unchanged.
    assert_eq!(bound_formula_expression_trailing("F"), "F");
  }

  #[test]
  fn infer_passage_dimension_check_passes_passage_with_english_context() {
    // Real-world passage with English context — engine extracts F and ma
    // and infers Passed.
    assert_eq!(
      infer_passage_dimension_check("F = ma is Newton's second law."),
      Some(FormulaDimensionCheckResolution::Passed)
    );
  }

  #[test]
  fn infer_passage_dimension_check_passes_einstein_passage_with_context() {
    assert_eq!(
      infer_passage_dimension_check("Einstein showed that E = mc^2 changed physics."),
      Some(FormulaDimensionCheckResolution::Passed)
    );
  }

  #[test]
  fn infer_passage_dimension_check_returns_none_when_bounded_rhs_is_empty() {
    // `F = Qa` — Q is not in the substrate's table, so the bounded
    // extraction drops the entire rhs (no recognizable formula). The
    // passage-level entry-point returns None — the gate consumer
    // should treat this as "no formula detected", not "formula failed
    // dimension check". For strict unknown-symbol Held semantics, use
    // `infer_formula_dimension_check("F", "Qa")` directly.
    assert_eq!(infer_passage_dimension_check("F = Qa"), None);
    // Strict mode still returns Held — the engine itself is
    // unchanged.
    assert_eq!(
      infer_formula_dimension_check("F", "Qa"),
      FormulaDimensionCheckResolution::Held
    );
  }

  #[test]
  fn dimension_algebra_inverse_of_inverse() {
    // (mass^-1)^-1 = mass^1 = mass
    let m = Dimension {
      mass: 1,
      length: 0,
      time: 0,
    };
    assert_eq!(m.pow(-1).pow(-1), m);
  }

  #[test]
  fn dimension_algebra_mul_div_inverse() {
    // m * v / v = m
    let m = lookup_symbol("m").unwrap();
    let v = lookup_symbol("v").unwrap();
    assert_eq!(m.mul(v).div(v), m);
  }
}
