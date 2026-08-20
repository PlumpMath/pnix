//! MCP(Model Context Protocol) 지원
//!
//! pnix-old의 symbolic_core/src/mcp에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 프로토콜 타입 정의만, 네트워크 I/O 없음
//!
//! ## 모듈 구성
//!
//! - `protocol`: JSON-RPC 2.0 및 MCP 타입 정의
//! - `server`: MCP 서버 구조 정의 (실행 로직 제외)

pub mod protocol;
pub mod server;

pub use protocol::{
  ClientInfo, InitializeParams, InitializeResponse, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
  McpError, ServerCapabilities, ServerInfo, ToolCallParams, ToolCallResponse, ToolContent,
  ToolDefinition, ToolsCapability, ToolsListResponse,
};
pub use server::McpServer;
