//! CLI 테스트: CLI 인자 파싱 및 실행 테스트

use super::args::{parse_input_pair, AgentVerb, GateAbsorbVerb, GateForwardVerb, GateReadVerb};
use super::*;
#[cfg(feature = "doghouse")]
use doghouse_core::store::{DoghouseStore, DoghouseStoreConfig};
use pnix_runtime_legacy::ir::{LegacyIr, LegacyOp};
use std::fs;
use tempfile::Builder;

const LEGACY_EVAL_MODE: &str = "legacy-eval";

fn temp_dir(label: &str) -> std::path::PathBuf {
  // 테스트 격리: 각 테스트마다 고유한 임시 디렉토리 생성
  // TempDir는 Drop 시 자동으로 정리되므로 keep() 제거
  // 테스트가 끝날 때까지 유지하기 위해 경로만 반환
  let mut builder = Builder::new();
  let prefix = format!("pnix-executor-{}-", label);
  builder.prefix(&prefix);
  let temp_dir = std::env::var_os("HOME")
    .map(std::path::PathBuf::from)
    .or({
      #[cfg(windows)]
      {
        std::env::var_os("USERPROFILE").map(std::path::PathBuf::from)
      }
      #[cfg(not(windows))]
      {
        None
      }
    })
    .and_then(|home| builder.tempdir_in(home).ok())
    .unwrap_or_else(|| builder.tempdir().expect("create temp dir"));
  let path = temp_dir.path().to_path_buf();
  // TempDir가 drop되지 않도록 유지 (테스트가 끝나면 자동 정리)
  std::mem::forget(temp_dir);
  path
}

#[test]
fn read_json_file_with_limits_rejects_large_inputs() {
  let dist = temp_dir("input-bytes");
  let path = dist.join("fxcore.canon.json");
  let payload = r#"{ "meta": { "version": "fxcore@0.1", "stage": 1 } }"#;
  fs::write(&path, payload).unwrap();

  let limits = ResourceLimits {
    max_nodes: 10,
    max_edges: 10,
    max_input_bytes: 8,
  };
  let err = read_json_file_with_limits::<serde_json::Value>(&path, &limits, "fxcore json")
    .expect_err("expected size limit failure");
  assert!(err.to_string().contains("Input exceeds resource limit"));
}

#[tokio::test]
async fn graph_mode_exits_error_when_nodes_fail() {
  let dist = temp_dir("graph-fail");
  std::fs::create_dir_all(dist.join("ir")).unwrap();

  std::fs::write(
    dist.join("pnix.replay.json"),
    r#"{ "replay_hash": "test" }"#,
  )
  .unwrap();

  // Uses an unsupported backend ("noop") so apply has a deterministic failure
  // without requiring any RPC servers.
  std::fs::write(
    dist.join("ir").join("fxcore.canon.json"),
    r#"
{
  "meta": { "version": "fxcore@0.1", "stage": 1 },
  "name": "test",
  "types": [],
  "inputs": [],
  "morphisms": [],
  "nodes": [
{ "name": "n1", "uses": "noop" }
  ],
  "edges": [],
  "scopes": []
}
"#,
  )
  .unwrap();

  let args = Args {
    bin_name: "pnix-executor-graph".into(),
    mode: ExecMode::Graph,
    agent: None,
    gate_absorb: None,
    gate_absorb_subject: None,
    gate_absorb_follow_related: None,
    gate_absorb_limit: None,
    gate_absorb_reset: false,
    gate_forward: None,
    gate_read: None,
    gate_forward_limit: None,
    gate_forward_kind: None,
    gate_forward_reset: false,
    gate_forward_url: None,
    gate_read_context: None,
    gate_read_predicate: None,
    gate_read_topic: None,
    gate_read_event_types: vec![],
    gate_read_tool_name: None,
    gate_read_arg_predicates: vec![],
    gate_read_limit: None,
    gate_read_min_confidence: None,
    gate_read_kind: None,
    gate_read_path: None,
    gate_read_proof_path: None,
    gate_read_schema_path: None,
    gate_read_expected_bundle_kind: None,
    gate_read_expected_lobe_profile: None,
    gate_read_expected_proof_kind: None,
    agent_request: None,
    agent_target_paths: vec![],
    agent_project_pack_roots: vec![],
    agent_history_pack_roots: vec![],
    agent_approved_commands: vec![],
    agent_forbidden_paths: vec![],
    agent_policy_bits: vec![],
    agent_current_plan_ref: None,
    agent_rollback_handle_ref: None,
    agent_last_verification_ref: None,
    agent_promotion_boundary_ref: None,
    agent_source_apply_artifact_ref: None,
    agent_source_handoff_ref: None,
    agent_promotion_boundary_join_ref: None,
    agent_promotion_decision: None,
    agent_candidate_patch: None,
    agent_provider_feedback_request_ref: None,
    agent_request_out: None,
    agent_plan_out: None,
    agent_patch_out: None,
    agent_verify_out: None,
    agent_rollback_out: None,
    agent_decision_out: None,
    engine: None,
    result: None,
    dist: Some(dist.clone()),
    clojure_url: "http://localhost:7777".into(),
    python_url: "http://localhost:7778".into(),
    deno_url: "http://localhost:7779".into(),
    blenderpy_url: "http://localhost:7781".into(),
    supervisor_sock: None,
    backend_specs: None,
    auto_ensure_backends: true,
    replay_trace: None,
    replay_mode: None,
    replay_allow: vec![],
    invocation_id: None,
    rpc_timeout_ms: 30_000,
    rpc_retry_attempts: 3,
    rpc_retry_backoff_ms: 100,
    max_nodes: 10_000,
    max_edges: 50_000,
    max_input_bytes: 10 * 1024 * 1024,
    use_batch: true,
    source: None,
    expr: None,
    test_filter: None,
    patch: None,
    deterministic: true,
    strict_ct: true,
    inputs: HashMap::new(),
    seed: None,
    now_ms: None,
    clock_step_ms: None,
    frp_dt: None,
    inputs_schema: false,
    list_modes: false,
    list_ir_eval_ops: false,
    version: false,
    dry_run: false,
    emit: false,
    emit_target: None,
    binary: false,
    emit_out: None,
    emit_manifest: None,
    fmt_check: false,
    live: false,
    live_dir: None,
    output_format: super::args::OutputFormat::Text,
  };

  let err = run_graph(&args).await.unwrap_err();
  assert!(err
    .to_string()
    .contains("apply_graph completed with failures"));

  let apply_txt = std::fs::read_to_string(dist.join("pnix.apply_graph.json")).unwrap();
  let apply_v: serde_json::Value = serde_json::from_str(&apply_txt).unwrap();
  assert_eq!(
    apply_v.get("status").and_then(|v| v.as_str()),
    Some("error")
  );
}

#[tokio::test]
async fn graph_mode_requires_supervisor_sock_for_process_capability() {
  let dist = temp_dir("graph-process-cap");
  std::fs::create_dir_all(dist.join("ir")).unwrap();
  std::fs::write(
    dist.join("ir").join("used_spec.canon.json"),
    r#"
{
  "used_builtins": {
    "processSpawn": {
      "name": "processSpawn",
      "signature": "ProcessSpec → ProcessHandle",
      "effect": "world",
      "capabilities": ["ProcessSpawn"],
      "arity": 1,
      "description": "Spawn a process"
    }
  },
  "used_types": {},
  "used_morphisms": {}
}
"#,
  )
  .unwrap();
  std::fs::write(
    dist.join("pnix.replay.json"),
    r#"{ "replay_hash": "test" }"#,
  )
  .unwrap();
  std::fs::write(
    dist.join("ir").join("fxcore.canon.json"),
    r#"
{
  "meta": { "version": "fxcore@0.1", "stage": 1 },
  "name": "test",
  "types": [],
  "inputs": [],
  "morphisms": [],
  "nodes": [],
  "edges": [],
  "scopes": []
}
"#,
  )
  .unwrap();

  let previous_sock = std::env::var_os("PNIX_SUPERVISOR_SOCK");
  let previous_endpoint = std::env::var_os("PNIX_SUPERVISOR_ENDPOINT");
  std::env::remove_var("PNIX_SUPERVISOR_SOCK");
  std::env::remove_var("PNIX_SUPERVISOR_ENDPOINT");

  let args = Args {
    bin_name: "pnix-executor-graph".into(),
    mode: ExecMode::Graph,
    agent: None,
    gate_absorb: None,
    gate_absorb_subject: None,
    gate_absorb_follow_related: None,
    gate_absorb_limit: None,
    gate_absorb_reset: false,
    gate_forward: None,
    gate_read: None,
    gate_forward_limit: None,
    gate_forward_kind: None,
    gate_forward_reset: false,
    gate_forward_url: None,
    gate_read_context: None,
    gate_read_predicate: None,
    gate_read_topic: None,
    gate_read_event_types: vec![],
    gate_read_tool_name: None,
    gate_read_arg_predicates: vec![],
    gate_read_limit: None,
    gate_read_min_confidence: None,
    gate_read_kind: None,
    gate_read_path: None,
    gate_read_proof_path: None,
    gate_read_schema_path: None,
    gate_read_expected_bundle_kind: None,
    gate_read_expected_lobe_profile: None,
    gate_read_expected_proof_kind: None,
    agent_request: None,
    agent_target_paths: vec![],
    agent_project_pack_roots: vec![],
    agent_history_pack_roots: vec![],
    agent_approved_commands: vec![],
    agent_forbidden_paths: vec![],
    agent_policy_bits: vec![],
    agent_current_plan_ref: None,
    agent_rollback_handle_ref: None,
    agent_last_verification_ref: None,
    agent_promotion_boundary_ref: None,
    agent_source_apply_artifact_ref: None,
    agent_source_handoff_ref: None,
    agent_promotion_boundary_join_ref: None,
    agent_promotion_decision: None,
    agent_candidate_patch: None,
    agent_provider_feedback_request_ref: None,
    agent_request_out: None,
    agent_plan_out: None,
    agent_patch_out: None,
    agent_verify_out: None,
    agent_rollback_out: None,
    agent_decision_out: None,
    engine: None,
    result: None,
    dist: Some(dist),
    clojure_url: "http://localhost:7777".into(),
    python_url: "http://localhost:7778".into(),
    deno_url: "http://localhost:7779".into(),
    blenderpy_url: "http://localhost:7781".into(),
    supervisor_sock: None,
    backend_specs: None,
    auto_ensure_backends: true,
    replay_trace: None,
    replay_mode: None,
    replay_allow: vec![],
    invocation_id: None,
    rpc_timeout_ms: 30_000,
    rpc_retry_attempts: 3,
    rpc_retry_backoff_ms: 100,
    max_nodes: 10_000,
    max_edges: 50_000,
    max_input_bytes: 10 * 1024 * 1024,
    use_batch: true,
    source: None,
    expr: None,
    test_filter: None,
    patch: None,
    deterministic: true,
    strict_ct: true,
    inputs: HashMap::new(),
    seed: None,
    now_ms: None,
    clock_step_ms: None,
    frp_dt: None,
    inputs_schema: false,
    list_modes: false,
    list_ir_eval_ops: false,
    version: false,
    dry_run: false,
    emit: false,
    emit_target: None,
    binary: false,
    emit_out: None,
    emit_manifest: None,
    fmt_check: false,
    live: false,
    live_dir: None,
    output_format: super::args::OutputFormat::Text,
  };

  let err = run_graph(&args)
    .await
    .expect_err("process capability should require supervisor socket");
  assert!(err.to_string().contains("PNIX_SUPERVISOR_SOCK"));

  if let Some(value) = previous_sock {
    std::env::set_var("PNIX_SUPERVISOR_SOCK", value);
  } else {
    std::env::remove_var("PNIX_SUPERVISOR_SOCK");
  }
  if let Some(value) = previous_endpoint {
    std::env::set_var("PNIX_SUPERVISOR_ENDPOINT", value);
  } else {
    std::env::remove_var("PNIX_SUPERVISOR_ENDPOINT");
  }
}

#[tokio::test]
async fn u06_graph_mode_rejects_unknown_inputs_keys() {
  let dist = temp_dir("graph-unknown-input");
  std::fs::create_dir_all(dist.join("ir")).unwrap();

  std::fs::write(
    dist.join("pnix.replay.json"),
    r#"{ "replay_hash": "test" }"#,
  )
  .unwrap();

  std::fs::write(
    dist.join("ir").join("fxcore.canon.json"),
    r#"
{
  "meta": { "version": "fxcore@0.1", "stage": 2 },
  "name": "test",
  "types": [],
  "inputs": [{ "name": "a_in", "ty": "real" }],
  "morphisms": [],
  "nodes": [],
  "edges": [],
  "scopes": []
}
"#,
  )
  .unwrap();

  let mut inputs = HashMap::new();
  inputs.insert("typo".to_string(), serde_json::json!(1));

  let args = Args {
    bin_name: "pnix-executor-graph".into(),
    mode: ExecMode::Graph,
    agent: None,
    gate_absorb: None,
    gate_absorb_subject: None,
    gate_absorb_follow_related: None,
    gate_absorb_limit: None,
    gate_absorb_reset: false,
    gate_forward: None,
    gate_read: None,
    gate_forward_limit: None,
    gate_forward_kind: None,
    gate_forward_reset: false,
    gate_forward_url: None,
    gate_read_context: None,
    gate_read_predicate: None,
    gate_read_topic: None,
    gate_read_event_types: vec![],
    gate_read_tool_name: None,
    gate_read_arg_predicates: vec![],
    gate_read_limit: None,
    gate_read_min_confidence: None,
    gate_read_kind: None,
    gate_read_path: None,
    gate_read_proof_path: None,
    gate_read_schema_path: None,
    gate_read_expected_bundle_kind: None,
    gate_read_expected_lobe_profile: None,
    gate_read_expected_proof_kind: None,
    agent_request: None,
    agent_target_paths: vec![],
    agent_project_pack_roots: vec![],
    agent_history_pack_roots: vec![],
    agent_approved_commands: vec![],
    agent_forbidden_paths: vec![],
    agent_policy_bits: vec![],
    agent_current_plan_ref: None,
    agent_rollback_handle_ref: None,
    agent_last_verification_ref: None,
    agent_promotion_boundary_ref: None,
    agent_source_apply_artifact_ref: None,
    agent_source_handoff_ref: None,
    agent_promotion_boundary_join_ref: None,
    agent_promotion_decision: None,
    agent_candidate_patch: None,
    agent_provider_feedback_request_ref: None,
    agent_request_out: None,
    agent_plan_out: None,
    agent_patch_out: None,
    agent_verify_out: None,
    agent_rollback_out: None,
    agent_decision_out: None,
    engine: None,
    result: None,
    dist: Some(dist),
    clojure_url: "http://localhost:7777".into(),
    python_url: "http://localhost:7778".into(),
    deno_url: "http://localhost:7779".into(),
    blenderpy_url: "http://localhost:7781".into(),
    supervisor_sock: None,
    backend_specs: None,
    auto_ensure_backends: true,
    replay_trace: None,
    replay_mode: None,
    replay_allow: vec![],
    invocation_id: None,
    rpc_timeout_ms: 30_000,
    rpc_retry_attempts: 3,
    rpc_retry_backoff_ms: 100,
    max_nodes: 10_000,
    max_edges: 50_000,
    max_input_bytes: 10 * 1024 * 1024,
    use_batch: true,
    source: None,
    expr: None,
    test_filter: None,
    patch: None,
    deterministic: true,
    strict_ct: true,
    inputs,
    seed: None,
    now_ms: None,
    clock_step_ms: None,
    frp_dt: None,
    inputs_schema: false,
    list_modes: false,
    list_ir_eval_ops: false,
    version: false,
    dry_run: false,
    emit: false,
    emit_target: None,
    binary: false,
    emit_out: None,
    emit_manifest: None,
    fmt_check: false,
    live: false,
    live_dir: None,
    output_format: super::args::OutputFormat::Text,
  };

  let err = run_graph(&args)
    .await
    .expect_err("unknown inputs must error");
  assert!(err.to_string().contains("unknown --inputs keys"));
  assert!(err.to_string().contains("typo"));
}

#[test]
fn u06_legacy_frp_rejects_unknown_inputs_keys() {
  let mut inputs = HashMap::new();
  inputs.insert("typo".to_string(), serde_json::json!(1.0));

  let args = Args {
    bin_name: "pnix-executor-graph".into(),
    mode: ExecMode::LegacyFrp,
    agent: None,
    gate_absorb: None,
    gate_absorb_subject: None,
    gate_absorb_follow_related: None,
    gate_absorb_limit: None,
    gate_absorb_reset: false,
    gate_forward: None,
    gate_read: None,
    gate_forward_limit: None,
    gate_forward_kind: None,
    gate_forward_reset: false,
    gate_forward_url: None,
    gate_read_context: None,
    gate_read_predicate: None,
    gate_read_topic: None,
    gate_read_event_types: vec![],
    gate_read_tool_name: None,
    gate_read_arg_predicates: vec![],
    gate_read_limit: None,
    gate_read_min_confidence: None,
    gate_read_kind: None,
    gate_read_path: None,
    gate_read_proof_path: None,
    gate_read_schema_path: None,
    gate_read_expected_bundle_kind: None,
    gate_read_expected_lobe_profile: None,
    gate_read_expected_proof_kind: None,
    agent_request: None,
    agent_target_paths: vec![],
    agent_project_pack_roots: vec![],
    agent_history_pack_roots: vec![],
    agent_approved_commands: vec![],
    agent_forbidden_paths: vec![],
    agent_policy_bits: vec![],
    agent_current_plan_ref: None,
    agent_rollback_handle_ref: None,
    agent_last_verification_ref: None,
    agent_promotion_boundary_ref: None,
    agent_source_apply_artifact_ref: None,
    agent_source_handoff_ref: None,
    agent_promotion_boundary_join_ref: None,
    agent_promotion_decision: None,
    agent_candidate_patch: None,
    agent_provider_feedback_request_ref: None,
    agent_request_out: None,
    agent_plan_out: None,
    agent_patch_out: None,
    agent_verify_out: None,
    agent_rollback_out: None,
    agent_decision_out: None,
    engine: None,
    result: None,
    dist: None,
    clojure_url: "http://localhost:7777".into(),
    python_url: "http://localhost:7778".into(),
    deno_url: "http://localhost:7779".into(),
    blenderpy_url: "http://localhost:7781".into(),
    supervisor_sock: None,
    backend_specs: None,
    auto_ensure_backends: true,
    replay_trace: None,
    replay_mode: None,
    replay_allow: vec![],
    invocation_id: None,
    rpc_timeout_ms: 30_000,
    rpc_retry_attempts: 3,
    rpc_retry_backoff_ms: 100,
    max_nodes: 10_000,
    max_edges: 50_000,
    max_input_bytes: 10 * 1024 * 1024,
    use_batch: true,
    source: None,
    expr: Some(
      r#"{ "signals": [ { "name": "x", "kind": "input", "default": 0.0 } ], "external_inputs": {} }"#
        .to_string(),
    ),
    test_filter: None,
    patch: None,
    deterministic: true,
    strict_ct: true,
    inputs,
    seed: None,
    now_ms: None,
    clock_step_ms: None,
    frp_dt: Some(0.1),
    inputs_schema: false,
    list_modes: false,
    list_ir_eval_ops: false,
    version: false,
    dry_run: false,
    emit: false,
    emit_target: None,
    binary: false,
    emit_out: None,
    emit_manifest: None,
    fmt_check: false,
    live: false,
    live_dir: None,
    output_format: super::args::OutputFormat::Text,
  };

  let err = run_legacy_frp(&args).expect_err("unknown inputs must error");
  assert!(err.to_string().contains("unknown --inputs keys"));
  assert!(err.to_string().contains("legacy-frp"));
  assert!(err.to_string().contains("typo"));
}

#[test]
fn u13_llvm_rejects_unknown_inputs_keys() {
  let fx_json = r#"
{
  "meta": { "version": "fxcore@0.1", "stage": 2 },
  "name": "test",
  "types": [],
  "inputs": [{ "name": "a", "ty": "real" }],
  "morphisms": [],
  "nodes": [],
  "edges": [],
  "scopes": []
}
"#;

  let mut inputs = HashMap::new();
  inputs.insert("typo".to_string(), serde_json::json!(1));

  let args = Args {
    bin_name: "pnix-executor-graph".into(),
    mode: ExecMode::Llvm,
    agent: None,
    gate_absorb: None,
    gate_absorb_subject: None,
    gate_absorb_follow_related: None,
    gate_absorb_limit: None,
    gate_absorb_reset: false,
    gate_forward: None,
    gate_read: None,
    gate_forward_limit: None,
    gate_forward_kind: None,
    gate_forward_reset: false,
    gate_forward_url: None,
    gate_read_context: None,
    gate_read_predicate: None,
    gate_read_topic: None,
    gate_read_event_types: vec![],
    gate_read_tool_name: None,
    gate_read_arg_predicates: vec![],
    gate_read_limit: None,
    gate_read_min_confidence: None,
    gate_read_kind: None,
    gate_read_path: None,
    gate_read_proof_path: None,
    gate_read_schema_path: None,
    gate_read_expected_bundle_kind: None,
    gate_read_expected_lobe_profile: None,
    gate_read_expected_proof_kind: None,
    agent_request: None,
    agent_target_paths: vec![],
    agent_project_pack_roots: vec![],
    agent_history_pack_roots: vec![],
    agent_approved_commands: vec![],
    agent_forbidden_paths: vec![],
    agent_policy_bits: vec![],
    agent_current_plan_ref: None,
    agent_rollback_handle_ref: None,
    agent_last_verification_ref: None,
    agent_promotion_boundary_ref: None,
    agent_source_apply_artifact_ref: None,
    agent_source_handoff_ref: None,
    agent_promotion_boundary_join_ref: None,
    agent_promotion_decision: None,
    agent_candidate_patch: None,
    agent_provider_feedback_request_ref: None,
    agent_request_out: None,
    agent_plan_out: None,
    agent_patch_out: None,
    agent_verify_out: None,
    agent_rollback_out: None,
    agent_decision_out: None,
    engine: None,
    result: None,
    dist: None,
    clojure_url: "http://localhost:7777".into(),
    python_url: "http://localhost:7778".into(),
    deno_url: "http://localhost:7779".into(),
    blenderpy_url: "http://localhost:7781".into(),
    supervisor_sock: None,
    backend_specs: None,
    auto_ensure_backends: true,
    replay_trace: None,
    replay_mode: None,
    replay_allow: vec![],
    invocation_id: None,
    rpc_timeout_ms: 30_000,
    rpc_retry_attempts: 3,
    rpc_retry_backoff_ms: 100,
    max_nodes: 10_000,
    max_edges: 50_000,
    max_input_bytes: 10 * 1024 * 1024,
    use_batch: true,
    source: None,
    expr: Some(fx_json.to_string()),
    test_filter: None,
    patch: None,
    deterministic: true,
    strict_ct: true,
    inputs,
    seed: None,
    now_ms: None,
    clock_step_ms: None,
    frp_dt: None,
    inputs_schema: false,
    list_modes: false,
    list_ir_eval_ops: false,
    version: false,
    dry_run: false,
    emit: false,
    emit_target: None,
    binary: false,
    emit_out: None,
    emit_manifest: None,
    fmt_check: false,
    live: false,
    live_dir: None,
    output_format: super::args::OutputFormat::Text,
  };

  let err = run_llvm(&args).expect_err("unknown inputs must error");
  assert!(err.to_string().contains("unknown --inputs keys"));
  assert!(err.to_string().contains("llvm"));
  assert!(err.to_string().contains("typo"));
}

#[test]
fn parse_emit_target_variants() {
  let legacy = parse_emit_target("javascript").unwrap();
  assert!(matches!(
    legacy,
    EmitTarget::Legacy(CodegenTarget::Javascript)
  ));

  let legacy_all = parse_emit_target("all").unwrap();
  assert!(matches!(legacy_all, EmitTarget::LegacyAll));

  let aot_default = parse_emit_target("aot").unwrap();
  assert!(matches!(aot_default, EmitTarget::Aot(_)));

  let aot_linux = parse_emit_target("aot:linux").unwrap();
  assert!(matches!(aot_linux, EmitTarget::Aot(AotTarget::LinuxX86_64)));
}

#[test]
fn parse_emit_target_invalid() {
  let err = parse_emit_target("nope").unwrap_err();
  assert!(err.to_string().contains("unknown emit target"));
}

#[test]
fn emit_backend_legacy_manifest_sorted_and_sized() {
  let mut ir = LegacyIr::new();
  ir.add("x".to_string(), LegacyOp::Literal(serde_json::json!(42)));
  ir.set_output("x".to_string());

  let out_dir = temp_dir("emit-backend-legacy");
  let result = emit_backend_legacy(&ir, &out_dir, None).unwrap();

  let files = result.get("files").and_then(|v| v.as_array()).unwrap();
  let total_size = result.get("total_size").and_then(|v| v.as_u64()).unwrap();

  let paths: Vec<String> = files
    .iter()
    .map(|entry| {
      entry
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string()
    })
    .collect();
  let mut sorted = paths.clone();
  sorted.sort();
  assert_eq!(paths, sorted, "manifest files must be sorted by path");

  let sum: u64 = files
    .iter()
    .map(|entry| entry.get("size").and_then(|v| v.as_u64()).unwrap())
    .sum();
  assert_eq!(total_size, sum, "total_size must equal sum of file sizes");
}

// ========================================
// S18-8: AOT emit manifest/path validation tests (LLVM-free)
// ========================================

#[test]
fn test_s18_8_parse_aot_target_aliases() {
  // Linux aliases
  assert!(matches!(
    parse_aot_target("linux"),
    Some(AotTarget::LinuxX86_64)
  ));
  assert!(matches!(
    parse_aot_target("linux-x86_64"),
    Some(AotTarget::LinuxX86_64)
  ));
  assert!(matches!(
    parse_aot_target("x86_64-unknown-linux-gnu"),
    Some(AotTarget::LinuxX86_64)
  ));

  // macOS x86_64 aliases
  assert!(matches!(
    parse_aot_target("macos"),
    Some(AotTarget::MacOSX86_64)
  ));
  assert!(matches!(
    parse_aot_target("macos-x86_64"),
    Some(AotTarget::MacOSX86_64)
  ));
  assert!(matches!(
    parse_aot_target("x86_64-apple-darwin"),
    Some(AotTarget::MacOSX86_64)
  ));

  // macOS ARM64 aliases
  assert!(matches!(
    parse_aot_target("macos-arm64"),
    Some(AotTarget::MacOSArm64)
  ));
  assert!(matches!(
    parse_aot_target("macos-aarch64"),
    Some(AotTarget::MacOSArm64)
  ));
  assert!(matches!(
    parse_aot_target("aarch64-apple-darwin"),
    Some(AotTarget::MacOSArm64)
  ));

  // Windows aliases
  assert!(matches!(
    parse_aot_target("windows"),
    Some(AotTarget::WindowsX86_64)
  ));
  assert!(matches!(
    parse_aot_target("windows-x86_64"),
    Some(AotTarget::WindowsX86_64)
  ));
  assert!(matches!(
    parse_aot_target("x86_64-pc-windows-msvc"),
    Some(AotTarget::WindowsX86_64)
  ));
}

#[test]
fn test_s18_8_parse_aot_target_case_insensitive() {
  assert!(matches!(
    parse_aot_target("LINUX"),
    Some(AotTarget::LinuxX86_64)
  ));
  assert!(matches!(
    parse_aot_target("Linux"),
    Some(AotTarget::LinuxX86_64)
  ));
  assert!(matches!(
    parse_aot_target("MACOS"),
    Some(AotTarget::MacOSX86_64)
  ));
  assert!(matches!(
    parse_aot_target("MacOS-ARM64"),
    Some(AotTarget::MacOSArm64)
  ));
  assert!(matches!(
    parse_aot_target("WINDOWS"),
    Some(AotTarget::WindowsX86_64)
  ));
}

#[test]
fn test_s18_8_parse_aot_target_whitespace() {
  assert!(matches!(
    parse_aot_target("  linux  "),
    Some(AotTarget::LinuxX86_64)
  ));
  assert!(matches!(
    parse_aot_target("\tmacos\t"),
    Some(AotTarget::MacOSX86_64)
  ));
}

#[test]
fn test_s18_8_parse_aot_target_invalid() {
  assert!(parse_aot_target("").is_none());
  assert!(parse_aot_target("unknown").is_none());
  assert!(parse_aot_target("linux-arm64").is_none()); // Not supported yet
  assert!(parse_aot_target("freebsd").is_none());
}

#[test]
fn test_s18_8_aot_target_label() {
  assert_eq!(aot_target_label(AotTarget::LinuxX86_64), "linux-x86_64");
  assert_eq!(aot_target_label(AotTarget::MacOSX86_64), "macos-x86_64");
  assert_eq!(aot_target_label(AotTarget::MacOSArm64), "macos-arm64");
  assert_eq!(aot_target_label(AotTarget::WindowsX86_64), "windows-x86_64");
}

#[test]
fn test_s18_8_emit_target_label_aot() {
  let linux = EmitTarget::Aot(AotTarget::LinuxX86_64);
  assert_eq!(emit_target_label(&linux), "aot:linux-x86_64");

  let macos_arm = EmitTarget::Aot(AotTarget::MacOSArm64);
  assert_eq!(emit_target_label(&macos_arm), "aot:macos-arm64");

  let windows = EmitTarget::Aot(AotTarget::WindowsX86_64);
  assert_eq!(emit_target_label(&windows), "aot:windows-x86_64");
}

#[test]
fn test_s18_8_default_aot_target_deterministic() {
  // default_aot_target should be deterministic based on OS/arch
  let target1 = default_aot_target();
  let target2 = default_aot_target();
  assert_eq!(
    aot_target_label(target1),
    aot_target_label(target2),
    "default_aot_target must be deterministic"
  );
}

#[test]
fn test_s18_8_parse_emit_target_aot_with_target() {
  let aot_linux = parse_emit_target("aot:linux").unwrap();
  assert!(matches!(aot_linux, EmitTarget::Aot(AotTarget::LinuxX86_64)));

  let aot_macos = parse_emit_target("aot:macos-arm64").unwrap();
  assert!(matches!(aot_macos, EmitTarget::Aot(AotTarget::MacOSArm64)));

  let aot_windows = parse_emit_target("aot:windows").unwrap();
  assert!(matches!(
    aot_windows,
    EmitTarget::Aot(AotTarget::WindowsX86_64)
  ));
}

#[test]
fn test_s18_8_parse_emit_target_aot_invalid_target() {
  let err = parse_emit_target("aot:freebsd").unwrap_err();
  assert!(err.to_string().contains("unknown aot target"));

  let err2 = parse_emit_target("aot:").unwrap_err();
  assert!(err2.to_string().contains("unknown aot target"));
}

#[test]
fn test_s18_8_aot_manifest_path_format() {
  // Test that manifest path follows expected format: manifest/{module_name}.json
  let module_name = "test_module";
  let expected_manifest_path = format!("manifest/{}.json", module_name);
  assert_eq!(expected_manifest_path, "manifest/test_module.json");

  // Binary path format: bin/{target.output_name(module_name)}
  let target = AotTarget::LinuxX86_64;
  let binary_name = target.output_name(module_name);
  let expected_bin_path = format!("bin/{}", binary_name);
  assert!(expected_bin_path.starts_with("bin/"));
}

#[test]
fn test_s18_8_aot_output_name_by_target() {
  let module = "myapp";

  // Linux: no extension
  let linux_name = AotTarget::LinuxX86_64.output_name(module);
  assert!(
    !linux_name.contains('.'),
    "Linux binary should have no extension"
  );

  // macOS: no extension
  let macos_name = AotTarget::MacOSX86_64.output_name(module);
  assert!(
    !macos_name.contains('.'),
    "macOS binary should have no extension"
  );

  // Windows: .exe extension
  let windows_name = AotTarget::WindowsX86_64.output_name(module);
  assert!(
    windows_name.ends_with(".exe"),
    "Windows binary should have .exe extension"
  );
}

#[test]
fn test_s18_8_aot_emit_json_structure() {
  // Verify the expected JSON structure from emit_aot
  // This tests the shape without actually calling emit_aot
  let expected_keys = [
    "target",
    "target_triple",
    "binary",
    "manifest",
    "entry_point",
  ];

  let sample_json = serde_json::json!({
      "target": "aot:linux-x86_64",
      "target_triple": "x86_64-unknown-linux-gnu",
      "binary": "bin/module",
      "manifest": "manifest/module.json",
      "entry_point": "pnix_entry",
  });

  for key in &expected_keys {
    assert!(
      sample_json.get(*key).is_some(),
      "emit_aot JSON must have '{}' key",
      key
    );
  }

  // Verify types
  assert!(sample_json["target"].is_string());
  assert!(sample_json["target_triple"].is_string());
  assert!(sample_json["binary"].is_string());
  assert!(sample_json["manifest"].is_string());
  assert!(sample_json["entry_point"].is_string());
}

fn parse_args_for_test(args: &[&str]) -> Result<Args> {
  parse_args_vec(args.iter().map(|arg| arg.to_string()).collect())
}

#[test]
fn gate_absorb_subcommand_parses_without_dist_requirement() {
  let args =
    parse_args_for_test(&["pnix-executor-graph", "gate-absorb", "url"]).expect("parse args");

  assert_eq!(args.gate_absorb, Some(GateAbsorbVerb::Url));
  assert_eq!(args.mode, ExecMode::Run);
  assert!(args.dist.is_none());
}

#[test]
fn gate_absorb_url_flags_are_captured() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "gate-absorb",
    "url",
    "file:///tmp/sample.html",
    "--follow-related",
    "3",
    "--dry-run",
  ])
  .expect("parse args");

  assert_eq!(args.gate_absorb, Some(GateAbsorbVerb::Url));
  assert_eq!(
    args.gate_absorb_subject.as_deref(),
    Some("file:///tmp/sample.html")
  );
  assert_eq!(args.gate_absorb_follow_related, Some(3));
  assert!(args.dry_run);
}

#[test]
fn gate_absorb_follow_related_is_rejected_outside_url() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "gate-absorb",
    "conversation",
    "fixture.json",
    "--follow-related",
    "1",
  ])
  .expect_err("follow-related must be url-only");

  assert!(err
    .to_string()
    .contains("--follow-related is only supported for `pnix gate-absorb url`"));
}

#[test]
fn gate_absorb_file_url_dry_run_hashes_content() {
  let dir = temp_dir("gate-absorb-url");
  let path = dir.join("sample-page.html");
  fs::write(&path, "<html><body>fixture</body></html>").unwrap();
  let url = format!("file://{}", path.display());

  let result = super::gate_absorb::visit_url(&url).expect("visit file url");
  assert_eq!(result.final_url, url);
  assert_eq!(result.content_type, "text/html");
  assert_eq!(result.content, "<html><body>fixture</body></html>");
  assert_eq!(result.content_sha256.len(), 64);

  let receipt =
    super::gate_absorb::build_url_fetch_receipt(&url, &result, result.content.chars().count())
      .expect("build fetch receipt");
  assert_eq!(receipt.artifact_family, "ankh.fetch-receipt");
  assert_eq!(receipt.final_url, url);
  assert_eq!(receipt.status, "fetched-candidate-no-extraction");
  assert_eq!(receipt.content_hash.len(), "sha256:".len() + 64);
  assert!(!receipt.direct_truth_source);
  assert_eq!(receipt.lang, "und");
  assert_eq!(receipt.source_identity_floor_status, "passed");
  assert_eq!(receipt.extraction_status, "not-extracted");
  assert!(receipt
    .next_required_artifacts
    .iter()
    .any(|value| value == "ankh.evidence-bridge"));

  let research =
    super::gate_absorb::build_url_research_evidence(&url, &result, result.content.chars().count())
      .expect("build research evidence ladder");
  assert_eq!(research.artifact_family, "ankh.research-evidence-ladder");
  assert!(!research.direct_truth_source);
  assert!(!research.judgement_ready);
  assert!(!research.promotion_ready);
  assert!(!research.store_mutation);
  assert!(!research.policy_mutation_applied);
  assert_eq!(
    research.extraction_candidate.artifact_family,
    "ankh.extraction-candidate"
  );
  assert_eq!(
    research.extraction_candidate.status,
    "candidate-metadata-only-no-raw-body"
  );
  assert_eq!(
    research.extraction_candidate.extraction_status,
    "metadata-only-no-raw-body"
  );
  assert_eq!(
    research.extraction_candidate.extraction_owner_status,
    "url-extract-rules-deferred"
  );
  assert!(!research.extraction_candidate.raw_body_available);
  assert!(!research.extraction_candidate.raw_text_retained);
  assert!(!research.extraction_candidate.direct_truth_source);
  assert!(!research.extraction_candidate.judgement_ready);
  assert!(!research.extraction_candidate.promotion_ready);
  assert_eq!(
    research.source_risk_floor.artifact_family,
    "ankh.source-risk-floor"
  );
  assert_eq!(
    research.source_risk_floor.status,
    "candidate-risk-floor-no-trust-score"
  );
  assert_eq!(
    research.source_risk_floor.source_trust_status,
    "unscored-policy-floor-only"
  );
  assert_eq!(
    research.source_risk_floor.trust_score_owner_status,
    "deferred-no-score"
  );
  assert_eq!(
    research.source_risk_floor.risk_score_owner_status,
    "deferred-no-score"
  );
  assert_eq!(
    research.source_risk_floor.benchmark_contamination_policy,
    "benchmark-contamination-review-before-use"
  );
  assert_eq!(
    research.source_risk_floor.adversarial_source_policy,
    "adversarial-source-risk-review-before-promotion"
  );
  assert!(!research.source_risk_floor.source_text_retained);
  assert!(!research.source_risk_floor.direct_truth_source);
  assert!(!research.source_risk_floor.judgement_ready);
  assert!(!research.source_risk_floor.promotion_ready);
  assert!(!research.source_risk_floor.policy_mutation_applied);
  assert!(!research.source_risk_floor.store_mutation);
  assert_eq!(
    research.truth_regime_classification.artifact_family,
    "ankh.truth-regime-classification"
  );
  assert_eq!(
    research.truth_regime_classification.truth_regime,
    "interpretive"
  );
  assert_eq!(
    research
      .truth_regime_classification
      .classification_confidence,
    "low-metadata-only"
  );
  assert!(!research.truth_regime_classification.direct_truth_source);
  assert!(!research.truth_regime_classification.judgement_ready);
  assert!(!research.truth_regime_classification.promotion_ready);
  assert_eq!(
    research.evidence_bridge.artifact_family,
    "ankh.evidence-bridge"
  );
  assert_eq!(
    research.evidence_bridge.status,
    "candidate-evidence-bridge-no-promotion"
  );
  assert_eq!(
    research.evidence_bridge.bridge_status,
    "candidate-only-judgement-required"
  );
  assert!(!research.evidence_bridge.direct_truth_source);
  assert!(!research.evidence_bridge.judgement_ready);
  assert!(!research.evidence_bridge.promotion_ready);
  assert_eq!(
    research.evidence_bridge.promotion_boundary,
    "research-judgement-required-before-knowledge-promotion"
  );
  assert_eq!(
    research.knowledge_promotion_candidate.artifact_family,
    "ankh.knowledge-promotion-candidate"
  );
  assert_eq!(
    research.knowledge_promotion_candidate.promotion_status,
    "held-pending-research-judgement"
  );
  assert_eq!(
    research.knowledge_promotion_candidate.verification_policy,
    "verification-only-accepted"
  );
  assert!(!research.knowledge_promotion_candidate.auto_promote_allowed);
  assert!(
    !research
      .knowledge_promotion_candidate
      .candidate_to_accepted_direct_allowed
  );
  assert!(
    !research
      .knowledge_promotion_candidate
      .candidate_to_candidate_verification_allowed
  );
  assert!(!research.knowledge_promotion_candidate.judgement_ready);
  assert!(!research.knowledge_promotion_candidate.promotion_ready);
  assert_eq!(
    research.research_judgement.artifact_family,
    "ankh.research-judgement"
  );
  assert_eq!(research.research_judgement.judgement_action, "held");
  assert_eq!(
    research.research_judgement.status,
    "held-pending-independent-verification"
  );
  assert!(!research.research_judgement.accepted);
  assert!(!research.research_judgement.rejected);
  assert!(!research.research_judgement.promotion_approved);
  assert!(!research.research_judgement.policy_mutation_applied);
  assert!(!research.research_judgement.store_mutation);
  assert_eq!(
    research.research_revision_receipt.artifact_family,
    "ankh.research-revision-receipt"
  );
  assert_eq!(
    research.research_revision_receipt.revision_status,
    "held-no-policy-mutation"
  );
  assert_eq!(
    research.research_revision_receipt.learning_loop_status,
    "held-return-to-policy-candidate"
  );
  assert!(!research.research_revision_receipt.policy_mutation_applied);
  assert!(!research.research_revision_receipt.store_mutation);
  assert!(
    !research
      .research_revision_receipt
      .knowledge_promotion_applied
  );
}

#[test]
fn gate_absorb_conversation_parse_and_vocab_summary_work() {
  let dir = temp_dir("gate-absorb-conversation");
  let path = dir.join("ko-en-ja-sample.json");
  fs::write(
    &path,
    r#"[
  {"speaker": "A", "text": "안녕하세요, 오늘 기분 어떠세요?"},
  {"speaker": "B", "text": "Hi! I'm doing great, thanks for asking."},
  {"speaker": "A", "text": "こんにちは、元気ですか？"},
  {"speaker": "B", "text": "元気です。あなたは？"},
  {"speaker": "A", "text": "저도 잘 지내요. 오늘 날씨 좋네요."},
  {"speaker": "B", "text": "Yes, perfect weather for a walk!"}
]"#,
  )
  .unwrap();

  let turns = super::gate_absorb::parse_transcript_file(&path).expect("parse transcript");
  assert_eq!(turns.len(), 6);
  assert_eq!(
    turns[0].language,
    super::gate_absorb::GateAbsorbLanguage::Ko
  );
  assert_eq!(
    turns[1].language,
    super::gate_absorb::GateAbsorbLanguage::En
  );
  assert_eq!(
    turns[2].language,
    super::gate_absorb::GateAbsorbLanguage::Ja
  );

  let token_total: usize = turns
    .iter()
    .map(|turn| super::gate_absorb::extract_vocab(turn).tokens.len())
    .sum();
  assert!(token_total >= 10);
}

#[test]
fn gate_forward_subcommand_parses_without_dist_requirement() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-forward"]).expect("parse args");

  assert_eq!(args.gate_forward, Some(GateForwardVerb::Run));
  assert_eq!(args.mode, ExecMode::Run);
  assert!(args.dist.is_none());
}

#[test]
fn gate_forward_flags_are_captured() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "gate-forward",
    "--limit",
    "7",
    "--kind",
    "note-atom",
    "--dry-run",
    "--reset",
    "--url",
    "http://127.0.0.1:9999",
  ])
  .expect("parse args");

  assert_eq!(args.gate_forward, Some(GateForwardVerb::Run));
  assert_eq!(args.gate_forward_limit, Some(7));
  assert_eq!(args.gate_forward_kind.as_deref(), Some("note-atom"));
  assert!(args.dry_run);
  assert!(args.gate_forward_reset);
  assert_eq!(
    args.gate_forward_url.as_deref(),
    Some("http://127.0.0.1:9999")
  );
}

#[test]
fn gate_forward_kind_is_rejected_outside_forward_lane() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "gate-read",
    "query-context",
    "--kind",
    "note-atom",
  ])
  .expect_err("gate-forward-only flag must fail outside lane");

  assert!(err
    .to_string()
    .contains("--kind is only supported for `pnix gate-read candidates`"));
}

#[test]
fn gate_read_recent_events_flags_are_captured() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "gate-read",
    "recent-events",
    "--limit",
    "7",
    "--event-type",
    "Stop",
    "--event-type",
    "PostToolUse",
  ])
  .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::RecentEvents));
  assert_eq!(args.gate_read_limit, Some(7));
  assert_eq!(
    args.gate_read_event_types,
    vec!["Stop".to_string(), "PostToolUse".to_string()]
  );
}

#[test]
fn gate_read_candidates_flags_are_captured() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "gate-read",
    "candidates",
    "--limit",
    "5",
    "--kind",
    "observation-atom",
  ])
  .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::Candidates));
  assert_eq!(args.gate_read_limit, Some(5));
  assert_eq!(args.gate_read_kind.as_deref(), Some("observation-atom"));
}

#[test]
fn gate_read_brain_ankh_policy_flags_are_captured() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "gate-read",
    "brain-ankh-policy",
    "--limit",
    "8",
  ])
  .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::BrainAnkhPolicy));
  assert_eq!(args.gate_read_limit, Some(8));
}

#[test]
fn gate_read_state_sink_contract_parses() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-read", "state-sink-contract"])
    .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::StateSinkContract));
}

#[test]
fn gate_read_ontology_coverage_parses() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-read", "ontology-coverage"])
    .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::OntologyCoverage));
}

#[test]
fn gate_read_meaning_bridges_parses() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-read", "meaning-bridges"])
    .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::MeaningBridges));
}

#[test]
fn gate_read_self_capabilities_parses() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-read", "self-capabilities"])
    .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::SelfCapabilities));
}

#[test]
fn gate_read_meta_protocols_parses() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-read", "meta-protocols"])
    .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::MetaProtocols));
}

#[test]
fn gate_read_lineage_floor_parses() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "gate-read",
    "lineage-floor",
    "--limit",
    "7",
  ])
  .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::LineageFloor));
  assert_eq!(args.gate_read_limit, Some(7));
}

#[test]
fn gate_read_lift_rule_coverage_parses() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-read", "lift-rule-coverage"])
    .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::LiftRuleCoverage));
}

#[test]
fn gate_read_store_budget_parses() {
  let args =
    parse_args_for_test(&["pnix-executor-graph", "gate-read", "store-budget"]).expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::StoreBudget));
}

#[test]
fn gate_read_artifact_ref_ratio_parses() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-read", "artifact-ref-ratio"])
    .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::ArtifactRefRatio));
}

#[test]
fn gate_read_storage_telemetry_parses() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-read", "storage-telemetry"])
    .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::StorageTelemetry));
}

#[test]
fn gate_read_provenance_floor_parses() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-read", "provenance-floor"])
    .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::ProvenanceFloor));
}

#[test]
fn gate_read_unsupported_kind_floor_parses() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-read", "unsupported-kind-floor"])
    .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::UnsupportedKindFloor));
}

#[test]
fn gate_read_brain_bundle_contract_parses() {
  let args = parse_args_for_test(&["pnix-executor-graph", "gate-read", "brain-bundle-contract"])
    .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::BrainBundleContract));
}

#[test]
fn gate_read_validate_brain_bundle_flags_are_captured() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "gate-read",
    "validate-brain-bundle",
    "--path",
    "docs/puck/examples/portable-domain-bundle.v0.1.json",
    "--proof-path",
    "docs/puck/examples/portable-domain-bundle.proof.v0.1.json",
    "--schema-path",
    "docs/puck/capability-manifest-v0.1.schema.json",
    "--expected-bundle-kind",
    "portable-domain-bundle",
    "--expected-lobe-profile",
    "domain-lobe",
    "--expected-proof-kind",
    "PortableDomainBundleProof",
  ])
  .expect("parse args");

  assert_eq!(args.gate_read, Some(GateReadVerb::ValidateBrainBundle));
  assert_eq!(
    args.gate_read_path.as_deref(),
    Some("docs/puck/examples/portable-domain-bundle.v0.1.json")
  );
  assert_eq!(
    args.gate_read_proof_path.as_deref(),
    Some("docs/puck/examples/portable-domain-bundle.proof.v0.1.json")
  );
  assert_eq!(
    args.gate_read_schema_path.as_deref(),
    Some("docs/puck/capability-manifest-v0.1.schema.json")
  );
  assert_eq!(
    args.gate_read_expected_bundle_kind.as_deref(),
    Some("portable-domain-bundle")
  );
  assert_eq!(
    args.gate_read_expected_lobe_profile.as_deref(),
    Some("domain-lobe")
  );
  assert_eq!(
    args.gate_read_expected_proof_kind.as_deref(),
    Some("PortableDomainBundleProof")
  );
}

#[test]
fn gate_read_validate_brain_bundle_requires_path() {
  let err = parse_args_for_test(&["pnix-executor-graph", "gate-read", "validate-brain-bundle"])
    .expect_err("validate-brain-bundle must require --path");

  assert!(err
    .to_string()
    .contains("--path is required for `pnix gate-read validate-brain-bundle`"));
}

#[test]
fn gate_read_event_type_is_rejected_outside_recent_events_lane() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "gate-read",
    "query-context",
    "--event-type",
    "Stop",
  ])
  .expect_err("recent-events-only flag must fail outside lane");

  assert!(err
    .to_string()
    .contains("--event-type/--event_type is only supported for `pnix gate-read recent-events`"));
}

#[test]
fn agent_subcommand_parses_without_dist_requirement() {
  let args =
    parse_args_for_test(&["pnix-executor-graph", "coding-agent", "ask"]).expect("parse args");

  assert_eq!(args.agent, Some(AgentVerb::Ask));
  assert_eq!(args.mode, ExecMode::Run);
  assert!(args.dist.is_none());
}

#[test]
fn agent_unknown_verb_is_rejected() {
  let err = parse_args_for_test(&["pnix-executor-graph", "coding-agent", "invent"])
    .expect_err("unknown verb must fail");

  assert!(err.to_string().contains("unknown coding-agent verb"));
}

#[test]
fn coding_agent_request_flags_are_captured() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "plan",
    "--request",
    "fix failing tests",
    "--target-path",
    "src/lib.rs",
    "--project-pack-root",
    "packs/project-a",
    "--history-pack-root",
    "packs/history-a",
    "--approved-command",
    "cargo test",
    "--forbidden-path",
    "secrets/",
    "--workspace-policy",
    "patch+test-allowed",
    "--current-plan-ref",
    "plan:123",
    "--rollback-handle-ref",
    "rollback:abc",
    "--last-verification-ref",
    "verify:xyz",
    "--agent-request-out",
    "runtime/request.json",
    "--agent-plan-out",
    "runtime/plan.json",
  ])
  .expect("parse args");

  assert_eq!(args.agent, Some(AgentVerb::Plan));
  assert_eq!(args.agent_request.as_deref(), Some("fix failing tests"));
  assert_eq!(args.agent_target_paths.len(), 1);
  assert_eq!(
    args.agent_target_paths[0],
    std::path::PathBuf::from("src/lib.rs")
  );
  assert_eq!(
    args.agent_project_pack_roots,
    vec![std::path::PathBuf::from("packs/project-a")]
  );
  assert_eq!(
    args.agent_history_pack_roots,
    vec![std::path::PathBuf::from("packs/history-a")]
  );
  assert_eq!(args.agent_approved_commands, vec!["cargo test"]);
  assert_eq!(args.agent_forbidden_paths.len(), 1);
  assert_eq!(
    args.agent_forbidden_paths[0],
    std::path::PathBuf::from("secrets/")
  );
  assert_eq!(args.agent_policy_bits, vec!["patch+test-allowed"]);
  assert_eq!(args.agent_current_plan_ref.as_deref(), Some("plan:123"));
  assert_eq!(
    args.agent_rollback_handle_ref.as_deref(),
    Some("rollback:abc")
  );
  assert_eq!(
    args.agent_last_verification_ref.as_deref(),
    Some("verify:xyz")
  );
  assert_eq!(
    args.agent_request_out.as_deref(),
    Some(std::path::Path::new("runtime/request.json"))
  );
  assert_eq!(
    args.agent_plan_out.as_deref(),
    Some(std::path::Path::new("runtime/plan.json"))
  );
}

#[test]
fn coding_agent_patch_out_is_captured_for_patch() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "cli.rs 버그 수정",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--agent-patch-out",
    "runtime/patch.json",
  ])
  .expect("parse args");

  assert_eq!(args.agent, Some(AgentVerb::Patch));
  assert_eq!(
    args.agent_patch_out.as_deref(),
    Some(std::path::Path::new("runtime/patch.json"))
  );
}

#[test]
fn coding_agent_candidate_patch_is_captured_for_patch() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "provider patch candidate 검토",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--candidate-patch",
    "runtime/provider.patch",
  ])
  .expect("parse args");

  assert_eq!(args.agent, Some(AgentVerb::Patch));
  assert_eq!(
    args.agent_candidate_patch.as_deref(),
    Some(std::path::Path::new("runtime/provider.patch"))
  );
}

#[test]
fn coding_agent_provider_feedback_request_ref_marks_candidate_patch_lineage() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "provider feedback response 검토",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--candidate-patch",
    "runtime/provider-response.patch",
    "--provider-feedback-request-ref",
    "coding.provider-feedback-request::abc",
  ])
  .expect("parse args");

  assert_eq!(
    args.agent_candidate_patch.as_deref(),
    Some(std::path::Path::new("runtime/provider-response.patch"))
  );
  assert_eq!(
    args.agent_provider_feedback_request_ref.as_deref(),
    Some("coding.provider-feedback-request::abc")
  );
}

#[test]
fn coding_agent_candidate_patch_is_rejected_for_non_patch_verbs() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "verify",
    "--request",
    "provider patch candidate 검토",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--candidate-patch",
    "runtime/provider.patch",
  ])
  .expect_err("candidate patch must be patch-only");

  assert!(err
    .to_string()
    .contains("--candidate-patch is only supported"));
}

#[test]
fn coding_agent_verify_out_is_captured_for_verify() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "verify",
    "--request",
    "cli.rs 수정 검증",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--agent-verify-out",
    "runtime/verify.json",
  ])
  .expect("parse args");

  assert_eq!(args.agent, Some(AgentVerb::Verify));
  assert_eq!(
    args.agent_verify_out.as_deref(),
    Some(std::path::Path::new("runtime/verify.json"))
  );
}

#[test]
fn coding_agent_rollback_out_is_captured_for_rollback() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "rollback",
    "--request",
    "cli.rs 수정 롤백 준비",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--agent-rollback-out",
    "runtime/rollback.json",
  ])
  .expect("parse args");

  assert_eq!(args.agent, Some(AgentVerb::Rollback));
  assert_eq!(
    args.agent_rollback_out.as_deref(),
    Some(std::path::Path::new("runtime/rollback.json"))
  );
}

#[test]
fn coding_agent_retention_verb_is_captured() {
  let args =
    parse_args_for_test(&["pnix-executor-graph", "coding-agent", "retention"]).expect("parse args");

  assert_eq!(args.agent, Some(AgentVerb::Retention));
}

#[test]
fn coding_agent_retention_rejects_request_artifact_flags() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "retention",
    "--request",
    "compact old coding memory",
  ])
  .expect_err("retention must not accept request-plane flags");

  assert!(err
    .to_string()
    .contains("request/workspace artifact flags are not supported"));
}

#[test]
fn coding_agent_positional_request_is_captured() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "plan",
    "이 테스트 실패 원인 분석",
    "--target-path",
    "tests/ui/math_rendering.rs",
  ])
  .expect("parse args");

  assert_eq!(args.agent, Some(AgentVerb::Plan));
  assert_eq!(
    args.agent_request.as_deref(),
    Some("이 테스트 실패 원인 분석")
  );
  assert_eq!(args.agent_target_paths.len(), 1);
}

#[test]
fn coding_agent_flags_are_rejected_without_subcommand() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "run",
    "--dist",
    "dist",
    "--request",
    "hello",
  ])
  .expect_err("coding-agent-only flags must fail outside coding-agent");

  assert!(err
    .to_string()
    .contains("only supported for `pnix coding-agent ...`"));
}

#[test]
fn coding_agent_plan_artifact_is_bounded_and_deterministic_shape() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "plan",
    "--request",
    "crates/pnix-executor-graph/src/cli.rs 실패 원인 분석",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--workspace-policy",
    "read-only-inspection",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Plan).expect("build request");
  let plan = build_coding_agent_plan(&args, request, Some("runtime/request.json".to_string()));

  assert_eq!(plan.artifact_family, "coding.plan");
  assert_eq!(plan.phase, "CAX.1c");
  assert_eq!(plan.verb, "plan");
  assert_eq!(
    plan.request_artifact_ref.as_deref(),
    Some("runtime/request.json")
  );
  assert_eq!(plan.status.progress_status, "계획제안완료");
  assert_eq!(plan.status.result_status, "부분완료");
  assert_eq!(plan.failure_policy, "fail-closed-before-patch-apply");
  assert_eq!(
    plan.expected_verification,
    vec!["cargo check -p pnix-executor-graph".to_string()]
  );
  assert_eq!(plan.bounded_step_family.len(), 5);
  assert_eq!(
    plan.bounded_step_family[0].step_family,
    "inspect-workspace-snapshot"
  );
  assert_eq!(
    plan.bounded_step_family[1].step_family,
    "inspect-target-scope"
  );
  assert_eq!(
    plan.bounded_step_family[2].step_family,
    "prepare-approved-verification"
  );
  assert!(matches!(
    plan.bounded_step_family[3].step_family,
    "record-manual-evidence-uncertainty" | "review-joined-manual-evidence"
  ));
  assert_eq!(
    plan.bounded_step_family[4].step_family,
    "emit-patch-proposal-before-write"
  );
  assert!(plan
    .current_interpretation
    .contains("targeted verification planning"));
  assert_eq!(
    plan.request.context_pack.artifact_family,
    "coding.context-pack"
  );
  assert_eq!(
    plan.request.context_pack.close_status,
    "bounded-read-only-pack"
  );
  assert!(plan
    .request
    .context_pack
    .context_pack_ref
    .starts_with("coding.context-pack::"));
  assert!(plan
    .request
    .context_pack
    .section_family
    .iter()
    .any(|section| section.section_family == "repo-graph-seed"));
  assert_eq!(
    plan.interpretation_set.artifact_family,
    "coding.interpretation-set"
  );
  assert_eq!(
    plan.interpretation_set.selected_interpretation,
    plan.current_interpretation
  );
  assert_eq!(plan.judgement.artifact_family, "coding.judgement");
  assert_eq!(plan.judgement.decision, "continue-to-patch-proposal");
  assert!(plan.judgement.blocked_reasons.is_empty());
  assert!(plan
    .judgement
    .required_next_artifacts
    .contains(&"coding.patch-proposal"));
  assert_eq!(plan.execution_plan.artifact_family, "coding.execution-plan");
  assert!(plan
    .execution_plan
    .execution_plan_ref
    .starts_with("coding.execution-plan::"));
  assert_eq!(plan.execution_plan.bounded_step_family.len(), 5);
  assert_eq!(plan.execution_plan.execution_requests.len(), 1);
  assert_eq!(plan.execution_plan.language_verify_targets.len(), 1);
  assert_eq!(
    plan.execution_plan.language_verify_targets[0].language,
    "rust"
  );
  assert_eq!(
    plan.execution_plan.execution_requests[0].artifact_family,
    "coding.execution-request"
  );
  assert_eq!(
    plan.execution_plan.execution_requests[0].permission_status,
    "declared-approved-command-not-executed"
  );
  assert!(plan.execution_plan.execution_requests[0]
    .candidate_verify_target_refs
    .iter()
    .any(|target_ref| target_ref.starts_with("pnix.verify-target-record::")));
  assert!(plan.execution_plan.execution_requests[0]
    .candidate_command_refs
    .iter()
    .any(|command_ref| command_ref == "candidate:rust:cargo check -p pnix-executor-graph"));
  assert_eq!(
    plan.request.grounding_seed.scan_mode,
    "explicit-target-scope"
  );
  assert_eq!(plan.request.grounding_seed.entries.len(), 1);
  assert_eq!(
    plan.request.grounding_seed.entries[0].path,
    "crates/pnix-executor-graph/src/cli.rs"
  );
  assert_eq!(plan.request.grounding_seed.entries[0].language, "rust");
  assert_eq!(
    plan.request.grounding_seed.entries[0].parser_backend,
    "pnix-lsp:FallbackLineBased"
  );
  assert_eq!(
    plan.request.grounding_seed.entries[0].parser_capability,
    "emergency-compatibility-only"
  );
  assert_eq!(
    plan.request.repo_graph_seed.graph_owner,
    "pnix-lsp::CpgBuilder::summarize_project_graph"
  );
  assert_eq!(
    plan.request.repo_graph_seed.bundle_scope,
    "multi-file-bounded-project-summary"
  );
  assert_eq!(
    plan.request.repo_graph_seed.graph_capability,
    "multi-file-bounded-project-summary"
  );
  assert_eq!(
    plan.request.repo_graph_seed.project_graph_status,
    "multi-file-bounded-fallback-parser-only"
  );
  assert_eq!(
    plan.request.repo_graph_seed.seto_enrichment_state,
    "seto-disabled-optional"
  );
  assert!(plan.request.repo_graph_seed.files.len() >= 2);
  assert!(plan
    .request
    .repo_graph_seed
    .files
    .iter()
    .any(|file| file.file_anchor == "crates/pnix-executor-graph/src/cli.rs#file"));
  assert!(plan
    .request
    .repo_graph_seed
    .files
    .iter()
    .all(|file| file.language == "rust"));
  assert!(plan
    .request
    .repo_graph_seed
    .files
    .iter()
    .all(|file| file.parser_capability == "emergency-compatibility-only"));
  assert!(plan.request.repo_graph_seed.files.iter().any(|file| file
    .symbol_nodes
    .iter()
    .any(|symbol| symbol.name == "run_cli")));
  assert!(plan
    .request
    .repo_graph_seed
    .files
    .iter()
    .any(|file| !file.reference_edges.is_empty()));
  assert_eq!(
    plan
      .request
      .repo_graph_seed
      .incremental_refresh
      .refresh_mode,
    "changed-file-plus-related-bounded-refresh"
  );
  assert_eq!(
    plan
      .request
      .repo_graph_seed
      .incremental_refresh
      .changed_files,
    vec!["crates/pnix-executor-graph/src/cli.rs".to_string()]
  );
  assert!(plan
    .request
    .repo_graph_seed
    .incremental_refresh
    .refresh_batch
    .contains(&"crates/pnix-executor-graph/src/cli.rs".to_string()));
  assert_eq!(
    plan.request.manual_evidence_seed.join_owner,
    "doghouse-core::docset_query::query_joined_docset_evidence"
  );
  assert_eq!(
    plan.request.manual_evidence_seed.join_policy,
    "manual-hit-never-justifies-patch-without-file-symbol-project-join"
  );
  assert!(plan
    .request
    .manual_evidence_seed
    .uncertainty_receipts
    .iter()
    .any(|receipt| receipt.starts_with("repo-graph-status:")));
}

#[test]
fn coding_agent_patch_proposal_is_bounded_and_deterministic_shape() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "crates/pnix-executor-graph/src/cli.rs 버그 수정",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--workspace-policy",
    "patch+test-allowed",
    "--current-plan-ref",
    "plan:cli-fix",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Patch).expect("build request");
  let proposal =
    build_coding_agent_patch_proposal(&args, request, Some("runtime/request.json".to_string()));

  assert_eq!(proposal.artifact_family, "coding.patch-proposal");
  assert_eq!(proposal.phase, "CAX.3a-partial");
  assert_eq!(proposal.verb, "patch");
  assert_eq!(
    proposal.request_artifact_ref.as_deref(),
    Some("runtime/request.json")
  );
  assert_eq!(proposal.current_plan_ref.as_deref(), Some("plan:cli-fix"));
  assert_eq!(proposal.status.progress_status, "패치제안완료");
  assert_eq!(proposal.status.result_status, "부분완료");
  assert_eq!(proposal.edit_family, "bugfix");
  assert_eq!(proposal.risk_class, "medium");
  assert_eq!(
    proposal.expected_verify_ref,
    vec!["cargo check -p pnix-executor-graph".to_string()]
  );
  assert_eq!(
    proposal.target_paths,
    vec!["crates/pnix-executor-graph/src/cli.rs".to_string()]
  );
  assert!(proposal.diff_ref.starts_with("coding.diff::proposal::"));
  assert!(proposal
    .effect_classes
    .contains(&"workspace-file-write:intent-only".to_string()));
  assert!(proposal
    .effect_classes
    .contains(&"verification-command:intent-only".to_string()));
  assert_eq!(proposal.apply_intent.intent_family, "coding.apply-intent");
  assert_eq!(
    proposal.apply_intent.apply_status,
    "proposal-only-not-applied"
  );
  assert!(proposal.apply_intent.apply_artifact_ref.is_none());
  assert!(proposal.apply_result.is_none());
  assert!(proposal.generated_patch_candidate.is_none());
  assert!(proposal.apply_intent.separated_from_proposal);
  assert_eq!(
    proposal.apply_intent.effect_classes,
    proposal.effect_classes
  );
  assert_eq!(
    proposal.semantic_review.artifact_family,
    "coding.semantic-patch-review"
  );
  assert_eq!(
    proposal.context_demand_replay.artifact_family,
    "coding.context-demand-replay"
  );
  assert_eq!(proposal.context_demand_replay.phase, "CAX.5g");
  assert_eq!(
    proposal.repair_recipe_replay.artifact_family,
    "coding.repair-recipe-replay"
  );
  assert_eq!(proposal.repair_recipe_replay.phase, "CAX.5h");
  assert_eq!(
    proposal.repair_recipe_replay.promotion_boundary,
    "candidate-only-not-patch-generator"
  );
  assert_eq!(proposal.semantic_review.phase, "CAX.5f");
  assert_eq!(
    proposal.semantic_review.review_status,
    "candidate-review-not-promoted"
  );
  assert_eq!(proposal.semantic_review.diff_ref, proposal.diff_ref);
  assert_eq!(
    proposal.semantic_review.meaning_impact_diff.artifact_family,
    "coding.meaning-impact-diff"
  );
  assert!(proposal
    .semantic_review
    .meaning_impact_diff
    .meaning_classes
    .iter()
    .any(|class| class == "rust-module-or-item-meaning"));
  assert!(proposal
    .semantic_review
    .patch_decision_link
    .evidence_refs
    .iter()
    .any(|evidence| evidence.starts_with("pnix.verify-target-record::")));
  assert_eq!(
    proposal.semantic_review.narrative_regression.proof_boundary,
    "review-candidate-not-promotion"
  );
  assert_eq!(
    proposal.request.grounding_seed.scan_mode,
    "explicit-target-scope"
  );
  assert_eq!(
    proposal.request.repo_graph_seed.graph_owner,
    "pnix-lsp::CpgBuilder::summarize_project_graph"
  );
  assert_eq!(
    proposal.request.manual_evidence_seed.join_policy,
    "manual-hit-never-justifies-patch-without-file-symbol-project-join"
  );
}

#[test]
fn coding_agent_provider_feedback_request_ref_requires_candidate_patch() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "provider feedback response 검토",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--provider-feedback-request-ref",
    "coding.provider-feedback-request::abc",
  ])
  .expect_err("provider feedback ref requires candidate patch");
  assert!(err
    .to_string()
    .contains("--provider-feedback-request-ref requires --candidate-patch"));
}

#[test]
fn coding_agent_patch_quarantines_generated_patch_candidate_without_applying() {
  let patch_dir = temp_dir("coding-agent-generated-patch");
  let patch_path = patch_dir.join("provider.patch");
  fs::write(
    &patch_path,
    "\
--- a/crates/pnix-executor-graph/src/cli.rs
+++ b/crates/pnix-executor-graph/src/cli.rs
@@ -1,1 +1,1 @@
-//! CLI 실행기: 명령행 인터페이스
+//! CLI 실행기: 명령행 인터페이스
",
  )
  .expect("write provider patch");
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "provider generated patch candidate 검토",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--candidate-patch",
    patch_path.to_str().expect("utf8 patch path"),
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Patch).expect("build request");
  let proposal =
    build_coding_agent_patch_proposal(&args, request, Some("runtime/request.json".to_string()));
  let candidate = proposal
    .generated_patch_candidate
    .as_ref()
    .expect("generated patch candidate");
  let review_receipt = proposal
    .generated_patch_review_receipt
    .as_ref()
    .expect("generated patch review receipt");

  assert!(proposal.apply_result.is_none());
  assert!(proposal.provider_feedback_request.is_none());
  assert!(proposal.feedback_retry_guard.is_none());
  assert!(proposal
    .effect_classes
    .contains(&"provider-patch-candidate:quarantined".to_string()));
  assert_eq!(
    candidate.artifact_family,
    "coding.generated-patch-candidate"
  );
  assert_eq!(candidate.phase, "CAX.5i");
  assert_eq!(
    candidate.quarantine_status,
    "quarantined-provider-patch-candidate"
  );
  assert_eq!(
    candidate.promotion_boundary,
    "candidate-only-not-apply-owner"
  );
  assert_eq!(
    candidate.parsed_target_paths,
    vec!["crates/pnix-executor-graph/src/cli.rs".to_string()]
  );
  assert!(candidate.rejected_target_paths.is_empty());
  assert!(candidate
    .required_next_artifacts
    .contains(&"explicit-apply-result-required-for-mutation".to_string()));
  assert!(candidate
    .proof_refs
    .contains(&"direct-apply:forbidden".to_string()));
  assert_eq!(
    review_receipt.artifact_family,
    "coding.generated-patch-review-receipt"
  );
  assert_eq!(review_receipt.phase, "CAX.5j");
  assert_eq!(
    review_receipt.review_status,
    "candidate-reviewed-awaiting-explicit-apply-result"
  );
  assert_eq!(review_receipt.candidate_ref, candidate.candidate_ref);
  assert!(review_receipt.diagnostic_records.is_empty());
  assert!(review_receipt.context_demands.is_empty());
  assert!(review_receipt
    .proof_refs
    .contains(&"direct-apply:forbidden".to_string()));
  assert!(proposal.semantic_review.proof_refs.iter().any(|proof| {
    proof == &format!("generated-patch-candidate-ref:{}", candidate.candidate_ref)
  }));
  assert!(proposal.semantic_review.proof_refs.iter().any(|proof| {
    proof == &format!("generated-patch-review-ref:{}", review_receipt.review_ref)
  }));
  assert!(proposal
    .semantic_review
    .patch_decision_link
    .evidence_refs
    .iter()
    .any(|evidence| {
      evidence == &format!("generated-patch-candidate-ref:{}", candidate.candidate_ref)
    }));
}

#[cfg(feature = "doghouse")]
#[test]
fn coding_agent_generated_patch_review_receipt_lowers_mismatch_to_context_demands() {
  let patch_dir = temp_dir("coding-agent-generated-patch-review");
  let patch_path = patch_dir.join("provider.patch");
  fs::write(
    &patch_path,
    "\
--- a/README.md
+++ b/README.md
@@ -1,1 +1,1 @@
-old
+new
",
  )
  .expect("write provider patch");
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "provider generated patch candidate mismatch 검토",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--candidate-patch",
    patch_path.to_str().expect("utf8 patch path"),
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Patch).expect("build request");
  let proposal =
    build_coding_agent_patch_proposal(&args, request, Some("runtime/request.json".to_string()));
  let candidate = proposal
    .generated_patch_candidate
    .as_ref()
    .expect("generated patch candidate");
  let review_receipt = proposal
    .generated_patch_review_receipt
    .as_ref()
    .expect("generated patch review receipt");
  let feedback_request = proposal
    .provider_feedback_request
    .as_ref()
    .expect("provider feedback request");
  assert!(proposal.feedback_retry_guard.is_none());

  assert_eq!(candidate.quarantine_status, "quarantined-target-mismatch");
  assert_eq!(
    candidate.rejected_target_paths,
    vec!["README.md".to_string()]
  );
  assert_eq!(
    review_receipt.review_status,
    "candidate-review-context-required"
  );
  assert_eq!(review_receipt.diagnostic_records.len(), 2);
  assert!(review_receipt
    .diagnostic_records
    .iter()
    .any(|diagnostic| diagnostic.diagnostic_family == "generated-patch-target-mismatch"));
  assert!(review_receipt
    .diagnostic_records
    .iter()
    .any(|diagnostic| diagnostic.diagnostic_family == "generated-patch-verification-missing"));
  assert!(review_receipt
    .failure_pattern_matches
    .iter()
    .any(|pattern| pattern.pattern_key == "generated-patch-target-mismatch"));
  assert!(review_receipt
    .context_demands
    .iter()
    .any(|demand| demand.demand_family == "generated-patch-target-scope-required"));
  assert!(review_receipt
    .context_demands
    .iter()
    .any(|demand| demand.demand_family == "generated-patch-verification-required"));
  assert!(review_receipt
    .required_next_artifacts
    .contains(&"revised-generated-patch-candidate".to_string()));
  assert!(proposal
    .semantic_review
    .patch_decision_link
    .evidence_refs
    .iter()
    .any(|evidence| evidence.starts_with("generated-patch-context-demand:")));
  assert_eq!(
    feedback_request.artifact_family,
    "coding.provider-feedback-request"
  );
  assert_eq!(feedback_request.phase, "CAX.5k");
  assert_eq!(
    feedback_request.source_review_ref,
    review_receipt.review_ref
  );
  assert_eq!(
    feedback_request.source_candidate_ref,
    candidate.candidate_ref
  );
  assert_eq!(
    feedback_request.context_demand_refs.len(),
    review_receipt.context_demands.len()
  );
  assert_eq!(
    feedback_request.feedback_packets.len(),
    review_receipt.context_demands.len()
  );
  assert!(feedback_request
    .feedback_packets
    .iter()
    .all(|packet| packet.requested_output == "revised-generated-patch-candidate"));
  assert!(feedback_request
    .feedback_packets
    .iter()
    .all(|packet| packet.truth_boundary == "provider-output-is-candidate-not-proof"));
  assert!(feedback_request
    .forbidden_effects
    .contains(&"provider-auto-apply".to_string()));
  assert!(proposal.semantic_review.proof_refs.iter().any(|proof| {
    proof
      == &format!(
        "provider-feedback-request-ref:{}",
        feedback_request.request_ref
      )
  }));
  assert!(proposal
    .semantic_review
    .patch_decision_link
    .evidence_refs
    .iter()
    .any(|evidence| evidence.starts_with("provider-feedback-packet:")));

  let store_dir = temp_dir("coding-memory-generated-patch-review");
  let store_path = store_dir.join("doghouse.redb");
  let artifact_id = persist_coding_memory_artifact_to_store(
    &store_path,
    review_receipt.artifact_family,
    Some(make_repo_snapshot_ref(&proposal.request.workspace)),
    review_receipt.target_paths.clone(),
    proposal.request.workspace.approved_commands.clone(),
    build_coding_memory_related_refs(
      Option::<&std::path::PathBuf>::None,
      [
        Some(proposal.diff_ref.as_str()),
        Some(review_receipt.candidate_ref.as_str()),
        Some(review_receipt.review_ref.as_str()),
      ],
    ),
    review_receipt,
  )
  .expect("persist generated patch review receipt");
  let store =
    DoghouseStore::open(DoghouseStoreConfig::new(store_path.clone())).expect("open store");
  let by_family = doghouse_core::store::read_coding_memory_artifacts_by_family_at(
    store.path(),
    "coding.generated-patch-review-receipt",
  )
  .expect("query generated patch review family");
  assert_eq!(by_family.len(), 1);
  assert_eq!(by_family[0].id, artifact_id);
  drop(store);

  let feedback_artifact_id = persist_coding_memory_artifact_to_store(
    &store_path,
    feedback_request.artifact_family,
    Some(make_repo_snapshot_ref(&proposal.request.workspace)),
    feedback_request.target_paths.clone(),
    proposal.request.workspace.approved_commands.clone(),
    build_coding_memory_related_refs(
      Option::<&std::path::PathBuf>::None,
      [
        Some(proposal.diff_ref.as_str()),
        Some(feedback_request.source_candidate_ref.as_str()),
        Some(feedback_request.source_review_ref.as_str()),
        Some(feedback_request.request_ref.as_str()),
      ],
    ),
    feedback_request,
  )
  .expect("persist provider feedback request");
  let store = DoghouseStore::open(DoghouseStoreConfig::new(store_path)).expect("open store");
  let by_feedback_family = doghouse_core::store::read_coding_memory_artifacts_by_family_at(
    store.path(),
    "coding.provider-feedback-request",
  )
  .expect("query provider feedback request family");
  assert_eq!(by_feedback_family.len(), 1);
  assert_eq!(by_feedback_family[0].id, feedback_artifact_id);
}

#[test]
fn coding_agent_provider_feedback_response_is_reingested_as_revised_candidate() {
  let patch_dir = temp_dir("coding-agent-provider-feedback-response");
  let bad_patch_path = patch_dir.join("provider-bad.patch");
  fs::write(
    &bad_patch_path,
    "\
--- a/README.md
+++ b/README.md
@@ -1,1 +1,1 @@
-old
+new
",
  )
  .expect("write mismatched provider patch");
  let initial_args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "provider generated patch feedback 요청",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--candidate-patch",
    bad_patch_path.to_str().expect("utf8 bad patch path"),
  ])
  .expect("parse initial args");
  let initial_request =
    build_coding_agent_request(&initial_args, AgentVerb::Patch).expect("build initial request");
  let initial_proposal = build_coding_agent_patch_proposal(
    &initial_args,
    initial_request,
    Some("runtime/request.json".to_string()),
  );
  let feedback_request = initial_proposal
    .provider_feedback_request
    .as_ref()
    .expect("provider feedback request");

  let revised_patch_path = patch_dir.join("provider-response.patch");
  fs::write(
    &revised_patch_path,
    "\
--- a/crates/pnix-executor-graph/src/cli.rs
+++ b/crates/pnix-executor-graph/src/cli.rs
@@ -1,1 +1,1 @@
-//! CLI 실행기: 명령행 인터페이스
+//! CLI 실행기: 명령행 인터페이스
",
  )
  .expect("write revised provider patch");
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "provider feedback response 후보 검토",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--candidate-patch",
    revised_patch_path
      .to_str()
      .expect("utf8 revised patch path"),
    "--provider-feedback-request-ref",
    feedback_request.request_ref.as_str(),
  ])
  .expect("parse revised args");

  let request = build_coding_agent_request(&args, AgentVerb::Patch).expect("build request");
  let proposal =
    build_coding_agent_patch_proposal(&args, request, Some("runtime/request.json".to_string()));
  let candidate = proposal
    .generated_patch_candidate
    .as_ref()
    .expect("generated patch candidate");
  let review_receipt = proposal
    .generated_patch_review_receipt
    .as_ref()
    .expect("generated patch review receipt");

  assert!(proposal.apply_result.is_none());
  assert!(proposal.provider_feedback_request.is_none());
  assert!(proposal.feedback_retry_guard.is_none());
  assert!(proposal
    .effect_classes
    .contains(&"provider-feedback-response:quarantined".to_string()));
  assert_eq!(candidate.phase, "CAX.5l");
  assert_eq!(
    candidate.lineage_status,
    "revised-candidate-from-provider-feedback"
  );
  assert_eq!(
    candidate.source_provider_feedback_request_ref.as_deref(),
    Some(feedback_request.request_ref.as_str())
  );
  assert_eq!(
    candidate.response_boundary,
    "provider-feedback-response-reingested-as-candidate-patch-not-truth"
  );
  assert!(candidate
    .required_next_artifacts
    .contains(&"generated-patch-review-receipt-before-feedback-close".to_string()));
  assert!(candidate.proof_refs.iter().any(|proof| {
    proof
      == &format!(
        "provider-feedback-request-ref:{}",
        feedback_request.request_ref
      )
  }));
  assert_eq!(
    review_receipt.review_status,
    "candidate-reviewed-awaiting-explicit-apply-result"
  );
  assert!(review_receipt.context_demands.is_empty());
  assert!(proposal.semantic_review.proof_refs.iter().any(|proof| {
    proof
      == &format!(
        "provider-feedback-request-ref:{}",
        feedback_request.request_ref
      )
  }));
  assert!(proposal
    .semantic_review
    .patch_decision_link
    .evidence_refs
    .iter()
    .any(|evidence| {
      evidence
        == &format!(
          "provider-feedback-request-ref:{}",
          feedback_request.request_ref
        )
    }));
}

#[test]
fn coding_agent_feedback_retry_guard_blocks_unbounded_provider_loop() {
  let patch_dir = temp_dir("coding-agent-feedback-retry-guard");
  let bad_patch_path = patch_dir.join("provider-bad.patch");
  fs::write(
    &bad_patch_path,
    "\
--- a/README.md
+++ b/README.md
@@ -1,1 +1,1 @@
-old
+new
",
  )
  .expect("write initial mismatched provider patch");
  let initial_args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "provider feedback retry guard 초기 요청",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--candidate-patch",
    bad_patch_path.to_str().expect("utf8 bad patch path"),
  ])
  .expect("parse initial args");
  let initial_request =
    build_coding_agent_request(&initial_args, AgentVerb::Patch).expect("build initial request");
  let initial_proposal = build_coding_agent_patch_proposal(
    &initial_args,
    initial_request,
    Some("runtime/request.json".to_string()),
  );
  let feedback_request = initial_proposal
    .provider_feedback_request
    .as_ref()
    .expect("provider feedback request");

  let retry_patch_path = patch_dir.join("provider-response-still-bad.patch");
  fs::write(
    &retry_patch_path,
    "\
--- a/README.md
+++ b/README.md
@@ -1,1 +1,1 @@
-old
+still-new
",
  )
  .expect("write still mismatched provider response");
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "provider feedback response 재시도 검토",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--candidate-patch",
    retry_patch_path.to_str().expect("utf8 retry patch path"),
    "--provider-feedback-request-ref",
    feedback_request.request_ref.as_str(),
  ])
  .expect("parse retry args");

  let request = build_coding_agent_request(&args, AgentVerb::Patch).expect("build request");
  let proposal =
    build_coding_agent_patch_proposal(&args, request, Some("runtime/request.json".to_string()));
  let candidate = proposal
    .generated_patch_candidate
    .as_ref()
    .expect("generated patch candidate");
  let review_receipt = proposal
    .generated_patch_review_receipt
    .as_ref()
    .expect("generated patch review receipt");
  let retry_guard = proposal
    .feedback_retry_guard
    .as_ref()
    .expect("feedback retry guard");

  assert!(proposal.apply_result.is_none());
  assert!(proposal.provider_feedback_request.is_none());
  assert!(proposal
    .effect_classes
    .contains(&"provider-feedback-retry:guarded".to_string()));
  assert_eq!(candidate.phase, "CAX.5l");
  assert_eq!(
    candidate.lineage_status,
    "revised-candidate-from-provider-feedback"
  );
  assert_eq!(
    review_receipt.review_status,
    "candidate-review-context-required"
  );
  assert!(!review_receipt.context_demands.is_empty());
  assert_eq!(retry_guard.artifact_family, "coding.feedback-retry-guard");
  assert_eq!(retry_guard.phase, "CAX.5m");
  assert_eq!(retry_guard.attempt_index, 1);
  assert_eq!(retry_guard.attempt_limit, 1);
  assert_eq!(retry_guard.retry_decision, "block-provider-auto-retry");
  assert_eq!(
    retry_guard.source_provider_feedback_request_ref,
    feedback_request.request_ref
  );
  assert_eq!(retry_guard.source_candidate_ref, candidate.candidate_ref);
  assert_eq!(retry_guard.source_review_ref, review_receipt.review_ref);
  assert!(retry_guard
    .forbidden_effects
    .contains(&"provider-feedback-auto-retry".to_string()));
  assert!(proposal
    .semantic_review
    .proof_refs
    .iter()
    .any(|proof| { proof == &format!("feedback-retry-guard-ref:{}", retry_guard.guard_ref) }));
  assert!(proposal
    .semantic_review
    .patch_decision_link
    .evidence_refs
    .iter()
    .any(|evidence| {
      evidence == &format!("feedback-retry-guard-ref:{}", retry_guard.guard_ref)
    }));
}

#[test]
fn coding_agent_patch_applies_explicit_unified_diff_and_records_rollback_ref() {
  let repo_root = probe_git_workspace(&std::env::current_dir().unwrap())
    .repo_root
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| std::env::current_dir().unwrap());
  let root = repo_root
    .join("target")
    .join(format!("coding-agent-apply-{}", current_time_ms()));
  let target_path = root.join("sample.txt");
  fs::create_dir_all(&root).unwrap();
  fs::write(&target_path, "alpha\nbeta\n").unwrap();
  let target_arg = path_to_slash(target_path.strip_prefix(&repo_root).unwrap());
  let patch_path = root.join("change.diff");
  fs::write(
    &patch_path,
    format!(
      "--- a/{0}\n+++ b/{0}\n@@ -1,2 +1,2 @@\n alpha\n-beta\n+gamma\n",
      target_arg
    ),
  )
  .unwrap();

  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "명시 patch 적용",
    "--target-path",
    target_arg.as_str(),
    "--patch",
    patch_path.to_str().unwrap(),
    "--current-plan-ref",
    "plan:apply-explicit",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Patch).expect("build request");
  let proposal =
    build_coding_agent_patch_proposal(&args, request, Some("runtime/request.json".to_string()));
  let apply_result = proposal.apply_result.as_ref().expect("apply result");
  let promotion_receipt = proposal
    .promotion_boundary_receipt
    .as_ref()
    .expect("promotion boundary receipt");

  assert_eq!(
    proposal.apply_intent.apply_status, "applied",
    "apply error: {:?}",
    apply_result.error
  );
  assert!(proposal.apply_intent.apply_artifact_ref.is_some());
  assert_eq!(apply_result.artifact_family, "coding.apply-result");
  assert_eq!(apply_result.apply_status, "applied");
  assert_eq!(apply_result.rollback_class, "rollbackable");
  assert!(apply_result.rollback_handle_ref.is_some());
  assert!(apply_result.inverse_plan_ref.is_some());
  assert_eq!(apply_result.applied_paths, vec![target_arg.clone()]);
  assert!(apply_result.rejected_paths.is_empty());
  assert_eq!(apply_result.file_results.len(), 1);
  assert_eq!(apply_result.file_results[0].status, "applied");
  assert!(apply_result.file_results[0]
    .before_snapshot_ref
    .as_deref()
    .unwrap_or_default()
    .starts_with("coding.file-snapshot::before::"));
  assert!(apply_result.file_results[0]
    .after_snapshot_ref
    .as_deref()
    .unwrap_or_default()
    .starts_with("coding.file-snapshot::after::"));
  assert_eq!(fs::read_to_string(&target_path).unwrap(), "alpha\ngamma\n");
  assert!(apply_result
    .proof_refs
    .iter()
    .any(|proof| proof.starts_with("rollback-handle-ref:")));
  assert!(apply_result.proof_refs.iter().any(|proof| {
    proof
      == &format!(
        "semantic-review-ref:{}",
        proposal.semantic_review.review_ref
      )
  }));
  assert_eq!(
    promotion_receipt.artifact_family,
    "coding.promotion-boundary-receipt"
  );
  assert_eq!(promotion_receipt.phase, "CAX.5o");
  assert_eq!(
    promotion_receipt.promotion_status,
    "promotion-held-pending-verify-receipt"
  );
  assert_eq!(
    promotion_receipt.source_apply_artifact_ref,
    apply_result.apply_artifact_ref
  );
  assert!(apply_result.proof_refs.iter().any(|proof| {
    proof
      == &format!(
        "promotion-boundary-receipt-ref:{}",
        promotion_receipt.receipt_ref
      )
  }));
  assert_eq!(
    proposal.semantic_review.patch_decision_link.decision_family,
    "explicit-apply-linked-to-review"
  );
  assert_eq!(
    proposal.semantic_review.apply_artifact_ref.as_deref(),
    Some(apply_result.apply_artifact_ref.as_str())
  );
  assert!(proposal
    .semantic_review
    .patch_decision_link
    .decision_refs
    .iter()
    .any(|decision| decision.starts_with("rollback-handle-ref:")));

  let _ = fs::remove_dir_all(&root);
}

#[test]
fn coding_agent_apply_handoff_accepts_reviewed_candidate_patch_input() {
  let repo_root = probe_git_workspace(&std::env::current_dir().unwrap())
    .repo_root
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| std::env::current_dir().unwrap());
  let root = repo_root
    .join("target")
    .join(format!("coding-agent-apply-handoff-{}", current_time_ms()));
  let target_path = root.join("sample.txt");
  fs::create_dir_all(&root).unwrap();
  fs::write(&target_path, "alpha\nbeta\n").unwrap();
  let target_arg = path_to_slash(target_path.strip_prefix(&repo_root).unwrap());
  let patch_path = root.join("candidate.diff");
  fs::write(
    &patch_path,
    format!(
      "--- a/{0}\n+++ b/{0}\n@@ -1,2 +1,2 @@\n alpha\n-beta\n+gamma\n",
      target_arg
    ),
  )
  .unwrap();

  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "reviewed candidate patch 적용 handoff",
    "--target-path",
    target_arg.as_str(),
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--candidate-patch",
    patch_path.to_str().unwrap(),
    "--patch",
    patch_path.to_str().unwrap(),
    "--dry-run",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Patch).expect("build request");
  let proposal =
    build_coding_agent_patch_proposal(&args, request, Some("runtime/request.json".to_string()));
  let apply_result = proposal.apply_result.as_ref().expect("apply result");
  let handoff_proof = proposal
    .apply_handoff_proof
    .as_ref()
    .expect("apply handoff proof");
  let promotion_receipt = proposal
    .promotion_boundary_receipt
    .as_ref()
    .expect("promotion boundary receipt");

  assert_eq!(apply_result.apply_status, "validated-not-applied");
  assert_eq!(handoff_proof.artifact_family, "coding.apply-handoff-proof");
  assert_eq!(handoff_proof.phase, "CAX.5n");
  assert_eq!(handoff_proof.handoff_status, "handoff-accepted");
  assert!(handoff_proof.failure_reason.is_none());
  assert_eq!(
    handoff_proof.candidate_patch_input_ref,
    handoff_proof.apply_patch_input_ref
  );
  assert_eq!(
    promotion_receipt.source_handoff_ref.as_deref(),
    Some(handoff_proof.handoff_ref.as_str())
  );
  assert_eq!(
    promotion_receipt.promotion_status,
    "promotion-held-dry-run-not-mutation"
  );
  assert!(proposal
    .effect_classes
    .contains(&"generated-patch-apply-handoff:checked".to_string()));
  assert!(apply_result
    .proof_refs
    .iter()
    .any(|proof| { proof == &format!("apply-handoff-proof-ref:{}", handoff_proof.handoff_ref) }));
  assert!(apply_result.proof_refs.iter().any(|proof| {
    proof
      == &format!(
        "promotion-boundary-receipt-ref:{}",
        promotion_receipt.receipt_ref
      )
  }));
  assert!(proposal
    .semantic_review
    .proof_refs
    .iter()
    .any(|proof| { proof == &format!("apply-handoff-status:{}", handoff_proof.handoff_status) }));
  assert!(proposal
    .semantic_review
    .patch_decision_link
    .evidence_refs
    .iter()
    .any(|evidence| {
      evidence == &format!("apply-handoff-proof-ref:{}", handoff_proof.handoff_ref)
    }));
  assert_eq!(fs::read_to_string(&target_path).unwrap(), "alpha\nbeta\n");

  let _ = fs::remove_dir_all(&root);
}

#[test]
fn coding_agent_apply_handoff_blocks_mismatched_candidate_and_apply_patch() {
  let repo_root = probe_git_workspace(&std::env::current_dir().unwrap())
    .repo_root
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| std::env::current_dir().unwrap());
  let root = repo_root.join("target").join(format!(
    "coding-agent-apply-handoff-block-{}",
    current_time_ms()
  ));
  let target_path = root.join("sample.txt");
  fs::create_dir_all(&root).unwrap();
  fs::write(&target_path, "alpha\nbeta\n").unwrap();
  let target_arg = path_to_slash(target_path.strip_prefix(&repo_root).unwrap());
  let candidate_patch_path = root.join("candidate.diff");
  fs::write(
    &candidate_patch_path,
    format!(
      "--- a/{0}\n+++ b/{0}\n@@ -1,2 +1,2 @@\n alpha\n-beta\n+gamma\n",
      target_arg
    ),
  )
  .unwrap();
  let apply_patch_path = root.join("apply.diff");
  fs::write(
    &apply_patch_path,
    format!(
      "--- a/{0}\n+++ b/{0}\n@@ -1,2 +1,2 @@\n alpha\n-beta\n+delta\n",
      target_arg
    ),
  )
  .unwrap();

  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "mismatched candidate patch handoff 차단",
    "--target-path",
    target_arg.as_str(),
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--candidate-patch",
    candidate_patch_path.to_str().unwrap(),
    "--patch",
    apply_patch_path.to_str().unwrap(),
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Patch).expect("build request");
  let proposal =
    build_coding_agent_patch_proposal(&args, request, Some("runtime/request.json".to_string()));
  let apply_result = proposal.apply_result.as_ref().expect("apply result");
  let handoff_proof = proposal
    .apply_handoff_proof
    .as_ref()
    .expect("apply handoff proof");

  assert_eq!(proposal.apply_intent.apply_status, "blocked");
  assert!(proposal.promotion_boundary_receipt.is_none());
  assert_eq!(apply_result.apply_status, "blocked");
  assert_eq!(handoff_proof.handoff_status, "handoff-blocked");
  assert_ne!(
    handoff_proof.candidate_patch_input_ref,
    handoff_proof.apply_patch_input_ref
  );
  assert!(handoff_proof
    .failure_reason
    .as_deref()
    .unwrap_or_default()
    .contains("apply patch input differs"));
  assert!(apply_result
    .error
    .as_deref()
    .unwrap_or_default()
    .contains("apply patch input differs"));
  assert!(apply_result.rejected_paths.contains(&target_arg));
  assert!(apply_result.rollback_handle_ref.is_none());
  assert_eq!(fs::read_to_string(&target_path).unwrap(), "alpha\nbeta\n");

  let _ = fs::remove_dir_all(&root);
}

#[test]
fn coding_agent_patch_blocks_unregistered_patch_target() {
  let repo_root = probe_git_workspace(&std::env::current_dir().unwrap())
    .repo_root
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| std::env::current_dir().unwrap());
  let root = repo_root
    .join("target")
    .join(format!("coding-agent-apply-block-{}", current_time_ms()));
  fs::create_dir_all(&root).unwrap();
  let declared_path = root.join("declared.txt");
  let undeclared_path = root.join("other.txt");
  fs::write(&declared_path, "alpha\nbeta\n").unwrap();
  fs::write(&undeclared_path, "alpha\nbeta\n").unwrap();
  let declared_arg = path_to_slash(declared_path.strip_prefix(&repo_root).unwrap());
  let undeclared_arg = path_to_slash(undeclared_path.strip_prefix(&repo_root).unwrap());
  let patch_path = root.join("change.diff");
  fs::write(
    &patch_path,
    format!(
      "--- a/{0}\n+++ b/{0}\n@@ -1,2 +1,2 @@\n alpha\n-beta\n+gamma\n",
      undeclared_arg
    ),
  )
  .unwrap();

  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "undeclared target 차단",
    "--target-path",
    declared_arg.as_str(),
    "--patch",
    patch_path.to_str().unwrap(),
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Patch).expect("build request");
  let proposal =
    build_coding_agent_patch_proposal(&args, request, Some("runtime/request.json".to_string()));
  let apply_result = proposal.apply_result.as_ref().expect("apply result");

  assert_eq!(proposal.apply_intent.apply_status, "blocked");
  assert!(proposal.apply_intent.apply_artifact_ref.is_some());
  assert_eq!(proposal.status.result_status, "차단");
  assert_eq!(apply_result.apply_status, "blocked");
  assert!(apply_result.rollback_handle_ref.is_none());
  assert!(apply_result
    .error
    .as_deref()
    .unwrap_or_default()
    .contains("outside declared --target-path"));
  assert_eq!(
    fs::read_to_string(&undeclared_path).unwrap(),
    "alpha\nbeta\n"
  );
  assert_eq!(
    proposal.semantic_review.review_status,
    "held-semantic-review-required"
  );
  assert_eq!(
    proposal.semantic_review.meaning_impact_diff.risk_signal,
    "mutation-blocked"
  );
  assert!(proposal
    .semantic_review
    .narrative_regression
    .risk_notes
    .iter()
    .any(|note| note.starts_with("apply-error-summary:")));

  let _ = fs::remove_dir_all(&root);
}

#[test]
fn coding_agent_verify_receipt_is_bounded_and_snapshot_tied() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "verify",
    "--request",
    "crates/pnix-executor-graph/src/cli.rs 수정 검증",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--current-plan-ref",
    "plan:cli-verify",
    "--last-verification-ref",
    "verify:previous",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Verify).expect("build request");
  let receipt =
    build_coding_agent_verify_receipt(&args, request, Some("runtime/request.json".to_string()));

  assert_eq!(receipt.artifact_family, "coding.verify-receipt");
  assert_eq!(receipt.phase, "CAX.3b-partial");
  assert_eq!(receipt.verb, "verify");
  assert_eq!(
    receipt.request_artifact_ref.as_deref(),
    Some("runtime/request.json")
  );
  assert_eq!(receipt.status.progress_status, "검증영수증준비완료");
  assert_eq!(receipt.status.result_status, "부분완료");
  assert_eq!(
    receipt.target_paths,
    vec!["crates/pnix-executor-graph/src/cli.rs".to_string()]
  );
  assert_eq!(
    receipt.target_commands,
    vec!["cargo check -p pnix-executor-graph".to_string()]
  );
  assert!(receipt
    .repo_snapshot_ref
    .starts_with("coding.repo-snapshot::"));
  assert!(receipt
    .before_artifact_ref
    .starts_with("coding.verify-snapshot::before::"));
  assert!(receipt
    .after_artifact_ref
    .starts_with("coding.verify-snapshot::after::"));
  assert!(receipt.diff_ref.starts_with("coding.diff::verify::"));
  assert_eq!(
    receipt.execution_result.artifact_family,
    "coding.execution-result"
  );
  assert!(receipt
    .execution_result
    .execution_result_ref
    .starts_with("coding.execution-result::"));
  assert_eq!(
    receipt.execution_result.execution_status,
    "not-run-command-execution-closed"
  );
  assert!(receipt.execution_result.exit_code.is_none());
  assert_eq!(
    receipt.learning_card.artifact_family,
    "coding.learning-card"
  );
  assert!(receipt
    .learning_card
    .learning_card_ref
    .starts_with("coding.learning-card::"));
  assert_eq!(
    receipt.learning_card.promotion_status,
    "candidate-only-not-promoted"
  );
  assert_eq!(receipt.learning_card.reuse_score, 0.0);
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| proof == "target-command:cargo check -p pnix-executor-graph"));
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| proof == "current-plan-ref:plan:cli-verify"));
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| proof == "last-verification-ref:verify:previous"));
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| proof.starts_with("repo-graph-status:")));
  assert_eq!(
    receipt.request.manual_evidence_seed.join_policy,
    "manual-hit-never-justifies-patch-without-file-symbol-project-join"
  );
}

#[test]
fn coding_agent_verify_promotion_boundary_join_receipt_links_apply_lineage() {
  let current_exe = std::env::current_exe().expect("current test binary");
  let command = format!("{} --help", current_exe.display());
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "verify",
    "--request",
    "promotion boundary 이후 검증 join",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    command.as_str(),
    "--promotion-boundary-ref",
    "coding.promotion-boundary-receipt::abc",
    "--source-apply-artifact-ref",
    "coding.apply-result::def",
    "--source-handoff-ref",
    "coding.apply-handoff-proof::ghi",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Verify).expect("build request");
  let mut receipt =
    build_coding_agent_verify_receipt(&args, request, Some("runtime/request.json".to_string()));
  let execution_result = run_coding_agent_verify_commands(
    &receipt.request,
    &receipt.repo_snapshot_ref,
    &receipt.diff_ref,
    &receipt.target_commands,
  );
  attach_coding_agent_verify_execution_result(&mut receipt, execution_result);
  attach_coding_agent_promotion_boundary_join_receipt(&mut receipt);

  let join = receipt
    .promotion_boundary_join_receipt
    .as_ref()
    .expect("promotion boundary join receipt");
  assert_eq!(
    join.artifact_family,
    "coding.promotion-boundary-join-receipt"
  );
  assert_eq!(join.phase, "CAX.5p");
  assert_eq!(
    join.source_promotion_boundary_receipt_ref,
    "coding.promotion-boundary-receipt::abc"
  );
  assert_eq!(join.source_apply_artifact_ref, "coding.apply-result::def");
  assert_eq!(
    join.source_handoff_ref.as_deref(),
    Some("coding.apply-handoff-proof::ghi")
  );
  assert_eq!(
    join.join_status,
    "joined-verify-passed-awaiting-human-judgement"
  );
  assert_eq!(
    join.promotion_boundary,
    "join-receipt-only-not-judgement-owner"
  );
  assert!(join
    .required_next_artifacts
    .iter()
    .any(|artifact| { artifact == "human-judgement-boundary-before-promotion" }));
  assert!(join
    .forbidden_effects
    .iter()
    .any(|effect| effect == "verify-receipt-auto-promotion"));
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| { proof == &format!("promotion-boundary-join-receipt-ref:{}", join.join_ref) }));
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| proof == "verify-receipt-is-not-promotion-owner"));
  assert!(receipt
    .learning_card
    .proof_refs
    .iter()
    .any(|proof| { proof == &format!("promotion-boundary-join-receipt-ref:{}", join.join_ref) }));
}

#[test]
fn coding_agent_verify_promotion_boundary_ref_requires_source_apply_ref() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "verify",
    "--request",
    "promotion boundary ref only",
    "--promotion-boundary-ref",
    "coding.promotion-boundary-receipt::abc",
  ])
  .expect_err("source apply artifact ref should be required");

  assert!(err
    .to_string()
    .contains("--promotion-boundary-ref requires --source-apply-artifact-ref"));
}

#[test]
fn coding_agent_decide_emits_human_promotion_decision_packet() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "decide",
    "--request",
    "human reviewed verify and accepts promotion boundary",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--promotion-boundary-join-ref",
    "coding.promotion-boundary-join-receipt::join",
    "--promotion-decision",
    "accepted",
    "--agent-decision-out",
    "runtime/decision.json",
  ])
  .expect("parse args");

  assert_eq!(args.agent, Some(AgentVerb::Decide));
  let request = build_coding_agent_request(&args, AgentVerb::Decide).expect("build request");
  let decision = build_coding_agent_human_promotion_decision(
    &args,
    request,
    Some("runtime/request.json".to_string()),
  )
  .expect("build decision");

  assert_eq!(decision.artifact_family, "coding.human-promotion-decision");
  assert_eq!(decision.phase, "CAX.5q");
  assert_eq!(decision.verb, "decide");
  assert!(decision
    .decision_ref
    .starts_with("coding.human-promotion-decision::"));
  assert_eq!(
    decision.source_promotion_boundary_join_ref,
    "coding.promotion-boundary-join-receipt::join"
  );
  assert_eq!(decision.human_decision, "accepted");
  assert_eq!(decision.decision_status, "accepted-by-human-judgement");
  assert_eq!(
    decision.promotion_status,
    "human-accepted-awaiting-release-owner"
  );
  assert_eq!(
    decision.promotion_boundary,
    "human-decision-packet-not-mutation-owner"
  );
  assert!(decision
    .required_next_artifacts
    .iter()
    .any(|artifact| { artifact == "release-or-merge-owner-before-production-promotion" }));
  assert!(decision
    .forbidden_effects
    .contains(&"human-decision-auto-merge".to_string()));
  assert!(decision
    .proof_refs
    .iter()
    .any(|proof| proof == "human-decision:accepted"));
  assert!(decision.proof_refs.iter().any(|proof| {
    proof == "promotion-boundary-join-receipt-ref:coding.promotion-boundary-join-receipt::join"
  }));
}

#[test]
fn coding_agent_decide_requires_join_ref_and_decision() {
  let missing_join = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "decide",
    "--promotion-decision",
    "accepted",
  ])
  .expect_err("join ref should be required");
  assert!(missing_join
    .to_string()
    .contains("pnix coding-agent decide requires --promotion-boundary-join-ref"));

  let invalid_decision = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "decide",
    "--promotion-boundary-join-ref",
    "coding.promotion-boundary-join-receipt::join",
    "--promotion-decision",
    "auto",
  ])
  .expect_err("decision enum should be bounded");
  assert!(invalid_decision
    .to_string()
    .contains("--promotion-decision must be one of accepted|rejected|held"));
}

#[cfg(feature = "doghouse")]
#[test]
fn coding_agent_decision_persists_to_doghouse_memory_and_rejection_stays_repair_bound() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "decide",
    "--request",
    "human rejected after review",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--promotion-boundary-join-ref",
    "coding.promotion-boundary-join-receipt::reject",
    "--promotion-decision",
    "rejected",
    "--agent-decision-out",
    "runtime/decision-rejected.json",
  ])
  .expect("parse args");
  let request = build_coding_agent_request(&args, AgentVerb::Decide).expect("build request");
  let decision = build_coding_agent_human_promotion_decision(
    &args,
    request,
    Some("runtime/request.json".to_string()),
  )
  .expect("build decision");

  assert_eq!(decision.human_decision, "rejected");
  assert_eq!(decision.decision_status, "rejected-by-human-judgement");
  assert_eq!(decision.promotion_status, "human-rejected-repair-required");
  assert!(decision
    .required_next_artifacts
    .iter()
    .any(|artifact| { artifact == "repair-patch-proposal-before-promotion" }));

  let store_dir = temp_dir("coding-memory-human-decision");
  let store_path = store_dir.join("doghouse.redb");
  let artifact_id = persist_coding_memory_artifact_to_store(
    &store_path,
    decision.artifact_family,
    Some(make_repo_snapshot_ref(&decision.request.workspace)),
    decision.target_paths.clone(),
    decision.target_commands.clone(),
    build_coding_memory_related_refs(
      args.agent_decision_out.as_ref(),
      [
        decision.request_artifact_ref.as_deref(),
        Some(decision.source_promotion_boundary_join_ref.as_str()),
        Some(decision.decision_ref.as_str()),
      ],
    ),
    &decision,
  )
  .expect("persist human promotion decision");

  let store = DoghouseStore::open(DoghouseStoreConfig::new(store_path)).expect("open store");
  let by_family = doghouse_core::store::read_coding_memory_artifacts_by_family_at(
    store.path(),
    "coding.human-promotion-decision",
  )
  .expect("query human promotion decisions");
  assert_eq!(by_family.len(), 1);
  assert_eq!(by_family[0].id, artifact_id);
}

#[test]
fn coding_agent_e2e_apply_verify_join_then_human_decision_chain() {
  let repo_root = probe_git_workspace(&std::env::current_dir().unwrap())
    .repo_root
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| std::env::current_dir().unwrap());
  let root = repo_root
    .join("target")
    .join(format!("coding-agent-e2e-{}", current_time_ms()));
  let target_path = root.join("sample.txt");
  fs::create_dir_all(&root).unwrap();
  fs::write(&target_path, "alpha\nbeta\n").unwrap();
  let target_arg = path_to_slash(target_path.strip_prefix(&repo_root).unwrap());
  let patch_path = root.join("change.diff");
  fs::write(
    &patch_path,
    format!(
      "--- a/{0}\n+++ b/{0}\n@@ -1,2 +1,2 @@\n alpha\n-beta\n+gamma\n",
      target_arg
    ),
  )
  .unwrap();

  let patch_args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "e2e explicit patch",
    "--target-path",
    target_arg.as_str(),
    "--patch",
    patch_path.to_str().unwrap(),
    "--current-plan-ref",
    "plan:e2e",
  ])
  .expect("parse patch args");
  let patch_request =
    build_coding_agent_request(&patch_args, AgentVerb::Patch).expect("build patch request");
  let patch_proposal = build_coding_agent_patch_proposal(
    &patch_args,
    patch_request,
    Some("runtime/request.json".to_string()),
  );
  let apply_result = patch_proposal.apply_result.as_ref().expect("apply result");
  let promotion_receipt = patch_proposal
    .promotion_boundary_receipt
    .as_ref()
    .expect("promotion boundary receipt");
  assert_eq!(apply_result.apply_status, "applied");

  let current_exe = std::env::current_exe().expect("current test binary");
  let command = format!("{} --help", current_exe.display());
  let verify_args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "verify",
    "--request",
    "e2e verify after apply",
    "--target-path",
    target_arg.as_str(),
    "--approved-command",
    command.as_str(),
    "--promotion-boundary-ref",
    promotion_receipt.receipt_ref.as_str(),
    "--source-apply-artifact-ref",
    apply_result.apply_artifact_ref.as_str(),
  ])
  .expect("parse verify args");
  let verify_request =
    build_coding_agent_request(&verify_args, AgentVerb::Verify).expect("build verify request");
  let mut verify_receipt = build_coding_agent_verify_receipt(
    &verify_args,
    verify_request,
    Some("runtime/verify-request.json".to_string()),
  );
  let execution_result = run_coding_agent_verify_commands(
    &verify_receipt.request,
    &verify_receipt.repo_snapshot_ref,
    &verify_receipt.diff_ref,
    &verify_receipt.target_commands,
  );
  attach_coding_agent_verify_execution_result(&mut verify_receipt, execution_result);
  attach_coding_agent_promotion_boundary_join_receipt(&mut verify_receipt);
  let join = verify_receipt
    .promotion_boundary_join_receipt
    .as_ref()
    .expect("promotion boundary join receipt");
  assert_eq!(
    join.join_status,
    "joined-verify-passed-awaiting-human-judgement"
  );

  let decide_args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "decide",
    "--request",
    "human accepts bounded e2e chain",
    "--target-path",
    target_arg.as_str(),
    "--promotion-boundary-join-ref",
    join.join_ref.as_str(),
    "--promotion-decision",
    "accepted",
  ])
  .expect("parse decide args");
  let decide_request =
    build_coding_agent_request(&decide_args, AgentVerb::Decide).expect("build decide request");
  let decision = build_coding_agent_human_promotion_decision(&decide_args, decide_request, None)
    .expect("build decision");

  assert_eq!(decision.human_decision, "accepted");
  assert_eq!(decision.source_promotion_boundary_join_ref, join.join_ref);
  assert!(decision
    .proof_refs
    .iter()
    .any(|proof| { proof == &format!("promotion-boundary-join-receipt-ref:{}", join.join_ref) }));
  assert_eq!(
    decision.promotion_boundary,
    "human-decision-packet-not-mutation-owner"
  );

  let _ = fs::remove_dir_all(&root);
}

#[test]
fn coding_agent_verify_execution_result_records_successful_approved_command() {
  let current_exe = std::env::current_exe().expect("current test binary");
  let command = format!("{} --help", current_exe.display());
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "verify",
    "--request",
    "approved command 실행 검증",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    command.as_str(),
    "--current-plan-ref",
    "plan:cli-verify-exec",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Verify).expect("build request");
  let mut receipt =
    build_coding_agent_verify_receipt(&args, request, Some("runtime/request.json".to_string()));
  let execution_result = run_coding_agent_verify_commands(
    &receipt.request,
    &receipt.repo_snapshot_ref,
    &receipt.diff_ref,
    &receipt.target_commands,
  );
  attach_coding_agent_verify_execution_result(&mut receipt, execution_result);

  assert_eq!(receipt.execution_result.execution_status, "passed");
  assert_eq!(receipt.status.progress_status, "검증실행완료");
  assert_eq!(receipt.status.result_status, "통과");
  assert_eq!(receipt.execution_result.command_results.len(), 1);
  assert_eq!(receipt.execution_result.command_results[0].status, "passed");
  assert_eq!(
    receipt.execution_result.command_results[0].exit_code,
    Some(0)
  );
  assert!(receipt.execution_result.command_results[0]
    .stdout_preview
    .contains("Usage"));
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| proof == "execution-status:passed"));
  assert!(receipt.diagnostic_records.is_empty());
  assert!(receipt.failure_pattern_matches.is_empty());
  assert!(receipt.context_demands.is_empty());
  assert!(receipt
    .learning_card
    .proof_refs
    .iter()
    .any(|proof| proof.starts_with("execution-result-ref:")));
}

#[test]
fn coding_agent_verify_execution_result_blocks_shell_control_syntax() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "verify",
    "--request",
    "shell passthrough 차단 검증",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "printf ok | cat",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Verify).expect("build request");
  let mut receipt =
    build_coding_agent_verify_receipt(&args, request, Some("runtime/request.json".to_string()));
  let execution_result = run_coding_agent_verify_commands(
    &receipt.request,
    &receipt.repo_snapshot_ref,
    &receipt.diff_ref,
    &receipt.target_commands,
  );
  attach_coding_agent_verify_execution_result(&mut receipt, execution_result);

  assert_eq!(receipt.execution_result.execution_status, "blocked");
  assert_eq!(receipt.status.progress_status, "검증실행차단");
  assert_eq!(receipt.status.result_status, "차단");
  assert_eq!(receipt.execution_result.command_results.len(), 1);
  assert_eq!(
    receipt.execution_result.command_results[0].status,
    "blocked"
  );
  assert!(receipt.execution_result.command_results[0]
    .error
    .as_deref()
    .unwrap_or_default()
    .contains("shell control syntax"));
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| proof == "execution-status:blocked"));
  assert_eq!(receipt.diagnostic_records.len(), 1);
  assert_eq!(
    receipt.diagnostic_records[0].artifact_family,
    "pnix.diagnostic-record"
  );
  assert_eq!(
    receipt.diagnostic_records[0].diagnostic_family,
    "verify-command-policy-blocked"
  );
  assert_eq!(receipt.diagnostic_records[0].severity, "hold");
  assert!(receipt.diagnostic_records[0]
    .message
    .contains("shell control syntax"));
  assert_eq!(receipt.failure_pattern_matches.len(), 1);
  assert_eq!(
    receipt.failure_pattern_matches[0].pattern_key,
    "verify-command-policy-blocked"
  );
  assert_eq!(receipt.context_demands.len(), 1);
  assert_eq!(
    receipt.context_demands[0].demand_family,
    "verify-failure-context-required"
  );
  assert_eq!(
    receipt.context_demands[0].request_boundary,
    "request-more-context-before-next-patch-proposal"
  );
  assert!(receipt.proof_refs.iter().any(|proof| {
    proof
      == &format!(
        "diagnostic-ref:{}",
        receipt.diagnostic_records[0].diagnostic_ref
      )
  }));
  assert!(receipt.proof_refs.iter().any(|proof| {
    proof
      == &format!(
        "context-demand-ref:{}",
        receipt.context_demands[0].context_demand_ref
      )
  }));
  let stdout_ref = receipt.execution_result.command_results[0]
    .stdout_ref
    .clone();
  let stderr_ref = receipt.execution_result.command_results[0]
    .stderr_ref
    .clone();
  assert!(!receipt
    .proof_refs
    .iter()
    .any(|proof| proof == &stdout_ref || proof == &format!("stdout-ref:{stdout_ref}")));
  assert!(!receipt
    .proof_refs
    .iter()
    .any(|proof| proof == &stderr_ref || proof == &format!("stderr-ref:{stderr_ref}")));
}

#[test]
fn coding_agent_rollback_handle_is_typed_and_classified() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "rollback",
    "--request",
    "crates/pnix-executor-graph/src/cli.rs 수정 롤백 준비",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--current-plan-ref",
    "plan:cli-rollback",
    "--last-verification-ref",
    "verify:previous",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Rollback).expect("build request");
  let handle =
    build_coding_agent_rollback_handle(&args, request, Some("runtime/request.json".to_string()));

  assert_eq!(handle.artifact_family, "coding.rollback-handle");
  assert_eq!(handle.phase, "CAX.3c-partial");
  assert_eq!(handle.verb, "rollback");
  assert_eq!(
    handle.request_artifact_ref.as_deref(),
    Some("runtime/request.json")
  );
  assert!(handle.handle_id.starts_with("coding.rollback-handle::"));
  assert!(handle
    .repo_snapshot_ref
    .starts_with("coding.repo-snapshot::"));
  assert!(handle
    .apply_artifact_ref
    .starts_with("coding.apply-intent::"));
  assert_eq!(handle.rollback_class, "acknowledge-risk");
  assert!(handle.inverse_plan_ref.is_none());
  assert!(handle.expires_at_ms.is_none());
  assert_eq!(handle.status.progress_status, "롤백핸들준비완료");
  assert_eq!(handle.status.result_status, "부분완료");
  assert!(handle
    .non_rollbackable_effects
    .contains(&"verification-command:intent-only".to_string()));
  assert!(handle.effect_contracts.iter().any(|contract| {
    contract.effect_class == "workspace-file-write:intent-only"
      && contract.rollback_contract == "rollbackable"
  }));
  assert!(handle.effect_contracts.iter().any(|contract| {
    contract.effect_class == "verification-command:intent-only"
      && contract.rollback_contract == "acknowledge-risk"
  }));
  assert!(handle
    .proof_refs
    .iter()
    .any(|proof| proof == "rollback-class:acknowledge-risk"));
  assert!(handle
    .proof_refs
    .iter()
    .any(|proof| proof == "current-plan-ref:plan:cli-rollback"));
  assert!(handle
    .proof_refs
    .iter()
    .any(|proof| proof == "last-verification-ref:verify:previous"));
  assert_eq!(
    handle.request.manual_evidence_seed.join_policy,
    "manual-hit-never-justifies-patch-without-file-symbol-project-join"
  );
}

#[test]
fn coding_agent_rollback_receipt_is_typed_and_relinked_to_verify() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "rollback",
    "--request",
    "crates/pnix-executor-graph/src/cli.rs 수정 롤백 실행 기록",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--current-plan-ref",
    "plan:cli-rollback",
    "--rollback-handle-ref",
    "coding.rollback-handle::existing",
    "--last-verification-ref",
    "verify:previous",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Rollback).expect("build request");
  let receipt =
    build_coding_agent_rollback_receipt(&args, request, Some("runtime/request.json".to_string()));

  assert_eq!(receipt.artifact_family, "coding.rollback-receipt");
  assert_eq!(receipt.phase, "CAX.3c-partial");
  assert_eq!(receipt.verb, "rollback");
  assert_eq!(
    receipt.request_artifact_ref.as_deref(),
    Some("runtime/request.json")
  );
  assert_eq!(receipt.handle_ref, "coding.rollback-handle::existing");
  assert_eq!(receipt.rollback_class, "acknowledge-risk");
  assert!(receipt
    .repo_snapshot_ref
    .starts_with("coding.repo-snapshot::"));
  assert!(receipt
    .apply_artifact_ref
    .starts_with("coding.apply-intent::"));
  assert!(receipt.inverse_plan_ref.is_none());
  assert!(receipt.restored_snapshot_ref.is_none());
  assert!(receipt.followup_verify_ref.is_some());
  assert_eq!(receipt.status.progress_status, "롤백영수증준비완료");
  assert_eq!(receipt.status.result_status, "부분완료");
  assert!(receipt
    .non_rollbackable_effects
    .contains(&"verification-command:intent-only".to_string()));
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| { proof == "handle-ref:coding.rollback-handle::existing" }));
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| proof.starts_with("followup-verify-ref:coding.verify-receipt::post-rollback::")));
  assert_eq!(
    receipt.request.manual_evidence_seed.join_policy,
    "manual-hit-never-justifies-patch-without-file-symbol-project-join"
  );
}

#[test]
fn coding_agent_rollback_receipt_applies_explicit_inverse_diff() {
  let repo_root = probe_git_workspace(&std::env::current_dir().unwrap())
    .repo_root
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| std::env::current_dir().unwrap());
  let root = repo_root
    .join("target")
    .join(format!("coding-agent-rollback-{}", current_time_ms()));
  let target_path = root.join("sample.txt");
  fs::create_dir_all(&root).unwrap();
  fs::write(&target_path, "alpha\ngamma\n").unwrap();
  let target_arg = path_to_slash(target_path.strip_prefix(&repo_root).unwrap());
  let patch_path = root.join("inverse.diff");
  fs::write(
    &patch_path,
    format!(
      "--- a/{0}\n+++ b/{0}\n@@ -1,2 +1,2 @@\n alpha\n-gamma\n+beta\n",
      target_arg
    ),
  )
  .unwrap();

  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "rollback",
    "--request",
    "명시 inverse diff 롤백",
    "--target-path",
    target_arg.as_str(),
    "--rollback-handle-ref",
    "coding.rollback-handle::existing",
    "--patch",
    patch_path.to_str().unwrap(),
    "--current-plan-ref",
    "plan:rollback-explicit",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Rollback).expect("build request");
  let receipt =
    build_coding_agent_rollback_receipt(&args, request, Some("runtime/request.json".to_string()));

  assert_eq!(receipt.rollback_class, "rollbackable");
  assert_eq!(receipt.rollback_status, "rolled-back");
  assert_eq!(receipt.status.progress_status, "롤백실행완료");
  assert_eq!(receipt.status.result_status, "부분완료");
  assert!(receipt.inverse_plan_ref.is_some());
  assert!(receipt
    .restored_snapshot_ref
    .as_deref()
    .unwrap_or_default()
    .starts_with("coding.repo-snapshot::restored::"));
  assert_eq!(receipt.restored_paths, vec![target_arg.clone()]);
  assert!(receipt.rejected_paths.is_empty());
  assert_eq!(receipt.file_results.len(), 1);
  assert_eq!(receipt.file_results[0].status, "restored");
  assert_eq!(fs::read_to_string(&target_path).unwrap(), "alpha\nbeta\n");
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| proof == "rollback-status:rolled-back"));
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| proof == &format!("rollback-file:{}:restored", target_arg)));

  let _ = fs::remove_dir_all(&root);
}

#[test]
fn coding_agent_rollback_receipt_blocks_unregistered_inverse_diff_target() {
  let repo_root = probe_git_workspace(&std::env::current_dir().unwrap())
    .repo_root
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| std::env::current_dir().unwrap());
  let root = repo_root
    .join("target")
    .join(format!("coding-agent-rollback-block-{}", current_time_ms()));
  fs::create_dir_all(&root).unwrap();
  let declared_path = root.join("declared.txt");
  let undeclared_path = root.join("other.txt");
  fs::write(&declared_path, "alpha\ngamma\n").unwrap();
  fs::write(&undeclared_path, "alpha\ngamma\n").unwrap();
  let declared_arg = path_to_slash(declared_path.strip_prefix(&repo_root).unwrap());
  let undeclared_arg = path_to_slash(undeclared_path.strip_prefix(&repo_root).unwrap());
  let patch_path = root.join("inverse.diff");
  fs::write(
    &patch_path,
    format!(
      "--- a/{0}\n+++ b/{0}\n@@ -1,2 +1,2 @@\n alpha\n-gamma\n+beta\n",
      undeclared_arg
    ),
  )
  .unwrap();

  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "rollback",
    "--request",
    "undeclared inverse diff 롤백 차단",
    "--target-path",
    declared_arg.as_str(),
    "--rollback-handle-ref",
    "coding.rollback-handle::existing",
    "--patch",
    patch_path.to_str().unwrap(),
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Rollback).expect("build request");
  let receipt =
    build_coding_agent_rollback_receipt(&args, request, Some("runtime/request.json".to_string()));

  assert_eq!(receipt.rollback_class, "rollbackable");
  assert_eq!(receipt.rollback_status, "blocked");
  assert_eq!(receipt.status.result_status, "차단");
  assert!(receipt.restored_snapshot_ref.is_none());
  assert!(receipt.restored_paths.is_empty());
  assert!(receipt
    .error
    .as_deref()
    .unwrap_or_default()
    .contains("outside declared --target-path"));
  assert_eq!(
    fs::read_to_string(&undeclared_path).unwrap(),
    "alpha\ngamma\n"
  );
  assert!(receipt
    .proof_refs
    .iter()
    .any(|proof| proof == "rollback-status:blocked"));

  let _ = fs::remove_dir_all(&root);
}

#[test]
fn coding_agent_plan_out_is_rejected_for_non_plan_verbs() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "ask",
    "--agent-plan-out",
    "runtime/plan.json",
  ])
  .expect_err("plan output path must be plan-only");

  assert!(err
    .to_string()
    .contains("--agent-plan-out is only supported for `pnix coding-agent plan`"));
}

#[test]
fn coding_agent_patch_out_is_rejected_for_non_patch_verbs() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "ask",
    "--agent-patch-out",
    "runtime/patch.json",
  ])
  .expect_err("patch output path must be patch-only");

  assert!(err
    .to_string()
    .contains("--agent-patch-out is only supported for `pnix coding-agent patch`"));
}

#[test]
fn coding_agent_verify_out_is_rejected_for_non_verify_verbs() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "ask",
    "--agent-verify-out",
    "runtime/verify.json",
  ])
  .expect_err("verify output path must be verify-only");

  assert!(err
    .to_string()
    .contains("--agent-verify-out is only supported for `pnix coding-agent verify`"));
}

#[test]
fn coding_agent_rollback_out_is_rejected_for_non_rollback_verbs() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "ask",
    "--agent-rollback-out",
    "runtime/rollback.json",
  ])
  .expect_err("rollback output path must be rollback-only");

  assert!(err
    .to_string()
    .contains("--agent-rollback-out is only supported for `pnix coding-agent rollback`"));
}

#[test]
fn coding_agent_plan_links_language_verify_targets_without_approval() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "plan",
    "--request",
    "python target 검증 계획",
    "--target-path",
    "tool/python/puckPy/puck_py.py",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Plan).expect("build request");
  let plan = build_coding_agent_plan(&args, request, Some("runtime/request.json".to_string()));
  assert_eq!(
    plan.expected_verification,
    vec!["manual-verification-contract-required".to_string()]
  );
  assert_eq!(plan.execution_plan.language_verify_targets.len(), 1);
  assert_eq!(
    plan.execution_plan.language_verify_targets[0].language,
    "python"
  );
  assert_eq!(plan.execution_plan.execution_requests.len(), 1);
  let request = &plan.execution_plan.execution_requests[0];
  assert_eq!(
    request.permission_status,
    "candidate-only-requires-approved-command"
  );
  assert_eq!(
    request.command_refs,
    vec!["manual-verification-contract-required".to_string()]
  );
  assert_eq!(request.candidate_verify_target_refs.len(), 1);
  assert!(request
    .candidate_command_refs
    .iter()
    .any(|command_ref| { command_ref == "candidate:python:python -m pytest" }));
  assert!(request
    .effect_classes
    .contains(&"verification-command:candidate-only".to_string()));
}

#[test]
fn coding_agent_request_emits_repo_root_grounding_seed_without_targets() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "ask",
    "--request",
    "현재 workspace 상태 요약해줘",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Ask).expect("build request");

  assert_eq!(request.grounding_seed.scan_mode, "repo-root-bounded-scan");
  assert!(!request.grounding_seed.entries.is_empty());
  assert!(request
    .grounding_seed
    .entries
    .iter()
    .all(|entry| entry.parser_capability == "emergency-compatibility-only"));
  assert_eq!(
    request.repo_graph_seed.seto_enrichment_state,
    "seto-disabled-optional"
  );
  assert_eq!(
    request.repo_graph_seed.bundle_scope,
    "multi-file-bounded-project-summary"
  );
  assert_eq!(
    request.repo_graph_seed.graph_capability,
    "multi-file-bounded-project-summary"
  );
  assert_eq!(
    request.repo_graph_seed.incremental_refresh.refresh_mode,
    "repo-bounded-refresh"
  );
  assert!(request
    .repo_graph_seed
    .incremental_refresh
    .changed_files
    .is_empty());
  assert!(!request.repo_graph_seed.files.is_empty());
  assert!(request
    .repo_graph_seed
    .files
    .iter()
    .all(|file| !file.symbol_nodes.is_empty()));
  assert_eq!(
    request.manual_evidence_seed.join_owner,
    "doghouse-core::docset_query::query_joined_docset_evidence"
  );
  assert!(!request.manual_evidence_seed.uncertainty_receipts.is_empty());
}

#[test]
fn coding_agent_request_emits_attached_pack_seed() {
  let root = temp_dir("coding-agent-pack-seed");
  let project_root = root.join("project-pack");
  let history_root = root.join("history-pack");
  fs::create_dir_all(&project_root).unwrap();
  fs::create_dir_all(history_root.join("timeline")).unwrap();
  fs::write(project_root.join("pack.json"), "{}").unwrap();
  fs::write(history_root.join("history.org"), "* history").unwrap();

  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "ask",
    "--request",
    "attach surface 보여줘",
    "--project-pack-root",
    project_root.to_str().unwrap(),
    "--history-pack-root",
    history_root.to_str().unwrap(),
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Ask).expect("build request");
  assert_eq!(
    request.attached_pack_seed.attach_owner,
    "pnix-executor-graph::coding-agent::attached-pack-seed"
  );
  assert_eq!(request.attached_pack_seed.project_pack_roots.len(), 1);
  assert_eq!(request.attached_pack_seed.history_pack_roots.len(), 1);
  assert_eq!(request.attached_pack_seed.total_entry_count, 3);
  assert_eq!(
    request.attached_pack_seed.project_pack_roots[0].pack_kind,
    "project-pack"
  );
  assert_eq!(
    request.attached_pack_seed.history_pack_roots[0].pack_kind,
    "history-pack"
  );
  assert_eq!(
    request.attached_pack_seed.history_pack_roots[0].entry_count,
    2
  );
  assert_eq!(
    request.attached_pack_seed.project_pack_roots[0].entries[0].entry_kind,
    "json-pack"
  );
  assert!(request.attached_pack_seed.history_pack_roots[0]
    .entries
    .iter()
    .any(|entry| entry.entry_kind == "org-pack" || entry.entry_kind == "directory-pack"));
}

#[test]
fn coding_agent_request_emits_pnix_and_rust_language_profile_records() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "ask",
    "--request",
    "CAX.5 adapter profile smoke",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--target-path",
    "demo/demo.px",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Ask).expect("build request");
  assert_eq!(
    request.language_profile.artifact_family,
    "coding.language-profile"
  );
  assert_eq!(
    request.language_profile.adapter_boundary,
    "language-adapter-produces-records-not-judgement"
  );
  let adapter_languages = request
    .language_profile
    .supported_adapters
    .iter()
    .map(|adapter| adapter.language.as_str())
    .collect::<Vec<_>>();
  assert!(adapter_languages.contains(&"rust"));
  assert!(adapter_languages.contains(&"pnix"));
  assert_eq!(request.language_profile.semantic_records.len(), 2);
  assert_eq!(request.language_profile.effect_records.len(), 2);
  assert_eq!(request.language_profile.verify_targets.len(), 2);
  assert!(request
    .language_profile
    .semantic_records
    .iter()
    .all(|record| record.artifact_family == "pnix.semantic-record"
      && record.judgement_boundary == "adapter-record-only-not-judgement"));
  assert!(request
    .language_profile
    .effect_records
    .iter()
    .any(|record| record.language == "rust"
      && record
        .effect_classes
        .contains(&"compile-impact:verify-required".to_string())));
  assert!(request
    .language_profile
    .effect_records
    .iter()
    .any(|record| record.language == "pnix"
      && record
        .effect_classes
        .contains(&"pnix-lowering-impact:verify-required".to_string())));
  assert!(request
    .language_profile
    .verify_targets
    .iter()
    .any(|target| target.language == "rust"
      && target
        .command_candidates
        .contains(&"cargo check -p pnix-executor-graph".to_string())));
  assert!(request
    .language_profile
    .verify_targets
    .iter()
    .any(|target| target.language == "pnix"
      && target
        .command_candidates
        .contains(&"pnix parse demo/demo.px".to_string())));
  assert!(request.language_profile.unsupported_targets.is_empty());
}

#[test]
fn coding_agent_language_profile_emits_diagnostic_bridge_for_unsupported_adapter() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "ask",
    "--request",
    "CAX.5b unsupported adapter bridge smoke",
    "--target-path",
    "docs/cli-coding-agent.md",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Ask).expect("build request");
  assert!(request.language_profile.semantic_records.is_empty());
  assert!(request.language_profile.effect_records.is_empty());
  assert!(request.language_profile.verify_targets.is_empty());
  assert_eq!(request.language_profile.unsupported_targets.len(), 1);
  assert_eq!(
    request.language_profile.unsupported_targets[0].detected_language,
    "unknown"
  );
  assert_eq!(
    request.language_profile.unsupported_targets[0].status,
    "unsupported"
  );
  assert_eq!(request.language_profile.diagnostic_records.len(), 1);
  assert_eq!(
    request.language_profile.diagnostic_records[0].artifact_family,
    "pnix.diagnostic-record"
  );
  assert_eq!(
    request.language_profile.diagnostic_records[0].diagnostic_family,
    "language-adapter-not-ready"
  );
  assert_eq!(request.language_profile.failure_pattern_matches.len(), 1);
  assert_eq!(
    request.language_profile.failure_pattern_matches[0].pattern_key,
    "missing-or-planned-language-adapter"
  );
  assert_eq!(request.language_profile.context_demands.len(), 1);
  assert_eq!(
    request.language_profile.context_demands[0].request_boundary,
    "request-more-context-before-patch-proposal"
  );
}

#[test]
fn coding_agent_language_profile_emits_planned_adapter_record_producers() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "ask",
    "--request",
    "CAX.5c planned adapter record producers",
    "--target-path",
    "tool/python/puckPy/puck_py.py",
    "--target-path",
    "editors/vscode/src/extension.ts",
    "--target-path",
    "tool/gimp/puckGimp/default.nix",
    "--target-path",
    "tool/clojure/puckClj/src/puck_clj/core.clj",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Ask).expect("build request");
  let adapter_languages = request
    .language_profile
    .supported_adapters
    .iter()
    .map(|adapter| (adapter.language.as_str(), adapter.adapter_status))
    .collect::<Vec<_>>();
  for language in ["python", "typescript", "nix", "clojure"] {
    assert!(adapter_languages
      .iter()
      .any(|(candidate, status)| candidate == &language && *status == "planned-record-producer"));
  }
  assert_eq!(request.language_profile.semantic_records.len(), 4);
  assert_eq!(request.language_profile.effect_records.len(), 4);
  assert_eq!(request.language_profile.verify_targets.len(), 4);
  assert!(request.language_profile.diagnostic_records.is_empty());
  assert!(request.language_profile.context_demands.is_empty());
  assert!(request.language_profile.unsupported_targets.is_empty());
  assert!(request
    .language_profile
    .semantic_records
    .iter()
    .all(|record| record.record_status == "candidate-planned-adapter"
      && record.parser_capability == "planned-record-producer-only-no-parser-ownership"));
  assert!(request
    .language_profile
    .verify_targets
    .iter()
    .any(|target| target.language == "python"
      && target
        .command_candidates
        .contains(&"python -m pytest".to_string())));
  assert!(request
    .language_profile
    .verify_targets
    .iter()
    .any(|target| target.language == "typescript"
      && target
        .command_candidates
        .contains(&"npx tsc --noEmit".to_string())));
  assert!(request
    .language_profile
    .verify_targets
    .iter()
    .any(|target| target.language == "nix"
      && target
        .command_candidates
        .contains(&"nix flake check".to_string())));
  assert!(request
    .language_profile
    .verify_targets
    .iter()
    .any(|target| target.language == "clojure"
      && target
        .command_candidates
        .contains(&"clojure -M:test".to_string())));
}

#[cfg(feature = "doghouse")]
#[test]
fn coding_agent_verify_receipt_persists_to_doghouse_coding_memory_store() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "verify",
    "--request",
    "crates/pnix-executor-graph/src/cli.rs 수정 검증",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--current-plan-ref",
    "plan:cli-verify",
    "--agent-verify-out",
    "runtime/verify.json",
  ])
  .expect("parse args");

  let request = build_coding_agent_request(&args, AgentVerb::Verify).expect("build request");
  let receipt =
    build_coding_agent_verify_receipt(&args, request, Some("runtime/request.json".to_string()));
  let store_dir = temp_dir("coding-memory-verify");
  let store_path = store_dir.join("doghouse.redb");
  let related_refs = build_coding_memory_related_refs(
    args.agent_verify_out.as_ref(),
    [
      receipt.request_artifact_ref.as_deref(),
      Some(receipt.before_artifact_ref.as_str()),
      Some(receipt.after_artifact_ref.as_str()),
      Some(receipt.diff_ref.as_str()),
    ],
  );

  let artifact_id = persist_coding_memory_artifact_to_store(
    &store_path,
    receipt.artifact_family,
    Some(receipt.repo_snapshot_ref.clone()),
    receipt.target_paths.clone(),
    receipt.target_commands.clone(),
    related_refs,
    &receipt,
  )
  .expect("persist coding memory artifact");

  let store = DoghouseStore::open(DoghouseStoreConfig::new(store_path)).expect("open store");
  let loaded = doghouse_core::store::read_coding_memory_artifact_at(store.path(), &artifact_id)
    .expect("load coding memory artifact")
    .expect("stored artifact exists");
  assert_eq!(loaded.artifact_family, "coding.verify-receipt");
  assert_eq!(
    loaded.repo_snapshot_ref.as_deref(),
    Some(receipt.repo_snapshot_ref.as_str())
  );
  assert_eq!(
    loaded.target_paths,
    vec!["crates/pnix-executor-graph/src/cli.rs".to_string()]
  );
  assert_eq!(
    loaded.command_refs,
    vec!["cargo check -p pnix-executor-graph".to_string()]
  );
  assert!(loaded
    .related_refs
    .contains(&"runtime/verify.json".to_string()));
  assert!(loaded
    .related_refs
    .contains(&"runtime/request.json".to_string()));

  let by_family = doghouse_core::store::read_coding_memory_artifacts_by_family_at(
    store.path(),
    "coding.verify-receipt",
  )
  .expect("query by family");
  assert_eq!(by_family.len(), 1);
  assert_eq!(by_family[0].id, artifact_id);

  let by_snapshot = doghouse_core::store::read_coding_memory_artifacts_for_repo_snapshot_at(
    store.path(),
    receipt.repo_snapshot_ref.as_str(),
  )
  .expect("query by snapshot");
  assert_eq!(by_snapshot.len(), 1);
  assert_eq!(by_snapshot[0].id, artifact_id);

  let by_target = doghouse_core::store::read_coding_memory_artifacts_for_target_path_at(
    store.path(),
    "crates/pnix-executor-graph/src/cli.rs",
  )
  .expect("query by target path");
  assert_eq!(by_target.len(), 1);
  assert_eq!(by_target[0].id, artifact_id);
}

#[cfg(feature = "doghouse")]
#[test]
fn coding_agent_semantic_patch_review_persists_to_doghouse_coding_memory_store() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "semantic patch review 저장",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--current-plan-ref",
    "plan:semantic-review",
  ])
  .expect("parse args");
  let request = build_coding_agent_request(&args, AgentVerb::Patch).expect("build request");
  let proposal =
    build_coding_agent_patch_proposal(&args, request, Some("runtime/request.json".to_string()));
  let store_dir = temp_dir("coding-memory-semantic-review");
  let store_path = store_dir.join("doghouse.redb");
  let review = &proposal.semantic_review;
  let artifact_id = persist_coding_memory_artifact_to_store(
    &store_path,
    review.artifact_family,
    Some(make_repo_snapshot_ref(&proposal.request.workspace)),
    review.target_paths.clone(),
    proposal.request.workspace.approved_commands.clone(),
    build_coding_memory_related_refs(
      Option::<&std::path::PathBuf>::None,
      [
        proposal.request_artifact_ref.as_deref(),
        proposal.current_plan_ref.as_deref(),
        Some(proposal.diff_ref.as_str()),
        Some(review.meaning_impact_diff.impact_ref.as_str()),
        Some(review.patch_decision_link.link_ref.as_str()),
      ],
    ),
    review,
  )
  .expect("persist semantic review");

  let store = DoghouseStore::open(DoghouseStoreConfig::new(store_path)).expect("open store");
  let loaded = doghouse_core::store::read_coding_memory_artifact_at(store.path(), &artifact_id)
    .expect("load semantic review")
    .expect("semantic review exists");
  assert_eq!(loaded.artifact_family, "coding.semantic-patch-review");
  assert!(loaded
    .related_refs
    .contains(&review.meaning_impact_diff.impact_ref));
  assert_eq!(
    loaded.payload["artifact_family"],
    "coding.semantic-patch-review"
  );
  let by_family = doghouse_core::store::read_coding_memory_artifacts_by_family_at(
    store.path(),
    "coding.semantic-patch-review",
  )
  .expect("query semantic review family");
  assert_eq!(by_family.len(), 1);
  assert_eq!(by_family[0].id, artifact_id);
}

#[cfg(feature = "doghouse")]
#[test]
fn coding_agent_context_demand_replay_reuses_verify_and_semantic_review_from_store() {
  let store_dir = temp_dir("coding-memory-context-replay");
  let store_path = store_dir.join("doghouse.redb");

  let verify_args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "verify",
    "--request",
    "prior verify failure",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "printf ok | cat",
  ])
  .expect("parse verify args");
  let verify_request =
    build_coding_agent_request(&verify_args, AgentVerb::Verify).expect("build verify request");
  let mut verify_receipt = build_coding_agent_verify_receipt(
    &verify_args,
    verify_request,
    Some("runtime/request.json".to_string()),
  );
  let execution_result = run_coding_agent_verify_commands(
    &verify_receipt.request,
    &verify_receipt.repo_snapshot_ref,
    &verify_receipt.diff_ref,
    &verify_receipt.target_commands,
  );
  attach_coding_agent_verify_execution_result(&mut verify_receipt, execution_result);
  assert_eq!(verify_receipt.context_demands.len(), 1);
  let verify_artifact_id = persist_coding_memory_artifact_to_store(
    &store_path,
    verify_receipt.artifact_family,
    Some(verify_receipt.repo_snapshot_ref.clone()),
    verify_receipt.target_paths.clone(),
    verify_receipt.target_commands.clone(),
    build_coding_memory_related_refs(
      Option::<&std::path::PathBuf>::None,
      [Some(verify_receipt.diff_ref.as_str())],
    ),
    &verify_receipt,
  )
  .expect("persist verify receipt");

  let prior_patch_args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "prior semantic review",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
  ])
  .expect("parse prior patch args");
  let prior_patch_request = build_coding_agent_request(&prior_patch_args, AgentVerb::Patch)
    .expect("build prior patch request");
  let prior_patch = build_coding_agent_patch_proposal(
    &prior_patch_args,
    prior_patch_request,
    Some("runtime/request.json".to_string()),
  );
  let semantic_review_id = persist_coding_memory_artifact_to_store(
    &store_path,
    prior_patch.semantic_review.artifact_family,
    Some(make_repo_snapshot_ref(&prior_patch.request.workspace)),
    prior_patch.semantic_review.target_paths.clone(),
    prior_patch.request.workspace.approved_commands.clone(),
    build_coding_memory_related_refs(
      Option::<&std::path::PathBuf>::None,
      [
        Some(prior_patch.diff_ref.as_str()),
        Some(prior_patch.semantic_review.review_ref.as_str()),
      ],
    ),
    &prior_patch.semantic_review,
  )
  .expect("persist semantic review");

  let next_patch_args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "next patch must reuse prior failure context",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--last-verification-ref",
    verify_artifact_id.as_str(),
  ])
  .expect("parse next patch args");
  let next_patch_request =
    build_coding_agent_request(&next_patch_args, AgentVerb::Patch).expect("build next request");
  let replay = build_coding_agent_context_demand_replay_from_store_path(
    &next_patch_request,
    Some(&store_path),
  );

  assert_eq!(replay.artifact_family, "coding.context-demand-replay");
  assert_eq!(replay.replay_status, "candidate-context-replayed");
  assert!(replay.source_artifact_refs.contains(&verify_artifact_id));
  assert!(replay.source_artifact_refs.contains(&semantic_review_id));
  assert!(replay.replayed_context_demands.iter().any(|demand| {
    demand.source_family == "coding.verify-receipt"
      && demand.demand_family == "verify-failure-context-required"
  }));
  assert!(replay.replayed_context_demands.iter().any(|demand| {
    demand.source_family == "coding.semantic-patch-review"
      && demand.demand_family == "semantic-review-followup-context-required"
  }));
  assert!(replay
    .semantic_review_refs
    .contains(&prior_patch.semantic_review.review_ref));
  assert!(replay
    .next_patch_requirements
    .contains(&"review-prior-meaning-impact-before-new-patch".to_string()));
  assert_eq!(
    replay.promotion_boundary,
    "candidate-only-requires-new-patch-proposal-review"
  );
  let replay_artifact_id = persist_coding_memory_artifact_to_store(
    &store_path,
    replay.artifact_family,
    Some(make_repo_snapshot_ref(&next_patch_request.workspace)),
    next_patch_request.workspace.target_paths.clone(),
    next_patch_request.workspace.approved_commands.clone(),
    replay.source_artifact_refs.clone(),
    &replay,
  )
  .expect("persist context demand replay");
  let store = DoghouseStore::open(DoghouseStoreConfig::new(store_path)).expect("open store");
  let by_family = doghouse_core::store::read_coding_memory_artifacts_by_family_at(
    store.path(),
    "coding.context-demand-replay",
  )
  .expect("query context demand replay family");
  assert_eq!(by_family.len(), 1);
  assert_eq!(by_family[0].id, replay_artifact_id);
}

#[cfg(feature = "doghouse")]
#[test]
fn coding_agent_repair_recipe_replay_reuses_learning_card_from_store() {
  let store_dir = temp_dir("coding-memory-repair-replay");
  let store_path = store_dir.join("doghouse.redb");

  let verify_args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "verify",
    "--request",
    "prior verify failure for repair recipe",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "printf ok | cat",
  ])
  .expect("parse verify args");
  let verify_request =
    build_coding_agent_request(&verify_args, AgentVerb::Verify).expect("build verify request");
  let mut verify_receipt = build_coding_agent_verify_receipt(
    &verify_args,
    verify_request,
    Some("runtime/request.json".to_string()),
  );
  let execution_result = run_coding_agent_verify_commands(
    &verify_receipt.request,
    &verify_receipt.repo_snapshot_ref,
    &verify_receipt.diff_ref,
    &verify_receipt.target_commands,
  );
  attach_coding_agent_verify_execution_result(&mut verify_receipt, execution_result);
  assert_eq!(verify_receipt.status.result_status, "차단");
  assert_eq!(
    verify_receipt.learning_card.artifact_family,
    "coding.learning-card"
  );

  let verify_artifact_id = persist_coding_memory_artifact_to_store(
    &store_path,
    verify_receipt.artifact_family,
    Some(verify_receipt.repo_snapshot_ref.clone()),
    verify_receipt.target_paths.clone(),
    verify_receipt.target_commands.clone(),
    build_coding_memory_related_refs(
      Option::<&std::path::PathBuf>::None,
      [
        Some(verify_receipt.diff_ref.as_str()),
        Some(verify_receipt.learning_card.learning_card_ref.as_str()),
      ],
    ),
    &verify_receipt,
  )
  .expect("persist verify receipt");
  let learning_card_artifact_id = persist_coding_memory_artifact_to_store(
    &store_path,
    verify_receipt.learning_card.artifact_family,
    Some(verify_receipt.repo_snapshot_ref.clone()),
    verify_receipt.target_paths.clone(),
    verify_receipt.target_commands.clone(),
    build_coding_memory_related_refs(
      Option::<&std::path::PathBuf>::None,
      [
        Some(verify_artifact_id.as_str()),
        Some(verify_receipt.diff_ref.as_str()),
        Some(verify_receipt.learning_card.learning_card_ref.as_str()),
      ],
    ),
    &verify_receipt.learning_card,
  )
  .expect("persist learning card");

  let next_patch_args = parse_args_for_test(&[
    "pnix-executor-graph",
    "coding-agent",
    "patch",
    "--request",
    "next patch should replay repair recipe",
    "--target-path",
    "crates/pnix-executor-graph/src/cli.rs",
    "--approved-command",
    "cargo check -p pnix-executor-graph",
    "--last-verification-ref",
    verify_artifact_id.as_str(),
  ])
  .expect("parse next patch args");
  let next_patch_request =
    build_coding_agent_request(&next_patch_args, AgentVerb::Patch).expect("build next request");
  let replay =
    build_coding_agent_repair_recipe_replay_from_store_path(&next_patch_request, Some(&store_path));

  assert_eq!(replay.artifact_family, "coding.repair-recipe-replay");
  assert_eq!(replay.phase, "CAX.5h");
  assert_eq!(replay.replay_status, "candidate-repair-recipes-replayed");
  assert!(replay.source_artifact_refs.contains(&verify_artifact_id));
  assert!(replay
    .source_artifact_refs
    .contains(&learning_card_artifact_id));
  assert!(replay
    .learning_card_refs
    .contains(&verify_receipt.learning_card.learning_card_ref));
  assert!(replay.repair_candidates.iter().any(|candidate| {
    candidate.source_ref == verify_receipt.learning_card.learning_card_ref
      && candidate.repair_pattern == verify_receipt.learning_card.repair_pattern
      && candidate.verify_pattern == verify_receipt.learning_card.verify_pattern
      && candidate.promotion_boundary == "candidate-only-requires-current-context-review"
  }));
  assert_eq!(
    replay.promotion_boundary,
    "candidate-only-not-patch-generator"
  );

  let replay_artifact_id = persist_coding_memory_artifact_to_store(
    &store_path,
    replay.artifact_family,
    Some(make_repo_snapshot_ref(&next_patch_request.workspace)),
    next_patch_request.workspace.target_paths.clone(),
    next_patch_request.workspace.approved_commands.clone(),
    replay.source_artifact_refs.clone(),
    &replay,
  )
  .expect("persist repair recipe replay");
  let store = DoghouseStore::open(DoghouseStoreConfig::new(store_path)).expect("open store");
  let by_family = doghouse_core::store::read_coding_memory_artifacts_by_family_at(
    store.path(),
    "coding.repair-recipe-replay",
  )
  .expect("query repair recipe replay family");
  assert_eq!(by_family.len(), 1);
  assert_eq!(by_family[0].id, replay_artifact_id);
}

#[test]
fn u14_inputs_merge_precedence() {
  let dir = temp_dir("u14-inputs-merge");
  let path = dir.join("inputs.json");
  std::fs::write(&path, r#"{"a":1,"b":2}"#).unwrap();

  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "graph",
    "--dist",
    "dist",
    "--inputs",
    path.to_str().unwrap(),
    "--inputs-json",
    r#"{"b":3,"c":4}"#,
    "--input",
    "b=5",
    "--input",
    "d=6",
  ])
  .expect("parse args");

  assert_eq!(args.inputs.get("a"), Some(&serde_json::json!(1)));
  assert_eq!(args.inputs.get("b"), Some(&serde_json::json!(5)));
  assert_eq!(args.inputs.get("c"), Some(&serde_json::json!(4)));
  assert_eq!(args.inputs.get("d"), Some(&serde_json::json!(6)));
  assert_eq!(args.inputs.len(), 4);
}

#[test]
fn u14_input_pair_parses_json_and_string() {
  let (key, value) = parse_input_pair("num=123").expect("parse input pair");
  assert_eq!(key, "num");
  assert_eq!(value, serde_json::json!(123));

  let (key, value) = parse_input_pair("str=\"abc\"").expect("parse input pair");
  assert_eq!(key, "str");
  assert_eq!(value, serde_json::json!("abc"));
}

#[test]
fn compile_mode_accepts_dist_and_dry_run() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "compile",
    "--dist",
    "dist",
    "--expr",
    "{ name = \"x\"; }",
    "--dry-run",
  ])
  .expect("parse args");

  assert_eq!(args.mode, ExecMode::Compile);
  assert!(args.dry_run);
  assert!(args.dist.is_some());
}

#[test]
fn project_lock_written_when_missing() {
  let root = temp_dir("project-lock");
  fs::write(
    root.join("pnix.toml"),
    "name = \"app\"\nversion = \"0.1.0\"\n",
  )
  .unwrap();

  let entry_dir = root.join("src");
  fs::create_dir_all(&entry_dir).unwrap();
  let entry_path = entry_dir.join("main.px");
  fs::write(&entry_path, "{ name = \"app\"; }").unwrap();

  sync_project_lock(&entry_path, false).expect("sync project lock");
  assert!(root.join("pnix.lock").exists());
}

#[test]
fn project_lock_dry_run_skips_write() {
  let root = temp_dir("project-lock-dry");
  fs::write(
    root.join("pnix.toml"),
    "name = \"app\"\nversion = \"0.1.0\"\n",
  )
  .unwrap();

  let entry_dir = root.join("src");
  fs::create_dir_all(&entry_dir).unwrap();
  let entry_path = entry_dir.join("main.px");
  fs::write(&entry_path, "{ name = \"app\"; }").unwrap();

  sync_project_lock(&entry_path, true).expect("sync project lock dry-run");
  assert!(!root.join("pnix.lock").exists());
}

#[test]
fn legacy_eval_deterministic_with_seed_now_clock_step() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    LEGACY_EVAL_MODE,
    "--expr",
    "let x = 4; in x * 3",
    "--seed",
    "123",
    "--now",
    "5000",
    "--clock-step",
    "16",
  ])
  .expect("parse args");

  assert_eq!(args.seed, Some(123));
  assert_eq!(args.now_ms, Some(5000));
  assert_eq!(args.clock_step_ms, Some(16));

  let first = legacy_eval_output_value(&args).expect("legacy eval 1");
  let second = legacy_eval_output_value(&args).expect("legacy eval 2");
  assert_eq!(first, second, "legacy eval should be deterministic");

  assert_eq!(first.as_i64(), Some(12));
}

#[test]
fn ct_deterministic_with_seed_now_clock_step() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "ct",
    "--expr",
    "sin(t)",
    "--seed",
    "42",
    "--now",
    "1200",
    "--clock-step",
    "33",
  ])
  .expect("parse args");

  let first = ct_output_value(&args).expect("ct output 1");
  let second = ct_output_value(&args).expect("ct output 2");
  assert_eq!(first, second, "ct output should be deterministic");
}

#[test]
fn ct_output_includes_diagram_for_valid_expr() {
  let args = parse_args_for_test(&["pnix-executor-graph", "--mode", "ct", "--expr", "sin(t)"])
    .expect("parse args");

  let output = ct_output_value(&args).expect("ct output");
  assert_eq!(output.get("ok").and_then(|v| v.as_bool()), Some(true));
  assert_eq!(output.get("success").and_then(|v| v.as_bool()), Some(true));
  assert_eq!(output.get("strict").and_then(|v| v.as_bool()), Some(true));

  let notes = output
    .get("notes")
    .and_then(|v| v.as_array())
    .expect("notes array");
  assert!(!notes.is_empty(), "ct output notes should not be empty");

  let diagram = output
    .get("diagram")
    .and_then(|v| v.as_object())
    .expect("diagram object");
  let objects = diagram
    .get("objects")
    .and_then(|v| v.as_array())
    .expect("diagram objects");
  let morphisms = diagram
    .get("morphisms")
    .and_then(|v| v.as_array())
    .expect("diagram morphisms");

  assert!(
    !objects.is_empty(),
    "ct output should include at least one object"
  );
  assert!(
    !morphisms.is_empty(),
    "ct output should include at least one morphism"
  );
}

#[test]
fn ct_lenient_invalid_expr_returns_ok_false_without_diagram() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "ct",
    "--expr",
    "sin(",
    "--lenient-ct",
  ])
  .expect("parse args");

  let output = ct_output_value(&args).expect("lenient ct output");
  assert_eq!(output.get("ok").and_then(|v| v.as_bool()), Some(false));
  assert_eq!(output.get("success").and_then(|v| v.as_bool()), Some(false));
  assert_eq!(output.get("strict").and_then(|v| v.as_bool()), Some(false));
  assert!(
    output.get("diagram").map(|v| v.is_null()).unwrap_or(false),
    "lenient invalid ct output should not include a diagram"
  );

  let notes = output
    .get("notes")
    .and_then(|v| v.as_array())
    .expect("notes array");
  assert!(
    notes
      .iter()
      .filter_map(|v| v.as_str())
      .any(|line| line.contains("Parse error")),
    "lenient ct output should include parse error note"
  );
}

#[test]
fn ct_strict_invalid_expr_returns_error() {
  let args = parse_args_for_test(&["pnix-executor-graph", "--mode", "ct", "--expr", "sin("])
    .expect("parse args");

  let err = ct_output_value(&args).expect_err("strict ct should fail on invalid expression");
  assert!(
    err.to_string().contains("ct runtime failed"),
    "strict ct error should preserve runtime context"
  );
}

#[test]
fn parse_args_rejects_emit_with_non_graph_mode() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--emit",
    "--mode",
    LEGACY_EVAL_MODE,
    "--dist",
    "dist",
  ])
  .expect_err("emit with legacy eval should fail");
  let expected = format!("--emit cannot be combined with --mode {}", LEGACY_EVAL_MODE);
  assert!(err.to_string().contains(&expected));
}

#[test]
fn parse_args_rejects_emit_with_ct_mode_alias() {
  let err = parse_args_for_test(&["pnix-executor-graph", "--emit", "--ct", "--dist", "dist"])
    .expect_err("emit with ct mode should fail");
  assert!(err
    .to_string()
    .contains("--emit cannot be combined with --mode ct"));
}

#[test]
fn u18_rejects_unknown_flag() {
  let err = parse_args_for_test(&["pnix-executor-graph", "--no-such-flag"])
    .expect_err("unknown flag must fail");
  assert!(err.to_string().contains("unknown flag"));
}

#[test]
fn u18_rejects_missing_flag_value() {
  let err = parse_args_for_test(&["pnix-executor-graph", "--mode", "run", "--dist"])
    .expect_err("missing flag value must fail");
  assert!(err.to_string().contains("--dist requires a value"));
}

#[test]
fn u30_graph_allows_rpc_timeout_ms() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "graph",
    "--dist",
    "dist",
    "--rpc-timeout-ms",
    "1234",
  ])
  .expect("parse args");

  assert_eq!(args.mode, ExecMode::Graph);
  assert_eq!(args.rpc_timeout_ms, 1234);
}

#[test]
fn u30_graph_allows_rpc_retry_attempts() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "graph",
    "--dist",
    "dist",
    "--rpc-retry-attempts",
    "5",
  ])
  .expect("parse args");

  assert_eq!(args.mode, ExecMode::Graph);
  assert_eq!(args.rpc_retry_attempts, 5);
}

#[test]
fn u30_graph_allows_rpc_retry_backoff_ms() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "graph",
    "--dist",
    "dist",
    "--rpc-retry-backoff-ms",
    "250",
  ])
  .expect("parse args");

  assert_eq!(args.mode, ExecMode::Graph);
  assert_eq!(args.rpc_retry_backoff_ms, 250);
}

#[test]
fn u30_graph_allows_blenderpy_url() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "graph",
    "--dist",
    "dist",
    "--blenderpy-url",
    "http://localhost:17781",
  ])
  .expect("parse args");

  assert_eq!(args.mode, ExecMode::Graph);
  assert_eq!(args.blenderpy_url, "http://localhost:17781");
}

#[test]
fn u30_rejects_rpc_timeout_ms_outside_graph() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "compile",
    "--dist",
    "dist",
    "--expr",
    "{ name = \"x\"; }",
    "--rpc-timeout-ms",
    "1",
  ])
  .expect_err("rpc timeout outside graph must fail");
  assert!(err
    .to_string()
    .contains("is only supported for graph execution"));
}

#[test]
fn u30_rejects_blenderpy_url_outside_graph() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "compile",
    "--dist",
    "dist",
    "--expr",
    "{ name = \"x\"; }",
    "--blenderpy-url",
    "http://localhost:17781",
  ])
  .expect_err("blenderpy url outside graph must fail");
  assert!(err
    .to_string()
    .contains("is only supported for graph execution"));
}

#[test]
fn u30_rejects_rpc_retry_attempts_outside_graph() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "compile",
    "--dist",
    "dist",
    "--expr",
    "{ name = \"x\"; }",
    "--rpc-retry-attempts",
    "2",
  ])
  .expect_err("rpc retry attempts outside graph must fail");
  assert!(err
    .to_string()
    .contains("is only supported for graph execution"));
}

#[test]
fn u30_rejects_rpc_retry_backoff_ms_outside_graph() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "compile",
    "--dist",
    "dist",
    "--expr",
    "{ name = \"x\"; }",
    "--rpc-retry-backoff-ms",
    "10",
  ])
  .expect_err("rpc retry backoff outside graph must fail");
  assert!(err
    .to_string()
    .contains("is only supported for graph execution"));
}

#[test]
fn u30_rejects_rpc_timeout_ms_zero() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "graph",
    "--dist",
    "dist",
    "--rpc-timeout-ms",
    "0",
  ])
  .expect_err("rpc timeout 0 must fail");
  assert!(err.to_string().contains("--rpc-timeout-ms must be >= 1"));
}

#[test]
fn u30_rejects_rpc_retry_attempts_zero() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "graph",
    "--dist",
    "dist",
    "--rpc-retry-attempts",
    "0",
  ])
  .expect_err("rpc retry attempts 0 must fail");
  assert!(err
    .to_string()
    .contains("--rpc-retry-attempts must be >= 1"));
}

#[test]
fn u30_rejects_rpc_retry_backoff_ms_zero() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "graph",
    "--dist",
    "dist",
    "--rpc-retry-backoff-ms",
    "0",
  ])
  .expect_err("rpc retry backoff 0 must fail");
  assert!(err
    .to_string()
    .contains("--rpc-retry-backoff-ms must be >= 1"));
}

#[test]
fn u19_rejects_dt_outside_legacy_frp() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    LEGACY_EVAL_MODE,
    "--expr",
    "1",
    "--dt",
    "0.1",
  ])
  .expect_err("dt outside legacy-frp must fail");
  assert!(err.to_string().contains("--dt is only supported"));
}

#[test]
fn u19_rejects_backend_flags_outside_graph() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    LEGACY_EVAL_MODE,
    "--expr",
    "1",
    "--clojure-url",
    "http://localhost:7777",
  ])
  .expect_err("backend url outside graph must fail");
  assert!(err.to_string().contains("graph execution"));
}

#[test]
fn u19_rejects_inputs_outside_graph_frp_llvm() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    LEGACY_EVAL_MODE,
    "--expr",
    "1",
    "--input",
    "x=1",
  ])
  .expect_err("inputs outside supported modes must fail");
  assert!(err.to_string().contains("external inputs flags"));
}

#[test]
fn u19_rejects_result_outside_ir_eval() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "run",
    "--engine",
    "graph",
    "--dist",
    "dist",
    "--result",
    "x",
  ])
  .expect_err("--result outside ir-eval must fail");
  assert!(err.to_string().contains("--result is only supported"));
}

#[test]
fn interpret_mode_allows_repl_without_source_or_expr() {
  let args = parse_args_for_test(&["pnix-executor-graph", "--mode", "interpret"])
    .expect("interpret repl parse should succeed");
  assert_eq!(args.mode, ExecMode::Interpret);
  assert!(args.source.is_none());
  assert!(args.expr.is_none());
}

#[test]
fn interpret_mode_eval_alias_allows_repl_without_source_or_expr() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "interpret",
    "--engine",
    "eval",
  ])
  .expect("interpret --engine eval repl parse should succeed");
  assert_eq!(args.mode, ExecMode::Interpret);
  assert_eq!(args.engine.as_deref(), Some("eval"));
  assert!(args.source.is_none());
  assert!(args.expr.is_none());
}

#[test]
fn interpret_mode_live_allows_legacy_repl_without_source_or_expr() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "interpret",
    "--live",
    "--live-dir",
    "target/repl-live-test",
  ])
  .expect("interpret --live (legacy-eval repl) should parse");
  assert_eq!(args.mode, ExecMode::Interpret);
  assert!(args.live);
  assert_eq!(
    args.live_dir,
    Some(std::path::PathBuf::from("target/repl-live-test"))
  );
}

#[test]
fn interpret_mode_live_dir_only_allows_legacy_repl_without_source_or_expr() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "interpret",
    "--live-dir",
    "target/repl-live-test-legacy-only-dir",
  ])
  .expect("interpret --live-dir (legacy-eval repl) should parse");
  assert_eq!(args.mode, ExecMode::Interpret);
  assert!(args.live);
  assert_eq!(
    args.live_dir,
    Some(std::path::PathBuf::from(
      "target/repl-live-test-legacy-only-dir"
    ))
  );
}

#[test]
fn interpret_mode_live_allows_eval_alias_repl_without_source_or_expr() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "interpret",
    "--engine",
    "eval",
    "--live",
    "--live-dir",
    "target/repl-live-test-eval",
  ])
  .expect("interpret --engine eval --live repl parse should succeed");
  assert_eq!(args.mode, ExecMode::Interpret);
  assert_eq!(args.engine.as_deref(), Some("eval"));
  assert!(args.live);
  assert_eq!(
    args.live_dir,
    Some(std::path::PathBuf::from("target/repl-live-test-eval"))
  );
}

#[test]
fn interpret_mode_live_dir_only_allows_eval_alias_repl_without_source_or_expr() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "interpret",
    "--engine",
    "eval",
    "--live-dir",
    "target/repl-live-test-eval-only-dir",
  ])
  .expect("interpret --engine eval --live-dir repl parse should succeed");
  assert_eq!(args.mode, ExecMode::Interpret);
  assert_eq!(args.engine.as_deref(), Some("eval"));
  assert!(args.live);
  assert_eq!(
    args.live_dir,
    Some(std::path::PathBuf::from(
      "target/repl-live-test-eval-only-dir"
    ))
  );
}

#[test]
fn interpret_mode_live_dir_only_allows_explicit_legacy_eval_repl_without_source_or_expr() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "interpret",
    "--engine",
    "legacy-eval",
    "--live-dir",
    "target/repl-live-test-legacy-explicit-only-dir",
  ])
  .expect("interpret --engine legacy-eval --live-dir repl parse should succeed");
  assert_eq!(args.mode, ExecMode::Interpret);
  assert_eq!(args.engine.as_deref(), Some("legacy-eval"));
  assert!(args.live);
  assert_eq!(
    args.live_dir,
    Some(std::path::PathBuf::from(
      "target/repl-live-test-legacy-explicit-only-dir"
    ))
  );
}

#[test]
fn interpret_mode_non_repl_engine_requires_source_or_expr() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "interpret",
    "--engine",
    "ui",
  ])
  .expect_err("ui interpret without source should fail");
  assert!(err
    .to_string()
    .contains("--source or --expr is required for --mode interpret --engine ui"));
}

#[test]
fn interpret_mode_graph_requires_dist() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "interpret",
    "--engine",
    "graph",
    "--source",
    "fixtures/pnix_module/hello.px",
  ])
  .expect_err("interpret --engine graph without dist should fail");
  assert!(err
    .to_string()
    .contains("--dist is required for --mode interpret --engine graph"));
}

#[test]
fn interpret_mode_graph_allows_graph_flags_and_dry_run() {
  let args = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "interpret",
    "--engine",
    "graph",
    "--source",
    "fixtures/pnix_module/hello.px",
    "--dist",
    "dist",
    "--inputs-json",
    "{\"x\":1}",
    "--clojure-url",
    "http://localhost:7777",
    "--no-batch",
    "--dry-run",
  ])
  .expect("interpret --engine graph parse should allow graph flags");

  assert_eq!(args.mode, ExecMode::Interpret);
  assert_eq!(args.engine.as_deref(), Some("graph"));
  assert_eq!(args.dist, Some(std::path::PathBuf::from("dist")));
  assert_eq!(args.clojure_url, "http://localhost:7777");
  assert!(!args.use_batch);
  assert!(args.dry_run);
  assert_eq!(args.inputs.get("x"), Some(&serde_json::json!(1)));
}

#[test]
fn interpret_mode_live_rejects_non_ui_non_legacy_engine() {
  let err = parse_args_for_test(&[
    "pnix-executor-graph",
    "--mode",
    "interpret",
    "--engine",
    "legacy-frp",
    "--source",
    "1",
    "--live",
  ])
  .expect_err("interpret --engine legacy-frp --live should fail");
  assert!(err
    .to_string()
    .contains("--live/--live-dir is only supported"));
  assert!(err.to_string().contains("legacy-eval/eval"));
}

#[test]
fn ir_eval_respects_to_port_ordering_for_sub() {
  let dist = temp_dir("ir-eval-ordering");
  std::fs::create_dir_all(dist.join("ir")).unwrap();

  // edges into sub_node are intentionally listed in the "wrong" order:
  // the `b` port (input d) appears before the `a` port (mul result).
  // ir-eval must still compute (a + b) * c - d.
  std::fs::write(
    dist.join("ir").join("fxcore.canon.json"),
    r#"
{
  "meta": { "version": "fxcore@0.1", "stage": 2 },
  "name": "arithmetic_chain",
  "types": ["Number"],
  "inputs": [
{ "name": "a", "ty": "Number" },
{ "name": "b", "ty": "Number" },
{ "name": "c", "ty": "Number" },
{ "name": "d", "ty": "Number" }
  ],
  "morphisms": [
{
  "effect": "pure",
  "input": "Number",
  "inputs": [
    { "name": "a", "ty": "Number" },
    { "name": "b", "ty": "Number" }
  ],
  "name": "py.add",
  "output": "Number",
  "outputs": [ { "name": "out", "ty": "Number" } ]
},
{
  "effect": "pure",
  "input": "Number",
  "inputs": [
    { "name": "a", "ty": "Number" },
    { "name": "b", "ty": "Number" }
  ],
  "name": "py.mul",
  "output": "Number",
  "outputs": [ { "name": "out", "ty": "Number" } ]
},
{
  "effect": "pure",
  "input": "Number",
  "inputs": [
    { "name": "a", "ty": "Number" },
    { "name": "b", "ty": "Number" }
  ],
  "name": "py.sub",
  "output": "Number",
  "outputs": [ { "name": "out", "ty": "Number" } ]
}
  ],
  "nodes": [
{ "name": "add_node", "uses": "py.add" },
{ "name": "mul_node", "uses": "py.mul" },
{ "name": "sub_node", "uses": "py.sub" }
  ],
  "edges": [
{ "from": "input", "from_input": "a", "to": "add_node", "to_port": "a" },
{ "from": "input", "from_input": "b", "to": "add_node", "to_port": "b" },
{ "from": "add_node", "from_port": "out", "to": "mul_node", "to_port": "a" },
{ "from": "input", "from_input": "c", "to": "mul_node", "to_port": "b" },
{ "from": "input", "from_input": "d", "to": "sub_node", "to_port": "b" },
{ "from": "mul_node", "from_port": "out", "to": "sub_node", "to_port": "a" }
  ],
  "scopes": []
}
"#,
  )
  .unwrap();

  let mut inputs = HashMap::new();
  inputs.insert("a".to_string(), serde_json::json!(1));
  inputs.insert("b".to_string(), serde_json::json!(2));
  inputs.insert("c".to_string(), serde_json::json!(3));
  inputs.insert("d".to_string(), serde_json::json!(4));

  let args = Args {
    bin_name: "pnix-executor-graph".into(),
    mode: ExecMode::Run,
    agent: None,
    gate_absorb: None,
    gate_absorb_subject: None,
    gate_absorb_follow_related: None,
    gate_absorb_limit: None,
    gate_absorb_reset: false,
    gate_forward: None,
    gate_read: None,
    gate_forward_limit: None,
    gate_forward_kind: None,
    gate_forward_reset: false,
    gate_forward_url: None,
    gate_read_context: None,
    gate_read_predicate: None,
    gate_read_topic: None,
    gate_read_event_types: vec![],
    gate_read_tool_name: None,
    gate_read_arg_predicates: vec![],
    gate_read_limit: None,
    gate_read_min_confidence: None,
    gate_read_kind: None,
    gate_read_path: None,
    gate_read_proof_path: None,
    gate_read_schema_path: None,
    gate_read_expected_bundle_kind: None,
    gate_read_expected_lobe_profile: None,
    gate_read_expected_proof_kind: None,
    agent_request: None,
    agent_target_paths: vec![],
    agent_project_pack_roots: vec![],
    agent_history_pack_roots: vec![],
    agent_approved_commands: vec![],
    agent_forbidden_paths: vec![],
    agent_policy_bits: vec![],
    agent_current_plan_ref: None,
    agent_rollback_handle_ref: None,
    agent_last_verification_ref: None,
    agent_promotion_boundary_ref: None,
    agent_source_apply_artifact_ref: None,
    agent_source_handoff_ref: None,
    agent_promotion_boundary_join_ref: None,
    agent_promotion_decision: None,
    agent_candidate_patch: None,
    agent_provider_feedback_request_ref: None,
    agent_request_out: None,
    agent_plan_out: None,
    agent_patch_out: None,
    agent_verify_out: None,
    agent_rollback_out: None,
    agent_decision_out: None,
    engine: Some("ir-eval".into()),
    result: None,
    dist: Some(dist),
    clojure_url: "http://localhost:7777".into(),
    python_url: "http://localhost:7778".into(),
    deno_url: "http://localhost:7779".into(),
    blenderpy_url: "http://localhost:7781".into(),
    supervisor_sock: None,
    backend_specs: None,
    auto_ensure_backends: true,
    replay_trace: None,
    replay_mode: None,
    replay_allow: vec![],
    invocation_id: None,
    rpc_timeout_ms: 30_000,
    rpc_retry_attempts: 3,
    rpc_retry_backoff_ms: 100,
    max_nodes: 10_000,
    max_edges: 50_000,
    max_input_bytes: 10 * 1024 * 1024,
    use_batch: true,
    source: None,
    expr: None,
    test_filter: None,
    patch: None,
    deterministic: true,
    strict_ct: true,
    inputs,
    seed: Some(0),
    now_ms: Some(0),
    clock_step_ms: Some(1),
    frp_dt: None,
    inputs_schema: false,
    list_modes: false,
    list_ir_eval_ops: false,
    version: false,
    dry_run: false,
    emit: false,
    emit_target: None,
    binary: false,
    emit_out: None,
    emit_manifest: None,
    fmt_check: false,
    live: false,
    live_dir: None,
    output_format: super::args::OutputFormat::Text,
  };

  let out = ir_eval_output_value(&args).expect("ir-eval output");
  assert_eq!(out.get("engine").and_then(|v| v.as_str()), Some("ir-eval"));
  assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
  assert_eq!(out.get("value").and_then(|v| v.as_f64()), Some(5.0));
}

#[test]
fn ir_eval_result_flag_selects_named_node() {
  let dist = temp_dir("ir-eval-result-flag");
  std::fs::create_dir_all(dist.join("ir")).unwrap();

  std::fs::write(
    dist.join("ir").join("fxcore.canon.json"),
    r#"
{
  "meta": { "version": "fxcore@0.1", "stage": 2 },
  "name": "arithmetic_chain",
  "types": ["Number"],
  "inputs": [
{ "name": "a", "ty": "Number" },
{ "name": "b", "ty": "Number" },
{ "name": "c", "ty": "Number" },
{ "name": "d", "ty": "Number" }
  ],
  "morphisms": [
{
  "effect": "pure",
  "input": "Number",
  "inputs": [
    { "name": "a", "ty": "Number" },
    { "name": "b", "ty": "Number" }
  ],
  "name": "py.add",
  "output": "Number",
  "outputs": [ { "name": "out", "ty": "Number" } ]
},
{
  "effect": "pure",
  "input": "Number",
  "inputs": [
    { "name": "a", "ty": "Number" },
    { "name": "b", "ty": "Number" }
  ],
  "name": "py.mul",
  "output": "Number",
  "outputs": [ { "name": "out", "ty": "Number" } ]
},
{
  "effect": "pure",
  "input": "Number",
  "inputs": [
    { "name": "a", "ty": "Number" },
    { "name": "b", "ty": "Number" }
  ],
  "name": "py.sub",
  "output": "Number",
  "outputs": [ { "name": "out", "ty": "Number" } ]
}
  ],
  "nodes": [
{ "name": "add_node", "uses": "py.add" },
{ "name": "mul_node", "uses": "py.mul" },
{ "name": "sub_node", "uses": "py.sub" }
  ],
  "edges": [
{ "from": "input", "from_input": "a", "to": "add_node", "to_port": "a" },
{ "from": "input", "from_input": "b", "to": "add_node", "to_port": "b" },
{ "from": "add_node", "from_port": "out", "to": "mul_node", "to_port": "a" },
{ "from": "input", "from_input": "c", "to": "mul_node", "to_port": "b" },
{ "from": "input", "from_input": "d", "to": "sub_node", "to_port": "b" },
{ "from": "mul_node", "from_port": "out", "to": "sub_node", "to_port": "a" }
  ],
  "scopes": []
}
"#,
  )
  .unwrap();

  let mut inputs = HashMap::new();
  inputs.insert("a".to_string(), serde_json::json!(1));
  inputs.insert("b".to_string(), serde_json::json!(2));
  inputs.insert("c".to_string(), serde_json::json!(3));
  inputs.insert("d".to_string(), serde_json::json!(4));

  let args = Args {
    bin_name: "pnix-executor-graph".into(),
    mode: ExecMode::Run,
    agent: None,
    gate_absorb: None,
    gate_absorb_subject: None,
    gate_absorb_follow_related: None,
    gate_absorb_limit: None,
    gate_absorb_reset: false,
    gate_forward: None,
    gate_read: None,
    gate_forward_limit: None,
    gate_forward_kind: None,
    gate_forward_reset: false,
    gate_forward_url: None,
    gate_read_context: None,
    gate_read_predicate: None,
    gate_read_topic: None,
    gate_read_event_types: vec![],
    gate_read_tool_name: None,
    gate_read_arg_predicates: vec![],
    gate_read_limit: None,
    gate_read_min_confidence: None,
    gate_read_kind: None,
    gate_read_path: None,
    gate_read_proof_path: None,
    gate_read_schema_path: None,
    gate_read_expected_bundle_kind: None,
    gate_read_expected_lobe_profile: None,
    gate_read_expected_proof_kind: None,
    agent_request: None,
    agent_target_paths: vec![],
    agent_project_pack_roots: vec![],
    agent_history_pack_roots: vec![],
    agent_approved_commands: vec![],
    agent_forbidden_paths: vec![],
    agent_policy_bits: vec![],
    agent_current_plan_ref: None,
    agent_rollback_handle_ref: None,
    agent_last_verification_ref: None,
    agent_promotion_boundary_ref: None,
    agent_source_apply_artifact_ref: None,
    agent_source_handoff_ref: None,
    agent_promotion_boundary_join_ref: None,
    agent_promotion_decision: None,
    agent_candidate_patch: None,
    agent_provider_feedback_request_ref: None,
    agent_request_out: None,
    agent_plan_out: None,
    agent_patch_out: None,
    agent_verify_out: None,
    agent_rollback_out: None,
    agent_decision_out: None,
    engine: Some("ir-eval".into()),
    result: Some("add_node".into()),
    dist: Some(dist),
    clojure_url: "http://localhost:7777".into(),
    python_url: "http://localhost:7778".into(),
    deno_url: "http://localhost:7779".into(),
    blenderpy_url: "http://localhost:7781".into(),
    supervisor_sock: None,
    backend_specs: None,
    auto_ensure_backends: true,
    replay_trace: None,
    replay_mode: None,
    replay_allow: vec![],
    invocation_id: None,
    rpc_timeout_ms: 30_000,
    rpc_retry_attempts: 3,
    rpc_retry_backoff_ms: 100,
    max_nodes: 10_000,
    max_edges: 50_000,
    max_input_bytes: 10 * 1024 * 1024,
    use_batch: true,
    source: None,
    expr: None,
    test_filter: None,
    patch: None,
    deterministic: true,
    strict_ct: true,
    inputs,
    seed: Some(0),
    now_ms: Some(0),
    clock_step_ms: Some(1),
    frp_dt: None,
    inputs_schema: false,
    list_modes: false,
    list_ir_eval_ops: false,
    version: false,
    dry_run: false,
    emit: false,
    emit_target: None,
    binary: false,
    emit_out: None,
    emit_manifest: None,
    fmt_check: false,
    live: false,
    live_dir: None,
    output_format: super::args::OutputFormat::Text,
  };

  let out = ir_eval_output_value(&args).expect("ir-eval output");
  assert_eq!(out.get("engine").and_then(|v| v.as_str()), Some("ir-eval"));
  assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
  assert_eq!(out.get("result").and_then(|v| v.as_str()), Some("add_node"));
  assert_eq!(out.get("value").and_then(|v| v.as_f64()), Some(3.0));
}

// =====================================================================
// CAX coding-agent middle-layer: server-friendly invocation lane.
//
// `build_coding_agent_request_with_probe` 는 caller 가 `cwd` 와
// `GitWorkspaceProbe` 를 inject 하므로 함수 본문 안에서
// `std::env::current_dir()` 도, `Command::new("git")` 도 호출하지 않는다.
// 이것이 doghouse-http (server) 가 pnix CLAUDE.md §16 (server subprocess
// 금지) 를 어기지 않고 typed `coding.request` artifact 를 emit 할 수
// 있는 lane 의 base 다.
// =====================================================================

#[test]
fn build_coding_agent_request_with_probe_carries_input_into_workspace_snapshot() {
  let tmp = temp_dir("coding-agent-with-probe");
  fs::create_dir_all(&tmp).expect("create temp dir");

  let input = super::args::CodingAgentRequestInput {
    agent_request: Some("이 코드를 봐줘".to_string()),
    agent_target_paths: vec![std::path::PathBuf::from("src/lib.rs")],
    agent_project_pack_roots: Vec::new(),
    agent_history_pack_roots: Vec::new(),
    agent_approved_commands: vec!["cargo check".to_string()],
    agent_forbidden_paths: vec![std::path::PathBuf::from("secrets/")],
    agent_policy_bits: vec!["candidate-only".to_string()],
    agent_current_plan_ref: Some("plan-ref-1".to_string()),
    agent_rollback_handle_ref: None,
    agent_last_verification_ref: None,
    agent_promotion_boundary_ref: None,
    agent_source_apply_artifact_ref: None,
    agent_source_handoff_ref: None,
    agent_promotion_boundary_join_ref: None,
    agent_promotion_decision: None,
  };

  // GitWorkspaceProbe::default() = repo_root None / branch None / dirty false.
  // server-side 호출에서 client 가 git context 를 envelope 으로 못 보낸 경우
  // (가장 단순한 envelope shape) 의 input 을 시뮬레이션.
  let probe = super::GitWorkspaceProbe::default();

  let artifact = super::build_coding_agent_request_with_probe(&input, AgentVerb::Ask, &tmp, &probe)
    .expect("build_coding_agent_request_with_probe");

  // typed artifact identity.
  assert_eq!(artifact.artifact_family, "coding.request");
  assert_eq!(artifact.phase, "CAX.2c-partial");
  assert_eq!(artifact.surface, "pnix coding-agent");
  assert_eq!(artifact.verb, "ask");
  assert_eq!(artifact.request.as_deref(), Some("이 코드를 봐줘"));

  // input → workspace parity (input 의 모든 carry 필드가 artifact 에 그대로 흘러야 한다).
  assert_eq!(
    artifact.workspace.target_paths,
    vec!["src/lib.rs".to_string()]
  );
  assert_eq!(
    artifact.workspace.approved_commands,
    vec!["cargo check".to_string()]
  );
  assert_eq!(
    artifact.workspace.forbidden_paths,
    vec!["secrets/".to_string()]
  );
  assert_eq!(
    artifact.workspace.policy_bits,
    vec!["candidate-only".to_string()]
  );
  assert_eq!(
    artifact.workspace.current_plan_ref.as_deref(),
    Some("plan-ref-1")
  );
  assert!(artifact.workspace.rollback_handle_ref.is_none());

  // probe → workspace parity. default probe 면 git 정보가 비어 있어야 한다
  // (server subprocess 0 의 직접 증거).
  assert!(artifact.workspace.repo_root.is_none());
  assert!(artifact.workspace.git_branch.is_none());
  assert!(artifact.workspace.git_head_commit.is_none());
  assert!(!artifact.workspace.git_dirty);
}

#[test]
fn build_coding_agent_request_with_probe_honors_injected_git_probe() {
  let tmp = temp_dir("coding-agent-with-injected-probe");
  fs::create_dir_all(&tmp).expect("create temp dir");

  let input = super::args::CodingAgentRequestInput {
    agent_request: Some("plan ahead".to_string()),
    ..Default::default()
  };

  // client 가 자기 git probe 결과를 envelope 으로 carry 한 시나리오.
  let probe = super::GitWorkspaceProbe {
    repo_root: Some("/client/repo/x".to_string()),
    branch: Some("feature/coding-agent-route".to_string()),
    head_commit: Some("0123abcd".to_string()),
    dirty: true,
  };

  let artifact =
    super::build_coding_agent_request_with_probe(&input, AgentVerb::Plan, &tmp, &probe)
      .expect("build with injected probe");

  assert_eq!(artifact.verb, "plan");
  assert_eq!(
    artifact.workspace.repo_root.as_deref(),
    Some("/client/repo/x")
  );
  assert_eq!(
    artifact.workspace.git_branch.as_deref(),
    Some("feature/coding-agent-route")
  );
  assert_eq!(
    artifact.workspace.git_head_commit.as_deref(),
    Some("0123abcd")
  );
  assert!(artifact.workspace.git_dirty);
}
