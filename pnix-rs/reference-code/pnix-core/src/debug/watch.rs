//! Watch Expression 구조 정의
//!
//! pnix-old의 pnix_debug_console/src/watch.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - WatchExpression: 워치 표현식 구조 정의
//! - 실제 평가 및 값 업데이트 로직은 executor에서 구현

use serde::{Deserialize, Serialize};

/// 워치 ID
pub type WatchId = u64;

/// 워치 표현식 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchExpression {
  /// 워치 ID
  pub id: WatchId,
  /// 표현식 문자열 (예: "x + y", "myVar.field")
  pub expression: String,
  /// 현재 값 (문자열로 표현, 구조 정의만)
  pub current_value: Option<String>,
  /// 이전 값 (변경 감지용, 구조 정의만)
  pub previous_value: Option<String>,
  /// 값이 변경될 때만 알림
  pub notify_on_change: bool,
  /// 활성화 여부
  pub enabled: bool,
  /// 평가 에러 (구조 정의만)
  pub error: Option<String>,
}

impl WatchExpression {
  /// 새로운 워치 표현식 생성
  pub fn new(id: WatchId, expression: impl Into<String>) -> Self {
    Self {
      id,
      expression: expression.into(),
      current_value: None,
      previous_value: None,
      notify_on_change: true,
      enabled: true,
      error: None,
    }
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - update_value(value) -> bool (값 업데이트 및 변경 감지)
// - has_changed() -> bool (변경 여부 확인)
// - set_error(error) (에러 설정)
// - clear_error() (에러 클리어)
// - WatchManager (상태 관리)
// - evaluate() (표현식 평가)
//
// 이 함수들은 값 계산, 상태 변경, 또는 표현식 평가를 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_watch_expression_creation() {
    let watch = WatchExpression::new(1, "x + y");
    assert_eq!(watch.id, 1);
    assert_eq!(watch.expression, "x + y");
    assert!(watch.enabled);
    assert!(watch.notify_on_change);
  }
}
