//! minimal-ontology-tesseract-v0.5.1 — proof-hygiene patches
//! on top of v0.5's two-lineage convergence closure. Codex
//! 2026-05-06 directive: import whitelist + content-sensitive
//! SI id + SI shape wording + positive/negative interpret
//! parity. NO Stage 2 cross-application implementation
//! (deferred to v0.5.2; this slice only pins design lock).
//!
//! Truth owner:        project-wiki/maps/minimal-ontology-tesseract-v0-map.md
//!                     §"v0.5.1 design decision — proof-hygiene
//!                       + Stage 2 cross-application design-lock"
//! Active scope:       project-wiki/maps/active-domain-constitution.md
//!                     Art. 6, Art. 7
//!
//! Test count: 5 invariants, indices 351..355.

use pnix_eval::{eval_expr, eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — same shape as v0.5 test file.
// ---------------------------------------------------------------

fn v0_5_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/meta-interpret-v0_5")
}

fn context_path() -> PathBuf {
  v0_5_root().join("v0_5_context.px")
}

fn meta_interpret_path() -> PathBuf {
  v0_5_root().join("v0_5_meta_interpret.px")
}

fn run_path() -> PathBuf {
  v0_5_root().join("v0_5_run.px")
}

fn run_negative_path() -> PathBuf {
  v0_5_root().join("v0_5_run_negative.px")
}

fn as_attrs(v: &Value) -> &BTreeMap<String, Value> {
  match v {
    Value::AttrSet(m) => m,
    other => panic!("expected attrset, got {:?}", other),
  }
}

fn as_str(v: &Value) -> &str {
  match v {
    Value::String(s) => s,
    Value::StringContext { text, .. } => text,
    other => panic!("expected string, got {:?}", other),
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

fn get_path<'a>(root: &'a Value, path: &[&str]) -> &'a Value {
  let mut cur = root;
  for key in path {
    cur = get(cur, key);
  }
  cur
}

fn collect_imports(content: &str) -> BTreeSet<String> {
  let mut imports = BTreeSet::new();
  for line in content.lines() {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
      continue;
    }
    if let Some(idx) = trimmed.find("import ") {
      let rest = &trimmed[idx + "import ".len()..];
      let tok = rest
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(';')
        .trim();
      if !tok.is_empty() {
        imports.insert(tok.to_string());
      }
    }
  }
  imports
}

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_5_1_strict_import_whitelist_load_bearing() {
  // Invariant 351.
  // Pin the exact import set of every v0.5 fixture file.
  // The cross-lineage anchor is v0_owner_law.px alone; no
  // other file under fixtures/minimal-tesseract-v0/ may be
  // transitively imported by v0.5.

  let context_content = std::fs::read_to_string(context_path()).unwrap();
  let meta_content = std::fs::read_to_string(meta_interpret_path()).unwrap();
  let run_content = std::fs::read_to_string(run_path()).unwrap();
  let neg_content = std::fs::read_to_string(run_negative_path()).unwrap();

  let context_imports = collect_imports(&context_content);
  let meta_imports = collect_imports(&meta_content);
  let run_imports = collect_imports(&run_content);
  let neg_imports = collect_imports(&neg_content);

  // v0_5_context.px: no imports
  assert!(
    context_imports.is_empty(),
    "v0_5_context.px must have NO imports (it's a data file)"
  );

  // v0_5_meta_interpret.px: exactly the cross-lineage anchor
  let expected_meta: BTreeSet<String> = ["../minimal-tesseract-v0/v0_owner_law.px"]
    .iter()
    .map(|s| s.to_string())
    .collect();
  assert_eq!(
    meta_imports, expected_meta,
    "v0_5_meta_interpret.px must import EXACTLY {{../minimal-tesseract-v0/v0_owner_law.px}} — the cross-lineage anchor; no broader cross-lineage drift"
  );

  // v0_5_run.px
  let expected_run: BTreeSet<String> = [
    "./v0_5_context.px",
    "./v0_5_meta_interpret.px",
    "../minimal-ontology-tesseract-v0/v0_3_run.px",
  ]
  .iter()
  .map(|s| s.to_string())
  .collect();
  assert_eq!(
    run_imports, expected_run,
    "v0_5_run.px must import EXACTLY {{./v0_5_context.px, ./v0_5_meta_interpret.px, ../minimal-ontology-tesseract-v0/v0_3_run.px}}"
  );

  // v0_5_run_negative.px
  let expected_neg: BTreeSet<String> = [
    "./v0_5_context.px",
    "./v0_5_meta_interpret.px",
    "./v0_5_run.px",
    "../minimal-ontology-tesseract-v0/v0_3_run_negative.px",
    "../minimal-ontology-tesseract-v0/v0_3_run.px",
  ]
  .iter()
  .map(|s| s.to_string())
  .collect();
  assert_eq!(
    neg_imports, expected_neg,
    "v0_5_run_negative.px must import EXACTLY 5 files"
  );

  // Convergence-anchor sanity: only v0_owner_law.px from the
  // minimal-tesseract-v0 lineage appears anywhere in v0.5
  // imports. No other v0_*.px from that lineage.
  let all_imports: BTreeSet<&String> = context_imports
    .iter()
    .chain(meta_imports.iter())
    .chain(run_imports.iter())
    .chain(neg_imports.iter())
    .collect();
  for imp in &all_imports {
    if imp.contains("minimal-tesseract-v0") {
      assert!(
        imp.ends_with("/v0_owner_law.px"),
        "the only allowed minimal-tesseract-v0 lineage import is v0_owner_law.px; found {:?}",
        imp
      );
    }
  }
}

#[test]
fn v0_5_1_si_shape_nine_metadata_keys_alone() {
  // Invariant 352.
  // Stage 1 alone (`buildSpecializedInterpreter context`)
  // returns the 9-metadata-key SI, with NO `interpret` key.
  // The `interpret` lambda is added by Stage 2 wiring in
  // v0_5_run.px's `attachInterpret`.
  //
  // v0.5 invariant 342 verifies the runtime-exposed 10-key
  // shape (9 metadata + interpret). 352 verifies the
  // pre-attach 9-key shape directly via Stage 1 evaluation.

  // Evaluate v0_5_meta_interpret.px and v0_5_context.px and
  // call `metaInterpret canonical` directly.
  let source = format!(
    "let mod = import {meta:?}; ctx = (import {ctx:?}).canonical; in mod.metaInterpret ctx",
    meta = meta_interpret_path(),
    ctx = context_path(),
  );
  let stage1_si =
    eval_expr(&source).expect("metaInterpret(canonical) must evaluate via Stage 1 alone");
  let keys: BTreeSet<&str> = as_attrs(&stage1_si).keys().map(|s| s.as_str()).collect();
  let expected: BTreeSet<&str> = [
    "id",
    "source-kind",
    "active-lens-set",
    "visible-capabilities",
    "active-need",
    "rule-functions",
    "expected-receipt-shape",
    "forbidden-actions",
    "owner-refs",
  ]
  .into_iter()
  .collect();
  assert_eq!(
    keys, expected,
    "Stage 1 alone (buildSpecializedInterpreter context) must produce exactly 9 metadata keys; no `interpret` lambda yet (Stage 2 wires that in v0_5_run.px)"
  );
}

#[test]
fn v0_5_1_content_sensitive_si_id_lens_load_bearing() {
  // Invariant 353.
  // Two contexts identical except active-lens-set content
  // produce DIFFERENT ids. The new `lens-content:` segment
  // captures the content; the old `lens:<count>` segment
  // stays the same when counts match.
  //
  // Also check: content-order matters (same elements in
  // different order produce different ids).

  let mk_id = |lens_set: &str| -> String {
    let source = format!(
      "let mod = import {meta:?}; \
        base = (import {ctx:?}).canonical; \
        ctx2 = base // {{ active-lens-set = {lens}; }}; \
        in (mod.metaInterpret ctx2).id",
      meta = meta_interpret_path(),
      ctx = context_path(),
      lens = lens_set,
    );
    let v = eval_expr(&source).unwrap();
    as_str(&v).to_string()
  };

  let id_a = mk_id("[ \"lens.A\" ]");
  let id_b = mk_id("[ \"lens.B\" ]");
  let id_ab = mk_id("[ \"lens.A\" \"lens.B\" ]");
  let id_ba = mk_id("[ \"lens.B\" \"lens.A\" ]");

  assert_ne!(
    id_a, id_b,
    "different lens content {{lens.A}} vs {{lens.B}} must produce different SI ids"
  );
  assert_ne!(
    id_ab, id_ba,
    "lens content order must matter: [lens.A,lens.B] vs [lens.B,lens.A] must produce different ids (deterministic ordering pinned)"
  );

  // Both single-element ids should contain `lens-content:`
  // segment with the literal content.
  assert!(id_a.contains("lens-content:lens.A"));
  assert!(id_b.contains("lens-content:lens.B"));
  // Both should keep the count segment (backward-compat with
  // v0.5 invariant 343 spot-check).
  assert!(id_a.contains("lens:1"));
  assert!(id_b.contains("lens:1"));
}

#[test]
fn v0_5_1_content_sensitive_si_id_roles_load_bearing() {
  // Invariant 354.
  // role-tags content variation must produce different SI
  // ids on the `roles-content:` segment.

  let mk_id = |roles_set: &str| -> String {
    let source = format!(
      "let mod = import {meta:?}; \
        base = (import {ctx:?}).canonical; \
        ctx2 = base // {{ role-tags = {roles}; }}; \
        in (mod.metaInterpret ctx2).id",
      meta = meta_interpret_path(),
      ctx = context_path(),
      roles = roles_set,
    );
    let v = eval_expr(&source).unwrap();
    as_str(&v).to_string()
  };

  let id_x = mk_id("[ \"role.X\" ]");
  let id_y = mk_id("[ \"role.Y\" ]");

  assert_ne!(
    id_x, id_y,
    "different role-tags content {{role.X}} vs {{role.Y}} must produce different SI ids"
  );
  assert!(id_x.contains("roles-content:role.X"));
  assert!(id_y.contains("roles-content:role.Y"));
  assert!(id_x.contains("roles:1"));
  assert!(id_y.contains("roles:1"));
}

#[test]
fn v0_5_1_positive_negative_interpret_parity() {
  // Invariant 355.
  // The positive runner (v0_5_run.px) and negative runner
  // (v0_5_run_negative.px) both build a canonical SI from
  // the same canonical context. Their `interpret`
  // dispatchers differ in COVERAGE (positive: A/B only;
  // negative: A/B/unknown/bareConstraint) but the SI
  // METADATA must be identical because the same
  // `metaInterpret` is applied to the same context.
  //
  // We assert metadata parity (id / source-kind /
  // rule-functions key set / forbidden-actions / owner-refs
  // / active-need / active-lens-set / visible-capabilities
  // / expected-receipt-shape) on the 9 non-`interpret`
  // keys. The `interpret` lambda itself is opaque to
  // Value::to_json comparison (Lambda inequality), so
  // invariant 355 narrows to metadata parity, NOT lambda
  // identity. Lambda dispatch parity on overlapping inputs
  // A/B is covered transitively via v0.5 invariants 346/347
  // (positive SI.interpret(A/B) byte-equal v0.3 fold) and
  // v0.3 fold determinism — both dispatchers route to the
  // SAME pre-built v0.3 fold output for inputs A and B.

  let pos = eval_file(&run_path()).unwrap();
  let neg = eval_file(&run_negative_path()).unwrap();

  let pos_si = get(&pos, "specialized-interpreter");
  let neg_si = get(&neg, "canonical-si");

  // Required metadata keys present on both SIs (positive
  // SI has 10 keys including `interpret`; negative
  // canonical-si has 10 keys for the same reason).
  let metadata_keys = [
    "id",
    "source-kind",
    "active-lens-set",
    "visible-capabilities",
    "active-need",
    "rule-functions",
    "expected-receipt-shape",
    "forbidden-actions",
    "owner-refs",
  ];

  for key in &metadata_keys {
    let pos_v = get(pos_si, key);
    let neg_v = get(neg_si, key);
    if *key == "rule-functions" {
      // rule-functions contains Lambdas; compare key set
      // only (Lambdas are opaque under to_json).
      let pos_keys: BTreeSet<&str> = as_attrs(pos_v).keys().map(|s| s.as_str()).collect();
      let neg_keys: BTreeSet<&str> = as_attrs(neg_v).keys().map(|s| s.as_str()).collect();
      assert_eq!(
        pos_keys, neg_keys,
        "positive SI and negative-canonical SI must share the same rule-functions key set (the 7 owner-law builders)"
      );
    } else {
      assert_eq!(
        pos_v.to_json(),
        neg_v.to_json(),
        "positive SI and negative-canonical SI must agree on metadata key `{}` (same metaInterpret on same canonical context)",
        key
      );
    }
  }

  // Specifically pin the SI id parity (already covered by
  // the loop above via key="id", but make it explicit since
  // it's the central content-sensitive identity claim).
  let pos_si_id = as_str(get(pos_si, "id"));
  let neg_si_id = as_str(get(neg_si, "id"));
  assert_eq!(
    pos_si_id, neg_si_id,
    "positive runner and negative runner canonical SIs must have byte-equal ids (deterministic context-key concat on the same canonical context)"
  );

  // Both must have `interpret` Lambdas (Stage 2 wired in
  // both runners via attachInterpret / attachInterpretFull).
  assert!(
    matches!(get(pos_si, "interpret"), Value::Lambda { .. }),
    "positive SI must have `interpret` Lambda"
  );
  assert!(
    matches!(get(neg_si, "interpret"), Value::Lambda { .. }),
    "negative-canonical SI must have `interpret` Lambda"
  );
}
