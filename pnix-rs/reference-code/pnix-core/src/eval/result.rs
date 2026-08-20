//! Unified Result 구조 정의
//!
//! pnix-old의 pnix_unified_eval/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 변환 실행 로직 제외

use serde::{Deserialize, Serialize};

/// 통합 평가 결과: 평가 결과를 나타내는 타입
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UnifiedResult {
  /// 실수 결과 (meaning_core 파이프라인에서)
  Numeric(f64),
  /// 정수 결과
  Int(i64),
  /// 불리언 결과
  Bool(bool),
  /// 문자열 결과
  String(String),
  /// 리스트 결과
  List(Vec<UnifiedResult>),
  /// AttrSet 결과
  AttrSet(Vec<(String, UnifiedResult)>),
  /// 다중 바인딩 (여러 정의를 가진 프로그램용)
  Bindings(Vec<(String, f64)>),
  /// 혼합 타입 다중 바인딩
  BindingsMulti(Vec<(String, UnifiedResult)>),
  /// 평가 중 에러
  Error(String),
}

/// 소스 코드 언어 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
  /// Nix 언어
  Nix,
  /// Python 언어
  Python,
  /// Clojure 언어
  Clojure,
  /// 자동 감지 언어
  Auto,
}

impl Language {
  /// 언어 이름을 문자열로 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn name(&self) -> &'static str {
    match self {
      Language::Nix => "nix",
      Language::Python => "python",
      Language::Clojure => "clojure",
      Language::Auto => "auto",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_unified_result_numeric() {
    let result = UnifiedResult::Numeric(42.0);
    match result {
      UnifiedResult::Numeric(n) => assert_eq!(n, 42.0),
      _ => panic!("Expected Numeric"),
    }
  }

  #[test]
  fn test_language_name() {
    assert_eq!(Language::Nix.name(), "nix");
    assert_eq!(Language::Python.name(), "python");
    assert_eq!(Language::Clojure.name(), "clojure");
  }
}
