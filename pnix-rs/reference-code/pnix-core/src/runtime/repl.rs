//! REPL 구조 정의
//!
//! pnix-old의 pnix_repl/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - ReplState: REPL 상태 구조 정의
//! - ReplCommand: REPL 명령어 enum 정의
//! - SymbolicResult: Symbolic 모드 결과 구조 정의
//! - 실제 REPL 실행, 명령어 처리, 상태 관리 로직은 executor에서 구현

use crate::config::ReplMode;
use serde::{Deserialize, Serialize};

/// REPL 상태 구조 (순수 데이터)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplState {
  /// 현재 모드
  pub mode: ReplMode,
  /// 히스토리 (최근 입력들)
  pub history: Vec<String>,
  /// 현재 입력 버퍼
  pub input_buffer: String,
  /// 마지막 결과
  pub last_result: Option<String>,
  /// 에러 메시지
  pub error: Option<String>,
}

/// REPL 명령어
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplCommand {
  /// 모드 전환
  SwitchMode(ReplMode),
  /// 히스토리 표시
  ShowHistory,
  /// 히스토리 지우기
  ClearHistory,
  /// 도움말 표시
  Help,
  /// 종료
  Exit,
  /// 커스텀 명령어
  Custom(String),
}

/// Symbolic 모드 결과 구조 (순수 데이터)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolicResult {
  /// 성공 여부
  pub success: bool,
  /// 결과 (성공 시)
  pub result: Option<String>,
  /// LaTeX 출력 (선택적)
  pub latex: Option<String>,
  /// 단계별 설명 (선택적)
  pub steps: Vec<String>,
  /// 에러 메시지 (실패 시)
  pub error: Option<String>,
  /// 신뢰도 (0.0 ~ 1.0)
  pub confidence: f64,
}

impl SymbolicResult {
  /// 빈 결과 생성
  pub fn empty() -> Self {
    Self {
      success: false,
      result: None,
      latex: None,
      steps: Vec::new(),
      error: None,
      confidence: 0.0,
    }
  }

  /// 에러 결과 생성
  pub fn error(message: impl Into<String>) -> Self {
    Self {
      success: false,
      result: None,
      latex: None,
      steps: Vec::new(),
      error: Some(message.into()),
      confidence: 0.0,
    }
  }

  /// 성공 결과 생성
  pub fn success(result: impl Into<String>) -> Self {
    Self {
      success: true,
      result: Some(result.into()),
      latex: None,
      steps: Vec::new(),
      error: None,
      confidence: 1.0,
    }
  }
}

impl ReplState {
  /// 새로운 REPL 상태 생성
  pub fn new(mode: ReplMode) -> Self {
    Self {
      mode,
      history: Vec::new(),
      input_buffer: String::new(),
      last_result: None,
      error: None,
    }
  }

  /// 모드 변경 (구조 변경만)
  pub fn set_mode(&mut self, mode: ReplMode) {
    self.mode = mode;
  }

  /// 입력 버퍼 설정 (구조 변경만)
  pub fn set_input_buffer(&mut self, input: impl Into<String>) {
    self.input_buffer = input.into();
  }

  /// 히스토리에 추가 (구조 변경만)
  pub fn add_to_history(&mut self, input: impl Into<String>) {
    self.history.push(input.into());
  }
}

impl Default for ReplState {
  fn default() -> Self {
    Self::new(ReplMode::default())
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 구조체와 메서드들은 executor/runtime 계층에서 구현하세요:
// - MultiModeRepl 구조체 및 메서드들 (run, process_input 등)
// - SymbolicHandler 구조체 및 메서드들 (process, handle_derivative 등)
// - 실제 REPL 루프 실행, 명령어 처리, 상태 업데이트 로직
//
// 이 함수들은 REPL 실행, 상태 관리, 또는 값 계산을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_repl_mode_next() {
    let mode = ReplMode::Programming;
    assert_eq!(mode.next(), ReplMode::Llm);
    assert_eq!(ReplMode::Llm.next(), ReplMode::Symbolic);
    assert_eq!(ReplMode::Symbolic.next(), ReplMode::Programming);
  }

  #[test]
  fn test_repl_state_creation() {
    let state = ReplState::new(ReplMode::Llm);
    assert_eq!(state.mode, ReplMode::Llm);
    assert_eq!(state.history.len(), 0);
  }

  #[test]
  fn test_symbolic_result() {
    let result = SymbolicResult::success("x^2");
    assert!(result.success);
    assert_eq!(result.result, Some("x^2".to_string()));
  }
}
