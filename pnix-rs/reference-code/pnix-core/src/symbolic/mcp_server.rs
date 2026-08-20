//! MCP Server 구조 정의
//!
//! pnix-old의 symbolic_core/src/mcp/server.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 서버 실행 로직 제외
//! - McpServer: MCP 서버 구조 정의
//! - 실제 서버 실행 (run_stdio, handle_message 등)은 executor에서 구현

use serde::{Deserialize, Serialize};

/// MCP 서버 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 서버 실행 로직 제외
/// - initialized: 초기화 상태 (구조 정의만)
/// - 실제 서버 실행 및 I/O는 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
  /// 초기화 상태 (구조 정의만)
  pub initialized: bool,
}

impl McpServer {
  /// 새로운 MCP 서버 생성
  pub fn new() -> Self {
    Self { initialized: false }
  }

  /// 초기화 상태 확인
  pub fn is_initialized(&self) -> bool {
    self.initialized
  }
}

impl Default for McpServer {
  fn default() -> Self {
    Self::new()
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - run_stdio() -> Result<()> (stdio 서버 실행, std::io I/O)
// - handle_message(msg) -> JsonRpcResponse (메시지 처리, JSON 파싱)
// - handle_request(req) -> JsonRpcResponse (요청 처리)
// - handle_initialize(id, params) -> JsonRpcResponse (초기화 처리)
// - handle_tools_list(id) -> JsonRpcResponse (도구 목록 처리)
// - handle_tools_call(id, params) -> JsonRpcResponse (도구 호출 처리)
//
// 이 함수들은 I/O 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_mcp_server_creation() {
    let server = McpServer::new();
    assert!(!server.is_initialized());
  }
}
