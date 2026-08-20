//! nREPL 프로토콜 변환 함수
//!
//! pnix-old의 symbolic_core/ipc/nrepl.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 프로토콜 변환만, nREPL 서버 실행 제외
//!
//! ## 참고
//!
//! 실제 사용 시에는 `bencode_rs` 의존성을 추가하고 `nrepl` feature를 활성화해야 합니다.
//! 현재는 구조 정의만 포함하여 헌법을 준수합니다.

use super::protocol::{IpcOp, IpcRequest, IpcResponse, IpcStatus};
use std::collections::HashMap;

/// nREPL 프로토콜 에러
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NreplError {
  /// 필수 필드 누락
  MissingField(&'static str),
  /// 알 수 없는 연산
  UnknownOp(String),
  /// 잘못된 형식
  InvalidFormat(String),
  /// 파싱 에러
  ParseError(String),
  /// IO 에러
  IoError(String),
  /// 파일 끝
  Eof,
}

impl std::fmt::Display for NreplError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::MissingField(field) => write!(f, "Missing required field: {}", field),
      Self::UnknownOp(op) => write!(f, "Unknown operation: {}", op),
      Self::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
      Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
      Self::IoError(msg) => write!(f, "IO error: {}", msg),
      Self::Eof => write!(f, "End of file"),
    }
  }
}

impl std::error::Error for NreplError {}

/// IpcOp를 nREPL op 문자열로 변환
pub fn ipc_op_to_nrepl_op(op: &IpcOp) -> &'static str {
  match op {
    IpcOp::Normalize => "normalize",
    IpcOp::Diff => "diff",
    IpcOp::Simulate => "simulate",
    IpcOp::Simplify => "simplify",
    IpcOp::Expand => "expand",
    IpcOp::Substitute => "substitute",
    IpcOp::Latex => "latex",
    IpcOp::Describe => "describe",
    IpcOp::Interrupt => "interrupt",
    IpcOp::Close => "close",
  }
}

/// nREPL op 문자열을 IpcOp로 변환
pub fn nrepl_op_to_ipc_op(op_str: &str) -> Result<IpcOp, NreplError> {
  match op_str {
    "normalize" => Ok(IpcOp::Normalize),
    "diff" => Ok(IpcOp::Diff),
    "simulate" => Ok(IpcOp::Simulate),
    "simplify" => Ok(IpcOp::Simplify),
    "expand" => Ok(IpcOp::Expand),
    "substitute" => Ok(IpcOp::Substitute),
    "latex" => Ok(IpcOp::Latex),
    "describe" => Ok(IpcOp::Describe),
    "interrupt" => Ok(IpcOp::Interrupt),
    "close" => Ok(IpcOp::Close),
    _ => Err(NreplError::UnknownOp(op_str.to_string())),
  }
}

/// IpcStatus를 nREPL status 문자열로 변환
pub fn ipc_status_to_nrepl_status(status: &IpcStatus) -> &'static str {
  match status {
    IpcStatus::Done => "done",
    IpcStatus::Error => "error",
    IpcStatus::Processing => "processing",
    IpcStatus::Session => "session",
  }
}

/// nREPL status 문자열을 IpcStatus로 변환
pub fn nrepl_status_to_ipc_status(status_str: &str) -> Result<IpcStatus, NreplError> {
  match status_str {
    "done" => Ok(IpcStatus::Done),
    "error" => Ok(IpcStatus::Error),
    "processing" => Ok(IpcStatus::Processing),
    "session" => Ok(IpcStatus::Session),
    _ => Err(NreplError::InvalidFormat(format!(
      "Unknown status: {}",
      status_str
    ))),
  }
}

/// IpcRequest를 nREPL 메시지 맵으로 변환 (구조 변환만)
///
/// 실제 bencode 인코딩은 feature gate로 제어됩니다.
pub fn ipc_request_to_nrepl_map(req: &IpcRequest) -> HashMap<String, String> {
  let mut map = HashMap::new();

  map.insert("op".to_string(), ipc_op_to_nrepl_op(&req.op).to_string());

  if let Some(ref session) = req.session {
    map.insert("session".to_string(), session.clone());
  }
  if let Some(ref id) = req.id {
    map.insert("id".to_string(), id.clone());
  }
  if let Some(ref code) = req.code {
    map.insert("code".to_string(), code.clone());
  }
  if let Some(ref var) = req.var {
    map.insert("var".to_string(), var.clone());
  }
  if let Some(t_min) = req.t_min {
    map.insert("t_min".to_string(), t_min.to_string());
  }
  if let Some(t_max) = req.t_max {
    map.insert("t_max".to_string(), t_max.to_string());
  }
  if let Some(steps) = req.steps {
    map.insert("steps".to_string(), steps.to_string());
  }
  if let Some(ref subs) = req.substitutions {
    // JSON 직렬화 (텍스트 변환만, 헌법 준수)
    if let Ok(json) = serde_json::to_string(subs) {
      map.insert("substitutions".to_string(), json);
    }
  }
  if let Some(max_iter) = req.max_iterations {
    map.insert("max_iterations".to_string(), max_iter.to_string());
  }

  map
}

/// nREPL 메시지 맵을 IpcRequest로 변환
pub fn nrepl_map_to_ipc_request(map: &HashMap<String, String>) -> Result<IpcRequest, NreplError> {
  let op_str = map.get("op").ok_or(NreplError::MissingField("op"))?;
  let op = nrepl_op_to_ipc_op(op_str)?;

  let session = map.get("session").cloned();
  let id = map.get("id").cloned();
  let code = map.get("code").cloned();
  let var = map.get("var").cloned();

  let t_min = map.get("t_min").and_then(|s| s.parse::<f64>().ok());
  let t_max = map.get("t_max").and_then(|s| s.parse::<f64>().ok());
  let steps = map.get("steps").and_then(|s| s.parse::<usize>().ok());
  let max_iterations = map
    .get("max_iterations")
    .and_then(|s| s.parse::<usize>().ok());

  let substitutions = map
    .get("substitutions")
    .and_then(|s| serde_json::from_str::<HashMap<String, f64>>(s).ok());

  Ok(IpcRequest {
    op,
    session,
    id,
    code,
    var,
    t_min,
    t_max,
    steps,
    substitutions,
    max_iterations,
  })
}

/// IpcResponse를 nREPL 메시지 맵으로 변환 (구조 변환만)
pub fn ipc_response_to_nrepl_map(resp: &IpcResponse) -> HashMap<String, String> {
  let mut map = HashMap::new();

  map.insert(
    "status".to_string(),
    format!("[\"{}\"]", ipc_status_to_nrepl_status(&resp.status)),
  );

  if let Some(ref session) = resp.session {
    map.insert("session".to_string(), session.clone());
  }
  if let Some(ref id) = resp.id {
    map.insert("id".to_string(), id.clone());
  }
  if let Some(ref value) = resp.value {
    map.insert("value".to_string(), value.clone());
  }
  if let Some(ref latex) = resp.latex {
    map.insert("latex".to_string(), latex.clone());
  }
  if let Some(ref error) = resp.error {
    map.insert("err".to_string(), error.clone());
  }
  if let (Some(ref times), Some(ref values)) = (&resp.times, &resp.values) {
    // JSON 직렬화 (텍스트 변환만, 헌법 준수)
    if let Ok(times_json) = serde_json::to_string(times) {
      map.insert("times".to_string(), times_json);
    }
    if let Ok(values_json) = serde_json::to_string(values) {
      map.insert("values".to_string(), values_json);
    }
  }
  if let Some(ref meta) = resp.meta {
    // JSON 직렬화 (텍스트 변환만, 헌법 준수)
    if let Ok(meta_json) = serde_json::to_string(meta) {
      map.insert("meta".to_string(), meta_json);
    }
  }

  map
}

/// nREPL 메시지 맵을 IpcResponse로 변환
pub fn nrepl_map_to_ipc_response(map: &HashMap<String, String>) -> Result<IpcResponse, NreplError> {
  // Status는 리스트 형식 ["done"] 또는 단일 문자열일 수 있음
  let status_raw = map
    .get("status")
    .ok_or(NreplError::MissingField("status"))?;
  let status = parse_nrepl_status(status_raw)?;

  let session = map.get("session").cloned();
  let id = map.get("id").cloned();
  let value = map.get("value").cloned();
  let latex = map.get("latex").cloned();
  let error = map.get("err").or_else(|| map.get("error")).cloned();

  let times = map
    .get("times")
    .and_then(|s| serde_json::from_str::<Vec<f64>>(s).ok());
  let values = map
    .get("values")
    .and_then(|s| serde_json::from_str::<Vec<f64>>(s).ok());
  let meta = map
    .get("meta")
    .and_then(|s| serde_json::from_str::<HashMap<String, serde_json::Value>>(s).ok());

  Ok(IpcResponse {
    status,
    session,
    id,
    value,
    latex,
    error,
    times,
    values,
    meta,
  })
}

fn parse_nrepl_status(status_raw: &str) -> Result<IpcStatus, NreplError> {
  let trimmed = status_raw.trim();

  if trimmed.starts_with('[') {
    if let Ok(list) = serde_json::from_str::<Vec<String>>(trimmed) {
      return pick_status_from_list(&list);
    }
  }

  let status_str = trimmed.trim_matches('"');
  nrepl_status_to_ipc_status(status_str)
}

fn pick_status_from_list(list: &[String]) -> Result<IpcStatus, NreplError> {
  if list.iter().any(|s| s == "error") {
    return Ok(IpcStatus::Error);
  }
  if list.iter().any(|s| s == "done") {
    return Ok(IpcStatus::Done);
  }
  if list.iter().any(|s| s == "processing") {
    return Ok(IpcStatus::Processing);
  }
  if list.iter().any(|s| s == "session") {
    return Ok(IpcStatus::Session);
  }

  Err(NreplError::InvalidFormat(format!(
    "Unknown status list: {:?}",
    list
  )))
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ipc_op_to_nrepl_op() {
    assert_eq!(ipc_op_to_nrepl_op(&IpcOp::Normalize), "normalize");
    assert_eq!(ipc_op_to_nrepl_op(&IpcOp::Diff), "diff");
  }

  #[test]
  fn test_nrepl_op_to_ipc_op() {
    assert_eq!(nrepl_op_to_ipc_op("normalize").unwrap(), IpcOp::Normalize);
    assert_eq!(nrepl_op_to_ipc_op("diff").unwrap(), IpcOp::Diff);
    assert!(nrepl_op_to_ipc_op("unknown").is_err());
  }

  #[test]
  fn test_ipc_request_to_nrepl_map() {
    let req = IpcRequest::normalize("sin(x)");
    let map = ipc_request_to_nrepl_map(&req);
    assert_eq!(map.get("op"), Some(&"normalize".to_string()));
    assert_eq!(map.get("code"), Some(&"sin(x)".to_string()));
  }

  #[test]
  fn test_nrepl_map_to_ipc_request() {
    let mut map = HashMap::new();
    map.insert("op".to_string(), "normalize".to_string());
    map.insert("code".to_string(), "sin(x)".to_string());
    map.insert("id".to_string(), "123".to_string());

    let req = nrepl_map_to_ipc_request(&map).unwrap();
    assert_eq!(req.op, IpcOp::Normalize);
    assert_eq!(req.code, Some("sin(x)".to_string()));
    assert_eq!(req.id, Some("123".to_string()));
  }

  #[test]
  fn test_ipc_response_to_nrepl_map() {
    let resp = IpcResponse::done("cos(x)", "\\cos(x)");
    let map = ipc_response_to_nrepl_map(&resp);
    assert!(map.get("status").unwrap().contains("done"));
    assert_eq!(map.get("value"), Some(&"cos(x)".to_string()));
  }

  #[test]
  fn test_nrepl_map_to_ipc_response() {
    let mut map = HashMap::new();
    map.insert("status".to_string(), "[\"done\"]".to_string());
    map.insert("value".to_string(), "cos(x)".to_string());
    map.insert("id".to_string(), "123".to_string());

    let resp = nrepl_map_to_ipc_response(&map).unwrap();
    assert_eq!(resp.status, IpcStatus::Done);
    assert_eq!(resp.value, Some("cos(x)".to_string()));
  }

  #[test]
  fn test_nrepl_map_to_ipc_response_multi_status() {
    let mut map = HashMap::new();
    map.insert("status".to_string(), "[\"done\",\"session\"]".to_string());
    let resp = nrepl_map_to_ipc_response(&map).unwrap();
    assert_eq!(resp.status, IpcStatus::Done);
  }

  #[test]
  fn test_nrepl_map_to_ipc_response_error_precedence() {
    let mut map = HashMap::new();
    map.insert("status".to_string(), "[\"error\",\"done\"]".to_string());
    let resp = nrepl_map_to_ipc_response(&map).unwrap();
    assert_eq!(resp.status, IpcStatus::Error);
  }

  #[test]
  fn test_roundtrip_request() {
    let req = IpcRequest::diff("x^2", "x");
    let map = ipc_request_to_nrepl_map(&req);
    let req_back = nrepl_map_to_ipc_request(&map).unwrap();

    assert_eq!(req.op, req_back.op);
    assert_eq!(req.code, req_back.code);
    assert_eq!(req.var, req_back.var);
  }

  #[test]
  fn test_roundtrip_response() {
    let resp = IpcResponse::simulation(vec![0.0, 1.0], vec![1.0, 2.0]);
    let map = ipc_response_to_nrepl_map(&resp);
    let resp_back = nrepl_map_to_ipc_response(&map).unwrap();

    assert_eq!(resp.status, resp_back.status);
    assert_eq!(resp.times, resp_back.times);
    assert_eq!(resp.values, resp_back.values);
  }
}
