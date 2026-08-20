//! IPC 서버 구조 정의
//!
//! pnix-old의 symbolic_core/src/ipc/server.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 서버 실행 로직(TcpListener, std::net I/O, 멀티스레딩) 제외
//!
//! ## 참고
//!
//! 실제 서버 실행 로직은 executor에서 구현합니다.
//! 이 모듈은 구조 정의만 포함합니다.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// IPC 서버 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcServer {
  /// 세션 상태 맵 (실제 관리는 executor에서)
  pub sessions: HashMap<String, SessionState>,
}

/// 세션 상태 구조
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
  /// 변수 바인딩 (실제 계산은 executor에서)
  pub bindings: HashMap<String, f64>,
  /// 단위 설정
  pub units: HashMap<String, String>,
}

impl IpcServer {
  /// 새 IPC 서버 구조 생성
  pub fn new() -> Self {
    Self {
      sessions: HashMap::new(),
    }
  }
}

impl Default for IpcServer {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ipc_server_creation() {
    let server = IpcServer::new();
    assert!(server.sessions.is_empty());
  }

  #[test]
  fn test_session_state() {
    let state = SessionState::default();
    assert!(state.bindings.is_empty());
    assert!(state.units.is_empty());
  }
}
