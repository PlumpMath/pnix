//! pnix CLI 구현 (pnix와 pnix-executor-graph에서 공유)
//!
//! FxCore Graph 적용기 - 의미 그래프를 백엔드 런타임에 적용
//!
//! 사용법:
//!   pnix-executor-graph --dist dist [--clojure-url URL] [--no-batch]
//!
//! 이것은 런타임이나 인터프리터가 아닙니다. "적용기(applicator)"로:
//! 1. dist/ir/fxcore.canon.json을 읽음
//! 2. 노드/엣지로부터 위상 정렬 실행 계획을 구성
//! 3. 백엔드 RPC를 순서대로 호출 (지원되는 경우 배치 처리)
//! 4. 결과를 dist/pnix.apply_graph.json에 기록
//!
//! 단계별 제한사항:
//! - Stage-1: 배치 적용 허용 (단순 DAG)
//! - Stage-2+: 배치 적용 비활성화 (입력/출력 맵)
//! - Stage-3+: 순차 실행만 (조건부 엣지, 게이트, 선택적 노드)
//! - Stage-4: try/catch 포함 (onfail 엣지) 및 스코프

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::model::{self, SUPPORTED_FXCORE_VERSIONS};
use crate::{
  apply, backend_catalog, emit_fs, eval_patch, frp_patch, module_loader, output, patch, plan,
  project,
};
use anyhow::{bail, Context, Result};
// ============================================================================
// DOGHOUSE-DEPRECATION (2026-06-01)
// 아래 `#[cfg(feature = "doghouse")]` 로 게이트된 doghouse_core 기반 경로
// (DoghouseStore coding-memory artifact persist/replay, docset evidence join,
// gate_read brain-ankh 입력 등 — puck/doghouse 와 연결돼 있던 store surface)
// 는 전부 **pnixc-meta substrate 로 대체될 예정**이다. doghouse 패키지/빌드는
// 이미 제거됐고 (default feature OFF), 정본 store/의미 owner 는 pnixc-meta 다.
// 참고: CLAUDE.md 2026-05-31 SUPERSEDE ("relay boundary 의 모든 semantic logic
// 은 pnixc-meta substrate 로 이동") + "pnixc-meta = canonical interpreter".
// → 이 doghouse 게이트 코드는 pnixc-meta store 경로가 박히면 통째로 교체된다.
// ============================================================================
#[cfg(feature = "doghouse")]
use doghouse_core::docset_query::{query_joined_docset_evidence, DocsetJoinedEvidence};
#[cfg(feature = "doghouse")]
use doghouse_core::store::{CodingMemoryArtifact, DoghouseStore, DoghouseStoreConfig};

// Non-doghouse fallback: doghouse-core is not linked, so provide a local
// placeholder for the joined-docset-evidence shape and a stub query that
// always returns an empty result. Mirrors the placeholder pattern in
// `pnix-runtime-legacy/src/ir/eval.rs`. The serialized shape matches the
// real `doghouse_core::docset_query::DocsetJoinedEvidence`.
#[cfg(not(feature = "doghouse"))]
#[derive(Debug, Clone, serde::Serialize)]
struct DocsetJoinedEvidence {
  manual_ref: String,
  manual_name: String,
  term: String,
  matched_symbol: Option<String>,
  matched_file: Option<String>,
  project_refs: Vec<String>,
  provenance_ref: String,
  join_status: &'static str,
  description_excerpt: String,
}

#[cfg(not(feature = "doghouse"))]
fn query_joined_docset_evidence(
  _language: Option<&str>,
  _term: &str,
  _symbol: Option<&str>,
  _file: Option<&str>,
  _project_refs: &[String],
) -> Vec<DocsetJoinedEvidence> {
  Vec::new()
}
use draw_ir::{Color, DrawCommand, DrawIR2D, FontSpec, Paint, Rect, TextRun, Transform2D};
use freecat_runtime_ui::{frame_from_json_value, FramePacket, UiError};
use pnix_core::contracts::{verify_input_size, verify_resource_limits, ResourceLimits};
use pnix_live::{
  default_live_dir, LiveMode, LivePaths, LiveUpdate, LIVE_SCHEMA_VERSION, LIVE_VERSION,
};
use pnix_lsp::{CpgBuilder, LspSetoIndex, RepoProjectGraphInput, TreeSitterManager};
use pnix_runtime_api::{
  CtConfig, CtRuntime, EvalConfig, EvalEngine, FrpConfig, FrpEngine, RuntimeCapability,
};
use pnix_runtime_ct::CtRuntimeEngine;
use pnix_runtime_legacy::ir::{IrEvalContext, LegacyInstr, LegacyIr, LegacyOp};
use pnix_runtime_legacy::ssa::{
  run_ssa_value, SSABlock as LegacySsaBlock, SSAOp as LegacySsaOp, SSARunContext, SsaValue,
};
use pnix_runtime_legacy::{
  frp_graph_json_from_xml_json, frp_graph_json_from_xml_str, validate_attrs_from_xml_json,
  validate_attrs_from_xml_str, validate_routes_from_xml_json, validate_routes_from_xml_str,
  x3d_schema_explain_xml_json, x3d_schema_normalize_xml_json, LegacyEvalEngine, LegacyFrpEngine,
  LegacyFrpGraph, LegacyFrpInput, LegacyModule,
};
use pnix_runtime_llvm::{AotArtifactManifest, AotConfig, AotEngine, AotTarget, JitEngine};
use tempfile::Builder;

use pnix_backend_legacy::{generate_from_ir_with_config, CodegenConfig, CodegenTarget};
use pnix_core::core::FxCoreModule as CoreFxCoreModule;
use pnix_core::lang::pnix::parse_expr;
use pnix_core::lang::pnix::ui_json::{normalize_pnix_list_separators, pnix_expr_to_json};
use pnix_core::ssa::SsaModule as CoreSsaModule;
use pnix_core::{
  compile_pnix_module, compile_pnix_module_ast, CompileOptions as CoreCompileOptions,
  SourceUnit as CoreSourceUnit,
};
use pnix_ir_adapter::{fxcore_to_legacy_ir, AdapterConfig};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;
use pnix_hash::{Digest, Sha256};

const CODING_AGENT_VERIFY_COMMAND_TIMEOUT_MS: u64 = 30_000;
const CODING_AGENT_COMMAND_OUTPUT_PREVIEW_BYTES: usize = 4096;

use crate::apply::BackendConfig;
use crate::backend_supervisor::BackendSupervisor;
use crate::bootstrap_seto;
use crate::capability_check;
use crate::error::ExecutionResult;
use crate::replay::{ReplayConfig, ReplayDB, ReplayMode};

mod args;
mod gate_absorb;
mod gate_forward;
mod gate_read;
mod legacy_compat;
mod ops;
mod print;
pub(crate) mod repl;
mod test_runner;
use self::args::{
  parse_args_vec, AgentVerb, Args, CodingAgentRequestInput, ExecMode, OutputFormat,
};
use self::gate_absorb::run_gate_absorb;
use self::gate_forward::run_gate_forward;
use self::gate_read::run_gate_read;
use self::ops::{is_ops_invocation, run_ops_invocation};
use self::print::{mode_label, print_inputs_schema, print_ir_eval_ops, print_modes, print_version};
use self::repl::run_repl;
use self::test_runner::{
  collect_tests_from_source, print_test_summary, run_tests, GraphTestConfig,
};

/// CLI 실행: 인자 파싱 및 명령 실행
pub async fn run_cli(argv: Vec<String>) -> ExecutionResult<()> {
  if is_ops_invocation(&argv) {
    run_ops_invocation(&argv)?;
    return Ok(());
  }

  if legacy_compat::is_serve_invocation(&argv) {
    legacy_compat::run_serve_compat(&argv)?;
    return Ok(());
  }
  if legacy_compat::should_use_eval_compat(&argv) {
    legacy_compat::run_eval_compat(&argv)?;
    return Ok(());
  }

  let args = parse_args_vec(argv)?;
  if let Ok(raw) = std::env::var("PNIX_PNIX_PARSE_DEPTH_LIMIT") {
    if let Ok(limit) = raw.parse::<usize>() {
      pnix_core::lang::pnix::parser::set_parse_recursion_depth_limit(limit);
    }
  }

  if args.version {
    print_version(args.bin_name.as_str());
    return Ok(());
  }

  if args.list_modes {
    print_modes();
    return Ok(());
  }

  if args.list_ir_eval_ops {
    print_ir_eval_ops()?;
    return Ok(());
  }

  if let Some(gate_absorb) = args.gate_absorb.as_ref() {
    std::process::exit(run_gate_absorb(&args, gate_absorb)?);
  }

  if let Some(gate_forward) = args.gate_forward.as_ref() {
    std::process::exit(run_gate_forward(&args, gate_forward)?);
  }

  if let Some(gate_read) = args.gate_read.as_ref() {
    std::process::exit(run_gate_read(&args, gate_read)?);
  }

  if let Some(agent) = args.agent {
    run_agent_stub(&args, agent)?;
    return Ok(());
  }

  if args.inputs_schema {
    print_inputs_schema()?;
    return Ok(());
  }

  if args.emit {
    run_emit(&args)?;
    return Ok(());
  }

  // SETO bootstrap is now a no-op (ego-sphere SetoRegistry removed)
  let _ = bootstrap_seto()?;

  // LOW: ANSI 색상 비터미널에서 출력 수정 완료
  // 현재는 eprintln!만 사용하므로 색상 코드가 없어 문제 없음
  // 향후 ANSI 색상 코드 사용 시 isatty 체크를 통해 터미널인지 확인하여 사용
  let _is_terminal = io::stderr().is_terminal();

  eprintln!(
    "info: mode={}, deterministic={}, seed={:?}, now_ms={:?}, clock_step_ms={:?}",
    mode_label(args.mode),
    args.deterministic,
    args.seed,
    args.now_ms,
    args.clock_step_ms
  );

  match args.mode {
    ExecMode::Run => {
      run_run(&args).await?;
      Ok(())
    }
    ExecMode::Interpret => {
      run_interpret(&args).await?;
      Ok(())
    }
    ExecMode::Compile => {
      run_compile(&args)?;
      Ok(())
    }
    ExecMode::Graph => {
      run_graph(&args).await?;
      Ok(())
    }
    ExecMode::LegacyEval => {
      warn_legacy_compat_path(
        "--mode legacy-eval",
        "--mode interpret or --mode run --engine ir-eval",
      );
      run_legacy_eval(&args)?;
      Ok(())
    }
    ExecMode::LegacyFrp => {
      warn_legacy_compat_path("--mode legacy-frp", "--mode interpret --engine legacy-frp");
      run_legacy_frp(&args)?;
      Ok(())
    }
    ExecMode::Ct => {
      run_ct(&args)?;
      Ok(())
    }
    ExecMode::Llvm => {
      run_llvm(&args)?;
      Ok(())
    }
    ExecMode::Test => {
      run_test(&args)?;
      Ok(())
    }
    ExecMode::Fmt => {
      run_fmt(&args)?;
      Ok(())
    }
    ExecMode::Lint => {
      run_lint(&args)?;
      Ok(())
    }
  }
}

fn run_agent_stub(args: &Args, verb: AgentVerb) -> Result<()> {
  if verb == AgentVerb::Retention {
    return run_coding_agent_retention(args);
  }

  let request = build_coding_agent_request(args, verb)?;
  if let Some(path) = args.agent_request_out.as_ref() {
    write_json_artifact(path, &request, "coding-agent request")?;
  }
  persist_coding_memory_artifact_if_configured(
    "coding.request",
    Some(make_repo_snapshot_ref(&request.workspace)),
    request.workspace.target_paths.clone(),
    request.workspace.approved_commands.clone(),
    build_coding_memory_related_refs(
      args.agent_request_out.as_ref(),
      [
        request.workspace.current_plan_ref.as_deref(),
        request.workspace.rollback_handle_ref.as_deref(),
        request.workspace.last_verification_ref.as_deref(),
        request.workspace.promotion_boundary_ref.as_deref(),
        request.workspace.promotion_boundary_join_ref.as_deref(),
      ],
    ),
    &request,
  )?;
  persist_coding_memory_artifact_if_configured(
    request.language_profile.artifact_family,
    Some(make_repo_snapshot_ref(&request.workspace)),
    request.workspace.target_paths.clone(),
    Vec::new(),
    vec![request.context_pack.context_pack_ref.clone()],
    &request.language_profile,
  )?;

  match verb {
    AgentVerb::Plan => {
      let request_artifact_ref = args
        .agent_request_out
        .as_ref()
        .map(|path| path_to_slash(path));
      let plan = build_coding_agent_plan(args, request, request_artifact_ref);
      if let Some(path) = args.agent_plan_out.as_ref() {
        write_json_artifact(path, &plan, "coding-agent plan")?;
      }
      persist_coding_memory_artifact_if_configured(
        plan.artifact_family,
        Some(make_repo_snapshot_ref(&plan.request.workspace)),
        plan.request.workspace.target_paths.clone(),
        plan.request.workspace.approved_commands.clone(),
        build_coding_memory_related_refs(
          args.agent_plan_out.as_ref(),
          [
            plan.request_artifact_ref.as_deref(),
            plan.request.workspace.current_plan_ref.as_deref(),
            plan.request.workspace.last_verification_ref.as_deref(),
          ],
        ),
        &plan,
      )?;

      match args.output_format {
        OutputFormat::Json => {
          println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::to_value(&plan)?)?
          );
        }
        OutputFormat::Text => {
          println!("진행상태: {}", plan.status.progress_status);
          println!("현재해석: {}", plan.current_interpretation);
          print_grounding_seed_text(&plan.request.grounding_seed);
          print_repo_graph_seed_text(&plan.request.repo_graph_seed);
          print_manual_evidence_seed_text(&plan.request.manual_evidence_seed);
          print_attached_pack_seed_text(&plan.request.attached_pack_seed);
          print_language_profile_text(&plan.request.language_profile);
          println!("실행 계획:");
          for step in &plan.bounded_step_family {
            println!(
              "  - {} [{}]: {}",
              step.step_family, step.capability_bound, step.summary
            );
          }
          if !plan.request.workspace.target_paths.is_empty() {
            println!(
              "적용범위: {}",
              plan.request.workspace.target_paths.join(", ")
            );
          } else {
            println!("적용범위: repo-local bounded scope");
          }
          println!("검증:");
          for target in &plan.expected_verification {
            println!("  - {}", target);
          }
          println!("결과상태: {}", plan.status.result_status);
          println!("failure-policy: {}", plan.failure_policy);
          if let Some(path) = args.agent_request_out.as_ref() {
            println!("request-artifact-out: {}", path.display());
          }
          if let Some(path) = args.agent_plan_out.as_ref() {
            println!("plan-artifact-out: {}", path.display());
          }
          println!("비고: {}", plan.status.note);
        }
      }
      Ok(())
    }
    AgentVerb::Decide => {
      let request_artifact_ref = args
        .agent_request_out
        .as_ref()
        .map(|path| path_to_slash(path));
      let decision =
        build_coding_agent_human_promotion_decision(args, request, request_artifact_ref)?;
      if let Some(path) = args.agent_decision_out.as_ref() {
        write_json_artifact(path, &decision, "coding-agent human promotion decision")?;
      }
      persist_coding_memory_artifact_if_configured(
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
            decision.request.workspace.current_plan_ref.as_deref(),
            decision.request.workspace.last_verification_ref.as_deref(),
          ],
        ),
        &decision,
      )?;

      match args.output_format {
        OutputFormat::Json => {
          println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::to_value(&decision)?)?
          );
        }
        OutputFormat::Text => {
          println!("진행상태: {}", decision.status.progress_status);
          println!("결과상태: {}", decision.status.result_status);
          println!("artifact-family: {}", decision.artifact_family);
          println!("decision-ref: {}", decision.decision_ref);
          println!("human-decision: {}", decision.human_decision);
          println!(
            "promotion-boundary-join-ref: {}",
            decision.source_promotion_boundary_join_ref
          );
          if !decision.target_paths.is_empty() {
            println!("target-paths: {}", decision.target_paths.join(", "));
          }
          if let Some(path) = args.agent_decision_out.as_ref() {
            println!("decision-artifact-out: {}", path.display());
          }
          println!("비고: {}", decision.status.note);
        }
      }
      Ok(())
    }
    AgentVerb::Patch => {
      let request_artifact_ref = args
        .agent_request_out
        .as_ref()
        .map(|path| path_to_slash(path));
      let patch_proposal = build_coding_agent_patch_proposal(args, request, request_artifact_ref);
      if let Some(path) = args.agent_patch_out.as_ref() {
        write_json_artifact(path, &patch_proposal, "coding-agent patch proposal")?;
      }
      persist_coding_memory_artifact_if_configured(
        patch_proposal.artifact_family,
        Some(make_repo_snapshot_ref(&patch_proposal.request.workspace)),
        patch_proposal.target_paths.clone(),
        patch_proposal.request.workspace.approved_commands.clone(),
        build_coding_memory_related_refs(
          args.agent_patch_out.as_ref(),
          [
            patch_proposal.request_artifact_ref.as_deref(),
            patch_proposal.current_plan_ref.as_deref(),
            Some(patch_proposal.diff_ref.as_str()),
          ],
        ),
        &patch_proposal,
      )?;
      let mut replay_related_refs = build_coding_memory_related_refs(
        Option::<&PathBuf>::None,
        [
          patch_proposal.request_artifact_ref.as_deref(),
          patch_proposal.current_plan_ref.as_deref(),
          patch_proposal
            .request
            .workspace
            .last_verification_ref
            .as_deref(),
          Some(patch_proposal.context_demand_replay.replay_ref.as_str()),
        ],
      );
      replay_related_refs.extend(
        patch_proposal
          .context_demand_replay
          .source_artifact_refs
          .iter()
          .cloned(),
      );
      replay_related_refs.sort();
      replay_related_refs.dedup();
      persist_coding_memory_artifact_if_configured(
        patch_proposal.context_demand_replay.artifact_family,
        Some(make_repo_snapshot_ref(&patch_proposal.request.workspace)),
        patch_proposal.target_paths.clone(),
        patch_proposal.request.workspace.approved_commands.clone(),
        replay_related_refs,
        &patch_proposal.context_demand_replay,
      )?;
      let mut repair_related_refs = build_coding_memory_related_refs(
        Option::<&PathBuf>::None,
        [
          patch_proposal.request_artifact_ref.as_deref(),
          patch_proposal.current_plan_ref.as_deref(),
          patch_proposal
            .request
            .workspace
            .last_verification_ref
            .as_deref(),
          Some(patch_proposal.repair_recipe_replay.replay_ref.as_str()),
        ],
      );
      repair_related_refs.extend(
        patch_proposal
          .repair_recipe_replay
          .source_artifact_refs
          .iter()
          .cloned(),
      );
      repair_related_refs.sort();
      repair_related_refs.dedup();
      persist_coding_memory_artifact_if_configured(
        patch_proposal.repair_recipe_replay.artifact_family,
        Some(make_repo_snapshot_ref(&patch_proposal.request.workspace)),
        patch_proposal.target_paths.clone(),
        patch_proposal.request.workspace.approved_commands.clone(),
        repair_related_refs,
        &patch_proposal.repair_recipe_replay,
      )?;
      if let Some(candidate) = patch_proposal.generated_patch_candidate.as_ref() {
        persist_coding_memory_artifact_if_configured(
          candidate.artifact_family,
          Some(make_repo_snapshot_ref(&patch_proposal.request.workspace)),
          candidate.target_paths.clone(),
          patch_proposal.request.workspace.approved_commands.clone(),
          build_coding_memory_related_refs(
            Option::<&PathBuf>::None,
            [
              patch_proposal.request_artifact_ref.as_deref(),
              patch_proposal.current_plan_ref.as_deref(),
              Some(patch_proposal.diff_ref.as_str()),
              Some(candidate.candidate_ref.as_str()),
              Some(candidate.patch_input_ref.as_str()),
              candidate.source_provider_feedback_request_ref.as_deref(),
            ],
          ),
          candidate,
        )?;
      }
      if let Some(review_receipt) = patch_proposal.generated_patch_review_receipt.as_ref() {
        persist_coding_memory_artifact_if_configured(
          review_receipt.artifact_family,
          Some(make_repo_snapshot_ref(&patch_proposal.request.workspace)),
          review_receipt.target_paths.clone(),
          patch_proposal.request.workspace.approved_commands.clone(),
          build_coding_memory_related_refs(
            Option::<&PathBuf>::None,
            [
              patch_proposal.request_artifact_ref.as_deref(),
              patch_proposal.current_plan_ref.as_deref(),
              Some(patch_proposal.diff_ref.as_str()),
              Some(review_receipt.candidate_ref.as_str()),
              Some(review_receipt.review_ref.as_str()),
            ],
          ),
          review_receipt,
        )?;
      }
      if let Some(feedback_request) = patch_proposal.provider_feedback_request.as_ref() {
        persist_coding_memory_artifact_if_configured(
          feedback_request.artifact_family,
          Some(make_repo_snapshot_ref(&patch_proposal.request.workspace)),
          feedback_request.target_paths.clone(),
          patch_proposal.request.workspace.approved_commands.clone(),
          build_coding_memory_related_refs(
            Option::<&PathBuf>::None,
            [
              patch_proposal.request_artifact_ref.as_deref(),
              patch_proposal.current_plan_ref.as_deref(),
              Some(patch_proposal.diff_ref.as_str()),
              Some(feedback_request.source_candidate_ref.as_str()),
              Some(feedback_request.source_review_ref.as_str()),
              Some(feedback_request.request_ref.as_str()),
            ],
          ),
          feedback_request,
        )?;
      }
      if let Some(retry_guard) = patch_proposal.feedback_retry_guard.as_ref() {
        persist_coding_memory_artifact_if_configured(
          retry_guard.artifact_family,
          Some(make_repo_snapshot_ref(&patch_proposal.request.workspace)),
          patch_proposal.target_paths.clone(),
          patch_proposal.request.workspace.approved_commands.clone(),
          build_coding_memory_related_refs(
            Option::<&PathBuf>::None,
            [
              patch_proposal.request_artifact_ref.as_deref(),
              patch_proposal.current_plan_ref.as_deref(),
              Some(patch_proposal.diff_ref.as_str()),
              Some(retry_guard.source_candidate_ref.as_str()),
              Some(retry_guard.source_review_ref.as_str()),
              Some(retry_guard.source_provider_feedback_request_ref.as_str()),
              Some(retry_guard.guard_ref.as_str()),
            ],
          ),
          retry_guard,
        )?;
      }
      if let Some(handoff_proof) = patch_proposal.apply_handoff_proof.as_ref() {
        persist_coding_memory_artifact_if_configured(
          handoff_proof.artifact_family,
          Some(make_repo_snapshot_ref(&patch_proposal.request.workspace)),
          handoff_proof.target_paths.clone(),
          patch_proposal.request.workspace.approved_commands.clone(),
          build_coding_memory_related_refs(
            Option::<&PathBuf>::None,
            [
              patch_proposal.request_artifact_ref.as_deref(),
              patch_proposal.current_plan_ref.as_deref(),
              Some(patch_proposal.diff_ref.as_str()),
              Some(handoff_proof.candidate_ref.as_str()),
              Some(handoff_proof.candidate_review_ref.as_str()),
              Some(handoff_proof.handoff_ref.as_str()),
              patch_proposal
                .apply_result
                .as_ref()
                .map(|apply_result| apply_result.apply_artifact_ref.as_str()),
            ],
          ),
          handoff_proof,
        )?;
      }
      if let Some(receipt) = patch_proposal.promotion_boundary_receipt.as_ref() {
        persist_coding_memory_artifact_if_configured(
          receipt.artifact_family,
          Some(make_repo_snapshot_ref(&patch_proposal.request.workspace)),
          patch_proposal.target_paths.clone(),
          patch_proposal.request.workspace.approved_commands.clone(),
          build_coding_memory_related_refs(
            Option::<&PathBuf>::None,
            [
              patch_proposal.request_artifact_ref.as_deref(),
              patch_proposal.current_plan_ref.as_deref(),
              Some(patch_proposal.diff_ref.as_str()),
              Some(receipt.source_apply_artifact_ref.as_str()),
              receipt.source_handoff_ref.as_deref(),
              Some(receipt.receipt_ref.as_str()),
            ],
          ),
          receipt,
        )?;
      }
      persist_coding_memory_artifact_if_configured(
        patch_proposal.semantic_review.artifact_family,
        Some(make_repo_snapshot_ref(&patch_proposal.request.workspace)),
        patch_proposal.semantic_review.target_paths.clone(),
        patch_proposal.request.workspace.approved_commands.clone(),
        build_coding_memory_related_refs(
          Option::<&PathBuf>::None,
          [
            patch_proposal.request_artifact_ref.as_deref(),
            patch_proposal.current_plan_ref.as_deref(),
            Some(patch_proposal.diff_ref.as_str()),
            patch_proposal.semantic_review.apply_artifact_ref.as_deref(),
            Some(
              patch_proposal
                .semantic_review
                .meaning_impact_diff
                .impact_ref
                .as_str(),
            ),
            Some(
              patch_proposal
                .semantic_review
                .patch_decision_link
                .link_ref
                .as_str(),
            ),
          ],
        ),
        &patch_proposal.semantic_review,
      )?;
      if let Some(apply_result) = patch_proposal.apply_result.as_ref() {
        persist_coding_memory_artifact_if_configured(
          apply_result.artifact_family,
          Some(make_repo_snapshot_ref(&patch_proposal.request.workspace)),
          apply_result.target_paths.clone(),
          patch_proposal.request.workspace.approved_commands.clone(),
          build_coding_memory_related_refs(
            Option::<&PathBuf>::None,
            [
              patch_proposal.request_artifact_ref.as_deref(),
              Some(patch_proposal.diff_ref.as_str()),
              apply_result.rollback_handle_ref.as_deref(),
              apply_result.inverse_plan_ref.as_deref(),
            ],
          ),
          apply_result,
        )?;
      }

      match args.output_format {
        OutputFormat::Json => {
          println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::to_value(&patch_proposal)?)?
          );
        }
        OutputFormat::Text => {
          println!("진행상태: {}", patch_proposal.status.progress_status);
          println!("결과상태: {}", patch_proposal.status.result_status);
          println!("coding-agent verb: {}", verb.as_str());
          println!("artifact-family: {}", patch_proposal.artifact_family);
          println!("현재해석: {}", patch_proposal.current_interpretation);
          println!(
            "edit-family: {} | risk-class: {}",
            patch_proposal.edit_family, patch_proposal.risk_class
          );
          println!("diff-ref: {}", patch_proposal.diff_ref);
          if !patch_proposal.target_paths.is_empty() {
            println!("target-paths: {}", patch_proposal.target_paths.join(", "));
          }
          if !patch_proposal.expected_verify_ref.is_empty() {
            println!(
              "expected-verify-ref: {}",
              patch_proposal.expected_verify_ref.join(", ")
            );
          }
          if !patch_proposal.effect_classes.is_empty() {
            println!(
              "effect-classes: {}",
              patch_proposal.effect_classes.join(", ")
            );
          }
          println!(
            "apply-intent: {} ({})",
            patch_proposal.apply_intent.intent_family, patch_proposal.apply_intent.apply_status
          );
          println!(
            "semantic-review-ref: {} ({})",
            patch_proposal.semantic_review.review_ref, patch_proposal.semantic_review.review_status
          );
          println!(
            "context-demand-replay: {} ({})",
            patch_proposal.context_demand_replay.replay_ref,
            patch_proposal.context_demand_replay.replay_status
          );
          println!(
            "repair-recipe-replay: {} ({})",
            patch_proposal.repair_recipe_replay.replay_ref,
            patch_proposal.repair_recipe_replay.replay_status
          );
          if let Some(candidate) = patch_proposal.generated_patch_candidate.as_ref() {
            println!(
              "generated-patch-candidate: {} ({}, {})",
              candidate.candidate_ref, candidate.quarantine_status, candidate.lineage_status
            );
          }
          if let Some(review_receipt) = patch_proposal.generated_patch_review_receipt.as_ref() {
            println!(
              "generated-patch-review-receipt: {} ({})",
              review_receipt.review_ref, review_receipt.review_status
            );
          }
          if let Some(feedback_request) = patch_proposal.provider_feedback_request.as_ref() {
            println!(
              "provider-feedback-request: {} ({})",
              feedback_request.request_ref, feedback_request.request_status
            );
          }
          if let Some(retry_guard) = patch_proposal.feedback_retry_guard.as_ref() {
            println!(
              "feedback-retry-guard: {} ({})",
              retry_guard.guard_ref, retry_guard.guard_status
            );
          }
          if let Some(handoff_proof) = patch_proposal.apply_handoff_proof.as_ref() {
            println!(
              "apply-handoff-proof: {} ({})",
              handoff_proof.handoff_ref, handoff_proof.handoff_status
            );
          }
          if let Some(receipt) = patch_proposal.promotion_boundary_receipt.as_ref() {
            println!(
              "promotion-boundary-receipt: {} ({})",
              receipt.receipt_ref, receipt.promotion_status
            );
          }
          if let Some(apply_result) = patch_proposal.apply_result.as_ref() {
            println!("apply-result: {}", apply_result.apply_status);
            println!("apply-artifact-ref: {}", apply_result.apply_artifact_ref);
            if let Some(rollback_handle_ref) = apply_result.rollback_handle_ref.as_deref() {
              println!("rollback-handle-ref: {}", rollback_handle_ref);
            }
            if !apply_result.applied_paths.is_empty() {
              println!("applied-paths: {}", apply_result.applied_paths.join(", "));
            }
            if !apply_result.rejected_paths.is_empty() {
              println!("rejected-paths: {}", apply_result.rejected_paths.join(", "));
            }
          }
          print_grounding_seed_text(&patch_proposal.request.grounding_seed);
          print_repo_graph_seed_text(&patch_proposal.request.repo_graph_seed);
          print_manual_evidence_seed_text(&patch_proposal.request.manual_evidence_seed);
          print_attached_pack_seed_text(&patch_proposal.request.attached_pack_seed);
          print_language_profile_text(&patch_proposal.request.language_profile);
          if let Some(path) = args.agent_request_out.as_ref() {
            println!("request-artifact-out: {}", path.display());
          }
          if let Some(path) = args.agent_patch_out.as_ref() {
            println!("patch-artifact-out: {}", path.display());
          }
          println!("비고: {}", patch_proposal.status.note);
        }
      }
      Ok(())
    }
    AgentVerb::Verify => {
      let request_artifact_ref = args
        .agent_request_out
        .as_ref()
        .map(|path| path_to_slash(path));
      let mut verify_receipt =
        build_coding_agent_verify_receipt(args, request, request_artifact_ref);
      if !args.dry_run {
        let execution_result = run_coding_agent_verify_commands(
          &verify_receipt.request,
          &verify_receipt.repo_snapshot_ref,
          &verify_receipt.diff_ref,
          &verify_receipt.target_commands,
        );
        attach_coding_agent_verify_execution_result(&mut verify_receipt, execution_result);
      }
      attach_coding_agent_promotion_boundary_join_receipt(&mut verify_receipt);
      if let Some(path) = args.agent_verify_out.as_ref() {
        write_json_artifact(path, &verify_receipt, "coding-agent verify receipt")?;
      }
      persist_coding_memory_artifact_if_configured(
        verify_receipt.execution_result.artifact_family,
        Some(verify_receipt.repo_snapshot_ref.clone()),
        verify_receipt.target_paths.clone(),
        verify_receipt.target_commands.clone(),
        build_coding_memory_related_refs(
          Option::<&PathBuf>::None,
          [
            verify_receipt.request_artifact_ref.as_deref(),
            Some(
              verify_receipt
                .execution_result
                .execution_result_ref
                .as_str(),
            ),
            Some(verify_receipt.diff_ref.as_str()),
          ],
        ),
        &verify_receipt.execution_result,
      )?;
      if let Some(join_receipt) = verify_receipt.promotion_boundary_join_receipt.as_ref() {
        persist_coding_memory_artifact_if_configured(
          join_receipt.artifact_family,
          Some(verify_receipt.repo_snapshot_ref.clone()),
          verify_receipt.target_paths.clone(),
          verify_receipt.target_commands.clone(),
          build_coding_memory_related_refs(
            args.agent_verify_out.as_ref(),
            [
              verify_receipt.request_artifact_ref.as_deref(),
              Some(join_receipt.source_promotion_boundary_receipt_ref.as_str()),
              Some(join_receipt.source_apply_artifact_ref.as_str()),
              join_receipt.source_handoff_ref.as_deref(),
              Some(join_receipt.verify_diff_ref.as_str()),
              Some(join_receipt.verify_execution_result_ref.as_str()),
              Some(join_receipt.join_ref.as_str()),
            ],
          ),
          join_receipt,
        )?;
      }
      persist_coding_memory_artifact_if_configured(
        verify_receipt.artifact_family,
        Some(verify_receipt.repo_snapshot_ref.clone()),
        verify_receipt.target_paths.clone(),
        verify_receipt.target_commands.clone(),
        build_coding_memory_related_refs(
          args.agent_verify_out.as_ref(),
          [
            verify_receipt.request_artifact_ref.as_deref(),
            Some(verify_receipt.before_artifact_ref.as_str()),
            Some(verify_receipt.after_artifact_ref.as_str()),
            Some(verify_receipt.diff_ref.as_str()),
          ],
        ),
        &verify_receipt,
      )?;
      persist_coding_memory_artifact_if_configured(
        verify_receipt.learning_card.artifact_family,
        Some(verify_receipt.repo_snapshot_ref.clone()),
        verify_receipt.target_paths.clone(),
        verify_receipt.target_commands.clone(),
        build_coding_memory_related_refs(
          Option::<&PathBuf>::None,
          [
            verify_receipt.request_artifact_ref.as_deref(),
            Some(verify_receipt.diff_ref.as_str()),
            Some(verify_receipt.learning_card.learning_card_ref.as_str()),
            Some(
              verify_receipt
                .execution_result
                .execution_result_ref
                .as_str(),
            ),
          ],
        ),
        &verify_receipt.learning_card,
      )?;

      match args.output_format {
        OutputFormat::Json => {
          println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::to_value(&verify_receipt)?)?
          );
        }
        OutputFormat::Text => {
          println!("진행상태: {}", verify_receipt.status.progress_status);
          println!("결과상태: {}", verify_receipt.status.result_status);
          println!("coding-agent verb: {}", verb.as_str());
          println!("artifact-family: {}", verify_receipt.artifact_family);
          println!("repo-snapshot-ref: {}", verify_receipt.repo_snapshot_ref);
          println!(
            "before-artifact-ref: {}",
            verify_receipt.before_artifact_ref
          );
          println!("after-artifact-ref: {}", verify_receipt.after_artifact_ref);
          println!("diff-ref: {}", verify_receipt.diff_ref);
          println!(
            "execution-result-ref: {}",
            verify_receipt.execution_result.execution_result_ref
          );
          println!(
            "execution-status: {}",
            verify_receipt.execution_result.execution_status
          );
          if !verify_receipt.target_paths.is_empty() {
            println!("target-paths: {}", verify_receipt.target_paths.join(", "));
          }
          if !verify_receipt.target_commands.is_empty() {
            println!(
              "target-commands: {}",
              verify_receipt.target_commands.join(", ")
            );
          }
          if !verify_receipt.execution_result.command_results.is_empty() {
            println!("command-results:");
            for command in verify_receipt
              .execution_result
              .command_results
              .iter()
              .take(6)
            {
              println!(
                "  - {} => {} exit={:?} duration_ms={}",
                command.command_ref, command.status, command.exit_code, command.duration_ms
              );
            }
          }
          if let Some(join_receipt) = verify_receipt.promotion_boundary_join_receipt.as_ref() {
            println!(
              "promotion-boundary-join-receipt: {} ({})",
              join_receipt.join_ref, join_receipt.join_status
            );
          }
          print_grounding_seed_text(&verify_receipt.request.grounding_seed);
          print_repo_graph_seed_text(&verify_receipt.request.repo_graph_seed);
          print_manual_evidence_seed_text(&verify_receipt.request.manual_evidence_seed);
          print_attached_pack_seed_text(&verify_receipt.request.attached_pack_seed);
          print_language_profile_text(&verify_receipt.request.language_profile);
          if !verify_receipt.proof_refs.is_empty() {
            println!("proof-refs:");
            for proof in verify_receipt.proof_refs.iter().take(6) {
              println!("  - {}", proof);
            }
          }
          if let Some(path) = args.agent_request_out.as_ref() {
            println!("request-artifact-out: {}", path.display());
          }
          if let Some(path) = args.agent_verify_out.as_ref() {
            println!("verify-artifact-out: {}", path.display());
          }
          println!("비고: {}", verify_receipt.status.note);
        }
      }
      Ok(())
    }
    AgentVerb::Rollback => {
      let request_artifact_ref = args
        .agent_request_out
        .as_ref()
        .map(|path| path_to_slash(path));
      if args.agent_rollback_handle_ref.is_some() {
        let rollback_receipt =
          build_coding_agent_rollback_receipt(args, request, request_artifact_ref);
        if let Some(path) = args.agent_rollback_out.as_ref() {
          write_json_artifact(path, &rollback_receipt, "coding-agent rollback receipt")?;
        }
        persist_coding_memory_artifact_if_configured(
          rollback_receipt.artifact_family,
          Some(rollback_receipt.repo_snapshot_ref.clone()),
          rollback_receipt.request.workspace.target_paths.clone(),
          rollback_receipt.request.workspace.approved_commands.clone(),
          build_coding_memory_related_refs(
            args.agent_rollback_out.as_ref(),
            [
              rollback_receipt.request_artifact_ref.as_deref(),
              Some(rollback_receipt.handle_ref.as_str()),
              Some(rollback_receipt.apply_artifact_ref.as_str()),
              rollback_receipt.inverse_plan_ref.as_deref(),
              rollback_receipt.restored_snapshot_ref.as_deref(),
              rollback_receipt.followup_verify_ref.as_deref(),
            ],
          ),
          &rollback_receipt,
        )?;

        match args.output_format {
          OutputFormat::Json => {
            println!(
              "{}",
              serde_json::to_string_pretty(&serde_json::to_value(&rollback_receipt)?)?
            );
          }
          OutputFormat::Text => {
            println!("진행상태: {}", rollback_receipt.status.progress_status);
            println!("결과상태: {}", rollback_receipt.status.result_status);
            println!("coding-agent verb: {}", verb.as_str());
            println!("artifact-family: {}", rollback_receipt.artifact_family);
            println!("handle-ref: {}", rollback_receipt.handle_ref);
            println!("rollback-class: {}", rollback_receipt.rollback_class);
            println!("rollback-status: {}", rollback_receipt.rollback_status);
            println!("repo-snapshot-ref: {}", rollback_receipt.repo_snapshot_ref);
            println!(
              "apply-artifact-ref: {}",
              rollback_receipt.apply_artifact_ref
            );
            if let Some(rollback_input_ref) = rollback_receipt.rollback_input_ref.as_deref() {
              println!("rollback-input-ref: {}", rollback_input_ref);
            }
            if let Some(restored_snapshot_ref) = rollback_receipt.restored_snapshot_ref.as_deref() {
              println!("restored-snapshot-ref: {}", restored_snapshot_ref);
            }
            if !rollback_receipt.restored_paths.is_empty() {
              println!(
                "restored-paths: {}",
                rollback_receipt.restored_paths.join(", ")
              );
            }
            if !rollback_receipt.rejected_paths.is_empty() {
              println!(
                "rejected-paths: {}",
                rollback_receipt.rejected_paths.join(", ")
              );
            }
            if let Some(inverse_plan_ref) = rollback_receipt.inverse_plan_ref.as_deref() {
              println!("inverse-plan-ref: {}", inverse_plan_ref);
            }
            if let Some(followup_verify_ref) = rollback_receipt.followup_verify_ref.as_deref() {
              println!("followup-verify-ref: {}", followup_verify_ref);
            }
            if !rollback_receipt.non_rollbackable_effects.is_empty() {
              println!(
                "non-rollbackable-effects: {}",
                rollback_receipt.non_rollbackable_effects.join(", ")
              );
            }
            println!("effect-contracts:");
            for contract in rollback_receipt.effect_contracts.iter().take(6) {
              println!(
                "  - {} => {} ({})",
                contract.effect_class, contract.rollback_contract, contract.rationale
              );
            }
            print_grounding_seed_text(&rollback_receipt.request.grounding_seed);
            print_repo_graph_seed_text(&rollback_receipt.request.repo_graph_seed);
            print_manual_evidence_seed_text(&rollback_receipt.request.manual_evidence_seed);
            print_attached_pack_seed_text(&rollback_receipt.request.attached_pack_seed);
            print_language_profile_text(&rollback_receipt.request.language_profile);
            if !rollback_receipt.proof_refs.is_empty() {
              println!("proof-refs:");
              for proof in rollback_receipt.proof_refs.iter().take(6) {
                println!("  - {}", proof);
              }
            }
            if let Some(path) = args.agent_request_out.as_ref() {
              println!("request-artifact-out: {}", path.display());
            }
            if let Some(path) = args.agent_rollback_out.as_ref() {
              println!("rollback-artifact-out: {}", path.display());
            }
            println!("비고: {}", rollback_receipt.status.note);
          }
        }
        Ok(())
      } else {
        let rollback_handle =
          build_coding_agent_rollback_handle(args, request, request_artifact_ref);
        if let Some(path) = args.agent_rollback_out.as_ref() {
          write_json_artifact(path, &rollback_handle, "coding-agent rollback handle")?;
        }
        persist_coding_memory_artifact_if_configured(
          rollback_handle.artifact_family,
          Some(rollback_handle.repo_snapshot_ref.clone()),
          rollback_handle.request.workspace.target_paths.clone(),
          rollback_handle.request.workspace.approved_commands.clone(),
          build_coding_memory_related_refs(
            args.agent_rollback_out.as_ref(),
            [
              rollback_handle.request_artifact_ref.as_deref(),
              Some(rollback_handle.handle_id.as_str()),
              Some(rollback_handle.apply_artifact_ref.as_str()),
              rollback_handle.inverse_plan_ref.as_deref(),
            ],
          ),
          &rollback_handle,
        )?;

        match args.output_format {
          OutputFormat::Json => {
            println!(
              "{}",
              serde_json::to_string_pretty(&serde_json::to_value(&rollback_handle)?)?
            );
          }
          OutputFormat::Text => {
            println!("진행상태: {}", rollback_handle.status.progress_status);
            println!("결과상태: {}", rollback_handle.status.result_status);
            println!("coding-agent verb: {}", verb.as_str());
            println!("artifact-family: {}", rollback_handle.artifact_family);
            println!("handle-id: {}", rollback_handle.handle_id);
            println!("rollback-class: {}", rollback_handle.rollback_class);
            println!("repo-snapshot-ref: {}", rollback_handle.repo_snapshot_ref);
            println!("apply-artifact-ref: {}", rollback_handle.apply_artifact_ref);
            if let Some(inverse_plan_ref) = rollback_handle.inverse_plan_ref.as_deref() {
              println!("inverse-plan-ref: {}", inverse_plan_ref);
            }
            if let Some(expires_at_ms) = rollback_handle.expires_at_ms {
              println!("expires-at-ms: {}", expires_at_ms);
            }
            if !rollback_handle.non_rollbackable_effects.is_empty() {
              println!(
                "non-rollbackable-effects: {}",
                rollback_handle.non_rollbackable_effects.join(", ")
              );
            }
            println!("effect-contracts:");
            for contract in rollback_handle.effect_contracts.iter().take(6) {
              println!(
                "  - {} => {} ({})",
                contract.effect_class, contract.rollback_contract, contract.rationale
              );
            }
            print_grounding_seed_text(&rollback_handle.request.grounding_seed);
            print_repo_graph_seed_text(&rollback_handle.request.repo_graph_seed);
            print_manual_evidence_seed_text(&rollback_handle.request.manual_evidence_seed);
            print_attached_pack_seed_text(&rollback_handle.request.attached_pack_seed);
            print_language_profile_text(&rollback_handle.request.language_profile);
            if !rollback_handle.proof_refs.is_empty() {
              println!("proof-refs:");
              for proof in rollback_handle.proof_refs.iter().take(6) {
                println!("  - {}", proof);
              }
            }
            if let Some(path) = args.agent_request_out.as_ref() {
              println!("request-artifact-out: {}", path.display());
            }
            if let Some(path) = args.agent_rollback_out.as_ref() {
              println!("rollback-artifact-out: {}", path.display());
            }
            println!("비고: {}", rollback_handle.status.note);
          }
        }
        Ok(())
      }
    }
    _ => {
      match args.output_format {
        OutputFormat::Json => {
          println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::to_value(&request)?)?
          );
        }
        OutputFormat::Text => {
          println!("진행상태: 맥락수집중");
          println!("결과상태: 부분완료");
          println!("coding-agent verb: {}", verb.as_str());
          println!("artifact-family: {}", request.artifact_family);
          println!("cwd: {}", request.workspace.cwd);
          if let Some(branch) = request.workspace.git_branch.as_deref() {
            println!("git-branch: {}", branch);
          }
          println!("git-dirty: {}", request.workspace.git_dirty);
          if let Some(request_text) = request.request.as_deref() {
            println!("request: {}", request_text);
          }
          if !request.workspace.target_paths.is_empty() {
            println!(
              "target-paths: {}",
              request.workspace.target_paths.join(", ")
            );
          }
          if !request.workspace.policy_bits.is_empty() {
            println!(
              "workspace-policy: {}",
              request.workspace.policy_bits.join(", ")
            );
          }
          print_grounding_seed_text(&request.grounding_seed);
          print_repo_graph_seed_text(&request.repo_graph_seed);
          print_manual_evidence_seed_text(&request.manual_evidence_seed);
          print_attached_pack_seed_text(&request.attached_pack_seed);
          print_language_profile_text(&request.language_profile);
          if let Some(path) = args.agent_request_out.as_ref() {
            println!("request-artifact-out: {}", path.display());
          }
          println!(
            "message: coding-agent request normalization, repo grounding, and joined manual evidence seed are landed; grounded patch/verify/rollback execution land in later CAX bundles"
          );
        }
      }
      Ok(())
    }
  }
}

/// Phase 145++++++++++(zzzzd-0) (vpl-gate.md 10.7 헌법, 2026-06-01):
/// retention caller 가 doghouse-core brain 함수 직접 호출에서
/// *doghouse relay → pnixc-meta /store/execute-native-action* HTTP
/// 경로로 이관.
///
/// 정직히:
/// - pnixc_meta::store::* 를 import 하지 *않음* (pnix-executor-graph
///   가 그 crate 의존을 들이지 않음).
/// - policy / lens_path / entry_fn / request_nix_expr / store_path
///   *어느 것도* caller 가 만들거나 보내지 *않음* (registry .px gate).
/// - doghouse relay 가 endpoint 까지 ferry; 직접 pnixc-meta 호출은
///   local/dev smoke 전용.
/// - 새 endpoint 가 *실제 append + remove* 실행 (planning only 아님).
///   기존 "delete/compaction executor is not opened" 문구는 제거.
fn run_coding_agent_retention(args: &Args) -> Result<()> {
  if args.dry_run {
    bail!(
      "dry-run unsupported after pnixc-meta retention endpoint migration; \
       future preview action required (zzzzd-0)"
    );
  }

  // zzzzd-0.5 (Codex audit): trim whitespace so operator-set env vars
  // with stray newlines / spaces don't silently mis-resolve.
  let doghouse_url = std::env::var("PNIX_GATE_DOGHOUSE_URL")
    .ok()
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| "http://127.0.0.1:8787".to_string());
  let evaluated_at_ms = current_time_ms();
  let request_body = format!(
    "{{\"action_name\":\"coding-memory-retention\",\"evaluated_at_ms\":{}}}",
    evaluated_at_ms
  );

  let response_body =
    post_store_native_action(&doghouse_url, &request_body).with_context(|| {
      format!(
        "POST {}/store/execute-native-action via doghouse relay (zzzzd-0; \
       set PNIX_GATE_DOGHOUSE_URL to override)",
        doghouse_url
      )
    })?;

  let bridge_receipt: serde_json::Value = serde_json::from_str(&response_body)
    .with_context(|| "parse BridgeReceipt JSON from doghouse relay")?;

  match args.output_format {
    OutputFormat::Json => {
      println!("{}", serde_json::to_string_pretty(&bridge_receipt)?);
    }
    OutputFormat::Text => {
      let lens_verdict = &bridge_receipt["lens_verdict"];
      let execution = &bridge_receipt["execution"];
      println!("substrate-action: pnixc-meta coding-memory-retention executed");
      if let Some(s) = bridge_receipt["lens_path"].as_str() {
        println!("lens-path: {}", s);
      }
      if let Some(s) = lens_verdict["policy"]["policy_id"].as_str() {
        println!("retention-policy-id: {}", s);
      }
      if let Some(n) = lens_verdict["evaluated_at_ms"].as_u64() {
        println!("evaluated-at-ms: {}", n);
      }
      if let Some(n) = lens_verdict["total_artifact_count"].as_u64() {
        println!("total-artifact-count: {}", n);
      }
      if let Some(n) = lens_verdict["summary"]["keep_count"].as_u64() {
        println!("keep-count: {}", n);
      }
      if let Some(n) = lens_verdict["summary"]["compact_candidate_count"].as_u64() {
        println!("compact-candidate-count: {}", n);
      }
      if let Some(n) = lens_verdict["summary"]["protected_count"].as_u64() {
        println!("protected-count: {}", n);
      }
      if let Some(n) = execution["successful_op_count"].as_u64() {
        println!("byte-io-op-count: {}", n);
      }
      if let Some(refs) = lens_verdict["proof_refs"].as_array() {
        if !refs.is_empty() {
          println!("proof-refs:");
          for proof in refs.iter().take(8) {
            if let Some(s) = proof.as_str() {
              println!("  - {}", s);
            }
          }
        }
      }
    }
  }
  Ok(())
}

/// Minimal blocking HTTP POST. No reqwest dep — pnix-executor-graph
/// already has heavy compile cost so we keep transport zero-dep.
/// Returns response body string on 2xx; otherwise anyhow error with
/// status + body.
fn post_store_native_action(base_url: &str, request_body: &str) -> Result<String> {
  use std::io::{Read, Write};
  use std::net::TcpStream;
  use std::time::Duration;

  let (host, port, path_prefix) = parse_http_url(base_url)?;
  let target = format!("{}/store/execute-native-action", path_prefix);
  let addr = format!("{}:{}", host, port);
  let mut stream =
    TcpStream::connect(&addr).with_context(|| format!("connect to doghouse relay at {}", addr))?;
  stream.set_read_timeout(Some(Duration::from_secs(60)))?;
  stream.set_write_timeout(Some(Duration::from_secs(10)))?;
  // zzzzd-0.5 (Codex audit): Host header carries full authority
  // (host:port) so virtual-host routing on the doghouse relay sees the
  // expected port; default-80 fallback only.
  let host_header = if port == 80 {
    host.clone()
  } else {
    format!("{}:{}", host, port)
  };
  let request = format!(
    "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
    target, host_header, request_body.len(), request_body
  );
  stream.write_all(request.as_bytes())?;
  let mut buf = String::new();
  stream.read_to_string(&mut buf)?;
  let head_end = buf
    .find("\r\n\r\n")
    .ok_or_else(|| anyhow::anyhow!("malformed HTTP response (no head/body split)"))?;
  let head = &buf[..head_end];
  let body = buf[head_end + 4..].to_string();
  let status_line = head.lines().next().unwrap_or("");
  let status_code: u16 = status_line
    .split_whitespace()
    .nth(1)
    .and_then(|s| s.parse().ok())
    .ok_or_else(|| anyhow::anyhow!("malformed status line: {}", status_line))?;
  if !(200..300).contains(&status_code) {
    bail!(
      "doghouse relay POST {} returned {} ({}): {}",
      target,
      status_code,
      status_line,
      body
    );
  }
  Ok(body)
}

/// Tiny URL parser — handles `http://host[:port][/prefix]`. Returns
/// (host, port, path_prefix). path_prefix is empty for root.
fn parse_http_url(url: &str) -> Result<(String, u16, String)> {
  let rest = url
    .strip_prefix("http://")
    .ok_or_else(|| anyhow::anyhow!("doghouse relay URL must start with http:// (got {})", url))?;
  let (authority, path_prefix) = match rest.find('/') {
    Some(i) => (&rest[..i], rest[i..].trim_end_matches('/').to_string()),
    None => (rest, String::new()),
  };
  let (host, port) = match authority.rfind(':') {
    Some(i) => {
      let host = authority[..i].to_string();
      let port: u16 = authority[i + 1..]
        .parse()
        .with_context(|| format!("parse port from {}", authority))?;
      (host, port)
    }
    None => (authority.to_string(), 80),
  };
  Ok((host, port, path_prefix))
}

fn write_json_artifact<T: Serialize>(path: &Path, artifact: &T, label: &str) -> Result<()> {
  if let Some(parent) = path.parent() {
    if !parent.as_os_str().is_empty() {
      fs::create_dir_all(parent)
        .with_context(|| format!("create {} dir {}", label, parent.display()))?;
    }
  }
  fs::write(path, serde_json::to_string_pretty(artifact)?)
    .with_context(|| format!("write {} artifact {}", label, path.display()))?;
  Ok(())
}

fn build_coding_memory_related_refs<const N: usize>(
  output_path: Option<&PathBuf>,
  refs: [Option<&str>; N],
) -> Vec<String> {
  let mut related_refs = Vec::new();
  if let Some(path) = output_path {
    related_refs.push(path_to_slash(path));
  }
  for value in refs.into_iter().flatten() {
    if !value.trim().is_empty() {
      related_refs.push(value.to_string());
    }
  }
  related_refs.sort();
  related_refs.dedup();
  related_refs
}

fn coding_memory_store_path_from_env() -> Option<PathBuf> {
  std::env::var_os("DOGHOUSE_STORE_PATH")
    .filter(|value| !value.is_empty())
    .map(PathBuf::from)
}

fn persist_coding_memory_artifact_if_configured<T: Serialize>(
  artifact_family: &str,
  repo_snapshot_ref: Option<String>,
  target_paths: Vec<String>,
  command_refs: Vec<String>,
  related_refs: Vec<String>,
  artifact: &T,
) -> Result<Option<String>> {
  let Some(store_path) = coding_memory_store_path_from_env() else {
    return Ok(None);
  };
  #[cfg(not(feature = "doghouse"))]
  {
    // doghouse feature off: coding memory store is unavailable, so there is
    // nothing to persist. Mirror the env-unset path and return None.
    let _ = (
      &store_path,
      artifact_family,
      repo_snapshot_ref,
      target_paths,
      command_refs,
      related_refs,
      artifact,
    );
    return Ok(None);
  }
  #[cfg(feature = "doghouse")]
  match persist_coding_memory_artifact_to_store(
    &store_path,
    artifact_family,
    repo_snapshot_ref,
    target_paths,
    command_refs,
    related_refs,
    artifact,
  ) {
    Ok(artifact_id) => Ok(Some(artifact_id)),
    Err(err) => {
      eprintln!(
        "warning: coding memory store append skipped for {} at {}: {:#}",
        artifact_family,
        store_path.display(),
        err
      );
      Ok(None)
    }
  }
}

#[cfg(feature = "doghouse")]
fn persist_coding_memory_artifact_to_store<T: Serialize>(
  store_path: &Path,
  artifact_family: &str,
  repo_snapshot_ref: Option<String>,
  target_paths: Vec<String>,
  command_refs: Vec<String>,
  related_refs: Vec<String>,
  artifact: &T,
) -> Result<String> {
  let payload = serde_json::to_value(artifact)?;
  let artifact_id = make_coding_memory_artifact_id(artifact_family, &payload)?;
  let stored_artifact = CodingMemoryArtifact {
    id: artifact_id.clone(),
    artifact_family: artifact_family.to_string(),
    source_surface: "pnix coding-agent".to_string(),
    stored_at_ms: current_time_ms(),
    repo_snapshot_ref,
    target_paths,
    command_refs,
    related_refs,
    payload,
  };
  let store = DoghouseStore::open(DoghouseStoreConfig::new(store_path.to_path_buf()))
    .with_context(|| format!("open doghouse coding memory store {}", store_path.display()))?;
  doghouse_core::store::append_coding_memory_artifact_at(store.path(), &stored_artifact)
    .with_context(|| {
      format!(
        "append coding-agent artifact {} to doghouse store {}",
        artifact_family,
        store_path.display()
      )
    })?;
  Ok(artifact_id)
}

// Phase 145++++++++++(zzzzd-1) (2026-06-01): retention brain dead helpers
// removed. caller migration (zzzzd-0) 후 caller 0 — pnixc-meta substrate
// 가 정본 owner. `build_coding_agent_retention_receipt_for_store` /
// `append_coding_agent_retention_receipt_to_store` 자리 삭제.

#[cfg(feature = "doghouse")]
fn make_coding_memory_artifact_id(
  artifact_family: &str,
  payload: &serde_json::Value,
) -> Result<String> {
  let mut hasher = Sha256::new();
  hasher.update(artifact_family.as_bytes());
  hasher.update(b"\n--payload--\n");
  hasher.update(&serde_json::to_vec(payload)?);
  Ok(format!("coding.memory::{:x}", hasher.finalize()))
}

#[derive(Debug, Serialize)]
struct CodingAgentRequestArtifact {
  artifact_family: &'static str,
  phase: &'static str,
  surface: &'static str,
  verb: &'static str,
  request: Option<String>,
  normalized_at_ms: u64,
  workspace: CodingAgentWorkspaceSnapshot,
  context_pack: CodingAgentContextPackSurface,
  grounding_seed: CodingAgentGroundingSeed,
  repo_graph_seed: CodingAgentRepoGraphSeed,
  manual_evidence_seed: CodingAgentManualEvidenceSeed,
  attached_pack_seed: CodingAgentAttachedPackSeed,
  language_profile: CodingAgentLanguageProfileSurface,
}

#[derive(Debug, Serialize)]
struct CodingAgentPlanArtifact {
  artifact_family: &'static str,
  phase: &'static str,
  surface: &'static str,
  verb: &'static str,
  planned_at_ms: u64,
  request_artifact_ref: Option<String>,
  current_interpretation: String,
  interpretation_set: CodingAgentInterpretationSetSurface,
  judgement: CodingAgentJudgementSurface,
  execution_plan: CodingAgentExecutionPlanSurface,
  bounded_step_family: Vec<CodingAgentPlanStep>,
  expected_verification: Vec<String>,
  failure_policy: &'static str,
  status: CodingAgentStatusSurface,
  request: CodingAgentRequestArtifact,
}

#[derive(Debug, Serialize)]
struct CodingAgentPatchProposalArtifact {
  artifact_family: &'static str,
  phase: &'static str,
  surface: &'static str,
  verb: &'static str,
  proposed_at_ms: u64,
  request_artifact_ref: Option<String>,
  current_plan_ref: Option<String>,
  current_interpretation: String,
  target_paths: Vec<String>,
  edit_family: &'static str,
  diff_ref: String,
  expected_verify_ref: Vec<String>,
  risk_class: &'static str,
  effect_classes: Vec<String>,
  generated_patch_candidate: Option<CodingAgentGeneratedPatchCandidateSurface>,
  generated_patch_review_receipt: Option<CodingAgentGeneratedPatchReviewReceiptSurface>,
  provider_feedback_request: Option<CodingAgentProviderFeedbackRequestSurface>,
  feedback_retry_guard: Option<CodingAgentFeedbackRetryGuardSurface>,
  apply_handoff_proof: Option<CodingAgentApplyHandoffProofSurface>,
  promotion_boundary_receipt: Option<CodingAgentPromotionBoundaryReceiptSurface>,
  apply_intent: CodingAgentApplyIntentSurface,
  apply_result: Option<CodingAgentApplyResultSurface>,
  context_demand_replay: CodingAgentContextDemandReplaySurface,
  repair_recipe_replay: CodingAgentRepairRecipeReplaySurface,
  semantic_review: CodingAgentSemanticPatchReviewSurface,
  status: CodingAgentStatusSurface,
  request: CodingAgentRequestArtifact,
}

#[derive(Debug, Serialize)]
struct CodingAgentVerifyReceiptArtifact {
  artifact_family: &'static str,
  phase: &'static str,
  surface: &'static str,
  verb: &'static str,
  verified_at_ms: u64,
  request_artifact_ref: Option<String>,
  repo_snapshot_ref: String,
  target_paths: Vec<String>,
  target_commands: Vec<String>,
  before_artifact_ref: String,
  after_artifact_ref: String,
  diff_ref: String,
  execution_result: CodingAgentExecutionResultSurface,
  diagnostic_records: Vec<CodingAgentDiagnosticRecordSurface>,
  failure_pattern_matches: Vec<CodingAgentFailurePatternMatchSurface>,
  context_demands: Vec<CodingAgentContextDemandSurface>,
  promotion_boundary_join_receipt: Option<CodingAgentPromotionBoundaryJoinReceiptSurface>,
  status: CodingAgentStatusSurface,
  proof_refs: Vec<String>,
  learning_card: CodingAgentLearningCardSurface,
  request: CodingAgentRequestArtifact,
}

#[derive(Debug, Serialize)]
struct CodingAgentRollbackHandleArtifact {
  artifact_family: &'static str,
  phase: &'static str,
  surface: &'static str,
  verb: &'static str,
  issued_at_ms: u64,
  request_artifact_ref: Option<String>,
  handle_id: String,
  repo_snapshot_ref: String,
  apply_artifact_ref: String,
  inverse_plan_ref: Option<String>,
  rollback_class: &'static str,
  effect_contracts: Vec<CodingAgentRollbackEffectContract>,
  non_rollbackable_effects: Vec<String>,
  expires_at_ms: Option<u64>,
  status: CodingAgentStatusSurface,
  proof_refs: Vec<String>,
  request: CodingAgentRequestArtifact,
}

#[derive(Debug, Serialize)]
struct CodingAgentRollbackReceiptArtifact {
  artifact_family: &'static str,
  phase: &'static str,
  surface: &'static str,
  verb: &'static str,
  rolled_back_at_ms: u64,
  request_artifact_ref: Option<String>,
  handle_ref: String,
  repo_snapshot_ref: String,
  apply_artifact_ref: String,
  inverse_plan_ref: Option<String>,
  restored_snapshot_ref: Option<String>,
  followup_verify_ref: Option<String>,
  rollback_status: &'static str,
  rollback_input_ref: Option<String>,
  dry_run: bool,
  restored_paths: Vec<String>,
  rejected_paths: Vec<String>,
  file_results: Vec<CodingAgentPatchFileApplyRecord>,
  error: Option<String>,
  rollback_class: &'static str,
  effect_contracts: Vec<CodingAgentRollbackEffectContract>,
  non_rollbackable_effects: Vec<String>,
  status: CodingAgentStatusSurface,
  proof_refs: Vec<String>,
  request: CodingAgentRequestArtifact,
}

#[derive(Debug, Serialize)]
struct CodingAgentRollbackEffectContract {
  effect_class: String,
  rollback_contract: &'static str,
  rationale: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentApplyIntentSurface {
  intent_family: &'static str,
  effect_classes: Vec<String>,
  apply_status: &'static str,
  apply_artifact_ref: Option<String>,
  separated_from_proposal: bool,
}

#[derive(Debug, Serialize)]
struct CodingAgentGeneratedPatchCandidateSurface {
  artifact_family: &'static str,
  phase: &'static str,
  candidate_ref: String,
  candidate_owner: &'static str,
  source_path: String,
  patch_input_ref: String,
  byte_len: usize,
  line_count: usize,
  target_paths: Vec<String>,
  parsed_target_paths: Vec<String>,
  rejected_target_paths: Vec<String>,
  quarantine_status: &'static str,
  lineage_status: &'static str,
  source_provider_feedback_request_ref: Option<String>,
  response_boundary: &'static str,
  promotion_boundary: &'static str,
  required_next_artifacts: Vec<String>,
  proof_refs: Vec<String>,
  error: Option<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentGeneratedPatchReviewReceiptSurface {
  artifact_family: &'static str,
  phase: &'static str,
  review_ref: String,
  review_owner: &'static str,
  candidate_ref: String,
  patch_input_ref: String,
  target_paths: Vec<String>,
  parsed_target_paths: Vec<String>,
  rejected_target_paths: Vec<String>,
  review_status: &'static str,
  diagnostic_records: Vec<CodingAgentDiagnosticRecordSurface>,
  failure_pattern_matches: Vec<CodingAgentFailurePatternMatchSurface>,
  context_demands: Vec<CodingAgentContextDemandSurface>,
  required_next_artifacts: Vec<String>,
  proof_refs: Vec<String>,
  promotion_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentProviderFeedbackRequestSurface {
  artifact_family: &'static str,
  phase: &'static str,
  request_ref: String,
  feedback_owner: &'static str,
  source_review_ref: String,
  source_candidate_ref: String,
  patch_input_ref: String,
  request_status: &'static str,
  provider_boundary: &'static str,
  target_paths: Vec<String>,
  context_demand_refs: Vec<String>,
  feedback_packets: Vec<CodingAgentProviderFeedbackPacketSurface>,
  required_evidence: Vec<String>,
  forbidden_effects: Vec<String>,
  proof_refs: Vec<String>,
  promotion_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentFeedbackRetryGuardSurface {
  artifact_family: &'static str,
  phase: &'static str,
  guard_ref: String,
  guard_owner: &'static str,
  source_provider_feedback_request_ref: String,
  source_candidate_ref: String,
  source_review_ref: String,
  attempt_index: u32,
  attempt_limit: u32,
  guard_status: &'static str,
  retry_decision: &'static str,
  context_demand_refs: Vec<String>,
  required_human_evidence: Vec<String>,
  forbidden_effects: Vec<String>,
  proof_refs: Vec<String>,
  promotion_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentProviderFeedbackPacketSurface {
  packet_ref: String,
  packet_kind: &'static str,
  source_context_demand_ref: String,
  target_path: String,
  demand_family: String,
  required_evidence: Vec<String>,
  requested_output: &'static str,
  response_boundary: &'static str,
  truth_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentApplyHandoffProofSurface {
  artifact_family: &'static str,
  phase: &'static str,
  handoff_ref: String,
  handoff_owner: &'static str,
  candidate_ref: String,
  candidate_review_ref: String,
  candidate_patch_input_ref: String,
  apply_patch_input_ref: String,
  apply_patch_source_path: String,
  handoff_status: &'static str,
  failure_reason: Option<String>,
  target_paths: Vec<String>,
  parsed_candidate_target_paths: Vec<String>,
  required_evidence: Vec<String>,
  forbidden_effects: Vec<String>,
  proof_refs: Vec<String>,
  promotion_boundary: &'static str,
}

#[derive(Debug)]
struct CodingAgentPatchApplyBuildResult {
  apply_result: Option<CodingAgentApplyResultSurface>,
  apply_handoff_proof: Option<CodingAgentApplyHandoffProofSurface>,
}

#[derive(Debug, Serialize)]
struct CodingAgentPromotionBoundaryReceiptSurface {
  artifact_family: &'static str,
  phase: &'static str,
  receipt_ref: String,
  receipt_owner: &'static str,
  source_apply_artifact_ref: String,
  source_handoff_ref: Option<String>,
  apply_status: &'static str,
  promotion_status: &'static str,
  required_next_artifacts: Vec<String>,
  forbidden_effects: Vec<String>,
  proof_refs: Vec<String>,
  promotion_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentPromotionBoundaryJoinReceiptSurface {
  artifact_family: &'static str,
  phase: &'static str,
  join_ref: String,
  join_owner: &'static str,
  source_promotion_boundary_receipt_ref: String,
  source_apply_artifact_ref: String,
  source_handoff_ref: Option<String>,
  verify_diff_ref: String,
  verify_execution_result_ref: String,
  verify_status: &'static str,
  execution_status: &'static str,
  join_status: &'static str,
  target_paths: Vec<String>,
  target_commands: Vec<String>,
  required_next_artifacts: Vec<String>,
  forbidden_effects: Vec<String>,
  proof_refs: Vec<String>,
  promotion_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentHumanPromotionDecisionArtifact {
  artifact_family: &'static str,
  phase: &'static str,
  surface: &'static str,
  verb: &'static str,
  decided_at_ms: u64,
  request_artifact_ref: Option<String>,
  decision_ref: String,
  decision_owner: &'static str,
  source_promotion_boundary_join_ref: String,
  human_decision: String,
  decision_status: &'static str,
  promotion_status: &'static str,
  human_rationale: Option<String>,
  target_paths: Vec<String>,
  target_commands: Vec<String>,
  required_next_artifacts: Vec<String>,
  forbidden_effects: Vec<String>,
  proof_refs: Vec<String>,
  promotion_boundary: &'static str,
  status: CodingAgentStatusSurface,
  request: CodingAgentRequestArtifact,
}

#[derive(Debug, Serialize)]
struct CodingAgentApplyResultSurface {
  artifact_family: &'static str,
  phase: &'static str,
  applied_at_ms: u64,
  apply_artifact_ref: String,
  patch_input_ref: Option<String>,
  apply_status: &'static str,
  dry_run: bool,
  target_paths: Vec<String>,
  applied_paths: Vec<String>,
  rejected_paths: Vec<String>,
  file_results: Vec<CodingAgentPatchFileApplyRecord>,
  rollback_class: &'static str,
  rollback_handle_ref: Option<String>,
  inverse_plan_ref: Option<String>,
  proof_refs: Vec<String>,
  error: Option<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentSemanticPatchReviewSurface {
  artifact_family: &'static str,
  phase: &'static str,
  review_ref: String,
  review_owner: &'static str,
  diff_ref: String,
  patch_input_ref: Option<String>,
  apply_artifact_ref: Option<String>,
  target_paths: Vec<String>,
  meaning_impact_diff: CodingAgentMeaningImpactDiffSurface,
  patch_decision_link: CodingAgentPatchDecisionLinkSurface,
  narrative_regression: CodingAgentNarrativeRegressionSurface,
  proof_refs: Vec<String>,
  review_status: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentContextDemandReplaySurface {
  artifact_family: &'static str,
  phase: &'static str,
  replay_ref: String,
  replay_owner: &'static str,
  source_refs: Vec<String>,
  source_artifact_refs: Vec<String>,
  replayed_context_demands: Vec<CodingAgentReplayedContextDemandSurface>,
  diagnostic_refs: Vec<String>,
  semantic_review_refs: Vec<String>,
  next_patch_requirements: Vec<String>,
  replay_status: &'static str,
  promotion_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentReplayedContextDemandSurface {
  artifact_family: &'static str,
  replay_item_ref: String,
  source_ref: String,
  source_family: String,
  language: String,
  target_path: String,
  demand_family: String,
  required_evidence: Vec<String>,
  request_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentRepairRecipeReplaySurface {
  artifact_family: &'static str,
  phase: &'static str,
  replay_ref: String,
  replay_owner: &'static str,
  source_refs: Vec<String>,
  source_artifact_refs: Vec<String>,
  learning_card_refs: Vec<String>,
  repair_candidates: Vec<CodingAgentRepairRecipeCandidateSurface>,
  replay_status: &'static str,
  promotion_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentRepairRecipeCandidateSurface {
  artifact_family: &'static str,
  candidate_ref: String,
  source_ref: String,
  source_family: String,
  trigger: String,
  repair_pattern: String,
  verify_pattern: String,
  reuse_score: f64,
  required_context_refs: Vec<String>,
  promotion_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentMeaningImpactDiffSurface {
  artifact_family: &'static str,
  impact_ref: String,
  diff_ref: String,
  target_paths: Vec<String>,
  meaning_classes: Vec<String>,
  changed_symbol_refs: Vec<String>,
  effect_refs: Vec<String>,
  verification_refs: Vec<String>,
  impact_summary: String,
  risk_signal: &'static str,
  promotion_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentPatchDecisionLinkSurface {
  artifact_family: &'static str,
  link_ref: String,
  diff_ref: String,
  decision_family: &'static str,
  decision_refs: Vec<String>,
  evidence_refs: Vec<String>,
  policy_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentNarrativeRegressionSurface {
  artifact_family: &'static str,
  regression_ref: String,
  narrative_status: &'static str,
  checked_dimensions: Vec<&'static str>,
  risk_notes: Vec<String>,
  proof_boundary: &'static str,
}

#[derive(Debug, Serialize, Clone)]
struct CodingAgentPatchFileApplyRecord {
  path: String,
  status: &'static str,
  before_snapshot_ref: Option<String>,
  after_snapshot_ref: Option<String>,
  byte_delta: i64,
  error: Option<String>,
}

struct PreparedPatchFileApply {
  path: String,
  absolute_path: PathBuf,
  after_content: String,
  before_snapshot_ref: Option<String>,
  after_snapshot_ref: String,
  byte_delta: i64,
}

struct ParsedUnifiedFilePatch {
  old_path: Option<String>,
  target_path: String,
  hunks: Vec<ParsedUnifiedHunk>,
}

struct ParsedUnifiedHunk {
  old_start: usize,
  lines: Vec<ParsedUnifiedLine>,
}

enum ParsedUnifiedLine {
  Context(String),
  Remove(String),
  Add(String),
}

struct CodingAgentExplicitRollbackResult {
  rollback_status: &'static str,
  rollback_input_ref: Option<String>,
  dry_run: bool,
  restored_paths: Vec<String>,
  rejected_paths: Vec<String>,
  file_results: Vec<CodingAgentPatchFileApplyRecord>,
  restored_snapshot_ref: Option<String>,
  error: Option<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentContextPackSurface {
  artifact_family: &'static str,
  phase: &'static str,
  context_pack_ref: String,
  pack_owner: &'static str,
  close_status: &'static str,
  section_family: Vec<CodingAgentContextPackSection>,
  target_paths: Vec<String>,
  forbidden_effects: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentContextPackSection {
  section_family: &'static str,
  item_count: usize,
  provenance_ref: String,
}

#[derive(Debug, Serialize)]
struct CodingAgentInterpretationSetSurface {
  artifact_family: &'static str,
  phase: &'static str,
  selected_interpretation: String,
  alternatives: Vec<String>,
  ambiguity_policy: &'static str,
  evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentJudgementSurface {
  artifact_family: &'static str,
  phase: &'static str,
  decision: &'static str,
  blocked_reasons: Vec<String>,
  required_next_artifacts: Vec<&'static str>,
  evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentExecutionPlanSurface {
  artifact_family: &'static str,
  phase: &'static str,
  execution_plan_ref: String,
  execution_owner: &'static str,
  effect_policy: &'static str,
  bounded_step_family: Vec<CodingAgentPlanStep>,
  expected_verification: Vec<String>,
  language_verify_targets: Vec<CodingAgentVerifyTargetSurface>,
  execution_requests: Vec<CodingAgentExecutionRequestSurface>,
}

#[derive(Debug, Serialize)]
struct CodingAgentExecutionRequestSurface {
  artifact_family: &'static str,
  phase: &'static str,
  request_ref: String,
  permission_status: &'static str,
  command_refs: Vec<String>,
  candidate_verify_target_refs: Vec<String>,
  candidate_command_refs: Vec<String>,
  effect_classes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentExecutionResultSurface {
  artifact_family: &'static str,
  phase: &'static str,
  observed_at_ms: u64,
  execution_result_ref: String,
  execution_status: &'static str,
  command_refs: Vec<String>,
  command_results: Vec<CodingAgentCommandExecutionRecord>,
  raw_result_refs: Vec<String>,
  exit_code: Option<i32>,
}

#[derive(Debug, Serialize)]
struct CodingAgentCommandExecutionRecord {
  command_ref: String,
  program: String,
  args: Vec<String>,
  cwd: String,
  status: &'static str,
  exit_code: Option<i32>,
  duration_ms: u64,
  stdout_ref: String,
  stderr_ref: String,
  stdout_preview: String,
  stderr_preview: String,
  error: Option<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentLearningCardSurface {
  artifact_family: &'static str,
  phase: &'static str,
  learning_card_ref: String,
  trigger: String,
  context_signature: String,
  repair_pattern: String,
  verify_pattern: String,
  reuse_score: f64,
  promotion_status: &'static str,
  proof_refs: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
struct CodingAgentPlanStep {
  step_family: &'static str,
  capability_bound: &'static str,
  summary: String,
}

#[derive(Debug, Serialize)]
struct CodingAgentStatusSurface {
  progress_status: &'static str,
  result_status: &'static str,
  note: String,
}

#[derive(Debug, Serialize)]
struct CodingAgentWorkspaceSnapshot {
  cwd: String,
  repo_root: Option<String>,
  git_branch: Option<String>,
  git_head_commit: Option<String>,
  git_dirty: bool,
  target_paths: Vec<String>,
  approved_commands: Vec<String>,
  forbidden_paths: Vec<String>,
  policy_bits: Vec<String>,
  current_plan_ref: Option<String>,
  rollback_handle_ref: Option<String>,
  last_verification_ref: Option<String>,
  promotion_boundary_ref: Option<String>,
  source_apply_artifact_ref: Option<String>,
  source_handoff_ref: Option<String>,
  promotion_boundary_join_ref: Option<String>,
  promotion_decision: Option<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentGroundingSeed {
  scan_root: String,
  scan_mode: &'static str,
  parser_owner: &'static str,
  entries: Vec<CodingAgentGroundingSeedEntry>,
}

#[derive(Debug, Serialize)]
struct CodingAgentGroundingSeedEntry {
  path: String,
  path_kind: &'static str,
  language: String,
  parser_backend: String,
  parser_capability: String,
  provenance_ref: String,
}

#[derive(Debug, Serialize)]
struct CodingAgentRepoGraphSeed {
  bundle_scope: &'static str,
  graph_owner: &'static str,
  graph_capability: &'static str,
  project_graph_status: &'static str,
  seto_enrichment_state: &'static str,
  project_reference_edges: Vec<CodingAgentRepoGraphProjectReference>,
  incremental_refresh: CodingAgentRepoGraphIncrementalRefresh,
  files: Vec<CodingAgentRepoGraphFile>,
}

#[derive(Debug, Serialize)]
struct CodingAgentRepoGraphFile {
  file_anchor: String,
  language: String,
  parser_backend: String,
  parser_capability: String,
  symbol_nodes: Vec<CodingAgentRepoGraphSymbol>,
  reference_edges: Vec<CodingAgentRepoGraphReference>,
  test_targets: Vec<String>,
  runtime_entrypoints: Vec<String>,
  provenance_ref: String,
}

#[derive(Debug, Serialize)]
struct CodingAgentRepoGraphProjectReference {
  from_file_anchor: String,
  to_file_anchor: String,
  to_symbol_id: String,
  via_term: String,
  edge_kind: String,
  provenance_ref: String,
}

#[derive(Debug, Serialize)]
struct CodingAgentRepoGraphIncrementalRefresh {
  refresh_owner: &'static str,
  refresh_mode: &'static str,
  changed_files: Vec<String>,
  refresh_batch: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentManualEvidenceSeed {
  join_owner: &'static str,
  join_policy: &'static str,
  language_hints: Vec<String>,
  hits: Vec<DocsetJoinedEvidence>,
  uncertainty_receipts: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentAttachedPackSeed {
  attach_owner: &'static str,
  project_pack_roots: Vec<CodingAgentAttachedPackRoot>,
  history_pack_roots: Vec<CodingAgentAttachedPackRoot>,
  total_entry_count: usize,
}

#[derive(Debug, Serialize)]
struct CodingAgentAttachedPackRoot {
  root: String,
  pack_kind: &'static str,
  status: &'static str,
  entry_count: usize,
  entries: Vec<CodingAgentAttachedPackEntry>,
}

#[derive(Debug, Serialize)]
struct CodingAgentAttachedPackEntry {
  entry_ref: String,
  entry_kind: &'static str,
  provenance_ref: String,
}

#[derive(Debug, Serialize)]
struct CodingAgentLanguageProfileSurface {
  artifact_family: &'static str,
  phase: &'static str,
  profile_owner: &'static str,
  adapter_boundary: &'static str,
  supported_adapters: Vec<CodingAgentLanguageAdapterSurface>,
  semantic_records: Vec<CodingAgentSemanticRecordSurface>,
  effect_records: Vec<CodingAgentEffectRecordSurface>,
  verify_targets: Vec<CodingAgentVerifyTargetSurface>,
  diagnostic_records: Vec<CodingAgentDiagnosticRecordSurface>,
  failure_pattern_matches: Vec<CodingAgentFailurePatternMatchSurface>,
  context_demands: Vec<CodingAgentContextDemandSurface>,
  unsupported_targets: Vec<CodingAgentUnsupportedLanguageTarget>,
  close_status: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentLanguageAdapterSurface {
  language: String,
  adapter_owner: String,
  adapter_status: &'static str,
  target_count: usize,
  record_families: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct CodingAgentSemanticRecordSurface {
  artifact_family: &'static str,
  record_ref: String,
  language: String,
  target_path: String,
  adapter_owner: String,
  parser_backend: String,
  parser_capability: String,
  meaning_class: &'static str,
  symbol_refs: Vec<String>,
  contract_refs: Vec<String>,
  provenance_refs: Vec<String>,
  judgement_boundary: &'static str,
  record_status: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentEffectRecordSurface {
  artifact_family: &'static str,
  record_ref: String,
  language: String,
  target_path: String,
  adapter_owner: String,
  effect_classes: Vec<String>,
  mutation_boundary: &'static str,
  rollback_expectation: &'static str,
  provenance_refs: Vec<String>,
  record_status: &'static str,
}

#[derive(Debug, Serialize, Clone)]
struct CodingAgentVerifyTargetSurface {
  artifact_family: &'static str,
  target_ref: String,
  language: String,
  target_path: String,
  verify_family: &'static str,
  command_candidates: Vec<String>,
  required_signals: Vec<String>,
  permission_status: &'static str,
  judgement_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentDiagnosticRecordSurface {
  artifact_family: &'static str,
  diagnostic_ref: String,
  language: String,
  target_path: String,
  diagnostic_family: &'static str,
  severity: &'static str,
  message: String,
  provenance_refs: Vec<String>,
  record_status: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentFailurePatternMatchSurface {
  artifact_family: &'static str,
  match_ref: String,
  diagnostic_ref: String,
  pattern_key: &'static str,
  confidence: f64,
  context_demand_ref: String,
  promotion_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentContextDemandSurface {
  artifact_family: &'static str,
  context_demand_ref: String,
  language: String,
  target_path: String,
  demand_family: &'static str,
  required_evidence: Vec<String>,
  request_boundary: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentUnsupportedLanguageTarget {
  target_path: String,
  detected_language: String,
  status: &'static str,
  reason: &'static str,
}

#[derive(Debug, Serialize)]
struct CodingAgentRepoGraphSymbol {
  symbol_id: String,
  name: String,
  kind: String,
  definition_anchor: String,
  use_anchors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CodingAgentRepoGraphReference {
  from_symbol_id: String,
  to_anchor: String,
  edge_kind: String,
}

struct GroundingSeedBuildResult {
  seed: CodingAgentGroundingSeed,
  changed_inputs: Vec<PathBuf>,
  graph_inputs: Vec<PathBuf>,
}

fn build_coding_agent_request(args: &Args, verb: AgentVerb) -> Result<CodingAgentRequestArtifact> {
  let cwd = std::env::current_dir().context("read current dir for coding-agent request")?;
  let git = probe_git_workspace(&cwd);
  let input = CodingAgentRequestInput::from(args);
  build_coding_agent_request_with_probe(&input, verb, &cwd, &git)
}

/// `build_coding_agent_request` 의 git-subprocess-free variant.
///
/// caller 가 자기 cwd 와 git probe 결과를 *envelope-in* 으로 inject 한다.
/// 이 함수 자체는 `Command::new("git")` 도, `std::env::current_dir()` 도
/// 호출하지 않는다.
///
/// **Client-side only** lane (헌법 §16 정합):
/// 이 fn 은 `cwd` 의 file system 을 *직접 scan* 한다 (grounding seed, attached
/// pack seed, language profile). 그래서 *client (puck-cli / freecat-cli)* 가
/// 자기 repo cwd 에서 호출해야 의미 있는 결과가 나온다. *server (doghouse-http)*
/// 는 이 fn 을 절대 호출하지 않는다 — server lane 은 client 가 emit 한 artifact
/// 를 `POST /coding-agent/artifact-push` (signed envelope contract:
/// `puck.coding-agent.artifact-push.v1`) 로 받아 `store.
/// append_coding_memory_artifact` 만 한다. 그래서 server 측 fs scan 0 + git
/// subprocess 0 가 정합.
///
/// callsite 검증: `pnix-executor-graph` crate 안에서만 호출되며, doghouse-core
/// 는 그 fn import 안 한다 (Cargo.toml deps + grep 으로 검증 가능). 만약 미래
/// server 측에서 *직접* coding-agent artifact 를 build 해야 한다면 별도
/// envelope-only variant 가 필요하지만, 현재 그런 lane 없음.
fn build_coding_agent_request_with_probe(
  input: &CodingAgentRequestInput,
  verb: AgentVerb,
  cwd: &Path,
  git: &GitWorkspaceProbe,
) -> Result<CodingAgentRequestArtifact> {
  let grounding_seed =
    build_coding_agent_grounding_seed(cwd, git.repo_root.as_deref(), &input.agent_target_paths);
  let repo_graph_seed = build_coding_agent_repo_graph_seed(
    cwd,
    git.repo_root.as_deref(),
    &grounding_seed.changed_inputs,
    &grounding_seed.graph_inputs,
  );
  let manual_evidence_seed = build_coding_agent_manual_evidence_seed(&repo_graph_seed);
  let attached_pack_seed = build_coding_agent_attached_pack_seed(
    cwd,
    git.repo_root.as_deref(),
    &input.agent_project_pack_roots,
    &input.agent_history_pack_roots,
  );
  let workspace = CodingAgentWorkspaceSnapshot {
    cwd: path_to_slash(cwd),
    repo_root: git.repo_root.clone(),
    git_branch: git.branch.clone(),
    git_head_commit: git.head_commit.clone(),
    git_dirty: git.dirty,
    target_paths: input
      .agent_target_paths
      .iter()
      .map(|p| path_to_slash(p))
      .collect(),
    approved_commands: input.agent_approved_commands.clone(),
    forbidden_paths: input
      .agent_forbidden_paths
      .iter()
      .map(|p| path_to_slash(p))
      .collect(),
    policy_bits: input.agent_policy_bits.clone(),
    current_plan_ref: input.agent_current_plan_ref.clone(),
    rollback_handle_ref: input.agent_rollback_handle_ref.clone(),
    last_verification_ref: input.agent_last_verification_ref.clone(),
    promotion_boundary_ref: input.agent_promotion_boundary_ref.clone(),
    source_apply_artifact_ref: input.agent_source_apply_artifact_ref.clone(),
    source_handoff_ref: input.agent_source_handoff_ref.clone(),
    promotion_boundary_join_ref: input.agent_promotion_boundary_join_ref.clone(),
    promotion_decision: input.agent_promotion_decision.clone(),
  };
  let context_pack = build_coding_agent_context_pack(
    &workspace,
    &grounding_seed.seed,
    &repo_graph_seed,
    &manual_evidence_seed,
    &attached_pack_seed,
  );
  let language_profile = build_coding_agent_language_profile(
    cwd,
    git.repo_root.as_deref(),
    &workspace,
    &grounding_seed.seed,
    &repo_graph_seed,
  );

  Ok(CodingAgentRequestArtifact {
    artifact_family: "coding.request",
    phase: "CAX.2c-partial",
    surface: "pnix coding-agent",
    verb: verb.as_str(),
    request: input.agent_request.clone(),
    normalized_at_ms: current_time_ms(),
    workspace,
    context_pack,
    grounding_seed: grounding_seed.seed,
    repo_graph_seed,
    manual_evidence_seed,
    attached_pack_seed,
    language_profile,
  })
}

fn build_coding_agent_grounding_seed(
  cwd: &Path,
  repo_root: Option<&str>,
  target_paths: &[PathBuf],
) -> GroundingSeedBuildResult {
  let scan_root = repo_root
    .map(PathBuf::from)
    .unwrap_or_else(|| cwd.to_path_buf());
  let scan_mode = if target_paths.is_empty() {
    "repo-root-bounded-scan"
  } else {
    "explicit-target-scope"
  };

  let mut entries = Vec::new();
  let mut changed_inputs = Vec::new();
  let mut graph_inputs = BTreeSet::new();
  if target_paths.is_empty() {
    let discovered = collect_repo_root_seed_paths(&scan_root, 12);
    for path in discovered {
      graph_inputs.insert(path.clone());
      entries.push(build_grounding_seed_entry(&path, &scan_root, cwd, "file"));
    }
  } else {
    for target in target_paths {
      let resolved = resolve_grounding_target(cwd, &scan_root, target);
      if resolved.is_dir() {
        let mut discovered = BTreeSet::new();
        collect_supported_scan_paths(&resolved, 2, 12, &mut discovered);
        if discovered.is_empty() {
          entries.push(build_directory_grounding_seed_entry(
            &resolved, &scan_root, cwd,
          ));
        } else {
          for path in discovered {
            changed_inputs.push(path.clone());
            graph_inputs.insert(path.clone());
            entries.push(build_grounding_seed_entry(&path, &scan_root, cwd, "file"));
          }
        }
      } else {
        let path_kind = if resolved.exists() { "file" } else { "missing" };
        if resolved.exists() {
          changed_inputs.push(resolved.clone());
          graph_inputs.insert(resolved.clone());
          for related in collect_related_graph_inputs(&resolved, 3) {
            graph_inputs.insert(related);
          }
        }
        entries.push(build_grounding_seed_entry(
          &resolved, &scan_root, cwd, path_kind,
        ));
      }
    }
  }

  GroundingSeedBuildResult {
    seed: CodingAgentGroundingSeed {
      scan_root: path_to_slash(&scan_root),
      scan_mode,
      parser_owner: "pnix-lsp::TreeSitterManager::parser_receipt_for_path",
      entries,
    },
    changed_inputs,
    graph_inputs: graph_inputs.into_iter().collect(),
  }
}

fn build_coding_agent_context_pack(
  workspace: &CodingAgentWorkspaceSnapshot,
  grounding_seed: &CodingAgentGroundingSeed,
  repo_graph_seed: &CodingAgentRepoGraphSeed,
  manual_evidence_seed: &CodingAgentManualEvidenceSeed,
  attached_pack_seed: &CodingAgentAttachedPackSeed,
) -> CodingAgentContextPackSurface {
  let context_pack_ref = make_context_pack_ref(
    workspace,
    grounding_seed,
    repo_graph_seed,
    manual_evidence_seed,
    attached_pack_seed,
  );
  let mut forbidden_effects = vec!["actual-write:forbidden-before-patch-proposal".to_string()];
  if workspace.approved_commands.is_empty() {
    forbidden_effects.push("command-execution:forbidden-until-approved-command".to_string());
  }
  for path in &workspace.forbidden_paths {
    forbidden_effects.push(format!("forbidden-path:{}", path));
  }

  CodingAgentContextPackSurface {
    artifact_family: "coding.context-pack",
    phase: "CAX.1b-partial",
    context_pack_ref,
    pack_owner: "pnix-executor-graph::coding-agent::context-pack",
    close_status: "bounded-read-only-pack",
    section_family: vec![
      CodingAgentContextPackSection {
        section_family: "workspace-snapshot",
        item_count: 1,
        provenance_ref: "coding.request.workspace".to_string(),
      },
      CodingAgentContextPackSection {
        section_family: "grounding-seed",
        item_count: grounding_seed.entries.len(),
        provenance_ref: grounding_seed.parser_owner.to_string(),
      },
      CodingAgentContextPackSection {
        section_family: "repo-graph-seed",
        item_count: repo_graph_seed.files.len() + repo_graph_seed.project_reference_edges.len(),
        provenance_ref: repo_graph_seed.graph_owner.to_string(),
      },
      CodingAgentContextPackSection {
        section_family: "manual-evidence-seed",
        item_count: manual_evidence_seed.hits.len()
          + manual_evidence_seed.uncertainty_receipts.len(),
        provenance_ref: manual_evidence_seed.join_owner.to_string(),
      },
      CodingAgentContextPackSection {
        section_family: "attached-pack-seed",
        item_count: attached_pack_seed.total_entry_count,
        provenance_ref: attached_pack_seed.attach_owner.to_string(),
      },
    ],
    target_paths: workspace.target_paths.clone(),
    forbidden_effects,
  }
}

fn collect_repo_root_seed_paths(scan_root: &Path, max_entries: usize) -> BTreeSet<PathBuf> {
  let mut discovered = BTreeSet::new();
  let preferred_roots = [
    scan_root.join("crates"),
    scan_root.join("src"),
    scan_root.join("tests"),
    scan_root.join("docs"),
    scan_root.join("scripts"),
  ];

  for root in preferred_roots {
    if discovered.len() >= max_entries {
      break;
    }
    if root.exists() {
      collect_supported_scan_paths(&root, 2, max_entries, &mut discovered);
    }
  }

  if discovered.is_empty() {
    collect_supported_scan_paths(scan_root, 1, max_entries, &mut discovered);
  }

  discovered
}

fn collect_related_graph_inputs(target_file: &Path, max_related: usize) -> BTreeSet<PathBuf> {
  let Some(parent) = target_file.parent() else {
    return BTreeSet::new();
  };
  let target_receipt = TreeSitterManager::parser_receipt_for_path(target_file);
  let target_language = target_receipt.language;

  let mut discovered = BTreeSet::new();
  if let Some(stem) = target_file.file_stem().and_then(|stem| stem.to_str()) {
    let module_dir = parent.join(stem);
    if module_dir.is_dir() {
      collect_supported_scan_paths(&module_dir, 2, max_related, &mut discovered);
    }
  }
  if discovered.len() < max_related {
    collect_supported_scan_paths(parent, 1, max_related + 1, &mut discovered);
  }
  discovered.remove(target_file);
  if let Some(language) = target_language {
    discovered
      .retain(|path| TreeSitterManager::parser_receipt_for_path(path).language == Some(language));
  }
  discovered.into_iter().take(max_related).collect()
}

fn resolve_grounding_target(cwd: &Path, scan_root: &Path, target: &Path) -> PathBuf {
  if target.is_absolute() {
    return target.to_path_buf();
  }

  let repo_relative = scan_root.join(target);
  if repo_relative.exists() {
    return repo_relative;
  }

  cwd.join(target)
}

fn build_grounding_seed_entry(
  path: &Path,
  scan_root: &Path,
  cwd: &Path,
  path_kind: &'static str,
) -> CodingAgentGroundingSeedEntry {
  let receipt = TreeSitterManager::parser_receipt_for_path(path);
  let display_path = display_grounding_path(path, scan_root, cwd);
  let language = receipt
    .language
    .map(|lang| lang.as_str().to_string())
    .unwrap_or_else(|| "unknown".to_string());

  CodingAgentGroundingSeedEntry {
    path: display_path.clone(),
    path_kind,
    language,
    parser_backend: receipt.parser_backend.as_str().to_string(),
    parser_capability: receipt.parser_capability.as_str().to_string(),
    provenance_ref: format!("{}#{}", receipt.provenance_ref, display_path),
  }
}

fn build_directory_grounding_seed_entry(
  path: &Path,
  scan_root: &Path,
  cwd: &Path,
) -> CodingAgentGroundingSeedEntry {
  let display_path = display_grounding_path(path, scan_root, cwd);
  CodingAgentGroundingSeedEntry {
    path: display_path.clone(),
    path_kind: "directory",
    language: "unknown".to_string(),
    parser_backend: "workspace-scan:DirectoryTarget".to_string(),
    parser_capability: "directory-scan-required".to_string(),
    provenance_ref: format!(
      "coding-agent::build_coding_agent_grounding_seed#{}",
      display_path
    ),
  }
}

fn collect_supported_scan_paths(
  root: &Path,
  depth_remaining: usize,
  max_entries: usize,
  out: &mut BTreeSet<PathBuf>,
) {
  if out.len() >= max_entries || depth_remaining == 0 {
    return;
  }
  let Ok(read_dir) = fs::read_dir(root) else {
    return;
  };

  let mut children = read_dir.filter_map(|entry| entry.ok()).collect::<Vec<_>>();
  children.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

  for entry in children {
    if out.len() >= max_entries {
      break;
    }
    let path = entry.path();
    if path.is_dir() {
      if is_ignored_grounding_dir(&path) {
        continue;
      }
      collect_supported_scan_paths(&path, depth_remaining.saturating_sub(1), max_entries, out);
      continue;
    }
    let receipt = TreeSitterManager::parser_receipt_for_path(&path);
    if receipt.language.is_some() {
      out.insert(path);
    }
  }
}

fn is_ignored_grounding_dir(path: &Path) -> bool {
  matches!(
    path.file_name().and_then(|name| name.to_str()),
    Some(".git" | "target" | "node_modules" | "dist" | "result")
  )
}

fn display_grounding_path(path: &Path, scan_root: &Path, cwd: &Path) -> String {
  if let Ok(relative) = path.strip_prefix(scan_root) {
    let text = path_to_slash(relative);
    if text.is_empty() {
      ".".to_string()
    } else {
      text
    }
  } else if let Ok(relative) = path.strip_prefix(cwd) {
    path_to_slash(relative)
  } else {
    path_to_slash(path)
  }
}

fn build_coding_agent_repo_graph_seed(
  cwd: &Path,
  repo_root: Option<&str>,
  changed_inputs: &[PathBuf],
  graph_inputs: &[PathBuf],
) -> CodingAgentRepoGraphSeed {
  let scan_root = repo_root
    .map(PathBuf::from)
    .unwrap_or_else(|| cwd.to_path_buf());
  let mut builder = CpgBuilder::new();
  let seto_enrichment_state = if LspSetoIndex::load().is_some() {
    "seto-loaded-enrichment"
  } else {
    "seto-disabled-optional"
  };

  let mut ordered_inputs = Vec::new();
  let mut seen_inputs = BTreeSet::new();
  for path in changed_inputs {
    if seen_inputs.insert(path.clone()) {
      ordered_inputs.push(path.clone());
    }
  }
  for path in graph_inputs {
    if seen_inputs.insert(path.clone()) {
      ordered_inputs.push(path.clone());
    }
  }

  let mut saw_non_emergency_parser = false;
  let project_inputs = ordered_inputs
    .iter()
    .filter_map(|path| {
      let receipt = TreeSitterManager::parser_receipt_for_path(path);
      let language = receipt.language?;
      if receipt.parser_capability.as_str() != "emergency-compatibility-only" {
        saw_non_emergency_parser = true;
      }
      let code = fs::read_to_string(path).ok()?;
      let display_path = display_grounding_path(path, &scan_root, cwd);
      Some((
        display_path.clone(),
        receipt.parser_backend.as_str().to_string(),
        receipt.parser_capability.as_str().to_string(),
        RepoProjectGraphInput {
          file_key: display_path,
          code,
          language,
        },
      ))
    })
    .take(4)
    .collect::<Vec<_>>();

  let receipt_map = project_inputs
    .iter()
    .map(|(display_path, backend, capability, _)| {
      (display_path.clone(), (backend.clone(), capability.clone()))
    })
    .collect::<BTreeMap<_, _>>();
  let summary = builder.summarize_project_graph(
    &project_inputs
      .iter()
      .map(|(_, _, _, input)| input.clone())
      .collect::<Vec<_>>(),
  );
  let pnix_lsp::RepoProjectGraphSummary {
    files: summary_files,
    cross_file_reference_edges: summary_project_reference_edges,
  } = summary;

  let files = summary_files
    .into_iter()
    .map(|file| {
      let (parser_backend, parser_capability) =
        receipt_map.get(&file.file_key).cloned().unwrap_or_else(|| {
          (
            "pnix-lsp:UnsupportedLanguage".to_string(),
            "unsupported-language".to_string(),
          )
        });
      let file_anchor = format!("{}#file", file.file_key);
      let symbol_nodes = file
        .file_summary
        .symbols
        .into_iter()
        .take(24)
        .map(|symbol| {
          let symbol_id = format!("{}#symbol:{}", file.file_key, symbol.ast_node_id);
          let definition_anchor = format!(
            "{}#byte:{}-{}",
            file.file_key, symbol.start_byte, symbol.end_byte
          );
          let use_anchors = symbol
            .use_node_ids
            .into_iter()
            .map(|use_id| format!("{}#ast:{}", file.file_key, use_id))
            .collect();
          CodingAgentRepoGraphSymbol {
            symbol_id,
            name: symbol.name,
            kind: symbol.kind,
            definition_anchor,
            use_anchors,
          }
        })
        .collect::<Vec<_>>();
      let reference_edges = file
        .file_summary
        .reference_edges
        .into_iter()
        .take(48)
        .map(|edge| CodingAgentRepoGraphReference {
          from_symbol_id: format!("{}#symbol:{}", file.file_key, edge.from_symbol_ast_id),
          to_anchor: format!("{}#ast:{}", file.file_key, edge.to_ast_node_id),
          edge_kind: edge.edge_kind.to_string(),
        })
        .collect::<Vec<_>>();
      let runtime_entrypoints = symbol_nodes
        .iter()
        .filter(|symbol| is_runtime_entrypoint_name(symbol.name.as_str()))
        .map(|symbol| format!("runtime-entrypoint:{}", symbol.name))
        .collect::<Vec<_>>();
      let test_targets = build_test_targets(file.file_key.as_str(), &symbol_nodes);

      CodingAgentRepoGraphFile {
        file_anchor,
        language: file.file_summary.language.as_str().to_string(),
        parser_backend,
        parser_capability,
        symbol_nodes,
        reference_edges,
        test_targets,
        runtime_entrypoints,
        provenance_ref: format!(
          "pnix-lsp::CpgBuilder::summarize_project_graph#{}",
          file.file_key
        ),
      }
    })
    .collect::<Vec<_>>();

  let project_reference_edges = summary_project_reference_edges
    .into_iter()
    .take(32)
    .map(|edge| CodingAgentRepoGraphProjectReference {
      from_file_anchor: format!("{}#file", edge.from_file_key),
      to_file_anchor: format!("{}#file", edge.to_file_key),
      to_symbol_id: format!("{}#symbol:{}", edge.to_file_key, edge.to_symbol_ast_id),
      via_term: edge.via_term.clone(),
      edge_kind: edge.edge_kind.to_string(),
      provenance_ref: format!(
        "pnix-lsp::CpgBuilder::summarize_project_graph#{}=>{}:{}",
        edge.from_file_key, edge.to_file_key, edge.via_term
      ),
    })
    .collect::<Vec<_>>();

  let refresh_batch = files
    .iter()
    .map(|file| file.file_anchor.trim_end_matches("#file").to_string())
    .collect::<Vec<_>>();
  let changed_files = changed_inputs
    .iter()
    .map(|path| display_grounding_path(path, &scan_root, cwd))
    .filter(|path| refresh_batch.iter().any(|batch| batch == path))
    .collect::<Vec<_>>();
  let bundle_scope = if files.len() > 1 {
    "multi-file-bounded-project-summary"
  } else {
    "single-file-bounded-project-summary"
  };
  let graph_capability = if files.len() > 1 {
    "multi-file-bounded-project-summary"
  } else {
    "same-file-shallow-summary-only"
  };
  let project_graph_status = if files.len() > 1 {
    if saw_non_emergency_parser {
      "multi-file-bounded-no-project-cache-yet"
    } else {
      "multi-file-bounded-fallback-parser-only"
    }
  } else if saw_non_emergency_parser {
    "project-graph-open-no-multi-file-closure-yet"
  } else {
    "fallback-parser-not-project-graph-primary"
  };
  let refresh_mode = if changed_files.is_empty() {
    "repo-bounded-refresh"
  } else if refresh_batch.len() > changed_files.len() {
    "changed-file-plus-related-bounded-refresh"
  } else {
    "changed-file-only-refresh"
  };

  CodingAgentRepoGraphSeed {
    bundle_scope,
    graph_owner: "pnix-lsp::CpgBuilder::summarize_project_graph",
    graph_capability,
    project_graph_status,
    seto_enrichment_state,
    project_reference_edges,
    incremental_refresh: CodingAgentRepoGraphIncrementalRefresh {
      refresh_owner: "pnix coding-agent repo_graph_seed",
      refresh_mode,
      changed_files,
      refresh_batch,
    },
    files,
  }
}

fn build_coding_agent_manual_evidence_seed(
  repo_graph_seed: &CodingAgentRepoGraphSeed,
) -> CodingAgentManualEvidenceSeed {
  let mut hits = Vec::new();
  let mut uncertainty_receipts = vec![format!(
    "repo-graph-status:{}",
    repo_graph_seed.project_graph_status
  )];
  let mut language_hints = BTreeSet::new();
  let mut queried_terms = BTreeSet::new();
  let changed_files = repo_graph_seed
    .incremental_refresh
    .changed_files
    .iter()
    .map(|path| format!("{}#file", path))
    .collect::<BTreeSet<_>>();

  for file in repo_graph_seed
    .files
    .iter()
    .filter(|file| changed_files.is_empty() || changed_files.contains(&file.file_anchor))
  {
    language_hints.insert(file.language.clone());

    for symbol in file
      .symbol_nodes
      .iter()
      .filter(|symbol| should_query_manual_evidence_symbol(symbol))
      .take(8)
    {
      let Some(term) = normalize_manual_evidence_term(symbol.name.as_str()) else {
        continue;
      };
      if !queried_terms.insert((file.language.clone(), term.clone())) {
        continue;
      }

      let project_refs = vec![file.file_anchor.clone(), symbol.definition_anchor.clone()];
      let joined = query_joined_docset_evidence(
        Some(file.language.as_str()),
        term.as_str(),
        Some(symbol.name.as_str()),
        Some(file.file_anchor.as_str()),
        &project_refs,
      );

      if joined.is_empty() {
        if uncertainty_receipts.len() < 10 {
          uncertainty_receipts.push(format!("manual-query-miss:{}:{}", file.language, term));
        }
        continue;
      }

      for entry in joined.into_iter().take(3) {
        hits.push(entry);
        if hits.len() >= 8 {
          break;
        }
      }

      if hits.len() >= 8 {
        break;
      }
    }

    if hits.len() >= 8 {
      break;
    }
  }

  if repo_graph_seed.files.is_empty() {
    uncertainty_receipts.push("manual-evidence-skipped:no-repo-graph-files".to_string());
  } else if queried_terms.is_empty() {
    uncertainty_receipts.push("manual-evidence-skipped:no-queryable-symbols".to_string());
  } else if hits.is_empty() {
    uncertainty_receipts.push("manual-evidence-skipped:no-joined-hits".to_string());
  }

  CodingAgentManualEvidenceSeed {
    join_owner: "doghouse-core::docset_query::query_joined_docset_evidence",
    join_policy: "manual-hit-never-justifies-patch-without-file-symbol-project-join",
    language_hints: language_hints.into_iter().collect(),
    hits,
    uncertainty_receipts,
  }
}

fn build_coding_agent_attached_pack_seed(
  cwd: &Path,
  repo_root: Option<&str>,
  project_pack_root_inputs: &[PathBuf],
  history_pack_root_inputs: &[PathBuf],
) -> CodingAgentAttachedPackSeed {
  let project_pack_roots = project_pack_root_inputs
    .iter()
    .map(|root| scan_coding_agent_attached_pack_root(cwd, repo_root, root, "project-pack"))
    .collect::<Vec<_>>();
  let history_pack_roots = history_pack_root_inputs
    .iter()
    .map(|root| scan_coding_agent_attached_pack_root(cwd, repo_root, root, "history-pack"))
    .collect::<Vec<_>>();
  let total_entry_count = project_pack_roots
    .iter()
    .chain(history_pack_roots.iter())
    .map(|root| root.entry_count)
    .sum();

  CodingAgentAttachedPackSeed {
    attach_owner: "pnix-executor-graph::coding-agent::attached-pack-seed",
    project_pack_roots,
    history_pack_roots,
    total_entry_count,
  }
}

fn scan_coding_agent_attached_pack_root(
  cwd: &Path,
  repo_root: Option<&str>,
  root: &Path,
  pack_kind: &'static str,
) -> CodingAgentAttachedPackRoot {
  let resolved_root = resolve_coding_agent_pack_root(cwd, repo_root, root);
  let root_label = path_to_slash(&resolved_root);
  if !resolved_root.exists() {
    return CodingAgentAttachedPackRoot {
      root: root_label,
      pack_kind,
      status: "missing-root",
      entry_count: 0,
      entries: Vec::new(),
    };
  }

  if resolved_root.is_file() {
    let entry = build_coding_agent_attached_pack_entry(&resolved_root);
    return CodingAgentAttachedPackRoot {
      root: root_label,
      pack_kind,
      status: "file-root",
      entry_count: 1,
      entries: vec![entry],
    };
  }

  let mut entries = Vec::new();
  let mut total_count = 0usize;
  if let Ok(read_dir) = fs::read_dir(&resolved_root) {
    let mut children = read_dir
      .flatten()
      .map(|entry| entry.path())
      .collect::<Vec<_>>();
    children.sort();
    total_count = children.len();
    for child in children.into_iter().take(8) {
      entries.push(build_coding_agent_attached_pack_entry(&child));
    }
  }

  CodingAgentAttachedPackRoot {
    root: root_label,
    pack_kind,
    status: "attached-read-only",
    entry_count: total_count,
    entries,
  }
}

fn resolve_coding_agent_pack_root(cwd: &Path, repo_root: Option<&str>, root: &Path) -> PathBuf {
  if root.is_absolute() {
    root.to_path_buf()
  } else if let Some(repo_root) = repo_root {
    PathBuf::from(repo_root).join(root)
  } else {
    cwd.join(root)
  }
}

fn build_coding_agent_attached_pack_entry(path: &Path) -> CodingAgentAttachedPackEntry {
  CodingAgentAttachedPackEntry {
    entry_ref: path_to_slash(path),
    entry_kind: classify_coding_agent_attached_pack_entry_kind(path),
    provenance_ref: format!("pack-entry:{}", path_to_slash(path)),
  }
}

fn classify_coding_agent_attached_pack_entry_kind(path: &Path) -> &'static str {
  if path.is_dir() {
    return "directory-pack";
  }
  match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
    "json" => "json-pack",
    "toml" => "toml-pack",
    "px" => "px-pack",
    "org" => "org-pack",
    "md" => "markdown-pack",
    "yaml" | "yml" => "yaml-pack",
    _ => "generic-pack-file",
  }
}

fn build_coding_agent_language_profile(
  cwd: &Path,
  repo_root: Option<&str>,
  workspace: &CodingAgentWorkspaceSnapshot,
  grounding_seed: &CodingAgentGroundingSeed,
  repo_graph_seed: &CodingAgentRepoGraphSeed,
) -> CodingAgentLanguageProfileSurface {
  let target_paths = if workspace.target_paths.is_empty() {
    grounding_seed
      .entries
      .iter()
      .filter(|entry| entry.path_kind == "file")
      .map(|entry| entry.path.clone())
      .collect::<Vec<_>>()
  } else {
    workspace.target_paths.clone()
  };

  let mut semantic_records = Vec::new();
  let mut effect_records = Vec::new();
  let mut verify_targets = Vec::new();
  let mut diagnostic_records = Vec::new();
  let mut failure_pattern_matches = Vec::new();
  let mut context_demands = Vec::new();
  let mut unsupported_targets = Vec::new();
  let mut adapter_counts: BTreeMap<String, usize> = BTreeMap::new();

  for target_path in target_paths {
    let resolved = resolve_language_profile_target(cwd, repo_root, target_path.as_str());
    let receipt = TreeSitterManager::parser_receipt_for_path(&resolved);
    let Some(language) = detect_coding_language_profile_language(&resolved, &receipt) else {
      push_language_profile_diagnostic_bridge(
        &mut diagnostic_records,
        &mut failure_pattern_matches,
        &mut context_demands,
        target_path.as_str(),
        "unknown",
        "no CAX.5 adapter skeleton is registered for this target",
      );
      unsupported_targets.push(CodingAgentUnsupportedLanguageTarget {
        target_path,
        detected_language: "unknown".to_string(),
        status: "unsupported",
        reason: "no CAX.5 adapter skeleton is registered for this target",
      });
      continue;
    };

    if !is_language_profile_record_producer(language.as_str()) {
      push_language_profile_diagnostic_bridge(
        &mut diagnostic_records,
        &mut failure_pattern_matches,
        &mut context_demands,
        target_path.as_str(),
        language.as_str(),
        "CAX.5b holds planned adapters until diagnostic/context-demand bridge is reviewed",
      );
      unsupported_targets.push(CodingAgentUnsupportedLanguageTarget {
        target_path,
        detected_language: language,
        status: "adapter-planned",
        reason: "CAX.5a only opens pnix and rust record producers",
      });
      continue;
    }

    let parser_backend = language_profile_parser_backend(language.as_str(), &receipt);
    let parser_capability = language_profile_parser_capability(language.as_str(), &receipt);
    let adapter_owner = format!("pnix-executor-graph::coding-agent::language-adapter::{language}");
    let provenance_refs =
      build_language_profile_provenance_refs(target_path.as_str(), language.as_str());
    let symbol_refs =
      build_language_profile_symbol_refs(target_path.as_str(), language.as_str(), repo_graph_seed);
    let contract_refs =
      build_language_profile_contract_refs(target_path.as_str(), language.as_str(), &symbol_refs);

    semantic_records.push(CodingAgentSemanticRecordSurface {
      artifact_family: "pnix.semantic-record",
      record_ref: make_language_profile_record_ref(
        "semantic",
        language.as_str(),
        target_path.as_str(),
      ),
      language: language.clone(),
      target_path: target_path.clone(),
      adapter_owner: adapter_owner.clone(),
      parser_backend,
      parser_capability,
      meaning_class: classify_language_profile_meaning_class(language.as_str()),
      symbol_refs,
      contract_refs,
      provenance_refs: provenance_refs.clone(),
      judgement_boundary: "adapter-record-only-not-judgement",
      record_status: language_profile_record_status(language.as_str()),
    });
    effect_records.push(CodingAgentEffectRecordSurface {
      artifact_family: "pnix.effect-record",
      record_ref: make_language_profile_record_ref(
        "effect",
        language.as_str(),
        target_path.as_str(),
      ),
      language: language.clone(),
      target_path: target_path.clone(),
      adapter_owner: adapter_owner.clone(),
      effect_classes: build_language_profile_effect_classes(
        target_path.as_str(),
        language.as_str(),
      ),
      mutation_boundary: "patch-proposal-required-before-write",
      rollback_expectation: "file-write-only-rollbackable-after-explicit-inverse-diff",
      provenance_refs: provenance_refs.clone(),
      record_status: language_profile_record_status(language.as_str()),
    });
    verify_targets.push(CodingAgentVerifyTargetSurface {
      artifact_family: "pnix.verify-target",
      target_ref: make_language_profile_record_ref(
        "verify-target",
        language.as_str(),
        target_path.as_str(),
      ),
      language: language.clone(),
      target_path: target_path.clone(),
      verify_family: classify_language_profile_verify_family(language.as_str()),
      command_candidates: build_language_profile_verify_commands(
        target_path.as_str(),
        language.as_str(),
      ),
      required_signals: build_language_profile_required_signals(language.as_str()),
      permission_status: "candidate-only-requires-approved-command",
      judgement_boundary: "verify-target-is-not-proof-until-execution-result",
    });

    *adapter_counts.entry(language).or_insert(0) += 1;
  }

  CodingAgentLanguageProfileSurface {
    artifact_family: "coding.language-profile",
    phase: "CAX.5a",
    profile_owner: "pnix-executor-graph::coding-agent::language-profile",
    adapter_boundary: "language-adapter-produces-records-not-judgement",
    supported_adapters: adapter_counts
      .into_iter()
      .map(
        |(language, target_count)| CodingAgentLanguageAdapterSurface {
          adapter_owner: format!("pnix-executor-graph::coding-agent::language-adapter::{language}"),
          adapter_status: language_profile_adapter_status(language.as_str()),
          language,
          target_count,
          record_families: vec![
            "pnix.semantic-record",
            "pnix.effect-record",
            "pnix.verify-target",
          ],
        },
      )
      .collect(),
    semantic_records,
    effect_records,
    verify_targets,
    diagnostic_records,
    failure_pattern_matches,
    context_demands,
    unsupported_targets,
    close_status: "record-producer-only-no-promotion",
  }
}

fn push_language_profile_diagnostic_bridge(
  diagnostic_records: &mut Vec<CodingAgentDiagnosticRecordSurface>,
  failure_pattern_matches: &mut Vec<CodingAgentFailurePatternMatchSurface>,
  context_demands: &mut Vec<CodingAgentContextDemandSurface>,
  target_path: &str,
  language: &str,
  message: &str,
) {
  let diagnostic_ref = make_language_profile_record_ref("diagnostic", language, target_path);
  let context_demand_ref =
    make_language_profile_record_ref("context-demand", language, target_path);
  diagnostic_records.push(CodingAgentDiagnosticRecordSurface {
    artifact_family: "pnix.diagnostic-record",
    diagnostic_ref: diagnostic_ref.clone(),
    language: language.to_string(),
    target_path: target_path.to_string(),
    diagnostic_family: "language-adapter-not-ready",
    severity: "hold",
    message: message.to_string(),
    provenance_refs: vec![
      format!("language-adapter:{language}:{target_path}"),
      "CAX.5b:diagnostic-context-demand-bridge".to_string(),
    ],
    record_status: "candidate",
  });
  failure_pattern_matches.push(CodingAgentFailurePatternMatchSurface {
    artifact_family: "coding.failure-pattern-match",
    match_ref: make_language_profile_record_ref("failure-pattern-match", language, target_path),
    diagnostic_ref,
    pattern_key: "missing-or-planned-language-adapter",
    confidence: 1.0,
    context_demand_ref: context_demand_ref.clone(),
    promotion_boundary: "candidate-only-not-judgement",
  });
  context_demands.push(CodingAgentContextDemandSurface {
    artifact_family: "coding.context-demand",
    context_demand_ref,
    language: language.to_string(),
    target_path: target_path.to_string(),
    demand_family: "language-adapter-profile-required",
    required_evidence: vec![
      "parser-capability-receipt".to_string(),
      "language-specific-verify-target-law".to_string(),
      "failure-ontology-mapping".to_string(),
    ],
    request_boundary: "request-more-context-before-patch-proposal",
  });
}

fn resolve_language_profile_target(
  cwd: &Path,
  repo_root: Option<&str>,
  target_path: &str,
) -> PathBuf {
  let path = Path::new(target_path);
  if path.is_absolute() {
    path.to_path_buf()
  } else if let Some(repo_root) = repo_root {
    PathBuf::from(repo_root).join(path)
  } else {
    cwd.join(path)
  }
}

fn detect_coding_language_profile_language(
  path: &Path,
  receipt: &pnix_lsp::tree_sitter_bridge::ParserCapabilityReceipt,
) -> Option<String> {
  if let Some(language) = detect_language_profile_extension_language(path) {
    return Some(language.to_string());
  }
  receipt
    .language
    .map(|language| language.as_str().to_string())
}

fn detect_language_profile_extension_language(path: &Path) -> Option<&'static str> {
  match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
    "px" => Some("pnix"),
    "nix" => Some("nix"),
    "clj" | "cljs" | "cljc" | "bb" => Some("clojure"),
    _ => None,
  }
}

fn is_language_profile_record_producer(language: &str) -> bool {
  matches!(
    language,
    "pnix" | "rust" | "python" | "typescript" | "javascript" | "nix" | "clojure"
  )
}

fn language_profile_adapter_status(language: &str) -> &'static str {
  match language {
    "pnix" | "rust" => "supported-skeleton",
    "python" | "typescript" | "javascript" | "nix" | "clojure" => "planned-record-producer",
    _ => "unsupported",
  }
}

fn language_profile_record_status(language: &str) -> &'static str {
  match language_profile_adapter_status(language) {
    "supported-skeleton" => "candidate",
    "planned-record-producer" => "candidate-planned-adapter",
    _ => "unsupported",
  }
}

fn language_profile_parser_backend(
  language: &str,
  receipt: &pnix_lsp::tree_sitter_bridge::ParserCapabilityReceipt,
) -> String {
  match language {
    "pnix" => "pnix-core::lang::pnix".to_string(),
    "rust" => receipt.parser_backend.as_str().to_string(),
    "python" | "typescript" | "javascript" | "nix" | "clojure" => {
      format!("pnix-executor-graph::coding-agent::language-adapter::{language}::record-producer")
    }
    _ => receipt.parser_backend.as_str().to_string(),
  }
}

fn language_profile_parser_capability(
  language: &str,
  receipt: &pnix_lsp::tree_sitter_bridge::ParserCapabilityReceipt,
) -> String {
  match language {
    "pnix" => "pnix-source-skeleton-parser-required".to_string(),
    "rust" => receipt.parser_capability.as_str().to_string(),
    "python" | "typescript" | "javascript" | "nix" | "clojure" => {
      "planned-record-producer-only-no-parser-ownership".to_string()
    }
    _ => receipt.parser_capability.as_str().to_string(),
  }
}

fn build_language_profile_provenance_refs(target_path: &str, language: &str) -> Vec<String> {
  vec![
    format!("language-adapter:{language}:{target_path}"),
    format!("file-anchor:{target_path}#file"),
  ]
}

fn build_language_profile_symbol_refs(
  target_path: &str,
  language: &str,
  repo_graph_seed: &CodingAgentRepoGraphSeed,
) -> Vec<String> {
  if language == "pnix" {
    return vec![format!("{target_path}#pnix-source")];
  }
  if language == "rust" {
    return repo_graph_seed
      .files
      .iter()
      .find(|file| file.file_anchor.trim_end_matches("#file") == target_path)
      .map(|file| {
        file
          .symbol_nodes
          .iter()
          .take(8)
          .map(|symbol| symbol.symbol_id.clone())
          .collect::<Vec<_>>()
      })
      .filter(|symbols| !symbols.is_empty())
      .unwrap_or_else(|| vec![format!("{target_path}#file")]);
  }
  vec![format!("{target_path}#language-profile:{language}")]
}

fn build_language_profile_contract_refs(
  target_path: &str,
  language: &str,
  symbol_refs: &[String],
) -> Vec<String> {
  let mut refs = vec![
    format!("contract:{language}:patch-proposal-before-write:{target_path}"),
    format!("contract:{language}:verify-target-before-promotion:{target_path}"),
  ];
  refs.extend(
    symbol_refs
      .iter()
      .take(4)
      .map(|symbol| format!("contract-symbol:{symbol}")),
  );
  refs
}

fn classify_language_profile_meaning_class(language: &str) -> &'static str {
  match language {
    "pnix" => "pnix-source-meaning",
    "rust" => "rust-module-or-item-meaning",
    "python" => "python-module-meaning",
    "typescript" => "typescript-module-meaning",
    "javascript" => "javascript-module-meaning",
    "nix" => "nix-expression-or-module-meaning",
    "clojure" => "clojure-namespace-meaning",
    _ => "unsupported-language-meaning",
  }
}

fn build_language_profile_effect_classes(target_path: &str, language: &str) -> Vec<String> {
  let mut effects = vec!["workspace-file-write:intent-only".to_string()];
  match language {
    "pnix" => {
      effects.push("pnix-source-meaning-change:intent-only".to_string());
      effects.push("pnix-lowering-impact:verify-required".to_string());
    }
    "rust" => {
      effects.push("rust-source-code-change:intent-only".to_string());
      effects.push("compile-impact:verify-required".to_string());
    }
    "python" => {
      effects.push("python-source-code-change:intent-only".to_string());
      effects.push("python-syntax-or-test-impact:verify-required".to_string());
    }
    "typescript" => {
      effects.push("typescript-source-code-change:intent-only".to_string());
      effects.push("typecheck-impact:verify-required".to_string());
    }
    "javascript" => {
      effects.push("javascript-source-code-change:intent-only".to_string());
      effects.push("node-runtime-impact:verify-required".to_string());
    }
    "nix" => {
      effects.push("nix-expression-change:intent-only".to_string());
      effects.push("nix-eval-or-build-env-impact:verify-required".to_string());
    }
    "clojure" => {
      effects.push("clojure-source-code-change:intent-only".to_string());
      effects.push("jvm-or-clojure-runtime-impact:verify-required".to_string());
    }
    _ => {
      effects.push("unsupported-language-change:hold".to_string());
    }
  }
  if is_test_path(target_path) {
    effects.push("test-file-write:intent-only".to_string());
  }
  effects.sort();
  effects.dedup();
  effects
}

fn classify_language_profile_verify_family(language: &str) -> &'static str {
  match language {
    "pnix" => "pnix-parse-and-lowering-check",
    "rust" => "rust-compile-test-check",
    "python" => "python-syntax-test-check",
    "typescript" => "typescript-typecheck-test-check",
    "javascript" => "javascript-syntax-test-check",
    "nix" => "nix-parse-eval-check",
    "clojure" => "clojure-load-test-check",
    _ => "manual-verification-required",
  }
}

fn build_language_profile_verify_commands(target_path: &str, language: &str) -> Vec<String> {
  match language {
    "pnix" => vec![
      "cargo test -p pnix-core parse_pnix --lib".to_string(),
      format!("pnix parse {target_path}"),
    ],
    "rust" => build_rust_language_profile_verify_commands(target_path),
    "python" => vec![
      format!("python -m py_compile {target_path}"),
      "python -m pytest".to_string(),
    ],
    "typescript" => vec!["npx tsc --noEmit".to_string(), "npm test".to_string()],
    "javascript" => vec![
      format!("node --check {target_path}"),
      "npm test".to_string(),
    ],
    "nix" => vec![
      format!("nix-instantiate --parse {target_path}"),
      "nix flake check".to_string(),
    ],
    "clojure" => vec![
      format!("clojure -M -e \"(load-file \\\"{target_path}\\\")\""),
      "clojure -M:test".to_string(),
    ],
    _ => Vec::new(),
  }
}

fn build_rust_language_profile_verify_commands(target_path: &str) -> Vec<String> {
  if let Some(crate_name) = rust_crate_name_from_target_path(target_path) {
    return vec![
      format!("cargo check -p {crate_name}"),
      format!("cargo test -p {crate_name} --lib"),
    ];
  }
  if target_path.starts_with("puck/") {
    return vec!["cargo test -p pnixc-meta puck_".to_string()];
  }
  vec!["cargo check".to_string()]
}

fn rust_crate_name_from_target_path(target_path: &str) -> Option<String> {
  let mut parts = target_path.split('/');
  if parts.next()? != "crates" {
    return None;
  }
  parts.next().map(|name| name.to_string())
}

fn build_language_profile_required_signals(language: &str) -> Vec<String> {
  match language {
    "pnix" => vec![
      "parse-success".to_string(),
      "lowering-contract-review".to_string(),
      "semantic-record-reviewed-by-judgement".to_string(),
    ],
    "rust" => vec![
      "cargo-check-success".to_string(),
      "targeted-test-success-or-explicit-skip".to_string(),
      "semantic-record-reviewed-by-judgement".to_string(),
    ],
    "python" => vec![
      "python-syntax-success".to_string(),
      "pytest-success-or-explicit-skip".to_string(),
      "semantic-record-reviewed-by-judgement".to_string(),
    ],
    "typescript" => vec![
      "typescript-typecheck-success".to_string(),
      "npm-test-success-or-explicit-skip".to_string(),
      "semantic-record-reviewed-by-judgement".to_string(),
    ],
    "javascript" => vec![
      "node-syntax-success".to_string(),
      "npm-test-success-or-explicit-skip".to_string(),
      "semantic-record-reviewed-by-judgement".to_string(),
    ],
    "nix" => vec![
      "nix-parse-success".to_string(),
      "nix-eval-or-flake-check-success-or-explicit-skip".to_string(),
      "semantic-record-reviewed-by-judgement".to_string(),
    ],
    "clojure" => vec![
      "clojure-load-success".to_string(),
      "clojure-test-success-or-explicit-skip".to_string(),
      "semantic-record-reviewed-by-judgement".to_string(),
    ],
    _ => vec!["manual-review-required".to_string()],
  }
}

fn normalize_manual_evidence_term(symbol_name: &str) -> Option<String> {
  let trimmed = symbol_name.trim();
  if trimmed.len() < 3 || trimmed.len() > 64 {
    return None;
  }
  if trimmed.contains("::") || trimmed.contains(' ') {
    return None;
  }
  if trimmed
    .chars()
    .all(|ch| ch == '_' || ch.is_ascii_uppercase())
  {
    return None;
  }
  if trimmed
    .chars()
    .all(|ch| ch == '_' || ch.is_ascii_digit() || !ch.is_ascii_alphanumeric())
  {
    return None;
  }
  if trimmed.starts_with("__") {
    return None;
  }
  let lowered = trimmed.to_ascii_lowercase();
  if matches!(
    lowered.as_str(),
    "pub"
      | "args"
      | "arg"
      | "ok"
      | "err"
      | "self"
      | "super"
      | "crate"
      | "mod"
      | "use"
      | "let"
      | "mut"
      | "enum"
      | "struct"
      | "impl"
      | "trait"
      | "type"
      | "result"
      | "string"
      | "path"
      | "file"
      | "value"
      | "request"
      | "response"
      | "plan"
      | "status"
      | "note"
      | "mode"
  ) {
    return None;
  }
  Some(trimmed.to_string())
}

fn should_query_manual_evidence_symbol(symbol: &CodingAgentRepoGraphSymbol) -> bool {
  matches!(
    symbol.kind.as_str(),
    "function_definition" | "call_expression" | "method_call_expression"
  ) && normalize_manual_evidence_term(symbol.name.as_str()).is_some()
}

fn build_test_targets(
  display_path: &str,
  symbol_nodes: &[CodingAgentRepoGraphSymbol],
) -> Vec<String> {
  let mut targets = Vec::new();
  if display_path.contains("/tests/") || display_path.starts_with("tests/") {
    targets.push(format!("cargo-test:file:{}", display_path));
  }
  for symbol in symbol_nodes {
    if symbol.name.starts_with("test_") || symbol.name.ends_with("_test") {
      targets.push(format!("cargo-test:symbol:{}", symbol.name));
    }
  }
  targets
}

fn is_runtime_entrypoint_name(name: &str) -> bool {
  matches!(name, "main" | "run" | "serve" | "start" | "entry")
}

fn build_coding_agent_plan(
  args: &Args,
  request: CodingAgentRequestArtifact,
  request_artifact_ref: Option<String>,
) -> CodingAgentPlanArtifact {
  let current_interpretation = classify_coding_agent_interpretation(
    request.request.as_deref(),
    &request.workspace.target_paths,
  );
  let mut bounded_step_family = vec![CodingAgentPlanStep {
    step_family: "inspect-workspace-snapshot",
    capability_bound: "read-only-inspection",
    summary: "현재 cwd/git/workspace policy를 기반으로 bounded scope를 고정한다.".to_string(),
  }];

  if request.workspace.target_paths.is_empty() {
    bounded_step_family.push(CodingAgentPlanStep {
      step_family: "infer-target-scope",
      capability_bound: "request-bounded-scope-only",
      summary: "명시 target path가 없으므로 request와 policy bits만으로 repo-local 범위를 좁힌다."
        .to_string(),
    });
  } else {
    bounded_step_family.push(CodingAgentPlanStep {
      step_family: "inspect-target-scope",
      capability_bound: "read-only-target-paths",
      summary: format!(
        "명시된 target path {} 를 우선 증거범위로 고정한다.",
        request.workspace.target_paths.join(", ")
      ),
    });
  }

  if request.workspace.approved_commands.is_empty() {
    bounded_step_family.push(CodingAgentPlanStep {
      step_family: "require-verification-contract",
      capability_bound: "fail-closed-no-exec",
      summary: "승인된 command가 없으므로 verify contract를 먼저 선언해야 다음 lane으로 진행한다."
        .to_string(),
    });
  } else {
    bounded_step_family.push(CodingAgentPlanStep {
      step_family: "prepare-approved-verification",
      capability_bound: "approved-command-only",
      summary: format!(
        "승인된 verify command {} 를 bounded verification target으로 예약한다.",
        request.workspace.approved_commands.join(", ")
      ),
    });
  }

  if request.manual_evidence_seed.hits.is_empty() {
    bounded_step_family.push(CodingAgentPlanStep {
      step_family: "record-manual-evidence-uncertainty",
      capability_bound: "joined-manual-evidence-required-before-plan-promotion",
      summary: "manual/docset evidence hit가 없거나 repo graph와 조인되지 않았음을 explicit receipt로 남긴다."
        .to_string(),
    });
  } else {
    bounded_step_family.push(CodingAgentPlanStep {
      step_family: "review-joined-manual-evidence",
      capability_bound: "read-only-manual-evidence",
      summary: format!(
        "joined manual evidence {}건을 file/symbol/project ref와 함께 review한다.",
        request.manual_evidence_seed.hits.len()
      ),
    });
  }

  bounded_step_family.push(CodingAgentPlanStep {
    step_family: "emit-patch-proposal-before-write",
    capability_bound: "proposal-only-no-write",
    summary: "실제 write/apply 전에 patch proposal과 evidence bundle을 먼저 생성한다.".to_string(),
  });

  let expected_verification = if request.workspace.approved_commands.is_empty() {
    vec!["manual-verification-contract-required".to_string()]
  } else {
    request.workspace.approved_commands.clone()
  };
  let failure_policy = if request.workspace.approved_commands.is_empty() {
    "fail-closed-until-verification-contract-is-declared"
  } else {
    "fail-closed-before-patch-apply"
  };
  let note = if args.agent_plan_out.is_some() {
    "bounded plan artifact를 생성했고 patch/apply/verify execution은 아직 열지 않았다."
  } else {
    "stdout에 bounded plan surface만 노출했고 patch/apply/verify execution은 아직 열지 않았다."
  };
  let interpretation_set = build_coding_agent_interpretation_set(&request, &current_interpretation);
  let judgement = build_coding_agent_judgement(&request, &interpretation_set);
  let execution_plan = build_coding_agent_execution_plan(
    &request,
    request_artifact_ref.as_deref(),
    &bounded_step_family,
    &expected_verification,
    failure_policy,
  );

  CodingAgentPlanArtifact {
    artifact_family: "coding.plan",
    phase: "CAX.1c",
    surface: "pnix coding-agent",
    verb: "plan",
    planned_at_ms: current_time_ms(),
    request_artifact_ref,
    current_interpretation,
    interpretation_set,
    judgement,
    execution_plan,
    bounded_step_family,
    expected_verification,
    failure_policy,
    status: CodingAgentStatusSurface {
      progress_status: "계획제안완료",
      result_status: "부분완료",
      note: note.to_string(),
    },
    request,
  }
}

fn build_coding_agent_interpretation_set(
  request: &CodingAgentRequestArtifact,
  current_interpretation: &str,
) -> CodingAgentInterpretationSetSurface {
  let mut alternatives = vec![current_interpretation.to_string()];
  if request.workspace.target_paths.is_empty() {
    alternatives
      .push("repo-local scope requires explicit target narrowing before patch".to_string());
  } else {
    alternatives.push("target scope may still require dependency/context expansion".to_string());
  }
  if request.workspace.approved_commands.is_empty() {
    alternatives.push("verification contract missing; hold before mutation".to_string());
  }
  alternatives.sort();
  alternatives.dedup();

  CodingAgentInterpretationSetSurface {
    artifact_family: "coding.interpretation-set",
    phase: "CAX.1c-partial",
    selected_interpretation: current_interpretation.to_string(),
    alternatives,
    ambiguity_policy: "prefer-read-only-grounding-before-patch-proposal",
    evidence_refs: build_coding_agent_context_evidence_refs(request),
  }
}

fn build_coding_agent_judgement(
  request: &CodingAgentRequestArtifact,
  interpretation_set: &CodingAgentInterpretationSetSurface,
) -> CodingAgentJudgementSurface {
  let mut blocked_reasons = Vec::new();
  if request.workspace.approved_commands.is_empty() {
    blocked_reasons.push("missing-approved-verification-command".to_string());
  }
  if request
    .context_pack
    .forbidden_effects
    .iter()
    .any(|effect| effect.starts_with("forbidden-path:"))
  {
    blocked_reasons.push("forbidden-path-policy-present".to_string());
  }
  let decision = if blocked_reasons.is_empty() {
    "continue-to-patch-proposal"
  } else {
    "hold-before-mutation"
  };

  let mut evidence_refs = interpretation_set.evidence_refs.clone();
  evidence_refs.push(format!(
    "selected-interpretation:{}",
    interpretation_set.selected_interpretation
  ));
  evidence_refs.sort();
  evidence_refs.dedup();

  CodingAgentJudgementSurface {
    artifact_family: "coding.judgement",
    phase: "CAX.1c-partial",
    decision,
    blocked_reasons,
    required_next_artifacts: vec![
      "coding.patch-proposal",
      "coding.execution-plan",
      "coding.verify-receipt",
    ],
    evidence_refs,
  }
}

fn build_coding_agent_execution_plan(
  request: &CodingAgentRequestArtifact,
  request_artifact_ref: Option<&str>,
  bounded_step_family: &[CodingAgentPlanStep],
  expected_verification: &[String],
  failure_policy: &'static str,
) -> CodingAgentExecutionPlanSurface {
  let language_verify_targets = request.language_profile.verify_targets.clone();
  let candidate_verify_target_refs = language_verify_targets
    .iter()
    .map(|target| target.target_ref.clone())
    .collect::<Vec<_>>();
  let execution_plan_ref = make_execution_plan_ref(
    request_artifact_ref,
    &request.context_pack.context_pack_ref,
    &request.workspace.target_paths,
    expected_verification,
    &candidate_verify_target_refs,
  );
  let execution_request = build_coding_agent_execution_request(
    &execution_plan_ref,
    &request.workspace.target_paths,
    expected_verification,
    &request.workspace.approved_commands,
    &language_verify_targets,
  );

  CodingAgentExecutionPlanSurface {
    artifact_family: "coding.execution-plan",
    phase: "CAX.1c-partial",
    execution_plan_ref,
    execution_owner: "pnix-executor-graph::coding-agent::execution-plan",
    effect_policy: failure_policy,
    bounded_step_family: bounded_step_family.to_vec(),
    expected_verification: expected_verification.to_vec(),
    language_verify_targets,
    execution_requests: vec![execution_request],
  }
}

fn build_coding_agent_execution_request(
  execution_plan_ref: &str,
  target_paths: &[String],
  expected_verification: &[String],
  approved_commands: &[String],
  language_verify_targets: &[CodingAgentVerifyTargetSurface],
) -> CodingAgentExecutionRequestSurface {
  let candidate_verify_target_refs = language_verify_targets
    .iter()
    .map(|target| target.target_ref.clone())
    .collect::<Vec<_>>();
  let candidate_command_refs = language_verify_targets
    .iter()
    .flat_map(|target| {
      target
        .command_candidates
        .iter()
        .map(|command| format!("candidate:{}:{}", target.language, command))
    })
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect::<Vec<_>>();
  let permission_status =
    if approved_commands.is_empty() && !candidate_verify_target_refs.is_empty() {
      "candidate-only-requires-approved-command"
    } else if approved_commands.is_empty() {
      "blocked:no-approved-command"
    } else {
      "declared-approved-command-not-executed"
    };
  let mut effect_classes = build_patch_effect_classes(target_paths, approved_commands);
  if approved_commands.is_empty() && !candidate_command_refs.is_empty() {
    effect_classes.push("verification-command:candidate-only".to_string());
    effect_classes.sort();
    effect_classes.dedup();
  }

  CodingAgentExecutionRequestSurface {
    artifact_family: "coding.execution-request",
    phase: "CAX.1c-partial",
    request_ref: make_execution_request_ref(
      execution_plan_ref,
      expected_verification,
      &candidate_verify_target_refs,
    ),
    permission_status,
    command_refs: expected_verification.to_vec(),
    candidate_verify_target_refs,
    candidate_command_refs,
    effect_classes,
  }
}

fn build_coding_agent_patch_proposal(
  args: &Args,
  request: CodingAgentRequestArtifact,
  request_artifact_ref: Option<String>,
) -> CodingAgentPatchProposalArtifact {
  let current_interpretation = classify_coding_agent_interpretation(
    request.request.as_deref(),
    &request.workspace.target_paths,
  );
  let target_paths = request.workspace.target_paths.clone();
  let edit_family = classify_patch_edit_family(request.request.as_deref(), &target_paths);
  let expected_verify_ref = if request.workspace.approved_commands.is_empty() {
    vec!["manual-verification-contract-required".to_string()]
  } else {
    request.workspace.approved_commands.clone()
  };
  let risk_class = classify_patch_risk_class(
    request.request.as_deref(),
    &target_paths,
    &request.workspace.approved_commands,
  );
  let diff_ref = make_patch_diff_ref(
    request.request.as_deref(),
    &target_paths,
    request.workspace.current_plan_ref.as_deref(),
  );
  let mut effect_classes =
    build_patch_effect_classes(&target_paths, &request.workspace.approved_commands);
  let generated_patch_candidate =
    build_coding_agent_generated_patch_candidate(args, &request, &diff_ref);
  if generated_patch_candidate.is_some() {
    effect_classes.push("provider-patch-candidate:quarantined".to_string());
    if args.agent_provider_feedback_request_ref.is_some() {
      effect_classes.push("provider-feedback-response:quarantined".to_string());
    }
    effect_classes.sort();
    effect_classes.dedup();
  }
  let generated_patch_review_receipt = generated_patch_candidate.as_ref().map(|candidate| {
    build_coding_agent_generated_patch_review_receipt(&request, &diff_ref, candidate)
  });
  let feedback_retry_guard = generated_patch_candidate
    .as_ref()
    .zip(generated_patch_review_receipt.as_ref())
    .and_then(|(candidate, review_receipt)| {
      build_coding_agent_feedback_retry_guard(&request, &diff_ref, candidate, review_receipt)
    });
  if feedback_retry_guard.is_some() {
    effect_classes.push("provider-feedback-retry:guarded".to_string());
    effect_classes.sort();
    effect_classes.dedup();
  }
  let provider_feedback_request = generated_patch_review_receipt
    .as_ref()
    .filter(|_| feedback_retry_guard.is_none())
    .and_then(|review_receipt| {
      build_coding_agent_provider_feedback_request(&request, &diff_ref, review_receipt)
    });
  let apply_build = build_coding_agent_patch_apply_result(
    args,
    &request,
    &diff_ref,
    generated_patch_candidate.as_ref(),
    generated_patch_review_receipt.as_ref(),
  );
  if apply_build.apply_handoff_proof.is_some() {
    effect_classes.push("generated-patch-apply-handoff:checked".to_string());
    effect_classes.sort();
    effect_classes.dedup();
  }
  let mut apply_result = apply_build.apply_result;
  let apply_handoff_proof = apply_build.apply_handoff_proof;
  let promotion_boundary_receipt = apply_result.as_ref().and_then(|apply_result| {
    build_coding_agent_promotion_boundary_receipt(apply_result, apply_handoff_proof.as_ref())
  });
  if let Some(apply_result) = apply_result.as_mut() {
    if let Some(receipt) = promotion_boundary_receipt.as_ref() {
      attach_promotion_boundary_receipt_to_apply_result(apply_result, receipt);
    }
  }
  let context_demand_replay = build_coding_agent_context_demand_replay(&request);
  let repair_recipe_replay = build_coding_agent_repair_recipe_replay(&request);
  let semantic_review = build_coding_agent_semantic_patch_review(
    &request,
    &diff_ref,
    edit_family,
    risk_class,
    effect_classes.as_slice(),
    apply_result.as_ref(),
    &context_demand_replay,
    &repair_recipe_replay,
    generated_patch_candidate.as_ref(),
    generated_patch_review_receipt.as_ref(),
    provider_feedback_request.as_ref(),
    feedback_retry_guard.as_ref(),
    apply_handoff_proof.as_ref(),
    promotion_boundary_receipt.as_ref(),
  );
  if let Some(apply_result) = apply_result.as_mut() {
    attach_semantic_patch_review_to_apply_result(apply_result, &semantic_review);
  }
  let apply_status = apply_result
    .as_ref()
    .map(|result| result.apply_status)
    .unwrap_or("proposal-only-not-applied");
  let apply_artifact_ref = apply_result
    .as_ref()
    .map(|result| result.apply_artifact_ref.clone());
  let note = if let Some(apply_result) = apply_result.as_ref() {
    match apply_result.apply_status {
      "applied" => {
        "typed patch proposal 과 분리된 apply-result를 생성했고 rollback handle ref를 연결했다."
      }
      "validated-not-applied" => {
        "typed patch proposal 과 apply-result dry-run proof를 생성했고 실제 write는 수행하지 않았다."
      }
      _ => "typed patch proposal 과 blocked apply-result를 생성했고 실제 write는 수행하지 않았다.",
    }
  } else if args.agent_patch_out.is_some() {
    "typed patch proposal artifact를 생성했고 actual apply/write lane은 아직 열지 않았다."
  } else {
    "stdout에 typed patch proposal surface만 노출했고 actual apply/write lane은 아직 열지 않았다."
  };
  let progress_status = if apply_status == "applied" {
    "패치적용완료"
  } else if apply_result.is_some() {
    "패치적용검토완료"
  } else {
    "패치제안완료"
  };
  let result_status = match apply_status {
    "applied" | "validated-not-applied" => "부분완료",
    "blocked" | "failed" => "차단",
    _ => "부분완료",
  };

  CodingAgentPatchProposalArtifact {
    artifact_family: "coding.patch-proposal",
    phase: "CAX.3a-partial",
    surface: "pnix coding-agent",
    verb: "patch",
    proposed_at_ms: current_time_ms(),
    request_artifact_ref,
    current_plan_ref: request.workspace.current_plan_ref.clone(),
    current_interpretation,
    target_paths,
    edit_family,
    diff_ref,
    expected_verify_ref,
    risk_class,
    effect_classes: effect_classes.clone(),
    generated_patch_candidate,
    generated_patch_review_receipt,
    provider_feedback_request,
    feedback_retry_guard,
    apply_handoff_proof,
    promotion_boundary_receipt,
    apply_intent: CodingAgentApplyIntentSurface {
      intent_family: "coding.apply-intent",
      effect_classes,
      apply_status,
      apply_artifact_ref,
      separated_from_proposal: true,
    },
    apply_result,
    context_demand_replay,
    repair_recipe_replay,
    semantic_review,
    status: CodingAgentStatusSurface {
      progress_status,
      result_status,
      note: note.to_string(),
    },
    request,
  }
}

fn build_coding_agent_generated_patch_candidate(
  args: &Args,
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
) -> Option<CodingAgentGeneratedPatchCandidateSurface> {
  let candidate_path = args.agent_candidate_patch.as_ref()?;
  let source_provider_feedback_request_ref = args.agent_provider_feedback_request_ref.clone();
  let (phase, lineage_status, response_boundary) = if source_provider_feedback_request_ref.is_some()
  {
    (
      "CAX.5l",
      "revised-candidate-from-provider-feedback",
      "provider-feedback-response-reingested-as-candidate-patch-not-truth",
    )
  } else {
    (
      "CAX.5i",
      "standalone-generated-patch-candidate",
      "provider-output-is-candidate-not-proof",
    )
  };
  let source_path = path_to_slash(candidate_path);
  let read_result = fs::read_to_string(candidate_path)
    .map_err(|err| format!("read candidate patch {}: {}", candidate_path.display(), err));
  let patch_input_ref = make_generated_patch_input_ref(
    candidate_path,
    read_result.as_deref().ok(),
    read_result.as_ref().err().map(String::as_str),
  );
  let mut parsed_target_paths = Vec::new();
  let mut rejected_target_paths = Vec::new();
  let mut error = read_result.as_ref().err().cloned();
  let (byte_len, line_count) = match read_result.as_deref() {
    Ok(patch_text) => {
      match parse_coding_agent_unified_patch(patch_text) {
        Ok(parsed) => {
          parsed_target_paths = parsed
            .iter()
            .map(|patch| patch.target_path.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
          rejected_target_paths =
            generated_patch_candidate_rejected_targets(request, &parsed_target_paths);
        }
        Err(err) => {
          error = Some(err);
        }
      }
      (patch_text.len(), patch_text.lines().count())
    }
    Err(_) => (0, 0),
  };
  let quarantine_status = if read_result.is_err() {
    "quarantined-unreadable-candidate"
  } else if error.is_some() {
    "quarantined-invalid-unified-diff"
  } else if request.workspace.target_paths.is_empty() {
    "quarantined-target-scope-required"
  } else if !rejected_target_paths.is_empty() {
    "quarantined-target-mismatch"
  } else {
    "quarantined-provider-patch-candidate"
  };
  let mut required_next_artifacts = BTreeSet::from([
    "current-context-review-before-patch-proposal".to_string(),
    "semantic-patch-review-before-apply".to_string(),
    "verify-receipt-before-promotion".to_string(),
  ]);
  if error.is_some()
    || !rejected_target_paths.is_empty()
    || request.workspace.target_paths.is_empty()
  {
    required_next_artifacts.insert("context-demand-before-apply".to_string());
  } else {
    required_next_artifacts.insert("explicit-apply-result-required-for-mutation".to_string());
  }
  if source_provider_feedback_request_ref.is_some() {
    required_next_artifacts
      .insert("generated-patch-review-receipt-before-feedback-close".to_string());
  }
  let candidate_ref = make_generated_patch_candidate_ref(
    request,
    &source_path,
    &patch_input_ref,
    &parsed_target_paths,
    quarantine_status,
    source_provider_feedback_request_ref.as_deref(),
  );
  let mut proof_refs = build_coding_agent_context_evidence_refs(request);
  proof_refs.extend([
    format!("candidate-ref:{}", candidate_ref),
    format!("diff-ref:{}", diff_ref),
    format!("patch-input-ref:{}", patch_input_ref),
    format!("candidate-source-path:{}", source_path),
    format!("quarantine-status:{}", quarantine_status),
    format!("lineage-status:{}", lineage_status),
    format!("response-boundary:{}", response_boundary),
    "direct-apply:forbidden".to_string(),
  ]);
  if let Some(feedback_request_ref) = source_provider_feedback_request_ref.as_deref() {
    proof_refs.push(format!(
      "provider-feedback-request-ref:{}",
      feedback_request_ref
    ));
    proof_refs.push("provider-feedback-response:not-truth-owner".to_string());
  }
  proof_refs.extend(
    parsed_target_paths
      .iter()
      .map(|target| format!("parsed-target-path:{target}")),
  );
  proof_refs.extend(
    rejected_target_paths
      .iter()
      .map(|target| format!("rejected-target-path:{target}")),
  );
  if let Some(err) = error.as_deref() {
    proof_refs.push(format!(
      "candidate-error:{}",
      truncate_diagnostic_message(err)
    ));
  }
  proof_refs.sort();
  proof_refs.dedup();

  Some(CodingAgentGeneratedPatchCandidateSurface {
    artifact_family: "coding.generated-patch-candidate",
    phase,
    candidate_ref,
    candidate_owner: "pnix-executor-graph::coding-agent::generated-patch-quarantine",
    source_path,
    patch_input_ref,
    byte_len,
    line_count,
    target_paths: request.workspace.target_paths.clone(),
    parsed_target_paths,
    rejected_target_paths,
    quarantine_status,
    lineage_status,
    source_provider_feedback_request_ref,
    response_boundary,
    promotion_boundary: "candidate-only-not-apply-owner",
    required_next_artifacts: required_next_artifacts.into_iter().collect(),
    proof_refs,
    error,
  })
}

fn generated_patch_candidate_rejected_targets(
  request: &CodingAgentRequestArtifact,
  parsed_target_paths: &[String],
) -> Vec<String> {
  let allowed_targets = request
    .workspace
    .target_paths
    .iter()
    .map(|path| path_to_slash(Path::new(path)))
    .collect::<BTreeSet<_>>();
  let forbidden_paths = request
    .workspace
    .forbidden_paths
    .iter()
    .map(|path| path_to_slash(Path::new(path)))
    .collect::<Vec<_>>();
  parsed_target_paths
    .iter()
    .filter(|target| {
      allowed_targets.is_empty()
        || !allowed_targets.contains(*target)
        || forbidden_paths
          .iter()
          .any(|forbidden| *target == forbidden || target.starts_with(&format!("{}/", forbidden)))
    })
    .cloned()
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect()
}

fn build_coding_agent_generated_patch_review_receipt(
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
  candidate: &CodingAgentGeneratedPatchCandidateSurface,
) -> CodingAgentGeneratedPatchReviewReceiptSurface {
  let review_ref = make_generated_patch_review_ref(
    "receipt",
    candidate.candidate_ref.as_str(),
    candidate.patch_input_ref.as_str(),
    candidate.quarantine_status,
  );
  let mut diagnostic_records = Vec::new();
  let mut failure_pattern_matches = Vec::new();
  let mut context_demands = Vec::new();

  if let Some(error) = candidate.error.as_deref() {
    push_generated_patch_review_bridge(
      &mut diagnostic_records,
      &mut failure_pattern_matches,
      &mut context_demands,
      candidate,
      generated_patch_review_primary_target(request, candidate).as_str(),
      "generated-patch-invalid-unified-diff",
      "malformed-generated-patch",
      "generated-patch-format-required",
      "error",
      format!(
        "generated patch candidate is not a valid bounded unified diff: {}",
        truncate_diagnostic_message(error)
      ),
      vec![
        "valid-unified-diff".to_string(),
        "declared-target-path".to_string(),
        "semantic-patch-review-before-apply".to_string(),
      ],
    );
  }
  if request.workspace.target_paths.is_empty() {
    push_generated_patch_review_bridge(
      &mut diagnostic_records,
      &mut failure_pattern_matches,
      &mut context_demands,
      candidate,
      "workspace",
      "generated-patch-target-scope-missing",
      "missing-generated-patch-target-scope",
      "generated-patch-target-scope-required",
      "hold",
      "generated patch candidate cannot be reviewed without an explicit --target-path scope"
        .to_string(),
      vec![
        "explicit-target-path".to_string(),
        "repo-snapshot-ref".to_string(),
        "patch-target-join-proof".to_string(),
      ],
    );
  }
  for target_path in &candidate.rejected_target_paths {
    push_generated_patch_review_bridge(
      &mut diagnostic_records,
      &mut failure_pattern_matches,
      &mut context_demands,
      candidate,
      target_path,
      "generated-patch-target-mismatch",
      "generated-patch-target-mismatch",
      "generated-patch-target-scope-required",
      "hold",
      format!(
        "generated patch target {} is outside declared target scope",
        target_path
      ),
      vec![
        "declared-target-path".to_string(),
        "patch-target-join-proof".to_string(),
        "human-review-before-apply".to_string(),
      ],
    );
  }
  if request.workspace.approved_commands.is_empty() {
    push_generated_patch_review_bridge(
      &mut diagnostic_records,
      &mut failure_pattern_matches,
      &mut context_demands,
      candidate,
      generated_patch_review_primary_target(request, candidate).as_str(),
      "generated-patch-verification-missing",
      "generated-patch-missing-verification",
      "generated-patch-verification-required",
      "hold",
      "generated patch candidate has no approved verification command".to_string(),
      vec![
        "approved-command".to_string(),
        "verify-target".to_string(),
        "verify-receipt-before-promotion".to_string(),
      ],
    );
  }

  let review_status = if context_demands.is_empty() {
    "candidate-reviewed-awaiting-explicit-apply-result"
  } else {
    "candidate-review-context-required"
  };
  let mut required_next_artifacts = candidate
    .required_next_artifacts
    .iter()
    .cloned()
    .collect::<BTreeSet<_>>();
  for demand in &context_demands {
    required_next_artifacts.extend(demand.required_evidence.iter().cloned());
  }
  if context_demands.is_empty() {
    required_next_artifacts.insert("explicit-apply-result-required-for-mutation".to_string());
  } else {
    required_next_artifacts.insert("revised-generated-patch-candidate".to_string());
  }

  let mut proof_refs = vec![
    format!("generated-patch-candidate-ref:{}", candidate.candidate_ref),
    format!("generated-patch-review-ref:{}", review_ref),
    format!("patch-input-ref:{}", candidate.patch_input_ref),
    format!("diff-ref:{}", diff_ref),
    format!("quarantine-status:{}", candidate.quarantine_status),
    format!("review-status:{}", review_status),
    "direct-apply:forbidden".to_string(),
  ];
  proof_refs.extend(build_coding_agent_context_evidence_refs(request));
  proof_refs.extend(
    diagnostic_records
      .iter()
      .map(|diagnostic| format!("diagnostic-ref:{}", diagnostic.diagnostic_ref)),
  );
  proof_refs.extend(
    context_demands
      .iter()
      .map(|demand| format!("context-demand-ref:{}", demand.context_demand_ref)),
  );
  proof_refs.sort();
  proof_refs.dedup();

  CodingAgentGeneratedPatchReviewReceiptSurface {
    artifact_family: "coding.generated-patch-review-receipt",
    phase: "CAX.5j",
    review_ref,
    review_owner: "pnix-executor-graph::coding-agent::generated-patch-review",
    candidate_ref: candidate.candidate_ref.clone(),
    patch_input_ref: candidate.patch_input_ref.clone(),
    target_paths: candidate.target_paths.clone(),
    parsed_target_paths: candidate.parsed_target_paths.clone(),
    rejected_target_paths: candidate.rejected_target_paths.clone(),
    review_status,
    diagnostic_records,
    failure_pattern_matches,
    context_demands,
    required_next_artifacts: required_next_artifacts.into_iter().collect(),
    proof_refs,
    promotion_boundary: "candidate-only-not-provider-feedback-owner",
  }
}

#[allow(clippy::too_many_arguments)]
fn push_generated_patch_review_bridge(
  diagnostic_records: &mut Vec<CodingAgentDiagnosticRecordSurface>,
  failure_pattern_matches: &mut Vec<CodingAgentFailurePatternMatchSurface>,
  context_demands: &mut Vec<CodingAgentContextDemandSurface>,
  candidate: &CodingAgentGeneratedPatchCandidateSurface,
  target_path: &str,
  diagnostic_family: &'static str,
  pattern_key: &'static str,
  demand_family: &'static str,
  severity: &'static str,
  message: String,
  required_evidence: Vec<String>,
) {
  let diagnostic_ref = make_generated_patch_review_bridge_ref(
    "diagnostic",
    candidate.candidate_ref.as_str(),
    target_path,
    diagnostic_family,
  );
  let context_demand_ref = make_generated_patch_review_bridge_ref(
    "context-demand",
    candidate.candidate_ref.as_str(),
    target_path,
    demand_family,
  );
  let match_ref = make_generated_patch_review_bridge_ref(
    "failure-pattern-match",
    candidate.candidate_ref.as_str(),
    target_path,
    pattern_key,
  );

  diagnostic_records.push(CodingAgentDiagnosticRecordSurface {
    artifact_family: "pnix.diagnostic-record",
    diagnostic_ref: diagnostic_ref.clone(),
    language: "diff".to_string(),
    target_path: target_path.to_string(),
    diagnostic_family,
    severity,
    message,
    provenance_refs: vec![
      format!("generated-patch-candidate-ref:{}", candidate.candidate_ref),
      format!("patch-input-ref:{}", candidate.patch_input_ref),
      "CAX.5j:generated-patch-review-context-demand-bridge".to_string(),
    ],
    record_status: "candidate",
  });
  failure_pattern_matches.push(CodingAgentFailurePatternMatchSurface {
    artifact_family: "coding.failure-pattern-match",
    match_ref,
    diagnostic_ref,
    pattern_key,
    confidence: 1.0,
    context_demand_ref: context_demand_ref.clone(),
    promotion_boundary: "candidate-only-not-judgement",
  });
  context_demands.push(CodingAgentContextDemandSurface {
    artifact_family: "coding.context-demand",
    context_demand_ref,
    language: "diff".to_string(),
    target_path: target_path.to_string(),
    demand_family,
    required_evidence,
    request_boundary: "request-revised-provider-patch-before-apply",
  });
}

fn generated_patch_review_primary_target(
  request: &CodingAgentRequestArtifact,
  candidate: &CodingAgentGeneratedPatchCandidateSurface,
) -> String {
  candidate
    .parsed_target_paths
    .first()
    .or_else(|| request.workspace.target_paths.first())
    .cloned()
    .unwrap_or_else(|| "workspace".to_string())
}

fn build_coding_agent_provider_feedback_request(
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
  review_receipt: &CodingAgentGeneratedPatchReviewReceiptSurface,
) -> Option<CodingAgentProviderFeedbackRequestSurface> {
  if review_receipt.context_demands.is_empty() {
    return None;
  }

  let feedback_packets = review_receipt
    .context_demands
    .iter()
    .map(|demand| CodingAgentProviderFeedbackPacketSurface {
      packet_ref: make_provider_feedback_packet_ref(
        review_receipt.review_ref.as_str(),
        demand.context_demand_ref.as_str(),
        demand.target_path.as_str(),
        demand.demand_family,
      ),
      packet_kind: "provider-feedback-context-demand",
      source_context_demand_ref: demand.context_demand_ref.clone(),
      target_path: demand.target_path.clone(),
      demand_family: demand.demand_family.to_string(),
      required_evidence: demand.required_evidence.clone(),
      requested_output: "revised-generated-patch-candidate",
      response_boundary: "provider-response-must-return-candidate-patch-not-direct-write",
      truth_boundary: "provider-output-is-candidate-not-proof",
    })
    .collect::<Vec<_>>();
  let context_demand_refs = review_receipt
    .context_demands
    .iter()
    .map(|demand| demand.context_demand_ref.clone())
    .collect::<Vec<_>>();
  let mut required_evidence = review_receipt
    .required_next_artifacts
    .iter()
    .cloned()
    .chain(
      feedback_packets
        .iter()
        .flat_map(|packet| packet.required_evidence.iter().cloned()),
    )
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect::<Vec<_>>();
  required_evidence.push("provider-response-must-be-reingested-as-candidate-patch".to_string());
  required_evidence.sort();
  required_evidence.dedup();
  let packet_refs = feedback_packets
    .iter()
    .map(|packet| packet.packet_ref.clone())
    .collect::<Vec<_>>();
  let request_ref = make_provider_feedback_request_ref(
    review_receipt.review_ref.as_str(),
    review_receipt.candidate_ref.as_str(),
    &context_demand_refs,
    &packet_refs,
  );
  let mut proof_refs = vec![
    format!("provider-feedback-request-ref:{}", request_ref),
    format!("generated-patch-review-ref:{}", review_receipt.review_ref),
    format!(
      "generated-patch-candidate-ref:{}",
      review_receipt.candidate_ref
    ),
    format!("patch-input-ref:{}", review_receipt.patch_input_ref),
    format!("diff-ref:{}", diff_ref),
    "provider-output-is-candidate-not-truth".to_string(),
    "direct-apply:forbidden".to_string(),
  ];
  proof_refs.extend(build_coding_agent_context_evidence_refs(request));
  proof_refs.extend(
    context_demand_refs
      .iter()
      .map(|demand_ref| format!("context-demand-ref:{demand_ref}")),
  );
  proof_refs.extend(
    packet_refs
      .iter()
      .map(|packet_ref| format!("provider-feedback-packet-ref:{packet_ref}")),
  );
  proof_refs.sort();
  proof_refs.dedup();

  Some(CodingAgentProviderFeedbackRequestSurface {
    artifact_family: "coding.provider-feedback-request",
    phase: "CAX.5k",
    request_ref,
    feedback_owner: "pnix-executor-graph::coding-agent::provider-feedback-request",
    source_review_ref: review_receipt.review_ref.clone(),
    source_candidate_ref: review_receipt.candidate_ref.clone(),
    patch_input_ref: review_receipt.patch_input_ref.clone(),
    request_status: "provider-feedback-request-ready",
    provider_boundary: "provider-is-candidate-generator-not-truth-or-apply-owner",
    target_paths: review_receipt.target_paths.clone(),
    context_demand_refs,
    feedback_packets,
    required_evidence,
    forbidden_effects: vec![
      "provider-direct-write".to_string(),
      "provider-auto-apply".to_string(),
      "provider-verify-promotion".to_string(),
    ],
    proof_refs,
    promotion_boundary: "candidate-only-request-packet-not-prompt-truth",
  })
}

fn build_coding_agent_feedback_retry_guard(
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
  candidate: &CodingAgentGeneratedPatchCandidateSurface,
  review_receipt: &CodingAgentGeneratedPatchReviewReceiptSurface,
) -> Option<CodingAgentFeedbackRetryGuardSurface> {
  let source_provider_feedback_request_ref =
    candidate.source_provider_feedback_request_ref.as_ref()?;
  if review_receipt.context_demands.is_empty() {
    return None;
  }

  let context_demand_refs = review_receipt
    .context_demands
    .iter()
    .map(|demand| demand.context_demand_ref.clone())
    .collect::<Vec<_>>();
  let guard_ref = make_feedback_retry_guard_ref(
    source_provider_feedback_request_ref.as_str(),
    candidate.candidate_ref.as_str(),
    review_receipt.review_ref.as_str(),
    &context_demand_refs,
  );
  let mut required_human_evidence = review_receipt
    .context_demands
    .iter()
    .flat_map(|demand| demand.required_evidence.iter().cloned())
    .collect::<BTreeSet<_>>();
  required_human_evidence.extend([
    "human-review-before-next-provider-retry".to_string(),
    "retry-attempt-limit-decision".to_string(),
    "patch-target-scope-confirmation".to_string(),
  ]);
  let mut proof_refs = vec![
    format!("feedback-retry-guard-ref:{}", guard_ref),
    format!(
      "provider-feedback-request-ref:{}",
      source_provider_feedback_request_ref
    ),
    format!("generated-patch-candidate-ref:{}", candidate.candidate_ref),
    format!("generated-patch-review-ref:{}", review_receipt.review_ref),
    format!("diff-ref:{}", diff_ref),
    "provider-feedback-auto-retry:forbidden".to_string(),
    "human-review-escalation-required".to_string(),
  ];
  proof_refs.extend(build_coding_agent_context_evidence_refs(request));
  proof_refs.extend(
    context_demand_refs
      .iter()
      .map(|demand_ref| format!("context-demand-ref:{demand_ref}")),
  );
  proof_refs.sort();
  proof_refs.dedup();

  Some(CodingAgentFeedbackRetryGuardSurface {
    artifact_family: "coding.feedback-retry-guard",
    phase: "CAX.5m",
    guard_ref,
    guard_owner: "pnix-executor-graph::coding-agent::feedback-retry-guard",
    source_provider_feedback_request_ref: source_provider_feedback_request_ref.clone(),
    source_candidate_ref: candidate.candidate_ref.clone(),
    source_review_ref: review_receipt.review_ref.clone(),
    attempt_index: 1,
    attempt_limit: 1,
    guard_status: "human-review-escalation-required",
    retry_decision: "block-provider-auto-retry",
    context_demand_refs,
    required_human_evidence: required_human_evidence.into_iter().collect(),
    forbidden_effects: vec![
      "provider-feedback-auto-retry".to_string(),
      "provider-direct-write".to_string(),
      "provider-auto-apply".to_string(),
    ],
    proof_refs,
    promotion_boundary: "guard-only-not-provider-loop-owner",
  })
}

fn build_coding_agent_apply_handoff_proof(
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
  apply_patch_path: &Path,
  apply_patch_text: &str,
  candidate: &CodingAgentGeneratedPatchCandidateSurface,
  review_receipt: &CodingAgentGeneratedPatchReviewReceiptSurface,
) -> CodingAgentApplyHandoffProofSurface {
  let apply_patch_source_path = path_to_slash(apply_patch_path);
  let apply_patch_input_ref =
    make_generated_patch_input_ref(apply_patch_path, Some(apply_patch_text), None);
  let mut failure_reasons = Vec::new();
  if candidate.patch_input_ref != apply_patch_input_ref {
    failure_reasons.push("apply patch input differs from reviewed generated-patch-candidate");
  }
  if review_receipt.candidate_ref != candidate.candidate_ref {
    failure_reasons.push("generated patch review receipt is not linked to candidate");
  }
  if review_receipt.review_status != "candidate-reviewed-awaiting-explicit-apply-result" {
    failure_reasons.push("generated patch review is not ready for explicit apply");
  }
  if !review_receipt.context_demands.is_empty() {
    failure_reasons.push("generated patch review still has context demands");
  }
  if candidate.error.is_some() {
    failure_reasons.push("generated patch candidate has parse/read error");
  }
  if !candidate.rejected_target_paths.is_empty() {
    failure_reasons.push("generated patch candidate has rejected targets");
  }

  let handoff_status = if failure_reasons.is_empty() {
    "handoff-accepted"
  } else {
    "handoff-blocked"
  };
  let failure_reason = if failure_reasons.is_empty() {
    None
  } else {
    Some(failure_reasons.join("; "))
  };
  let handoff_ref = make_apply_handoff_ref(
    candidate.candidate_ref.as_str(),
    review_receipt.review_ref.as_str(),
    candidate.patch_input_ref.as_str(),
    apply_patch_input_ref.as_str(),
    handoff_status,
  );
  let mut required_evidence = BTreeSet::from([
    "reviewed-generated-patch-candidate".to_string(),
    "explicit-apply-result-required-for-mutation".to_string(),
    "verify-receipt-before-promotion".to_string(),
  ]);
  if handoff_status == "handoff-blocked" {
    required_evidence.insert("human-review-before-apply".to_string());
    required_evidence.insert("matching-patch-input-lineage".to_string());
  }
  let mut proof_refs = vec![
    format!("apply-handoff-proof-ref:{}", handoff_ref),
    format!("apply-handoff-status:{}", handoff_status),
    format!("generated-patch-candidate-ref:{}", candidate.candidate_ref),
    format!("generated-patch-review-ref:{}", review_receipt.review_ref),
    format!("candidate-patch-input-ref:{}", candidate.patch_input_ref),
    format!("apply-patch-input-ref:{}", apply_patch_input_ref),
    format!("apply-patch-source-path:{}", apply_patch_source_path),
    format!("diff-ref:{}", diff_ref),
    "generated-patch-auto-apply:forbidden".to_string(),
  ];
  if let Some(failure_reason) = failure_reason.as_deref() {
    proof_refs.push(format!("apply-handoff-blocked:{}", failure_reason));
  }
  proof_refs.extend(build_coding_agent_context_evidence_refs(request));
  proof_refs.sort();
  proof_refs.dedup();

  CodingAgentApplyHandoffProofSurface {
    artifact_family: "coding.apply-handoff-proof",
    phase: "CAX.5n",
    handoff_ref,
    handoff_owner: "pnix-executor-graph::coding-agent::apply-handoff-proof",
    candidate_ref: candidate.candidate_ref.clone(),
    candidate_review_ref: review_receipt.review_ref.clone(),
    candidate_patch_input_ref: candidate.patch_input_ref.clone(),
    apply_patch_input_ref,
    apply_patch_source_path,
    handoff_status,
    failure_reason,
    target_paths: request.workspace.target_paths.clone(),
    parsed_candidate_target_paths: candidate.parsed_target_paths.clone(),
    required_evidence: required_evidence.into_iter().collect(),
    forbidden_effects: vec![
      "generated-patch-auto-apply".to_string(),
      "apply-handoff-mismatch-write".to_string(),
      "provider-direct-write".to_string(),
    ],
    proof_refs,
    promotion_boundary: "proof-only-not-apply-owner",
  }
}

fn build_coding_agent_promotion_boundary_receipt(
  apply_result: &CodingAgentApplyResultSurface,
  handoff_proof: Option<&CodingAgentApplyHandoffProofSurface>,
) -> Option<CodingAgentPromotionBoundaryReceiptSurface> {
  if !matches!(
    apply_result.apply_status,
    "applied" | "validated-not-applied"
  ) {
    return None;
  }
  let promotion_status = if apply_result.apply_status == "validated-not-applied" {
    "promotion-held-dry-run-not-mutation"
  } else {
    "promotion-held-pending-verify-receipt"
  };
  let source_handoff_ref = handoff_proof.map(|proof| proof.handoff_ref.clone());
  let receipt_ref = make_promotion_boundary_receipt_ref(
    apply_result.apply_artifact_ref.as_str(),
    source_handoff_ref.as_deref(),
    apply_result.apply_status,
    promotion_status,
  );
  let mut required_next_artifacts = BTreeSet::from([
    "verify-receipt-before-promotion".to_string(),
    "human-judgement-boundary-before-promotion".to_string(),
    "learning-card-after-verify".to_string(),
  ]);
  if apply_result.apply_status == "validated-not-applied" {
    required_next_artifacts.insert("actual-apply-result-before-promotion".to_string());
  }
  let mut proof_refs = vec![
    format!("promotion-boundary-receipt-ref:{}", receipt_ref),
    format!("apply-artifact-ref:{}", apply_result.apply_artifact_ref),
    format!("apply-status:{}", apply_result.apply_status),
    format!("promotion-status:{}", promotion_status),
    "apply-result-is-not-promotion-proof".to_string(),
  ];
  if let Some(handoff_ref) = source_handoff_ref.as_deref() {
    proof_refs.push(format!("apply-handoff-proof-ref:{}", handoff_ref));
  }
  proof_refs.sort();
  proof_refs.dedup();

  Some(CodingAgentPromotionBoundaryReceiptSurface {
    artifact_family: "coding.promotion-boundary-receipt",
    phase: "CAX.5o",
    receipt_ref,
    receipt_owner: "pnix-executor-graph::coding-agent::promotion-boundary-receipt",
    source_apply_artifact_ref: apply_result.apply_artifact_ref.clone(),
    source_handoff_ref,
    apply_status: apply_result.apply_status,
    promotion_status,
    required_next_artifacts: required_next_artifacts.into_iter().collect(),
    forbidden_effects: vec![
      "apply-result-auto-promotion".to_string(),
      "provider-output-promotion".to_string(),
      "verification-bypass-promotion".to_string(),
    ],
    proof_refs,
    promotion_boundary: "receipt-only-not-judgement-owner",
  })
}

fn build_coding_agent_patch_apply_result(
  args: &Args,
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
  generated_patch_candidate: Option<&CodingAgentGeneratedPatchCandidateSurface>,
  generated_patch_review_receipt: Option<&CodingAgentGeneratedPatchReviewReceiptSurface>,
) -> CodingAgentPatchApplyBuildResult {
  let Some(patch_path) = args.patch.as_ref() else {
    return CodingAgentPatchApplyBuildResult {
      apply_result: None,
      apply_handoff_proof: None,
    };
  };
  let applied_at_ms = current_time_ms();
  let apply_artifact_ref = make_apply_artifact_ref(
    request.request.as_deref(),
    &request.workspace.target_paths,
    request.workspace.current_plan_ref.as_deref(),
  );
  let patch_input_ref = Some(format!("patch-input:{}", path_to_slash(patch_path)));
  let repo_snapshot_ref = make_repo_snapshot_ref(&request.workspace);

  let patch_text = match fs::read_to_string(patch_path) {
    Ok(text) => text,
    Err(err) => {
      return CodingAgentPatchApplyBuildResult {
        apply_result: Some(blocked_coding_agent_apply_result(
          applied_at_ms,
          apply_artifact_ref,
          patch_input_ref,
          request,
          diff_ref,
          format!("read patch {}: {}", patch_path.display(), err),
        )),
        apply_handoff_proof: None,
      };
    }
  };
  let apply_handoff_proof = generated_patch_candidate
    .zip(generated_patch_review_receipt)
    .map(|(candidate, review_receipt)| {
      build_coding_agent_apply_handoff_proof(
        request,
        diff_ref,
        patch_path,
        &patch_text,
        candidate,
        review_receipt,
      )
    });
  if let Some(handoff_proof) = apply_handoff_proof.as_ref() {
    if handoff_proof.handoff_status != "handoff-accepted" {
      let mut blocked = blocked_coding_agent_apply_result(
        applied_at_ms,
        apply_artifact_ref,
        patch_input_ref,
        request,
        diff_ref,
        handoff_proof
          .failure_reason
          .clone()
          .unwrap_or_else(|| "generated patch apply handoff blocked".to_string()),
      );
      attach_apply_handoff_proof_to_apply_result(&mut blocked, handoff_proof);
      return CodingAgentPatchApplyBuildResult {
        apply_result: Some(blocked),
        apply_handoff_proof,
      };
    }
  }
  let parsed = match parse_coding_agent_unified_patch(&patch_text) {
    Ok(parsed) => parsed,
    Err(err) => {
      let mut blocked = blocked_coding_agent_apply_result(
        applied_at_ms,
        apply_artifact_ref,
        patch_input_ref,
        request,
        diff_ref,
        err,
      );
      if let Some(handoff_proof) = apply_handoff_proof.as_ref() {
        attach_apply_handoff_proof_to_apply_result(&mut blocked, handoff_proof);
      }
      return CodingAgentPatchApplyBuildResult {
        apply_result: Some(blocked),
        apply_handoff_proof,
      };
    }
  };
  let prepared = match prepare_coding_agent_patch_application(request, parsed.as_slice()) {
    Ok(prepared) => prepared,
    Err(err) => {
      let mut blocked = blocked_coding_agent_apply_result(
        applied_at_ms,
        apply_artifact_ref,
        patch_input_ref,
        request,
        diff_ref,
        err,
      );
      if let Some(handoff_proof) = apply_handoff_proof.as_ref() {
        attach_apply_handoff_proof_to_apply_result(&mut blocked, handoff_proof);
      }
      return CodingAgentPatchApplyBuildResult {
        apply_result: Some(blocked),
        apply_handoff_proof,
      };
    }
  };

  let mut file_results = prepared
    .iter()
    .map(|prepared| CodingAgentPatchFileApplyRecord {
      path: prepared.path.clone(),
      status: if args.dry_run {
        "validated"
      } else {
        "pending-write"
      },
      before_snapshot_ref: prepared.before_snapshot_ref.clone(),
      after_snapshot_ref: Some(prepared.after_snapshot_ref.clone()),
      byte_delta: prepared.byte_delta,
      error: None,
    })
    .collect::<Vec<_>>();

  let mut apply_status = if args.dry_run {
    "validated-not-applied"
  } else {
    "applied"
  };
  let mut error = None;
  if !args.dry_run {
    for (index, prepared) in prepared.iter().enumerate() {
      if let Some(parent) = prepared.absolute_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
          apply_status = "failed";
          error = Some(format!("create parent {}: {}", parent.display(), err));
          file_results[index].status = "failed";
          file_results[index].error = error.clone();
          break;
        }
      }
      if let Err(err) = fs::write(&prepared.absolute_path, &prepared.after_content) {
        apply_status = "failed";
        error = Some(format!("write {}: {}", prepared.path, err));
        file_results[index].status = "failed";
        file_results[index].error = error.clone();
        break;
      }
      file_results[index].status = "applied";
    }
  }

  let applied_paths = if matches!(apply_status, "applied" | "validated-not-applied") {
    prepared
      .iter()
      .map(|prepared| prepared.path.clone())
      .collect::<Vec<_>>()
  } else {
    Vec::new()
  };
  let rejected_paths = if matches!(apply_status, "applied" | "validated-not-applied") {
    Vec::new()
  } else {
    prepared
      .iter()
      .map(|prepared| prepared.path.clone())
      .collect::<Vec<_>>()
  };
  let effect_contracts = build_rollback_effect_contracts(&applied_paths, &[]);
  let rollback_class = if applied_paths.is_empty() {
    "forbidden"
  } else {
    classify_rollback_class(&effect_contracts)
  };
  let inverse_plan_ref = if apply_status == "applied" && rollback_class == "rollbackable" {
    Some(make_inverse_plan_ref(&apply_artifact_ref))
  } else {
    None
  };
  let rollback_handle_ref = if apply_status == "applied" {
    Some(make_rollback_handle_id(
      &repo_snapshot_ref,
      &apply_artifact_ref,
      rollback_class,
    ))
  } else {
    None
  };
  let proof_refs = build_patch_apply_proof_refs(
    request,
    diff_ref,
    &apply_artifact_ref,
    rollback_handle_ref.as_deref(),
    inverse_plan_ref.as_deref(),
    &file_results,
  );

  let mut apply_result = CodingAgentApplyResultSurface {
    artifact_family: "coding.apply-result",
    phase: "CAX.3a",
    applied_at_ms,
    apply_artifact_ref,
    patch_input_ref,
    apply_status,
    dry_run: args.dry_run,
    target_paths: request.workspace.target_paths.clone(),
    applied_paths,
    rejected_paths,
    file_results,
    rollback_class,
    rollback_handle_ref,
    inverse_plan_ref,
    proof_refs,
    error,
  };
  if let Some(handoff_proof) = apply_handoff_proof.as_ref() {
    attach_apply_handoff_proof_to_apply_result(&mut apply_result, handoff_proof);
  }
  CodingAgentPatchApplyBuildResult {
    apply_result: Some(apply_result),
    apply_handoff_proof,
  }
}

fn blocked_coding_agent_apply_result(
  applied_at_ms: u64,
  apply_artifact_ref: String,
  patch_input_ref: Option<String>,
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
  error: String,
) -> CodingAgentApplyResultSurface {
  let mut proof_refs = vec![
    format!("diff-ref:{}", diff_ref),
    format!("apply-artifact-ref:{}", apply_artifact_ref),
    format!("apply-blocked:{}", error),
  ];
  proof_refs.extend(build_coding_agent_context_evidence_refs(request));
  proof_refs.sort();
  proof_refs.dedup();

  CodingAgentApplyResultSurface {
    artifact_family: "coding.apply-result",
    phase: "CAX.3a",
    applied_at_ms,
    apply_artifact_ref,
    patch_input_ref,
    apply_status: "blocked",
    dry_run: false,
    target_paths: request.workspace.target_paths.clone(),
    applied_paths: Vec::new(),
    rejected_paths: request.workspace.target_paths.clone(),
    file_results: Vec::new(),
    rollback_class: "forbidden",
    rollback_handle_ref: None,
    inverse_plan_ref: None,
    proof_refs,
    error: Some(error),
  }
}

fn build_coding_agent_context_demand_replay(
  request: &CodingAgentRequestArtifact,
) -> CodingAgentContextDemandReplaySurface {
  let store_path = coding_memory_store_path_from_env();
  build_coding_agent_context_demand_replay_from_store_path(request, store_path.as_deref())
}

fn build_coding_agent_context_demand_replay_from_store_path(
  request: &CodingAgentRequestArtifact,
  store_path: Option<&Path>,
) -> CodingAgentContextDemandReplaySurface {
  let mut source_refs = Vec::new();
  if let Some(last_ref) = request.workspace.last_verification_ref.as_deref() {
    source_refs.push(last_ref.to_string());
  }
  if let Some(plan_ref) = request.workspace.current_plan_ref.as_deref() {
    source_refs.push(plan_ref.to_string());
  }
  source_refs.sort();
  source_refs.dedup();

  let mut replayed_context_demands = request
    .language_profile
    .context_demands
    .iter()
    .map(|demand| {
      replay_context_demand_from_value(
        "current-language-profile",
        request.language_profile.artifact_family,
        &serde_json::json!({
          "context_demand_ref": demand.context_demand_ref,
          "language": demand.language,
          "target_path": demand.target_path,
          "demand_family": demand.demand_family,
          "required_evidence": demand.required_evidence,
        }),
      )
    })
    .collect::<Vec<_>>();
  let mut source_artifact_refs: Vec<String> = Vec::new();
  let mut diagnostic_refs = request
    .language_profile
    .diagnostic_records
    .iter()
    .map(|diagnostic| diagnostic.diagnostic_ref.clone())
    .collect::<Vec<_>>();
  let mut semantic_review_refs: Vec<String> = Vec::new();
  #[cfg_attr(not(feature = "doghouse"), allow(unused_mut))]
  let mut store_status = "store-not-configured";

  #[cfg(feature = "doghouse")]
  if let Some(store_path) = store_path {
    match load_coding_agent_replay_sources_from_store(request, store_path) {
      Ok(sources) => {
        store_status = if sources.is_empty() {
          "store-open-no-matching-prior-artifacts"
        } else {
          "store-open-replayed-prior-artifacts"
        };
        for artifact in sources {
          source_artifact_refs.push(artifact.id.clone());
          collect_replay_context_from_artifact(
            request,
            &artifact,
            &mut replayed_context_demands,
            &mut diagnostic_refs,
            &mut semantic_review_refs,
          );
        }
      }
      Err(err) => {
        store_status = "store-unavailable";
        source_refs.push(format!(
          "store-error:{}",
          truncate_diagnostic_message(&err.to_string())
        ));
      }
    }
  }
  #[cfg(not(feature = "doghouse"))]
  {
    // doghouse feature off: no coding memory store to replay from.
    let _ = store_path;
  }

  replayed_context_demands.sort_by(|left, right| {
    left
      .replay_item_ref
      .cmp(&right.replay_item_ref)
      .then(left.source_ref.cmp(&right.source_ref))
  });
  replayed_context_demands.dedup_by(|left, right| left.replay_item_ref == right.replay_item_ref);
  diagnostic_refs.sort();
  diagnostic_refs.dedup();
  semantic_review_refs.sort();
  semantic_review_refs.dedup();
  source_artifact_refs.sort();
  source_artifact_refs.dedup();

  let next_patch_requirements = build_context_demand_replay_requirements(
    &replayed_context_demands,
    &semantic_review_refs,
    request,
  );
  let replay_status = if !replayed_context_demands.is_empty() || !semantic_review_refs.is_empty() {
    "candidate-context-replayed"
  } else {
    store_status
  };
  let replay_ref = make_context_demand_replay_ref(
    request,
    &source_refs,
    &source_artifact_refs,
    &replayed_context_demands,
    &semantic_review_refs,
  );

  CodingAgentContextDemandReplaySurface {
    artifact_family: "coding.context-demand-replay",
    phase: "CAX.5g",
    replay_ref,
    replay_owner: "pnix-executor-graph::coding-agent::context-demand-replay",
    source_refs,
    source_artifact_refs,
    replayed_context_demands,
    diagnostic_refs,
    semantic_review_refs,
    next_patch_requirements,
    replay_status,
    promotion_boundary: "candidate-only-requires-new-patch-proposal-review",
  }
}

#[cfg(feature = "doghouse")]
fn load_coding_agent_replay_sources_from_store(
  request: &CodingAgentRequestArtifact,
  store_path: &Path,
) -> Result<Vec<CodingMemoryArtifact>> {
  let store = DoghouseStore::open(DoghouseStoreConfig::new(store_path.to_path_buf()))
    .with_context(|| format!("open doghouse coding memory store {}", store_path.display()))?;
  let mut artifacts = Vec::new();
  for source_ref in [
    request.workspace.last_verification_ref.as_deref(),
    request.workspace.current_plan_ref.as_deref(),
  ]
  .into_iter()
  .flatten()
  {
    if let Some(artifact) =
      doghouse_core::store::read_coding_memory_artifact_at(store.path(), source_ref)
        .with_context(|| format!("load coding memory artifact {source_ref}"))?
    {
      artifacts.push(artifact);
    }
  }

  if request.workspace.last_verification_ref.is_some() {
    let repo_snapshot_ref = make_repo_snapshot_ref(&request.workspace);
    for artifact in doghouse_core::store::read_coding_memory_artifacts_by_family_at(
      store.path(),
      "coding.semantic-patch-review",
    )
    .context("query semantic-patch-review coding memory artifacts")?
    {
      if artifact.repo_snapshot_ref.as_deref() == Some(repo_snapshot_ref.as_str())
        && coding_memory_artifact_targets_overlap(request, &artifact)
      {
        artifacts.push(artifact);
      }
    }
    for artifact in doghouse_core::store::read_coding_memory_artifacts_by_family_at(
      store.path(),
      "coding.patch-proposal",
    )
    .context("query patch-proposal coding memory artifacts")?
    {
      if artifact.repo_snapshot_ref.as_deref() == Some(repo_snapshot_ref.as_str())
        && coding_memory_artifact_targets_overlap(request, &artifact)
      {
        artifacts.push(artifact);
      }
    }
  }

  artifacts.sort_by(|left, right| {
    right
      .stored_at_ms
      .cmp(&left.stored_at_ms)
      .then(left.id.cmp(&right.id))
  });
  artifacts.dedup_by(|left, right| left.id == right.id);
  artifacts.truncate(8);
  Ok(artifacts)
}

#[cfg(feature = "doghouse")]
fn coding_memory_artifact_targets_overlap(
  request: &CodingAgentRequestArtifact,
  artifact: &CodingMemoryArtifact,
) -> bool {
  request.workspace.target_paths.is_empty()
    || artifact.target_paths.is_empty()
    || request
      .workspace
      .target_paths
      .iter()
      .any(|path| artifact.target_paths.iter().any(|target| target == path))
}

#[cfg(feature = "doghouse")]
fn collect_replay_context_from_artifact(
  request: &CodingAgentRequestArtifact,
  artifact: &CodingMemoryArtifact,
  replayed_context_demands: &mut Vec<CodingAgentReplayedContextDemandSurface>,
  diagnostic_refs: &mut Vec<String>,
  semantic_review_refs: &mut Vec<String>,
) {
  let source_family = artifact
    .payload
    .get("artifact_family")
    .and_then(serde_json::Value::as_str)
    .unwrap_or(artifact.artifact_family.as_str());
  collect_replay_diagnostics_from_value(&artifact.payload, diagnostic_refs);
  collect_replay_demands_from_value(
    artifact.id.as_str(),
    source_family,
    &artifact.payload,
    replayed_context_demands,
  );
  collect_semantic_review_refs_from_value(
    request,
    artifact.id.as_str(),
    source_family,
    &artifact.payload,
    replayed_context_demands,
    semantic_review_refs,
  );
}

#[cfg(feature = "doghouse")]
fn collect_replay_diagnostics_from_value(
  value: &serde_json::Value,
  diagnostic_refs: &mut Vec<String>,
) {
  if let Some(records) = value
    .get("diagnostic_records")
    .and_then(serde_json::Value::as_array)
  {
    for record in records {
      if let Some(diagnostic_ref) = record
        .get("diagnostic_ref")
        .and_then(serde_json::Value::as_str)
      {
        diagnostic_refs.push(diagnostic_ref.to_string());
      }
    }
  }
}

#[cfg(feature = "doghouse")]
fn collect_replay_demands_from_value(
  source_ref: &str,
  source_family: &str,
  value: &serde_json::Value,
  replayed_context_demands: &mut Vec<CodingAgentReplayedContextDemandSurface>,
) {
  if let Some(demands) = value
    .get("context_demands")
    .and_then(serde_json::Value::as_array)
  {
    for demand in demands {
      replayed_context_demands.push(replay_context_demand_from_value(
        source_ref,
        source_family,
        demand,
      ));
    }
  }
  if let Some(language_profile) = value.get("language_profile") {
    collect_replay_demands_from_value(
      source_ref,
      "coding.language-profile",
      language_profile,
      replayed_context_demands,
    );
  }
  if let Some(request) = value.get("request") {
    collect_replay_demands_from_value(source_ref, source_family, request, replayed_context_demands);
  }
}

#[cfg(feature = "doghouse")]
fn collect_semantic_review_refs_from_value(
  request: &CodingAgentRequestArtifact,
  source_ref: &str,
  source_family: &str,
  value: &serde_json::Value,
  replayed_context_demands: &mut Vec<CodingAgentReplayedContextDemandSurface>,
  semantic_review_refs: &mut Vec<String>,
) {
  let semantic_review = if source_family == "coding.semantic-patch-review" {
    Some(value)
  } else {
    value.get("semantic_review")
  };
  let Some(review) = semantic_review else {
    return;
  };
  let review_ref = review
    .get("review_ref")
    .and_then(serde_json::Value::as_str)
    .unwrap_or(source_ref);
  semantic_review_refs.push(review_ref.to_string());
  if let Some(impact_ref) = review
    .get("meaning_impact_diff")
    .and_then(|impact| impact.get("impact_ref"))
    .and_then(serde_json::Value::as_str)
  {
    semantic_review_refs.push(impact_ref.to_string());
  }
  if let Some(link_ref) = review
    .get("patch_decision_link")
    .and_then(|link| link.get("link_ref"))
    .and_then(serde_json::Value::as_str)
  {
    semantic_review_refs.push(link_ref.to_string());
  }
  replayed_context_demands.push(replay_context_demand_from_semantic_review(
    request,
    source_ref,
    source_family,
    review,
  ));
}

fn replay_context_demand_from_value(
  source_ref: &str,
  source_family: &str,
  value: &serde_json::Value,
) -> CodingAgentReplayedContextDemandSurface {
  let original_ref = value
    .get("context_demand_ref")
    .and_then(serde_json::Value::as_str)
    .unwrap_or("context-demand:unknown");
  let language = value
    .get("language")
    .and_then(serde_json::Value::as_str)
    .unwrap_or("unknown");
  let target_path = value
    .get("target_path")
    .and_then(serde_json::Value::as_str)
    .unwrap_or("workspace");
  let demand_family = value
    .get("demand_family")
    .and_then(serde_json::Value::as_str)
    .unwrap_or("context-demand-replay");
  let required_evidence = value
    .get("required_evidence")
    .and_then(serde_json::Value::as_array)
    .map(|items| {
      items
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
    })
    .unwrap_or_else(|| vec!["replayed-context-demand-review".to_string()]);

  CodingAgentReplayedContextDemandSurface {
    artifact_family: "coding.context-demand",
    replay_item_ref: make_replayed_context_demand_ref(source_ref, original_ref, target_path),
    source_ref: source_ref.to_string(),
    source_family: source_family.to_string(),
    language: language.to_string(),
    target_path: target_path.to_string(),
    demand_family: demand_family.to_string(),
    required_evidence,
    request_boundary: "reuse-before-next-patch-proposal",
  }
}

#[cfg(feature = "doghouse")]
fn replay_context_demand_from_semantic_review(
  request: &CodingAgentRequestArtifact,
  source_ref: &str,
  source_family: &str,
  review: &serde_json::Value,
) -> CodingAgentReplayedContextDemandSurface {
  let review_ref = review
    .get("review_ref")
    .and_then(serde_json::Value::as_str)
    .unwrap_or(source_ref);
  let target_path = review
    .get("target_paths")
    .and_then(serde_json::Value::as_array)
    .and_then(|paths| paths.iter().find_map(serde_json::Value::as_str))
    .or_else(|| request.workspace.target_paths.first().map(String::as_str))
    .unwrap_or("workspace");
  let mut required_evidence = vec![
    "prior-semantic-patch-review".to_string(),
    "meaning-impact-diff".to_string(),
    "patch-decision-link".to_string(),
    "narrative-regression".to_string(),
    "new-targeted-repair-plan".to_string(),
  ];
  if let Some(impact_ref) = review
    .get("meaning_impact_diff")
    .and_then(|impact| impact.get("impact_ref"))
    .and_then(serde_json::Value::as_str)
  {
    required_evidence.push(format!("meaning-impact-ref:{impact_ref}"));
  }
  if let Some(link_ref) = review
    .get("patch_decision_link")
    .and_then(|link| link.get("link_ref"))
    .and_then(serde_json::Value::as_str)
  {
    required_evidence.push(format!("patch-decision-link-ref:{link_ref}"));
  }

  CodingAgentReplayedContextDemandSurface {
    artifact_family: "coding.context-demand",
    replay_item_ref: make_replayed_context_demand_ref(source_ref, review_ref, target_path),
    source_ref: source_ref.to_string(),
    source_family: source_family.to_string(),
    language: "unknown".to_string(),
    target_path: target_path.to_string(),
    demand_family: "semantic-review-followup-context-required".to_string(),
    required_evidence,
    request_boundary: "reuse-before-next-patch-proposal",
  }
}

fn build_context_demand_replay_requirements(
  replayed_context_demands: &[CodingAgentReplayedContextDemandSurface],
  semantic_review_refs: &[String],
  request: &CodingAgentRequestArtifact,
) -> Vec<String> {
  let mut requirements = BTreeSet::new();
  for demand in replayed_context_demands {
    requirements.insert(format!("satisfy-context-demand:{}", demand.demand_family));
    for evidence in demand.required_evidence.iter().take(6) {
      requirements.insert(format!("required-evidence:{evidence}"));
    }
  }
  if !semantic_review_refs.is_empty() {
    requirements.insert("review-prior-meaning-impact-before-new-patch".to_string());
    requirements.insert("link-new-patch-decision-to-prior-review".to_string());
  }
  if request.workspace.approved_commands.is_empty() {
    requirements.insert("declare-approved-command-before-next-mutation".to_string());
  }
  if requirements.is_empty() {
    requirements.insert("no-prior-context-demand-replayed".to_string());
  }
  requirements.into_iter().collect()
}

fn build_coding_agent_repair_recipe_replay(
  request: &CodingAgentRequestArtifact,
) -> CodingAgentRepairRecipeReplaySurface {
  let store_path = coding_memory_store_path_from_env();
  build_coding_agent_repair_recipe_replay_from_store_path(request, store_path.as_deref())
}

fn build_coding_agent_repair_recipe_replay_from_store_path(
  request: &CodingAgentRequestArtifact,
  store_path: Option<&Path>,
) -> CodingAgentRepairRecipeReplaySurface {
  let mut source_refs = Vec::new();
  if let Some(last_ref) = request.workspace.last_verification_ref.as_deref() {
    source_refs.push(last_ref.to_string());
  }
  if let Some(plan_ref) = request.workspace.current_plan_ref.as_deref() {
    source_refs.push(plan_ref.to_string());
  }
  source_refs.sort();
  source_refs.dedup();

  let mut source_artifact_refs: Vec<String> = Vec::new();
  let mut learning_card_refs: Vec<String> = Vec::new();
  let mut repair_candidates: Vec<CodingAgentRepairRecipeCandidateSurface> = Vec::new();
  #[cfg_attr(not(feature = "doghouse"), allow(unused_mut))]
  let mut store_status = "store-not-configured";

  #[cfg(feature = "doghouse")]
  if let Some(store_path) = store_path {
    match load_coding_agent_repair_recipe_sources_from_store(request, store_path) {
      Ok(sources) => {
        store_status = if sources.is_empty() {
          "store-open-no-matching-learning-cards"
        } else {
          "store-open-replayed-learning-cards"
        };
        for artifact in sources {
          source_artifact_refs.push(artifact.id.clone());
          collect_repair_recipe_candidates_from_artifact(
            &artifact,
            &mut learning_card_refs,
            &mut repair_candidates,
          );
        }
      }
      Err(err) => {
        store_status = "store-unavailable";
        source_refs.push(format!(
          "store-error:{}",
          truncate_diagnostic_message(&err.to_string())
        ));
      }
    }
  }
  #[cfg(not(feature = "doghouse"))]
  {
    // doghouse feature off: no coding memory store to replay repair recipes from.
    let _ = store_path;
  }

  repair_candidates.sort_by(|left, right| {
    left
      .candidate_ref
      .cmp(&right.candidate_ref)
      .then(left.source_ref.cmp(&right.source_ref))
  });
  repair_candidates.dedup_by(|left, right| left.candidate_ref == right.candidate_ref);
  learning_card_refs.sort();
  learning_card_refs.dedup();
  source_artifact_refs.sort();
  source_artifact_refs.dedup();

  let replay_status = if repair_candidates.is_empty() {
    store_status
  } else {
    "candidate-repair-recipes-replayed"
  };
  let replay_ref = make_repair_recipe_replay_ref(
    request,
    &source_refs,
    &source_artifact_refs,
    &learning_card_refs,
    &repair_candidates,
  );

  CodingAgentRepairRecipeReplaySurface {
    artifact_family: "coding.repair-recipe-replay",
    phase: "CAX.5h",
    replay_ref,
    replay_owner: "pnix-executor-graph::coding-agent::repair-recipe-replay",
    source_refs,
    source_artifact_refs,
    learning_card_refs,
    repair_candidates,
    replay_status,
    promotion_boundary: "candidate-only-not-patch-generator",
  }
}

#[cfg(feature = "doghouse")]
fn load_coding_agent_repair_recipe_sources_from_store(
  request: &CodingAgentRequestArtifact,
  store_path: &Path,
) -> Result<Vec<CodingMemoryArtifact>> {
  let store = DoghouseStore::open(DoghouseStoreConfig::new(store_path.to_path_buf()))
    .with_context(|| format!("open doghouse coding memory store {}", store_path.display()))?;
  let mut artifacts = Vec::new();
  for source_ref in [
    request.workspace.last_verification_ref.as_deref(),
    request.workspace.current_plan_ref.as_deref(),
  ]
  .into_iter()
  .flatten()
  {
    if let Some(artifact) =
      doghouse_core::store::read_coding_memory_artifact_at(store.path(), source_ref)
        .with_context(|| format!("load coding memory artifact {source_ref}"))?
    {
      artifacts.push(artifact);
    }
  }

  if request.workspace.last_verification_ref.is_some() {
    let repo_snapshot_ref = make_repo_snapshot_ref(&request.workspace);
    for family in ["coding.learning-card", "coding.verify-receipt"] {
      for artifact in
        doghouse_core::store::read_coding_memory_artifacts_by_family_at(store.path(), family)
          .with_context(|| format!("query {family} coding memory artifacts"))?
      {
        if artifact.repo_snapshot_ref.as_deref() == Some(repo_snapshot_ref.as_str())
          && coding_memory_artifact_targets_overlap(request, &artifact)
        {
          artifacts.push(artifact);
        }
      }
    }
  }

  artifacts.sort_by(|left, right| {
    right
      .stored_at_ms
      .cmp(&left.stored_at_ms)
      .then(left.id.cmp(&right.id))
  });
  artifacts.dedup_by(|left, right| left.id == right.id);
  artifacts.truncate(8);
  Ok(artifacts)
}

#[cfg(feature = "doghouse")]
fn collect_repair_recipe_candidates_from_artifact(
  artifact: &CodingMemoryArtifact,
  learning_card_refs: &mut Vec<String>,
  repair_candidates: &mut Vec<CodingAgentRepairRecipeCandidateSurface>,
) {
  let source_family = artifact
    .payload
    .get("artifact_family")
    .and_then(serde_json::Value::as_str)
    .unwrap_or(artifact.artifact_family.as_str());
  if source_family == "coding.learning-card" {
    if let Some(candidate) = repair_recipe_candidate_from_learning_card(
      artifact.id.as_str(),
      source_family,
      &artifact.payload,
    ) {
      learning_card_refs.push(candidate.source_ref.clone());
      repair_candidates.push(candidate);
    }
  }
  if let Some(card) = artifact.payload.get("learning_card") {
    if let Some(candidate) =
      repair_recipe_candidate_from_learning_card(artifact.id.as_str(), source_family, card)
    {
      if let Some(card_ref) = card
        .get("learning_card_ref")
        .and_then(serde_json::Value::as_str)
      {
        learning_card_refs.push(card_ref.to_string());
      }
      repair_candidates.push(candidate);
    }
  }
}

#[cfg(feature = "doghouse")]
fn repair_recipe_candidate_from_learning_card(
  source_ref: &str,
  source_family: &str,
  value: &serde_json::Value,
) -> Option<CodingAgentRepairRecipeCandidateSurface> {
  let learning_card_ref = value
    .get("learning_card_ref")
    .and_then(serde_json::Value::as_str)?;
  let trigger = value
    .get("trigger")
    .and_then(serde_json::Value::as_str)
    .unwrap_or("prior coding-agent learning card");
  let repair_pattern = value
    .get("repair_pattern")
    .and_then(serde_json::Value::as_str)
    .unwrap_or("bounded-edit");
  let verify_pattern = value
    .get("verify_pattern")
    .and_then(serde_json::Value::as_str)
    .unwrap_or("manual-verification-contract-required");
  let reuse_score = value
    .get("reuse_score")
    .and_then(serde_json::Value::as_f64)
    .unwrap_or(0.0);
  let required_context_refs = value
    .get("proof_refs")
    .and_then(serde_json::Value::as_array)
    .map(|refs| {
      refs
        .iter()
        .filter_map(serde_json::Value::as_str)
        .take(8)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
    })
    .unwrap_or_else(|| vec!["prior-learning-card-proof-ref-required".to_string()]);

  Some(CodingAgentRepairRecipeCandidateSurface {
    artifact_family: "coding.repair-recipe-candidate",
    candidate_ref: make_repair_recipe_candidate_ref(source_ref, learning_card_ref, repair_pattern),
    source_ref: learning_card_ref.to_string(),
    source_family: source_family.to_string(),
    trigger: trigger.to_string(),
    repair_pattern: repair_pattern.to_string(),
    verify_pattern: verify_pattern.to_string(),
    reuse_score,
    required_context_refs,
    promotion_boundary: "candidate-only-requires-current-context-review",
  })
}

fn build_coding_agent_semantic_patch_review(
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
  edit_family: &'static str,
  risk_class: &'static str,
  effect_classes: &[String],
  apply_result: Option<&CodingAgentApplyResultSurface>,
  context_demand_replay: &CodingAgentContextDemandReplaySurface,
  repair_recipe_replay: &CodingAgentRepairRecipeReplaySurface,
  generated_patch_candidate: Option<&CodingAgentGeneratedPatchCandidateSurface>,
  generated_patch_review_receipt: Option<&CodingAgentGeneratedPatchReviewReceiptSurface>,
  provider_feedback_request: Option<&CodingAgentProviderFeedbackRequestSurface>,
  feedback_retry_guard: Option<&CodingAgentFeedbackRetryGuardSurface>,
  apply_handoff_proof: Option<&CodingAgentApplyHandoffProofSurface>,
  promotion_boundary_receipt: Option<&CodingAgentPromotionBoundaryReceiptSurface>,
) -> CodingAgentSemanticPatchReviewSurface {
  let apply_status = apply_result
    .map(|result| result.apply_status)
    .unwrap_or("proposal-only-not-applied");
  let patch_input_ref = apply_result.and_then(|result| result.patch_input_ref.clone());
  let apply_artifact_ref = apply_result.map(|result| result.apply_artifact_ref.clone());
  let review_ref =
    make_semantic_patch_review_ref("semantic-patch-review", request, diff_ref, apply_status);
  let meaning_impact_diff = build_coding_agent_meaning_impact_diff(
    request,
    diff_ref,
    edit_family,
    risk_class,
    effect_classes,
    apply_status,
  );
  let patch_decision_link = build_coding_agent_patch_decision_link(
    request,
    diff_ref,
    apply_status,
    apply_result,
    &meaning_impact_diff,
    context_demand_replay,
    repair_recipe_replay,
    generated_patch_candidate,
    generated_patch_review_receipt,
    provider_feedback_request,
    feedback_retry_guard,
    apply_handoff_proof,
    promotion_boundary_receipt,
  );
  let narrative_regression = build_coding_agent_narrative_regression(
    request,
    risk_class,
    apply_status,
    apply_result,
    &meaning_impact_diff,
  );
  let mut proof_refs = vec![
    format!("semantic-review-ref:{}", review_ref),
    format!("meaning-impact-ref:{}", meaning_impact_diff.impact_ref),
    format!("patch-decision-link-ref:{}", patch_decision_link.link_ref),
    format!(
      "narrative-regression-ref:{}",
      narrative_regression.regression_ref
    ),
    format!("diff-ref:{}", diff_ref),
    format!("apply-status:{}", apply_status),
    format!(
      "context-demand-replay-ref:{}",
      context_demand_replay.replay_ref
    ),
    format!(
      "repair-recipe-replay-ref:{}",
      repair_recipe_replay.replay_ref
    ),
  ];
  if let Some(apply_artifact_ref) = apply_artifact_ref.as_deref() {
    proof_refs.push(format!("apply-artifact-ref:{}", apply_artifact_ref));
  }
  if let Some(candidate) = generated_patch_candidate {
    proof_refs.push(format!(
      "generated-patch-candidate-ref:{}",
      candidate.candidate_ref
    ));
    proof_refs.push(format!(
      "generated-patch-quarantine-status:{}",
      candidate.quarantine_status
    ));
    proof_refs.push(format!(
      "generated-patch-lineage-status:{}",
      candidate.lineage_status
    ));
    if let Some(feedback_request_ref) = candidate.source_provider_feedback_request_ref.as_deref() {
      proof_refs.push(format!(
        "provider-feedback-request-ref:{}",
        feedback_request_ref
      ));
    }
  }
  if let Some(review_receipt) = generated_patch_review_receipt {
    proof_refs.push(format!(
      "generated-patch-review-ref:{}",
      review_receipt.review_ref
    ));
    proof_refs.extend(review_receipt.context_demands.iter().take(4).map(|demand| {
      format!(
        "generated-patch-context-demand-ref:{}",
        demand.context_demand_ref
      )
    }));
  }
  if let Some(feedback_request) = provider_feedback_request {
    proof_refs.push(format!(
      "provider-feedback-request-ref:{}",
      feedback_request.request_ref
    ));
  }
  if let Some(retry_guard) = feedback_retry_guard {
    proof_refs.push(format!(
      "feedback-retry-guard-ref:{}",
      retry_guard.guard_ref
    ));
    proof_refs.push(format!(
      "feedback-retry-decision:{}",
      retry_guard.retry_decision
    ));
  }
  if let Some(handoff_proof) = apply_handoff_proof {
    proof_refs.push(format!(
      "apply-handoff-proof-ref:{}",
      handoff_proof.handoff_ref
    ));
    proof_refs.push(format!(
      "apply-handoff-status:{}",
      handoff_proof.handoff_status
    ));
  }
  if let Some(receipt) = promotion_boundary_receipt {
    proof_refs.push(format!(
      "promotion-boundary-receipt-ref:{}",
      receipt.receipt_ref
    ));
    proof_refs.push(format!("promotion-status:{}", receipt.promotion_status));
  }
  proof_refs.sort();
  proof_refs.dedup();

  CodingAgentSemanticPatchReviewSurface {
    artifact_family: "coding.semantic-patch-review",
    phase: "CAX.5f",
    review_ref,
    review_owner: "pnix-executor-graph::coding-agent::semantic-patch-review",
    diff_ref: diff_ref.to_string(),
    patch_input_ref,
    apply_artifact_ref,
    target_paths: request.workspace.target_paths.clone(),
    meaning_impact_diff,
    patch_decision_link,
    narrative_regression,
    proof_refs,
    review_status: semantic_patch_review_status(request, apply_status),
  }
}

fn build_coding_agent_meaning_impact_diff(
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
  edit_family: &'static str,
  risk_class: &'static str,
  effect_classes: &[String],
  apply_status: &'static str,
) -> CodingAgentMeaningImpactDiffSurface {
  let semantic_records = request
    .language_profile
    .semantic_records
    .iter()
    .filter(|record| semantic_review_targets_path(request, record.target_path.as_str()))
    .collect::<Vec<_>>();
  let effect_records = request
    .language_profile
    .effect_records
    .iter()
    .filter(|record| semantic_review_targets_path(request, record.target_path.as_str()))
    .collect::<Vec<_>>();
  let verify_targets = request
    .language_profile
    .verify_targets
    .iter()
    .filter(|target| semantic_review_targets_path(request, target.target_path.as_str()))
    .collect::<Vec<_>>();
  let mut meaning_classes = semantic_records
    .iter()
    .map(|record| record.meaning_class.to_string())
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect::<Vec<_>>();
  if meaning_classes.is_empty() {
    meaning_classes.push("unresolved-meaning-record".to_string());
  }
  let mut changed_symbol_refs = semantic_records
    .iter()
    .flat_map(|record| record.symbol_refs.iter().cloned())
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect::<Vec<_>>();
  if changed_symbol_refs.is_empty() {
    changed_symbol_refs = request
      .workspace
      .target_paths
      .iter()
      .map(|path| format!("file-anchor:{}", path))
      .collect();
  }
  let mut effect_refs = effect_records
    .iter()
    .map(|record| record.record_ref.clone())
    .chain(
      effect_classes
        .iter()
        .map(|effect| format!("effect-class:{effect}")),
    )
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect::<Vec<_>>();
  if effect_refs.is_empty() {
    effect_refs.push("effect-record:missing".to_string());
  }
  let mut verification_refs = verify_targets
    .iter()
    .map(|target| target.target_ref.clone())
    .chain(
      request
        .workspace
        .approved_commands
        .iter()
        .map(|command| format!("approved-command:{command}")),
    )
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect::<Vec<_>>();
  if verification_refs.is_empty() {
    verification_refs.push("manual-verification-contract-required".to_string());
  }
  let target_label = if request.workspace.target_paths.is_empty() {
    "unbounded target scope".to_string()
  } else {
    request.workspace.target_paths.join(", ")
  };

  CodingAgentMeaningImpactDiffSurface {
    artifact_family: "coding.meaning-impact-diff",
    impact_ref: make_semantic_patch_review_ref(
      "meaning-impact-diff",
      request,
      diff_ref,
      apply_status,
    ),
    diff_ref: diff_ref.to_string(),
    target_paths: request.workspace.target_paths.clone(),
    meaning_classes,
    changed_symbol_refs,
    effect_refs,
    verification_refs,
    impact_summary: format!(
      "{edit_family} patch candidate touches {target_label}; apply-status={apply_status}; risk-class={risk_class}"
    ),
    risk_signal: semantic_patch_risk_signal(risk_class, apply_status),
    promotion_boundary: "candidate-only-not-proof-without-review",
  }
}

fn build_coding_agent_patch_decision_link(
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
  apply_status: &'static str,
  apply_result: Option<&CodingAgentApplyResultSurface>,
  meaning_impact_diff: &CodingAgentMeaningImpactDiffSurface,
  context_demand_replay: &CodingAgentContextDemandReplaySurface,
  repair_recipe_replay: &CodingAgentRepairRecipeReplaySurface,
  generated_patch_candidate: Option<&CodingAgentGeneratedPatchCandidateSurface>,
  generated_patch_review_receipt: Option<&CodingAgentGeneratedPatchReviewReceiptSurface>,
  provider_feedback_request: Option<&CodingAgentProviderFeedbackRequestSurface>,
  feedback_retry_guard: Option<&CodingAgentFeedbackRetryGuardSurface>,
  apply_handoff_proof: Option<&CodingAgentApplyHandoffProofSurface>,
  promotion_boundary_receipt: Option<&CodingAgentPromotionBoundaryReceiptSurface>,
) -> CodingAgentPatchDecisionLinkSurface {
  let mut decision_refs = vec![
    format!("context-pack-ref:{}", request.context_pack.context_pack_ref),
    format!("apply-status:{}", apply_status),
    format!("risk-signal:{}", meaning_impact_diff.risk_signal),
  ];
  if let Some(plan_ref) = request.workspace.current_plan_ref.as_deref() {
    decision_refs.push(format!("current-plan-ref:{}", plan_ref));
  }
  if let Some(apply_result) = apply_result {
    decision_refs.push(format!(
      "apply-artifact-ref:{}",
      apply_result.apply_artifact_ref
    ));
    if let Some(rollback_handle_ref) = apply_result.rollback_handle_ref.as_deref() {
      decision_refs.push(format!("rollback-handle-ref:{}", rollback_handle_ref));
    }
  }
  decision_refs.sort();
  decision_refs.dedup();

  let mut evidence_refs = build_coding_agent_context_evidence_refs(request);
  evidence_refs.push(format!("diff-ref:{}", diff_ref));
  evidence_refs.push(format!(
    "meaning-impact-ref:{}",
    meaning_impact_diff.impact_ref
  ));
  evidence_refs.push(format!(
    "context-demand-replay-ref:{}",
    context_demand_replay.replay_ref
  ));
  evidence_refs.push(format!(
    "repair-recipe-replay-ref:{}",
    repair_recipe_replay.replay_ref
  ));
  evidence_refs.extend(meaning_impact_diff.effect_refs.iter().take(6).cloned());
  evidence_refs.extend(
    meaning_impact_diff
      .verification_refs
      .iter()
      .take(6)
      .cloned(),
  );
  if let Some(apply_result) = apply_result {
    evidence_refs.push(format!("apply-status:{}", apply_result.apply_status));
    evidence_refs.push(format!("rollback-class:{}", apply_result.rollback_class));
  }
  evidence_refs.extend(
    context_demand_replay
      .replayed_context_demands
      .iter()
      .take(6)
      .map(|demand| format!("replayed-context-demand:{}", demand.replay_item_ref)),
  );
  evidence_refs.extend(
    context_demand_replay
      .semantic_review_refs
      .iter()
      .take(4)
      .map(|review_ref| format!("prior-semantic-review-ref:{review_ref}")),
  );
  evidence_refs.extend(
    repair_recipe_replay
      .repair_candidates
      .iter()
      .take(4)
      .map(|candidate| format!("repair-recipe-candidate:{}", candidate.candidate_ref)),
  );
  if let Some(candidate) = generated_patch_candidate {
    evidence_refs.push(format!(
      "generated-patch-candidate-ref:{}",
      candidate.candidate_ref
    ));
    evidence_refs.push(format!(
      "generated-patch-quarantine-status:{}",
      candidate.quarantine_status
    ));
    evidence_refs.push(format!(
      "generated-patch-lineage-status:{}",
      candidate.lineage_status
    ));
    if let Some(feedback_request_ref) = candidate.source_provider_feedback_request_ref.as_deref() {
      evidence_refs.push(format!(
        "provider-feedback-request-ref:{}",
        feedback_request_ref
      ));
    }
    evidence_refs.extend(
      candidate
        .rejected_target_paths
        .iter()
        .take(4)
        .map(|target| format!("generated-patch-rejected-target:{target}")),
    );
  }
  if let Some(review_receipt) = generated_patch_review_receipt {
    evidence_refs.push(format!(
      "generated-patch-review-ref:{}",
      review_receipt.review_ref
    ));
    evidence_refs.extend(review_receipt.context_demands.iter().take(4).map(|demand| {
      format!(
        "generated-patch-context-demand:{}",
        demand.context_demand_ref
      )
    }));
  }
  if let Some(feedback_request) = provider_feedback_request {
    evidence_refs.push(format!(
      "provider-feedback-request-ref:{}",
      feedback_request.request_ref
    ));
    evidence_refs.extend(
      feedback_request
        .feedback_packets
        .iter()
        .take(4)
        .map(|packet| format!("provider-feedback-packet:{}", packet.packet_ref)),
    );
  }
  if let Some(retry_guard) = feedback_retry_guard {
    evidence_refs.push(format!(
      "feedback-retry-guard-ref:{}",
      retry_guard.guard_ref
    ));
    evidence_refs.push(format!(
      "feedback-retry-decision:{}",
      retry_guard.retry_decision
    ));
  }
  if let Some(handoff_proof) = apply_handoff_proof {
    evidence_refs.push(format!(
      "apply-handoff-proof-ref:{}",
      handoff_proof.handoff_ref
    ));
    evidence_refs.push(format!(
      "apply-handoff-status:{}",
      handoff_proof.handoff_status
    ));
    evidence_refs.push(format!(
      "apply-patch-input-ref:{}",
      handoff_proof.apply_patch_input_ref
    ));
  }
  if let Some(receipt) = promotion_boundary_receipt {
    evidence_refs.push(format!(
      "promotion-boundary-receipt-ref:{}",
      receipt.receipt_ref
    ));
    evidence_refs.push(format!("promotion-status:{}", receipt.promotion_status));
  }
  evidence_refs.sort();
  evidence_refs.dedup();

  CodingAgentPatchDecisionLinkSurface {
    artifact_family: "coding.patch-decision-link",
    link_ref: make_semantic_patch_review_ref(
      "patch-decision-link",
      request,
      diff_ref,
      apply_status,
    ),
    diff_ref: diff_ref.to_string(),
    decision_family: semantic_patch_decision_family(apply_status),
    decision_refs,
    evidence_refs,
    policy_boundary: "candidate-only-not-architecture-decision",
  }
}

fn build_coding_agent_narrative_regression(
  request: &CodingAgentRequestArtifact,
  risk_class: &'static str,
  apply_status: &'static str,
  apply_result: Option<&CodingAgentApplyResultSurface>,
  meaning_impact_diff: &CodingAgentMeaningImpactDiffSurface,
) -> CodingAgentNarrativeRegressionSurface {
  let mut risk_notes = Vec::new();
  if request.workspace.approved_commands.is_empty() {
    risk_notes.push("verification-contract-missing".to_string());
  }
  if meaning_impact_diff
    .meaning_classes
    .iter()
    .any(|class| class == "unresolved-meaning-record")
  {
    risk_notes.push("semantic-record-missing-for-target".to_string());
  }
  if matches!(apply_status, "blocked" | "failed") {
    risk_notes.push(format!("mutation-not-accepted:{apply_status}"));
  }
  if risk_class.starts_with("high") {
    risk_notes.push(format!("risk-class:{risk_class}"));
  }
  if let Some(apply_error) = apply_result.and_then(|result| result.error.as_deref()) {
    risk_notes.push(format!(
      "apply-error-summary:{}",
      truncate_diagnostic_message(apply_error)
    ));
  }
  if risk_notes.is_empty() {
    risk_notes.push("no-narrative-regression-detected-by-record-producer".to_string());
  }

  CodingAgentNarrativeRegressionSurface {
    artifact_family: "coding.narrative-regression",
    regression_ref: make_semantic_patch_review_ref(
      "narrative-regression",
      request,
      meaning_impact_diff.diff_ref.as_str(),
      apply_status,
    ),
    narrative_status: semantic_patch_narrative_status(request, apply_status),
    checked_dimensions: vec![
      "human-readable-rationale",
      "codebase-coherence",
      "duplicate-risk",
      "rollbackability",
      "verification-contract",
      "language-adapter-boundary",
    ],
    risk_notes,
    proof_boundary: "review-candidate-not-promotion",
  }
}

fn attach_semantic_patch_review_to_apply_result(
  apply_result: &mut CodingAgentApplyResultSurface,
  semantic_review: &CodingAgentSemanticPatchReviewSurface,
) {
  apply_result.proof_refs.push(format!(
    "semantic-review-ref:{}",
    semantic_review.review_ref
  ));
  apply_result.proof_refs.push(format!(
    "meaning-impact-ref:{}",
    semantic_review.meaning_impact_diff.impact_ref
  ));
  apply_result.proof_refs.push(format!(
    "patch-decision-link-ref:{}",
    semantic_review.patch_decision_link.link_ref
  ));
  apply_result.proof_refs.sort();
  apply_result.proof_refs.dedup();
}

fn attach_apply_handoff_proof_to_apply_result(
  apply_result: &mut CodingAgentApplyResultSurface,
  handoff_proof: &CodingAgentApplyHandoffProofSurface,
) {
  apply_result.proof_refs.extend([
    format!("apply-handoff-proof-ref:{}", handoff_proof.handoff_ref),
    format!("apply-handoff-status:{}", handoff_proof.handoff_status),
    format!(
      "generated-patch-candidate-ref:{}",
      handoff_proof.candidate_ref
    ),
    format!(
      "generated-patch-review-ref:{}",
      handoff_proof.candidate_review_ref
    ),
  ]);
  apply_result.proof_refs.sort();
  apply_result.proof_refs.dedup();
}

fn attach_promotion_boundary_receipt_to_apply_result(
  apply_result: &mut CodingAgentApplyResultSurface,
  receipt: &CodingAgentPromotionBoundaryReceiptSurface,
) {
  apply_result.proof_refs.extend([
    format!("promotion-boundary-receipt-ref:{}", receipt.receipt_ref),
    format!("promotion-status:{}", receipt.promotion_status),
    "apply-result-is-not-promotion-proof".to_string(),
  ]);
  apply_result.proof_refs.sort();
  apply_result.proof_refs.dedup();
}

fn semantic_review_targets_path(
  request: &CodingAgentRequestArtifact,
  candidate_path: &str,
) -> bool {
  request.workspace.target_paths.is_empty()
    || request
      .workspace
      .target_paths
      .iter()
      .any(|target| target == candidate_path)
}

fn semantic_patch_review_status(
  request: &CodingAgentRequestArtifact,
  apply_status: &'static str,
) -> &'static str {
  if matches!(apply_status, "blocked" | "failed") {
    "held-semantic-review-required"
  } else if request.workspace.approved_commands.is_empty() {
    "candidate-review-missing-verify-contract"
  } else {
    "candidate-review-not-promoted"
  }
}

fn semantic_patch_risk_signal(
  risk_class: &'static str,
  apply_status: &'static str,
) -> &'static str {
  if matches!(apply_status, "blocked" | "failed") {
    "mutation-blocked"
  } else if risk_class.starts_with("high") {
    "high-risk-review-required"
  } else {
    "bounded-review-required"
  }
}

fn semantic_patch_decision_family(apply_status: &'static str) -> &'static str {
  match apply_status {
    "applied" => "explicit-apply-linked-to-review",
    "validated-not-applied" => "dry-run-apply-linked-to-review",
    "blocked" | "failed" => "patch-held-by-apply-result",
    _ => "proposal-only-review",
  }
}

fn semantic_patch_narrative_status(
  request: &CodingAgentRequestArtifact,
  apply_status: &'static str,
) -> &'static str {
  if matches!(apply_status, "blocked" | "failed") {
    "regression-risk-held"
  } else if request.workspace.approved_commands.is_empty() {
    "verification-story-incomplete"
  } else {
    "reviewed-candidate-no-regression-proof"
  }
}

fn parse_coding_agent_unified_patch(
  patch_text: &str,
) -> std::result::Result<Vec<ParsedUnifiedFilePatch>, String> {
  if patch_text.contains("GIT binary patch") || patch_text.contains("Binary files ") {
    return Err("binary patches are not supported by coding-agent patch apply".to_string());
  }

  let lines = split_preserving_line_endings(patch_text);
  let mut patches = Vec::new();
  let mut index = 0usize;
  while index < lines.len() {
    let line = lines[index].as_str();
    if !line.starts_with("--- ") {
      index += 1;
      continue;
    }
    let old_path = parse_unified_patch_path(line, "--- ")?;
    index += 1;
    if index >= lines.len() || !lines[index].starts_with("+++ ") {
      return Err("unified patch missing +++ path after --- path".to_string());
    }
    let new_path = parse_unified_patch_path(lines[index].as_str(), "+++ ")?;
    index += 1;
    let target_path =
      normalize_unified_patch_target_path(old_path.as_deref(), new_path.as_deref())?;
    let mut hunks = Vec::new();
    while index < lines.len() {
      let hunk_header = lines[index].as_str();
      if hunk_header.starts_with("--- ") {
        break;
      }
      if !hunk_header.starts_with("@@ ") {
        index += 1;
        continue;
      }
      let (old_start, _old_count) = parse_unified_hunk_header(hunk_header)?;
      index += 1;
      let mut hunk_lines = Vec::new();
      while index < lines.len() {
        let patch_line = lines[index].as_str();
        if patch_line.starts_with("@@ ") || patch_line.starts_with("--- ") {
          break;
        }
        if patch_line.starts_with("\\ No newline at end of file") {
          index += 1;
          continue;
        }
        let Some((prefix, rest)) = patch_line.split_at_checked(1) else {
          index += 1;
          continue;
        };
        match prefix {
          " " => hunk_lines.push(ParsedUnifiedLine::Context(rest.to_string())),
          "-" => hunk_lines.push(ParsedUnifiedLine::Remove(rest.to_string())),
          "+" => hunk_lines.push(ParsedUnifiedLine::Add(rest.to_string())),
          _ => {}
        }
        index += 1;
      }
      hunks.push(ParsedUnifiedHunk {
        old_start,
        lines: hunk_lines,
      });
    }
    if hunks.is_empty() {
      return Err(format!("unified patch for {} has no hunks", target_path));
    }
    patches.push(ParsedUnifiedFilePatch {
      old_path,
      target_path,
      hunks,
    });
  }

  if patches.is_empty() {
    return Err("no unified diff file patches found".to_string());
  }
  Ok(patches)
}

fn split_preserving_line_endings(text: &str) -> Vec<String> {
  if text.is_empty() {
    return Vec::new();
  }
  text
    .split_inclusive('\n')
    .map(ToString::to_string)
    .chain(if text.ends_with('\n') {
      None
    } else {
      Some(String::new())
    })
    .filter(|line| !line.is_empty())
    .collect()
}

fn parse_unified_patch_path(
  line: &str,
  prefix: &str,
) -> std::result::Result<Option<String>, String> {
  let raw = line
    .strip_prefix(prefix)
    .ok_or_else(|| format!("unified patch path missing prefix {}", prefix))?
    .trim();
  let path = raw.split_whitespace().next().unwrap_or(raw);
  if path == "/dev/null" {
    return Ok(None);
  }
  Ok(Some(strip_diff_path_prefix(path)))
}

fn strip_diff_path_prefix(path: &str) -> String {
  path
    .strip_prefix("a/")
    .or_else(|| path.strip_prefix("b/"))
    .unwrap_or(path)
    .to_string()
}

fn normalize_unified_patch_target_path(
  old_path: Option<&str>,
  new_path: Option<&str>,
) -> std::result::Result<String, String> {
  if new_path.is_none() {
    return Err("delete-only patches are not supported in this bounded apply lane".to_string());
  }
  let target = new_path.or(old_path).unwrap_or_default();
  if target.trim().is_empty() {
    return Err("unified patch target path is empty".to_string());
  }
  validate_relative_workspace_path(target)?;
  Ok(path_to_slash(Path::new(target)))
}

fn parse_unified_hunk_header(line: &str) -> std::result::Result<(usize, usize), String> {
  let header = line.trim();
  let end = header[3..]
    .find("@@")
    .map(|idx| idx + 3)
    .ok_or_else(|| format!("invalid unified hunk header: {}", header))?;
  let range_part = &header[3..end];
  let old_range = range_part
    .split_whitespace()
    .find(|part| part.starts_with('-'))
    .ok_or_else(|| format!("invalid unified hunk old range: {}", header))?;
  parse_unified_range(old_range.trim_start_matches('-'))
}

fn parse_unified_range(raw: &str) -> std::result::Result<(usize, usize), String> {
  let (start, count) = raw.split_once(',').unwrap_or((raw, "1"));
  let start = start
    .parse::<usize>()
    .map_err(|err| format!("invalid unified hunk start '{}': {}", start, err))?;
  let count = count
    .parse::<usize>()
    .map_err(|err| format!("invalid unified hunk count '{}': {}", count, err))?;
  Ok((start, count))
}

fn prepare_coding_agent_patch_application(
  request: &CodingAgentRequestArtifact,
  patches: &[ParsedUnifiedFilePatch],
) -> std::result::Result<Vec<PreparedPatchFileApply>, String> {
  if request.workspace.target_paths.is_empty() {
    return Err("patch apply requires at least one --target-path".to_string());
  }
  let allowed_targets = request
    .workspace
    .target_paths
    .iter()
    .map(|path| path_to_slash(Path::new(path)))
    .collect::<BTreeSet<_>>();
  let forbidden_paths = request
    .workspace
    .forbidden_paths
    .iter()
    .map(|path| path_to_slash(Path::new(path)))
    .collect::<Vec<_>>();
  let workspace_root = request
    .workspace
    .repo_root
    .as_deref()
    .unwrap_or(request.workspace.cwd.as_str());
  let workspace_root_path = Path::new(workspace_root);
  let mut prepared = Vec::new();

  for patch in patches {
    validate_relative_workspace_path(&patch.target_path)?;
    if !allowed_targets.contains(&patch.target_path) {
      return Err(format!(
        "patch target {} is outside declared --target-path set",
        patch.target_path
      ));
    }
    if forbidden_paths.iter().any(|forbidden| {
      patch.target_path == *forbidden || patch.target_path.starts_with(&format!("{}/", forbidden))
    }) {
      return Err(format!("patch target {} is forbidden", patch.target_path));
    }
    let absolute_path = workspace_root_path.join(&patch.target_path);
    let before_content = fs::read_to_string(&absolute_path).ok();
    if before_content.is_none() && patch.old_path.is_some() {
      return Err(format!(
        "patch target {} does not exist for update patch",
        patch.target_path
      ));
    }
    let after_content =
      apply_unified_file_patch_to_content(before_content.as_deref().unwrap_or(""), patch)?;
    let before_snapshot_ref = before_content
      .as_ref()
      .map(|content| make_file_snapshot_ref("before", &patch.target_path, content.as_bytes()));
    let after_snapshot_ref =
      make_file_snapshot_ref("after", &patch.target_path, after_content.as_bytes());
    let before_len = before_content
      .as_ref()
      .map(|content| content.len())
      .unwrap_or(0);
    let byte_delta = after_content.len() as i64 - before_len as i64;
    prepared.push(PreparedPatchFileApply {
      path: patch.target_path.clone(),
      absolute_path,
      after_content,
      before_snapshot_ref,
      after_snapshot_ref,
      byte_delta,
    });
  }

  Ok(prepared)
}

fn validate_relative_workspace_path(path: &str) -> std::result::Result<(), String> {
  let path = Path::new(path);
  if path.is_absolute() {
    return Err(format!(
      "absolute patch path {} is not allowed",
      path.display()
    ));
  }
  for component in path.components() {
    if matches!(
      component,
      std::path::Component::ParentDir
        | std::path::Component::RootDir
        | std::path::Component::Prefix(_)
    ) {
      return Err(format!(
        "unsafe patch path {} is not allowed",
        path.display()
      ));
    }
  }
  Ok(())
}

fn apply_unified_file_patch_to_content(
  before_content: &str,
  patch: &ParsedUnifiedFilePatch,
) -> std::result::Result<String, String> {
  let mut content_lines = split_preserving_line_endings(before_content);
  let mut offset: isize = 0;
  for hunk in &patch.hunks {
    let base_index = hunk.old_start.saturating_sub(1) as isize + offset;
    if base_index < 0 {
      return Err(format!(
        "hunk for {} points before file start",
        patch.target_path
      ));
    }
    let start = base_index as usize;
    let mut old_segment = Vec::new();
    let mut new_segment = Vec::new();
    for line in &hunk.lines {
      match line {
        ParsedUnifiedLine::Context(value) => {
          old_segment.push(value.clone());
          new_segment.push(value.clone());
        }
        ParsedUnifiedLine::Remove(value) => old_segment.push(value.clone()),
        ParsedUnifiedLine::Add(value) => new_segment.push(value.clone()),
      }
    }
    let end = start.saturating_add(old_segment.len());
    if end > content_lines.len() || content_lines[start..end] != old_segment[..] {
      return Err(format!(
        "hunk context mismatch for {} at old line {}",
        patch.target_path, hunk.old_start
      ));
    }
    content_lines.splice(start..end, new_segment.clone());
    offset += new_segment.len() as isize - old_segment.len() as isize;
  }
  Ok(content_lines.concat())
}

fn make_file_snapshot_ref(kind: &str, path: &str, bytes: &[u8]) -> String {
  let mut hasher = Sha256::new();
  hasher.update(kind.as_bytes());
  hasher.update(b"\n--path--\n");
  hasher.update(path.as_bytes());
  hasher.update(b"\n--bytes--\n");
  hasher.update(bytes);
  format!("coding.file-snapshot::{}::{:x}", kind, hasher.finalize())
}

fn build_patch_apply_proof_refs(
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
  apply_artifact_ref: &str,
  rollback_handle_ref: Option<&str>,
  inverse_plan_ref: Option<&str>,
  file_results: &[CodingAgentPatchFileApplyRecord],
) -> Vec<String> {
  let mut proof_refs = build_coding_agent_context_evidence_refs(request);
  proof_refs.push(format!("diff-ref:{}", diff_ref));
  proof_refs.push(format!("apply-artifact-ref:{}", apply_artifact_ref));
  for file in file_results.iter().take(8) {
    proof_refs.push(format!("apply-file:{}:{}", file.path, file.status));
    if let Some(after) = file.after_snapshot_ref.as_deref() {
      proof_refs.push(format!("after-snapshot-ref:{}", after));
    }
  }
  if let Some(rollback_handle_ref) = rollback_handle_ref {
    proof_refs.push(format!("rollback-handle-ref:{}", rollback_handle_ref));
  }
  if let Some(inverse_plan_ref) = inverse_plan_ref {
    proof_refs.push(format!("inverse-plan-ref:{}", inverse_plan_ref));
  }
  proof_refs.sort();
  proof_refs.dedup();
  proof_refs
}

fn build_coding_agent_verify_receipt(
  args: &Args,
  request: CodingAgentRequestArtifact,
  request_artifact_ref: Option<String>,
) -> CodingAgentVerifyReceiptArtifact {
  let target_paths = request.workspace.target_paths.clone();
  let target_commands = if request.workspace.approved_commands.is_empty() {
    vec!["manual-verification-contract-required".to_string()]
  } else {
    request.workspace.approved_commands.clone()
  };
  let repo_snapshot_ref = make_repo_snapshot_ref(&request.workspace);
  let snapshot_suffix = repo_snapshot_ref
    .rsplit_once("::")
    .map(|(_, suffix)| suffix)
    .unwrap_or(repo_snapshot_ref.as_str());
  let before_artifact_ref = format!("coding.verify-snapshot::before::{}", snapshot_suffix);
  let diff_ref = make_verify_diff_ref(
    &repo_snapshot_ref,
    &target_paths,
    &target_commands,
    request.workspace.current_plan_ref.as_deref(),
  );
  let after_artifact_ref = make_after_verify_artifact_ref(&diff_ref);
  let proof_refs = build_verify_proof_refs(
    &request,
    &repo_snapshot_ref,
    &before_artifact_ref,
    &after_artifact_ref,
    &diff_ref,
    &target_commands,
  );
  let note = if request.workspace.approved_commands.is_empty() {
    "verify-receipt artifact를 생성했지만 approved command가 없어 execution contract는 차단된 상태다."
  } else if args.agent_verify_out.is_some() {
    "typed verify receipt artifact를 생성했고 actual command execution lane은 아직 열지 않았다."
  } else {
    "stdout에 typed verify receipt surface만 노출했고 actual command execution lane은 아직 열지 않았다."
  };
  let execution_result =
    build_coding_agent_execution_result(&repo_snapshot_ref, &diff_ref, &target_commands);
  let learning_card = build_coding_agent_learning_card(
    &request,
    &repo_snapshot_ref,
    &diff_ref,
    &execution_result,
    &proof_refs,
  );

  CodingAgentVerifyReceiptArtifact {
    artifact_family: "coding.verify-receipt",
    phase: "CAX.3b-partial",
    surface: "pnix coding-agent",
    verb: "verify",
    verified_at_ms: current_time_ms(),
    request_artifact_ref,
    repo_snapshot_ref,
    target_paths,
    target_commands,
    before_artifact_ref,
    after_artifact_ref,
    diff_ref,
    execution_result,
    diagnostic_records: Vec::new(),
    failure_pattern_matches: Vec::new(),
    context_demands: Vec::new(),
    promotion_boundary_join_receipt: None,
    status: CodingAgentStatusSurface {
      progress_status: if request.workspace.approved_commands.is_empty() {
        "검증계약부족"
      } else {
        "검증영수증준비완료"
      },
      result_status: if request.workspace.approved_commands.is_empty() {
        "차단"
      } else {
        "부분완료"
      },
      note: note.to_string(),
    },
    proof_refs,
    learning_card,
    request,
  }
}

fn build_coding_agent_execution_result(
  repo_snapshot_ref: &str,
  diff_ref: &str,
  target_commands: &[String],
) -> CodingAgentExecutionResultSurface {
  let observed_at_ms = current_time_ms();
  CodingAgentExecutionResultSurface {
    artifact_family: "coding.execution-result",
    phase: "CAX.3b-partial",
    observed_at_ms,
    execution_result_ref: make_execution_result_ref(
      repo_snapshot_ref,
      diff_ref,
      target_commands,
      observed_at_ms,
    ),
    execution_status: "not-run-command-execution-closed",
    command_refs: target_commands.to_vec(),
    command_results: Vec::new(),
    raw_result_refs: vec!["raw-command-log:not-created".to_string()],
    exit_code: None,
  }
}

fn run_coding_agent_verify_commands(
  request: &CodingAgentRequestArtifact,
  repo_snapshot_ref: &str,
  diff_ref: &str,
  target_commands: &[String],
) -> CodingAgentExecutionResultSurface {
  let observed_at_ms = current_time_ms();
  let cwd = request
    .workspace
    .repo_root
    .as_deref()
    .unwrap_or(request.workspace.cwd.as_str());
  let command_results = target_commands
    .iter()
    .map(|command| run_coding_agent_verify_command(command, cwd))
    .collect::<Vec<_>>();
  let execution_status = summarize_coding_agent_execution_status(&command_results);
  let raw_result_refs = build_execution_raw_result_refs(&command_results);
  let exit_code = summarize_coding_agent_exit_code(&command_results);

  CodingAgentExecutionResultSurface {
    artifact_family: "coding.execution-result",
    phase: "CAX.3b",
    observed_at_ms,
    execution_result_ref: make_execution_result_ref(
      repo_snapshot_ref,
      diff_ref,
      target_commands,
      observed_at_ms,
    ),
    execution_status,
    command_refs: target_commands.to_vec(),
    command_results,
    raw_result_refs,
    exit_code,
  }
}

fn run_coding_agent_verify_command(
  command_ref: &str,
  cwd: &str,
) -> CodingAgentCommandExecutionRecord {
  let started = Instant::now();
  let parsed = match parse_coding_agent_approved_command(command_ref) {
    Ok(parsed) => parsed,
    Err(err) => {
      return CodingAgentCommandExecutionRecord {
        command_ref: command_ref.to_string(),
        program: String::new(),
        args: Vec::new(),
        cwd: cwd.to_string(),
        status: "blocked",
        exit_code: None,
        duration_ms: elapsed_ms(started),
        stdout_ref: command_output_ref("stdout", command_ref, &[]),
        stderr_ref: command_output_ref("stderr", command_ref, err.as_bytes()),
        stdout_preview: String::new(),
        stderr_preview: String::new(),
        error: Some(err),
      };
    }
  };

  let program = parsed[0].clone();
  let command_args = parsed.iter().skip(1).cloned().collect::<Vec<_>>();
  let spawn_result = Command::new(&program)
    .args(&command_args)
    .current_dir(cwd)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn();
  let mut child = match spawn_result {
    Ok(child) => child,
    Err(err) => {
      return CodingAgentCommandExecutionRecord {
        command_ref: command_ref.to_string(),
        program,
        args: command_args,
        cwd: cwd.to_string(),
        status: "spawn-error",
        exit_code: None,
        duration_ms: elapsed_ms(started),
        stdout_ref: command_output_ref("stdout", command_ref, &[]),
        stderr_ref: command_output_ref("stderr", command_ref, err.to_string().as_bytes()),
        stdout_preview: String::new(),
        stderr_preview: String::new(),
        error: Some(err.to_string()),
      };
    }
  };

  loop {
    match child.try_wait() {
      Ok(Some(_status)) => {
        return match child.wait_with_output() {
          Ok(output) => {
            let status = if output.status.success() {
              "passed"
            } else {
              "failed"
            };
            CodingAgentCommandExecutionRecord {
              command_ref: command_ref.to_string(),
              program,
              args: command_args,
              cwd: cwd.to_string(),
              status,
              exit_code: output.status.code(),
              duration_ms: elapsed_ms(started),
              stdout_ref: command_output_ref("stdout", command_ref, &output.stdout),
              stderr_ref: command_output_ref("stderr", command_ref, &output.stderr),
              stdout_preview: command_output_preview(&output.stdout),
              stderr_preview: command_output_preview(&output.stderr),
              error: None,
            }
          }
          Err(err) => CodingAgentCommandExecutionRecord {
            command_ref: command_ref.to_string(),
            program,
            args: command_args,
            cwd: cwd.to_string(),
            status: "wait-error",
            exit_code: None,
            duration_ms: elapsed_ms(started),
            stdout_ref: command_output_ref("stdout", command_ref, &[]),
            stderr_ref: command_output_ref("stderr", command_ref, err.to_string().as_bytes()),
            stdout_preview: String::new(),
            stderr_preview: String::new(),
            error: Some(err.to_string()),
          },
        };
      }
      Ok(None) => {
        if started.elapsed() >= Duration::from_millis(CODING_AGENT_VERIFY_COMMAND_TIMEOUT_MS) {
          let _ = child.kill();
          let output = child.wait_with_output();
          return match output {
            Ok(output) => CodingAgentCommandExecutionRecord {
              command_ref: command_ref.to_string(),
              program,
              args: command_args,
              cwd: cwd.to_string(),
              status: "timed-out",
              exit_code: output.status.code(),
              duration_ms: elapsed_ms(started),
              stdout_ref: command_output_ref("stdout", command_ref, &output.stdout),
              stderr_ref: command_output_ref("stderr", command_ref, &output.stderr),
              stdout_preview: command_output_preview(&output.stdout),
              stderr_preview: command_output_preview(&output.stderr),
              error: Some(format!(
                "command timed out after {}ms",
                CODING_AGENT_VERIFY_COMMAND_TIMEOUT_MS
              )),
            },
            Err(err) => CodingAgentCommandExecutionRecord {
              command_ref: command_ref.to_string(),
              program,
              args: command_args,
              cwd: cwd.to_string(),
              status: "timed-out",
              exit_code: None,
              duration_ms: elapsed_ms(started),
              stdout_ref: command_output_ref("stdout", command_ref, &[]),
              stderr_ref: command_output_ref("stderr", command_ref, err.to_string().as_bytes()),
              stdout_preview: String::new(),
              stderr_preview: String::new(),
              error: Some(format!(
                "command timed out after {}ms; wait failed: {}",
                CODING_AGENT_VERIFY_COMMAND_TIMEOUT_MS, err
              )),
            },
          };
        }
        std::thread::sleep(Duration::from_millis(20));
      }
      Err(err) => {
        let _ = child.kill();
        let _ = child.wait();
        return CodingAgentCommandExecutionRecord {
          command_ref: command_ref.to_string(),
          program,
          args: command_args,
          cwd: cwd.to_string(),
          status: "wait-error",
          exit_code: None,
          duration_ms: elapsed_ms(started),
          stdout_ref: command_output_ref("stdout", command_ref, &[]),
          stderr_ref: command_output_ref("stderr", command_ref, err.to_string().as_bytes()),
          stdout_preview: String::new(),
          stderr_preview: String::new(),
          error: Some(err.to_string()),
        };
      }
    }
  }
}

fn attach_coding_agent_verify_execution_result(
  receipt: &mut CodingAgentVerifyReceiptArtifact,
  execution_result: CodingAgentExecutionResultSurface,
) {
  let (diagnostic_records, failure_pattern_matches, context_demands) =
    build_verify_execution_diagnostic_bridge(&receipt.request, &execution_result);
  receipt.proof_refs.push(format!(
    "execution-result-ref:{}",
    execution_result.execution_result_ref
  ));
  receipt.proof_refs.push(format!(
    "execution-status:{}",
    execution_result.execution_status
  ));
  for command in execution_result.command_results.iter().take(4) {
    receipt.proof_refs.push(format!(
      "execution-command:{}:{}",
      command.command_ref, command.status
    ));
  }
  for diagnostic in diagnostic_records.iter().take(4) {
    receipt
      .proof_refs
      .push(format!("diagnostic-ref:{}", diagnostic.diagnostic_ref));
  }
  for demand in context_demands.iter().take(4) {
    receipt
      .proof_refs
      .push(format!("context-demand-ref:{}", demand.context_demand_ref));
  }
  receipt.proof_refs.sort();
  receipt.proof_refs.dedup();
  receipt.status = status_for_coding_agent_execution_result(&execution_result);
  receipt.execution_result = execution_result;
  receipt.diagnostic_records = diagnostic_records;
  receipt.failure_pattern_matches = failure_pattern_matches;
  receipt.context_demands = context_demands;
  receipt.learning_card = build_coding_agent_learning_card(
    &receipt.request,
    &receipt.repo_snapshot_ref,
    &receipt.diff_ref,
    &receipt.execution_result,
    &receipt.proof_refs,
  );
}

fn attach_coding_agent_promotion_boundary_join_receipt(
  receipt: &mut CodingAgentVerifyReceiptArtifact,
) {
  let Some(join_receipt) = build_coding_agent_promotion_boundary_join_receipt(receipt) else {
    return;
  };
  let mut learning_proof_refs = join_receipt.proof_refs.clone();
  learning_proof_refs.extend(receipt.proof_refs.iter().cloned());
  receipt
    .proof_refs
    .extend(join_receipt.proof_refs.iter().cloned());
  receipt.proof_refs.sort();
  receipt.proof_refs.dedup();
  receipt.learning_card = build_coding_agent_learning_card(
    &receipt.request,
    &receipt.repo_snapshot_ref,
    &receipt.diff_ref,
    &receipt.execution_result,
    &learning_proof_refs,
  );
  receipt.promotion_boundary_join_receipt = Some(join_receipt);
}

fn build_coding_agent_promotion_boundary_join_receipt(
  receipt: &CodingAgentVerifyReceiptArtifact,
) -> Option<CodingAgentPromotionBoundaryJoinReceiptSurface> {
  let source_promotion_boundary_receipt_ref =
    receipt.request.workspace.promotion_boundary_ref.clone()?;
  let source_apply_artifact_ref = receipt
    .request
    .workspace
    .source_apply_artifact_ref
    .clone()?;
  let source_handoff_ref = receipt.request.workspace.source_handoff_ref.clone();
  let join_status = match receipt.execution_result.execution_status {
    "passed" => "joined-verify-passed-awaiting-human-judgement",
    "not-run-command-execution-closed" => "join-held-verification-not-run",
    "blocked" => "join-held-verification-blocked",
    _ => "join-held-verification-failed",
  };
  let join_ref = make_promotion_boundary_join_receipt_ref(
    source_promotion_boundary_receipt_ref.as_str(),
    source_apply_artifact_ref.as_str(),
    source_handoff_ref.as_deref(),
    receipt.diff_ref.as_str(),
    receipt.execution_result.execution_result_ref.as_str(),
    join_status,
  );
  let mut required_next_artifacts = BTreeSet::from([
    "human-judgement-boundary-before-promotion".to_string(),
    "learning-card-review-before-promotion".to_string(),
  ]);
  if receipt.execution_result.execution_status != "passed" {
    required_next_artifacts.insert("repair-patch-proposal-before-promotion".to_string());
  }
  let mut proof_refs = vec![
    format!("promotion-boundary-join-receipt-ref:{}", join_ref),
    format!(
      "promotion-boundary-receipt-ref:{}",
      source_promotion_boundary_receipt_ref
    ),
    format!("apply-artifact-ref:{}", source_apply_artifact_ref),
    format!("verify-diff-ref:{}", receipt.diff_ref),
    format!(
      "execution-result-ref:{}",
      receipt.execution_result.execution_result_ref
    ),
    format!(
      "execution-status:{}",
      receipt.execution_result.execution_status
    ),
    format!("verify-status:{}", receipt.status.result_status),
    format!("join-status:{}", join_status),
    "verify-receipt-is-not-promotion-owner".to_string(),
  ];
  if let Some(source_handoff_ref) = source_handoff_ref.as_deref() {
    proof_refs.push(format!("apply-handoff-proof-ref:{}", source_handoff_ref));
  }
  proof_refs.sort();
  proof_refs.dedup();

  Some(CodingAgentPromotionBoundaryJoinReceiptSurface {
    artifact_family: "coding.promotion-boundary-join-receipt",
    phase: "CAX.5p",
    join_ref,
    join_owner: "pnix-executor-graph::coding-agent::promotion-boundary-join-receipt",
    source_promotion_boundary_receipt_ref,
    source_apply_artifact_ref,
    source_handoff_ref,
    verify_diff_ref: receipt.diff_ref.clone(),
    verify_execution_result_ref: receipt.execution_result.execution_result_ref.clone(),
    verify_status: receipt.status.result_status,
    execution_status: receipt.execution_result.execution_status,
    join_status,
    target_paths: receipt.target_paths.clone(),
    target_commands: receipt.target_commands.clone(),
    required_next_artifacts: required_next_artifacts.into_iter().collect(),
    forbidden_effects: vec![
      "verify-receipt-auto-promotion".to_string(),
      "apply-result-auto-promotion".to_string(),
      "provider-output-promotion".to_string(),
      "human-judgement-bypass".to_string(),
    ],
    proof_refs,
    promotion_boundary: "join-receipt-only-not-judgement-owner",
  })
}

fn build_coding_agent_human_promotion_decision(
  args: &Args,
  request: CodingAgentRequestArtifact,
  request_artifact_ref: Option<String>,
) -> Result<CodingAgentHumanPromotionDecisionArtifact> {
  let source_promotion_boundary_join_ref = args
    .agent_promotion_boundary_join_ref
    .clone()
    .context("pnix coding-agent decide requires --promotion-boundary-join-ref")?;
  let human_decision = args
    .agent_promotion_decision
    .clone()
    .context("pnix coding-agent decide requires --promotion-decision")?;
  let (decision_status, promotion_status, progress_status, result_status, note) =
    match human_decision.as_str() {
      "accepted" => (
        "accepted-by-human-judgement",
        "human-accepted-awaiting-release-owner",
        "판단기록완료",
        "승인",
        "human judgement packet 을 남겼다. release/merge/promotion executor 는 별도 owner 다.",
      ),
      "rejected" => (
        "rejected-by-human-judgement",
        "human-rejected-repair-required",
        "판단기록완료",
        "거절",
        "human judgement rejection 을 남겼고 다음 단계는 repair patch proposal 이다.",
      ),
      "held" => (
        "held-for-human-judgement",
        "human-held-more-evidence-required",
        "판단보류기록완료",
        "보류",
        "human judgement hold 를 남겼고 추가 evidence/verify 없이는 승격하지 않는다.",
      ),
      _ => bail!("--promotion-decision must be one of accepted|rejected|held"),
    };
  let mut required_next_artifacts = BTreeSet::new();
  match human_decision.as_str() {
    "accepted" => {
      required_next_artifacts
        .insert("release-or-merge-owner-before-production-promotion".to_string());
      required_next_artifacts.insert("post-promotion-monitoring-receipt".to_string());
    }
    "rejected" => {
      required_next_artifacts.insert("repair-patch-proposal-before-promotion".to_string());
      required_next_artifacts.insert("decision-rationale-review".to_string());
    }
    "held" => {
      required_next_artifacts.insert("additional-human-evidence-before-promotion".to_string());
      required_next_artifacts.insert("repeat-verify-or-review-before-promotion".to_string());
    }
    _ => {}
  }
  let human_rationale = request
    .request
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(ToOwned::to_owned);
  let repo_snapshot_ref = make_repo_snapshot_ref(&request.workspace);
  let decision_ref = make_human_promotion_decision_ref(
    source_promotion_boundary_join_ref.as_str(),
    human_decision.as_str(),
    human_rationale.as_deref(),
    repo_snapshot_ref.as_str(),
  );
  let mut proof_refs = vec![
    format!("human-promotion-decision-ref:{}", decision_ref),
    format!(
      "promotion-boundary-join-receipt-ref:{}",
      source_promotion_boundary_join_ref
    ),
    format!("human-decision:{}", human_decision),
    format!("decision-status:{}", decision_status),
    format!("promotion-status:{}", promotion_status),
    format!("repo-snapshot-ref:{}", repo_snapshot_ref),
    format!("context-pack-ref:{}", request.context_pack.context_pack_ref),
    "decision-packet-is-not-mutation-owner".to_string(),
    "human-judgement-required-before-promotion".to_string(),
  ];
  if human_rationale.is_some() {
    proof_refs.push("human-rationale-present:true".to_string());
  } else {
    proof_refs.push("human-rationale-present:false".to_string());
  }
  if let Some(plan_ref) = request.workspace.current_plan_ref.as_deref() {
    proof_refs.push(format!("current-plan-ref:{}", plan_ref));
  }
  if let Some(last_ref) = request.workspace.last_verification_ref.as_deref() {
    proof_refs.push(format!("last-verification-ref:{}", last_ref));
  }
  for path in request.workspace.target_paths.iter().take(4) {
    proof_refs.push(format!("target-path:{}", path));
  }
  for command in request.workspace.approved_commands.iter().take(4) {
    proof_refs.push(format!("target-command:{}", command));
  }
  proof_refs.sort();
  proof_refs.dedup();

  Ok(CodingAgentHumanPromotionDecisionArtifact {
    artifact_family: "coding.human-promotion-decision",
    phase: "CAX.5q",
    surface: "pnix coding-agent",
    verb: "decide",
    decided_at_ms: current_time_ms(),
    request_artifact_ref,
    decision_ref,
    decision_owner: "human-via-pnix-executor-graph::coding-agent::decide",
    source_promotion_boundary_join_ref,
    human_decision,
    decision_status,
    promotion_status,
    human_rationale,
    target_paths: request.workspace.target_paths.clone(),
    target_commands: request.workspace.approved_commands.clone(),
    required_next_artifacts: required_next_artifacts.into_iter().collect(),
    forbidden_effects: vec![
      "human-decision-auto-merge".to_string(),
      "verify-green-auto-promotion".to_string(),
      "provider-output-promotion".to_string(),
      "decision-packet-file-write".to_string(),
    ],
    proof_refs,
    promotion_boundary: "human-decision-packet-not-mutation-owner",
    status: CodingAgentStatusSurface {
      progress_status,
      result_status,
      note: note.to_string(),
    },
    request,
  })
}

fn build_verify_execution_diagnostic_bridge(
  request: &CodingAgentRequestArtifact,
  execution_result: &CodingAgentExecutionResultSurface,
) -> (
  Vec<CodingAgentDiagnosticRecordSurface>,
  Vec<CodingAgentFailurePatternMatchSurface>,
  Vec<CodingAgentContextDemandSurface>,
) {
  let mut diagnostic_records = Vec::new();
  let mut failure_pattern_matches = Vec::new();
  let mut context_demands = Vec::new();

  for command in execution_result.command_results.iter() {
    if command.status == "passed" {
      continue;
    }

    let verify_target = find_verify_target_for_command(request, command);
    let language = verify_target
      .map(|target| target.language.as_str())
      .unwrap_or("unknown");
    let target_path = verify_target
      .map(|target| target.target_path.as_str())
      .or_else(|| request.workspace.target_paths.first().map(String::as_str))
      .unwrap_or("workspace");
    let diagnostic_family = verify_command_diagnostic_family(command.status);
    let pattern_key = verify_command_failure_pattern_key(command.status);
    let diagnostic_ref = make_verify_execution_bridge_ref(
      "diagnostic",
      &execution_result.execution_result_ref,
      &command.command_ref,
      target_path,
      command.status,
    );
    let context_demand_ref = make_verify_execution_bridge_ref(
      "context-demand",
      &execution_result.execution_result_ref,
      &command.command_ref,
      target_path,
      command.status,
    );
    let match_ref = make_verify_execution_bridge_ref(
      "failure-pattern-match",
      &execution_result.execution_result_ref,
      &command.command_ref,
      target_path,
      command.status,
    );

    diagnostic_records.push(CodingAgentDiagnosticRecordSurface {
      artifact_family: "pnix.diagnostic-record",
      diagnostic_ref: diagnostic_ref.clone(),
      language: language.to_string(),
      target_path: target_path.to_string(),
      diagnostic_family,
      severity: verify_command_diagnostic_severity(command.status),
      message: summarize_verify_command_diagnostic_message(command),
      provenance_refs: vec![
        format!(
          "execution-result-ref:{}",
          execution_result.execution_result_ref
        ),
        format!(
          "execution-command:{}:{}",
          command.command_ref, command.status
        ),
        "CAX.5e:verify-execution-diagnostic-bridge".to_string(),
      ],
      record_status: "candidate",
    });
    failure_pattern_matches.push(CodingAgentFailurePatternMatchSurface {
      artifact_family: "coding.failure-pattern-match",
      match_ref,
      diagnostic_ref,
      pattern_key,
      confidence: 1.0,
      context_demand_ref: context_demand_ref.clone(),
      promotion_boundary: "candidate-only-not-judgement",
    });
    context_demands.push(CodingAgentContextDemandSurface {
      artifact_family: "coding.context-demand",
      context_demand_ref,
      language: language.to_string(),
      target_path: target_path.to_string(),
      demand_family: "verify-failure-context-required",
      required_evidence: vec![
        "language-profile.verify-target".to_string(),
        "bounded-command-result".to_string(),
        "stderr-summary-not-raw-log-proof".to_string(),
        "targeted-repair-plan".to_string(),
      ],
      request_boundary: "request-more-context-before-next-patch-proposal",
    });
  }

  (diagnostic_records, failure_pattern_matches, context_demands)
}

fn find_verify_target_for_command<'a>(
  request: &'a CodingAgentRequestArtifact,
  command: &CodingAgentCommandExecutionRecord,
) -> Option<&'a CodingAgentVerifyTargetSurface> {
  request
    .language_profile
    .verify_targets
    .iter()
    .find(|target| {
      target
        .command_candidates
        .iter()
        .any(|candidate| candidate == &command.command_ref)
    })
    .or_else(|| request.language_profile.verify_targets.first())
}

fn verify_command_diagnostic_family(status: &str) -> &'static str {
  match status {
    "blocked" => "verify-command-policy-blocked",
    "timed-out" => "verify-command-timed-out",
    "spawn-error" | "wait-error" => "verify-command-runtime-error",
    _ => "verify-command-failed",
  }
}

fn verify_command_failure_pattern_key(status: &str) -> &'static str {
  match status {
    "blocked" => "verify-command-policy-blocked",
    "timed-out" => "verify-command-timed-out",
    "spawn-error" | "wait-error" => "verify-command-runtime-error",
    _ => "verify-command-failed",
  }
}

fn verify_command_diagnostic_severity(status: &str) -> &'static str {
  match status {
    "blocked" => "hold",
    _ => "fail",
  }
}

fn summarize_verify_command_diagnostic_message(
  command: &CodingAgentCommandExecutionRecord,
) -> String {
  let mut message = format!(
    "verify command `{}` ended with status `{}`",
    command.command_ref, command.status
  );
  if let Some(exit_code) = command.exit_code {
    message.push_str(format!(" and exit code {exit_code}").as_str());
  }
  if let Some(error) = command.error.as_deref().filter(|value| !value.is_empty()) {
    message.push_str(": ");
    message.push_str(&truncate_diagnostic_message(error));
  } else if let Some(stderr) = first_nonempty_line(command.stderr_preview.as_str()) {
    message.push_str(": ");
    message.push_str(&truncate_diagnostic_message(stderr));
  } else if let Some(stdout) = first_nonempty_line(command.stdout_preview.as_str()) {
    message.push_str(": ");
    message.push_str(&truncate_diagnostic_message(stdout));
  }
  message
}

fn first_nonempty_line(value: &str) -> Option<&str> {
  value.lines().find(|line| !line.trim().is_empty())
}

fn truncate_diagnostic_message(value: &str) -> String {
  const MAX_DIAGNOSTIC_MESSAGE_CHARS: usize = 240;
  let mut chars = value.chars();
  let truncated = chars
    .by_ref()
    .take(MAX_DIAGNOSTIC_MESSAGE_CHARS)
    .collect::<String>();
  if chars.next().is_some() {
    format!("{truncated}...")
  } else {
    truncated
  }
}

fn parse_coding_agent_approved_command(raw: &str) -> std::result::Result<Vec<String>, String> {
  let trimmed = raw.trim();
  if trimmed.is_empty() || trimmed == "manual-verification-contract-required" {
    return Err("approved command is missing".to_string());
  }
  if trimmed.chars().any(is_shell_control_char) {
    return Err(
      "approved command contains shell control syntax; use direct program arguments only"
        .to_string(),
    );
  }
  if trimmed.contains('"') || trimmed.contains('\'') || trimmed.contains('\\') {
    return Err(
      "approved command quoting/escaping is not supported in the bounded verifier".to_string(),
    );
  }

  let parts = trimmed
    .split_whitespace()
    .map(ToString::to_string)
    .collect::<Vec<_>>();
  if parts.is_empty() {
    return Err("approved command is missing".to_string());
  }
  if parts[0].contains('=') {
    return Err("environment assignment as program is not supported".to_string());
  }
  if is_shell_program(parts[0].as_str())
    && parts
      .iter()
      .skip(1)
      .any(|part| part == "-c" || part == "-lc")
  {
    return Err("shell -c execution is not supported by coding-agent verify".to_string());
  }
  Ok(parts)
}

fn is_shell_control_char(ch: char) -> bool {
  matches!(ch, '|' | '&' | ';' | '<' | '>' | '$' | '`' | '\n' | '\r')
}

fn is_shell_program(program: &str) -> bool {
  let name = Path::new(program)
    .file_name()
    .and_then(|name| name.to_str())
    .unwrap_or(program);
  matches!(name, "sh" | "bash" | "zsh" | "fish")
}

fn summarize_coding_agent_execution_status(
  command_results: &[CodingAgentCommandExecutionRecord],
) -> &'static str {
  if command_results.is_empty() {
    return "blocked:no-approved-command";
  }
  if command_results
    .iter()
    .any(|record| record.status == "blocked")
  {
    "blocked"
  } else if command_results
    .iter()
    .any(|record| record.status == "timed-out")
  {
    "timed-out"
  } else if command_results
    .iter()
    .any(|record| matches!(record.status, "spawn-error" | "wait-error"))
  {
    "error"
  } else if command_results
    .iter()
    .all(|record| record.status == "passed")
  {
    "passed"
  } else {
    "failed"
  }
}

fn summarize_coding_agent_exit_code(
  command_results: &[CodingAgentCommandExecutionRecord],
) -> Option<i32> {
  for record in command_results {
    if record.status != "passed" {
      return record.exit_code;
    }
  }
  if command_results.is_empty() {
    None
  } else {
    Some(0)
  }
}

fn status_for_coding_agent_execution_result(
  execution_result: &CodingAgentExecutionResultSurface,
) -> CodingAgentStatusSurface {
  let (progress_status, result_status, note) = match execution_result.execution_status {
    "passed" => (
      "검증실행완료",
      "통과",
      "approved command를 bounded verifier로 실행했고 execution-result를 verify-receipt에 연결했다.",
    ),
    "failed" => (
      "검증실패",
      "실패",
      "approved command 실행이 실패했으며 실패도 execution-result/verify-receipt로 보존했다.",
    ),
    "blocked" | "blocked:no-approved-command" => (
      "검증실행차단",
      "차단",
      "approved command가 없거나 bounded verifier policy를 위반해 실행하지 않았다.",
    ),
    "timed-out" => (
      "검증시간초과",
      "실패",
      "approved command가 bounded timeout을 초과해 종료되었다.",
    ),
    "error" => (
      "검증실행오류",
      "실패",
      "approved command 실행 중 spawn/wait 오류가 발생했다.",
    ),
    _ => (
      "검증영수증준비완료",
      "부분완료",
      "typed verify receipt artifact를 생성했고 actual command execution lane은 아직 열지 않았다.",
    ),
  };
  CodingAgentStatusSurface {
    progress_status,
    result_status,
    note: note.to_string(),
  }
}

fn build_execution_raw_result_refs(
  command_results: &[CodingAgentCommandExecutionRecord],
) -> Vec<String> {
  let mut refs = Vec::new();
  for result in command_results {
    refs.push(result.stdout_ref.clone());
    refs.push(result.stderr_ref.clone());
  }
  refs.sort();
  refs.dedup();
  refs
}

fn command_output_ref(kind: &str, command_ref: &str, bytes: &[u8]) -> String {
  let mut hasher = Sha256::new();
  hasher.update(kind.as_bytes());
  hasher.update(b"\n--command--\n");
  hasher.update(command_ref.as_bytes());
  hasher.update(b"\n--bytes--\n");
  hasher.update(bytes);
  format!("coding.command-output::{}::{:x}", kind, hasher.finalize())
}

fn command_output_preview(bytes: &[u8]) -> String {
  let take_len = bytes.len().min(CODING_AGENT_COMMAND_OUTPUT_PREVIEW_BYTES);
  let mut preview = String::from_utf8_lossy(&bytes[..take_len]).to_string();
  if bytes.len() > take_len {
    preview.push_str("\n[truncated]");
  }
  preview
}

fn elapsed_ms(started: Instant) -> u64 {
  let millis = started.elapsed().as_millis();
  millis.min(u128::from(u64::MAX)) as u64
}

fn build_coding_agent_learning_card(
  request: &CodingAgentRequestArtifact,
  repo_snapshot_ref: &str,
  diff_ref: &str,
  execution_result: &CodingAgentExecutionResultSurface,
  proof_refs: &[String],
) -> CodingAgentLearningCardSurface {
  let trigger = request
    .request
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .unwrap_or("coding-agent verify receipt")
    .to_string();
  let verify_pattern = if request.workspace.approved_commands.is_empty() {
    "declare-approved-command-before-verification".to_string()
  } else {
    request.workspace.approved_commands.join(" && ")
  };
  let repair_pattern =
    classify_patch_edit_family(request.request.as_deref(), &request.workspace.target_paths)
      .to_string();
  let mut card_proofs = proof_refs.iter().take(6).cloned().collect::<Vec<_>>();
  card_proofs.push(format!(
    "execution-result-ref:{}",
    execution_result.execution_result_ref
  ));
  card_proofs.sort();
  card_proofs.dedup();

  CodingAgentLearningCardSurface {
    artifact_family: "coding.learning-card",
    phase: "CAX.4b-partial",
    learning_card_ref: make_learning_card_ref(repo_snapshot_ref, diff_ref, &trigger),
    trigger,
    context_signature: request.context_pack.context_pack_ref.clone(),
    repair_pattern,
    verify_pattern,
    reuse_score: 0.0,
    promotion_status: "candidate-only-not-promoted",
    proof_refs: card_proofs,
  }
}

fn build_coding_agent_rollback_handle(
  args: &Args,
  request: CodingAgentRequestArtifact,
  request_artifact_ref: Option<String>,
) -> CodingAgentRollbackHandleArtifact {
  let effect_contracts = build_rollback_effect_contracts(
    &request.workspace.target_paths,
    &request.workspace.approved_commands,
  );
  let rollback_class = classify_rollback_class(&effect_contracts);
  let non_rollbackable_effects = effect_contracts
    .iter()
    .filter(|contract| contract.rollback_contract != "rollbackable")
    .map(|contract| contract.effect_class.clone())
    .collect::<Vec<_>>();
  let repo_snapshot_ref = make_repo_snapshot_ref(&request.workspace);
  let apply_artifact_ref = make_apply_artifact_ref(
    request.request.as_deref(),
    &request.workspace.target_paths,
    request.workspace.current_plan_ref.as_deref(),
  );
  let inverse_plan_ref = if rollback_class == "rollbackable" {
    Some(make_inverse_plan_ref(&apply_artifact_ref))
  } else {
    None
  };
  let handle_id = make_rollback_handle_id(&repo_snapshot_ref, &apply_artifact_ref, rollback_class);
  let expires_at_ms = if rollback_class == "rollbackable" {
    Some(current_time_ms() + 3_600_000)
  } else {
    None
  };
  let proof_refs = build_rollback_proof_refs(
    &request,
    &repo_snapshot_ref,
    &apply_artifact_ref,
    &handle_id,
    rollback_class,
    &effect_contracts,
  );
  let note = if rollback_class == "rollbackable" {
    if args.agent_rollback_out.is_some() {
      "typed rollback handle artifact를 생성했다. 실행은 rollback receipt + explicit inverse diff gate 에서만 열린다."
    } else {
      "stdout에 typed rollback handle surface만 노출했다. 실행은 rollback receipt + explicit inverse diff gate 에서만 열린다."
    }
  } else {
    "rollback handle artifact를 생성했지만 일부 effect는 acknowledge-risk 또는 forbidden contract로만 분류되어 explicit rollback 실행 대상이 아니다."
  };

  CodingAgentRollbackHandleArtifact {
    artifact_family: "coding.rollback-handle",
    phase: "CAX.3c-partial",
    surface: "pnix coding-agent",
    verb: "rollback",
    issued_at_ms: current_time_ms(),
    request_artifact_ref,
    handle_id,
    repo_snapshot_ref,
    apply_artifact_ref,
    inverse_plan_ref,
    rollback_class,
    effect_contracts,
    non_rollbackable_effects,
    expires_at_ms,
    status: CodingAgentStatusSurface {
      progress_status: if rollback_class == "forbidden" {
        "롤백계약차단"
      } else {
        "롤백핸들준비완료"
      },
      result_status: if rollback_class == "forbidden" {
        "차단"
      } else {
        "부분완료"
      },
      note: note.to_string(),
    },
    proof_refs,
    request,
  }
}

fn build_coding_agent_rollback_receipt(
  args: &Args,
  request: CodingAgentRequestArtifact,
  request_artifact_ref: Option<String>,
) -> CodingAgentRollbackReceiptArtifact {
  let effect_contracts = build_rollback_effect_contracts(
    &request.workspace.target_paths,
    &request.workspace.approved_commands,
  );
  let rollback_class = classify_rollback_class(&effect_contracts);
  let non_rollbackable_effects = effect_contracts
    .iter()
    .filter(|contract| contract.rollback_contract != "rollbackable")
    .map(|contract| contract.effect_class.clone())
    .collect::<Vec<_>>();
  let repo_snapshot_ref = make_repo_snapshot_ref(&request.workspace);
  let apply_artifact_ref = make_apply_artifact_ref(
    request.request.as_deref(),
    &request.workspace.target_paths,
    request.workspace.current_plan_ref.as_deref(),
  );
  let inverse_plan_ref = if rollback_class == "rollbackable" {
    Some(make_inverse_plan_ref(&apply_artifact_ref))
  } else {
    None
  };
  let handle_ref = args.agent_rollback_handle_ref.clone().unwrap_or_else(|| {
    make_rollback_handle_id(&repo_snapshot_ref, &apply_artifact_ref, rollback_class)
  });
  let explicit_rollback = build_coding_agent_explicit_rollback_result(
    args,
    &request,
    &handle_ref,
    &repo_snapshot_ref,
    rollback_class,
  );
  let rollback_status = explicit_rollback
    .as_ref()
    .map(|result| result.rollback_status)
    .unwrap_or("receipt-only-not-executed");
  let rollback_input_ref = explicit_rollback
    .as_ref()
    .and_then(|result| result.rollback_input_ref.clone());
  let dry_run = explicit_rollback
    .as_ref()
    .map(|result| result.dry_run)
    .unwrap_or(false);
  let restored_paths = explicit_rollback
    .as_ref()
    .map(|result| result.restored_paths.clone())
    .unwrap_or_default();
  let rejected_paths = explicit_rollback
    .as_ref()
    .map(|result| result.rejected_paths.clone())
    .unwrap_or_default();
  let file_results = explicit_rollback
    .as_ref()
    .map(|result| result.file_results.clone())
    .unwrap_or_default();
  let restored_snapshot_ref = explicit_rollback
    .as_ref()
    .and_then(|result| result.restored_snapshot_ref.clone());
  let rollback_error = explicit_rollback
    .as_ref()
    .and_then(|result| result.error.clone());
  let followup_verify_ref = Some(make_followup_verify_ref(
    restored_snapshot_ref
      .as_deref()
      .unwrap_or(repo_snapshot_ref.as_str()),
    &handle_ref,
  ));
  let proof_refs = build_rollback_receipt_proof_refs(
    &request,
    &handle_ref,
    &repo_snapshot_ref,
    &apply_artifact_ref,
    rollback_class,
    restored_snapshot_ref.as_deref(),
    followup_verify_ref.as_deref(),
    &effect_contracts,
    rollback_status,
    rollback_input_ref.as_deref(),
    &file_results,
  );
  let note = if rollback_status == "rolled-back" {
    "explicit inverse diff 를 declared target 안에서 적용했고 post-rollback verify 는 followup receipt 로 남겼다."
  } else if rollback_status == "validated-not-rolled-back" {
    "explicit inverse diff 를 declared target 안에서 검증했고 dry-run 이라 실제 write 는 수행하지 않았다."
  } else if matches!(rollback_status, "blocked" | "failed") {
    "rollback receipt artifact를 생성했지만 explicit rollback 실행은 blocked/failed 로 닫았고 silent rollback completion 으로 간주하지 않았다."
  } else if rollback_class == "rollbackable" {
    "typed rollback receipt artifact를 생성했고 explicit inverse diff 가 없어서 actual rollback execution 은 수행하지 않았다."
  } else {
    "rollback receipt artifact를 생성했지만 effect contract 상 acknowledge-risk 또는 forbidden family가 남아 있어 silent rollback completion 으로 간주하지 않았다."
  };
  let progress_status = match rollback_status {
    "rolled-back" => "롤백실행완료",
    "validated-not-rolled-back" => "롤백실행검증완료",
    "blocked" | "failed" => "롤백영수증차단",
    _ if rollback_class == "forbidden" => "롤백영수증차단",
    _ => "롤백영수증준비완료",
  };
  let result_status =
    if matches!(rollback_status, "blocked" | "failed") || rollback_class == "forbidden" {
      "차단"
    } else {
      "부분완료"
    };

  CodingAgentRollbackReceiptArtifact {
    artifact_family: "coding.rollback-receipt",
    phase: "CAX.3c-partial",
    surface: "pnix coding-agent",
    verb: "rollback",
    rolled_back_at_ms: current_time_ms(),
    request_artifact_ref,
    handle_ref,
    repo_snapshot_ref,
    apply_artifact_ref,
    inverse_plan_ref,
    restored_snapshot_ref,
    followup_verify_ref,
    rollback_status,
    rollback_input_ref,
    dry_run,
    restored_paths,
    rejected_paths,
    file_results,
    error: rollback_error,
    rollback_class,
    effect_contracts,
    non_rollbackable_effects,
    status: CodingAgentStatusSurface {
      progress_status,
      result_status,
      note: note.to_string(),
    },
    proof_refs,
    request,
  }
}

fn build_coding_agent_explicit_rollback_result(
  args: &Args,
  request: &CodingAgentRequestArtifact,
  handle_ref: &str,
  repo_snapshot_ref: &str,
  rollback_class: &str,
) -> Option<CodingAgentExplicitRollbackResult> {
  let patch_path = args.patch.as_ref()?;
  let rollback_input_ref = Some(format!(
    "rollback-patch-input:{}",
    path_to_slash(patch_path)
  ));

  if rollback_class != "rollbackable" {
    return Some(blocked_coding_agent_rollback_result(
      rollback_input_ref,
      args.dry_run,
      request.workspace.target_paths.clone(),
      format!(
        "rollback effect contract is {}; explicit inverse diff cannot silently cover non-file effects",
        rollback_class
      ),
    ));
  }

  let patch_text = match fs::read_to_string(patch_path) {
    Ok(text) => text,
    Err(err) => {
      return Some(blocked_coding_agent_rollback_result(
        rollback_input_ref,
        args.dry_run,
        request.workspace.target_paths.clone(),
        format!("read rollback patch {}: {}", patch_path.display(), err),
      ));
    }
  };
  let parsed = match parse_coding_agent_unified_patch(&patch_text) {
    Ok(parsed) => parsed,
    Err(err) => {
      return Some(blocked_coding_agent_rollback_result(
        rollback_input_ref,
        args.dry_run,
        request.workspace.target_paths.clone(),
        err,
      ));
    }
  };
  let prepared = match prepare_coding_agent_patch_application(request, parsed.as_slice()) {
    Ok(prepared) => prepared,
    Err(err) => {
      return Some(blocked_coding_agent_rollback_result(
        rollback_input_ref,
        args.dry_run,
        request.workspace.target_paths.clone(),
        err,
      ));
    }
  };

  let mut file_results = prepared
    .iter()
    .map(|prepared| CodingAgentPatchFileApplyRecord {
      path: prepared.path.clone(),
      status: if args.dry_run {
        "validated"
      } else {
        "pending-write"
      },
      before_snapshot_ref: prepared.before_snapshot_ref.clone(),
      after_snapshot_ref: Some(prepared.after_snapshot_ref.clone()),
      byte_delta: prepared.byte_delta,
      error: None,
    })
    .collect::<Vec<_>>();

  let mut rollback_status = if args.dry_run {
    "validated-not-rolled-back"
  } else {
    "rolled-back"
  };
  let mut error = None;
  if !args.dry_run {
    for (index, prepared) in prepared.iter().enumerate() {
      if let Some(parent) = prepared.absolute_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
          rollback_status = "failed";
          error = Some(format!("create parent {}: {}", parent.display(), err));
          file_results[index].status = "failed";
          file_results[index].error = error.clone();
          break;
        }
      }
      if let Err(err) = fs::write(&prepared.absolute_path, &prepared.after_content) {
        rollback_status = "failed";
        error = Some(format!("write rollback target {}: {}", prepared.path, err));
        file_results[index].status = "failed";
        file_results[index].error = error.clone();
        break;
      }
      file_results[index].status = "restored";
    }
  }

  let rollback_succeeded = matches!(rollback_status, "rolled-back" | "validated-not-rolled-back");
  let restored_paths = if rollback_succeeded {
    prepared
      .iter()
      .map(|prepared| prepared.path.clone())
      .collect::<Vec<_>>()
  } else {
    Vec::new()
  };
  let rejected_paths = if rollback_succeeded {
    Vec::new()
  } else {
    prepared
      .iter()
      .map(|prepared| prepared.path.clone())
      .collect::<Vec<_>>()
  };
  let restored_snapshot_ref = if rollback_succeeded {
    Some(make_restored_snapshot_ref_from_files(
      repo_snapshot_ref,
      handle_ref,
      &file_results,
    ))
  } else {
    None
  };

  Some(CodingAgentExplicitRollbackResult {
    rollback_status,
    rollback_input_ref,
    dry_run: args.dry_run,
    restored_paths,
    rejected_paths,
    file_results,
    restored_snapshot_ref,
    error,
  })
}

fn blocked_coding_agent_rollback_result(
  rollback_input_ref: Option<String>,
  dry_run: bool,
  rejected_paths: Vec<String>,
  error: String,
) -> CodingAgentExplicitRollbackResult {
  CodingAgentExplicitRollbackResult {
    rollback_status: "blocked",
    rollback_input_ref,
    dry_run,
    restored_paths: Vec::new(),
    rejected_paths,
    file_results: Vec::new(),
    restored_snapshot_ref: None,
    error: Some(error),
  }
}

fn classify_coding_agent_interpretation(request: Option<&str>, target_paths: &[String]) -> String {
  let scope = match target_paths {
    [] => "repo-local".to_string(),
    [single] => format!("{} scope", single),
    many => format!("{} target paths", many.len()),
  };

  let Some(request) = request.map(str::trim).filter(|value| !value.is_empty()) else {
    return format!("{} bounded planning", scope);
  };

  let lower = request.to_ascii_lowercase();
  let family = if contains_any(&lower, &["fix", "repair", "patch", "수정", "고쳐", "버그"]) {
    "bounded repair planning"
  } else if contains_any(&lower, &["test", "verify", "검증", "실패", "failure"]) {
    "targeted verification planning"
  } else if contains_any(&lower, &["review", "리뷰", "risk", "위험"]) {
    "risk review planning"
  } else if contains_any(&lower, &["explain", "설명", "원인", "analy"]) {
    "repo inspection and explanation planning"
  } else {
    "repo-local bounded planning"
  };

  format!("{} {}", scope, family)
}

fn classify_patch_edit_family(request: Option<&str>, target_paths: &[String]) -> &'static str {
  let lower = request.unwrap_or_default().to_ascii_lowercase();
  if contains_any(&lower, &["refactor", "리팩터", "정리"]) {
    "refactor"
  } else if target_paths.iter().any(|path| is_test_path(path))
    || contains_any(&lower, &["test", "테스트"])
  {
    "test-fix"
  } else if target_paths.iter().any(|path| is_docs_path(path))
    || contains_any(&lower, &["doc", "readme", "문서"])
  {
    "docs-update"
  } else if contains_any(&lower, &["fix", "bug", "repair", "수정", "고쳐", "버그"]) {
    "bugfix"
  } else {
    "bounded-edit"
  }
}

fn classify_patch_risk_class(
  request: Option<&str>,
  target_paths: &[String],
  approved_commands: &[String],
) -> &'static str {
  if approved_commands.is_empty() {
    return "high-unverified";
  }
  if target_paths.is_empty() || target_paths.len() > 1 {
    return "high-broad-scope";
  }

  let lower = request.unwrap_or_default().to_ascii_lowercase();
  if target_paths
    .iter()
    .all(|path| is_docs_path(path) || is_test_path(path))
    || contains_any(&lower, &["readme", "doc", "문서", "test", "테스트"])
  {
    "low"
  } else {
    "medium"
  }
}

fn build_patch_effect_classes(
  target_paths: &[String],
  approved_commands: &[String],
) -> Vec<String> {
  let mut effect_classes = BTreeSet::new();
  effect_classes.insert("workspace-file-write:intent-only".to_string());
  for path in target_paths {
    if is_test_path(path) {
      effect_classes.insert("test-file-write:intent-only".to_string());
    }
    if is_docs_path(path) {
      effect_classes.insert("docs-file-write:intent-only".to_string());
    }
  }
  if !approved_commands.is_empty() {
    effect_classes.insert("verification-command:intent-only".to_string());
  }
  effect_classes.into_iter().collect()
}

fn build_coding_agent_context_evidence_refs(request: &CodingAgentRequestArtifact) -> Vec<String> {
  let mut evidence_refs = vec![
    format!("context-pack-ref:{}", request.context_pack.context_pack_ref),
    format!(
      "repo-graph-status:{}",
      request.repo_graph_seed.project_graph_status
    ),
  ];
  for path in request.workspace.target_paths.iter().take(4) {
    evidence_refs.push(format!("target-path:{}", path));
  }
  if let Some(receipt) = request.manual_evidence_seed.uncertainty_receipts.first() {
    evidence_refs.push(format!("manual-evidence-receipt:{}", receipt));
  } else if !request.manual_evidence_seed.hits.is_empty() {
    evidence_refs.push(format!(
      "manual-evidence-hit-count:{}",
      request.manual_evidence_seed.hits.len()
    ));
  }
  evidence_refs.sort();
  evidence_refs.dedup();
  evidence_refs
}

fn make_context_pack_ref(
  workspace: &CodingAgentWorkspaceSnapshot,
  grounding_seed: &CodingAgentGroundingSeed,
  repo_graph_seed: &CodingAgentRepoGraphSeed,
  manual_evidence_seed: &CodingAgentManualEvidenceSeed,
  attached_pack_seed: &CodingAgentAttachedPackSeed,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(make_repo_snapshot_ref(workspace).as_bytes());
  hasher.update(b"\n--grounding--\n");
  hasher.update(grounding_seed.scan_root.as_bytes());
  hasher.update(b"\n");
  for entry in &grounding_seed.entries {
    hasher.update(entry.path.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--repo-graph--\n");
  hasher.update(repo_graph_seed.project_graph_status.as_bytes());
  hasher.update(b"\n");
  hasher.update(repo_graph_seed.files.len().to_string().as_bytes());
  hasher.update(b"\n--manual--\n");
  hasher.update(manual_evidence_seed.hits.len().to_string().as_bytes());
  hasher.update(b"\n");
  hasher.update(
    manual_evidence_seed
      .uncertainty_receipts
      .len()
      .to_string()
      .as_bytes(),
  );
  hasher.update(b"\n--attached--\n");
  hasher.update(attached_pack_seed.total_entry_count.to_string().as_bytes());
  format!("coding.context-pack::{:x}", hasher.finalize())
}

fn make_language_profile_record_ref(kind: &str, language: &str, target_path: &str) -> String {
  let mut hasher = Sha256::new();
  hasher.update(kind.as_bytes());
  hasher.update(b"\n--language--\n");
  hasher.update(language.as_bytes());
  hasher.update(b"\n--target--\n");
  hasher.update(target_path.as_bytes());
  format!("pnix.{kind}-record::{:x}", hasher.finalize())
}

fn make_repo_snapshot_ref(workspace: &CodingAgentWorkspaceSnapshot) -> String {
  let mut hasher = Sha256::new();
  hasher.update(workspace.cwd.as_bytes());
  hasher.update(b"\n--repo-root--\n");
  hasher.update(
    workspace
      .repo_root
      .as_deref()
      .unwrap_or_default()
      .as_bytes(),
  );
  hasher.update(b"\n--git-branch--\n");
  hasher.update(
    workspace
      .git_branch
      .as_deref()
      .unwrap_or_default()
      .as_bytes(),
  );
  hasher.update(b"\n--git-head--\n");
  hasher.update(
    workspace
      .git_head_commit
      .as_deref()
      .unwrap_or_default()
      .as_bytes(),
  );
  hasher.update(b"\n--git-dirty--\n");
  hasher.update(if workspace.git_dirty { b"1" } else { b"0" });
  hasher.update(b"\n--targets--\n");
  for path in &workspace.target_paths {
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
  }
  format!("coding.repo-snapshot::{:x}", hasher.finalize())
}

fn make_patch_diff_ref(
  request: Option<&str>,
  target_paths: &[String],
  current_plan_ref: Option<&str>,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(request.unwrap_or_default().as_bytes());
  hasher.update(b"\n--targets--\n");
  for path in target_paths {
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--plan--\n");
  hasher.update(current_plan_ref.unwrap_or_default().as_bytes());
  let digest = hasher.finalize();
  format!("coding.diff::proposal::{:x}", digest)
}

fn make_generated_patch_input_ref(
  source_path: &Path,
  patch_text: Option<&str>,
  read_error: Option<&str>,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(path_to_slash(source_path).as_bytes());
  hasher.update(b"\n--patch-text--\n");
  hasher.update(patch_text.unwrap_or_default().as_bytes());
  hasher.update(b"\n--read-error--\n");
  hasher.update(read_error.unwrap_or_default().as_bytes());
  format!("coding.generated-patch-input::{:x}", hasher.finalize())
}

fn make_generated_patch_candidate_ref(
  request: &CodingAgentRequestArtifact,
  source_path: &str,
  patch_input_ref: &str,
  parsed_target_paths: &[String],
  quarantine_status: &str,
  source_provider_feedback_request_ref: Option<&str>,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(make_repo_snapshot_ref(&request.workspace).as_bytes());
  hasher.update(b"\n--context-pack--\n");
  hasher.update(request.context_pack.context_pack_ref.as_bytes());
  hasher.update(b"\n--source-path--\n");
  hasher.update(source_path.as_bytes());
  hasher.update(b"\n--patch-input--\n");
  hasher.update(patch_input_ref.as_bytes());
  hasher.update(b"\n--parsed-targets--\n");
  for target_path in parsed_target_paths {
    hasher.update(target_path.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--quarantine-status--\n");
  hasher.update(quarantine_status.as_bytes());
  hasher.update(b"\n--provider-feedback-request--\n");
  hasher.update(
    source_provider_feedback_request_ref
      .unwrap_or_default()
      .as_bytes(),
  );
  format!("coding.generated-patch-candidate::{:x}", hasher.finalize())
}

fn make_generated_patch_review_ref(
  kind: &str,
  candidate_ref: &str,
  patch_input_ref: &str,
  review_status: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(kind.as_bytes());
  hasher.update(b"\n--candidate--\n");
  hasher.update(candidate_ref.as_bytes());
  hasher.update(b"\n--patch-input--\n");
  hasher.update(patch_input_ref.as_bytes());
  hasher.update(b"\n--review-status--\n");
  hasher.update(review_status.as_bytes());
  format!(
    "coding.generated-patch-review-{kind}::{:x}",
    hasher.finalize()
  )
}

fn make_generated_patch_review_bridge_ref(
  kind: &str,
  candidate_ref: &str,
  target_path: &str,
  issue_key: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(kind.as_bytes());
  hasher.update(b"\n--candidate--\n");
  hasher.update(candidate_ref.as_bytes());
  hasher.update(b"\n--target--\n");
  hasher.update(target_path.as_bytes());
  hasher.update(b"\n--issue--\n");
  hasher.update(issue_key.as_bytes());
  format!(
    "coding.generated-patch-review-{kind}::{:x}",
    hasher.finalize()
  )
}

fn make_provider_feedback_packet_ref(
  review_ref: &str,
  context_demand_ref: &str,
  target_path: &str,
  demand_family: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(review_ref.as_bytes());
  hasher.update(b"\n--context-demand--\n");
  hasher.update(context_demand_ref.as_bytes());
  hasher.update(b"\n--target--\n");
  hasher.update(target_path.as_bytes());
  hasher.update(b"\n--demand-family--\n");
  hasher.update(demand_family.as_bytes());
  format!("coding.provider-feedback-packet::{:x}", hasher.finalize())
}

fn make_provider_feedback_request_ref(
  review_ref: &str,
  candidate_ref: &str,
  context_demand_refs: &[String],
  packet_refs: &[String],
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(review_ref.as_bytes());
  hasher.update(b"\n--candidate--\n");
  hasher.update(candidate_ref.as_bytes());
  hasher.update(b"\n--context-demands--\n");
  for context_demand_ref in context_demand_refs {
    hasher.update(context_demand_ref.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--packets--\n");
  for packet_ref in packet_refs {
    hasher.update(packet_ref.as_bytes());
    hasher.update(b"\n");
  }
  format!("coding.provider-feedback-request::{:x}", hasher.finalize())
}

fn make_feedback_retry_guard_ref(
  provider_feedback_request_ref: &str,
  candidate_ref: &str,
  review_ref: &str,
  context_demand_refs: &[String],
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(provider_feedback_request_ref.as_bytes());
  hasher.update(b"\n--candidate--\n");
  hasher.update(candidate_ref.as_bytes());
  hasher.update(b"\n--review--\n");
  hasher.update(review_ref.as_bytes());
  hasher.update(b"\n--context-demands--\n");
  for context_demand_ref in context_demand_refs {
    hasher.update(context_demand_ref.as_bytes());
    hasher.update(b"\n");
  }
  format!("coding.feedback-retry-guard::{:x}", hasher.finalize())
}

fn make_apply_handoff_ref(
  candidate_ref: &str,
  review_ref: &str,
  candidate_patch_input_ref: &str,
  apply_patch_input_ref: &str,
  handoff_status: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(candidate_ref.as_bytes());
  hasher.update(b"\n--review--\n");
  hasher.update(review_ref.as_bytes());
  hasher.update(b"\n--candidate-patch-input--\n");
  hasher.update(candidate_patch_input_ref.as_bytes());
  hasher.update(b"\n--apply-patch-input--\n");
  hasher.update(apply_patch_input_ref.as_bytes());
  hasher.update(b"\n--handoff-status--\n");
  hasher.update(handoff_status.as_bytes());
  format!("coding.apply-handoff-proof::{:x}", hasher.finalize())
}

fn make_promotion_boundary_receipt_ref(
  apply_artifact_ref: &str,
  handoff_ref: Option<&str>,
  apply_status: &str,
  promotion_status: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(apply_artifact_ref.as_bytes());
  hasher.update(b"\n--handoff--\n");
  hasher.update(handoff_ref.unwrap_or_default().as_bytes());
  hasher.update(b"\n--apply-status--\n");
  hasher.update(apply_status.as_bytes());
  hasher.update(b"\n--promotion-status--\n");
  hasher.update(promotion_status.as_bytes());
  format!("coding.promotion-boundary-receipt::{:x}", hasher.finalize())
}

fn make_promotion_boundary_join_receipt_ref(
  promotion_boundary_receipt_ref: &str,
  source_apply_artifact_ref: &str,
  source_handoff_ref: Option<&str>,
  verify_diff_ref: &str,
  verify_execution_result_ref: &str,
  join_status: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(promotion_boundary_receipt_ref.as_bytes());
  hasher.update(b"\n--apply--\n");
  hasher.update(source_apply_artifact_ref.as_bytes());
  hasher.update(b"\n--handoff--\n");
  hasher.update(source_handoff_ref.unwrap_or_default().as_bytes());
  hasher.update(b"\n--verify-diff--\n");
  hasher.update(verify_diff_ref.as_bytes());
  hasher.update(b"\n--execution-result--\n");
  hasher.update(verify_execution_result_ref.as_bytes());
  hasher.update(b"\n--join-status--\n");
  hasher.update(join_status.as_bytes());
  format!(
    "coding.promotion-boundary-join-receipt::{:x}",
    hasher.finalize()
  )
}

fn make_human_promotion_decision_ref(
  promotion_boundary_join_ref: &str,
  human_decision: &str,
  human_rationale: Option<&str>,
  repo_snapshot_ref: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(promotion_boundary_join_ref.as_bytes());
  hasher.update(b"\n--decision--\n");
  hasher.update(human_decision.as_bytes());
  hasher.update(b"\n--rationale--\n");
  hasher.update(human_rationale.unwrap_or_default().as_bytes());
  hasher.update(b"\n--repo-snapshot--\n");
  hasher.update(repo_snapshot_ref.as_bytes());
  format!("coding.human-promotion-decision::{:x}", hasher.finalize())
}

fn make_execution_plan_ref(
  request_artifact_ref: Option<&str>,
  context_pack_ref: &str,
  target_paths: &[String],
  expected_verification: &[String],
  candidate_verify_target_refs: &[String],
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(request_artifact_ref.unwrap_or_default().as_bytes());
  hasher.update(b"\n--context-pack--\n");
  hasher.update(context_pack_ref.as_bytes());
  hasher.update(b"\n--targets--\n");
  for path in target_paths {
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--expected-verify--\n");
  for command in expected_verification {
    hasher.update(command.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--candidate-verify-targets--\n");
  for target_ref in candidate_verify_target_refs {
    hasher.update(target_ref.as_bytes());
    hasher.update(b"\n");
  }
  format!("coding.execution-plan::{:x}", hasher.finalize())
}

fn make_execution_request_ref(
  execution_plan_ref: &str,
  expected_verification: &[String],
  candidate_verify_target_refs: &[String],
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(execution_plan_ref.as_bytes());
  hasher.update(b"\n--execution-request--\n");
  for command in expected_verification {
    hasher.update(command.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--candidate-verify-targets--\n");
  for target_ref in candidate_verify_target_refs {
    hasher.update(target_ref.as_bytes());
    hasher.update(b"\n");
  }
  format!("coding.execution-request::{:x}", hasher.finalize())
}

fn make_verify_diff_ref(
  repo_snapshot_ref: &str,
  target_paths: &[String],
  target_commands: &[String],
  current_plan_ref: Option<&str>,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(repo_snapshot_ref.as_bytes());
  hasher.update(b"\n--verify-targets--\n");
  for path in target_paths {
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--verify-commands--\n");
  for command in target_commands {
    hasher.update(command.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--plan-ref--\n");
  hasher.update(current_plan_ref.unwrap_or_default().as_bytes());
  format!("coding.diff::verify::{:x}", hasher.finalize())
}

fn make_execution_result_ref(
  repo_snapshot_ref: &str,
  diff_ref: &str,
  target_commands: &[String],
  observed_at_ms: u64,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(repo_snapshot_ref.as_bytes());
  hasher.update(b"\n--diff--\n");
  hasher.update(diff_ref.as_bytes());
  hasher.update(b"\n--commands--\n");
  for command in target_commands {
    hasher.update(command.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--observed-at-ms--\n");
  hasher.update(observed_at_ms.to_string().as_bytes());
  format!("coding.execution-result::{:x}", hasher.finalize())
}

fn make_verify_execution_bridge_ref(
  kind: &str,
  execution_result_ref: &str,
  command_ref: &str,
  target_path: &str,
  status: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(kind.as_bytes());
  hasher.update(b"\n--execution-result--\n");
  hasher.update(execution_result_ref.as_bytes());
  hasher.update(b"\n--command--\n");
  hasher.update(command_ref.as_bytes());
  hasher.update(b"\n--target--\n");
  hasher.update(target_path.as_bytes());
  hasher.update(b"\n--status--\n");
  hasher.update(status.as_bytes());
  format!("coding.{kind}::{:x}", hasher.finalize())
}

fn make_semantic_patch_review_ref(
  kind: &str,
  request: &CodingAgentRequestArtifact,
  diff_ref: &str,
  apply_status: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(kind.as_bytes());
  hasher.update(b"\n--repo-snapshot--\n");
  hasher.update(make_repo_snapshot_ref(&request.workspace).as_bytes());
  hasher.update(b"\n--context-pack--\n");
  hasher.update(request.context_pack.context_pack_ref.as_bytes());
  hasher.update(b"\n--diff--\n");
  hasher.update(diff_ref.as_bytes());
  hasher.update(b"\n--targets--\n");
  for target_path in &request.workspace.target_paths {
    hasher.update(target_path.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--apply-status--\n");
  hasher.update(apply_status.as_bytes());
  format!("coding.{kind}::{:x}", hasher.finalize())
}

fn make_context_demand_replay_ref(
  request: &CodingAgentRequestArtifact,
  source_refs: &[String],
  source_artifact_refs: &[String],
  replayed_context_demands: &[CodingAgentReplayedContextDemandSurface],
  semantic_review_refs: &[String],
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(make_repo_snapshot_ref(&request.workspace).as_bytes());
  hasher.update(b"\n--context-pack--\n");
  hasher.update(request.context_pack.context_pack_ref.as_bytes());
  hasher.update(b"\n--source-refs--\n");
  for source_ref in source_refs {
    hasher.update(source_ref.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--source-artifacts--\n");
  for source_ref in source_artifact_refs {
    hasher.update(source_ref.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--context-demands--\n");
  for demand in replayed_context_demands {
    hasher.update(demand.replay_item_ref.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--semantic-review-refs--\n");
  for review_ref in semantic_review_refs {
    hasher.update(review_ref.as_bytes());
    hasher.update(b"\n");
  }
  format!("coding.context-demand-replay::{:x}", hasher.finalize())
}

fn make_replayed_context_demand_ref(
  source_ref: &str,
  original_ref: &str,
  target_path: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(source_ref.as_bytes());
  hasher.update(b"\n--original-context-demand--\n");
  hasher.update(original_ref.as_bytes());
  hasher.update(b"\n--target--\n");
  hasher.update(target_path.as_bytes());
  format!("coding.context-demand::replayed::{:x}", hasher.finalize())
}

fn make_repair_recipe_replay_ref(
  request: &CodingAgentRequestArtifact,
  source_refs: &[String],
  source_artifact_refs: &[String],
  learning_card_refs: &[String],
  repair_candidates: &[CodingAgentRepairRecipeCandidateSurface],
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(make_repo_snapshot_ref(&request.workspace).as_bytes());
  hasher.update(b"\n--context-pack--\n");
  hasher.update(request.context_pack.context_pack_ref.as_bytes());
  hasher.update(b"\n--source-refs--\n");
  for source_ref in source_refs {
    hasher.update(source_ref.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--source-artifacts--\n");
  for source_ref in source_artifact_refs {
    hasher.update(source_ref.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--learning-cards--\n");
  for card_ref in learning_card_refs {
    hasher.update(card_ref.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--repair-candidates--\n");
  for candidate in repair_candidates {
    hasher.update(candidate.candidate_ref.as_bytes());
    hasher.update(b"\n");
  }
  format!("coding.repair-recipe-replay::{:x}", hasher.finalize())
}

#[cfg(feature = "doghouse")]
fn make_repair_recipe_candidate_ref(
  source_ref: &str,
  learning_card_ref: &str,
  repair_pattern: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(source_ref.as_bytes());
  hasher.update(b"\n--learning-card--\n");
  hasher.update(learning_card_ref.as_bytes());
  hasher.update(b"\n--repair-pattern--\n");
  hasher.update(repair_pattern.as_bytes());
  format!("coding.repair-recipe-candidate::{:x}", hasher.finalize())
}

fn make_learning_card_ref(repo_snapshot_ref: &str, diff_ref: &str, trigger: &str) -> String {
  let mut hasher = Sha256::new();
  hasher.update(repo_snapshot_ref.as_bytes());
  hasher.update(b"\n--diff--\n");
  hasher.update(diff_ref.as_bytes());
  hasher.update(b"\n--trigger--\n");
  hasher.update(trigger.as_bytes());
  format!("coding.learning-card::{:x}", hasher.finalize())
}

fn make_after_verify_artifact_ref(diff_ref: &str) -> String {
  let suffix = diff_ref
    .rsplit_once("::")
    .map(|(_, suffix)| suffix)
    .unwrap_or(diff_ref);
  format!("coding.verify-snapshot::after::{}", suffix)
}

fn build_verify_proof_refs(
  request: &CodingAgentRequestArtifact,
  repo_snapshot_ref: &str,
  before_artifact_ref: &str,
  after_artifact_ref: &str,
  diff_ref: &str,
  target_commands: &[String],
) -> Vec<String> {
  let mut proof_refs = Vec::new();
  proof_refs.push(format!("repo-snapshot-ref:{}", repo_snapshot_ref));
  proof_refs.push(format!("before-artifact-ref:{}", before_artifact_ref));
  proof_refs.push(format!("after-artifact-ref:{}", after_artifact_ref));
  proof_refs.push(format!("diff-ref:{}", diff_ref));
  for path in request.workspace.target_paths.iter().take(4) {
    proof_refs.push(format!("target-path:{}", path));
  }
  for command in target_commands.iter().take(4) {
    proof_refs.push(format!("target-command:{}", command));
  }
  if let Some(plan_ref) = request.workspace.current_plan_ref.as_deref() {
    proof_refs.push(format!("current-plan-ref:{}", plan_ref));
  }
  if let Some(last_ref) = request.workspace.last_verification_ref.as_deref() {
    proof_refs.push(format!("last-verification-ref:{}", last_ref));
  }
  if let Some(rollback_ref) = request.workspace.rollback_handle_ref.as_deref() {
    proof_refs.push(format!("rollback-handle-ref:{}", rollback_ref));
  }
  if let Some(promotion_ref) = request.workspace.promotion_boundary_ref.as_deref() {
    proof_refs.push(format!("promotion-boundary-receipt-ref:{}", promotion_ref));
  }
  if let Some(apply_ref) = request.workspace.source_apply_artifact_ref.as_deref() {
    proof_refs.push(format!("apply-artifact-ref:{}", apply_ref));
  }
  if let Some(handoff_ref) = request.workspace.source_handoff_ref.as_deref() {
    proof_refs.push(format!("apply-handoff-proof-ref:{}", handoff_ref));
  }
  proof_refs.push(format!(
    "repo-graph-status:{}",
    request.repo_graph_seed.project_graph_status
  ));
  if let Some(receipt) = request.manual_evidence_seed.uncertainty_receipts.first() {
    proof_refs.push(format!("manual-evidence-receipt:{}", receipt));
  } else if !request.manual_evidence_seed.hits.is_empty() {
    proof_refs.push(format!(
      "manual-evidence-hit-count:{}",
      request.manual_evidence_seed.hits.len()
    ));
  }
  proof_refs
}

fn build_rollback_effect_contracts(
  target_paths: &[String],
  approved_commands: &[String],
) -> Vec<CodingAgentRollbackEffectContract> {
  let mut contracts = Vec::new();
  let effect_classes = build_patch_effect_classes(target_paths, approved_commands);

  if target_paths.is_empty() {
    contracts.push(CodingAgentRollbackEffectContract {
      effect_class: "workspace-scope-unbounded".to_string(),
      rollback_contract: "forbidden",
      rationale: "target path 없이 rollback origin을 열면 workspace-wide destructive lane으로 과장되므로 차단한다.",
    });
  }

  for effect_class in effect_classes {
    let (rollback_contract, rationale) = match effect_class.as_str() {
      "workspace-file-write:intent-only" => (
        "rollbackable",
        "workspace file write는 inverse plan/checkpoint restore ref로 되감을 수 있는 1차 family다.",
      ),
      "docs-file-write:intent-only" => (
        "rollbackable",
        "docs file write는 repo snapshot diff 기준 inverse plan 생성이 가능한 low-risk family다.",
      ),
      "test-file-write:intent-only" => (
        "rollbackable",
        "test file write는 repo-local diff 기준 inverse plan 생성이 가능한 bounded family다.",
      ),
      "verification-command:intent-only" => (
        "acknowledge-risk",
        "verification command는 side effect 가능성이 있어 silent rollback 대신 acknowledge-risk contract로 표면화한다.",
      ),
      _ => (
        "acknowledge-risk",
        "알 수 없는 effect class는 generic rollback 보장 대신 acknowledge-risk로 남긴다.",
      ),
    };
    contracts.push(CodingAgentRollbackEffectContract {
      effect_class,
      rollback_contract,
      rationale,
    });
  }

  contracts
}

fn classify_rollback_class(effect_contracts: &[CodingAgentRollbackEffectContract]) -> &'static str {
  if effect_contracts
    .iter()
    .any(|contract| contract.rollback_contract == "forbidden")
  {
    "forbidden"
  } else if effect_contracts
    .iter()
    .any(|contract| contract.rollback_contract == "acknowledge-risk")
  {
    "acknowledge-risk"
  } else {
    "rollbackable"
  }
}

fn make_apply_artifact_ref(
  request: Option<&str>,
  target_paths: &[String],
  current_plan_ref: Option<&str>,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(request.unwrap_or_default().as_bytes());
  hasher.update(b"\n--apply-targets--\n");
  for path in target_paths {
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
  }
  hasher.update(b"--apply-plan--\n");
  hasher.update(current_plan_ref.unwrap_or_default().as_bytes());
  format!("coding.apply-intent::{:x}", hasher.finalize())
}

fn make_inverse_plan_ref(apply_artifact_ref: &str) -> String {
  let suffix = apply_artifact_ref
    .rsplit_once("::")
    .map(|(_, suffix)| suffix)
    .unwrap_or(apply_artifact_ref);
  format!("coding.inverse-plan::{}", suffix)
}

fn make_rollback_handle_id(
  repo_snapshot_ref: &str,
  apply_artifact_ref: &str,
  rollback_class: &str,
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(repo_snapshot_ref.as_bytes());
  hasher.update(b"\n--apply-artifact--\n");
  hasher.update(apply_artifact_ref.as_bytes());
  hasher.update(b"\n--rollback-class--\n");
  hasher.update(rollback_class.as_bytes());
  format!("coding.rollback-handle::{:x}", hasher.finalize())
}

fn make_restored_snapshot_ref_from_files(
  repo_snapshot_ref: &str,
  handle_ref: &str,
  file_results: &[CodingAgentPatchFileApplyRecord],
) -> String {
  let mut hasher = Sha256::new();
  hasher.update(repo_snapshot_ref.as_bytes());
  hasher.update(b"\n--rollback-handle--\n");
  hasher.update(handle_ref.as_bytes());
  hasher.update(b"\n--restored-files--\n");
  for file in file_results {
    hasher.update(file.path.as_bytes());
    hasher.update(b"\n");
    hasher.update(file.status.as_bytes());
    hasher.update(b"\n");
    hasher.update(
      file
        .after_snapshot_ref
        .as_deref()
        .unwrap_or_default()
        .as_bytes(),
    );
    hasher.update(b"\n");
  }
  format!("coding.repo-snapshot::restored::{:x}", hasher.finalize())
}

fn make_followup_verify_ref(restored_snapshot_ref: &str, handle_ref: &str) -> String {
  let mut hasher = Sha256::new();
  hasher.update(restored_snapshot_ref.as_bytes());
  hasher.update(b"\n--rollback-handle--\n");
  hasher.update(handle_ref.as_bytes());
  format!(
    "coding.verify-receipt::post-rollback::{:x}",
    hasher.finalize()
  )
}

fn build_rollback_proof_refs(
  request: &CodingAgentRequestArtifact,
  repo_snapshot_ref: &str,
  apply_artifact_ref: &str,
  handle_id: &str,
  rollback_class: &str,
  effect_contracts: &[CodingAgentRollbackEffectContract],
) -> Vec<String> {
  let mut proof_refs = Vec::new();
  proof_refs.push(format!("repo-snapshot-ref:{}", repo_snapshot_ref));
  proof_refs.push(format!("apply-artifact-ref:{}", apply_artifact_ref));
  proof_refs.push(format!("handle-id:{}", handle_id));
  proof_refs.push(format!("rollback-class:{}", rollback_class));
  if let Some(plan_ref) = request.workspace.current_plan_ref.as_deref() {
    proof_refs.push(format!("current-plan-ref:{}", plan_ref));
  }
  if let Some(last_ref) = request.workspace.last_verification_ref.as_deref() {
    proof_refs.push(format!("last-verification-ref:{}", last_ref));
  }
  if let Some(rollback_ref) = request.workspace.rollback_handle_ref.as_deref() {
    proof_refs.push(format!("previous-rollback-handle-ref:{}", rollback_ref));
  }
  for contract in effect_contracts.iter().take(6) {
    proof_refs.push(format!(
      "effect-contract:{}=>{}",
      contract.effect_class, contract.rollback_contract
    ));
  }
  if let Some(receipt) = request.manual_evidence_seed.uncertainty_receipts.first() {
    proof_refs.push(format!("manual-evidence-receipt:{}", receipt));
  }
  proof_refs
}

fn build_rollback_receipt_proof_refs(
  request: &CodingAgentRequestArtifact,
  handle_ref: &str,
  repo_snapshot_ref: &str,
  apply_artifact_ref: &str,
  rollback_class: &str,
  restored_snapshot_ref: Option<&str>,
  followup_verify_ref: Option<&str>,
  effect_contracts: &[CodingAgentRollbackEffectContract],
  rollback_status: &str,
  rollback_input_ref: Option<&str>,
  file_results: &[CodingAgentPatchFileApplyRecord],
) -> Vec<String> {
  let mut proof_refs = Vec::new();
  proof_refs.push(format!("handle-ref:{}", handle_ref));
  proof_refs.push(format!("repo-snapshot-ref:{}", repo_snapshot_ref));
  proof_refs.push(format!("apply-artifact-ref:{}", apply_artifact_ref));
  proof_refs.push(format!("rollback-class:{}", rollback_class));
  proof_refs.push(format!("rollback-status:{}", rollback_status));
  if let Some(rollback_input_ref) = rollback_input_ref {
    proof_refs.push(format!("rollback-input-ref:{}", rollback_input_ref));
  }
  if let Some(restored_snapshot_ref) = restored_snapshot_ref {
    proof_refs.push(format!("restored-snapshot-ref:{}", restored_snapshot_ref));
  }
  if let Some(followup_verify_ref) = followup_verify_ref {
    proof_refs.push(format!("followup-verify-ref:{}", followup_verify_ref));
  }
  if let Some(plan_ref) = request.workspace.current_plan_ref.as_deref() {
    proof_refs.push(format!("current-plan-ref:{}", plan_ref));
  }
  if let Some(last_ref) = request.workspace.last_verification_ref.as_deref() {
    proof_refs.push(format!("last-verification-ref:{}", last_ref));
  }
  for contract in effect_contracts.iter().take(6) {
    proof_refs.push(format!(
      "effect-contract:{}=>{}",
      contract.effect_class, contract.rollback_contract
    ));
  }
  for file in file_results.iter().take(8) {
    proof_refs.push(format!("rollback-file:{}:{}", file.path, file.status));
    if let Some(after) = file.after_snapshot_ref.as_deref() {
      proof_refs.push(format!("rollback-after-snapshot-ref:{}", after));
    }
  }
  if let Some(receipt) = request.manual_evidence_seed.uncertainty_receipts.first() {
    proof_refs.push(format!("manual-evidence-receipt:{}", receipt));
  }
  proof_refs
}

fn is_docs_path(path: &str) -> bool {
  path.ends_with(".md") || path.starts_with("docs/") || path.contains("/docs/")
}

fn is_test_path(path: &str) -> bool {
  path.starts_with("tests/")
    || path.contains("/tests/")
    || path.ends_with("_test.rs")
    || path.ends_with("_spec.rs")
}

fn print_grounding_seed_text(grounding_seed: &CodingAgentGroundingSeed) {
  println!(
    "grounding-seed: {} entries via {} ({})",
    grounding_seed.entries.len(),
    grounding_seed.parser_owner,
    grounding_seed.scan_mode
  );
  for entry in grounding_seed.entries.iter().take(5) {
    println!(
      "  - {} [{} | {} | {}]",
      entry.path, entry.language, entry.parser_backend, entry.parser_capability
    );
  }
}

fn print_repo_graph_seed_text(repo_graph_seed: &CodingAgentRepoGraphSeed) {
  println!(
    "repo-graph-seed: {} files via {} ({}, {}, {})",
    repo_graph_seed.files.len(),
    repo_graph_seed.graph_owner,
    repo_graph_seed.bundle_scope,
    repo_graph_seed.graph_capability,
    repo_graph_seed.project_graph_status
  );
  println!(
    "repo-graph-refresh: {} changed={} batch={} project-refs={} seto={}",
    repo_graph_seed.incremental_refresh.refresh_mode,
    repo_graph_seed.incremental_refresh.changed_files.len(),
    repo_graph_seed.incremental_refresh.refresh_batch.len(),
    repo_graph_seed.project_reference_edges.len(),
    repo_graph_seed.seto_enrichment_state
  );
  for file in repo_graph_seed.files.iter().take(3) {
    println!(
      "  - {} [{} | {} | {}] symbols={} refs={} tests={} entrypoints={}",
      file.file_anchor,
      file.language,
      file.parser_backend,
      file.parser_capability,
      file.symbol_nodes.len(),
      file.reference_edges.len(),
      file.test_targets.len(),
      file.runtime_entrypoints.len()
    );
  }
  for edge in repo_graph_seed.project_reference_edges.iter().take(3) {
    println!(
      "  - project-ref {} -> {} via {} ({})",
      edge.from_file_anchor, edge.to_file_anchor, edge.via_term, edge.edge_kind
    );
  }
}

fn print_manual_evidence_seed_text(manual_evidence_seed: &CodingAgentManualEvidenceSeed) {
  println!(
    "manual-evidence-seed: {} hits via {}",
    manual_evidence_seed.hits.len(),
    manual_evidence_seed.join_owner
  );
  println!(
    "manual-evidence-policy: {}",
    manual_evidence_seed.join_policy
  );
  if !manual_evidence_seed.language_hints.is_empty() {
    println!(
      "manual-evidence-languages: {}",
      manual_evidence_seed.language_hints.join(", ")
    );
  }
  for hit in manual_evidence_seed.hits.iter().take(3) {
    println!(
      "  - {} [{} | {}]",
      hit.manual_ref, hit.term, hit.join_status
    );
  }
  for receipt in manual_evidence_seed.uncertainty_receipts.iter().take(4) {
    println!("  - receipt: {}", receipt);
  }
}

fn print_attached_pack_seed_text(attached_pack_seed: &CodingAgentAttachedPackSeed) {
  let root_count =
    attached_pack_seed.project_pack_roots.len() + attached_pack_seed.history_pack_roots.len();
  if root_count == 0 {
    return;
  }
  println!(
    "attached-pack-seed: {} roots / {} entries via {}",
    root_count, attached_pack_seed.total_entry_count, attached_pack_seed.attach_owner
  );
  for root in attached_pack_seed
    .project_pack_roots
    .iter()
    .chain(attached_pack_seed.history_pack_roots.iter())
    .take(4)
  {
    println!(
      "  - {} [{} | {} | entries={}]",
      root.root, root.pack_kind, root.status, root.entry_count
    );
    for entry in root.entries.iter().take(2) {
      println!("    - {} ({})", entry.entry_ref, entry.entry_kind);
    }
  }
}

fn print_language_profile_text(language_profile: &CodingAgentLanguageProfileSurface) {
  if language_profile.supported_adapters.is_empty()
    && language_profile.unsupported_targets.is_empty()
  {
    return;
  }
  println!(
    "language-profile: {} semantic / {} effect / {} verify / {} diagnostics via {}",
    language_profile.semantic_records.len(),
    language_profile.effect_records.len(),
    language_profile.verify_targets.len(),
    language_profile.diagnostic_records.len(),
    language_profile.profile_owner
  );
  println!(
    "language-profile-boundary: {}",
    language_profile.adapter_boundary
  );
  for adapter in language_profile.supported_adapters.iter().take(4) {
    println!(
      "  - adapter {} [{} targets={}]",
      adapter.language, adapter.adapter_status, adapter.target_count
    );
  }
  for target in language_profile.verify_targets.iter().take(4) {
    println!(
      "  - verify-target {} [{} | {}]",
      target.target_path, target.language, target.verify_family
    );
  }
  for target in language_profile.unsupported_targets.iter().take(3) {
    println!(
      "  - unsupported {} [{} | {}]",
      target.target_path, target.detected_language, target.status
    );
  }
  for demand in language_profile.context_demands.iter().take(3) {
    println!(
      "  - context-demand {} [{} | {}]",
      demand.target_path, demand.language, demand.demand_family
    );
  }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
  needles.iter().any(|needle| haystack.contains(needle))
}

#[derive(Debug, Default)]
struct GitWorkspaceProbe {
  repo_root: Option<String>,
  branch: Option<String>,
  head_commit: Option<String>,
  dirty: bool,
}

fn probe_git_workspace(cwd: &Path) -> GitWorkspaceProbe {
  let Some(repo_root) = run_git_capture(cwd, ["rev-parse", "--show-toplevel"]) else {
    return GitWorkspaceProbe::default();
  };

  let branch = run_git_capture(cwd, ["rev-parse", "--abbrev-ref", "HEAD"]);
  let head_commit = run_git_capture(cwd, ["rev-parse", "HEAD"]);
  let dirty = run_git_capture(cwd, ["status", "--porcelain"])
    .map(|output| !output.trim().is_empty())
    .unwrap_or(false);

  GitWorkspaceProbe {
    repo_root: Some(repo_root),
    branch,
    head_commit,
    dirty,
  }
}

fn run_git_capture<const N: usize>(cwd: &Path, args: [&str; N]) -> Option<String> {
  let output = Command::new("git")
    .args(args)
    .current_dir(cwd)
    .output()
    .ok()?;
  if !output.status.success() {
    return None;
  }
  let text = String::from_utf8(output.stdout).ok()?;
  let trimmed = text.trim();
  if trimmed.is_empty() {
    None
  } else {
    Some(trimmed.to_string())
  }
}

fn normalize_newlines(input: &str) -> Cow<'_, str> {
  if input.contains('\r') {
    Cow::Owned(input.replace("\r\n", "\n").replace('\r', "\n"))
  } else {
    Cow::Borrowed(input)
  }
}

fn path_to_slash(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/")
}

#[derive(Debug, Clone)]
enum EmitTarget {
  Legacy(CodegenTarget),
  LegacyAll,
  Aot(AotTarget),
}

fn normalize_supervisor_endpoint(raw: &str) -> String {
  let value = raw.trim();
  if value.starts_with("uds:") || value.starts_with("tls:") || value.starts_with("tls://") {
    value.to_string()
  } else {
    format!("uds:{}", value)
  }
}

fn maybe_register_trace_artifact(
  invocation_id: Option<&str>,
  supervisor_endpoint: Option<&str>,
  trace_path: &Path,
) {
  let Some(invocation_id) = invocation_id
    .map(str::trim)
    .filter(|value| !value.is_empty())
  else {
    return;
  };
  let Some(endpoint) = supervisor_endpoint
    .map(str::trim)
    .filter(|value| !value.is_empty())
  else {
    return;
  };

  let trace_path = match std::fs::canonicalize(trace_path) {
    Ok(path) => path,
    Err(error) => {
      eprintln!(
        "warning: trace.register skipped (canonicalize failed for {}): {}",
        trace_path.display(),
        error
      );
      return;
    }
  };
  let size_bytes = std::fs::metadata(&trace_path).ok().map(|meta| meta.len());

  let client =
    match pnix_runtime_supervisor::client::SupervisorClient::connect(endpoint.to_string()) {
      Ok(client) => client,
      Err(error) => {
        eprintln!(
          "warning: trace.register skipped (connect {} failed): {}",
          endpoint, error
        );
        return;
      }
    };

  if let Err(error) = client.call(
    "trace.register",
    json!({
      "invocation_id": invocation_id,
      "path": trace_path,
      "trace_mode": "full",
      "size_bytes": size_bytes,
    }),
  ) {
    eprintln!(
      "warning: trace.register failed for invocation {} at {}: {}",
      invocation_id,
      trace_path.display(),
      error
    );
  }
}

async fn run_graph(args: &Args) -> Result<()> {
  if let Some(endpoint) = args.supervisor_sock.as_ref() {
    let normalized = normalize_supervisor_endpoint(endpoint);
    std::env::set_var("PNIX_SUPERVISOR_ENDPOINT", &normalized);
    std::env::set_var("PNIX_SUPERVISOR_SOCK", endpoint);
  }

  let dist = args
    .dist
    .as_ref()
    .ok_or_else(|| anyhow::anyhow!("--dist is required for mode graph"))?;
  let resource_limits = resource_limits_from_args(args);

  // Read replay.json for hash (async I/O)
  let replay_path = dist.join("pnix.replay.json");
  let replay_v: serde_json::Value =
    read_json_file_with_limits_async(&replay_path, &resource_limits, "replay json").await?;
  let replay_hash = replay_v
    .get("replay_hash")
    .and_then(|v| v.as_str())
    .unwrap_or("<unknown>");

  // Read fxcore.canon.json (async I/O)
  let fx_path = dist.join("ir").join("fxcore.canon.json");
  let mut fx: model::FxCoreModule =
    read_json_file_with_limits_async(&fx_path, &resource_limits, "fxcore json").await?;

  let mut patch_results: Option<Vec<patch::PatchOpResult>> = None;
  if let Some(patch_path) = args.patch.as_ref() {
    let patch: patch::FxCorePatch =
      read_json_file_with_limits_async(patch_path, &resource_limits, "patch json").await?;
    let (patched, results) = patch::apply_patch_with_results(fx, patch)?;
    fx = patched;
    patch_results = Some(results);
    eprintln!("info: applied patch {}", patch_path.display());
  }

  let replay_config = if let Some(trace_path) = args.replay_trace.as_ref() {
    let mode = ReplayMode::parse(args.replay_mode.as_deref())?;
    let db = ReplayDB::load(trace_path)?;
    if mode != ReplayMode::Off {
      std::env::set_var("PNIX_REPLAY_BLOCK_PROCESS", "1");
    }
    Some(ReplayConfig {
      mode,
      trace_path: trace_path.display().to_string(),
      db,
      allow_classes: args.replay_allow.iter().cloned().collect(),
    })
  } else {
    None
  };

  // Version check
  if !fx.meta.version.is_empty() && !SUPPORTED_FXCORE_VERSIONS.contains(&fx.meta.version.as_str()) {
    anyhow::bail!(
      "IR version mismatch: IR has '{}' but executor supports {:?}\n\
             Please regenerate IR with compatible pnix-core or upgrade executor",
      fx.meta.version,
      SUPPORTED_FXCORE_VERSIONS
    );
  }

  validate_graph_inputs(&fx, &args.inputs)?;
  verify_resource_limits(&fx, &resource_limits)?;

  // W06b: Capability 체크 (UsedSpec 읽기)
  let required_caps = if let Some(used_spec) = capability_check::load_used_spec_from_dist(dist)? {
    capability_check::extract_required_capabilities(&used_spec)?
  } else {
    Vec::new()
  };
  if !required_caps.is_empty() {
    eprintln!("info: required capabilities: {:?}", required_caps);
  }

  let requires_process = required_caps.iter().any(|cap| {
    matches!(
      cap,
      RuntimeCapability::Process
        | RuntimeCapability::ProcessSpawn
        | RuntimeCapability::ProcessSignal
        | RuntimeCapability::ProcessObserve
    )
  });
  let supervisor_endpoint = std::env::var("PNIX_SUPERVISOR_ENDPOINT")
    .ok()
    .filter(|value| !value.trim().is_empty())
    .or_else(|| {
      std::env::var("PNIX_SUPERVISOR_SOCK")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| normalize_supervisor_endpoint(&value))
    })
    .or_else(|| {
      args
        .supervisor_sock
        .as_deref()
        .map(normalize_supervisor_endpoint)
    });
  let has_supervisor = supervisor_endpoint.is_some();

  let backend_specs_env = std::env::var("PNIX_BACKEND_SPECS")
    .ok()
    .filter(|value| !value.trim().is_empty())
    .map(PathBuf::from);
  let backend_catalog = if args.auto_ensure_backends {
    backend_catalog::BackendCatalog::load(
      args.backend_specs.as_deref(),
      backend_specs_env.as_deref(),
      Some(Path::new("config/backends.json")),
    )?
  } else {
    None
  };

  let has_external_backend_nodes = fx.nodes.iter().any(|node| {
    let backend = crate::rpc::backend_of(&node.uses);
    backend != "builtins" && backend != "nix"
  });
  let implicit_process_need =
    args.auto_ensure_backends && backend_catalog.is_some() && has_external_backend_nodes;
  let needs_process_guard = requires_process || implicit_process_need;

  if needs_process_guard && !has_supervisor {
    match replay_config.as_ref().map(|cfg| cfg.mode) {
      Some(ReplayMode::Strict) => {
        eprintln!(
          "info: supervisor missing but strict replay mode is enabled; runtime process orchestration is skipped"
        );
      }
      Some(ReplayMode::NondetSafe) | Some(ReplayMode::Verify) => {
        let mut missing = Vec::new();
        for node in &fx.nodes {
          let (_, replay_class) =
            crate::replay_classify::classify_uses(&node.uses, node.meta.as_ref(), None);
          let is_external_runtime = matches!(
            replay_class.as_deref(),
            Some("external_world/process") | Some("external_world/backend")
          );
          if !is_external_runtime {
            continue;
          }
          let replay_key = node
            .meta
            .as_ref()
            .and_then(|meta| meta.replay_key.as_deref());
          let Some(replay_key) = replay_key else {
            missing.push(format!("{}:<missing replay_key>", node.name));
            continue;
          };
          if let Some(cfg) = replay_config.as_ref() {
            if !cfg.db.contains_replay_key(replay_key) {
              missing.push(format!("{}:{}", node.name, replay_key));
            }
          }
        }
        if !missing.is_empty() {
          bail!(
            "process/external runtime execution requires supervisor or replay coverage; missing replay keys: {:?}",
            missing
          );
        }
      }
      _ => {
        bail!(
          "graph requires runtime process orchestration ({:?}), but supervisor endpoint is not set.\n\
           set PNIX_SUPERVISOR_ENDPOINT=tls://host:port (or uds:/path),\n\
           or set PNIX_SUPERVISOR_SOCK=/tmp/pnix-supervisor.sock,\n\
           start supervisor: pnix-supervisor --uds /tmp/pnix-supervisor.sock --force\n\
           or pass: --supervisor-sock /tmp/pnix-supervisor.sock",
          required_caps
        );
      }
    }
  }

  let backend_supervisor = if let (Some(catalog), Some(endpoint)) =
    (backend_catalog.clone(), supervisor_endpoint.as_ref())
  {
    Some(BackendSupervisor::new(endpoint.clone(), catalog)?)
  } else {
    None
  };

  // Batch apply restriction: only allowed for Stage-1
  // Stage-2+ uses inputs/outputs map which may have runtime dependencies
  // Stage-3+ has conditional edges, gates, optional nodes requiring sequential execution
  let effective_batch = if fx.meta.stage > 1 && args.use_batch {
    eprintln!(
      "info: batch apply disabled for Stage-{} IR (requires sequential execution)",
      fx.meta.stage
    );
    false
  } else {
    args.use_batch
  };

  // Build execution plan
  let plan = plan::build_plan(&fx)?;

  if plan.order.is_empty() {
    // Stage-0 mode: no nodes to apply
    eprintln!("info: no nodes in graph, nothing to apply");
    let mut output = serde_json::json!({
        "status": "ok",
        "replay_hash": replay_hash,
        "note": "no nodes",
    });
    if let Some(results) = patch_results {
      output["patch_results"] = serde_json::to_value(results)?;
    }
    println!("{}", serde_json::to_string(&output)?);
    return Ok(());
  }

  eprintln!(
    "info: applying {} nodes in order: {:?}",
    plan.order.len(),
    plan.order
  );

  if !args.inputs.is_empty() {
    let mut keys: Vec<String> = args.inputs.keys().cloned().collect();
    keys.sort();
    eprintln!(
      "info: external inputs: count={}, keys={:?}",
      keys.len(),
      keys
    );
  }

  if args.dry_run {
    eprintln!("info: dry-run enabled, skipping apply_graph");
    let mut output = serde_json::json!({
      "status": "ok",
      "mode": "dry-run",
      "replay_hash": replay_hash,
      "nodes_planned": plan.order.len(),
      "stage": fx.meta.stage,
    });
    if let Some(results) = patch_results {
      output["patch_results"] = serde_json::to_value(results)?;
    }
    println!("{}", serde_json::to_string(&output)?);
    return Ok(());
  }

  // Apply graph
  let inputs: BTreeMap<String, serde_json::Value> = args
    .inputs
    .iter()
    .map(|(k, v)| (k.clone(), v.clone()))
    .collect();
  let allow_non_atomic_effects = env_flag_enabled("PNIX_ALLOW_NON_ATOMIC_EFFECTS");
  if allow_non_atomic_effects {
    eprintln!("warn: allowing non-atomic side-effect execution (PNIX_ALLOW_NON_ATOMIC_EFFECTS=1)");
  }
  let config = BackendConfig {
    clojure_url: args.clojure_url.clone(),
    python_url: args.python_url.clone(),
    deno_url: args.deno_url.clone(),
    blenderpy_url: args.blenderpy_url.clone(),
    rpc_timeout_ms: args.rpc_timeout_ms,
    rpc_retry_attempts: args.rpc_retry_attempts,
    rpc_retry_backoff_ms: args.rpc_retry_backoff_ms,
    rpc_retry_seed: 0,
    use_batch_apply: effective_batch,
    allow_non_atomic_effects,
    inputs,
    resource_limits,
  };

  let result = apply::apply_graph_with_options(
    &fx,
    &plan,
    replay_hash,
    &config,
    apply::ApplyOptions {
      replay: replay_config.as_ref(),
      backend_supervisor: backend_supervisor.as_ref(),
      invocation_id: args.invocation_id.as_deref(),
    },
  )
  .await?;

  // Write outputs
  output::write_apply_graph(dist, &result)?;
  output::write_trace(dist, &result, Some(&fx))?;
  maybe_register_trace_artifact(
    args.invocation_id.as_deref(),
    supervisor_endpoint.as_deref(),
    &dist.join("pnix.apply_trace.jsonl"),
  );

  let mode = if result.batch_applied {
    "batch"
  } else {
    "individual"
  };
  eprintln!("info: apply mode: {}, stage: {}", mode, fx.meta.stage);

  let status_str = match result.status {
    apply::ApplyStatus::Ok => "ok",
    apply::ApplyStatus::Partial => "partial",
    apply::ApplyStatus::Error => "error",
  };

  let mut output = serde_json::json!({
      "status": status_str,
      "replay_hash": replay_hash,
      "nodes_total": plan.order.len(),
      "nodes_applied": result.nodes_ok + result.nodes_failed,
      "nodes_ok": result.nodes_ok,
      "nodes_failed": result.nodes_failed,
      "nodes_skipped": result.nodes_skipped,
      "batch_applied": result.batch_applied,
      "stage": fx.meta.stage,
  });
  if let Some(results) = patch_results {
    output["patch_results"] = serde_json::to_value(results)?;
  }
  println!("{}", serde_json::to_string(&output)?);

  if result.nodes_failed > 0 {
    anyhow::bail!(
      "apply_graph completed with failures (status={}, nodes_failed={})",
      status_str,
      result.nodes_failed
    );
  }

  Ok(())
}

async fn run_run(args: &Args) -> Result<()> {
  let dist = args
    .dist
    .as_ref()
    .ok_or_else(|| anyhow::anyhow!("--dist is required for mode run"))?;

  // If a source is provided, treat it as a Pnix module and compile it to dist first.
  // (We ignore --dry-run for the compile step because subsequent engines require dist files.)
  if args.source.is_some() || args.expr.is_some() {
    let mut compile_args = args.clone();
    compile_args.dry_run = false;
    run_compile_quiet(&compile_args)?;
  }

  let engine = match args.engine.as_deref() {
    Some("auto") => {
      let selected = select_run_engine(args, dist)?;
      eprintln!("info: auto-selected engine={}", selected);
      selected
    }
    Some(value) => value.to_string(),
    None => "graph".to_string(),
  };

  match engine.as_str() {
        "graph" => run_graph(args).await,
        "ir-eval" | "ir" => run_ir_eval(args),
        "ssa" | "legacy-ssa" => run_ssa(args),
        "parity" => run_parity(args),
        "ui" => run_ui(args),
        "emit" => {
            if args.dry_run {
                anyhow::bail!("--dry-run is not supported for run --engine emit");
            }
            run_emit(args)
        }
        "llvm" => {
            if args.dry_run {
                anyhow::bail!("--dry-run is not supported for run --engine llvm");
            }
            let mut llvm_args = args.clone();
            llvm_args.expr = None;
            llvm_args.source = Some(dist.join("ir").join("fxcore.canon.json").display().to_string());
            run_llvm(&llvm_args)
        }
        other => anyhow::bail!(
            "unknown run engine '{}'\n\
             supported: auto, graph, ir-eval, ssa, parity, ui, emit, llvm\n\
             (advanced compat modes: graph/emit/llvm can also be invoked via --mode graph/--emit/--mode llvm)",
            other
        ),
    }
}

fn select_run_engine(args: &Args, dist: &Path) -> Result<String> {
  let resource_limits = resource_limits_from_args(args);
  let fx = load_fxcore_from_dist(dist, &resource_limits)?;

  if requires_graph_engine(&fx) {
    return Ok("graph".to_string());
  }

  if reject_scopes_for_ir_eval(&fx).is_err() {
    return Ok("graph".to_string());
  }

  Ok("ir-eval".to_string())
}

fn requires_graph_engine(fx: &CoreFxCoreModule) -> bool {
  fx.nodes.iter().any(|node| {
    let backend = crate::rpc::backend_of(&node.uses);
    backend != "builtins"
  })
}

fn env_flag_enabled(name: &str) -> bool {
  std::env::var(name)
    .ok()
    .map(|raw| {
      let value = raw.trim();
      value == "1"
        || value.eq_ignore_ascii_case("true")
        || value.eq_ignore_ascii_case("yes")
        || value.eq_ignore_ascii_case("on")
    })
    .unwrap_or(false)
}

fn run_ir_eval(args: &Args) -> Result<()> {
  let output = ir_eval_output_value(args)?;
  println!("{}", serde_json::to_string(&output)?);
  Ok(())
}

fn run_ssa(args: &Args) -> Result<()> {
  let output = ssa_output_value(args)?;
  println!("{}", serde_json::to_string(&output)?);
  Ok(())
}

fn run_parity(args: &Args) -> Result<()> {
  let resource_limits = resource_limits_from_args(args);
  let ir_value = extract_engine_value(ir_eval_output_value(args)?);
  let ssa_value = extract_engine_value(ssa_output_value(args)?);

  let legacy_expr = std::env::var("PNIX_PARITY_LEGACY_EXPR")
    .ok()
    .and_then(|raw| {
      let trimmed = raw.trim();
      if trimmed.is_empty() {
        None
      } else {
        Some(trimmed.to_string())
      }
    });
  let legacy_value = if let Some(path) = legacy_expr.as_deref() {
    Some(extract_engine_value(legacy_eval_value_with_inputs(
      path, args,
    )?))
  } else {
    None
  };

  let run_llvm = std::env::var("PNIX_PARITY_LLVM")
    .ok()
    .map(|raw| {
      let normalized = raw.trim().to_ascii_lowercase();
      !normalized.is_empty() && normalized != "0" && normalized != "false"
    })
    .unwrap_or(false);
  let llvm_value = if run_llvm {
    let dist = args
      .dist
      .as_ref()
      .ok_or_else(|| anyhow::anyhow!("--dist is required for run --engine parity"))?;
    let fx_path = dist.join("ir").join("fxcore.canon.json");
    let fx: model::FxCoreModule =
      read_json_file_with_limits(&fx_path, &resource_limits, "fxcore json")?;
    Some(extract_engine_value(llvm_value_from_fxcore(args, &fx)?))
  } else {
    None
  };

  let output = serde_json::json!({
    "ok": true,
    "ir": ir_value,
    "ssa": ssa_value,
    "legacy": legacy_value.unwrap_or(serde_json::Value::Null),
    "llvm": llvm_value.unwrap_or(serde_json::Value::Null),
  });
  println!("{}", serde_json::to_string(&output)?);
  Ok(())
}

fn run_ui(args: &Args) -> Result<()> {
  let dist = args
    .dist
    .as_ref()
    .ok_or_else(|| anyhow::anyhow!("--dist is required for run --engine ui"))?;

  let raw = ir_eval_output_value(args)?;
  let value = extract_engine_value(raw);
  let frame = frame_from_engine_value(value)?;

  if args.live {
    emit_live_snapshot(args, Some(dist), &frame)?;
  }

  let output = serde_json::json!({
    "ok": true,
    "engine": "ui",
    "frame": frame
  });
  println!("{}", serde_json::to_string(&output)?);
  Ok(())
}

fn run_ui_interpret(args: &Args) -> Result<()> {
  let value = if args.patch.is_none() {
    if let Some(value) = ui_value_from_source(args) {
      normalize_json_numbers(value)
    } else {
      extract_engine_value(legacy_eval_output_value(args)?)
    }
  } else {
    extract_engine_value(legacy_eval_output_value(args)?)
  };
  let frame = frame_from_engine_value(value)?;

  if args.live {
    emit_live_snapshot(args, None, &frame)?;
  }

  let output = serde_json::json!({
    "ok": true,
    "engine": "ui",
    "frame": frame
  });
  println!("{}", serde_json::to_string(&output)?);
  Ok(())
}

fn frame_from_engine_value(value: serde_json::Value) -> Result<FramePacket> {
  match frame_from_json_value(value.clone()) {
    Ok(frame) => Ok(frame),
    Err(UiError::TypeMismatch { .. }) => Ok(frame_from_text_value(value)),
    Err(err) => Err(anyhow::anyhow!("ui decode failed: {}", err)),
  }
}

fn frame_from_text_value(value: serde_json::Value) -> FramePacket {
  let text = match value {
    serde_json::Value::String(value) => value,
    other => serde_json::to_string(&other).unwrap_or_else(|_| "<unprintable>".to_string()),
  };

  let commands = vec![
    DrawCommand::DrawRect {
      rect: Rect::new(0.0, 0.0, 640.0, 360.0),
      paint: Paint::Solid(Color::rgba(0.08, 0.1, 0.14, 1.0)),
      stroke: None,
    },
    DrawCommand::Transform(Transform2D {
      translate: [32.0, 48.0],
      scale: [1.0, 1.0],
      rotate: 0.0,
    }),
    DrawCommand::DrawText {
      text: TextRun::new(text, FontSpec::new("Default", 500, false), 24.0),
      paint: Paint::Solid(Color::rgba(0.94, 0.96, 0.98, 1.0)),
    },
  ];

  let mut frame = FramePacket::empty(1);
  frame.draw_ir_2d = DrawIR2D::new(commands);
  frame
}

fn ui_value_from_source(args: &Args) -> Option<serde_json::Value> {
  let (source, _path) = read_source(args).ok()?;
  let normalized = normalize_pnix_list_separators(&source);
  let expr = parse_expr(&normalized).ok()?;
  pnix_expr_to_json(&expr).ok()
}

fn emit_live_snapshot(
  args: &Args,
  dist: Option<&Path>,
  frame: &freecat_runtime_ui::FramePacket,
) -> Result<()> {
  let live_dir = args.live_dir.clone().unwrap_or_else(default_live_dir);
  let paths = LivePaths::new(live_dir);
  let resource_limits = resource_limits_from_args(args);

  let (graph_hash, ir_nodes) = match dist {
    Some(dist) => (
      load_replay_hash(dist, &resource_limits).unwrap_or_default(),
      load_fxcore_from_dist(dist, &resource_limits)
        .map(|fx| fx.nodes.len())
        .unwrap_or(0),
    ),
    None => (String::new(), 0),
  };
  let input = args
    .expr
    .as_deref()
    .or(args.source.as_deref())
    .map(|value| value.to_string())
    .or_else(|| dist.map(|dist| dist.display().to_string()))
    .unwrap_or_else(|| "<expr>".to_string());

  let now_ms = current_time_ms();
  let update = LiveUpdate {
    version: LIVE_VERSION,
    schema_version: LIVE_SCHEMA_VERSION.to_string(),
    seq: now_ms,
    timestamp_ms: now_ms,
    mode: LiveMode::Code,
    input,
    seto_ids: Vec::new(),
    ir_nodes,
    graph_hash,
    output: None,
    query_spec: None,
    algo_result: None,
    algo_code: None,
    algo_lang: None,
    frame: frame.clone(),
  };

  write_live_snapshot(&paths, &update)
}

fn write_live_snapshot(paths: &LivePaths, update: &LiveUpdate) -> Result<()> {
  std::fs::create_dir_all(&paths.dir)
    .map_err(|err| anyhow::anyhow!("failed to create live dir: {}", err))?;
  let payload = serde_json::to_vec_pretty(update)?;
  let tmp_path = paths.snapshot.with_extension("json.tmp");

  if let Err(err) = std::fs::write(&tmp_path, &payload) {
    let _ = std::fs::remove_file(&tmp_path);
    return Err(anyhow::anyhow!("failed to write live snapshot: {}", err));
  }

  std::fs::rename(&tmp_path, &paths.snapshot)
    .map_err(|err| anyhow::anyhow!("failed to finalize live snapshot: {}", err))?;
  Ok(())
}

fn load_replay_hash(dist: &Path, limits: &ResourceLimits) -> Option<String> {
  let replay_path = dist.join("pnix.replay.json");
  let value: serde_json::Value =
    read_json_file_with_limits(&replay_path, limits, "replay json").ok()?;
  value
    .get("replay_hash")
    .and_then(|value| value.as_str())
    .map(|value| value.to_string())
}

fn current_time_ms() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_millis() as u64)
    .unwrap_or(0)
}

#[cfg(test)]
mod live_tests {
  use super::*;

  #[test]
  fn write_live_snapshot_creates_snapshot_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = LivePaths::new(dir.path().to_path_buf());
    let frame = freecat_runtime_ui::FramePacket::empty(1);
    let update = LiveUpdate {
      version: LIVE_VERSION,
      schema_version: LIVE_SCHEMA_VERSION.to_string(),
      seq: 1,
      timestamp_ms: 1,
      mode: LiveMode::Code,
      input: "ui".to_string(),
      seto_ids: Vec::new(),
      ir_nodes: 0,
      graph_hash: "hash".to_string(),
      output: None,
      query_spec: None,
      algo_result: None,
      algo_code: None,
      algo_lang: None,
      frame: frame.clone(),
    };

    write_live_snapshot(&paths, &update).expect("write snapshot");

    let contents = std::fs::read_to_string(&paths.snapshot).expect("read snapshot");
    let decoded: LiveUpdate = serde_json::from_str(&contents).expect("decode snapshot");
    assert_eq!(decoded.version, LIVE_VERSION);
    assert_eq!(decoded.frame, frame);
  }
}

fn ir_eval_output_value(args: &Args) -> Result<serde_json::Value> {
  let dist = args
    .dist
    .as_ref()
    .ok_or_else(|| anyhow::anyhow!("--dist is required for run --engine ir-eval"))?;
  let resource_limits = resource_limits_from_args(args);

  let mut fx = load_fxcore_from_dist(dist, &resource_limits)?;
  let mut patch_results: Option<Vec<patch::PatchOpResult>> = None;
  if let Some(patch_path) = args.patch.as_ref() {
    let patch: patch::FxCorePatch =
      read_json_file_with_limits(patch_path, &resource_limits, "patch json")?;
    let (patched, results) = patch::apply_patch_with_results(fx, patch)?;
    fx = patched;
    patch_results = Some(results);
    eprintln!("info: applied patch {}", patch_path.display());
  }

  verify_resource_limits(&fx, &resource_limits)?;
  reject_scopes_for_ir_eval(&fx)?;
  let onfail_allowlist = collect_onfail_allowlist(&fx);
  let allow_onfail_approx = std::env::var("PNIX_IR_EVAL_ALLOW_ONFAIL_APPROX")
    .map(|raw| {
      matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
      )
    })
    .unwrap_or(false);
  if !onfail_allowlist.is_empty() && !allow_onfail_approx {
    anyhow::bail!(
      "run --engine ir-eval does not support EdgeCond::OnFail semantics safely (nodes: {:?})\n\
             use --mode run --engine graph, or set PNIX_IR_EVAL_ALLOW_ONFAIL_APPROX=1 to allow best-effort fail-as-null approximation",
      onfail_allowlist
    );
  }

  if let Some(used_spec) = capability_check::load_used_spec_from_dist(dist)? {
    let required_caps = capability_check::extract_required_capabilities(&used_spec)?;
    let checker = pnix_runtime_api::capability_example::LegacyEvalCapabilities;
    let result = capability_check::check_engine_capabilities(&checker, &required_caps, &[], &[])?;
    if !result.success {
      anyhow::bail!("{}", result.message);
    }
  }

  let adapter_output = fxcore_to_legacy_ir(&fx, &AdapterConfig { strict: true })
    .context("fxcore->legacy ir failed")?;
  if !adapter_output.report.warnings.is_empty() {
    eprintln!(
      "warn: adapter reported {} warning(s) in strict mode",
      adapter_output.report.warnings.len()
    );
  }
  let adapter_report = adapter_report_json(&adapter_output.report);
  let mut ir: LegacyIr = adapter_output.legacy;

  // For offline evaluation, we inject external inputs directly as variables (input_<name>)
  // and drop adapter-generated `inputs(...)` bindings which are for codegen.
  let required_inputs = collect_required_inputs(&fx);
  if !required_inputs.is_empty() && args.inputs.is_empty() {
    anyhow::bail!(
      "run --engine ir-eval requires --inputs/--inputs-json when module has inputs: {:?}",
      required_inputs
    );
  }
  if !args.inputs.is_empty() {
    validate_inputs_keys(
      "ir-eval",
      &args.inputs,
      required_inputs.iter().cloned().collect(),
    )?;
  }

  // Drop input binding instructions (they are codegen-oriented).
  ir.instructions
    .retain(|instr| !is_codegen_input_binding(instr));

  let config = EvalConfig {
    deterministic: args.deterministic,
    seed: args.seed,
    now_ms: args.now_ms,
    clock_step_ms: args.clock_step_ms,
    float_pattern_tolerance: None,
    verbose: false,
  };
  let mut ctx = IrEvalContext::from_config(&config);

  for input_name in &required_inputs {
    let value = args.inputs.get(input_name).ok_or_else(|| {
      anyhow::anyhow!(
        "missing required input '{}' for ir-eval (provide via --input/--inputs-json)",
        input_name
      )
    })?;
    ctx.set(format!("input_{}", input_name), value.clone());
  }

  let eval_result = if onfail_allowlist.is_empty() {
    pnix_runtime_legacy::ir::eval_ir(&ir, &mut ctx)
  } else {
    eprintln!(
      "warn: running ir-eval with best-effort OnFail approximation (PNIX_IR_EVAL_ALLOW_ONFAIL_APPROX=1)"
    );
    pnix_runtime_legacy::ir::eval_ir_with_fail_as_null(&ir, &mut ctx, &onfail_allowlist)
  };
  let value = eval_result.map_err(|err| {
    let hint = pnix_runtime_legacy::ir::IR_EVAL_HELP_HINT;
    match err {
      pnix_runtime_legacy::ir::IrEvalError::Unsupported(_)
      | pnix_runtime_legacy::ir::IrEvalError::UnknownOp(_) => {
        anyhow::anyhow!("legacy ir eval failed: {}\n{}", err, hint)
      }
      _ => anyhow::anyhow!("legacy ir eval failed: {}", err),
    }
  })?;

  let mut out = serde_json::Map::new();
  out.insert("ok".to_string(), serde_json::Value::Bool(true));
  out.insert(
    "engine".to_string(),
    serde_json::Value::String("ir-eval".to_string()),
  );
  // 2026-05-06: `adapter_report` is diagnostic output (the
  // morphism / port / capability adapter's running notes,
  // unsupported-edge list, warnings). It is NOT a result value.
  // Including it in the default `--engine ir-eval` JSON makes
  // every fixture's expected JSON stale on every adapter
  // change, which is reward-hacked drift. Opt in via
  // `PNIX_IR_EVAL_ADAPTER_REPORT=1` for debug runs; default off
  // so result-comparison fixtures stay stable across adapter
  // metadata churn.
  if std::env::var("PNIX_IR_EVAL_ADAPTER_REPORT")
    .map(|v| v != "0" && !v.is_empty())
    .unwrap_or(false)
  {
    out.insert("adapter_report".to_string(), adapter_report);
  } else {
    // Discard the locally-bound `adapter_report` so the
    // borrow-checker doesn't complain about the unused value
    // when the env-var is off.
    let _ = adapter_report;
  }
  if !onfail_allowlist.is_empty() {
    out.insert(
      "onfail_mode".to_string(),
      serde_json::Value::String("approx_fail_as_null".to_string()),
    );
  }

  if let Some(spec) = args.result.as_deref() {
    let selector = resolve_ir_eval_result_selector(spec, &fx)?;
    let selected = select_ir_eval_result_value(spec, &selector, &ctx, &fx)?;
    out.insert(
      "result".to_string(),
      serde_json::Value::String(selector.canonical),
    );
    out.insert("value".to_string(), selected);
    if let Some(results) = patch_results.as_ref() {
      out.insert("patch_results".to_string(), serde_json::to_value(results)?);
    }
    return Ok(serde_json::Value::Object(out));
  }

  // Stable "program ABI": by default return sink node outputs (nodes that have no outgoing edges).
  // If there is exactly one sink, keep the legacy shape: { ok, engine, value }.
  let sink_nodes = collect_sink_nodes(&fx);
  match sink_nodes.len() {
    0 => {
      out.insert("value".to_string(), value);
    }
    1 => {
      let node = &sink_nodes[0];
      let sink_value = ctx.get(node).cloned().unwrap_or(value);
      out.insert("value".to_string(), sink_value);
    }
    _ => {
      let mut outputs = serde_json::Map::new();
      for node in sink_nodes {
        outputs.insert(
          node.clone(),
          ctx.get(&node).cloned().unwrap_or(serde_json::Value::Null),
        );
      }
      out.insert("outputs".to_string(), serde_json::Value::Object(outputs));
    }
  }
  if let Some(results) = patch_results.as_ref() {
    out.insert("patch_results".to_string(), serde_json::to_value(results)?);
  }
  Ok(serde_json::Value::Object(out))
}

fn ssa_output_value(args: &Args) -> Result<serde_json::Value> {
  let dist = args
    .dist
    .as_ref()
    .ok_or_else(|| anyhow::anyhow!("--dist is required for run --engine ssa"))?;
  let resource_limits = resource_limits_from_args(args);

  let mut fx = load_fxcore_from_dist(dist, &resource_limits)?;
  let mut patch_results: Option<Vec<patch::PatchOpResult>> = None;
  let mut patched = false;
  if let Some(patch_path) = args.patch.as_ref() {
    let patch: patch::FxCorePatch =
      read_json_file_with_limits(patch_path, &resource_limits, "patch json")?;
    let (patched_fx, results) = patch::apply_patch_with_results(fx, patch)?;
    fx = patched_fx;
    patch_results = Some(results);
    patched = true;
    eprintln!("info: applied patch {}", patch_path.display());
  }
  verify_resource_limits(&fx, &resource_limits)?;
  reject_scopes_for_ssa(&fx)?;

  if let Some(used_spec) = capability_check::load_used_spec_from_dist(dist)? {
    let required_caps = capability_check::extract_required_capabilities(&used_spec)?;
    let checker = pnix_runtime_api::capability_example::LegacyEvalCapabilities;
    let result = capability_check::check_engine_capabilities(&checker, &required_caps, &[], &[])?;
    if !result.success {
      anyhow::bail!("{}", result.message);
    }
  }

  let ssa_module: CoreSsaModule = if patched {
    let diags = pnix_core::diagnostics::Diagnostics::default();
    pnix_core::passes::lowering::lower_to_ssa(&fx, &diags)
      .map_err(|err| anyhow::anyhow!("ssa lowering failed: {}", err))?
  } else {
    let ssa_path = dist.join("ir").join("ssa.canon.json");
    read_json_file_with_limits(&ssa_path, &resource_limits, "ssa json")?
  };

  let block = match ssa_module.blocks.as_slice() {
    [block] => convert_ssa_block(block)?,
    _ => anyhow::bail!(
      "run --engine ssa expects exactly one SSA block (got {})",
      ssa_module.blocks.len()
    ),
  };

  let required_inputs = collect_required_inputs(&fx);
  if !required_inputs.is_empty() && args.inputs.is_empty() {
    anyhow::bail!(
      "run --engine ssa requires --inputs/--inputs-json when module has inputs: {:?}",
      required_inputs
    );
  }
  if !args.inputs.is_empty() {
    validate_inputs_keys(
      "ssa",
      &args.inputs,
      required_inputs.iter().cloned().collect(),
    )?;
  }

  let config = EvalConfig {
    deterministic: args.deterministic,
    seed: args.seed,
    now_ms: args.now_ms,
    clock_step_ms: args.clock_step_ms,
    float_pattern_tolerance: None,
    verbose: false,
  };
  let mut ctx = SSARunContext::from_config(&config);
  for input_name in &required_inputs {
    let value = args.inputs.get(input_name).ok_or_else(|| {
      anyhow::anyhow!(
        "missing required input '{}' for ssa (provide via --input/--inputs-json)",
        input_name
      )
    })?;
    let number = ssa_value_to_f64(input_name, value)?;
    ctx = ctx.with_var(format!("input_{}", input_name), number);
  }

  let value =
    run_ssa_value(&block, &ctx).map_err(|err| anyhow::anyhow!("ssa run failed: {}", err))?;
  let value = ssa_output_json(&fx, &value)?;
  let mut out = serde_json::json!({
    "ok": true,
    "engine": "ssa",
    "value": value
  });
  if let Some(results) = patch_results {
    out["patch_results"] = serde_json::to_value(results)?;
  }
  Ok(out)
}

fn extract_engine_value(raw: serde_json::Value) -> serde_json::Value {
  let value = match raw {
    serde_json::Value::Object(mut map) => {
      if let Some(value) = map.remove("value") {
        value
      } else if let Some(value) = map.remove("outputs") {
        value
      } else {
        serde_json::Value::Object(map)
      }
    }
    other => other,
  };
  normalize_json_numbers(value)
}

fn normalize_json_numbers(value: serde_json::Value) -> serde_json::Value {
  match value {
    serde_json::Value::Number(n) => {
      if n.is_f64() {
        if let Some(f) = n.as_f64() {
          if f.is_finite() && f.fract() == 0.0 && f >= (i64::MIN as f64) && f <= (i64::MAX as f64) {
            return serde_json::Value::Number(serde_json::Number::from(f as i64));
          }
        }
      }
      serde_json::Value::Number(n)
    }
    serde_json::Value::Array(items) => serde_json::Value::Array(
      items
        .into_iter()
        .map(normalize_json_numbers)
        .collect::<Vec<_>>(),
    ),
    serde_json::Value::Object(map) => {
      let normalized = map
        .into_iter()
        .map(|(k, v)| (k, normalize_json_numbers(v)))
        .collect();
      serde_json::Value::Object(normalized)
    }
    other => other,
  }
}

fn legacy_eval_value_with_inputs(expr_path: &str, args: &Args) -> Result<serde_json::Value> {
  let source = std::fs::read_to_string(expr_path)
    .map_err(|e| anyhow::anyhow!("failed to read legacy expr {}: {}", expr_path, e))?;
  let module = LegacyModule::from_source(source).with_path(expr_path.to_string());
  let mut engine = LegacyEvalEngine::new();
  if !args.inputs.is_empty() {
    let patches: Vec<serde_json::Value> = args
      .inputs
      .iter()
      .map(|(name, value)| {
        serde_json::json!({
          "op": "set_input",
          "name": name,
          "value": value,
        })
      })
      .collect();
    let patch = eval_patch::EvalPatchFile {
      version: 1,
      patch_id: None,
      idempotency_key: None,
      committer: None,
      patches,
    };
    let _ = eval_patch::apply_patches(&mut engine, patch)?;
  }
  let config = EvalConfig {
    deterministic: args.deterministic,
    seed: args.seed,
    now_ms: args.now_ms,
    clock_step_ms: args.clock_step_ms,
    float_pattern_tolerance: None,
    verbose: false,
  };
  let result = engine
    .eval(&module, &config)
    .context("legacy eval failed")?;
  Ok(result.value.as_json().clone())
}

fn llvm_value_from_fxcore(args: &Args, fx: &model::FxCoreModule) -> Result<serde_json::Value> {
  verify_resource_limits(fx, &resource_limits_from_args(args))?;
  if !args.inputs.is_empty() {
    let allowed: BTreeSet<String> = fx.inputs.iter().map(|i| i.name.clone()).collect();
    validate_inputs_keys("llvm", &args.inputs, allowed)?;
  }

  let module_name = if !fx.name.is_empty() {
    fx.name.clone()
  } else {
    "inline".to_string()
  };
  let ir_json = serde_json::to_vec(fx)
    .map_err(|err| anyhow::anyhow!("failed to serialize FxCore module: {}", err))?;

  let mut engine = JitEngine::new();
  let module = engine
    .compile(&module_name, &ir_json)
    .context("llvm compile failed")?;
  let config = EvalConfig {
    deterministic: args.deterministic,
    seed: None,
    now_ms: None,
    clock_step_ms: None,
    float_pattern_tolerance: None,
    verbose: false,
  };
  let inputs_bytes = if args.inputs.is_empty() {
    Vec::new()
  } else {
    let mut obj = serde_json::Map::new();
    for (key, value) in &args.inputs {
      obj.insert(key.clone(), value.clone());
    }
    serde_json::to_vec(&serde_json::Value::Object(obj))?
  };
  let result = if inputs_bytes.is_empty() {
    engine
      .execute(&module, &config)
      .context("llvm execute failed")?
  } else {
    engine
      .execute_with_inputs(&module, &config, &inputs_bytes)
      .context("llvm execute failed")?
  };
  let value: serde_json::Value = serde_json::from_slice(&result.value.data)
    .map_err(|err| anyhow::anyhow!("llvm output is not valid JSON: {}", err))?;
  Ok(value)
}

fn convert_ssa_block(block: &pnix_core::ssa::SsaBlock) -> Result<LegacySsaBlock> {
  let mut ops = Vec::with_capacity(block.ops.len());
  for (value, op) in &block.ops {
    let reg = pnix_runtime_legacy::ssa::SSAValue(value.0);
    let op = match op {
      pnix_core::ssa::SSAOp::ConstInt(v) => LegacySsaOp::ConstInt(*v),
      pnix_core::ssa::SSAOp::ConstFloat(v) => LegacySsaOp::ConstFloat(*v),
      pnix_core::ssa::SSAOp::ConstBool(v) => LegacySsaOp::ConstBool(*v),
      pnix_core::ssa::SSAOp::ConstString(v) => LegacySsaOp::ConstString(v.clone()),
      pnix_core::ssa::SSAOp::LoadTime => LegacySsaOp::LoadTime,
      pnix_core::ssa::SSAOp::LoadDeltaTime => LegacySsaOp::LoadDeltaTime,
      pnix_core::ssa::SSAOp::LoadSignal(id) => LegacySsaOp::LoadSignal(id.0),
      pnix_core::ssa::SSAOp::LoadVar(name) => LegacySsaOp::LoadVar(name.clone()),
      pnix_core::ssa::SSAOp::LoadAttr(base, attr) => {
        LegacySsaOp::LoadAttr(pnix_runtime_legacy::ssa::SSAValue(base.0), attr.clone())
      }
      pnix_core::ssa::SSAOp::Lambda {
        param,
        body,
        captures,
        self_name,
      } => {
        let legacy_body = convert_ssa_block(body.as_ref())?;
        let legacy_captures = captures
          .iter()
          .map(|(name, value)| (name.clone(), legacy_ssa_value(value)))
          .collect();
        LegacySsaOp::Lambda {
          param: param.clone(),
          body: Box::new(legacy_body),
          captures: legacy_captures,
          self_name: self_name.clone(),
        }
      }
      pnix_core::ssa::SSAOp::Call { func, args } => LegacySsaOp::Call {
        func: legacy_ssa_value(func),
        args: args.iter().map(legacy_ssa_value).collect(),
      },
      pnix_core::ssa::SSAOp::TailCall { func, args } => LegacySsaOp::TailCall {
        func: legacy_ssa_value(func),
        args: args.iter().map(legacy_ssa_value).collect(),
      },
      pnix_core::ssa::SSAOp::Add(a, b) => {
        LegacySsaOp::Add(legacy_ssa_value(a), legacy_ssa_value(b))
      }
      pnix_core::ssa::SSAOp::Sub(a, b) => {
        LegacySsaOp::Sub(legacy_ssa_value(a), legacy_ssa_value(b))
      }
      pnix_core::ssa::SSAOp::Mul(a, b) => {
        LegacySsaOp::Mul(legacy_ssa_value(a), legacy_ssa_value(b))
      }
      pnix_core::ssa::SSAOp::Div(a, b) => {
        LegacySsaOp::Div(legacy_ssa_value(a), legacy_ssa_value(b))
      }
      pnix_core::ssa::SSAOp::Mod(a, b) => {
        LegacySsaOp::Mod(legacy_ssa_value(a), legacy_ssa_value(b))
      }
      pnix_core::ssa::SSAOp::Pow(a, b) => {
        LegacySsaOp::Pow(legacy_ssa_value(a), legacy_ssa_value(b))
      }
      pnix_core::ssa::SSAOp::Neg(a) => LegacySsaOp::Neg(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Floor(a) => LegacySsaOp::Floor(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Ceil(a) => LegacySsaOp::Ceil(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Abs(a) => LegacySsaOp::Abs(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Sqrt(a) => LegacySsaOp::Sqrt(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Sin(a) => LegacySsaOp::Sin(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Cos(a) => LegacySsaOp::Cos(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Tan(a) => LegacySsaOp::Tan(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Exp(a) => LegacySsaOp::Exp(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Ln(a) => LegacySsaOp::Ln(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Lt(a, b) => LegacySsaOp::Lt(legacy_ssa_value(a), legacy_ssa_value(b)),
      pnix_core::ssa::SSAOp::Gt(a, b) => LegacySsaOp::Gt(legacy_ssa_value(a), legacy_ssa_value(b)),
      pnix_core::ssa::SSAOp::Le(a, b) => LegacySsaOp::Le(legacy_ssa_value(a), legacy_ssa_value(b)),
      pnix_core::ssa::SSAOp::Ge(a, b) => LegacySsaOp::Ge(legacy_ssa_value(a), legacy_ssa_value(b)),
      pnix_core::ssa::SSAOp::Eq(a, b) => LegacySsaOp::Eq(legacy_ssa_value(a), legacy_ssa_value(b)),
      pnix_core::ssa::SSAOp::Ne(a, b) => LegacySsaOp::Ne(legacy_ssa_value(a), legacy_ssa_value(b)),
      pnix_core::ssa::SSAOp::And(a, b) => {
        LegacySsaOp::And(legacy_ssa_value(a), legacy_ssa_value(b))
      }
      pnix_core::ssa::SSAOp::Or(a, b) => LegacySsaOp::Or(legacy_ssa_value(a), legacy_ssa_value(b)),
      pnix_core::ssa::SSAOp::Not(a) => LegacySsaOp::Not(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Select(c, t, e) => LegacySsaOp::Select(
        legacy_ssa_value(c),
        legacy_ssa_value(t),
        legacy_ssa_value(e),
      ),
      pnix_core::ssa::SSAOp::Alias(a) => LegacySsaOp::Alias(legacy_ssa_value(a)),
      pnix_core::ssa::SSAOp::Derived(_, _) => {
        anyhow::bail!("ssa eval does not support Derived ops yet")
      }
      pnix_core::ssa::SSAOp::CallExtern { name, .. } => {
        anyhow::bail!("ssa eval does not support CallExtern op: {}", name)
      }
      pnix_core::ssa::SSAOp::ListConstruct(items) => {
        LegacySsaOp::ListConstruct(items.iter().map(legacy_ssa_value).collect())
      }
      pnix_core::ssa::SSAOp::AttrSetConstruct(pairs) => LegacySsaOp::AttrSetConstruct(
        pairs
          .iter()
          .map(|(key, value)| (key.clone(), legacy_ssa_value(value)))
          .collect(),
      ),
      pnix_core::ssa::SSAOp::Throw(msg) => LegacySsaOp::Throw(msg.clone()),
    };
    ops.push((reg, op));
  }

  Ok(LegacySsaBlock {
    ops,
    ret: legacy_ssa_value(&block.ret),
  })
}

fn legacy_ssa_value(value: &pnix_core::ssa::SSAValue) -> pnix_runtime_legacy::ssa::SSAValue {
  pnix_runtime_legacy::ssa::SSAValue(value.0)
}

fn ssa_value_to_f64(name: &str, value: &serde_json::Value) -> Result<f64> {
  if let Some(num) = value.as_f64() {
    return Ok(num);
  }
  if let Some(num) = value.as_i64() {
    return Ok(num as f64);
  }
  if let Some(b) = value.as_bool() {
    return Ok(if b { 1.0 } else { 0.0 });
  }
  anyhow::bail!("ssa input '{}' must be a number or bool", name)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SsaOutputKind {
  Bool,
  Int,
  Float,
  String,
}

fn ssa_output_kind(fx: &CoreFxCoreModule) -> Option<SsaOutputKind> {
  let output_node = fx
    .nodes
    .iter()
    .find(|node| node.name == "result")
    .or_else(|| fx.nodes.last())?;
  let morphism = fx
    .morphisms
    .iter()
    .find(|morphism| morphism.name == output_node.uses)?;

  let port_ty = morphism.outputs.first().map(|port| port.ty.as_str());
  let output_ty = if let Some(ty) = port_ty {
    ty
  } else {
    morphism.output.as_str()
  };
  if let Some(kind) = ssa_output_kind_from_type(output_ty) {
    return Some(kind);
  }

  let op = morphism
    .name
    .strip_prefix("builtins.")
    .unwrap_or(morphism.name.as_str());
  if matches!(op, "eq" | "ne" | "lt" | "le" | "gt" | "ge") {
    return Some(SsaOutputKind::Bool);
  }

  None
}

fn ssa_output_kind_from_type(raw: &str) -> Option<SsaOutputKind> {
  let ty = raw.trim().to_ascii_lowercase();
  match ty.as_str() {
    "bool" | "boolean" => Some(SsaOutputKind::Bool),
    "int" | "i32" | "i64" => Some(SsaOutputKind::Int),
    "float" | "f64" | "real" | "num" => Some(SsaOutputKind::Float),
    "string" | "str" | "text" => Some(SsaOutputKind::String),
    _ => None,
  }
}

fn ssa_output_json(fx: &CoreFxCoreModule, value: &SsaValue) -> Result<serde_json::Value> {
  match ssa_output_kind(fx) {
    Some(SsaOutputKind::Bool) => Ok(serde_json::Value::Bool(ssa_value_to_bool(value)?)),
    Some(SsaOutputKind::Int) => ssa_value_to_int_json(value),
    Some(SsaOutputKind::Float) => ssa_value_to_float_json(value),
    Some(SsaOutputKind::String) => ssa_value_to_string_json(value),
    None => ssa_value_to_json(value),
  }
}

fn ssa_value_to_bool(value: &SsaValue) -> Result<bool> {
  match value {
    SsaValue::Bool(v) => Ok(*v),
    SsaValue::Num(v) => Ok(*v != 0.0),
    other => bail!("ssa output expected bool, got {}", other.kind()),
  }
}

fn ssa_value_to_float_json(value: &SsaValue) -> Result<serde_json::Value> {
  let number = ssa_output_value_to_f64(value)?;
  ssa_number_to_json(number)
}

fn ssa_value_to_int_json(value: &SsaValue) -> Result<serde_json::Value> {
  let number = ssa_output_value_to_f64(value)?;
  if number.fract() == 0.0 && number >= (i64::MIN as f64) && number <= (i64::MAX as f64) {
    Ok(serde_json::Value::Number(serde_json::Number::from(
      number as i64,
    )))
  } else {
    ssa_number_to_json(number)
  }
}

fn ssa_value_to_string_json(value: &SsaValue) -> Result<serde_json::Value> {
  match value {
    SsaValue::String(v) => Ok(serde_json::Value::String(v.clone())),
    other => bail!("ssa output expected string, got {}", other.kind()),
  }
}

fn ssa_number_to_json(value: f64) -> Result<serde_json::Value> {
  if !value.is_finite() {
    bail!("ssa output is not finite");
  }
  serde_json::Number::from_f64(value)
    .map(serde_json::Value::Number)
    .ok_or_else(|| anyhow::anyhow!("ssa output is not finite"))
}

fn ssa_output_value_to_f64(value: &SsaValue) -> Result<f64> {
  match value {
    SsaValue::Num(v) => Ok(*v),
    SsaValue::Bool(v) => Ok(if *v { 1.0 } else { 0.0 }),
    other => bail!("ssa output expected number, got {}", other.kind()),
  }
}

fn ssa_value_to_json(value: &SsaValue) -> Result<serde_json::Value> {
  match value {
    SsaValue::Num(v) => ssa_number_to_json(*v),
    SsaValue::Bool(v) => Ok(serde_json::Value::Bool(*v)),
    SsaValue::String(v) => Ok(serde_json::Value::String(v.clone())),
    SsaValue::List(items) => Ok(serde_json::Value::Array(
      items
        .iter()
        .map(ssa_value_to_json)
        .collect::<Result<Vec<_>>>()?,
    )),
    SsaValue::AttrSet(map) => {
      let mut obj = serde_json::Map::new();
      for (key, value) in map {
        obj.insert(key.clone(), ssa_value_to_json(value)?);
      }
      Ok(serde_json::Value::Object(obj))
    }
    SsaValue::Closure(_) => Ok(serde_json::Value::String("<closure>".to_string())),
  }
}

#[derive(Debug, Clone)]
struct IrEvalResultSelector {
  node: String,
  port: Option<String>,
  /// The env variable name (node or node.port).
  var: String,
  /// Canonical selector string (node or node.port; `.out` is normalized to `node`).
  canonical: String,
}

fn resolve_ir_eval_result_selector(
  spec: &str,
  fx: &CoreFxCoreModule,
) -> Result<IrEvalResultSelector> {
  let spec = spec.trim();
  if spec.is_empty() {
    anyhow::bail!("--result requires a non-empty value");
  }

  // Prefer parsing `node.port` when `node` exists (disambiguates node names that may contain '.').
  if let Some(dot) = spec.rfind('.') {
    let node = spec[..dot].trim();
    let port = spec[dot + 1..].trim();
    if !node.is_empty() && !port.is_empty() && fx.nodes.iter().any(|n| n.name == node) {
      validate_ir_eval_result_port(node, port, fx)?;
      if port == "out" {
        return Ok(IrEvalResultSelector {
          node: node.to_string(),
          port: None,
          var: node.to_string(),
          canonical: node.to_string(),
        });
      }
      return Ok(IrEvalResultSelector {
        node: node.to_string(),
        port: Some(port.to_string()),
        var: format!("{}.{}", node, port),
        canonical: format!("{}.{}", node, port),
      });
    }
  }

  if !fx.nodes.iter().any(|n| n.name == spec) {
    let mut nodes: Vec<String> = fx.nodes.iter().map(|n| n.name.clone()).collect();
    nodes.sort();
    anyhow::bail!(
      "unknown --result node '{}'\n\
             available nodes: {:?}",
      spec,
      nodes
    );
  }

  Ok(IrEvalResultSelector {
    node: spec.to_string(),
    port: None,
    var: spec.to_string(),
    canonical: spec.to_string(),
  })
}

fn validate_ir_eval_result_port(node: &str, port: &str, fx: &CoreFxCoreModule) -> Result<()> {
  if port == "out" {
    return Ok(());
  }

  let uses = fx
    .nodes
    .iter()
    .find(|n| n.name == node)
    .map(|n| n.uses.as_str())
    .unwrap_or("");
  if uses.is_empty() {
    return Ok(());
  }

  let morphism = match fx.morphisms.iter().find(|m| m.name == uses) {
    Some(m) => m,
    None => return Ok(()),
  };

  let mut ports: Vec<String> = morphism.outputs.iter().map(|p| p.name.clone()).collect();
  ports.sort();
  if ports.contains(&port.to_string()) {
    return Ok(());
  }

  anyhow::bail!(
    "unknown output port '{}' for node '{}' (uses '{}')\n\
         known output ports: {:?}",
    port,
    node,
    uses,
    ports
  )
}

fn select_ir_eval_result_value(
  spec: &str,
  selector: &IrEvalResultSelector,
  ctx: &IrEvalContext,
  fx: &CoreFxCoreModule,
) -> Result<serde_json::Value> {
  if let Some(v) = ctx.get(&selector.var) {
    return Ok(v.clone());
  }

  if let Some(port) = selector.port.as_deref() {
    if let Some(serde_json::Value::Object(obj)) = ctx.get(&selector.node) {
      if let Some(v) = obj.get(port) {
        return Ok(v.clone());
      }
    }
  }

  let sinks = collect_sink_nodes(fx);
  anyhow::bail!(
    "requested --result '{}' was not produced by ir-eval (missing env var '{}')\n\
         available sink nodes: {:?}",
    spec,
    selector.var,
    sinks
  )
}

fn collect_sink_nodes(fx: &CoreFxCoreModule) -> Vec<String> {
  let mut normal_nodes = BTreeSet::new();
  for node in &fx.nodes {
    if node.kind == pnix_core::core::NodeKind::Normal {
      normal_nodes.insert(node.name.clone());
    }
  }

  let mut has_outgoing = BTreeSet::new();
  for edge in &fx.edges {
    if edge.from != "input" {
      has_outgoing.insert(edge.from.clone());
    }
  }

  normal_nodes
    .into_iter()
    .filter(|name| !has_outgoing.contains(name))
    .collect()
}

fn collect_required_inputs(fx: &CoreFxCoreModule) -> Vec<String> {
  let mut set = BTreeSet::new();
  for input in &fx.inputs {
    set.insert(input.name.clone());
  }
  for edge in &fx.edges {
    if let Some(name) = &edge.from_input {
      set.insert(name.clone());
    }
  }
  set.into_iter().collect()
}

fn collect_onfail_allowlist(fx: &CoreFxCoreModule) -> BTreeSet<String> {
  let mut set = BTreeSet::new();
  for edge in &fx.edges {
    if let Some(pnix_core::core::EdgeCond::OnFail(node)) = &edge.cond {
      set.insert(node.clone());
    }
  }
  set
}

fn reject_scopes_for_ir_eval(fx: &CoreFxCoreModule) -> Result<()> {
  if !fx.scopes.is_empty() {
    let mut names: Vec<String> = fx.scopes.iter().map(|s| s.name.clone()).collect();
    names.sort();
    anyhow::bail!(
      "run --engine ir-eval does not support FxCore scopes yet (found scopes: {:?})\n\
             use --mode run --engine graph for full Stage-4 scope policies",
      names
    );
  }

  let mut non_global_nodes: Vec<String> = fx
    .nodes
    .iter()
    .filter(|n| n.scope != "global")
    .map(|n| n.name.clone())
    .collect();
  non_global_nodes.sort();
  if !non_global_nodes.is_empty() {
    anyhow::bail!(
      "run --engine ir-eval does not support node scopes yet (non-global nodes: {:?})\n\
             use --mode run --engine graph for full Stage-4 scope policies",
      non_global_nodes
    );
  }

  Ok(())
}

fn reject_scopes_for_ssa(fx: &CoreFxCoreModule) -> Result<()> {
  if !fx.scopes.is_empty() {
    let mut names: Vec<String> = fx.scopes.iter().map(|s| s.name.clone()).collect();
    names.sort();
    anyhow::bail!(
      "run --engine ssa does not support FxCore scopes yet (found scopes: {:?})\n\
             use --mode run --engine graph for full Stage-4 scope policies",
      names
    );
  }

  let mut non_global_nodes: Vec<String> = fx
    .nodes
    .iter()
    .filter(|n| n.scope != "global")
    .map(|n| n.name.clone())
    .collect();
  non_global_nodes.sort();
  if !non_global_nodes.is_empty() {
    anyhow::bail!(
      "run --engine ssa does not support node scopes yet (non-global nodes: {:?})\n\
             use --mode run --engine graph for full Stage-4 scope policies",
      non_global_nodes
    );
  }

  Ok(())
}

fn is_codegen_input_binding(instr: &LegacyInstr) -> bool {
  if !instr.var.starts_with("input_") {
    return false;
  }
  matches!(
      &instr.op,
      LegacyOp::Apply { func, .. } if func == "inputs"
  )
}

fn run_compile(args: &Args) -> Result<()> {
  run_compile_impl(args, true)
}

fn run_compile_quiet(args: &Args) -> Result<()> {
  run_compile_impl(args, false)
}

fn run_compile_impl(args: &Args, print_stdout: bool) -> Result<()> {
  let dist = args
    .dist
    .as_ref()
    .ok_or_else(|| anyhow::anyhow!("--dist is required for mode compile"))?;
  let (source, path) = read_source(args)?;
  if let Some(entry_path) = path.as_ref() {
    sync_project_lock(Path::new(entry_path), args.dry_run)?;
  }
  let source_name = path.clone().unwrap_or_else(|| "<expr>".to_string());

  let src = CoreSourceUnit {
    name: source_name.clone(),
    text: source.clone(),
  };

  let opts = CoreCompileOptions {
    deterministic: args.deterministic,
    resource_limits: resource_limits_from_args(args),
    ..Default::default()
  };

  let out = if let Some(path) = path {
    let entry_path = Path::new(&path);
    let ast = module_loader::load_pnix_module_from_source(entry_path, &source)
      .with_context(|| format!("failed to load module graph from {}", path))?;
    compile_pnix_module_ast(ast, &opts).context("compile failed")?
  } else {
    compile_pnix_module(&src, &opts).context("compile failed")?
  };

  let pnix_core::codegen::Artifacts {
    fxcore_json,
    ssa_json,
    build_ir_json,
    replay_hash,
    spec_canon_json,
    used_spec_canon_json,
  } = out.artifacts;

  let report = emit_fs::EmitReport {
    ok: out.report.ok,
    closure: emit_fs::EmitClosure {
      s2_reference_closure: out.report.closure.s2_reference_closure,
      s3_contracts: out.report.closure.s3_contracts,
      s4_dependency_closure: out.report.closure.s4_dependency_closure,
      s5_deterministic_artifacts: out.report.closure.s5_deterministic_artifacts,
    },
    notes: out.report.notes.clone(),
    diagnostics: out
      .diags
      .sorted()
      .into_iter()
      .map(|d| emit_fs::EmitDiagnostic {
        message: d.message,
        span: d.span.map(|s| emit_fs::EmitSpan {
          start: s.start,
          end: s.end,
          file: s.file,
        }),
      })
      .collect(),
  };

  if args.dry_run {
    let line = format!(
      r#"{{"ok":true,"mode":"compile","dry_run":true,"replay_hash":"{}","dist":"{}"}}"#,
      replay_hash,
      dist.display()
    );
    if print_stdout {
      println!("{}", line);
    }
    return Ok(());
  }

  let manifest = emit_fs::EmitManifest {
    pnix_core_version: format!("pnix-core@{}", pnix_core::PNIX_CORE_VERSION),
    source: emit_fs::EmitSource {
      name: source_name,
      bytes: source.len(),
    },
    compile_options: emit_fs::EmitCompileOptions {
      target_os: os_label(&opts.target_os).to_string(),
      target_arch: arch_label(&opts.target_arch).to_string(),
      deterministic: opts.deterministic,
    },
  };

  let input = emit_fs::EmitInput {
    artifacts: emit_fs::EmitArtifacts {
      fxcore_json,
      ssa_json,
      build_ir_json,
      spec_canon_json,
      used_spec_canon_json,
      replay_hash: replay_hash.clone(),
    },
    manifest,
    report,
  };

  emit_fs::emit_to_dir(&input, dist)?;

  // --binary 옵션: compile 완료 후 바로 바이너리 생성
  if args.binary {
    eprintln!("info: --binary flag detected, generating binary...");
    let mut emit_args = args.clone();
    if emit_args.emit_target.is_none() {
      emit_args.emit_target = Some("aot".to_string());
    }
    if let Some(ref target) = emit_args.emit_target {
      let target = target.trim().to_ascii_lowercase();
      if !(target == "aot" || target.starts_with("aot:")) {
        anyhow::bail!(
          "--binary requires an AOT emit target (use --emit-target aot:<target> or --target <aot-target>)"
        );
      }
    }
    emit_args.emit_out = Some(dist.join("emit"));
    emit_args.emit = true;
    // dist가 이미 생성되었으므로 run_emit 호출 가능
    let _ = emit_summary(&emit_args)?;
  }

  let line = format!(
    r#"{{"ok":true,"mode":"compile","replay_hash":"{}","dist":"{}"}}"#,
    replay_hash,
    dist.display()
  );
  if print_stdout {
    println!("{}", line);
  }
  Ok(())
}

fn warn_legacy_compat_path(path: &str, preferred: &str) {
  eprintln!(
    "warn: '{}' is a legacy compatibility path. Prefer '{}' for new workflows.",
    path, preferred
  );
}

async fn run_interpret(args: &Args) -> Result<()> {
  let engine = args.engine.as_deref().unwrap_or("legacy-eval");

  // Y12a: REPL 모드 (--source/--expr이 없으면 REPL 진입)
  if args.source.is_none() && args.expr.is_none() {
    if matches!(engine, "legacy-eval" | "eval") {
      let config = EvalConfig {
        deterministic: args.deterministic,
        seed: args.seed,
        now_ms: args.now_ms,
        clock_step_ms: args.clock_step_ms,
        float_pattern_tolerance: None,
        verbose: false,
      };
      let history_file = if args.live {
        let live_root = args.live_dir.clone().unwrap_or_else(default_live_dir);
        eprintln!(
          "info: interpret --live enables REPL state persistence (dir={})",
          live_root.display()
        );
        repl::resolve_history_file(Some(&live_root))
      } else {
        repl::resolve_history_file(args.dist.as_ref())
      };
      return run_repl(config, history_file);
    }

    anyhow::bail!(
      "--source or --expr is required for --mode interpret --engine {}",
      engine
    );
  }

  // 기존 단일 표현식 평가 모드
  if matches!(args.engine.as_deref(), Some("legacy-eval" | "eval")) {
    warn_legacy_compat_path(
      "--mode interpret --engine legacy-eval",
      "--mode run --engine ir-eval",
    );
  }
  if matches!(engine, "legacy-frp" | "frp") {
    warn_legacy_compat_path(
      "--mode interpret --engine legacy-frp",
      "--mode run --engine graph",
    );
  }
  match engine {
    "graph" => run_run(args).await,
    "legacy-eval" | "eval" => run_legacy_eval(args),
    "ct" => run_ct(args),
    "legacy-frp" | "frp" => run_legacy_frp(args),
    "ui" => run_ui_interpret(args),
    other => anyhow::bail!(
      "unknown interpret engine '{}'\n\
             supported: graph, legacy-eval, ct, legacy-frp, ui\n\
             (advanced compat modes: legacy-eval/ct/legacy-frp can also be invoked via --mode legacy-eval/--ct/--legacy-frp)",
      other
    ),
  }
}

fn run_legacy_eval(args: &Args) -> Result<()> {
  // W06b: Capability 체크 (dist에서 UsedSpec 읽기)
  if let Some(dist) = args.dist.as_ref() {
    if let Some(used_spec) = capability_check::load_used_spec_from_dist(dist)? {
      let required_caps = capability_check::extract_required_capabilities(&used_spec)?;
      // LegacyEvalCapabilities 체크
      let checker = pnix_runtime_api::capability_example::LegacyEvalCapabilities;
      let result = capability_check::check_engine_capabilities(&checker, &required_caps, &[], &[])?;

      if !result.success {
        anyhow::bail!("{}", result.message);
      }
    }
  }

  let output = legacy_eval_output_value(args)?;
  println!("{}", serde_json::to_string_pretty(&output)?);
  Ok(())
}

fn run_legacy_frp(args: &Args) -> Result<()> {
  let (source, path) = read_source(args)?;
  let mut graph = load_legacy_frp_graph(&source, path.as_deref())?;
  let patch_results = if let Some(patch_path) = args.patch.as_ref() {
    let patch_txt = std::fs::read_to_string(patch_path)
      .map_err(|e| anyhow::anyhow!("failed to read patch {}: {}", patch_path.display(), e))?;
    let patch = frp_patch::FrpPatchFile::from_json_str(&patch_txt)
      .map_err(|e| anyhow::anyhow!("invalid frp patch {}: {}", patch_path.display(), e))?;
    Some(frp_patch::apply_patches(&mut graph, patch)?)
  } else {
    None
  };
  graph.export_all();

  validate_legacy_frp_inputs(&graph, &args.inputs)?;

  let mut input_values = HashMap::new();
  for (name, value) in &args.inputs {
    let number = value
      .as_f64()
      .ok_or_else(|| anyhow::anyhow!("legacy-frp input '{}' must be a number", name))?;
    input_values.insert(name.clone(), number);
  }
  if !input_values.is_empty() {
    graph.runtime.inject_external_inputs(&input_values);
  }

  let dt = match (args.clock_step_ms, args.frp_dt) {
    (Some(ms), _) => ms as f64 / 1000.0,
    (None, Some(dt)) => dt,
    (None, None) => {
      anyhow::bail!("--dt or --clock-step is required for --mode legacy-frp");
    }
  };

  let config = FrpConfig {
    tick_index: 0,
    seed: args.seed,
    now_ms: args.now_ms,
    clock_step_ms: args.clock_step_ms,
  };

  let mut engine = LegacyFrpEngine::new();
  let input = LegacyFrpInput::new(dt);
  let result = engine
    .tick(&graph, input, &config)
    .context("legacy frp tick failed")?;

  let values: Vec<serde_json::Value> = result
    .output
    .values_deterministic()
    .into_iter()
    .map(|(id, value)| serde_json::json!({ "id": id.0, "value": value }))
    .collect();

  let mut external_inputs: Vec<serde_json::Value> = graph
    .runtime
    .external_inputs()
    .iter()
    .map(|entry| {
      serde_json::json!({
          "name": entry.name,
          "id": entry.signal_id.0,
          "default": entry.default_value,
      })
    })
    .collect();
  external_inputs.sort_by(|a, b| {
    let name_a = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let name_b = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
    name_a.cmp(name_b)
  });

  let mut output = serde_json::json!({
      "ok": true,
      "values": values,
      "external_inputs": external_inputs,
  });
  if let Some(results) = patch_results {
    let patches: Vec<serde_json::Value> = results
      .into_iter()
      .map(|r| {
        serde_json::json!({
            "success": r.success,
            "signal_id": r.signal_id.map(|id| id.0),
            "error": r.error,
        })
      })
      .collect();
    if let Some(obj) = output.as_object_mut() {
      obj.insert(
        "patch_results".to_string(),
        serde_json::Value::Array(patches),
      );
    }
  }
  let output = serde_json::to_string_pretty(&output)?;
  println!("{}", output);
  Ok(())
}

fn load_legacy_frp_graph(source: &str, path: Option<&str>) -> Result<LegacyFrpGraph> {
  if let Some(path) = path {
    if is_x3d_xml_json_path(path) {
      let xml_json: serde_json::Value =
        serde_json::from_str(source).context("failed to parse X3D xml.json")?;
      let issues = validate_routes_from_xml_json(&xml_json);
      if !issues.is_empty() {
        eprintln!(
          "warn: x3d route validation reported {} issue(s)",
          issues.len()
        );
        for issue in &issues {
          if issue.to_node.is_empty() && issue.from_field == "USE" {
            eprintln!("warn: use {}: {}", issue.from_node, issue.message);
          } else {
            eprintln!(
              "warn: route {}.{} -> {}.{}: {}",
              issue.from_node, issue.from_field, issue.to_node, issue.to_field, issue.message
            );
          }
        }
      }
      let attr_issues = validate_attrs_from_xml_json(&xml_json);
      if !attr_issues.is_empty() {
        eprintln!(
          "warn: x3d attr validation reported {} issue(s)",
          attr_issues.len()
        );
        for issue in &attr_issues {
          eprintln!(
            "warn: attr {}.{}: {}",
            issue.element, issue.attr, issue.message
          );
        }
      }
      warn_x3d_schema_issues(&xml_json);
      let graph_json = frp_graph_json_from_xml_json(&xml_json);
      // LOW: ANSI 색상 비터미널에서 출력 수정 완료
      // 현재는 eprintln!만 사용하므로 색상 코드가 없어 문제 없음
      // 향후 ANSI 색상 코드 사용 시 isatty 체크를 통해 터미널인지 확인하여 사용
      return LegacyFrpGraph::from_json(&graph_json)
        .context("legacy frp graph load failed (x3d xml.json)");
    }
    if is_x3d_xml_path(path) {
      let issues = validate_routes_from_xml_str(source)
        .map_err(|err| anyhow::anyhow!("failed to parse X3D XML for validation: {}", err))?;
      if !issues.is_empty() {
        eprintln!(
          "warn: x3d route validation reported {} issue(s)",
          issues.len()
        );
        for issue in &issues {
          if issue.to_node.is_empty() && issue.from_field == "USE" {
            eprintln!("warn: use {}: {}", issue.from_node, issue.message);
          } else {
            eprintln!(
              "warn: route {}.{} -> {}.{}: {}",
              issue.from_node, issue.from_field, issue.to_node, issue.to_field, issue.message
            );
          }
        }
      }
      let attr_issues = validate_attrs_from_xml_str(source)
        .map_err(|err| anyhow::anyhow!("failed to parse X3D XML for attr validation: {}", err))?;
      if !attr_issues.is_empty() {
        eprintln!(
          "warn: x3d attr validation reported {} issue(s)",
          attr_issues.len()
        );
        for issue in &attr_issues {
          eprintln!(
            "warn: attr {}.{}: {}",
            issue.element, issue.attr, issue.message
          );
        }
      }
      match xml_x3d_core::xml_json_from_xml_str(source) {
        Ok(xml_json) => warn_x3d_schema_issues(&xml_json),
        Err(err) => eprintln!("warn: x3d schema parse failed: {}", err),
      }
      let graph_json = frp_graph_json_from_xml_str(source)
        .map_err(|err| anyhow::anyhow!("failed to parse X3D XML: {}", err))?;
      return LegacyFrpGraph::from_json(&graph_json)
        .context("legacy frp graph load failed (x3d xml)");
    }
  }
  LegacyFrpGraph::from_json_str(source).context("legacy frp graph load failed")
}

fn warn_x3d_schema_issues(xml_json: &serde_json::Value) {
  let mut normalized = xml_json.clone();
  xml_x3d_core::x3d_normalize_xml_json_with_defaults(&mut normalized);
  let normalized = match x3d_schema_normalize_xml_json(&normalized) {
    Ok(value) => value,
    Err(err) => {
      eprintln!("warn: x3d schema normalize failed: {}", err);
      return;
    }
  };
  let report = match x3d_schema_explain_xml_json(&normalized) {
    Ok(text) => text,
    Err(err) => {
      eprintln!("warn: x3d schema explain failed: {}", err);
      return;
    }
  };
  if report.is_empty() {
    return;
  }
  let lines: Vec<&str> = report.lines().collect();
  let total = lines.len();
  eprintln!("warn: x3d schema validation reported {} issue(s)", total);
  let limit = 50usize;
  for line in lines.iter().take(limit) {
    eprintln!("warn: {}", line);
  }
  if total > limit {
    eprintln!("warn: ... {} more", total - limit);
  }
}

fn is_x3d_xml_json_path(path: &str) -> bool {
  path.ends_with(".xml.json")
}

fn is_x3d_xml_path(path: &str) -> bool {
  path.ends_with(".x3d") || path.ends_with(".xml")
}

fn validate_graph_inputs(
  fx: &model::FxCoreModule,
  inputs: &HashMap<String, serde_json::Value>,
) -> Result<()> {
  if inputs.is_empty() {
    return Ok(());
  }

  let allowed: BTreeSet<String> = fx.inputs.iter().map(|i| i.name.clone()).collect();
  validate_inputs_keys("graph", inputs, allowed)
}

fn validate_legacy_frp_inputs(
  graph: &LegacyFrpGraph,
  inputs: &HashMap<String, serde_json::Value>,
) -> Result<()> {
  if inputs.is_empty() {
    return Ok(());
  }

  let allowed: BTreeSet<String> = graph
    .runtime
    .external_inputs()
    .iter()
    .map(|i| i.name.clone())
    .collect();
  validate_inputs_keys("legacy-frp", inputs, allowed)
}

fn validate_inputs_keys(
  context: &str,
  inputs: &HashMap<String, serde_json::Value>,
  allowed: BTreeSet<String>,
) -> Result<()> {
  let mut unknown: Vec<String> = inputs
    .keys()
    .filter(|key| !allowed.contains(*key))
    .cloned()
    .collect();
  if unknown.is_empty() {
    return Ok(());
  }
  unknown.sort();

  let allowed: Vec<String> = allowed.into_iter().collect();
  anyhow::bail!(
    "unknown --inputs keys for {}: {:?}\nallowed keys: {:?}",
    context,
    unknown,
    allowed
  );
}

fn resource_limits_from_args(args: &Args) -> ResourceLimits {
  ResourceLimits {
    max_nodes: args.max_nodes,
    max_edges: args.max_edges,
    max_input_bytes: args.max_input_bytes,
  }
}

fn run_test(args: &Args) -> Result<()> {
  // Y11c: 테스트 러너 구현
  // 소스 파일 또는 표현식에서 테스트 수집 및 실행

  let source_text = if let Some(expr) = &args.expr {
    // --expr 사용: 표현식을 직접 사용
    expr.clone()
  } else if let Some(source) = &args.source {
    // --source 사용: 파일에서 읽기
    let source_path = PathBuf::from(source);
    sync_project_lock(&source_path, args.dry_run)?;
    std::fs::read_to_string(&source_path)
      .with_context(|| format!("Failed to read source file: {}", source_path.display()))?
  } else {
    anyhow::bail!("--source or --expr is required for --mode test");
  };

  let source_path = args.source.as_ref().map(PathBuf::from);

  // 테스트 수집
  let tests = collect_tests_from_source(&source_text, source_path.as_deref())?;

  if tests.is_empty() {
    if let Some(ref path) = source_path {
      eprintln!("info: No tests found in {}", path.display());
    } else {
      eprintln!("info: No tests found in expression");
    }
    return Ok(());
  }

  eprintln!("info: Found {} test(s)", tests.len());

  // 테스트 실행 (필터 옵션 적용)
  let filter = args.test_filter.as_deref();
  let config = EvalConfig {
    deterministic: args.deterministic,
    seed: args.seed,
    now_ms: args.now_ms,
    clock_step_ms: args.clock_step_ms,
    float_pattern_tolerance: None,
    verbose: false,
  };
  let graph_config = GraphTestConfig {
    clojure_url: args.clojure_url.clone(),
    python_url: args.python_url.clone(),
    deno_url: args.deno_url.clone(),
    blenderpy_url: args.blenderpy_url.clone(),
    rpc_timeout_ms: args.rpc_timeout_ms,
    rpc_retry_attempts: args.rpc_retry_attempts,
    rpc_retry_backoff_ms: args.rpc_retry_backoff_ms,
    rpc_retry_seed: 0,
    use_batch_apply: args.use_batch,
    allow_non_atomic_effects: env_flag_enabled("PNIX_ALLOW_NON_ATOMIC_EFFECTS"),
    resource_limits: resource_limits_from_args(args),
  };
  let summary = run_tests(tests, filter, &config, Some(&graph_config));

  // 결과 출력
  print_test_summary(&summary);

  // 실패한 테스트가 있으면 에러 반환
  if summary.failed > 0 {
    anyhow::bail!("{} test(s) failed", summary.failed);
  }

  Ok(())
}

fn run_ct(args: &Args) -> Result<()> {
  let output = ct_output_value(args)?;
  println!("{}", serde_json::to_string_pretty(&output)?);
  Ok(())
}

fn legacy_eval_output_value(args: &Args) -> Result<serde_json::Value> {
  let (source, path) = read_source(args)?;
  let mut module = LegacyModule::from_source(source);
  if let Some(path) = path {
    module = module.with_path(path);
  }

  let mut engine = LegacyEvalEngine::new();
  let patch_results = if let Some(patch_path) = args.patch.as_ref() {
    let patch_txt = std::fs::read_to_string(patch_path)
      .map_err(|e| anyhow::anyhow!("failed to read patch {}: {}", patch_path.display(), e))?;
    let patch = eval_patch::EvalPatchFile::from_json_str(&patch_txt)
      .map_err(|e| anyhow::anyhow!("invalid eval patch {}: {}", patch_path.display(), e))?;
    Some(eval_patch::apply_patches(&mut engine, patch)?)
  } else {
    None
  };
  let config = EvalConfig {
    deterministic: args.deterministic,
    seed: args.seed,
    now_ms: args.now_ms,
    clock_step_ms: args.clock_step_ms,
    float_pattern_tolerance: None,
    verbose: false,
  };
  let result = engine
    .eval(&module, &config)
    .context("legacy eval failed")?;

  if let Some(results) = patch_results {
    let patches: Vec<serde_json::Value> = results
      .into_iter()
      .map(|r| {
        serde_json::json!({
            "success": r.success,
            "message": r.message,
        })
      })
      .collect();
    Ok(serde_json::json!({
        "ok": true,
        "value": result.value.as_json(),
        "patch_results": patches,
    }))
  } else {
    Ok(result.value.as_json().clone())
  }
}

fn ct_output_value(args: &Args) -> Result<serde_json::Value> {
  let (expr, _path) = read_source(args)?;
  let mut engine = CtRuntimeEngine::new();
  let config = CtConfig {
    strict: args.strict_ct,
    seed: args.seed,
    now_ms: args.now_ms,
    clock_step_ms: args.clock_step_ms,
  };
  let spec = pnix_runtime_api::CtSpec::new(expr);
  let result = engine.verify(&spec, &config).context("ct runtime failed")?;
  let mut notes = result.notes;
  notes.sort();
  let diagram = result.diagram.map(|mut d| {
    d.objects.sort_by(|a, b| {
      a.name
        .cmp(&b.name)
        .then_with(|| a.id.cmp(&b.id))
        .then_with(|| a.ct_type.cmp(&b.ct_type))
    });
    d.morphisms.sort_by(|a, b| {
      a.name
        .cmp(&b.name)
        .then_with(|| a.source.cmp(&b.source))
        .then_with(|| a.target.cmp(&b.target))
    });
    d
  });

  Ok(serde_json::json!({
      "ok": result.success,
      "success": result.success,
      "strict": args.strict_ct,
      "notes": notes,
      "diagram": diagram,
  }))
}

fn run_llvm(args: &Args) -> Result<()> {
  let (source, path) = read_source(args)?;
  let resource_limits = resource_limits_from_args(args);
  verify_input_size_with_label("llvm input", source.len(), &resource_limits)?;
  let mut fx: model::FxCoreModule = serde_json::from_str(&source)
    .map_err(|err| anyhow::anyhow!("llvm mode expects FxCore JSON input: {}", err))?;
  let mut patch_results: Option<Vec<patch::PatchOpResult>> = None;
  if let Some(patch_path) = args.patch.as_ref() {
    let patch: patch::FxCorePatch =
      read_json_file_with_limits(patch_path, &resource_limits, "patch json")?;
    let (patched, results) = patch::apply_patch_with_results(fx, patch)?;
    fx = patched;
    patch_results = Some(results);
    eprintln!("info: applied patch {}", patch_path.display());
  }
  verify_resource_limits(&fx, &resource_limits)?;
  if !args.inputs.is_empty() {
    let allowed: BTreeSet<String> = fx.inputs.iter().map(|i| i.name.clone()).collect();
    validate_inputs_keys("llvm", &args.inputs, allowed)?;
  }
  let module_name = if !fx.name.is_empty() {
    fx.name.clone()
  } else {
    path
      .as_deref()
      .and_then(|p| std::path::Path::new(p).file_stem())
      .and_then(|s| s.to_str())
      .unwrap_or("inline")
      .to_string()
  };
  let ir_json = serde_json::to_vec(&fx)
    .map_err(|err| anyhow::anyhow!("failed to serialize FxCore module: {}", err))?;

  let mut engine = JitEngine::new();
  let module = engine
    .compile(&module_name, &ir_json)
    .context("llvm compile failed")?;
  let config = EvalConfig {
    deterministic: args.deterministic,
    seed: args.seed,
    now_ms: args.now_ms,
    clock_step_ms: args.clock_step_ms,
    float_pattern_tolerance: None,
    verbose: false,
  };
  let inputs_bytes = if args.inputs.is_empty() {
    Vec::new()
  } else {
    let mut obj = serde_json::Map::new();
    for (key, value) in &args.inputs {
      obj.insert(key.clone(), value.clone());
    }
    serde_json::to_vec(&serde_json::Value::Object(obj))?
  };
  let result = if inputs_bytes.is_empty() {
    engine
      .execute(&module, &config)
      .context("llvm execute failed")?
  } else {
    engine
      .execute_with_inputs(&module, &config, &inputs_bytes)
      .context("llvm execute failed")?
  };

  let parsed_value = serde_json::from_slice(&result.value.data).ok();
  let mut output = serde_json::json!({
      "ok": true,
      "value_bytes": result.value.data,
  });
  if let Some(value) = parsed_value {
    output["value"] = value;
  }
  if let Some(results) = patch_results {
    output["patch_results"] = serde_json::to_value(results)?;
  }
  println!("{}", serde_json::to_string_pretty(&output)?);
  Ok(())
}

fn run_emit(args: &Args) -> Result<()> {
  let (summary, _manifest_path) = emit_summary(args)?;
  println!("{}", serde_json::to_string(&summary)?);
  Ok(())
}

fn emit_summary(args: &Args) -> Result<(serde_json::Value, PathBuf)> {
  let dist = args
    .dist
    .as_ref()
    .ok_or_else(|| anyhow::anyhow!("--dist is required for --emit"))?;
  let resource_limits = resource_limits_from_args(args);
  let emit_out = args.emit_out.clone().unwrap_or_else(|| dist.join("emit"));
  let emit_target_raw = args.emit_target.as_deref().unwrap_or("nix");
  let emit_target = parse_emit_target(emit_target_raw)?;
  let manifest_path = args
    .emit_manifest
    .clone()
    .unwrap_or_else(|| emit_out.join("emit.manifest.json"));

  let mut fx = load_fxcore_from_dist(dist, &resource_limits)?;
  let mut patch_results: Option<Vec<patch::PatchOpResult>> = None;
  if let Some(patch_path) = args.patch.as_ref() {
    let patch: patch::FxCorePatch =
      read_json_file_with_limits(patch_path, &resource_limits, "patch json")?;
    let (patched, results) = patch::apply_patch_with_results(fx, patch)?;
    fx = patched;
    patch_results = Some(results);
    eprintln!("info: applied patch {}", patch_path.display());
  }

  if !fx.meta.version.is_empty() && !SUPPORTED_FXCORE_VERSIONS.contains(&fx.meta.version.as_str()) {
    anyhow::bail!(
      "IR version mismatch: IR has '{}' but executor supports {:?}\n\
             Please regenerate IR with compatible pnix-core or upgrade executor",
      fx.meta.version,
      SUPPORTED_FXCORE_VERSIONS
    );
  }

  verify_resource_limits(&fx, &resource_limits)?;

  let mut summary = serde_json::Map::new();
  summary.insert("ok".to_string(), serde_json::Value::Bool(true));
  summary.insert(
    "engine".to_string(),
    serde_json::Value::String("emit".to_string()),
  );
  summary.insert(
    "emit_target".to_string(),
    serde_json::Value::String(emit_target_label(&emit_target)),
  );
  summary.insert(
    "emit_out".to_string(),
    serde_json::Value::String(path_to_slash(emit_out.as_path())),
  );
  summary.insert(
    "emit_manifest".to_string(),
    serde_json::Value::String(path_to_slash(&manifest_path)),
  );
  if let Some(results) = patch_results {
    summary.insert("patch_results".to_string(), serde_json::to_value(results)?);
  }

  match emit_target {
    EmitTarget::Legacy(target) => {
      let adapter_output =
        fxcore_to_legacy_ir(&fx, &AdapterConfig::default()).context("fxcore->legacy ir failed")?;
      if !adapter_output.report.warnings.is_empty() {
        eprintln!(
          "warn: adapter reported {} warning(s)",
          adapter_output.report.warnings.len()
        );
      }
      let legacy = emit_backend_legacy(&adapter_output.legacy, emit_out.as_path(), Some(target))?;
      summary.insert("legacy".to_string(), legacy);
      summary.insert(
        "adapter_report".to_string(),
        adapter_report_json(&adapter_output.report),
      );
    }
    EmitTarget::LegacyAll => {
      let adapter_output =
        fxcore_to_legacy_ir(&fx, &AdapterConfig::default()).context("fxcore->legacy ir failed")?;
      if !adapter_output.report.warnings.is_empty() {
        eprintln!(
          "warn: adapter reported {} warning(s)",
          adapter_output.report.warnings.len()
        );
      }
      let legacy = emit_backend_legacy(&adapter_output.legacy, emit_out.as_path(), None)?;
      summary.insert("legacy".to_string(), legacy);
      summary.insert(
        "adapter_report".to_string(),
        adapter_report_json(&adapter_output.report),
      );
    }
    EmitTarget::Aot(target) => {
      let aot = emit_aot(&fx, target, emit_out.as_path())?;
      summary.insert("aot".to_string(), aot);
    }
  }

  let summary = serde_json::Value::Object(summary);
  write_text(
    emit_out.as_path(),
    &manifest_path,
    &serde_json::to_string_pretty(&summary)?,
  )?;
  eprintln!("info: emit summary written to {}", manifest_path.display());
  Ok((summary, manifest_path))
}

fn run_fmt(args: &Args) -> Result<()> {
  let root = find_workspace_root()?;
  let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
  eprintln!("info: running cargo fmt in {}", root.display());

  let mut cmd = std::process::Command::new(cargo);
  cmd.current_dir(&root).arg("fmt").arg("--all");
  if args.fmt_check {
    cmd.arg("--").arg("--check");
  }

  let status = cmd
    .status()
    .map_err(|e| anyhow::anyhow!("failed to spawn cargo fmt: {}", e))?;
  if !status.success() {
    anyhow::bail!("pnix fmt failed (exit {:?})", status.code());
  }
  Ok(())
}

fn run_lint(_args: &Args) -> Result<()> {
  let root = find_workspace_root()?;
  let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
  eprintln!("info: running cargo clippy in {}", root.display());

  let mut cmd = std::process::Command::new(cargo);
  cmd
    .current_dir(&root)
    .arg("clippy")
    .arg("--workspace")
    .arg("--all-targets");
  if let Ok(raw) = std::env::var("PNIX_LINT_EXCLUDE_PACKAGES") {
    let mut excluded = Vec::new();
    for pkg in raw.split(',').map(str::trim).filter(|pkg| !pkg.is_empty()) {
      cmd.arg("--exclude").arg(pkg);
      excluded.push(pkg.to_string());
    }
    if !excluded.is_empty() {
      eprintln!("info: lint exclude packages={}", excluded.join(","));
    }
  }
  let strict = std::env::var("PNIX_LINT_STRICT")
    .ok()
    .map(|value| {
      let value = value.trim().to_ascii_lowercase();
      // 빈 문자열도 false로 처리
      !(value.is_empty() || value == "0" || value == "false" || value == "no")
    })
    .unwrap_or(false);
  if strict {
    cmd.arg("--").arg("-D").arg("warnings");
  }

  let output = cmd
    .output()
    .map_err(|e| anyhow::anyhow!("failed to spawn cargo clippy: {}", e))?;
  // LOW: from_utf8_lossy가 silent 데이터 손상 수정
  // 잘못된 UTF-8이 U+FFFD로 대체될 때 경고 출력
  let stdout = match String::from_utf8(output.stdout.clone()) {
    Ok(s) => s,
    Err(e) => {
      eprintln!(
        "Warning: Linker stdout contains invalid UTF-8, replacing with U+FFFD: {}",
        e
      );
      String::from_utf8_lossy(&output.stdout).to_string()
    }
  };
  let stderr = match String::from_utf8(output.stderr.clone()) {
    Ok(s) => s,
    Err(e) => {
      eprintln!(
        "Warning: Linker stderr contains invalid UTF-8, replacing with U+FFFD: {}",
        e
      );
      String::from_utf8_lossy(&output.stderr).to_string()
    }
  };
  print!("{}", stdout);
  eprint!("{}", stderr);
  if !output.status.success() {
    let has_error = stdout
      .lines()
      .chain(stderr.lines())
      .any(|line| line.starts_with("error:") || line.starts_with("error["));
    if has_error || strict {
      anyhow::bail!("pnix lint failed (exit {:?})", output.status.code());
    }
    eprintln!(
      "warn: clippy exited {:?} without error diagnostics; treating as warnings only",
      output.status.code()
    );
  }
  Ok(())
}

fn find_workspace_root() -> Result<PathBuf> {
  if let Ok(root) = std::env::var("PNIX_WORKSPACE_ROOT") {
    let root = root.trim();
    if !root.is_empty() {
      return Ok(PathBuf::from(root));
    }
  }

  let mut current =
    std::env::current_dir().map_err(|e| anyhow::anyhow!("failed to read current dir: {}", e))?;
  loop {
    let cargo = current.join("Cargo.toml");
    if cargo.is_file() {
      if let Ok(text) = std::fs::read_to_string(&cargo) {
        if text.contains("[workspace]") {
          return Ok(current);
        }
      }
    }

    if !current.pop() {
      break;
    }
  }

  anyhow::bail!("workspace root not found (set PNIX_WORKSPACE_ROOT)")
}

fn read_source(args: &Args) -> Result<(String, Option<String>)> {
  if let Some(expr) = &args.expr {
    return Ok((normalize_newlines(expr).into_owned(), None));
  }
  if let Some(path) = &args.source {
    let meta =
      std::fs::metadata(path).map_err(|err| anyhow::anyhow!("failed to read {}: {}", path, err))?;
    if meta.is_dir() {
      anyhow::bail!("--source path is a directory: {}", path);
    }
    if !meta.is_file() {
      anyhow::bail!("--source path is not a file: {}", path);
    }
    let contents = std::fs::read_to_string(path)
      .map_err(|err| anyhow::anyhow!("failed to read {}: {}", path, err))?;
    let normalized = normalize_newlines(&contents);
    let contents = match normalized {
      Cow::Borrowed(_) => contents,
      Cow::Owned(value) => value,
    };
    return Ok((contents, Some(path.clone())));
  }
  anyhow::bail!("--expr or --source is required for this mode")
}

fn sync_project_lock(entry_path: &Path, dry_run: bool) -> Result<()> {
  let entry_dir = entry_path
    .parent()
    .ok_or_else(|| anyhow::anyhow!("source path has no parent: {}", entry_path.display()))?;
  let manifest_path = match project::find_manifest(entry_dir) {
    Some(path) => path,
    None => return Ok(()),
  };
  let project_root = manifest_path.parent().ok_or_else(|| {
    anyhow::anyhow!(
      "manifest path has no parent directory: {}",
      manifest_path.display()
    )
  })?;

  let resolver = project::DependencyResolver::new(project_root);
  let graph = resolver.resolve(project_root).with_context(|| {
    format!(
      "dependency resolution failed for {}",
      project_root.display()
    )
  })?;

  graph
    .build_order()
    .map_err(|err| anyhow::anyhow!("dependency graph invalid: {}", err))?;

  let lock_path = project_root.join("pnix.lock");
  if lock_path.exists() {
    let lock = project::LockFile::load(&lock_path)
      .with_context(|| format!("failed to read {}", lock_path.display()))?;
    project::verify_lock_file(&lock, &graph, project_root)
      .with_context(|| format!("lock file verification failed: {}", lock_path.display()))?;
    eprintln!("info: verified {}", lock_path.display());
    return Ok(());
  }

  if dry_run {
    eprintln!("info: dry-run, skip writing {}", lock_path.display());
    return Ok(());
  }

  let lock = project::generate_lock_file(&graph, project_root)?;
  lock
    .save(&lock_path)
    .with_context(|| format!("failed to write {}", lock_path.display()))?;
  eprintln!("info: wrote {}", lock_path.display());
  Ok(())
}

fn verify_input_size_with_label(
  label: &str,
  input_bytes: usize,
  limits: &ResourceLimits,
) -> Result<()> {
  verify_input_size(input_bytes, limits).map_err(|err| anyhow::anyhow!("{}: {}", label, err))
}

fn read_text_file_with_limits(path: &Path, limits: &ResourceLimits, label: &str) -> Result<String> {
  let meta = std::fs::metadata(path)
    .map_err(|err| anyhow::anyhow!("failed to read {}: {}", path.display(), err))?;
  if meta.is_dir() {
    bail!("{} is a directory", path.display());
  }
  let size = usize::try_from(meta.len())
    .map_err(|_| anyhow::anyhow!("{} too large: {} bytes", path.display(), meta.len()))?;
  let label = format!("{} {}", label, path.display());
  verify_input_size_with_label(label.as_str(), size, limits)?;
  std::fs::read_to_string(path)
    .map_err(|err| anyhow::anyhow!("failed to read {}: {}", path.display(), err))
}

async fn read_text_file_with_limits_async(
  path: &Path,
  limits: &ResourceLimits,
  label: &str,
) -> Result<String> {
  let meta = tokio::fs::metadata(path)
    .await
    .map_err(|err| anyhow::anyhow!("failed to read {}: {}", path.display(), err))?;
  if meta.is_dir() {
    bail!("{} is a directory", path.display());
  }
  let size = usize::try_from(meta.len())
    .map_err(|_| anyhow::anyhow!("{} too large: {} bytes", path.display(), meta.len()))?;
  let label = format!("{} {}", label, path.display());
  verify_input_size_with_label(label.as_str(), size, limits)?;
  tokio::fs::read_to_string(path)
    .await
    .map_err(|err| anyhow::anyhow!("failed to read {}: {}", path.display(), err))
}

fn read_json_file_with_limits<T: DeserializeOwned>(
  path: &Path,
  limits: &ResourceLimits,
  label: &str,
) -> Result<T> {
  let contents = read_text_file_with_limits(path, limits, label)?;
  serde_json::from_str(&contents)
    .map_err(|err| anyhow::anyhow!("invalid {} {}: {}", label, path.display(), err))
}

async fn read_json_file_with_limits_async<T: DeserializeOwned>(
  path: &Path,
  limits: &ResourceLimits,
  label: &str,
) -> Result<T> {
  let contents = read_text_file_with_limits_async(path, limits, label).await?;
  serde_json::from_str(&contents)
    .map_err(|err| anyhow::anyhow!("invalid {} {}: {}", label, path.display(), err))
}

fn load_fxcore_from_dist(dist: &Path, limits: &ResourceLimits) -> Result<CoreFxCoreModule> {
  let fx_path = dist.join("ir").join("fxcore.canon.json");
  read_json_file_with_limits(&fx_path, limits, "fxcore json")
}

fn os_label(os: &pnix_core::build_ir::Os) -> &'static str {
  match os {
    pnix_core::build_ir::Os::Linux => "linux",
    pnix_core::build_ir::Os::Darwin => "darwin",
    pnix_core::build_ir::Os::Windows => "windows",
  }
}

fn arch_label(arch: &pnix_core::build_ir::Arch) -> &'static str {
  match arch {
    pnix_core::build_ir::Arch::X86_64 => "x86_64",
    pnix_core::build_ir::Arch::Aarch64 => "aarch64",
  }
}

fn emit_backend_legacy(
  ir: &pnix_runtime_legacy::ir::LegacyIr,
  emit_out: &Path,
  target: Option<CodegenTarget>,
) -> Result<serde_json::Value> {
  let targets = match target {
    Some(t) => vec![t],
    None => CodegenTarget::all(),
  };
  let mut config = CodegenConfig::new(targets[0]);
  config = config.with_deterministic(true);
  if target.is_none() {
    config = config.with_selected_targets(targets.clone());
  }

  let output = generate_from_ir_with_config(ir, &config)
    .map_err(|err| anyhow::anyhow!("codegen failed: {:?}", err))?;
  write_codegen_files(&output.files, emit_out)?;

  let manifest = output
    .manifest
    .ok_or_else(|| anyhow::anyhow!("missing codegen manifest"))?;
  let mut files = manifest.files.clone();
  files.sort_by(|a, b| a.path.cmp(&b.path));

  let target_labels: Vec<String> = targets
    .into_iter()
    .map(codegen_target_label)
    .map(str::to_string)
    .collect();

  Ok(serde_json::json!({
      "targets": target_labels,
      "files": files,
      "total_size": manifest.total_size
  }))
}

fn emit_aot(
  fx: &CoreFxCoreModule,
  target: AotTarget,
  emit_out: &Path,
) -> Result<serde_json::Value> {
  let module_name = if fx.name.is_empty() {
    "module".to_string()
  } else {
    fx.name.clone()
  };

  // Create AOT engine with target configuration
  let mut config = AotConfig {
    target,
    ..Default::default()
  };
  if let Ok(raw) = std::env::var("PNIX_AOT_MAIN_SYMBOL") {
    let symbol = raw.trim();
    if !symbol.is_empty() {
      config.main_symbol = symbol.to_string();
    }
  }
  let engine = AotEngine::with_config(config);

  // Validate emit target (path validation)
  engine
    .validate_emit_target(emit_out)
    .context("aot emit validation failed")?;

  // Convert FxCoreModule to IR bytes (JSON)
  let ir_bytes = serde_json::to_vec(fx)
    .map_err(|err| anyhow::anyhow!("failed to serialize FxCoreModule to IR: {}", err))?;

  // Use IR-based compilation API
  let output = engine
    .compile_from_ir(&module_name, &ir_bytes)
    .context("aot compile failed")?;

  let bin_dir = emit_out.join("bin");
  let manifest_dir = emit_out.join("manifest");
  std::fs::create_dir_all(&bin_dir)?;
  std::fs::create_dir_all(&manifest_dir)?;

  let binary_name = target.output_name(&module_name);
  let bin_path = bin_dir.join(&binary_name);
  write_bytes(emit_out, &bin_path, &output.binary)?;

  // Create manifest with target triple support
  let target_triple = engine.config.effective_target_triple();
  let mut manifest = if engine.config.target_triple_override.is_some() {
    AotArtifactManifest::new_with_triple(
      module_name.clone(),
      target,
      target_triple,
      output.entry_point.clone(),
    )
  } else {
    AotArtifactManifest::new(module_name.clone(), target, output.entry_point.clone())
  };
  manifest.library_path = None;
  let manifest_path = manifest_dir.join(format!("{}.json", module_name));
  write_text(emit_out, &manifest_path, &manifest.to_json()?)?;

  Ok(serde_json::json!({
      "target": aot_target_label(target),
      "target_triple": target_triple,
      "binary": format!("bin/{}", binary_name),
      "manifest": format!("manifest/{}.json", module_name),
      "entry_point": output.entry_point,
  }))
}

fn parse_emit_target(raw: &str) -> Result<EmitTarget> {
  let raw = raw.trim();
  if raw.eq_ignore_ascii_case("all") {
    return Ok(EmitTarget::LegacyAll);
  }
  if let Some(stripped) = raw.strip_prefix("aot:") {
    let target = parse_aot_target(stripped)
      .ok_or_else(|| anyhow::anyhow!("unknown aot target '{}'", stripped))?;
    return Ok(EmitTarget::Aot(target));
  }
  if raw.eq_ignore_ascii_case("aot") {
    return Ok(EmitTarget::Aot(default_aot_target()));
  }
  if let Some(target) = CodegenTarget::from_str(raw) {
    return Ok(EmitTarget::Legacy(target));
  }
  Err(anyhow::anyhow!("unknown emit target '{}'", raw))
}

fn parse_aot_target(raw: &str) -> Option<AotTarget> {
  let value = raw.trim().to_lowercase();
  match value.as_str() {
    "linux" | "linux-x86_64" | "x86_64-unknown-linux-gnu" => Some(AotTarget::LinuxX86_64),
    "macos" | "macos-x86_64" | "x86_64-apple-darwin" => Some(AotTarget::MacOSX86_64),
    "macos-arm64" | "macos-aarch64" | "aarch64-apple-darwin" => Some(AotTarget::MacOSArm64),
    "windows" | "windows-x86_64" | "x86_64-pc-windows-msvc" => Some(AotTarget::WindowsX86_64),
    _ => None,
  }
}

fn default_aot_target() -> AotTarget {
  let os = std::env::consts::OS;
  let arch = std::env::consts::ARCH;
  match (os, arch) {
    ("macos", "aarch64") => AotTarget::MacOSArm64,
    ("macos", "x86_64") => AotTarget::MacOSX86_64,
    ("windows", "x86_64") => AotTarget::WindowsX86_64,
    ("linux", "x86_64") => AotTarget::LinuxX86_64,
    _ => AotTarget::LinuxX86_64,
  }
}

fn emit_target_label(target: &EmitTarget) -> String {
  match target {
    EmitTarget::Legacy(t) => codegen_target_label(*t).to_string(),
    EmitTarget::LegacyAll => "all".to_string(),
    EmitTarget::Aot(t) => format!("aot:{}", aot_target_label(*t)),
  }
}

fn codegen_target_label(target: CodegenTarget) -> &'static str {
  match target {
    CodegenTarget::Javascript => "js",
    CodegenTarget::Typescript => "ts",
    CodegenTarget::Python => "python",
    CodegenTarget::Clojure => "clojure",
    CodegenTarget::Nix => "nix",
  }
}

fn aot_target_label(target: AotTarget) -> &'static str {
  match target {
    AotTarget::LinuxX86_64 => "linux-x86_64",
    AotTarget::MacOSX86_64 => "macos-x86_64",
    AotTarget::MacOSArm64 => "macos-arm64",
    AotTarget::WindowsX86_64 => "windows-x86_64",
  }
}

fn adapter_report_json(report: &pnix_ir_adapter::AdapterReport) -> serde_json::Value {
  let mut warnings = report.warnings.clone();
  let mut info = report.info.clone();
  let mut unsupported_nodes = report.unsupported_nodes.clone();
  let mut unsupported_edges = report.unsupported_edges.clone();
  warnings.sort();
  info.sort();
  unsupported_nodes.sort();
  unsupported_edges.sort();
  serde_json::json!({
      "warnings": warnings,
      "info": info,
      "unsupported_nodes": unsupported_nodes,
      "unsupported_edges": unsupported_edges
  })
}

fn write_codegen_files(
  files: &[pnix_backend_legacy::GeneratedFile],
  emit_out: &Path,
) -> Result<()> {
  for file in files {
    let path = Path::new(&file.path);

    // Fix: Reject absolute paths to prevent path escape attacks
    // Absolute paths in file.path can escape emit_out directory
    if path.is_absolute() {
      bail!(
        "absolute paths are not allowed in generated file paths: {}",
        file.path
      );
    }

    // Also check for path traversal attempts (..)
    if path
      .components()
      .any(|c| matches!(c, std::path::Component::ParentDir))
    {
      bail!(
        "path traversal (..) is not allowed in generated file paths: {}",
        file.path
      );
    }

    let full_path = emit_out.join(path);
    write_text(emit_out, &full_path, &file.contents)?;
  }
  Ok(())
}

fn ensure_parent_dirs(root: &Path, path: &Path) -> Result<()> {
  let parent = match path.parent() {
    Some(parent) if !parent.as_os_str().is_empty() => parent,
    _ => return Ok(()),
  };

  if !root.exists() {
    std::fs::create_dir_all(root)
      .with_context(|| format!("failed to create root directory {}", root.display()))?;
  }

  let relative_parent = parent.strip_prefix(root).with_context(|| {
    format!(
      "path must be within root directory (root={}, path={})",
      root.display(),
      parent.display()
    )
  })?;

  let mut current = root.to_path_buf();
  for component in relative_parent.components() {
    match component {
      std::path::Component::CurDir => {}
      std::path::Component::ParentDir => {
        bail!(
          "path must not contain '..' when creating parent directories: {}",
          parent.display()
        );
      }
      std::path::Component::Normal(part) => {
        current.push(part);
        if current.exists() {
          let meta = std::fs::symlink_metadata(&current)
            .with_context(|| format!("failed to stat parent directory {}", current.display()))?;
          if meta.file_type().is_symlink() {
            bail!(
              "refusing to write through symlinked directory: {}",
              current.display()
            );
          }
          if !meta.is_dir() {
            bail!("{} is not a directory", current.display());
          }
        } else {
          std::fs::create_dir(&current)
            .with_context(|| format!("failed to create parent directory {}", current.display()))?;
        }
      }
      std::path::Component::Prefix(_) | std::path::Component::RootDir => {
        bail!(
          "absolute paths are not allowed when creating parent directories: {}",
          parent.display()
        );
      }
    }
  }
  Ok(())
}

fn write_file_atomic(root: &Path, path: &Path, contents: &[u8]) -> Result<()> {
  path.strip_prefix(root).with_context(|| {
    format!(
      "path must be within root directory (root={}, path={})",
      root.display(),
      path.display()
    )
  })?;
  ensure_parent_dirs(root, path)?;

  let parent = match path.parent() {
    Some(parent) if !parent.as_os_str().is_empty() => parent,
    _ => root,
  };
  let mut temp = Builder::new()
    .prefix(".tmp")
    .tempfile_in(parent)
    .with_context(|| format!("failed to create temp file in {}", parent.display()))?;
  temp
    .write_all(contents)
    .with_context(|| format!("failed to write temp file for {}", path.display()))?;
  temp
    .flush()
    .with_context(|| format!("failed to flush temp file for {}", path.display()))?;
  match temp.persist(path) {
    Ok(_) => Ok(()),
    Err(err) => {
      if err.error.kind() == io::ErrorKind::AlreadyExists {
        if let Err(remove_err) = std::fs::remove_file(path) {
          if remove_err.kind() != io::ErrorKind::NotFound {
            return Err(anyhow::anyhow!(
              "failed to remove existing file {}: {}",
              path.display(),
              remove_err
            ));
          }
        }
        err.file.persist(path).map_err(|persist_err| {
          anyhow::anyhow!(
            "failed to persist temp file to {}: {}",
            path.display(),
            persist_err.error
          )
        })?;
        return Ok(());
      }
      Err(anyhow::anyhow!(
        "failed to persist temp file to {}: {}",
        path.display(),
        err.error
      ))
    }
  }
}

fn write_text(root: &Path, path: &Path, contents: &str) -> Result<()> {
  write_file_atomic(root, path, contents.as_bytes())
}

fn write_bytes(root: &Path, path: &Path, contents: &[u8]) -> Result<()> {
  write_file_atomic(root, path, contents)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_runner_tests {
  use super::test_runner::{collect_tests_from_source, run_tests, TestCaseKind};
  use pnix_runtime_api::EvalConfig;
  use std::path::PathBuf;

  #[test]
  fn test_runner_collects_tests() {
    let source = r#"
type Real
test test_one = assert(true)
test test_two = assertEqual(1, 1)
node n1 uses builtins.add
"#;

    let tests =
      collect_tests_from_source(source, Some(PathBuf::from("test.sam").as_path())).unwrap();

    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0].name, "test_one");
    assert!(matches!(
      &tests[0].kind,
      TestCaseKind::Expr(expr) if expr == "assert(true)"
    ));
    assert_eq!(tests[1].name, "test_two");
    assert!(matches!(
      &tests[1].kind,
      TestCaseKind::Expr(expr) if expr == "assertEqual(1, 1)"
    ));
  }

  #[test]
  fn test_runner_executes_passing_tests() {
    let source = r#"
test test_pass = assert true
test test_equal = assertEqual 1 1
"#;

    let tests =
      collect_tests_from_source(source, Some(PathBuf::from("test.sam").as_path())).unwrap();
    let config = EvalConfig::default();
    let summary = run_tests(tests, None, &config, None);

    assert_eq!(summary.total, 2);
    assert_eq!(summary.passed, 2);
    assert_eq!(summary.failed, 0);
  }

  #[test]
  fn test_runner_detects_failing_tests() {
    let source = r#"
test test_fail = assert false
test test_not_equal = assertEqual 1 2
"#;

    let tests =
      collect_tests_from_source(source, Some(PathBuf::from("test.sam").as_path())).unwrap();
    let config = EvalConfig::default();
    let summary = run_tests(tests, None, &config, None);

    assert_eq!(summary.total, 2);
    assert_eq!(summary.passed, 0);
    assert_eq!(summary.failed, 2);

    // 에러 메시지 확인
    assert!(summary.results[0].error.is_some());
    assert!(summary.results[0]
      .error
      .as_ref()
      .unwrap()
      .contains("assertion failed"));
    assert!(summary.results[1].error.is_some());
    assert!(summary.results[1]
      .error
      .as_ref()
      .unwrap()
      .contains("assertion failed"));
    assert!(summary.results[1]
      .error
      .as_ref()
      .unwrap()
      .contains("expected"));
    assert!(summary.results[1].error.as_ref().unwrap().contains("found"));
  }

  #[test]
  fn test_runner_filter_option() {
    let source = r#"
test test_one = assert true
test test_two = assert true
test test_three = assert true
"#;

    let tests =
      collect_tests_from_source(source, Some(PathBuf::from("test.sam").as_path())).unwrap();
    let config = EvalConfig::default();

    // 필터: "one"만 실행
    let summary = run_tests(tests.clone(), Some("one"), &config, None);
    assert_eq!(summary.total, 3); // 필터링된 테스트도 카운트에 포함
    assert_eq!(summary.passed, 1); // 필터링된 테스트는 skipped로 카운트
    assert_eq!(summary.skipped, 2); // 필터링된 테스트는 skipped로 카운트

    // 필터 없음: 모두 실행
    let summary = run_tests(tests, None, &config, None);
    assert_eq!(summary.total, 3);
    assert_eq!(summary.passed, 3);
    assert_eq!(summary.failed, 0);
  }

  #[test]
  fn test_runner_deterministic_order() {
    // 테스트 실행 순서가 결정론적인지 확인 (이름순 정렬)
    let source = r#"
test test_z = assert true
test test_a = assert true
test test_m = assert true
"#;

    let tests =
      collect_tests_from_source(source, Some(PathBuf::from("test.sam").as_path())).unwrap();
    let config = EvalConfig::default();
    let summary = run_tests(tests, None, &config, None);

    // 이름순으로 정렬되어야 함: a, m, z
    assert_eq!(summary.results[0].name, "test_a");
    assert_eq!(summary.results[1].name, "test_m");
    assert_eq!(summary.results[2].name, "test_z");
  }

  #[test]
  fn test_runner_executes_test_node_builtins() {
    let source = r#"
@test node test_node uses builtins.add
"#;

    let tests =
      collect_tests_from_source(source, Some(PathBuf::from("test.sam").as_path())).unwrap();
    let config = EvalConfig::default();
    let summary = run_tests(tests, None, &config, None);

    assert_eq!(summary.total, 1);
    assert_eq!(summary.passed, 1);
    assert_eq!(summary.failed, 0);
  }

  #[test]
  fn test_runner_rejects_test_node_non_builtin() {
    let source = r#"
@test node test_node uses py.unknown
"#;

    let tests =
      collect_tests_from_source(source, Some(PathBuf::from("test.sam").as_path())).unwrap();
    let config = EvalConfig::default();
    let summary = run_tests(tests, None, &config, None);

    assert_eq!(summary.total, 1);
    assert_eq!(summary.passed, 0);
    assert_eq!(summary.failed, 1);
    assert!(summary.results[0]
      .error
      .as_ref()
      .unwrap()
      .contains("unsupported @test node uses"));
  }
}

// ─────────────────────────────────────────────────────────────────────
// Phase 145++++++++++(zzzzd-0.5) (Codex audit, 2026-06-01):
// parse_http_url unit tests. Pure parser, no I/O.
#[cfg(test)]
mod parse_http_url_tests {
  use super::parse_http_url;

  #[test]
  fn root_with_default_port_80() {
    let (host, port, prefix) = parse_http_url("http://localhost").unwrap();
    assert_eq!(host, "localhost");
    assert_eq!(port, 80);
    assert_eq!(prefix, "");
  }

  #[test]
  fn explicit_port_no_path() {
    let (host, port, prefix) = parse_http_url("http://127.0.0.1:8787").unwrap();
    assert_eq!(host, "127.0.0.1");
    assert_eq!(port, 8787);
    assert_eq!(prefix, "");
  }

  #[test]
  fn explicit_port_with_prefix() {
    let (host, port, prefix) = parse_http_url("http://127.0.0.1:8787/prefix").unwrap();
    assert_eq!(host, "127.0.0.1");
    assert_eq!(port, 8787);
    assert_eq!(prefix, "/prefix");
  }

  #[test]
  fn trailing_slash_stripped() {
    let (_, _, prefix) = parse_http_url("http://h:1/a/").unwrap();
    assert_eq!(prefix, "/a");
  }

  #[test]
  fn https_rejected() {
    let err = parse_http_url("https://example.com").unwrap_err();
    assert!(err.to_string().contains("http://"));
  }

  #[test]
  fn empty_url_rejected() {
    assert!(parse_http_url("").is_err());
  }

  #[test]
  fn bad_port_rejected() {
    assert!(parse_http_url("http://h:notaport").is_err());
  }
}
