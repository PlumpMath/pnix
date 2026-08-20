//! IPC Server 구조 정의
//!
//! pnix-old의 symbolic_core/src/ipc/server.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 서버 실행 로직 제외
//! - IpcServer: IPC 서버 구조 정의
//! - SessionState: 세션 상태 구조 정의
//! - 실제 서버 실행 (listen_tcp, handle_request 등)은 executor에서 구현

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 세션 상태 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실행 로직 제외
/// - bindings: 변수 바인딩 (구조 정의만, 실제 값 계산은 executor에서)
/// - units: 단위 설정 (구조 정의만)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
  /// 변수 바인딩 (구조 정의만)
  pub bindings: HashMap<String, f64>,
  /// 단위 설정 (구조 정의만)
  pub units: HashMap<String, String>,
}

impl SessionState {
  /// 새로운 세션 상태 생성
  pub fn new() -> Self {
    Self {
      bindings: HashMap::new(),
      units: HashMap::new(),
    }
  }
}

impl Default for SessionState {
  fn default() -> Self {
    Self::new()
  }
}

/// IPC 서버 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 서버 실행 로직 제외
/// - sessions: 세션 상태 맵 (구조 정의만)
/// - 실제 서버 실행 및 TCP I/O는 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcServer {
  /// 세션 상태 맵 (구조 정의만, 실제 세션 관리는 executor에서)
  pub sessions: HashMap<String, SessionState>,
}

impl IpcServer {
  /// 새로운 IPC 서버 생성
  pub fn new() -> Self {
    Self {
      sessions: HashMap::new(),
    }
  }

  /// 세션 상태 조회
  pub fn get_session(&self, session_id: &str) -> Option<&SessionState> {
    self.sessions.get(session_id)
  }
}

impl Default for IpcServer {
  fn default() -> Self {
    Self::new()
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - listen_tcp(addr) -> Result<()> (TCP 서버 시작, TcpListener, std::net I/O)
// - handle_client(stream, ...) -> Result<()> (클라이언트 처리, std::thread::spawn)
// - handle_request(req) -> IpcResponse (요청 처리)
// - handle_normalize(req) -> IpcResponse (정규화 처리)
// - handle_diff(req) -> IpcResponse (미분 처리)
// - handle_simulate(req) -> IpcResponse (시뮬레이션 처리)
//
// 이 함수들은 I/O 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_session_state_creation() {
    let state = SessionState::new();
    assert!(state.bindings.is_empty());
    assert!(state.units.is_empty());
  }

  #[test]
  fn test_ipc_server_creation() {
    let server = IpcServer::new();
    assert!(server.sessions.is_empty());
  }
}
