use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use pnix_core::lang::pnix::parse_expr;
use pnix_query_runtime::px_eval_json;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, LOCATION, USER_AGENT};
use reqwest::redirect::Policy;
use reqwest::Url;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Map, Value};
use pnix_hash::{Digest, Sha256};

use super::args::{Args, GateAbsorbVerb, OutputFormat};

const GATE_ABSORB_VERSION: &str = "0.6.1";
const EX_OK: i32 = 0;
const EX_SOURCE: i32 = 2;
const EX_PARSE: i32 = 3;
const EX_USAGE: i32 = 64;
const DEFAULT_USER_AGENT: &str = "pnix-gate-absorb/0.6 (+https://pnix.local)";
const DEFAULT_TIMEOUT_MS: u64 = 10_000;
const DEFAULT_MAX_REDIRECTS: usize = 3;
const USAGE_LINE: &str = "usage: pnix gate-absorb {help|url|topic|conversation|events} [args...]";
const TRANSCRIPT_RULE_SET: [&str; 8] = [
  "transcript-provider-message-v0",
  "transcript-tool-call-v0",
  "transcript-tool-output-v0",
  "transcript-custom-tool-v0",
  "transcript-custom-tool-output-v0",
  "px-syntax-v0",
  "px-syntax-apply-result-v0",
  "transcript-web-search-v0",
];
const EVENT_RULE_SET: [&str; 8] = [
  "bash-invocation-v0",
  "post-tool-application-v0",
  "user-prompt-v0",
  "session-lifecycle-v0",
  "learn-prompt-injected-v0",
  "context-clipped-v0",
  "hook-return-packet-v0",
  "provider-hook-drift-v0",
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct GateAbsorbUrlDryRunSummary {
  pub(super) subcommand: &'static str,
  pub(super) mode: &'static str,
  pub(super) url: String,
  pub(super) status: u16,
  pub(super) content_type: String,
  pub(super) content_sha256: String,
  pub(super) content_length: usize,
  pub(super) fetch_receipt: GateAbsorbFetchReceipt,
  pub(super) extraction_candidate: GateAbsorbExtractionCandidate,
  pub(super) source_risk_floor: GateAbsorbSourceRiskFloor,
  pub(super) truth_regime_classification: GateAbsorbTruthRegimeClassification,
  pub(super) evidence_bridge: GateAbsorbEvidenceBridge,
  pub(super) knowledge_promotion_candidate: GateAbsorbKnowledgePromotionCandidate,
  pub(super) research_judgement: GateAbsorbResearchJudgement,
  pub(super) research_revision_receipt: GateAbsorbResearchRevisionReceipt,
  pub(super) follow_related: Option<usize>,
  pub(super) note: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct GateAbsorbFetchReceipt {
  pub(super) artifact_family: String,
  pub(super) receipt_ref: String,
  pub(super) source_candidate_ref: Option<String>,
  pub(super) candidate_id: String,
  pub(super) kind: String,
  pub(super) provider: String,
  pub(super) model: Option<String>,
  pub(super) session_id: String,
  pub(super) source_rule: String,
  pub(super) requested_url: String,
  pub(super) final_url: String,
  pub(super) status_code: u16,
  pub(super) content_type: String,
  pub(super) content_hash: String,
  pub(super) content_length: usize,
  pub(super) fetched_at: String,
  pub(super) extractor_version: String,
  pub(super) truth_regime: String,
  pub(super) status: String,
  pub(super) direct_truth_source: bool,
  pub(super) source_id: String,
  pub(super) source_version: String,
  pub(super) source_checksum: String,
  pub(super) entity_key: String,
  pub(super) member_path: String,
  pub(super) lang: String,
  pub(super) rule_version: String,
  pub(super) source_identity_floor_status: String,
  pub(super) citation_policy: String,
  pub(super) freshness_policy: String,
  pub(super) license_policy: String,
  pub(super) raw_retention_policy: String,
  pub(super) extraction_status: String,
  pub(super) next_required_artifacts: Vec<String>,
  pub(super) promotion_boundary: String,
  pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct GateAbsorbResearchEvidence {
  pub(super) artifact_family: String,
  pub(super) status: String,
  pub(super) fetch_receipt: GateAbsorbFetchReceipt,
  pub(super) extraction_candidate: GateAbsorbExtractionCandidate,
  pub(super) source_risk_floor: GateAbsorbSourceRiskFloor,
  pub(super) truth_regime_classification: GateAbsorbTruthRegimeClassification,
  pub(super) evidence_bridge: GateAbsorbEvidenceBridge,
  pub(super) knowledge_promotion_candidate: GateAbsorbKnowledgePromotionCandidate,
  pub(super) research_judgement: GateAbsorbResearchJudgement,
  pub(super) research_revision_receipt: GateAbsorbResearchRevisionReceipt,
  pub(super) artifact_families: Vec<String>,
  pub(super) direct_truth_source: bool,
  pub(super) judgement_ready: bool,
  pub(super) promotion_ready: bool,
  pub(super) store_mutation: bool,
  pub(super) policy_mutation_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct GateAbsorbExtractionCandidate {
  pub(super) artifact_family: String,
  pub(super) extraction_ref: String,
  pub(super) fetch_receipt_ref: String,
  pub(super) source_candidate_ref: Option<String>,
  pub(super) candidate_id: String,
  pub(super) kind: String,
  pub(super) provider: String,
  pub(super) model: Option<String>,
  pub(super) session_id: String,
  pub(super) source_rule: String,
  pub(super) requested_url: String,
  pub(super) final_url: String,
  pub(super) content_type: String,
  pub(super) content_hash: String,
  pub(super) content_length: usize,
  pub(super) fetched_at: String,
  pub(super) extractor_version: String,
  pub(super) extraction_owner_status: String,
  pub(super) extraction_scope: String,
  pub(super) extraction_status: String,
  pub(super) status: String,
  pub(super) truth_regime: String,
  pub(super) raw_body_available: bool,
  pub(super) raw_text_retained: bool,
  pub(super) direct_truth_source: bool,
  pub(super) judgement_ready: bool,
  pub(super) promotion_ready: bool,
  pub(super) source_id: String,
  pub(super) source_version: String,
  pub(super) source_checksum: String,
  pub(super) entity_key: String,
  pub(super) member_path: String,
  pub(super) lang: String,
  pub(super) rule_version: String,
  pub(super) source_identity_floor_status: String,
  pub(super) citation_policy: String,
  pub(super) freshness_policy: String,
  pub(super) license_policy: String,
  pub(super) raw_retention_policy: String,
  pub(super) next_required_artifacts: Vec<String>,
  pub(super) promotion_boundary: String,
  pub(super) extracted_claim_refs: Vec<String>,
  pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct GateAbsorbSourceRiskFloor {
  pub(super) artifact_family: String,
  pub(super) risk_floor_ref: String,
  pub(super) fetch_receipt_ref: String,
  pub(super) extraction_candidate_ref: String,
  pub(super) source_candidate_ref: Option<String>,
  pub(super) kind: String,
  pub(super) provider: String,
  pub(super) model: Option<String>,
  pub(super) session_id: String,
  pub(super) source_rule: String,
  pub(super) status: String,
  pub(super) source_trust_status: String,
  pub(super) trust_score_owner_status: String,
  pub(super) risk_score_owner_status: String,
  pub(super) source_id: String,
  pub(super) source_version: String,
  pub(super) source_checksum: String,
  pub(super) entity_key: String,
  pub(super) member_path: String,
  pub(super) lang: String,
  pub(super) rule_version: String,
  pub(super) citation_policy: String,
  pub(super) freshness_policy: String,
  pub(super) license_policy: String,
  pub(super) redistribution_policy: String,
  pub(super) benchmark_contamination_policy: String,
  pub(super) adversarial_prompt_policy: String,
  pub(super) adversarial_source_policy: String,
  pub(super) raw_retention_policy: String,
  pub(super) source_text_retained: bool,
  pub(super) direct_truth_source: bool,
  pub(super) judgement_ready: bool,
  pub(super) promotion_ready: bool,
  pub(super) policy_mutation_applied: bool,
  pub(super) store_mutation: bool,
  pub(super) next_required_artifacts: Vec<String>,
  pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct GateAbsorbTruthRegimeClassification {
  pub(super) artifact_family: String,
  pub(super) classification_ref: String,
  pub(super) extraction_candidate_ref: String,
  pub(super) fetch_receipt_ref: String,
  pub(super) source_candidate_ref: Option<String>,
  pub(super) kind: String,
  pub(super) provider: String,
  pub(super) model: Option<String>,
  pub(super) session_id: String,
  pub(super) source_rule: String,
  pub(super) taxonomy_source: String,
  pub(super) allowed_truth_regimes: Vec<String>,
  pub(super) truth_regime: String,
  pub(super) truth_regime_status: String,
  pub(super) acceptance_boundary: String,
  pub(super) classification_confidence: String,
  pub(super) classification_basis: Vec<String>,
  pub(super) status: String,
  pub(super) direct_truth_source: bool,
  pub(super) judgement_ready: bool,
  pub(super) promotion_ready: bool,
  pub(super) source_id: String,
  pub(super) source_version: String,
  pub(super) source_checksum: String,
  pub(super) entity_key: String,
  pub(super) member_path: String,
  pub(super) lang: String,
  pub(super) rule_version: String,
  pub(super) next_required_artifacts: Vec<String>,
  pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct GateAbsorbEvidenceBridge {
  pub(super) artifact_family: String,
  pub(super) bridge_ref: String,
  pub(super) fetch_receipt_ref: String,
  pub(super) extraction_candidate_ref: String,
  pub(super) truth_regime_classification_ref: String,
  pub(super) source_risk_floor_ref: String,
  pub(super) source_candidate_ref: Option<String>,
  pub(super) kind: String,
  pub(super) provider: String,
  pub(super) model: Option<String>,
  pub(super) session_id: String,
  pub(super) source_rule: String,
  pub(super) status: String,
  pub(super) bridge_status: String,
  pub(super) truth_regime: String,
  pub(super) direct_truth_source: bool,
  pub(super) judgement_ready: bool,
  pub(super) promotion_ready: bool,
  pub(super) source_id: String,
  pub(super) source_version: String,
  pub(super) source_checksum: String,
  pub(super) entity_key: String,
  pub(super) member_path: String,
  pub(super) lang: String,
  pub(super) rule_version: String,
  pub(super) citation_policy: String,
  pub(super) freshness_policy: String,
  pub(super) license_policy: String,
  pub(super) redistribution_policy: String,
  pub(super) benchmark_contamination_policy: String,
  pub(super) adversarial_prompt_policy: String,
  pub(super) adversarial_source_policy: String,
  pub(super) adversarial_risk_policy: String,
  pub(super) raw_retention_policy: String,
  pub(super) promotion_boundary: String,
  pub(super) next_required_artifacts: Vec<String>,
  pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct GateAbsorbKnowledgePromotionCandidate {
  pub(super) artifact_family: String,
  pub(super) promotion_candidate_ref: String,
  pub(super) evidence_bridge_ref: String,
  pub(super) fetch_receipt_ref: String,
  pub(super) extraction_candidate_ref: String,
  pub(super) truth_regime_classification_ref: String,
  pub(super) source_candidate_ref: Option<String>,
  pub(super) kind: String,
  pub(super) provider: String,
  pub(super) model: Option<String>,
  pub(super) session_id: String,
  pub(super) source_rule: String,
  pub(super) status: String,
  pub(super) promotion_status: String,
  pub(super) truth_regime: String,
  pub(super) verification_policy: String,
  pub(super) human_review_policy: String,
  pub(super) direct_truth_source: bool,
  pub(super) auto_promote_allowed: bool,
  pub(super) candidate_to_accepted_direct_allowed: bool,
  pub(super) candidate_to_candidate_verification_allowed: bool,
  pub(super) judgement_ready: bool,
  pub(super) promotion_ready: bool,
  pub(super) source_id: String,
  pub(super) source_version: String,
  pub(super) source_checksum: String,
  pub(super) entity_key: String,
  pub(super) member_path: String,
  pub(super) lang: String,
  pub(super) rule_version: String,
  pub(super) citation_policy: String,
  pub(super) freshness_policy: String,
  pub(super) license_policy: String,
  pub(super) adversarial_risk_policy: String,
  pub(super) promotion_boundary: String,
  pub(super) next_required_artifacts: Vec<String>,
  pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct GateAbsorbResearchJudgement {
  pub(super) artifact_family: String,
  pub(super) judgement_ref: String,
  pub(super) promotion_candidate_ref: String,
  pub(super) evidence_bridge_ref: String,
  pub(super) kind: String,
  pub(super) provider: String,
  pub(super) model: Option<String>,
  pub(super) session_id: String,
  pub(super) source_rule: String,
  pub(super) status: String,
  pub(super) judgement_action: String,
  pub(super) hold_reason: String,
  pub(super) truth_regime: String,
  pub(super) accepted: bool,
  pub(super) rejected: bool,
  pub(super) promotion_approved: bool,
  pub(super) direct_truth_source: bool,
  pub(super) policy_mutation_applied: bool,
  pub(super) store_mutation: bool,
  pub(super) source_id: String,
  pub(super) source_version: String,
  pub(super) source_checksum: String,
  pub(super) entity_key: String,
  pub(super) member_path: String,
  pub(super) lang: String,
  pub(super) rule_version: String,
  pub(super) next_required_artifacts: Vec<String>,
  pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct GateAbsorbResearchRevisionReceipt {
  pub(super) artifact_family: String,
  pub(super) revision_receipt_ref: String,
  pub(super) research_judgement_ref: String,
  pub(super) promotion_candidate_ref: String,
  pub(super) evidence_bridge_ref: String,
  pub(super) kind: String,
  pub(super) provider: String,
  pub(super) model: Option<String>,
  pub(super) session_id: String,
  pub(super) source_rule: String,
  pub(super) status: String,
  pub(super) revision_status: String,
  pub(super) learning_loop_status: String,
  pub(super) judgement_action: String,
  pub(super) hold_reason: String,
  pub(super) policy_mutation_applied: bool,
  pub(super) store_mutation: bool,
  pub(super) knowledge_promotion_applied: bool,
  pub(super) source_id: String,
  pub(super) source_version: String,
  pub(super) source_checksum: String,
  pub(super) entity_key: String,
  pub(super) member_path: String,
  pub(super) lang: String,
  pub(super) rule_version: String,
  pub(super) evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct GateAbsorbConversationDryRunSummary {
  pub(super) subcommand: &'static str,
  pub(super) mode: &'static str,
  pub(super) path: String,
  pub(super) turn_count: usize,
  pub(super) language_counts: BTreeMap<String, usize>,
  pub(super) token_total: usize,
  pub(super) speakers: Vec<String>,
  pub(super) note: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct GateAbsorbConversationIngestSummary {
  pub(super) subcommand: &'static str,
  pub(super) mode: &'static str,
  pub(super) path: String,
  pub(super) lines_total: usize,
  pub(super) lines_new: usize,
  pub(super) matched: usize,
  pub(super) emitted: Vec<GateAbsorbEmittedCandidate>,
  pub(super) dry_run: bool,
  pub(super) reset: bool,
  pub(super) line_offset_before: usize,
  pub(super) line_offset_after: usize,
  pub(super) session_id: String,
  pub(super) rule_set: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct GateAbsorbEventsIngestSummary {
  pub(super) subcommand: &'static str,
  pub(super) mode: &'static str,
  pub(super) path: String,
  pub(super) processed: usize,
  pub(super) fresh: usize,
  pub(super) matched: usize,
  pub(super) emitted: Vec<GateAbsorbEmittedCandidate>,
  pub(super) dry_run: bool,
  pub(super) reset: bool,
  pub(super) cursor_before: Option<String>,
  pub(super) cursor_after: Option<String>,
  pub(super) rule_set: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct GateAbsorbEmittedCandidate {
  pub(super) filename: String,
  pub(super) path: String,
  pub(super) status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct GateAbsorbConversationTurn {
  pub(super) turn: usize,
  pub(super) speaker: String,
  pub(super) text: String,
  pub(super) language: GateAbsorbLanguage,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct GateAbsorbVocabSummary {
  pub(super) language: GateAbsorbLanguage,
  pub(super) tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct GateAbsorbErrorSummary {
  pub(super) subcommand: &'static str,
  pub(super) mode: &'static str,
  pub(super) path_or_url: String,
  pub(super) status: Option<u16>,
  pub(super) error: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) enum GateAbsorbLanguage {
  Ko,
  En,
  Ja,
  Unknown,
}

impl GateAbsorbLanguage {
  fn key(&self) -> &'static str {
    match self {
      Self::Ko => "ko",
      Self::En => "en",
      Self::Ja => "ja",
      Self::Unknown => "unknown",
    }
  }
}

#[derive(Debug)]
pub(super) struct GateAbsorbVisitResult {
  pub(super) final_url: String,
  pub(super) content_type: String,
  pub(super) content_sha256: String,
  pub(super) content: String,
}

#[derive(Debug)]
pub(super) struct GateAbsorbVisitError {
  pub(super) status: Option<u16>,
  pub(super) message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
struct TranscriptCursorRecord {
  line_offset: usize,
  session_id: String,
  updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
struct EventCursorRecord {
  cursor: String,
  updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct GateAbsorbFloorOwnerResult {
  attrs: Map<String, Value>,
  status: Option<String>,
  quarantine_reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct TranscriptContext {
  session_id: String,
  turn_id: Option<String>,
  cwd: Option<String>,
  model: Option<String>,
  model_provider: Option<String>,
}

#[derive(Debug, Clone)]
struct PendingCallRecord {
  call_id: String,
  tool_name: String,
  input_text: String,
  ctx: TranscriptContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct CandidateDraft {
  dedupe_key: String,
  attrs: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct TranscriptToolOutputParse {
  surface: String,
  provider_command: Option<String>,
  chunk_id: Option<String>,
  wall_time_seconds: Option<f64>,
  exit_code: Option<i64>,
  running_session_id: Option<String>,
  original_token_count: Option<u64>,
  response_preview: String,
  response_length: u64,
  duration_seconds: Option<f64>,
  error_message: Option<String>,
  updated_paths: Vec<String>,
  change_kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct ApplyPatchInputParse {
  surface: String,
  has_begin_patch: bool,
  has_end_patch: bool,
  op_total: u64,
  px_op_total: u64,
  add_op_total: u64,
  px_add_op_total: u64,
  ops: Vec<ApplyPatchOpParse>,
  add_ops: Vec<ApplyPatchOpParse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct ApplyPatchOpParse {
  operation: String,
  path: String,
  is_px_path: bool,
  move_to_path: Option<String>,
  move_to_is_px_path: bool,
  body_text: String,
  body_bytes: u64,
  body_prefix_error_total: u64,
  parse_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct ApplyPatchFinalFileProof {
  operation: String,
  operation_kind: String,
  path: String,
  source_path: Option<String>,
  move_to_path: Option<String>,
  apply_result_path_seen: bool,
  apply_result_change_kinds: Vec<String>,
  final_read_status: String,
  final_file_present: bool,
  final_parse_status: String,
  final_parse_error: Option<String>,
  final_bytes: Option<u64>,
  source_read_status: Option<String>,
  source_file_present: Option<bool>,
  proof_source: String,
  proof_boundary: String,
}

#[derive(Debug, Clone, Default)]
struct UpdatedFileSummary {
  change_kinds: Vec<String>,
  updated_paths: Vec<String>,
}

pub(super) fn run_gate_absorb(args: &Args, verb: &GateAbsorbVerb) -> Result<i32> {
  match verb {
    GateAbsorbVerb::Missing => {
      eprintln!("{}", USAGE_LINE);
      Ok(EX_USAGE)
    }
    GateAbsorbVerb::Unknown(raw) => {
      eprintln!("pnix-gate-absorb: unknown subcommand: {:?}", raw);
      eprintln!("{}", USAGE_LINE);
      Ok(EX_USAGE)
    }
    GateAbsorbVerb::Help => {
      print_help_banner();
      Ok(EX_OK)
    }
    GateAbsorbVerb::Topic => run_gate_absorb_topic(args),
    GateAbsorbVerb::Url => run_gate_absorb_url(args),
    GateAbsorbVerb::Conversation => run_gate_absorb_conversation(args),
    GateAbsorbVerb::Events => run_gate_absorb_events(args),
  }
}

fn print_help_banner() {
  println!(
    "pnix-gate-absorb {} — standalone knowledge absorption CLI",
    GATE_ABSORB_VERSION
  );
  println!();
  println!("usage:");
  println!("  pnix gate-absorb help");
  println!("  pnix gate-absorb url <URL> [--follow-related N] [--dry-run]");
  println!("  pnix gate-absorb topic <QUERY> [--dry-run]");
  println!("  pnix gate-absorb conversation <FILE> [--dry-run]");
  println!("  pnix gate-absorb events [EVENTS.jsonl] [--limit N] [--reset] [--dry-run]");
}

fn run_gate_absorb_topic(args: &Args) -> Result<i32> {
  let summary = format!(
    "pnix-gate-absorb: topic not yet implemented (Phase A2+). args: {{subject={:?}, dry-run={}}}",
    args.gate_absorb_subject, args.dry_run
  );
  if args.dry_run {
    println!("{summary}");
    Ok(EX_OK)
  } else {
    eprintln!("{summary}");
    Ok(EX_USAGE)
  }
}

fn run_gate_absorb_url(args: &Args) -> Result<i32> {
  let Some(url) = args.gate_absorb_subject.as_deref() else {
    eprintln!("pnix-gate-absorb url: exactly one URL is required");
    eprintln!("{}", USAGE_LINE);
    return Ok(EX_USAGE);
  };
  if !args.dry_run {
    eprintln!(
      "pnix-gate-absorb: url not yet implemented (Phase A2+). args: {{subject={:?}, dry-run=false}}",
      args.gate_absorb_subject
    );
    return Ok(EX_USAGE);
  }
  match visit_url(url) {
    Ok(result) => {
      let content_length = result.content.chars().count();
      let research_evidence = build_url_research_evidence(url, &result, content_length)?;
      let summary = GateAbsorbUrlDryRunSummary {
        subcommand: "url",
        mode: "dry-run",
        url: result.final_url,
        status: 200,
        content_type: result.content_type,
        content_sha256: result.content_sha256,
        content_length,
        fetch_receipt: research_evidence.fetch_receipt,
        extraction_candidate: research_evidence.extraction_candidate,
        source_risk_floor: research_evidence.source_risk_floor,
        truth_regime_classification: research_evidence.truth_regime_classification,
        evidence_bridge: research_evidence.evidence_bridge,
        knowledge_promotion_candidate: research_evidence.knowledge_promotion_candidate,
        research_judgement: research_evidence.research_judgement,
        research_revision_receipt: research_evidence.research_revision_receipt,
        follow_related: args.gate_absorb_follow_related,
        note: "research evidence ladder projected; no .px emitted",
      };
      print_url_summary(args.output_format, &summary)?;
      Ok(EX_OK)
    }
    Err(error) => {
      let summary = GateAbsorbErrorSummary {
        subcommand: "url",
        mode: "dry-run",
        path_or_url: url.to_string(),
        status: error.status,
        error: error.message,
      };
      print_error_summary(args.output_format, &summary)?;
      Ok(EX_SOURCE)
    }
  }
}

#[cfg(test)]
pub(super) fn build_url_fetch_receipt(
  requested_url: &str,
  result: &GateAbsorbVisitResult,
  content_length: usize,
) -> Result<GateAbsorbFetchReceipt> {
  Ok(build_url_research_evidence(requested_url, result, content_length)?.fetch_receipt)
}

pub(super) fn build_url_research_evidence(
  requested_url: &str,
  result: &GateAbsorbVisitResult,
  content_length: usize,
) -> Result<GateAbsorbResearchEvidence> {
  parse_json_value(eval_gate_absorb_url_owner(
    "researchEvidence",
    &json!({
      "requested_url": requested_url,
      "final_url": result.final_url,
      "status_code": 200,
      "content_type": result.content_type,
      "content_sha256": result.content_sha256,
      "content_length": content_length,
      "fetched_at": unix_ms().to_string(),
      "session_id": "gate-absorb-url-dry-run",
    }),
  )?)
}

fn run_gate_absorb_conversation(args: &Args) -> Result<i32> {
  let Some(path) = args.gate_absorb_subject.as_deref() else {
    eprintln!("pnix-gate-absorb conversation: exactly one transcript file is required");
    eprintln!("{}", USAGE_LINE);
    return Ok(EX_USAGE);
  };
  let transcript_path = Path::new(path);
  if !transcript_path.exists() {
    let summary = GateAbsorbErrorSummary {
      subcommand: "conversation",
      mode: if args.dry_run { "dry-run" } else { "emit" },
      path_or_url: path.to_string(),
      status: None,
      error: "file not found".to_string(),
    };
    print_error_summary(args.output_format, &summary)?;
    return Ok(EX_SOURCE);
  }
  if transcript_looks_like_jsonl(transcript_path)? {
    match ingest_transcript_jsonl(transcript_path, args.dry_run) {
      Ok(summary) => {
        print_conversation_ingest_summary(args.output_format, &summary)?;
        Ok(EX_OK)
      }
      Err(error) => {
        let summary = GateAbsorbErrorSummary {
          subcommand: "conversation",
          mode: if args.dry_run { "dry-run" } else { "emit" },
          path_or_url: path.to_string(),
          status: None,
          error: error.to_string(),
        };
        print_error_summary(args.output_format, &summary)?;
        Ok(EX_PARSE)
      }
    }
  } else if args.dry_run {
    match parse_transcript_file(transcript_path) {
      Ok(turns) => {
        let mut language_counts = BTreeMap::new();
        let mut token_total = 0;
        let mut speakers = Vec::new();
        for turn in &turns {
          *language_counts
            .entry(turn.language.key().to_string())
            .or_insert(0) += 1;
          token_total += extract_vocab(turn).tokens.len();
          if !speakers.iter().any(|speaker| speaker == &turn.speaker) {
            speakers.push(turn.speaker.clone());
          }
        }
        let summary = GateAbsorbConversationDryRunSummary {
          subcommand: "conversation",
          mode: "dry-run",
          path: path.to_string(),
          turn_count: turns.len(),
          language_counts,
          token_total,
          speakers,
          note: "no .px emitted",
        };
        print_conversation_summary(args.output_format, &summary)?;
        Ok(EX_OK)
      }
      Err(error) => {
        let summary = GateAbsorbErrorSummary {
          subcommand: "conversation",
          mode: "dry-run",
          path_or_url: path.to_string(),
          status: None,
          error: error.to_string(),
        };
        print_error_summary(args.output_format, &summary)?;
        Ok(EX_PARSE)
      }
    }
  } else {
    eprintln!(
      "pnix-gate-absorb: conversation emit currently requires transcript JSONL input. path={:?}",
      args.gate_absorb_subject
    );
    Ok(EX_USAGE)
  }
}

fn run_gate_absorb_events(args: &Args) -> Result<i32> {
  let path = args
    .gate_absorb_subject
    .as_deref()
    .map(PathBuf::from)
    .unwrap_or(gate_store_root()?.join("events.jsonl"));
  if !path.exists() {
    let summary = GateAbsorbErrorSummary {
      subcommand: "events",
      mode: if args.dry_run { "dry-run" } else { "emit" },
      path_or_url: path.display().to_string(),
      status: None,
      error: "file not found".to_string(),
    };
    print_error_summary(args.output_format, &summary)?;
    return Ok(EX_SOURCE);
  }
  match ingest_events_jsonl(
    &path,
    args.gate_absorb_limit.unwrap_or(50),
    args.dry_run,
    args.gate_absorb_reset,
  ) {
    Ok(summary) => {
      print_events_ingest_summary(args.output_format, &summary)?;
      Ok(EX_OK)
    }
    Err(error) => {
      let summary = GateAbsorbErrorSummary {
        subcommand: "events",
        mode: if args.dry_run { "dry-run" } else { "emit" },
        path_or_url: path.display().to_string(),
        status: None,
        error: error.to_string(),
      };
      print_error_summary(args.output_format, &summary)?;
      Ok(EX_PARSE)
    }
  }
}

fn print_url_summary(format: OutputFormat, summary: &GateAbsorbUrlDryRunSummary) -> Result<()> {
  match format {
    OutputFormat::Json => {
      println!("{}", serde_json::to_string_pretty(summary)?);
    }
    OutputFormat::Text => {
      let mut rendered = String::new();
      writeln!(&mut rendered, "{{:subcommand \"{}\"", summary.subcommand)?;
      writeln!(&mut rendered, " :mode \"{}\"", summary.mode)?;
      writeln!(&mut rendered, " :url \"{}\"", escape_text(&summary.url))?;
      writeln!(&mut rendered, " :status {}", summary.status)?;
      writeln!(
        &mut rendered,
        " :content-type \"{}\"",
        escape_text(&summary.content_type)
      )?;
      writeln!(
        &mut rendered,
        " :content-sha256 \"{}\"",
        summary.content_sha256
      )?;
      writeln!(&mut rendered, " :content-length {}", summary.content_length)?;
      writeln!(
        &mut rendered,
        " :fetch-receipt-family \"{}\"",
        summary.fetch_receipt.artifact_family
      )?;
      writeln!(
        &mut rendered,
        " :fetch-receipt-status \"{}\"",
        summary.fetch_receipt.status
      )?;
      writeln!(
        &mut rendered,
        " :fetch-receipt-content-hash \"{}\"",
        summary.fetch_receipt.content_hash
      )?;
      writeln!(
        &mut rendered,
        " :fetch-receipt-direct-truth-source {}",
        summary.fetch_receipt.direct_truth_source
      )?;
      writeln!(
        &mut rendered,
        " :fetch-receipt-source-identity-floor \"{}\"",
        summary.fetch_receipt.source_identity_floor_status
      )?;
      writeln!(
        &mut rendered,
        " :fetch-receipt-lang \"{}\"",
        summary.fetch_receipt.lang
      )?;
      writeln!(
        &mut rendered,
        " :fetch-receipt-promotion-boundary \"{}\"",
        summary.fetch_receipt.promotion_boundary
      )?;
      writeln!(
        &mut rendered,
        " :extraction-candidate-family \"{}\"",
        summary.extraction_candidate.artifact_family
      )?;
      writeln!(
        &mut rendered,
        " :extraction-candidate-status \"{}\"",
        summary.extraction_candidate.status
      )?;
      writeln!(
        &mut rendered,
        " :extraction-candidate-scope \"{}\"",
        summary.extraction_candidate.extraction_scope
      )?;
      writeln!(
        &mut rendered,
        " :extraction-candidate-raw-body-available {}",
        summary.extraction_candidate.raw_body_available
      )?;
      writeln!(
        &mut rendered,
        " :source-risk-floor-family \"{}\"",
        summary.source_risk_floor.artifact_family
      )?;
      writeln!(
        &mut rendered,
        " :source-risk-floor-status \"{}\"",
        summary.source_risk_floor.status
      )?;
      writeln!(
        &mut rendered,
        " :source-risk-floor-trust-score-owner \"{}\"",
        summary.source_risk_floor.trust_score_owner_status
      )?;
      writeln!(
        &mut rendered,
        " :source-risk-floor-benchmark-policy \"{}\"",
        summary.source_risk_floor.benchmark_contamination_policy
      )?;
      writeln!(
        &mut rendered,
        " :source-risk-floor-adversarial-policy \"{}\"",
        summary.source_risk_floor.adversarial_source_policy
      )?;
      writeln!(
        &mut rendered,
        " :source-risk-floor-direct-truth-source {}",
        summary.source_risk_floor.direct_truth_source
      )?;
      writeln!(
        &mut rendered,
        " :truth-regime-classification-family \"{}\"",
        summary.truth_regime_classification.artifact_family
      )?;
      writeln!(
        &mut rendered,
        " :truth-regime \"{}\"",
        summary.truth_regime_classification.truth_regime
      )?;
      writeln!(
        &mut rendered,
        " :truth-regime-classification-status \"{}\"",
        summary.truth_regime_classification.status
      )?;
      writeln!(
        &mut rendered,
        " :evidence-bridge-family \"{}\"",
        summary.evidence_bridge.artifact_family
      )?;
      writeln!(
        &mut rendered,
        " :evidence-bridge-status \"{}\"",
        summary.evidence_bridge.status
      )?;
      writeln!(
        &mut rendered,
        " :evidence-bridge-direct-truth-source {}",
        summary.evidence_bridge.direct_truth_source
      )?;
      writeln!(
        &mut rendered,
        " :evidence-bridge-judgement-ready {}",
        summary.evidence_bridge.judgement_ready
      )?;
      writeln!(
        &mut rendered,
        " :evidence-bridge-promotion-ready {}",
        summary.evidence_bridge.promotion_ready
      )?;
      writeln!(
        &mut rendered,
        " :evidence-bridge-promotion-boundary \"{}\"",
        summary.evidence_bridge.promotion_boundary
      )?;
      writeln!(
        &mut rendered,
        " :knowledge-promotion-candidate-family \"{}\"",
        summary.knowledge_promotion_candidate.artifact_family
      )?;
      writeln!(
        &mut rendered,
        " :knowledge-promotion-status \"{}\"",
        summary.knowledge_promotion_candidate.promotion_status
      )?;
      writeln!(
        &mut rendered,
        " :knowledge-promotion-auto-promote-allowed {}",
        summary.knowledge_promotion_candidate.auto_promote_allowed
      )?;
      writeln!(
        &mut rendered,
        " :knowledge-promotion-direct-accepted-allowed {}",
        summary
          .knowledge_promotion_candidate
          .candidate_to_accepted_direct_allowed
      )?;
      writeln!(
        &mut rendered,
        " :research-judgement-family \"{}\"",
        summary.research_judgement.artifact_family
      )?;
      writeln!(
        &mut rendered,
        " :research-judgement-action \"{}\"",
        summary.research_judgement.judgement_action
      )?;
      writeln!(
        &mut rendered,
        " :research-judgement-promotion-approved {}",
        summary.research_judgement.promotion_approved
      )?;
      writeln!(
        &mut rendered,
        " :research-revision-receipt-family \"{}\"",
        summary.research_revision_receipt.artifact_family
      )?;
      writeln!(
        &mut rendered,
        " :research-revision-status \"{}\"",
        summary.research_revision_receipt.revision_status
      )?;
      writeln!(
        &mut rendered,
        " :research-revision-policy-mutation-applied {}",
        summary.research_revision_receipt.policy_mutation_applied
      )?;
      if let Some(follow_related) = summary.follow_related {
        writeln!(&mut rendered, " :follow-related {}", follow_related)?;
      }
      writeln!(&mut rendered, " :note \"{}\"}}", summary.note)?;
      print!("{rendered}");
    }
  }
  Ok(())
}

fn print_conversation_summary(
  format: OutputFormat,
  summary: &GateAbsorbConversationDryRunSummary,
) -> Result<()> {
  match format {
    OutputFormat::Json => {
      println!("{}", serde_json::to_string_pretty(summary)?);
    }
    OutputFormat::Text => {
      let mut rendered = String::new();
      writeln!(&mut rendered, "{{:subcommand \"{}\"", summary.subcommand)?;
      writeln!(&mut rendered, " :mode \"{}\"", summary.mode)?;
      writeln!(&mut rendered, " :path \"{}\"", escape_text(&summary.path))?;
      writeln!(&mut rendered, " :turn-count {}", summary.turn_count)?;
      write!(&mut rendered, " :language-counts {{")?;
      let mut first = true;
      for (language, count) in &summary.language_counts {
        if !first {
          write!(&mut rendered, " ")?;
        }
        first = false;
        write!(&mut rendered, ":{} {}", language, count)?;
      }
      writeln!(&mut rendered, "}}")?;
      writeln!(&mut rendered, " :token-total {}", summary.token_total)?;
      write!(&mut rendered, " :speakers [")?;
      for (index, speaker) in summary.speakers.iter().enumerate() {
        if index > 0 {
          write!(&mut rendered, " ")?;
        }
        write!(&mut rendered, "\"{}\"", escape_text(speaker))?;
      }
      writeln!(&mut rendered, "]")?;
      writeln!(&mut rendered, " :note \"{}\"}}", summary.note)?;
      print!("{rendered}");
    }
  }
  Ok(())
}

fn print_conversation_ingest_summary(
  format: OutputFormat,
  summary: &GateAbsorbConversationIngestSummary,
) -> Result<()> {
  match format {
    OutputFormat::Json => {
      println!("{}", serde_json::to_string_pretty(summary)?);
    }
    OutputFormat::Text => {
      let mut rendered = String::new();
      writeln!(&mut rendered, "{{:subcommand \"{}\"", summary.subcommand)?;
      writeln!(&mut rendered, " :mode \"{}\"", summary.mode)?;
      writeln!(&mut rendered, " :path \"{}\"", escape_text(&summary.path))?;
      writeln!(&mut rendered, " :lines-total {}", summary.lines_total)?;
      writeln!(&mut rendered, " :lines-new {}", summary.lines_new)?;
      writeln!(&mut rendered, " :matched {}", summary.matched)?;
      writeln!(&mut rendered, " :dry-run {}", summary.dry_run)?;
      writeln!(&mut rendered, " :reset {}", summary.reset)?;
      writeln!(
        &mut rendered,
        " :line-offset-before {}",
        summary.line_offset_before
      )?;
      writeln!(
        &mut rendered,
        " :line-offset-after {}",
        summary.line_offset_after
      )?;
      writeln!(
        &mut rendered,
        " :session-id \"{}\"",
        escape_text(&summary.session_id)
      )?;
      write!(&mut rendered, " :rule-set [")?;
      for (index, rule) in summary.rule_set.iter().enumerate() {
        if index > 0 {
          write!(&mut rendered, " ")?;
        }
        write!(&mut rendered, "\"{}\"", escape_text(rule))?;
      }
      writeln!(&mut rendered, "]")?;
      write!(&mut rendered, " :emitted [")?;
      for (index, item) in summary.emitted.iter().enumerate() {
        if index > 0 {
          write!(&mut rendered, " ")?;
        }
        write!(
          &mut rendered,
          "{{:filename \"{}\" :status \"{}\" :path \"{}\"}}",
          escape_text(&item.filename),
          escape_text(&item.status),
          escape_text(&item.path)
        )?;
      }
      writeln!(&mut rendered, "]}}")?;
      print!("{rendered}");
    }
  }
  Ok(())
}

fn print_events_ingest_summary(
  format: OutputFormat,
  summary: &GateAbsorbEventsIngestSummary,
) -> Result<()> {
  match format {
    OutputFormat::Json => {
      println!("{}", serde_json::to_string_pretty(summary)?);
    }
    OutputFormat::Text => {
      let mut rendered = String::new();
      writeln!(&mut rendered, "{{:subcommand \"{}\"", summary.subcommand)?;
      writeln!(&mut rendered, " :mode \"{}\"", summary.mode)?;
      writeln!(&mut rendered, " :path \"{}\"", escape_text(&summary.path))?;
      writeln!(&mut rendered, " :processed {}", summary.processed)?;
      writeln!(&mut rendered, " :fresh {}", summary.fresh)?;
      writeln!(&mut rendered, " :matched {}", summary.matched)?;
      writeln!(&mut rendered, " :dry-run {}", summary.dry_run)?;
      writeln!(&mut rendered, " :reset {}", summary.reset)?;
      if let Some(cursor_before) = &summary.cursor_before {
        writeln!(
          &mut rendered,
          " :cursor-before \"{}\"",
          escape_text(cursor_before)
        )?;
      }
      if let Some(cursor_after) = &summary.cursor_after {
        writeln!(
          &mut rendered,
          " :cursor-after \"{}\"",
          escape_text(cursor_after)
        )?;
      }
      write!(&mut rendered, " :rule-set [")?;
      for (index, rule) in summary.rule_set.iter().enumerate() {
        if index > 0 {
          write!(&mut rendered, " ")?;
        }
        write!(&mut rendered, "\"{}\"", escape_text(rule))?;
      }
      writeln!(&mut rendered, "]")?;
      write!(&mut rendered, " :emitted [")?;
      for (index, item) in summary.emitted.iter().enumerate() {
        if index > 0 {
          write!(&mut rendered, " ")?;
        }
        write!(
          &mut rendered,
          "{{:filename \"{}\" :status \"{}\" :path \"{}\"}}",
          escape_text(&item.filename),
          escape_text(&item.status),
          escape_text(&item.path)
        )?;
      }
      writeln!(&mut rendered, "]}}")?;
      print!("{rendered}");
    }
  }
  Ok(())
}

fn print_error_summary(format: OutputFormat, summary: &GateAbsorbErrorSummary) -> Result<()> {
  match format {
    OutputFormat::Json => {
      eprintln!("{}", serde_json::to_string_pretty(summary)?);
    }
    OutputFormat::Text => {
      let mut rendered = String::new();
      writeln!(&mut rendered, "{{:subcommand \"{}\"", summary.subcommand)?;
      writeln!(&mut rendered, " :mode \"{}\"", summary.mode)?;
      writeln!(
        &mut rendered,
        " :path-or-url \"{}\"",
        escape_text(&summary.path_or_url)
      )?;
      if let Some(status) = summary.status {
        writeln!(&mut rendered, " :status {}", status)?;
      }
      writeln!(
        &mut rendered,
        " :error \"{}\"}}",
        escape_text(&summary.error)
      )?;
      eprint!("{rendered}");
    }
  }
  Ok(())
}

fn escape_text(value: &str) -> String {
  value
    .replace('\\', "\\\\")
    .replace('"', "\\\"")
    .replace('\n', "\\n")
}

pub(super) fn visit_url(
  url: &str,
) -> std::result::Result<GateAbsorbVisitResult, GateAbsorbVisitError> {
  if url.starts_with("file://") {
    return visit_file_url(url);
  }
  if url.starts_with("http://") || url.starts_with("https://") {
    return visit_http_url(url);
  }
  Err(GateAbsorbVisitError {
    status: None,
    message: format!("unsupported scheme: {}", url),
  })
}

fn visit_file_url(url: &str) -> std::result::Result<GateAbsorbVisitResult, GateAbsorbVisitError> {
  let parsed = Url::parse(url).map_err(|err| GateAbsorbVisitError {
    status: None,
    message: format!("invalid file url: {}", err),
  })?;
  let path = parsed.to_file_path().map_err(|_| GateAbsorbVisitError {
    status: None,
    message: "invalid file url".to_string(),
  })?;
  let bytes = fs::read(&path).map_err(|err| GateAbsorbVisitError {
    status: Some(404),
    message: format!("file not found: {} ({})", path.display(), err),
  })?;
  let content = String::from_utf8(bytes).map_err(|err| GateAbsorbVisitError {
    status: None,
    message: format!("invalid utf-8: {}", err),
  })?;
  Ok(GateAbsorbVisitResult {
    final_url: url.to_string(),
    content_type: if path
      .extension()
      .and_then(|ext| ext.to_str())
      .map(|ext| ext.eq_ignore_ascii_case("html"))
      .unwrap_or(false)
    {
      "text/html".to_string()
    } else {
      "application/octet-stream".to_string()
    },
    content_sha256: sha256_hex(content.as_bytes()),
    content,
  })
}

fn visit_http_url(url: &str) -> std::result::Result<GateAbsorbVisitResult, GateAbsorbVisitError> {
  let client = Client::builder()
    .timeout(Duration::from_millis(DEFAULT_TIMEOUT_MS))
    .redirect(Policy::none())
    .build()
    .map_err(|err| GateAbsorbVisitError {
      status: None,
      message: format!("build http client: {}", err),
    })?;
  let mut current = url.to_string();
  let mut hops = 0usize;
  loop {
    let response = client
      .get(&current)
      .header(USER_AGENT, DEFAULT_USER_AGENT)
      .send()
      .map_err(|err| GateAbsorbVisitError {
        status: None,
        message: format!("fetch failed: {}", err),
      })?;
    let status = response.status();
    if status.is_success() {
      let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("text/html")
        .to_string();
      let content = response.text().map_err(|err| GateAbsorbVisitError {
        status: Some(status.as_u16()),
        message: format!("read body: {}", err),
      })?;
      return Ok(GateAbsorbVisitResult {
        final_url: current,
        content_type,
        content_sha256: sha256_hex(content.as_bytes()),
        content,
      });
    }
    if status.is_redirection() {
      let Some(location) = response.headers().get(LOCATION) else {
        return Err(GateAbsorbVisitError {
          status: Some(status.as_u16()),
          message: "redirect without Location header".to_string(),
        });
      };
      if hops >= DEFAULT_MAX_REDIRECTS {
        return Err(GateAbsorbVisitError {
          status: Some(status.as_u16()),
          message: format!("exceeded max-redirects={}", DEFAULT_MAX_REDIRECTS),
        });
      }
      let location = location.to_str().map_err(|err| GateAbsorbVisitError {
        status: Some(status.as_u16()),
        message: format!("invalid redirect location: {}", err),
      })?;
      let base = Url::parse(&current).map_err(|err| GateAbsorbVisitError {
        status: Some(status.as_u16()),
        message: format!("invalid current url: {}", err),
      })?;
      current = base
        .join(location)
        .map_err(|err| GateAbsorbVisitError {
          status: Some(status.as_u16()),
          message: format!("resolve redirect: {}", err),
        })?
        .to_string();
      hops += 1;
      continue;
    }
    return Err(GateAbsorbVisitError {
      status: Some(status.as_u16()),
      message: format!("http error: {}", status.as_u16()),
    });
  }
}

fn transcript_looks_like_jsonl(path: &Path) -> Result<bool> {
  let file = fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
  let reader = BufReader::new(file);
  for line in reader.lines().take(12) {
    let line = line.with_context(|| format!("read {}", path.display()))?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
      continue;
    }
    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
      return Ok(false);
    };
    let Some(object) = value.as_object() else {
      return Ok(false);
    };
    return Ok(object.contains_key("type") && object.contains_key("payload"));
  }
  Ok(
    path
      .extension()
      .and_then(|ext| ext.to_str())
      .map(|ext| ext.eq_ignore_ascii_case("jsonl"))
      .unwrap_or(false),
  )
}

fn ingest_transcript_jsonl(
  path: &Path,
  dry_run: bool,
) -> Result<GateAbsorbConversationIngestSummary> {
  let cursor_path = gate_store_root()?.join("transcript-cursors.json");
  let mut cursors = read_transcript_cursors(&cursor_path)?;
  let path_key = path.display().to_string();
  let line_offset_before = cursors
    .get(&path_key)
    .map(|entry| entry.line_offset)
    .unwrap_or(0);
  let initial_session_id = cursors
    .get(&path_key)
    .map(|entry| entry.session_id.clone())
    .filter(|value| !value.trim().is_empty() && value != "unknown")
    .or_else(|| session_id_from_rollout_path(path));
  let file = fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
  let reader = BufReader::new(file);
  let mut ctx = TranscriptContext {
    session_id: initial_session_id.unwrap_or_else(|| "unknown".to_string()),
    ..TranscriptContext::default()
  };
  let mut pending_function_calls: HashMap<String, PendingCallRecord> = HashMap::new();
  let mut pending_custom_tool_calls: HashMap<String, PendingCallRecord> = HashMap::new();
  let mut seen = HashSet::new();
  let mut emitted = Vec::new();
  let mut lines_total = 0usize;
  let mut lines_new = 0usize;
  let mut matched = 0usize;
  for (index, line) in reader.lines().enumerate() {
    let line = line.with_context(|| format!("read {}", path.display()))?;
    lines_total += 1;
    if index < line_offset_before {
      advance_bootstrap_state_from_line(
        &line,
        &mut ctx,
        &mut pending_function_calls,
        &mut pending_custom_tool_calls,
      );
      continue;
    }
    lines_new += 1;
    if line.trim().is_empty() {
      continue;
    }
    let Some(entry) = parse_transcript_entry(&line) else {
      continue;
    };
    let old_pending_function_calls = pending_function_calls.clone();
    let old_pending_custom_tool_calls = pending_custom_tool_calls.clone();
    advance_transcript_pending_state(
      &entry,
      &mut ctx,
      &mut pending_function_calls,
      &mut pending_custom_tool_calls,
    );
    let drafts = build_candidate_drafts(
      &entry,
      &ctx,
      &old_pending_function_calls,
      &old_pending_custom_tool_calls,
    )?;
    for draft in drafts {
      if !seen.insert(draft.dedupe_key.clone()) {
        continue;
      }
      matched += 1;
      if dry_run {
        continue;
      }
      emitted.push(write_transcript_candidate(&draft.attrs)?);
    }
  }
  let line_offset_after = if dry_run {
    line_offset_before
  } else {
    lines_total
  };
  if !dry_run {
    cursors.insert(
      path_key.clone(),
      TranscriptCursorRecord {
        line_offset: line_offset_after,
        session_id: ctx.session_id.clone(),
        updated_at: unix_ms().to_string(),
      },
    );
    write_transcript_cursors(&cursor_path, &cursors)?;
  }
  Ok(GateAbsorbConversationIngestSummary {
    subcommand: "conversation",
    mode: if dry_run { "dry-run" } else { "emit" },
    path: path_key,
    lines_total,
    lines_new,
    matched,
    emitted,
    dry_run,
    reset: false,
    line_offset_before,
    line_offset_after,
    session_id: ctx.session_id,
    rule_set: TRANSCRIPT_RULE_SET.to_vec(),
  })
}

fn ingest_events_jsonl(
  path: &Path,
  limit: usize,
  dry_run: bool,
  reset: bool,
) -> Result<GateAbsorbEventsIngestSummary> {
  let cursor_path = gate_store_root()?.join("distill-cursor.json");
  let cursor_before = if reset {
    None
  } else {
    read_event_cursor(&cursor_path)?.map(|record| record.cursor)
  };
  let mut events = read_jsonl_values(path)?;
  let processed = events.len();
  let session_model_index = session_model_index(&events);
  if events.len() > limit {
    let start = events.len() - limit;
    events = events.split_off(start);
  }
  let fresh = if reset {
    events
  } else {
    events
      .into_iter()
      .filter(|event| after_event_cursor(cursor_before.as_deref(), event))
      .collect::<Vec<_>>()
  };
  let fresh_len = fresh.len();
  let mut seen = HashSet::new();
  let mut matched = 0usize;
  let mut emitted = Vec::new();
  for event in &fresh {
    let enriched = enrich_event_with_session_model(event, &session_model_index);
    for draft in build_event_candidate_drafts(&enriched)? {
      if !seen.insert(draft.dedupe_key.clone()) {
        continue;
      }
      matched += 1;
      if !dry_run {
        emitted.push(write_event_candidate(&draft.attrs)?);
      }
    }
  }
  let cursor_after = if dry_run || reset {
    cursor_before.clone()
  } else {
    max_event_recorded_at(&fresh).or(cursor_before.clone())
  };
  if !dry_run && !reset {
    if let Some(cursor) = cursor_after.clone() {
      write_event_cursor(
        &cursor_path,
        &EventCursorRecord {
          cursor,
          updated_at: unix_ms().to_string(),
        },
      )?;
    }
  }
  Ok(GateAbsorbEventsIngestSummary {
    subcommand: "events",
    mode: if dry_run { "dry-run" } else { "emit" },
    path: path.display().to_string(),
    processed,
    fresh: fresh_len,
    matched,
    emitted,
    dry_run,
    reset,
    cursor_before,
    cursor_after,
    rule_set: EVENT_RULE_SET.to_vec(),
  })
}

fn read_event_cursor(path: &Path) -> Result<Option<EventCursorRecord>> {
  if !path.exists() {
    return Ok(None);
  }
  let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
  let record = serde_json::from_str::<EventCursorRecord>(&raw)
    .with_context(|| format!("parse {}", path.display()))?;
  if record.cursor.trim().is_empty() {
    Ok(None)
  } else {
    Ok(Some(record))
  }
}

fn write_event_cursor(path: &Path, record: &EventCursorRecord) -> Result<()> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
  }
  fs::write(path, serde_json::to_string_pretty(record)?)
    .with_context(|| format!("write {}", path.display()))?;
  Ok(())
}

fn read_jsonl_values(path: &Path) -> Result<Vec<Value>> {
  let file = fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
  let reader = BufReader::new(file);
  let mut values = Vec::new();
  for line in reader.lines() {
    let line = line.with_context(|| format!("read {}", path.display()))?;
    if line.trim().is_empty() {
      continue;
    }
    if let Ok(value) = serde_json::from_str::<Value>(&line) {
      values.push(value);
    }
  }
  Ok(values)
}

fn session_model_index(events: &[Value]) -> HashMap<String, String> {
  let mut index = HashMap::new();
  for event in events {
    if event_field_string(event, &["event"]).as_deref() != Some("SessionStart") {
      continue;
    }
    if let (Some(session_id), Some(model)) = (
      event_field_string(event, &["session_id", "session-id"]),
      event_field_string(event, &["model", "model-name"]),
    ) {
      index.insert(session_id, model);
    }
  }
  index
}

fn enrich_event_with_session_model(event: &Value, models: &HashMap<String, String>) -> Value {
  let mut cloned = event.clone();
  let already_has_model = event_field_string(event, &["model", "model-name"]).is_some();
  if already_has_model {
    return cloned;
  }
  let event_name = event_field_string(event, &["event"]).unwrap_or_default();
  let allow_backfill = matches!(
    event_name.as_str(),
    "UserPromptSubmit" | "Stop" | "SessionContextClipped" | "LearnPromptInjected"
  );
  if !allow_backfill {
    return cloned;
  }
  let Some(session_id) = event_field_string(event, &["session_id", "session-id"]) else {
    return cloned;
  };
  let Some(model) = models.get(&session_id) else {
    return cloned;
  };
  if let Some(object) = cloned.as_object_mut() {
    object.insert("model".to_string(), Value::String(model.clone()));
  }
  cloned
}

fn after_event_cursor(cursor: Option<&str>, event: &Value) -> bool {
  match (
    cursor,
    event_field_string(event, &["recorded_at", "recorded-at"]),
  ) {
    (None, _) => true,
    (Some(_), None) => false,
    (Some(cursor), Some(recorded_at)) => recorded_at.as_str() > cursor,
  }
}

fn max_event_recorded_at(events: &[Value]) -> Option<String> {
  events
    .iter()
    .filter_map(|event| event_field_string(event, &["recorded_at", "recorded-at"]))
    .max()
}

fn build_event_candidate_drafts(event: &Value) -> Result<Vec<CandidateDraft>> {
  let event_name = event_field_string(event, &["event"]).unwrap_or_default();
  Ok(match event_name.as_str() {
    "PostToolUse" => {
      if let Some(tool_name) = event_field_string(event, &["tool_name", "tool-name"]) {
        match canonical_tool_name(&tool_name).as_str() {
          "Bash" => build_px_owned_event_candidate_drafts(event, "postToolBashCandidate")?,
          "Write" | "Edit" | "MultiEdit" => {
            build_px_owned_event_candidate_drafts(event, "postToolApplicationCandidate")?
          }
          _ => Vec::new(),
        }
      } else {
        Vec::new()
      }
    }
    "UserPromptSubmit" => build_px_owned_event_candidate_drafts(event, "userPromptCandidate")?,
    "SessionStart" => build_px_owned_event_candidate_drafts(event, "sessionStartCandidate")?,
    "Stop" => build_px_owned_event_candidate_drafts(event, "stopCandidate")?,
    "LearnPromptInjected" => {
      build_px_owned_event_candidate_drafts(event, "learnPromptInjectedCandidate")?
    }
    "SessionContextClipped" => {
      build_px_owned_event_candidate_drafts(event, "sessionContextClippedCandidate")?
    }
    "HookReturnPacketEmitted" => {
      build_px_owned_event_candidate_drafts(event, "hookReturnPacketCandidates")?
    }
    "ProviderHookDrift" => {
      build_px_owned_event_candidate_drafts(event, "providerHookDriftCandidate")?
    }
    _ => Vec::new(),
  })
}

fn build_post_tool_bash_owner_payload(event: &Value) -> Option<Map<String, Value>> {
  let mut payload = base_event_owner_payload(event)?;
  let command = event_field_string(event, &["command"]).unwrap_or_default();
  let (response_preview, response_length) =
    response_preview_fields(event_field_value(event, &["response"]));
  let tool_name =
    event_field_string(event, &["tool_name", "tool-name"]).unwrap_or_else(|| "Bash".to_string());
  payload.insert("tool_name".to_string(), Value::String(tool_name));
  if let Some(tool_use_id) = event_field_string(event, &["tool_use_id", "tool-use-id"]) {
    payload.insert("tool_use_id".to_string(), Value::String(tool_use_id));
  }
  payload.insert("command".to_string(), Value::String(command));
  payload.insert(
    "response_preview".to_string(),
    Value::String(response_preview),
  );
  payload.insert(
    "response_length".to_string(),
    json_u64(response_length as u64),
  );
  Some(payload)
}

fn build_post_tool_application_owner_payload(event: &Value) -> Option<Map<String, Value>> {
  let mut payload = base_event_owner_payload(event)?;
  let tool_name = event_field_string(event, &["tool_name", "tool-name"])?;
  let file_path = event_field_string(event, &["file_path", "file-path"]).unwrap_or_default();
  let target_px = event_field_bool(event, &["target_px", "target-px"]).unwrap_or(false);
  payload.insert("tool_name".to_string(), Value::String(tool_name));
  if let Some(tool_use_id) = event_field_string(event, &["tool_use_id", "tool-use-id"]) {
    payload.insert("tool_use_id".to_string(), Value::String(tool_use_id));
  }
  payload.insert("file_path".to_string(), Value::String(file_path));
  payload.insert("target_px".to_string(), Value::Bool(target_px));
  copy_string_alias(
    event,
    &mut payload,
    "content_preview",
    &["content_preview", "content-preview"],
  );
  copy_u64_alias(
    event,
    &mut payload,
    "content_length",
    &["content_length", "content-length"],
  );
  copy_string_alias(
    event,
    &mut payload,
    "new_string_preview",
    &["new_string_preview", "new-string-preview"],
  );
  copy_u64_alias(
    event,
    &mut payload,
    "new_string_length",
    &["new_string_length", "new-string-length"],
  );
  copy_u64_alias(
    event,
    &mut payload,
    "old_string_len",
    &["old_string_len", "old-string-len"],
  );
  copy_u64_alias(
    event,
    &mut payload,
    "edit_count",
    &["edit_count", "edit-count"],
  );
  Some(payload)
}

fn base_event_owner_payload(event: &Value) -> Option<Map<String, Value>> {
  let session_id = event_field_string(event, &["session_id", "session-id"])?;
  let mut payload = Map::new();
  payload.insert("session_id".to_string(), Value::String(session_id));
  if let Some(event_name) = event_field_string(event, &["event"]) {
    payload.insert("event_name".to_string(), Value::String(event_name));
  }
  if let Some(turn_id) = event_field_string(event, &["turn_id", "turn-id"]) {
    payload.insert("turn_id".to_string(), Value::String(turn_id));
  }
  if let Some(recorded_at) = event_field_string(event, &["recorded_at", "recorded-at"]) {
    payload.insert("recorded_at".to_string(), Value::String(recorded_at));
  }
  if let Some(model) = event_field_string(event, &["model", "model-name"]) {
    payload.insert("model".to_string(), Value::String(model));
  }
  Some(payload)
}

fn build_user_prompt_owner_payload(event: &Value) -> Option<Map<String, Value>> {
  let mut payload = base_event_owner_payload(event)?;
  let prompt_preview = event_field_string(
    event,
    &[
      "prompt_clipped",
      "prompt-clipped",
      "prompt_preview",
      "prompt-preview",
      "prompt",
    ],
  )
  .unwrap_or_default();
  let prompt_length = event_field_u64(event, &["prompt_length", "prompt-length"])
    .unwrap_or(prompt_preview.chars().count() as u64);
  payload.insert("prompt_preview".to_string(), Value::String(prompt_preview));
  payload.insert("prompt_length".to_string(), json_u64(prompt_length));
  Some(payload)
}

fn build_session_start_owner_payload(event: &Value) -> Option<Map<String, Value>> {
  let mut payload = base_event_owner_payload(event)?;
  let source = event_field_string(event, &["source"]).unwrap_or_else(|| "unknown".to_string());
  payload.insert("source".to_string(), Value::String(source));
  if let Some(cwd) = event_field_string(event, &["cwd", "cwd-path"]) {
    payload.insert("cwd".to_string(), Value::String(cwd));
  }
  Some(payload)
}

fn build_stop_owner_payload(event: &Value) -> Option<Map<String, Value>> {
  let mut payload = base_event_owner_payload(event)?;
  let assistant_preview = event_field_string(
    event,
    &[
      "last_assistant_preview",
      "last-assistant-preview",
      "last_assistant_message",
    ],
  )
  .unwrap_or_default();
  let assistant_length =
    event_field_u64(event, &["last_assistant_length", "last-assistant-length"])
      .unwrap_or(assistant_preview.chars().count() as u64);
  payload.insert(
    "assistant_preview".to_string(),
    Value::String(assistant_preview),
  );
  payload.insert("assistant_length".to_string(), json_u64(assistant_length));
  Some(payload)
}

fn build_learn_prompt_injected_owner_payload(event: &Value) -> Option<Map<String, Value>> {
  let mut payload = base_event_owner_payload(event)?;
  let mut object = Map::new();
  copy_string_alias(
    event,
    &mut object,
    "requested_prompt_surface",
    &["requested_prompt_surface", "requested-prompt-surface"],
  );
  copy_string_alias(
    event,
    &mut object,
    "prompt_surface",
    &["prompt_surface", "prompt-surface"],
  );
  copy_string_alias(
    event,
    &mut object,
    "fallback_reason",
    &["fallback_reason", "fallback-reason"],
  );
  copy_string_alias(
    event,
    &mut object,
    "warning_code",
    &["warning_code", "warning-code"],
  );
  copy_string_alias(
    event,
    &mut object,
    "warning_state",
    &["warning_state", "warning-state"],
  );
  copy_bool_alias(
    event,
    &mut object,
    "managed_session",
    &["managed_session", "managed-session"],
  );
  copy_bool_alias(
    event,
    &mut object,
    "auto_learning_enabled",
    &["auto_learning_enabled", "auto-learning-enabled"],
  );
  copy_bool_alias(
    event,
    &mut object,
    "learn_mode_enabled",
    &["learn_mode_enabled", "learn-mode-enabled"],
  );
  copy_string_alias(
    event,
    &mut object,
    "worktree_root",
    &["worktree_root", "worktree-root"],
  );
  payload.insert("object_attrs".to_string(), Value::Object(object));
  Some(payload)
}

fn build_session_context_clipped_owner_payload(event: &Value) -> Option<Map<String, Value>> {
  let mut payload = base_event_owner_payload(event)?;
  let mut object = Map::new();
  copy_string_alias(
    event,
    &mut object,
    "context_role",
    &["context_role", "context-role"],
  );
  copy_string_alias(
    event,
    &mut object,
    "output_surface",
    &["output_surface", "output-surface"],
  );
  copy_string_alias(
    event,
    &mut object,
    "budget_kind",
    &["budget_kind", "budget-kind"],
  );
  copy_string_alias(
    event,
    &mut object,
    "delivery_mode",
    &["delivery_mode", "delivery-mode"],
  );
  copy_string_alias(
    event,
    &mut object,
    "artifact_ref",
    &["artifact_ref", "artifact-ref"],
  );
  copy_string_alias(
    event,
    &mut object,
    "transcript_path",
    &["transcript_path", "transcript-path"],
  );
  copy_u64_alias(
    event,
    &mut object,
    "budget_bytes",
    &["budget_bytes", "budget-bytes"],
  );
  copy_u64_alias(
    event,
    &mut object,
    "original_bytes",
    &["original_bytes", "original-bytes"],
  );
  copy_u64_alias(
    event,
    &mut object,
    "delivered_bytes",
    &["delivered_bytes", "delivered-bytes"],
  );
  payload.insert("object_attrs".to_string(), Value::Object(object));
  Some(payload)
}

fn build_provider_hook_drift_owner_payload(event: &Value) -> Option<Map<String, Value>> {
  let mut payload = base_event_owner_payload(event)?;
  let provider = event_field_string(event, &["provider", "provider-name"])
    .unwrap_or_else(|| "unknown".to_string());
  let issue_kinds = event
    .get("issues")
    .and_then(Value::as_array)
    .map(|items| {
      items
        .iter()
        .flat_map(|item| {
          let kind = event_field_string(item, &["kind", "kind-name"])
            .unwrap_or_else(|| "unknown".to_string());
          let fields = event_field_string_vec(item, &["fields", "field_names", "field-names"]);
          if fields.is_empty() {
            vec![kind]
          } else {
            fields
              .into_iter()
              .map(|field| format!("{kind}:{field}"))
              .collect()
          }
        })
        .collect::<Vec<_>>()
    })
    .unwrap_or_default();
  let mut object = Map::new();
  copy_string_alias(event, &mut object, "phase", &["phase", "phase-name"]);
  copy_string_alias(
    event,
    &mut object,
    "expected_event",
    &["expected_event", "expected-event"],
  );
  copy_string_alias(
    event,
    &mut object,
    "actual_event",
    &["actual_event", "actual-event"],
  );
  copy_string_alias(event, &mut object, "tool_name", &["tool_name", "tool-name"]);
  copy_string_alias(
    event,
    &mut object,
    "tool_use_id",
    &["tool_use_id", "tool-use-id"],
  );
  payload.insert("provider".to_string(), Value::String(provider));
  payload.insert(
    "issue_kinds".to_string(),
    Value::Array(issue_kinds.into_iter().map(Value::String).collect()),
  );
  payload.insert("object_attrs".to_string(), Value::Object(object));
  Some(payload)
}

fn build_px_owned_event_candidate_drafts(
  event: &Value,
  owner_fn: &str,
) -> Result<Vec<CandidateDraft>> {
  let owner_payload = match owner_fn {
    "userPromptCandidate" => build_user_prompt_owner_payload(event),
    "sessionStartCandidate" => build_session_start_owner_payload(event),
    "stopCandidate" => build_stop_owner_payload(event),
    "learnPromptInjectedCandidate" => build_learn_prompt_injected_owner_payload(event),
    "sessionContextClippedCandidate" => build_session_context_clipped_owner_payload(event),
    "postToolBashCandidate" => build_post_tool_bash_owner_payload(event),
    "postToolApplicationCandidate" => build_post_tool_application_owner_payload(event),
    "hookReturnPacketCandidates" => build_hook_return_packet_owner_payload(event),
    "providerHookDriftCandidate" => build_provider_hook_drift_owner_payload(event),
    _ => None,
  };
  let Some(owner_payload) = owner_payload else {
    return Ok(Vec::new());
  };
  let owner_value = eval_gate_absorb_events_owner(owner_fn, &Value::Object(owner_payload))?;
  if owner_fn == "hookReturnPacketCandidates" {
    return parse_json_value(owner_value);
  }
  Ok(vec![parse_json_value(owner_value)?])
}

fn build_hook_return_packet_owner_payload(event: &Value) -> Option<Map<String, Value>> {
  let mut payload = base_event_owner_payload(event)?;
  copy_string_alias(
    event,
    &mut payload,
    "turn_scope",
    &["turn_scope", "turn-scope"],
  );
  copy_string_alias(
    event,
    &mut payload,
    "output_surface",
    &["output_surface", "output-surface"],
  );
  copy_string_alias(event, &mut payload, "target", &["target"]);
  copy_string_alias(
    event,
    &mut payload,
    "store_profile_kind",
    &["store_profile_kind", "store-profile-kind"],
  );
  copy_string_alias(
    event,
    &mut payload,
    "store_profile_source",
    &["store_profile_source", "store-profile-source"],
  );
  let packets = event
    .get("packets")
    .and_then(Value::as_array)
    .cloned()
    .unwrap_or_default();
  payload.insert("packets".to_string(), Value::Array(packets));
  let fingerprints = event_field_string_vec(event, &["packet_fingerprints", "packet-fingerprints"]);
  payload.insert(
    "packet_fingerprints".to_string(),
    Value::Array(fingerprints.into_iter().map(Value::String).collect()),
  );
  Some(payload)
}

fn write_event_candidate(attrs: &Map<String, Value>) -> Result<GateAbsorbEmittedCandidate> {
  let final_attrs = apply_event_write_floors(attrs.clone())?;
  let kind = "observation-atom";
  let content = emit_px_file(kind, &final_attrs, "distill-events")?;
  let gate_root = gate_store_root()?;
  let status = final_attrs
    .get("status")
    .and_then(Value::as_str)
    .unwrap_or("candidate")
    .to_string();
  let quarantine_reason = final_attrs
    .get("quarantine-reason")
    .and_then(Value::as_str)
    .map(ToOwned::to_owned);
  let relative_dir = candidate_relative_dir(&status, quarantine_reason.as_deref());
  let target_dir = gate_root.join(relative_dir);
  fs::create_dir_all(&target_dir).with_context(|| format!("create {}", target_dir.display()))?;
  let filename = format!(
    "{}-{}-{}.px",
    kind,
    unix_ms(),
    &sha256_hex(content.as_bytes())[..8]
  );
  let path = target_dir.join(&filename);
  fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
  Ok(GateAbsorbEmittedCandidate {
    filename,
    path: path.display().to_string(),
    status,
  })
}

fn apply_event_write_floors(attrs: Map<String, Value>) -> Result<Map<String, Value>> {
  let report: GateAbsorbFloorOwnerResult = parse_json_value(eval_gate_absorb_floor_owner(
    "eventWriteFloors",
    &Value::Object(Map::from_iter([(
      "attrs".to_string(),
      Value::Object(attrs),
    )])),
  )?)?;
  Ok(report.attrs)
}

fn canonical_tool_name(raw: &str) -> String {
  match raw {
    "Bash" | "run_shell_command" | "exec_command" => "Bash".to_string(),
    "Write" | "write_file" => "Write".to_string(),
    "Edit" | "replace" => "Edit".to_string(),
    "MultiEdit" => "MultiEdit".to_string(),
    other => other.to_string(),
  }
}

fn response_preview_fields(value: Option<&Value>) -> (String, usize) {
  match value {
    Some(Value::String(text)) => (clip_owned(text, 160), text.chars().count()),
    Some(Value::Object(map)) => {
      let text = first_present_string(&[map.get("text"), map.get("head"), map.get("preview")])
        .unwrap_or_default();
      let length = map
        .get("length")
        .and_then(Value::as_u64)
        .unwrap_or(text.chars().count() as u64);
      (clip_owned(&text, 160), length as usize)
    }
    Some(other) => {
      let text = serde_json::to_string(other).unwrap_or_default();
      (clip_owned(&text, 160), text.chars().count())
    }
    None => (String::new(), 0),
  }
}

fn event_field_value<'a>(value: &'a Value, fields: &[&str]) -> Option<&'a Value> {
  let object = value.as_object()?;
  for field in fields {
    if let Some(found) = object.get(*field) {
      return Some(found);
    }
  }
  None
}

fn event_field_string(value: &Value, fields: &[&str]) -> Option<String> {
  event_field_value(value, fields)
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(ToOwned::to_owned)
}

fn event_field_bool(value: &Value, fields: &[&str]) -> Option<bool> {
  event_field_value(value, fields).and_then(|value| match value {
    Value::Bool(raw) => Some(*raw),
    Value::String(raw) => match raw.trim().to_ascii_lowercase().as_str() {
      "1" | "true" | "yes" | "on" => Some(true),
      "0" | "false" | "no" | "off" => Some(false),
      _ => None,
    },
    _ => None,
  })
}

fn event_field_u64(value: &Value, fields: &[&str]) -> Option<u64> {
  event_field_value(value, fields).and_then(|value| match value {
    Value::Number(number) => number.as_u64(),
    Value::String(raw) => raw.trim().parse::<u64>().ok(),
    _ => None,
  })
}

fn event_field_string_vec(value: &Value, fields: &[&str]) -> Vec<String> {
  event_field_value(value, fields)
    .and_then(Value::as_array)
    .map(|items| {
      items
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

fn copy_string_alias(source: &Value, target: &mut Map<String, Value>, key: &str, fields: &[&str]) {
  if let Some(value) = event_field_string(source, fields) {
    target.insert(key.to_string(), Value::String(value));
  }
}

fn copy_bool_alias(source: &Value, target: &mut Map<String, Value>, key: &str, fields: &[&str]) {
  if let Some(value) = event_field_bool(source, fields) {
    target.insert(key.to_string(), Value::Bool(value));
  }
}

fn copy_u64_alias(source: &Value, target: &mut Map<String, Value>, key: &str, fields: &[&str]) {
  if let Some(value) = event_field_u64(source, fields) {
    target.insert(key.to_string(), json_u64(value));
  }
}

fn parse_transcript_entry(line: &str) -> Option<Value> {
  serde_json::from_str::<Value>(line).ok()
}

fn read_transcript_cursors(path: &Path) -> Result<HashMap<String, TranscriptCursorRecord>> {
  if !path.exists() {
    return Ok(HashMap::new());
  }
  let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
  serde_json::from_str::<HashMap<String, TranscriptCursorRecord>>(&raw)
    .with_context(|| format!("parse {}", path.display()))
}

fn write_transcript_cursors(
  path: &Path,
  cursors: &HashMap<String, TranscriptCursorRecord>,
) -> Result<()> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
  }
  fs::write(path, serde_json::to_string_pretty(cursors)?)
    .with_context(|| format!("write {}", path.display()))?;
  Ok(())
}

fn gate_store_root() -> Result<PathBuf> {
  if let Some(raw) = nonblank(env::var("PNIX_GATE_STORE_DIR").ok()) {
    return Ok(PathBuf::from(raw));
  }
  if let Some(raw) = nonblank(env::var("DOGHOUSE_RUNTIME_DIR").ok()) {
    return Ok(PathBuf::from(raw).join("pnix-gate"));
  }
  if let Some(raw) = nonblank(env::var("XDG_STATE_HOME").ok()) {
    return Ok(
      PathBuf::from(raw)
        .join("uppnix")
        .join("doghouse")
        .join("pnix-gate"),
    );
  }
  if let Some(home) = env::var_os("HOME") {
    let default_state = PathBuf::from(home)
      .join(".local")
      .join("state")
      .join("uppnix")
      .join("doghouse")
      .join("pnix-gate");
    if default_state.exists() {
      return Ok(default_state);
    }
  }
  Ok(workspace_root()?.join("pnix-gate").join(".store"))
}

fn workspace_root() -> Result<PathBuf> {
  let mut current = env::current_dir().context("read current directory")?;
  loop {
    if current.join("convergence.md").exists() && current.join("pnix-gate").exists() {
      return Ok(current);
    }
    if !current.pop() {
      break;
    }
  }
  bail!("could not resolve pnix workspace root from current directory");
}

fn nonblank(value: Option<String>) -> Option<String> {
  value.and_then(|raw| {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
      None
    } else {
      Some(trimmed.to_string())
    }
  })
}

fn eval_px_source_json(source: &str) -> Result<Value> {
  let json_text = px_eval_json::eval_px_source_to_json(source)?;
  serde_json::from_str(&json_text).context("parse pnix-query-runtime helper JSON")
}

fn emit_px_json_value(value: &Value) -> String {
  match value {
    Value::Null => "null".to_string(),
    Value::Bool(v) => v.to_string(),
    Value::Number(v) => v.to_string(),
    Value::String(v) => format!("{:?}", v),
    Value::Array(items) => format!(
      "[ {} ]",
      items
        .iter()
        .map(emit_px_json_value)
        .collect::<Vec<_>>()
        .join(" ")
    ),
    Value::Object(map) => {
      let mut rendered = String::from("{ ");
      for (key, value) in map {
        rendered.push_str(&emit_px_key(key));
        rendered.push_str(" = ");
        rendered.push_str(&emit_px_json_value(value));
        rendered.push_str("; ");
      }
      rendered.push('}');
      rendered
    }
  }
}

fn eval_gate_absorb_floor_owner(owner_fn: &str, payload: &Value) -> Result<Value> {
  let source = format!(
    "let\n  root = \"{}\";\n  owner = import (root + \"/stdlib/lib/gate/absorb-floors.px\");\n  payload = {};\nin owner.{} payload\n",
    workspace_root()?.display(),
    emit_px_json_value(payload),
    owner_fn
  );
  eval_px_source_json(&source)
}

fn eval_gate_absorb_url_owner(owner_fn: &str, payload: &Value) -> Result<Value> {
  let source = format!(
    "let\n  root = \"{}\";\n  owner = import (root + \"/stdlib/lib/gate/absorb-url.px\");\n  payload = {};\nin owner.{} payload\n",
    workspace_root()?.display(),
    emit_px_json_value(payload),
    owner_fn
  );
  eval_px_source_json(&source)
}

fn eval_gate_absorb_transcript_owner(owner_fn: &str, payload: &Value) -> Result<Value> {
  let source = format!(
    "let\n  root = \"{}\";\n  owner = import (root + \"/stdlib/lib/gate/absorb-transcript.px\");\n  payload = {};\nin owner.{} payload\n",
    workspace_root()?.display(),
    emit_px_json_value(payload),
    owner_fn
  );
  eval_px_source_json(&source)
}

fn eval_gate_absorb_events_owner(owner_fn: &str, payload: &Value) -> Result<Value> {
  let owner_file = match owner_fn {
    "postToolBashCandidate" | "postToolApplicationCandidate" => "absorb-events-post-tool.px",
    "hookReturnPacketCandidates" => "absorb-events-hook-return.px",
    _ => "absorb-events.px",
  };
  let source = format!(
    "let\n  root = \"{}\";\n  owner = import (root + \"/stdlib/lib/gate/{}\");\n  payload = {};\nin owner.{} payload\n",
    workspace_root()?.display(),
    owner_file,
    emit_px_json_value(payload),
    owner_fn
  );
  eval_px_source_json(&source)
}

fn parse_json_value<T: DeserializeOwned>(value: Value) -> Result<T> {
  serde_json::from_value(value).context("parse gate-absorb owner report")
}

fn unix_ms() -> u128 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_millis())
    .unwrap_or(0)
}

fn session_id_from_rollout_path(path: &Path) -> Option<String> {
  let filename = path.file_name()?.to_str()?;
  if !filename.starts_with("rollout-") || !filename.ends_with(".jsonl") {
    return None;
  }
  filename
    .strip_suffix(".jsonl")
    .and_then(|trimmed| trimmed.rsplit('-').next())
    .filter(|candidate| candidate.len() >= 8)
    .map(ToOwned::to_owned)
}

fn advance_bootstrap_state_from_line(
  line: &str,
  ctx: &mut TranscriptContext,
  pending_function_calls: &mut HashMap<String, PendingCallRecord>,
  pending_custom_tool_calls: &mut HashMap<String, PendingCallRecord>,
) {
  if line.trim().is_empty() {
    return;
  }
  let Some(entry) = parse_transcript_entry(line) else {
    return;
  };
  advance_transcript_pending_state(
    &entry,
    ctx,
    pending_function_calls,
    pending_custom_tool_calls,
  );
}

fn advance_transcript_pending_state(
  entry: &Value,
  ctx: &mut TranscriptContext,
  pending_function_calls: &mut HashMap<String, PendingCallRecord>,
  pending_custom_tool_calls: &mut HashMap<String, PendingCallRecord>,
) {
  update_context_from_entry(ctx, entry);
  if let Some(record) = tracked_function_call_record(entry, ctx) {
    pending_function_calls.insert(record.call_id.clone(), record);
  }
  if let Some(call_id) = function_call_output_call_id(entry) {
    pending_function_calls.remove(&call_id);
  }
  if let Some(record) = tracked_custom_tool_call_record(entry, ctx) {
    pending_custom_tool_calls.insert(record.call_id.clone(), record);
  }
  if let Some(call_id) = custom_tool_output_call_id(entry) {
    pending_custom_tool_calls.remove(&call_id);
  }
}

fn build_candidate_drafts(
  entry: &Value,
  ctx: &TranscriptContext,
  pending_function_calls: &HashMap<String, PendingCallRecord>,
  pending_custom_tool_calls: &HashMap<String, PendingCallRecord>,
) -> Result<Vec<CandidateDraft>> {
  let mut drafts = Vec::new();
  if let Some(draft) = build_response_message_candidate(entry, ctx) {
    drafts.push(draft);
  }
  if let Some(draft) = build_event_message_candidate(entry, ctx) {
    drafts.push(draft);
  }
  drafts.extend(build_function_call_candidates(entry, ctx)?);
  drafts.extend(build_custom_tool_call_candidates(entry, ctx)?);
  if let Some(call_id) = function_call_output_call_id(entry) {
    if let Some(pending) = pending_function_calls.get(&call_id) {
      drafts.push(build_paired_function_output_candidate(pending, entry, ctx)?);
    } else {
      drafts.push(build_orphan_output_candidate(
        "function_call_output",
        call_id,
        entry,
        ctx,
      )?);
    }
  }
  if let Some(call_id) = custom_tool_output_call_id(entry) {
    if let Some(pending) = pending_custom_tool_calls.get(&call_id) {
      drafts.extend(build_paired_custom_output_candidates(pending, entry, ctx)?);
    } else {
      drafts.push(build_orphan_output_candidate(
        "custom_tool_call_output",
        call_id,
        entry,
        ctx,
      )?);
    }
  }
  if let Some(draft) = build_web_search_candidate(entry, ctx) {
    drafts.push(draft);
  }
  Ok(drafts)
}

fn build_response_message_candidate(
  entry: &Value,
  ctx: &TranscriptContext,
) -> Option<CandidateDraft> {
  let payload = payload(entry)?;
  if payload.get("type")?.as_str()? != "message" {
    return None;
  }
  let role = payload
    .get("role")
    .and_then(Value::as_str)
    .unwrap_or("unknown");
  let phase = payload.get("phase").and_then(Value::as_str);
  let body = message_content_text(payload.get("content"))?;
  Some(build_provider_message_candidate(
    ctx,
    entry,
    "response_item",
    "message",
    role,
    phase,
    &body,
  ))
}

fn build_event_message_candidate(entry: &Value, ctx: &TranscriptContext) -> Option<CandidateDraft> {
  if entry.get("type")?.as_str()? != "event_msg" {
    return None;
  }
  let payload = payload(entry)?;
  let surface_type = payload.get("type")?.as_str()?;
  let role = match surface_type {
    "agent_message" => "assistant",
    "user_message" => "user",
    _ => return None,
  };
  let phase = payload.get("phase").and_then(Value::as_str);
  let body = payload
    .get("message")
    .and_then(Value::as_str)
    .unwrap_or_default();
  Some(build_provider_message_candidate(
    ctx,
    entry,
    "event_msg",
    surface_type,
    role,
    phase,
    body,
  ))
}

fn build_provider_message_candidate(
  ctx: &TranscriptContext,
  entry: &Value,
  provider_surface: &str,
  surface_type: &str,
  role: &str,
  phase: Option<&str>,
  body: &str,
) -> CandidateDraft {
  let packet = gate_observation_packet(
    ctx,
    entry,
    "provider-message",
    provider_surface,
    Some(surface_type),
    None,
    None,
    Some(role),
    phase,
    body,
  );
  let mut object = Map::new();
  object.insert(
    "provider_surface".to_string(),
    Value::String(provider_surface.to_string()),
  );
  object.insert("message_role".to_string(), Value::String(role.to_string()));
  object.insert(
    "message_length".to_string(),
    json_u64(body.chars().count() as u64),
  );
  object.insert(
    "message_preview".to_string(),
    Value::String(clip_owned(body, 160)),
  );
  if role == "assistant" {
    let mut assistant = Map::new();
    assistant.insert("role".to_string(), Value::String("assistant".to_string()));
    assistant.insert("preview".to_string(), Value::String(clip_owned(body, 160)));
    assistant.insert("length".to_string(), json_u64(body.chars().count() as u64));
    object.insert("assistant_response".to_string(), Value::Object(assistant));
  }
  let mut attrs = base_candidate_attrs(ctx);
  attrs.insert(
    "type".to_string(),
    Value::String("ObservationAtom".to_string()),
  );
  attrs.insert(
    "predicate".to_string(),
    Value::String("provider-message".to_string()),
  );
  attrs.insert("object".to_string(), Value::Object(object));
  attrs.insert(
    "granularity".to_string(),
    Value::String("provider-surface".to_string()),
  );
  attrs.insert("status".to_string(), Value::String("candidate".to_string()));
  attrs.insert(
    "gate_observation_packet".to_string(),
    Value::Object(packet.clone()),
  );
  let mut provenance = vec![
    format!("session:{}", ctx.session_id),
    "source:transcript".to_string(),
    "rule:transcript-provider-message-v0".to_string(),
    format!("surface:{}", provider_surface),
    format!("role:{}", role),
  ];
  if let Some(turn_id) = &ctx.turn_id {
    provenance.push(format!("turn:{}", turn_id));
  }
  if let Some(phase) = phase {
    provenance.push(format!("phase:{}", phase));
  }
  attrs.insert(
    "provenance".to_string(),
    Value::Array(provenance.into_iter().map(Value::String).collect()),
  );
  CandidateDraft {
    dedupe_key: format!(
      "provider-message|{}|{}|{}|{}|{}",
      packet
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("codex"),
      role,
      phase.unwrap_or(""),
      body_hash(body),
      ctx.turn_id.clone().unwrap_or_default()
    ),
    attrs,
  }
}

fn build_function_call_candidates(
  entry: &Value,
  ctx: &TranscriptContext,
) -> Result<Vec<CandidateDraft>> {
  let Some(payload) = payload(entry) else {
    return Ok(Vec::new());
  };
  if payload.get("type").and_then(Value::as_str) != Some("function_call") {
    return Ok(Vec::new());
  }
  let tool_name = payload
    .get("name")
    .and_then(Value::as_str)
    .unwrap_or("unknown")
    .to_string();
  if is_bash_transcript_tool(&tool_name) {
    return Ok(Vec::new());
  }
  let Some(call_id) = transcript_call_id(payload) else {
    return Ok(Vec::new());
  };
  let input = payload
    .get("arguments")
    .and_then(Value::as_str)
    .unwrap_or_default();
  let packet = gate_observation_packet(
    ctx,
    entry,
    "tool-call",
    "response_item",
    Some("function_call"),
    Some(&tool_name),
    Some(&call_id),
    None,
    None,
    input,
  );
  let mut owner_payload = Map::new();
  owner_payload.insert(
    "session_id".to_string(),
    Value::String(ctx.session_id.clone()),
  );
  if let Some(turn_id) = &ctx.turn_id {
    owner_payload.insert("turn_id".to_string(), Value::String(turn_id.clone()));
  }
  if let Some(model) = &ctx.model {
    owner_payload.insert("model".to_string(), Value::String(model.clone()));
  }
  owner_payload.insert("tool_name".to_string(), Value::String(tool_name));
  owner_payload.insert("call_id".to_string(), Value::String(call_id));
  owner_payload.insert("input_text".to_string(), Value::String(input.to_string()));
  owner_payload.insert("packet".to_string(), Value::Object(packet));
  owner_payload.insert(
    "parsed_input".to_string(),
    serde_json::to_value(apply_patch_input_parse(input, "function_call"))
      .context("serialize function tool input parse")?,
  );
  let mut drafts = vec![parse_json_value(eval_gate_absorb_transcript_owner(
    "functionToolCallCandidate",
    &Value::Object(owner_payload.clone()),
  )?)?];
  drafts.extend(parse_json_value::<Vec<CandidateDraft>>(
    eval_gate_absorb_transcript_owner(
      "patchSyntaxValidationCandidates",
      &Value::Object(owner_payload),
    )?,
  )?);
  Ok(drafts)
}

fn build_custom_tool_call_candidates(
  entry: &Value,
  ctx: &TranscriptContext,
) -> Result<Vec<CandidateDraft>> {
  let Some(payload) = payload(entry) else {
    return Ok(Vec::new());
  };
  if payload.get("type").and_then(Value::as_str) != Some("custom_tool_call") {
    return Ok(Vec::new());
  }
  let tool_name = payload
    .get("name")
    .and_then(Value::as_str)
    .unwrap_or("unknown")
    .to_string();
  let Some(call_id) = transcript_call_id(payload) else {
    return Ok(Vec::new());
  };
  let input = payload
    .get("input")
    .and_then(Value::as_str)
    .unwrap_or_default();
  let packet = gate_observation_packet(
    ctx,
    entry,
    "custom-tool-call",
    "response_item",
    Some("custom_tool_call"),
    Some(&tool_name),
    Some(&call_id),
    None,
    None,
    input,
  );
  let mut owner_payload = Map::new();
  owner_payload.insert(
    "session_id".to_string(),
    Value::String(ctx.session_id.clone()),
  );
  if let Some(turn_id) = &ctx.turn_id {
    owner_payload.insert("turn_id".to_string(), Value::String(turn_id.clone()));
  }
  if let Some(model) = &ctx.model {
    owner_payload.insert("model".to_string(), Value::String(model.clone()));
  }
  owner_payload.insert("tool_name".to_string(), Value::String(tool_name));
  owner_payload.insert("call_id".to_string(), Value::String(call_id));
  owner_payload.insert("input_text".to_string(), Value::String(input.to_string()));
  owner_payload.insert("packet".to_string(), Value::Object(packet));
  owner_payload.insert(
    "parsed_input".to_string(),
    serde_json::to_value(apply_patch_input_parse(input, "custom_tool_call"))
      .context("serialize custom tool input parse")?,
  );
  let mut drafts = vec![parse_json_value(eval_gate_absorb_transcript_owner(
    "customToolCallCandidate",
    &Value::Object(owner_payload.clone()),
  )?)?];
  drafts.extend(parse_json_value::<Vec<CandidateDraft>>(
    eval_gate_absorb_transcript_owner(
      "patchSyntaxValidationCandidates",
      &Value::Object(owner_payload),
    )?,
  )?);
  Ok(drafts)
}

fn build_paired_function_output_candidate(
  pending: &PendingCallRecord,
  entry: &Value,
  output_ctx: &TranscriptContext,
) -> Result<CandidateDraft> {
  let output_payload = payload(entry).expect("payload");
  let raw_output = output_payload
    .get("output")
    .and_then(Value::as_str)
    .unwrap_or_default();
  let parsed_output = function_call_output_parse(raw_output);
  let packet = gate_observation_packet(
    output_ctx,
    entry,
    "tool-call-output",
    "response_item",
    Some("function_call_output"),
    Some(&pending.tool_name),
    Some(&pending.call_id),
    None,
    None,
    raw_output,
  );
  let subject_ctx = if pending.ctx.session_id.is_empty() {
    output_ctx
  } else {
    &pending.ctx
  };
  let mut owner_payload = Map::new();
  owner_payload.insert(
    "session_id".to_string(),
    Value::String(subject_ctx.session_id.clone()),
  );
  if let Some(turn_id) = subject_ctx.turn_id.as_ref().or(output_ctx.turn_id.as_ref()) {
    owner_payload.insert("turn_id".to_string(), Value::String(turn_id.clone()));
  }
  if let Some(model) = subject_ctx.model.as_ref().or(output_ctx.model.as_ref()) {
    owner_payload.insert("model".to_string(), Value::String(model.clone()));
  }
  owner_payload.insert(
    "tool_name".to_string(),
    Value::String(pending.tool_name.clone()),
  );
  owner_payload.insert(
    "call_id".to_string(),
    Value::String(pending.call_id.clone()),
  );
  owner_payload.insert(
    "input_text".to_string(),
    Value::String(pending.input_text.clone()),
  );
  owner_payload.insert("packet".to_string(), Value::Object(packet));
  owner_payload.insert(
    "parsed_output".to_string(),
    serde_json::to_value(parsed_output).context("serialize function tool output parse")?,
  );
  parse_json_value(eval_gate_absorb_transcript_owner(
    "functionToolOutputCandidate",
    &Value::Object(owner_payload),
  )?)
}

fn build_paired_custom_output_candidates(
  pending: &PendingCallRecord,
  entry: &Value,
  output_ctx: &TranscriptContext,
) -> Result<Vec<CandidateDraft>> {
  let output_payload = payload(entry).expect("payload");
  let raw_output = output_payload
    .get("output")
    .and_then(Value::as_str)
    .unwrap_or_default();
  let parsed_output = custom_tool_output_parse(raw_output);
  let parsed_input = apply_patch_input_parse(&pending.input_text, "custom_tool_call");
  let packet = gate_observation_packet(
    output_ctx,
    entry,
    "custom-tool-output",
    "response_item",
    Some("custom_tool_call_output"),
    Some(&pending.tool_name),
    Some(&pending.call_id),
    None,
    None,
    raw_output,
  );
  let subject_ctx = if pending.ctx.session_id.is_empty() {
    output_ctx
  } else {
    &pending.ctx
  };
  let mut owner_payload = Map::new();
  owner_payload.insert(
    "session_id".to_string(),
    Value::String(subject_ctx.session_id.clone()),
  );
  if let Some(turn_id) = subject_ctx.turn_id.as_ref().or(output_ctx.turn_id.as_ref()) {
    owner_payload.insert("turn_id".to_string(), Value::String(turn_id.clone()));
  }
  if let Some(model) = subject_ctx.model.as_ref().or(output_ctx.model.as_ref()) {
    owner_payload.insert("model".to_string(), Value::String(model.clone()));
  }
  owner_payload.insert(
    "tool_name".to_string(),
    Value::String(pending.tool_name.clone()),
  );
  owner_payload.insert(
    "call_id".to_string(),
    Value::String(pending.call_id.clone()),
  );
  owner_payload.insert(
    "input_text".to_string(),
    Value::String(pending.input_text.clone()),
  );
  owner_payload.insert("packet".to_string(), Value::Object(packet));
  owner_payload.insert(
    "parsed_output".to_string(),
    serde_json::to_value(&parsed_output).context("serialize custom tool output parse")?,
  );
  let mut drafts = vec![parse_json_value(eval_gate_absorb_transcript_owner(
    "customToolOutputCandidate",
    &Value::Object(owner_payload.clone()),
  )?)?];

  if tool_name_candidates(&pending.tool_name)
    .iter()
    .any(|candidate| candidate == "apply_patch")
  {
    let final_files =
      apply_patch_final_file_proofs(&parsed_input, &parsed_output, subject_ctx, output_ctx);
    owner_payload.insert(
      "parsed_input".to_string(),
      serde_json::to_value(parsed_input).context("serialize apply_patch input parse")?,
    );
    owner_payload.insert(
      "final_files".to_string(),
      serde_json::to_value(final_files).context("serialize apply_patch final file proofs")?,
    );
    drafts.extend(parse_json_value::<Vec<CandidateDraft>>(
      eval_gate_absorb_transcript_owner(
        "patchSyntaxApplyResultCandidates",
        &Value::Object(owner_payload),
      )?,
    )?);
  }

  Ok(drafts)
}

fn build_orphan_output_candidate(
  surface: &str,
  call_id: String,
  entry: &Value,
  ctx: &TranscriptContext,
) -> Result<CandidateDraft> {
  let raw_output = payload(entry)
    .and_then(|payload| payload.get("output"))
    .and_then(Value::as_str)
    .unwrap_or_default();
  let parsed_output = if surface == "function_call_output" {
    function_call_output_parse(raw_output)
  } else {
    custom_tool_output_parse(raw_output)
  };
  let packet = gate_observation_packet(
    ctx,
    entry,
    "unpaired-tool-output",
    "response_item",
    Some(surface),
    Some("unknown"),
    Some(&call_id),
    None,
    None,
    raw_output,
  );
  let mut owner_payload = Map::new();
  owner_payload.insert(
    "session_id".to_string(),
    Value::String(ctx.session_id.clone()),
  );
  if let Some(turn_id) = &ctx.turn_id {
    owner_payload.insert("turn_id".to_string(), Value::String(turn_id.clone()));
  }
  if let Some(model) = &ctx.model {
    owner_payload.insert("model".to_string(), Value::String(model.clone()));
  }
  owner_payload.insert("surface".to_string(), Value::String(surface.to_string()));
  owner_payload.insert("call_id".to_string(), Value::String(call_id));
  owner_payload.insert("packet".to_string(), Value::Object(packet));
  owner_payload.insert(
    "parsed_output".to_string(),
    serde_json::to_value(parsed_output).context("serialize orphan tool output parse")?,
  );
  parse_json_value(eval_gate_absorb_transcript_owner(
    "orphanToolOutputCandidate",
    &Value::Object(owner_payload),
  )?)
}

fn build_web_search_candidate(entry: &Value, ctx: &TranscriptContext) -> Option<CandidateDraft> {
  let payload = payload(entry)?;
  if payload.get("type")?.as_str()? != "web_search_call" {
    return None;
  }
  let queries = payload
    .get("action")
    .and_then(Value::as_object)
    .map(|action| {
      if let Some(items) = action.get("queries").and_then(Value::as_array) {
        items
          .iter()
          .filter_map(Value::as_str)
          .map(ToOwned::to_owned)
          .collect::<Vec<_>>()
      } else {
        action
          .get("query")
          .and_then(Value::as_str)
          .map(|query| vec![query.to_string()])
          .unwrap_or_default()
      }
    })
    .unwrap_or_default();
  let primary = queries.first().cloned().unwrap_or_default();
  let packet = gate_observation_packet(
    ctx,
    entry,
    "web-search",
    "response_item",
    Some("web_search_call"),
    None,
    None,
    None,
    None,
    &queries.join("\n"),
  );
  let mut attrs = base_candidate_attrs(ctx);
  attrs.insert(
    "type".to_string(),
    Value::String("ObservationAtom".to_string()),
  );
  attrs.insert(
    "predicate".to_string(),
    Value::String("web-searched".to_string()),
  );
  attrs.insert(
    "object".to_string(),
    Value::String(clip_owned(&primary, 200)),
  );
  attrs.insert(
    "granularity".to_string(),
    Value::String("web-search".to_string()),
  );
  attrs.insert("status".to_string(), Value::String("candidate".to_string()));
  attrs.insert("gate_observation_packet".to_string(), Value::Object(packet));
  let mut provenance = vec![
    format!("session:{}", ctx.session_id),
    "source:transcript".to_string(),
    "rule:transcript-web-search-v0".to_string(),
    format!("query_count:{}", queries.len()),
  ];
  if let Some(turn_id) = &ctx.turn_id {
    provenance.push(format!("turn:{}", turn_id));
  }
  attrs.insert(
    "provenance".to_string(),
    Value::Array(provenance.into_iter().map(Value::String).collect()),
  );
  Some(CandidateDraft {
    dedupe_key: format!(
      "web-search|{}|{}",
      ctx.session_id,
      body_hash(&queries.join("|"))
    ),
    attrs,
  })
}

fn base_candidate_attrs(ctx: &TranscriptContext) -> Map<String, Value> {
  let mut attrs = Map::new();
  attrs.insert(
    "subject".to_string(),
    Value::String(format!("session:{}", ctx.session_id)),
  );
  if let Some(model) = &ctx.model {
    attrs.insert("model".to_string(), Value::String(model.clone()));
  }
  attrs
}

fn update_context_from_entry(ctx: &mut TranscriptContext, entry: &Value) {
  let Some(kind) = entry.get("type").and_then(Value::as_str) else {
    return;
  };
  let Some(payload) = payload(entry) else {
    return;
  };
  match kind {
    "session_meta" => {
      if let Some(session_id) = first_present_string(&[
        payload.get("id"),
        payload.get("session_id"),
        payload.get("session-id"),
        payload.get("sessionId"),
      ]) {
        ctx.session_id = session_id;
      }
      if let Some(cwd) = first_present_string(&[
        payload.get("cwd"),
        payload.get("worktree_root"),
        payload.get("worktree-root"),
      ]) {
        ctx.cwd = Some(cwd);
      }
      if let Some(model) = payload
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
      {
        ctx.model = Some(model.to_string());
      }
      if let Some(model_provider) = first_present_string(&[
        payload.get("model_provider"),
        payload.get("model-provider"),
        payload.get("modelProvider"),
        payload.get("provider"),
      ]) {
        ctx.model_provider = Some(model_provider);
      }
    }
    "turn_context" => {
      if let Some(turn_id) = first_present_string(&[
        payload.get("turn_id"),
        payload.get("turn-id"),
        payload.get("turnId"),
      ]) {
        ctx.turn_id = Some(turn_id);
      }
      if let Some(cwd) = first_present_string(&[
        payload.get("cwd"),
        payload.get("worktree_root"),
        payload.get("worktree-root"),
      ]) {
        ctx.cwd = Some(cwd);
      }
      if let Some(model) = payload
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
      {
        ctx.model = Some(model.to_string());
      }
    }
    _ => {}
  }
}

fn tracked_function_call_record(
  entry: &Value,
  ctx: &TranscriptContext,
) -> Option<PendingCallRecord> {
  let payload = payload(entry)?;
  if payload.get("type")?.as_str()? != "function_call" {
    return None;
  }
  let call_id = transcript_call_id(payload)?;
  let tool_name = payload
    .get("name")
    .and_then(Value::as_str)
    .unwrap_or("unknown")
    .to_string();
  if is_bash_transcript_tool(&tool_name) {
    return None;
  }
  let input_text = payload
    .get("arguments")
    .and_then(Value::as_str)
    .unwrap_or_default()
    .to_string();
  Some(PendingCallRecord {
    call_id,
    tool_name,
    input_text,
    ctx: ctx.clone(),
  })
}

fn tracked_custom_tool_call_record(
  entry: &Value,
  ctx: &TranscriptContext,
) -> Option<PendingCallRecord> {
  let payload = payload(entry)?;
  if payload.get("type")?.as_str()? != "custom_tool_call" {
    return None;
  }
  let call_id = transcript_call_id(payload)?;
  let tool_name = payload
    .get("name")
    .and_then(Value::as_str)
    .unwrap_or("unknown")
    .to_string();
  let input_text = payload
    .get("input")
    .and_then(Value::as_str)
    .unwrap_or_default()
    .to_string();
  Some(PendingCallRecord {
    call_id,
    tool_name,
    input_text,
    ctx: ctx.clone(),
  })
}

fn function_call_output_call_id(entry: &Value) -> Option<String> {
  let payload = payload(entry)?;
  if payload.get("type")?.as_str()? != "function_call_output" {
    return None;
  }
  transcript_call_id(payload)
}

fn custom_tool_output_call_id(entry: &Value) -> Option<String> {
  let payload = payload(entry)?;
  if payload.get("type")?.as_str()? != "custom_tool_call_output" {
    return None;
  }
  transcript_call_id(payload)
}

fn payload(entry: &Value) -> Option<&Map<String, Value>> {
  entry.get("payload")?.as_object()
}

fn transcript_call_id(payload: &Map<String, Value>) -> Option<String> {
  first_present_string(&[payload.get("call_id"), payload.get("tool_use_id")])
}

fn first_present_string(values: &[Option<&Value>]) -> Option<String> {
  values
    .iter()
    .filter_map(|value| value.and_then(Value::as_str))
    .map(str::trim)
    .find(|value| !value.is_empty())
    .map(ToOwned::to_owned)
}

fn message_content_text(content: Option<&Value>) -> Option<String> {
  let content = content?;
  if let Some(text) = content.as_str() {
    return Some(text.to_string());
  }
  let items = content.as_array()?;
  let mut lines = Vec::new();
  for item in items {
    if let Some(text) = item.as_str() {
      let trimmed = text.trim();
      if !trimmed.is_empty() {
        lines.push(trimmed.to_string());
      }
      continue;
    }
    if let Some(object) = item.as_object() {
      if let Some(text) = object.get("text").and_then(Value::as_str) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
          lines.push(trimmed.to_string());
        }
      }
    }
  }
  if lines.is_empty() {
    None
  } else {
    Some(lines.join("\n"))
  }
}

fn is_bash_transcript_tool(tool_name: &str) -> bool {
  tool_name_candidates(tool_name)
    .iter()
    .any(|candidate| candidate == "exec_command")
}

fn tool_name_candidates(tool_name: &str) -> Vec<String> {
  let raw = tool_name.trim();
  if raw.is_empty() {
    return Vec::new();
  }
  let mut candidates = vec![raw.to_string()];
  if let Some(last) = raw.rsplit('.').next() {
    if last != raw {
      candidates.push(last.to_string());
    }
  }
  candidates
}

fn gate_observation_packet(
  ctx: &TranscriptContext,
  entry: &Value,
  event_kind: &str,
  provider_surface: &str,
  surface_type: Option<&str>,
  tool_name: Option<&str>,
  call_id: Option<&str>,
  message_role: Option<&str>,
  phase: Option<&str>,
  raw_content: &str,
) -> Map<String, Value> {
  let mut packet = Map::new();
  packet.insert("provider".to_string(), Value::String("codex".to_string()));
  packet.insert(
    "session_id".to_string(),
    Value::String(ctx.session_id.clone()),
  );
  packet.insert(
    "event_kind".to_string(),
    Value::String(event_kind.to_string()),
  );
  if let Some(timestamp) = entry.get("timestamp").and_then(Value::as_str) {
    packet.insert(
      "recorded_at".to_string(),
      Value::String(timestamp.to_string()),
    );
  }
  packet.insert(
    "content_hash".to_string(),
    Value::String(body_hash(raw_content)),
  );
  packet.insert(
    "provider_surface".to_string(),
    Value::String(provider_surface.to_string()),
  );
  packet.insert(
    "truth_regime".to_string(),
    Value::String("interpretive".to_string()),
  );
  packet.insert("direct_truth_source".to_string(), Value::Bool(false));
  if let Some(turn_id) = &ctx.turn_id {
    packet.insert("turn_id".to_string(), Value::String(turn_id.clone()));
  }
  if let Some(model) = &ctx.model {
    packet.insert("model".to_string(), Value::String(model.clone()));
  }
  if let Some(model_provider) = &ctx.model_provider {
    packet.insert(
      "model_provider".to_string(),
      Value::String(model_provider.clone()),
    );
  }
  if let Some(tool_name) = tool_name {
    packet.insert(
      "tool_name".to_string(),
      Value::String(tool_name.to_string()),
    );
  }
  if let Some(call_id) = call_id {
    packet.insert(
      "tool_call_id".to_string(),
      Value::String(call_id.to_string()),
    );
  }
  if let Some(surface_type) = surface_type {
    packet.insert(
      "surface_type".to_string(),
      Value::String(surface_type.to_string()),
    );
  }
  if let Some(message_role) = message_role {
    packet.insert(
      "message_role".to_string(),
      Value::String(message_role.to_string()),
    );
  }
  if let Some(phase) = phase {
    packet.insert("phase".to_string(), Value::String(phase.to_string()));
  }
  if let Some(cwd) = &ctx.cwd {
    packet.insert("cwd".to_string(), Value::String(cwd.clone()));
  }
  packet
}

fn body_hash(body: &str) -> String {
  sha256_hex(body.as_bytes())
}

fn clip_owned(text: &str, limit: usize) -> String {
  if text.chars().count() <= limit {
    return text.to_string();
  }
  let mut output = String::new();
  for (index, ch) in text.chars().enumerate() {
    if index >= limit {
      break;
    }
    output.push(ch);
  }
  output.push('…');
  output
}

fn function_call_output_parse(raw_output: &str) -> TranscriptToolOutputParse {
  let command = line_prefix_value(raw_output, "Command:");
  let chunk_id = line_prefix_value(raw_output, "Chunk ID:");
  let wall_time_seconds = line_prefix_value(raw_output, "Wall time:")
    .and_then(|raw| {
      raw
        .strip_suffix("seconds")
        .map(str::trim)
        .map(ToOwned::to_owned)
    })
    .and_then(|raw| raw.parse::<f64>().ok());
  let exit_code = line_prefix_value(raw_output, "Process exited with code")
    .and_then(|raw| raw.parse::<i64>().ok());
  let running_session_id = line_prefix_value(raw_output, "Process running with session ID");
  let original_token_count =
    line_prefix_value(raw_output, "Original token count:").and_then(|raw| raw.parse::<u64>().ok());
  let response_body =
    text_after_marker(raw_output, "\nOutput:\n").unwrap_or_else(|| raw_output.to_string());
  TranscriptToolOutputParse {
    surface: "function_call_output".to_string(),
    provider_command: command,
    chunk_id,
    wall_time_seconds,
    exit_code,
    running_session_id,
    original_token_count,
    response_preview: clip_owned(&response_body, 160),
    response_length: raw_output.chars().count() as u64,
    duration_seconds: None,
    error_message: None,
    updated_paths: Vec::new(),
    change_kinds: Vec::new(),
  }
}

fn custom_tool_output_parse(raw_output: &str) -> TranscriptToolOutputParse {
  let parsed = serde_json::from_str::<Value>(raw_output).ok();
  let metadata = parsed
    .as_ref()
    .and_then(|value| value.get("metadata"))
    .and_then(Value::as_object);
  let output_body = parsed
    .as_ref()
    .and_then(|value| value.get("output"))
    .and_then(Value::as_str)
    .unwrap_or(raw_output)
    .to_string();
  let exit_code = metadata
    .and_then(|meta| meta.get("exit_code"))
    .and_then(value_to_i64);
  let duration_seconds = metadata
    .and_then(|meta| meta.get("duration_seconds"))
    .and_then(value_to_f64);
  let error_message = parsed
    .as_ref()
    .and_then(|value| value.get("error"))
    .and_then(Value::as_str)
    .map(ToOwned::to_owned);
  let summary = updated_file_summary(&output_body);
  TranscriptToolOutputParse {
    surface: "custom_tool_call_output".to_string(),
    provider_command: None,
    chunk_id: None,
    wall_time_seconds: None,
    exit_code,
    running_session_id: None,
    original_token_count: None,
    response_preview: clip_owned(&output_body, 160),
    response_length: raw_output.chars().count() as u64,
    duration_seconds,
    error_message,
    updated_paths: summary.updated_paths,
    change_kinds: summary.change_kinds,
  }
}

fn apply_patch_input_parse(raw_input: &str, surface: &str) -> ApplyPatchInputParse {
  fn patch_operation_header(line: &str) -> Option<(&'static str, &str)> {
    if let Some(path) = line.strip_prefix("*** Add File:") {
      Some(("add", path.trim()))
    } else if let Some(path) = line.strip_prefix("*** Update File:") {
      Some(("update", path.trim()))
    } else if let Some(path) = line.strip_prefix("*** Delete File:") {
      Some(("delete", path.trim()))
    } else {
      None
    }
  }

  let lines = raw_input
    .lines()
    .map(|line| line.trim_end_matches('\r'))
    .collect::<Vec<_>>();
  let has_begin_patch = lines.iter().any(|line| *line == "*** Begin Patch");
  let has_end_patch = lines.iter().any(|line| *line == "*** End Patch");
  let mut ops = Vec::new();
  let mut add_ops = Vec::new();
  let mut index = 0usize;
  while index < lines.len() {
    let line = lines[index];
    let Some((operation, path)) = patch_operation_header(line).filter(|(_, path)| !path.is_empty())
    else {
      index += 1;
      continue;
    };
    index += 1;
    let mut move_to_path = None;
    if operation == "update" && index < lines.len() {
      if let Some(candidate) = lines[index]
        .strip_prefix("*** Move to:")
        .map(str::trim)
        .filter(|value| !value.is_empty())
      {
        move_to_path = Some(candidate.to_string());
        index += 1;
      }
    }
    let mut body_lines = Vec::new();
    let mut body_prefix_error_total = 0u64;
    while index < lines.len() {
      let current = lines[index];
      if patch_operation_header(current).is_some() || current == "*** End Patch" {
        break;
      }
      if current == "\\ No newline at end of file" {
        index += 1;
        continue;
      }
      if operation == "add" {
        if let Some(rest) = current.strip_prefix('+') {
          body_lines.push(rest.to_string());
        } else {
          body_prefix_error_total += 1;
        }
      } else {
        body_lines.push(current.to_string());
      }
      index += 1;
    }
    let body_text = body_lines.join("\n");
    let is_px_path = path.ends_with(".px");
    let move_to_is_px_path = move_to_path
      .as_ref()
      .map(|candidate| candidate.ends_with(".px"))
      .unwrap_or(false);
    let parse_error = if operation == "add" && is_px_path {
      parse_expr(&body_text).err().map(|err| err.to_string())
    } else {
      None
    };
    let op = ApplyPatchOpParse {
      operation: operation.to_string(),
      path: path.to_string(),
      is_px_path,
      move_to_path,
      move_to_is_px_path,
      body_bytes: body_text.as_bytes().len() as u64,
      body_text,
      body_prefix_error_total,
      parse_error,
    };
    if operation == "add" {
      add_ops.push(op.clone());
    }
    ops.push(op);
  }
  let px_op_total = ops
    .iter()
    .filter(|op| op.is_px_path || op.move_to_is_px_path)
    .count() as u64;
  let px_add_op_total = add_ops.iter().filter(|op| op.is_px_path).count() as u64;
  ApplyPatchInputParse {
    surface: surface.to_string(),
    has_begin_patch,
    has_end_patch,
    op_total: ops.len() as u64,
    px_op_total,
    add_op_total: add_ops.len() as u64,
    px_add_op_total,
    ops,
    add_ops,
  }
}

fn apply_patch_final_file_proofs(
  parsed_input: &ApplyPatchInputParse,
  parsed_output: &TranscriptToolOutputParse,
  subject_ctx: &TranscriptContext,
  output_ctx: &TranscriptContext,
) -> Vec<ApplyPatchFinalFileProof> {
  let cwd = subject_ctx.cwd.as_deref().or(output_ctx.cwd.as_deref());
  parsed_input
    .ops
    .iter()
    .filter(|op| op.is_px_path || op.move_to_is_px_path)
    .filter_map(|op| {
      let operation_kind = apply_patch_operation_kind(op);
      if operation_kind == "add" {
        return None;
      }
      let path = apply_patch_final_target_path(op, operation_kind)?;
      resolve_workspace_path_for_proof(cwd, path).0.as_ref()?;
      let apply_result_path_seen =
        apply_result_mentions_path(cwd, &parsed_output.updated_paths, path);
      let (
        final_read_status,
        final_file_present,
        final_parse_status,
        final_parse_error,
        final_bytes,
      ) = if apply_result_path_seen {
        read_final_px_parse_fact(cwd, path)
      } else {
        (
          "not-read-missing-apply-result-path".to_string(),
          false,
          "not-available".to_string(),
          None,
          None,
        )
      };
      let (source_read_status, source_file_present) = if operation_kind == "move" {
        let (status, present, _, _, _) = read_final_px_parse_fact(cwd, &op.path);
        (Some(status), Some(present))
      } else {
        (None, None)
      };
      Some(ApplyPatchFinalFileProof {
        operation: op.operation.clone(),
        operation_kind: operation_kind.to_string(),
        path: path.to_string(),
        source_path: if operation_kind == "move" {
          Some(op.path.clone())
        } else {
          None
        },
        move_to_path: op.move_to_path.clone(),
        apply_result_path_seen,
        apply_result_change_kinds: parsed_output.change_kinds.clone(),
        final_read_status,
        final_file_present,
        final_parse_status,
        final_parse_error,
        final_bytes,
        source_read_status,
        source_file_present,
        proof_source: "custom_tool_call_output+current-worktree".to_string(),
        proof_boundary: "apply-result-lineage-not-promotion".to_string(),
      })
    })
    .collect()
}

fn apply_patch_operation_kind(op: &ApplyPatchOpParse) -> &'static str {
  if op.operation == "update" && op.move_to_path.is_some() {
    "move"
  } else if op.operation == "update" {
    "update"
  } else if op.operation == "delete" {
    "delete"
  } else {
    "add"
  }
}

fn apply_patch_final_target_path<'a>(
  op: &'a ApplyPatchOpParse,
  operation_kind: &str,
) -> Option<&'a str> {
  if operation_kind == "move" {
    op.move_to_path.as_deref()
  } else {
    Some(op.path.as_str())
  }
}

fn apply_result_mentions_path(
  cwd: Option<&str>,
  updated_paths: &[String],
  target_path: &str,
) -> bool {
  if updated_paths.is_empty() {
    return false;
  }
  let target_resolved = resolve_workspace_path_for_proof(cwd, target_path).0;
  updated_paths.iter().any(|candidate| {
    if candidate == target_path {
      return true;
    }
    let candidate_resolved = resolve_workspace_path_for_proof(cwd, candidate).0;
    match (target_resolved.as_ref(), candidate_resolved.as_ref()) {
      (Some(target), Some(candidate)) => target == candidate,
      _ => false,
    }
  })
}

fn read_final_px_parse_fact(
  cwd: Option<&str>,
  path: &str,
) -> (String, bool, String, Option<String>, Option<u64>) {
  let (absolute_path, blocked_status) = resolve_workspace_path_for_proof(cwd, path);
  let Some(absolute_path) = absolute_path else {
    return (
      blocked_status.unwrap_or_else(|| "invalid-path".to_string()),
      false,
      "not-available".to_string(),
      None,
      None,
    );
  };
  match fs::read_to_string(&absolute_path) {
    Ok(content) => {
      let bytes = content.as_bytes().len() as u64;
      match parse_expr(&content) {
        Ok(_) => (
          "read".to_string(),
          true,
          "valid".to_string(),
          None,
          Some(bytes),
        ),
        Err(err) => (
          "read".to_string(),
          true,
          "invalid".to_string(),
          Some(err.to_string()),
          Some(bytes),
        ),
      }
    }
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => (
      "missing".to_string(),
      false,
      "not-available".to_string(),
      None,
      None,
    ),
    Err(err) => (
      format!("read-error:{}", err.kind()),
      false,
      "not-available".to_string(),
      Some(err.to_string()),
      None,
    ),
  }
}

fn resolve_workspace_path_for_proof(
  cwd: Option<&str>,
  path: &str,
) -> (Option<PathBuf>, Option<String>) {
  if path.trim().is_empty() {
    return (None, Some("empty-path".to_string()));
  }
  let Some(cwd) = cwd.filter(|value| !value.trim().is_empty()) else {
    return (None, Some("missing-cwd".to_string()));
  };
  let cwd_path = Path::new(cwd);
  let raw_path = Path::new(path);
  if raw_path
    .components()
    .any(|component| matches!(component, std::path::Component::ParentDir))
  {
    return (None, Some("parent-dir-path".to_string()));
  }
  if raw_path.is_absolute() {
    if !raw_path.starts_with(cwd_path) {
      return (None, Some("outside-workspace".to_string()));
    }
    (Some(raw_path.to_path_buf()), None)
  } else {
    (Some(cwd_path.join(raw_path)), None)
  }
}

fn updated_file_summary(text: &str) -> UpdatedFileSummary {
  let mut change_kinds = BTreeSet::new();
  let mut updated_paths = Vec::new();
  for line in text.lines() {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
      continue;
    }
    let mut parts = trimmed.split_whitespace();
    let Some(op) = parts.next() else {
      continue;
    };
    let Some(path) = parts.next() else {
      continue;
    };
    if op.len() == 1 && op.chars().all(|ch| ch.is_ascii_uppercase()) {
      change_kinds.insert(op.to_string());
      updated_paths.push(path.to_string());
    }
  }
  UpdatedFileSummary {
    change_kinds: change_kinds.into_iter().collect(),
    updated_paths,
  }
}

fn line_prefix_value(text: &str, prefix: &str) -> Option<String> {
  for line in text.lines() {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix(prefix) {
      let value = rest.trim();
      if !value.is_empty() {
        return Some(value.to_string());
      }
    }
  }
  None
}

fn text_after_marker(text: &str, marker: &str) -> Option<String> {
  text
    .find(marker)
    .map(|index| text[index + marker.len()..].to_string())
}

fn value_to_i64(value: &Value) -> Option<i64> {
  value
    .as_i64()
    .or_else(|| value.as_u64().and_then(|raw| i64::try_from(raw).ok()))
    .or_else(|| {
      value
        .as_str()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
    })
}

fn value_to_f64(value: &Value) -> Option<f64> {
  value.as_f64().or_else(|| {
    value
      .as_str()
      .and_then(|raw| raw.trim().parse::<f64>().ok())
  })
}

fn write_transcript_candidate(attrs: &Map<String, Value>) -> Result<GateAbsorbEmittedCandidate> {
  let (status, quarantine_reason, final_attrs) = apply_transcript_write_floors(attrs.clone())?;
  let kind = transcript_candidate_kind(&final_attrs);
  let content = emit_px_file(&kind, &final_attrs, "transcript-ingest")?;
  let gate_root = gate_store_root()?;
  let relative_dir = candidate_relative_dir(&status, quarantine_reason.as_deref());
  let target_dir = gate_root.join(relative_dir);
  fs::create_dir_all(&target_dir).with_context(|| format!("create {}", target_dir.display()))?;
  let filename = format!(
    "{}-{}-{}.px",
    kind,
    unix_ms(),
    &sha256_hex(content.as_bytes())[..8]
  );
  let path = target_dir.join(&filename);
  fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
  Ok(GateAbsorbEmittedCandidate {
    filename,
    path: path.display().to_string(),
    status,
  })
}

fn candidate_relative_dir(status: &str, quarantine_reason: Option<&str>) -> String {
  match status {
    "candidate" => "px/candidates".to_string(),
    "quarantined" => format!(
      "px/quarantined/{}",
      quarantine_reason.unwrap_or("generic-quarantine")
    ),
    other => format!("px/{}", other),
  }
}

fn apply_transcript_write_floors(
  attrs: Map<String, Value>,
) -> Result<(String, Option<String>, Map<String, Value>)> {
  let report: GateAbsorbFloorOwnerResult = parse_json_value(eval_gate_absorb_floor_owner(
    "transcriptWriteFloors",
    &Value::Object(Map::from_iter([(
      "attrs".to_string(),
      Value::Object(attrs),
    )])),
  )?)?;
  let final_attrs = report.attrs;
  let status = report
    .status
    .or_else(|| {
      final_attrs
        .get("status")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    })
    .unwrap_or_else(|| "candidate".to_string());
  let quarantine_reason = report.quarantine_reason.or_else(|| {
    final_attrs
      .get("quarantine-reason")
      .and_then(Value::as_str)
      .map(ToOwned::to_owned)
  });
  Ok((status, quarantine_reason, final_attrs))
}

fn transcript_candidate_kind(attrs: &Map<String, Value>) -> String {
  if let Some(kind) = attrs.get("kind").and_then(Value::as_str) {
    return kind.to_string();
  }
  match attrs.get("type").and_then(Value::as_str) {
    Some("ValidationRecord") => "validation-record".to_string(),
    Some("ChooserJudgement") => "chooser-judgement".to_string(),
    Some("SelectionTrace") => "selection-trace".to_string(),
    Some("RepairRecipe") => "repair-recipe".to_string(),
    Some("DispatchExecution") => "dispatch-execution".to_string(),
    _ => "observation-atom".to_string(),
  }
}

fn emit_px_file(kind: &str, attrs: &Map<String, Value>, source: &str) -> Result<String> {
  let attrs = canonicalize_candidate_envelope(kind, attrs, source);
  let recorded_at = attrs
    .get("recorded-at")
    .and_then(Value::as_str)
    .unwrap_or("0");
  let header = format!(
    "# pnix-gate candidate\n# kind: {}\n# recorded_at: {}\n# source: {} rule pipeline\n\n",
    kind, recorded_at, source
  );
  Ok(format!(
    "{}builtins.ontologyEmit \"{}\" {}\n",
    header,
    kind,
    emit_px_value(&Value::Object(attrs), "")
  ))
}

fn canonicalize_candidate_envelope(
  kind: &str,
  attrs: &Map<String, Value>,
  source: &str,
) -> Map<String, Value> {
  let mut merged = attrs.clone();
  let packet = attrs
    .get("gate_observation_packet")
    .and_then(Value::as_object)
    .cloned()
    .unwrap_or_default();
  let recorded_at = attrs
    .get("recorded-at")
    .and_then(Value::as_str)
    .map(ToOwned::to_owned)
    .or_else(|| {
      packet
        .get("recorded_at")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    })
    .unwrap_or_else(|| unix_ms().to_string());
  let provider = attrs
    .get("provider")
    .and_then(Value::as_str)
    .map(ToOwned::to_owned)
    .or_else(|| {
      packet
        .get("provider")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    })
    .unwrap_or_else(|| "pnix-gate".to_string());
  let model = attrs
    .get("model")
    .and_then(Value::as_str)
    .map(ToOwned::to_owned)
    .or_else(|| {
      packet
        .get("model")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    });
  let session_id = attrs
    .get("session-id")
    .and_then(Value::as_str)
    .map(ToOwned::to_owned)
    .or_else(|| {
      attrs
        .get("session_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    })
    .or_else(|| {
      packet
        .get("session_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    })
    .unwrap_or_else(|| "unknown".to_string());
  let turn_id = attrs
    .get("turn-id")
    .and_then(Value::as_str)
    .map(ToOwned::to_owned)
    .or_else(|| {
      attrs
        .get("turn_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    })
    .or_else(|| {
      packet
        .get("turn_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    });
  let tool_call_id = attrs
    .get("tool-call-id")
    .and_then(Value::as_str)
    .map(ToOwned::to_owned)
    .or_else(|| {
      attrs
        .get("tool_call_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    })
    .or_else(|| {
      packet
        .get("tool_call_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    });
  let source_rule = attrs
    .get("source-rule")
    .and_then(Value::as_str)
    .map(ToOwned::to_owned)
    .or_else(|| {
      attrs
        .get("provenance")
        .and_then(Value::as_array)
        .and_then(|items| {
          items.iter().find_map(|item| {
            item
              .as_str()
              .and_then(|text| text.strip_prefix("rule:"))
              .map(ToOwned::to_owned)
          })
        })
    })
    .unwrap_or_else(|| source.to_string());
  let content_hash = attrs
    .get("content-hash")
    .and_then(Value::as_str)
    .map(ToOwned::to_owned)
    .or_else(|| {
      packet
        .get("content_hash")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    })
    .unwrap_or_else(|| body_hash(&serde_json::to_string(attrs).unwrap_or_default()));
  let truth_regime = attrs
    .get("truth-regime")
    .and_then(Value::as_str)
    .map(ToOwned::to_owned)
    .or_else(|| {
      attrs
        .get("truth_regime")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
    })
    .unwrap_or_else(|| "interpretive".to_string());
  let status = attrs
    .get("status")
    .and_then(Value::as_str)
    .unwrap_or("candidate")
    .to_string();
  let seed = format!(
    "{}\n{}\n{}",
    kind,
    recorded_at,
    serde_json::to_string(attrs).unwrap_or_default()
  );
  let candidate_id = format!("candidate:{}:{}", kind, &body_hash(&seed)[..16]);
  merged.insert("candidate-id".to_string(), Value::String(candidate_id));
  merged.insert("kind".to_string(), Value::String(kind.to_string()));
  merged.insert("recorded-at".to_string(), Value::String(recorded_at));
  merged.insert("provider".to_string(), Value::String(provider));
  if let Some(model) = model {
    merged.insert("model".to_string(), Value::String(model));
  }
  merged.insert("session-id".to_string(), Value::String(session_id));
  if let Some(turn_id) = turn_id {
    merged.insert("turn-id".to_string(), Value::String(turn_id));
  }
  if let Some(tool_call_id) = tool_call_id {
    merged.insert("tool-call-id".to_string(), Value::String(tool_call_id));
  }
  merged.insert("source-rule".to_string(), Value::String(source_rule));
  merged.insert("content-hash".to_string(), Value::String(content_hash));
  merged.insert("convergence-closes".to_string(), Value::Array(Vec::new()));
  merged.insert("truth-regime".to_string(), Value::String(truth_regime));
  merged.insert("status".to_string(), Value::String(status));
  merged
}

fn emit_px_value(value: &Value, indent: &str) -> String {
  match value {
    Value::Null => "null".to_string(),
    Value::Bool(value) => value.to_string(),
    Value::Number(value) => value.to_string(),
    Value::String(value) => format!("\"{}\"", escape_text(value)),
    Value::Array(items) => {
      if items.is_empty() {
        "[]".to_string()
      } else {
        let inner_indent = format!("{}  ", indent);
        let inner = items
          .iter()
          .map(|item| format!("{}{}", inner_indent, emit_px_value(item, &inner_indent)))
          .collect::<Vec<_>>()
          .join("\n");
        format!("[\n{}\n{}]", inner, indent)
      }
    }
    Value::Object(map) => {
      if map.is_empty() {
        "{}".to_string()
      } else {
        let inner_indent = format!("{}  ", indent);
        let entries = sorted_px_entries(map)
          .into_iter()
          .map(|(key, value)| {
            format!(
              "{}{} = {};",
              inner_indent,
              emit_px_key(key),
              emit_px_value(value, &inner_indent)
            )
          })
          .collect::<Vec<_>>()
          .join("\n");
        format!("{{\n{}\n{}}}", entries, indent)
      }
    }
  }
}

fn sorted_px_entries<'a>(map: &'a Map<String, Value>) -> Vec<(&'a String, &'a Value)> {
  let canonical = [
    "candidate-id",
    "kind",
    "recorded-at",
    "provider",
    "model",
    "session-id",
    "turn-id",
    "tool-call-id",
    "source-rule",
    "content-hash",
    "convergence-closes",
    "truth-regime",
    "status",
  ];
  let order: HashMap<&str, usize> = canonical
    .iter()
    .enumerate()
    .map(|(idx, key)| (*key, idx))
    .collect();
  let mut entries = map.iter().collect::<Vec<_>>();
  entries.sort_by(|(left_key, _), (right_key, _)| {
    let left_rank = order.get(left_key.as_str()).copied().unwrap_or(usize::MAX);
    let right_rank = order.get(right_key.as_str()).copied().unwrap_or(usize::MAX);
    left_rank
      .cmp(&right_rank)
      .then_with(|| left_key.cmp(right_key))
  });
  entries
}

fn emit_px_key(key: &str) -> String {
  let valid = key.chars().enumerate().all(|(index, ch)| {
    if index == 0 {
      ch.is_ascii_alphabetic() || ch == '_'
    } else {
      ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
    }
  });
  if valid {
    key.to_string()
  } else {
    format!("\"{}\"", escape_text(key))
  }
}

fn json_u64(value: u64) -> Value {
  Value::Number(serde_json::Number::from(value))
}

pub(super) fn parse_transcript_file(path: &Path) -> Result<Vec<GateAbsorbConversationTurn>> {
  let raw =
    fs::read_to_string(path).with_context(|| format!("read transcript file {}", path.display()))?;
  if looks_json_array(path, &raw) {
    if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
      let mut turns = Vec::with_capacity(parsed.len());
      for (index, item) in parsed.into_iter().enumerate() {
        let speaker = item
          .get("speaker")
          .or_else(|| item.get("role"))
          .and_then(|value| value.as_str())
          .map(ToOwned::to_owned)
          .unwrap_or_else(|| format!("speaker-{}", index));
        let text = item
          .get("text")
          .or_else(|| item.get("content"))
          .and_then(|value| value.as_str())
          .map(ToOwned::to_owned)
          .unwrap_or_default();
        turns.push(GateAbsorbConversationTurn {
          turn: index,
          speaker,
          language: detect_language(&text),
          text,
        });
      }
      return Ok(turns);
    }
  }
  parse_plain_transcript(&raw)
}

fn looks_json_array(path: &Path, raw: &str) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| ext.eq_ignore_ascii_case("json"))
    .unwrap_or(false)
    || raw.trim_start().starts_with('[')
}

fn parse_plain_transcript(raw: &str) -> Result<Vec<GateAbsorbConversationTurn>> {
  let mut turns = Vec::new();
  for line in raw.lines() {
    let trimmed = line.trim();
    if trimmed.is_empty() {
      continue;
    }
    if let Some((speaker, text)) = trimmed.split_once(':') {
      let speaker = speaker.trim().to_string();
      let text = text.trim().to_string();
      let turn = turns.len();
      turns.push(GateAbsorbConversationTurn {
        turn,
        speaker: if speaker.is_empty() {
          format!("speaker-{}", turn)
        } else {
          speaker
        },
        language: detect_language(&text),
        text,
      });
      continue;
    }
    if let Some(previous) = turns.last_mut() {
      if !previous.text.is_empty() {
        previous.text.push(' ');
      }
      previous.text.push_str(trimmed);
      previous.language = detect_language(&previous.text);
      continue;
    }
    turns.push(GateAbsorbConversationTurn {
      turn: 0,
      speaker: "speaker-0".to_string(),
      language: detect_language(trimmed),
      text: trimmed.to_string(),
    });
  }
  Ok(turns)
}

pub(super) fn extract_vocab(turn: &GateAbsorbConversationTurn) -> GateAbsorbVocabSummary {
  let tokens = if matches!(turn.language, GateAbsorbLanguage::En) {
    turn
      .text
      .split_whitespace()
      .filter(|token| !token.is_empty())
      .map(|token| token.to_lowercase())
      .collect()
  } else {
    turn
      .text
      .chars()
      .filter(|ch| allowed_token_char(*ch))
      .map(|ch| ch.to_string())
      .collect()
  };
  GateAbsorbVocabSummary {
    language: turn.language.clone(),
    tokens,
  }
}

pub(super) fn detect_language(text: &str) -> GateAbsorbLanguage {
  let codepoints = text
    .chars()
    .filter(|ch| !ch.is_whitespace())
    .collect::<Vec<_>>();
  if codepoints.is_empty() {
    return GateAbsorbLanguage::Unknown;
  }
  let total = codepoints.len() as f64;
  let hangul = codepoints.iter().filter(|ch| is_hangul(**ch)).count() as f64;
  let hiragana = codepoints.iter().filter(|ch| is_hiragana(**ch)).count() as f64;
  let katakana = codepoints.iter().filter(|ch| is_katakana(**ch)).count() as f64;
  let cjk = codepoints
    .iter()
    .filter(|ch| is_cjk_ideograph(**ch))
    .count() as f64;
  let latin = codepoints
    .iter()
    .filter(|ch| is_basic_latin_letter(**ch))
    .count() as f64;
  if hangul / total > 0.30 {
    GateAbsorbLanguage::Ko
  } else if (hiragana + katakana) / total > 0.15 {
    GateAbsorbLanguage::Ja
  } else if latin / total > 0.60 && cjk == 0.0 {
    GateAbsorbLanguage::En
  } else {
    GateAbsorbLanguage::Unknown
  }
}

fn allowed_token_char(ch: char) -> bool {
  is_basic_latin_letter(ch)
    || is_hangul(ch)
    || is_hiragana(ch)
    || is_katakana(ch)
    || is_cjk_ideograph(ch)
}

fn is_hangul(ch: char) -> bool {
  ('\u{AC00}'..='\u{D7A3}').contains(&ch)
}

fn is_hiragana(ch: char) -> bool {
  ('\u{3040}'..='\u{309F}').contains(&ch)
}

fn is_katakana(ch: char) -> bool {
  ('\u{30A0}'..='\u{30FF}').contains(&ch)
}

fn is_cjk_ideograph(ch: char) -> bool {
  ('\u{4E00}'..='\u{9FFF}').contains(&ch)
}

fn is_basic_latin_letter(ch: char) -> bool {
  ch.is_ascii_alphabetic()
}

fn sha256_hex(bytes: &[u8]) -> String {
  let mut hasher = Sha256::new();
  hasher.update(bytes);
  format!("{:x}", hasher.finalize())
}
