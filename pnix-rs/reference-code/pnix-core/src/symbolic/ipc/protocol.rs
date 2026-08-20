//! IPC 프로토콜 타입 정의
//!
//! pnix-old의 symbolic_core/ipc/protocol.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 없음

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// IPC 연산 종류
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IpcOp {
  /// 표현식 정규화
  Normalize,
  /// 미분
  Diff,
  /// 시뮬레이션
  Simulate,
  /// 단순화
  Simplify,
  /// 전개 (분배법칙)
  Expand,
  /// 대입
  Substitute,
  /// LaTeX 변환
  Latex,
  /// 세션 정보
  Describe,
  /// 인터럽트
  Interrupt,
  /// 세션 종료
  Close,
}

/// IPC 요청
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
  /// 연산 종류
  pub op: IpcOp,

  /// 세션 ID (nREPL 호환)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub session: Option<String>,

  /// 요청 ID (응답 매칭용)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,

  /// 표현식 코드
  #[serde(skip_serializing_if = "Option::is_none")]
  pub code: Option<String>,

  /// 미분 변수
  #[serde(skip_serializing_if = "Option::is_none")]
  pub var: Option<String>,

  /// 시뮬레이션 파라미터
  #[serde(skip_serializing_if = "Option::is_none")]
  pub t_min: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub t_max: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub steps: Option<usize>,

  /// 대입용 변수 맵
  #[serde(skip_serializing_if = "Option::is_none")]
  pub substitutions: Option<HashMap<String, f64>>,

  /// 단순화 반복 횟수
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_iterations: Option<usize>,
}

impl IpcRequest {
  /// 간단한 normalize 요청 생성
  pub fn normalize(code: &str) -> Self {
    Self {
      op: IpcOp::Normalize,
      session: None,
      id: None,
      code: Some(code.to_string()),
      var: None,
      t_min: None,
      t_max: None,
      steps: None,
      substitutions: None,
      max_iterations: None,
    }
  }

  /// diff 요청 생성
  pub fn diff(code: &str, var: &str) -> Self {
    Self {
      op: IpcOp::Diff,
      session: None,
      id: None,
      code: Some(code.to_string()),
      var: Some(var.to_string()),
      t_min: None,
      t_max: None,
      steps: None,
      substitutions: None,
      max_iterations: None,
    }
  }
}

/// IPC 응답 상태
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IpcStatus {
  /// 성공적으로 완료
  Done,
  /// 오류 발생
  Error,
  /// 처리 중 (스트리밍용)
  Processing,
  /// 세션 정보
  Session,
}

/// IPC 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
  /// 응답 상태
  pub status: IpcStatus,

  /// 세션 ID
  #[serde(skip_serializing_if = "Option::is_none")]
  pub session: Option<String>,

  /// 요청 ID (echo back)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,

  /// 결과 값 (정규화된 표현식)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub value: Option<String>,

  /// LaTeX 출력
  #[serde(skip_serializing_if = "Option::is_none")]
  pub latex: Option<String>,

  /// 에러 메시지
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,

  /// 시뮬레이션 결과: 시간 배열
  #[serde(skip_serializing_if = "Option::is_none")]
  pub times: Option<Vec<f64>>,

  /// 시뮬레이션 결과: 값 배열
  #[serde(skip_serializing_if = "Option::is_none")]
  pub values: Option<Vec<f64>>,

  /// 추가 메타데이터
  #[serde(skip_serializing_if = "Option::is_none")]
  pub meta: Option<HashMap<String, serde_json::Value>>,
}

impl IpcResponse {
  /// 성공 응답
  pub fn done(value: &str, latex: &str) -> Self {
    Self {
      status: IpcStatus::Done,
      session: None,
      id: None,
      value: Some(value.to_string()),
      latex: Some(latex.to_string()),
      error: None,
      times: None,
      values: None,
      meta: None,
    }
  }

  /// 에러 응답
  pub fn error(message: &str) -> Self {
    Self {
      status: IpcStatus::Error,
      session: None,
      id: None,
      value: None,
      latex: None,
      error: Some(message.to_string()),
      times: None,
      values: None,
      meta: None,
    }
  }

  /// 시뮬레이션 결과 응답
  pub fn simulation(times: Vec<f64>, values: Vec<f64>) -> Self {
    Self {
      status: IpcStatus::Done,
      session: None,
      id: None,
      value: None,
      latex: None,
      error: None,
      times: Some(times),
      values: Some(values),
      meta: None,
    }
  }

  /// 세션 ID 설정
  pub fn with_session(mut self, session: &str) -> Self {
    self.session = Some(session.to_string());
    self
  }

  /// 요청 ID 설정
  pub fn with_id(mut self, id: &str) -> Self {
    self.id = Some(id.to_string());
    self
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request_serialization() {
    let req = IpcRequest::normalize("sin(x)");
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"op\":\"normalize\""));
    assert!(json.contains("\"code\":\"sin(x)\""));
  }

  #[test]
  fn test_request_deserialization() {
    let json = r#"{"op":"diff","code":"x^2","var":"x"}"#;
    let req: IpcRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.op, IpcOp::Diff);
    assert_eq!(req.code, Some("x^2".to_string()));
    assert_eq!(req.var, Some("x".to_string()));
  }

  #[test]
  fn test_response_done() {
    let resp = IpcResponse::done("cos(x)", "\\cos(x)");
    assert_eq!(resp.status, IpcStatus::Done);
    assert_eq!(resp.value, Some("cos(x)".to_string()));
  }

  #[test]
  fn test_response_error() {
    let resp = IpcResponse::error("Parse error");
    assert_eq!(resp.status, IpcStatus::Error);
    assert_eq!(resp.error, Some("Parse error".to_string()));
  }
}
