//! Meta-circular gate: the slice #31-#47 invariants fixture must
//! evaluate to an attrset where every value is `true`.
//!
//! This fixture lives at
//! `fixtures/pnix_eval_audit/audit_invariants.px` so any
//! evaluator (Stage 0 native pnix-eval, Stage 1/2 meta-circular,
//! p-puck wrapper) can replay it. A passing evaluator produces an
//! attrset whose `attrValues` are all `true`. A regressed
//! evaluator produces some `false` cells — each `false` names the
//! slice and case that regressed.
//!
//! 2026-05-05 (slice #48): consolidated all behavioural invariants
//! from slices #31-#47 into a single `.px` fixture for the
//! meta-circular round-trip gate. Each invariant is asserted via
//! either `(builtins.tryEval expr).success == false` (for
//! should-error cases) or direct value equality (for should-succeed
//! cases). The fixture is self-contained — uses only the builtins
//! that the audit covers, no external dependencies.
//!
//! Truth-owner files:
//! - `fixtures/pnix_eval_audit/audit_invariants.px` — the
//!   fixture itself. Editable as the audit grows; new slices add
//!   cells with `<NN>_<letter>_<slug>` keys.

use pnix_eval::{eval_expr, Value};
use std::fs;
use std::path::PathBuf;

fn workspace_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("..")
    .join("..")
}

fn fixture_path() -> PathBuf {
  workspace_dir()
    .join("fixtures")
    .join("pnix_eval_audit")
    .join("audit_invariants.px")
}

#[test]
fn audit_invariants_fixture_all_true() {
  let path = fixture_path();
  let src = fs::read_to_string(&path)
    .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
  let value =
    eval_expr(&src).unwrap_or_else(|e| panic!("fixture eval failed: {e}\nfixture:\n{src}"));

  let Value::AttrSet(map) = value else {
    panic!("fixture must evaluate to an attrset, got {}", value);
  };

  let mut failures: Vec<String> = Vec::new();
  for (key, cell) in map.iter() {
    match cell {
      Value::Bool(true) => continue,
      Value::Bool(false) => failures.push(format!("  {} = false", key)),
      other => failures.push(format!("  {} = {} (expected Bool)", key, other.to_json())),
    }
  }

  if !failures.is_empty() {
    panic!(
      "audit invariants fixture has {} regressed cell(s) out of {}:\n{}",
      failures.len(),
      map.len(),
      failures.join("\n")
    );
  }
}

#[test]
fn audit_invariants_fixture_covers_every_slice() {
  // Defensive: the fixture must mention every audit slice. If a
  // future slice is added, this test fails until the fixture is
  // extended. The list of slices below tracks the audit's
  // closed-fix family — update when a new slice closes a fix
  // (and add the slice's invariant cells to the fixture).
  let path = fixture_path();
  let src = fs::read_to_string(&path).expect("read fixture");
  // Slice #48 is the milestone slice (fixture itself, no new
  // fix), so it has no cell prefix in the fixture body — skip
  // it. Slice #49 onwards re-enters the additive sequence.
  let expected_slices: &[u32] = &[
    31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 49, 50, 51, 52, 53, 54, 55,
    56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
    80,
  ];
  for &slice_num in expected_slices {
    let prefix = format!("{}_", slice_num);
    assert!(
      src.contains(&prefix),
      "fixture missing slice #{} cells (no key starting with `\"{}_...`)",
      slice_num,
      slice_num
    );
  }
}
