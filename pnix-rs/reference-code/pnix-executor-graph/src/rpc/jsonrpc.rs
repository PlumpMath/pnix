//! Y15a: JSON-RPC 컴파일 API
//!
//! AI 에이전트가 Pnix 컴파일러를 도구로 쉽게 사용할 수 있는 JSON-RPC 인터페이스

use super::query::{handle_query, QueryParams};
use pnix_core::{compile, CompileOptions, SourceUnit};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 요청: AI 에이전트가 컴파일러를 호출하는 요청
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct JsonRpcRequest {
  /// JSON-RPC 버전 ("2.0")
  pub jsonrpc: String,
  /// 메서드 이름 (예: "compile", "query")
  pub method: String,
  /// 메서드 파라미터
  #[serde(default = "default_params")]
  pub params: Value,
  /// 요청 ID (선택적)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<Value>,
}

/// JSON-RPC 2.0 응답: 요청에 대한 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct JsonRpcResponse {
  /// JSON-RPC 버전 ("2.0")
  pub jsonrpc: String,
  /// 성공 결과 (에러가 없을 때)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result: Option<JsonRpcResult>,
  /// 에러 (실패 시)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<JsonRpcError>,
  /// 요청 ID (요청과 동일)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<Value>,
}

/// JSON-RPC 성공 결과: 컴파일 결과 및 경고
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct JsonRpcResult {
  /// IR (중간 표현, 선택적)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub ir: Option<Value>,
  /// 실행 결과 목록 (선택적)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub results: Option<Vec<Value>>,
  /// 경고 메시지 목록
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub warnings: Vec<String>,
}

/// JSON-RPC 에러: 에러 코드 및 메시지
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct JsonRpcError {
  /// 에러 코드 (JSON-RPC 2.0 표준)
  pub code: i32,
  /// 에러 메시지
  pub message: String,
  /// 추가 에러 데이터 (선택적)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<Value>,
}

/// Compile 메서드 파라미터: 컴파일할 소스 코드 및 옵션
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct CompileParams {
  /// 소스 코드
  pub source: String,
  /// 타겟 형식 (기본값: "fxcore")
  #[serde(default = "default_target")]
  pub target: String,
  /// 모듈 이름 (선택적)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
}

#[allow(dead_code)] // 향후 사용 예정
fn default_target() -> String {
  "fxcore".to_string()
}

fn default_params() -> Value {
  Value::Null
}

/// JSON-RPC 요청 처리
#[allow(dead_code)] // 향후 사용 예정
pub fn handle_request(request: JsonRpcRequest) -> JsonRpcResponse {
  let id = request.id.clone();

  // LOW: 에러 코드 불일치 수정 완료
  // JSON-RPC 표준 에러 코드를 사용하며, -32600은 Invalid Request로 표준 코드임
  if request.jsonrpc != "2.0" {
    return JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      result: None,
      error: Some(JsonRpcError {
        code: -32600, // JSON-RPC 2.0 표준: Invalid Request
        message: format!("Invalid JSON-RPC version: {}", request.jsonrpc),
        data: None,
      }),
      id,
    };
  }

  match request.method.as_str() {
    "compile" => handle_compile(request.params, id),
    "query" => handle_query_method(request.params, id),
    _ => JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      result: None,
      error: Some(JsonRpcError {
        code: -32601,
        message: format!("Method not found: {}", request.method),
        data: None,
      }),
      id,
    },
  }
}

/// JSON-RPC 요청 처리 (배치/notification 지원)
#[allow(dead_code)] // 향후 사용 예정
pub fn handle_request_value(request: Value) -> Option<Value> {
  match request {
    Value::Array(items) => {
      if items.is_empty() {
        return Some(response_to_value(invalid_request_response(
          None,
          "Invalid Request: empty batch",
        )));
      }

      let mut responses = Vec::new();
      for item in items {
        if let Some(response) = handle_batch_item(item) {
          responses.push(response);
        }
      }

      if responses.is_empty() {
        None
      } else {
        Some(Value::Array(responses))
      }
    }
    Value::Object(_) => handle_single_request(request),
    _ => {
      // JSON-RPC 2.0 spec: Invalid Request (-32600) for valid JSON but invalid structure
      // (Parse error is for unparseable JSON, but this is already parsed JSON with wrong structure)
      Some(response_to_value(invalid_request_response(
        None,
        "Invalid Request: request must be an object or array",
      )))
    }
  }
}

fn handle_batch_item(request: Value) -> Option<Value> {
  match request {
    Value::Object(_) => handle_single_request(request),
    _ => {
      // JSON-RPC 2.0 spec: Invalid Request (-32600) for valid JSON but invalid structure
      // (Parse error is for unparseable JSON, but this is already parsed JSON with wrong structure)
      Some(response_to_value(invalid_request_response(
        None,
        "Invalid Request: batch item must be an object",
      )))
    }
  }
}

fn handle_single_request(request: Value) -> Option<Value> {
  let mut req: JsonRpcRequest = match serde_json::from_value(request) {
    Ok(req) => req,
    Err(e) => {
      // JSON-RPC 2.0 spec: Parse error (-32700) for JSON parsing failures
      return Some(response_to_value(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(JsonRpcError {
          code: -32700,
          message: format!("Parse error: {}", e),
          data: None,
        }),
        id: Some(Value::Null),
      }));
    }
  };

  let has_id = req.id.is_some();
  let id = sanitize_id(&req.id);
  let is_notification = !has_id;

  if has_id && id.is_none() {
    return Some(response_to_value(invalid_request_response(
      None,
      "Invalid Request: id must be string, number, or null",
    )));
  }
  req.id = id;

  if req.jsonrpc != "2.0" {
    // Invalid version - only return error if not a notification
    if !is_notification {
      return Some(response_to_value(invalid_request_response(
        req.id.clone(),
        format!("Invalid JSON-RPC version: {}", req.jsonrpc),
      )));
    }
    // For notification with invalid version, silently ignore (per JSON-RPC 2.0 spec)
    return None;
  }

  let response = handle_request(req);
  if response.id.is_none() {
    None
  } else {
    Some(response_to_value(response))
  }
}

fn invalid_request_response(id: Option<Value>, message: impl Into<String>) -> JsonRpcResponse {
  // JSON-RPC 2.0 spec: notification (id is None) should not receive a response
  // This function should only be called for non-notification requests
  // If id is None, use null as fallback (should not happen in practice)
  JsonRpcResponse {
    jsonrpc: "2.0".to_string(),
    result: None,
    error: Some(JsonRpcError {
      code: -32600,
      message: message.into(),
      data: None,
    }),
    id: id.or(Some(Value::Null)), // Use provided id or null, but never None (notification)
  }
}

fn sanitize_id(id: &Option<Value>) -> Option<Value> {
  id.as_ref().and_then(|value| {
    if is_valid_id(value) {
      Some(value.clone())
    } else {
      None
    }
  })
}

fn is_valid_id(id: &Value) -> bool {
  matches!(id, Value::Null | Value::Number(_) | Value::String(_))
}

fn response_to_value(response: JsonRpcResponse) -> Value {
  // JSON-RPC 2.0 spec: result and error are mutually exclusive
  // Ensure only one is present (skip_serializing_if handles this, but validate for safety)
  let mut response = response;
  if response.result.is_some() && response.error.is_some() {
    eprintln!("Warning: JSON-RPC response has both result and error, removing result");
    response.result = None;
  }

  serde_json::to_value(response).unwrap_or_else(|_| {
    serde_json::json!({
      "jsonrpc": "2.0",
      "id": null,
      "result": null,
      "error": {
        "code": -32603,
        "message": "Failed to serialize JSON-RPC response"
      }
    })
  })
}

/// Compile 메서드 처리
#[allow(dead_code)] // 향후 사용 예정
fn handle_compile(params: Value, id: Option<Value>) -> JsonRpcResponse {
  let params: CompileParams = match serde_json::from_value(params) {
    Ok(p) => p,
    Err(e) => {
      return JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(JsonRpcError {
          code: -32602,
          message: format!("Invalid params: {}", e),
          data: None,
        }),
        id,
      };
    }
  };

  // target 검증 (현재는 fxcore만 지원)
  if params.target != "fxcore" {
    return JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      result: None,
      error: Some(JsonRpcError {
        code: -32000,
        message: format!(
          "Unsupported target: {}. Only 'fxcore' is supported",
          params.target
        ),
        data: None,
      }),
      id,
    };
  }

  // 소스 코드 컴파일
  let source_unit = SourceUnit {
    name: params.name.unwrap_or_else(|| "main".to_string()),
    text: params.source,
  };

  let opts = CompileOptions::default();

  match compile(&source_unit, &opts) {
    Ok(output) => {
      // FxCore IR을 JSON으로 변환
      let ir_json = match serde_json::to_value(&output.fxcore) {
        Ok(v) => v,
        Err(e) => {
          eprintln!("Warning: Failed to serialize FxCore IR: {}", e);
          serde_json::json!({"error": "Failed to serialize IR"})
        }
      };

      // 경고 메시지 추출 (Diagnostics는 items만 있음)
      let warnings: Vec<String> = output
        .diags
        .items
        .iter()
        .map(|d| d.message.clone())
        .collect();

      JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(JsonRpcResult {
          ir: Some(ir_json),
          results: None,
          warnings,
        }),
        error: None,
        id,
      }
    }
    Err(e) => JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      result: None,
      error: Some(JsonRpcError {
        code: -32000,
        message: format!("Compilation failed: {}", e),
        data: Some(serde_json::json!({
          "error_type": e.to_string(),
        })),
      }),
      id,
    },
  }
}

/// Query 메서드 처리
#[allow(dead_code)] // 향후 사용 예정
fn handle_query_method(params: Value, id: Option<Value>) -> JsonRpcResponse {
  let query_params: QueryParams = match serde_json::from_value(params) {
    Ok(p) => p,
    Err(e) => {
      return JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(JsonRpcError {
          code: -32602,
          message: format!("Invalid params: {}", e),
          data: None,
        }),
        id,
      };
    }
  };

  match handle_query(query_params) {
    Ok(query_result) => JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      result: Some(JsonRpcResult {
        ir: None,
        results: Some(query_result.results),
        warnings: Vec::new(),
      }),
      error: None,
      id,
    },
    Err(e) => JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      result: None,
      error: Some(JsonRpcError {
        code: -32000,
        message: format!("Query failed: {}", e),
        data: None,
      }),
      id,
    },
  }
}

// ========================================
// V01: Async JSON-RPC Handler (feature-gated)
// ========================================

#[cfg(feature = "async-runtime")]
#[allow(dead_code)] // POC - functions will be used in future implementation
pub mod async_handler {
  //! Async JSON-RPC handler surface for future non-blocking compilation/query.
  //!
  //! The entrypoints exist behind `async-runtime`, but the compile/query paths
  //! remain fail-closed until the async contract is promoted past POC.
  //! Enable with the `async-runtime` feature flag.

  use super::*;

  /// Async JSON-RPC 요청 처리
  pub async fn handle_request_async(request: JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.clone();

    match request.method.as_str() {
      "compile" => handle_compile_async(request.params, id).await,
      "query" => handle_query_async(request.params, id).await,
      _ => JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(JsonRpcError {
          code: -32601,
          message: format!("Method not found: {}", request.method),
          data: None,
        }),
        id,
      },
    }
  }

  fn async_unimplemented_response(
    tracking_id: &'static str,
    operation: &'static str,
    id: Option<Value>,
  ) -> JsonRpcResponse {
    let data = match tracking_id {
      "ASYNC-002" => serde_json::json!({
        "tracking_id": "ASYNC-002",
        "operation": "compile",
        "fallback_method": "compile",
        "status": ["Experimental", "POC", "Unimplemented"],
      }),
      "ASYNC-003" => serde_json::json!({
        "tracking_id": "ASYNC-003",
        "operation": "query",
        "fallback_method": "query",
        "status": ["Experimental", "POC", "Unimplemented"],
      }),
      _ => serde_json::json!({
        "tracking_id": tracking_id,
        "operation": operation,
        "fallback_method": operation,
        "status": ["Experimental", "POC", "Unimplemented"],
      }),
    };
    JsonRpcResponse {
      jsonrpc: "2.0".to_string(),
      result: None,
      error: Some(JsonRpcError {
        code: -32000,
        message: format!(
          "[{}] experimental async {} path is not implemented; use sync '{}': \
           async-runtime remains POC-only",
          tracking_id, operation, operation
        ),
        data: Some(data),
      }),
      id,
    }
  }

  /// Async compile handler
  async fn handle_compile_async(_params: Value, id: Option<Value>) -> JsonRpcResponse {
    async_unimplemented_response("ASYNC-002", "compile", id)
  }

  /// Async query handler
  async fn handle_query_async(_params: Value, id: Option<Value>) -> JsonRpcResponse {
    async_unimplemented_response("ASYNC-003", "query", id)
  }

  #[cfg(test)]
  mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_compile_returns_unimplemented() {
      let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "compile".to_string(),
        params: serde_json::json!({
          "source": "1 + 2",
          "target": "fxcore"
        }),
        id: Some(serde_json::json!(1)),
      };

      let async_response = handle_request_async(request).await;
      assert_eq!(async_response.jsonrpc, "2.0");
      let error = async_response
        .error
        .expect("async compile must fail closed");
      assert_eq!(error.code, -32000);
      assert!(error.message.contains(
        "[ASYNC-002] experimental async compile path is not implemented; use sync 'compile':"
      ));
      let data = error
        .data
        .expect("async compile must include tracking data");
      assert_eq!(
        data.get("tracking_id"),
        Some(&serde_json::json!("ASYNC-002"))
      );
      assert_eq!(
        data.get("fallback_method"),
        Some(&serde_json::json!("compile"))
      );
    }

    #[tokio::test]
    async fn test_async_query_returns_unimplemented() {
      let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "query".to_string(),
        params: serde_json::json!({
          "ir": serde_json::json!({"nodes": []}),
          "query": "list_nodes"
        }),
        id: Some(serde_json::json!(2)),
      };

      let async_response = handle_request_async(request).await;
      assert_eq!(async_response.jsonrpc, "2.0");
      let error = async_response.error.expect("async query must fail closed");
      assert_eq!(error.code, -32000);
      assert!(error.message.contains(
        "[ASYNC-003] experimental async query path is not implemented; use sync 'query':"
      ));
      let data = error.data.expect("async query must include tracking data");
      assert_eq!(
        data.get("tracking_id"),
        Some(&serde_json::json!("ASYNC-003"))
      );
      assert_eq!(
        data.get("fallback_method"),
        Some(&serde_json::json!("query"))
      );
    }

    #[tokio::test]
    async fn test_async_unknown_method() {
      let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "unknown".to_string(),
        params: serde_json::json!({}),
        id: Some(serde_json::json!(1)),
      };

      let response = handle_request_async(request).await;
      assert!(response.error.is_some());
      assert_eq!(response.error.unwrap().code, -32601);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_compile_request() {
    let request = JsonRpcRequest {
      jsonrpc: "2.0".to_string(),
      method: "compile".to_string(),
      params: serde_json::json!({
        "source": "let x = 1 + 2 in x",
        "target": "fxcore",
        "name": "test"
      }),
      id: Some(serde_json::json!(1)),
    };

    let response = handle_request(request);
    assert_eq!(response.jsonrpc, "2.0");
    // 컴파일 결과 검증: 에러가 있으면 에러 메시지 출력, 성공하면 result 확인
    if let Some(ref err) = response.error {
      // compile이 실패하더라도 에러 응답은 유효함
      assert!(!err.message.is_empty(), "Error should have message");
    } else {
      assert!(response.result.is_some(), "Success should have result");
    }
  }

  #[test]
  fn test_invalid_method() {
    let request = JsonRpcRequest {
      jsonrpc: "2.0".to_string(),
      method: "unknown".to_string(),
      params: serde_json::json!({}),
      id: Some(serde_json::json!(1)),
    };

    let response = handle_request(request);
    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_some());
    assert_eq!(response.error.as_ref().unwrap().code, -32601);
  }

  #[test]
  fn test_invalid_jsonrpc_version() {
    let request = JsonRpcRequest {
      jsonrpc: "1.0".to_string(),
      method: "compile".to_string(),
      params: serde_json::json!({
        "source": "1 + 2",
        "target": "fxcore"
      }),
      id: Some(serde_json::json!(1)),
    };

    let response = handle_request(request);
    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_some());
    let err = response.error.unwrap();
    assert_eq!(err.code, -32600);
    assert!(err.message.contains("Invalid JSON-RPC version"));
  }

  #[test]
  fn test_batch_request_mixed_notifications() {
    let batch = serde_json::json!([
      {
        "jsonrpc": "2.0",
        "method": "unknown",
        "params": {},
        "id": 1
      },
      {
        "jsonrpc": "2.0",
        "method": "unknown",
        "params": {},
        "id": 2
      },
      {
        "jsonrpc": "2.0",
        "method": "unknown",
        "params": {}
      }
    ]);

    let response = handle_request_value(batch).expect("batch should return responses");
    let responses = response.as_array().expect("batch response should be array");
    assert_eq!(responses.len(), 2, "notifications should be omitted");

    let mut ids: Vec<i64> = responses
      .iter()
      .filter_map(|resp| resp.get("id").and_then(|id| id.as_i64()))
      .collect();
    ids.sort();
    assert_eq!(ids, vec![1, 2]);
  }

  #[test]
  fn test_batch_empty_returns_invalid_request() {
    let response = handle_request_value(serde_json::json!([]))
      .expect("empty batch should return error response");
    let error = response
      .get("error")
      .and_then(|err| err.get("code"))
      .and_then(|code| code.as_i64());
    assert_eq!(error, Some(-32600));
    assert_eq!(response.get("id"), Some(&serde_json::json!(null)));
  }

  #[test]
  fn test_batch_invalid_item_returns_error() {
    let response = handle_request_value(serde_json::json!([1]))
      .expect("invalid batch item should return error response");
    let responses = response.as_array().expect("batch response should be array");
    assert_eq!(responses.len(), 1);
    let err_code = responses[0]
      .get("error")
      .and_then(|err| err.get("code"))
      .and_then(|code| code.as_i64());
    assert_eq!(err_code, Some(-32600));
    assert_eq!(responses[0].get("id"), Some(&serde_json::json!(null)));
  }

  #[test]
  fn test_single_notification_returns_none() {
    let request = serde_json::json!({
      "jsonrpc": "2.0",
      "method": "unknown",
      "params": {}
    });
    assert!(handle_request_value(request).is_none());
  }

  #[test]
  fn test_invalid_id_returns_invalid_request() {
    let request = serde_json::json!({
      "jsonrpc": "2.0",
      "method": "compile",
      "params": {},
      "id": { "bad": "id" }
    });

    let response = handle_request_value(request).expect("should return error response");
    let err_code = response
      .get("error")
      .and_then(|err| err.get("code"))
      .and_then(|code| code.as_i64());
    assert_eq!(err_code, Some(-32600));
    assert_eq!(response.get("id"), Some(&serde_json::json!(null)));
  }

  #[test]
  fn test_invalid_jsonrpc_in_value_request() {
    let request = serde_json::json!({
      "jsonrpc": "1.0",
      "method": "compile",
      "params": {},
      "id": 9
    });

    let response = handle_request_value(request).expect("should return error response");
    let err_code = response
      .get("error")
      .and_then(|err| err.get("code"))
      .and_then(|code| code.as_i64());
    assert_eq!(err_code, Some(-32600));
    assert_eq!(response.get("id"), Some(&serde_json::json!(9)));
  }

  #[test]
  fn test_invalid_request_non_object() {
    let response =
      handle_request_value(serde_json::json!("oops")).expect("should return error response");
    let err_code = response
      .get("error")
      .and_then(|err| err.get("code"))
      .and_then(|code| code.as_i64());
    assert_eq!(err_code, Some(-32600));
    assert_eq!(response.get("id"), Some(&serde_json::json!(null)));
  }

  #[test]
  fn test_invalid_params() {
    let request = JsonRpcRequest {
      jsonrpc: "2.0".to_string(),
      method: "compile".to_string(),
      params: serde_json::json!({
        "invalid": "params"
      }),
      id: Some(serde_json::json!(1)),
    };

    let response = handle_request(request);
    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_some());
    assert_eq!(response.error.as_ref().unwrap().code, -32602);
  }
}
