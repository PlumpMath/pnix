//! MCP JSON-RPC 프로토콜 타입 정의
//!
//! pnix-old의 symbolic_core/src/mcp/protocol.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 프로토콜 타입 정의만, 네트워크 I/O 없음
//!
//! ## 설계 철학
//!
//! MCP(Model Context Protocol)는 외부 LLM agent 와 외부 도구 사이의
//! JSON-RPC 2.0 기반 통신 프로토콜이다. pnix 는 LLM 없이 작동하는
//! deterministic AI substrate (`CLAUDE.md` OWNER-LAW CONSTITUTION) 이며,
//! 이 module 의 protocol type 정의는 substrate 의 의미/판단 owner 가
//! 아니라 외부 도구 surface 다. pnix substrate 가 MCP server 역할을
//! 하더라도 이는 외부 agent 가 호출하는 외부 transport 이고, substrate
//! 안의 promotion authority 는 PNIX + Human 두 owner 만 가진다.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ─────────────────────────────────────────────
// JSON-RPC 2.0 기본 타입
// ─────────────────────────────────────────────

/// JSON-RPC 2.0 요청
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
  /// JSON-RPC 버전 (보통 "2.0")
  pub jsonrpc: String,
  /// 요청 ID
  pub id: Value,
  /// 메서드 이름
  pub method: String,
  /// 파라미터 (선택적)
  #[serde(default)]
  pub params: Value,
}

/// JSON-RPC 2.0 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
  /// JSON-RPC 버전 (보통 "2.0")
  pub jsonrpc: String,
  /// 요청 ID
  pub id: Value,
  /// 성공 결과 (선택적)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result: Option<Value>,
  /// 에러 정보 (선택적)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<JsonRpcError>,
}

/// JSON-RPC 에러
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
  /// 에러 코드
  pub code: i32,
  /// 에러 메시지
  pub message: String,
  /// 추가 에러 데이터 (선택적)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<Value>,
}

impl JsonRpcResponse {
  /// 성공 응답 생성
  pub fn success(id: Value, result: Value) -> Self {
    Self {
      jsonrpc: "2.0".to_string(),
      id,
      result: Some(result),
      error: None,
    }
  }

  /// 에러 응답 생성
  pub fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
    Self {
      jsonrpc: "2.0".to_string(),
      id,
      result: None,
      error: Some(JsonRpcError {
        code,
        message: message.into(),
        data: None,
      }),
    }
  }
}

/// MCP 표준 에러 코드
pub struct McpError;

impl McpError {
  /// 파싱 에러 (-32700)
  pub const PARSE_ERROR: i32 = -32700;
  /// 잘못된 요청 (-32600)
  pub const INVALID_REQUEST: i32 = -32600;
  /// 메서드 없음 (-32601)
  pub const METHOD_NOT_FOUND: i32 = -32601;
  /// 잘못된 파라미터 (-32602)
  pub const INVALID_PARAMS: i32 = -32602;
  /// 내부 에러 (-32603)
  pub const INTERNAL_ERROR: i32 = -32603;
}

// ─────────────────────────────────────────────
// MCP 특화 타입
// ─────────────────────────────────────────────

/// MCP 툴 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
  /// 툴 이름
  pub name: String,
  /// 툴 설명
  pub description: String,
  /// 입력 스키마
  #[serde(rename = "inputSchema")]
  pub input_schema: Value,
}

/// tools/list 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResponse {
  /// 툴 목록
  pub tools: Vec<ToolDefinition>,
}

/// tools/call 요청 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
  /// 툴 이름
  pub name: String,
  /// 툴 인자
  pub arguments: Value,
}

/// tools/call 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
  /// 응답 컨텐츠 목록
  pub content: Vec<ToolContent>,
  /// 에러 여부 (선택적)
  #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
  pub is_error: Option<bool>,
}

/// 툴 응답 컨텐츠
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContent {
  /// 컨텐츠 타입
  #[serde(rename = "type")]
  pub content_type: String,
  /// 컨텐츠 텍스트
  pub text: String,
}

impl ToolContent {
  /// 텍스트 컨텐츠 생성
  pub fn text(s: impl Into<String>) -> Self {
    Self {
      content_type: "text".to_string(),
      text: s.into(),
    }
  }
}

impl ToolCallResponse {
  /// 성공 응답 생성
  pub fn success(text: impl Into<String>) -> Self {
    Self {
      content: vec![ToolContent::text(text)],
      is_error: None,
    }
  }

  /// 에러 응답 생성
  pub fn error(text: impl Into<String>) -> Self {
    Self {
      content: vec![ToolContent::text(text)],
      is_error: Some(true),
    }
  }
}

// ─────────────────────────────────────────────
// Initialize 프로토콜
// ─────────────────────────────────────────────

/// initialize 요청 파라미터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
  /// 프로토콜 버전
  #[serde(rename = "protocolVersion")]
  pub protocol_version: String,
  /// 클라이언트 기능
  pub capabilities: Value,
  /// 클라이언트 정보
  #[serde(rename = "clientInfo")]
  pub client_info: ClientInfo,
}

/// 클라이언트 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
  /// 클라이언트 이름
  pub name: String,
  /// 클라이언트 버전
  pub version: String,
}

/// initialize 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
  /// 프로토콜 버전
  #[serde(rename = "protocolVersion")]
  pub protocol_version: String,
  /// 서버 기능
  pub capabilities: ServerCapabilities,
  /// 서버 정보
  #[serde(rename = "serverInfo")]
  pub server_info: ServerInfo,
}

/// 서버 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
  /// 서버 이름
  pub name: String,
  /// 서버 버전
  pub version: String,
}

/// 서버 기능
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
  /// 툴 기능
  pub tools: ToolsCapability,
}

/// 툴 기능
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
  /// 리스트 변경 알림 지원 여부 (선택적)
  #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
  pub list_changed: Option<bool>,
}

impl Default for ServerCapabilities {
  fn default() -> Self {
    Self {
      tools: ToolsCapability { list_changed: None },
    }
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn test_jsonrpc_request_serialize() {
    let req = JsonRpcRequest {
      jsonrpc: "2.0".to_string(),
      id: json!(1),
      method: "tools/list".to_string(),
      params: json!({}),
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"method\":\"tools/list\""));
  }

  #[test]
  fn test_jsonrpc_response_success() {
    let resp = JsonRpcResponse::success(json!(1), json!({"result": "ok"}));
    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
  }

  #[test]
  fn test_jsonrpc_response_error() {
    let resp = JsonRpcResponse::error(json!(1), McpError::METHOD_NOT_FOUND, "Method not found");
    assert!(resp.result.is_none());
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32601);
  }

  #[test]
  fn test_tool_call_response() {
    let resp = ToolCallResponse::success("Hello, world!");
    assert_eq!(resp.content.len(), 1);
    assert_eq!(resp.content[0].content_type, "text");
    assert_eq!(resp.content[0].text, "Hello, world!");
    assert!(resp.is_error.is_none());
  }

  #[test]
  fn test_tool_call_error() {
    let resp = ToolCallResponse::error("Something went wrong");
    assert!(resp.is_error.unwrap());
  }

  #[test]
  fn test_mcp_error_codes() {
    assert_eq!(McpError::PARSE_ERROR, -32700);
    assert_eq!(McpError::INVALID_REQUEST, -32600);
    assert_eq!(McpError::METHOD_NOT_FOUND, -32601);
    assert_eq!(McpError::INVALID_PARAMS, -32602);
    assert_eq!(McpError::INTERNAL_ERROR, -32603);
  }
}
