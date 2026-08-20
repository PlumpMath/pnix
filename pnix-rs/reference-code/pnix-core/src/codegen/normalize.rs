//! JSON normalization for stable replay hash
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 변환만, 값 계산 없음

use serde_json::Value;

/// FxCore JSON 정규화
///
/// 정규화 규칙:
/// - 객체 키는 사전순으로 정렬
/// - 배열은 이름 기준 정렬하여 안정화
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn normalize_fxcore(mut v: Value) -> Value {
  // v = {"name": "...", "morphisms": [...]}
  if let Some(morphs) = v.get_mut("morphisms").and_then(|x| x.as_array_mut()) {
    morphs.sort_by(|a, b| {
      let an = a.get("name").and_then(|x| x.as_str()).unwrap_or("");
      let bn = b.get("name").and_then(|x| x.as_str()).unwrap_or("");
      an.cmp(bn)
    });
  }
  canonicalize(v)
}

/// SSA JSON 정규화
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn normalize_ssa(mut v: Value) -> Value {
  if let Some(blocks) = v.get_mut("blocks").and_then(|x| x.as_array_mut()) {
    blocks.sort_by(|a, b| {
      let al = a.get("label").and_then(|x| x.as_str()).unwrap_or("");
      let bl = b.get("label").and_then(|x| x.as_str()).unwrap_or("");
      al.cmp(bl)
    });
  }
  canonicalize(v)
}

/// Build IR JSON 정규화
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn normalize_build_ir(mut v: Value) -> Value {
  // toolchain/lib는 순서 비의미면 정렬
  if let Some(tools) = v
    .pointer_mut("/build/deps/toolchain")
    .and_then(|x| x.as_array_mut())
  {
    tools.sort_by(|a, b| a.as_str().unwrap_or("").cmp(b.as_str().unwrap_or("")));
  }
  if let Some(libs) = v
    .pointer_mut("/build/deps/lib")
    .and_then(|x| x.as_array_mut())
  {
    libs.sort_by(|a, b| a.as_str().unwrap_or("").cmp(b.as_str().unwrap_or("")));
  }
  canonicalize(v)
}

/// JSON 정규화: JSON object 키를 사전순으로 재구성 (재귀)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn canonicalize(v: Value) -> Value {
  match v {
    Value::Object(map) => {
      let mut keys: Vec<_> = map.keys().cloned().collect();
      keys.sort();
      let mut new = serde_json::Map::new();
      for k in keys {
        let vv = map.get(&k).cloned().unwrap_or(Value::Null);
        new.insert(k, canonicalize(vv));
      }
      Value::Object(new)
    }
    Value::Array(arr) => Value::Array(arr.into_iter().map(canonicalize).collect()),
    other => other,
  }
}

/// 정규화된 JSON을 예쁘게 포맷된 문자열로 변환 (안정적)
///
/// ## 헌법 준수 (P0-1)
///
/// 텍스트 생성만, 파일 I/O 없음
///
/// Panics if serialization fails (enforces "no silent success" principle)
pub fn to_pretty(v: &Value) -> String {
  serde_json::to_string_pretty(v).expect("JSON serialization should not fail for normalized Value")
}
