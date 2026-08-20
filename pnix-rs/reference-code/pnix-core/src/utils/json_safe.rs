//! 안전한 JSON 직렬화 유틸리티
//!
//! NaN/Infinity float 값을 검증하고 명시적 에러를 반환하는 헬퍼 함수들

use serde_json::Value;

/// JSON 직렬화 에러 타입
#[derive(Debug, thiserror::Error)]
pub enum JsonSafeError {
  #[error("Cannot serialize NaN to JSON")]
  NaNValue,
  #[error("Cannot serialize Infinity to JSON")]
  InfinityValue,
  #[error("Cannot serialize -Infinity to JSON")]
  NegativeInfinityValue,
  #[error("JSON serialization error: {0}")]
  SerdeError(#[from] serde_json::Error),
}

/// JSON Value에서 NaN/Infinity를 검증하고 에러 반환
///
/// NaN과 Infinity는 JSON 표준에서 지원하지 않으므로 명시적 에러를 반환합니다.
/// "조용한 성공 금지" 원칙에 따라 silent conversion 대신 에러를 반환합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn sanitize_json_value(value: Value) -> Result<Value, JsonSafeError> {
  match value {
    Value::Number(n) => {
      if let Some(f) = n.as_f64() {
        if f.is_nan() {
          return Err(JsonSafeError::NaNValue);
        } else if f.is_infinite() {
          if f.is_sign_positive() {
            return Err(JsonSafeError::InfinityValue);
          } else {
            return Err(JsonSafeError::NegativeInfinityValue);
          }
        }
      }
      Ok(Value::Number(n))
    }
    Value::Array(arr) => {
      let sanitized: Result<Vec<_>, _> = arr.into_iter().map(sanitize_json_value).collect();
      Ok(Value::Array(sanitized?))
    }
    Value::Object(obj) => {
      let sanitized: Result<Vec<_>, _> = obj
        .into_iter()
        .map(|(k, v)| sanitize_json_value(v).map(|v| (k, v)))
        .collect();
      Ok(Value::Object(sanitized?.into_iter().collect()))
    }
    other => Ok(other),
  }
}

/// 안전한 JSON 직렬화 (NaN/Infinity 처리)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn to_string_safe(value: &Value) -> Result<String, JsonSafeError> {
  let sanitized = sanitize_json_value(value.clone())?;
  Ok(serde_json::to_string(&sanitized)?)
}

/// 안전한 JSON 직렬화 (pretty print, NaN/Infinity 처리)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn to_string_pretty_safe(value: &Value) -> Result<String, JsonSafeError> {
  let sanitized = sanitize_json_value(value.clone())?;
  Ok(serde_json::to_string_pretty(&sanitized)?)
}

/// T를 안전하게 JSON 문자열로 직렬화 (NaN/Infinity 처리)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn serialize_safe<T: serde::Serialize>(value: &T) -> Result<String, JsonSafeError> {
  let json_value = serde_json::to_value(value)?;
  to_string_safe(&json_value)
}

/// T를 안전하게 JSON 문자열로 직렬화 (pretty print, NaN/Infinity 처리)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn serialize_pretty_safe<T: serde::Serialize>(value: &T) -> Result<String, JsonSafeError> {
  let json_value = serde_json::to_value(value)?;
  to_string_pretty_safe(&json_value)
}
