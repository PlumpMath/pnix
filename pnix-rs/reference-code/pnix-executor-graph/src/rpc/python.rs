//! Python backend RPC client
//!
//! Calls python backend via HTTP JSON (same protocol as clojure)
//!
//! Stage-2: inputs/outputs map 기반으로 전환
//! - 요청: { "op": "call", "name": "...", "inputs": {...}, "args": [...] }
//! - 응답: { "status": "ok", "outputs": {...} } 또는 { "status": "ok", "result": ... }

use serde_json::Value;

use super::client::{RpcClient, RpcError};

/// Call a python backend morphism
///
/// # Arguments
/// * `name` - Morphism name (prefix removed, e.g., "add")
/// * `args` - JSON value (Stage-2: map 선호, Stage-1 호환: array도 지원)
pub async fn call(client: &RpcClient, name: &str, args: Value) -> Result<Value, RpcError> {
  client.call(name, args).await
}
