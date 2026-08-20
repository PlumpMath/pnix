//! Determinism tests for pnix-core
//!
//! 헌법 준수: 동일 입력 → 동일 출력 보장
//!
//! 이 테스트는 pnix-core가 결정적(deterministic) 출력을 생성함을 증명합니다.
//! 같은 소스 코드를 여러 번 컴파일하면 항상 같은 결과가 나와야 합니다.

use pnix_core::codegen::normalize::canonicalize;
use serde_json::Value;

/// 간단한 테스트 fixture
const SIMPLE_FIXTURE: &str = r#"
{
  "name": "test",
  "morphisms": [
    {"name": "b", "inputs": ["a"]},
    {"name": "a", "inputs": []}
  ]
}
"#;

#[test]
fn canonicalize_is_deterministic() {
  // 같은 JSON을 여러 번 정규화해도 같은 결과가 나와야 함
  let json1: Value = serde_json::from_str(SIMPLE_FIXTURE).unwrap();
  let json2: Value = serde_json::from_str(SIMPLE_FIXTURE).unwrap();

  let canon1 = canonicalize(json1);
  let canon2 = canonicalize(json2);

  let str1 = serde_json::to_string(&canon1).unwrap();
  let str2 = serde_json::to_string(&canon2).unwrap();

  assert_eq!(str1, str2, "canonicalize should be deterministic");
}

#[test]
fn canonicalize_sorts_keys() {
  // 키 순서가 다른 JSON도 같은 결과로 정규화되어야 함
  let json1: Value = serde_json::from_str(r#"{"b": 1, "a": 2}"#).unwrap();
  let json2: Value = serde_json::from_str(r#"{"a": 2, "b": 1}"#).unwrap();

  let canon1 = canonicalize(json1);
  let canon2 = canonicalize(json2);

  let str1 = serde_json::to_string(&canon1).unwrap();
  let str2 = serde_json::to_string(&canon2).unwrap();

  assert_eq!(
    str1, str2,
    "canonicalize should sort keys deterministically"
  );
}

#[test]
fn normalize_fxcore_is_deterministic() {
  // FxCore 정규화도 결정적이어야 함
  use pnix_core::codegen::normalize::normalize_fxcore;

  let json1: Value = serde_json::from_str(SIMPLE_FIXTURE).unwrap();
  let json2: Value = serde_json::from_str(SIMPLE_FIXTURE).unwrap();

  let norm1 = normalize_fxcore(json1);
  let norm2 = normalize_fxcore(json2);

  let str1 = serde_json::to_string(&norm1).unwrap();
  let str2 = serde_json::to_string(&norm2).unwrap();

  assert_eq!(str1, str2, "normalize_fxcore should be deterministic");
}

#[test]
fn normalize_fxcore_sorts_morphisms() {
  // morphisms 배열도 이름 순으로 정렬되어야 함
  use pnix_core::codegen::normalize::normalize_fxcore;

  let unsorted: Value = serde_json::from_str(
    r#"
    {
      "name": "test",
      "morphisms": [
        {"name": "z", "inputs": []},
        {"name": "a", "inputs": []},
        {"name": "m", "inputs": []}
      ]
    }
    "#,
  )
  .unwrap();

  let normalized = normalize_fxcore(unsorted);
  let morphisms = normalized
    .get("morphisms")
    .and_then(|v| v.as_array())
    .unwrap();

  // 이름 순으로 정렬되어야 함
  assert_eq!(morphisms[0].get("name").and_then(|v| v.as_str()), Some("a"));
  assert_eq!(morphisms[1].get("name").and_then(|v| v.as_str()), Some("m"));
  assert_eq!(morphisms[2].get("name").and_then(|v| v.as_str()), Some("z"));
}

#[test]
fn replay_hash_consistency() {
  // replay_hash는 정규화된 JSON의 해시여야 함
  // 같은 입력은 같은 해시를 생성해야 함
  use pnix_core::codegen::normalize::normalize_fxcore;
  use pnix_hash::{Digest, Sha256};

  let json1: Value = serde_json::from_str(SIMPLE_FIXTURE).unwrap();
  let json2: Value = serde_json::from_str(SIMPLE_FIXTURE).unwrap();

  let norm1 = normalize_fxcore(json1);
  let norm2 = normalize_fxcore(json2);

  // replay_hash는 정규화된 JSON의 SHA256 해시
  let str1 = serde_json::to_string(&norm1).unwrap();
  let str2 = serde_json::to_string(&norm2).unwrap();

  let mut hasher1 = Sha256::new();
  hasher1.update(str1.as_bytes());
  let hash1 = format!("{:x}", hasher1.finalize());

  let mut hasher2 = Sha256::new();
  hasher2.update(str2.as_bytes());
  let hash2 = format!("{:x}", hasher2.finalize());

  assert_eq!(
    hash1, hash2,
    "replay_hash should be consistent for same input"
  );
  assert_eq!(str1, str2, "normalized JSON should be identical");
}

#[test]
fn determinism_across_runs() {
  // 여러 번 실행해도 같은 결과가 나와야 함
  use pnix_core::codegen::normalize::normalize_fxcore;

  let json: Value = serde_json::from_str(SIMPLE_FIXTURE).unwrap();

  let mut results = Vec::new();
  for _ in 0..10 {
    let json_clone = json.clone();
    let normalized = normalize_fxcore(json_clone);
    let serialized = serde_json::to_string(&normalized).unwrap();
    results.push(serialized);
  }

  // 모든 결과가 같아야 함
  let first = &results[0];
  for result in results.iter().skip(1) {
    assert_eq!(
      first, result,
      "determinism should hold across multiple runs"
    );
  }
}

#[test]
fn normalize_ssa_sorts_blocks() {
  use pnix_core::codegen::normalize::normalize_ssa;

  let unsorted: Value = serde_json::from_str(
    r#"
    {
      "blocks": [
        { "label": "z" },
        { "label": "a" },
        { "label": "m" }
      ]
    }
    "#,
  )
  .unwrap();

  let normalized = normalize_ssa(unsorted);
  let labels: Vec<_> = normalized
    .get("blocks")
    .and_then(|v| v.as_array())
    .unwrap()
    .iter()
    .map(|block| block.get("label").and_then(|v| v.as_str()).unwrap_or(""))
    .collect();

  assert_eq!(labels, vec!["a", "m", "z"]);
}

#[test]
fn normalize_build_ir_sorts_deps() {
  use pnix_core::codegen::normalize::normalize_build_ir;

  let unsorted: Value = serde_json::from_str(
    r#"
    {
      "build": {
        "deps": {
          "toolchain": ["z", "a"],
          "lib": ["b", "a"]
        }
      }
    }
    "#,
  )
  .unwrap();

  let normalized = normalize_build_ir(unsorted);
  let tools = normalized
    .pointer("/build/deps/toolchain")
    .and_then(|v| v.as_array())
    .unwrap();
  let libs = normalized
    .pointer("/build/deps/lib")
    .and_then(|v| v.as_array())
    .unwrap();

  assert_eq!(tools[0].as_str(), Some("a"));
  assert_eq!(tools[1].as_str(), Some("z"));
  assert_eq!(libs[0].as_str(), Some("a"));
  assert_eq!(libs[1].as_str(), Some("b"));
}
