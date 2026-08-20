//! MCP 서버 구조 정의
//!
//! pnix-old의 symbolic_core/src/mcp/server.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 서버 실행 로직(std::io I/O, 네트워크 I/O) 제외
//!
//! ## 참고
//!
//! 실제 서버 실행 로직은 executor에서 구현합니다.
//! 이 모듈은 구조 정의만 포함합니다.

use serde::{Deserialize, Serialize};

/// MCP 서버 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
  /// 초기화 상태 (실제 초기화는 executor에서)
  pub initialized: bool,
}

impl McpServer {
  /// 새 MCP 서버 구조 생성
  pub fn new() -> Self {
    Self { initialized: false }
  }
}

impl Default for McpServer {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_mcp_server_creation() {
    let server = McpServer::new();
    assert!(!server.initialized);
  }
}
