//! Debugger 구조 정의
//!
//! pnix-old의 pnix_debug_console/src/debugger.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - DebuggerState: 디버거 상태 enum 정의
//! - StepType: 스텝 타입 enum 정의
//! - CallFrame: 호출 스택 프레임 구조 정의
//! - VariableInfo: 변수 정보 구조 정의
//! - 실제 실행 로직 (continue_execution, step_over, push_frame 등)은 executor에서 구현

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 디버거 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebuggerState {
  /// 실행 중
  Running,
  /// 일시 정지됨 (브레이크포인트 등)
  Paused,
  /// 스텝 모드
  Stepping,
}

/// 스텝 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepType {
  /// Step Over (현재 라인 완료 후 중단)
  Over,
  /// Step Into (함수 호출 시 진입)
  Into,
  /// Step Out (현재 함수 종료까지 실행)
  Out,
}

/// 호출 스택 프레임
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallFrame {
  /// 함수 이름
  pub function_name: String,
  /// 파일 경로
  pub file: String,
  /// 라인 번호
  pub line: u32,
  /// 컬럼 번호
  pub column: u32,
  /// 로컬 변수들 (변수 이름 → 값 문자열)
  pub locals: HashMap<String, String>,
  /// 인자들 (인자 이름 → 값 문자열)
  pub arguments: HashMap<String, String>,
}

impl CallFrame {
  /// 새로운 호출 프레임 생성
  pub fn new(function_name: impl Into<String>, file: impl Into<String>, line: u32) -> Self {
    Self {
      function_name: function_name.into(),
      file: file.into(),
      line,
      column: 0,
      locals: HashMap::new(),
      arguments: HashMap::new(),
    }
  }

  /// 컬럼 번호 설정
  pub fn with_column(mut self, column: u32) -> Self {
    self.column = column;
    self
  }

  /// 로컬 변수 추가 (구조 변경만)
  pub fn with_local(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
    self.locals.insert(name.into(), value.into());
    self
  }

  /// 인자 추가 (구조 변경만)
  pub fn with_argument(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
    self.arguments.insert(name.into(), value.into());
    self
  }
}

/// 변수 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableInfo {
  /// 변수 이름
  pub name: String,
  /// 변수 값 (문자열 표현, 구조 정의만)
  pub value: String,
  /// 변수 타입
  pub type_name: String,
  /// 중첩된 변수들 (구조체/객체의 필드)
  pub children: Vec<VariableInfo>,
}

impl VariableInfo {
  /// 새로운 변수 정보 생성
  pub fn new(
    name: impl Into<String>,
    value: impl Into<String>,
    type_name: impl Into<String>,
  ) -> Self {
    Self {
      name: name.into(),
      value: value.into(),
      type_name: type_name.into(),
      children: Vec::new(),
    }
  }

  /// 자식 변수 추가 (구조 변경만)
  pub fn with_child(mut self, child: VariableInfo) -> Self {
    self.children.push(child);
    self
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - Debugger 구조체 (상태 관리)
// - continue_execution() (실행 재개)
// - pause() (일시 정지)
// - step_over(), step_into(), step_out() (스텝 실행)
// - push_frame(), pop_frame() (호출 스택 관리)
// - get_variable_value() (변수 값 조회, 런타임 상태 접근)
// - should_break_on_step() (스텝 중단 여부 확인, 값 계산)
// - on_line_execution() (라인 실행 처리, 상태 변경)
// - inspect_variable() (변수 검사, 값 파싱)
//
// 이 함수들은 값 계산, 상태 변경, 또는 실행 로직을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_call_frame_creation() {
    let frame = CallFrame::new("test_func", "test.rs", 10);
    assert_eq!(frame.function_name, "test_func");
    assert_eq!(frame.file, "test.rs");
    assert_eq!(frame.line, 10);
  }

  #[test]
  fn test_call_frame_with_local() {
    let frame = CallFrame::new("test_func", "test.rs", 10).with_local("x", "42");
    assert_eq!(frame.locals.get("x"), Some(&"42".to_string()));
  }

  #[test]
  fn test_variable_info_creation() {
    let var = VariableInfo::new("x", "42", "Int");
    assert_eq!(var.name, "x");
    assert_eq!(var.value, "42");
    assert_eq!(var.type_name, "Int");
  }
}
