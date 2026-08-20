//! Ontology-driven owner ↔ carrier sync verifier.
//!
//! OWNER-LAW (2026-05-12): this test replaces the older shell-based
//! `scripts/check-code-transform-owner-carrier-sync.sh` regex guard
//! with a structured Rust verifier that consumes:
//!
//!   1. The `.px` owner law file at `stdlib/lib/gate/code-transform/*.px`
//!      (source of truth — what kebab strings the law declares).
//!   2. Each Rust carrier's `<HeldKind>::ALL` slice + `SUPPORTED_LANGUAGES`
//!      const (what kebab strings the carrier actually emits, derived
//!      via `as_str()` from a typed enum so rustfmt line-wrapping and
//!      variant renames can't drift past the type system).
//!
//! The verifier asserts bidirectional parity per transform. Adding a
//! new transform here is a *single registry row*, not a copy-paste
//! block — see the `transforms_registry()` function below.
//!
//! Why a Rust test instead of shell + grep?
//!   - Structured diffs with both sides surfaced.
//!   - rustfmt multi-line variant wrapping doesn't break it (no regex
//!     on the carrier side; we read from typed `pub const ALL`).
//!   - The registry IS the ontology manifest the project will lean on
//!     for future code-gen (carrier-from-px, wrapper-from-px).
//!
//! Run via `cargo test -p pnix-core --test code_transform_owner_carrier_sync`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pnix_core::algorithm_synthesis::{
  algorithm_sentence_sequence, ankh_retrieval_cache, axis_separation_gate, candidate_row_proposal,
  fact_cue_registry, held_to_query, intent_recognition, macro_fold_gate,
  operation_candidate_mapping, owner_law_gate, parameter_resolution, registry_overlay,
  regression_proof_gate, retrieval_execution, runtime_hot_reload, schema_mapping_gate,
  structural_cue_registry, verb_cue_registry,
};
use pnix_core::code_transform::{
  add_edge, add_extern_decl, add_import, add_node, add_test_stub, change_signature,
  extract_function, inline_function, move_symbol, remove_unused_import, rename_symbol,
};

/// Locate `<repo>/stdlib/lib/gate/code-transform/*.px`. The test binary
/// runs with `CARGO_MANIFEST_DIR == crates/pnix-core`, so the repo root
/// is two levels up.
fn repo_root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("..")
    .join("..")
    .canonicalize()
    .expect("canonicalize repo root")
}

fn read_px(rel: &str) -> String {
  let p = repo_root().join(rel);
  std::fs::read_to_string(&p).unwrap_or_else(|err| panic!("read {}: {err}", p.display()))
}

/// Extract every `held_kind = "..."` literal from a `.px` source. The
/// `.px` ladder uses this exact spelling for every Held/Rejected case.
/// `held_kind = null` (the Ready arm) is skipped, and Nix comment lines
/// (`#`) are skipped so doc-strings explaining the pattern don't
/// pollute the extractor output.
fn px_held_kinds(px_src: &str) -> Vec<String> {
  let mut out = Vec::new();
  for line in px_src.lines() {
    // Skip Nix comment lines so doc strings like `# held_kind = "..."`
    // don't get picked up.
    if line.trim_start().starts_with('#') {
      continue;
    }
    let Some(idx) = line.find("held_kind") else {
      continue;
    };
    let rest = &line[idx + "held_kind".len()..];
    let after_eq = rest.trim_start();
    if !after_eq.starts_with('=') {
      continue;
    }
    let rhs = after_eq[1..].trim_start();
    // String literal? Skip nulls / non-string RHS.
    if !rhs.starts_with('"') {
      continue;
    }
    let body = &rhs[1..];
    let Some(close) = body.find('"') else {
      continue;
    };
    let s = &body[..close];
    if !s.is_empty() {
      out.push(s.to_string());
    }
  }
  out
}

/// Extract a `nameOfList = [ "x" "y" ... ];` declarative list from a
/// `.px` file. Returns the strings in declaration order.
fn px_list(px_src: &str, list_name: &str) -> Vec<String> {
  // Find the line introducing `<list_name> =` at column 2 (the .px
  // owner law convention).
  let mut found_start: Option<usize> = None;
  for (i, line) in px_src.lines().enumerate() {
    if line.starts_with(&format!("  {list_name} =")) {
      found_start = Some(i);
      break;
    }
  }
  let Some(start) = found_start else {
    return Vec::new();
  };
  let lines: Vec<&str> = px_src.lines().collect();
  let mut buf = String::new();
  for line in lines.iter().skip(start) {
    // Strip inline `#` comments so quoted strings inside doc comments
    // don't get picked up by the substring scanner below. Nix string
    // literals don't contain unescaped `#`, so the first `#` outside
    // a string starts a comment.
    let active = match line.find('#') {
      Some(hash) => &line[..hash],
      None => *line,
    };
    buf.push_str(active);
    buf.push('\n');
    if active.trim_end().ends_with(';') {
      break;
    }
  }
  // Now scan for `"..."` substrings inside `buf`.
  let mut out = Vec::new();
  let bytes = buf.as_bytes();
  let mut i = 0usize;
  while i < bytes.len() {
    if bytes[i] == b'"' {
      let start = i + 1;
      let mut j = start;
      while j < bytes.len() && bytes[j] != b'"' {
        j += 1;
      }
      if j < bytes.len() {
        if let Ok(s) = std::str::from_utf8(&bytes[start..j]) {
          out.push(s.to_string());
        }
        i = j + 1;
        continue;
      }
    }
    i += 1;
  }
  out
}

/// Extract quoted strings from a top-level `.px` list whose items may
/// be attrsets with their own semicolons. Unlike `px_list`, this stops
/// only at the list terminator (`  ];`), not at the first field
/// terminator inside an item.
fn px_nested_list_strings(px_src: &str, list_name: &str) -> Vec<String> {
  let mut in_list = false;
  let mut buf = String::new();
  for line in px_src.lines() {
    if !in_list {
      if line.starts_with(&format!("  {list_name} =")) {
        in_list = true;
      } else {
        continue;
      }
    }
    let active = match line.find('#') {
      Some(hash) => &line[..hash],
      None => line,
    };
    buf.push_str(active);
    buf.push('\n');
    if in_list && active.trim() == "];" {
      break;
    }
  }

  let mut out = Vec::new();
  let bytes = buf.as_bytes();
  let mut i = 0usize;
  while i < bytes.len() {
    if bytes[i] == b'"' {
      let start = i + 1;
      let mut j = start;
      while j < bytes.len() && bytes[j] != b'"' {
        j += 1;
      }
      if j < bytes.len() {
        if let Ok(s) = std::str::from_utf8(&bytes[start..j]) {
          out.push(s.to_string());
        }
        i = j + 1;
        continue;
      }
    }
    i += 1;
  }
  out
}

/// `.px` `held_kinds` are deduplicated (the same kind can appear more
/// than once across the ladder, e.g. `target-path-invalid` for both
/// missing and out-of-project conditions). We compare sets, not
/// sequences — the order is incidental to parity.
fn dedup_sorted(mut xs: Vec<String>) -> Vec<String> {
  let set: BTreeSet<String> = xs.drain(..).collect();
  set.into_iter().collect()
}

/// Convert a `&[T]` of held-kind variants to a sorted, deduplicated
/// `Vec<String>` via `as_str`.
fn all_strs<T: Copy>(slice: &[T], as_str: impl Fn(T) -> &'static str) -> Vec<String> {
  let v: Vec<String> = slice.iter().map(|x| as_str(*x).to_string()).collect();
  dedup_sorted(v)
}

/// One owner ↔ carrier pair worth verifying.
struct TransformSync {
  /// Human label used in assertion messages.
  name: &'static str,
  /// `.px` owner law path relative to repo root.
  px_path: &'static str,
  /// `<HeldKind>::ALL` mapped through `as_str()` and sorted.
  carrier_held_kinds: Vec<String>,
  /// `SUPPORTED_LANGUAGES` const from the carrier. `None` for owners
  /// that are language-agnostic (the algorithm-synthesis family
  /// processes NL + code together and does not declare a per-language
  /// surface).
  carrier_supported_langs: Option<&'static [&'static str]>,
  /// Extra `(list-name-in-.px, expected-values-from-carrier)` pairs.
  /// e.g. for change-signature this carries
  /// `("validChangeKinds", ChangeSignatureKind::ALL → kebab strings)`.
  /// For rename-symbol and remove-unused-import this carries the scope
  /// enum values. Empty for transforms without auxiliary enums.
  extra_lists: Vec<(&'static str, Vec<String>)>,
}

fn carrier_held_kinds_rename() -> Vec<String> {
  all_strs(rename_symbol::RenameHeldKind::ALL, |k| k.as_str())
}
fn carrier_held_kinds_rmi() -> Vec<String> {
  all_strs(remove_unused_import::RemoveUnusedImportHeldKind::ALL, |k| {
    k.as_str()
  })
}
fn carrier_held_kinds_ats() -> Vec<String> {
  all_strs(add_test_stub::AddTestStubHeldKind::ALL, |k| k.as_str())
}
fn carrier_held_kinds_ai() -> Vec<String> {
  all_strs(add_import::AddImportHeldKind::ALL, |k| k.as_str())
}
fn carrier_held_kinds_aed() -> Vec<String> {
  all_strs(add_extern_decl::AddExternDeclHeldKind::ALL, |k| k.as_str())
}
fn carrier_held_kinds_an() -> Vec<String> {
  all_strs(add_node::AddNodeHeldKind::ALL, |k| k.as_str())
}
fn carrier_held_kinds_ae() -> Vec<String> {
  all_strs(add_edge::AddEdgeHeldKind::ALL, |k| k.as_str())
}
fn carrier_held_kinds_ef() -> Vec<String> {
  all_strs(extract_function::ExtractFunctionHeldKind::ALL, |k| {
    k.as_str()
  })
}
fn carrier_held_kinds_if() -> Vec<String> {
  all_strs(inline_function::InlineFunctionHeldKind::ALL, |k| k.as_str())
}
fn carrier_held_kinds_ms() -> Vec<String> {
  all_strs(move_symbol::MoveSymbolHeldKind::ALL, |k| k.as_str())
}
fn carrier_held_kinds_cs() -> Vec<String> {
  all_strs(change_signature::ChangeSignatureHeldKind::ALL, |k| {
    k.as_str()
  })
}

fn transforms_registry() -> Vec<TransformSync> {
  vec![
    TransformSync {
      name: "rename-symbol",
      px_path: "stdlib/lib/gate/code-transform/rename-symbol.px",
      carrier_held_kinds: carrier_held_kinds_rename(),
      carrier_supported_langs: Some(rename_symbol::SUPPORTED_LANGUAGES),
      extra_lists: vec![(
        "validScopes",
        all_strs(rename_symbol::RenameScope::ALL, |k| k.as_str()),
      )],
    },
    TransformSync {
      name: "remove-unused-import",
      px_path: "stdlib/lib/gate/code-transform/remove-unused-import.px",
      carrier_held_kinds: carrier_held_kinds_rmi(),
      carrier_supported_langs: Some(remove_unused_import::SUPPORTED_LANGUAGES),
      extra_lists: vec![(
        "validScopes",
        all_strs(remove_unused_import::RemoveUnusedImportScope::ALL, |k| {
          k.as_str()
        }),
      )],
    },
    TransformSync {
      name: "add-test-stub",
      px_path: "stdlib/lib/gate/code-transform/add-test-stub.px",
      carrier_held_kinds: carrier_held_kinds_ats(),
      carrier_supported_langs: Some(add_test_stub::SUPPORTED_LANGUAGES),
      extra_lists: vec![(
        "validPlaces",
        all_strs(add_test_stub::AddTestStubPlace::ALL, |k| k.as_str()),
      )],
    },
    TransformSync {
      name: "add-import",
      px_path: "stdlib/lib/gate/code-transform/add-import.px",
      carrier_held_kinds: carrier_held_kinds_ai(),
      carrier_supported_langs: Some(add_import::SUPPORTED_LANGUAGES),
      extra_lists: vec![(
        "validIfAlreadyPresent",
        all_strs(add_import::AddImportIfAlreadyPresent::ALL, |k| k.as_str()),
      )],
    },
    // First pnix-native transform — Phase 5-C. Mirrors
    // `stdlib/lib/gate/code-transform/add-extern-decl.px` and
    // `crates/pnix-core/src/code_transform/add_extern_decl.rs`.
    TransformSync {
      name: "add-extern-decl",
      px_path: "stdlib/lib/gate/code-transform/add-extern-decl.px",
      carrier_held_kinds: carrier_held_kinds_aed(),
      carrier_supported_langs: Some(add_extern_decl::SUPPORTED_LANGUAGES),
      extra_lists: vec![(
        "validIfAlreadyPresent",
        all_strs(add_extern_decl::AddExternDeclIfAlreadyPresent::ALL, |k| {
          k.as_str()
        }),
      )],
    },
    // Second pnix-native transform — Phase 5-C continuation.
    // Mirrors `stdlib/lib/gate/code-transform/add-node.px` and
    // `crates/pnix-core/src/code_transform/add_node.rs`. Pairs
    // with add-extern-decl (declares the primitive) and the
    // future add-edge (wires nodes).
    TransformSync {
      name: "add-node",
      px_path: "stdlib/lib/gate/code-transform/add-node.px",
      carrier_held_kinds: carrier_held_kinds_an(),
      carrier_supported_langs: Some(add_node::SUPPORTED_LANGUAGES),
      extra_lists: vec![(
        "validIfAlreadyPresent",
        all_strs(add_node::AddNodeIfAlreadyPresent::ALL, |k| k.as_str()),
      )],
    },
    // Third pnix-native transform — graph-edit primitive trio.
    // Mirrors `stdlib/lib/gate/code-transform/add-edge.px` and
    // `crates/pnix-core/src/code_transform/add_edge.rs`. Endpoint
    // shape is an `#[serde(untagged)]` enum byte-equal with the
    // `.px` attrset `{ input = ...; }` / `{ node = ...; port? = ...; }`.
    TransformSync {
      name: "add-edge",
      px_path: "stdlib/lib/gate/code-transform/add-edge.px",
      carrier_held_kinds: carrier_held_kinds_ae(),
      carrier_supported_langs: Some(add_edge::SUPPORTED_LANGUAGES),
      extra_lists: vec![(
        "validIfAlreadyPresent",
        all_strs(add_edge::AddEdgeIfAlreadyPresent::ALL, |k| k.as_str()),
      )],
    },
    TransformSync {
      name: "extract-function",
      px_path: "stdlib/lib/gate/code-transform/extract-function.px",
      carrier_held_kinds: carrier_held_kinds_ef(),
      carrier_supported_langs: Some(extract_function::SUPPORTED_LANGUAGES),
      extra_lists: vec![],
    },
    TransformSync {
      name: "inline-function",
      px_path: "stdlib/lib/gate/code-transform/inline-function.px",
      carrier_held_kinds: carrier_held_kinds_if(),
      carrier_supported_langs: Some(inline_function::SUPPORTED_LANGUAGES),
      extra_lists: vec![],
    },
    TransformSync {
      name: "move-symbol",
      px_path: "stdlib/lib/gate/code-transform/move-symbol.px",
      carrier_held_kinds: carrier_held_kinds_ms(),
      carrier_supported_langs: Some(move_symbol::SUPPORTED_LANGUAGES),
      extra_lists: vec![],
    },
    TransformSync {
      name: "change-signature",
      px_path: "stdlib/lib/gate/code-transform/change-signature.px",
      carrier_held_kinds: carrier_held_kinds_cs(),
      carrier_supported_langs: Some(change_signature::SUPPORTED_LANGUAGES),
      extra_lists: vec![(
        "validChangeKinds",
        all_strs(change_signature::ChangeSignatureKind::ALL, |k| k.as_str()),
      )],
    },
    // ─── algorithm-synthesis family ──────────────────────────────
    // Constitutional core engine — see ontology.md §15-4c, §16.
    // No SUPPORTED_LANGUAGES because synthesis is language-agnostic
    // (processes NL + code together; the per-language surface lives
    // in code_transform owners that synthesis lowers into).
    TransformSync {
      name: "algorithm-synthesis.intent-recognition",
      px_path: "stdlib/lib/gate/algorithm-synthesis/intent-recognition.px",
      carrier_held_kinds: all_strs(intent_recognition::IntentHeldKind::ALL, |k| k.as_str()),
      carrier_supported_langs: None,
      extra_lists: vec![(
        "validIntents",
        dedup_sorted(
          intent_recognition::VALID_INTENTS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        ),
      )],
    },
  ]
}

/// Pretty-print two sets side-by-side for assertion failure messages.
fn set_diff_msg(label_a: &str, a: &[String], label_b: &str, b: &[String]) -> String {
  let a_set: BTreeSet<&str> = a.iter().map(|s| s.as_str()).collect();
  let b_set: BTreeSet<&str> = b.iter().map(|s| s.as_str()).collect();
  let only_a: Vec<&&str> = a_set.difference(&b_set).collect();
  let only_b: Vec<&&str> = b_set.difference(&a_set).collect();
  format!(
    "    {label_a}: {a:?}\n    {label_b}: {b:?}\n    only in {label_a}: {only_a:?}\n    only in {label_b}: {only_b:?}"
  )
}

#[test]
fn held_kind_parity_per_transform() {
  let registry = transforms_registry();
  for t in &registry {
    let px_src = read_px(t.px_path);
    // The `.px` law contract is: classify-ladder `held_kind = "..."`
    // assignments PLUS an optional `hostHeldKinds = [ ... ]` list of
    // strict-preflight kinds the host carrier may emit on top. Both
    // count as "px-sanctioned" surface.
    let mut px_pool: Vec<String> = px_held_kinds(&px_src);
    px_pool.extend(px_list(&px_src, "hostHeldKinds"));
    let px = dedup_sorted(px_pool);
    let rs = &t.carrier_held_kinds;
    assert!(
      !px.is_empty(),
      "{}: no held_kind strings extracted from {} — extraction broken",
      t.name,
      t.px_path
    );
    assert!(
      !rs.is_empty(),
      "{}: Rust carrier ALL slice is empty",
      t.name
    );
    assert_eq!(
      &px,
      rs,
      "{}: held_kind parity mismatch\n{}",
      t.name,
      set_diff_msg("px", &px, "rs", rs)
    );
  }
}

#[test]
fn supported_languages_parity_per_transform() {
  let registry = transforms_registry();
  for t in &registry {
    // Synthesis-family owners are language-agnostic and have no
    // `supportedLanguages` list — skip them here. Their parity is
    // covered by `held_kind_parity_per_transform` and the extra-list
    // mechanism (e.g. `validIntents`).
    let Some(langs) = t.carrier_supported_langs else {
      continue;
    };
    let px_src = read_px(t.px_path);
    let px = dedup_sorted(px_list(&px_src, "supportedLanguages"));
    let rs: Vec<String> = dedup_sorted(langs.iter().map(|s| s.to_string()).collect());
    assert!(
      !px.is_empty(),
      "{}: no supportedLanguages extracted from {}",
      t.name,
      t.px_path
    );
    assert_eq!(
      px,
      rs,
      "{}: supportedLanguages parity mismatch\n{}",
      t.name,
      set_diff_msg("px", &px, "rs", &rs)
    );
  }
}

#[test]
fn extra_enum_lists_parity_per_transform() {
  let registry = transforms_registry();
  for t in &registry {
    if t.extra_lists.is_empty() {
      continue;
    }
    let px_src = read_px(t.px_path);
    for (list_name, rs_values) in &t.extra_lists {
      let px = dedup_sorted(px_list(&px_src, list_name));
      let rs = dedup_sorted(rs_values.clone());
      assert!(
        !px.is_empty(),
        "{}: no values for `{}` in {}",
        t.name,
        list_name,
        t.px_path
      );
      assert_eq!(
        px,
        rs,
        "{}: extra-list `{}` parity mismatch\n{}",
        t.name,
        list_name,
        set_diff_msg("px", &px, "rs", &rs)
      );
    }
  }
}

/// Sanity: `<HeldKind>::ALL` covers every variant. We can't enumerate
/// non-`Copy` enum variants directly, but a unique-set check on the
/// `as_str` mapping (no duplicates, no missing) catches missing-from-
/// ALL drift indirectly through `held_kind_parity_per_transform` —
/// because if a variant is missing from `ALL`, the carrier side will
/// be missing the kebab string, and the parity check fails with a
/// clear diff. This separate test surfaces the same condition with a
/// per-transform "no duplicates in ALL" assertion.
#[test]
fn carrier_all_slices_have_no_duplicates() {
  let registry = transforms_registry();
  for t in &registry {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for v in &t.carrier_held_kinds {
      assert!(
        seen.insert(v.as_str()),
        "{}: duplicate kebab '{}' in ALL — variant listed twice",
        t.name,
        v
      );
    }
  }
}

/// Static sanity: ensure each transform's repo path actually exists.
#[test]
fn px_owner_law_files_exist() {
  for t in &transforms_registry() {
    let p = repo_root().join(t.px_path);
    assert!(
      Path::new(&p).exists(),
      "{}: owner-law file {} does not exist",
      t.name,
      p.display()
    );
  }
}

/// `operation-candidate-mapping.px` ↔ Rust `OPERATION_MAP` parity.
///
/// The third algorithm-synthesis owner has its own registry shape.
/// We assert:
///   - the held-kind universe (`validHeldKinds` in `.px`,
///     `OperationMappingHeldKind::ALL` in Rust) matches
///   - the transform universe (`validTransforms`) matches
///   - the row count is equal (catches accidentally dropping rows
///     on either side)
#[test]
fn operation_candidate_mapping_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/operation-candidate-mapping.px";
  let px_src = read_px(px_path);

  // held kinds
  let px_held = dedup_sorted(px_list(&px_src, "validHeldKinds"));
  let rs_held = all_strs(
    operation_candidate_mapping::OperationMappingHeldKind::ALL,
    |k| k.as_str(),
  );
  assert_eq!(
    px_held,
    rs_held,
    "operation-candidate-mapping: validHeldKinds parity mismatch\n{}",
    set_diff_msg("px", &px_held, "rs", &rs_held)
  );

  // valid transforms
  let px_transforms = dedup_sorted(px_list(&px_src, "validTransforms"));
  let rs_transforms: Vec<String> = dedup_sorted(
    operation_candidate_mapping::VALID_TRANSFORMS
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );
  assert_eq!(
    px_transforms,
    rs_transforms,
    "operation-candidate-mapping: validTransforms parity mismatch\n{}",
    set_diff_msg("px", &px_transforms, "rs", &rs_transforms)
  );

  // Row count parity. Each `.px` row uses `(mkOpEntry` as an
  // application — exclude comment / definition lines that mention
  // the function bare.
  let px_row_count = px_src
    .lines()
    .filter(|l| !l.trim_start().starts_with('#'))
    .filter(|l| l.contains("(mkOpEntry "))
    .count();
  let rs_row_count = operation_candidate_mapping::OPERATION_MAP.len();
  assert_eq!(
    px_row_count, rs_row_count,
    "operation-candidate-mapping: row count mismatch (px={px_row_count}, rs={rs_row_count})"
  );
}

/// `parameter-resolution.px` ↔ Rust `parameter_resolution` parity.
/// Asserts the held-kind universe matches and that
/// `validResolvedTransforms` agrees with `VALID_RESOLVED_TRANSFORMS`.
#[test]
fn parameter_resolution_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/parameter-resolution.px";
  let px_src = read_px(px_path);

  let px_held = dedup_sorted(px_list(&px_src, "validHeldKinds"));
  let rs_held = all_strs(parameter_resolution::ResolutionHeldKind::ALL, |k| {
    k.as_str()
  });
  assert_eq!(
    px_held,
    rs_held,
    "parameter-resolution: validHeldKinds parity mismatch\n{}",
    set_diff_msg("px", &px_held, "rs", &rs_held)
  );

  let px_transforms = dedup_sorted(px_list(&px_src, "validResolvedTransforms"));
  let rs_transforms: Vec<String> = dedup_sorted(
    parameter_resolution::VALID_RESOLVED_TRANSFORMS
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );
  assert_eq!(
    px_transforms,
    rs_transforms,
    "parameter-resolution: validResolvedTransforms parity mismatch\n{}",
    set_diff_msg("px", &px_transforms, "rs", &rs_transforms)
  );

  // importSpecPatterns: assert the set of `language` values declared
  // in `.px` matches the languages covered by the Rust mirror's
  // `IMPORT_SPEC_PATTERNS`. The full per-row attrset (lead/middle/
  // shape/name_kind) is more delicate; this language-set check is
  // the minimum guard against silent drift.
  let mut px_langs: Vec<String> = Vec::new();
  for line in px_src.lines() {
    let trim = line.trim_start();
    if trim.starts_with('#') {
      continue;
    }
    let mut rest = line;
    while let Some(idx) = rest.find("language") {
      let after = &rest[idx + "language".len()..];
      let after_trim = after.trim_start_matches([' ', '\t']);
      let Some(after_eq) = after_trim.strip_prefix('=') else {
        rest = &rest[idx + "language".len()..];
        continue;
      };
      let after_eq = after_eq.trim_start_matches([' ', '\t']);
      let Some(after_q) = after_eq.strip_prefix('"') else {
        rest = &rest[idx + "language".len()..];
        continue;
      };
      let Some(end_q) = after_q.find('"') else {
        break;
      };
      // Only count rows inside importSpecPatterns context — line
      // must contain `lead =` or `shape =` near the language.
      if line.contains("lead =") || line.contains("shape =") {
        px_langs.push(after_q[..end_q].to_string());
      }
      rest = &after_q[end_q + 1..];
    }
  }
  let px_langs = dedup_sorted(px_langs);
  let rs_langs: Vec<String> = dedup_sorted(
    parameter_resolution::IMPORT_SPEC_PATTERNS
      .iter()
      .map(|p| p.language.to_string())
      .collect(),
  );
  assert_eq!(
    px_langs,
    rs_langs,
    "parameter-resolution: importSpecPatterns language coverage parity mismatch\n{}",
    set_diff_msg("px", &px_langs, "rs", &rs_langs)
  );
}

/// `algorithm-sentence-sequence.px` ↔ Rust
/// `algorithm_sentence_sequence` parity. Asserts both the
/// temporal-shape universe and the step-kind universe match.
#[test]
fn algorithm_sentence_sequence_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/algorithm-sentence-sequence.px";
  let px_src = read_px(px_path);

  let px_shapes = dedup_sorted(px_list(&px_src, "validTemporalShapes"));
  let rs_shapes = all_strs(algorithm_sentence_sequence::TemporalShape::ALL, |k| {
    k.as_str()
  });
  assert_eq!(
    px_shapes,
    rs_shapes,
    "algorithm-sentence-sequence: validTemporalShapes parity mismatch\n{}",
    set_diff_msg("px", &px_shapes, "rs", &rs_shapes)
  );

  let px_kinds = dedup_sorted(px_list(&px_src, "validStepKinds"));
  let rs_kinds = all_strs(algorithm_sentence_sequence::StepKind::ALL, |k| k.as_str());
  assert_eq!(
    px_kinds,
    rs_kinds,
    "algorithm-sentence-sequence: validStepKinds parity mismatch\n{}",
    set_diff_msg("px", &px_kinds, "rs", &rs_kinds)
  );
}

/// `structural-cue-registry.px` ↔ Rust `STRUCTURAL_CUES` parity.
/// Asserts both the cue universe and the test-file path patterns.
#[test]
fn structural_cue_registry_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/structural-cue-registry.px";
  let px_src = read_px(px_path);

  let px_cues = dedup_sorted(px_list(&px_src, "validStructuralCues"));
  let rs_cues: Vec<String> = dedup_sorted(
    structural_cue_registry::STRUCTURAL_CUES
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );
  assert_eq!(
    px_cues,
    rs_cues,
    "structural-cue-registry: validStructuralCues parity mismatch\n{}",
    set_diff_msg("px", &px_cues, "rs", &rs_cues)
  );

  let px_paths = dedup_sorted(px_list(&px_src, "testFilePathPatterns"));
  let rs_paths: Vec<String> = dedup_sorted(
    structural_cue_registry::TEST_FILE_PATH_PATTERNS
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );
  assert_eq!(
    px_paths,
    rs_paths,
    "structural-cue-registry: testFilePathPatterns parity mismatch\n{}",
    set_diff_msg("px", &px_paths, "rs", &rs_paths)
  );

  let px_langs = dedup_sorted(px_list(&px_src, "codeAnalysisSupportedLanguages"));
  let rs_langs: Vec<String> = dedup_sorted(
    structural_cue_registry::CODE_ANALYSIS_SUPPORTED_LANGUAGES
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );
  assert_eq!(
    px_langs,
    rs_langs,
    "structural-cue-registry: codeAnalysisSupportedLanguages parity mismatch\n{}",
    set_diff_msg("px", &px_langs, "rs", &rs_langs)
  );

  // extensionToLanguage pair-set parity. Each .px row has two named
  // fields (`extension` and `language`); we extract them
  // sequentially and zip into `"ext|lang"` strings for set
  // comparison, mirroring the schema-mapping-gate test's pair-set
  // approach.
  let extract_sequential = |key: &str| -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in px_src.lines() {
      let trim = line.trim_start();
      if trim.starts_with('#') {
        continue;
      }
      let mut rest = line;
      while let Some(idx) = rest.find(key) {
        let after = &rest[idx + key.len()..];
        let after_trim = after.trim_start_matches([' ', '\t']);
        let Some(after_eq) = after_trim.strip_prefix('=') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let after_eq = after_eq.trim_start_matches([' ', '\t']);
        let Some(after_q) = after_eq.strip_prefix('"') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let Some(end_q) = after_q.find('"') else {
          break;
        };
        let preceding_ok = idx == 0
          || !{
            let b = rest.as_bytes()[idx - 1];
            b.is_ascii_alphanumeric() || b == b'_'
          };
        if preceding_ok {
          out.push(after_q[..end_q].to_string());
        }
        rest = &after_q[end_q + 1..];
      }
    }
    out
  };
  let px_extensions = extract_sequential("extension");
  let px_languages = extract_sequential("language");
  assert_eq!(
    px_extensions.len(),
    px_languages.len(),
    "structural-cue-registry: extension/language row count mismatch \
     (extensions: {}, languages: {})",
    px_extensions.len(),
    px_languages.len()
  );
  let px_pairs: Vec<String> = dedup_sorted(
    px_extensions
      .iter()
      .zip(px_languages.iter())
      .map(|(e, l)| format!("{e}|{l}"))
      .collect(),
  );
  let rs_pairs: Vec<String> = dedup_sorted(
    structural_cue_registry::EXTENSION_TO_LANGUAGE
      .iter()
      .map(|(e, l)| format!("{e}|{l}"))
      .collect(),
  );
  assert_eq!(
    px_pairs,
    rs_pairs,
    "structural-cue-registry: extensionToLanguage pair parity mismatch\n{}",
    set_diff_msg("px", &px_pairs, "rs", &rs_pairs)
  );

  // Cross-list consistency: every language in extensionToLanguage's
  // value column that ALSO appears in codeAnalysisSupportedLanguages
  // means the Rust dispatch can fire detectors on files inferred via
  // path extension. (Other languages without a dispatch detector are
  // still valid mappings — they just yield no code-shape cues.)
  let mapped_langs: BTreeSet<&str> = structural_cue_registry::EXTENSION_TO_LANGUAGE
    .iter()
    .map(|(_, l)| *l)
    .collect();
  for analysis_lang in structural_cue_registry::CODE_ANALYSIS_SUPPORTED_LANGUAGES {
    assert!(
      mapped_langs.contains(analysis_lang),
      "structural-cue-registry: code-analysis language `{}` has no \
       extension mapping — `infer_language_from_path` cannot route to \
       its detector from a target_path",
      analysis_lang
    );
  }
}

/// `held-to-query.px` ↔ Rust `held_to_query` parity.
/// Asserts:
///   1. Recovery-channel universe matches `HeldQueryRecoveryChannel::ALL`.
///   2. Every held kind named in `.px` `heldRoutingMap` has a matching
///      `ResolutionHeldKind` AND a routing entry in
///      `HELD_ROUTING`.
///   3. Every `query_kind` named in `.px` `queryKindMap` appears in
///      the Rust `HELD_ROUTING` rows' `query_kind` fields.
#[test]
fn held_to_query_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/held-to-query.px";
  let px_src = read_px(px_path);

  // (1) recovery channels universe
  let px_channels = dedup_sorted(px_list(&px_src, "validRecoveryChannels"));
  let rs_channels: Vec<String> = dedup_sorted(
    held_to_query::HeldQueryRecoveryChannel::ALL
      .iter()
      .map(|c| c.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_channels,
    rs_channels,
    "held-to-query: validRecoveryChannels parity mismatch\n{}",
    set_diff_msg("px", &px_channels, "rs", &rs_channels)
  );

  // (2) every `held = "..."` literal in either heldRoutingMap or
  //     queryKindMap is a registered ResolutionHeldKind. The
  //     attrset rows are inline (`{ held = "..."; primary = ...; }`)
  //     so we anchor on the literal substring `held = "` rather
  //     than splitting on `=` at the line head.
  let mut px_held_kinds: Vec<String> = Vec::new();
  for line in px_src.lines() {
    let trim = line.trim_start();
    if trim.starts_with('#') {
      continue;
    }
    let mut rest = line;
    while let Some(idx) = rest.find("held") {
      let after = &rest[idx + "held".len()..];
      // Must be followed by ` = "` (optional whitespace).
      let after_trim = after.trim_start_matches([' ', '\t']);
      let Some(after_eq) = after_trim.strip_prefix('=') else {
        rest = &rest[idx + "held".len()..];
        continue;
      };
      let after_eq = after_eq.trim_start_matches([' ', '\t']);
      let Some(after_q) = after_eq.strip_prefix('"') else {
        rest = &rest[idx + "held".len()..];
        continue;
      };
      let Some(end_q) = after_q.find('"') else {
        break;
      };
      // Skip if the matched `held` is preceded by an identifier
      // character (e.g. `validHeldKinds`). The position relative to
      // the slice tells us whether `held` is a token boundary.
      let preceding_ok = idx == 0
        || !{
          let b = rest.as_bytes()[idx - 1];
          b.is_ascii_alphanumeric() || b == b'_'
        };
      if preceding_ok {
        px_held_kinds.push(after_q[..end_q].to_string());
      }
      rest = &after_q[end_q + 1..];
    }
  }
  let px_held_kinds = dedup_sorted(px_held_kinds);
  let rs_held_kinds: Vec<String> = dedup_sorted(
    parameter_resolution::ResolutionHeldKind::ALL
      .iter()
      .map(|k| k.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_held_kinds,
    rs_held_kinds,
    "held-to-query: per-held entries must cover every ResolutionHeldKind\n{}",
    set_diff_msg(
      "px held",
      &px_held_kinds,
      "rs ResolutionHeldKind",
      &rs_held_kinds
    )
  );

  // (3) every `query_kind = "..."` literal in `.px` queryKindMap
  //     appears in a Rust HELD_ROUTING row, and vice versa. Same
  //     inline-attrset anchor as above.
  let mut px_query_kinds: Vec<String> = Vec::new();
  for line in px_src.lines() {
    let trim = line.trim_start();
    if trim.starts_with('#') {
      continue;
    }
    let mut rest = line;
    while let Some(idx) = rest.find("query_kind") {
      let after = &rest[idx + "query_kind".len()..];
      let after_trim = after.trim_start_matches([' ', '\t']);
      let Some(after_eq) = after_trim.strip_prefix('=') else {
        rest = &rest[idx + "query_kind".len()..];
        continue;
      };
      let after_eq = after_eq.trim_start_matches([' ', '\t']);
      let Some(after_q) = after_eq.strip_prefix('"') else {
        rest = &rest[idx + "query_kind".len()..];
        continue;
      };
      let Some(end_q) = after_q.find('"') else {
        break;
      };
      px_query_kinds.push(after_q[..end_q].to_string());
      rest = &after_q[end_q + 1..];
    }
  }
  let px_query_kinds = dedup_sorted(px_query_kinds);
  let rs_query_kinds: Vec<String> = dedup_sorted(
    held_to_query::HELD_ROUTING
      .iter()
      .map(|e| e.query_kind.to_string())
      .collect(),
  );
  assert_eq!(
    px_query_kinds,
    rs_query_kinds,
    "held-to-query: query_kind universe parity mismatch\n{}",
    set_diff_msg("px", &px_query_kinds, "rs", &rs_query_kinds)
  );

  // (4) message-template universe: every `.px` queryMessageTemplates
  //     row's `query_kind` matches a Rust `QUERY_MESSAGE_TEMPLATES`
  //     row's `query_kind`, and the universe equals the routing
  //     `query_kind` universe (no orphan templates).
  let rs_msg_query_kinds: Vec<String> = dedup_sorted(
    held_to_query::QUERY_MESSAGE_TEMPLATES
      .iter()
      .map(|t| t.query_kind.to_string())
      .collect(),
  );
  assert_eq!(
    rs_msg_query_kinds,
    rs_query_kinds,
    "held-to-query: Rust message-template universe must equal routing universe\n{}",
    set_diff_msg("templates", &rs_msg_query_kinds, "routing", &rs_query_kinds)
  );
}

/// `retrieval-execution.px` ↔ Rust `retrieval_execution` parity.
/// Asserts:
///   1. Channel-implementation-status table covers every recovery
///      channel from `HeldQueryRecoveryChannel::ALL`.
///   2. Per-channel status string (implemented / deferred) matches
///      between `.px` and Rust.
///   3. `evidenceSlotToHostKey` and `slotToResolutionInputField`
///      slot sets match between `.px` and Rust.
#[test]
fn retrieval_execution_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/retrieval-execution.px";
  let px_src = read_px(px_path);

  // (1) + (2) channel + status parity. The `.px` rows are
  // multi-line attrsets, so we extract every
  // `<key> = "<value>"` literal in the file in document order
  // and zip the `channel` / `status` sequences together. Both keys
  // appear exactly once per row.
  let extract_sequential = |key: &str| -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in px_src.lines() {
      let trim = line.trim_start();
      if trim.starts_with('#') {
        continue;
      }
      let mut rest = line;
      while let Some(idx) = rest.find(key) {
        let after = &rest[idx + key.len()..];
        let after_trim = after.trim_start_matches([' ', '\t']);
        let Some(after_eq) = after_trim.strip_prefix('=') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let after_eq = after_eq.trim_start_matches([' ', '\t']);
        let Some(after_q) = after_eq.strip_prefix('"') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let Some(end_q) = after_q.find('"') else {
          break;
        };
        // Boundary check: prev char must not be ident-ish (filters
        // `channelImplementationStatus` etc.).
        let preceding_ok = idx == 0
          || !{
            let b = rest.as_bytes()[idx - 1];
            b.is_ascii_alphanumeric() || b == b'_'
          };
        if preceding_ok {
          out.push(after_q[..end_q].to_string());
        }
        rest = &after_q[end_q + 1..];
      }
    }
    out
  };
  let px_channels = extract_sequential("channel");
  let px_statuses = extract_sequential("status");
  assert_eq!(
    px_channels.len(),
    px_statuses.len(),
    "retrieval-execution: channel/status row count mismatch in `.px`"
  );
  let px_pairs: Vec<(String, String)> = px_channels
    .into_iter()
    .zip(px_statuses.into_iter())
    .collect();
  // Build sorted lookup keys: "channel|status".
  let px_lookup: Vec<String> = {
    let mut v: Vec<String> = px_pairs.iter().map(|(c, s)| format!("{c}|{s}")).collect();
    v.sort();
    v.dedup();
    v
  };
  let rs_lookup: Vec<String> = {
    let mut v: Vec<String> = retrieval_execution::CHANNEL_IMPLEMENTATION_STATUS
      .iter()
      .map(|r| format!("{}|{}", r.channel.as_str(), r.status.as_str()))
      .collect();
    v.sort();
    v.dedup();
    v
  };
  assert_eq!(
    px_lookup,
    rs_lookup,
    "retrieval-execution: channel implementation status parity mismatch\n{}",
    set_diff_msg("px", &px_lookup, "rs", &rs_lookup)
  );

  // (3a) evidenceSlotToHostKey: extract `slot` literals.
  let extract_keyed_literals = |key: &str| -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in px_src.lines() {
      let trim = line.trim_start();
      if trim.starts_with('#') {
        continue;
      }
      let mut rest = line;
      while let Some(idx) = rest.find(key) {
        let after = &rest[idx + key.len()..];
        let after_trim = after.trim_start_matches([' ', '\t']);
        let Some(after_eq) = after_trim.strip_prefix('=') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let after_eq = after_eq.trim_start_matches([' ', '\t']);
        let Some(after_q) = after_eq.strip_prefix('"') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let Some(end_q) = after_q.find('"') else {
          break;
        };
        // Boundary check: previous char must not be ident-ish.
        let preceding_ok = idx == 0
          || !{
            let b = rest.as_bytes()[idx - 1];
            b.is_ascii_alphanumeric() || b == b'_'
          };
        if preceding_ok {
          out.push(after_q[..end_q].to_string());
        }
        rest = &after_q[end_q + 1..];
      }
    }
    out
  };
  let px_slots = dedup_sorted(extract_keyed_literals("slot"));
  let rs_slot_host: Vec<String> = dedup_sorted(
    retrieval_execution::EVIDENCE_SLOT_TO_HOST_KEY
      .iter()
      .map(|(s, _)| s.to_string())
      .collect(),
  );
  let rs_slot_field: Vec<String> = dedup_sorted(
    retrieval_execution::SLOT_TO_RESOLUTION_INPUT_FIELD
      .iter()
      .map(|(s, _)| s.to_string())
      .collect(),
  );
  assert_eq!(
    px_slots,
    rs_slot_host,
    "retrieval-execution: evidenceSlotToHostKey slot parity mismatch\n{}",
    set_diff_msg("px", &px_slots, "rs", &rs_slot_host)
  );
  assert_eq!(
    rs_slot_host, rs_slot_field,
    "retrieval-execution: EVIDENCE_SLOT_TO_HOST_KEY and SLOT_TO_RESOLUTION_INPUT_FIELD slot sets must match"
  );
}

/// `ankh-retrieval-cache.px` ↔ Rust `ankh_retrieval_cache` parity.
/// Asserts:
///   1. Provenance-source universe matches
///      `AnkhProvenanceSource::ALL`.
///   2. Ankh-key fields match `ANKH_KEY_FIELDS` in declared order.
///   3. Required-provenance-fields each appear as a non-Option
///      field on the Rust `AnkhEntry` struct (verified indirectly
///      by ensuring the field count is at least the required count
///      — the Rust struct's compile-time signature is the typed
///      enforcement; this guard catches drift in the `.px` table).
#[test]
fn ankh_retrieval_cache_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/ankh-retrieval-cache.px";
  let px_src = read_px(px_path);

  // (1) provenance sources
  let px_sources = dedup_sorted(px_list(&px_src, "validProvenanceSources"));
  let rs_sources: Vec<String> = dedup_sorted(
    ankh_retrieval_cache::AnkhProvenanceSource::ALL
      .iter()
      .map(|s| s.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_sources,
    rs_sources,
    "ankh-retrieval-cache: validProvenanceSources parity mismatch\n{}",
    set_diff_msg("px", &px_sources, "rs", &rs_sources)
  );

  // (2) ankh-key fields — order-sensitive parity
  let px_fields = px_list(&px_src, "ankhKeyFields");
  let rs_fields: Vec<String> = ankh_retrieval_cache::ANKH_KEY_FIELDS
    .iter()
    .map(|s| s.to_string())
    .collect();
  assert_eq!(
    px_fields, rs_fields,
    "ankh-retrieval-cache: ankhKeyFields order must match between `.px` and Rust"
  );

  // (3) required provenance fields — at least asserts coverage
  // count + presence; the Rust struct's compile-time signature is
  // the actual enforcement that these fields are non-Option.
  let px_required = dedup_sorted(px_list(&px_src, "requiredProvenanceFields"));
  // Hard-coded expected set on the Rust side: these names match
  // AnkhEntry's non-Option public fields. Drift here means the
  // `.px` and Rust struct diverged on which fields are
  // load-bearing for provenance.
  let rs_required: Vec<String> = dedup_sorted(
    [
      "provenance_source",
      "contributing_actor_id",
      "contributing_tenant_id",
      "stored_at_ms",
      "query_kind",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect(),
  );
  assert_eq!(
    px_required,
    rs_required,
    "ankh-retrieval-cache: requiredProvenanceFields parity mismatch\n{}",
    set_diff_msg("px", &px_required, "rs", &rs_required)
  );
}

/// `candidate-row-proposal.px` ↔ Rust `candidate_row_proposal`
/// parity. Asserts:
///   1. Firewall-gate universe matches `FIREWALL_GATES`.
///   2. Candidate-kind universe matches `CandidateKind::ALL`.
///   3. Gate-status universe matches `GateStatus::ALL`.
///   4. Every candidate kind in `.px` has a threshold row + a
///      target-owner row, with matching counts.
#[test]
fn candidate_row_proposal_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/candidate-row-proposal.px";
  let px_src = read_px(px_path);

  // (1) firewall gates
  let px_gates = px_list(&px_src, "firewallGates");
  let rs_gates: Vec<String> = candidate_row_proposal::FIREWALL_GATES
    .iter()
    .map(|s| s.to_string())
    .collect();
  assert_eq!(
    px_gates, rs_gates,
    "candidate-row-proposal: firewallGates order must match"
  );

  // (2) candidate kinds
  let px_kinds = dedup_sorted(px_list(&px_src, "validCandidateKinds"));
  let rs_kinds: Vec<String> = dedup_sorted(
    candidate_row_proposal::CandidateKind::ALL
      .iter()
      .map(|k| k.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_kinds,
    rs_kinds,
    "candidate-row-proposal: validCandidateKinds parity mismatch\n{}",
    set_diff_msg("px", &px_kinds, "rs", &rs_kinds)
  );

  // (3) gate statuses
  let px_statuses = dedup_sorted(px_list(&px_src, "validGateStatuses"));
  let rs_statuses: Vec<String> = dedup_sorted(
    candidate_row_proposal::GateStatus::ALL
      .iter()
      .map(|s| s.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_statuses,
    rs_statuses,
    "candidate-row-proposal: validGateStatuses parity mismatch\n{}",
    set_diff_msg("px", &px_statuses, "rs", &rs_statuses)
  );

  // (3.5) ankh-derived learned-intent lowering shape. This keeps
  // the Rust proposer from inventing field names / provenance rules
  // that the `.px` owner law did not declare.
  let px_learned_intent_query_kinds = px_list(&px_src, "learnedIntentSignalAnkhQueryKinds");
  let rs_learned_intent_query_kinds: Vec<String> =
    candidate_row_proposal::LEARNED_INTENT_SIGNAL_ANKH_QUERY_KINDS
      .iter()
      .map(|s| s.to_string())
      .collect();
  assert_eq!(
    px_learned_intent_query_kinds, rs_learned_intent_query_kinds,
    "candidate-row-proposal: learned intent ankh query kinds must match"
  );

  let px_learned_intent_provenance =
    px_list(&px_src, "learnedIntentSignalAnkhAllowedProvenanceSources");
  let rs_learned_intent_provenance: Vec<String> =
    candidate_row_proposal::LEARNED_INTENT_SIGNAL_ANKH_ALLOWED_PROVENANCE
      .iter()
      .map(|p| p.as_str().to_string())
      .collect();
  assert_eq!(
    px_learned_intent_provenance, rs_learned_intent_provenance,
    "candidate-row-proposal: learned intent ankh provenance rules must match"
  );

  let px_learned_intent_required =
    px_list(&px_src, "learnedIntentSignalAnkhRequiredSuppliedParameters");
  let rs_learned_intent_required: Vec<String> =
    candidate_row_proposal::LEARNED_INTENT_SIGNAL_ANKH_REQUIRED_SUPPLIED_PARAMETERS
      .iter()
      .map(|s| s.to_string())
      .collect();
  assert_eq!(
    px_learned_intent_required, rs_learned_intent_required,
    "candidate-row-proposal: learned intent ankh required supplied_parameters must match"
  );

  let assert_px_list_matches = |list_name: &str, rs_values: Vec<String>| {
    let px_values = px_list(&px_src, list_name);
    assert_eq!(
      px_values, rs_values,
      "candidate-row-proposal: `{list_name}` parity mismatch"
    );
  };
  assert_px_list_matches(
    "learnedOperationMapAnkhQueryKinds",
    candidate_row_proposal::LEARNED_OPERATION_MAP_ANKH_QUERY_KINDS
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );
  assert_px_list_matches(
    "learnedOperationMapAnkhAllowedProvenanceSources",
    candidate_row_proposal::LEARNED_OPERATION_MAP_ANKH_ALLOWED_PROVENANCE
      .iter()
      .map(|p| p.as_str().to_string())
      .collect(),
  );
  assert_px_list_matches(
    "learnedOperationMapAnkhRequiredSuppliedParameters",
    candidate_row_proposal::LEARNED_OPERATION_MAP_ANKH_REQUIRED_SUPPLIED_PARAMETERS
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );
  assert_px_list_matches(
    "learnedParameterResolutionAnkhQueryKinds",
    candidate_row_proposal::LEARNED_PARAMETER_RESOLUTION_ANKH_QUERY_KINDS
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );
  assert_px_list_matches(
    "learnedParameterResolutionAnkhAllowedProvenanceSources",
    candidate_row_proposal::LEARNED_PARAMETER_RESOLUTION_ANKH_ALLOWED_PROVENANCE
      .iter()
      .map(|p| p.as_str().to_string())
      .collect(),
  );
  assert_px_list_matches(
    "learnedParameterResolutionAnkhRequiredSuppliedParameters",
    candidate_row_proposal::LEARNED_PARAMETER_RESOLUTION_ANKH_REQUIRED_SUPPLIED_PARAMETERS
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );
  assert_px_list_matches(
    "learnedFactCuePhrasePatternAnkhQueryKinds",
    candidate_row_proposal::LEARNED_FACT_CUE_PHRASE_PATTERN_ANKH_QUERY_KINDS
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );
  assert_px_list_matches(
    "learnedFactCuePhrasePatternAnkhAllowedProvenanceSources",
    candidate_row_proposal::LEARNED_FACT_CUE_PHRASE_PATTERN_ANKH_ALLOWED_PROVENANCE
      .iter()
      .map(|p| p.as_str().to_string())
      .collect(),
  );
  assert_px_list_matches(
    "learnedFactCuePhrasePatternAnkhRequiredSuppliedParameters",
    candidate_row_proposal::LEARNED_FACT_CUE_PHRASE_PATTERN_ANKH_REQUIRED_SUPPLIED_PARAMETERS
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );

  // (4) threshold rows + target owner rows: same kind set, same
  // count as validCandidateKinds.
  let extract_keyed_strings = |key: &str| -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in px_src.lines() {
      let trim = line.trim_start();
      if trim.starts_with('#') {
        continue;
      }
      let mut rest = line;
      while let Some(idx) = rest.find(key) {
        let after = &rest[idx + key.len()..];
        let after_trim = after.trim_start_matches([' ', '\t']);
        let Some(after_eq) = after_trim.strip_prefix('=') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let after_eq = after_eq.trim_start_matches([' ', '\t']);
        let Some(after_q) = after_eq.strip_prefix('"') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let Some(end_q) = after_q.find('"') else {
          break;
        };
        let preceding_ok = idx == 0
          || !{
            let b = rest.as_bytes()[idx - 1];
            b.is_ascii_alphanumeric() || b == b'_'
          };
        if preceding_ok {
          out.push(after_q[..end_q].to_string());
        }
        rest = &after_q[end_q + 1..];
      }
    }
    out
  };
  let px_kind_in_threshold = dedup_sorted(extract_keyed_strings("kind"));
  // `kind` appears in BOTH minimumEvidenceCounts and
  // candidateTargetOwners rows. We want to verify the SET equals
  // validCandidateKinds.
  assert_eq!(
    px_kind_in_threshold, px_kinds,
    "candidate-row-proposal: `kind` literal universe in threshold/target tables must match validCandidateKinds\n{}",
    set_diff_msg("kind tables", &px_kind_in_threshold, "validCandidateKinds", &px_kinds)
  );
  let rs_threshold_kinds: Vec<String> = dedup_sorted(
    candidate_row_proposal::MINIMUM_EVIDENCE_COUNTS
      .iter()
      .map(|r| r.kind.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    rs_threshold_kinds, rs_kinds,
    "candidate-row-proposal: Rust MINIMUM_EVIDENCE_COUNTS must cover all CandidateKind::ALL"
  );
  let rs_target_kinds: Vec<String> = dedup_sorted(
    candidate_row_proposal::CANDIDATE_TARGET_OWNERS
      .iter()
      .map(|r| r.kind.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    rs_target_kinds, rs_kinds,
    "candidate-row-proposal: Rust CANDIDATE_TARGET_OWNERS must cover all CandidateKind::ALL"
  );
}

/// `macro-fold-gate.px` ↔ Rust `macro_fold_gate` parity.
#[test]
fn macro_fold_gate_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/macro-fold-gate.px";
  let px_src = read_px(px_path);

  // (1) fold outcomes universe
  let px_outcomes = dedup_sorted(px_list(&px_src, "validFoldOutcomes"));
  let rs_outcomes: Vec<String> = dedup_sorted(
    macro_fold_gate::MacroFoldOutcome::ALL
      .iter()
      .map(|o| o.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_outcomes,
    rs_outcomes,
    "macro-fold-gate: validFoldOutcomes parity mismatch\n{}",
    set_diff_msg("px", &px_outcomes, "rs", &rs_outcomes)
  );

  // (2) format_id universe — every Rust ATTRSET_SYNTAX_RULES row's
  // format_id appears in `.px` attrsetSyntaxRules; defaultFormat
  // names a real format.
  let mut px_format_ids: Vec<String> = Vec::new();
  for line in px_src.lines() {
    let trim = line.trim_start();
    if trim.starts_with('#') {
      continue;
    }
    let mut rest = line;
    while let Some(idx) = rest.find("format_id") {
      let after = &rest[idx + "format_id".len()..];
      let after_trim = after.trim_start_matches([' ', '\t']);
      let Some(after_eq) = after_trim.strip_prefix('=') else {
        rest = &rest[idx + "format_id".len()..];
        continue;
      };
      let after_eq = after_eq.trim_start_matches([' ', '\t']);
      let Some(after_q) = after_eq.strip_prefix('"') else {
        rest = &rest[idx + "format_id".len()..];
        continue;
      };
      let Some(end_q) = after_q.find('"') else {
        break;
      };
      px_format_ids.push(after_q[..end_q].to_string());
      rest = &after_q[end_q + 1..];
    }
  }
  let px_format_ids = dedup_sorted(px_format_ids);
  let rs_format_ids: Vec<String> = dedup_sorted(
    macro_fold_gate::ATTRSET_SYNTAX_RULES
      .iter()
      .map(|r| r.format_id.to_string())
      .collect(),
  );
  assert_eq!(
    px_format_ids,
    rs_format_ids,
    "macro-fold-gate: attrsetSyntaxRules format_id parity mismatch\n{}",
    set_diff_msg("px", &px_format_ids, "rs", &rs_format_ids)
  );

  // (3) defaultFormat must be in the format_id universe.
  assert!(
    rs_format_ids.contains(&macro_fold_gate::DEFAULT_FORMAT.to_string()),
    "macro-fold-gate: DEFAULT_FORMAT `{}` not in format_id universe",
    macro_fold_gate::DEFAULT_FORMAT
  );
}

/// `axis-separation-gate.px` ↔ Rust `axis_separation_gate` parity.
#[test]
fn axis_separation_gate_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/axis-separation-gate.px";
  let px_src = read_px(px_path);

  // (1) axis outcomes universe
  let px_outcomes = dedup_sorted(px_list(&px_src, "validAxisOutcomes"));
  let rs_outcomes: Vec<String> = dedup_sorted(
    axis_separation_gate::AxisSeparationOutcome::ALL
      .iter()
      .map(|o| o.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_outcomes,
    rs_outcomes,
    "axis-separation-gate: validAxisOutcomes parity mismatch\n{}",
    set_diff_msg("px", &px_outcomes, "rs", &rs_outcomes)
  );

  // (2) target table schema (owner, table) pair set. The `.px`
  // rows use multi-line attrsets with `target_owner = "..."` +
  // `target_table = "..."` on separate lines, so we extract each
  // key sequentially and zip.
  let extract_sequential = |key: &str| -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in px_src.lines() {
      let trim = line.trim_start();
      if trim.starts_with('#') {
        continue;
      }
      let mut rest = line;
      while let Some(idx) = rest.find(key) {
        let after = &rest[idx + key.len()..];
        let after_trim = after.trim_start_matches([' ', '\t']);
        let Some(after_eq) = after_trim.strip_prefix('=') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let after_eq = after_eq.trim_start_matches([' ', '\t']);
        let Some(after_q) = after_eq.strip_prefix('"') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let Some(end_q) = after_q.find('"') else {
          break;
        };
        let preceding_ok = idx == 0
          || !{
            let b = rest.as_bytes()[idx - 1];
            b.is_ascii_alphanumeric() || b == b'_'
          };
        if preceding_ok {
          out.push(after_q[..end_q].to_string());
        }
        rest = &after_q[end_q + 1..];
      }
    }
    out
  };
  let px_owners = extract_sequential("target_owner");
  let px_tables = extract_sequential("target_table");
  assert_eq!(
    px_owners.len(),
    px_tables.len(),
    "axis-separation-gate: target_owner/target_table row count mismatch"
  );
  let px_pairs: Vec<String> = px_owners
    .into_iter()
    .zip(px_tables.into_iter())
    .map(|(o, t)| format!("{o}|{t}"))
    .collect();
  let px_pairs = dedup_sorted(px_pairs);
  let rs_pairs: Vec<String> = dedup_sorted(
    axis_separation_gate::TARGET_TABLE_SCHEMAS
      .iter()
      .map(|s| format!("{}|{}", s.target_owner, s.target_table))
      .collect(),
  );
  assert_eq!(
    px_pairs,
    rs_pairs,
    "axis-separation-gate: targetTableSchemas (owner,table) parity mismatch\n{}",
    set_diff_msg("px", &px_pairs, "rs", &rs_pairs)
  );
}

/// `regression-proof-gate.px` ↔ Rust `regression_proof_gate` parity.
#[test]
fn regression_proof_gate_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/regression-proof-gate.px";
  let px_src = read_px(px_path);

  // (1) regression outcomes
  let px_outcomes = dedup_sorted(px_list(&px_src, "validRegressionOutcomes"));
  let rs_outcomes: Vec<String> = dedup_sorted(
    regression_proof_gate::RegressionOutcome::ALL
      .iter()
      .map(|o| o.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_outcomes,
    rs_outcomes,
    "regression-proof-gate: validRegressionOutcomes parity mismatch\n{}",
    set_diff_msg("px", &px_outcomes, "rs", &rs_outcomes)
  );

  // (2) target uniqueness (owner, table) pairs.
  let extract_sequential = |key: &str| -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in px_src.lines() {
      let trim = line.trim_start();
      if trim.starts_with('#') {
        continue;
      }
      let mut rest = line;
      while let Some(idx) = rest.find(key) {
        let after = &rest[idx + key.len()..];
        let after_trim = after.trim_start_matches([' ', '\t']);
        let Some(after_eq) = after_trim.strip_prefix('=') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let after_eq = after_eq.trim_start_matches([' ', '\t']);
        let Some(after_q) = after_eq.strip_prefix('"') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let Some(end_q) = after_q.find('"') else {
          break;
        };
        let preceding_ok = idx == 0
          || !{
            let b = rest.as_bytes()[idx - 1];
            b.is_ascii_alphanumeric() || b == b'_'
          };
        if preceding_ok {
          out.push(after_q[..end_q].to_string());
        }
        rest = &after_q[end_q + 1..];
      }
    }
    out
  };
  let px_owners = extract_sequential("target_owner");
  let px_tables = extract_sequential("target_table");
  assert_eq!(
    px_owners.len(),
    px_tables.len(),
    "regression-proof-gate: target_owner/target_table count mismatch"
  );
  let px_pairs: Vec<String> = dedup_sorted(
    px_owners
      .into_iter()
      .zip(px_tables.into_iter())
      .map(|(o, t)| format!("{o}|{t}"))
      .collect(),
  );
  let rs_pairs: Vec<String> = dedup_sorted(
    regression_proof_gate::TARGET_TABLE_UNIQUENESS_KEYS
      .iter()
      .map(|u| format!("{}|{}", u.target_owner, u.target_table))
      .collect(),
  );
  assert_eq!(
    px_pairs,
    rs_pairs,
    "regression-proof-gate: targetTableUniquenessKeys (owner,table) parity mismatch\n{}",
    set_diff_msg("px", &px_pairs, "rs", &rs_pairs)
  );

  // (3) every regression-proof target pair must also have an
  // axis-separation schema entry (a table without a schema cannot
  // be regression-proven against — defense in depth).
  let axis_pairs: std::collections::BTreeSet<String> = axis_separation_gate::TARGET_TABLE_SCHEMAS
    .iter()
    .map(|s| format!("{}|{}", s.target_owner, s.target_table))
    .collect();
  for p in &rs_pairs {
    assert!(
      axis_pairs.contains(p),
      "regression-proof-gate: target pair `{p}` has no axis-separation schema entry"
    );
  }
}

/// `owner-law-gate.px` ↔ Rust `owner_law_gate` parity.
#[test]
fn owner_law_gate_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/owner-law-gate.px";
  let px_src = read_px(px_path);

  // (1) owner-law outcomes universe
  let px_outcomes = dedup_sorted(px_list(&px_src, "validOwnerLawOutcomes"));
  let rs_outcomes: Vec<String> = dedup_sorted(
    owner_law_gate::OwnerLawOutcome::ALL
      .iter()
      .map(|o| o.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_outcomes,
    rs_outcomes,
    "owner-law-gate: validOwnerLawOutcomes parity mismatch\n{}",
    set_diff_msg("px", &px_outcomes, "rs", &rs_outcomes)
  );

  // (2) approval decisions universe
  let px_decisions = dedup_sorted(px_list(&px_src, "validApprovalDecisions"));
  let rs_decisions: Vec<String> = dedup_sorted(
    owner_law_gate::PromotionApprovalDecision::ALL
      .iter()
      .map(|d| d.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_decisions,
    rs_decisions,
    "owner-law-gate: validApprovalDecisions parity mismatch\n{}",
    set_diff_msg("px", &px_decisions, "rs", &rs_decisions)
  );

  // (3) required approval fields — must match the non-Option
  // public fields on `PromotionApproval`.
  let px_required = dedup_sorted(px_list(&px_src, "requiredApprovalFields"));
  let rs_required: Vec<String> = dedup_sorted(
    [
      "actor_id",
      "tenant_id",
      "approved_at_ms",
      "decision",
      "candidate_fingerprint",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect(),
  );
  assert_eq!(
    px_required,
    rs_required,
    "owner-law-gate: requiredApprovalFields parity mismatch\n{}",
    set_diff_msg("px", &px_required, "rs", &rs_required)
  );

  // (4) approval identity-shape policy. This is not authentication,
  // but the owner law must remain the source of truth for which
  // namespaced principal prefixes Rust enforces.
  let px_identity_policy = px_nested_list_strings(&px_src, "approvalIdentityFieldPolicies");
  let rs_identity_policy: Vec<String> = vec![
    "actor_id".to_string(),
    owner_law_gate::APPROVAL_ACTOR_ID_VALID_PREFIXES[0].to_string(),
    owner_law_gate::APPROVAL_ACTOR_ID_VALID_PREFIXES[1].to_string(),
    "tenant_id".to_string(),
    owner_law_gate::APPROVAL_TENANT_ID_VALID_PREFIXES[0].to_string(),
    owner_law_gate::APPROVAL_TENANT_ID_VALID_PREFIXES[1].to_string(),
  ];
  assert_eq!(
    px_identity_policy, rs_identity_policy,
    "owner-law-gate: approvalIdentityFieldPolicies parity mismatch"
  );

  // (5) scope router covers every CandidateKind.
  let mut px_router_kinds: Vec<String> = Vec::new();
  for line in px_src.lines() {
    let trim = line.trim_start();
    if trim.starts_with('#') {
      continue;
    }
    if let Some(idx) = line.find("candidate_kind") {
      let after = &line[idx + "candidate_kind".len()..];
      let after_trim = after.trim_start_matches([' ', '\t']);
      let Some(after_eq) = after_trim.strip_prefix('=') else {
        continue;
      };
      let after_eq = after_eq.trim_start_matches([' ', '\t']);
      let Some(after_q) = after_eq.strip_prefix('"') else {
        continue;
      };
      let Some(end_q) = after_q.find('"') else {
        continue;
      };
      px_router_kinds.push(after_q[..end_q].to_string());
    }
  }
  let px_router_kinds = dedup_sorted(px_router_kinds);
  let rs_candidate_kinds: Vec<String> = dedup_sorted(
    candidate_row_proposal::CandidateKind::ALL
      .iter()
      .map(|k| k.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_router_kinds,
    rs_candidate_kinds,
    "owner-law-gate: promotionScopeRouterPolicy must cover every CandidateKind\n{}",
    set_diff_msg(
      "px router",
      &px_router_kinds,
      "rs CandidateKind",
      &rs_candidate_kinds
    )
  );
}

/// `registry-overlay.px` ↔ Rust `registry_overlay` parity.
#[test]
fn registry_overlay_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/registry-overlay.px";
  let px_src = read_px(px_path);

  let px_targets = dedup_sorted(px_list(&px_src, "validRegistryTargets"));
  let rs_targets: Vec<String> = dedup_sorted(
    registry_overlay::RegistryOverlayTarget::ALL
      .iter()
      .map(|t| t.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_targets,
    rs_targets,
    "registry-overlay: validRegistryTargets parity mismatch\n{}",
    set_diff_msg("px", &px_targets, "rs", &rs_targets)
  );

  let px_required = dedup_sorted(px_list(&px_src, "requiredOverlayFields"));
  let rs_required: Vec<String> = dedup_sorted(
    [
      "source_hot_reload_plan_fingerprint",
      "stored_at_ms",
      "contributing_actor_id",
      "contributing_tenant_id",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect(),
  );
  assert_eq!(
    px_required,
    rs_required,
    "registry-overlay: requiredOverlayFields parity mismatch\n{}",
    set_diff_msg("px", &px_required, "rs", &rs_required)
  );
}

/// `schema-mapping-gate.px` ↔ Rust `schema_mapping_gate` parity.
#[test]
fn schema_mapping_gate_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/schema-mapping-gate.px";
  let px_src = read_px(px_path);

  // (1) outcomes
  let px_outcomes = dedup_sorted(px_list(&px_src, "validMappingOutcomes"));
  let rs_outcomes: Vec<String> = dedup_sorted(
    schema_mapping_gate::SchemaMappingOutcome::ALL
      .iter()
      .map(|o| o.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_outcomes,
    rs_outcomes,
    "schema-mapping-gate: validMappingOutcomes parity mismatch\n{}",
    set_diff_msg("px", &px_outcomes, "rs", &rs_outcomes)
  );

  // (2) queryKindToHeldKind reverse map — assert every `.px` entry
  // has a matching Rust entry AND that every entry's held kind
  // exists in the static held-to-query forward map.
  let extract_sequential = |key: &str| -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in px_src.lines() {
      let trim = line.trim_start();
      if trim.starts_with('#') {
        continue;
      }
      let mut rest = line;
      while let Some(idx) = rest.find(key) {
        let after = &rest[idx + key.len()..];
        let after_trim = after.trim_start_matches([' ', '\t']);
        let Some(after_eq) = after_trim.strip_prefix('=') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let after_eq = after_eq.trim_start_matches([' ', '\t']);
        let Some(after_q) = after_eq.strip_prefix('"') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let Some(end_q) = after_q.find('"') else {
          break;
        };
        let preceding_ok = idx == 0
          || !{
            let b = rest.as_bytes()[idx - 1];
            b.is_ascii_alphanumeric() || b == b'_'
          };
        if preceding_ok {
          out.push(after_q[..end_q].to_string());
        }
        rest = &after_q[end_q + 1..];
      }
    }
    out
  };
  let px_query_kinds = extract_sequential("query_kind");
  let px_held_kinds_in_reverse_map = extract_sequential("held");
  assert_eq!(
    px_query_kinds.len(),
    px_held_kinds_in_reverse_map.len(),
    "schema-mapping-gate: query_kind/held row count mismatch"
  );
  let px_pairs: Vec<String> = px_query_kinds
    .iter()
    .zip(px_held_kinds_in_reverse_map.iter())
    .map(|(q, h)| format!("{q}|{h}"))
    .collect();
  let px_pairs = dedup_sorted(px_pairs);
  let rs_pairs: Vec<String> = dedup_sorted(
    schema_mapping_gate::QUERY_KIND_TO_HELD_KIND
      .iter()
      .map(|(q, h)| format!("{q}|{}", h.as_str()))
      .collect(),
  );
  assert_eq!(
    px_pairs,
    rs_pairs,
    "schema-mapping-gate: queryKindToHeldKind parity mismatch\n{}",
    set_diff_msg("px", &px_pairs, "rs", &rs_pairs)
  );

  // (3) cross-gate consistency: every (query_kind, held) pair here
  // must align with held-to-query's forward map.
  for (query_kind, held) in schema_mapping_gate::QUERY_KIND_TO_HELD_KIND {
    let forward = held_to_query::HELD_ROUTING.iter().find(|e| e.held == *held);
    assert!(
      forward.is_some(),
      "schema-mapping-gate: held kind `{}` has no forward HELD_ROUTING entry",
      held.as_str()
    );
    let forward = forward.unwrap();
    assert_eq!(
      forward.query_kind,
      *query_kind,
      "schema-mapping-gate: held `{}` maps to `{query_kind}` in reverse but forward gives `{}`",
      held.as_str(),
      forward.query_kind
    );
  }
}

/// `runtime-hot-reload.px` ↔ Rust `runtime_hot_reload` parity.
#[test]
fn runtime_hot_reload_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/runtime-hot-reload.px";
  let px_src = read_px(px_path);

  let px_outcomes = dedup_sorted(px_list(&px_src, "validHotReloadOutcomes"));
  let rs_outcomes: Vec<String> = dedup_sorted(
    runtime_hot_reload::HotReloadOutcome::ALL
      .iter()
      .map(|o| o.as_str().to_string())
      .collect(),
  );
  assert_eq!(
    px_outcomes,
    rs_outcomes,
    "runtime-hot-reload: validHotReloadOutcomes parity mismatch\n{}",
    set_diff_msg("px", &px_outcomes, "rs", &rs_outcomes)
  );

  let extract_sequential = |key: &str| -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in px_src.lines() {
      let trim = line.trim_start();
      if trim.starts_with('#') {
        continue;
      }
      let mut rest = line;
      while let Some(idx) = rest.find(key) {
        let after = &rest[idx + key.len()..];
        let after_trim = after.trim_start_matches([' ', '\t']);
        let Some(after_eq) = after_trim.strip_prefix('=') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let after_eq = after_eq.trim_start_matches([' ', '\t']);
        let Some(after_q) = after_eq.strip_prefix('"') else {
          rest = &rest[idx + key.len()..];
          continue;
        };
        let Some(end_q) = after_q.find('"') else {
          break;
        };
        let preceding_ok = idx == 0
          || !{
            let b = rest.as_bytes()[idx - 1];
            b.is_ascii_alphanumeric() || b == b'_'
          };
        if preceding_ok {
          out.push(after_q[..end_q].to_string());
        }
        rest = &after_q[end_q + 1..];
      }
    }
    out
  };
  let px_owners = extract_sequential("target_owner");
  let px_tables = extract_sequential("target_table");
  assert_eq!(
    px_owners.len(),
    px_tables.len(),
    "runtime-hot-reload: target_owner/target_table count mismatch"
  );
  let px_pairs: Vec<String> = dedup_sorted(
    px_owners
      .into_iter()
      .zip(px_tables.into_iter())
      .map(|(o, t)| format!("{o}|{t}"))
      .collect(),
  );
  let rs_pairs: Vec<String> = dedup_sorted(
    runtime_hot_reload::INSERTION_ANCHORS
      .iter()
      .map(|a| format!("{}|{}", a.target_owner, a.target_table))
      .collect(),
  );
  assert_eq!(
    px_pairs,
    rs_pairs,
    "runtime-hot-reload: insertionAnchors (owner,table) parity mismatch\n{}",
    set_diff_msg("px", &px_pairs, "rs", &rs_pairs)
  );

  // Every hot-reload target must also be a regression-proof target.
  let rp_pairs: std::collections::BTreeSet<String> =
    regression_proof_gate::TARGET_TABLE_UNIQUENESS_KEYS
      .iter()
      .map(|u| format!("{}|{}", u.target_owner, u.target_table))
      .collect();
  for p in &rs_pairs {
    assert!(
      rp_pairs.contains(p),
      "runtime-hot-reload: target pair `{p}` has no regression-proof entry"
    );
  }
}

/// Asserts both the cue universe and that every fact cue named in
/// `.px` `factPhrasePatterns` is in the registered universe (and
/// vice versa — no orphaned patterns).
#[test]
fn fact_cue_registry_parity() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/fact-cue-registry.px";
  let px_src = read_px(px_path);

  let px_cues = dedup_sorted(px_list(&px_src, "validFactCues"));
  let rs_cues: Vec<String> = dedup_sorted(
    fact_cue_registry::FACT_CUES
      .iter()
      .map(|s| s.to_string())
      .collect(),
  );
  assert_eq!(
    px_cues,
    rs_cues,
    "fact-cue-registry: validFactCues parity mismatch\n{}",
    set_diff_msg("px", &px_cues, "rs", &rs_cues)
  );

  // Every `.px` `cue = "fact:..."` line inside `factPhrasePatterns`
  // must be a registered cue. We extract those lines specifically
  // (rather than from `px_list`) to make this assertion structural.
  let mut px_phrase_cues: Vec<String> = Vec::new();
  for line in px_src.lines() {
    let trim = line.trim_start();
    if trim.starts_with('#') {
      continue;
    }
    // Lines like `cue = "fact:contradicted";` inside the patterns
    // attrset. Skip the `validFactCues` list entries (they have no
    // `cue =` prefix).
    let Some(eq_idx) = trim.find('=') else {
      continue;
    };
    let key = trim[..eq_idx].trim();
    if key != "cue" {
      continue;
    }
    let rest = &trim[eq_idx + 1..];
    let Some(first_q) = rest.find('"') else {
      continue;
    };
    let after = &rest[first_q + 1..];
    let Some(end_q) = after.find('"') else {
      continue;
    };
    px_phrase_cues.push(after[..end_q].to_string());
  }
  let px_phrase_cues = dedup_sorted(px_phrase_cues);

  // Structural fact cues (math/chemistry question detection) are
  // exempted from phrase-pattern coverage by design — they fire
  // structurally via parameter-resolution canonical-form extractors,
  // not via phrase markers. Mirror Rust's `STRUCTURAL_FACT_CUES`.
  let px_structural: BTreeSet<String> =
    px_list(&px_src, "structuralFactCues").into_iter().collect();
  let rs_structural: BTreeSet<String> = fact_cue_registry::STRUCTURAL_FACT_CUES
    .iter()
    .map(|s| s.to_string())
    .collect();
  assert_eq!(
    dedup_sorted(px_structural.iter().cloned().collect()),
    dedup_sorted(rs_structural.iter().cloned().collect()),
    "fact-cue-registry: structuralFactCues parity mismatch"
  );

  let non_structural_valid: Vec<String> = px_cues
    .iter()
    .filter(|c| !px_structural.contains(*c))
    .cloned()
    .collect();
  assert_eq!(
    px_phrase_cues,
    non_structural_valid,
    "fact-cue-registry: phrase pattern cues differ from non-structural validFactCues\n{}",
    set_diff_msg(
      "phrase",
      &px_phrase_cues,
      "non-structural valid",
      &non_structural_valid
    )
  );
}

/// `verb-cue-registry.px` ↔ Rust `VERB_CUE_REGISTRY` parity.
///
/// The verb-cue registry has a different shape from the
/// `TransformSync` rows above (no held_kinds, no supportedLanguages).
/// We assert that the set of cue names declared in the `.px`
/// `validCueNames` (and emitted by the `verbCueRegistry` rows) equals
/// the set of `cue` fields in the Rust `VERB_CUE_REGISTRY` slice.
///
/// Per-cue patterns are NOT compared here — patterns are localization
/// data that's expected to evolve; the carrier sync test ensures the
/// cue universe stays aligned.
#[test]
fn verb_cue_registry_cue_universe_in_sync() {
  let px_path = "stdlib/lib/gate/algorithm-synthesis/verb-cue-registry.px";
  let px_src = read_px(px_path);

  // Extract every `cue = "..."` literal from the `.px`. Same Nix-
  // comment-skip handling as `px_held_kinds`.
  let mut px_cues: Vec<String> = Vec::new();
  for line in px_src.lines() {
    if line.trim_start().starts_with('#') {
      continue;
    }
    // `(mkCueEntry "verb:xxx" [...])` — extract the first quoted
    // string. Only lines containing `mkCueEntry` count, so the
    // ownerName / meta literals don't pollute.
    if !line.contains("mkCueEntry") {
      continue;
    }
    let Some(open) = line.find('"') else { continue };
    let body = &line[open + 1..];
    let Some(close) = body.find('"') else {
      continue;
    };
    let cue = &body[..close];
    if cue.starts_with("verb:") || cue.starts_with("metaphor:") {
      px_cues.push(cue.to_string());
    }
  }
  let px_set = dedup_sorted(px_cues);

  let rs_set: Vec<String> = dedup_sorted(
    verb_cue_registry::VERB_CUE_REGISTRY
      .iter()
      .map(|e| e.cue.to_string())
      .collect(),
  );

  assert!(
    !px_set.is_empty(),
    "no verb/metaphor cues extracted from {px_path} — extraction broken"
  );
  assert_eq!(
    px_set,
    rs_set,
    "verb-cue-registry: cue-universe parity mismatch\n{}",
    set_diff_msg("px", &px_set, "rs", &rs_set)
  );
}
