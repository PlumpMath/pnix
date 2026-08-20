//! RPC 재시도 테스트: RPC 호출 재시도 정책 테스트
//!
//! RPC 호출 실패 시 재시도 정책이 올바르게 동작하는지 검증합니다.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use pnix_executor_graph::{run_with_retry, RpcError, RpcRetryPolicy};
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn retries_on_server_error_then_succeeds() {
  let policy = RpcRetryPolicy::new(3, 0);
  let attempts = Arc::new(AtomicUsize::new(0));
  let attempts_shared = attempts.clone();

  let result = run_with_retry(&policy, || {
    let attempts = attempts_shared.clone();
    async move {
      let current = attempts.fetch_add(1, Ordering::SeqCst);
      if current < 2 {
        Err(RpcError::HttpStatus {
          status: StatusCode::INTERNAL_SERVER_ERROR,
          body: "backend boom".to_string(),
        })
      } else {
        Ok(json!({"ok": true}))
      }
    }
  })
  .await;

  assert!(result.is_ok());
  assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn does_not_retry_on_backend_error() {
  let policy = RpcRetryPolicy::new(5, 0);
  let attempts = Arc::new(AtomicUsize::new(0));
  let attempts_shared = attempts.clone();

  let result: Result<serde_json::Value, RpcError> = run_with_retry(&policy, || {
    let attempts = attempts_shared.clone();
    async move {
      attempts.fetch_add(1, Ordering::SeqCst);
      Err(RpcError::Backend {
        name: "demo".to_string(),
        body: json!({"status": "error", "message": "bad input"}),
      })
    }
  })
  .await;

  assert!(result.is_err());
  assert_eq!(attempts.load(Ordering::SeqCst), 1);
}
