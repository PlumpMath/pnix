use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
// DOGHOUSE-DEPRECATION (2026-06-01): 아래 doghouse 게이트 코드(brain-ankh
// coding-memory store 입력)는 pnixc-meta substrate 로 대체될 예정. doghouse
// 빌드는 제거됐고 default OFF. 자세한 건 cli.rs 상단 DOGHOUSE-DEPRECATION 주석.
#[cfg(feature = "doghouse")]
use doghouse_core::store::{CodingMemoryArtifact, DoghouseStore, DoghouseStoreConfig};
use pnix_query_runtime::px_eval_json;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};

use super::args::{Args, GateReadVerb, OutputFormat};

const EX_OK: i32 = 0;
const EX_USAGE: i32 = 64;
const GATE_READ_USAGE: &str =
  "usage: pnix gate-read {help|status|state-sink-contract|ontology-coverage|meaning-bridges|self-capabilities|meta-protocols|lift-rule-coverage|store-budget|artifact-ref-ratio|storage-telemetry|provenance-floor|unsupported-kind-floor|lineage-floor|recent-events|candidates|brain-ankh-policy|brain-bundle-contract|validate-brain-bundle|curriculum-current-target|ontology-lookup-related|recipe-match-current|query-context}";
const DEFAULT_RECIPE_STALE_REPEAT_THRESHOLD: usize = 2;
const RECIPE_MATCH_TELEMETRY_HISTORY_LIMIT: usize = 5000;
const DEFAULT_HOT_STORE_BUDGET_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CurriculumCurrentTargetReport {
  metric_version: String,
  generated_at: String,
  target: String,
  target_state: Option<String>,
  target_anchor_path: Option<String>,
  target_anchor_line_hint: Option<Value>,
  target_description_from_convergence_md: Option<String>,
  target_resolution: Option<String>,
  closed_anchors_in_stage: Vec<String>,
  progress_ratio: Option<Value>,
  next_candidates: Vec<String>,
  advance_guard_state: Option<Value>,
  advance_ready: Option<bool>,
  closure_report_source: Option<String>,
  closure_report_commit_grounded: Option<bool>,
  curriculum_stage_order_source: Option<String>,
  read_owner: Option<String>,
  notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateReadLookupFact {
  subj: String,
  pred: String,
  obj: String,
  status: String,
  provenance_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateReadLookupReport {
  metric_version: String,
  generated_at: String,
  read_owner: Option<String>,
  lookup_status: String,
  result_source: String,
  adapter_mode: String,
  query_owner: String,
  selection_owner: String,
  context: String,
  predicate: Option<String>,
  limit: usize,
  min_confidence: f64,
  query_fingerprint: String,
  ranked_candidate_ids: Vec<String>,
  result_fingerprint: String,
  accepted_fact_total: usize,
  matched_candidate_total: usize,
  fact_total: usize,
  facts: Vec<GateReadLookupFact>,
  notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryContextCandidateHit {
  path: String,
  kind: Option<String>,
  status: Option<String>,
  recorded_at: Option<String>,
  subject: Option<String>,
  predicate: Option<String>,
  object: Option<String>,
  query: Option<String>,
  intent: Option<String>,
  source_rule: Option<String>,
  trace_ref: Option<String>,
  topic_hit_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryContextEventHit {
  event: Option<String>,
  recorded_at: Option<String>,
  provider: Option<String>,
  session_id: Option<String>,
  turn_id: Option<String>,
  tool_name: Option<String>,
  message_role: Option<String>,
  phase: Option<String>,
  topic_hit_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryContextRecipeHit {
  recipe_id: String,
  tool_name: String,
  context: String,
  arg_predicates: Vec<String>,
  subject: String,
  confidence: f64,
  lineage_status: Option<String>,
  last_effective_at: Option<String>,
  topic_hit_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryContextReport {
  metric_version: String,
  generated_at: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  read_owner: Option<String>,
  query_status: String,
  result_source: String,
  topic: String,
  topic_tokens: Vec<String>,
  limit: usize,
  lookup_preview: GateReadLookupPreview,
  accepted_fact_hits: Vec<GateReadLookupFact>,
  candidate_hits: Vec<QueryContextCandidateHit>,
  event_hits: Vec<QueryContextEventHit>,
  recipe_hits: Vec<QueryContextRecipeHit>,
  accepted_fact_total: usize,
  candidate_hit_total: usize,
  event_hit_total: usize,
  recipe_hit_total: usize,
  notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct GateReadStatusReport {
  metric_version: &'static str,
  generated_at: String,
  status_owner: &'static str,
  status_mode: &'static str,
  store_root: String,
  events_path: String,
  candidate_dir: String,
  event_total: usize,
  candidate_total: usize,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct StateSinkLaneSpec {
  status: String,
  relative_path: String,
  materialized_path: String,
  tier: String,
  lifecycle_role: String,
}

#[derive(Debug, Clone, Serialize)]
struct StateSinkContractReport {
  metric_version: &'static str,
  generated_at: String,
  read_owner: &'static str,
  contract_version: String,
  abi_name: String,
  store_root: String,
  store_profile_kind: String,
  store_profile_source: String,
  materialization_kind: String,
  portable_profile_kinds: Vec<String>,
  storage_tier_contract_version: String,
  storage_tier_names: Vec<String>,
  correctness_requires_central_service: bool,
  judgement_owner: String,
  lifecycle_statuses: Vec<String>,
  status_count: usize,
  statuses: Vec<StateSinkLaneSpec>,
  candidate_lane: StateSinkLaneSpec,
  auxiliary_lanes: Vec<StateSinkLaneSpec>,
  state_sink_ready: bool,
  state_sink_present_total: usize,
  state_sink_presence: BTreeMap<String, bool>,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct StoreBudgetReport {
  metric_version: &'static str,
  generated_at: String,
  read_owner: &'static str,
  status: String,
  hot_store_budget_bytes: u64,
  hot_store_bytes: u64,
  effective_hot_store_bytes: u64,
  candidate_store_bytes: u64,
  control_plane_bytes: u64,
  events_bytes: u64,
  pending_inline_bytes: u64,
  live_buffer_open_bytes: u64,
  live_buffer_dirty_bytes: u64,
  live_buffer_open_count: u64,
  live_buffer_dirty_count: u64,
  live_buffer_error_count: u64,
  live_buffer_parse_pass_rate: Option<f64>,
  live_buffer_snapshot_updated_at: Option<String>,
  live_buffer_snapshot_source: Option<String>,
  budget_remaining_bytes: u64,
  budget_exceeded: bool,
  pressure_ratio: Option<f64>,
  inline_blob_mode: String,
  checkpoint_total: usize,
  budget_exceeded_total: usize,
  suppressed_total: usize,
  session_total: usize,
  latest_recorded_at: Option<String>,
  latest_inline_blob_mode: Option<String>,
  latest_budget_exceeded: Option<bool>,
  latest_hot_store_bytes: Option<u64>,
  latest_effective_hot_store_bytes: Option<u64>,
  latest_budget_remaining_bytes: Option<u64>,
  latest_pressure_ratio: Option<f64>,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct ArtifactRefCoverageStatusSummary {
  record_total: usize,
  with_artifact_ref_total: usize,
  field_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
struct ArtifactRefCoverageReport {
  metric_version: &'static str,
  generated_at: String,
  read_owner: &'static str,
  record_total: usize,
  with_artifact_ref_total: usize,
  record_ratio: f64,
  candidate_record_total: usize,
  candidate_with_artifact_ref_total: usize,
  candidate_record_ratio: f64,
  state_sink_record_total: usize,
  state_sink_with_artifact_ref_total: usize,
  state_sink_record_ratio: f64,
  field_counts: BTreeMap<String, usize>,
  status_counts: BTreeMap<String, ArtifactRefCoverageStatusSummary>,
  artifact_ref_candidate_total: usize,
  artifact_ref_ratio: f64,
  artifact_ref_record_total: usize,
  artifact_ref_record_ratio: f64,
  artifact_ref_state_sink_total: usize,
  artifact_ref_state_sink_ratio: f64,
  artifact_ref_field_counts: BTreeMap<String, usize>,
  artifact_ref_status_counts: BTreeMap<String, ArtifactRefCoverageStatusSummary>,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct StorageTelemetryReport {
  metric_version: &'static str,
  generated_at: String,
  read_owner: &'static str,
  store_bytes_total: u64,
  hot_store_bytes: u64,
  control_plane_bytes: u64,
  candidate_store_bytes: u64,
  events_bytes: u64,
  live_buffer_open_bytes: u64,
  live_buffer_dirty_bytes: u64,
  live_buffer_open_count: u64,
  live_buffer_dirty_count: u64,
  live_buffer_error_count: u64,
  live_buffer_parse_pass_rate: Option<f64>,
  warm_store_bytes: u64,
  state_sink_bytes: u64,
  artifact_store_bytes: u64,
  cold_store_bytes: u64,
  tier_bytes: BTreeMap<String, u64>,
  storage_tier_contract_version: String,
  storage_tier_abi_name: String,
  storage_tier_names: Vec<String>,
  storage_tier_contract: Value,
  hot_store_budget_bytes: u64,
  hot_store_budget_remaining_bytes: u64,
  hot_store_budget_exceeded: bool,
  hot_store_pressure_ratio: Option<f64>,
  inline_blob_mode: String,
  hot_store_checkpoint_total: usize,
  hot_store_checkpoint_exceeded_total: usize,
  hot_store_checkpoint_suppressed_total: usize,
  hot_store_checkpoint_latest_recorded_at: Option<String>,
  hot_store_checkpoint_latest_inline_blob_mode: Option<String>,
  candidate_total: usize,
  artifact_ref_candidate_total: usize,
  artifact_ref_ratio: f64,
  artifact_ref_record_total: usize,
  artifact_ref_record_ratio: f64,
  artifact_ref_state_sink_total: usize,
  artifact_ref_state_sink_ratio: f64,
  artifact_ref_field_counts: BTreeMap<String, usize>,
  artifact_ref_status_counts: BTreeMap<String, ArtifactRefCoverageStatusSummary>,
  gc_reclaimed_bytes: u64,
  ttl_violation_count: Option<u64>,
  dangling_ref_count: Option<u64>,
  state_sink_ready: bool,
  state_sink_present_total: usize,
  state_sink_contract_version: String,
  state_sink_abi_name: String,
  state_sink_profile_kind: String,
  state_sink_profile_source: String,
  state_sink_materialization_kind: String,
  state_sink_status_count: usize,
  state_sink_statuses: Vec<String>,
  state_sink_relative_paths: BTreeMap<String, String>,
  storage_snapshot_source: String,
  storage_snapshot_status: String,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct ProvenanceFloorSample {
  path: String,
  status: String,
  session_id: Option<String>,
  turn_id: Option<String>,
  tool_call_id: Option<String>,
  tool_call_required: bool,
  failure_codes: Vec<String>,
  quarantine_reason: Option<String>,
  provenance_floor_status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ProvenanceFloorReport {
  metric_version: &'static str,
  generated_at: String,
  read_owner: &'static str,
  status: String,
  record_total: usize,
  accepted_total: usize,
  tool_call_required_total: usize,
  weak_total: usize,
  weak_record_ratio: f64,
  weak_accepted_total: usize,
  accepted_floor_pass_rate: f64,
  quarantined_missing_provenance_total: usize,
  session_unknown_total: usize,
  missing_session_total: usize,
  missing_turn_total: usize,
  missing_tool_call_total: usize,
  sample_weak: Vec<ProvenanceFloorSample>,
  sample_quarantined_missing_provenance: Vec<ProvenanceFloorSample>,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct UnsupportedKindSample {
  path: String,
  status: String,
  kind: String,
  quarantine_reason: Option<String>,
  kind_support_status: Option<String>,
  schema_todo_status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct UnsupportedKindFloorReport {
  metric_version: &'static str,
  generated_at: String,
  read_owner: &'static str,
  status: String,
  record_total: usize,
  unsupported_kind_total: usize,
  unsupported_kind_record_ratio: f64,
  unsupported_kind_quarantined_total: usize,
  unsupported_kind_leaking_total: usize,
  unsupported_kind_quarantine_rate: f64,
  unsupported_kind_counts: BTreeMap<String, usize>,
  unsupported_kind_inventory: Vec<String>,
  sample_unsupported_kind: Vec<UnsupportedKindSample>,
  sample_unsupported_kind_leaks: Vec<UnsupportedKindSample>,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct LineageFloorSample {
  path: String,
  status: String,
  kind: String,
  source_session_id: Option<String>,
  source_turn_id: Option<String>,
  derived_from_candidate_id: Option<String>,
  parent_packet_ids: Vec<String>,
  lineage_anchor_required: bool,
  lineage_floor_status: Option<String>,
  computed_lineage_floor_status: Option<String>,
  failure_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct LineageFloorReport {
  generated_at: String,
  metric_version: &'static str,
  read_owner: &'static str,
  status: String,
  record_total: usize,
  weak_total: usize,
  passed_total: usize,
  partial_total: usize,
  missing_source_context_total: usize,
  lineage_anchor_required_total: usize,
  missing_source_session_total: usize,
  missing_source_turn_total: usize,
  missing_derived_from_candidate_total: usize,
  missing_parent_packet_total: usize,
  missing_floor_status_total: usize,
  stale_floor_status_total: usize,
  unparseable_attrset_total: usize,
  status_counts: BTreeMap<String, usize>,
  kind_counts: BTreeMap<String, usize>,
  lineage_floor_status_counts: BTreeMap<String, usize>,
  sample_weak: Vec<LineageFloorSample>,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct LiftRuleCoverageReport {
  generated_at: String,
  metric_version: &'static str,
  read_owner: &'static str,
  status: String,
  score: f64,
  rule_total: usize,
  complete_total: usize,
  ready_total: usize,
  contextual_fact_total: usize,
  canonical_kind_total: usize,
  missing_kinds: Vec<String>,
  duplicate_kinds: Vec<String>,
  extra_kinds: Vec<String>,
  constructor: Option<String>,
  evaluation_shape: Option<String>,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct OntologyCoverageFileScore {
  id: &'static str,
  score: f64,
}

#[derive(Debug, Clone, Serialize)]
struct OntologyCoverageBreadth {
  concept_total: usize,
  domain_total: usize,
  triple_total: usize,
  meaning_bridge_total: usize,
  meta_protocol_total: usize,
  intent_route_total: usize,
  self_capability_total: usize,
  recipe_total: usize,
  lift_rule_total: usize,
  tool_spec_total: usize,
}

#[derive(Debug, Clone, Serialize)]
struct OntologyCoverageReport {
  generated_at: String,
  metric_version: &'static str,
  read_owner: &'static str,
  score: f64,
  parse_pass_rate: f64,
  file_scores: Vec<OntologyCoverageFileScore>,
  breadth: OntologyCoverageBreadth,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct MeaningBridgesReport {
  generated_at: String,
  metric_version: &'static str,
  read_owner: &'static str,
  score: f64,
  bridge_total: usize,
  complete_total: usize,
  roundtrip_ready_total: usize,
  avg_latent_link_total: f64,
  meta_bridge_count: Option<usize>,
  meta_focus: Option<String>,
  reports: Vec<Value>,
  issues: Vec<Value>,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct SelfCapabilitiesReport {
  generated_at: String,
  metric_version: &'static str,
  read_owner: &'static str,
  score: f64,
  capability_total: usize,
  complete_total: usize,
  self_referential_total: usize,
  meta_capability_count: Option<usize>,
  meta_priority: Option<String>,
  reports: Vec<Value>,
  issues: Vec<Value>,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct MetaProtocolsReport {
  generated_at: String,
  metric_version: &'static str,
  read_owner: &'static str,
  score: f64,
  protocol_total: usize,
  complete_total: usize,
  reuse_ready_total: usize,
  meta_protocol_count: Option<usize>,
  meta_focus: Option<String>,
  reports: Vec<Value>,
  issues: Vec<Value>,
  notes: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateReadRecentEvent {
  event: Option<String>,
  recorded_at: Option<String>,
  provider: Option<String>,
  session_id: Option<String>,
  turn_id: Option<String>,
  tool_name: Option<String>,
  hook_event_name: Option<String>,
  phase: Option<String>,
  output_surface: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateReadRecentEventsReport {
  metric_version: String,
  generated_at: String,
  read_owner: String,
  event_types: Vec<String>,
  limit: usize,
  event_total: usize,
  events: Vec<GateReadRecentEvent>,
  notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateReadCandidateEntry {
  filename: String,
  path: String,
  kind: Option<String>,
  status: Option<String>,
  recorded_at: Option<String>,
  subject: Option<String>,
  query: Option<String>,
  intent: Option<String>,
  trace_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateReadCandidatesReport {
  metric_version: String,
  generated_at: String,
  read_owner: String,
  kind_filter: Option<String>,
  limit: usize,
  candidate_total: usize,
  candidates: Vec<GateReadCandidateEntry>,
  notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhGateSignalInput {
  source_ref: String,
  source_path: String,
  source_status: String,
  kind: String,
  recorded_at: Option<String>,
  trace_ref: Option<String>,
  subject: Option<String>,
  predicate: Option<String>,
  object: Option<String>,
  query: Option<String>,
  intent: Option<String>,
  selected_card: Option<String>,
  ranked_cards: Vec<String>,
  chooser: Option<String>,
  judgement: Option<String>,
  confidence: Option<String>,
  dispatch_status: Option<String>,
  tool: Option<String>,
  evidence: Vec<String>,
  reasons: Vec<String>,
  provenance: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhCodingMemoryInput {
  source_ref: String,
  source_family: String,
  source_surface: String,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<String>,
  target_paths: Vec<String>,
  command_refs: Vec<String>,
  related_refs: Vec<String>,
  status: Option<String>,
  subject: Option<String>,
  evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhPolicyInputTotals {
  gate_signal_total: usize,
  coding_memory_total: usize,
  observation_atom_total: usize,
  selection_trace_total: usize,
  chooser_judgement_total: usize,
  dispatch_execution_total: usize,
  validation_record_total: usize,
  repair_recipe_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhPolicyCandidateProjection {
  artifact_family: String,
  candidate_ref: String,
  source_ref: String,
  source_family: String,
  source_surface: String,
  signal_class: String,
  proposed_change_axis: String,
  proposed_action: String,
  source_status: Option<String>,
  subject: Option<String>,
  evidence_refs: Vec<String>,
  replay_status: String,
  promotion_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhRoutingDecisionProjection {
  artifact_family: String,
  decision_ref: String,
  source_ref: String,
  trace_ref: Option<String>,
  query: Option<String>,
  intent: Option<String>,
  selected_route: String,
  ranked_routes: Vec<String>,
  evidence_refs: Vec<String>,
  decision_status: String,
  promotion_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhSelfExplanationProjection {
  artifact_family: String,
  explanation_ref: String,
  source_ref: String,
  trace_ref: Option<String>,
  chooser: Option<String>,
  judgement: Option<String>,
  confidence: Option<String>,
  reasons: Vec<String>,
  evidence_refs: Vec<String>,
  explanation_status: String,
  explanation_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhAxisDecisionProjection {
  artifact_family: String,
  decision_ref: String,
  source_ref: String,
  source_family: String,
  source_surface: String,
  decision_axis: String,
  selected_policy_candidate: String,
  proposed_action: String,
  subject: Option<String>,
  evidence_refs: Vec<String>,
  decision_status: String,
  promotion_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhResearchIntentProjection {
  artifact_family: String,
  intent_ref: String,
  source_candidate_ref: String,
  source_ref: String,
  source_family: String,
  query_goal: String,
  source_scope: Vec<String>,
  risk_controls: Vec<String>,
  stop_condition: String,
  direct_truth_source: bool,
  intent_status: String,
  promotion_boundary: String,
  evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhSourceCandidateProjection {
  artifact_family: String,
  source_candidate_ref: String,
  research_intent_ref: String,
  candidate_id: String,
  kind: String,
  provider: String,
  model: Option<String>,
  session_id: String,
  source_rule: String,
  content_hash: String,
  truth_regime: String,
  status: String,
  direct_truth_source: bool,
  source_id: String,
  source_version: String,
  source_checksum: String,
  entity_key: String,
  member_path: String,
  rule_version: String,
  query_goal: String,
  source_scope: Vec<String>,
  risk_controls: Vec<String>,
  citation_policy: String,
  freshness_policy: String,
  license_policy: String,
  benchmark_contamination_policy: String,
  adversarial_prompt_policy: String,
  raw_retention_policy: String,
  next_required_artifacts: Vec<String>,
  promotion_boundary: String,
  evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhPolicyRevisionReceiptProjection {
  artifact_family: String,
  receipt_ref: String,
  source_ref: String,
  source_family: String,
  observed_outcome: String,
  revision_status: String,
  policy_mutation_applied: bool,
  changed_policy_refs: Vec<String>,
  proof_refs: Vec<String>,
  next_required_artifacts: Vec<String>,
  receipt_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhMindDeltaCandidateProjection {
  artifact_family: String,
  delta_ref: String,
  source_ref: String,
  source_policy_candidate: String,
  affected_slice_ref: String,
  changed_axis: String,
  changed_mind_part: String,
  changed_mind_scope: String,
  event_id: String,
  judgement_id: String,
  provenance_refs: Vec<String>,
  required_refs_status: String,
  delta_status: String,
  delta_boundary: String,
  whole_brain_snapshot_claimed: bool,
  system_brain_snapshot_is_proof: bool,
  ankh_family_snapshot_is_proof: bool,
  mind_map_layout_is_proof: bool,
  graph_completeness_claimed: bool,
  xml_parse_success_is_proof: bool,
  p_puck_green_is_proof: bool,
  proof_reuse_allowed: bool,
  store_mutation: bool,
  policy_mutation_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhAffectedMindSliceProjection {
  artifact_family: String,
  slice_ref: String,
  source_policy_candidate: String,
  source_ref: String,
  source_family: String,
  changed_axis: String,
  changed_mind_part: String,
  slice_kind: String,
  dependency_edge_kind: String,
  affected_contexts: Vec<String>,
  target_replay_surfaces: Vec<String>,
  p_puck_proof_selection: Vec<String>,
  pnixc_meta_incremental_compile_targets: Vec<String>,
  event_id: String,
  judgement_id: String,
  provenance_refs: Vec<String>,
  mind_delta_candidate_ref: String,
  migration_rejudge_receipt_ref: String,
  required_refs_status: String,
  slice_status: String,
  store_mutation: bool,
  policy_mutation_applied: bool,
  proof_reuse_allowed: bool,
  proof_reuse_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhSemanticDependencyNode {
  node_ref: String,
  node_family: String,
  node_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhSemanticDependencyEdge {
  edge_ref: String,
  edge_kind: String,
  from_ref: String,
  to_ref: String,
  source_ref: String,
  event_id: String,
  judgement_id: String,
  provenance_refs: Vec<String>,
  migration_rejudge_receipt_ref: String,
  edge_status: String,
  proof_reuse_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhSemanticDependencyGraph {
  artifact_family: String,
  graph_ref: String,
  graph_owner: String,
  node_total: usize,
  edge_total: usize,
  nodes: Vec<BrainAnkhSemanticDependencyNode>,
  edges: Vec<BrainAnkhSemanticDependencyEdge>,
  graph_status: String,
  graph_completeness_claimed: bool,
  xml_parse_success_is_proof: bool,
  p_puck_green_is_proof: bool,
  proof_reuse_allowed: bool,
  proof_reuse_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhRejudgeReceiptCandidateProjection {
  artifact_family: String,
  receipt_ref: String,
  source_ref: String,
  source_policy_candidate: String,
  affected_slice_ref: String,
  changed_axis: String,
  changed_mind_part: String,
  event_id: String,
  judgement_id: String,
  provenance_refs: Vec<String>,
  required_refs_status: String,
  rejudge_status: String,
  target_replay_surfaces: Vec<String>,
  p_puck_proof_selection: Vec<String>,
  pnixc_meta_incremental_compile_targets: Vec<String>,
  previous_green_reuse_allowed: bool,
  proof_reuse_allowed: bool,
  proof_reuse_boundary: String,
  graph_completeness_claimed: bool,
  xml_parse_success_is_proof: bool,
  p_puck_green_is_proof: bool,
  old_green_is_current_proof: bool,
  receipt_write_status: String,
  store_mutation: bool,
  policy_mutation_applied: bool,
  promotion_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhTargetedReplayPlanProjection {
  artifact_family: String,
  plan_ref: String,
  source_ref: String,
  source_policy_candidate: String,
  affected_slice_ref: String,
  rejudge_receipt_candidate_ref: String,
  changed_axis: String,
  required_refs_status: String,
  replay_status: String,
  replay_execution_status: String,
  proof_reuse_allowed: bool,
  proof_reuse_boundary: String,
  p_puck_green_is_proof: bool,
  old_green_is_current_proof: bool,
  stale_latest_report_is_proof: bool,
  profile_speedup_is_completion_proof: bool,
  replay_selection_boundary: String,
  store_mutation: bool,
  policy_mutation_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhProofSelectionCandidateProjection {
  artifact_family: String,
  selection_ref: String,
  source_ref: String,
  source_policy_candidate: String,
  affected_slice_ref: String,
  targeted_replay_plan_ref: String,
  rejudge_receipt_candidate_ref: String,
  changed_axis: String,
  required_refs_status: String,
  selection_status: String,
  target_proof_presets: Vec<String>,
  proof_selection_owner: String,
  proof_selection_status_boundary: String,
  proof_reuse_allowed: bool,
  old_green_is_current_proof: bool,
  p_puck_green_is_proof: bool,
  stale_latest_report_is_proof: bool,
  profile_speedup_is_completion_proof: bool,
  store_mutation: bool,
  policy_mutation_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhIncrementalSelfCompileCandidateProjection {
  artifact_family: String,
  candidate_ref: String,
  source_ref: String,
  source_policy_candidate: String,
  affected_slice_ref: String,
  targeted_replay_plan_ref: String,
  rejudge_receipt_candidate_ref: String,
  changed_axis: String,
  changed_mind_part: String,
  required_refs_status: String,
  compile_targets: Vec<String>,
  compile_target_total: usize,
  compile_status: String,
  compile_execution_status: String,
  compiler_owner: String,
  compiler_owner_boundary: String,
  incremental_compile_boundary: String,
  proof_reuse_allowed: bool,
  previous_green_reuse_allowed: bool,
  old_green_is_current_proof: bool,
  p_puck_green_is_proof: bool,
  stale_latest_report_is_proof: bool,
  profile_speedup_is_completion_proof: bool,
  compile_reuse_allowed: bool,
  self_compile_mutation_applied: bool,
  store_mutation: bool,
  policy_mutation_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhSystemBrainSnapshotCandidateProjection {
  artifact_family: String,
  snapshot_ref: String,
  source_ref: String,
  fixed_point_ref: String,
  source_of_truth: String,
  snapshot_status: String,
  contextual_fact_refs: Vec<String>,
  judgement_topology_refs: Vec<String>,
  guard_state_refs: Vec<String>,
  policy_candidate_refs: Vec<String>,
  mind_delta_refs: Vec<String>,
  affected_slice_refs: Vec<String>,
  rejudge_receipt_refs: Vec<String>,
  xml_carrier_status: String,
  xml_is_truth: bool,
  knowledge_db_export: bool,
  p_puck_green_is_proof: bool,
  mind_map_layout_is_proof: bool,
  proof_reuse_allowed: bool,
  same_judge_lifecycle_required: bool,
  store_mutation: bool,
  policy_mutation_applied: bool,
  promotion_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhFamilyObjectProjection {
  object_ref: String,
  object_kind: String,
  object_role: String,
  visibility: String,
  owner_boundary: String,
  same_judge_lifecycle_required: bool,
  accepted_without_rejudge: bool,
  object_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhFamilyMorphismProjection {
  morphism_ref: String,
  morphism_kind: String,
  from_ref: String,
  to_ref: String,
  morphism_role: String,
  receipt_boundary: String,
  same_judge_lifecycle_required: bool,
  accepted_without_receipt: bool,
  morphism_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhAnkhFamilySnapshotCandidateProjection {
  artifact_family: String,
  family_ref: String,
  source_ref: String,
  system_brain_snapshot_ref: String,
  object_total: usize,
  morphism_total: usize,
  objects: Vec<BrainAnkhFamilyObjectProjection>,
  morphisms: Vec<BrainAnkhFamilyMorphismProjection>,
  family_status: String,
  formal_category_theory_proof_claimed: bool,
  same_judge_lifecycle_required: bool,
  incompatible_family_acceptance_allowed: bool,
  not_closed_family_status: String,
  xml_is_truth: bool,
  p_puck_green_is_proof: bool,
  proof_reuse_allowed: bool,
  store_mutation: bool,
  policy_mutation_applied: bool,
  promotion_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhMindMapProjectionCandidateProjection {
  artifact_family: String,
  projection_ref: String,
  source_ref: String,
  source_of_truth: String,
  projection_status: String,
  system_brain_snapshot_refs: Vec<String>,
  ankh_family_snapshot_refs: Vec<String>,
  mind_delta_refs: Vec<String>,
  affected_slice_refs: Vec<String>,
  rejudge_receipt_refs: Vec<String>,
  policy_candidate_refs: Vec<String>,
  layout_is_proof: bool,
  node_edge_schema_is_truth: bool,
  graph_layout_owner: String,
  graph_completeness_claimed: bool,
  xml_is_truth: bool,
  p_puck_green_is_proof: bool,
  proof_reuse_allowed: bool,
  store_mutation: bool,
  policy_mutation_applied: bool,
  promotion_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhBrainDiagramPacketCandidateProjection {
  artifact_family: String,
  packet_ref: String,
  source_ref: String,
  mind_map_projection_ref: String,
  packet_status: String,
  diagram_node_total: usize,
  diagram_edge_total: usize,
  diagram_node_kinds: Vec<String>,
  diagram_edge_kinds: Vec<String>,
  contextual_fact_node_kind: String,
  lifecycle_flow: Vec<String>,
  contextual_fact_node_is_raw_memory_row: bool,
  lifecycle_flow_required: bool,
  static_graph_viewer_close_allowed: bool,
  projection_surface_only: bool,
  second_truth_owner: bool,
  layout_is_proof: bool,
  graph_completeness_claimed: bool,
  store_mutation: bool,
  policy_mutation_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhDashboardProjectionCandidateProjection {
  artifact_family: String,
  dashboard_ref: String,
  source_ref: String,
  mind_map_projection_ref: String,
  brain_diagram_packet_ref: String,
  operator_cockpit_status: String,
  user_surface_status: String,
  stale_projection_visible: bool,
  false_green_visible: bool,
  slow_proof_path_visible: bool,
  affected_proof_slice_visible: bool,
  unsupported_quarantine_visible: bool,
  missing_provenance_visible: bool,
  raw_screenshot_sync_allowed: bool,
  private_lineage_exposed_to_freecat: bool,
  privileged_inference_exposed_to_freecat: bool,
  provider_secret_exposed_to_freecat: bool,
  canonical_merge_law_exposed_to_freecat: bool,
  projection_surface_only: bool,
  second_truth_owner: bool,
  store_mutation: bool,
  policy_mutation_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainAnkhPolicyProjectionReport {
  metric_version: String,
  generated_at: String,
  read_owner: String,
  projection_owner: String,
  projection_status: String,
  read_only: bool,
  store_mutation: bool,
  gate_store_root: String,
  coding_memory_store_path: Option<String>,
  limit: usize,
  input_totals: BrainAnkhPolicyInputTotals,
  policy_candidate_total: usize,
  routing_decision_total: usize,
  attach_decision_total: usize,
  priority_decision_total: usize,
  research_intent_total: usize,
  source_candidate_total: usize,
  self_explanation_total: usize,
  policy_revision_receipt_total: usize,
  mind_delta_candidate_total: usize,
  affected_mind_slice_total: usize,
  semantic_dependency_edge_total: usize,
  rejudge_receipt_candidate_total: usize,
  targeted_replay_plan_total: usize,
  proof_selection_candidate_total: usize,
  incremental_self_compile_candidate_total: usize,
  system_brain_snapshot_candidate_total: usize,
  ankh_family_snapshot_candidate_total: usize,
  mind_map_projection_candidate_total: usize,
  brain_diagram_packet_candidate_total: usize,
  dashboard_projection_candidate_total: usize,
  proof_reuse_allowed: bool,
  policy_candidates: Vec<BrainAnkhPolicyCandidateProjection>,
  routing_decisions: Vec<BrainAnkhRoutingDecisionProjection>,
  attach_decisions: Vec<BrainAnkhAxisDecisionProjection>,
  priority_decisions: Vec<BrainAnkhAxisDecisionProjection>,
  research_intents: Vec<BrainAnkhResearchIntentProjection>,
  source_candidates: Vec<BrainAnkhSourceCandidateProjection>,
  self_explanations: Vec<BrainAnkhSelfExplanationProjection>,
  policy_revision_receipts: Vec<BrainAnkhPolicyRevisionReceiptProjection>,
  mind_delta_candidates: Vec<BrainAnkhMindDeltaCandidateProjection>,
  affected_mind_slices: Vec<BrainAnkhAffectedMindSliceProjection>,
  semantic_dependency_graph: BrainAnkhSemanticDependencyGraph,
  rejudge_receipt_candidates: Vec<BrainAnkhRejudgeReceiptCandidateProjection>,
  targeted_replay_plans: Vec<BrainAnkhTargetedReplayPlanProjection>,
  proof_selection_candidates: Vec<BrainAnkhProofSelectionCandidateProjection>,
  incremental_self_compile_candidates: Vec<BrainAnkhIncrementalSelfCompileCandidateProjection>,
  system_brain_snapshot_candidates: Vec<BrainAnkhSystemBrainSnapshotCandidateProjection>,
  ankh_family_snapshot_candidates: Vec<BrainAnkhAnkhFamilySnapshotCandidateProjection>,
  mind_map_projection_candidates: Vec<BrainAnkhMindMapProjectionCandidateProjection>,
  brain_diagram_packet_candidates: Vec<BrainAnkhBrainDiagramPacketCandidateProjection>,
  dashboard_projection_candidates: Vec<BrainAnkhDashboardProjectionCandidateProjection>,
  targeted_replay: Value,
  snapshot_family: Value,
  diagram_schema: Value,
  notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainBundleRequiredInvariant {
  field: String,
  expected: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainBundleValidationReport {
  metric_version: String,
  generated_at: String,
  read_owner: String,
  status: String,
  bundle_present: bool,
  proof_ref_present: bool,
  proof_file_present: bool,
  capability_manifest_schema_present: bool,
  bundle_path: Option<String>,
  proof_path: Option<String>,
  capability_manifest_schema_path: Option<String>,
  bundle_path_exists: bool,
  proof_path_exists: bool,
  capability_manifest_schema_path_exists: bool,
  bundle_parse_error: bool,
  proof_parse_error: bool,
  capability_manifest_schema_parse_error: bool,
  expected_bundle_kind: Option<String>,
  expected_lobe_profile: Option<String>,
  expected_proof_kind: Option<String>,
  bundle_id: Option<String>,
  bundle_kind: Option<String>,
  rule_version: Option<String>,
  lobe_profile: Option<String>,
  signature_proof_ref: Option<String>,
  capability_manifest_api_version: Option<String>,
  capability_manifest_kind: Option<String>,
  capability_manifest_default_policy: Option<String>,
  migration_contract_kind: Option<String>,
  migration_same_query_held_reopen: Option<bool>,
  migration_central_service_required: Option<bool>,
  migration_judge_owner: Option<String>,
  proof_same_query_held_reopen: Option<bool>,
  proof_central_service_required: Option<bool>,
  proof_judge_owner: Option<String>,
  proof_file_api_version: Option<String>,
  proof_file_kind: Option<String>,
  proof_file_bundle_id: Option<String>,
  proof_file_same_query_held_reopen: Option<bool>,
  proof_file_central_service_required: Option<bool>,
  proof_file_judge_owner: Option<String>,
  proof_file_lobe_profile: Option<String>,
  no_second_judge: Option<bool>,
  missing_fields: Vec<String>,
  invariant_failures: Vec<String>,
  grant_scope_total: usize,
  grant_scopes: Vec<String>,
  capability_total: usize,
  capabilities: Vec<String>,
  notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrainBundleContractReport {
  metric_version: String,
  generated_at: String,
  read_owner: String,
  contract_version: String,
  abi_name: String,
  bundle_kinds: Vec<String>,
  portable_profile_kinds: Vec<String>,
  required_manifest_fields: Vec<String>,
  required_capability_manifest_fields: Vec<String>,
  required_migration_contract_fields: Vec<String>,
  required_proof_fields: Vec<String>,
  required_invariants: Vec<BrainBundleRequiredInvariant>,
  judgement_owner: String,
  bundle_proof_judge_owner_token: String,
  correctness_requires_central_service: bool,
  example_validation: BrainBundleValidationReport,
  notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateReadLookupPreview {
  lookup_status: String,
  result_source: String,
  adapter_mode: String,
  query_owner: String,
  selection_owner: String,
  query_fingerprint: String,
  result_fingerprint: String,
  ranked_candidate_ids: Vec<String>,
  fact_total: usize,
  facts: Vec<GateReadLookupFact>,
}

#[derive(Debug, Clone, Serialize)]
struct AcceptedFactRecord {
  subj: String,
  pred: String,
  obj: String,
  confidence: f64,
  provenance_refs: Vec<String>,
  searchable: String,
}

#[derive(Debug, Clone)]
struct LookupQuerySpec {
  context: String,
  predicate: Option<String>,
  limit: usize,
  min_confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
struct RecipeRecord {
  recipe_id: String,
  tool_name: String,
  context: String,
  arg_predicates: Vec<String>,
  confidence: f64,
  confidence_raw: String,
  subject: String,
  past_failure_summary: String,
  recommended_sequence: Vec<String>,
  steps: Vec<String>,
  source_id: Option<String>,
  source_version: Option<String>,
  source_checksum: Option<String>,
  entity_key: Option<String>,
  member_path: Option<String>,
  rule_version: Option<String>,
  path_diff_kind: Option<String>,
  old_path: Option<String>,
  new_path: Option<String>,
  migration_epoch: Option<String>,
  supersedes: Vec<String>,
  invalidated_by: Vec<String>,
  validated_by_outcome: Vec<String>,
  conflicts_with: Vec<String>,
  hold_reason: Option<String>,
  last_effective_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecipeMatchReason {
  predicate_hit_total: usize,
  #[serde(skip_serializing_if = "Option::is_none")]
  tool_match: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  context_match: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecipeMetadata {
  recipe_id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  source_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  source_version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  source_checksum: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  entity_key: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  member_path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  rule_version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  path_diff_kind: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  old_path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  new_path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  migration_epoch: Option<String>,
  supersedes: Vec<String>,
  conflicts_with: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  last_effective_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecipeBlockedMetadata {
  recipe_id: String,
  lineage_status: String,
  invalidated_by: Vec<String>,
  validated_by_outcome: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  hold_reason: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  source_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  source_version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  source_checksum: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  entity_key: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  member_path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  rule_version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  path_diff_kind: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  old_path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  new_path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  migration_epoch: Option<String>,
  supersedes: Vec<String>,
  conflicts_with: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  last_effective_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecipeWarningBlock {
  packet_kind: String,
  warning_kind: String,
  recipe_id: String,
  subject: String,
  confidence: String,
  past_failure_summary: String,
  recommended_sequence: Vec<String>,
  steps: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  source_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  source_version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  source_checksum: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  entity_key: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  member_path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  rule_version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  path_diff_kind: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  old_path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  new_path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  migration_epoch: Option<String>,
  supersedes: Vec<String>,
  conflicts_with: Vec<String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  invalidated_by: Vec<String>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  validated_by_outcome: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  hold_reason: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  last_effective_at: Option<String>,
  match_reason: RecipeMatchReason,
  source_rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StaleRepeatCandidate {
  recipe_id: String,
  observed_failure_total: usize,
  invalidated_by: Vec<String>,
  validated_by_outcome: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecipeMatchReport {
  metric_version: String,
  generated_at: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  read_owner: Option<String>,
  tool_name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  context: Option<String>,
  arg_predicates: Vec<String>,
  min_confidence: f64,
  limit: usize,
  repair_recipe_doc_path: String,
  repair_recipe_doc_ok: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  override_recipe_id: Option<String>,
  override_blocked_recipe_ids: Vec<String>,
  stale_repeat_threshold: usize,
  signature_matched_recipe_ids: Vec<String>,
  held_recipe_ids: Vec<String>,
  invalidated_recipe_ids: Vec<String>,
  lineage_invalidated_recipe_ids: Vec<String>,
  stale_retired_recipe_ids: Vec<String>,
  stale_repeat_candidates: Vec<StaleRepeatCandidate>,
  conflict_blocked_recipe_ids: Vec<String>,
  blocked_recipe_ids: Vec<String>,
  match_status: String,
  evaluated_recipe_total: usize,
  match_total: usize,
  matched_recipe_ids: Vec<String>,
  matched_recipe_metadata: Vec<RecipeMetadata>,
  blocked_recipe_metadata: Vec<RecipeBlockedMetadata>,
  warning_blocks: Vec<RecipeWarningBlock>,
  notes: Vec<String>,
}

pub(super) fn run_gate_read(args: &Args, verb: &GateReadVerb) -> Result<i32> {
  match verb {
    GateReadVerb::Missing => {
      eprintln!("{}", GATE_READ_USAGE);
      Ok(EX_USAGE)
    }
    GateReadVerb::Unknown(raw) => {
      eprintln!("pnix gate-read: unknown subcommand: {:?}", raw);
      eprintln!("{}", GATE_READ_USAGE);
      Ok(EX_USAGE)
    }
    GateReadVerb::Help => {
      eprintln!("{}", GATE_READ_USAGE);
      Ok(EX_OK)
    }
    GateReadVerb::Status => {
      let report = gate_status_report()?;
      print_json_or_text(args.output_format, &report, render_gate_status_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::StateSinkContract => {
      let report = state_sink_contract_report()?;
      print_json_or_text(
        args.output_format,
        &report,
        render_state_sink_contract_report,
      )?;
      Ok(EX_OK)
    }
    GateReadVerb::OntologyCoverage => {
      let report = ontology_coverage_report()?;
      print_json_or_text(args.output_format, &report, render_ontology_coverage_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::MeaningBridges => {
      let report = meaning_bridges_report()?;
      print_json_or_text(args.output_format, &report, render_meaning_bridges_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::SelfCapabilities => {
      let report = self_capabilities_report()?;
      print_json_or_text(args.output_format, &report, render_self_capabilities_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::MetaProtocols => {
      let report = meta_protocols_report()?;
      print_json_or_text(args.output_format, &report, render_meta_protocols_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::LiftRuleCoverage => {
      let report = lift_rule_coverage_report()?;
      print_json_or_text(
        args.output_format,
        &report,
        render_lift_rule_coverage_report,
      )?;
      Ok(EX_OK)
    }
    GateReadVerb::StoreBudget => {
      let report = store_budget_report()?;
      print_json_or_text(args.output_format, &report, render_store_budget_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::ArtifactRefRatio => {
      let report = artifact_ref_ratio_report()?;
      print_json_or_text(
        args.output_format,
        &report,
        render_artifact_ref_ratio_report,
      )?;
      Ok(EX_OK)
    }
    GateReadVerb::StorageTelemetry => {
      let report = storage_telemetry_report()?;
      print_json_or_text(args.output_format, &report, render_storage_telemetry_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::ProvenanceFloor => {
      let report = provenance_floor_report()?;
      print_json_or_text(args.output_format, &report, render_provenance_floor_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::UnsupportedKindFloor => {
      let report = unsupported_kind_floor_report()?;
      print_json_or_text(
        args.output_format,
        &report,
        render_unsupported_kind_floor_report,
      )?;
      Ok(EX_OK)
    }
    GateReadVerb::LineageFloor => {
      let report = lineage_floor_report(args)?;
      print_json_or_text(args.output_format, &report, render_lineage_floor_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::RecentEvents => {
      let report = recent_events_report(args)?;
      print_json_or_text(args.output_format, &report, render_recent_events_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::Candidates => {
      let report = candidate_listing_report(args)?;
      print_json_or_text(args.output_format, &report, render_candidates_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::BrainAnkhPolicy => {
      let report = brain_ankh_policy_projection_report(args)?;
      print_json_or_text(
        args.output_format,
        &report,
        render_brain_ankh_policy_projection_report,
      )?;
      Ok(EX_OK)
    }
    GateReadVerb::BrainBundleContract => {
      let report = brain_bundle_contract_report()?;
      print_json_or_text(
        args.output_format,
        &report,
        render_brain_bundle_contract_report,
      )?;
      Ok(EX_OK)
    }
    GateReadVerb::ValidateBrainBundle => {
      let report = brain_bundle_validation_report_from_args(args)?;
      print_json_or_text(
        args.output_format,
        &report,
        render_brain_bundle_validation_report,
      )?;
      Ok(EX_OK)
    }
    GateReadVerb::CurriculumCurrentTarget => {
      let report = curriculum_current_target_report()?;
      print_json_or_text(args.output_format, &report, render_curriculum_target)?;
      Ok(EX_OK)
    }
    GateReadVerb::OntologyLookupRelated => {
      let report = ontology_lookup_related_report(args)?;
      print_json_or_text(args.output_format, &report, render_lookup_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::RecipeMatchCurrent => {
      let report = recipe_match_current_report(args)?;
      print_json_or_text(args.output_format, &report, render_recipe_match_report)?;
      Ok(EX_OK)
    }
    GateReadVerb::QueryContext => {
      let report = query_context_report(args)?;
      print_json_or_text(args.output_format, &report, render_query_context_report)?;
      Ok(EX_OK)
    }
  }
}

fn print_json_or_text<T, F>(format: OutputFormat, report: &T, render: F) -> Result<()>
where
  T: Serialize,
  F: Fn(&T) -> String,
{
  match format {
    OutputFormat::Json => println!("{}", serde_json::to_string_pretty(report)?),
    OutputFormat::Text => println!("{}", render(report)),
  }
  Ok(())
}

fn render_curriculum_target(report: &CurriculumCurrentTargetReport) -> String {
  format!(
    "target: {}\nstate: {}\nanchor: {}\nprogress: {}\nnext-candidates: {}\nresolution: {}",
    report.target,
    report
      .target_state
      .clone()
      .unwrap_or_else(|| "unknown".to_string()),
    report
      .target_anchor_path
      .clone()
      .unwrap_or_else(|| "unknown".to_string()),
    report
      .progress_ratio
      .as_ref()
      .map(Value::to_string)
      .unwrap_or_else(|| "null".to_string()),
    if report.next_candidates.is_empty() {
      "[]".to_string()
    } else {
      report.next_candidates.join(", ")
    },
    report
      .target_resolution
      .clone()
      .unwrap_or_else(|| "unknown".to_string())
  )
}

fn render_gate_status_report(report: &GateReadStatusReport) -> String {
  format!(
    "status-owner: {}\nstore-root: {}\nevents: {}\ncandidates: {}",
    report.status_owner, report.store_root, report.event_total, report.candidate_total
  )
}

fn render_state_sink_contract_report(report: &StateSinkContractReport) -> String {
  format!(
    "read-owner: {}\nabi-name: {}\nprofile-kind: {}\nready: {}\npresent-total: {}\nstatuses: {}",
    report.read_owner,
    report.abi_name,
    report.store_profile_kind,
    report.state_sink_ready,
    report.state_sink_present_total,
    report.lifecycle_statuses.join(", ")
  )
}

fn render_ontology_coverage_report(report: &OntologyCoverageReport) -> String {
  let mut lines = vec![
    format!("read-owner: {}", report.read_owner),
    format!("score: {:.4}", report.score),
    format!("parse-pass-rate: {:.4}", report.parse_pass_rate),
    format!("concept-total: {}", report.breadth.concept_total),
    format!("domain-total: {}", report.breadth.domain_total),
    format!("triple-total: {}", report.breadth.triple_total),
    format!(
      "meaning-bridge-total: {}",
      report.breadth.meaning_bridge_total
    ),
    format!(
      "meta-protocol-total: {}",
      report.breadth.meta_protocol_total
    ),
    format!("intent-route-total: {}", report.breadth.intent_route_total),
    format!(
      "self-capability-total: {}",
      report.breadth.self_capability_total
    ),
    format!("recipe-total: {}", report.breadth.recipe_total),
    format!("lift-rule-total: {}", report.breadth.lift_rule_total),
    format!("tool-spec-total: {}", report.breadth.tool_spec_total),
  ];
  for entry in &report.file_scores {
    lines.push(format!("file-score:{}={:.4}", entry.id, entry.score));
  }
  lines.extend(report.notes.iter().map(|note| format!("note: {}", note)));
  lines.join("\n")
}

fn render_meaning_bridges_report(report: &MeaningBridgesReport) -> String {
  let mut lines = vec![
    format!("read-owner: {}", report.read_owner),
    format!("score: {:.4}", report.score),
    format!("bridge-total: {}", report.bridge_total),
    format!("complete-total: {}", report.complete_total),
    format!("roundtrip-ready-total: {}", report.roundtrip_ready_total),
    format!("avg-latent-link-total: {:.4}", report.avg_latent_link_total),
    format!(
      "meta-bridge-count: {}",
      report
        .meta_bridge_count
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
    ),
    format!(
      "meta-focus: {}",
      report.meta_focus.as_deref().unwrap_or("none")
    ),
    format!("report-total: {}", report.reports.len()),
    format!("issue-total: {}", report.issues.len()),
  ];
  lines.extend(report.notes.iter().map(|note| format!("note: {}", note)));
  lines.join("\n")
}

fn render_self_capabilities_report(report: &SelfCapabilitiesReport) -> String {
  let mut lines = vec![
    format!("read-owner: {}", report.read_owner),
    format!("score: {:.4}", report.score),
    format!("capability-total: {}", report.capability_total),
    format!("complete-total: {}", report.complete_total),
    format!("self-referential-total: {}", report.self_referential_total),
    format!(
      "meta-capability-count: {}",
      report
        .meta_capability_count
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
    ),
    format!(
      "meta-priority: {}",
      report.meta_priority.as_deref().unwrap_or("none")
    ),
    format!("report-total: {}", report.reports.len()),
    format!("issue-total: {}", report.issues.len()),
  ];
  lines.extend(report.notes.iter().map(|note| format!("note: {}", note)));
  lines.join("\n")
}

fn render_meta_protocols_report(report: &MetaProtocolsReport) -> String {
  let mut lines = vec![
    format!("read-owner: {}", report.read_owner),
    format!("score: {:.4}", report.score),
    format!("protocol-total: {}", report.protocol_total),
    format!("complete-total: {}", report.complete_total),
    format!("reuse-ready-total: {}", report.reuse_ready_total),
    format!(
      "meta-protocol-count: {}",
      report
        .meta_protocol_count
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
    ),
    format!(
      "meta-focus: {}",
      report.meta_focus.as_deref().unwrap_or("none")
    ),
    format!("report-total: {}", report.reports.len()),
    format!("issue-total: {}", report.issues.len()),
  ];
  lines.extend(report.notes.iter().map(|note| format!("note: {}", note)));
  lines.join("\n")
}

fn render_lift_rule_coverage_report(report: &LiftRuleCoverageReport) -> String {
  let mut lines = vec![
    format!("read-owner: {}", report.read_owner),
    format!("status: {}", report.status),
    format!("score: {:.1}", report.score),
    format!("rule-total: {}", report.rule_total),
    format!("complete-total: {}", report.complete_total),
    format!("ready-total: {}", report.ready_total),
    format!("contextual-fact-total: {}", report.contextual_fact_total),
    format!("canonical-kind-total: {}", report.canonical_kind_total),
    format!(
      "missing-kinds: {}",
      join_or_none(
        &report
          .missing_kinds
          .iter()
          .map(String::as_str)
          .collect::<Vec<_>>()
      )
    ),
    format!(
      "duplicate-kinds: {}",
      join_or_none(
        &report
          .duplicate_kinds
          .iter()
          .map(String::as_str)
          .collect::<Vec<_>>()
      )
    ),
    format!(
      "extra-kinds: {}",
      join_or_none(
        &report
          .extra_kinds
          .iter()
          .map(String::as_str)
          .collect::<Vec<_>>()
      )
    ),
    format!(
      "constructor: {}",
      report.constructor.as_deref().unwrap_or("none")
    ),
    format!(
      "evaluation-shape: {}",
      report.evaluation_shape.as_deref().unwrap_or("none")
    ),
  ];
  lines.extend(report.notes.iter().map(|note| format!("note: {}", note)));
  lines.join("\n")
}

fn render_store_budget_report(report: &StoreBudgetReport) -> String {
  format!(
    "read-owner: {}\nstatus: {}\nbudget-bytes: {}\neffective-hot-bytes: {}\nremaining-bytes: {}\ninline-blob-mode: {}\ncheckpoint-total: {}",
    report.read_owner,
    report.status,
    report.hot_store_budget_bytes,
    report.effective_hot_store_bytes,
    report.budget_remaining_bytes,
    report.inline_blob_mode,
    report.checkpoint_total
  )
}

fn render_artifact_ref_ratio_report(report: &ArtifactRefCoverageReport) -> String {
  format!(
    "read-owner: {}\nrecord-total: {}\nrecord-ratio: {:.4}\ncandidate-ratio: {:.4}\nstate-sink-ratio: {:.4}\nfield-total: {}\nstatuses: {}",
    report.read_owner,
    report.record_total,
    report.record_ratio,
    report.candidate_record_ratio,
    report.state_sink_record_ratio,
    report.field_counts.len(),
    report
      .status_counts
      .keys()
      .cloned()
      .collect::<Vec<_>>()
      .join(", ")
  )
}

fn render_storage_telemetry_report(report: &StorageTelemetryReport) -> String {
  format!(
    "read-owner: {}\nstore-bytes-total: {}\nhot-store-bytes: {}\nwarm-store-bytes: {}\ncold-store-bytes: {}\nartifact-ref-ratio: {:.4}\nttl-violation-count: {}\ndangling-ref-count: {}\nstate-sink-ready: {}",
    report.read_owner,
    report.store_bytes_total,
    report.hot_store_bytes,
    report.warm_store_bytes,
    report.cold_store_bytes,
    report.artifact_ref_ratio,
    report
      .ttl_violation_count
      .map(|value| value.to_string())
      .unwrap_or_else(|| "null".to_string()),
    report
      .dangling_ref_count
      .map(|value| value.to_string())
      .unwrap_or_else(|| "null".to_string()),
    report.state_sink_ready
  )
}

fn render_provenance_floor_report(report: &ProvenanceFloorReport) -> String {
  format!(
    "read-owner: {}\nstatus: {}\naccepted-total: {}\nweak-total: {}\nweak-accepted-total: {}\nquarantined-missing-provenance-total: {}",
    report.read_owner,
    report.status,
    report.accepted_total,
    report.weak_total,
    report.weak_accepted_total,
    report.quarantined_missing_provenance_total
  )
}

fn render_unsupported_kind_floor_report(report: &UnsupportedKindFloorReport) -> String {
  format!(
    "read-owner: {}\nstatus: {}\nunsupported-kind-total: {}\nunsupported-kind-leaking-total: {}\nunsupported-kind-quarantine-rate: {:.4}\nunsupported-kind-inventory: {}",
    report.read_owner,
    report.status,
    report.unsupported_kind_total,
    report.unsupported_kind_leaking_total,
    report.unsupported_kind_quarantine_rate,
    if report.unsupported_kind_inventory.is_empty() {
      "[]".to_string()
    } else {
      report.unsupported_kind_inventory.join(", ")
    }
  )
}

fn render_lineage_floor_report(report: &LineageFloorReport) -> String {
  format!(
    "read-owner: {}\nstatus: {}\nrecord-total: {}\nweak-total: {}\npassed-total: {}\npartial-total: {}\nmissing-source-context-total: {}\nlineage-anchor-required-total: {}\nmissing-source-session-total: {}\nmissing-source-turn-total: {}\nmissing-derived-from-candidate-total: {}\nmissing-parent-packet-total: {}\nmissing-floor-status-total: {}\nstale-floor-status-total: {}\nunparseable-attrset-total: {}\nsample-weak-total: {}",
    report.read_owner,
    report.status,
    report.record_total,
    report.weak_total,
    report.passed_total,
    report.partial_total,
    report.missing_source_context_total,
    report.lineage_anchor_required_total,
    report.missing_source_session_total,
    report.missing_source_turn_total,
    report.missing_derived_from_candidate_total,
    report.missing_parent_packet_total,
    report.missing_floor_status_total,
    report.stale_floor_status_total,
    report.unparseable_attrset_total,
    report.sample_weak.len()
  )
}

fn render_recent_events_report(report: &GateReadRecentEventsReport) -> String {
  format!(
    "read-owner: {}\nevent-types: {}\nevent-total: {}",
    report.read_owner,
    if report.event_types.is_empty() {
      "[]".to_string()
    } else {
      report.event_types.join(", ")
    },
    report.event_total
  )
}

fn render_candidates_report(report: &GateReadCandidatesReport) -> String {
  format!(
    "read-owner: {}\nkind-filter: {}\ncandidate-total: {}",
    report.read_owner,
    report
      .kind_filter
      .clone()
      .unwrap_or_else(|| "all".to_string()),
    report.candidate_total
  )
}

fn render_brain_ankh_policy_projection_report(report: &BrainAnkhPolicyProjectionReport) -> String {
  format!(
    "read-owner: {}\nprojection-status: {}\npolicy-candidates: {}\nrouting-decisions: {}\nattach-decisions: {}\npriority-decisions: {}\nresearch-intents: {}\nsource-candidates: {}\nself-explanations: {}\npolicy-revision-receipts: {}\nmind-delta-candidates: {}\naffected-mind-slices: {}\nsemantic-dependency-edges: {}\nrejudge-receipt-candidates: {}\ntargeted-replay-plans: {}\nproof-selection-candidates: {}\nincremental-self-compile-candidates: {}\nsystem-brain-snapshots: {}\nankh-family-snapshots: {}\nmind-map-projections: {}\nbrain-diagram-packets: {}\ndashboard-projections: {}\nproof-reuse-allowed: {}\nstore-mutation: {}",
    report.read_owner,
    report.projection_status,
    report.policy_candidate_total,
    report.routing_decision_total,
    report.attach_decision_total,
    report.priority_decision_total,
    report.research_intent_total,
    report.source_candidate_total,
    report.self_explanation_total,
    report.policy_revision_receipt_total,
    report.mind_delta_candidate_total,
    report.affected_mind_slice_total,
    report.semantic_dependency_edge_total,
    report.rejudge_receipt_candidate_total,
    report.targeted_replay_plan_total,
    report.proof_selection_candidate_total,
    report.incremental_self_compile_candidate_total,
    report.system_brain_snapshot_candidate_total,
    report.ankh_family_snapshot_candidate_total,
    report.mind_map_projection_candidate_total,
    report.brain_diagram_packet_candidate_total,
    report.dashboard_projection_candidate_total,
    report.proof_reuse_allowed,
    report.store_mutation
  )
}

fn render_brain_bundle_contract_report(report: &BrainBundleContractReport) -> String {
  format!(
    "read-owner: {}\nabi-name: {}\nbundle-kinds: {}\nportable-profile-kinds: {}\nexample-status: {}",
    report.read_owner,
    report.abi_name,
    report.bundle_kinds.join(", "),
    report.portable_profile_kinds.join(", "),
    report.example_validation.status
  )
}

fn render_brain_bundle_validation_report(report: &BrainBundleValidationReport) -> String {
  format!(
    "read-owner: {}\nstatus: {}\nbundle-kind: {}\nlobe-profile: {}\nmissing-fields: {}\ninvariant-failures: {}",
    report.read_owner,
    report.status,
    report
      .bundle_kind
      .clone()
      .unwrap_or_else(|| "unknown".to_string()),
    report
      .lobe_profile
      .clone()
      .unwrap_or_else(|| "unknown".to_string()),
    if report.missing_fields.is_empty() {
      "[]".to_string()
    } else {
      report.missing_fields.join(", ")
    },
    if report.invariant_failures.is_empty() {
      "[]".to_string()
    } else {
      report.invariant_failures.join(", ")
    }
  )
}

fn render_lookup_report(report: &GateReadLookupReport) -> String {
  let facts = report
    .facts
    .iter()
    .map(|fact| format!("{} {} {}", fact.subj, fact.pred, fact.obj))
    .collect::<Vec<_>>()
    .join(" | ");
  format!(
    "lookup-status: {}\ncontext: {}\npredicate: {}\nfact-total: {}\nfacts: {}",
    report.lookup_status,
    report.context,
    report.predicate.clone().unwrap_or_default(),
    report.fact_total,
    facts
  )
}

fn render_query_context_report(report: &QueryContextReport) -> String {
  format!(
    "query-status: {}\ntopic: {}\naccepted-facts: {}\ncandidate-hits: {}\nevent-hits: {}\nrecipe-hits: {}",
    report.query_status,
    report.topic,
    report.accepted_fact_total,
    report.candidate_hit_total,
    report.event_hit_total,
    report.recipe_hit_total
  )
}

fn render_recipe_match_report(report: &RecipeMatchReport) -> String {
  format!(
    "match-status: {}\ntool-name: {}\ncontext: {}\nmatched-recipe-ids: {}\nblocked-recipe-ids: {}",
    report.match_status,
    report.tool_name,
    report.context.clone().unwrap_or_default(),
    if report.matched_recipe_ids.is_empty() {
      "[]".to_string()
    } else {
      report.matched_recipe_ids.join(", ")
    },
    if report.blocked_recipe_ids.is_empty() {
      "[]".to_string()
    } else {
      report.blocked_recipe_ids.join(", ")
    }
  )
}

fn curriculum_current_target_report() -> Result<CurriculumCurrentTargetReport> {
  let path = gate_store_root()?
    .join("control-plane")
    .join("curriculum-state.json");
  let raw = fs::read_to_string(&path)
    .with_context(|| format!("read curriculum state {}", path.display()))?;
  let payload: Value = serde_json::from_str(&raw)
    .with_context(|| format!("parse curriculum state {}", path.display()))?;
  parse_json_value(eval_gate_read_metrics_owner(
    "curriculumCurrentTarget",
    &json!({
      "generated_at": iso_now(),
      "read_owner": "pnix gate-read curriculum-current-target",
      "target_override": nonblank(env::var("PNIX_GATE_TARGET").ok()),
      "state": payload,
    }),
  )?)
}

fn gate_status_report() -> Result<GateReadStatusReport> {
  let store_root = gate_store_root()?;
  let events_path = store_root.join("events.jsonl");
  let candidate_dir = store_root.join("px").join("candidates");
  let event_total = read_gate_events()?.len();
  let candidate_total = collect_px_files(&candidate_dir)?.len();
  Ok(GateReadStatusReport {
    metric_version: "gate-read-status-v1",
    generated_at: iso_now(),
    status_owner: "pnix gate-read status",
    status_mode: "read-only-store-summary",
    store_root: store_root.display().to_string(),
    events_path: events_path.display().to_string(),
    candidate_dir: candidate_dir.display().to_string(),
    event_total,
    candidate_total,
    notes: vec![
      "operator shell owner is upstream pnix gate-read",
      "legacy Babashka gate_status registry entry is removed",
    ],
  })
}

fn state_sink_contract_report() -> Result<StateSinkContractReport> {
  let store_root = gate_store_root()?;
  let owner_value = eval_gate_read_metrics_owner(
    "stateSinkContract",
    &json!({
      "store_root": store_root.display().to_string(),
      "store_profile_kind": store_profile_kind(&store_root),
      "store_profile_source": store_profile_source(),
      "px_entries": directory_entry_names(&store_root.join("px"))?,
    }),
  )?;
  Ok(StateSinkContractReport {
    metric_version: "state-sink-contract-v1",
    generated_at: iso_now(),
    read_owner: "pnix gate-read state-sink-contract",
    contract_version: value_string(owner_value.get("contract_version"))
      .unwrap_or_else(|| "v1".to_string()),
    abi_name: value_string(owner_value.get("abi_name"))
      .unwrap_or_else(|| "pnix-gate-state-sink.v1".to_string()),
    store_root: value_string(owner_value.get("store_root"))
      .unwrap_or_else(|| store_root.display().to_string()),
    store_profile_kind: value_string(owner_value.get("store_profile_kind"))
      .unwrap_or_else(|| store_profile_kind(&store_root)),
    store_profile_source: value_string(owner_value.get("store_profile_source"))
      .unwrap_or_else(store_profile_source),
    materialization_kind: value_string(owner_value.get("materialization_kind"))
      .unwrap_or_else(|| "directory-tree".to_string()),
    portable_profile_kinds: value_string_list(owner_value.get("portable_profile_kinds")),
    storage_tier_contract_version: value_string(owner_value.get("storage_tier_contract_version"))
      .unwrap_or_else(|| "v1".to_string()),
    storage_tier_names: value_string_list(owner_value.get("storage_tier_names")),
    correctness_requires_central_service: owner_value
      .get("correctness_requires_central_service")
      .and_then(Value::as_bool)
      .unwrap_or(false),
    judgement_owner: value_string(owner_value.get("judgement_owner"))
      .unwrap_or_else(|| "pnix runtime/kernel".to_string()),
    lifecycle_statuses: value_string_list(owner_value.get("lifecycle_statuses")),
    status_count: owner_value.get("status_count").and_then(value_as_u64).unwrap_or(0) as usize,
    statuses: parse_state_sink_lane_specs(owner_value.get("statuses")),
    candidate_lane: owner_value
      .get("candidate_lane")
      .map(parse_state_sink_lane_spec)
      .unwrap_or_else(|| StateSinkLaneSpec {
        status: "candidate".to_string(),
        relative_path: "px/candidates".to_string(),
        materialized_path: store_root.join("px").join("candidates").display().to_string(),
        tier: "hot".to_string(),
        lifecycle_role: "candidate".to_string(),
      }),
    auxiliary_lanes: parse_state_sink_lane_specs(owner_value.get("auxiliary_lanes")),
    state_sink_ready: owner_value
      .get("state_sink_ready")
      .and_then(Value::as_bool)
      .unwrap_or(false),
    state_sink_present_total: owner_value
      .get("state_sink_present_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    state_sink_presence: value_bool_map(owner_value.get("state_sink_presence")),
    notes: vec![
      "operator shell owner is upstream pnix gate-read state-sink-contract",
      "relative paths remain the contract even when store_root changes across shared-runtime|local-embedded|domain-lobe",
      "legacy Babashka describe_state_sink_contract registry entry is removed",
    ],
  })
}

fn ontology_coverage_report() -> Result<OntologyCoverageReport> {
  let concepts_value = eval_px_file_json(&live_px_dir()?.join("concepts.px"))?;
  let observations_value = eval_px_file_json(&live_px_dir()?.join("observations.px"))?;
  let meaning_bridges_value = eval_px_file_json(&live_px_dir()?.join("meaning-bridges.px"))?;
  let meta_protocols_value = eval_px_file_json(&live_px_dir()?.join("meta-protocols.px"))?;
  let intent_routes_value = eval_px_file_json(&live_px_dir()?.join("intent-routes.px"))?;
  let self_capabilities_value = eval_px_file_json(&live_px_dir()?.join("self-capabilities.px"))?;
  let repair_recipes_value = eval_px_file_json(&live_px_dir()?.join("repair-recipes.px"))?;
  let tool_specs_value = eval_px_file_json(
    &workspace_root()?
      .join("pnix-gate")
      .join("px")
      .join("tools.px"),
  )?;
  let owner_value = eval_gate_read_metrics_owner(
    "ontologyCoverage",
    &json!({
      "concepts": concepts_value,
      "observations": observations_value,
      "meaning_bridges": meaning_bridges_value,
      "meta_protocols": meta_protocols_value,
      "intent_routes": intent_routes_value,
      "self_capabilities": self_capabilities_value,
      "repair_recipes": repair_recipes_value,
      "lift_rules": eval_px_file_json(&live_px_dir()?.join("lift-rules.px"))?,
      "tool_specs": tool_specs_value,
    }),
  )?;
  let file_score_map = owner_value
    .get("file_scores")
    .and_then(Value::as_object)
    .cloned()
    .unwrap_or_default();
  let breadth = owner_value
    .get("breadth")
    .and_then(Value::as_object)
    .cloned()
    .unwrap_or_default();
  let score_for = |key: &str| {
    file_score_map
      .get(key)
      .and_then(Value::as_f64)
      .unwrap_or(0.0)
  };
  let file_scores = vec![
    OntologyCoverageFileScore {
      id: "concepts",
      score: score_for("concepts"),
    },
    OntologyCoverageFileScore {
      id: "observations",
      score: score_for("observations"),
    },
    OntologyCoverageFileScore {
      id: "meaning_bridges",
      score: score_for("meaning_bridges"),
    },
    OntologyCoverageFileScore {
      id: "meta_protocols",
      score: score_for("meta_protocols"),
    },
    OntologyCoverageFileScore {
      id: "intent_routes",
      score: score_for("intent_routes"),
    },
    OntologyCoverageFileScore {
      id: "self_capabilities",
      score: score_for("self_capabilities"),
    },
    OntologyCoverageFileScore {
      id: "repair_recipes",
      score: score_for("repair_recipes"),
    },
    OntologyCoverageFileScore {
      id: "lift_rules",
      score: score_for("lift_rules"),
    },
    OntologyCoverageFileScore {
      id: "tool_specs",
      score: score_for("tool_specs"),
    },
  ];

  Ok(OntologyCoverageReport {
    generated_at: iso_now(),
    metric_version: "m9-v5",
    read_owner: "pnix gate-read ontology-coverage",
    score: owner_value
      .get("score")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    parse_pass_rate: owner_value
      .get("parse_pass_rate")
      .and_then(Value::as_f64)
      .unwrap_or(1.0),
    file_scores,
    breadth: OntologyCoverageBreadth {
      concept_total: breadth
        .get("concept_total")
        .and_then(value_as_u64)
        .unwrap_or(0) as usize,
      domain_total: breadth
        .get("domain_total")
        .and_then(value_as_u64)
        .unwrap_or(0) as usize,
      triple_total: breadth
        .get("triple_total")
        .and_then(value_as_u64)
        .unwrap_or(0) as usize,
      meaning_bridge_total: breadth
        .get("meaning_bridge_total")
        .and_then(value_as_u64)
        .unwrap_or(0) as usize,
      meta_protocol_total: breadth
        .get("meta_protocol_total")
        .and_then(value_as_u64)
        .unwrap_or(0) as usize,
      intent_route_total: breadth
        .get("intent_route_total")
        .and_then(value_as_u64)
        .unwrap_or(0) as usize,
      self_capability_total: breadth
        .get("self_capability_total")
        .and_then(value_as_u64)
        .unwrap_or(0) as usize,
      recipe_total: breadth
        .get("recipe_total")
        .and_then(value_as_u64)
        .unwrap_or(0) as usize,
      lift_rule_total: breadth
        .get("lift_rule_total")
        .and_then(value_as_u64)
        .unwrap_or(0) as usize,
      tool_spec_total: breadth
        .get("tool_spec_total")
        .and_then(value_as_u64)
        .unwrap_or(0) as usize,
    },
    notes: vec![
      "Coverage score measures structural completeness of parseable `.px` files",
      "This score does not claim semantic truth or model-weight learning",
    ],
  })
}

fn meaning_bridges_report() -> Result<MeaningBridgesReport> {
  let value = eval_px_file_json(&live_px_dir()?.join("meaning-bridges.px"))?;
  let owner_value = eval_gate_read_metrics_owner("meaningBridges", &value)?;

  Ok(MeaningBridgesReport {
    generated_at: iso_now(),
    metric_version: "m9-v2",
    read_owner: "pnix gate-read meaning-bridges",
    score: owner_value
      .get("score")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    bridge_total: owner_value
      .get("bridge_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    complete_total: owner_value
      .get("complete_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    roundtrip_ready_total: owner_value
      .get("roundtrip_ready_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    avg_latent_link_total: owner_value
      .get("avg_latent_link_total")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    meta_bridge_count: owner_value
      .get("meta_bridge_count")
      .and_then(value_as_u64)
      .map(|value| value as usize),
    meta_focus: value_string(owner_value.get("meta_focus")),
    reports: owner_value
      .get("reports")
      .and_then(Value::as_array)
      .cloned()
      .unwrap_or_default(),
    issues: owner_value
      .get("issues")
      .and_then(Value::as_array)
      .cloned()
      .unwrap_or_default(),
    notes: vec![
      "Meaning bridge score measures whether A/B surfaces are connected by explicit latent cause",
      "It is still a structural score, not a truth guarantee",
    ],
  })
}

fn self_capabilities_report() -> Result<SelfCapabilitiesReport> {
  let value = eval_px_file_json(&live_px_dir()?.join("self-capabilities.px"))?;
  let owner_value = eval_gate_read_metrics_owner("selfCapabilities", &value)?;

  Ok(SelfCapabilitiesReport {
    generated_at: iso_now(),
    metric_version: "m9-v4",
    read_owner: "pnix gate-read self-capabilities",
    score: owner_value
      .get("score")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    capability_total: owner_value
      .get("capability_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    complete_total: owner_value
      .get("complete_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    self_referential_total: owner_value
      .get("self_referential_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    meta_capability_count: owner_value
      .get("meta_capability_count")
      .and_then(value_as_u64)
      .map(|value| value as usize),
    meta_priority: value_string(owner_value.get("meta_priority")),
    reports: owner_value
      .get("reports")
      .and_then(Value::as_array)
      .cloned()
      .unwrap_or_default(),
    issues: owner_value
      .get("issues")
      .and_then(Value::as_array)
      .cloned()
      .unwrap_or_default(),
    notes: vec![
      "Self capability score measures whether pnix-gate explicitly models its own analysis tools",
      "It rewards self-reference, introspection, and upgrade paths",
    ],
  })
}

fn meta_protocols_report() -> Result<MetaProtocolsReport> {
  let value = eval_px_file_json(&live_px_dir()?.join("meta-protocols.px"))?;
  let owner_value = eval_gate_read_metrics_owner("metaProtocols", &value)?;

  Ok(MetaProtocolsReport {
    generated_at: iso_now(),
    metric_version: "m9-v4",
    read_owner: "pnix gate-read meta-protocols",
    score: owner_value.get("score").and_then(Value::as_f64).unwrap_or(0.0),
    protocol_total: owner_value
      .get("protocol_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    complete_total: owner_value
      .get("complete_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    reuse_ready_total: owner_value
      .get("reuse_ready_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    meta_protocol_count: owner_value
      .get("meta_protocol_count")
      .and_then(value_as_u64)
      .map(|value| value as usize),
    meta_focus: value_string(owner_value.get("meta_focus")),
    reports: owner_value
      .get("reports")
      .and_then(Value::as_array)
      .cloned()
      .unwrap_or_default(),
    issues: owner_value
      .get("issues")
      .and_then(Value::as_array)
      .cloned()
      .unwrap_or_default(),
    notes: vec![
      "Meta protocol score measures whether babashka and pnix eval are connected by an explicit reuse contract",
      "This is still readiness scoring, not proof of full pnix eval execution",
    ],
  })
}

fn lift_rule_coverage_report() -> Result<LiftRuleCoverageReport> {
  let path = live_px_dir()?.join("lift-rules.px");
  let owner_value = eval_gate_read_metrics_owner("liftRuleCoverage", &eval_px_file_json(&path)?)?;

  Ok(LiftRuleCoverageReport {
    generated_at: iso_now(),
    metric_version: "m9-v1",
    read_owner: "pnix gate-read lift-rule-coverage",
    status: value_string(owner_value.get("status")).unwrap_or_else(|| "partial".to_string()),
    score: owner_value.get("score").and_then(Value::as_f64).unwrap_or(0.0),
    rule_total: owner_value.get("rule_total").and_then(value_as_u64).unwrap_or(0) as usize,
    complete_total: owner_value
      .get("complete_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    ready_total: owner_value.get("ready_total").and_then(value_as_u64).unwrap_or(0) as usize,
    contextual_fact_total: owner_value
      .get("contextual_fact_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    canonical_kind_total: owner_value
      .get("canonical_kind_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    missing_kinds: value_string_list(owner_value.get("missing_kinds")),
    duplicate_kinds: value_string_list(owner_value.get("duplicate_kinds")),
    extra_kinds: value_string_list(owner_value.get("extra_kinds")),
    constructor: value_string(owner_value.get("constructor")),
    evaluation_shape: value_string(owner_value.get("evaluation_shape")),
    notes: vec![
      "lift-rules owner surface should cover the canonical 7 candidate kinds exactly once",
      "all canonical rules should target ContextualFact and stay declared as mkLiftRule + builtins.map rules",
    ],
  })
}

fn store_budget_report() -> Result<StoreBudgetReport> {
  let store_root = gate_store_root()?;
  let control_plane_dir = store_root.join("control-plane");
  let candidate_dir = store_root.join("px").join("candidates");
  let events_path = store_root.join("events.jsonl");
  let live_buffer = live_buffer_pressure_summary(&control_plane_dir.join("live-buffers.json"))?;
  let hot_store_budget_bytes = env::var("PNIX_GATE_HOT_STORE_BUDGET_BYTES")
    .ok()
    .and_then(|raw| raw.trim().parse::<u64>().ok())
    .unwrap_or(DEFAULT_HOT_STORE_BUDGET_BYTES);
  let checkpoint_summary = hot_store_budget_checkpoint_summary()?;
  let owner_value = eval_gate_storage_telemetry_owner(
    "storeBudget",
    &json!({
      "hot_store_budget_bytes": hot_store_budget_bytes,
      "candidate_store_bytes": path_total_bytes(&candidate_dir)?,
      "control_plane_bytes": path_total_bytes(&control_plane_dir)?,
      "events_bytes": path_total_bytes(&events_path)?,
      "pending_inline_bytes": 0,
      "live_buffer": {
        "live_buffer_open_count": live_buffer.live_buffer_open_count,
        "live_buffer_dirty_count": live_buffer.live_buffer_dirty_count,
        "live_buffer_error_count": live_buffer.live_buffer_error_count,
        "live_buffer_open_bytes": live_buffer.live_buffer_open_bytes,
        "live_buffer_dirty_bytes": live_buffer.live_buffer_dirty_bytes,
        "live_buffer_parse_pass_rate": live_buffer.live_buffer_parse_pass_rate,
        "snapshot_updated_at": live_buffer.snapshot_updated_at,
        "snapshot_source": live_buffer.snapshot_source,
      },
      "checkpoint": {
        "checkpoint_total": checkpoint_summary.checkpoint_total,
        "budget_exceeded_total": checkpoint_summary.budget_exceeded_total,
        "suppressed_total": checkpoint_summary.suppressed_total,
        "session_total": checkpoint_summary.session_total,
        "latest_recorded_at": checkpoint_summary.latest_recorded_at,
        "latest_inline_blob_mode": checkpoint_summary.latest_inline_blob_mode,
        "latest_budget_exceeded": checkpoint_summary.latest_budget_exceeded,
        "latest_hot_store_bytes": checkpoint_summary.latest_hot_store_bytes,
        "latest_effective_hot_store_bytes": checkpoint_summary.latest_effective_hot_store_bytes,
        "latest_budget_remaining_bytes": checkpoint_summary.latest_budget_remaining_bytes,
        "latest_pressure_ratio": checkpoint_summary.latest_pressure_ratio,
      },
    }),
  )?;
  Ok(StoreBudgetReport {
    metric_version: "store-budget-v1",
    generated_at: iso_now(),
    read_owner: "pnix gate-read store-budget",
    status: value_string(owner_value.get("status"))
      .unwrap_or_else(|| checkpoint_summary.status.clone()),
    hot_store_budget_bytes: owner_value
      .get("hot_store_budget_bytes")
      .and_then(value_as_u64)
      .unwrap_or(hot_store_budget_bytes),
    hot_store_bytes: owner_value
      .get("hot_store_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    effective_hot_store_bytes: owner_value
      .get("effective_hot_store_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    candidate_store_bytes: owner_value
      .get("candidate_store_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    control_plane_bytes: owner_value
      .get("control_plane_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    events_bytes: owner_value.get("events_bytes").and_then(value_as_u64).unwrap_or(0),
    pending_inline_bytes: owner_value
      .get("pending_inline_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_open_bytes: owner_value
      .get("live_buffer_open_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_dirty_bytes: owner_value
      .get("live_buffer_dirty_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_open_count: owner_value
      .get("live_buffer_open_count")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_dirty_count: owner_value
      .get("live_buffer_dirty_count")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_error_count: owner_value
      .get("live_buffer_error_count")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_parse_pass_rate: owner_value
      .get("live_buffer_parse_pass_rate")
      .and_then(Value::as_f64),
    live_buffer_snapshot_updated_at: value_string(owner_value.get("live_buffer_snapshot_updated_at")),
    live_buffer_snapshot_source: value_string(owner_value.get("live_buffer_snapshot_source")),
    budget_remaining_bytes: owner_value
      .get("budget_remaining_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    budget_exceeded: owner_value
      .get("budget_exceeded")
      .and_then(Value::as_bool)
      .unwrap_or(false),
    pressure_ratio: owner_value.get("pressure_ratio").and_then(Value::as_f64),
    inline_blob_mode: value_string(owner_value.get("inline_blob_mode"))
      .unwrap_or_else(|| "bounded-preview".to_string()),
    checkpoint_total: owner_value
      .get("checkpoint_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    budget_exceeded_total: owner_value
      .get("budget_exceeded_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    suppressed_total: owner_value
      .get("suppressed_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    session_total: owner_value
      .get("session_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    latest_recorded_at: value_string(owner_value.get("latest_recorded_at")),
    latest_inline_blob_mode: value_string(owner_value.get("latest_inline_blob_mode")),
    latest_budget_exceeded: owner_value
      .get("latest_budget_exceeded")
      .and_then(Value::as_bool),
    latest_hot_store_bytes: owner_value.get("latest_hot_store_bytes").and_then(value_as_u64),
    latest_effective_hot_store_bytes: owner_value
      .get("latest_effective_hot_store_bytes")
      .and_then(value_as_u64),
    latest_budget_remaining_bytes: owner_value
      .get("latest_budget_remaining_bytes")
      .and_then(value_as_u64),
    latest_pressure_ratio: owner_value
      .get("latest_pressure_ratio")
      .and_then(Value::as_f64),
    notes: vec![
      "operator shell owner is upstream pnix gate-read store-budget",
      "events.jsonl hot lane is counted in the same denominator as candidates and control-plane snapshots",
      "dirty live-buffer bytes are treated as projected hot pressure before the next snapshot flush",
      "legacy Babashka measure_store_budget registry entry is removed",
    ],
  })
}

fn artifact_ref_ratio_report() -> Result<ArtifactRefCoverageReport> {
  let store_root = gate_store_root()?;
  let records = collect_artifact_ref_records(&store_root)?;
  let owner_value = eval_gate_storage_telemetry_owner(
    "artifactRefRatio",
    &json!({
      "records": records.iter().map(|record| {
        json!({
          "status": record.status,
          "artifact_ref_fields": record.artifact_ref_fields,
          "has_artifact_ref": record.has_artifact_ref,
        })
      }).collect::<Vec<_>>(),
    }),
  )?;
  Ok(ArtifactRefCoverageReport {
    metric_version: "artifact-ref-coverage-v1",
    generated_at: iso_now(),
    read_owner: "pnix gate-read artifact-ref-ratio",
    record_total: owner_value
      .get("record_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    with_artifact_ref_total: owner_value
      .get("with_artifact_ref_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    record_ratio: owner_value
      .get("record_ratio")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    candidate_record_total: owner_value
      .get("candidate_record_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    candidate_with_artifact_ref_total: owner_value
      .get("candidate_with_artifact_ref_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    candidate_record_ratio: owner_value
      .get("candidate_record_ratio")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    state_sink_record_total: owner_value
      .get("state_sink_record_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    state_sink_with_artifact_ref_total: owner_value
      .get("state_sink_with_artifact_ref_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    state_sink_record_ratio: owner_value
      .get("state_sink_record_ratio")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    field_counts: value_u64_map(owner_value.get("field_counts"))
      .into_iter()
      .map(|(field, count)| (field, count as usize))
      .collect(),
    status_counts: parse_artifact_ref_status_counts(owner_value.get("status_counts")),
    artifact_ref_candidate_total: owner_value
      .get("artifact_ref_candidate_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    artifact_ref_ratio: owner_value
      .get("artifact_ref_ratio")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    artifact_ref_record_total: owner_value
      .get("artifact_ref_record_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    artifact_ref_record_ratio: owner_value
      .get("artifact_ref_record_ratio")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    artifact_ref_state_sink_total: owner_value
      .get("artifact_ref_state_sink_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    artifact_ref_state_sink_ratio: owner_value
      .get("artifact_ref_state_sink_ratio")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    artifact_ref_field_counts: value_u64_map(owner_value.get("artifact_ref_field_counts"))
      .into_iter()
      .map(|(field, count)| (field, count as usize))
      .collect(),
    artifact_ref_status_counts: parse_artifact_ref_status_counts(
      owner_value.get("artifact_ref_status_counts"),
    ),
    notes: vec![
      "reads candidate + durable state sink `.px` rows directly from gate store",
      "counts only actual field-line ownership and ignores comment mentions",
      "legacy Babashka measure_artifact_ref_ratio registry entry is removed",
    ],
  })
}

fn storage_telemetry_report() -> Result<StorageTelemetryReport> {
  let store_root = gate_store_root()?;
  let control_plane_dir = store_root.join("control-plane");
  let candidate_dir = store_root.join("px").join("candidates");
  let events_path = store_root.join("events.jsonl");
  let artifacts_dir = store_root.join("artifacts");
  let live_buffer = live_buffer_pressure_summary(&control_plane_dir.join("live-buffers.json"))?;
  let storage_snapshot = storage_snapshot_summary(&control_plane_dir);
  let hot_store_budget_bytes = env::var("PNIX_GATE_HOT_STORE_BUDGET_BYTES")
    .ok()
    .and_then(|raw| raw.trim().parse::<u64>().ok())
    .unwrap_or(DEFAULT_HOT_STORE_BUDGET_BYTES);
  let checkpoint_summary = hot_store_budget_checkpoint_summary()?;
  let owner_value = eval_gate_storage_telemetry_owner(
    "storageTelemetry",
    &json!({
      "store_root": store_root.display().to_string(),
      "store_profile_kind": store_profile_kind(&store_root),
      "store_profile_source": store_profile_source(),
      "px_entries": directory_entry_names(&store_root.join("px"))?,
      "px_dir_bytes": directory_entry_byte_map(&store_root.join("px"))?,
      "store_bytes_total": path_total_bytes(&store_root)?,
      "control_plane_bytes": path_total_bytes(&control_plane_dir)?,
      "candidate_store_bytes": path_total_bytes(&candidate_dir)?,
      "events_bytes": path_total_bytes(&events_path)?,
      "artifact_store_bytes": path_total_bytes(&artifacts_dir)?,
      "hot_store_budget_bytes": hot_store_budget_bytes,
      "pending_inline_bytes": 0,
      "live_buffer": {
        "live_buffer_open_count": live_buffer.live_buffer_open_count,
        "live_buffer_dirty_count": live_buffer.live_buffer_dirty_count,
        "live_buffer_error_count": live_buffer.live_buffer_error_count,
        "live_buffer_open_bytes": live_buffer.live_buffer_open_bytes,
        "live_buffer_dirty_bytes": live_buffer.live_buffer_dirty_bytes,
        "live_buffer_parse_pass_rate": live_buffer.live_buffer_parse_pass_rate,
        "snapshot_updated_at": live_buffer.snapshot_updated_at,
        "snapshot_source": live_buffer.snapshot_source,
      },
      "checkpoint": {
        "checkpoint_total": checkpoint_summary.checkpoint_total,
        "budget_exceeded_total": checkpoint_summary.budget_exceeded_total,
        "suppressed_total": checkpoint_summary.suppressed_total,
        "session_total": checkpoint_summary.session_total,
        "latest_recorded_at": checkpoint_summary.latest_recorded_at,
        "latest_inline_blob_mode": checkpoint_summary.latest_inline_blob_mode,
        "latest_budget_exceeded": checkpoint_summary.latest_budget_exceeded,
        "latest_hot_store_bytes": checkpoint_summary.latest_hot_store_bytes,
        "latest_effective_hot_store_bytes": checkpoint_summary.latest_effective_hot_store_bytes,
        "latest_budget_remaining_bytes": checkpoint_summary.latest_budget_remaining_bytes,
        "latest_pressure_ratio": checkpoint_summary.latest_pressure_ratio,
      },
      "artifact_ref_records": collect_artifact_ref_records(&store_root)?.iter().map(|record| {
        json!({
          "status": record.status,
          "artifact_ref_fields": record.artifact_ref_fields,
          "has_artifact_ref": record.has_artifact_ref,
        })
      }).collect::<Vec<_>>(),
      "storage_snapshot": {
        "source": storage_snapshot.source,
        "status": storage_snapshot.status,
        "ttl_violation_count": storage_snapshot.ttl_violation_count,
        "dangling_ref_count": storage_snapshot.dangling_ref_count,
        "gc_reclaimed_bytes": storage_snapshot.gc_reclaimed_bytes,
      },
    }),
  )?;
  Ok(StorageTelemetryReport {
    metric_version: "storage-telemetry-v1",
    generated_at: iso_now(),
    read_owner: "pnix gate-read storage-telemetry",
    store_bytes_total: owner_value
      .get("store_bytes_total")
      .and_then(value_as_u64)
      .unwrap_or(0),
    hot_store_bytes: owner_value
      .get("hot_store_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    control_plane_bytes: owner_value
      .get("control_plane_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    candidate_store_bytes: owner_value
      .get("candidate_store_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    events_bytes: owner_value.get("events_bytes").and_then(value_as_u64).unwrap_or(0),
    live_buffer_open_bytes: owner_value
      .get("live_buffer_open_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_dirty_bytes: owner_value
      .get("live_buffer_dirty_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_open_count: owner_value
      .get("live_buffer_open_count")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_dirty_count: owner_value
      .get("live_buffer_dirty_count")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_error_count: owner_value
      .get("live_buffer_error_count")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_parse_pass_rate: owner_value
      .get("live_buffer_parse_pass_rate")
      .and_then(Value::as_f64),
    warm_store_bytes: owner_value
      .get("warm_store_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    state_sink_bytes: owner_value
      .get("state_sink_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    artifact_store_bytes: owner_value
      .get("artifact_store_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    cold_store_bytes: owner_value
      .get("cold_store_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    tier_bytes: value_u64_map(owner_value.get("tier_bytes")),
    storage_tier_contract_version: value_string(owner_value.get("storage_tier_contract_version"))
      .unwrap_or_else(|| "v1".to_string()),
    storage_tier_abi_name: value_string(owner_value.get("storage_tier_abi_name"))
      .unwrap_or_else(|| "pnix-gate-storage-tier.v1".to_string()),
    storage_tier_names: value_string_list(owner_value.get("storage_tier_names")),
    storage_tier_contract: owner_value
      .get("storage_tier_contract")
      .cloned()
      .unwrap_or_else(|| json!({})),
    hot_store_budget_bytes: owner_value
      .get("hot_store_budget_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    hot_store_budget_remaining_bytes: owner_value
      .get("hot_store_budget_remaining_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    hot_store_budget_exceeded: owner_value
      .get("hot_store_budget_exceeded")
      .and_then(Value::as_bool)
      .unwrap_or(false),
    hot_store_pressure_ratio: owner_value
      .get("hot_store_pressure_ratio")
      .and_then(Value::as_f64),
    inline_blob_mode: value_string(owner_value.get("inline_blob_mode"))
      .unwrap_or_else(|| "bounded-preview".to_string()),
    hot_store_checkpoint_total: owner_value
      .get("hot_store_checkpoint_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    hot_store_checkpoint_exceeded_total: owner_value
      .get("hot_store_checkpoint_exceeded_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    hot_store_checkpoint_suppressed_total: owner_value
      .get("hot_store_checkpoint_suppressed_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    hot_store_checkpoint_latest_recorded_at: value_string(
      owner_value.get("hot_store_checkpoint_latest_recorded_at"),
    ),
    hot_store_checkpoint_latest_inline_blob_mode: value_string(
      owner_value.get("hot_store_checkpoint_latest_inline_blob_mode"),
    ),
    candidate_total: owner_value
      .get("candidate_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    artifact_ref_candidate_total: owner_value
      .get("artifact_ref_candidate_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    artifact_ref_ratio: owner_value
      .get("artifact_ref_ratio")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    artifact_ref_record_total: owner_value
      .get("artifact_ref_record_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    artifact_ref_record_ratio: owner_value
      .get("artifact_ref_record_ratio")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    artifact_ref_state_sink_total: owner_value
      .get("artifact_ref_state_sink_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    artifact_ref_state_sink_ratio: owner_value
      .get("artifact_ref_state_sink_ratio")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    artifact_ref_field_counts: value_u64_map(owner_value.get("artifact_ref_field_counts"))
      .into_iter()
      .map(|(field, count)| (field, count as usize))
      .collect(),
    artifact_ref_status_counts: parse_artifact_ref_status_counts(
      owner_value.get("artifact_ref_status_counts"),
    ),
    gc_reclaimed_bytes: owner_value
      .get("gc_reclaimed_bytes")
      .and_then(value_as_u64)
      .unwrap_or(0),
    ttl_violation_count: owner_value
      .get("ttl_violation_count")
      .and_then(value_as_u64),
    dangling_ref_count: owner_value
      .get("dangling_ref_count")
      .and_then(value_as_u64),
    state_sink_ready: owner_value
      .get("state_sink_ready")
      .and_then(Value::as_bool)
      .unwrap_or(false),
    state_sink_present_total: owner_value
      .get("state_sink_present_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    state_sink_contract_version: value_string(owner_value.get("state_sink_contract_version"))
      .unwrap_or_else(|| "v1".to_string()),
    state_sink_abi_name: value_string(owner_value.get("state_sink_abi_name"))
      .unwrap_or_else(|| "pnix-gate-state-sink.v1".to_string()),
    state_sink_profile_kind: value_string(owner_value.get("state_sink_profile_kind"))
      .unwrap_or_default(),
    state_sink_profile_source: value_string(owner_value.get("state_sink_profile_source"))
      .unwrap_or_default(),
    state_sink_materialization_kind: value_string(owner_value.get("state_sink_materialization_kind"))
      .unwrap_or_else(|| "directory-tree".to_string()),
    state_sink_status_count: owner_value
      .get("state_sink_status_count")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    state_sink_statuses: value_string_list(owner_value.get("state_sink_statuses")),
    state_sink_relative_paths: owner_value
      .get("state_sink_relative_paths")
      .and_then(Value::as_object)
      .map(|items| {
        items
          .iter()
          .filter_map(|(status, path)| value_string(Some(path)).map(|path| (status.clone(), path)))
          .collect()
      })
      .unwrap_or_default(),
    storage_snapshot_source: value_string(owner_value.get("storage_snapshot_source"))
      .unwrap_or_else(|| storage_snapshot.source.to_string()),
    storage_snapshot_status: value_string(owner_value.get("storage_snapshot_status"))
      .unwrap_or_else(|| storage_snapshot.status.to_string()),
    notes: vec![
      "composes state-sink-contract, store-budget, artifact-ref-ratio, and control-plane storage snapshot under one upstream gate-read owner",
      "ttl_violation_count and dangling_ref_count are read from the latest control-plane storage snapshot until deeper retention/verifier manual shells are fully split out",
      "legacy Babashka measure_storage_telemetry registry entry is removed",
    ],
  })
}

#[derive(Debug, Clone, Deserialize)]
struct ProvenanceFloorRecord {
  path: String,
  status: String,
  session_id: Option<String>,
  turn_id: Option<String>,
  tool_call_id: Option<String>,
  tool_call_required: bool,
  failure_codes: Vec<String>,
  quarantine_reason: Option<String>,
  provenance_floor_status: Option<String>,
}

fn provenance_floor_report() -> Result<ProvenanceFloorReport> {
  let store_root = gate_store_root()?;
  let mut raw_records = Vec::new();

  for (status, root) in gate_status_roots(&store_root) {
    for path in collect_px_files(&root)? {
      let content = fs::read_to_string(&path).unwrap_or_default();
      let session_id = candidate_field_string(&content, "session-id")
        .or_else(|| candidate_field_string(&content, "session_id"))
        .or_else(|| candidate_field_string(&content, "source-session-id"))
        .or_else(|| candidate_field_string(&content, "source_session_id"))
        .or_else(|| {
          provenance_tag_value(&candidate_field_list(&content, "provenance"), "session:")
        });
      let turn_id = candidate_field_string(&content, "turn-id")
        .or_else(|| candidate_field_string(&content, "turn_id"))
        .or_else(|| candidate_field_string(&content, "source-turn-id"))
        .or_else(|| candidate_field_string(&content, "source_turn_id"))
        .or_else(|| provenance_tag_value(&candidate_field_list(&content, "provenance"), "turn:"));
      let tool_call_id = candidate_field_string(&content, "tool-call-id")
        .or_else(|| candidate_field_string(&content, "tool_call_id"))
        .or_else(|| candidate_field_string(&content, "tool_use_id"))
        .or_else(|| {
          provenance_tag_value(&candidate_field_list(&content, "provenance"), "call_id:")
        });
      let predicate = candidate_field_string(&content, "predicate");
      let tool_name = candidate_field_string(&content, "tool_name")
        .or_else(|| candidate_field_string(&content, "tool-name"));
      let event_kind = candidate_field_string(&content, "event_kind")
        .or_else(|| candidate_field_string(&content, "event-kind"));
      raw_records.push(json!({
        "path": repo_relative_path(Some(&path)).unwrap_or_else(|| path.display().to_string()),
        "status": status.to_string(),
        "session_id": session_id,
        "turn_id": turn_id,
        "tool_call_id": tool_call_id,
        "predicate": predicate,
        "tool_name": tool_name,
        "event_kind": event_kind,
        "provenance": candidate_field_list(&content, "provenance"),
        "quarantine_reason": candidate_field_string(&content, "quarantine-reason")
          .or_else(|| candidate_field_string(&content, "quarantine_reason")),
        "provenance_floor_status": candidate_field_string(&content, "provenance-floor-status")
          .or_else(|| candidate_field_string(&content, "provenance_floor_status")),
      }));
    }
  }
  let owner_value = eval_gate_read_metrics_owner(
    "provenanceFloor",
    &json!({
      "records": raw_records,
      "limit": 20,
    }),
  )?;
  let weak_records: Vec<ProvenanceFloorRecord> = parse_json_array(owner_value.get("weak_records"));
  let quarantined_missing_records: Vec<ProvenanceFloorRecord> =
    parse_json_array(owner_value.get("quarantined_missing_provenance_records"));

  Ok(ProvenanceFloorReport {
    metric_version: "m9-v1",
    generated_at: iso_now(),
    read_owner: "pnix gate-read provenance-floor",
    status: value_string(owner_value.get("status")).unwrap_or_else(|| "empty".to_string()),
    record_total: owner_value
      .get("record_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    accepted_total: owner_value
      .get("accepted_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    tool_call_required_total: owner_value
      .get("tool_call_required_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    weak_total: owner_value
      .get("weak_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    weak_record_ratio: owner_value
      .get("weak_record_ratio")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    weak_accepted_total: owner_value
      .get("weak_accepted_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    accepted_floor_pass_rate: owner_value
      .get("accepted_floor_pass_rate")
      .and_then(Value::as_f64)
      .unwrap_or(1.0),
    quarantined_missing_provenance_total: owner_value
      .get("quarantined_missing_provenance_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    session_unknown_total: owner_value
      .get("session_unknown_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    missing_session_total: owner_value
      .get("missing_session_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    missing_turn_total: owner_value
      .get("missing_turn_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    missing_tool_call_total: owner_value
      .get("missing_tool_call_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    sample_weak: weak_records.iter().take(20).map(provenance_floor_sample).collect(),
    sample_quarantined_missing_provenance: quarantined_missing_records
      .iter()
      .take(20)
      .map(provenance_floor_sample)
      .collect(),
    notes: vec![
      "provenance floor blocks Accepted writes that still lack session/turn or required tool-call-id",
      "dry-run keeps weak candidate backlog visible instead of silently normalizing it away",
      "tool-call-id is required only for tool-derived records (predicate/tool_name/event_kind inferred)",
      "legacy Babashka measure_provenance_floor registry entry is removed",
    ],
  })
}

#[derive(Debug, Clone, Deserialize)]
struct UnsupportedKindRecord {
  path: String,
  status: String,
  kind: String,
  kind_support_status: Option<String>,
  schema_todo_status: Option<String>,
  quarantine_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LineageFloorRecord {
  path: String,
  status: String,
  kind: String,
  source_session_id: Option<String>,
  source_turn_id: Option<String>,
  derived_from_candidate_id: Option<String>,
  parent_packet_ids: Vec<String>,
  lineage_anchor_required: bool,
  failure_codes: Vec<String>,
  lineage_floor_status: Option<String>,
  computed_lineage_floor_status: Option<String>,
}

fn unsupported_kind_floor_report() -> Result<UnsupportedKindFloorReport> {
  let store_root = gate_store_root()?;
  let mut raw_records = Vec::new();

  for (status, root) in gate_status_roots(&store_root) {
    for path in collect_px_files(&root)? {
      let content = fs::read_to_string(&path).unwrap_or_default();
      raw_records.push(json!({
        "path": repo_relative_path(Some(&path)).unwrap_or_else(|| path.display().to_string()),
        "status": status.to_string(),
        "kind": unsupported_kind_value(&content),
        "kind_support_status": candidate_field_string(&content, "kind-support-status")
          .or_else(|| candidate_field_string(&content, "kind_support_status")),
        "schema_todo_status": candidate_field_string(&content, "schema-todo-status")
          .or_else(|| candidate_field_string(&content, "schema_todo_status")),
        "quarantine_reason": candidate_field_string(&content, "quarantine-reason")
          .or_else(|| candidate_field_string(&content, "quarantine_reason")),
      }));
    }
  }
  let owner_value = eval_gate_read_metrics_owner(
    "unsupportedKindFloor",
    &json!({
      "records": raw_records,
      "limit": 20,
    }),
  )?;
  let unsupported_records: Vec<UnsupportedKindRecord> =
    parse_json_array(owner_value.get("unsupported_kind_records"));
  let leaking_records: Vec<UnsupportedKindRecord> =
    parse_json_array(owner_value.get("leaking_records"));

  Ok(UnsupportedKindFloorReport {
    metric_version: "m9-v1",
    generated_at: iso_now(),
    read_owner: "pnix gate-read unsupported-kind-floor",
    status: value_string(owner_value.get("status")).unwrap_or_else(|| "empty".to_string()),
    record_total: owner_value
      .get("record_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    unsupported_kind_total: owner_value
      .get("unsupported_kind_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    unsupported_kind_record_ratio: owner_value
      .get("unsupported_kind_record_ratio")
      .and_then(Value::as_f64)
      .unwrap_or(0.0),
    unsupported_kind_quarantined_total: owner_value
      .get("unsupported_kind_quarantined_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    unsupported_kind_leaking_total: owner_value
      .get("unsupported_kind_leaking_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    unsupported_kind_quarantine_rate: owner_value
      .get("unsupported_kind_quarantine_rate")
      .and_then(Value::as_f64)
      .unwrap_or(1.0),
    unsupported_kind_counts: value_u64_map(owner_value.get("unsupported_kind_counts"))
      .into_iter()
      .map(|(kind, count)| (kind, count as usize))
      .collect(),
    unsupported_kind_inventory: value_string_list(owner_value.get("unsupported_kind_inventory")),
    sample_unsupported_kind: unsupported_records
      .iter()
      .take(20)
      .map(unsupported_kind_sample)
      .collect(),
    sample_unsupported_kind_leaks: leaking_records
      .iter()
      .take(20)
      .map(unsupported_kind_sample)
      .collect(),
    notes: vec![
      "unsupported kind floor blocks non-canonical kind writes before they can masquerade as a supported fact type",
      "schema-todo markers stay attached to quarantined records so the missing lift-rule work is explicit",
      "reads candidate/state sink `.px` rows directly under upstream pnix gate-read ownership",
      "legacy Babashka measure_unsupported_kind_floor registry entry is removed",
    ],
  })
}

fn lineage_floor_report(args: &Args) -> Result<LineageFloorReport> {
  let store_root = gate_store_root()?;
  let limit = args.gate_read_limit.unwrap_or(20);
  let mut raw_records = Vec::new();

  for (root_status, root) in gate_status_roots(&store_root) {
    for path in collect_px_files(&root)? {
      let content = fs::read_to_string(&path).unwrap_or_default();
      let status =
        candidate_field_string(&content, "status").unwrap_or_else(|| root_status.to_string());
      let parsed_attrset = eval_px_source_json(&content).ok().filter(Value::is_object);
      let gate_packet = parsed_attrset
        .as_ref()
        .and_then(|value| value.get("gate_observation_packet"));
      let provenance = candidate_field_list(&content, "provenance");
      let source_session_id = parsed_attrset
        .as_ref()
        .and_then(|value| {
          value_string(object_value_aliases(
            value,
            &[
              "source-session-id",
              "source_session_id",
              "session-id",
              "session_id",
            ],
          ))
        })
        .or_else(|| {
          gate_packet.and_then(|value| {
            value_string(object_value_aliases(
              value,
              &["source_session_id", "session_id"],
            ))
          })
        })
        .or_else(|| provenance_tag_value(&provenance, "session:"));
      let source_turn_id = parsed_attrset
        .as_ref()
        .and_then(|value| {
          value_string(object_value_aliases(
            value,
            &["source-turn-id", "source_turn_id", "turn-id", "turn_id"],
          ))
        })
        .or_else(|| {
          gate_packet.and_then(|value| {
            value_string(object_value_aliases(value, &["source_turn_id", "turn_id"]))
          })
        })
        .or_else(|| provenance_tag_value(&provenance, "turn:"));
      let reopen_from_candidate_id = parsed_attrset
        .as_ref()
        .and_then(|value| {
          value_string(object_value_aliases(
            value,
            &["reopen-from-candidate-id", "reopen_from_candidate_id"],
          ))
        })
        .or_else(|| {
          candidate_field_string(&content, "reopen-from-candidate-id")
            .or_else(|| candidate_field_string(&content, "reopen_from_candidate_id"))
        });
      let derived_from_candidate_id = parsed_attrset
        .as_ref()
        .and_then(|value| {
          value_string(object_value_aliases(
            value,
            &[
              "derived-from-candidate-id",
              "derived_from_candidate_id",
              "reopen-from-candidate-id",
              "reopen_from_candidate_id",
            ],
          ))
        })
        .or_else(|| {
          candidate_field_string(&content, "derived-from-candidate-id")
            .or_else(|| candidate_field_string(&content, "derived_from_candidate_id"))
            .or_else(|| reopen_from_candidate_id.clone())
        });
      let mut parent_packet_ids = parsed_attrset
        .as_ref()
        .map(|value| {
          value_string_list(object_value_aliases(
            value,
            &["parent-packet-ids", "parent_packet_ids"],
          ))
        })
        .unwrap_or_else(|| {
          let mut values = candidate_field_list(&content, "parent-packet-ids");
          if values.is_empty() {
            values = candidate_field_list(&content, "parent_packet_ids");
          }
          values
        });
      if parent_packet_ids.is_empty() && matches!(status.as_str(), "reopened" | "retired") {
        if let Some(value) = derived_from_candidate_id
          .clone()
          .or_else(|| reopen_from_candidate_id.clone())
        {
          parent_packet_ids.push(value);
        }
      }
      let floor_status = candidate_field_string(&content, "lineage-floor-status")
        .or_else(|| candidate_field_string(&content, "lineage_floor_status"));
      raw_records.push(json!({
        "path": repo_relative_path(Some(&path)).unwrap_or_else(|| path.display().to_string()),
        "status": status,
        "kind": candidate_kind_value(&content),
        "source_session_id": source_session_id,
        "source_turn_id": source_turn_id,
        "derived_from_candidate_id": derived_from_candidate_id,
        "parent_packet_ids": parent_packet_ids,
        "lineage_floor_status": floor_status,
        "parsed_attrset_present": parsed_attrset.is_some(),
      }));
    }
  }
  let owner_value = eval_gate_read_metrics_owner(
    "lineageFloor",
    &json!({
      "records": raw_records,
      "limit": limit,
    }),
  )?;
  let weak_records: Vec<LineageFloorRecord> = parse_json_array(owner_value.get("weak_records"));

  Ok(LineageFloorReport {
    generated_at: iso_now(),
    metric_version: "m9-v1",
    read_owner: "pnix gate-read lineage-floor",
    status: value_string(owner_value.get("status")).unwrap_or_else(|| "empty".to_string()),
    record_total: owner_value
      .get("record_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    weak_total: owner_value
      .get("weak_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    passed_total: owner_value
      .get("passed_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    partial_total: owner_value
      .get("partial_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    missing_source_context_total: owner_value
      .get("missing_source_context_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    lineage_anchor_required_total: owner_value
      .get("lineage_anchor_required_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    missing_source_session_total: owner_value
      .get("missing_source_session_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    missing_source_turn_total: owner_value
      .get("missing_source_turn_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    missing_derived_from_candidate_total: owner_value
      .get("missing_derived_from_candidate_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    missing_parent_packet_total: owner_value
      .get("missing_parent_packet_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    missing_floor_status_total: owner_value
      .get("missing_floor_status_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    stale_floor_status_total: owner_value
      .get("stale_floor_status_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    unparseable_attrset_total: owner_value
      .get("unparseable_attrset_total")
      .and_then(value_as_u64)
      .unwrap_or(0) as usize,
    status_counts: value_u64_map(owner_value.get("status_counts"))
      .into_iter()
      .map(|(status, count)| (status, count as usize))
      .collect(),
    kind_counts: value_u64_map(owner_value.get("kind_counts"))
      .into_iter()
      .map(|(kind, count)| (kind, count as usize))
      .collect(),
    lineage_floor_status_counts: value_u64_map(owner_value.get("lineage_floor_status_counts"))
      .into_iter()
      .map(|(status, count)| (status, count as usize))
      .collect(),
    sample_weak: weak_records
      .iter()
      .take(limit)
      .map(lineage_floor_sample)
      .collect(),
    notes: vec![
      "lineage floor requires canonical source-session-id/source-turn-id on internal records",
      "reopened and retired records additionally require derived-from-candidate-id plus parent-packet-ids lineage anchors",
      "missing-floor-status means historical rows predate the canonical lineage floor marker even if source context is still inferable",
      "legacy Babashka measure_lineage_floor registry entry is removed",
    ],
  })
}

fn recent_events_report(args: &Args) -> Result<GateReadRecentEventsReport> {
  parse_json_value(eval_gate_read_metrics_owner(
    "recentEvents",
    &json!({
      "generated_at": iso_now(),
      "read_owner": "pnix gate-read recent-events",
      "event_types": args.gate_read_event_types,
      "limit": args.gate_read_limit.unwrap_or(20).max(1),
      "events": read_gate_events()?,
    }),
  )?)
}

fn candidate_listing_report(args: &Args) -> Result<GateReadCandidatesReport> {
  let limit = args.gate_read_limit.unwrap_or(50).max(1);
  let kind_filter = nonblank(args.gate_read_kind.clone());
  let candidate_dir = gate_store_root()?.join("px").join("candidates");
  let mut files = collect_px_files(&candidate_dir)?;
  files.sort_by(|a, b| {
    a.file_name()
      .cmp(&b.file_name())
      .then_with(|| a.as_os_str().cmp(b.as_os_str()))
  });
  let mut candidates = Vec::new();
  for path in files {
    let filename = path
      .file_name()
      .and_then(|value| value.to_str())
      .unwrap_or_default()
      .to_string();
    if let Some(kind) = kind_filter.as_deref() {
      if !filename.starts_with(kind) {
        continue;
      }
    }
    let content = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    candidates.push(GateReadCandidateEntry {
      filename,
      path: path.display().to_string(),
      kind: candidate_field_string(&content, "kind"),
      status: candidate_field_string(&content, "status"),
      recorded_at: candidate_recorded_at(&content),
      subject: candidate_field_string(&content, "subject"),
      query: candidate_field_string(&content, "query"),
      intent: candidate_field_string(&content, "intent"),
      trace_ref: candidate_field_string(&content, "trace_ref")
        .or_else(|| candidate_field_string(&content, "trace-ref")),
    });
  }
  parse_json_value(eval_gate_read_metrics_owner(
    "candidateListing",
    &json!({
      "generated_at": iso_now(),
      "read_owner": "pnix gate-read candidates",
      "kind_filter": kind_filter,
      "limit": limit,
      "candidates": candidates,
    }),
  )?)
}

fn brain_ankh_policy_projection_report(args: &Args) -> Result<BrainAnkhPolicyProjectionReport> {
  let limit = args.gate_read_limit.unwrap_or(50).max(1);
  let gate_store_root = gate_store_root()?;
  let gate_signals = load_brain_ankh_gate_signals(&gate_store_root, limit)?;
  let coding_store_path = env::var_os("DOGHOUSE_STORE_PATH")
    .filter(|value| !value.is_empty())
    .map(PathBuf::from);
  let mut notes = Vec::new();
  let coding_inputs = match coding_store_path.as_deref() {
    #[cfg(feature = "doghouse")]
    Some(path) => match load_brain_ankh_coding_memory_inputs(path, limit) {
      Ok(inputs) => inputs,
      Err(err) => {
        notes.push(format!(
          "coding-memory-store-unavailable:{}",
          truncate_report_note(&err.to_string())
        ));
        Vec::new()
      }
    },
    // doghouse feature off: the coding memory store is unavailable, so the
    // configured path cannot be read. Emit a note and omit coding.* inputs.
    #[cfg(not(feature = "doghouse"))]
    Some(_path) => {
      notes.push(
        "doghouse feature disabled; DOGHOUSE_STORE_PATH configured but coding.* inputs omitted"
          .to_string(),
      );
      Vec::new()
    }
    None => {
      notes.push("DOGHOUSE_STORE_PATH not configured; coding.* inputs omitted".to_string());
      Vec::new()
    }
  };
  let mut report = build_brain_ankh_policy_projection_report(
    iso_now(),
    gate_store_root.display().to_string(),
    coding_store_path.map(|path| path.display().to_string()),
    limit,
    gate_signals,
    coding_inputs,
  )?;
  report.notes.extend(notes);
  Ok(report)
}

fn build_brain_ankh_policy_projection_report(
  generated_at: String,
  gate_store_root: String,
  coding_memory_store_path: Option<String>,
  limit: usize,
  gate_signals: Vec<BrainAnkhGateSignalInput>,
  coding_inputs: Vec<BrainAnkhCodingMemoryInput>,
) -> Result<BrainAnkhPolicyProjectionReport> {
  parse_json_value(eval_brain_ankh_policy_owner(
    "project",
    &json!({
      "generated_at": generated_at,
      "read_owner": "pnix gate-read brain-ankh-policy",
      "gate_store_root": gate_store_root,
      "coding_memory_store_path": coding_memory_store_path,
      "limit": limit,
      "gate_signals": gate_signals,
      "coding_inputs": coding_inputs,
      "notes": [],
    }),
  )?)
}

fn load_brain_ankh_gate_signals(
  store_root: &Path,
  limit: usize,
) -> Result<Vec<BrainAnkhGateSignalInput>> {
  let mut signals = Vec::new();
  for (source_status, root) in gate_status_roots(store_root) {
    for path in collect_px_files(&root)? {
      let content =
        fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
      let kind = candidate_kind_value(&content);
      if !matches!(
        kind.as_str(),
        "observation-atom"
          | "selection-trace"
          | "chooser-judgement"
          | "dispatch-execution"
          | "validation-record"
          | "repair-recipe"
      ) {
        continue;
      }
      let source_ref = candidate_field_string(&content, "candidate-id")
        .or_else(|| candidate_field_string(&content, "candidate_id"))
        .or_else(|| candidate_field_string(&content, "trace_ref"))
        .or_else(|| candidate_field_string(&content, "trace-ref"))
        .unwrap_or_else(|| path.display().to_string());
      signals.push(BrainAnkhGateSignalInput {
        source_ref,
        source_path: path.display().to_string(),
        source_status: source_status.to_string(),
        kind,
        recorded_at: candidate_recorded_at(&content),
        trace_ref: candidate_field_string(&content, "trace_ref")
          .or_else(|| candidate_field_string(&content, "trace-ref")),
        subject: candidate_field_string(&content, "subject"),
        predicate: candidate_field_string(&content, "predicate"),
        object: candidate_field_string(&content, "object"),
        query: candidate_field_string(&content, "query"),
        intent: candidate_field_string(&content, "intent"),
        selected_card: candidate_field_string(&content, "selected_card")
          .or_else(|| candidate_field_string(&content, "selected-card")),
        ranked_cards: field_list_aliases(&content, &["ranked_cards", "ranked-cards"]),
        chooser: candidate_field_string(&content, "chooser"),
        judgement: candidate_field_string(&content, "judgement"),
        confidence: candidate_field_string(&content, "confidence"),
        dispatch_status: candidate_field_string(&content, "dispatch_status")
          .or_else(|| candidate_field_string(&content, "dispatch-status")),
        tool: candidate_field_string(&content, "tool"),
        evidence: candidate_field_list(&content, "evidence"),
        reasons: candidate_field_list(&content, "reasons"),
        provenance: candidate_field_list(&content, "provenance"),
      });
    }
  }
  signals.sort_by(|left, right| {
    right
      .recorded_at
      .cmp(&left.recorded_at)
      .then_with(|| left.source_ref.cmp(&right.source_ref))
  });
  let mut per_kind_counts = BTreeMap::new();
  signals.retain(|signal| {
    let count = per_kind_counts.entry(signal.kind.clone()).or_insert(0usize);
    if *count >= limit {
      return false;
    }
    *count += 1;
    true
  });
  Ok(signals)
}

#[cfg(feature = "doghouse")]
fn load_brain_ankh_coding_memory_inputs(
  store_path: &Path,
  limit: usize,
) -> Result<Vec<BrainAnkhCodingMemoryInput>> {
  let store = DoghouseStore::open(DoghouseStoreConfig::new(store_path.to_path_buf()))
    .with_context(|| format!("open doghouse coding memory store {}", store_path.display()))?;
  let mut inputs = Vec::new();
  for artifact in doghouse_core::store::read_all_coding_memory_artifacts_at(store.path())
    .with_context(|| format!("scan coding memory artifacts {}", store_path.display()))?
  {
    if !is_brain_ankh_coding_policy_input_family(&artifact.artifact_family) {
      continue;
    }
    inputs.push(coding_memory_input_from_artifact(artifact));
  }
  inputs.sort_by(|left, right| {
    right
      .stored_at_ms
      .cmp(&left.stored_at_ms)
      .then_with(|| left.source_ref.cmp(&right.source_ref))
  });
  inputs.truncate(limit.saturating_mul(4));
  Ok(inputs)
}

#[cfg(feature = "doghouse")]
fn is_brain_ankh_coding_policy_input_family(family: &str) -> bool {
  matches!(
    family,
    "coding.verify-receipt"
      | "coding.execution-result"
      | "coding.context-demand"
      | "coding.semantic-patch-review"
      | "coding.learning-card"
      | "coding.repair-recipe-replay"
      | "coding.context-demand-replay"
      | "coding.human-promotion-decision"
      | "coding.generated-patch-review-receipt"
      | "coding.provider-feedback-request"
      | "coding.feedback-retry-guard"
      | "coding.patch-proposal"
      | "coding.apply-result"
  )
}

#[cfg(feature = "doghouse")]
fn coding_memory_input_from_artifact(artifact: CodingMemoryArtifact) -> BrainAnkhCodingMemoryInput {
  let mut evidence_refs = artifact.related_refs.clone();
  if let Some(repo_snapshot_ref) = artifact.repo_snapshot_ref.as_deref() {
    evidence_refs.push(format!("repo-snapshot-ref:{repo_snapshot_ref}"));
  }
  for command_ref in &artifact.command_refs {
    evidence_refs.push(format!("command-ref:{command_ref}"));
  }
  collect_ref_like_values(&artifact.payload, &mut evidence_refs);
  evidence_refs.sort();
  evidence_refs.dedup();
  evidence_refs.truncate(32);

  let status = coding_payload_status(&artifact.payload);
  let subject = value_string(
    object_value_aliases(
      &artifact.payload,
      &[
        "current_interpretation",
        "trigger",
        "review_status",
        "verify_status",
        "execution_status",
        "human_decision",
      ],
    )
    .or_else(|| nested_object_value(Some(&artifact.payload), &["status", "result_status"])),
  );

  BrainAnkhCodingMemoryInput {
    source_ref: artifact.id,
    source_family: artifact.artifact_family,
    source_surface: artifact.source_surface,
    stored_at_ms: artifact.stored_at_ms,
    repo_snapshot_ref: artifact.repo_snapshot_ref,
    target_paths: artifact.target_paths,
    command_refs: artifact.command_refs,
    related_refs: artifact.related_refs,
    status,
    subject,
    evidence_refs,
  }
}

#[cfg(feature = "doghouse")]
fn coding_payload_status(value: &Value) -> Option<String> {
  value_string(
    object_value_aliases(
      value,
      &[
        "verify_status",
        "execution_status",
        "review_status",
        "replay_status",
        "decision_status",
        "human_decision",
        "apply_status",
      ],
    )
    .or_else(|| nested_object_value(Some(value), &["status", "result_status"]))
    .or_else(|| nested_object_value(Some(value), &["execution_result", "execution_status"])),
  )
}

#[cfg(feature = "doghouse")]
fn collect_ref_like_values(value: &Value, out: &mut Vec<String>) {
  match value {
    Value::Object(map) => {
      for (key, item) in map {
        if is_ref_like_field(key) {
          collect_string_values(item, out);
        }
        collect_ref_like_values(item, out);
      }
    }
    Value::Array(items) => {
      for item in items {
        collect_ref_like_values(item, out);
      }
    }
    _ => {}
  }
}

#[cfg(feature = "doghouse")]
fn collect_string_values(value: &Value, out: &mut Vec<String>) {
  match value {
    Value::String(raw) if !raw.trim().is_empty() => out.push(raw.trim().to_string()),
    Value::Array(items) => {
      for item in items {
        collect_string_values(item, out);
      }
    }
    _ => {}
  }
}

#[cfg(feature = "doghouse")]
fn is_ref_like_field(key: &str) -> bool {
  let normalized = key.replace('-', "_");
  normalized == "id"
    || normalized == "proof_refs"
    || normalized == "related_refs"
    || normalized == "source_artifact_refs"
    || normalized.ends_with("_ref")
    || normalized.ends_with("_refs")
}

#[cfg(feature = "doghouse")]
fn truncate_report_note(value: &str) -> String {
  const MAX_LEN: usize = 180;
  let trimmed = value.trim();
  if trimmed.len() <= MAX_LEN {
    trimmed.to_string()
  } else {
    let mut end = MAX_LEN;
    while !trimmed.is_char_boundary(end) {
      end -= 1;
    }
    format!("{}...", &trimmed[..end])
  }
}

fn brain_bundle_contract_report() -> Result<BrainBundleContractReport> {
  let generated_at = iso_now();
  let bundle_source = json_path_report(Some(portable_domain_bundle_path()?));
  let bundle_dir = bundle_source
    .resolved_path
    .as_ref()
    .and_then(|path| path.parent().map(Path::to_path_buf));
  let signature_proof_ref = bundle_source
    .value
    .as_ref()
    .and_then(|value| value.get("signature_proof_ref"))
    .and_then(scalar_string);
  let proof_source = {
    let proof_path = signature_proof_ref
      .as_deref()
      .map(|raw| resolve_readable_path(raw, bundle_dir.as_deref()))
      .transpose()?;
    json_path_report(proof_path)
  };
  let schema_source = json_path_report(Some(capability_manifest_schema_path()?));
  let example_validation_payload = brain_bundle_validation_payload(
    bundle_source,
    proof_source,
    schema_source,
    Some("portable-domain-bundle".to_string()),
    Some("domain-lobe".to_string()),
    Some("PortableDomainBundleProof".to_string()),
    "pnix gate-read validate-brain-bundle",
    iso_now(),
  );
  parse_json_value(eval_gate_read_metrics_owner(
    "brainBundleContract",
    &json!({
      "generated_at": generated_at,
      "read_owner": "pnix gate-read brain-bundle-contract",
      "example_validation_payload": example_validation_payload,
    }),
  )?)
}

fn brain_bundle_validation_report_from_args(args: &Args) -> Result<BrainBundleValidationReport> {
  let bundle_path = args
    .gate_read_path
    .as_deref()
    .map(|raw| resolve_readable_path(raw, None))
    .transpose()?
    .ok_or_else(|| anyhow!("--path is required for `pnix gate-read validate-brain-bundle`"))?;
  let proof_path = args
    .gate_read_proof_path
    .as_deref()
    .map(|raw| resolve_readable_path(raw, bundle_path.parent()))
    .transpose()?;
  let schema_path = args
    .gate_read_schema_path
    .as_deref()
    .map(|raw| resolve_readable_path(raw, bundle_path.parent()))
    .transpose()?;
  brain_bundle_validation_report(
    Some(bundle_path),
    proof_path,
    schema_path,
    args.gate_read_expected_bundle_kind.clone(),
    args.gate_read_expected_lobe_profile.clone(),
    args.gate_read_expected_proof_kind.clone(),
    "pnix gate-read validate-brain-bundle",
  )
}

fn brain_bundle_validation_report(
  bundle_path: Option<PathBuf>,
  proof_path_override: Option<PathBuf>,
  schema_path_override: Option<PathBuf>,
  expected_bundle_kind: Option<String>,
  expected_lobe_profile: Option<String>,
  expected_proof_kind: Option<String>,
  read_owner: &'static str,
) -> Result<BrainBundleValidationReport> {
  let generated_at = iso_now();
  let bundle_source = json_path_report(bundle_path);
  let bundle = bundle_source.value.as_ref();
  let bundle_dir = bundle_source
    .resolved_path
    .as_ref()
    .and_then(|path| path.parent().map(Path::to_path_buf));
  let signature_proof_ref = bundle
    .and_then(|value| value.get("signature_proof_ref"))
    .and_then(scalar_string);
  let proof_source = if proof_path_override.is_some() {
    json_path_report(proof_path_override)
  } else {
    let proof_path = signature_proof_ref
      .as_deref()
      .map(|raw| resolve_readable_path(raw, bundle_dir.as_deref()))
      .transpose()?;
    json_path_report(proof_path)
  };
  let schema_source = if let Some(path) = schema_path_override {
    json_path_report(Some(path))
  } else {
    json_path_report(Some(capability_manifest_schema_path()?))
  };
  parse_json_value(eval_gate_read_metrics_owner(
    "validateBrainBundle",
    &brain_bundle_validation_payload(
      bundle_source,
      proof_source,
      schema_source,
      expected_bundle_kind,
      expected_lobe_profile,
      expected_proof_kind,
      read_owner,
      generated_at,
    ),
  )?)
}

fn ontology_lookup_related_report(args: &Args) -> Result<GateReadLookupReport> {
  let query = lookup_query_spec(args)?;
  let facts = accepted_fact_records_for_query(&query, accepted_fact_records()?);
  parse_json_value(eval_gate_lookup_context_owner(
    "ontologyLookupRelated",
    &json!({
      "generated_at": iso_now(),
      "read_owner": "pnix gate-read ontology-lookup-related",
      "result_source": "lookup-owner-pnix-direct",
      "adapter_mode": "pnix-query-runtime-direct",
      "query_owner": "stdlib/lib/gate/lookup-rules.px",
      "selection_owner": "stdlib/lib/gate/lookup-select.px",
      "context": query.context,
      "predicate": query.predicate,
      "limit": query.limit,
      "min_confidence": query.min_confidence,
      "accepted_facts": facts,
    }),
  )?)
}

fn query_context_report(args: &Args) -> Result<QueryContextReport> {
  let topic = nonblank(args.gate_read_topic.clone()).ok_or_else(|| {
    anyhow!("--topic/--query-topic is required for `pnix gate-read query-context`")
  })?;
  let limit = args.gate_read_limit.unwrap_or(5).max(1);
  let topic_search = normalize_intent(&topic);
  let topic_tokens = intent_words(&topic);
  if topic_tokens.is_empty() {
    bail!("query-context topic must contain at least one non-empty token");
  }
  let lookup = ontology_lookup_related_report(&Args {
    gate_read_context: Some(topic.clone()),
    gate_read_predicate: None,
    gate_read_limit: Some(limit),
    gate_read_min_confidence: None,
    ..args.clone()
  })?;
  parse_json_value(eval_gate_lookup_context_owner(
    "queryContext",
    &json!({
      "generated_at": iso_now(),
      "read_owner": "pnix gate-read query-context",
      "topic": topic,
      "topic_search": topic_search,
      "topic_tokens": topic_tokens,
      "limit": limit,
      "lookup_preview": serde_json::to_value(&GateReadLookupPreview {
        lookup_status: lookup.lookup_status,
        result_source: lookup.result_source,
        adapter_mode: lookup.adapter_mode,
        query_owner: lookup.query_owner,
        selection_owner: lookup.selection_owner,
        query_fingerprint: lookup.query_fingerprint.clone(),
        result_fingerprint: lookup.result_fingerprint.clone(),
        ranked_candidate_ids: lookup.ranked_candidate_ids.clone(),
        fact_total: lookup.fact_total,
        facts: lookup.facts.clone(),
      })?,
      "candidate_hits": query_context_candidate_records(&topic_tokens)?,
      "event_hits": query_context_event_records(&topic_tokens)?,
      "recipe_hits": query_context_recipe_records(&topic_tokens)?,
    }),
  )?)
}

fn recipe_match_current_report(args: &Args) -> Result<RecipeMatchReport> {
  let tool_name = normalize_intent(
    &nonblank(args.gate_read_tool_name.clone())
      .ok_or_else(|| anyhow!("--tool-name/--tool_name is required"))?,
  );
  let context = nonblank(args.gate_read_context.clone()).map(|value| normalize_intent(&value));
  let arg_predicates = args
    .gate_read_arg_predicates
    .iter()
    .map(|value| normalize_intent(value))
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>();
  let min_confidence = args.gate_read_min_confidence.unwrap_or(0.7);
  let limit = args.gate_read_limit.unwrap_or(5).max(1);
  let stale_repeat_threshold = recipe_stale_repeat_threshold();
  let (doc_ok, doc_path, records) = repair_recipe_records()?;
  let override_recipe_id = nonblank(env::var("PNIX_GATE_RECIPE_OVERRIDE").ok());
  let telemetry_events = recipe_match_telemetry_events()?;
  let report: RecipeMatchReport = parse_json_value(eval_gate_read_metrics_owner(
    "recipeMatchCurrent",
    &json!({
      "generated_at": iso_now(),
      "read_owner": "pnix gate-read recipe-match-current",
      "tool_name": tool_name.clone(),
      "context": context.clone(),
      "arg_predicates": arg_predicates.clone(),
      "min_confidence": min_confidence,
      "limit": limit,
      "repair_recipe_doc_path": doc_path.clone(),
      "repair_recipe_doc_ok": doc_ok,
      "override_recipe_id": override_recipe_id.clone(),
      "stale_repeat_threshold": stale_repeat_threshold,
      "recipes": serde_json::to_value(&records)?,
      "telemetry_events": telemetry_events,
    }),
  )?)?;

  append_recipe_match_telemetry(&report)?;
  Ok(report)
}

fn recipe_stale_repeat_threshold() -> usize {
  env::var("PNIX_GATE_RECIPE_STALE_REPEAT_THRESHOLD")
    .ok()
    .and_then(|raw| raw.trim().parse::<usize>().ok())
    .map(|value| value.max(1))
    .unwrap_or(DEFAULT_RECIPE_STALE_REPEAT_THRESHOLD)
}

fn repair_recipe_records() -> Result<(bool, String, Vec<RecipeRecord>)> {
  let path = live_px_dir()?.join("repair-recipes.px");
  let path_string = path.display().to_string();
  if !path.exists() {
    return Ok((false, path_string, Vec::new()));
  }
  let Ok(content) = fs::read_to_string(&path) else {
    return Ok((false, path_string, Vec::new()));
  };
  let mut records = quoted_attrset_entries(&content, "recipes")
    .into_iter()
    .map(|(recipe_id, block)| parse_recipe_record(&recipe_id, &block))
    .collect::<Vec<_>>();
  records.sort_by(|a, b| a.recipe_id.cmp(&b.recipe_id));
  Ok((true, path_string, records))
}

fn parse_recipe_record(recipe_id: &str, block: &str) -> RecipeRecord {
  let signature = attrset_field_block(block, "signature");
  let signature_block = signature.as_deref().unwrap_or_default();
  let confidence_raw = field_string_aliases(block, &["confidence"]).unwrap_or_default();
  RecipeRecord {
    recipe_id: recipe_id.to_string(),
    tool_name: normalize_intent(
      &field_string_aliases(signature_block, &["tool_name", "tool-name"])
        .or_else(|| field_string_aliases(block, &["tool"]))
        .unwrap_or_default(),
    ),
    context: normalize_intent(
      &field_string_aliases(signature_block, &["context"]).unwrap_or_default(),
    ),
    arg_predicates: field_list_aliases(signature_block, &["arg_predicates", "arg-predicates"])
      .into_iter()
      .map(|value| normalize_intent(&value))
      .collect(),
    confidence: confidence_raw.parse::<f64>().unwrap_or(0.0),
    confidence_raw,
    subject: field_string_aliases(block, &["subject"]).unwrap_or_default(),
    past_failure_summary: field_string_aliases(
      block,
      &["past_failure_summary", "past-failure-summary"],
    )
    .unwrap_or_default(),
    recommended_sequence: field_list_aliases(
      block,
      &["recommended_sequence", "recommended-sequence"],
    ),
    steps: field_list_aliases(block, &["steps"]),
    source_id: field_string_aliases(block, &["source-id", "source_id"]),
    source_version: field_string_aliases(block, &["source-version", "source_version"]),
    source_checksum: field_string_aliases(block, &["source-checksum", "source_checksum"]),
    entity_key: field_string_aliases(block, &["entity-key", "entity_key"]),
    member_path: field_string_aliases(block, &["member-path", "member_path"]),
    rule_version: field_string_aliases(block, &["rule-version", "rule_version"]),
    path_diff_kind: field_string_aliases(block, &["path-diff-kind", "path_diff_kind"]),
    old_path: field_string_aliases(block, &["old-path", "old_path"]),
    new_path: field_string_aliases(block, &["new-path", "new_path"]),
    migration_epoch: field_string_aliases(block, &["migration-epoch", "migration_epoch"]),
    supersedes: field_list_aliases(block, &["supersedes"]),
    invalidated_by: field_list_aliases(block, &["invalidated-by", "invalidated_by"]),
    validated_by_outcome: field_list_aliases(
      block,
      &["validated-by-outcome", "validated_by_outcome"],
    ),
    conflicts_with: field_list_aliases(block, &["conflicts-with", "conflicts_with"]),
    hold_reason: nonblank(field_string_aliases(block, &["hold-reason", "hold_reason"])),
    last_effective_at: field_string_aliases(block, &["last-effective-at", "last_effective_at"]),
  }
}

fn recipe_match_telemetry_events() -> Result<Vec<Value>> {
  let events_path = gate_store_root()?.join("events.jsonl");
  if !events_path.exists() {
    return Ok(Vec::new());
  }
  let content =
    fs::read_to_string(&events_path).with_context(|| format!("read {}", events_path.display()))?;
  let mut events = content
    .lines()
    .filter_map(|line| serde_json::from_str::<Value>(line).ok())
    .filter(|value| value.get("event").and_then(Value::as_str) == Some("RecipeMatchTelemetry"))
    .collect::<Vec<_>>();
  if events.len() > RECIPE_MATCH_TELEMETRY_HISTORY_LIMIT {
    let start = events.len() - RECIPE_MATCH_TELEMETRY_HISTORY_LIMIT;
    events = events.split_off(start);
  }
  Ok(events)
}

fn append_recipe_match_telemetry(report: &RecipeMatchReport) -> Result<()> {
  let matched_last_effective_ats = report
    .matched_recipe_metadata
    .iter()
    .filter_map(|item| item.last_effective_at.clone())
    .collect::<Vec<_>>();
  let blocked_last_effective_ats = report
    .blocked_recipe_metadata
    .iter()
    .filter_map(|item| item.last_effective_at.clone())
    .collect::<Vec<_>>();
  let last_effective_at = blocked_last_effective_ats
    .first()
    .cloned()
    .or_else(|| matched_last_effective_ats.first().cloned());
  let payload = json!({
    "event": "RecipeMatchTelemetry",
    "metric_version": "recipe-match-telemetry-v1",
    "source": "pnix gate-read recipe-match-current",
    "recorded_at": iso_now(),
    "tool_name": report.tool_name,
    "context": report.context,
    "arg_predicates": report.arg_predicates,
    "min_confidence": report.min_confidence,
    "limit": report.limit,
    "match_status": report.match_status,
    "match_total": report.match_total,
    "stale_repeat_threshold": report.stale_repeat_threshold,
    "signature_matched_recipe_ids": report.signature_matched_recipe_ids,
    "matched_recipe_ids": report.matched_recipe_ids,
    "held_recipe_ids": report.held_recipe_ids,
    "invalidated_recipe_ids": report.invalidated_recipe_ids,
    "lineage_invalidated_recipe_ids": report.lineage_invalidated_recipe_ids,
    "stale_retired_recipe_ids": report.stale_retired_recipe_ids,
    "conflict_blocked_recipe_ids": report.conflict_blocked_recipe_ids,
    "blocked_recipe_ids": report.blocked_recipe_ids,
    "matched_recipe_last_effective_ats": matched_last_effective_ats,
    "blocked_recipe_last_effective_ats": blocked_last_effective_ats,
    "last_effective_at": last_effective_at,
    "override_recipe_id": report.override_recipe_id,
    "override_blocked_recipe_ids": report.override_blocked_recipe_ids,
    "warning_block_total": report.match_total,
    "evaluated_recipe_total": report.evaluated_recipe_total,
    "repair_recipe_doc_ok": report.repair_recipe_doc_ok,
  });
  append_gate_event(&payload)
}

fn lookup_query_spec(args: &Args) -> Result<LookupQuerySpec> {
  let context = nonblank(args.gate_read_context.clone())
    .ok_or_else(|| anyhow!("--context/--lookup-context is required"))?;
  let predicate = nonblank(args.gate_read_predicate.clone()).map(|value| normalize_intent(&value));
  let limit = args.gate_read_limit.unwrap_or(5).max(1);
  let min_confidence = args.gate_read_min_confidence.unwrap_or(0.6);
  let payload = json!({
    "context": context,
    "predicate": predicate,
    "limit": limit,
    "min_confidence": min_confidence,
  });
  let source = format!(
    "let\n  root = \"{}\";\n  rules = import (root + \"/stdlib/lib/gate/lookup-rules.px\");\n  spec = {};\n  rule = rules.mkLookupRule spec;\nin {{\n  proof = \"pnix-gate-ontology-lookup-query-adapter\";\n  query = rule.query;\n  packet_shape = rule.packet-shape or \"facts+provenance_refs-only\";\n  empty_packet = rule.empty-packet or \"facts=[];lookup_status=empty\";\n}}\n",
    workspace_root()?.display(),
    emit_px_json_value(&payload)
  );
  let value = eval_px_source_json(&source)?;
  let query = value.get("query").cloned().unwrap_or(Value::Null);
  let context = query
    .get("context")
    .and_then(Value::as_str)
    .map(str::to_string)
    .ok_or_else(|| anyhow!("lookup-rules owner did not return query.context"))?;
  Ok(LookupQuerySpec {
    context,
    predicate: query
      .get("predicate")
      .and_then(Value::as_str)
      .map(|value| normalize_intent(value)),
    limit: query
      .get("limit")
      .and_then(Value::as_u64)
      .map(|value| value as usize)
      .unwrap_or(limit),
    min_confidence: query
      .get("min_confidence")
      .and_then(Value::as_f64)
      .unwrap_or(min_confidence),
  })
}

fn accepted_fact_records() -> Result<Vec<AcceptedFactRecord>> {
  let accepted_dir = gate_store_root()?.join("px").join("accepted");
  let files = collect_px_files(&accepted_dir)?;
  let mut facts = Vec::new();
  for path in files {
    let content = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let subj = candidate_field_string(&content, "subject");
    let pred = candidate_field_string(&content, "predicate");
    let obj = candidate_field_string(&content, "object");
    if let (Some(subj), Some(pred), Some(obj)) = (subj, pred, obj) {
      let provenance_refs = candidate_field_list(&content, "provenance");
      let confidence = candidate_field_string(&content, "confidence")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(1.0);
      let searchable = normalize_intent(&format!(
        "{} {} {} {} {}",
        subj,
        pred,
        obj,
        path.display(),
        provenance_refs.join(" ")
      ));
      facts.push(AcceptedFactRecord {
        subj,
        pred,
        obj,
        confidence,
        provenance_refs,
        searchable,
      });
    }
  }
  Ok(facts)
}

fn accepted_fact_records_for_query(
  query: &LookupQuerySpec,
  facts: Vec<AcceptedFactRecord>,
) -> Vec<Value> {
  let context_tokens = intent_words(&query.context);
  facts
    .into_iter()
    .map(|fact| {
      let context_hit_total = topic_hit_total(&context_tokens, &fact.searchable);
      json!({
        "subj": fact.subj,
        "pred": fact.pred,
        "obj": fact.obj,
        "confidence": fact.confidence,
        "provenance_refs": fact.provenance_refs,
        "searchable": fact.searchable,
        "context_hit_total": context_hit_total,
      })
    })
    .collect()
}

fn query_context_candidate_records(topic_tokens: &[String]) -> Result<Vec<Value>> {
  let candidate_dir = gate_store_root()?.join("px").join("candidates");
  let files = collect_px_files(&candidate_dir)?;
  let mut hits = Vec::new();
  for path in files {
    let content = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let searchable = normalize_intent(&content);
    let topic_hit_total = topic_hit_total(topic_tokens, &searchable);
    hits.push(json!({
      "path": path.display().to_string(),
      "kind": candidate_field_string(&content, "kind"),
      "status": candidate_field_string(&content, "status"),
      "recorded_at": candidate_recorded_at(&content),
      "subject": candidate_field_string(&content, "subject"),
      "predicate": candidate_field_string(&content, "predicate"),
      "object": candidate_field_string(&content, "object"),
      "query": candidate_field_string(&content, "query"),
      "intent": candidate_field_string(&content, "intent"),
      "source_rule": candidate_field_string(&content, "source-rule"),
      "trace_ref": candidate_field_string(&content, "trace_ref")
        .or_else(|| candidate_field_string(&content, "trace-ref")),
      "searchable": searchable,
      "topic_hit_total": topic_hit_total,
    }));
  }
  Ok(hits)
}

fn query_context_event_records(topic_tokens: &[String]) -> Result<Vec<Value>> {
  let events_path = gate_store_root()?.join("events.jsonl");
  if !events_path.exists() {
    return Ok(Vec::new());
  }
  let content =
    fs::read_to_string(&events_path).with_context(|| format!("read {}", events_path.display()))?;
  let mut hits = Vec::new();
  for line in content.lines() {
    if line.trim().is_empty() {
      continue;
    }
    let Ok(value) = serde_json::from_str::<Value>(line) else {
      continue;
    };
    let searchable = normalize_intent(line);
    let topic_hit_total = topic_hit_total(topic_tokens, &searchable);
    hits.push(json!({
      "event": value.get("event").and_then(Value::as_str).map(str::to_string),
      "recorded_at": value.get("recorded_at").and_then(Value::as_str).map(str::to_string),
      "provider": value.get("provider").and_then(Value::as_str).map(str::to_string),
      "session_id": value.get("session_id").and_then(Value::as_str).map(str::to_string),
      "turn_id": value.get("turn_id").and_then(Value::as_str).map(str::to_string),
      "tool_name": value.get("tool_name").and_then(Value::as_str).map(str::to_string),
      "message_role": value.get("message_role").and_then(Value::as_str).map(str::to_string),
      "phase": value.get("phase").and_then(Value::as_str).map(str::to_string),
      "searchable": searchable,
      "topic_hit_total": topic_hit_total,
    }));
  }
  Ok(hits)
}

fn query_context_recipe_records(topic_tokens: &[String]) -> Result<Vec<Value>> {
  let path = live_px_dir()?.join("repair-recipes.px");
  if !path.exists() {
    return Ok(Vec::new());
  }
  let content = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
  let mut hits = Vec::new();
  for (recipe_id, block) in quoted_attrset_entries(&content, "recipes") {
    let signature = attrset_field_block(&block, "signature");
    let tool_name = signature
      .as_deref()
      .and_then(|sig| candidate_field_string(sig, "tool_name"))
      .unwrap_or_default();
    let context = signature
      .as_deref()
      .and_then(|sig| candidate_field_string(sig, "context"))
      .unwrap_or_default();
    let arg_predicates = signature
      .as_deref()
      .map(|sig| candidate_field_list(sig, "arg_predicates"))
      .unwrap_or_default();
    let subject = candidate_field_string(&block, "subject").unwrap_or_default();
    let confidence = candidate_field_string(&block, "confidence")
      .and_then(|raw| raw.parse::<f64>().ok())
      .unwrap_or(0.0);
    let invalidated_by = candidate_field_list(&block, "invalidated-by");
    let hold_reason = nonblank(candidate_field_string(&block, "hold-reason"));
    let last_effective_at = candidate_field_string(&block, "last-effective-at");
    let searchable = normalize_intent(&format!(
      "{} {} {} {} {}",
      recipe_id,
      tool_name,
      context,
      subject,
      arg_predicates.join(" ")
    ));
    let topic_hit_total = topic_hit_total(topic_tokens, &searchable);
    hits.push(json!({
      "recipe_id": recipe_id,
      "tool_name": tool_name,
      "context": context,
      "arg_predicates": arg_predicates,
      "subject": subject,
      "confidence": confidence,
      "invalidated_by": invalidated_by,
      "hold_reason": hold_reason,
      "last_effective_at": last_effective_at,
      "searchable": searchable,
      "topic_hit_total": topic_hit_total,
    }));
  }
  Ok(hits)
}

fn quoted_attrset_entries(content: &str, field: &str) -> Vec<(String, String)> {
  let Some(block) = attrset_field_block(content, field) else {
    return Vec::new();
  };
  let mut entries = Vec::new();
  let mut current_key: Option<String> = None;
  let mut current_block = String::new();
  let mut depth = 0isize;

  for line in block.lines() {
    let trimmed = line.trim_start();
    if current_key.is_none() {
      if let Some(rest) = trimmed.strip_prefix('"') {
        if let Some((key, _)) = rest.split_once("\" = {") {
          current_key = Some(key.to_string());
          depth = brace_delta(line);
          current_block.push_str(line);
          current_block.push('\n');
          if depth == 0 {
            entries.push((key.to_string(), std::mem::take(&mut current_block)));
            current_key = None;
          }
        }
      }
      continue;
    }

    depth += brace_delta(line);
    current_block.push_str(line);
    current_block.push('\n');
    if depth == 0 {
      if let Some(key) = current_key.take() {
        entries.push((key, std::mem::take(&mut current_block)));
      }
    }
  }

  entries
}

fn attrset_field_block(content: &str, field: &str) -> Option<String> {
  let marker = format!("{field} = {{");
  let mut found = false;
  let mut depth = 0isize;
  let mut block = String::new();

  for line in content.lines() {
    let trimmed = line.trim_start();
    if !found {
      if trimmed.starts_with(&marker) {
        found = true;
        depth = brace_delta(line);
        block.push_str(line);
        block.push('\n');
        if depth == 0 {
          return Some(block);
        }
      }
      continue;
    }

    depth += brace_delta(line);
    block.push_str(line);
    block.push('\n');
    if depth == 0 {
      return Some(block);
    }
  }

  None
}

fn brace_delta(line: &str) -> isize {
  line.chars().fold(0isize, |acc, ch| match ch {
    '{' => acc + 1,
    '}' => acc - 1,
    _ => acc,
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
        rendered.push_str(key);
        rendered.push_str(" = ");
        rendered.push_str(&emit_px_json_value(value));
        rendered.push_str("; ");
      }
      rendered.push('}');
      rendered
    }
  }
}

fn workspace_root() -> Result<PathBuf> {
  if let Some(raw) = nonblank(env::var("PNIX_WORKSPACE_ROOT").ok()) {
    return Ok(PathBuf::from(raw));
  }
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
  if let Some(home) = home_dir() {
    let default_state = home
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

fn live_px_dir() -> Result<PathBuf> {
  if let Some(raw) = nonblank(env::var("PNIX_GATE_LIVE_PX_DIR").ok()) {
    return Ok(PathBuf::from(raw));
  }
  Ok(workspace_root()?.join("pnix-gate").join("px").join("live"))
}

fn eval_px_file_json(path: &Path) -> Result<Value> {
  let source = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
  eval_px_source_json(&source)
}

fn append_gate_event(payload: &Value) -> Result<()> {
  let path = gate_store_root()?.join("events.jsonl");
  if let Some(parent) = path.parent() {
    if !parent.as_os_str().is_empty() {
      fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
  }
  let mut file = fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(&path)
    .with_context(|| format!("open {}", path.display()))?;
  writeln!(file, "{}", serde_json::to_string(payload)?)
    .with_context(|| format!("append {}", path.display()))?;
  Ok(())
}

fn eval_gate_read_metrics_owner(owner_fn: &str, payload: &Value) -> Result<Value> {
  let source = format!(
    "let\n  root = \"{}\";\n  owner = import (root + \"/stdlib/lib/gate/read-metrics.px\");\n  payload = {};\nin owner.{} payload\n",
    workspace_root()?.display(),
    emit_px_json_value(payload),
    owner_fn
  );
  eval_px_source_json(&source)
}

fn eval_gate_lookup_context_owner(owner_fn: &str, payload: &Value) -> Result<Value> {
  let source = format!(
    "let\n  root = \"{}\";\n  owner = import (root + \"/stdlib/lib/gate/lookup-context.px\");\n  payload = {};\nin owner.{} payload\n",
    workspace_root()?.display(),
    emit_px_json_value(payload),
    owner_fn
  );
  eval_px_source_json(&source)
}

fn eval_gate_storage_telemetry_owner(owner_fn: &str, payload: &Value) -> Result<Value> {
  let source = format!(
    "let\n  root = \"{}\";\n  owner = import (root + \"/stdlib/lib/gate/storage-telemetry.px\");\n  payload = {};\nin owner.{} payload\n",
    workspace_root()?.display(),
    emit_px_json_value(payload),
    owner_fn
  );
  eval_px_source_json(&source)
}

fn eval_brain_ankh_policy_owner(owner_fn: &str, payload: &Value) -> Result<Value> {
  let source = format!(
    "let\n  root = \"{}\";\n  owner = import (root + \"/stdlib/lib/gate/brain-ankh-policy.px\");\n  payload = {};\nin owner.{} payload\n",
    workspace_root()?.display(),
    emit_px_json_value(payload),
    owner_fn
  );
  let json_text = pnix_runtime_legacy::eval_and_format(
    &source,
    false,
    pnix_runtime_legacy::output::OutputFormat::Json,
  )
  .map_err(|err| anyhow!("evaluate brain-ankh-policy .px owner: {err}"))?;
  serde_json::from_str(&json_text).context("parse brain-ankh-policy owner JSON")
}

fn parse_json_value<T: DeserializeOwned>(value: Value) -> Result<T> {
  serde_json::from_value(value).context("parse gate-read owner report")
}

fn parse_json_array<T: DeserializeOwned>(value: Option<&Value>) -> Vec<T> {
  value
    .and_then(Value::as_array)
    .map(|items| {
      items
        .iter()
        .filter_map(|item| serde_json::from_value::<T>(item.clone()).ok())
        .collect()
    })
    .unwrap_or_default()
}

fn read_gate_events() -> Result<Vec<Value>> {
  let path = gate_store_root()?.join("events.jsonl");
  if !path.exists() {
    return Ok(Vec::new());
  }
  let content = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
  Ok(
    content
      .lines()
      .filter(|line| !line.trim().is_empty())
      .filter_map(|line| serde_json::from_str::<Value>(line).ok())
      .collect(),
  )
}

fn collect_px_files(root: &Path) -> Result<Vec<PathBuf>> {
  let mut files = Vec::new();
  if !root.exists() {
    return Ok(files);
  }
  collect_px_files_inner(root, &mut files)?;
  files.sort();
  Ok(files)
}

fn collect_px_files_inner(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
  for entry in fs::read_dir(root).with_context(|| format!("read dir {}", root.display()))? {
    let entry = entry?;
    let path = entry.path();
    if path.is_dir() {
      collect_px_files_inner(&path, out)?;
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("px") {
      out.push(path);
    }
  }
  Ok(())
}

fn directory_entry_names(root: &Path) -> Result<Vec<String>> {
  if !root.exists() {
    return Ok(Vec::new());
  }
  let mut names = fs::read_dir(root)
    .with_context(|| format!("read dir {}", root.display()))?
    .filter_map(|entry| {
      entry.ok().and_then(|item| {
        item
          .file_name()
          .to_str()
          .map(str::trim)
          .filter(|name| !name.is_empty())
          .map(str::to_string)
      })
    })
    .collect::<Vec<_>>();
  names.sort();
  Ok(names)
}

fn directory_entry_byte_map(root: &Path) -> Result<BTreeMap<String, u64>> {
  if !root.exists() {
    return Ok(BTreeMap::new());
  }
  let mut counts = BTreeMap::new();
  for entry in fs::read_dir(root).with_context(|| format!("read dir {}", root.display()))? {
    let entry = entry?;
    let name = entry.file_name();
    let Some(name) = name.to_str().map(str::trim).filter(|name| !name.is_empty()) else {
      continue;
    };
    counts.insert(name.to_string(), path_total_bytes(&entry.path())?);
  }
  Ok(counts)
}

fn gate_status_roots(store_root: &Path) -> Vec<(&'static str, PathBuf)> {
  vec![
    ("candidate", store_root.join("px").join("candidates")),
    ("accepted", store_root.join("px").join("accepted")),
    ("held", store_root.join("px").join("held")),
    ("rejected", store_root.join("px").join("rejected")),
    ("quarantined", store_root.join("px").join("quarantined")),
    ("completed", store_root.join("px").join("completed")),
    ("reopened", store_root.join("px").join("reopened")),
    ("retired", store_root.join("px").join("retired")),
  ]
}

fn value_string(value: Option<&Value>) -> Option<String> {
  value.and_then(Value::as_str).and_then(|raw| {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
      None
    } else {
      Some(trimmed.to_string())
    }
  })
}

fn value_string_list(value: Option<&Value>) -> Vec<String> {
  value
    .and_then(Value::as_array)
    .map(|items| {
      items
        .iter()
        .filter_map(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
    })
    .unwrap_or_default()
}

fn value_bool_map(value: Option<&Value>) -> BTreeMap<String, bool> {
  value
    .and_then(Value::as_object)
    .map(|items| {
      items
        .iter()
        .filter_map(|(key, value)| value.as_bool().map(|flag| (key.clone(), flag)))
        .collect()
    })
    .unwrap_or_default()
}

fn value_u64_map(value: Option<&Value>) -> BTreeMap<String, u64> {
  value
    .and_then(Value::as_object)
    .map(|items| {
      items
        .iter()
        .filter_map(|(key, value)| value_as_u64(value).map(|count| (key.clone(), count)))
        .collect()
    })
    .unwrap_or_default()
}

fn parse_state_sink_lane_spec(value: &Value) -> StateSinkLaneSpec {
  StateSinkLaneSpec {
    status: value_string(value.get("status")).unwrap_or_default(),
    relative_path: value_string(value.get("relative_path")).unwrap_or_default(),
    materialized_path: value_string(value.get("materialized_path")).unwrap_or_default(),
    tier: value_string(value.get("tier")).unwrap_or_default(),
    lifecycle_role: value_string(value.get("lifecycle_role")).unwrap_or_default(),
  }
}

fn parse_state_sink_lane_specs(value: Option<&Value>) -> Vec<StateSinkLaneSpec> {
  value
    .and_then(Value::as_array)
    .map(|items| items.iter().map(parse_state_sink_lane_spec).collect())
    .unwrap_or_default()
}

fn parse_artifact_ref_status_counts(
  value: Option<&Value>,
) -> BTreeMap<String, ArtifactRefCoverageStatusSummary> {
  value
    .and_then(Value::as_object)
    .map(|items| {
      items
        .iter()
        .map(|(status, summary)| {
          (
            status.clone(),
            ArtifactRefCoverageStatusSummary {
              record_total: summary
                .get("record_total")
                .and_then(value_as_u64)
                .unwrap_or(0) as usize,
              with_artifact_ref_total: summary
                .get("with_artifact_ref_total")
                .and_then(value_as_u64)
                .unwrap_or(0) as usize,
              field_counts: value_u64_map(summary.get("field_counts"))
                .into_iter()
                .map(|(field, count)| (field, count as usize))
                .collect(),
            },
          )
        })
        .collect()
    })
    .unwrap_or_default()
}

fn object_value_aliases<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
  value
    .as_object()
    .and_then(|map| keys.iter().find_map(|key| map.get(*key)))
}

fn join_or_none(values: &[&str]) -> String {
  if values.is_empty() {
    "none".to_string()
  } else {
    values.join(", ")
  }
}

#[derive(Debug, Clone)]
struct ArtifactRefCoverageRecord {
  status: String,
  artifact_ref_fields: Vec<String>,
  has_artifact_ref: bool,
}

#[derive(Debug, Clone)]
struct StorageSnapshotSummary {
  source: &'static str,
  status: &'static str,
  ttl_violation_count: Option<u64>,
  dangling_ref_count: Option<u64>,
  gc_reclaimed_bytes: u64,
}

fn artifact_ref_field_names(content: &str) -> Vec<String> {
  let mut fields = BTreeSet::new();
  for line in content.lines() {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
      continue;
    }
    let Some((raw_name, _)) = trimmed.split_once('=') else {
      continue;
    };
    let field = raw_name.trim();
    if is_artifact_ref_field_name(field) {
      fields.insert(field.to_string());
    }
  }
  fields.into_iter().collect()
}

fn is_artifact_ref_field_name(field: &str) -> bool {
  let normalized = field.replace('-', "_");
  !normalized.is_empty()
    && normalized
      .chars()
      .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    && (normalized == "artifact_ref" || normalized.ends_with("_artifact_ref"))
}

fn collect_artifact_ref_records(store_root: &Path) -> Result<Vec<ArtifactRefCoverageRecord>> {
  let mut records = Vec::new();
  for (status, root) in gate_status_roots(store_root) {
    for path in collect_px_files(&root)? {
      let content = fs::read_to_string(&path).unwrap_or_default();
      let artifact_ref_fields = artifact_ref_field_names(&content);
      records.push(ArtifactRefCoverageRecord {
        status: status.to_string(),
        artifact_ref_fields: artifact_ref_fields.clone(),
        has_artifact_ref: !artifact_ref_fields.is_empty(),
      });
    }
  }
  Ok(records)
}

fn storage_snapshot_summary(control_plane_dir: &Path) -> StorageSnapshotSummary {
  let evaluation = json_path_report(Some(control_plane_dir.join("evaluation.json")));
  if let Some(storage) = nested_object_value(evaluation.value.as_ref(), &["details", "storage"]) {
    return StorageSnapshotSummary {
      source: "control-plane/evaluation.json.details.storage",
      status: "evaluation-storage",
      ttl_violation_count: storage.get("ttl_violation_count").and_then(value_as_u64),
      dangling_ref_count: storage.get("dangling_ref_count").and_then(value_as_u64),
      gc_reclaimed_bytes: storage
        .get("gc_reclaimed_bytes")
        .and_then(value_as_u64)
        .unwrap_or(0),
    };
  }
  let overview = json_path_report(Some(control_plane_dir.join("overview.json")));
  if let Some(storage) = nested_object_value(overview.value.as_ref(), &["storage"]) {
    return StorageSnapshotSummary {
      source: "control-plane/overview.json.storage",
      status: "overview-storage",
      ttl_violation_count: storage.get("ttl_violation_count").and_then(value_as_u64),
      dangling_ref_count: storage.get("dangling_ref_count").and_then(value_as_u64),
      gc_reclaimed_bytes: storage
        .get("gc_reclaimed_bytes")
        .and_then(value_as_u64)
        .unwrap_or(0),
    };
  }
  StorageSnapshotSummary {
    source: "missing-control-plane-storage",
    status: "missing",
    ttl_violation_count: None,
    dangling_ref_count: None,
    gc_reclaimed_bytes: 0,
  }
}

fn nested_object_value<'a>(value: Option<&'a Value>, path: &[&str]) -> Option<&'a Value> {
  let mut current = value?;
  for segment in path {
    current = current.get(*segment)?;
  }
  Some(current)
}

fn candidate_field_string(content: &str, field: &str) -> Option<String> {
  field_string_aliases(content, &[field])
}

fn candidate_kind_value(content: &str) -> String {
  candidate_field_string(content, "kind")
    .or_else(|| candidate_header_kind(content))
    .filter(|value| !value.trim().is_empty())
    .unwrap_or_else(|| "<missing-kind>".to_string())
}

fn unsupported_kind_value(content: &str) -> String {
  candidate_field_string(content, "unsupported-kind")
    .or_else(|| candidate_field_string(content, "unsupported_kind"))
    .or_else(|| candidate_field_string(content, "schema-todo-kind"))
    .or_else(|| candidate_field_string(content, "schema_todo_kind"))
    .or_else(|| candidate_header_kind(content))
    .filter(|value| !value.trim().is_empty())
    .unwrap_or_else(|| "<missing-kind>".to_string())
}

fn candidate_field_list(content: &str, field: &str) -> Vec<String> {
  field_list_aliases(content, &[field])
}

fn candidate_header_kind(content: &str) -> Option<String> {
  content.lines().find_map(|line| {
    line
      .trim()
      .strip_prefix("# kind: ")
      .map(str::trim)
      .filter(|value| !value.is_empty())
      .map(str::to_string)
  })
}

fn field_string_aliases(content: &str, fields: &[&str]) -> Option<String> {
  field_value_fragment(content, fields).and_then(parse_quoted_string)
}

fn field_list_aliases(content: &str, fields: &[&str]) -> Vec<String> {
  field_value_fragment(content, fields)
    .map(|fragment| parse_quoted_string_list(&fragment))
    .unwrap_or_default()
}

fn field_value_fragment(content: &str, fields: &[&str]) -> Option<String> {
  let prefixes = fields
    .iter()
    .map(|field| format!("{field} ="))
    .collect::<Vec<_>>();
  let mut collecting = false;
  let mut fragment = String::new();
  let mut square_depth = 0isize;
  let mut curly_depth = 0isize;
  let mut in_string = false;
  let mut escape = false;

  for line in content.lines() {
    let trimmed = line.trim();
    let segment = if collecting {
      trimmed
    } else if let Some(prefix) = prefixes
      .iter()
      .find(|prefix| trimmed.starts_with(prefix.as_str()))
    {
      collecting = true;
      trimmed[prefix.len()..].trim()
    } else {
      continue;
    };

    for ch in segment.chars() {
      fragment.push(ch);
      if in_string {
        if escape {
          escape = false;
          continue;
        }
        match ch {
          '\\' => escape = true,
          '"' => in_string = false,
          _ => {}
        }
        continue;
      }
      match ch {
        '"' => in_string = true,
        '[' => square_depth += 1,
        ']' => square_depth -= 1,
        '{' => curly_depth += 1,
        '}' => curly_depth -= 1,
        ';' if square_depth == 0 && curly_depth == 0 => {
          return Some(fragment.trim().trim_end_matches(';').trim().to_string());
        }
        _ => {}
      }
    }
    fragment.push('\n');
  }
  None
}

fn parse_quoted_string(fragment: String) -> Option<String> {
  fragment
    .trim()
    .strip_prefix('"')
    .and_then(|value| value.strip_suffix('"'))
    .map(str::to_string)
}

fn parse_quoted_string_list(fragment: &str) -> Vec<String> {
  let mut items = Vec::new();
  let mut current = String::new();
  let mut in_string = false;
  let mut escape = false;
  for ch in fragment.chars() {
    if in_string {
      if escape {
        current.push(ch);
        escape = false;
        continue;
      }
      match ch {
        '\\' => escape = true,
        '"' => {
          items.push(std::mem::take(&mut current));
          in_string = false;
        }
        _ => current.push(ch),
      }
      continue;
    }
    if ch == '"' {
      in_string = true;
    }
  }
  items
}

fn candidate_recorded_at(content: &str) -> Option<String> {
  content.lines().find_map(|line| {
    line
      .trim()
      .strip_prefix("# recorded_at: ")
      .map(str::to_string)
  })
}

fn provenance_tag_value(tags: &[String], prefix: &str) -> Option<String> {
  tags
    .iter()
    .find_map(|value| value.strip_prefix(prefix).map(str::to_string))
}

fn provenance_floor_sample(record: &ProvenanceFloorRecord) -> ProvenanceFloorSample {
  ProvenanceFloorSample {
    path: record.path.clone(),
    status: record.status.clone(),
    session_id: record.session_id.clone(),
    turn_id: record.turn_id.clone(),
    tool_call_id: record.tool_call_id.clone(),
    tool_call_required: record.tool_call_required,
    failure_codes: record.failure_codes.clone(),
    quarantine_reason: record.quarantine_reason.clone(),
    provenance_floor_status: record.provenance_floor_status.clone(),
  }
}

fn unsupported_kind_sample(record: &UnsupportedKindRecord) -> UnsupportedKindSample {
  UnsupportedKindSample {
    path: record.path.clone(),
    status: record.status.clone(),
    kind: record.kind.clone(),
    quarantine_reason: record.quarantine_reason.clone(),
    kind_support_status: record.kind_support_status.clone(),
    schema_todo_status: record.schema_todo_status.clone(),
  }
}

fn lineage_floor_sample(record: &LineageFloorRecord) -> LineageFloorSample {
  LineageFloorSample {
    path: record.path.clone(),
    status: record.status.clone(),
    kind: record.kind.clone(),
    source_session_id: record.source_session_id.clone(),
    source_turn_id: record.source_turn_id.clone(),
    derived_from_candidate_id: record.derived_from_candidate_id.clone(),
    parent_packet_ids: record.parent_packet_ids.clone(),
    lineage_anchor_required: record.lineage_anchor_required,
    lineage_floor_status: record.lineage_floor_status.clone(),
    computed_lineage_floor_status: record.computed_lineage_floor_status.clone(),
    failure_codes: record.failure_codes.clone(),
  }
}

fn normalize_intent(input: &str) -> String {
  input.trim().to_ascii_lowercase()
}

fn intent_words(input: &str) -> Vec<String> {
  let mut current = String::new();
  let mut words = Vec::new();
  for ch in normalize_intent(input).chars() {
    if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
      current.push(ch);
    } else if !current.is_empty() {
      words.push(std::mem::take(&mut current));
    }
  }
  if !current.is_empty() {
    words.push(current);
  }
  words
}

fn topic_hit_total(topic_tokens: &[String], searchable: &str) -> usize {
  let searchable = normalize_intent(searchable);
  topic_tokens
    .iter()
    .filter(|token| !token.is_empty() && searchable.contains(token.as_str()))
    .count()
}

fn opt_string(value: &Value, key: &str) -> Option<String> {
  value.get(key).and_then(Value::as_str).map(str::to_string)
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

#[derive(Debug, Clone)]
struct JsonPathReportInternal {
  resolved_path: Option<PathBuf>,
  exists: bool,
  parse_error: bool,
  value: Option<Value>,
}

#[derive(Debug, Clone)]
struct LiveBufferPressureSummary {
  live_buffer_open_count: u64,
  live_buffer_dirty_count: u64,
  live_buffer_error_count: u64,
  live_buffer_open_bytes: u64,
  live_buffer_dirty_bytes: u64,
  live_buffer_parse_pass_rate: Option<f64>,
  snapshot_updated_at: Option<String>,
  snapshot_source: Option<String>,
}

#[derive(Debug, Clone)]
struct HotStoreBudgetCheckpointSummary {
  status: String,
  checkpoint_total: usize,
  budget_exceeded_total: usize,
  suppressed_total: usize,
  session_total: usize,
  latest_recorded_at: Option<String>,
  latest_inline_blob_mode: Option<String>,
  latest_budget_exceeded: Option<bool>,
  latest_hot_store_bytes: Option<u64>,
  latest_effective_hot_store_bytes: Option<u64>,
  latest_budget_remaining_bytes: Option<u64>,
  latest_pressure_ratio: Option<f64>,
}

fn supported_store_profile_kinds() -> Vec<String> {
  vec![
    "domain-lobe".to_string(),
    "local-embedded".to_string(),
    "shared-runtime".to_string(),
  ]
}

fn store_profile_kind(store_root: &Path) -> String {
  if let Some(override_value) = normalized_store_profile_override() {
    return override_value;
  }
  let root = store_root.display().to_string().to_lowercase();
  if root.contains("/doghouse-") {
    "domain-lobe".to_string()
  } else if root.contains("/uppnix/doghouse") {
    "shared-runtime".to_string()
  } else {
    "local-embedded".to_string()
  }
}

fn normalized_store_profile_override() -> Option<String> {
  let raw = nonblank(env::var("PNIX_GATE_STORE_PROFILE").ok())?;
  let lowered = raw.trim().to_lowercase();
  supported_store_profile_kinds()
    .into_iter()
    .find(|candidate| candidate == &lowered)
}

fn store_profile_source() -> String {
  if normalized_store_profile_override().is_some() {
    "PNIX_GATE_STORE_PROFILE".to_string()
  } else if nonblank(env::var("PNIX_GATE_STORE_DIR").ok()).is_some() {
    "PNIX_GATE_STORE_DIR".to_string()
  } else if nonblank(env::var("DOGHOUSE_RUNTIME_DIR").ok()).is_some() {
    "DOGHOUSE_RUNTIME_DIR".to_string()
  } else if nonblank(env::var("XDG_STATE_HOME").ok()).is_some() {
    "XDG_STATE_HOME".to_string()
  } else {
    "default-home".to_string()
  }
}

fn portable_domain_bundle_path() -> Result<PathBuf> {
  Ok(
    workspace_root()?
      .join("docs")
      .join("puck")
      .join("examples")
      .join("portable-domain-bundle.v0.1.json"),
  )
}

fn capability_manifest_schema_path() -> Result<PathBuf> {
  Ok(
    workspace_root()?
      .join("docs")
      .join("puck")
      .join("capability-manifest-v0.1.schema.json"),
  )
}

fn json_path_payload(source: JsonPathReportInternal) -> Value {
  json!({
    "path": repo_relative_path(source.resolved_path.as_deref()),
    "exists": source.exists,
    "parse_error": source.parse_error,
    "value": source.value,
  })
}

fn brain_bundle_validation_payload(
  bundle_source: JsonPathReportInternal,
  proof_source: JsonPathReportInternal,
  schema_source: JsonPathReportInternal,
  expected_bundle_kind: Option<String>,
  expected_lobe_profile: Option<String>,
  expected_proof_kind: Option<String>,
  read_owner: &'static str,
  generated_at: String,
) -> Value {
  json!({
    "generated_at": generated_at,
    "read_owner": read_owner,
    "bundle_source": json_path_payload(bundle_source),
    "proof_source": json_path_payload(proof_source),
    "schema_source": json_path_payload(schema_source),
    "expected_bundle_kind": expected_bundle_kind,
    "expected_lobe_profile": expected_lobe_profile,
    "expected_proof_kind": expected_proof_kind,
  })
}

fn resolve_readable_path(raw: &str, base_dir: Option<&Path>) -> Result<PathBuf> {
  let raw = raw.trim();
  if raw.is_empty() {
    bail!("path must not be empty");
  }
  let raw_path = PathBuf::from(raw);
  let mut candidates = Vec::new();
  let mut push_candidate = |path: PathBuf| {
    if !candidates.iter().any(|existing| existing == &path) {
      candidates.push(path);
    }
  };
  if raw_path.is_absolute() {
    push_candidate(raw_path.clone());
  } else {
    if let Some(base) = base_dir {
      push_candidate(base.join(&raw_path));
    }
    push_candidate(workspace_root()?.join(&raw_path));
    if let Ok(current_dir) = env::current_dir() {
      push_candidate(current_dir.join(&raw_path));
    }
    push_candidate(raw_path.clone());
  }
  let fallback = candidates
    .first()
    .cloned()
    .unwrap_or_else(|| PathBuf::from(raw));
  Ok(
    candidates
      .into_iter()
      .find(|path| path.exists())
      .unwrap_or(fallback),
  )
}

fn json_path_report(path: Option<PathBuf>) -> JsonPathReportInternal {
  let resolved_path = path.filter(|path| !path.as_os_str().is_empty());
  let exists = resolved_path
    .as_ref()
    .map(|path| path.exists())
    .unwrap_or(false);
  let value = resolved_path
    .as_ref()
    .filter(|_| exists)
    .and_then(|path| fs::read_to_string(path).ok())
    .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());
  JsonPathReportInternal {
    resolved_path,
    exists,
    parse_error: exists && value.is_none(),
    value,
  }
}

fn repo_relative_path(path: Option<&Path>) -> Option<String> {
  let path = path?;
  let root = workspace_root().ok()?;
  path
    .strip_prefix(&root)
    .map(|relative| relative.display().to_string())
    .ok()
    .or_else(|| Some(path.display().to_string()))
}

fn scalar_string(value: &Value) -> Option<String> {
  match value {
    Value::Null => None,
    Value::String(raw) => nonblank(Some(raw.clone())),
    Value::Bool(flag) => Some(flag.to_string()),
    Value::Number(number) => Some(number.to_string()),
    _ => None,
  }
}

fn live_buffer_pressure_summary(path: &Path) -> Result<LiveBufferPressureSummary> {
  if !path.exists() {
    return Ok(LiveBufferPressureSummary {
      live_buffer_open_count: 0,
      live_buffer_dirty_count: 0,
      live_buffer_error_count: 0,
      live_buffer_open_bytes: 0,
      live_buffer_dirty_bytes: 0,
      live_buffer_parse_pass_rate: None,
      snapshot_updated_at: None,
      snapshot_source: None,
    });
  }
  let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
  let payload: Value =
    serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
  let buffers = payload
    .get("buffers")
    .and_then(Value::as_array)
    .cloned()
    .unwrap_or_default();
  let dirty_buffers = buffers
    .iter()
    .filter(|value| value.get("dirty").and_then(Value::as_bool) == Some(true))
    .collect::<Vec<_>>();
  let dirty_bytes = dirty_buffers
    .iter()
    .map(|value| value.get("length").and_then(value_as_u64).unwrap_or(0))
    .sum::<u64>();
  let open_bytes = buffers
    .iter()
    .map(|value| value.get("length").and_then(value_as_u64).unwrap_or(0))
    .sum::<u64>();
  let summary = payload.get("summary").cloned().unwrap_or_else(|| json!({}));
  Ok(LiveBufferPressureSummary {
    live_buffer_open_count: summary
      .get("open_count")
      .and_then(value_as_u64)
      .unwrap_or(buffers.len() as u64),
    live_buffer_dirty_count: summary
      .get("dirty_count")
      .and_then(value_as_u64)
      .unwrap_or(dirty_buffers.len() as u64),
    live_buffer_error_count: summary
      .get("error_count")
      .and_then(value_as_u64)
      .unwrap_or(0),
    live_buffer_open_bytes: summary
      .get("total_char_count")
      .and_then(value_as_u64)
      .unwrap_or(open_bytes),
    live_buffer_dirty_bytes: dirty_bytes,
    live_buffer_parse_pass_rate: summary.get("parse_pass_rate").and_then(Value::as_f64),
    snapshot_updated_at: payload.get("updated_at").and_then(scalar_string),
    snapshot_source: payload.get("source").and_then(scalar_string),
  })
}

fn hot_store_budget_checkpoint_summary() -> Result<HotStoreBudgetCheckpointSummary> {
  let checkpoints = read_gate_events()?
    .into_iter()
    .filter(|value| value.get("event").and_then(Value::as_str) == Some("HotStoreBudgetCheckpoint"))
    .collect::<Vec<_>>();
  let checkpoint_total = checkpoints.len();
  let budget_exceeded_total = checkpoints
    .iter()
    .filter(|value| {
      value
        .get("hot_store_budget_exceeded")
        .and_then(Value::as_bool)
        == Some(true)
    })
    .count();
  let suppressed_total = checkpoints
    .iter()
    .filter(|value| {
      value.get("inline_blob_mode").and_then(Value::as_str) == Some("artifact-ref-only")
    })
    .count();
  let session_total = checkpoints
    .iter()
    .filter_map(|value| value.get("session_id").and_then(Value::as_str))
    .collect::<BTreeSet<_>>()
    .len();
  let latest = checkpoints
    .iter()
    .max_by(|a, b| opt_string(a, "recorded_at").cmp(&opt_string(b, "recorded_at")));
  let latest_budget_exceeded = latest.and_then(|value| {
    value
      .get("hot_store_budget_exceeded")
      .and_then(Value::as_bool)
  });
  let status = if checkpoint_total == 0 {
    "empty".to_string()
  } else if latest_budget_exceeded == Some(true) {
    "pressure".to_string()
  } else if suppressed_total == checkpoint_total {
    "suppressed".to_string()
  } else {
    "within-budget".to_string()
  };
  Ok(HotStoreBudgetCheckpointSummary {
    status,
    checkpoint_total,
    budget_exceeded_total,
    suppressed_total,
    session_total,
    latest_recorded_at: latest.and_then(|value| opt_string(value, "recorded_at")),
    latest_inline_blob_mode: latest.and_then(|value| opt_string(value, "inline_blob_mode")),
    latest_budget_exceeded,
    latest_hot_store_bytes: latest
      .and_then(|value| value.get("hot_store_bytes").and_then(value_as_u64)),
    latest_effective_hot_store_bytes: latest.and_then(|value| {
      value
        .get("effective_hot_store_bytes")
        .and_then(value_as_u64)
    }),
    latest_budget_remaining_bytes: latest.and_then(|value| {
      value
        .get("hot_store_budget_remaining_bytes")
        .and_then(value_as_u64)
    }),
    latest_pressure_ratio: latest.and_then(|value| {
      value
        .get("hot_store_pressure_ratio")
        .and_then(Value::as_f64)
    }),
  })
}

fn path_total_bytes(path: &Path) -> Result<u64> {
  if !path.exists() {
    return Ok(0);
  }
  if path.is_file() {
    return Ok(
      fs::metadata(path)
        .with_context(|| format!("stat {}", path.display()))?
        .len(),
    );
  }
  let mut total = 0_u64;
  for entry in fs::read_dir(path).with_context(|| format!("read dir {}", path.display()))? {
    let entry = entry?;
    total = total.saturating_add(path_total_bytes(&entry.path())?);
  }
  Ok(total)
}

fn value_as_u64(value: &Value) -> Option<u64> {
  match value {
    Value::Number(number) => number.as_u64(),
    Value::String(raw) => raw.trim().parse::<u64>().ok(),
    _ => None,
  }
}

fn home_dir() -> Option<PathBuf> {
  env::var("HOME").ok().map(PathBuf::from)
}

fn iso_now() -> String {
  let ms = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis();
  format!("{ms}")
}

#[cfg(test)]
mod tests {
  use super::*;

  static PX_EVAL_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

  fn gate_signal(kind: &str, source_ref: &str) -> BrainAnkhGateSignalInput {
    BrainAnkhGateSignalInput {
      source_ref: source_ref.to_string(),
      source_path: format!("/tmp/{kind}.px"),
      source_status: "candidate".to_string(),
      kind: kind.to_string(),
      recorded_at: Some("2026-04-29T00:00:00Z".to_string()),
      trace_ref: Some(format!("trace:{source_ref}")),
      subject: Some(format!("subject:{source_ref}")),
      predicate: Some("px-valid".to_string()),
      object: Some("object-detail".to_string()),
      query: Some("check progress".to_string()),
      intent: Some("measure".to_string()),
      selected_card: Some("measure_learning_progress".to_string()),
      ranked_cards: vec![
        "measure_learning_progress".to_string(),
        "measure_dispatch_cycles".to_string(),
      ],
      chooser: Some("pnix-ontology-query-repl".to_string()),
      judgement: Some("measure-first".to_string()),
      confidence: Some("0.91".to_string()),
      dispatch_status: Some("ok".to_string()),
      tool: Some("cargo-test".to_string()),
      evidence: vec!["matched-route::measure-learning".to_string()],
      reasons: vec!["structural progress score is safest first".to_string()],
      provenance: vec!["session:test".to_string()],
    }
  }

  fn assert_snapshot_candidates(report: &BrainAnkhPolicyProjectionReport) {
    assert_eq!(report.system_brain_snapshot_candidate_total, 1);
    assert_eq!(report.ankh_family_snapshot_candidate_total, 1);

    let system_snapshot = &report.system_brain_snapshot_candidates[0];
    assert_eq!(
      system_snapshot.artifact_family,
      "ankh.system-brain-snapshot-candidate"
    );
    assert_eq!(
      system_snapshot.source_of_truth,
      "append-only-event-judgement-promotion-lineage"
    );
    assert_eq!(
      system_snapshot.xml_carrier_status,
      "projection-carrier-only"
    );
    assert!(!system_snapshot.xml_is_truth);
    assert!(!system_snapshot.knowledge_db_export);
    assert!(!system_snapshot.p_puck_green_is_proof);
    assert!(!system_snapshot.mind_map_layout_is_proof);
    assert!(!system_snapshot.proof_reuse_allowed);
    assert!(system_snapshot.same_judge_lifecycle_required);
    assert!(!system_snapshot.store_mutation);
    assert!(!system_snapshot.policy_mutation_applied);
    assert_eq!(
      system_snapshot.promotion_boundary,
      "append-only-lineage-rejudge-receipt-required-before-snapshot-close"
    );

    let family_snapshot = &report.ankh_family_snapshot_candidates[0];
    assert_eq!(
      family_snapshot.artifact_family,
      "ankh.ankh-family-snapshot-candidate"
    );
    assert_eq!(family_snapshot.object_total, 7);
    assert_eq!(family_snapshot.morphism_total, 9);
    assert!(!family_snapshot.formal_category_theory_proof_claimed);
    assert!(family_snapshot.same_judge_lifecycle_required);
    assert!(!family_snapshot.incompatible_family_acceptance_allowed);
    assert_eq!(
      family_snapshot.not_closed_family_status,
      "held-or-reopen-not-accepted"
    );
    assert!(!family_snapshot.xml_is_truth);
    assert!(!family_snapshot.p_puck_green_is_proof);
    assert!(!family_snapshot.proof_reuse_allowed);
    assert!(!family_snapshot.store_mutation);
    assert!(!family_snapshot.policy_mutation_applied);
    assert!(family_snapshot
      .objects
      .iter()
      .all(|object| object.same_judge_lifecycle_required && !object.accepted_without_rejudge));
    assert!(family_snapshot.morphisms.iter().all(|morphism| {
      morphism.same_judge_lifecycle_required && !morphism.accepted_without_receipt
    }));
  }

  fn assert_diagram_candidates(report: &BrainAnkhPolicyProjectionReport) {
    assert_eq!(report.mind_map_projection_candidate_total, 1);
    assert_eq!(report.brain_diagram_packet_candidate_total, 1);
    assert_eq!(report.dashboard_projection_candidate_total, 1);

    let mind_map = &report.mind_map_projection_candidates[0];
    assert_eq!(
      mind_map.artifact_family,
      "ankh.mind-map-projection-candidate"
    );
    assert_eq!(
      mind_map.source_of_truth,
      "append-only-event-judgement-promotion-lineage"
    );
    assert_eq!(
      mind_map.projection_status,
      "candidate-mind-map-projection-observed"
    );
    assert_eq!(mind_map.graph_layout_owner, "projection-surface-only");
    assert!(!mind_map.layout_is_proof);
    assert!(!mind_map.node_edge_schema_is_truth);
    assert!(!mind_map.graph_completeness_claimed);
    assert!(!mind_map.xml_is_truth);
    assert!(!mind_map.p_puck_green_is_proof);
    assert!(!mind_map.proof_reuse_allowed);
    assert!(!mind_map.store_mutation);
    assert!(!mind_map.policy_mutation_applied);
    assert_eq!(
      mind_map.promotion_boundary,
      "lineage-rejudge-receipt-required-before-diagram-close"
    );

    let packet = &report.brain_diagram_packet_candidates[0];
    assert_eq!(
      packet.artifact_family,
      "ankh.brain-diagram-packet-candidate"
    );
    assert_eq!(
      packet.packet_status,
      "candidate-brain-diagram-packet-observed"
    );
    assert_eq!(packet.diagram_node_total, 15);
    assert_eq!(packet.diagram_edge_total, 9);
    assert_eq!(packet.contextual_fact_node_kind, "fact");
    assert!(packet.diagram_node_kinds.iter().any(|kind| kind == "fact"));
    assert!(packet
      .diagram_edge_kinds
      .iter()
      .any(|kind| kind == "requires-proof"));
    assert!(packet
      .lifecycle_flow
      .iter()
      .any(|stage| stage == "ContextualFact"));
    assert!(!packet.contextual_fact_node_is_raw_memory_row);
    assert!(packet.lifecycle_flow_required);
    assert!(!packet.static_graph_viewer_close_allowed);
    assert!(packet.projection_surface_only);
    assert!(!packet.second_truth_owner);
    assert!(!packet.layout_is_proof);
    assert!(!packet.graph_completeness_claimed);
    assert!(!packet.store_mutation);
    assert!(!packet.policy_mutation_applied);

    let dashboard = &report.dashboard_projection_candidates[0];
    assert_eq!(
      dashboard.artifact_family,
      "ankh.dashboard-projection-candidate"
    );
    assert_eq!(
      dashboard.operator_cockpit_status,
      "candidate-private-operator-cockpit"
    );
    assert_eq!(
      dashboard.user_surface_status,
      "candidate-redacted-user-situation-map"
    );
    assert!(dashboard.stale_projection_visible);
    assert!(dashboard.false_green_visible);
    assert!(dashboard.slow_proof_path_visible);
    assert!(dashboard.affected_proof_slice_visible);
    assert!(dashboard.unsupported_quarantine_visible);
    assert!(dashboard.missing_provenance_visible);
    assert!(!dashboard.raw_screenshot_sync_allowed);
    assert!(!dashboard.private_lineage_exposed_to_freecat);
    assert!(!dashboard.privileged_inference_exposed_to_freecat);
    assert!(!dashboard.provider_secret_exposed_to_freecat);
    assert!(!dashboard.canonical_merge_law_exposed_to_freecat);
    assert!(dashboard.projection_surface_only);
    assert!(!dashboard.second_truth_owner);
    assert!(!dashboard.store_mutation);
    assert!(!dashboard.policy_mutation_applied);
  }

  #[test]
  fn brain_ankh_policy_projection_reuses_existing_gate_kinds() {
    let _guard = PX_EVAL_TEST_LOCK.lock().expect("px eval test lock");
    let report = build_brain_ankh_policy_projection_report(
      "2026-04-29T00:00:00Z".to_string(),
      "/tmp/gate".to_string(),
      None,
      20,
      vec![
        gate_signal("selection-trace", "selection-1"),
        gate_signal("chooser-judgement", "chooser-1"),
        gate_signal("dispatch-execution", "dispatch-1"),
        gate_signal("validation-record", "validation-1"),
        gate_signal("repair-recipe", "repair-1"),
      ],
      Vec::new(),
    )
    .expect("brain ankh policy projection report");

    assert_eq!(report.projection_status, "read-only-projection-ready");
    assert!(report.read_only);
    assert!(!report.store_mutation);
    assert_eq!(report.input_totals.selection_trace_total, 1);
    assert_eq!(report.policy_candidate_total, 5);
    assert_eq!(report.routing_decision_total, 1);
    assert_eq!(report.attach_decision_total, 2);
    assert_eq!(report.priority_decision_total, 1);
    assert_eq!(report.research_intent_total, 0);
    assert_eq!(report.source_candidate_total, 0);
    assert_eq!(report.self_explanation_total, 1);
    assert_eq!(report.policy_revision_receipt_total, 3);
    assert_eq!(report.mind_delta_candidate_total, 5);
    assert_eq!(report.affected_mind_slice_total, 5);
    assert_eq!(report.semantic_dependency_edge_total, 5);
    assert_eq!(report.rejudge_receipt_candidate_total, 5);
    assert_eq!(report.targeted_replay_plan_total, 5);
    assert_eq!(report.proof_selection_candidate_total, 5);
    assert_eq!(report.incremental_self_compile_candidate_total, 5);
    assert_snapshot_candidates(&report);
    assert_diagram_candidates(&report);
    assert!(!report.proof_reuse_allowed);
    assert!(!report.semantic_dependency_graph.graph_completeness_claimed);
    assert!(!report.semantic_dependency_graph.xml_parse_success_is_proof);
    assert!(!report.semantic_dependency_graph.p_puck_green_is_proof);
    assert!(report.mind_delta_candidates.iter().all(|delta| {
      !delta.store_mutation
        && !delta.policy_mutation_applied
        && !delta.proof_reuse_allowed
        && !delta.whole_brain_snapshot_claimed
        && !delta.system_brain_snapshot_is_proof
        && !delta.ankh_family_snapshot_is_proof
        && !delta.mind_map_layout_is_proof
        && !delta.graph_completeness_claimed
        && !delta.xml_parse_success_is_proof
        && !delta.p_puck_green_is_proof
        && delta.changed_mind_scope == "changed-fact-rule-route-evaluator-or-ankh-attach-only"
    }));
    assert!(report
      .affected_mind_slices
      .iter()
      .all(|slice| !slice.store_mutation && !slice.policy_mutation_applied));
    assert!(report
      .semantic_dependency_graph
      .edges
      .iter()
      .all(|edge| !edge.proof_reuse_allowed));
    assert!(report.rejudge_receipt_candidates.iter().all(|receipt| {
      !receipt.store_mutation
        && !receipt.policy_mutation_applied
        && !receipt.proof_reuse_allowed
        && !receipt.previous_green_reuse_allowed
        && !receipt.graph_completeness_claimed
        && !receipt.xml_parse_success_is_proof
        && !receipt.p_puck_green_is_proof
        && !receipt.old_green_is_current_proof
        && receipt.receipt_write_status == "candidate-only-no-store-write"
    }));
    assert!(report.targeted_replay_plans.iter().all(|plan| {
      !plan.store_mutation
        && !plan.policy_mutation_applied
        && !plan.proof_reuse_allowed
        && !plan.old_green_is_current_proof
        && !plan.stale_latest_report_is_proof
        && !plan.profile_speedup_is_completion_proof
        && !plan.p_puck_green_is_proof
        && plan.replay_execution_status == "candidate-only-no-run"
    }));
    assert!(report.proof_selection_candidates.iter().all(|candidate| {
      !candidate.store_mutation
        && !candidate.policy_mutation_applied
        && !candidate.proof_reuse_allowed
        && !candidate.old_green_is_current_proof
        && !candidate.stale_latest_report_is_proof
        && !candidate.profile_speedup_is_completion_proof
        && !candidate.p_puck_green_is_proof
        && candidate.proof_selection_status_boundary == "candidate-only-no-proof-execution"
    }));
    assert!(report
      .incremental_self_compile_candidates
      .iter()
      .all(|candidate| {
        !candidate.store_mutation
          && !candidate.policy_mutation_applied
          && !candidate.self_compile_mutation_applied
          && !candidate.compile_reuse_allowed
          && !candidate.proof_reuse_allowed
          && !candidate.previous_green_reuse_allowed
          && !candidate.old_green_is_current_proof
          && !candidate.stale_latest_report_is_proof
          && !candidate.profile_speedup_is_completion_proof
          && !candidate.p_puck_green_is_proof
          && candidate.compiler_owner == "pnixc-meta"
          && candidate.compile_execution_status == "candidate-only-no-compile-run"
      }));
    assert_eq!(
      report.routing_decisions[0].artifact_family,
      "ankh.routing-decision"
    );
    assert_eq!(
      report.self_explanations[0].explanation_boundary,
      "replayable-evidence-link-not-user-facing-prose"
    );
    assert_eq!(
      report.attach_decisions[0].artifact_family,
      "ankh.attach-decision"
    );
    assert_eq!(report.attach_decisions[0].decision_axis, "attach");
    assert_eq!(
      report.priority_decisions[0].artifact_family,
      "ankh.priority-decision"
    );
    assert_eq!(report.priority_decisions[0].decision_axis, "priority");
    assert!(report
      .policy_revision_receipts
      .iter()
      .all(|receipt| !receipt.policy_mutation_applied));
  }

  #[test]
  fn brain_ankh_policy_projection_maps_relevant_observation_atoms_only() {
    let _guard = PX_EVAL_TEST_LOCK.lock().expect("px eval test lock");
    let mut held = gate_signal("observation-atom", "observation-held-1");
    held.subject = Some("held-reason".to_string());
    held.predicate = Some("feeds".to_string());
    held.object = Some("brain-ankh-self-upgrade-loop".to_string());

    let mut irrelevant = gate_signal("observation-atom", "observation-irrelevant-1");
    irrelevant.subject = Some("freecat".to_string());
    irrelevant.predicate = Some("is".to_string());
    irrelevant.object = Some("projection-only-surface".to_string());

    let report = build_brain_ankh_policy_projection_report(
      "2026-04-29T00:00:00Z".to_string(),
      "/tmp/gate".to_string(),
      None,
      20,
      vec![held, irrelevant],
      Vec::new(),
    )
    .expect("brain ankh policy projection report");

    assert_eq!(report.input_totals.observation_atom_total, 2);
    assert_eq!(report.policy_candidate_total, 1);
    assert_eq!(report.attach_decision_total, 1);
    assert_eq!(report.research_intent_total, 1);
    assert_eq!(report.source_candidate_total, 1);
    assert_eq!(report.mind_delta_candidate_total, 1);
    assert_eq!(report.affected_mind_slice_total, 1);
    assert_eq!(
      report.mind_delta_candidates[0].artifact_family,
      "ankh.mind-delta-candidate"
    );
    assert_eq!(
      report.mind_delta_candidates[0].delta_status,
      "candidate-mind-delta-observed"
    );
    assert!(!report.mind_delta_candidates[0].whole_brain_snapshot_claimed);
    assert!(!report.mind_delta_candidates[0].xml_parse_success_is_proof);
    assert!(!report.mind_delta_candidates[0].p_puck_green_is_proof);
    assert_eq!(report.affected_mind_slices[0].changed_axis, "attach");
    assert_eq!(
      report.affected_mind_slices[0].proof_reuse_boundary,
      "rejudge-receipt-required-before-proof-reuse"
    );
    assert_eq!(report.semantic_dependency_edge_total, 1);
    assert_eq!(report.rejudge_receipt_candidate_total, 1);
    assert_eq!(report.targeted_replay_plan_total, 1);
    assert_eq!(report.proof_selection_candidate_total, 1);
    assert_eq!(report.incremental_self_compile_candidate_total, 1);
    assert_snapshot_candidates(&report);
    assert_diagram_candidates(&report);
    assert_eq!(
      report.rejudge_receipt_candidates[0].artifact_family,
      "ankh.rejudge-receipt-candidate"
    );
    assert_eq!(
      report.rejudge_receipt_candidates[0].proof_reuse_boundary,
      "fresh-rejudge-receipt-required-before-proof-reuse"
    );
    assert!(!report.rejudge_receipt_candidates[0].old_green_is_current_proof);
    assert_eq!(
      report.targeted_replay_plans[0].artifact_family,
      "ankh.targeted-replay-plan"
    );
    assert_eq!(
      report.targeted_replay_plans[0].replay_execution_status,
      "candidate-only-no-run"
    );
    assert!(!report.targeted_replay_plans[0].stale_latest_report_is_proof);
    assert_eq!(
      report.proof_selection_candidates[0].artifact_family,
      "ankh.proof-selection-candidate"
    );
    assert_eq!(
      report.proof_selection_candidates[0].proof_selection_status_boundary,
      "candidate-only-no-proof-execution"
    );
    assert!(!report.proof_selection_candidates[0].old_green_is_current_proof);
    assert_eq!(
      report.incremental_self_compile_candidates[0].artifact_family,
      "ankh.incremental-self-compile-candidate"
    );
    assert_eq!(
      report.incremental_self_compile_candidates[0].compile_execution_status,
      "candidate-only-no-compile-run"
    );
    assert_eq!(
      report.incremental_self_compile_candidates[0].compiler_owner,
      "pnixc-meta"
    );
    assert!(!report.incremental_self_compile_candidates[0].compile_reuse_allowed);
    assert!(!report.incremental_self_compile_candidates[0].old_green_is_current_proof);
    assert_eq!(
      report.policy_candidates[0].source_family,
      "observation-atom"
    );
    assert_eq!(
      report.policy_candidates[0].signal_class,
      "held-reason-signal"
    );
    assert_eq!(report.policy_candidates[0].proposed_change_axis, "attach");
    assert!(report.policy_candidates[0]
      .evidence_refs
      .iter()
      .any(|value| value == "object:brain-ankh-self-upgrade-loop"));
    assert_eq!(
      report.research_intents[0].artifact_family,
      "ankh.research-intent"
    );
    assert_eq!(
      report.research_intents[0].source_candidate_ref,
      report.policy_candidates[0].candidate_ref
    );
    assert!(!report.research_intents[0].direct_truth_source);
    assert_eq!(
      report.research_intents[0].intent_status,
      "bounded-intent-only-no-fetch"
    );
    assert_eq!(
      report.source_candidates[0].artifact_family,
      "ankh.source-candidate"
    );
    assert_eq!(
      report.source_candidates[0].research_intent_ref,
      report.research_intents[0].intent_ref
    );
    assert!(!report.source_candidates[0].direct_truth_source);
    assert_eq!(
      report.source_candidates[0].status,
      "candidate-only-no-fetch"
    );
    assert_eq!(
      report.source_candidates[0].content_hash,
      "pending-fetch-receipt"
    );
    assert!(report.source_candidates[0]
      .next_required_artifacts
      .iter()
      .any(|value| value == "ankh.evidence-bridge"));
  }

  #[test]
  fn brain_ankh_policy_projection_maps_coding_memory_without_writes() {
    let _guard = PX_EVAL_TEST_LOCK.lock().expect("px eval test lock");
    let coding_input = BrainAnkhCodingMemoryInput {
      source_ref: "coding.verify.1".to_string(),
      source_family: "coding.verify-receipt".to_string(),
      source_surface: "pnix coding-agent".to_string(),
      stored_at_ms: 1,
      repo_snapshot_ref: Some("coding.repo-snapshot::abc".to_string()),
      target_paths: vec!["crates/pnix-executor-graph/src/cli.rs".to_string()],
      command_refs: vec!["cargo test -p pnix-executor-graph".to_string()],
      related_refs: vec!["diff:abc".to_string()],
      status: Some("failed".to_string()),
      subject: Some("verify failed".to_string()),
      evidence_refs: vec!["proof:verify".to_string()],
    };

    let report = build_brain_ankh_policy_projection_report(
      "2026-04-29T00:00:00Z".to_string(),
      "/tmp/gate".to_string(),
      Some("/tmp/doghouse.redb".to_string()),
      20,
      Vec::new(),
      vec![coding_input],
    )
    .expect("brain ankh policy projection report");

    assert_eq!(report.input_totals.coding_memory_total, 1);
    assert_eq!(report.policy_candidate_total, 1);
    assert_eq!(report.attach_decision_total, 0);
    assert_eq!(report.priority_decision_total, 0);
    assert_eq!(report.research_intent_total, 0);
    assert_eq!(report.source_candidate_total, 0);
    assert_eq!(report.mind_delta_candidate_total, 1);
    assert_eq!(report.affected_mind_slice_total, 1);
    assert_eq!(
      report.mind_delta_candidates[0].delta_status,
      "candidate-mind-delta-observed"
    );
    assert_eq!(
      report.mind_delta_candidates[0].delta_boundary,
      "event-judgement-provenance-required-before-mind-delta-close"
    );
    assert!(!report.mind_delta_candidates[0].store_mutation);
    assert!(!report.mind_delta_candidates[0].policy_mutation_applied);
    assert_eq!(report.affected_mind_slices[0].changed_axis, "route");
    assert_eq!(report.semantic_dependency_edge_total, 1);
    assert_eq!(report.rejudge_receipt_candidate_total, 1);
    assert_eq!(report.targeted_replay_plan_total, 1);
    assert_eq!(report.proof_selection_candidate_total, 1);
    assert_eq!(report.incremental_self_compile_candidate_total, 1);
    assert_snapshot_candidates(&report);
    assert_diagram_candidates(&report);
    assert_eq!(
      report.policy_candidates[0].artifact_family,
      "ankh.policy-candidate"
    );
    assert_eq!(
      report.policy_candidates[0].source_family,
      "coding.verify-receipt"
    );
    assert_eq!(report.policy_candidates[0].proposed_change_axis, "route");
    assert_eq!(report.policy_revision_receipt_total, 1);
    assert_eq!(
      report.policy_revision_receipts[0].revision_status,
      "rejected"
    );
    assert!(!report.policy_revision_receipts[0].policy_mutation_applied);
  }
}
