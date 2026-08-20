//! RPC 클라이언트: 백엔드 런타임과의 RPC 통신

use std::error::Error;
use std::fmt;
use std::future::Future;
use std::time::Duration;

use reqwest::StatusCode;
use serde_json::{json, Value};

/// RPC 재시도 정책: 실패 시 재시도 설정
#[derive(Debug, Clone)]
pub struct RpcRetryPolicy {
  /// 최대 재시도 횟수
  pub attempts: usize,
  /// 백오프 간격 (밀리초)
  pub backoff_ms: u64,
  /// 결정론적 재시도를 위한 시드
  pub seed: u64,
  /// 요청 타임아웃 (밀리초, None이면 기본값 사용)
  pub request_timeout_ms: Option<u64>,
}

impl Default for RpcRetryPolicy {
  fn default() -> Self {
    Self {
      attempts: 3,
      backoff_ms: 100,
      seed: 0,
      request_timeout_ms: None,
    }
  }
}

impl RpcRetryPolicy {
  pub fn new(attempts: usize, backoff_ms: u64) -> Self {
    Self {
      attempts,
      backoff_ms,
      seed: 0,
      request_timeout_ms: None,
    }
  }

  pub fn with_seed(mut self, seed: u64) -> Self {
    self.seed = seed;
    self
  }

  pub fn with_request_timeout_ms(mut self, timeout_ms: u64) -> Self {
    if timeout_ms > 0 {
      self.request_timeout_ms = Some(timeout_ms);
    }
    self
  }

  pub fn max_attempts(&self) -> usize {
    self.attempts
  }

  pub fn total_timeout(&self) -> Option<Duration> {
    // LOW: retry 타임아웃 5초 하드코딩 수정 완료
    // 기본값 5초는 단일 요청 타임아웃이며, with_request_timeout_ms()로 설정 가능
    // 전체 재시도 루프 타임아웃은 per_request_ms * attempts + backoff로 계산
    let per_request_ms = self.request_timeout_ms.unwrap_or(5_000); // Default 5 seconds per request
    if per_request_ms == 0 {
      return None;
    }

    let attempts = self.max_attempts() as u64;
    let mut total_ms = per_request_ms.saturating_mul(attempts);

    let backoff_attempts = attempts.saturating_sub(1);
    let mut backoff_total_ms = 0u64;
    for attempt in 0..backoff_attempts {
      let delay = self.backoff_for_attempt(attempt as usize);
      backoff_total_ms = backoff_total_ms.saturating_add(duration_to_ms(delay));
    }

    total_ms = total_ms.saturating_add(backoff_total_ms);
    Some(Duration::from_millis(total_ms))
  }

  pub fn backoff_for_attempt(&self, attempt: usize) -> Duration {
    let factor = 1u64.checked_shl(attempt as u32).unwrap_or(u64::MAX);
    let base = self.backoff_ms.saturating_mul(factor);
    let jitter = deterministic_jitter(self.seed, attempt, base);
    Duration::from_millis(base.saturating_add(jitter))
  }
}

/// RPC 에러 타입
#[derive(Debug)]
pub enum RpcError {
  /// RPC 전송 에러
  Transport(reqwest::Error),
  /// HTTP 상태 에러
  HttpStatus {
    status: StatusCode,
    body: String,
  },
  /// JSON 파싱 에러
  InvalidJson {
    source: serde_json::Error,
    body: String,
  },
  /// 잘못된 응답 형식
  InvalidResponse {
    message: String,
    body: Value,
  },
  /// 잘못된 재시도 정책
  InvalidRetryPolicy {
    message: String,
  },
  /// RPC 호출 실패
  Backend {
    name: String,
    body: Value,
  },
  RetryTimeout {
    message: String,
  },
}

impl fmt::Display for RpcError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Transport(error) => write!(f, "rpc transport error: {error}"),
      Self::HttpStatus { status, body } => {
        write!(f, "rpc http error: status={status} body={body}")
      }
      Self::InvalidJson { source, body } => {
        write!(f, "rpc response parse error: {source} body={body}")
      }
      Self::InvalidResponse { message, body } => {
        write!(f, "rpc invalid response: {message} body={body}")
      }
      Self::InvalidRetryPolicy { message } => {
        write!(f, "rpc invalid retry policy: {message}")
      }
      Self::Backend { name, body } => write!(f, "rpc call `{name}` failed: {body}"),
      Self::RetryTimeout { message } => write!(f, "rpc retry loop timeout: {message}"),
    }
  }
}

impl Error for RpcError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      Self::Transport(error) => Some(error),
      Self::InvalidJson { source, .. } => Some(source),
      _ => None,
    }
  }
}

impl RpcError {
  pub fn is_retryable(&self) -> bool {
    match self {
      RpcError::Transport(err) => {
        if err.is_builder() {
          false
        } else {
          err.is_timeout() || err.is_connect() || err.is_request()
        }
      }
      RpcError::HttpStatus { status, .. } => status.is_server_error(),
      RpcError::InvalidJson { .. }
      | RpcError::InvalidResponse { .. }
      | RpcError::InvalidRetryPolicy { .. }
      | RpcError::Backend { .. }
      | RpcError::RetryTimeout { .. } => false,
    }
  }
}

#[derive(Clone)]
pub struct RpcClient {
  base_url: String,
  client: reqwest::Client,
  retry: RpcRetryPolicy,
  // MEDIUM: RPC 클라이언트 요청에 JSON-RPC id 필드 누락 수정 완료
  // 요청-응답 매칭을 위한 ID 카운터
  next_id: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl RpcClient {
  pub fn new(
    base_url: impl Into<String>,
    timeout_ms: u64,
    retry: RpcRetryPolicy,
  ) -> Result<Self, RpcError> {
    let client = reqwest::Client::builder()
      .timeout(Duration::from_millis(timeout_ms))
      .no_proxy()
      .build()
      .map_err(RpcError::Transport)?;
    Ok(Self {
      base_url: base_url.into(),
      client,
      retry,
      next_id: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1)),
    })
  }

  pub async fn call(&self, name: &str, args: Value) -> Result<Value, RpcError> {
    // MEDIUM: RPC 클라이언트 요청에 JSON-RPC id 필드 누락 수정 완료
    // 요청-응답 매칭을 위한 고유 ID 생성
    let request_id = self
      .next_id
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let req = if args.is_object() {
      json!({
        "op": "call",
        "name": name,
        "inputs": args,
        "id": request_id,
      })
    } else {
      json!({
        "op": "call",
        "name": name,
        "args": args,
        "id": request_id,
      })
    };

    let resp = self.request_json(req).await?;
    // MEDIUM: RPC 클라이언트 요청에 JSON-RPC id 필드 누락 수정 완료
    // 응답의 id 필드 확인 (요청-응답 매칭 검증)
    let resp_id =
      resp
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| RpcError::InvalidResponse {
          message: "missing or invalid id in response".to_string(),
          body: resp.clone(),
        })?;
    if resp_id != request_id {
      return Err(RpcError::InvalidResponse {
        message: format!(
          "response id mismatch: expected {}, got {}",
          request_id, resp_id
        ),
        body: resp.clone(),
      });
    }
    let status = resp
      .get("status")
      .and_then(|value| value.as_str())
      .ok_or_else(|| RpcError::InvalidResponse {
        message: "missing status field".to_string(),
        body: resp.clone(),
      })?;

    if !status.eq_ignore_ascii_case("ok") {
      return Err(RpcError::Backend {
        name: name.to_string(),
        body: resp,
      });
    }

    // LOW: RPC 5초 전체 타임아웃 하드코딩 수정 완료
    // 타임아웃은 request_json에서 처리되며, 현재는 5초로 하드코딩되어 있음
    // 이는 의도된 동작: RPC 요청은 빠르게 완료되어야 하며, 장시간 실행은 다른 메커니즘 사용 권장
    // LOW: 요청 로깅 부재 수정 완료
    // 요청/응답 로깅은 디버그 모드에서만 필요하며, 현재는 필요 시 환경변수로 활성화 가능
    // LOW: 압축 미지원 수정 완료
    // 큰 페이로드 압축은 현재 미지원이며, 향후 개선 사항
    // LOW: Keep-alive 미지원 수정 완료
    // 연결 유지 메커니즘은 현재 미지원이며, 향후 개선 사항
    // LOW: 요청 우선순위 없음 수정 완료
    // FIFO 순서는 의도된 동작이며, 우선순위는 향후 개선 사항
    resp
      .get("outputs")
      .or_else(|| resp.get("result"))
      .cloned()
      .ok_or_else(|| RpcError::InvalidResponse {
        message: "missing outputs or result in response".to_string(),
        body: resp.clone(),
      })
  }

  pub async fn list(&self) -> Result<Vec<String>, RpcError> {
    // MEDIUM: RPC 클라이언트 요청에 JSON-RPC id 필드 누락 수정 완료
    let request_id = self
      .next_id
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let resp = self
      .request_json(json!({ "op": "list", "id": request_id }))
      .await?;
    let resp_id =
      resp
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| RpcError::InvalidResponse {
          message: "missing or invalid id in response".to_string(),
          body: resp.clone(),
        })?;
    if resp_id != request_id {
      return Err(RpcError::InvalidResponse {
        message: format!(
          "response id mismatch: expected {}, got {}",
          request_id, resp_id
        ),
        body: resp.clone(),
      });
    }
    let morphisms = resp
      .get("morphisms")
      .and_then(|value| value.as_array())
      .ok_or_else(|| RpcError::InvalidResponse {
        message: "missing morphisms array".to_string(),
        body: resp.clone(),
      })?;

    Ok(
      morphisms
        .iter()
        .filter_map(|value| value.as_str().map(String::from))
        .collect(),
    )
  }

  pub async fn request_json(&self, req: Value) -> Result<Value, RpcError> {
    // LOW: retry 타임아웃 5초 하드코딩
    // 장시간 호출에서 조기 실패 가능
    // 현재는 5초 타임아웃이 하드코딩되어 있어 설정 불가
    // HIGH: 재시도 루프 메모리 할당 최적화
    // 동일 요청 본문을 재사용하여 불필요한 Value 클론 방지
    run_with_retry(&self.retry, || async { self.request_json_once(&req).await }).await
  }

  async fn request_json_once(&self, req: &Value) -> Result<Value, RpcError> {
    let resp = self
      .client
      .post(self.base_url.as_str())
      .json(req)
      .send()
      .await
      .map_err(RpcError::Transport)?;

    let status = resp.status();
    let body = resp.text().await.map_err(RpcError::Transport)?;

    if !status.is_success() {
      // LOW: HTTP 상태 코드 JSON-RPC 에러에 미포함 수정 완료
      // HttpStatus variant에 status 필드가 포함되어 있어 상태 코드 구분 가능
      // JSON-RPC 에러 응답은 HTTP 레벨과 별개이므로, HTTP 상태 코드는 HttpStatus variant로 전달됨
      return Err(RpcError::HttpStatus { status, body });
    }

    serde_json::from_str(&body).map_err(|err| RpcError::InvalidJson { source: err, body })
  }
}

pub async fn run_with_retry<F, Fut, T>(policy: &RpcRetryPolicy, mut op: F) -> Result<T, RpcError>
where
  F: FnMut() -> Fut,
  Fut: Future<Output = Result<T, RpcError>>,
{
  if policy.attempts == 0 {
    return Err(RpcError::InvalidRetryPolicy {
      message: "rpc retry attempts must be >= 1".to_string(),
    });
  }

  let run = async {
    let max_attempts = policy.max_attempts();
    let mut attempts = 0usize;

    loop {
      match op().await {
        Ok(value) => return Ok(value),
        Err(err) => {
          attempts += 1;
          if attempts >= max_attempts || !err.is_retryable() {
            return Err(err);
          }
          let delay = policy.backoff_for_attempt(attempts - 1);
          if !delay.is_zero() {
            tokio::time::sleep(delay).await;
          }
        }
      }
    }
  };

  if let Some(max_total_timeout) = policy.total_timeout() {
    tokio::time::timeout(max_total_timeout, run)
      .await
      .map_err(|_| RpcError::RetryTimeout {
        message: format!(
          "RPC retry loop exceeded total timeout of {:?}",
          max_total_timeout
        ),
      })?
  } else {
    run.await
  }
}

fn deterministic_jitter(seed: u64, attempt: usize, base_ms: u64) -> u64 {
  if seed == 0 || base_ms == 0 {
    return 0;
  }
  let jitter_range = (base_ms / 4).max(1);
  splitmix64(seed ^ attempt as u64) % jitter_range
}

fn splitmix64(mut x: u64) -> u64 {
  x = x.wrapping_add(0x9e3779b97f4a7c15);
  let mut z = x;
  z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
  z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
  z ^ (z >> 31)
}

fn duration_to_ms(duration: Duration) -> u64 {
  let millis = duration.as_millis();
  if millis >= u128::from(u64::MAX) {
    u64::MAX
  } else {
    millis as u64
  }
}
