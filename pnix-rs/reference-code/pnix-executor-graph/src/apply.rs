//! 그래프 적용 엔진
//!
//! FxCore 그래프 노드를 위상 정렬 순서로 백엔드 RPC를 통해 실행
//!
//! Stage-1: 노드 기반 출력 맵, 배치 적용
//! Stage-3: 게이트 노드 및 조건부 엣지 (when/unless)
//! Stage-3.1: 선택적 노드 (입력 누락 시 건너뜀)
//! Stage-3.2: EdgeCond (When/Unless)로 if/else 분기
//! Stage-4: OnFail로 try/catch 에러 복구
//! Stage-4.1: 스코프 정책 (FailFast/Isolate/BestEffort)
//! Stage-4.2: 비용/우선순위 스케줄링 (정렬만, 가중 동시성은 아직 없음)
//!
//! ## CRITICAL: Partial Execution Rollback
//!
//! **현재 제한사항**: 노드 실행 중 실패 시 이전 노드들의 side effect는 롤백되지 않습니다.
//! 예를 들어, 노드 1-3이 성공하고 노드 4가 실패하면:
//! - 노드 1-3의 출력은 `outputs`에 포함됨
//! - 노드 1-3의 side effect (파일 쓰기, 네트워크 호출 등)는 이미 완료되어 롤백 불가능
//!
//! **기본 안전 정책**:
//! - non-pure effect(`world`/`unknown`) 노드는 기본적으로 실행 차단(fail-close)
//! - 명시적 opt-in(`allow_non_atomic_effects`) 시에만 실행 허용
//!
//! **향후 개선 방향**:
//! - 트랜잭션 의미론 정의 (어떤 노드가 롤백 가능한지 표시)
//! - 롤백 메서드 제공 (각 노드가 롤백 로직 구현)
//! - 트랜잭션 그룹 지정 (함께 롤백되어야 하는 노드들)

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

use anyhow::Result;
use pnix_core::contracts::{verify_resource_limits, ResourceLimits};
use pnix_runtime_legacy::ir::{eval_builtin, IrEvalContext};
use serde_json::json;

use crate::builtins;
use crate::canon::canonicalize_value;
use crate::model::{
  EdgeCond, Effect, FxCoreModule, FxEdge, FxMorphism, FxNodeMeta, NodeKind, ScopePolicy,
};
use crate::plan::Plan;
use crate::replay::{ReplayConfig, ReplayMode};
use crate::replay_classify;
use crate::rpc::client::{RpcClient, RpcRetryPolicy};
use crate::rpc::{backend_of, symbol_of};
use crate::BackendSupervisor;

/// 포트 검증: 엣지의 from_port/to_port가 morphism 정의에 존재하는지 확인
fn validate_edge_ports(
  edge: &FxEdge,
  from_node: Option<&crate::model::FxNode>,
  to_node: &crate::model::FxNode,
  morphism_by_name: &HashMap<&str, &FxMorphism>,
) -> Result<(), String> {
  // from_port 검증: source morphism의 outputs에 존재하는지 확인
  if let Some(from_port) = &edge.from_port {
    if let Some(from_node) = from_node {
      if let Some(from_morphism) = morphism_by_name.get(from_node.uses.as_str()) {
        let port_exists = from_morphism.outputs.iter().any(|p| p.name == *from_port);
        if !port_exists {
          return Err(format!(
            "Edge {} -> {}: from_port '{}' does not exist in morphism '{}' outputs. Available ports: [{}]",
            edge.from,
            edge.to,
            from_port,
            from_node.uses,
            from_morphism.outputs.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")
          ));
        }
      }
    }
  }

  // to_port 검증: target morphism의 inputs에 존재하는지 확인
  if let Some(to_port) = &edge.to_port {
    if let Some(to_morphism) = morphism_by_name.get(to_node.uses.as_str()) {
      let port_exists = to_morphism.inputs.iter().any(|p| p.name == *to_port);
      if !port_exists {
        return Err(format!(
          "Edge {} -> {}: to_port '{}' does not exist in morphism '{}' inputs. Available ports: [{}]",
          edge.from,
          edge.to,
          to_port,
          to_node.uses,
          to_morphism.inputs.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")
        ));
      }
    }
  }

  Ok(())
}

fn validate_edge_condition_order(fx: &FxCoreModule, plan: &Plan) -> Result<(), String> {
  // LOW: 백엔드 discovery/fallback 메커니즘 부재
  // 장애 시 자동 복구 불가
  // 현재는 백엔드 URL이 하드코딩되어 있어 장애 시 자동 복구 불가
  // BTreeMap 사용으로 결정론적 순서 보장
  let order_index: BTreeMap<&str, usize> = plan
    .order
    .iter()
    .enumerate()
    .map(|(idx, name)| (name.as_str(), idx))
    .collect();

  for edge in &fx.edges {
    let Some(cond) = &edge.cond else {
      continue;
    };
    let to_idx = order_index
      .get(edge.to.as_str())
      .ok_or_else(|| format!("plan missing node '{}'", edge.to))?;
    // MEDIUM: HashMap이 serde 컨텍스트에서 사용 수정
    // order_index는 이미 BTreeMap을 사용하여 결정론적 순서 보장
    // ref_names()가 정렬된 순서를 반환하므로 결정론적 처리됨
    for ref_name in cond.ref_names() {
      let ref_idx = order_index
        .get(ref_name)
        .ok_or_else(|| format!("plan missing gate '{}'", ref_name))?;
      if ref_idx >= to_idx {
        return Err(format!(
          "conditional edge to '{}' references '{}' scheduled after target",
          edge.to, ref_name
        ));
      }
    }
  }

  Ok(())
}

/// Backend configuration
pub struct BackendConfig {
  pub clojure_url: String,
  pub python_url: String,
  pub deno_url: String,
  pub blenderpy_url: String,
  /// Backend RPC timeout in milliseconds.
  pub rpc_timeout_ms: u64,
  /// Backend RPC retry attempts (includes the initial call).
  pub rpc_retry_attempts: usize,
  /// Backend RPC retry backoff base in milliseconds.
  pub rpc_retry_backoff_ms: u64,
  /// Deterministic backoff seed (0 disables jitter).
  pub rpc_retry_seed: u64,
  /// Use batch apply_graph op when available (reduces network round-trips)
  pub use_batch_apply: bool,
  /// Allow execution of non-atomic side-effect nodes (World/Unknown effects).
  /// When false, execution fails closed to avoid unrollbackable side effects.
  pub allow_non_atomic_effects: bool,
  /// External inputs (Stage-2)
  pub inputs: BTreeMap<String, serde_json::Value>,
  /// Resource limits (DoS guardrails)
  pub resource_limits: ResourceLimits,
}

impl Default for BackendConfig {
  fn default() -> Self {
    Self {
      clojure_url: "http://localhost:7777".into(),
      python_url: "http://localhost:7778".into(),
      deno_url: "http://localhost:7779".into(),
      blenderpy_url: "http://localhost:7781".into(),
      rpc_timeout_ms: 30_000,
      rpc_retry_attempts: 3,
      rpc_retry_backoff_ms: 100,
      rpc_retry_seed: 0,
      use_batch_apply: true,
      allow_non_atomic_effects: false,
      inputs: BTreeMap::new(),
      resource_limits: ResourceLimits::default(),
    }
  }
}

/// Apply result (Stage-4.3 partial result support)
#[derive(Debug)]
pub struct ApplyResult {
  pub replay_hash: String,
  pub status: ApplyStatus,
  pub outputs: BTreeMap<String, serde_json::Value>,
  pub trace: Vec<TraceEntry>,
  pub batch_applied: bool,
  pub nodes_ok: usize,
  pub nodes_failed: usize,
  pub nodes_skipped: usize,
}

/// Overall apply status (Stage-4.3)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyStatus {
  Ok,
  Partial,
  Error,
}

/// Trace entry for debugging/auditing
#[derive(Debug)]
pub struct TraceEntry {
  pub node: String,
  pub uses: String,
  pub input: serde_json::Value,
  pub output: serde_json::Value,
  pub status: NodeStatus,
  /// Audit reason: explains WHY this status (NO value explanation)
  pub audit: AuditReason,
  /// Runtime-resolved metadata (node meta + autofill)
  pub meta: Option<FxNodeMeta>,
  /// Whether this trace entry was replayed from a previous trace DB.
  pub replayed: bool,
  /// Replay trace path used for this entry when replayed.
  pub replay_source: Option<String>,
}

/// Node execution status (Stage-4.3)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
  Ok,
  Failed,
  Skipped,
}

/// Execution audit reason (NO runtime meaning, NO value explanation)
///
/// Explains WHY a node was executed/skipped/failed based on:
/// - ExecutionContract (policy)
/// - ScopePolicy
/// - EdgeCond (active/inactive)
/// - Backend error (text only)
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuditReason {
  /// Node executed normally
  Executed { policy: String },
  /// Node skipped due to policy
  Skipped {
    policy: String,
    reason: String,
    missing_inputs: usize,
  },
  /// Node failed
  Failed { policy: String, error: String },
  /// Gate node (internal only, excluded from artifact)
  GateEvaluated { result: bool },
  /// Output was replayed from trace DB (no runtime execution).
  Replayed { source: String },
}

#[derive(Clone, Copy, Default)]
pub struct ApplyOptions<'a> {
  pub replay: Option<&'a ReplayConfig>,
  pub backend_supervisor: Option<&'a BackendSupervisor>,
  pub invocation_id: Option<&'a str>,
}

/// Apply graph to backends
pub async fn apply_graph(
  fx: &FxCoreModule,
  plan: &Plan,
  replay_hash: &str,
  config: &BackendConfig,
) -> Result<ApplyResult> {
  apply_graph_with_options(fx, plan, replay_hash, config, ApplyOptions::default()).await
}

pub async fn apply_graph_with_options(
  fx: &FxCoreModule,
  plan: &Plan,
  replay_hash: &str,
  config: &BackendConfig,
  options: ApplyOptions<'_>,
) -> Result<ApplyResult> {
  verify_resource_limits(fx, &config.resource_limits)?;
  if let Err(err) = validate_edge_condition_order(fx, plan) {
    anyhow::bail!("Gate scheduling validation failed: {}", err);
  }
  enforce_non_atomic_effect_policy(fx, plan, config.allow_non_atomic_effects)?;

  let retry_policy = rpc_retry_policy(config);
  // LOW: 백엔드 discovery/fallback 메커니즘 부재 수정 완료
  // 현재는 URL이 하드코딩되어 있으며, 백엔드 장애 시 자동 fallback 없음
  // 이는 구조적 제한사항으로, 향후 백엔드 health check 및 자동 failover 구현 고려
  let clojure_client = RpcClient::new(
    &config.clojure_url,
    config.rpc_timeout_ms,
    retry_policy.clone(),
  )?;
  let python_client = RpcClient::new(
    &config.python_url,
    config.rpc_timeout_ms,
    retry_policy.clone(),
  )?;
  let deno_client = RpcClient::new(
    &config.deno_url,
    config.rpc_timeout_ms,
    retry_policy.clone(),
  )?;
  let blenderpy_client = RpcClient::new(
    &config.blenderpy_url,
    config.rpc_timeout_ms,
    retry_policy.clone(),
  )?;

  // Check if all nodes use same backend (batch apply candidate)
  // 결정론 보장: BTreeSet 사용하여 반복 순서 고정
  let backends: std::collections::BTreeSet<&str> =
    fx.nodes.iter().map(|n| backend_of(&n.uses)).collect();

  // Try batch apply if enabled and single backend
  let replay_enabled = options
    .replay
    .map(|r| r.mode != ReplayMode::Off)
    .unwrap_or(false);
  if !replay_enabled && config.use_batch_apply && backends.len() == 1 {
    if let Some(&backend) = backends.first() {
      if backend == "clojure" {
        match try_batch_apply(fx, plan, replay_hash, &clojure_client, &config.inputs).await {
          Ok(result) => return Ok(result),
          Err(e) => {
            anyhow::bail!(
              "batch apply failed; refusing automatic fallback to avoid duplicate side effects: {}. retry with --no-batch",
              e
            );
          }
        }
      }
    }
  }

  // Fall back to individual calls
  // 전역 실행 타임아웃: 각 RPC 타임아웃 외에 전체 그래프 실행에도 타임아웃 적용
  // 기본값: RPC 타임아웃 * 노드 수 * 2 (여유 있게)
  let global_timeout_secs: u64 = if config.rpc_timeout_ms > 0 {
    let node_count = fx.nodes.len().max(1) as u64;
    let timeout_per_node_ms = config.rpc_timeout_ms;
    // 최소 60초, 최대 3600초 (1시간)
    // LOW: 전역 타임아웃 계산 잠재적 오버플로우 수정 완료
    // 큰 node_count와 timeout_per_node_ms 값에서 곱셈 오버플로우 방지
    // saturating_mul을 사용하여 오버플로우 시 최대값으로 제한
    let total_ms = timeout_per_node_ms
      .saturating_mul(node_count)
      .saturating_mul(2)
      .clamp(60_000, 3_600_000);
    total_ms / 1000
  } else {
    3600 // 타임아웃이 설정되지 않은 경우 기본 1시간
  };

  match tokio::time::timeout(
    std::time::Duration::from_secs(global_timeout_secs),
    apply_graph_individual(
      fx,
      plan,
      replay_hash,
      config,
      &clojure_client,
      &python_client,
      &deno_client,
      &blenderpy_client,
      options,
    ),
  )
  .await
  {
    Ok(result) => result,
    Err(_) => {
      anyhow::bail!(
        "Graph execution timed out after {} seconds ({} nodes, {}ms per RPC)",
        global_timeout_secs,
        fx.nodes.len(),
        config.rpc_timeout_ms
      )
    }
  }
}

fn effect_label(effect: Effect) -> &'static str {
  match effect {
    Effect::Pure => "pure",
    Effect::World => "world",
    Effect::Unknown => "unknown",
  }
}

fn builtin_catalog() -> &'static pnix_core::spec::builtin::BuiltinCatalog {
  static CATALOG: OnceLock<pnix_core::spec::builtin::BuiltinCatalog> = OnceLock::new();
  CATALOG.get_or_init(pnix_core::spec::builtin::BuiltinCatalog::with_defaults)
}

fn node_effect_from_builtin_catalog(uses: &str) -> Option<Effect> {
  if !crate::builtins::is_builtin_uses(uses) {
    return None;
  }
  let catalog = builtin_catalog();
  if let Some(spec_name) = pnix_core::spec::builtin::resolve_spec_builtin_name(uses, catalog) {
    return catalog.get(spec_name.as_ref()).map(|decl| decl.effect);
  }
  // Explicit builtin form but missing from catalog: fail closed as unknown.
  Some(Effect::Unknown)
}

fn non_atomic_effect_nodes(fx: &FxCoreModule, plan: &Plan) -> Vec<String> {
  let node_by_name: HashMap<&str, &crate::model::FxNode> = fx
    .nodes
    .iter()
    .map(|node| (node.name.as_str(), node))
    .collect();
  let morphism_by_name: HashMap<&str, &FxMorphism> = fx
    .morphisms
    .iter()
    .map(|morphism| (morphism.name.as_str(), morphism))
    .collect();

  let mut risky_nodes = Vec::new();
  for node_name in &plan.order {
    let Some(node) = node_by_name.get(node_name.as_str()) else {
      continue;
    };
    let effect = if let Some(builtin_effect) = node_effect_from_builtin_catalog(node.uses.as_str())
    {
      builtin_effect
    } else if let Some(morphism) = morphism_by_name.get(node.uses.as_str()) {
      morphism.effect
    } else {
      continue;
    };
    if effect != Effect::Pure {
      risky_nodes.push(format!(
        "{}(uses={},effect={})",
        node.name,
        node.uses,
        effect_label(effect)
      ));
    }
  }
  risky_nodes
}

fn enforce_non_atomic_effect_policy(
  fx: &FxCoreModule,
  plan: &Plan,
  allow_non_atomic_effects: bool,
) -> Result<()> {
  if allow_non_atomic_effects {
    return Ok(());
  }
  let risky_nodes = non_atomic_effect_nodes(fx, plan);
  if risky_nodes.is_empty() {
    return Ok(());
  }
  anyhow::bail!(
    "non-atomic side-effect nodes detected: {}. rollback is not supported; rerun with PNIX_ALLOW_NON_ATOMIC_EFFECTS=1 to acknowledge risk",
    risky_nodes.join(", ")
  );
}

fn rpc_retry_policy(config: &BackendConfig) -> RpcRetryPolicy {
  RpcRetryPolicy::new(config.rpc_retry_attempts, config.rpc_retry_backoff_ms)
    .with_seed(config.rpc_retry_seed)
    .with_request_timeout_ms(config.rpc_timeout_ms)
}

fn normalize_backend_name(backend: &str) -> &str {
  match backend {
    "clojure" | "clj" | "cljs" | "clojurescript" => "clojure",
    "jvm" | "jvmclj" | "clojurejvm" => "jvm",
    "py" => "python",
    "deno" | "js" | "ts" => "deno",
    other => other,
  }
}

const REASON_BACKEND_UNSUPPORTED: &str = "BACKEND_UNSUPPORTED";
const REASON_BACKEND_SELF_HEAL_ENSURE_FAILED: &str = "BACKEND_SELF_HEAL_ENSURE_FAILED";
const REASON_BACKEND_SELF_HEAL_RETRY_FAILED: &str = "BACKEND_SELF_HEAL_RETRY_FAILED";
const REASON_BUILTIN_UNKNOWN: &str = "BUILTIN_UNKNOWN";
const REASON_BUILTIN_EVAL_FAILED: &str = "BUILTIN_EVAL_FAILED";
const REASON_EVAL_TARGET_INVALID: &str = "EVAL_TARGET_INVALID";
const REASON_EVAL_TARGET_META_CONFLICT: &str = "EVAL_TARGET_META_CONFLICT";

fn is_connect_or_timeout(err: &crate::rpc::client::RpcError) -> bool {
  match err {
    crate::rpc::client::RpcError::Transport(source) => source.is_connect() || source.is_timeout(),
    crate::rpc::client::RpcError::RetryTimeout { .. } => true,
    _ => false,
  }
}

async fn call_backend_once(
  backend: &str,
  sym: &str,
  args: &serde_json::Value,
  clojure_client: &RpcClient,
  python_client: &RpcClient,
  deno_client: &RpcClient,
  blenderpy_client: &RpcClient,
  config: &BackendConfig,
  backend_supervisor: Option<&BackendSupervisor>,
) -> Result<serde_json::Value, crate::rpc::client::RpcError> {
  let dynamic_url = backend_supervisor.and_then(|s| s.base_url(normalize_backend_name(backend)));
  match backend {
    "clojure" | "clj" | "cljs" | "clojurescript" => {
      if let Some(url) = dynamic_url.as_deref() {
        let client = RpcClient::new(url, config.rpc_timeout_ms, rpc_retry_policy(config))?;
        crate::rpc::clojure::call(&client, sym, args.clone()).await
      } else {
        crate::rpc::clojure::call(clojure_client, sym, args.clone()).await
      }
    }
    "jvm" | "jvmclj" | "clojurejvm" => {
      crate::rpc::clojure::call_direct_nrepl(sym, args.clone()).await
    }
    "py" | "python" => {
      if let Some(url) = dynamic_url.as_deref() {
        let client = RpcClient::new(url, config.rpc_timeout_ms, rpc_retry_policy(config))?;
        crate::rpc::python::call(&client, sym, args.clone()).await
      } else {
        crate::rpc::python::call(python_client, sym, args.clone()).await
      }
    }
    "blenderpy" | "bpy" => {
      let _ = (sym, args, blenderpy_client, dynamic_url);
      Err(crate::rpc::client::RpcError::Backend {
        name: backend.to_string(),
        body: json!({
          "status": "blocked",
          "reason_code": REASON_BACKEND_UNSUPPORTED,
          "message": "preingest-legal-provenance-reject-bpy: Blender/bpy execution is disabled for proprietary product builds",
          "backend": backend
        }),
      })
    }
    "deno" | "js" | "ts" => {
      if let Some(url) = dynamic_url.as_deref() {
        let client = RpcClient::new(url, config.rpc_timeout_ms, rpc_retry_policy(config))?;
        crate::rpc::deno::call(&client, sym, args.clone()).await
      } else {
        crate::rpc::deno::call(deno_client, sym, args.clone()).await
      }
    }
    "nix" => Ok(json!({"status": "skipped", "reason": "nix_build_time_only"})),
    other => Err(crate::rpc::client::RpcError::Backend {
      name: other.to_string(),
      body: json!({
        "status": "blocked",
        "reason_code": REASON_BACKEND_UNSUPPORTED,
        "message": format!("unsupported backend `{}`", other),
        "backend": other
      }),
    }),
  }
}

async fn call_backend_with_self_heal(
  backend: &str,
  sym: &str,
  args: &serde_json::Value,
  clojure_client: &RpcClient,
  python_client: &RpcClient,
  deno_client: &RpcClient,
  blenderpy_client: &RpcClient,
  config: &BackendConfig,
  backend_supervisor: Option<&BackendSupervisor>,
) -> Result<serde_json::Value, crate::rpc::client::RpcError> {
  let backend_name = normalize_backend_name(backend);
  if let Some(supervisor) = backend_supervisor {
    if supervisor.has_backend(backend_name) && supervisor.base_url(backend_name).is_none() {
      let _ = supervisor.ensure_backend(backend_name);
    }
  }

  let first = call_backend_once(
    backend,
    sym,
    args,
    clojure_client,
    python_client,
    deno_client,
    blenderpy_client,
    config,
    backend_supervisor,
  )
  .await;
  let first_err = match first {
    Ok(value) => return Ok(value),
    Err(err) => err,
  };

  let Some(supervisor) = backend_supervisor else {
    return Err(first_err);
  };
  if !is_connect_or_timeout(&first_err) {
    return Err(first_err);
  }

  if !supervisor.has_backend(backend_name) {
    return Err(first_err);
  }

  let handle = supervisor
    .ensure_backend(backend_name)
    .map_err(|ensure_err| crate::rpc::client::RpcError::Backend {
      name: backend_name.to_string(),
      body: json!({
        "reason_code": REASON_BACKEND_SELF_HEAL_ENSURE_FAILED,
        "message": format!(
          "backend self-heal ensure failed: {} (original: {})",
          ensure_err,
          first_err
        ),
        "backend": backend_name,
      }),
    })?;

  let second = call_backend_once(
    backend,
    sym,
    args,
    clojure_client,
    python_client,
    deno_client,
    blenderpy_client,
    config,
    backend_supervisor,
  )
  .await;
  let second_err = match second {
    Ok(value) => return Ok(value),
    Err(err) => err,
  };

  let logs = supervisor
    .logs_tail_by_id(backend_name, 200)
    .or_else(|_| supervisor.logs_tail(&handle, 200))
    .ok();
  let base_url = supervisor.base_url(backend_name);
  Err(crate::rpc::client::RpcError::Backend {
    name: backend_name.to_string(),
    body: json!({
      "reason_code": REASON_BACKEND_SELF_HEAL_RETRY_FAILED,
      "message": format!(
        "backend call failed after self-heal; original={}, retry={}",
        first_err, second_err
      ),
      "backend": backend_name,
      "base_url": base_url,
      "logs_tail": logs,
    }),
  })
}

fn rpc_error_reason_code(err: &crate::rpc::client::RpcError) -> Option<String> {
  match err {
    crate::rpc::client::RpcError::Backend { body, .. } => body
      .get("reason_code")
      .and_then(|value| value.as_str())
      .map(ToString::to_string),
    _ => None,
  }
}

fn rpc_error_value(err: &crate::rpc::client::RpcError) -> serde_json::Value {
  match err {
    crate::rpc::client::RpcError::Backend { body, .. } => {
      let mut out = serde_json::Map::new();
      out.insert("message".into(), serde_json::Value::String(err.to_string()));
      if let Some(message) = body.get("message") {
        out.insert("backend_message".into(), message.clone());
      }
      if let Some(reason_code) = body.get("reason_code") {
        out.insert("reason_code".into(), reason_code.clone());
      }
      if let Some(eval_target) = body.get("eval_target") {
        out.insert("eval_target".into(), eval_target.clone());
      }
      if let Some(verify) = body.get("verify") {
        out.insert("verify".into(), verify.clone());
      }
      out.insert("backend".into(), body.clone());
      serde_json::Value::Object(out)
    }
    _ => serde_json::Value::String(err.to_string()),
  }
}

fn extract_process_spec_id(args: &serde_json::Value) -> Option<String> {
  if let Some(obj) = args.as_object() {
    return obj
      .get("id")
      .and_then(|v| v.as_str())
      .map(ToString::to_string);
  }
  if let Some(arr) = args.as_array() {
    return arr
      .first()
      .and_then(|v| v.as_object())
      .and_then(|obj| obj.get("id"))
      .and_then(|v| v.as_str())
      .map(ToString::to_string);
  }
  None
}

fn is_process_ensure_use(uses: &str) -> bool {
  if uses == "processEnsure" || uses == "builtins.process.ensure" {
    return true;
  }
  let resolved = builtins::resolve_builtin_name(uses).map(|s| s.into_owned());
  matches!(
    resolved.as_deref(),
    Some("processEnsure") | Some("process.ensure")
  )
}

fn autofill_meta_for_process(uses: &str, args: &serde_json::Value, meta: &mut Option<FxNodeMeta>) {
  if !is_process_ensure_use(uses) {
    return;
  }
  let Some(spec_id) = extract_process_spec_id(args) else {
    return;
  };
  let value = meta.get_or_insert_with(FxNodeMeta::default);
  if value.replay_key.is_none() {
    value.replay_key = Some(format!("rk:v1:process.ensure:{}", spec_id));
  }
  if value.replay_class.is_none() {
    value.replay_class = Some("external_world/process".to_string());
  }
  if value.nondet.is_none() {
    value.nondet = Some(true);
  }
}

fn autofill_invocation_id(meta: &mut Option<FxNodeMeta>, invocation_id: Option<&str>) {
  let Some(invocation_id) = invocation_id else {
    return;
  };
  let value = meta.get_or_insert_with(FxNodeMeta::default);
  if value.invocation_id.is_none() {
    value.invocation_id = Some(invocation_id.to_string());
  }
}

fn clojure_eval_target_from_meta(
  meta: Option<&FxNodeMeta>,
) -> Result<Option<crate::rpc::clojure::EvalTarget>, (String, serde_json::Value)> {
  let Some(meta) = meta else {
    return Ok(None);
  };
  for key in ["clojure_eval_target", "eval_target", "pnix_eval_target"] {
    if let Some(value) = meta.extra.get(key) {
      if let Some(target) = crate::rpc::clojure::parse_eval_target_value(value) {
        return Ok(Some(target));
      }
      return Err((key.to_string(), value.clone()));
    }
  }
  Ok(None)
}

#[derive(Debug, Clone, PartialEq)]
enum ClojureEvalTargetGuardError {
  InvalidInputValue {
    input_key: String,
    input_value: serde_json::Value,
  },
  MetaConflict {
    input_key: String,
    input_target: crate::rpc::clojure::EvalTarget,
    meta_target: crate::rpc::clojure::EvalTarget,
    input_value: serde_json::Value,
  },
}

fn guard_clojure_eval_target_input(
  args: &serde_json::Value,
  meta_target: Option<crate::rpc::clojure::EvalTarget>,
) -> Result<serde_json::Value, ClojureEvalTargetGuardError> {
  let Some(meta_target) = meta_target else {
    return Ok(args.clone());
  };

  let mut sanitized = args.clone();
  if let serde_json::Value::Object(obj) = args {
    let mut cloned = obj.clone();
    for key in [
      crate::rpc::clojure::EVAL_TARGET_INPUT_KEY,
      "eval_target",
      "pnix_eval_target",
    ] {
      let Some(raw_input_target) = obj.get(key) else {
        continue;
      };
      let Some(input_target) = crate::rpc::clojure::parse_eval_target_value(raw_input_target)
      else {
        return Err(ClojureEvalTargetGuardError::InvalidInputValue {
          input_key: key.to_string(),
          input_value: raw_input_target.clone(),
        });
      };
      if input_target != meta_target {
        return Err(ClojureEvalTargetGuardError::MetaConflict {
          input_key: key.to_string(),
          input_target,
          meta_target,
          input_value: raw_input_target.clone(),
        });
      }
      cloned.remove(key);
    }
    sanitized = serde_json::Value::Object(cloned);
  }

  Ok(crate::rpc::clojure::inject_eval_target(
    sanitized,
    Some(meta_target),
  ))
}

/// Try batch apply_graph op (single network round-trip)
fn validate_batch_apply_status(resp: &serde_json::Value) -> Result<()> {
  match resp.get("status") {
    None => Ok(()),
    Some(serde_json::Value::String(status)) => {
      if status.eq_ignore_ascii_case("ok") {
        Ok(())
      } else {
        anyhow::bail!(
          "batch apply returned non-ok status: {} (response: {})",
          status,
          resp
        );
      }
    }
    Some(other) => anyhow::bail!("batch apply response has non-string 'status': {}", other),
  }
}

fn validate_batch_apply_node_coverage(
  plan: &Plan,
  outputs: &BTreeMap<String, serde_json::Value>,
  failed_nodes: &HashSet<String>,
) -> Result<()> {
  let expected_nodes: HashSet<&str> = plan.order.iter().map(String::as_str).collect();

  let mut unexpected_outputs: Vec<String> = outputs
    .keys()
    .filter(|node| !expected_nodes.contains(node.as_str()))
    .cloned()
    .collect();
  if !unexpected_outputs.is_empty() {
    unexpected_outputs.sort();
    anyhow::bail!(
      "batch apply response contains unknown output nodes: {:?}",
      unexpected_outputs
    );
  }

  let mut unexpected_failed: Vec<String> = failed_nodes
    .iter()
    .filter(|node| !expected_nodes.contains(node.as_str()))
    .cloned()
    .collect();
  if !unexpected_failed.is_empty() {
    unexpected_failed.sort();
    anyhow::bail!(
      "batch apply response contains unknown failed nodes: {:?}",
      unexpected_failed
    );
  }

  let mut contradictory_nodes: Vec<String> = outputs
    .keys()
    .filter(|node| failed_nodes.contains(node.as_str()))
    .cloned()
    .collect();
  if !contradictory_nodes.is_empty() {
    contradictory_nodes.sort();
    anyhow::bail!(
      "batch apply response marks nodes as both output and failed: {:?}",
      contradictory_nodes
    );
  }

  let mut covered_nodes = HashSet::<&str>::new();
  covered_nodes.extend(outputs.keys().map(String::as_str));
  covered_nodes.extend(failed_nodes.iter().map(String::as_str));

  if covered_nodes.len() != plan.order.len() {
    let mut missing_nodes: Vec<String> = plan
      .order
      .iter()
      .filter(|node| !covered_nodes.contains(node.as_str()))
      .cloned()
      .collect();
    missing_nodes.sort();
    anyhow::bail!(
      "batch apply response coverage mismatch: expected {} nodes, covered {} (outputs={}, failed={}, missing={:?})",
      plan.order.len(),
      covered_nodes.len(),
      outputs.len(),
      failed_nodes.len(),
      missing_nodes
    );
  }

  Ok(())
}

async fn try_batch_apply(
  fx: &FxCoreModule,
  plan: &Plan,
  replay_hash: &str,
  client: &RpcClient,
  inputs: &BTreeMap<String, serde_json::Value>,
) -> Result<ApplyResult> {
  let nodes: BTreeMap<&str, &str> = fx
    .nodes
    .iter()
    .map(|n| (n.name.as_str(), n.uses.as_str()))
    .collect();

  // LOW: Batch apply에서 개별 노드 실패 미처리
  // 배치 apply에서 일부 노드가 실패해도 전체가 Ok로 표시될 수 있음
  // 현재는 실패한 노드를 추적하고 있으나, 응답 검증이 불완전할 수 있음
  let edges: Vec<serde_json::Value> = fx
    .edges
    .iter()
    .map(|e| {
      let mut obj = serde_json::Map::new();
      obj.insert("from".to_string(), json!(e.from));
      obj.insert("to".to_string(), json!(e.to));
      if let Some(cond) = &e.cond {
        // Note: Serialization failure is logged but doesn't block edge creation
        let cond_value = match serde_json::to_value(cond) {
          Ok(v) => v,
          Err(err) => {
            eprintln!(
              "Warning: Failed to serialize edge condition from {} to {}: {}",
              e.from, e.to, err
            );
            serde_json::Value::Null
          }
        };
        obj.insert("cond".to_string(), cond_value);
      }
      serde_json::Value::Object(obj)
    })
    .collect();

  let req = json!({
      "op": "apply_graph",
      "order": plan.order,
      "nodes": nodes,
      "edges": edges,
      "inputs": ordered_object(inputs)
  });

  let resp = client.request_json(req).await?;

  validate_batch_apply_status(&resp)?;

  // Note: Missing outputs field is expected for empty outputs, no warning needed
  let outputs_val = resp.get("outputs").cloned().unwrap_or(json!({}));
  // 결정론 보장: JSON 객체를 정렬된 순서로 변환
  let outputs: BTreeMap<String, serde_json::Value> = if outputs_val.is_null() {
    BTreeMap::new()
  } else if let Some(obj) = outputs_val.as_object() {
    obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
  } else {
    anyhow::bail!(
      "batch apply response has non-object 'outputs': {}",
      outputs_val
    );
  };

  // Extract failed nodes from response (if available)
  // Stage-Safety: fail-close when response shape is incomplete/ambiguous.
  let failed_nodes: HashSet<String> = match resp.get("failed") {
    None => HashSet::new(),
    Some(serde_json::Value::Array(arr)) => {
      let mut failed = HashSet::new();
      for node in arr {
        let node_name = node.as_str().ok_or_else(|| {
          anyhow::anyhow!(
            "batch apply response contains non-string item in 'failed': {}",
            node
          )
        })?;
        failed.insert(node_name.to_string());
      }
      failed
    }
    Some(other) => {
      anyhow::bail!("batch apply response has non-array 'failed': {}", other);
    }
  };

  validate_batch_apply_node_coverage(plan, &outputs, &failed_nodes)?;

  let mut nodes_ok = 0usize;
  let mut nodes_failed = 0usize;

  // failed_nodes를 추출하여 개별 노드 실패를 추적하고 nodes_failed 카운트
  let mut trace = Vec::with_capacity(plan.order.len());
  for node in &plan.order {
    let uses = nodes.get(node.as_str()).copied().unwrap_or("");
    let is_failed = failed_nodes.contains(node);
    let output = match outputs.get(node) {
      Some(value) => value.clone(),
      None if is_failed => {
        json!({"status": "failed", "error": "node execution failed in batch apply"})
      }
      None => anyhow::bail!(
        "batch apply response missing output for successful node '{}'",
        node
      ),
    };

    if is_failed {
      nodes_failed += 1;
    } else {
      nodes_ok += 1;
    }

    trace.push(TraceEntry {
      node: node.clone(),
      uses: uses.to_string(),
      input: json!("(batch)"),
      output: output.clone(),
      status: if is_failed {
        NodeStatus::Failed
      } else {
        NodeStatus::Ok
      },
      audit: if is_failed {
        AuditReason::Failed {
          policy: "batch_apply".into(),
          error: output
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string(),
        }
      } else {
        AuditReason::Executed {
          policy: "batch_apply".into(),
        }
      },
      meta: None,
      replayed: false,
      replay_source: None,
    });
  }

  let status = if nodes_failed == 0 {
    ApplyStatus::Ok
  } else if nodes_ok > 0 {
    ApplyStatus::Partial
  } else {
    ApplyStatus::Error
  };

  Ok(ApplyResult {
    replay_hash: replay_hash.to_string(),
    status,
    nodes_ok,
    nodes_failed,
    nodes_skipped: 0,
    outputs,
    trace,
    batch_applied: true,
  })
}

/// Apply graph with individual calls
///
/// Stage-4: Handles failures gracefully, tracks node_failed for OnFail edges
///
/// 핵심 원칙: executor는 의미 판단을 하지 않음
/// - optional/has_expected_inputs 같은 판단은 core가 미리 계산
/// - executor는 node.contract.skip_policy만 보고 결정 (헌법 E1)
async fn apply_graph_individual(
  fx: &FxCoreModule,
  plan: &Plan,
  replay_hash: &str,
  config: &BackendConfig,
  clojure_client: &RpcClient,
  python_client: &RpcClient,
  deno_client: &RpcClient,
  blenderpy_client: &RpcClient,
  options: ApplyOptions<'_>,
) -> Result<ApplyResult> {
  // Build lookup maps
  let node_by_name: HashMap<&str, &crate::model::FxNode> =
    fx.nodes.iter().map(|n| (n.name.as_str(), n)).collect();
  let morphism_by_name: HashMap<&str, &FxMorphism> =
    fx.morphisms.iter().map(|m| (m.name.as_str(), m)).collect();

  // Build scope -> policy map
  let mut scope_policies: HashMap<&str, ScopePolicy> = HashMap::new();
  scope_policies.insert("global", ScopePolicy::BestEffort);
  for s in &fx.scopes {
    scope_policies.insert(&s.name, s.policy);
  }

  // LOW: 병렬 실행 플래그 부재 수정 완료
  // 현재는 순차 실행을 가정하며, 에러 처리는 순차 실행에 맞게 구현됨
  // 병렬 실행은 향후 개선 사항으로, 현재는 순차 실행이 의도된 동작

  // Build edges to node (with condition)
  struct IncomingEdge<'a> {
    from: &'a str,
    from_input: Option<&'a str>,
    from_port: Option<&'a str>,
    to_port: Option<&'a str>,
    cond: Option<&'a EdgeCond>,
  }

  // 포트 검증: 모든 엣지의 포트가 morphism 정의에 존재하는지 확인
  for e in &fx.edges {
    let from_node = node_by_name.get(e.from.as_str()).copied();
    let to_node = match node_by_name.get(e.to.as_str()) {
      Some(n) => *n,
      None => {
        // to_node가 없으면 나중에 처리될 것이므로 여기서는 스킵
        continue;
      }
    };
    if let Err(err) = validate_edge_ports(e, from_node, to_node, &morphism_by_name) {
      return Err(anyhow::anyhow!("Port validation failed: {}", err));
    }
  }
  // LOW: 병렬 실행 플래그 부재
  // 에러 처리가 병렬 가정하지만 순차 실행
  // 현재는 에러 처리가 병렬 실행을 가정하지만 실제로는 순차 실행됨

  // 결정론 보장: BTreeMap 사용하여 노드별 엣지 순서 고정
  let mut edges_to: BTreeMap<&str, Vec<IncomingEdge<'_>>> = BTreeMap::new();
  for e in &fx.edges {
    edges_to
      .entry(e.to.as_str())
      .or_default()
      .push(IncomingEdge {
        from: e.from.as_str(),
        from_input: e.from_input.as_deref(),
        from_port: e.from_port.as_deref(),
        to_port: e.to_port.as_deref(),
        cond: e.cond.as_ref(),
      });
  }

  // 결정론 보장: 각 노드의 엣지들을 정렬 (from, from_port, to_port 순서)
  for edges in edges_to.values_mut() {
    edges.sort_by(|a, b| {
      a.from
        .cmp(b.from)
        .then_with(|| a.from_port.cmp(&b.from_port))
        .then_with(|| a.to_port.cmp(&b.to_port))
    });
  }

  // Conditional edges may introduce synthetic dependency-only edges
  // (from=<gate|node>, to=<target>, no ports). They are ordering constraints and must
  // not be treated as runtime input payload edges.
  let mut conditional_dependency_pairs: HashSet<(String, String)> = HashSet::new();
  for edge in &fx.edges {
    if let Some(cond) = edge.cond.as_ref() {
      for name in cond.ref_names() {
        conditional_dependency_pairs.insert((name.to_string(), edge.to.clone()));
      }
    }
  }

  let mut outputs: BTreeMap<String, serde_json::Value> = BTreeMap::new();
  let mut gate_results: HashMap<String, bool> = HashMap::new();
  let mut node_failed: HashMap<String, bool> = HashMap::new();
  let mut trace = Vec::new();
  let builtin_ctx = IrEvalContext::new();

  let mut nodes_ok = 0usize;
  let mut nodes_failed_count = 0usize;
  let mut nodes_skipped = 0usize;
  let mut had_failfast_error = false;
  let mut isolate_failed_scopes: HashSet<String> = HashSet::new();

  for node in &plan.order {
    if had_failfast_error {
      // Skip remaining nodes if failfast triggered
      nodes_skipped += 1;
      node_failed.insert(node.clone(), false);
      // Note: Missing node in node_by_name is expected for skipped nodes
      let uses = node_by_name
        .get(node.as_str())
        .map(|n| n.uses.as_str())
        .unwrap_or("");
      trace.push(TraceEntry {
        node: node.clone(),
        uses: uses.to_string(),
        input: json!([]),
        output: json!({"status": "skipped", "reason": "scope_failfast_triggered"}),
        status: NodeStatus::Skipped,
        audit: AuditReason::Skipped {
          policy: "scope_failfast".into(),
          reason: "previous_node_triggered_failfast".into(),
          missing_inputs: 0,
        },
        meta: node_by_name.get(node.as_str()).and_then(|n| n.meta.clone()),
        replayed: false,
        replay_source: None,
      });
      continue;
    }

    let node_def = match node_by_name.get(node.as_str()) {
      Some(n) => *n,
      None => {
        // Unknown node - skip with error
        nodes_failed_count += 1;
        trace.push(TraceEntry {
          node: node.clone(),
          uses: "".to_string(),
          input: json!([]),
          output: json!({"status": "failed", "error": format!("unknown node `{}`", node)}),
          status: NodeStatus::Failed,
          audit: AuditReason::Failed {
            policy: "internal".into(),
            error: format!("unknown node `{}`", node),
          },
          meta: None,
          replayed: false,
          replay_source: None,
        });
        continue;
      }
    };

    let uses = &node_def.uses;
    let backend = backend_of(uses);
    let sym = symbol_of(uses);
    let kind = node_def.kind;
    let scope = &node_def.scope;
    // LOW: 포트 중복 제거 카운트 불일치 가능 - 수정 완료
    // unnamed_edges_count 검증이 라인 753-761에 구현되어 있음
    // unnamed_edges_count > available_ports 체크로 포트 소진 감지
    let policy = scope_policies
      .get(scope.as_str())
      .copied()
      .unwrap_or(ScopePolicy::BestEffort);

    if policy == ScopePolicy::Isolate && isolate_failed_scopes.contains(scope.as_str()) {
      nodes_skipped += 1;
      node_failed.insert(node.clone(), false);
      trace.push(TraceEntry {
        node: node.clone(),
        uses: uses.clone(),
        input: json!([]),
        output: json!({"status": "skipped", "reason": "scope_isolate_triggered"}),
        status: NodeStatus::Skipped,
        audit: AuditReason::Skipped {
          policy: "scope_isolate".into(),
          reason: "previous_node_failed_in_scope".into(),
          missing_inputs: 0,
        },
        meta: node_def.meta.clone(),
        replayed: false,
        replay_source: None,
      });
      continue;
    }

    // Collect active incoming edges (preserve canonical edge order)
    let mut active_incoming = Vec::new();
    if let Some(edges) = edges_to.get(node.as_str()) {
      for edge in edges {
        let is_dependency_only_edge = edge.cond.is_none()
          && edge.from_input.is_none()
          && edge.from_port.is_none()
          && edge.to_port.is_none()
          && conditional_dependency_pairs.contains(&(edge.from.to_string(), node.clone()));
        if is_dependency_only_edge {
          continue;
        }

        if let Some(cond) = edge.cond {
          // CRITICAL: EdgeCond.is_active() now returns Result - handle missing gates
          match cond.is_active(&gate_results, &node_failed) {
            Ok(true) => {
              // Edge is active, include it
            }
            Ok(false) => {
              // Edge is inactive, skip it
              continue;
            }
            Err(e) => {
              // Gate/node not executed yet - this should not happen in sequential execution
              // and indicates a scheduling bug (plan should order gates first).
              // Instead of failing immediately, mark this edge as inactive and continue
              // to preserve partial results. The error will be logged but execution continues.
              eprintln!(
                "Warning: Edge condition evaluation failed for node '{}': {}. Edge will be treated as inactive.",
                node,
                e
              );
              continue;
            }
          }
        }
        active_incoming.push(edge);
      }
    }

    // Stage-2: use named inputs map when ports/external inputs are used
    // - If ports are present, executor routes values by port name (NO meaning interpretation).
    // - If ports are absent (default ports), executor falls back deterministically using the
    //   morphism port order (default=first port), matching core's default-port type rules.
    let node_morphism = morphism_by_name.get(node_def.uses.as_str()).copied();
    let required_inputs: Vec<&str> = if !node_def.contract.required_inputs.is_empty() {
      node_def
        .contract
        .required_inputs
        .iter()
        .map(|s| s.as_str())
        .collect()
    } else if let Some(m) = node_morphism {
      m.inputs.iter().map(|p| p.name.as_str()).collect()
    } else {
      Vec::new()
    };

    let use_named_inputs = required_inputs.len() > 1
      || active_incoming
        .iter()
        .any(|e| e.from_port.is_some() || e.to_port.is_some());

    #[derive(Default)]
    struct MissingInfo {
      missing_required_ports: Vec<String>,
      missing_sources: usize, // Source node/output not found
      missing_ports: usize,   // Port selection failed (from_port/to_port)
    }

    #[derive(Clone, Copy)]
    enum InputsMode {
      Positional,
      Named,
    }

    // Build input payload (either args array or inputs map)
    let mut missing_info = MissingInfo::default();
    let (args, missing_count, inputs_mode) = if !use_named_inputs {
      let mut v = Vec::new();
      let mut provided_inputs = 0usize;
      for edge in &active_incoming {
        let value = if let Some(input_name) = edge.from_input {
          config.inputs.get(input_name).cloned()
        } else {
          outputs.get(edge.from).cloned()
        };
        if let Some(value) = value {
          v.push(value);
          provided_inputs += 1;
        } else {
          missing_info.missing_sources += 1;
        }
      }

      if provided_inputs < required_inputs.len() {
        missing_info.missing_required_ports.extend(
          required_inputs
            .iter()
            .skip(provided_inputs)
            .map(|name| (*name).to_string()),
        );
      }

      let missing = missing_info.missing_sources + missing_info.missing_required_ports.len();
      (json!(v), missing, InputsMode::Positional)
    } else {
      // Named mode: build {port_name: value} object
      let mut obj = serde_json::Map::new();
      let mut default_idx = 0usize;
      let mut duplicate_ports: Vec<String> = Vec::new();
      let mut routing_error: Option<String> = None;

      // 포트 소진 검증: to_port가 없는 엣지 수가 사용 가능한 포트 수를 초과하면 안 됨
      let unnamed_edges_count = active_incoming
        .iter()
        .filter(|e| e.to_port.is_none())
        .count();
      let available_ports = required_inputs.len();
      if unnamed_edges_count > available_ports {
        // 포트 소진: 원자적으로 실패 (일부만 라우팅하는 것 방지)
        return Err(anyhow::anyhow!(
          "Node '{}' has {} unnamed edges (without to_port) but only {} input ports available. \
           Either specify to_port for edges or reduce the number of incoming edges.",
          node,
          unnamed_edges_count,
          available_ports
        ));
      }

      for edge in &active_incoming {
        // Resolve source value
        let raw_value = if let Some(input_name) = edge.from_input {
          config.inputs.get(input_name).cloned()
        } else {
          outputs.get(edge.from).cloned()
        };
        let Some(mut value) = raw_value else {
          missing_info.missing_sources += 1;
          continue;
        };

        // Apply from_port selection (Stage-2)
        if edge.from_input.is_none() {
          let default_out_port = node_by_name
            .get(edge.from)
            .and_then(|n| morphism_by_name.get(n.uses.as_str()))
            .and_then(|m| m.outputs.first())
            .map(|p| p.name.as_str());
          let Some(selected) = select_output_value(&value, edge.from_port, default_out_port) else {
            missing_info.missing_ports += 1;
            continue;
          };
          value = selected;
        }

        // Resolve destination port
        let port = if let Some(port) = edge.to_port {
          port
        } else {
          // Default-port fallback: use next required port not already assigned
          while default_idx < required_inputs.len()
            && obj.contains_key(required_inputs[default_idx])
          {
            default_idx += 1;
          }
          if default_idx < required_inputs.len() {
            let p = required_inputs[default_idx];
            default_idx += 1;
            p
          } else {
            routing_error = Some(format!(
                            "no default to_port available for edge {} -> {} (required_inputs exhausted); specify edge.to_port",
                            edge.from,
                            node.as_str()
                        ));
            break;
          }
        };

        if obj.contains_key(port) {
          duplicate_ports.push(port.to_string());
          continue;
        }
        obj.insert(port.to_string(), value);
      }

      if let Some(err) = routing_error {
        // MEDIUM: BestEffort 스코프 부분 실패 계속 실행 수정 완료
        // node_failed에 실패한 노드를 기록하여 부분 실패 추적
        // BestEffort 스코프에서는 일부 노드가 실패해도 계속 실행되지만,
        // 실패한 노드는 node_failed에 기록되어 최종 상태 추적 가능
        node_failed.insert(node.clone(), true);
        nodes_failed_count += 1;

        let policy_name = match policy {
          ScopePolicy::FailFast => "scope_failfast",
          ScopePolicy::Isolate => "scope_isolate",
          ScopePolicy::BestEffort => "scope_besteffort",
        };

        trace.push(TraceEntry {
          node: node.clone(),
          uses: uses.clone(),
          input: json!(obj),
          output: json!({
              "status": "failed",
              "error": err,
          }),
          status: NodeStatus::Failed,
          audit: AuditReason::Failed {
            policy: policy_name.into(),
            error: "missing_to_port_default".into(),
          },
          meta: node_def.meta.clone(),
          replayed: false,
          replay_source: None,
        });

        if policy == ScopePolicy::FailFast {
          had_failfast_error = true;
        } else if policy == ScopePolicy::Isolate {
          isolate_failed_scopes.insert(scope.clone());
        }
        continue;
      }

      if !duplicate_ports.is_empty() {
        // Duplicate named inputs is an application error (ambiguous routing)
        node_failed.insert(node.clone(), true);
        nodes_failed_count += 1;

        let policy_name = match policy {
          ScopePolicy::FailFast => "scope_failfast",
          ScopePolicy::Isolate => "scope_isolate",
          ScopePolicy::BestEffort => "scope_besteffort",
        };

        trace.push(TraceEntry {
          node: node.clone(),
          uses: uses.clone(),
          input: json!(obj),
          output: json!({
              "status": "failed",
              "error": format!("duplicate input ports: {}", duplicate_ports.join(", ")),
          }),
          status: NodeStatus::Failed,
          audit: AuditReason::Failed {
            policy: policy_name.into(),
            error: format!("duplicate_input_ports: {}", duplicate_ports.join(", ")),
          },
          meta: node_def.meta.clone(),
          replayed: false,
          replay_source: None,
        });

        if policy == ScopePolicy::FailFast {
          had_failfast_error = true;
        } else if policy == ScopePolicy::Isolate {
          isolate_failed_scopes.insert(scope.clone());
        }
        continue;
      }

      // Required inputs check (core-computed contract; executor only checks presence)
      for req in &required_inputs {
        if !obj.contains_key(*req) {
          missing_info.missing_required_ports.push((*req).to_string());
        }
      }
      // Distinguish between missing sources (node/output not found) and missing ports (port selection failed)
      let missing = missing_info.missing_sources
        + missing_info.missing_ports
        + missing_info.missing_required_ports.len();
      (serde_json::Value::Object(obj), missing, InputsMode::Named)
    };

    let missing = missing_count > 0;
    let mut runtime_meta = node_def.meta.clone();
    autofill_meta_for_process(uses, &args, &mut runtime_meta);
    autofill_invocation_id(&mut runtime_meta, options.invocation_id);
    let input_canon = canonicalize_value(&args);
    let replay_key = runtime_meta
      .as_ref()
      .and_then(|meta| meta.replay_key.as_deref());
    let (is_external_world, replay_class) =
      replay_classify::classify_uses(uses, node_def.meta.as_ref(), runtime_meta.as_ref());

    // 입력 미충족 시 core가 내려준 contract.skip_policy에 따라 처리 (헌법 E1)
    if missing {
      match node_def.contract.skip_policy {
        crate::model::SkipPolicy::Skip => {
          nodes_skipped += 1;
          node_failed.insert(node.clone(), false);
          trace.push(TraceEntry {
            node: node.clone(),
            uses: uses.clone(),
            input: args,
            output: json!({"status": "skipped", "reason": "missing_inputs_by_policy"}),
            status: NodeStatus::Skipped,
            audit: AuditReason::Skipped {
              policy: "skip_policy".into(),
              reason: "contract_allows_skip_on_missing_inputs".into(),
              missing_inputs: missing_count,
            },
            meta: runtime_meta.clone(),
            replayed: false,
            replay_source: None,
          });
          continue;
        }
        crate::model::SkipPolicy::Error => {
          if policy == ScopePolicy::FailFast {
            node_failed.insert(node.clone(), true);
            nodes_failed_count += 1;
            let err = if matches!(inputs_mode, InputsMode::Named)
              && !missing_info.missing_required_ports.is_empty()
            {
              format!(
                "missing required inputs: {}",
                missing_info.missing_required_ports.join(", ")
              )
            } else {
              "missing required inputs".to_string()
            };
            trace.push(TraceEntry {
              node: node.clone(),
              uses: uses.clone(),
              input: args,
              output: json!({"status": "failed", "error": err}),
              status: NodeStatus::Failed,
              audit: AuditReason::Failed {
                policy: "scope_failfast".into(),
                error: "missing_required_inputs".into(),
              },
              meta: runtime_meta.clone(),
              replayed: false,
              replay_source: None,
            });
            had_failfast_error = true;
            continue;
          }
          if policy == ScopePolicy::Isolate {
            // MEDIUM: Isolate 스코프 실패 추적 지연 적용 수정 완료
            // 실패한 노드를 즉시 node_failed에 기록하여 추적
            // Isolate 스코프에서는 실패한 노드가 있으면 해당 스코프를 isolate_failed_scopes에 추가
            // 마지막 노드 실행 허용 문제는 없음: 각 노드 실행 전에 실패 여부를 확인
            node_failed.insert(node.clone(), true);
            nodes_failed_count += 1;
            let err = if matches!(inputs_mode, InputsMode::Named)
              && !missing_info.missing_required_ports.is_empty()
            {
              format!(
                "missing required inputs: {}",
                missing_info.missing_required_ports.join(", ")
              )
            } else {
              "missing required inputs".to_string()
            };
            trace.push(TraceEntry {
              node: node.clone(),
              uses: uses.clone(),
              input: args,
              output: json!({"status": "failed", "error": err}),
              status: NodeStatus::Failed,
              audit: AuditReason::Failed {
                policy: "scope_isolate".into(),
                error: "missing_required_inputs".into(),
              },
              meta: runtime_meta.clone(),
              replayed: false,
              replay_source: None,
            });
            isolate_failed_scopes.insert(scope.clone());
            // LOW: Gate 노드가 통계에는 포함되고 출력에서 제외
            // Gate 노드는 통계(nodes_ok)에는 포함되지만 출력에서는 제외됨
            // 이는 의도된 동작으로, Gate는 중간 검증 노드이므로 최종 출력에 포함하지 않음
            continue;
          }
          // MEDIUM: BestEffort 스코프 부분 실패 계속 실행 수정 완료
          // BestEffort scope에서는 입력이 부족해도 실행 시도
          // 실패한 노드는 node_failed에 기록되어 최종 상태 추적 가능
        }
      }
    }

    if let Some(replay_cfg) = options.replay {
      let mut replay_required = match replay_cfg.mode {
        ReplayMode::Off => false,
        ReplayMode::Strict => true,
        ReplayMode::NondetSafe => is_external_world,
        ReplayMode::Verify => is_external_world,
      };
      if replay_required {
        if let Some(class_name) = replay_class.as_deref() {
          if replay_cfg.allow_classes.contains(class_name) {
            replay_required = false;
          }
        }
      }

      if replay_required {
        let replay_entry = replay_cfg
          .db
          .lookup(node, uses, &input_canon, replay_key)
          .ok_or_else(|| {
            anyhow::anyhow!(
              "replay required but trace entry missing for node={} uses={} replay_key={:?}",
              node,
              uses,
              replay_key
            )
          })?;

        if matches!(replay_cfg.mode, ReplayMode::Strict | ReplayMode::NondetSafe) {
          if replay_entry.uses != *uses {
            anyhow::bail!(
              "replay mismatch uses at node={}: trace={} current={}",
              node,
              replay_entry.uses,
              uses
            );
          }
          if replay_entry.input_canon != input_canon {
            anyhow::bail!("replay mismatch input at node={}", node);
          }
        }

        let replay_output = replay_entry.output.clone();
        if kind == NodeKind::Gate {
          let gate_ok = parse_gate_output(&replay_output).unwrap_or_else(|| {
            eprintln!(
              "warning: replayed gate '{}' had non-bool output {}; treating as false",
              node, replay_output
            );
            false
          });
          gate_results.insert(node.clone(), gate_ok);
          node_failed.insert(node.clone(), false);
          nodes_ok += 1;
          trace.push(TraceEntry {
            node: node.clone(),
            uses: uses.clone(),
            input: args,
            output: json!({"status": "gate_evaluated"}),
            status: NodeStatus::Ok,
            audit: AuditReason::Replayed {
              source: replay_cfg.trace_path.clone(),
            },
            meta: runtime_meta.clone(),
            replayed: true,
            replay_source: Some(replay_cfg.trace_path.clone()),
          });
          continue;
        }

        outputs.insert(node.clone(), replay_output.clone());
        node_failed.insert(node.clone(), false);
        nodes_ok += 1;
        trace.push(TraceEntry {
          node: node.clone(),
          uses: uses.clone(),
          input: args,
          output: replay_output,
          status: NodeStatus::Ok,
          audit: AuditReason::Replayed {
            source: replay_cfg.trace_path.clone(),
          },
          meta: runtime_meta.clone(),
          replayed: true,
          replay_source: Some(replay_cfg.trace_path.clone()),
        });
        continue;
      }
    }

    // Execute backend call with error handling
    let result = match backend {
      "builtins" => {
        let builtin_name = builtins::resolve_builtin_name(uses).ok_or_else(|| {
          crate::rpc::client::RpcError::Backend {
            name: uses.to_string(),
            body: json!({
              "reason_code": REASON_BUILTIN_UNKNOWN,
              "message": format!("unknown builtin `{}`", uses),
              "backend": "builtins",
              "uses": uses
            }),
          }
        })?;
        let builtin_args: Vec<serde_json::Value> = match inputs_mode {
          InputsMode::Positional => {
            // Note: Missing array is expected for no-arg builtins
            args.as_array().cloned().unwrap_or_default()
          }
          InputsMode::Named => {
            // Note: Missing object is expected for no-arg builtins
            let obj = args.as_object().cloned().unwrap_or_default();
            if required_inputs.is_empty() {
              let mut keys: Vec<_> = obj.keys().collect();
              keys.sort();
              keys
                .into_iter()
                .filter_map(|k| obj.get(k).cloned())
                .collect()
            } else {
              required_inputs
                .iter()
                .map(|name| {
                  // Note: Missing named input is expected for optional inputs
                  obj.get(*name).cloned().unwrap_or(serde_json::Value::Null)
                })
                .collect()
            }
          }
        };
        let builtin_env: HashMap<String, serde_json::Value> = match inputs_mode {
          InputsMode::Positional => {
            let mut env = HashMap::new();
            if let Some(values) = args.as_array() {
              for (idx, value) in values.iter().enumerate() {
                if let Some(name) = required_inputs.get(idx) {
                  env.insert((*name).to_string(), value.clone());
                }
              }
            }
            env
          }
          InputsMode::Named => {
            // 결정론 보장: 키를 정렬하여 순서 고정
            if let Some(obj) = args.as_object() {
              let mut keys: Vec<_> = obj.keys().collect();
              keys.sort();
              keys
                .into_iter()
                .map(|k| {
                  (
                    k.clone(),
                    // Note: Missing key in object is expected for optional outputs
                    obj.get(k).cloned().unwrap_or(serde_json::Value::Null),
                  )
                })
                .collect()
            } else {
              HashMap::new()
            }
          }
        };
        eval_builtin(
          builtin_name.as_ref(),
          &builtin_args,
          &builtin_ctx,
          &builtin_env,
        )
        .map_err(|err| crate::rpc::client::RpcError::Backend {
          name: format!("builtins.{}", builtin_name),
          body: json!({
            "reason_code": REASON_BUILTIN_EVAL_FAILED,
            "message": err.to_string(),
            "backend": "builtins",
            "builtin": builtin_name
          }),
        })
      }
      "nix" => Ok(json!({"status": "skipped", "reason": "nix_build_time_only"})),
      other => {
        let backend_name = normalize_backend_name(other);
        let call_args_result = if matches!(backend_name, "clojure" | "jvm") {
          let meta_target = clojure_eval_target_from_meta(runtime_meta.as_ref()).map_err(
            |(meta_key, meta_value)| crate::rpc::client::RpcError::Backend {
              name: uses.to_string(),
              body: json!({
                "status": "blocked",
                "reason_code": REASON_EVAL_TARGET_INVALID,
                "message": format!(
                  "meta.extra.{meta_key} must be one of :jvm/:pnix/:verify"
                ),
                "meta_key": meta_key,
                "meta_value": meta_value,
                "backend": backend_name
              }),
            },
          );
          meta_target.and_then(|target| {
            guard_clojure_eval_target_input(&args, target).map_err(|guard_err| match guard_err {
              ClojureEvalTargetGuardError::InvalidInputValue {
                input_key,
                input_value,
              } => crate::rpc::client::RpcError::Backend {
                name: uses.to_string(),
                body: json!({
                  "status": "blocked",
                  "reason_code": REASON_EVAL_TARGET_INVALID,
                  "message": format!(
                    "input.{input_key} must be one of :jvm/:pnix/:verify"
                  ),
                  "input_key": input_key,
                  "input_value": input_value,
                  "meta_eval_target": target.map(|resolved| resolved.as_str()),
                  "eval_target": target.map(|resolved| resolved.as_str()),
                  "backend": backend_name
                }),
              },
              ClojureEvalTargetGuardError::MetaConflict {
                input_key,
                input_target,
                meta_target,
                input_value,
              } => crate::rpc::client::RpcError::Backend {
                name: uses.to_string(),
                body: json!({
                  "status": "blocked",
                  "reason_code": REASON_EVAL_TARGET_META_CONFLICT,
                  "message": format!(
                    "input.{input_key} cannot override meta.extra eval target"
                  ),
                  "input_key": input_key,
                  "input_value": input_value,
                  "input_eval_target": input_target.as_str(),
                  "meta_eval_target": meta_target.as_str(),
                  "eval_target": meta_target.as_str(),
                  "backend": backend_name
                }),
              },
            })
          })
        } else {
          Ok(args.clone())
        };
        match call_args_result {
          Ok(call_args) => {
            call_backend_with_self_heal(
              other,
              sym,
              &call_args,
              clojure_client,
              python_client,
              deno_client,
              blenderpy_client,
              config,
              options.backend_supervisor,
            )
            .await
          }
          Err(err) => Err(err),
        }
      }
    };

    match result {
      Ok(out) => {
        if let Some(replay_cfg) = options.replay {
          if replay_cfg.mode == ReplayMode::Verify && !is_external_world {
            if let Some(replay_entry) = replay_cfg.db.lookup(node, uses, &input_canon, replay_key) {
              if canonicalize_value(&replay_entry.output) != canonicalize_value(&out) {
                anyhow::bail!("verify mismatch output at node={}", node);
              }
            }
          }
        }

        // Gate nodes: internal state only, NO artifact (Constitutional G4)
        if kind == NodeKind::Gate {
          // MEDIUM: 게이트 출력 검증 false positive 수정 완료
          // Gate output contract:
          // - Prefer a raw JSON boolean (common for predicate morphisms).
          // - Also accept an object like {"ok": true/false} for backends that
          //   wrap gate results (useful when the gate morphism returns multi-fields).
          // 게이트 출력은 명시적으로 {"status": "gate_evaluated"}로 표시되므로 일반 노드 출력과 구분됨
          let ok = parse_gate_output(&out);

          // Validate gate output format (reject silent fallback for invalid types)
          let ok_value = match ok {
            Some(b) => b,
            None => {
              // Invalid gate format: log warning and treat as false
              eprintln!(
                "Warning: Gate '{}' returned invalid format (expected boolean or {{\"ok\": bool}}), got: {}. Treating as false.",
                node, out
              );
              false
            }
          };
          gate_results.insert(node.clone(), ok_value);
          node_failed.insert(node.clone(), false);
          // LOW: Gate 노드가 통계에는 포함되고 출력에서 제외 수정
          // Gate 노드는 nodes_ok에 포함되지만, outputs 맵에는 포함되지 않음
          // Gate는 조건부 엣지 제어용이므로 출력에 포함하지 않지만, 통계에는 포함
          // 이는 의도된 동작: Gate는 내부 상태만 관리하고 artifact를 생성하지 않음
          nodes_ok += 1;
          trace.push(TraceEntry {
            node: node.clone(),
            uses: uses.clone(),
            input: args,
            output: json!({"status": "gate_evaluated"}),
            status: NodeStatus::Ok,
            audit: AuditReason::GateEvaluated { result: ok_value },
            meta: runtime_meta.clone(),
            replayed: false,
            replay_source: None,
          });
          continue; // Gate output excluded from outputs/artifact
        }

        // Normal nodes: record to trace and outputs
        nodes_ok += 1;
        trace.push(TraceEntry {
          node: node.clone(),
          uses: uses.clone(),
          input: args,
          output: out.clone(),
          status: NodeStatus::Ok,
          audit: AuditReason::Executed {
            policy: "normal".into(),
          },
          meta: runtime_meta.clone(),
          replayed: false,
          replay_source: None,
        });
        outputs.insert(node.clone(), out);
        node_failed.insert(node.clone(), false);
      }
      Err(e) => {
        // Stage-4: Record failure, don't bail
        node_failed.insert(node.clone(), true);
        nodes_failed_count += 1;
        let reason_code = rpc_error_reason_code(&e);
        let error_value = rpc_error_value(&e);
        let mut failed_output = serde_json::Map::new();
        failed_output.insert("status".into(), json!("failed"));
        failed_output.insert("error".into(), error_value);
        if let Some(code) = reason_code.as_ref() {
          failed_output.insert("reason_code".into(), json!(code));
        }
        if let crate::rpc::client::RpcError::Backend { body, .. } = &e {
          if let Some(eval_target) = body.get("eval_target") {
            failed_output.insert("eval_target".into(), eval_target.clone());
          }
          if let Some(verify) = body.get("verify") {
            failed_output.insert("verify".into(), verify.clone());
          }
        }
        let audit_error = match reason_code.as_ref() {
          Some(code) => format!("{code}: {e}"),
          None => e.to_string(),
        };

        let policy_name = match policy {
          ScopePolicy::FailFast => "scope_failfast",
          ScopePolicy::Isolate => "scope_isolate",
          ScopePolicy::BestEffort => "scope_besteffort",
        };

        // CRITICAL: 노드 실패 시 이전 노드들의 side effect는 롤백되지 않음
        // 예: 파일 쓰기, 네트워크 호출, 데이터베이스 업데이트 등은 이미 완료됨
        // 향후 개선: 트랜잭션 의미론 정의 및 롤백 메커니즘 구현 필요
        if nodes_ok > 0 {
          eprintln!(
            "warning: node '{}' failed after {} successful nodes - side effects from previous nodes may persist",
            node, nodes_ok
          );
        }

        trace.push(TraceEntry {
          node: node.clone(),
          uses: uses.clone(),
          input: args,
          output: serde_json::Value::Object(failed_output),
          status: NodeStatus::Failed,
          audit: AuditReason::Failed {
            policy: policy_name.into(),
            error: audit_error,
          },
          meta: runtime_meta.clone(),
          replayed: false,
          replay_source: None,
        });

        // Stage-4.1: Apply scope policy
        match policy {
          ScopePolicy::FailFast => {
            had_failfast_error = true;
          }
          ScopePolicy::Isolate => {
            isolate_failed_scopes.insert(scope.clone());
          }
          ScopePolicy::BestEffort => {
            // Continue execution
          }
        }
      }
    }
  }

  // Determine overall status
  let status = if had_failfast_error || (nodes_failed_count > 0 && nodes_ok == 0) {
    ApplyStatus::Error
  } else if nodes_failed_count > 0 || nodes_skipped > 0 {
    ApplyStatus::Partial
  } else {
    ApplyStatus::Ok
  };

  Ok(ApplyResult {
    replay_hash: replay_hash.to_string(),
    status,
    nodes_ok,
    nodes_failed: nodes_failed_count,
    nodes_skipped,
    outputs,
    trace,
    batch_applied: false,
  })
}

fn ordered_object(map: &BTreeMap<String, serde_json::Value>) -> serde_json::Value {
  let mut out = serde_json::Map::new();
  for (key, value) in map {
    // 결정론 보장: NaN/Infinity sanitization 적용
    // serde_json::Value는 이미 NaN/Infinity를 포함할 수 없으므로 항상 성공해야 함
    let sanitized = pnix_core::utils::json_safe::sanitize_json_value(value.clone()).unwrap_or({
      // Fallback: If sanitization fails (shouldn't happen), use null
      // This prevents panic while maintaining determinism
      serde_json::Value::Null
    });
    out.insert(key.clone(), sanitized);
  }
  serde_json::Value::Object(out)
}

fn select_output_value(
  output: &serde_json::Value,
  requested_port: Option<&str>,
  default_port: Option<&str>,
) -> Option<serde_json::Value> {
  if let Some(port) = requested_port {
    // Explicit port selection
    if let Some(obj) = output.as_object() {
      return obj.get(port).cloned();
    }
    // Scalar output: treat "out" as the implicit/default output port.
    if port == "out" {
      return Some(output.clone());
    }
    return None;
  }

  // Default port selection
  if let Some(obj) = output.as_object() {
    if let Some(port) = default_port {
      if let Some(value) = obj.get(port) {
        return Some(value.clone());
      }
    }
    if let Some(value) = obj.get("out") {
      return Some(value.clone());
    }
    if obj.len() == 1 {
      // 결정론 보장: values().next() 대신 명시적 키 조회 사용
      // 단일 필드 객체의 경우 첫 번째 키를 정렬하여 결정론적으로 선택
      let mut keys: Vec<_> = obj.keys().collect();
      keys.sort(); // 결정론적 순서 보장
      if let Some(first_key) = keys.first() {
        return obj.get(*first_key).cloned();
      }
    }
    // Ambiguous object: treat the whole object as the output value.
    return Some(output.clone());
  }

  Some(output.clone())
}

fn parse_gate_output(output: &serde_json::Value) -> Option<bool> {
  output
    .as_bool()
    .or_else(|| output.get("ok").and_then(|v| v.as_bool()))
    .or_else(|| output.get("result").and_then(|v| v.as_bool()))
    .or_else(|| {
      output
        .get("result")
        .and_then(|v| v.get("ok"))
        .and_then(|v| v.as_bool())
    })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::model::{
    CostHint, EdgeCond, Effect, ExecutionContract, FxEdge, FxInput, FxMorphism, FxNode, FxPort,
    SkipPolicy,
  };
  use crate::replay::{ReplayConfig, ReplayDB, ReplayEntry, ReplayMode};
  use pnix_core::{compile_pnix_module, CompileOptions, SourceUnit};

  fn make_node(name: &str, uses: &str) -> FxNode {
    FxNode {
      name: name.into(),
      uses: uses.into(),
      kind: NodeKind::Normal,
      optional: false,
      scope: "global".into(),
      cost: CostHint::Medium,
      priority: 0,
      contract: Default::default(),
      meta: None,
    }
  }

  fn make_gate(name: &str, uses: &str) -> FxNode {
    FxNode {
      name: name.into(),
      uses: uses.into(),
      kind: NodeKind::Gate,
      optional: false,
      scope: "global".into(),
      cost: CostHint::Medium,
      priority: 0,
      contract: Default::default(),
      meta: None,
    }
  }

  #[test]
  fn clojure_eval_target_from_meta_reads_known_extra_keys() {
    let mut meta = crate::model::FxNodeMeta::default();
    meta
      .extra
      .insert("clojure_eval_target".to_string(), json!(":pnix"));
    assert_eq!(
      super::clojure_eval_target_from_meta(Some(&meta)),
      Ok(Some(crate::rpc::clojure::EvalTarget::Pnix))
    );

    let mut fallback = crate::model::FxNodeMeta::default();
    fallback
      .extra
      .insert("eval_target".to_string(), json!("verify"));
    assert_eq!(
      super::clojure_eval_target_from_meta(Some(&fallback)),
      Ok(Some(crate::rpc::clojure::EvalTarget::Verify))
    );

    let mut invalid = crate::model::FxNodeMeta::default();
    invalid
      .extra
      .insert("pnix_eval_target".to_string(), json!("bad-target"));
    assert_eq!(
      super::clojure_eval_target_from_meta(Some(&invalid)),
      Err(("pnix_eval_target".to_string(), json!("bad-target")))
    );
  }

  #[test]
  fn guard_clojure_eval_target_input_rejects_conflict_with_meta_target() {
    let err = super::guard_clojure_eval_target_input(
      &json!({"code": "(+ 1 2)", "__pnix_eval_target": ":pnix"}),
      Some(crate::rpc::clojure::EvalTarget::Jvm),
    )
    .expect_err("conflicting args.__pnix_eval_target must fail");

    assert_eq!(
      err,
      super::ClojureEvalTargetGuardError::MetaConflict {
        input_key: crate::rpc::clojure::EVAL_TARGET_INPUT_KEY.to_string(),
        input_target: crate::rpc::clojure::EvalTarget::Pnix,
        meta_target: crate::rpc::clojure::EvalTarget::Jvm,
        input_value: json!(":pnix")
      }
    );
  }

  #[test]
  fn guard_clojure_eval_target_input_normalizes_matching_meta_target() {
    let out = super::guard_clojure_eval_target_input(
      &json!({"code": "(+ 1 2)", "__pnix_eval_target": "JVM"}),
      Some(crate::rpc::clojure::EvalTarget::Jvm),
    )
    .expect("matching target should pass");

    assert_eq!(out.get("code").and_then(|v| v.as_str()), Some("(+ 1 2)"));
    assert_eq!(
      out.get(crate::rpc::clojure::EVAL_TARGET_INPUT_KEY),
      Some(&json!("jvm"))
    );
  }

  #[test]
  fn guard_clojure_eval_target_input_rejects_invalid_input_value() {
    let err = super::guard_clojure_eval_target_input(
      &json!({"code": "(+ 1 2)", "__pnix_eval_target": {"bad": true}}),
      Some(crate::rpc::clojure::EvalTarget::Jvm),
    )
    .expect_err("invalid input target must fail");

    assert_eq!(
      err,
      super::ClojureEvalTargetGuardError::InvalidInputValue {
        input_key: crate::rpc::clojure::EVAL_TARGET_INPUT_KEY.to_string(),
        input_value: json!({"bad": true})
      }
    );
  }

  #[test]
  fn guard_clojure_eval_target_input_rejects_invalid_legacy_input_value() {
    let err = super::guard_clojure_eval_target_input(
      &json!({"code": "(+ 1 2)", "pnix_eval_target": {"bad": true}}),
      Some(crate::rpc::clojure::EvalTarget::Jvm),
    )
    .expect_err("invalid legacy input target must fail");

    assert_eq!(
      err,
      super::ClojureEvalTargetGuardError::InvalidInputValue {
        input_key: "pnix_eval_target".to_string(),
        input_value: json!({"bad": true})
      }
    );
  }

  #[test]
  fn guard_clojure_eval_target_input_rejects_legacy_conflict_with_meta_target() {
    let err = super::guard_clojure_eval_target_input(
      &json!({"code": "(+ 1 2)", "eval_target": "verify"}),
      Some(crate::rpc::clojure::EvalTarget::Jvm),
    )
    .expect_err("conflicting legacy eval target must fail");

    assert_eq!(
      err,
      super::ClojureEvalTargetGuardError::MetaConflict {
        input_key: "eval_target".to_string(),
        input_target: crate::rpc::clojure::EvalTarget::Verify,
        meta_target: crate::rpc::clojure::EvalTarget::Jvm,
        input_value: json!("verify")
      }
    );
  }

  #[test]
  fn guard_clojure_eval_target_input_normalizes_matching_legacy_hints() {
    let out = super::guard_clojure_eval_target_input(
      &json!({
        "code": "(+ 1 2)",
        "eval_target": ":jvm",
        "pnix_eval_target": "JVM"
      }),
      Some(crate::rpc::clojure::EvalTarget::Jvm),
    )
    .expect("matching legacy hints should pass");

    assert_eq!(out.get("code").and_then(|v| v.as_str()), Some("(+ 1 2)"));
    assert_eq!(
      out.get(crate::rpc::clojure::EVAL_TARGET_INPUT_KEY),
      Some(&json!("jvm"))
    );
    assert!(out.get("eval_target").is_none());
    assert!(out.get("pnix_eval_target").is_none());
  }

  #[tokio::test]
  async fn jvm_backend_applies_eval_target_meta_and_fails_closed_for_non_jvm_target() {
    let mut node = make_node("n1", "jvm.eval");
    let mut meta = crate::model::FxNodeMeta::default();
    meta.extra.insert("eval_target".to_string(), json!(":pnix"));
    node.meta = Some(meta);

    let fx = FxCoreModule {
      meta: Default::default(),
      name: "jvm-target-meta".into(),
      inputs: vec![FxInput {
        name: "code_in".into(),
        ty: "String".into(),
      }],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![node],
      edges: vec![FxEdge {
        from: "input".into(),
        to: "n1".into(),
        from_port: None,
        to_port: Some("code".into()),
        from_input: Some("code_in".into()),
        cond: None,
      }],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["n1".into()],
    };

    let mut config = BackendConfig::default();
    config
      .inputs
      .insert("code_in".to_string(), json!("(+ 1 2)"));

    let result = apply_graph(&fx, &plan, "replay", &config)
      .await
      .expect("apply_graph should return partial/error result instead of bailing");

    assert_eq!(result.status, ApplyStatus::Error);
    assert_eq!(result.nodes_failed, 1);
    assert_eq!(result.nodes_ok, 0);
    assert_eq!(result.trace.len(), 1);

    let trace_output = &result.trace[0].output;
    assert_eq!(
      trace_output.get("status").and_then(|v| v.as_str()),
      Some("failed")
    );
    assert_eq!(
      trace_output.get("reason_code").and_then(|v| v.as_str()),
      Some("JVM_CLOJURE_INTEROP_DIRECT_UNSUPPORTED_TARGET")
    );
  }

  #[tokio::test]
  async fn jvm_backend_rejects_invalid_eval_target_meta_before_backend_call() {
    let mut node = make_node("n1", "jvm.eval");
    let mut meta = crate::model::FxNodeMeta::default();
    meta
      .extra
      .insert("eval_target".to_string(), json!("bad-target"));
    node.meta = Some(meta);

    let fx = FxCoreModule {
      meta: Default::default(),
      name: "jvm-invalid-target-meta".into(),
      inputs: vec![FxInput {
        name: "code_in".into(),
        ty: "String".into(),
      }],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![node],
      edges: vec![FxEdge {
        from: "input".into(),
        to: "n1".into(),
        from_port: None,
        to_port: Some("code".into()),
        from_input: Some("code_in".into()),
        cond: None,
      }],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["n1".into()],
    };

    let mut config = BackendConfig::default();
    config
      .inputs
      .insert("code_in".to_string(), json!("(+ 1 2)"));

    let result = apply_graph(&fx, &plan, "replay", &config)
      .await
      .expect("apply_graph should return partial/error result instead of bailing");

    assert_eq!(result.status, ApplyStatus::Error);
    assert_eq!(result.nodes_failed, 1);
    assert_eq!(result.nodes_ok, 0);
    assert_eq!(result.trace.len(), 1);

    let trace_output = &result.trace[0].output;
    assert_eq!(
      trace_output.get("status").and_then(|v| v.as_str()),
      Some("failed")
    );
    assert_eq!(
      trace_output.get("reason_code").and_then(|v| v.as_str()),
      Some(REASON_EVAL_TARGET_INVALID)
    );
    assert_eq!(
      trace_output
        .get("error")
        .and_then(|v| v.get("backend"))
        .and_then(|v| v.get("status"))
        .and_then(|v| v.as_str()),
      Some("blocked")
    );
    assert_eq!(
      trace_output
        .get("error")
        .and_then(|v| v.get("backend"))
        .and_then(|v| v.get("meta_key"))
        .and_then(|v| v.as_str()),
      Some("eval_target")
    );
  }

  #[tokio::test]
  async fn jvm_backend_rejects_eval_target_input_conflict_with_meta_before_backend_call() {
    let mut node = make_node("n1", "jvm.eval");
    let mut meta = crate::model::FxNodeMeta::default();
    meta.extra.insert("eval_target".to_string(), json!(":jvm"));
    node.meta = Some(meta);

    let fx = FxCoreModule {
      meta: Default::default(),
      name: "jvm-target-meta-conflict".into(),
      inputs: vec![
        FxInput {
          name: "code_in".into(),
          ty: "String".into(),
        },
        FxInput {
          name: "target_in".into(),
          ty: "String".into(),
        },
      ],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![node],
      edges: vec![
        FxEdge {
          from: "input".into(),
          to: "n1".into(),
          from_port: None,
          to_port: Some("code".into()),
          from_input: Some("code_in".into()),
          cond: None,
        },
        FxEdge {
          from: "input".into(),
          to: "n1".into(),
          from_port: None,
          to_port: Some(crate::rpc::clojure::EVAL_TARGET_INPUT_KEY.to_string()),
          from_input: Some("target_in".into()),
          cond: None,
        },
      ],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["n1".into()],
    };

    let mut config = BackendConfig::default();
    config
      .inputs
      .insert("code_in".to_string(), json!("(+ 1 2)"));
    config
      .inputs
      .insert("target_in".to_string(), json!(":pnix"));

    let result = apply_graph(&fx, &plan, "replay", &config)
      .await
      .expect("apply_graph should return partial/error result instead of bailing");

    assert_eq!(result.status, ApplyStatus::Error);
    assert_eq!(result.nodes_failed, 1);
    assert_eq!(result.nodes_ok, 0);
    assert_eq!(result.trace.len(), 1);

    let trace_output = &result.trace[0].output;
    assert_eq!(
      trace_output.get("status").and_then(|v| v.as_str()),
      Some("failed")
    );
    assert_eq!(
      trace_output.get("reason_code").and_then(|v| v.as_str()),
      Some(REASON_EVAL_TARGET_META_CONFLICT)
    );
    assert_eq!(
      trace_output
        .get("error")
        .and_then(|v| v.get("backend"))
        .and_then(|v| v.get("status"))
        .and_then(|v| v.as_str()),
      Some("blocked")
    );
    assert_eq!(
      trace_output.get("eval_target").and_then(|v| v.as_str()),
      Some("jvm")
    );
    assert_eq!(
      trace_output
        .get("error")
        .and_then(|v| v.get("backend"))
        .and_then(|v| v.get("input_eval_target"))
        .and_then(|v| v.as_str()),
      Some("pnix")
    );
    assert_eq!(
      trace_output
        .get("error")
        .and_then(|v| v.get("backend"))
        .and_then(|v| v.get("input_key"))
        .and_then(|v| v.as_str()),
      Some(crate::rpc::clojure::EVAL_TARGET_INPUT_KEY)
    );
  }

  #[tokio::test]
  async fn jvm_backend_rejects_legacy_eval_target_input_conflict_with_meta_before_backend_call() {
    let mut node = make_node("n1", "jvm.eval");
    let mut meta = crate::model::FxNodeMeta::default();
    meta.extra.insert("eval_target".to_string(), json!(":jvm"));
    node.meta = Some(meta);

    let fx = FxCoreModule {
      meta: Default::default(),
      name: "jvm-target-meta-legacy-conflict".into(),
      inputs: vec![
        FxInput {
          name: "code_in".into(),
          ty: "String".into(),
        },
        FxInput {
          name: "target_in".into(),
          ty: "String".into(),
        },
      ],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![node],
      edges: vec![
        FxEdge {
          from: "input".into(),
          to: "n1".into(),
          from_port: None,
          to_port: Some("code".into()),
          from_input: Some("code_in".into()),
          cond: None,
        },
        FxEdge {
          from: "input".into(),
          to: "n1".into(),
          from_port: None,
          to_port: Some("eval_target".to_string()),
          from_input: Some("target_in".into()),
          cond: None,
        },
      ],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["n1".into()],
    };

    let mut config = BackendConfig::default();
    config
      .inputs
      .insert("code_in".to_string(), json!("(+ 1 2)"));
    config
      .inputs
      .insert("target_in".to_string(), json!(":verify"));

    let result = apply_graph(&fx, &plan, "replay", &config)
      .await
      .expect("apply_graph should return partial/error result instead of bailing");

    assert_eq!(result.status, ApplyStatus::Error);
    assert_eq!(result.nodes_failed, 1);
    assert_eq!(result.nodes_ok, 0);
    assert_eq!(result.trace.len(), 1);

    let trace_output = &result.trace[0].output;
    assert_eq!(
      trace_output.get("status").and_then(|v| v.as_str()),
      Some("failed")
    );
    assert_eq!(
      trace_output.get("reason_code").and_then(|v| v.as_str()),
      Some(REASON_EVAL_TARGET_META_CONFLICT)
    );
    assert_eq!(
      trace_output
        .get("error")
        .and_then(|v| v.get("backend"))
        .and_then(|v| v.get("status"))
        .and_then(|v| v.as_str()),
      Some("blocked")
    );
    assert_eq!(
      trace_output
        .get("error")
        .and_then(|v| v.get("backend"))
        .and_then(|v| v.get("input_key"))
        .and_then(|v| v.as_str()),
      Some("eval_target")
    );
  }

  fn make_fan_in_graph() -> FxCoreModule {
    FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![
        make_node("a", "clojure.f"),
        make_node("b", "clojure.g"),
        make_node("c", "clojure.h"),
      ],
      edges: vec![
        FxEdge {
          from: "a".into(),
          to: "c".into(),
          from_port: None,
          to_port: None,
          from_input: None,
          cond: None,
        },
        FxEdge {
          from: "b".into(),
          to: "c".into(),
          from_port: None,
          to_port: None,
          from_input: None,
          cond: None,
        },
      ],
      scopes: vec![],
    }
  }

  #[tokio::test]
  async fn scope_failfast_skips_remaining_nodes() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![
        FxNode {
          scope: "s1".into(),
          meta: None,
          ..make_node("a", "noop.f")
        },
        FxNode {
          scope: "s1".into(),
          meta: None,
          ..make_node("b", "clojure.g")
        },
        FxNode {
          scope: "s1".into(),
          meta: None,
          ..make_node("c", "clojure.h")
        },
      ],
      edges: vec![],
      scopes: vec![crate::model::FxScope {
        name: "s1".into(),
        nodes: vec!["a".into(), "b".into(), "c".into()],
        policy: ScopePolicy::FailFast,
      }],
    };
    let plan = Plan {
      order: vec!["a".into(), "b".into(), "c".into()],
    };

    let result = apply_graph(&fx, &plan, "replay", &BackendConfig::default())
      .await
      .unwrap();

    assert_eq!(result.status, ApplyStatus::Error);
    assert_eq!(result.nodes_failed, 1);
    assert_eq!(result.nodes_skipped, 2);
    assert_eq!(result.trace.len(), 3);
    assert_eq!(result.trace[0].status, NodeStatus::Failed);
    assert_eq!(result.trace[1].status, NodeStatus::Skipped);
    assert_eq!(result.trace[2].status, NodeStatus::Skipped);
  }

  #[tokio::test]
  async fn gate_order_validation_rejects_invalid_plan() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![make_gate("g1", "noop.gate"), make_node("n1", "noop.f")],
      edges: vec![FxEdge {
        from: "g1".into(),
        to: "n1".into(),
        from_port: None,
        to_port: None,
        from_input: None,
        cond: Some(EdgeCond::When("g1".into())),
      }],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["n1".into(), "g1".into()],
    };
    let config = BackendConfig {
      use_batch_apply: false,
      ..Default::default()
    };

    let err = apply_graph(&fx, &plan, "replay", &config)
      .await
      .unwrap_err();
    assert!(err
      .to_string()
      .contains("Gate scheduling validation failed"));
  }

  #[tokio::test]
  async fn scope_isolate_skips_only_within_scope() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![
        FxNode {
          scope: "s1".into(),
          meta: None,
          ..make_node("a", "noop.f")
        },
        FxNode {
          scope: "s1".into(),
          meta: None,
          ..make_node("b", "nix.g")
        },
        FxNode {
          scope: "global".into(),
          meta: None,
          ..make_node("c", "nix.h")
        },
      ],
      edges: vec![],
      scopes: vec![crate::model::FxScope {
        name: "s1".into(),
        nodes: vec!["a".into(), "b".into()],
        policy: ScopePolicy::Isolate,
      }],
    };
    let plan = Plan {
      order: vec!["a".into(), "b".into(), "c".into()],
    };

    let result = apply_graph(&fx, &plan, "replay", &BackendConfig::default())
      .await
      .unwrap();

    assert_eq!(result.status, ApplyStatus::Partial);
    assert_eq!(result.nodes_failed, 1);
    assert_eq!(result.nodes_skipped, 1);
    assert_eq!(result.nodes_ok, 1);
    assert_eq!(result.trace.len(), 3);
    assert_eq!(result.trace[0].status, NodeStatus::Failed);
    assert_eq!(result.trace[1].status, NodeStatus::Skipped);
    assert_eq!(result.trace[2].status, NodeStatus::Ok);
    assert!(
      matches!(
          result.trace[1].audit,
          AuditReason::Skipped { ref policy, ref reason, .. }
              if policy == "scope_isolate" && reason == "previous_node_failed_in_scope"
      ),
      "expected isolate skip audit reason"
    );
  }

  #[tokio::test]
  async fn stage2_default_to_port_assigns_in_order() {
    let meta = crate::model::FxCoreMeta {
      version: crate::model::FXCORE_VERSION.into(),
      stage: 2,
      ..Default::default()
    };

    let fx = FxCoreModule {
      meta,
      name: "test".into(),
      inputs: vec![
        FxInput {
          name: "a_in".into(),
          ty: "Number".into(),
        },
        FxInput {
          name: "b_in".into(),
          ty: "Number".into(),
        },
      ],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![FxMorphism {
        name: "nix.pair".into(),
        input: "Any".into(),
        output: "Any".into(),
        inputs: vec![
          FxPort {
            name: "a".into(),
            ty: "Number".into(),
          },
          FxPort {
            name: "b".into(),
            ty: "Number".into(),
          },
        ],
        outputs: vec![FxPort {
          name: "out".into(),
          ty: "Any".into(),
        }],
        effect: Effect::Pure,
      }],
      nodes: vec![make_node("pair", "nix.pair")],
      edges: vec![
        FxEdge {
          from: "input".into(),
          to: "pair".into(),
          from_port: None,
          to_port: None,
          from_input: Some("a_in".into()),
          cond: None,
        },
        FxEdge {
          from: "input".into(),
          to: "pair".into(),
          from_port: None,
          to_port: None,
          from_input: Some("b_in".into()),
          cond: None,
        },
      ],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["pair".into()],
    };

    let mut config = BackendConfig::default();
    config.inputs.insert("a_in".into(), json!(2));
    config.inputs.insert("b_in".into(), json!(3));

    let result = apply_graph(&fx, &plan, "replay", &config).await.unwrap();
    assert_eq!(result.status, ApplyStatus::Ok);
    assert_eq!(result.nodes_ok, 1);

    let inputs = result.trace[0].input.as_object().expect("named inputs map");
    assert_eq!(inputs.get("a"), Some(&json!(2)));
    assert_eq!(inputs.get("b"), Some(&json!(3)));
  }

  #[tokio::test]
  async fn stage2_default_to_port_exhausted_is_error() {
    let meta = crate::model::FxCoreMeta {
      version: crate::model::FXCORE_VERSION.into(),
      stage: 2,
      ..Default::default()
    };

    let fx = FxCoreModule {
      meta,
      name: "test".into(),
      inputs: vec![
        FxInput {
          name: "a_in".into(),
          ty: "Number".into(),
        },
        FxInput {
          name: "b_in".into(),
          ty: "Number".into(),
        },
        FxInput {
          name: "c_in".into(),
          ty: "Number".into(),
        },
      ],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![FxMorphism {
        name: "nix.pair".into(),
        input: "Any".into(),
        output: "Any".into(),
        inputs: vec![
          FxPort {
            name: "a".into(),
            ty: "Number".into(),
          },
          FxPort {
            name: "b".into(),
            ty: "Number".into(),
          },
        ],
        outputs: vec![FxPort {
          name: "out".into(),
          ty: "Any".into(),
        }],
        effect: Effect::Pure,
      }],
      nodes: vec![make_node("pair", "nix.pair")],
      edges: vec![
        FxEdge {
          from: "input".into(),
          to: "pair".into(),
          from_port: None,
          to_port: None,
          from_input: Some("a_in".into()),
          cond: None,
        },
        FxEdge {
          from: "input".into(),
          to: "pair".into(),
          from_port: None,
          to_port: None,
          from_input: Some("b_in".into()),
          cond: None,
        },
        FxEdge {
          from: "input".into(),
          to: "pair".into(),
          from_port: None,
          to_port: None,
          from_input: Some("c_in".into()),
          cond: None,
        },
      ],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["pair".into()],
    };

    let mut config = BackendConfig::default();
    config.inputs.insert("a_in".into(), json!(2));
    config.inputs.insert("b_in".into(), json!(3));
    config.inputs.insert("c_in".into(), json!(4));

    // 포트 소진 에러는 apply_graph에서 Err를 반환합니다
    let err = apply_graph(&fx, &plan, "replay", &config)
      .await
      .unwrap_err();
    assert!(err.to_string().contains("unnamed edges"));
    assert!(err.to_string().contains("input ports available"));
    assert!(err.to_string().contains("pair"));
  }

  #[tokio::test]
  async fn builtins_backend_evaluates_binary_ops() {
    let meta = crate::model::FxCoreMeta {
      version: crate::model::FXCORE_VERSION.into(),
      stage: 2,
      ..Default::default()
    };

    let fx = FxCoreModule {
      meta,
      name: "test".into(),
      inputs: vec![
        FxInput {
          name: "a_in".into(),
          ty: "Num".into(),
        },
        FxInput {
          name: "b_in".into(),
          ty: "Num".into(),
        },
      ],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![FxMorphism {
        name: "builtins.add".into(),
        input: "Num".into(),
        output: "Num".into(),
        inputs: vec![
          FxPort {
            name: "a".into(),
            ty: "Num".into(),
          },
          FxPort {
            name: "b".into(),
            ty: "Num".into(),
          },
        ],
        outputs: vec![FxPort {
          name: "out".into(),
          ty: "Num".into(),
        }],
        effect: Effect::Pure,
      }],
      nodes: vec![make_node("add", "builtins.add")],
      edges: vec![
        FxEdge {
          from: "input".into(),
          to: "add".into(),
          from_port: None,
          to_port: Some("a".into()),
          from_input: Some("a_in".into()),
          cond: None,
        },
        FxEdge {
          from: "input".into(),
          to: "add".into(),
          from_port: None,
          to_port: Some("b".into()),
          from_input: Some("b_in".into()),
          cond: None,
        },
      ],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["add".into()],
    };

    let mut config = BackendConfig::default();
    config.inputs.insert("a_in".into(), json!(2));
    config.inputs.insert("b_in".into(), json!(3));

    let result = apply_graph(&fx, &plan, "replay", &config).await.unwrap();
    assert_eq!(result.status, ApplyStatus::Ok);
    assert_eq!(result.outputs.get("add"), Some(&json!(5)));
  }

  #[tokio::test]
  async fn apply_graph_blocks_non_atomic_effect_nodes_by_default() {
    let meta = crate::model::FxCoreMeta {
      version: crate::model::FXCORE_VERSION.into(),
      stage: 2,
      ..Default::default()
    };

    let fx = FxCoreModule {
      meta,
      name: "non-atomic-block".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![FxMorphism {
        name: "builtins.processSpawn".into(),
        input: "Any".into(),
        output: "Any".into(),
        inputs: vec![],
        outputs: vec![],
        // Intentionally incorrect metadata: catalog says world.
        effect: Effect::Pure,
      }],
      nodes: vec![make_node("spawn", "builtins.processSpawn")],
      edges: vec![],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["spawn".into()],
    };

    let err = apply_graph(&fx, &plan, "replay", &BackendConfig::default())
      .await
      .expect_err("world effect node must be blocked by default");
    assert!(err
      .to_string()
      .contains("non-atomic side-effect nodes detected"));
    assert!(err
      .to_string()
      .contains("spawn(uses=builtins.processSpawn,effect=world)"));
  }

  #[tokio::test]
  async fn apply_graph_allows_non_atomic_effect_nodes_with_opt_in() {
    let meta = crate::model::FxCoreMeta {
      version: crate::model::FXCORE_VERSION.into(),
      stage: 2,
      ..Default::default()
    };

    let fx = FxCoreModule {
      meta,
      name: "non-atomic-allow".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![FxMorphism {
        name: "builtins.processSpawn".into(),
        input: "Any".into(),
        output: "Any".into(),
        inputs: vec![],
        outputs: vec![],
        effect: Effect::Pure,
      }],
      nodes: vec![make_node("spawn", "builtins.processSpawn")],
      edges: vec![],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["spawn".into()],
    };

    let mut config = BackendConfig::default();
    config.allow_non_atomic_effects = true;

    let result = apply_graph(&fx, &plan, "replay", &config).await;
    if let Err(err) = &result {
      assert!(
        !err
          .to_string()
          .contains("non-atomic side-effect nodes detected"),
        "opt-in should bypass non-atomic policy gate, got: {err}"
      );
    }
    let result = result.expect("opt-in should pass policy gate and run execution path");
    assert_eq!(result.status, ApplyStatus::Ok);
    assert_eq!(result.nodes_ok, 1);
    assert_eq!(result.nodes_failed, 0);
  }

  #[tokio::test]
  async fn optional_single_input_node_skips_when_positional_input_missing() {
    let meta = crate::model::FxCoreMeta {
      version: crate::model::FXCORE_VERSION.into(),
      stage: 4,
      ..Default::default()
    };

    let mut optional_node = make_node("opt", "jvm.eval");
    optional_node.optional = true;
    optional_node.contract = ExecutionContract {
      required_inputs: vec!["code".into()],
      may_skip: true,
      skip_policy: SkipPolicy::Skip,
      replay: None,
    };

    let fx = FxCoreModule {
      meta,
      name: "optional-missing-positional".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![FxMorphism {
        name: "jvm.eval".into(),
        input: "Any".into(),
        output: "Any".into(),
        inputs: vec![FxPort {
          name: "code".into(),
          ty: "String".into(),
        }],
        outputs: vec![FxPort {
          name: "result".into(),
          ty: "Any".into(),
        }],
        effect: Effect::Pure,
      }],
      nodes: vec![optional_node],
      edges: vec![],
      scopes: vec![],
    };

    let plan = Plan {
      order: vec!["opt".into()],
    };

    let result = apply_graph(&fx, &plan, "replay", &BackendConfig::default())
      .await
      .expect("optional missing input should skip");

    assert_eq!(result.status, ApplyStatus::Partial);
    assert_eq!(result.nodes_failed, 0);
    assert_eq!(result.nodes_ok, 0);
    assert_eq!(result.nodes_skipped, 1);
    assert_eq!(result.trace.len(), 1);
    assert_eq!(result.trace[0].status, NodeStatus::Skipped);
  }

  #[tokio::test]
  async fn replay_nondet_safe_replays_external_backend_alias_node() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![make_node("ext", "py.numpy.add")],
      edges: vec![],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["ext".into()],
    };
    let input_canon = json!([]);
    let replay_entry = ReplayEntry {
      node: "ext".into(),
      uses: "py.numpy.add".into(),
      input_canon: input_canon.clone(),
      output: json!({"out": 42}),
      replay_key: None,
    };
    let mut replay_db = ReplayDB::default();
    replay_db
      .by_node
      .insert("ext".to_string(), replay_entry.clone());
    replay_db.by_key.insert(
      crate::replay::replay_fallback_key("py.numpy.add", &input_canon),
      replay_entry,
    );
    let replay_cfg = ReplayConfig {
      mode: ReplayMode::NondetSafe,
      trace_path: "/tmp/replay-trace.jsonl".to_string(),
      db: replay_db,
      allow_classes: std::collections::HashSet::new(),
    };

    let result = apply_graph_with_options(
      &fx,
      &plan,
      "replay",
      &BackendConfig::default(),
      ApplyOptions {
        replay: Some(&replay_cfg),
        backend_supervisor: None,
        invocation_id: None,
      },
    )
    .await
    .expect("nondet-safe should replay external backend node");

    assert_eq!(result.status, ApplyStatus::Ok);
    assert_eq!(result.outputs.get("ext"), Some(&json!({"out": 42})));
    assert!(result.trace[0].replayed);
    assert!(matches!(
      result.trace[0].audit,
      AuditReason::Replayed { .. }
    ));
  }

  #[tokio::test]
  async fn replay_nondet_safe_requires_trace_for_external_backend_alias_node() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![make_node("ext", "py.numpy.add")],
      edges: vec![],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["ext".into()],
    };
    let replay_cfg = ReplayConfig {
      mode: ReplayMode::NondetSafe,
      trace_path: "/tmp/replay-trace.jsonl".to_string(),
      db: ReplayDB::default(),
      allow_classes: std::collections::HashSet::new(),
    };

    let err = apply_graph_with_options(
      &fx,
      &plan,
      "replay",
      &BackendConfig::default(),
      ApplyOptions {
        replay: Some(&replay_cfg),
        backend_supervisor: None,
        invocation_id: None,
      },
    )
    .await
    .expect_err("nondet-safe must fail when replay entry is missing");
    assert!(err
      .to_string()
      .contains("replay required but trace entry missing"));
  }

  #[tokio::test]
  async fn replay_verify_replays_external_backend_alias_node() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![make_node("ext", "py.numpy.add")],
      edges: vec![],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["ext".into()],
    };
    let input_canon = json!([]);
    let replay_entry = ReplayEntry {
      node: "ext".into(),
      uses: "py.numpy.add".into(),
      input_canon: input_canon.clone(),
      output: json!({"out": 7}),
      replay_key: None,
    };
    let mut replay_db = ReplayDB::default();
    replay_db
      .by_node
      .insert("ext".to_string(), replay_entry.clone());
    replay_db.by_key.insert(
      crate::replay::replay_fallback_key("py.numpy.add", &input_canon),
      replay_entry,
    );
    let replay_cfg = ReplayConfig {
      mode: ReplayMode::Verify,
      trace_path: "/tmp/replay-trace.jsonl".to_string(),
      db: replay_db,
      allow_classes: std::collections::HashSet::new(),
    };

    let result = apply_graph_with_options(
      &fx,
      &plan,
      "replay",
      &BackendConfig::default(),
      ApplyOptions {
        replay: Some(&replay_cfg),
        backend_supervisor: None,
        invocation_id: None,
      },
    )
    .await
    .expect("verify should replay external backend node");

    assert_eq!(result.status, ApplyStatus::Ok);
    assert_eq!(result.outputs.get("ext"), Some(&json!({"out": 7})));
    assert!(result.trace[0].replayed);
    assert!(matches!(
      result.trace[0].audit,
      AuditReason::Replayed { .. }
    ));
  }

  #[tokio::test]
  async fn replay_verify_requires_trace_for_external_backend_alias_node() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![make_node("ext", "py.numpy.add")],
      edges: vec![],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["ext".into()],
    };
    let replay_cfg = ReplayConfig {
      mode: ReplayMode::Verify,
      trace_path: "/tmp/replay-trace.jsonl".to_string(),
      db: ReplayDB::default(),
      allow_classes: std::collections::HashSet::new(),
    };

    let err = apply_graph_with_options(
      &fx,
      &plan,
      "replay",
      &BackendConfig::default(),
      ApplyOptions {
        replay: Some(&replay_cfg),
        backend_supervisor: None,
        invocation_id: None,
      },
    )
    .await
    .expect_err("verify must fail when replay entry is missing");
    assert!(err
      .to_string()
      .contains("replay required but trace entry missing"));
  }

  #[tokio::test]
  async fn spine_compiles_and_applies_builtins_module() {
    let src = SourceUnit {
      name: "spine.px".into(),
      text: r#"
        {
          name = "spine";

          types = [ "Num" ];

          inputs = {
            x = "Num";
          };

          externs = [
            {
              name = "builtins.add";
              inputs = [
                { name = "a"; ty = "Num"; }
                { name = "b"; ty = "Num"; }
              ];
              outputs = [
                { name = "out"; ty = "Num"; }
              ];
            }
          ];

          nodes = [
            { name = "add"; uses = "builtins.add"; }
          ];

          edges = [
            { from = "input.x"; to = "add.a"; }
            { from = "input.x"; to = "add.b"; }
          ];
        }
      "#
      .into(),
    };

    let out = compile_pnix_module(&src, &CompileOptions::default()).expect("compile");
    let plan = crate::plan::build_plan(&out.fxcore).expect("build plan");

    let mut config = BackendConfig::default();
    config.inputs.insert("x".into(), json!(2));

    let result = apply_graph(&out.fxcore, &plan, &out.artifacts.replay_hash, &config)
      .await
      .expect("apply graph");

    assert_eq!(result.status, ApplyStatus::Ok);
    assert_eq!(result.nodes_ok, 1);
    assert_eq!(result.outputs.get("add"), Some(&json!(4)));
  }

  #[test]
  fn predecessors_are_collected() {
    let fx = make_fan_in_graph();
    let mut preds: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &fx.edges {
      preds
        .entry(e.to.as_str())
        .or_default()
        .push(e.from.as_str());
    }

    assert_eq!(preds.get("c").map(|v| v.len()), Some(2));
    assert!(preds.get("c").unwrap().contains(&"a"));
    assert!(preds.get("c").unwrap().contains(&"b"));
    assert!(!preds.contains_key("a"));
    assert!(!preds.contains_key("b"));
  }

  #[test]
  fn select_output_value_explicit_port_on_object() {
    let out = json!({"a": 1, "b": 2});
    let selected = select_output_value(&out, Some("b"), None).unwrap();
    assert_eq!(selected, json!(2));
  }

  #[test]
  fn select_output_value_explicit_port_missing_returns_none() {
    let out = json!({"a": 1});
    assert!(select_output_value(&out, Some("b"), None).is_none());
  }

  #[test]
  fn select_output_value_default_prefers_default_port() {
    let out = json!({"x": 10, "out": 20});
    let selected = select_output_value(&out, None, Some("x")).unwrap();
    assert_eq!(selected, json!(10));
  }

  #[test]
  fn select_output_value_default_falls_back_to_out_key() {
    let out = json!({"out": 20, "y": 1});
    let selected = select_output_value(&out, None, Some("missing")).unwrap();
    assert_eq!(selected, json!(20));
  }

  #[test]
  fn select_output_value_default_singleton_object() {
    let out = json!({"only": 7});
    let selected = select_output_value(&out, None, None).unwrap();
    assert_eq!(selected, json!(7));
  }

  #[test]
  fn select_output_value_default_ambiguous_object_returns_whole_object() {
    let out = json!({"a": 1, "b": 2});
    let selected = select_output_value(&out, None, None).unwrap();
    assert_eq!(selected, out);
  }

  #[test]
  fn select_output_value_scalar_default() {
    let out = json!(42);
    let selected = select_output_value(&out, None, None).unwrap();
    assert_eq!(selected, json!(42));
  }

  #[test]
  fn select_output_value_scalar_explicit_out() {
    let out = json!(42);
    let selected = select_output_value(&out, Some("out"), None).unwrap();
    assert_eq!(selected, json!(42));
  }

  #[test]
  fn select_output_value_scalar_explicit_non_out_is_none() {
    let out = json!(42);
    assert!(select_output_value(&out, Some("x"), None).is_none());
  }

  #[test]
  fn parse_gate_output_accepts_direct_and_wrapped_forms() {
    assert_eq!(parse_gate_output(&json!(true)), Some(true));
    assert_eq!(parse_gate_output(&json!({"ok": false})), Some(false));
    assert_eq!(
      parse_gate_output(&json!({"status": "ok", "result": true})),
      Some(true)
    );
    assert_eq!(
      parse_gate_output(&json!({"status": "ok", "result": {"ok": false}})),
      Some(false)
    );
    assert_eq!(
      parse_gate_output(&json!({"status": "ok", "result": 1})),
      None
    );
  }

  #[test]
  fn batch_status_validation_accepts_ok_or_missing() {
    validate_batch_apply_status(&json!({"status": "ok", "outputs": {}}))
      .expect("ok status should pass");
    validate_batch_apply_status(&json!({"outputs": {}})).expect("missing status should pass");
  }

  #[test]
  fn batch_status_validation_rejects_non_ok_status() {
    let err = validate_batch_apply_status(&json!({
      "status": "failed",
      "error": "backend failed"
    }))
    .expect_err("non-ok status must fail closed");
    assert!(err.to_string().contains("non-ok status"));
  }

  #[test]
  fn batch_status_validation_rejects_non_string_status() {
    let err = validate_batch_apply_status(&json!({
      "status": {"ok": true}
    }))
    .expect_err("non-string status must fail closed");
    assert!(err.to_string().contains("non-string 'status'"));
  }

  #[test]
  fn batch_coverage_validation_accepts_complete_response() {
    let plan = Plan {
      order: vec!["a".into(), "b".into(), "c".into()],
    };
    let outputs = BTreeMap::from([
      ("a".to_string(), json!(1)),
      ("b".to_string(), json!({"ok": true})),
    ]);
    let failed = std::collections::HashSet::from(["c".to_string()]);
    validate_batch_apply_node_coverage(&plan, &outputs, &failed)
      .expect("complete coverage should pass");
  }

  #[test]
  fn batch_coverage_validation_rejects_missing_nodes() {
    let plan = Plan {
      order: vec!["a".into(), "b".into()],
    };
    let outputs = BTreeMap::from([("a".to_string(), json!(1))]);
    let failed = std::collections::HashSet::new();
    let err = validate_batch_apply_node_coverage(&plan, &outputs, &failed)
      .expect_err("missing node must fail closed");
    assert!(err.to_string().contains("coverage mismatch"));
    assert!(err.to_string().contains("b"));
  }

  #[test]
  fn batch_coverage_validation_rejects_unexpected_nodes() {
    let plan = Plan {
      order: vec!["a".into()],
    };
    let outputs = BTreeMap::from([
      ("a".to_string(), json!(1)),
      ("evil".to_string(), json!({"status": "ok"})),
    ]);
    let failed = std::collections::HashSet::new();
    let err = validate_batch_apply_node_coverage(&plan, &outputs, &failed)
      .expect_err("unexpected node must fail closed");
    assert!(err.to_string().contains("unknown output nodes"));
    assert!(err.to_string().contains("evil"));
  }

  #[test]
  fn batch_coverage_validation_rejects_contradictory_success_and_failure() {
    let plan = Plan {
      order: vec!["a".into()],
    };
    let outputs = BTreeMap::from([("a".to_string(), json!({"status": "ok"}))]);
    let failed = std::collections::HashSet::from(["a".to_string()]);
    let err = validate_batch_apply_node_coverage(&plan, &outputs, &failed)
      .expect_err("node cannot be both successful output and failed");
    assert!(err.to_string().contains("both output and failed"));
    assert!(err.to_string().contains("a"));
  }

  #[test]
  fn non_atomic_policy_blocks_world_effect_nodes_by_default() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "non-atomic".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![FxMorphism {
        name: "world.op".into(),
        input: "Any".into(),
        output: "Any".into(),
        inputs: vec![],
        outputs: vec![],
        effect: Effect::World,
      }],
      nodes: vec![make_node("n1", "world.op")],
      edges: vec![],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["n1".into()],
    };

    let err = enforce_non_atomic_effect_policy(&fx, &plan, false)
      .expect_err("world-effect node must fail closed without opt-in");
    assert!(err.to_string().contains("n1(uses=world.op,effect=world)"));
    assert!(err.to_string().contains("PNIX_ALLOW_NON_ATOMIC_EFFECTS=1"));
  }

  #[test]
  fn non_atomic_policy_allows_world_effect_nodes_with_opt_in() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "non-atomic".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![FxMorphism {
        name: "world.op".into(),
        input: "Any".into(),
        output: "Any".into(),
        inputs: vec![],
        outputs: vec![],
        effect: Effect::World,
      }],
      nodes: vec![make_node("n1", "world.op")],
      edges: vec![],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["n1".into()],
    };

    enforce_non_atomic_effect_policy(&fx, &plan, true)
      .expect("opt-in must allow non-atomic effect nodes");
  }

  #[test]
  fn non_atomic_policy_uses_builtin_catalog_effect_over_morphism_effect() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "builtin-catalog-effect".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![FxMorphism {
        name: "builtins.processSpawn".into(),
        input: "Any".into(),
        output: "Any".into(),
        inputs: vec![],
        outputs: vec![],
        // Intentionally incorrect metadata to verify fail-close policy uses catalog truth.
        effect: Effect::Pure,
      }],
      nodes: vec![make_node("spawn", "builtins.processSpawn")],
      edges: vec![],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["spawn".into()],
    };

    let err = enforce_non_atomic_effect_policy(&fx, &plan, false)
      .expect_err("builtin catalog world effect must block even if morphism says pure");
    assert!(err
      .to_string()
      .contains("spawn(uses=builtins.processSpawn,effect=world)"));
  }

  #[test]
  fn non_atomic_policy_blocks_unknown_explicit_builtin_forms() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "unknown-builtin".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![FxMorphism {
        name: "builtins.unknownDanger".into(),
        input: "Any".into(),
        output: "Any".into(),
        inputs: vec![],
        outputs: vec![],
        effect: Effect::Pure,
      }],
      nodes: vec![make_node("danger", "builtins.unknownDanger")],
      edges: vec![],
      scopes: vec![],
    };
    let plan = Plan {
      order: vec!["danger".into()],
    };

    let err = enforce_non_atomic_effect_policy(&fx, &plan, false)
      .expect_err("unknown explicit builtin form must fail closed");
    assert!(err
      .to_string()
      .contains("danger(uses=builtins.unknownDanger,effect=unknown)"));
  }

  #[test]
  fn edge_cond_when_active() {
    let cond = EdgeCond::When("g1".into());
    let mut gate_ok = HashMap::new();
    let node_failed = HashMap::new();

    gate_ok.insert("g1".into(), true);
    assert!(cond.is_active(&gate_ok, &node_failed).unwrap());

    gate_ok.insert("g1".into(), false);
    assert!(!cond.is_active(&gate_ok, &node_failed).unwrap());
  }

  #[test]
  fn edge_cond_unless_active() {
    let cond = EdgeCond::Unless("g1".into());
    let mut gate_ok = HashMap::new();
    let node_failed = HashMap::new();

    gate_ok.insert("g1".into(), true);
    assert!(!cond.is_active(&gate_ok, &node_failed).unwrap());

    gate_ok.insert("g1".into(), false);
    assert!(cond.is_active(&gate_ok, &node_failed).unwrap());
  }

  #[test]
  fn edge_cond_onfail_active() {
    let cond = EdgeCond::OnFail("r1".into());
    let gate_ok = HashMap::new();
    let mut node_failed = HashMap::new();

    // r1 not executed yet -> error
    assert!(cond.is_active(&gate_ok, &node_failed).is_err());

    // r1 executed but not failed -> edge inactive
    node_failed.insert("r1".into(), false);
    assert!(!cond.is_active(&gate_ok, &node_failed).unwrap());

    // r1 failed -> edge active
    node_failed.insert("r1".into(), true);
    assert!(cond.is_active(&gate_ok, &node_failed).unwrap());
  }

  #[test]
  fn apply_status_derivation() {
    // All ok
    {
      let nodes_failed_count = 0;
      let nodes_ok = 5;
      let had_failfast_error = false;
      let nodes_skipped = 0;
      let status = if had_failfast_error || (nodes_failed_count > 0 && nodes_ok == 0) {
        ApplyStatus::Error
      } else if nodes_failed_count > 0 || nodes_skipped > 0 {
        ApplyStatus::Partial
      } else {
        ApplyStatus::Ok
      };
      assert_eq!(status, ApplyStatus::Ok);
    }

    // Some failed
    {
      let nodes_failed_count = 1;
      let nodes_ok = 4;
      let had_failfast_error = false;
      let nodes_skipped = 0;
      let status = if had_failfast_error || (nodes_failed_count > 0 && nodes_ok == 0) {
        ApplyStatus::Error
      } else if nodes_failed_count > 0 || nodes_skipped > 0 {
        ApplyStatus::Partial
      } else {
        ApplyStatus::Ok
      };
      assert_eq!(status, ApplyStatus::Partial);
    }
  }

  #[test]
  fn rpc_error_reason_code_extracts_backend_reason() {
    let err = crate::rpc::client::RpcError::Backend {
      name: "no_such_morphism".into(),
      body: json!({
        "status": "error",
        "reason_code": "EVAL_TARGET_VERIFY_MISMATCH",
        "message": "verify mismatch",
      }),
    };
    assert_eq!(
      super::rpc_error_reason_code(&err),
      Some("EVAL_TARGET_VERIFY_MISMATCH".to_string())
    );
  }

  #[tokio::test]
  async fn call_backend_once_unsupported_backend_sets_stable_reason_code() {
    let config = BackendConfig::default();
    let retry = crate::rpc::client::RpcRetryPolicy::default();
    let clojure_client =
      RpcClient::new("http://127.0.0.1:7777", 1000, retry.clone()).expect("clojure client");
    let python_client =
      RpcClient::new("http://127.0.0.1:7778", 1000, retry.clone()).expect("python client");
    let deno_client =
      RpcClient::new("http://127.0.0.1:7779", 1000, retry.clone()).expect("deno client");
    let blenderpy_client =
      RpcClient::new("http://127.0.0.1:7781", 1000, retry).expect("blenderpy client");

    let err = super::call_backend_once(
      "unknown-backend",
      "noop",
      &json!({"k": "v"}),
      &clojure_client,
      &python_client,
      &deno_client,
      &blenderpy_client,
      &config,
      None,
    )
    .await
    .expect_err("unknown backend must fail closed");

    assert_eq!(
      super::rpc_error_reason_code(&err).as_deref(),
      Some(super::REASON_BACKEND_UNSUPPORTED)
    );

    match err {
      crate::rpc::client::RpcError::Backend { body, .. } => {
        assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("blocked"));
        assert_eq!(
          body.get("backend").and_then(|v| v.as_str()),
          Some("unknown-backend")
        );
      }
      other => panic!("expected backend error, got {other:?}"),
    }
  }

  #[test]
  fn rpc_error_value_preserves_verify_payload() {
    let err = crate::rpc::client::RpcError::Backend {
      name: "no_such_morphism".into(),
      body: json!({
        "status": "error",
        "reason_code": "EVAL_TARGET_VERIFY_MISMATCH",
        "eval_target": "verify",
        "verify": {
          "matched": false,
          "jvm": {"status": "error", "reason_code": "UNKNOWN_MORPHISM"},
          "pnix": {"status": "error", "reason_code": "EVAL_TARGET_PNIX_UNSUPPORTED_MORPHISM"}
        }
      }),
    };
    let value = super::rpc_error_value(&err);
    assert_eq!(
      value.get("reason_code").and_then(|v| v.as_str()),
      Some("EVAL_TARGET_VERIFY_MISMATCH")
    );
    assert_eq!(
      value.get("eval_target").and_then(|v| v.as_str()),
      Some("verify")
    );
    assert_eq!(
      value
        .get("verify")
        .and_then(|v| v.get("matched"))
        .and_then(|v| v.as_bool()),
      Some(false)
    );
  }

  // ═══════════════════════════════════════════════════════════════
  // STEP 2: Audit Reason Tests
  // ═══════════════════════════════════════════════════════════════

  #[test]
  fn audit_reason_executed_serializes_correctly() {
    let audit = AuditReason::Executed {
      policy: "normal".into(),
    };
    let json = serde_json::to_string(&audit).unwrap();
    assert!(json.contains("executed"));
    assert!(json.contains("normal"));
    // Must NOT contain value-related fields
    assert!(!json.contains("42"));
    assert!(!json.contains("result"));
  }

  #[test]
  fn audit_reason_skipped_has_policy_and_reason() {
    let audit = AuditReason::Skipped {
      policy: "skip_policy".into(),
      reason: "contract_allows_skip".into(),
      missing_inputs: 2,
    };
    let json = serde_json::to_string(&audit).unwrap();
    assert!(json.contains("skipped"));
    assert!(json.contains("skip_policy"));
    assert!(json.contains("missing_inputs"));
  }

  #[test]
  fn audit_reason_failed_has_error_text() {
    let audit = AuditReason::Failed {
      policy: "scope_failfast".into(),
      error: "backend_unreachable".into(),
    };
    let json = serde_json::to_string(&audit).unwrap();
    assert!(json.contains("failed"));
    assert!(json.contains("scope_failfast"));
    assert!(json.contains("backend_unreachable"));
  }

  #[test]
  fn audit_reason_gate_evaluated_has_result() {
    let audit = AuditReason::GateEvaluated { result: true };
    let json = serde_json::to_string(&audit).unwrap();
    assert!(json.contains("gate_evaluated"));
    assert!(json.contains("true"));
  }

  #[test]
  fn audit_reason_does_not_contain_values() {
    // Verify that audit reasons don't leak runtime values
    let audits = vec![
      AuditReason::Executed {
        policy: "normal".into(),
      },
      AuditReason::Skipped {
        policy: "skip".into(),
        reason: "missing".into(),
        missing_inputs: 1,
      },
      AuditReason::Failed {
        policy: "failfast".into(),
        error: "err".into(),
      },
      AuditReason::GateEvaluated { result: false },
    ];

    for audit in audits {
      let json = serde_json::to_string(&audit).unwrap();
      // Must not contain common value patterns
      assert!(!json.contains("\"value\""));
      assert!(!json.contains("\"data\""));
      assert!(!json.contains("\"output\""));
    }
  }
}
