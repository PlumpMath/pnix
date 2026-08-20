//! LSP Code Actions 구조 정의
//!
//! pnix-old의 pnix_lsp_code_actions/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - HoverInfo: Hover 정보 구조 정의
//! - DiagnosticEnhancement: 진단 개선 정보 구조 정의
//! - 실제 Code Action 생성, Hover 제공, 진단 개선 로직은 executor에서 구현

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Hover 정보 구조 (순수 데이터)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverInfo {
  /// 표시할 내용 (문자열, 구조 정의만)
  pub contents: String,
  /// 범위 시작 라인 (선택적)
  pub range_start_line: Option<u32>,
  /// 범위 시작 컬럼 (선택적)
  pub range_start_column: Option<u32>,
  /// 범위 끝 라인 (선택적)
  pub range_end_line: Option<u32>,
  /// 범위 끝 컬럼 (선택적)
  pub range_end_column: Option<u32>,
}

impl HoverInfo {
  /// 새로운 Hover 정보 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(contents: impl Into<String>) -> Self {
    Self {
      contents: contents.into(),
      range_start_line: None,
      range_start_column: None,
      range_end_line: None,
      range_end_column: None,
    }
  }

  /// 범위 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_range(
    mut self,
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
  ) -> Self {
    self.range_start_line = Some(start_line);
    self.range_start_column = Some(start_column);
    self.range_end_line = Some(end_line);
    self.range_end_column = Some(end_column);
    self
  }
}

/// 진단 개선 정보 구조 (순수 데이터)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticEnhancement {
  /// 개선된 메시지
  pub message: String,
  /// 관련 코드 액션 제목들
  pub code_action_titles: Vec<String>,
  /// 심각도 (숫자: 1=Error, 2=Warning, 3=Info, 4=Hint)
  pub severity: u32,
  /// 관련 정보 (문자열, 구조 정의만)
  pub related_information: Vec<String>,
}

impl DiagnosticEnhancement {
  /// 새로운 진단 개선 정보 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(message: impl Into<String>, severity: u32) -> Self {
    Self {
      message: message.into(),
      code_action_titles: Vec::new(),
      severity,
      related_information: Vec::new(),
    }
  }

  /// 코드 액션 제목 추가 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn with_code_action(mut self, title: impl Into<String>) -> Self {
    self.code_action_titles.push(title.into());
    self
  }

  /// 관련 정보 추가 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn with_related_info(mut self, info: impl Into<String>) -> Self {
    self.related_information.push(info.into());
    self
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - CodeActionProvider 구조체 및 provide_code_actions() (Code Action 생성)
// - HoverProvider 구조체 및 provide_hover() (Hover 정보 제공)
// - DiagnosticEnhancer 구조체 및 enhance() (진단 개선)
//
// 이 함수들은 파일 분석, 코드 생성, 또는 실행 로직을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_hover_info_creation() {
    let hover = HoverInfo::new("test content");
    assert_eq!(hover.contents, "test content");
  }

  #[test]
  fn test_hover_info_with_range() {
    let hover = HoverInfo::new("test").with_range(1, 0, 1, 10);
    assert_eq!(hover.range_start_line, Some(1));
    assert_eq!(hover.range_end_column, Some(10));
  }

  #[test]
  fn test_diagnostic_enhancement_creation() {
    let enhancement = DiagnosticEnhancement::new("error message", 1);
    assert_eq!(enhancement.message, "error message");
    assert_eq!(enhancement.severity, 1);
  }

  #[test]
  fn test_diagnostic_enhancement_with_code_action() {
    let enhancement = DiagnosticEnhancement::new("error", 1).with_code_action("Fix syntax");
    assert_eq!(enhancement.code_action_titles.len(), 1);
  }
}
