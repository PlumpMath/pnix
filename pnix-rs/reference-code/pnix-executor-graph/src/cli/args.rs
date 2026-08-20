//! CLI 인자 파싱: 명령줄 인자 파싱 및 검증

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Result;
use pnix_core::contracts::ResourceLimits;

use super::print::{
  mode_label, print_agent_help, print_gate_absorb_help, print_gate_forward_help,
  print_gate_read_help, print_help,
};

/// 실행 모드: 실행기 동작 모드
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExecMode {
  Run,
  Interpret,
  Compile,
  Graph,
  LegacyEval,
  LegacyFrp,
  Ct,
  Llvm,
  Test, // Y11c: 테스트 러너
  Fmt,
  Lint,
}

impl ExecMode {
  pub(super) fn parse(value: &str) -> Option<Self> {
    match value {
      "run" => Some(Self::Run),
      "interpret" => Some(Self::Interpret),
      "compile" => Some(Self::Compile),
      "graph" => Some(Self::Graph),
      "legacy-eval" => Some(Self::LegacyEval),
      "legacy-frp" => Some(Self::LegacyFrp),
      "ct" => Some(Self::Ct),
      "llvm" => Some(Self::Llvm),
      "test" => Some(Self::Test), // Y11c
      "fmt" => Some(Self::Fmt),
      "lint" => Some(Self::Lint),
      _ => None,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AgentVerb {
  Ask,
  Plan,
  Patch,
  Verify,
  Rollback,
  Decide,
  Retention,
}

impl AgentVerb {
  pub(super) fn parse(value: &str) -> Option<Self> {
    match value {
      "ask" => Some(Self::Ask),
      "plan" => Some(Self::Plan),
      "patch" => Some(Self::Patch),
      "verify" => Some(Self::Verify),
      "rollback" => Some(Self::Rollback),
      "decide" => Some(Self::Decide),
      "retention" => Some(Self::Retention),
      _ => None,
    }
  }

  pub(super) fn as_str(self) -> &'static str {
    match self {
      Self::Ask => "ask",
      Self::Plan => "plan",
      Self::Patch => "patch",
      Self::Verify => "verify",
      Self::Rollback => "rollback",
      Self::Decide => "decide",
      Self::Retention => "retention",
    }
  }
}

/// 외부 호출자가 `Args` 전체를 만들 수 없는 환경에서도
/// `build_coding_agent_request_with_probe` 를 호출할 수 있도록 `agent_*` 필드만
/// carry 하는 input bundle. `Args` 의 agent_* slice 와 동일 schema.
///
/// pnix CLAUDE.md §16 (server subprocess 금지) 와 §15 (.px first) 의 server-side
/// invocation lane 을 여는 input shape. server (예: doghouse-http) 는 자기 cwd /
/// git probe 를 *envelope 으로 받아서* `CodingAgentRequestInput` 과 함께 host
/// adapter 에게 넘긴다. server 측 git subprocess / file system scan 호출 0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CodingAgentRequestInput {
  pub(super) agent_request: Option<String>,
  pub(super) agent_target_paths: Vec<PathBuf>,
  pub(super) agent_project_pack_roots: Vec<PathBuf>,
  pub(super) agent_history_pack_roots: Vec<PathBuf>,
  pub(super) agent_approved_commands: Vec<String>,
  pub(super) agent_forbidden_paths: Vec<PathBuf>,
  pub(super) agent_policy_bits: Vec<String>,
  pub(super) agent_current_plan_ref: Option<String>,
  pub(super) agent_rollback_handle_ref: Option<String>,
  pub(super) agent_last_verification_ref: Option<String>,
  pub(super) agent_promotion_boundary_ref: Option<String>,
  pub(super) agent_source_apply_artifact_ref: Option<String>,
  pub(super) agent_source_handoff_ref: Option<String>,
  pub(super) agent_promotion_boundary_join_ref: Option<String>,
  pub(super) agent_promotion_decision: Option<String>,
}

impl From<&Args> for CodingAgentRequestInput {
  fn from(args: &Args) -> Self {
    Self {
      agent_request: args.agent_request.clone(),
      agent_target_paths: args.agent_target_paths.clone(),
      agent_project_pack_roots: args.agent_project_pack_roots.clone(),
      agent_history_pack_roots: args.agent_history_pack_roots.clone(),
      agent_approved_commands: args.agent_approved_commands.clone(),
      agent_forbidden_paths: args.agent_forbidden_paths.clone(),
      agent_policy_bits: args.agent_policy_bits.clone(),
      agent_current_plan_ref: args.agent_current_plan_ref.clone(),
      agent_rollback_handle_ref: args.agent_rollback_handle_ref.clone(),
      agent_last_verification_ref: args.agent_last_verification_ref.clone(),
      agent_promotion_boundary_ref: args.agent_promotion_boundary_ref.clone(),
      agent_source_apply_artifact_ref: args.agent_source_apply_artifact_ref.clone(),
      agent_source_handoff_ref: args.agent_source_handoff_ref.clone(),
      agent_promotion_boundary_join_ref: args.agent_promotion_boundary_join_ref.clone(),
      agent_promotion_decision: args.agent_promotion_decision.clone(),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GateAbsorbVerb {
  Missing,
  Help,
  Url,
  Topic,
  Conversation,
  Events,
  Unknown(String),
}

impl GateAbsorbVerb {
  pub(super) fn parse(value: &str) -> Self {
    match value {
      "help" => Self::Help,
      "url" => Self::Url,
      "topic" => Self::Topic,
      "conversation" => Self::Conversation,
      "events" => Self::Events,
      other => Self::Unknown(other.to_string()),
    }
  }

  pub(super) fn as_str(&self) -> &str {
    match self {
      Self::Missing => "missing",
      Self::Help => "help",
      Self::Url => "url",
      Self::Topic => "topic",
      Self::Conversation => "conversation",
      Self::Events => "events",
      Self::Unknown(raw) => raw.as_str(),
    }
  }

  pub(super) fn expects_subject(&self) -> bool {
    matches!(
      self,
      Self::Url | Self::Topic | Self::Conversation | Self::Events
    )
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GateReadVerb {
  Missing,
  Help,
  Status,
  StateSinkContract,
  OntologyCoverage,
  MeaningBridges,
  SelfCapabilities,
  MetaProtocols,
  LiftRuleCoverage,
  StoreBudget,
  ArtifactRefRatio,
  StorageTelemetry,
  ProvenanceFloor,
  UnsupportedKindFloor,
  LineageFloor,
  RecentEvents,
  Candidates,
  BrainAnkhPolicy,
  BrainBundleContract,
  ValidateBrainBundle,
  CurriculumCurrentTarget,
  OntologyLookupRelated,
  RecipeMatchCurrent,
  QueryContext,
  Unknown(String),
}

impl GateReadVerb {
  pub(super) fn parse(value: &str) -> Self {
    match value {
      "help" => Self::Help,
      "status" => Self::Status,
      "state-sink-contract" => Self::StateSinkContract,
      "ontology-coverage" => Self::OntologyCoverage,
      "meaning-bridges" => Self::MeaningBridges,
      "self-capabilities" => Self::SelfCapabilities,
      "meta-protocols" => Self::MetaProtocols,
      "lift-rule-coverage" => Self::LiftRuleCoverage,
      "store-budget" => Self::StoreBudget,
      "artifact-ref-ratio" => Self::ArtifactRefRatio,
      "storage-telemetry" => Self::StorageTelemetry,
      "provenance-floor" => Self::ProvenanceFloor,
      "unsupported-kind-floor" => Self::UnsupportedKindFloor,
      "lineage-floor" => Self::LineageFloor,
      "recent-events" => Self::RecentEvents,
      "candidates" => Self::Candidates,
      "brain-ankh-policy" => Self::BrainAnkhPolicy,
      "brain-bundle-contract" => Self::BrainBundleContract,
      "validate-brain-bundle" => Self::ValidateBrainBundle,
      "curriculum-current-target" => Self::CurriculumCurrentTarget,
      "ontology-lookup-related" => Self::OntologyLookupRelated,
      "recipe-match-current" => Self::RecipeMatchCurrent,
      "query-context" => Self::QueryContext,
      other => Self::Unknown(other.to_string()),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GateForwardVerb {
  Help,
  Run,
  Unknown(String),
}

impl GateForwardVerb {
  pub(super) fn parse(value: &str) -> Self {
    match value {
      "help" => Self::Help,
      "run" => Self::Run,
      other => Self::Unknown(other.to_string()),
    }
  }
}

#[derive(Debug, Clone)]
pub(super) struct Args {
  pub(super) bin_name: String,
  pub(super) mode: ExecMode,
  pub(super) agent: Option<AgentVerb>,
  pub(super) gate_absorb: Option<GateAbsorbVerb>,
  pub(super) gate_forward: Option<GateForwardVerb>,
  pub(super) gate_read: Option<GateReadVerb>,
  pub(super) gate_absorb_subject: Option<String>,
  pub(super) gate_absorb_follow_related: Option<usize>,
  pub(super) gate_absorb_limit: Option<usize>,
  pub(super) gate_absorb_reset: bool,
  pub(super) gate_forward_limit: Option<usize>,
  pub(super) gate_forward_kind: Option<String>,
  pub(super) gate_forward_reset: bool,
  pub(super) gate_forward_url: Option<String>,
  pub(super) gate_read_context: Option<String>,
  pub(super) gate_read_predicate: Option<String>,
  pub(super) gate_read_topic: Option<String>,
  pub(super) gate_read_event_types: Vec<String>,
  pub(super) gate_read_tool_name: Option<String>,
  pub(super) gate_read_arg_predicates: Vec<String>,
  pub(super) gate_read_limit: Option<usize>,
  pub(super) gate_read_min_confidence: Option<f64>,
  pub(super) gate_read_kind: Option<String>,
  pub(super) gate_read_path: Option<String>,
  pub(super) gate_read_proof_path: Option<String>,
  pub(super) gate_read_schema_path: Option<String>,
  pub(super) gate_read_expected_bundle_kind: Option<String>,
  pub(super) gate_read_expected_lobe_profile: Option<String>,
  pub(super) gate_read_expected_proof_kind: Option<String>,
  pub(super) agent_request: Option<String>,
  pub(super) agent_target_paths: Vec<PathBuf>,
  pub(super) agent_project_pack_roots: Vec<PathBuf>,
  pub(super) agent_history_pack_roots: Vec<PathBuf>,
  pub(super) agent_approved_commands: Vec<String>,
  pub(super) agent_forbidden_paths: Vec<PathBuf>,
  pub(super) agent_policy_bits: Vec<String>,
  pub(super) agent_current_plan_ref: Option<String>,
  pub(super) agent_rollback_handle_ref: Option<String>,
  pub(super) agent_last_verification_ref: Option<String>,
  pub(super) agent_promotion_boundary_ref: Option<String>,
  pub(super) agent_source_apply_artifact_ref: Option<String>,
  pub(super) agent_source_handoff_ref: Option<String>,
  pub(super) agent_promotion_boundary_join_ref: Option<String>,
  pub(super) agent_promotion_decision: Option<String>,
  pub(super) agent_candidate_patch: Option<PathBuf>,
  pub(super) agent_provider_feedback_request_ref: Option<String>,
  pub(super) agent_request_out: Option<PathBuf>,
  pub(super) agent_plan_out: Option<PathBuf>,
  pub(super) agent_patch_out: Option<PathBuf>,
  pub(super) agent_verify_out: Option<PathBuf>,
  pub(super) agent_rollback_out: Option<PathBuf>,
  pub(super) agent_decision_out: Option<PathBuf>,
  pub(super) engine: Option<String>,
  /// Output selector for `run --engine ir-eval` (node[.port])
  pub(super) result: Option<String>,
  pub(super) dist: Option<PathBuf>,
  pub(super) clojure_url: String,
  pub(super) python_url: String,
  pub(super) deno_url: String,
  pub(super) blenderpy_url: String,
  pub(super) supervisor_sock: Option<String>,
  pub(super) auto_ensure_backends: bool,
  pub(super) backend_specs: Option<PathBuf>,
  pub(super) replay_trace: Option<PathBuf>,
  pub(super) replay_mode: Option<String>,
  pub(super) replay_allow: Vec<String>,
  pub(super) invocation_id: Option<String>,
  /// Backend RPC timeout in milliseconds (graph apply only)
  pub(super) rpc_timeout_ms: u64,
  /// Backend RPC retry attempts (graph apply only)
  pub(super) rpc_retry_attempts: usize,
  /// Backend RPC retry backoff base in milliseconds (graph apply only)
  pub(super) rpc_retry_backoff_ms: u64,
  pub(super) max_nodes: usize,
  pub(super) max_edges: usize,
  pub(super) max_input_bytes: usize,
  pub(super) use_batch: bool,
  pub(super) source: Option<String>,
  pub(super) expr: Option<String>,
  pub(super) test_filter: Option<String>,
  pub(super) patch: Option<PathBuf>,
  pub(super) deterministic: bool,
  pub(super) strict_ct: bool,
  pub(super) inputs: HashMap<String, serde_json::Value>,
  pub(super) seed: Option<u64>,
  pub(super) now_ms: Option<i64>,
  pub(super) clock_step_ms: Option<i64>,
  pub(super) frp_dt: Option<f64>,
  pub(super) inputs_schema: bool,
  pub(super) list_modes: bool,
  pub(super) list_ir_eval_ops: bool,
  pub(super) version: bool,
  pub(super) dry_run: bool,
  pub(super) emit: bool,
  pub(super) emit_target: Option<String>,
  pub(super) binary: bool, // --binary: compile to binary (shortcut for --emit-target aot)
  pub(super) emit_out: Option<PathBuf>,
  pub(super) emit_manifest: Option<PathBuf>,
  pub(super) fmt_check: bool,
  pub(super) live: bool,
  pub(super) live_dir: Option<PathBuf>,
  /// Y15c: Output format (text or json)
  #[allow(dead_code)] // 향후 사용 예정
  pub(super) output_format: OutputFormat,
}

/// Y15c: Output format enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutputFormat {
  Text,
  Json,
}

impl Default for OutputFormat {
  fn default() -> Self {
    Self::Text
  }
}

fn is_known_flag(raw: &str) -> bool {
  matches!(
    raw,
    "--mode"
      | "--follow-related"
      | "--context"
      | "--lookup-context"
      | "--predicate"
      | "--lookup-predicate"
      | "--event-type"
      | "--event_type"
      | "--topic"
      | "--query-topic"
      | "--tool-name"
      | "--tool_name"
      | "--arg-predicate"
      | "--arg_predicate"
      | "--arg-predicates"
      | "--arg_predicates"
      | "--limit"
      | "--query-limit"
      | "--min-confidence"
      | "--min_confidence"
      | "--request"
      | "--target-path"
      | "--project-pack-root"
      | "--history-pack-root"
      | "--approved-command"
      | "--forbidden-path"
      | "--workspace-policy"
      | "--current-plan-ref"
      | "--rollback-handle-ref"
      | "--last-verification-ref"
      | "--promotion-boundary-ref"
      | "--source-apply-artifact-ref"
      | "--source-handoff-ref"
      | "--promotion-boundary-join-ref"
      | "--promotion-decision"
      | "--candidate-patch"
      | "--provider-feedback-request-ref"
      | "--agent-request-out"
      | "--agent-plan-out"
      | "--agent-patch-out"
      | "--agent-verify-out"
      | "--agent-rollback-out"
      | "--agent-decision-out"
      | "--run"
      | "--interpret"
      | "--compile"
      | "--legacy-eval"
      | "--legacy-frp"
      | "--ct"
      | "--llvm"
      | "--test"
      | "--engine"
      | "--result"
      | "--dist"
      | "--source"
      | "--expr"
      | "--filter"
      | "--patch"
      | "--inputs"
      | "--inputs-json"
      | "--input"
      | "--inputs-schema"
      | "--list-ir-eval-ops"
      | "--emit"
      | "--emit-target"
      | "--target"
      | "--emit-out"
      | "--emit-manifest"
      | "--live"
      | "--live-dir"
      | "--fmt"
      | "--lint"
      | "--check"
      | "--list-modes"
      | "--version"
      | "-V"
      | "--seed"
      | "--now"
      | "--clock-step"
      | "--dt"
      | "--clojure-url"
      | "--python-url"
      | "--deno-url"
      | "--blenderpy-url"
      | "--supervisor-sock"
      | "--auto-ensure-backends"
      | "--no-auto-ensure-backends"
      | "--backend-specs"
      | "--replay"
      | "--replay-mode"
      | "--replay-allow"
      | "--invocation-id"
      | "--rpc-timeout-ms"
      | "--rpc-retry-attempts"
      | "--rpc-retry-backoff-ms"
      | "--max-nodes"
      | "--max-edges"
      | "--max-input-bytes"
      | "--no-batch"
      | "--dry-run"
      | "--non-deterministic"
      | "--lenient-ct"
      | "--help"
      | "-h"
      | "--output-format"
  )
}

fn take_flag_value(args: &[String], i: &mut usize, flag: &'static str) -> Result<String> {
  *i += 1;
  if *i >= args.len() {
    anyhow::bail!("{} requires a value", flag);
  }
  let value = args[*i].clone();
  if is_known_flag(value.as_str()) {
    anyhow::bail!("{} requires a value", flag);
  }
  // LOW: 옵션 값 검증 누락 수정
  // 빈 문자열은 유효하지 않은 값으로 처리
  if value.is_empty() {
    anyhow::bail!("{} requires a non-empty value", flag);
  }
  Ok(value)
}

fn merge_inputs(
  inputs: &mut HashMap<String, serde_json::Value>,
  value: serde_json::Value,
  source: &str,
) -> Result<()> {
  let obj = value
    .as_object()
    .ok_or_else(|| anyhow::anyhow!("inputs from {} must be a JSON object", source))?;
  for (key, value) in obj {
    if inputs.insert(key.clone(), value.clone()).is_some() {
      eprintln!(
        "Warning: Duplicate input key '{}' from {}, previous value will be overwritten",
        key, source
      );
    }
  }
  Ok(())
}

pub(super) fn parse_input_pair(pair: &str) -> Result<(String, serde_json::Value)> {
  let (key, raw) = pair
    .split_once('=')
    .ok_or_else(|| anyhow::anyhow!("invalid --input '{}', expected key=value", pair))?;
  let key = key.trim();
  if key.is_empty() {
    return Err(anyhow::anyhow!(
      "invalid --input '{}', key cannot be empty",
      pair
    ));
  }
  let value = serde_json::from_str(raw).map_err(|err| {
    anyhow::anyhow!(
      "invalid --input '{}': value must be valid JSON (example: --input {}=\"value\"): {}",
      pair,
      key,
      err
    )
  })?;
  Ok((key.to_string(), value))
}

fn parse_u64_arg(raw: &str, flag: &str) -> Result<u64> {
  raw
    .parse::<u64>()
    .map_err(|err| anyhow::anyhow!("invalid {} '{}': {}", flag, raw, err))
}

fn parse_usize_arg(raw: &str, flag: &str) -> Result<usize> {
  raw
    .parse::<usize>()
    .map_err(|err| anyhow::anyhow!("invalid {} '{}': {}", flag, raw, err))
}

fn parse_i64_arg(raw: &str, flag: &str) -> Result<i64> {
  raw
    .parse::<i64>()
    .map_err(|err| anyhow::anyhow!("invalid {} '{}': {}", flag, raw, err))
}

fn parse_f64_arg(raw: &str, flag: &str) -> Result<f64> {
  raw
    .parse::<f64>()
    .map_err(|err| anyhow::anyhow!("invalid {} '{}': {}", flag, raw, err))
}

fn parse_bool_arg(raw: &str, flag: &str) -> Result<bool> {
  match raw.trim().to_ascii_lowercase().as_str() {
    "1" | "true" | "yes" | "on" => Ok(true),
    "0" | "false" | "no" | "off" => Ok(false),
    other => anyhow::bail!(
      "invalid {} '{}': expected true|false (or 1|0/yes|no/on|off)",
      flag,
      other
    ),
  }
}

fn bump_input_bytes(current: &mut usize, added: usize, max: usize) -> Result<()> {
  *current = current.saturating_add(added);
  if *current > max {
    anyhow::bail!(
      "inputs exceed max-input-bytes limit: bytes={} > max={}",
      *current,
      max
    );
  }
  Ok(())
}

fn read_file_limited(path: &str, max_bytes: usize) -> Result<Vec<u8>> {
  let file =
    std::fs::File::open(path).map_err(|err| anyhow::anyhow!("failed to read {}: {}", path, err))?;
  let mut buf = Vec::new();
  let mut reader = file.take((max_bytes as u64) + 1);
  reader
    .read_to_end(&mut buf)
    .map_err(|err| anyhow::anyhow!("failed to read {}: {}", path, err))?;
  if buf.len() > max_bytes {
    anyhow::bail!(
      "inputs file too large: {} bytes (limit {})",
      buf.len(),
      max_bytes
    );
  }
  Ok(buf)
}

pub(super) fn parse_args_vec(args: Vec<String>) -> Result<Args> {
  let bin_name = args
    .first()
    .and_then(|raw| Path::new(raw).file_name().and_then(|s| s.to_str()))
    .unwrap_or("pnix")
    .to_string();

  // Default user-facing mode: `run` (graph apply on dist).
  let mut mode = ExecMode::Run;
  let mut engine: Option<String> = None;
  let mut result: Option<String> = None;
  let mut dist: Option<String> = None;
  let mut clojure_url = "http://localhost:7777".to_string();
  let mut python_url = "http://localhost:7778".to_string();
  let mut deno_url = "http://localhost:7779".to_string();
  let mut blenderpy_url = "http://localhost:7781".to_string();
  let mut supervisor_sock: Option<String> = None;
  let mut auto_ensure_backends = true;
  let mut auto_ensure_backends_set = false;
  let mut backend_specs: Option<String> = None;
  let mut replay_trace: Option<String> = None;
  let mut replay_mode: Option<String> = None;
  let mut replay_allow: Vec<String> = Vec::new();
  let mut invocation_id: Option<String> = None;
  let mut clojure_url_set = false;
  let mut python_url_set = false;
  let mut deno_url_set = false;
  let mut blenderpy_url_set = false;
  let mut rpc_timeout_ms: u64 = 30_000;
  let mut rpc_timeout_ms_set = false;
  let mut rpc_retry_attempts: usize = 3;
  let mut rpc_retry_attempts_set = false;
  let mut rpc_retry_backoff_ms: u64 = 100;
  let mut rpc_retry_backoff_ms_set = false;
  let default_limits = ResourceLimits::default();
  let mut max_nodes = default_limits.max_nodes;
  let mut max_edges = default_limits.max_edges;
  let mut max_input_bytes = default_limits.max_input_bytes;
  let mut use_batch = true;
  let mut source: Option<String> = None;
  let mut expr: Option<String> = None;
  let mut test_filter: Option<String> = None;
  let mut deterministic = true;
  let mut strict_ct = true;
  let mut inputs_file: Option<String> = None;
  let mut inputs_json: Option<String> = None;
  let mut input_pairs: Vec<String> = Vec::new();
  let mut patch: Option<String> = None;
  let mut seed: Option<u64> = None;
  let mut now_ms: Option<i64> = None;
  let mut clock_step_ms: Option<i64> = None;
  let mut frp_dt: Option<f64> = None;
  let mut inputs_schema = false;
  let mut list_modes = false;
  let mut list_ir_eval_ops = false;
  let mut version = false;
  let mut dry_run = false;
  let mut emit = false;
  let mut emit_target: Option<String> = None;
  let mut aot_target: Option<String> = None;
  let mut binary = false;
  let mut emit_out: Option<String> = None;
  let mut emit_manifest: Option<String> = None;
  let mut output_format = OutputFormat::Text;
  let mut fmt_check = false;
  let mut live = false;
  let mut live_dir: Option<String> = None;
  let mut agent: Option<AgentVerb> = None;
  let mut gate_absorb: Option<GateAbsorbVerb> = None;
  let mut gate_forward: Option<GateForwardVerb> = None;
  let mut gate_read: Option<GateReadVerb> = None;
  let mut gate_absorb_subject: Option<String> = None;
  let mut gate_absorb_follow_related: Option<usize> = None;
  let mut gate_absorb_limit: Option<usize> = None;
  let mut gate_absorb_reset = false;
  let mut gate_forward_limit: Option<usize> = None;
  let mut gate_forward_kind: Option<String> = None;
  let mut gate_forward_reset = false;
  let mut gate_forward_url: Option<String> = None;
  let mut gate_read_context: Option<String> = None;
  let mut gate_read_predicate: Option<String> = None;
  let mut gate_read_topic: Option<String> = None;
  let mut gate_read_event_types: Vec<String> = Vec::new();
  let mut gate_read_tool_name: Option<String> = None;
  let mut gate_read_arg_predicates: Vec<String> = Vec::new();
  let mut gate_read_limit: Option<usize> = None;
  let mut gate_read_min_confidence: Option<f64> = None;
  let mut gate_read_kind: Option<String> = None;
  let mut gate_read_path: Option<String> = None;
  let mut gate_read_proof_path: Option<String> = None;
  let mut gate_read_schema_path: Option<String> = None;
  let mut gate_read_expected_bundle_kind: Option<String> = None;
  let mut gate_read_expected_lobe_profile: Option<String> = None;
  let mut gate_read_expected_proof_kind: Option<String> = None;
  let mut agent_request: Option<String> = None;
  let mut agent_target_paths: Vec<String> = Vec::new();
  let mut agent_project_pack_roots: Vec<String> = Vec::new();
  let mut agent_history_pack_roots: Vec<String> = Vec::new();
  let mut agent_approved_commands: Vec<String> = Vec::new();
  let mut agent_forbidden_paths: Vec<String> = Vec::new();
  let mut agent_policy_bits: Vec<String> = Vec::new();
  let mut agent_current_plan_ref: Option<String> = None;
  let mut agent_rollback_handle_ref: Option<String> = None;
  let mut agent_last_verification_ref: Option<String> = None;
  let mut agent_promotion_boundary_ref: Option<String> = None;
  let mut agent_source_apply_artifact_ref: Option<String> = None;
  let mut agent_source_handoff_ref: Option<String> = None;
  let mut agent_promotion_boundary_join_ref: Option<String> = None;
  let mut agent_promotion_decision: Option<String> = None;
  let mut agent_candidate_patch: Option<String> = None;
  let mut agent_provider_feedback_request_ref: Option<String> = None;
  let mut agent_request_out: Option<String> = None;
  let mut agent_plan_out: Option<String> = None;
  let mut agent_patch_out: Option<String> = None;
  let mut agent_verify_out: Option<String> = None;
  let mut agent_rollback_out: Option<String> = None;
  let mut agent_decision_out: Option<String> = None;

  let mut i = 1;
  if let Some(cmd) = args.get(1) {
    if !cmd.starts_with('-') {
      match cmd.as_str() {
        "fmt" => {
          mode = ExecMode::Fmt;
          i = 2;
        }
        "lint" => {
          mode = ExecMode::Lint;
          i = 2;
        }
        "coding-agent" | "agent" => {
          i = 2;
          match args.get(i).map(|s| s.as_str()) {
            None => {
              print_agent_help(bin_name.as_str(), None);
              std::process::exit(0);
            }
            Some("help") | Some("--help") | Some("-h") => {
              print_agent_help(bin_name.as_str(), None);
              std::process::exit(0);
            }
            Some(raw) if !raw.starts_with('-') => {
              let verb = AgentVerb::parse(raw)
                .ok_or_else(|| anyhow::anyhow!("unknown coding-agent verb '{}'", raw))?;
              agent = Some(verb);
              i = 3;
              if matches!(args.get(i).map(|s| s.as_str()), Some("--help" | "-h")) {
                print_agent_help(bin_name.as_str(), Some(verb));
                std::process::exit(0);
              }
              if matches!(args.get(i), Some(value) if !value.starts_with('-')) {
                agent_request = Some(args[i].clone());
                i += 1;
              }
            }
            Some(raw) => {
              anyhow::bail!("unexpected coding-agent argument '{}'", raw);
            }
          }
        }
        "gate-absorb" => {
          i = 2;
          match args.get(i).map(|s| s.as_str()) {
            None => {
              gate_absorb = Some(GateAbsorbVerb::Missing);
            }
            Some("help") | Some("--help") | Some("-h") => {
              print_gate_absorb_help(bin_name.as_str(), None);
              std::process::exit(0);
            }
            Some(raw) if !raw.starts_with('-') => {
              let verb = GateAbsorbVerb::parse(raw);
              if matches!(verb, GateAbsorbVerb::Help) {
                print_gate_absorb_help(bin_name.as_str(), None);
                std::process::exit(0);
              }
              gate_absorb = Some(verb.clone());
              i = 3;
              if matches!(args.get(i).map(|s| s.as_str()), Some("--help" | "-h")) {
                print_gate_absorb_help(bin_name.as_str(), Some(&verb));
                std::process::exit(0);
              }
              if verb.expects_subject()
                && matches!(args.get(i), Some(value) if !value.starts_with('-'))
              {
                gate_absorb_subject = Some(args[i].clone());
                i += 1;
              }
            }
            Some(raw) => {
              anyhow::bail!("unexpected gate-absorb argument '{}'", raw);
            }
          }
        }
        "gate-read" => {
          i = 2;
          match args.get(i).map(|s| s.as_str()) {
            None => {
              gate_read = Some(GateReadVerb::Missing);
            }
            Some("help") | Some("--help") | Some("-h") => {
              print_gate_read_help(bin_name.as_str(), None);
              std::process::exit(0);
            }
            Some(raw) if !raw.starts_with('-') => {
              let verb = GateReadVerb::parse(raw);
              if matches!(verb, GateReadVerb::Help) {
                print_gate_read_help(bin_name.as_str(), None);
                std::process::exit(0);
              }
              gate_read = Some(verb.clone());
              i = 3;
              if matches!(args.get(i).map(|s| s.as_str()), Some("--help" | "-h")) {
                print_gate_read_help(bin_name.as_str(), Some(&verb));
                std::process::exit(0);
              }
            }
            Some(raw) => {
              anyhow::bail!("unexpected gate-read argument '{}'", raw);
            }
          }
        }
        "gate-forward" => {
          i = 2;
          match args.get(i).map(|s| s.as_str()) {
            None => {
              gate_forward = Some(GateForwardVerb::Run);
            }
            Some("help") | Some("--help") | Some("-h") => {
              print_gate_forward_help(bin_name.as_str(), None);
              std::process::exit(0);
            }
            Some(raw) if !raw.starts_with('-') => {
              let verb = GateForwardVerb::parse(raw);
              if matches!(verb, GateForwardVerb::Help) {
                print_gate_forward_help(bin_name.as_str(), None);
                std::process::exit(0);
              }
              gate_forward = Some(verb.clone());
              i = 3;
              if matches!(args.get(i).map(|s| s.as_str()), Some("--help" | "-h")) {
                print_gate_forward_help(bin_name.as_str(), Some(&verb));
                std::process::exit(0);
              }
            }
            Some(_) => {
              gate_forward = Some(GateForwardVerb::Run);
            }
          }
        }
        _ => {}
      }
    }
  }
  while i < args.len() {
    match args[i].as_str() {
      "--follow-related" => {
        let raw = take_flag_value(&args, &mut i, "--follow-related")?;
        gate_absorb_follow_related = Some(parse_usize_arg(&raw, "--follow-related")?);
      }
      "--context" | "--lookup-context" => {
        gate_read_context = Some(take_flag_value(&args, &mut i, "--context")?);
      }
      "--predicate" | "--lookup-predicate" => {
        gate_read_predicate = Some(take_flag_value(&args, &mut i, "--predicate")?);
      }
      "--event-type" | "--event_type" => {
        gate_read_event_types.push(take_flag_value(&args, &mut i, "--event-type")?);
      }
      "--topic" | "--query-topic" => {
        gate_read_topic = Some(take_flag_value(&args, &mut i, "--topic")?);
      }
      "--tool-name" | "--tool_name" => {
        gate_read_tool_name = Some(take_flag_value(&args, &mut i, "--tool-name")?);
      }
      "--arg-predicate" | "--arg_predicate" | "--arg-predicates" | "--arg_predicates" => {
        gate_read_arg_predicates.push(take_flag_value(&args, &mut i, "--arg-predicate")?);
      }
      "--limit" | "--query-limit" => {
        let raw = take_flag_value(&args, &mut i, "--limit")?;
        if gate_forward.is_some() {
          gate_forward_limit = Some(parse_usize_arg(&raw, "--limit")?);
        } else if gate_absorb == Some(GateAbsorbVerb::Events) {
          gate_absorb_limit = Some(parse_usize_arg(&raw, "--limit")?);
        } else {
          gate_read_limit = Some(parse_usize_arg(&raw, "--limit")?);
        }
      }
      "--kind" => {
        let value = take_flag_value(&args, &mut i, "--kind")?;
        if gate_read == Some(GateReadVerb::Candidates) {
          gate_read_kind = Some(value);
        } else {
          gate_forward_kind = Some(value);
        }
      }
      "--path" => {
        gate_read_path = Some(take_flag_value(&args, &mut i, "--path")?);
      }
      "--proof-path" => {
        gate_read_proof_path = Some(take_flag_value(&args, &mut i, "--proof-path")?);
      }
      "--schema-path" => {
        gate_read_schema_path = Some(take_flag_value(&args, &mut i, "--schema-path")?);
      }
      "--expected-bundle-kind" => {
        gate_read_expected_bundle_kind =
          Some(take_flag_value(&args, &mut i, "--expected-bundle-kind")?);
      }
      "--expected-lobe-profile" => {
        gate_read_expected_lobe_profile =
          Some(take_flag_value(&args, &mut i, "--expected-lobe-profile")?);
      }
      "--expected-proof-kind" => {
        gate_read_expected_proof_kind =
          Some(take_flag_value(&args, &mut i, "--expected-proof-kind")?);
      }
      "--reset" => {
        if gate_absorb == Some(GateAbsorbVerb::Events) {
          gate_absorb_reset = true;
        } else {
          gate_forward_reset = true;
        }
      }
      "--url" => {
        gate_forward_url = Some(take_flag_value(&args, &mut i, "--url")?);
      }
      "--min-confidence" | "--min_confidence" => {
        let raw = take_flag_value(&args, &mut i, "--min-confidence")?;
        gate_read_min_confidence = Some(parse_f64_arg(&raw, "--min-confidence")?);
      }
      "--mode" => {
        let raw = take_flag_value(&args, &mut i, "--mode")?;
        mode = ExecMode::parse(&raw).ok_or_else(|| anyhow::anyhow!("unknown mode '{}'", raw))?;
      }
      "--request" => {
        agent_request = Some(take_flag_value(&args, &mut i, "--request")?);
      }
      "--target-path" => {
        agent_target_paths.push(take_flag_value(&args, &mut i, "--target-path")?);
      }
      "--project-pack-root" => {
        agent_project_pack_roots.push(take_flag_value(&args, &mut i, "--project-pack-root")?);
      }
      "--history-pack-root" => {
        agent_history_pack_roots.push(take_flag_value(&args, &mut i, "--history-pack-root")?);
      }
      "--approved-command" => {
        agent_approved_commands.push(take_flag_value(&args, &mut i, "--approved-command")?);
      }
      "--forbidden-path" => {
        agent_forbidden_paths.push(take_flag_value(&args, &mut i, "--forbidden-path")?);
      }
      "--workspace-policy" => {
        agent_policy_bits.push(take_flag_value(&args, &mut i, "--workspace-policy")?);
      }
      "--current-plan-ref" => {
        agent_current_plan_ref = Some(take_flag_value(&args, &mut i, "--current-plan-ref")?);
      }
      "--rollback-handle-ref" => {
        agent_rollback_handle_ref = Some(take_flag_value(&args, &mut i, "--rollback-handle-ref")?);
      }
      "--last-verification-ref" => {
        agent_last_verification_ref =
          Some(take_flag_value(&args, &mut i, "--last-verification-ref")?);
      }
      "--promotion-boundary-ref" => {
        agent_promotion_boundary_ref =
          Some(take_flag_value(&args, &mut i, "--promotion-boundary-ref")?);
      }
      "--source-apply-artifact-ref" => {
        agent_source_apply_artifact_ref = Some(take_flag_value(
          &args,
          &mut i,
          "--source-apply-artifact-ref",
        )?);
      }
      "--source-handoff-ref" => {
        agent_source_handoff_ref = Some(take_flag_value(&args, &mut i, "--source-handoff-ref")?);
      }
      "--promotion-boundary-join-ref" => {
        agent_promotion_boundary_join_ref = Some(take_flag_value(
          &args,
          &mut i,
          "--promotion-boundary-join-ref",
        )?);
      }
      "--promotion-decision" => {
        agent_promotion_decision = Some(take_flag_value(&args, &mut i, "--promotion-decision")?);
      }
      "--candidate-patch" => {
        agent_candidate_patch = Some(take_flag_value(&args, &mut i, "--candidate-patch")?);
      }
      "--provider-feedback-request-ref" => {
        agent_provider_feedback_request_ref = Some(take_flag_value(
          &args,
          &mut i,
          "--provider-feedback-request-ref",
        )?);
      }
      "--agent-request-out" => {
        agent_request_out = Some(take_flag_value(&args, &mut i, "--agent-request-out")?);
      }
      "--agent-plan-out" => {
        agent_plan_out = Some(take_flag_value(&args, &mut i, "--agent-plan-out")?);
      }
      "--agent-patch-out" => {
        agent_patch_out = Some(take_flag_value(&args, &mut i, "--agent-patch-out")?);
      }
      "--agent-verify-out" => {
        agent_verify_out = Some(take_flag_value(&args, &mut i, "--agent-verify-out")?);
      }
      "--agent-rollback-out" => {
        agent_rollback_out = Some(take_flag_value(&args, &mut i, "--agent-rollback-out")?);
      }
      "--agent-decision-out" => {
        agent_decision_out = Some(take_flag_value(&args, &mut i, "--agent-decision-out")?);
      }
      "--run" => {
        mode = ExecMode::Run;
      }
      "--interpret" => {
        mode = ExecMode::Interpret;
      }
      "--legacy-eval" => {
        mode = ExecMode::LegacyEval;
      }
      "--compile" => {
        mode = ExecMode::Compile;
      }
      "--legacy-frp" => {
        mode = ExecMode::LegacyFrp;
      }
      "--ct" => {
        mode = ExecMode::Ct;
      }
      "--llvm" => {
        mode = ExecMode::Llvm;
      }
      "--test" => {
        mode = ExecMode::Test;
      }
      "--fmt" => {
        mode = ExecMode::Fmt;
      }
      "--lint" => {
        mode = ExecMode::Lint;
      }
      "--engine" => {
        engine = Some(take_flag_value(&args, &mut i, "--engine")?);
      }
      "--result" => {
        result = Some(take_flag_value(&args, &mut i, "--result")?);
      }
      "--dist" => {
        dist = Some(take_flag_value(&args, &mut i, "--dist")?);
      }
      "--source" => {
        source = Some(take_flag_value(&args, &mut i, "--source")?);
      }
      "--expr" => {
        expr = Some(take_flag_value(&args, &mut i, "--expr")?);
      }
      "--filter" => {
        test_filter = Some(take_flag_value(&args, &mut i, "--filter")?);
      }
      "--patch" => {
        patch = Some(take_flag_value(&args, &mut i, "--patch")?);
      }
      "--inputs" => {
        inputs_file = Some(take_flag_value(&args, &mut i, "--inputs")?);
      }
      "--inputs-json" => {
        inputs_json = Some(take_flag_value(&args, &mut i, "--inputs-json")?);
      }
      "--input" => {
        input_pairs.push(take_flag_value(&args, &mut i, "--input")?);
      }
      "--inputs-schema" => {
        inputs_schema = true;
      }
      "--emit" => {
        emit = true;
      }
      "--emit-target" => {
        emit_target = Some(take_flag_value(&args, &mut i, "--emit-target")?);
      }
      "--target" => {
        aot_target = Some(take_flag_value(&args, &mut i, "--target")?);
      }
      "--binary" => {
        binary = true;
      }
      "--emit-out" => {
        emit_out = Some(take_flag_value(&args, &mut i, "--emit-out")?);
      }
      "--emit-manifest" => {
        emit_manifest = Some(take_flag_value(&args, &mut i, "--emit-manifest")?);
      }
      "--live" => {
        live = true;
      }
      "--live-dir" => {
        live = true;
        live_dir = Some(take_flag_value(&args, &mut i, "--live-dir")?);
      }
      "--check" => {
        fmt_check = true;
      }
      "--output-format" => {
        let raw = take_flag_value(&args, &mut i, "--output-format")?;
        output_format = match raw.as_str() {
          "text" => OutputFormat::Text,
          "json" => OutputFormat::Json,
          _ => anyhow::bail!(
            "invalid --output-format '{}', expected 'text' or 'json'",
            raw
          ),
        };
      }
      "--list-modes" => {
        list_modes = true;
      }
      "--list-ir-eval-ops" => {
        list_ir_eval_ops = true;
      }
      "--version" | "-V" => {
        version = true;
      }
      "--seed" => {
        let raw = take_flag_value(&args, &mut i, "--seed")?;
        seed = Some(parse_u64_arg(&raw, "--seed")?);
      }
      "--now" => {
        let raw = take_flag_value(&args, &mut i, "--now")?;
        let ts = parse_i64_arg(&raw, "--now")?;
        if ts < 0 {
          anyhow::bail!(
            "--now must be non-negative (milliseconds since epoch), got: {}",
            ts
          );
        }
        // 경고: 비현실적으로 큰 값 (100년 후 = 3,155,760,000,000ms)
        if ts > 3_155_760_000_000 {
          eprintln!(
            "Warning: --now value is very large ({}), may be incorrect",
            ts
          );
        }
        now_ms = Some(ts);
      }
      "--clock-step" => {
        let raw = take_flag_value(&args, &mut i, "--clock-step")?;
        let step = parse_i64_arg(&raw, "--clock-step")?;
        if step < 0 {
          anyhow::bail!("--clock-step must be non-negative, got: {}", step);
        }
        clock_step_ms = Some(step);
      }
      "--dt" => {
        let raw = take_flag_value(&args, &mut i, "--dt")?;
        let dt = parse_f64_arg(&raw, "--dt")?;
        if dt < 0.0 {
          anyhow::bail!("--dt must be non-negative, got: {}", dt);
        }
        frp_dt = Some(dt);
      }
      "--clojure-url" => {
        clojure_url_set = true;
        clojure_url = take_flag_value(&args, &mut i, "--clojure-url")?;
      }
      "--python-url" => {
        python_url_set = true;
        python_url = take_flag_value(&args, &mut i, "--python-url")?;
      }
      "--deno-url" => {
        deno_url_set = true;
        deno_url = take_flag_value(&args, &mut i, "--deno-url")?;
      }
      "--blenderpy-url" => {
        blenderpy_url_set = true;
        blenderpy_url = take_flag_value(&args, &mut i, "--blenderpy-url")?;
      }
      "--supervisor-sock" => {
        supervisor_sock = Some(take_flag_value(&args, &mut i, "--supervisor-sock")?);
      }
      "--auto-ensure-backends" => {
        auto_ensure_backends_set = true;
        let raw = take_flag_value(&args, &mut i, "--auto-ensure-backends")?;
        auto_ensure_backends = parse_bool_arg(&raw, "--auto-ensure-backends")?;
      }
      "--no-auto-ensure-backends" => {
        auto_ensure_backends_set = true;
        auto_ensure_backends = false;
      }
      "--backend-specs" => {
        backend_specs = Some(take_flag_value(&args, &mut i, "--backend-specs")?);
      }
      "--replay" => {
        replay_trace = Some(take_flag_value(&args, &mut i, "--replay")?);
      }
      "--replay-mode" => {
        replay_mode = Some(take_flag_value(&args, &mut i, "--replay-mode")?);
      }
      "--replay-allow" => {
        replay_allow.push(take_flag_value(&args, &mut i, "--replay-allow")?);
      }
      "--invocation-id" => {
        invocation_id = Some(take_flag_value(&args, &mut i, "--invocation-id")?);
      }
      "--rpc-timeout-ms" => {
        rpc_timeout_ms_set = true;
        let raw = take_flag_value(&args, &mut i, "--rpc-timeout-ms")?;
        rpc_timeout_ms = parse_u64_arg(&raw, "--rpc-timeout-ms")?;
      }
      "--rpc-retry-attempts" => {
        rpc_retry_attempts_set = true;
        let raw = take_flag_value(&args, &mut i, "--rpc-retry-attempts")?;
        rpc_retry_attempts = parse_usize_arg(&raw, "--rpc-retry-attempts")?;
      }
      "--rpc-retry-backoff-ms" => {
        rpc_retry_backoff_ms_set = true;
        let raw = take_flag_value(&args, &mut i, "--rpc-retry-backoff-ms")?;
        rpc_retry_backoff_ms = parse_u64_arg(&raw, "--rpc-retry-backoff-ms")?;
      }
      "--max-nodes" => {
        let raw = take_flag_value(&args, &mut i, "--max-nodes")?;
        max_nodes = parse_usize_arg(&raw, "--max-nodes")?;
      }
      "--max-edges" => {
        let raw = take_flag_value(&args, &mut i, "--max-edges")?;
        max_edges = parse_usize_arg(&raw, "--max-edges")?;
      }
      "--max-input-bytes" => {
        let raw = take_flag_value(&args, &mut i, "--max-input-bytes")?;
        max_input_bytes = parse_usize_arg(&raw, "--max-input-bytes")?;
      }
      "--no-batch" => {
        use_batch = false;
      }
      "--dry-run" => {
        dry_run = true;
      }
      "--non-deterministic" => {
        deterministic = false;
      }
      "--lenient-ct" => {
        strict_ct = false;
      }
      "--help" | "-h" => {
        print_help(bin_name.as_str());
        std::process::exit(0);
      }
      flag if flag.starts_with('-') => {
        anyhow::bail!("unknown flag '{}'", flag);
      }
      arg => {
        anyhow::bail!("unexpected argument '{}'", arg);
      }
    }
    i += 1;
  }

  if max_nodes == 0 {
    anyhow::bail!("--max-nodes must be >= 1");
  }
  if max_edges == 0 {
    anyhow::bail!("--max-edges must be >= 1");
  }
  if max_input_bytes == 0 {
    anyhow::bail!("--max-input-bytes must be >= 1");
  }

  if agent_plan_out.is_some() && agent != Some(AgentVerb::Plan) {
    anyhow::bail!("--agent-plan-out is only supported for `pnix coding-agent plan`");
  }
  if agent_patch_out.is_some() && agent != Some(AgentVerb::Patch) {
    anyhow::bail!("--agent-patch-out is only supported for `pnix coding-agent patch`");
  }
  if agent_verify_out.is_some() && agent != Some(AgentVerb::Verify) {
    anyhow::bail!("--agent-verify-out is only supported for `pnix coding-agent verify`");
  }
  if agent_rollback_out.is_some() && agent != Some(AgentVerb::Rollback) {
    anyhow::bail!("--agent-rollback-out is only supported for `pnix coding-agent rollback`");
  }
  if agent_candidate_patch.is_some() && agent != Some(AgentVerb::Patch) {
    anyhow::bail!("--candidate-patch is only supported for `pnix coding-agent patch`");
  }
  if agent_provider_feedback_request_ref.is_some() && agent != Some(AgentVerb::Patch) {
    anyhow::bail!(
      "--provider-feedback-request-ref is only supported for `pnix coding-agent patch`"
    );
  }
  if agent_provider_feedback_request_ref.is_some() && agent_candidate_patch.is_none() {
    anyhow::bail!("--provider-feedback-request-ref requires --candidate-patch");
  }
  if agent_promotion_boundary_ref.is_some() && agent != Some(AgentVerb::Verify) {
    anyhow::bail!("--promotion-boundary-ref is only supported for `pnix coding-agent verify`");
  }
  if agent_source_apply_artifact_ref.is_some() && agent != Some(AgentVerb::Verify) {
    anyhow::bail!("--source-apply-artifact-ref is only supported for `pnix coding-agent verify`");
  }
  if agent_source_handoff_ref.is_some() && agent != Some(AgentVerb::Verify) {
    anyhow::bail!("--source-handoff-ref is only supported for `pnix coding-agent verify`");
  }
  if agent_promotion_boundary_ref.is_some() && agent_source_apply_artifact_ref.is_none() {
    anyhow::bail!("--promotion-boundary-ref requires --source-apply-artifact-ref");
  }
  if agent_source_apply_artifact_ref.is_some() && agent_promotion_boundary_ref.is_none() {
    anyhow::bail!("--source-apply-artifact-ref requires --promotion-boundary-ref");
  }
  if agent_source_handoff_ref.is_some() && agent_promotion_boundary_ref.is_none() {
    anyhow::bail!("--source-handoff-ref requires --promotion-boundary-ref");
  }
  if agent_promotion_boundary_join_ref.is_some() && agent != Some(AgentVerb::Decide) {
    anyhow::bail!("--promotion-boundary-join-ref is only supported for `pnix coding-agent decide`");
  }
  if agent_promotion_decision.is_some() && agent != Some(AgentVerb::Decide) {
    anyhow::bail!("--promotion-decision is only supported for `pnix coding-agent decide`");
  }
  if agent_decision_out.is_some() && agent != Some(AgentVerb::Decide) {
    anyhow::bail!("--agent-decision-out is only supported for `pnix coding-agent decide`");
  }
  if agent == Some(AgentVerb::Decide) && agent_promotion_boundary_join_ref.is_none() {
    anyhow::bail!("pnix coding-agent decide requires --promotion-boundary-join-ref");
  }
  if agent == Some(AgentVerb::Decide) && agent_promotion_decision.is_none() {
    anyhow::bail!("pnix coding-agent decide requires --promotion-decision");
  }
  if let Some(decision) = agent_promotion_decision.as_deref() {
    if !matches!(decision, "accepted" | "rejected" | "held") {
      anyhow::bail!("--promotion-decision must be one of accepted|rejected|held");
    }
  }
  let retention_agent_flags_specified = agent_request.is_some()
    || !agent_target_paths.is_empty()
    || !agent_project_pack_roots.is_empty()
    || !agent_history_pack_roots.is_empty()
    || !agent_approved_commands.is_empty()
    || !agent_forbidden_paths.is_empty()
    || !agent_policy_bits.is_empty()
    || agent_current_plan_ref.is_some()
    || agent_rollback_handle_ref.is_some()
    || agent_last_verification_ref.is_some()
    || agent_promotion_boundary_ref.is_some()
    || agent_source_apply_artifact_ref.is_some()
    || agent_source_handoff_ref.is_some()
    || agent_promotion_boundary_join_ref.is_some()
    || agent_promotion_decision.is_some()
    || agent_candidate_patch.is_some()
    || agent_provider_feedback_request_ref.is_some()
    || agent_request_out.is_some()
    || agent_plan_out.is_some()
    || agent_patch_out.is_some()
    || agent_verify_out.is_some()
    || agent_rollback_out.is_some()
    || agent_decision_out.is_some();
  if agent == Some(AgentVerb::Retention) && retention_agent_flags_specified {
    anyhow::bail!(
      "request/workspace artifact flags are not supported for `pnix coding-agent retention`; it reads DOGHOUSE_STORE_PATH"
    );
  }
  if gate_absorb_follow_related.is_some() && gate_absorb != Some(GateAbsorbVerb::Url) {
    anyhow::bail!("--follow-related is only supported for `pnix gate-absorb url`");
  }
  if gate_absorb_limit.is_some() && gate_absorb != Some(GateAbsorbVerb::Events) {
    anyhow::bail!("--limit/--query-limit is only supported for `pnix gate-absorb events`");
  }
  if gate_absorb_reset && gate_absorb != Some(GateAbsorbVerb::Events) {
    anyhow::bail!("--reset is only supported for `pnix gate-absorb events`");
  }
  if gate_forward_limit.is_some() && gate_forward != Some(GateForwardVerb::Run) {
    anyhow::bail!("--limit/--query-limit is only supported for `pnix gate-forward`");
  }
  if gate_forward_kind.is_some() && gate_forward != Some(GateForwardVerb::Run) {
    anyhow::bail!("--kind is only supported for `pnix gate-forward`");
  }
  if gate_forward_reset && gate_forward != Some(GateForwardVerb::Run) {
    anyhow::bail!("--reset is only supported for `pnix gate-forward`");
  }
  if gate_forward_url.is_some() && gate_forward != Some(GateForwardVerb::Run) {
    anyhow::bail!("--url is only supported for `pnix gate-forward`");
  }
  if gate_read_context.is_some()
    && !matches!(
      gate_read,
      Some(GateReadVerb::OntologyLookupRelated | GateReadVerb::RecipeMatchCurrent)
    )
  {
    anyhow::bail!(
      "--context/--lookup-context is only supported for `pnix gate-read ontology-lookup-related|recipe-match-current`"
    );
  }
  if gate_read_predicate.is_some() && gate_read != Some(GateReadVerb::OntologyLookupRelated) {
    anyhow::bail!(
      "--predicate/--lookup-predicate is only supported for `pnix gate-read ontology-lookup-related`"
    );
  }
  if gate_read_topic.is_some() && gate_read != Some(GateReadVerb::QueryContext) {
    anyhow::bail!("--topic/--query-topic is only supported for `pnix gate-read query-context`");
  }
  if !gate_read_event_types.is_empty() && gate_read != Some(GateReadVerb::RecentEvents) {
    anyhow::bail!("--event-type/--event_type is only supported for `pnix gate-read recent-events`");
  }
  if gate_read_tool_name.is_some() && gate_read != Some(GateReadVerb::RecipeMatchCurrent) {
    anyhow::bail!(
      "--tool-name/--tool_name is only supported for `pnix gate-read recipe-match-current`"
    );
  }
  if !gate_read_arg_predicates.is_empty() && gate_read != Some(GateReadVerb::RecipeMatchCurrent) {
    anyhow::bail!(
      "--arg-predicate/--arg_predicate is only supported for `pnix gate-read recipe-match-current`"
    );
  }
  if gate_read_limit.is_some()
    && !matches!(
      gate_read,
      Some(
        GateReadVerb::OntologyLookupRelated
          | GateReadVerb::LineageFloor
          | GateReadVerb::RecentEvents
          | GateReadVerb::Candidates
          | GateReadVerb::BrainAnkhPolicy
          | GateReadVerb::QueryContext
          | GateReadVerb::RecipeMatchCurrent
      )
    )
  {
    anyhow::bail!(
      "--limit/--query-limit is only supported for `pnix gate-read ontology-lookup-related|lineage-floor|recent-events|candidates|brain-ankh-policy|query-context|recipe-match-current`"
    );
  }
  if gate_read_min_confidence.is_some()
    && !matches!(
      gate_read,
      Some(GateReadVerb::OntologyLookupRelated | GateReadVerb::RecipeMatchCurrent)
    )
  {
    anyhow::bail!(
      "--min-confidence/--min_confidence is only supported for `pnix gate-read ontology-lookup-related|recipe-match-current`"
    );
  }
  if gate_read == Some(GateReadVerb::RecipeMatchCurrent) && gate_read_tool_name.is_none() {
    anyhow::bail!("--tool-name/--tool_name is required for `pnix gate-read recipe-match-current`");
  }
  if gate_read_kind.is_some() && gate_read != Some(GateReadVerb::Candidates) {
    anyhow::bail!("--kind is only supported for `pnix gate-read candidates`");
  }
  if gate_read_path.is_some() && gate_read != Some(GateReadVerb::ValidateBrainBundle) {
    anyhow::bail!("--path is only supported for `pnix gate-read validate-brain-bundle`");
  }
  if gate_read_proof_path.is_some() && gate_read != Some(GateReadVerb::ValidateBrainBundle) {
    anyhow::bail!("--proof-path is only supported for `pnix gate-read validate-brain-bundle`");
  }
  if gate_read_schema_path.is_some() && gate_read != Some(GateReadVerb::ValidateBrainBundle) {
    anyhow::bail!("--schema-path is only supported for `pnix gate-read validate-brain-bundle`");
  }
  if gate_read_expected_bundle_kind.is_some()
    && gate_read != Some(GateReadVerb::ValidateBrainBundle)
  {
    anyhow::bail!(
      "--expected-bundle-kind is only supported for `pnix gate-read validate-brain-bundle`"
    );
  }
  if gate_read_expected_lobe_profile.is_some()
    && gate_read != Some(GateReadVerb::ValidateBrainBundle)
  {
    anyhow::bail!(
      "--expected-lobe-profile is only supported for `pnix gate-read validate-brain-bundle`"
    );
  }
  if gate_read_expected_proof_kind.is_some() && gate_read != Some(GateReadVerb::ValidateBrainBundle)
  {
    anyhow::bail!(
      "--expected-proof-kind is only supported for `pnix gate-read validate-brain-bundle`"
    );
  }
  if gate_read == Some(GateReadVerb::ValidateBrainBundle) && gate_read_path.is_none() {
    anyhow::bail!("--path is required for `pnix gate-read validate-brain-bundle`");
  }

  let mut inputs = HashMap::new();
  let mut input_bytes: usize = 0;
  let inputs_specified = inputs_file.is_some() || inputs_json.is_some() || !input_pairs.is_empty();
  if let Some(path) = inputs_file {
    let remaining = max_input_bytes.saturating_sub(input_bytes);
    let contents_bytes = read_file_limited(&path, remaining)?;
    bump_input_bytes(&mut input_bytes, contents_bytes.len(), max_input_bytes)?;
    let contents = String::from_utf8(contents_bytes)
      .map_err(|err| anyhow::anyhow!("invalid inputs JSON ({}): {}", path, err))?;
    let value: serde_json::Value = serde_json::from_str(&contents)
      .map_err(|err| anyhow::anyhow!("invalid inputs JSON ({}): {}", path, err))?;
    merge_inputs(&mut inputs, value, &path)?;
  }

  if let Some(raw) = inputs_json {
    bump_input_bytes(&mut input_bytes, raw.len(), max_input_bytes)?;
    let value: serde_json::Value = serde_json::from_str(&raw)
      .map_err(|err| anyhow::anyhow!("invalid --inputs-json: {}", err))?;
    merge_inputs(&mut inputs, value, "--inputs-json")?;
  }

  for pair in input_pairs {
    bump_input_bytes(&mut input_bytes, pair.len(), max_input_bytes)?;
    let (key, value) = parse_input_pair(&pair)?;
    if inputs.insert(key.clone(), value).is_some() {
      eprintln!(
        "Warning: Duplicate --input key '{}', previous value will be overwritten",
        key
      );
    }
  }

  if let Some(raw_target) = aot_target {
    let raw_target = raw_target.trim();
    if raw_target.is_empty() {
      anyhow::bail!("--target requires a value");
    }
    let normalized = if raw_target.eq_ignore_ascii_case("aot")
      || raw_target.to_ascii_lowercase().starts_with("aot:")
    {
      raw_target.to_string()
    } else {
      format!("aot:{}", raw_target)
    };
    // Normalize both for consistent comparison
    let normalized_lower = normalized.trim().to_ascii_lowercase();
    match emit_target.as_deref() {
      None => {
        emit_target = Some(normalized);
      }
      Some(existing) => {
        let existing_lower = existing.trim().to_ascii_lowercase();
        if existing_lower == "aot" {
          emit_target = Some(normalized);
        } else if existing_lower.starts_with("aot:") {
          if existing_lower != normalized_lower {
            anyhow::bail!(
              "--target '{}' conflicts with --emit-target '{}'",
              raw_target,
              existing
            );
          }
          // Values match, keep existing emit_target
        } else {
          anyhow::bail!(
            "--target only applies to AOT emit targets, but --emit-target is '{}'",
            existing
          );
        }
      }
    }
  }

  if inputs_schema || list_modes || version {
    return Ok(Args {
      bin_name,
      mode,
      agent,
      gate_absorb,
      gate_forward,
      gate_read,
      gate_absorb_subject,
      gate_absorb_follow_related,
      gate_absorb_limit,
      gate_absorb_reset,
      gate_forward_limit,
      gate_forward_kind: gate_forward_kind.map(String::from),
      gate_forward_reset,
      gate_forward_url: gate_forward_url.map(String::from),
      gate_read_context: gate_read_context.map(String::from),
      gate_read_predicate: gate_read_predicate.map(String::from),
      gate_read_topic: gate_read_topic.map(String::from),
      gate_read_event_types: gate_read_event_types.iter().map(String::from).collect(),
      gate_read_tool_name: gate_read_tool_name.map(String::from),
      gate_read_arg_predicates: gate_read_arg_predicates.iter().map(String::from).collect(),
      gate_read_limit,
      gate_read_min_confidence,
      gate_read_kind: gate_read_kind.map(String::from),
      gate_read_path: gate_read_path.map(String::from),
      gate_read_proof_path: gate_read_proof_path.map(String::from),
      gate_read_schema_path: gate_read_schema_path.map(String::from),
      gate_read_expected_bundle_kind: gate_read_expected_bundle_kind.map(String::from),
      gate_read_expected_lobe_profile: gate_read_expected_lobe_profile.map(String::from),
      gate_read_expected_proof_kind: gate_read_expected_proof_kind.map(String::from),
      agent_request,
      agent_target_paths: agent_target_paths.iter().map(PathBuf::from).collect(),
      agent_project_pack_roots: agent_project_pack_roots.iter().map(PathBuf::from).collect(),
      agent_history_pack_roots: agent_history_pack_roots.iter().map(PathBuf::from).collect(),
      agent_approved_commands,
      agent_forbidden_paths: agent_forbidden_paths.iter().map(PathBuf::from).collect(),
      agent_policy_bits,
      agent_current_plan_ref,
      agent_rollback_handle_ref,
      agent_last_verification_ref,
      agent_promotion_boundary_ref,
      agent_source_apply_artifact_ref,
      agent_source_handoff_ref,
      agent_promotion_boundary_join_ref,
      agent_promotion_decision,
      agent_candidate_patch: agent_candidate_patch.clone().map(PathBuf::from),
      agent_provider_feedback_request_ref: agent_provider_feedback_request_ref.clone(),
      agent_request_out: agent_request_out.map(PathBuf::from),
      agent_plan_out: agent_plan_out.map(PathBuf::from),
      agent_patch_out: agent_patch_out.map(PathBuf::from),
      agent_verify_out: agent_verify_out.map(PathBuf::from),
      agent_rollback_out: agent_rollback_out.map(PathBuf::from),
      agent_decision_out: agent_decision_out.map(PathBuf::from),
      engine,
      result,
      dist: dist.map(PathBuf::from),
      clojure_url,
      python_url,
      deno_url,
      blenderpy_url,
      supervisor_sock: supervisor_sock.clone(),
      auto_ensure_backends,
      backend_specs: backend_specs.clone().map(PathBuf::from),
      replay_trace: replay_trace.clone().map(PathBuf::from),
      replay_mode: replay_mode.clone(),
      replay_allow: replay_allow.clone(),
      invocation_id: invocation_id.clone(),
      rpc_timeout_ms,
      rpc_retry_attempts,
      rpc_retry_backoff_ms,
      max_nodes,
      max_edges,
      max_input_bytes,
      use_batch,
      source,
      expr,
      test_filter,
      patch: patch.map(PathBuf::from),
      deterministic,
      strict_ct,
      inputs,
      seed,
      now_ms,
      clock_step_ms,
      frp_dt,
      inputs_schema,
      list_modes,
      list_ir_eval_ops,
      version,
      dry_run,
      emit,
      emit_target,
      emit_out: emit_out.map(PathBuf::from),
      emit_manifest: emit_manifest.map(PathBuf::from),
      binary,
      fmt_check,
      live,
      live_dir: live_dir.map(PathBuf::from),
      output_format,
    });
  }

  if list_ir_eval_ops {
    return Ok(Args {
      bin_name,
      mode,
      agent,
      gate_absorb,
      gate_forward,
      gate_read,
      gate_absorb_subject,
      gate_absorb_follow_related,
      gate_absorb_limit,
      gate_absorb_reset,
      gate_forward_limit,
      gate_forward_kind: gate_forward_kind.map(String::from),
      gate_forward_reset,
      gate_forward_url: gate_forward_url.map(String::from),
      gate_read_context: gate_read_context.map(String::from),
      gate_read_predicate: gate_read_predicate.map(String::from),
      gate_read_topic: gate_read_topic.map(String::from),
      gate_read_event_types: gate_read_event_types.iter().map(String::from).collect(),
      gate_read_tool_name: gate_read_tool_name.map(String::from),
      gate_read_arg_predicates: gate_read_arg_predicates.iter().map(String::from).collect(),
      gate_read_limit,
      gate_read_min_confidence,
      gate_read_kind: gate_read_kind.map(String::from),
      gate_read_path: gate_read_path.map(String::from),
      gate_read_proof_path: gate_read_proof_path.map(String::from),
      gate_read_schema_path: gate_read_schema_path.map(String::from),
      gate_read_expected_bundle_kind: gate_read_expected_bundle_kind.map(String::from),
      gate_read_expected_lobe_profile: gate_read_expected_lobe_profile.map(String::from),
      gate_read_expected_proof_kind: gate_read_expected_proof_kind.map(String::from),
      agent_request,
      agent_target_paths: agent_target_paths.iter().map(PathBuf::from).collect(),
      agent_project_pack_roots: agent_project_pack_roots.iter().map(PathBuf::from).collect(),
      agent_history_pack_roots: agent_history_pack_roots.iter().map(PathBuf::from).collect(),
      agent_approved_commands,
      agent_forbidden_paths: agent_forbidden_paths.iter().map(PathBuf::from).collect(),
      agent_policy_bits,
      agent_current_plan_ref,
      agent_rollback_handle_ref,
      agent_last_verification_ref,
      agent_promotion_boundary_ref,
      agent_source_apply_artifact_ref,
      agent_source_handoff_ref,
      agent_promotion_boundary_join_ref,
      agent_promotion_decision,
      agent_candidate_patch: agent_candidate_patch.clone().map(PathBuf::from),
      agent_provider_feedback_request_ref: agent_provider_feedback_request_ref.clone(),
      agent_request_out: agent_request_out.map(PathBuf::from),
      agent_plan_out: agent_plan_out.map(PathBuf::from),
      agent_patch_out: agent_patch_out.map(PathBuf::from),
      agent_verify_out: agent_verify_out.map(PathBuf::from),
      agent_rollback_out: agent_rollback_out.map(PathBuf::from),
      agent_decision_out: agent_decision_out.map(PathBuf::from),
      engine,
      result,
      dist: dist.map(PathBuf::from),
      clojure_url,
      python_url,
      deno_url,
      blenderpy_url,
      supervisor_sock: supervisor_sock.clone(),
      auto_ensure_backends,
      backend_specs: backend_specs.clone().map(PathBuf::from),
      replay_trace: replay_trace.clone().map(PathBuf::from),
      replay_mode: replay_mode.clone(),
      replay_allow: replay_allow.clone(),
      invocation_id: invocation_id.clone(),
      rpc_timeout_ms,
      rpc_retry_attempts,
      rpc_retry_backoff_ms,
      max_nodes,
      max_edges,
      max_input_bytes,
      use_batch,
      source,
      expr,
      test_filter,
      patch: patch.map(PathBuf::from),
      deterministic,
      strict_ct,
      inputs,
      seed,
      now_ms,
      clock_step_ms,
      frp_dt,
      inputs_schema,
      list_modes,
      list_ir_eval_ops,
      version,
      dry_run,
      emit,
      emit_target,
      emit_out: emit_out.map(PathBuf::from),
      emit_manifest: emit_manifest.map(PathBuf::from),
      binary,
      fmt_check,
      live,
      live_dir: live_dir.map(PathBuf::from),
      output_format,
    });
  }

  if agent.is_some() || gate_absorb.is_some() || gate_forward.is_some() || gate_read.is_some() {
    return Ok(Args {
      bin_name,
      mode,
      agent,
      gate_absorb,
      gate_forward,
      gate_read,
      gate_absorb_subject,
      gate_absorb_follow_related,
      gate_absorb_limit,
      gate_absorb_reset,
      gate_forward_limit,
      gate_forward_kind: gate_forward_kind.map(String::from),
      gate_forward_reset,
      gate_forward_url: gate_forward_url.map(String::from),
      gate_read_context: gate_read_context.map(String::from),
      gate_read_predicate: gate_read_predicate.map(String::from),
      gate_read_topic: gate_read_topic.map(String::from),
      gate_read_event_types: gate_read_event_types.iter().map(String::from).collect(),
      gate_read_tool_name: gate_read_tool_name.map(String::from),
      gate_read_arg_predicates: gate_read_arg_predicates.iter().map(String::from).collect(),
      gate_read_limit,
      gate_read_min_confidence,
      gate_read_kind: gate_read_kind.map(String::from),
      gate_read_path: gate_read_path.map(String::from),
      gate_read_proof_path: gate_read_proof_path.map(String::from),
      gate_read_schema_path: gate_read_schema_path.map(String::from),
      gate_read_expected_bundle_kind: gate_read_expected_bundle_kind.map(String::from),
      gate_read_expected_lobe_profile: gate_read_expected_lobe_profile.map(String::from),
      gate_read_expected_proof_kind: gate_read_expected_proof_kind.map(String::from),
      agent_request,
      agent_target_paths: agent_target_paths.iter().map(PathBuf::from).collect(),
      agent_project_pack_roots: agent_project_pack_roots.iter().map(PathBuf::from).collect(),
      agent_history_pack_roots: agent_history_pack_roots.iter().map(PathBuf::from).collect(),
      agent_approved_commands,
      agent_forbidden_paths: agent_forbidden_paths.iter().map(PathBuf::from).collect(),
      agent_policy_bits,
      agent_current_plan_ref,
      agent_rollback_handle_ref,
      agent_last_verification_ref,
      agent_promotion_boundary_ref,
      agent_source_apply_artifact_ref,
      agent_source_handoff_ref,
      agent_promotion_boundary_join_ref,
      agent_promotion_decision,
      agent_candidate_patch: agent_candidate_patch.clone().map(PathBuf::from),
      agent_provider_feedback_request_ref: agent_provider_feedback_request_ref.clone(),
      agent_request_out: agent_request_out.map(PathBuf::from),
      agent_plan_out: agent_plan_out.map(PathBuf::from),
      agent_patch_out: agent_patch_out.map(PathBuf::from),
      agent_verify_out: agent_verify_out.map(PathBuf::from),
      agent_rollback_out: agent_rollback_out.map(PathBuf::from),
      agent_decision_out: agent_decision_out.map(PathBuf::from),
      engine,
      result,
      dist: dist.map(PathBuf::from),
      clojure_url,
      python_url,
      deno_url,
      blenderpy_url,
      supervisor_sock,
      auto_ensure_backends,
      backend_specs: backend_specs.map(PathBuf::from),
      replay_trace: replay_trace.map(PathBuf::from),
      replay_mode,
      replay_allow,
      invocation_id,
      rpc_timeout_ms,
      rpc_retry_attempts,
      rpc_retry_backoff_ms,
      max_nodes,
      max_edges,
      max_input_bytes,
      use_batch,
      source,
      expr,
      test_filter,
      patch: patch.map(PathBuf::from),
      deterministic,
      strict_ct,
      inputs,
      seed,
      now_ms,
      clock_step_ms,
      frp_dt,
      inputs_schema,
      list_modes,
      list_ir_eval_ops,
      version,
      dry_run,
      emit,
      emit_target,
      emit_out: emit_out.map(PathBuf::from),
      emit_manifest: emit_manifest.map(PathBuf::from),
      binary,
      fmt_check,
      live,
      live_dir: live_dir.map(PathBuf::from),
      output_format,
    });
  }

  if engine.is_some() && mode != ExecMode::Run && mode != ExecMode::Interpret {
    anyhow::bail!("--engine is only supported for --mode run or interpret");
  }
  if test_filter.is_some() && mode != ExecMode::Test {
    anyhow::bail!("--filter is only supported for --mode test");
  }

  if fmt_check && mode != ExecMode::Fmt {
    anyhow::bail!("--check is only supported for fmt");
  }

  if result.is_some()
    && !(mode == ExecMode::Run && matches!(engine.as_deref(), Some("ir-eval" | "ir")))
  {
    anyhow::bail!("--result is only supported for --mode run --engine ir-eval");
  }

  if live {
    let live_supported = match mode {
      ExecMode::Run => matches!(engine.as_deref(), Some("ui")),
      ExecMode::Interpret => {
        matches!(
          engine.as_deref().unwrap_or("legacy-eval"),
          "ui" | "legacy-eval" | "eval"
        )
      }
      _ => false,
    };

    if !live_supported {
      anyhow::bail!(
        "--live/--live-dir is only supported for --engine ui (run/interpret) or interpret --engine legacy-eval/eval"
      );
    }
  }

  if rpc_timeout_ms_set && rpc_timeout_ms == 0 {
    anyhow::bail!("--rpc-timeout-ms must be >= 1");
  }
  if rpc_retry_attempts_set && rpc_retry_attempts == 0 {
    anyhow::bail!("--rpc-retry-attempts must be >= 1");
  }
  if rpc_retry_backoff_ms_set && rpc_retry_backoff_ms == 0 {
    anyhow::bail!("--rpc-retry-backoff-ms must be >= 1");
  }

  let backend_flags_specified = !use_batch
    || clojure_url_set
    || python_url_set
    || deno_url_set
    || blenderpy_url_set
    || supervisor_sock.is_some()
    || auto_ensure_backends_set
    || backend_specs.is_some()
    || replay_trace.is_some()
    || replay_mode.is_some()
    || !replay_allow.is_empty()
    || invocation_id.is_some()
    || rpc_timeout_ms_set
    || rpc_retry_attempts_set
    || rpc_retry_backoff_ms_set;
  let coding_agent_flags_specified = agent_request.is_some()
    || !agent_target_paths.is_empty()
    || !agent_project_pack_roots.is_empty()
    || !agent_history_pack_roots.is_empty()
    || !agent_approved_commands.is_empty()
    || !agent_forbidden_paths.is_empty()
    || !agent_policy_bits.is_empty()
    || agent_current_plan_ref.is_some()
    || agent_rollback_handle_ref.is_some()
    || agent_last_verification_ref.is_some()
    || agent_candidate_patch.is_some()
    || agent_provider_feedback_request_ref.is_some()
    || agent_request_out.is_some()
    || agent_plan_out.is_some()
    || agent_patch_out.is_some()
    || agent_verify_out.is_some()
    || agent_rollback_out.is_some();
  let emit_flags_specified = emit_target.is_some() || emit_out.is_some() || emit_manifest.is_some();
  let time_flags_specified = seed.is_some() || now_ms.is_some() || clock_step_ms.is_some();

  let run_engine = if mode == ExecMode::Run {
    engine.as_deref().unwrap_or("graph")
  } else {
    ""
  };
  let interpret_engine = if mode == ExecMode::Interpret {
    engine.as_deref().unwrap_or("legacy-eval")
  } else {
    ""
  };

  let will_ui_interpret = mode == ExecMode::Interpret && interpret_engine == "ui";
  let will_emit = emit || binary || (mode == ExecMode::Run && run_engine == "emit");
  let will_graph = mode == ExecMode::Graph
    || (mode == ExecMode::Run && (run_engine == "graph" || run_engine == "auto"))
    || (mode == ExecMode::Interpret && interpret_engine == "graph");
  let will_llvm = mode == ExecMode::Llvm || (mode == ExecMode::Run && run_engine == "llvm");
  let will_ir_eval = mode == ExecMode::Run
    && (run_engine == "ir-eval"
      || run_engine == "ir"
      || run_engine == "ui"
      || run_engine == "auto");
  let will_ssa = mode == ExecMode::Run && (run_engine == "ssa" || run_engine == "legacy-ssa");
  let will_parity = mode == ExecMode::Run && run_engine == "parity";
  let will_ct = mode == ExecMode::Ct || (mode == ExecMode::Interpret && interpret_engine == "ct");
  let will_legacy_eval = mode == ExecMode::LegacyEval
    || (mode == ExecMode::Interpret
      && (interpret_engine == "legacy-eval" || interpret_engine == "eval"));
  let will_legacy_frp = mode == ExecMode::LegacyFrp
    || (mode == ExecMode::Interpret
      && (interpret_engine == "legacy-frp" || interpret_engine == "frp"));

  if mode == ExecMode::Graph && (source.is_some() || expr.is_some()) {
    anyhow::bail!("--source/--expr is not supported for --mode graph");
  }

  if mode == ExecMode::Interpret && source.is_none() && expr.is_none() {
    match engine.as_deref().unwrap_or("legacy-eval") {
      "legacy-eval" | "eval" => {}
      other => anyhow::bail!(
        "--source or --expr is required for --mode interpret --engine {}",
        other
      ),
    }
  }

  if dist.is_some()
    && mode != ExecMode::Run
    && mode != ExecMode::Compile
    && mode != ExecMode::Graph
    && !(mode == ExecMode::Interpret && interpret_engine == "graph")
    && !emit
  {
    anyhow::bail!(
      "--dist is only supported for --mode run, compile, graph, or interpret --engine graph"
    );
  }

  if backend_flags_specified && !will_graph {
    anyhow::bail!(
            "--no-batch/--clojure-url/--python-url/--deno-url/--blenderpy-url/--supervisor-sock/--auto-ensure-backends/--no-auto-ensure-backends/--backend-specs/--replay/--replay-mode/--replay-allow/--invocation-id/--rpc-timeout-ms/--rpc-retry-attempts/--rpc-retry-backoff-ms is only supported for graph execution"
        );
  }
  if coding_agent_flags_specified && agent.is_none() {
    anyhow::bail!(
      "--request/--target-path/--project-pack-root/--history-pack-root/--approved-command/--forbidden-path/--workspace-policy/--current-plan-ref/--rollback-handle-ref/--last-verification-ref/--promotion-boundary-ref/--source-apply-artifact-ref/--source-handoff-ref/--promotion-boundary-join-ref/--promotion-decision/--candidate-patch/--provider-feedback-request-ref/--agent-request-out/--agent-plan-out/--agent-patch-out/--agent-verify-out/--agent-rollback-out/--agent-decision-out are only supported for `pnix coding-agent ...`"
    );
  }
  if replay_mode.is_some() && replay_trace.is_none() {
    anyhow::bail!("--replay-mode requires --replay <trace.jsonl>");
  }
  if !replay_allow.is_empty() && replay_trace.is_none() {
    anyhow::bail!("--replay-allow requires --replay <trace.jsonl>");
  }

  if emit_flags_specified && !will_emit {
    anyhow::bail!("--emit-target/--emit-out/--emit-manifest requires emit");
  }
  if binary && mode != ExecMode::Compile {
    anyhow::bail!("--binary is only supported for --mode compile");
  }

  if frp_dt.is_some() && !will_legacy_frp {
    anyhow::bail!("--dt is only supported for legacy-frp execution");
  }

  if !strict_ct && !will_ct {
    anyhow::bail!("--lenient-ct is only supported for ct execution");
  }

  if time_flags_specified
    && !(will_legacy_eval
      || will_legacy_frp
      || will_ct
      || will_llvm
      || will_ir_eval
      || will_ssa
      || will_parity
      || will_ui_interpret)
  {
    anyhow::bail!("--seed/--now/--clock-step is only supported for legacy-eval, legacy-frp, ct, llvm, run --engine ir-eval, run --engine ui, run --engine ssa, run --engine parity, or interpret --engine ui");
  }

  if inputs_specified
    && !(will_graph || will_legacy_frp || will_llvm || will_ir_eval || will_ssa || will_parity)
  {
    anyhow::bail!(
            "external inputs flags are only supported for graph, legacy-frp, llvm, run --engine ir-eval, run --engine ui, run --engine ssa, or run --engine parity"
        );
  }

  if emit && mode != ExecMode::Graph && mode != ExecMode::Run {
    anyhow::bail!("--emit cannot be combined with --mode {}", mode_label(mode));
  }
  if emit && dry_run {
    anyhow::bail!("--dry-run is not supported for --emit");
  }

  if dry_run
    && mode != ExecMode::Graph
    && mode != ExecMode::Compile
    && mode != ExecMode::Run
    && !(mode == ExecMode::Interpret && interpret_engine == "graph")
  {
    anyhow::bail!(
      "--dry-run is only supported for --mode run, graph, compile, or interpret --engine graph"
    );
  }
  if patch.is_some()
    && mode != ExecMode::Graph
    && mode != ExecMode::Run
    && mode != ExecMode::Interpret
    && mode != ExecMode::LegacyEval
    && mode != ExecMode::LegacyFrp
    && mode != ExecMode::Llvm
  {
    anyhow::bail!(
            "--patch is only supported for --mode run, interpret, graph, legacy-eval, legacy-frp, or llvm"
        );
  }
  if mode == ExecMode::Interpret && patch.is_some() && matches!(engine.as_deref(), Some("ct")) {
    anyhow::bail!("--patch is not supported for --mode interpret --engine ct");
  }

  if emit {
    if dist.is_none() {
      anyhow::bail!("--dist is required for --emit");
    }
  } else {
    if (mode == ExecMode::Graph
      || mode == ExecMode::Compile
      || mode == ExecMode::Run
      || (mode == ExecMode::Interpret && interpret_engine == "graph"))
      && dist.is_none()
    {
      if mode == ExecMode::Interpret {
        anyhow::bail!("--dist is required for --mode interpret --engine graph");
      }
      anyhow::bail!("--dist is required for --mode {}", mode_label(mode));
    }
    if (mode == ExecMode::Compile
      || mode == ExecMode::LegacyEval
      || mode == ExecMode::LegacyFrp
      || mode == ExecMode::Ct
      || mode == ExecMode::Llvm
      || mode == ExecMode::Test)
      && source.is_none()
      && expr.is_none()
    {
      anyhow::bail!(
        "--source or --expr is required for --mode {}",
        mode_label(mode)
      );
    }
  }

  Ok(Args {
    bin_name,
    mode,
    agent,
    gate_absorb,
    gate_forward,
    gate_read,
    gate_absorb_subject,
    gate_absorb_follow_related,
    gate_absorb_limit,
    gate_absorb_reset,
    gate_forward_limit,
    gate_forward_kind: gate_forward_kind.map(String::from),
    gate_forward_reset,
    gate_forward_url: gate_forward_url.map(String::from),
    gate_read_context: gate_read_context.map(String::from),
    gate_read_predicate: gate_read_predicate.map(String::from),
    gate_read_topic: gate_read_topic.map(String::from),
    gate_read_event_types: gate_read_event_types.iter().map(String::from).collect(),
    gate_read_tool_name: gate_read_tool_name.map(String::from),
    gate_read_arg_predicates: gate_read_arg_predicates.iter().map(String::from).collect(),
    gate_read_limit,
    gate_read_min_confidence,
    gate_read_kind: gate_read_kind.map(String::from),
    gate_read_path: gate_read_path.map(String::from),
    gate_read_proof_path: gate_read_proof_path.map(String::from),
    gate_read_schema_path: gate_read_schema_path.map(String::from),
    gate_read_expected_bundle_kind: gate_read_expected_bundle_kind.map(String::from),
    gate_read_expected_lobe_profile: gate_read_expected_lobe_profile.map(String::from),
    gate_read_expected_proof_kind: gate_read_expected_proof_kind.map(String::from),
    agent_request,
    agent_target_paths: agent_target_paths.iter().map(PathBuf::from).collect(),
    agent_project_pack_roots: agent_project_pack_roots.iter().map(PathBuf::from).collect(),
    agent_history_pack_roots: agent_history_pack_roots.iter().map(PathBuf::from).collect(),
    agent_approved_commands,
    agent_forbidden_paths: agent_forbidden_paths.iter().map(PathBuf::from).collect(),
    agent_policy_bits,
    agent_current_plan_ref,
    agent_rollback_handle_ref,
    agent_last_verification_ref,
    agent_promotion_boundary_ref,
    agent_source_apply_artifact_ref,
    agent_source_handoff_ref,
    agent_promotion_boundary_join_ref,
    agent_promotion_decision,
    agent_candidate_patch: agent_candidate_patch.map(PathBuf::from),
    agent_provider_feedback_request_ref,
    agent_request_out: agent_request_out.map(PathBuf::from),
    agent_plan_out: agent_plan_out.map(PathBuf::from),
    agent_patch_out: agent_patch_out.map(PathBuf::from),
    agent_verify_out: agent_verify_out.map(PathBuf::from),
    agent_rollback_out: agent_rollback_out.map(PathBuf::from),
    agent_decision_out: agent_decision_out.map(PathBuf::from),
    engine,
    result,
    dist: dist.map(PathBuf::from),
    clojure_url,
    python_url,
    deno_url,
    blenderpy_url,
    supervisor_sock,
    auto_ensure_backends,
    backend_specs: backend_specs.map(PathBuf::from),
    replay_trace: replay_trace.map(PathBuf::from),
    replay_mode,
    replay_allow,
    invocation_id,
    rpc_timeout_ms,
    rpc_retry_attempts,
    rpc_retry_backoff_ms,
    max_nodes,
    max_edges,
    max_input_bytes,
    use_batch,
    source,
    expr,
    test_filter,
    patch: patch.map(PathBuf::from),
    deterministic,
    strict_ct,
    inputs,
    seed,
    now_ms,
    clock_step_ms,
    frp_dt,
    inputs_schema,
    list_modes,
    list_ir_eval_ops,
    version,
    dry_run,
    emit,
    emit_target,
    emit_out: emit_out.map(PathBuf::from),
    emit_manifest: emit_manifest.map(PathBuf::from),
    binary,
    fmt_check,
    live,
    live_dir: live_dir.map(PathBuf::from),
    output_format,
  })
}
