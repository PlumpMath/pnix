//! nREPL Client 구조 정의
//!
//! pnix-old의 pnix_nrepl_client/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - ConnectionState: 연결 상태 enum 정의
//! - NreplResponse: nREPL 응답 구조 정의
//! - 실제 연결, 메시지 전송/수신 로직은 executor에서 구현

use serde::{Deserialize, Serialize};

/// nREPL 연결 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
  /// 연결됨
  Connected,
  /// 연결 끊김
  Disconnected,
  /// 재연결 시도 중
  Reconnecting,
}

/// nREPL 응답 구조 (순수 데이터)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NreplResponse {
  /// 표준 출력
  StdOut(String),
  /// 표준 에러 출력
  StdErr(String),
  /// 예외
  Exception(String),
  /// 값 (평가 결과)
  Value(String),
  /// 네임스페이스 변경
  Namespace(String),
  /// 기타 응답
  Other(String),
}

impl NreplResponse {
  /// 새로운 표준 출력 응답 생성
  pub fn stdout(content: impl Into<String>) -> Self {
    Self::StdOut(content.into())
  }

  /// 새로운 표준 에러 응답 생성
  pub fn stderr(content: impl Into<String>) -> Self {
    Self::StdErr(content.into())
  }

  /// 새로운 예외 응답 생성
  pub fn exception(content: impl Into<String>) -> Self {
    Self::Exception(content.into())
  }

  /// 새로운 값 응답 생성
  pub fn value(content: impl Into<String>) -> Self {
    Self::Value(content.into())
  }

  /// 새로운 네임스페이스 응답 생성
  pub fn namespace(ns: impl Into<String>) -> Self {
    Self::Namespace(ns.into())
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - NreplClient 구조체 및 메서드들 (연결 관리, 메시지 전송/수신)
// - send(), recv() (네트워크 I/O)
// - reconnect() (재연결 로직)
//
// 이 함수들은 네트워크 I/O, 상태 변경, 또는 실행 로직을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_connection_state() {
    assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
    assert_ne!(ConnectionState::Connected, ConnectionState::Disconnected);
  }

  #[test]
  fn test_nrepl_response_creation() {
    let resp = NreplResponse::stdout("output");
    assert!(matches!(resp, NreplResponse::StdOut(_)));
  }
}
