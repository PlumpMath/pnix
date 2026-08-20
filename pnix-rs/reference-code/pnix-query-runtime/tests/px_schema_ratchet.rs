//! batch 258 (2026-04-18): pnix-query-runtime/data 미러에도 동일한 schema
//! ratchet 을 설치한다.
//!
//! `data_px_mirror_drift` 가 byte-identical 을 강제하므로 원리상 두 미러가
//! 같은 내용을 가진다. 그러나 drift 테스트는 `pnix-query-runtime` crate 에
//! 없고 `doghouse-core` 쪽에만 있다. pnix-query-runtime 을 doghouse 없이
//! (standalone) 빌드/테스트하는 경로에서도 같은 schema invariant 를 보장하려면,
//! 이 crate 에 직접 schema 테스트를 둔다.

use pnix_query_runtime::px::{parse_px_file, PxValue};
use std::path::PathBuf;

fn data_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}

fn load(name: &str) -> PxValue {
  let path = data_dir().join(name);
  parse_px_file(&path).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn root_list<'a>(root: &'a PxValue, key: &str) -> Vec<String> {
  let attrset = root
    .as_attrset()
    .unwrap_or_else(|| panic!("root must be attrset"));
  let value = attrset
    .get(key)
    .unwrap_or_else(|| panic!("missing key `{key}`"));
  match value {
    PxValue::List(items) => items
      .iter()
      .filter_map(|v| v.as_str().map(String::from))
      .collect(),
    other => panic!("key `{key}` must be list, got {:?}", other),
  }
}

fn root_scalar(root: &PxValue, key: &str) -> String {
  let attrset = root
    .as_attrset()
    .unwrap_or_else(|| panic!("root must be attrset"));
  let value = attrset
    .get(key)
    .unwrap_or_else(|| panic!("missing key `{key}`"));
  value
    .as_str()
    .unwrap_or_else(|| panic!("key `{key}` must be string, got {:?}", value))
    .to_string()
}

#[test]
fn batch_251_arithmetic_char_markers_present() {
  let px = load("query-classifiers.px");
  for key in &[
    "equation-char-markers",
    "intent-math-op-char-markers",
    "mixed-math-op-char-markers",
    "variable-char-markers",
  ] {
    let items = root_list(&px, key);
    assert!(!items.is_empty(), "`{key}` must be non-empty list");
  }
}

#[test]
fn batch_253_math_context_and_particle_chars_present() {
  let px = load("query-classifiers.px");
  for key in &[
    "math-context-char-markers",
    "korean-sentence-particle-chars",
  ] {
    let items = root_list(&px, key);
    assert!(!items.is_empty(), "`{key}` must be non-empty list");
  }
}

#[test]
fn batch_254_korean_morpheme_owners_present() {
  let px = load("query-classifiers.px");
  let possessive = root_scalar(&px, "korean-power-expression-particle");
  assert_eq!(possessive.chars().count(), 1);
  let ordinal = root_scalar(&px, "korean-ordinal-prefix-char");
  assert_eq!(ordinal.chars().count(), 1);
  let endings = root_list(&px, "korean-equation-copula-endings");
  assert!(!endings.is_empty());
  // 순서 invariant: 긴 어미 먼저.
  let lengths: Vec<usize> = endings.iter().map(|s| s.chars().count()).collect();
  for win in lengths.windows(2) {
    assert!(win[0] >= win[1], "ordering violated: {:?}", lengths);
  }
}

#[test]
fn batch_255_suggested_query_template_present() {
  let px = load("auto-learn-policy.px");
  let template = root_scalar(&px, "suggested-query-template");
  assert!(template.contains("{subject}"));
  assert!(template.contains("{object}"));
}

#[test]
fn batch_256_emergent_search_messages_present() {
  let px = load("korean-morphology.px");
  let empty = root_scalar(&px, "emergent-search-empty-result");
  assert!(!empty.is_empty());
  let hint = root_scalar(&px, "emergent-search-empty-follow-up");
  assert!(!hint.is_empty());
  let template = root_scalar(&px, "emergent-search-ko-definition-template");
  assert!(template.contains("{clean}"));
}

#[test]
fn batch_257_computation_cleanup_affixes_present_and_ordered() {
  let px = load("korean-morphology.px");
  let items = root_list(&px, "computation-cleanup-affixes");
  assert!(!items.is_empty());
  let long = items
    .iter()
    .position(|s| s == "계산해줘")
    .expect("`계산해줘` must appear");
  let mid = items
    .iter()
    .position(|s| s == "계산해")
    .expect("`계산해` must appear");
  let short = items
    .iter()
    .position(|s| s == "계산")
    .expect("`계산` must appear");
  assert!(long < mid && mid < short, "order violated: {:?}", items);
}
