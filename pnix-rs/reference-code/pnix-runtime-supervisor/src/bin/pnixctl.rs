use anyhow::{Context, Result};
use clap::{ArgAction, Parser, Subcommand};
use pnix_runtime_supervisor::client::{BatchCall, SupervisorClient};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Parser)]
#[command(name = "pnixctl")]
#[command(about = "pnix supervisor operator CLI")]
struct Args {
  /// Supervisor endpoint (uds:/path.sock or tls://host:port)
  #[arg(long)]
  endpoint: Option<String>,
  /// Supervisor auth token (overrides PNIX_SUPERVISOR_TOKEN)
  #[arg(long)]
  token: Option<String>,
  #[command(subcommand)]
  cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Run connectivity and capability checks
  Doctor {
    /// Namespace for query checks
    #[arg(long, default_value = "default")]
    ns: String,
  },
  /// Query current process state snapshot
  Status {
    /// Namespace for the query
    #[arg(long, default_value = "default")]
    ns: String,
    /// Maximum result rows
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Apply desired patch JSON through supervisor
  DesiredApply {
    /// Desired patch JSON file path
    #[arg(long)]
    file: PathBuf,
    /// Optional namespace override
    #[arg(long)]
    ns: Option<String>,
  },
  /// Run lightweight latency benchmark against supervisor
  Bench {
    /// Namespace used for query payload
    #[arg(long, default_value = "default")]
    ns: String,
    /// Total call count
    #[arg(long, default_value_t = 1000)]
    n: usize,
    /// Batch size (1 = non-batch path)
    #[arg(long, default_value_t = 1)]
    batch: usize,
  },
  /// Query audit mirror/log events with namespace filtering
  Audit {
    /// Namespace for the query
    #[arg(long, default_value = "default")]
    ns: String,
    /// Maximum result rows
    #[arg(long, default_value_t = 200)]
    limit: usize,
    /// Optional decision filter (allow|deny)
    #[arg(long)]
    decision: Option<String>,
    /// Optional status filter (ok|error)
    #[arg(long)]
    status: Option<String>,
    /// Optional operation prefix filter
    #[arg(long)]
    op_prefix: Option<String>,
    /// Optional invocation id filter
    #[arg(long)]
    invocation_id: Option<String>,
  },
  /// Upsert gateway policy object from JSON file (auth/access/ratelimit/waf/header)
  GatewayPolicyUpsert {
    /// Policy kind: auth-provider|auth-policy|access-policy|ratelimit-policy|waf-policy|header-policy
    #[arg(long)]
    kind: String,
    /// Optional namespace override (otherwise payload ns or token default namespace)
    #[arg(long)]
    ns: Option<String>,
    /// JSON payload file for `<kind>.create` operation
    #[arg(long)]
    file: PathBuf,
  },
  /// Get gateway policy by id
  GatewayPolicyGet {
    /// Policy kind: auth-provider|auth-policy|access-policy|ratelimit-policy|waf-policy|header-policy
    #[arg(long)]
    kind: String,
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    id: String,
  },
  /// List gateway policies
  GatewayPolicyList {
    /// Policy kind: auth-provider|auth-policy|access-policy|ratelimit-policy|waf-policy|header-policy
    #[arg(long)]
    kind: String,
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Disable gateway policy by id
  GatewayPolicyDisable {
    /// Policy kind: auth-provider|auth-policy|access-policy|ratelimit-policy|waf-policy|header-policy
    #[arg(long)]
    kind: String,
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    id: String,
  },
  /// Attach gateway policies to a route
  RoutePolicyAttach {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    route_id: String,
    #[arg(long)]
    auth_policy_id: Option<String>,
    #[arg(long)]
    access_policy_id: Option<String>,
    #[arg(long)]
    rlp_id: Option<String>,
    #[arg(long)]
    waf_id: Option<String>,
    #[arg(long)]
    hp_id: Option<String>,
  },
  /// Get route policy attachment
  RoutePolicyGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    route_id: String,
  },
  /// Read gateway decision rollup stats
  GatewayDecisionStats {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    route_id: Option<String>,
    #[arg(long)]
    code: Option<String>,
    #[arg(long)]
    since_ms: Option<u128>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Register trace artifact metadata for an invocation
  TraceRegister {
    /// Namespace for the trace
    #[arg(long, default_value = "default")]
    ns: String,
    /// Invocation id
    #[arg(long)]
    invocation_id: String,
    /// Absolute trace file path
    #[arg(long)]
    path: PathBuf,
    /// Trace mode (minimal|full)
    #[arg(long, default_value = "full")]
    trace_mode: String,
    /// Compression marker (e.g. zstd)
    #[arg(long)]
    compressed: Option<String>,
    /// Optional known trace size
    #[arg(long)]
    size_bytes: Option<u64>,
  },
  /// Get trace summary by invocation id
  TraceSummary {
    /// Namespace for the trace
    #[arg(long, default_value = "default")]
    ns: String,
    /// Invocation id
    #[arg(long)]
    invocation_id: String,
  },
  /// Tail trace lines by invocation id
  TraceTail {
    /// Namespace for the trace
    #[arg(long, default_value = "default")]
    ns: String,
    /// Invocation id
    #[arg(long)]
    invocation_id: String,
    /// Tail line count
    #[arg(long, default_value_t = 200)]
    n: usize,
    /// Maximum returned bytes
    #[arg(long, default_value_t = 262_144)]
    max_bytes: usize,
  },
  /// Create forensic bundle for invocation/session
  BundleCreate {
    /// Namespace for bundle generation
    #[arg(long, default_value = "default")]
    ns: String,
    /// Invocation id target
    #[arg(long)]
    invocation_id: Option<String>,
    /// Session id target (alternative to invocation id)
    #[arg(long)]
    session_id: Option<String>,
    /// Bundle mode (safe|full)
    #[arg(long, default_value = "safe")]
    mode: String,
    /// Trace include mode (none|summary|full)
    #[arg(long, default_value = "summary")]
    include_trace: String,
    /// Include process log tail snapshots
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    include_logs: bool,
    /// Max bytes used for log/trace tail payloads
    #[arg(long, default_value_t = 262_144)]
    logs_tail_bytes: u64,
    /// Wait for completion and print final job state/result
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    wait: bool,
    /// Wait timeout in milliseconds
    #[arg(long, default_value_t = 120_000)]
    timeout_ms: u64,
  },
  /// Get bundle metadata by id
  BundleGet {
    /// Namespace for bundle lookup
    #[arg(long, default_value = "default")]
    ns: String,
    /// Bundle id
    #[arg(long)]
    bundle_id: String,
  },
  /// List bundles in namespace
  BundleList {
    /// Namespace for listing
    #[arg(long, default_value = "default")]
    ns: String,
    /// Optional invocation id filter
    #[arg(long)]
    invocation_id: Option<String>,
    /// Optional session id filter
    #[arg(long)]
    session_id: Option<String>,
    /// Maximum rows
    #[arg(long, default_value_t = 100)]
    limit: usize,
  },
  /// Upsert artifact store registry entry
  ArtifactStoreUpsert {
    #[arg(long)]
    store_id: String,
    #[arg(long)]
    endpoint: String,
    #[arg(long, default_value = "edge")]
    kind: String,
    #[arg(long)]
    zone: Option<String>,
    #[arg(long)]
    read_weight: Option<u32>,
    #[arg(long)]
    write_weight: Option<u32>,
    #[arg(long)]
    supports_range: Option<bool>,
    #[arg(long)]
    supports_put: Option<bool>,
    #[arg(long)]
    egress_weight: Option<u32>,
    #[arg(long)]
    latency_weight: Option<u32>,
  },
  /// Upsert artifact replica metadata
  ArtifactReplicaUpsert {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    sha256: String,
    #[arg(long)]
    store_id: String,
    #[arg(long, default_value = "present")]
    state: String,
    #[arg(long)]
    size_bytes: Option<u64>,
    #[arg(long)]
    verified_ms: Option<u128>,
  },
  /// Locate best artifact candidates for a blob
  ArtifactLocate {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    sha256: String,
    #[arg(long)]
    prefer_zone: Option<String>,
    #[arg(long)]
    need_range: bool,
  },
  /// Print artifact registry/replica stats
  ArtifactStats {
    #[arg(long, default_value = "default")]
    ns: String,
  },
  /// Request/track PKI certificate issuance metadata
  PkiCertRequest {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    cert_id: Option<String>,
    #[arg(long)]
    kind: String,
    #[arg(long)]
    subject: String,
    /// JSON array string (e.g. ["api.example.com"])
    #[arg(long)]
    san_dns_json: Option<String>,
    /// JSON array string (e.g. ["spiffe://pnix/ns/projA/service/myservice"])
    #[arg(long)]
    san_uri_json: Option<String>,
    #[arg(long)]
    issuer: Option<String>,
    #[arg(long)]
    issuer_ref: Option<String>,
    #[arg(long)]
    key_secret_ref: Option<String>,
    #[arg(long)]
    cert_ref_id: Option<String>,
    #[arg(long)]
    ttl_ms: Option<u64>,
  },
  /// Get certificate row by id
  PkiCertGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    cert_id: String,
  },
  /// List certificate rows
  PkiCertList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    kind: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Revoke certificate row by id
  PkiCertRevoke {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    cert_id: String,
    #[arg(long)]
    reason: Option<String>,
  },
  /// Add or update GitOps source
  GitopsSourceUpsert {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    source_id: Option<String>,
    #[arg(long)]
    name: String,
    #[arg(long)]
    connector_id: String,
    #[arg(long)]
    repo_url: String,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long)]
    subdir: Option<String>,
    #[arg(long)]
    mode: Option<String>,
    /// JSON object string for source policy
    #[arg(long)]
    policy_json: Option<String>,
    #[arg(long)]
    status: Option<String>,
  },
  /// Get GitOps source by id
  GitopsSourceGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    source_id: String,
  },
  /// List GitOps sources
  GitopsSourceList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    mode: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Disable GitOps source by id
  GitopsSourceDisable {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    source_id: String,
  },
  /// Trigger one GitOps sync cycle
  GitopsSyncNow {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    source_id: String,
    #[arg(long)]
    commit_sha: Option<String>,
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    apply: bool,
  },
  /// Read GitOps sync status by source
  GitopsStatus {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    source_id: String,
  },
  /// Evaluate admission policy with explainable decision output
  AdmissionCheck {
    /// JSON file containing admission input payload
    #[arg(long)]
    file: PathBuf,
  },
  /// List admission policy evaluation logs
  PolicyEvalList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    channel: Option<String>,
    #[arg(long)]
    target: Option<String>,
    #[arg(long)]
    decision: Option<String>,
    #[arg(long)]
    since_ms: Option<u128>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Create tenant
  TenantCreate {
    #[arg(long)]
    name: String,
    #[arg(long)]
    tenant_id: Option<String>,
    /// JSON object string for tenant owner metadata
    #[arg(long)]
    owner_json: Option<String>,
    #[arg(long)]
    status: Option<String>,
  },
  /// Get tenant by id
  TenantGet {
    #[arg(long)]
    tenant_id: String,
  },
  /// List tenants
  TenantList {
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Update tenant fields
  TenantUpdate {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    name: Option<String>,
    /// JSON object string for tenant owner metadata
    #[arg(long)]
    owner_json: Option<String>,
    #[arg(long)]
    status: Option<String>,
  },
  /// Suspend tenant
  TenantSuspend {
    #[arg(long)]
    tenant_id: String,
  },
  /// Attach namespace to tenant
  TenantAttachNamespace {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    ns: String,
  },
  /// Detach namespace from tenant
  TenantDetachNamespace {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    ns: String,
  },
  /// List namespaces attached to tenant
  TenantNamespacesList {
    #[arg(long)]
    tenant_id: String,
    #[arg(long, default_value_t = 500)]
    limit: usize,
  },
  /// Set tenant quota policy
  QuotaSet {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    policy_id: Option<String>,
    /// JSON object string
    #[arg(long)]
    scope_json: Option<String>,
    /// JSON object string
    #[arg(long)]
    limits_json: Option<String>,
  },
  /// Get tenant quota policy
  QuotaGet {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    policy_id: Option<String>,
  },
  /// Set tenant budget policy
  BudgetSet {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    policy_id: Option<String>,
    #[arg(long)]
    period: Option<String>,
    #[arg(long)]
    currency: Option<String>,
    #[arg(long)]
    budget_minor: i64,
    /// JSON object string
    #[arg(long)]
    thresholds_json: Option<String>,
    /// JSON object string
    #[arg(long)]
    actions_json: Option<String>,
  },
  /// Get tenant budget policy
  BudgetGet {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    policy_id: Option<String>,
  },
  /// Upsert budget evaluation state row
  BudgetStateUpsert {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    policy_id: Option<String>,
    #[arg(long)]
    period_key: String,
    #[arg(long)]
    spent_minor: i64,
    #[arg(long)]
    budget_minor: i64,
    #[arg(long)]
    level: Option<String>,
    #[arg(long)]
    last_enforcement_change_id: Option<String>,
  },
  /// Get budget evaluation state row
  BudgetStateGet {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    policy_id: Option<String>,
    #[arg(long)]
    period_key: String,
  },
  /// Upsert usage rollup row
  UsageRollupUpsert {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    ns: String,
    #[arg(long)]
    ts_ms: u128,
    #[arg(long)]
    region: Option<String>,
    #[arg(long)]
    cpu_millis_avg: u64,
    #[arg(long)]
    cpu_millis_hour: u64,
    #[arg(long)]
    mem_bytes_avg: u64,
    #[arg(long)]
    mem_bytes_hour: u64,
    #[arg(long)]
    proc_count_avg: u64,
    #[arg(long)]
    artifact_bytes_avg: Option<u64>,
    #[arg(long)]
    egress_bytes: Option<u64>,
    #[arg(long)]
    gateway_req_count: Option<u64>,
    #[arg(long)]
    gateway_err5xx_count: Option<u64>,
  },
  /// Query usage rollup rows
  UsageQuery {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    ns: Option<String>,
    #[arg(long)]
    from_ts_ms: Option<u128>,
    #[arg(long)]
    to_ts_ms: Option<u128>,
    #[arg(long)]
    region: Option<String>,
    #[arg(long, default_value_t = 500)]
    limit: usize,
  },
  /// Upsert cost rollup row
  CostRollupUpsert {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    ts_ms: u128,
    #[arg(long)]
    currency: Option<String>,
    #[arg(long)]
    cost_minor: i64,
    #[arg(long)]
    rate_card_id: String,
    /// JSON object string
    #[arg(long)]
    breakdown_json: Option<String>,
  },
  /// Query cost rollup rows
  CostQuery {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    from_ts_ms: Option<u128>,
    #[arg(long)]
    to_ts_ms: Option<u128>,
    #[arg(long)]
    rate_card_id: Option<String>,
    #[arg(long, default_value_t = 500)]
    limit: usize,
  },
  /// Create or update a rate card
  RateCardCreate {
    #[arg(long)]
    name: String,
    #[arg(long)]
    card_id: Option<String>,
    #[arg(long)]
    currency: Option<String>,
    /// JSON object string
    #[arg(long)]
    rates_json: Option<String>,
    #[arg(long)]
    valid_from_ms: u128,
    #[arg(long)]
    valid_to_ms: Option<u128>,
    #[arg(long)]
    status: Option<String>,
  },
  /// Activate a rate card by id
  RateCardActivate {
    #[arg(long)]
    card_id: String,
  },
  /// Get rate card by id
  RateCardGet {
    #[arg(long)]
    card_id: String,
  },
  /// List rate cards
  RateCardList {
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    currency: Option<String>,
    #[arg(long, default_value_t = 500)]
    limit: usize,
  },
  /// Generate invoice from cost rollups
  InvoiceGenerate {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    period: String,
    #[arg(long)]
    currency: Option<String>,
    #[arg(long)]
    from_ts_ms: Option<u128>,
    #[arg(long)]
    to_ts_ms: Option<u128>,
    #[arg(long)]
    rate_card_id: Option<String>,
    #[arg(long)]
    invoice_id: Option<String>,
    #[arg(long)]
    report_ref_id: Option<String>,
  },
  /// Get invoice by id
  InvoiceGet {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    invoice_id: String,
  },
  /// List invoices for tenant
  InvoiceList {
    #[arg(long)]
    tenant_id: String,
    #[arg(long)]
    period: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long, default_value_t = 500)]
    limit: usize,
  },
  /// Create or update forecast profile
  ForecastProfileSet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    profile_id: Option<String>,
    #[arg(long)]
    scope_kind: String,
    #[arg(long)]
    scope_id: String,
    #[arg(long)]
    channel: Option<String>,
    #[arg(long)]
    model_kind: Option<String>,
    #[arg(long)]
    horizon_ms: Option<u64>,
    #[arg(long)]
    step_ms: Option<u64>,
    /// JSON object string
    #[arg(long)]
    features_json: Option<String>,
    /// JSON object string
    #[arg(long)]
    guardrails_json: Option<String>,
  },
  /// Get forecast profile by id
  ForecastProfileGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    profile_id: String,
  },
  /// List forecast profiles
  ForecastProfileList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    scope_kind: Option<String>,
    #[arg(long)]
    scope_id: Option<String>,
    #[arg(long)]
    channel: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Get latest forecast run for a scope
  ForecastRunLatest {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    scope_kind: String,
    #[arg(long)]
    scope_id: String,
    #[arg(long)]
    channel: Option<String>,
  },
  /// Trigger predictive run now (forecast + plan proposal)
  PredictiveRunNow {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    scope_kind: String,
    #[arg(long)]
    scope_id: String,
    #[arg(long)]
    channel: Option<String>,
    #[arg(long)]
    profile_id: Option<String>,
  },
  /// Get predictive action plan by id
  PredictivePlanGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    plan_id: String,
  },
  /// List predictive action plans
  PredictivePlanList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    scope_kind: Option<String>,
    #[arg(long)]
    scope_id: Option<String>,
    #[arg(long)]
    channel: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Run what-if simulation for a source change
  SimulationRun {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    source_kind: String,
    #[arg(long)]
    source_id: String,
    /// JSON object string
    #[arg(long)]
    base_ref_json: Option<String>,
    /// JSON object string
    #[arg(long)]
    proposed_ref_json: Option<String>,
    #[arg(long)]
    env_snapshot_ref_id: Option<String>,
  },
  /// Get latest simulation result by source
  SimulationGetLatest {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    source_kind: String,
    #[arg(long)]
    source_id: String,
  },
  /// Create or update alert rule
  AlertRuleSet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    rule_id: Option<String>,
    #[arg(long)]
    name: String,
    #[arg(long)]
    source_kind: String,
    /// JSON object string
    #[arg(long)]
    selector_json: Option<String>,
    #[arg(long)]
    window_ms: Option<u64>,
    #[arg(long)]
    eval_every_ms: Option<u64>,
    /// JSON object string
    #[arg(long)]
    condition_json: Option<String>,
    #[arg(long)]
    severity: Option<String>,
    #[arg(long)]
    dedupe_key_template: Option<String>,
    /// JSON object string
    #[arg(long)]
    labels_json: Option<String>,
    /// JSON object string
    #[arg(long)]
    autopilot_json: Option<String>,
    #[arg(long)]
    status: Option<String>,
  },
  /// Get alert rule by id
  AlertRuleGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    rule_id: String,
  },
  /// List alert rules
  AlertRuleList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    source_kind: Option<String>,
    #[arg(long)]
    severity: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Disable alert rule by id
  AlertRuleDisable {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    rule_id: String,
  },
  /// Evaluate alert rule once
  AlertEvaluate {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    rule_id: String,
    #[arg(long)]
    dedupe_key: Option<String>,
    /// JSON object string
    #[arg(long)]
    value_json: Option<String>,
  },
  /// Get incident by id
  IncidentGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    incident_id: String,
  },
  /// List incidents
  IncidentList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    severity: Option<String>,
    #[arg(long)]
    primary_object_kind: Option<String>,
    #[arg(long)]
    primary_object_id: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Acknowledge incident
  IncidentAck {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    incident_id: String,
    /// JSON object string
    #[arg(long)]
    owner_json: Option<String>,
  },
  /// Append incident note
  IncidentNote {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    incident_id: String,
    #[arg(long)]
    message: String,
    /// JSON object string
    #[arg(long)]
    meta_json: Option<String>,
  },
  /// Resolve incident
  IncidentResolve {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    incident_id: String,
    #[arg(long)]
    summary: Option<String>,
    #[arg(long)]
    root_cause: Option<String>,
  },
  /// Create or update runbook
  RunbookSet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    runbook_id: Option<String>,
    #[arg(long)]
    name: String,
    #[arg(long)]
    version: Option<String>,
    #[arg(long)]
    bundle_ref_id: Option<String>,
    #[arg(long)]
    sha256: Option<String>,
    /// JSON object string
    #[arg(long)]
    policy_json: Option<String>,
    /// JSON array string
    #[arg(long)]
    steps_json: Option<String>,
    #[arg(long)]
    status: Option<String>,
  },
  /// Get runbook by name
  RunbookGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    name: String,
  },
  /// List runbooks
  RunbookList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Execute runbook for incident
  RunbookExecute {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    incident_id: String,
    #[arg(long)]
    runbook_name: String,
    #[arg(long)]
    mode: Option<String>,
    #[arg(long)]
    auto_run: Option<bool>,
  },
  /// Generate postmortem draft
  PostmortemGenerate {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    incident_id: String,
    #[arg(long)]
    pm_id: Option<String>,
    #[arg(long)]
    report_ref_id: Option<String>,
  },
  /// Get postmortem by incident id
  PostmortemGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    incident_id: String,
  },
  /// Publish chaos experiment
  ChaosExperimentPublish {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    exp_id: Option<String>,
    #[arg(long)]
    name: String,
    #[arg(long)]
    bundle_ref_id: String,
    #[arg(long)]
    sha256: String,
    /// JSON object string
    #[arg(long)]
    spec_json: Option<String>,
    #[arg(long)]
    status: Option<String>,
  },
  /// Get chaos experiment by id
  ChaosExperimentGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    exp_id: String,
  },
  /// List chaos experiments
  ChaosExperimentList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Approve chaos experiment
  ChaosExperimentApprove {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    exp_id: String,
  },
  /// Disable chaos experiment
  ChaosExperimentDisable {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    exp_id: String,
  },
  /// Start chaos run
  ChaosRunStart {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    exp_id: String,
  },
  /// Abort chaos run
  ChaosRunAbort {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    run_id: String,
  },
  /// Get chaos run by id
  ChaosRunGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    run_id: String,
  },
  /// Get latest chaos run for experiment
  ChaosRunLatest {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    exp_id: String,
  },
  /// Add or rotate a keyring public key
  KeyringAdd {
    #[arg(long)]
    key_id: String,
    #[arg(long)]
    public_key_b64: String,
    /// JSON object string for key scope metadata
    #[arg(long)]
    scope_json: Option<String>,
  },
  /// Revoke key by id
  KeyringRevoke {
    #[arg(long)]
    key_id: String,
  },
  /// List keyring public keys
  KeyringList,
  /// Verify and index attestation envelope JSON
  AttestVerify {
    #[arg(long, default_value = "default")]
    ns: String,
    /// JSON file containing DSSE envelope
    #[arg(long)]
    file: PathBuf,
    #[arg(long)]
    att_id: Option<String>,
    #[arg(long)]
    att_ref_id: Option<String>,
    #[arg(long)]
    invocation_id: Option<String>,
    #[arg(long)]
    session_id: Option<String>,
    #[arg(long)]
    release_id: Option<String>,
  },
  /// List attestation index rows
  AttestList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    subject_sha256: Option<String>,
    #[arg(long)]
    attestation_type: Option<String>,
    #[arg(long, default_value_t = 100)]
    limit: usize,
  },
  /// Get compliance policy for namespace/channel
  CompliancePolicyGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long, default_value = "prod")]
    channel: String,
  },
  /// Set compliance policy_json from file for namespace/channel
  CompliancePolicySet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long, default_value = "prod")]
    channel: String,
    /// JSON file containing compliance policy object
    #[arg(long)]
    file: PathBuf,
  },
  /// Create compliance exception (time-boxed override)
  ComplianceExceptionCreate {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long, default_value = "prod")]
    channel: String,
    #[arg(long)]
    target: String,
    #[arg(long)]
    reason: String,
    #[arg(long)]
    subject_id: Option<String>,
    /// JSON object string for scoped exception rule
    #[arg(long)]
    rule_json: Option<String>,
    #[arg(long)]
    duration_ms: Option<u64>,
    #[arg(long)]
    approvals_required: Option<u32>,
  },
  /// Approve compliance exception by id
  ComplianceExceptionApprove {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    exc_id: String,
  },
  /// Revoke compliance exception by id
  ComplianceExceptionRevoke {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    exc_id: String,
  },
  /// List compliance exceptions
  ComplianceExceptionList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    channel: Option<String>,
    #[arg(long)]
    target: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    subject_id: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Run compliance scan for namespace/channel
  ComplianceScan {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long, default_value = "prod")]
    channel: String,
  },
  /// Get latest compliance report for namespace/channel
  ComplianceReportLatest {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long, default_value = "prod")]
    channel: String,
  },
  /// Upsert runtime catalog entry
  RuntimeCatalogUpsert {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    family: String,
    #[arg(long)]
    version: String,
    #[arg(long)]
    exec_ref_id: String,
    #[arg(long)]
    sha256: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    platform_os: Option<String>,
    #[arg(long)]
    platform_arch: Option<String>,
    #[arg(long)]
    sbom_ref_id: Option<String>,
    #[arg(long)]
    vuln_report_id: Option<String>,
    #[arg(long)]
    attestation_ref_id: Option<String>,
    #[arg(long)]
    status: Option<String>,
  },
  /// List runtime catalog entries
  RuntimeCatalogList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    family: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    platform_os: Option<String>,
    #[arg(long)]
    platform_arch: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Set runtime alias (e.g. jvm stable -> 17.0.10)
  RuntimeAliasSet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    family: String,
    #[arg(long)]
    alias: String,
    #[arg(long)]
    version: String,
  },
  /// List runtime aliases
  RuntimeAliasList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    family: Option<String>,
  },
  /// Resolve runtime alias to concrete catalog entry
  RuntimeAliasResolve {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    family: String,
    #[arg(long)]
    alias: String,
    #[arg(long)]
    platform_os: Option<String>,
    #[arg(long)]
    platform_arch: Option<String>,
  },
  /// Compose security verdict for runtime+inputs
  SecurityComposeVerdict {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    runtime_ref_id: String,
    /// Comma-separated input refs or repeated with multiple --inputs flags not supported
    #[arg(long)]
    inputs: String,
    #[arg(long)]
    policy_id: Option<String>,
  },
  /// Get composed security verdict by id
  SecurityVerdictGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    verdict_id: String,
  },
  /// Lookup composed verdicts by runtime/app sha pair
  SecurityVerdictLookup {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    runtime_sha256: String,
    #[arg(long)]
    app_sha256: String,
    #[arg(long, default_value_t = 20)]
    limit: usize,
  },
  /// Publish immutable blueprint bundle metadata
  BlueprintBundlePublish {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    name: String,
    #[arg(long)]
    version: String,
    #[arg(long)]
    bundle_ref_id: String,
    #[arg(long)]
    sha256: String,
    #[arg(long)]
    attestation_ref_id: Option<String>,
    #[arg(long)]
    status: Option<String>,
    /// Optional JSON file containing blueprint manifest payload
    #[arg(long)]
    manifest_file: Option<PathBuf>,
  },
  /// List blueprint bundles
  BlueprintBundleList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Get blueprint bundle by id
  BlueprintBundleGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    bundle_id: String,
  },
  /// Approve blueprint bundle
  BlueprintBundleApprove {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    bundle_id: String,
  },
  /// Revoke blueprint bundle
  BlueprintBundleRevoke {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    bundle_id: String,
  },
  /// Create service instance from blueprint
  ServiceCreate {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    service_name: String,
    #[arg(long, default_value = "dev")]
    channel: String,
    #[arg(long)]
    blueprint_name: String,
    /// JSON object for params
    #[arg(long)]
    params_file: PathBuf,
    /// Optional JSON object for overrides
    #[arg(long)]
    overrides_file: Option<PathBuf>,
  },
  /// Update service params (full replace)
  ServiceUpdateParams {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    service_name: String,
    #[arg(long, default_value = "dev")]
    channel: String,
    /// JSON object for new params
    #[arg(long)]
    params_file: PathBuf,
  },
  /// Get service instance
  ServiceGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    service_name: String,
    #[arg(long, default_value = "dev")]
    channel: String,
  },
  /// List service instances
  ServiceList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    channel: Option<String>,
    #[arg(long)]
    blueprint_name: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Create service release candidate
  ServiceReleaseCreate {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    service_name: String,
    #[arg(long, default_value = "dev")]
    channel: String,
    #[arg(long)]
    blueprint_bundle_id: String,
    #[arg(long)]
    notes: Option<String>,
  },
  /// Approve service release
  ServiceReleaseApprove {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    release_id: String,
  },
  /// Activate service release
  ServiceReleaseActivate {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    release_id: String,
  },
  /// Roll back from a release to previous one
  ServiceReleaseRollback {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    release_id: String,
  },
  /// Render service desired preview (validate/dry-run)
  ServiceRenderPreview {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    service_name: String,
    #[arg(long, default_value = "dev")]
    channel: String,
    #[arg(long)]
    blueprint_bundle_id: Option<String>,
    #[arg(long)]
    params_file: Option<PathBuf>,
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    validate_only: bool,
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    dry_run: bool,
  },
  /// Get latest rendered desired cache for a service
  ServiceRenderGetLatest {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    service_name: String,
    #[arg(long, default_value = "dev")]
    channel: String,
  },
  /// Register or refresh a fleet node
  NodeHello {
    #[arg(long)]
    node_id: String,
    #[arg(long)]
    zone: Option<String>,
    #[arg(long)]
    region: Option<String>,
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    endpoint: Option<String>,
    /// JSON object string for node labels
    #[arg(long)]
    labels_json: Option<String>,
    /// Comma-separated capability names
    #[arg(long)]
    caps: Option<String>,
    /// JSON object string for capacity snapshot
    #[arg(long)]
    capacity_json: Option<String>,
    #[arg(long)]
    lease_ms: Option<u64>,
  },
  /// Heartbeat from a fleet node
  NodeHeartbeat {
    #[arg(long)]
    node_id: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    zone: Option<String>,
    #[arg(long)]
    region: Option<String>,
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    endpoint: Option<String>,
    /// JSON object string for node labels
    #[arg(long)]
    labels_json: Option<String>,
    /// Comma-separated capability names
    #[arg(long)]
    caps: Option<String>,
    /// JSON object string for capacity snapshot
    #[arg(long)]
    capacity_json: Option<String>,
    #[arg(long)]
    lease_ms: Option<u64>,
  },
  /// Set node status (ready|not_ready|draining)
  NodeSetStatus {
    #[arg(long)]
    node_id: String,
    #[arg(long)]
    status: String,
  },
  /// List fleet nodes
  NodeList {
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    zone: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
  },
  /// Upsert replica assignment
  ReplicaAssignmentUpsert {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    rs_id: String,
    #[arg(long)]
    replica_ordinal: u32,
    #[arg(long)]
    process_id: String,
    #[arg(long)]
    assigned_node: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    service_name: Option<String>,
    #[arg(long)]
    channel: Option<String>,
    #[arg(long)]
    release_id: Option<String>,
  },
  /// List replica assignments
  ReplicaAssignmentList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    rs_id: Option<String>,
    #[arg(long)]
    assigned_node: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    service_name: Option<String>,
    #[arg(long)]
    channel: Option<String>,
    #[arg(long, default_value_t = 500)]
    limit: usize,
  },
  /// Upsert a service endpoint entry
  ServiceEndpointUpsert {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    service_name: String,
    #[arg(long, default_value = "dev")]
    channel: String,
    #[arg(long)]
    process_id: String,
    #[arg(long)]
    node_id: String,
    #[arg(long)]
    host: String,
    #[arg(long)]
    port: u16,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    release_id: Option<String>,
    #[arg(long)]
    rs_id: Option<String>,
  },
  /// Get service endpoints for a service/channel
  ServiceEndpointsGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    service_name: String,
    #[arg(long, default_value = "dev")]
    channel: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long, default_value_t = 500)]
    limit: usize,
  },
  /// Create remediation plan (from file or inline options)
  RemediationPlanCreate {
    #[arg(long, default_value = "default")]
    ns: String,
    /// Optional JSON payload file for remediation.plan.create
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long, default_value = "medium")]
    risk: String,
    #[arg(long)]
    summary: Option<String>,
    /// JSON object string for trigger payload
    #[arg(long)]
    trigger_json: Option<String>,
    /// JSON array string for steps
    #[arg(long)]
    steps_json: Option<String>,
    #[arg(long)]
    invocation_id: Option<String>,
    #[arg(long)]
    session_id: Option<String>,
  },
  /// Get remediation plan by id
  RemediationPlanGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    plan_id: String,
  },
  /// List remediation plans
  RemediationPlanList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    risk: Option<String>,
    #[arg(long, default_value_t = 100)]
    limit: usize,
  },
  /// Escalate remediation plan to change request
  RemediationPlanEscalate {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    plan_id: String,
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    reason: Option<String>,
    #[arg(long)]
    source: Option<String>,
    #[arg(long)]
    risk: Option<String>,
    #[arg(long, default_value_t = false)]
    auto_run: bool,
  },
  /// Get change request by id
  ChangeGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    change_id: String,
  },
  /// List change requests
  ChangeList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    risk: Option<String>,
    #[arg(long, default_value_t = 100)]
    limit: usize,
  },
  /// Approve change request
  ChangeApprove {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    change_id: String,
    #[arg(long)]
    note: Option<String>,
  },
  /// Reject change request
  ChangeReject {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    change_id: String,
    #[arg(long)]
    note: Option<String>,
  },
  /// Run change request
  ChangeRun {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    change_id: String,
  },
  /// Cancel change request
  ChangeCancel {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    change_id: String,
  },
  /// Submit an arbitrary job
  JobSubmit {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    kind: String,
    /// JSON object payload for job params
    #[arg(long)]
    params: Option<String>,
    #[arg(long)]
    dedupe_key: Option<String>,
    #[arg(long, default_value_t = 100)]
    priority: u32,
    #[arg(long)]
    invocation_id: Option<String>,
    #[arg(long)]
    session_id: Option<String>,
    #[arg(long, default_value_t = 5)]
    max_attempt: u32,
  },
  /// Get job details
  JobGet {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    job_id: String,
  },
  /// List jobs
  JobList {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    state: Option<String>,
    #[arg(long)]
    kind_prefix: Option<String>,
    #[arg(long)]
    invocation_id: Option<String>,
    #[arg(long)]
    session_id: Option<String>,
    #[arg(long, default_value_t = 100)]
    limit: usize,
  },
  /// Cancel a queued/running job
  JobCancel {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    job_id: String,
  },
  /// Tail job logs
  JobLogs {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    job_id: String,
    #[arg(long, default_value_t = 200)]
    n: usize,
    #[arg(long, default_value_t = 262_144)]
    max_bytes: usize,
  },
  /// Wait for job terminal state
  JobWait {
    #[arg(long, default_value = "default")]
    ns: String,
    #[arg(long)]
    job_id: String,
    #[arg(long, default_value_t = 120_000)]
    timeout_ms: u64,
  },
  /// Install systemd unit and baseline config templates
  InstallSystemd {
    /// Target root (use / for real install, or a temp dir for staging)
    #[arg(long, default_value = "/")]
    root: PathBuf,
    /// Source repository root containing packaging/ and config/templates/
    #[arg(long)]
    source_root: Option<PathBuf>,
    /// Print operations without writing files
    #[arg(long, default_value_t = false)]
    dry_run: bool,
  },
}

fn resolve_endpoint(cli_endpoint: Option<String>) -> String {
  if let Some(endpoint) = cli_endpoint {
    return endpoint;
  }
  if let Ok(endpoint) = std::env::var("PNIX_SUPERVISOR_ENDPOINT") {
    if !endpoint.trim().is_empty() {
      return endpoint;
    }
  }
  if let Ok(sock) = std::env::var("PNIX_SUPERVISOR_SOCK") {
    if !sock.trim().is_empty() {
      return format!("uds:{}", sock);
    }
  }
  "uds:/tmp/pnix-supervisor.sock".to_string()
}

fn set_token_env(token: Option<String>) {
  if let Some(token) = token {
    // SAFETY: setting process env is required to pass auth token into SupervisorClient.
    unsafe {
      std::env::set_var("PNIX_SUPERVISOR_TOKEN", token);
    }
  }
}

fn main() -> Result<()> {
  let args = Args::parse();
  set_token_env(args.token);
  let endpoint = resolve_endpoint(args.endpoint);
  let client = SupervisorClient::connect(endpoint.clone())?.with_timeout(Duration::from_secs(3));

  match args.cmd {
    Command::Doctor { ns } => cmd_doctor(&client, endpoint.as_str(), ns.as_str()),
    Command::Status { ns, limit } => cmd_status(&client, ns.as_str(), limit),
    Command::DesiredApply { file, ns } => cmd_desired_apply(&client, &file, ns.as_deref()),
    Command::Bench { ns, n, batch } => cmd_bench(&client, ns.as_str(), n, batch),
    Command::Audit {
      ns,
      limit,
      decision,
      status,
      op_prefix,
      invocation_id,
    } => cmd_audit(
      &client,
      ns.as_str(),
      limit,
      decision.as_deref(),
      status.as_deref(),
      op_prefix.as_deref(),
      invocation_id.as_deref(),
    ),
    Command::GatewayPolicyUpsert { kind, ns, file } => {
      cmd_gateway_policy_upsert(&client, kind.as_str(), ns.as_deref(), &file)
    }
    Command::GatewayPolicyGet { kind, ns, id } => {
      cmd_gateway_policy_get(&client, kind.as_str(), ns.as_str(), id.as_str())
    }
    Command::GatewayPolicyList {
      kind,
      ns,
      name,
      status,
      limit,
    } => cmd_gateway_policy_list(
      &client,
      kind.as_str(),
      ns.as_str(),
      name.as_deref(),
      status.as_deref(),
      limit,
    ),
    Command::GatewayPolicyDisable { kind, ns, id } => {
      cmd_gateway_policy_disable(&client, kind.as_str(), ns.as_str(), id.as_str())
    }
    Command::RoutePolicyAttach {
      ns,
      route_id,
      auth_policy_id,
      access_policy_id,
      rlp_id,
      waf_id,
      hp_id,
    } => cmd_route_policy_attach(
      &client,
      ns.as_str(),
      route_id.as_str(),
      auth_policy_id.as_deref(),
      access_policy_id.as_deref(),
      rlp_id.as_deref(),
      waf_id.as_deref(),
      hp_id.as_deref(),
    ),
    Command::RoutePolicyGet { ns, route_id } => {
      cmd_route_policy_get(&client, ns.as_str(), route_id.as_str())
    }
    Command::GatewayDecisionStats {
      ns,
      route_id,
      code,
      since_ms,
      limit,
    } => cmd_gateway_decision_stats(
      &client,
      ns.as_str(),
      route_id.as_deref(),
      code.as_deref(),
      since_ms,
      limit,
    ),
    Command::TraceRegister {
      ns,
      invocation_id,
      path,
      trace_mode,
      compressed,
      size_bytes,
    } => cmd_trace_register(
      &client,
      ns.as_str(),
      invocation_id.as_str(),
      &path,
      trace_mode.as_str(),
      compressed.as_deref(),
      size_bytes,
    ),
    Command::TraceSummary { ns, invocation_id } => {
      cmd_trace_summary(&client, ns.as_str(), invocation_id.as_str())
    }
    Command::TraceTail {
      ns,
      invocation_id,
      n,
      max_bytes,
    } => cmd_trace_tail(&client, ns.as_str(), invocation_id.as_str(), n, max_bytes),
    Command::BundleCreate {
      ns,
      invocation_id,
      session_id,
      mode,
      include_trace,
      include_logs,
      logs_tail_bytes,
      wait,
      timeout_ms,
    } => cmd_bundle_create(
      &client,
      ns.as_str(),
      invocation_id.as_deref(),
      session_id.as_deref(),
      mode.as_str(),
      include_trace.as_str(),
      include_logs,
      logs_tail_bytes,
      wait,
      timeout_ms,
    ),
    Command::BundleGet { ns, bundle_id } => {
      cmd_bundle_get(&client, ns.as_str(), bundle_id.as_str())
    }
    Command::BundleList {
      ns,
      invocation_id,
      session_id,
      limit,
    } => cmd_bundle_list(
      &client,
      ns.as_str(),
      invocation_id.as_deref(),
      session_id.as_deref(),
      limit,
    ),
    Command::ArtifactStoreUpsert {
      store_id,
      endpoint,
      kind,
      zone,
      read_weight,
      write_weight,
      supports_range,
      supports_put,
      egress_weight,
      latency_weight,
    } => cmd_artifact_store_upsert(
      &client,
      store_id.as_str(),
      endpoint.as_str(),
      kind.as_str(),
      zone.as_deref(),
      read_weight,
      write_weight,
      supports_range,
      supports_put,
      egress_weight,
      latency_weight,
    ),
    Command::ArtifactReplicaUpsert {
      ns,
      sha256,
      store_id,
      state,
      size_bytes,
      verified_ms,
    } => cmd_artifact_replica_upsert(
      &client,
      ns.as_str(),
      sha256.as_str(),
      store_id.as_str(),
      state.as_str(),
      size_bytes,
      verified_ms,
    ),
    Command::ArtifactLocate {
      ns,
      sha256,
      prefer_zone,
      need_range,
    } => cmd_artifact_locate(
      &client,
      ns.as_str(),
      sha256.as_str(),
      prefer_zone.as_deref(),
      need_range,
    ),
    Command::ArtifactStats { ns } => cmd_artifact_stats(&client, ns.as_str()),
    Command::PkiCertRequest {
      ns,
      cert_id,
      kind,
      subject,
      san_dns_json,
      san_uri_json,
      issuer,
      issuer_ref,
      key_secret_ref,
      cert_ref_id,
      ttl_ms,
    } => cmd_pki_cert_request(
      &client,
      ns.as_str(),
      cert_id.as_deref(),
      kind.as_str(),
      subject.as_str(),
      san_dns_json.as_deref(),
      san_uri_json.as_deref(),
      issuer.as_deref(),
      issuer_ref.as_deref(),
      key_secret_ref.as_deref(),
      cert_ref_id.as_deref(),
      ttl_ms,
    ),
    Command::PkiCertGet { ns, cert_id } => cmd_pki_cert_get(&client, ns.as_str(), cert_id.as_str()),
    Command::PkiCertList {
      ns,
      status,
      kind,
      limit,
    } => cmd_pki_cert_list(
      &client,
      ns.as_str(),
      status.as_deref(),
      kind.as_deref(),
      limit,
    ),
    Command::PkiCertRevoke {
      ns,
      cert_id,
      reason,
    } => cmd_pki_cert_revoke(&client, ns.as_str(), cert_id.as_str(), reason.as_deref()),
    Command::GitopsSourceUpsert {
      ns,
      source_id,
      name,
      connector_id,
      repo_url,
      branch,
      subdir,
      mode,
      policy_json,
      status,
    } => cmd_gitops_source_upsert(
      &client,
      ns.as_str(),
      source_id.as_deref(),
      name.as_str(),
      connector_id.as_str(),
      repo_url.as_str(),
      branch.as_deref(),
      subdir.as_deref(),
      mode.as_deref(),
      policy_json.as_deref(),
      status.as_deref(),
    ),
    Command::GitopsSourceGet { ns, source_id } => {
      cmd_gitops_source_get(&client, ns.as_str(), source_id.as_str())
    }
    Command::GitopsSourceList {
      ns,
      status,
      mode,
      limit,
    } => cmd_gitops_source_list(
      &client,
      ns.as_str(),
      status.as_deref(),
      mode.as_deref(),
      limit,
    ),
    Command::GitopsSourceDisable { ns, source_id } => {
      cmd_gitops_source_disable(&client, ns.as_str(), source_id.as_str())
    }
    Command::GitopsSyncNow {
      ns,
      source_id,
      commit_sha,
      apply,
    } => cmd_gitops_sync_now(
      &client,
      ns.as_str(),
      source_id.as_str(),
      commit_sha.as_deref(),
      apply,
    ),
    Command::GitopsStatus { ns, source_id } => {
      cmd_gitops_status(&client, ns.as_str(), source_id.as_str())
    }
    Command::AdmissionCheck { file } => cmd_admission_check(&client, &file),
    Command::PolicyEvalList {
      ns,
      channel,
      target,
      decision,
      since_ms,
      limit,
    } => cmd_policy_eval_list(
      &client,
      ns.as_str(),
      channel.as_deref(),
      target.as_deref(),
      decision.as_deref(),
      since_ms,
      limit,
    ),
    Command::TenantCreate {
      name,
      tenant_id,
      owner_json,
      status,
    } => cmd_tenant_create(
      &client,
      name.as_str(),
      tenant_id.as_deref(),
      owner_json.as_deref(),
      status.as_deref(),
    ),
    Command::TenantGet { tenant_id } => cmd_tenant_get(&client, tenant_id.as_str()),
    Command::TenantList {
      status,
      name,
      limit,
    } => cmd_tenant_list(&client, status.as_deref(), name.as_deref(), limit),
    Command::TenantUpdate {
      tenant_id,
      name,
      owner_json,
      status,
    } => cmd_tenant_update(
      &client,
      tenant_id.as_str(),
      name.as_deref(),
      owner_json.as_deref(),
      status.as_deref(),
    ),
    Command::TenantSuspend { tenant_id } => cmd_tenant_suspend(&client, tenant_id.as_str()),
    Command::TenantAttachNamespace { tenant_id, ns } => {
      cmd_tenant_attach_namespace(&client, tenant_id.as_str(), ns.as_str())
    }
    Command::TenantDetachNamespace { tenant_id, ns } => {
      cmd_tenant_detach_namespace(&client, tenant_id.as_str(), ns.as_str())
    }
    Command::TenantNamespacesList { tenant_id, limit } => {
      cmd_tenant_namespaces_list(&client, tenant_id.as_str(), limit)
    }
    Command::QuotaSet {
      tenant_id,
      policy_id,
      scope_json,
      limits_json,
    } => cmd_quota_set(
      &client,
      tenant_id.as_str(),
      policy_id.as_deref(),
      scope_json.as_deref(),
      limits_json.as_deref(),
    ),
    Command::QuotaGet {
      tenant_id,
      policy_id,
    } => cmd_quota_get(&client, tenant_id.as_str(), policy_id.as_deref()),
    Command::BudgetSet {
      tenant_id,
      policy_id,
      period,
      currency,
      budget_minor,
      thresholds_json,
      actions_json,
    } => cmd_budget_set(
      &client,
      tenant_id.as_str(),
      policy_id.as_deref(),
      period.as_deref(),
      currency.as_deref(),
      budget_minor,
      thresholds_json.as_deref(),
      actions_json.as_deref(),
    ),
    Command::BudgetGet {
      tenant_id,
      policy_id,
    } => cmd_budget_get(&client, tenant_id.as_str(), policy_id.as_deref()),
    Command::BudgetStateUpsert {
      tenant_id,
      policy_id,
      period_key,
      spent_minor,
      budget_minor,
      level,
      last_enforcement_change_id,
    } => cmd_budget_state_upsert(
      &client,
      tenant_id.as_str(),
      policy_id.as_deref(),
      period_key.as_str(),
      spent_minor,
      budget_minor,
      level.as_deref(),
      last_enforcement_change_id.as_deref(),
    ),
    Command::BudgetStateGet {
      tenant_id,
      policy_id,
      period_key,
    } => cmd_budget_state_get(
      &client,
      tenant_id.as_str(),
      policy_id.as_deref(),
      period_key.as_str(),
    ),
    Command::UsageRollupUpsert {
      tenant_id,
      ns,
      ts_ms,
      region,
      cpu_millis_avg,
      cpu_millis_hour,
      mem_bytes_avg,
      mem_bytes_hour,
      proc_count_avg,
      artifact_bytes_avg,
      egress_bytes,
      gateway_req_count,
      gateway_err5xx_count,
    } => cmd_usage_rollup_upsert(
      &client,
      tenant_id.as_str(),
      ns.as_str(),
      ts_ms,
      region.as_deref(),
      cpu_millis_avg,
      cpu_millis_hour,
      mem_bytes_avg,
      mem_bytes_hour,
      proc_count_avg,
      artifact_bytes_avg,
      egress_bytes,
      gateway_req_count,
      gateway_err5xx_count,
    ),
    Command::UsageQuery {
      tenant_id,
      ns,
      from_ts_ms,
      to_ts_ms,
      region,
      limit,
    } => cmd_usage_query(
      &client,
      tenant_id.as_str(),
      ns.as_deref(),
      from_ts_ms,
      to_ts_ms,
      region.as_deref(),
      limit,
    ),
    Command::CostRollupUpsert {
      tenant_id,
      ts_ms,
      currency,
      cost_minor,
      rate_card_id,
      breakdown_json,
    } => cmd_cost_rollup_upsert(
      &client,
      tenant_id.as_str(),
      ts_ms,
      currency.as_deref(),
      cost_minor,
      rate_card_id.as_str(),
      breakdown_json.as_deref(),
    ),
    Command::CostQuery {
      tenant_id,
      from_ts_ms,
      to_ts_ms,
      rate_card_id,
      limit,
    } => cmd_cost_query(
      &client,
      tenant_id.as_str(),
      from_ts_ms,
      to_ts_ms,
      rate_card_id.as_deref(),
      limit,
    ),
    Command::RateCardCreate {
      name,
      card_id,
      currency,
      rates_json,
      valid_from_ms,
      valid_to_ms,
      status,
    } => cmd_rate_card_create(
      &client,
      name.as_str(),
      card_id.as_deref(),
      currency.as_deref(),
      rates_json.as_deref(),
      valid_from_ms,
      valid_to_ms,
      status.as_deref(),
    ),
    Command::RateCardActivate { card_id } => cmd_rate_card_activate(&client, card_id.as_str()),
    Command::RateCardGet { card_id } => cmd_rate_card_get(&client, card_id.as_str()),
    Command::RateCardList {
      status,
      currency,
      limit,
    } => cmd_rate_card_list(&client, status.as_deref(), currency.as_deref(), limit),
    Command::InvoiceGenerate {
      tenant_id,
      period,
      currency,
      from_ts_ms,
      to_ts_ms,
      rate_card_id,
      invoice_id,
      report_ref_id,
    } => cmd_invoice_generate(
      &client,
      tenant_id.as_str(),
      period.as_str(),
      currency.as_deref(),
      from_ts_ms,
      to_ts_ms,
      rate_card_id.as_deref(),
      invoice_id.as_deref(),
      report_ref_id.as_deref(),
    ),
    Command::InvoiceGet {
      tenant_id,
      invoice_id,
    } => cmd_invoice_get(&client, tenant_id.as_str(), invoice_id.as_str()),
    Command::InvoiceList {
      tenant_id,
      period,
      status,
      limit,
    } => cmd_invoice_list(
      &client,
      tenant_id.as_str(),
      period.as_deref(),
      status.as_deref(),
      limit,
    ),
    Command::ForecastProfileSet {
      ns,
      profile_id,
      scope_kind,
      scope_id,
      channel,
      model_kind,
      horizon_ms,
      step_ms,
      features_json,
      guardrails_json,
    } => cmd_forecast_profile_set(
      &client,
      ns.as_str(),
      profile_id.as_deref(),
      scope_kind.as_str(),
      scope_id.as_str(),
      channel.as_deref(),
      model_kind.as_deref(),
      horizon_ms,
      step_ms,
      features_json.as_deref(),
      guardrails_json.as_deref(),
    ),
    Command::ForecastProfileGet { ns, profile_id } => {
      cmd_forecast_profile_get(&client, ns.as_str(), profile_id.as_str())
    }
    Command::ForecastProfileList {
      ns,
      scope_kind,
      scope_id,
      channel,
      limit,
    } => cmd_forecast_profile_list(
      &client,
      ns.as_str(),
      scope_kind.as_deref(),
      scope_id.as_deref(),
      channel.as_deref(),
      limit,
    ),
    Command::ForecastRunLatest {
      ns,
      scope_kind,
      scope_id,
      channel,
    } => cmd_forecast_run_latest(
      &client,
      ns.as_str(),
      scope_kind.as_str(),
      scope_id.as_str(),
      channel.as_deref(),
    ),
    Command::PredictiveRunNow {
      ns,
      scope_kind,
      scope_id,
      channel,
      profile_id,
    } => cmd_predictive_run_now(
      &client,
      ns.as_str(),
      scope_kind.as_str(),
      scope_id.as_str(),
      channel.as_deref(),
      profile_id.as_deref(),
    ),
    Command::PredictivePlanGet { ns, plan_id } => {
      cmd_predictive_plan_get(&client, ns.as_str(), plan_id.as_str())
    }
    Command::PredictivePlanList {
      ns,
      scope_kind,
      scope_id,
      channel,
      status,
      limit,
    } => cmd_predictive_plan_list(
      &client,
      ns.as_str(),
      scope_kind.as_deref(),
      scope_id.as_deref(),
      channel.as_deref(),
      status.as_deref(),
      limit,
    ),
    Command::SimulationRun {
      ns,
      source_kind,
      source_id,
      base_ref_json,
      proposed_ref_json,
      env_snapshot_ref_id,
    } => cmd_simulation_run(
      &client,
      ns.as_str(),
      source_kind.as_str(),
      source_id.as_str(),
      base_ref_json.as_deref(),
      proposed_ref_json.as_deref(),
      env_snapshot_ref_id.as_deref(),
    ),
    Command::SimulationGetLatest {
      ns,
      source_kind,
      source_id,
    } => cmd_simulation_get_latest(
      &client,
      ns.as_str(),
      source_kind.as_str(),
      source_id.as_str(),
    ),
    Command::AlertRuleSet {
      ns,
      rule_id,
      name,
      source_kind,
      selector_json,
      window_ms,
      eval_every_ms,
      condition_json,
      severity,
      dedupe_key_template,
      labels_json,
      autopilot_json,
      status,
    } => cmd_alert_rule_set(
      &client,
      ns.as_str(),
      rule_id.as_deref(),
      name.as_str(),
      source_kind.as_str(),
      selector_json.as_deref(),
      window_ms,
      eval_every_ms,
      condition_json.as_deref(),
      severity.as_deref(),
      dedupe_key_template.as_deref(),
      labels_json.as_deref(),
      autopilot_json.as_deref(),
      status.as_deref(),
    ),
    Command::AlertRuleGet { ns, rule_id } => {
      cmd_alert_rule_get(&client, ns.as_str(), rule_id.as_str())
    }
    Command::AlertRuleList {
      ns,
      source_kind,
      severity,
      status,
      limit,
    } => cmd_alert_rule_list(
      &client,
      ns.as_str(),
      source_kind.as_deref(),
      severity.as_deref(),
      status.as_deref(),
      limit,
    ),
    Command::AlertRuleDisable { ns, rule_id } => {
      cmd_alert_rule_disable(&client, ns.as_str(), rule_id.as_str())
    }
    Command::AlertEvaluate {
      ns,
      rule_id,
      dedupe_key,
      value_json,
    } => cmd_alert_evaluate(
      &client,
      ns.as_str(),
      rule_id.as_str(),
      dedupe_key.as_deref(),
      value_json.as_deref(),
    ),
    Command::IncidentGet { ns, incident_id } => {
      cmd_incident_get(&client, ns.as_str(), incident_id.as_str())
    }
    Command::IncidentList {
      ns,
      status,
      severity,
      primary_object_kind,
      primary_object_id,
      limit,
    } => cmd_incident_list(
      &client,
      ns.as_str(),
      status.as_deref(),
      severity.as_deref(),
      primary_object_kind.as_deref(),
      primary_object_id.as_deref(),
      limit,
    ),
    Command::IncidentAck {
      ns,
      incident_id,
      owner_json,
    } => cmd_incident_ack(
      &client,
      ns.as_str(),
      incident_id.as_str(),
      owner_json.as_deref(),
    ),
    Command::IncidentNote {
      ns,
      incident_id,
      message,
      meta_json,
    } => cmd_incident_note(
      &client,
      ns.as_str(),
      incident_id.as_str(),
      message.as_str(),
      meta_json.as_deref(),
    ),
    Command::IncidentResolve {
      ns,
      incident_id,
      summary,
      root_cause,
    } => cmd_incident_resolve(
      &client,
      ns.as_str(),
      incident_id.as_str(),
      summary.as_deref(),
      root_cause.as_deref(),
    ),
    Command::RunbookSet {
      ns,
      runbook_id,
      name,
      version,
      bundle_ref_id,
      sha256,
      policy_json,
      steps_json,
      status,
    } => cmd_runbook_set(
      &client,
      ns.as_str(),
      runbook_id.as_deref(),
      name.as_str(),
      version.as_deref(),
      bundle_ref_id.as_deref(),
      sha256.as_deref(),
      policy_json.as_deref(),
      steps_json.as_deref(),
      status.as_deref(),
    ),
    Command::RunbookGet { ns, name } => cmd_runbook_get(&client, ns.as_str(), name.as_str()),
    Command::RunbookList {
      ns,
      status,
      name,
      limit,
    } => cmd_runbook_list(
      &client,
      ns.as_str(),
      status.as_deref(),
      name.as_deref(),
      limit,
    ),
    Command::RunbookExecute {
      ns,
      incident_id,
      runbook_name,
      mode,
      auto_run,
    } => cmd_runbook_execute(
      &client,
      ns.as_str(),
      incident_id.as_str(),
      runbook_name.as_str(),
      mode.as_deref(),
      auto_run,
    ),
    Command::PostmortemGenerate {
      ns,
      incident_id,
      pm_id,
      report_ref_id,
    } => cmd_postmortem_generate(
      &client,
      ns.as_str(),
      incident_id.as_str(),
      pm_id.as_deref(),
      report_ref_id.as_deref(),
    ),
    Command::PostmortemGet { ns, incident_id } => {
      cmd_postmortem_get(&client, ns.as_str(), incident_id.as_str())
    }
    Command::ChaosExperimentPublish {
      ns,
      exp_id,
      name,
      bundle_ref_id,
      sha256,
      spec_json,
      status,
    } => cmd_chaos_experiment_publish(
      &client,
      ns.as_str(),
      exp_id.as_deref(),
      name.as_str(),
      bundle_ref_id.as_str(),
      sha256.as_str(),
      spec_json.as_deref(),
      status.as_deref(),
    ),
    Command::ChaosExperimentGet { ns, exp_id } => {
      cmd_chaos_experiment_get(&client, ns.as_str(), exp_id.as_str())
    }
    Command::ChaosExperimentList {
      ns,
      status,
      name,
      limit,
    } => cmd_chaos_experiment_list(
      &client,
      ns.as_str(),
      status.as_deref(),
      name.as_deref(),
      limit,
    ),
    Command::ChaosExperimentApprove { ns, exp_id } => {
      cmd_chaos_experiment_approve(&client, ns.as_str(), exp_id.as_str())
    }
    Command::ChaosExperimentDisable { ns, exp_id } => {
      cmd_chaos_experiment_disable(&client, ns.as_str(), exp_id.as_str())
    }
    Command::ChaosRunStart { ns, exp_id } => {
      cmd_chaos_run_start(&client, ns.as_str(), exp_id.as_str())
    }
    Command::ChaosRunAbort { ns, run_id } => {
      cmd_chaos_run_abort(&client, ns.as_str(), run_id.as_str())
    }
    Command::ChaosRunGet { ns, run_id } => cmd_chaos_run_get(&client, ns.as_str(), run_id.as_str()),
    Command::ChaosRunLatest { ns, exp_id } => {
      cmd_chaos_run_latest(&client, ns.as_str(), exp_id.as_str())
    }
    Command::KeyringAdd {
      key_id,
      public_key_b64,
      scope_json,
    } => cmd_keyring_add(
      &client,
      key_id.as_str(),
      public_key_b64.as_str(),
      scope_json.as_deref(),
    ),
    Command::KeyringRevoke { key_id } => cmd_keyring_revoke(&client, key_id.as_str()),
    Command::KeyringList => cmd_keyring_list(&client),
    Command::AttestVerify {
      ns,
      file,
      att_id,
      att_ref_id,
      invocation_id,
      session_id,
      release_id,
    } => cmd_attest_verify(
      &client,
      ns.as_str(),
      &file,
      att_id.as_deref(),
      att_ref_id.as_deref(),
      invocation_id.as_deref(),
      session_id.as_deref(),
      release_id.as_deref(),
    ),
    Command::AttestList {
      ns,
      subject_sha256,
      attestation_type,
      limit,
    } => cmd_attest_list(
      &client,
      ns.as_str(),
      subject_sha256.as_deref(),
      attestation_type.as_deref(),
      limit,
    ),
    Command::CompliancePolicyGet { ns, channel } => {
      cmd_compliance_policy_get(&client, ns.as_str(), channel.as_str())
    }
    Command::CompliancePolicySet { ns, channel, file } => {
      cmd_compliance_policy_set(&client, ns.as_str(), channel.as_str(), &file)
    }
    Command::ComplianceExceptionCreate {
      ns,
      channel,
      target,
      reason,
      subject_id,
      rule_json,
      duration_ms,
      approvals_required,
    } => cmd_compliance_exception_create(
      &client,
      ns.as_str(),
      channel.as_str(),
      target.as_str(),
      reason.as_str(),
      subject_id.as_deref(),
      rule_json.as_deref(),
      duration_ms,
      approvals_required,
    ),
    Command::ComplianceExceptionApprove { ns, exc_id } => cmd_compliance_exception_decision(
      &client,
      "compliance.exception.approve",
      ns.as_str(),
      exc_id.as_str(),
    ),
    Command::ComplianceExceptionRevoke { ns, exc_id } => cmd_compliance_exception_decision(
      &client,
      "compliance.exception.revoke",
      ns.as_str(),
      exc_id.as_str(),
    ),
    Command::ComplianceExceptionList {
      ns,
      channel,
      target,
      status,
      subject_id,
      limit,
    } => cmd_compliance_exception_list(
      &client,
      ns.as_str(),
      channel.as_deref(),
      target.as_deref(),
      status.as_deref(),
      subject_id.as_deref(),
      limit,
    ),
    Command::ComplianceScan { ns, channel } => {
      cmd_compliance_scan(&client, ns.as_str(), channel.as_str())
    }
    Command::ComplianceReportLatest { ns, channel } => {
      cmd_compliance_report_latest(&client, ns.as_str(), channel.as_str())
    }
    Command::RuntimeCatalogUpsert {
      ns,
      family,
      version,
      exec_ref_id,
      sha256,
      name,
      platform_os,
      platform_arch,
      sbom_ref_id,
      vuln_report_id,
      attestation_ref_id,
      status,
    } => cmd_runtime_catalog_upsert(
      &client,
      ns.as_str(),
      family.as_str(),
      version.as_str(),
      exec_ref_id.as_str(),
      sha256.as_str(),
      name.as_deref(),
      platform_os.as_deref(),
      platform_arch.as_deref(),
      sbom_ref_id.as_deref(),
      vuln_report_id.as_deref(),
      attestation_ref_id.as_deref(),
      status.as_deref(),
    ),
    Command::RuntimeCatalogList {
      ns,
      family,
      status,
      platform_os,
      platform_arch,
      limit,
    } => cmd_runtime_catalog_list(
      &client,
      ns.as_str(),
      family.as_deref(),
      status.as_deref(),
      platform_os.as_deref(),
      platform_arch.as_deref(),
      limit,
    ),
    Command::RuntimeAliasSet {
      ns,
      family,
      alias,
      version,
    } => cmd_runtime_alias_set(
      &client,
      ns.as_str(),
      family.as_str(),
      alias.as_str(),
      version.as_str(),
    ),
    Command::RuntimeAliasList { ns, family } => {
      cmd_runtime_alias_list(&client, ns.as_str(), family.as_deref())
    }
    Command::RuntimeAliasResolve {
      ns,
      family,
      alias,
      platform_os,
      platform_arch,
    } => cmd_runtime_alias_resolve(
      &client,
      ns.as_str(),
      family.as_str(),
      alias.as_str(),
      platform_os.as_deref(),
      platform_arch.as_deref(),
    ),
    Command::SecurityComposeVerdict {
      ns,
      runtime_ref_id,
      inputs,
      policy_id,
    } => cmd_security_compose_verdict(
      &client,
      ns.as_str(),
      runtime_ref_id.as_str(),
      inputs.as_str(),
      policy_id.as_deref(),
    ),
    Command::SecurityVerdictGet { ns, verdict_id } => {
      cmd_security_verdict_get(&client, ns.as_str(), verdict_id.as_str())
    }
    Command::SecurityVerdictLookup {
      ns,
      runtime_sha256,
      app_sha256,
      limit,
    } => cmd_security_verdict_lookup(
      &client,
      ns.as_str(),
      runtime_sha256.as_str(),
      app_sha256.as_str(),
      limit,
    ),
    Command::BlueprintBundlePublish {
      ns,
      name,
      version,
      bundle_ref_id,
      sha256,
      attestation_ref_id,
      status,
      manifest_file,
    } => cmd_blueprint_bundle_publish(
      &client,
      ns.as_str(),
      name.as_str(),
      version.as_str(),
      bundle_ref_id.as_str(),
      sha256.as_str(),
      attestation_ref_id.as_deref(),
      status.as_deref(),
      manifest_file.as_ref(),
    ),
    Command::BlueprintBundleList {
      ns,
      name,
      status,
      limit,
    } => cmd_blueprint_bundle_list(
      &client,
      ns.as_str(),
      name.as_deref(),
      status.as_deref(),
      limit,
    ),
    Command::BlueprintBundleGet { ns, bundle_id } => {
      cmd_blueprint_bundle_get(&client, ns.as_str(), bundle_id.as_str())
    }
    Command::BlueprintBundleApprove { ns, bundle_id } => {
      cmd_blueprint_bundle_approve(&client, ns.as_str(), bundle_id.as_str())
    }
    Command::BlueprintBundleRevoke { ns, bundle_id } => {
      cmd_blueprint_bundle_revoke(&client, ns.as_str(), bundle_id.as_str())
    }
    Command::ServiceCreate {
      ns,
      service_name,
      channel,
      blueprint_name,
      params_file,
      overrides_file,
    } => cmd_service_create(
      &client,
      ns.as_str(),
      service_name.as_str(),
      channel.as_str(),
      blueprint_name.as_str(),
      &params_file,
      overrides_file.as_ref(),
    ),
    Command::ServiceUpdateParams {
      ns,
      service_name,
      channel,
      params_file,
    } => cmd_service_update_params(
      &client,
      ns.as_str(),
      service_name.as_str(),
      channel.as_str(),
      &params_file,
    ),
    Command::ServiceGet {
      ns,
      service_name,
      channel,
    } => cmd_service_get(
      &client,
      ns.as_str(),
      service_name.as_str(),
      channel.as_str(),
    ),
    Command::ServiceList {
      ns,
      channel,
      blueprint_name,
      status,
      limit,
    } => cmd_service_list(
      &client,
      ns.as_str(),
      channel.as_deref(),
      blueprint_name.as_deref(),
      status.as_deref(),
      limit,
    ),
    Command::ServiceReleaseCreate {
      ns,
      service_name,
      channel,
      blueprint_bundle_id,
      notes,
    } => cmd_service_release_create(
      &client,
      ns.as_str(),
      service_name.as_str(),
      channel.as_str(),
      blueprint_bundle_id.as_str(),
      notes.as_deref(),
    ),
    Command::ServiceReleaseApprove { ns, release_id } => {
      cmd_service_release_approve(&client, ns.as_str(), release_id.as_str())
    }
    Command::ServiceReleaseActivate { ns, release_id } => {
      cmd_service_release_activate(&client, ns.as_str(), release_id.as_str())
    }
    Command::ServiceReleaseRollback { ns, release_id } => {
      cmd_service_release_rollback(&client, ns.as_str(), release_id.as_str())
    }
    Command::ServiceRenderPreview {
      ns,
      service_name,
      channel,
      blueprint_bundle_id,
      params_file,
      validate_only,
      dry_run,
    } => cmd_service_render_preview(
      &client,
      ns.as_str(),
      service_name.as_str(),
      channel.as_str(),
      blueprint_bundle_id.as_deref(),
      params_file.as_ref(),
      validate_only,
      dry_run,
    ),
    Command::ServiceRenderGetLatest {
      ns,
      service_name,
      channel,
    } => cmd_service_render_get_latest(
      &client,
      ns.as_str(),
      service_name.as_str(),
      channel.as_str(),
    ),
    Command::NodeHello {
      node_id,
      zone,
      region,
      host,
      endpoint,
      labels_json,
      caps,
      capacity_json,
      lease_ms,
    } => cmd_node_hello(
      &client,
      node_id.as_str(),
      zone.as_deref(),
      region.as_deref(),
      host.as_deref(),
      endpoint.as_deref(),
      labels_json.as_deref(),
      caps.as_deref(),
      capacity_json.as_deref(),
      lease_ms,
    ),
    Command::NodeHeartbeat {
      node_id,
      status,
      zone,
      region,
      host,
      endpoint,
      labels_json,
      caps,
      capacity_json,
      lease_ms,
    } => cmd_node_heartbeat(
      &client,
      node_id.as_str(),
      status.as_deref(),
      zone.as_deref(),
      region.as_deref(),
      host.as_deref(),
      endpoint.as_deref(),
      labels_json.as_deref(),
      caps.as_deref(),
      capacity_json.as_deref(),
      lease_ms,
    ),
    Command::NodeSetStatus { node_id, status } => {
      cmd_node_set_status(&client, node_id.as_str(), status.as_str())
    }
    Command::NodeList {
      status,
      zone,
      limit,
    } => cmd_node_list(&client, status.as_deref(), zone.as_deref(), limit),
    Command::ReplicaAssignmentUpsert {
      ns,
      rs_id,
      replica_ordinal,
      process_id,
      assigned_node,
      status,
      service_name,
      channel,
      release_id,
    } => cmd_replica_assignment_upsert(
      &client,
      ns.as_str(),
      rs_id.as_str(),
      replica_ordinal,
      process_id.as_str(),
      assigned_node.as_deref(),
      status.as_deref(),
      service_name.as_deref(),
      channel.as_deref(),
      release_id.as_deref(),
    ),
    Command::ReplicaAssignmentList {
      ns,
      rs_id,
      assigned_node,
      status,
      service_name,
      channel,
      limit,
    } => cmd_replica_assignment_list(
      &client,
      ns.as_str(),
      rs_id.as_deref(),
      assigned_node.as_deref(),
      status.as_deref(),
      service_name.as_deref(),
      channel.as_deref(),
      limit,
    ),
    Command::ServiceEndpointUpsert {
      ns,
      service_name,
      channel,
      process_id,
      node_id,
      host,
      port,
      status,
      release_id,
      rs_id,
    } => cmd_service_endpoint_upsert(
      &client,
      ns.as_str(),
      service_name.as_str(),
      channel.as_str(),
      process_id.as_str(),
      node_id.as_str(),
      host.as_str(),
      port,
      status.as_deref(),
      release_id.as_deref(),
      rs_id.as_deref(),
    ),
    Command::ServiceEndpointsGet {
      ns,
      service_name,
      channel,
      status,
      limit,
    } => cmd_service_endpoints_get(
      &client,
      ns.as_str(),
      service_name.as_str(),
      channel.as_str(),
      status.as_deref(),
      limit,
    ),
    Command::RemediationPlanCreate {
      ns,
      file,
      risk,
      summary,
      trigger_json,
      steps_json,
      invocation_id,
      session_id,
    } => cmd_remediation_plan_create(
      &client,
      ns.as_str(),
      file.as_ref(),
      risk.as_str(),
      summary.as_deref(),
      trigger_json.as_deref(),
      steps_json.as_deref(),
      invocation_id.as_deref(),
      session_id.as_deref(),
    ),
    Command::RemediationPlanGet { ns, plan_id } => {
      cmd_remediation_plan_get(&client, ns.as_str(), plan_id.as_str())
    }
    Command::RemediationPlanList {
      ns,
      status,
      risk,
      limit,
    } => cmd_remediation_plan_list(
      &client,
      ns.as_str(),
      status.as_deref(),
      risk.as_deref(),
      limit,
    ),
    Command::RemediationPlanEscalate {
      ns,
      plan_id,
      title,
      reason,
      source,
      risk,
      auto_run,
    } => cmd_remediation_plan_escalate(
      &client,
      ns.as_str(),
      plan_id.as_str(),
      title.as_deref(),
      reason.as_deref(),
      source.as_deref(),
      risk.as_deref(),
      auto_run,
    ),
    Command::ChangeGet { ns, change_id } => {
      cmd_change_get(&client, ns.as_str(), change_id.as_str())
    }
    Command::ChangeList {
      ns,
      status,
      risk,
      limit,
    } => cmd_change_list(
      &client,
      ns.as_str(),
      status.as_deref(),
      risk.as_deref(),
      limit,
    ),
    Command::ChangeApprove {
      ns,
      change_id,
      note,
    } => cmd_change_decision(
      &client,
      "change.approve",
      ns.as_str(),
      change_id.as_str(),
      note.as_deref(),
    ),
    Command::ChangeReject {
      ns,
      change_id,
      note,
    } => cmd_change_decision(
      &client,
      "change.reject",
      ns.as_str(),
      change_id.as_str(),
      note.as_deref(),
    ),
    Command::ChangeRun { ns, change_id } => {
      cmd_change_run(&client, ns.as_str(), change_id.as_str())
    }
    Command::ChangeCancel { ns, change_id } => {
      cmd_change_cancel(&client, ns.as_str(), change_id.as_str())
    }
    Command::JobSubmit {
      ns,
      kind,
      params,
      dedupe_key,
      priority,
      invocation_id,
      session_id,
      max_attempt,
    } => cmd_job_submit(
      &client,
      ns.as_str(),
      kind.as_str(),
      params.as_deref(),
      dedupe_key.as_deref(),
      priority,
      invocation_id.as_deref(),
      session_id.as_deref(),
      max_attempt,
    ),
    Command::JobGet { ns, job_id } => cmd_job_get(&client, ns.as_str(), job_id.as_str()),
    Command::JobList {
      ns,
      state,
      kind_prefix,
      invocation_id,
      session_id,
      limit,
    } => cmd_job_list(
      &client,
      ns.as_str(),
      state.as_deref(),
      kind_prefix.as_deref(),
      invocation_id.as_deref(),
      session_id.as_deref(),
      limit,
    ),
    Command::JobCancel { ns, job_id } => cmd_job_cancel(&client, ns.as_str(), job_id.as_str()),
    Command::JobLogs {
      ns,
      job_id,
      n,
      max_bytes,
    } => cmd_job_logs(&client, ns.as_str(), job_id.as_str(), n, max_bytes),
    Command::JobWait {
      ns,
      job_id,
      timeout_ms,
    } => cmd_job_wait(&client, ns.as_str(), job_id.as_str(), timeout_ms),
    Command::InstallSystemd {
      root,
      source_root,
      dry_run,
    } => cmd_install_systemd(&root, source_root.as_ref(), dry_run),
  }
}

fn cmd_doctor(client: &SupervisorClient, endpoint: &str, ns: &str) -> Result<()> {
  let hello = client.hello().context("hello handshake")?;
  let list = client.list().context("list supervisor morphisms")?;
  let query = client.call(
    "process.query",
    json!({
      "ns": ns,
      "limit": 1,
      "select": ["ns","id","status","pid","generation","base_url","fail_count"]
    }),
  )?;
  let events = client.call(
    "events.poll",
    json!({
      "ns": ns,
      "cursor": 0,
      "timeout_ms": 1,
      "max": 1,
    }),
  )?;

  println!(
    "{}",
    serde_json::to_string_pretty(&json!({
      "ok": true,
      "endpoint": endpoint,
      "ns": ns,
      "hello": hello,
      "ops_count": list.len(),
      "has_query": list.iter().any(|op| op == "process.query"),
      "has_watch": list.iter().any(|op| op == "process.watch.poll"),
      "query_probe": query,
      "events_probe": events,
    }))?
  );
  Ok(())
}

fn cmd_bench(client: &SupervisorClient, ns: &str, n: usize, batch: usize) -> Result<()> {
  let n = n.max(1);
  let batch = batch.max(1);
  let payload = json!({
    "ns": ns,
    "limit": 1,
    "select": ["ns","id","status","pid","generation"]
  });

  let started = Instant::now();
  if batch == 1 {
    for _ in 0..n {
      let _ = client.call("process.query", payload.clone())?;
    }
  } else {
    let mut remaining = n;
    while remaining > 0 {
      let size = remaining.min(batch);
      let calls = (0..size)
        .map(|_| BatchCall::new("process.query", payload.clone()))
        .collect::<Vec<_>>();
      let _ = client.call_batch(&calls)?;
      remaining -= size;
    }
  }
  let elapsed = started.elapsed();
  let total_ms = elapsed.as_secs_f64() * 1000.0;
  let avg_ms = total_ms / (n as f64);
  let qps = (n as f64) / elapsed.as_secs_f64().max(1e-9);

  println!(
    "{}",
    serde_json::to_string_pretty(&json!({
      "ok": true,
      "bench": "process.query",
      "ns": ns,
      "calls": n,
      "batch": batch,
      "elapsed_ms": total_ms,
      "avg_call_ms": avg_ms,
      "qps": qps,
    }))?
  );
  Ok(())
}

fn cmd_audit(
  client: &SupervisorClient,
  ns: &str,
  limit: usize,
  decision: Option<&str>,
  status: Option<&str>,
  op_prefix: Option<&str>,
  invocation_id: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "audit.query",
    json!({
      "ns": ns,
      "limit": limit,
      "decision": decision,
      "status": status,
      "op_prefix": op_prefix,
      "invocation_id": invocation_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn gateway_policy_base(kind: &str) -> Result<&'static str> {
  let normalized = kind.trim().to_ascii_lowercase();
  match normalized.as_str() {
    "auth-provider" | "auth_provider" | "auth.provider" => Ok("auth.provider"),
    "auth-policy" | "auth_policy" | "auth.policy" => Ok("auth.policy"),
    "access-policy" | "access_policy" | "access.policy" => Ok("access.policy"),
    "ratelimit-policy" | "ratelimit_policy" | "ratelimit.policy" => Ok("ratelimit.policy"),
    "waf-policy" | "waf_policy" | "waf.policy" => Ok("waf.policy"),
    "header-policy" | "header_policy" | "header.policy" => Ok("header.policy"),
    _ => anyhow::bail!(
      "unsupported --kind `{}` (expected auth-provider|auth-policy|access-policy|ratelimit-policy|waf-policy|header-policy)",
      kind
    ),
  }
}

fn cmd_gateway_policy_upsert(
  client: &SupervisorClient,
  kind: &str,
  ns: Option<&str>,
  file: &PathBuf,
) -> Result<()> {
  let prefix = gateway_policy_base(kind)?;
  let raw = std::fs::read_to_string(file)
    .with_context(|| format!("read gateway policy payload {}", file.display()))?;
  let mut payload: Value =
    serde_json::from_str(&raw).with_context(|| format!("parse {}", file.display()))?;
  let object = payload.as_object_mut().with_context(|| {
    format!(
      "gateway policy payload {} must be a JSON object",
      file.display()
    )
  })?;
  if let Some(ns) = ns {
    object.insert("ns".to_string(), json!(ns));
  }
  let op = format!("{}.create", prefix);
  let outputs = client.call(op.as_str(), payload)?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_gateway_policy_get(client: &SupervisorClient, kind: &str, ns: &str, id: &str) -> Result<()> {
  let prefix = gateway_policy_base(kind)?;
  let op = format!("{}.get", prefix);
  let outputs = client.call(
    op.as_str(),
    json!({
      "ns": ns,
      "id": id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_gateway_policy_list(
  client: &SupervisorClient,
  kind: &str,
  ns: &str,
  name: Option<&str>,
  status: Option<&str>,
  limit: usize,
) -> Result<()> {
  let prefix = gateway_policy_base(kind)?;
  let op = format!("{}.list", prefix);
  let outputs = client.call(
    op.as_str(),
    json!({
      "ns": ns,
      "name": name,
      "status": status,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_gateway_policy_disable(
  client: &SupervisorClient,
  kind: &str,
  ns: &str,
  id: &str,
) -> Result<()> {
  let prefix = gateway_policy_base(kind)?;
  let op = format!("{}.disable", prefix);
  let outputs = client.call(
    op.as_str(),
    json!({
      "ns": ns,
      "id": id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_route_policy_attach(
  client: &SupervisorClient,
  ns: &str,
  route_id: &str,
  auth_policy_id: Option<&str>,
  access_policy_id: Option<&str>,
  rlp_id: Option<&str>,
  waf_id: Option<&str>,
  hp_id: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "route.policy.attach",
    json!({
      "ns": ns,
      "route_id": route_id,
      "auth_policy_id": auth_policy_id,
      "access_policy_id": access_policy_id,
      "rlp_id": rlp_id,
      "waf_id": waf_id,
      "hp_id": hp_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_route_policy_get(client: &SupervisorClient, ns: &str, route_id: &str) -> Result<()> {
  let outputs = client.call(
    "route.policy.get",
    json!({
      "ns": ns,
      "route_id": route_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_gateway_decision_stats(
  client: &SupervisorClient,
  ns: &str,
  route_id: Option<&str>,
  code: Option<&str>,
  since_ms: Option<u128>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "gateway.decision.stats",
    json!({
      "ns": ns,
      "route_id": route_id,
      "code": code,
      "since_ms": since_ms,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_trace_register(
  client: &SupervisorClient,
  ns: &str,
  invocation_id: &str,
  path: &PathBuf,
  trace_mode: &str,
  compressed: Option<&str>,
  size_bytes: Option<u64>,
) -> Result<()> {
  let outputs = client.call(
    "trace.register",
    json!({
      "ns": ns,
      "invocation_id": invocation_id,
      "path": path,
      "trace_mode": trace_mode,
      "compressed": compressed,
      "size_bytes": size_bytes,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_trace_summary(client: &SupervisorClient, ns: &str, invocation_id: &str) -> Result<()> {
  let outputs = client.call(
    "trace.summary",
    json!({
      "ns": ns,
      "invocation_id": invocation_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_trace_tail(
  client: &SupervisorClient,
  ns: &str,
  invocation_id: &str,
  n: usize,
  max_bytes: usize,
) -> Result<()> {
  let outputs = client.call(
    "trace.tail",
    json!({
      "ns": ns,
      "invocation_id": invocation_id,
      "n": n,
      "max_bytes": max_bytes,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_bundle_create(
  client: &SupervisorClient,
  ns: &str,
  invocation_id: Option<&str>,
  session_id: Option<&str>,
  mode: &str,
  include_trace: &str,
  include_logs: bool,
  logs_tail_bytes: u64,
  wait: bool,
  timeout_ms: u64,
) -> Result<()> {
  let outputs = client.call(
    "bundle.create",
    json!({
      "ns": ns,
      "invocation_id": invocation_id,
      "session_id": session_id,
      "mode": mode,
      "include_trace": include_trace,
      "include_logs": include_logs,
      "logs_tail_bytes": logs_tail_bytes,
    }),
  )?;
  if wait {
    let job_id = outputs
      .get("job_id")
      .and_then(Value::as_str)
      .context("bundle.create wait requested but response is missing job_id")?;
    let waited = client.call(
      "job.wait",
      json!({
        "ns": ns,
        "job_id": job_id,
        "timeout_ms": timeout_ms,
      }),
    )?;
    println!(
      "{}",
      serde_json::to_string_pretty(&json!({
        "submitted": outputs,
        "wait": waited,
      }))?
    );
    return Ok(());
  }
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_bundle_get(client: &SupervisorClient, ns: &str, bundle_id: &str) -> Result<()> {
  let outputs = client.call(
    "bundle.get",
    json!({
      "ns": ns,
      "bundle_id": bundle_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_bundle_list(
  client: &SupervisorClient,
  ns: &str,
  invocation_id: Option<&str>,
  session_id: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "bundle.list",
    json!({
      "ns": ns,
      "invocation_id": invocation_id,
      "session_id": session_id,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_artifact_store_upsert(
  client: &SupervisorClient,
  store_id: &str,
  endpoint: &str,
  kind: &str,
  zone: Option<&str>,
  read_weight: Option<u32>,
  write_weight: Option<u32>,
  supports_range: Option<bool>,
  supports_put: Option<bool>,
  egress_weight: Option<u32>,
  latency_weight: Option<u32>,
) -> Result<()> {
  let outputs = client.call(
    "artifact.store.upsert",
    json!({
      "store_id": store_id,
      "endpoint": endpoint,
      "kind": kind,
      "zone": zone,
      "read_weight": read_weight,
      "write_weight": write_weight,
      "supports_range": supports_range,
      "supports_put": supports_put,
      "egress_weight": egress_weight,
      "latency_weight": latency_weight,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_artifact_replica_upsert(
  client: &SupervisorClient,
  ns: &str,
  sha256: &str,
  store_id: &str,
  state: &str,
  size_bytes: Option<u64>,
  verified_ms: Option<u128>,
) -> Result<()> {
  let outputs = client.call(
    "artifact.replica.upsert",
    json!({
      "ns": ns,
      "sha256": sha256,
      "store_id": store_id,
      "state": state,
      "size_bytes": size_bytes,
      "verified_ms": verified_ms,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_artifact_locate(
  client: &SupervisorClient,
  ns: &str,
  sha256: &str,
  prefer_zone: Option<&str>,
  need_range: bool,
) -> Result<()> {
  let outputs = client.call(
    "artifact.locate",
    json!({
      "ns": ns,
      "sha256": sha256,
      "prefer_zone": prefer_zone,
      "need_range": need_range,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_artifact_stats(client: &SupervisorClient, ns: &str) -> Result<()> {
  let outputs = client.call(
    "artifact.stats",
    json!({
      "ns": ns,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_pki_cert_request(
  client: &SupervisorClient,
  ns: &str,
  cert_id: Option<&str>,
  kind: &str,
  subject: &str,
  san_dns_json: Option<&str>,
  san_uri_json: Option<&str>,
  issuer: Option<&str>,
  issuer_ref: Option<&str>,
  key_secret_ref: Option<&str>,
  cert_ref_id: Option<&str>,
  ttl_ms: Option<u64>,
) -> Result<()> {
  let san_dns_json = san_dns_json
    .map(|raw| serde_json::from_str::<Value>(raw).context("parse --san-dns-json JSON"))
    .transpose()?;
  let san_uri_json = san_uri_json
    .map(|raw| serde_json::from_str::<Value>(raw).context("parse --san-uri-json JSON"))
    .transpose()?;
  let outputs = client.call(
    "pki.cert.request",
    json!({
      "ns": ns,
      "cert_id": cert_id,
      "kind": kind,
      "subject": subject,
      "san_dns_json": san_dns_json,
      "san_uri_json": san_uri_json,
      "issuer": issuer,
      "issuer_ref": issuer_ref,
      "key_secret_ref": key_secret_ref,
      "cert_ref_id": cert_ref_id,
      "ttl_ms": ttl_ms,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_pki_cert_get(client: &SupervisorClient, ns: &str, cert_id: &str) -> Result<()> {
  let outputs = client.call(
    "pki.cert.get",
    json!({
      "ns": ns,
      "cert_id": cert_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_pki_cert_list(
  client: &SupervisorClient,
  ns: &str,
  status: Option<&str>,
  kind: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "pki.cert.list",
    json!({
      "ns": ns,
      "status": status,
      "kind": kind,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_pki_cert_revoke(
  client: &SupervisorClient,
  ns: &str,
  cert_id: &str,
  reason: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "pki.cert.revoke",
    json!({
      "ns": ns,
      "cert_id": cert_id,
      "reason": reason,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_gitops_source_upsert(
  client: &SupervisorClient,
  ns: &str,
  source_id: Option<&str>,
  name: &str,
  connector_id: &str,
  repo_url: &str,
  branch: Option<&str>,
  subdir: Option<&str>,
  mode: Option<&str>,
  policy_json: Option<&str>,
  status: Option<&str>,
) -> Result<()> {
  let policy_json = parse_json_arg(policy_json, "policy_json", json!({}))?;
  let outputs = client.call(
    "gitops.source.add",
    json!({
      "ns": ns,
      "source_id": source_id,
      "name": name,
      "connector_id": connector_id,
      "repo_url": repo_url,
      "branch": branch,
      "subdir": subdir,
      "mode": mode,
      "policy_json": policy_json,
      "status": status,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_gitops_source_get(client: &SupervisorClient, ns: &str, source_id: &str) -> Result<()> {
  let outputs = client.call(
    "gitops.source.get",
    json!({
      "ns": ns,
      "source_id": source_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_gitops_source_list(
  client: &SupervisorClient,
  ns: &str,
  status: Option<&str>,
  mode: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "gitops.source.list",
    json!({
      "ns": ns,
      "status": status,
      "mode": mode,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_gitops_source_disable(client: &SupervisorClient, ns: &str, source_id: &str) -> Result<()> {
  let outputs = client.call(
    "gitops.source.disable",
    json!({
      "ns": ns,
      "source_id": source_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_gitops_sync_now(
  client: &SupervisorClient,
  ns: &str,
  source_id: &str,
  commit_sha: Option<&str>,
  apply: bool,
) -> Result<()> {
  let outputs = client.call(
    "gitops.sync.now",
    json!({
      "ns": ns,
      "source_id": source_id,
      "commit_sha": commit_sha,
      "apply": apply,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_gitops_status(client: &SupervisorClient, ns: &str, source_id: &str) -> Result<()> {
  let outputs = client.call(
    "gitops.status",
    json!({
      "ns": ns,
      "source_id": source_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_admission_check(client: &SupervisorClient, file: &PathBuf) -> Result<()> {
  let raw = std::fs::read_to_string(file)
    .with_context(|| format!("read admission input {}", file.display()))?;
  let payload: Value =
    serde_json::from_str(&raw).with_context(|| format!("parse {}", file.display()))?;
  let outputs = client.call("admission.check", payload)?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_policy_eval_list(
  client: &SupervisorClient,
  ns: &str,
  channel: Option<&str>,
  target: Option<&str>,
  decision: Option<&str>,
  since_ms: Option<u128>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "policy.eval.list",
    json!({
      "ns": ns,
      "channel": channel,
      "target": target,
      "decision": decision,
      "since_ms": since_ms,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_keyring_add(
  client: &SupervisorClient,
  key_id: &str,
  public_key_b64: &str,
  scope_json: Option<&str>,
) -> Result<()> {
  let scope: Value = match scope_json {
    Some(raw) if !raw.trim().is_empty() => {
      serde_json::from_str(raw).context("parse --scope-json JSON")?
    }
    _ => json!({}),
  };
  let outputs = client.call(
    "keyring.add",
    json!({
      "key_id": key_id,
      "public_key_b64": public_key_b64,
      "scope_json": scope,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_keyring_revoke(client: &SupervisorClient, key_id: &str) -> Result<()> {
  let outputs = client.call(
    "keyring.revoke",
    json!({
      "key_id": key_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_keyring_list(client: &SupervisorClient) -> Result<()> {
  let outputs = client.call("keyring.list", json!({}))?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_attest_verify(
  client: &SupervisorClient,
  ns: &str,
  file: &PathBuf,
  att_id: Option<&str>,
  att_ref_id: Option<&str>,
  invocation_id: Option<&str>,
  session_id: Option<&str>,
  release_id: Option<&str>,
) -> Result<()> {
  let raw = std::fs::read_to_string(file)
    .with_context(|| format!("read attestation envelope {}", file.display()))?;
  let envelope: Value =
    serde_json::from_str(&raw).with_context(|| format!("parse {}", file.display()))?;
  let outputs = client.call(
    "attest.verify",
    json!({
      "ns": ns,
      "att_id": att_id,
      "att_ref_id": att_ref_id,
      "invocation_id": invocation_id,
      "session_id": session_id,
      "release_id": release_id,
      "envelope": envelope,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_attest_list(
  client: &SupervisorClient,
  ns: &str,
  subject_sha256: Option<&str>,
  attestation_type: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "attest.list",
    json!({
      "ns": ns,
      "subject_sha256": subject_sha256,
      "type": attestation_type,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_compliance_policy_get(client: &SupervisorClient, ns: &str, channel: &str) -> Result<()> {
  let outputs = client.call(
    "compliance.policy.get",
    json!({
      "ns": ns,
      "channel": channel,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_compliance_policy_set(
  client: &SupervisorClient,
  ns: &str,
  channel: &str,
  file: &PathBuf,
) -> Result<()> {
  let raw = std::fs::read_to_string(file)
    .with_context(|| format!("read compliance policy file {}", file.display()))?;
  let policy_json: Value =
    serde_json::from_str(&raw).with_context(|| format!("parse {}", file.display()))?;
  let outputs = client.call(
    "compliance.policy.set",
    json!({
      "ns": ns,
      "channel": channel,
      "policy_json": policy_json,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_compliance_exception_create(
  client: &SupervisorClient,
  ns: &str,
  channel: &str,
  target: &str,
  reason: &str,
  subject_id: Option<&str>,
  rule_json: Option<&str>,
  duration_ms: Option<u64>,
  approvals_required: Option<u32>,
) -> Result<()> {
  let rule = match rule_json {
    Some(raw) if !raw.trim().is_empty() => {
      serde_json::from_str(raw).context("parse --rule-json JSON")?
    }
    _ => json!({}),
  };
  let outputs = client.call(
    "compliance.exception.create",
    json!({
      "ns": ns,
      "channel": channel,
      "target": target,
      "subject_id": subject_id,
      "reason": reason,
      "rule_json": rule,
      "duration_ms": duration_ms,
      "approvals_required": approvals_required,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_compliance_exception_decision(
  client: &SupervisorClient,
  op: &str,
  ns: &str,
  exc_id: &str,
) -> Result<()> {
  let outputs = client.call(
    op,
    json!({
      "ns": ns,
      "exc_id": exc_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_compliance_exception_list(
  client: &SupervisorClient,
  ns: &str,
  channel: Option<&str>,
  target: Option<&str>,
  status: Option<&str>,
  subject_id: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "compliance.exception.list",
    json!({
      "ns": ns,
      "channel": channel,
      "target": target,
      "status": status,
      "subject_id": subject_id,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_compliance_scan(client: &SupervisorClient, ns: &str, channel: &str) -> Result<()> {
  let outputs = client.call(
    "compliance.scan",
    json!({
      "ns": ns,
      "channel": channel,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_compliance_report_latest(client: &SupervisorClient, ns: &str, channel: &str) -> Result<()> {
  let outputs = client.call(
    "compliance.report.latest",
    json!({
      "ns": ns,
      "channel": channel,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_runtime_catalog_upsert(
  client: &SupervisorClient,
  ns: &str,
  family: &str,
  version: &str,
  exec_ref_id: &str,
  sha256: &str,
  name: Option<&str>,
  platform_os: Option<&str>,
  platform_arch: Option<&str>,
  sbom_ref_id: Option<&str>,
  vuln_report_id: Option<&str>,
  attestation_ref_id: Option<&str>,
  status: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "runtime.catalog.upsert",
    json!({
      "ns": ns,
      "family": family,
      "version": version,
      "exec_ref_id": exec_ref_id,
      "sha256": sha256,
      "name": name,
      "platform_os": platform_os,
      "platform_arch": platform_arch,
      "sbom_ref_id": sbom_ref_id,
      "vuln_report_id": vuln_report_id,
      "attestation_ref_id": attestation_ref_id,
      "status": status,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_runtime_catalog_list(
  client: &SupervisorClient,
  ns: &str,
  family: Option<&str>,
  status: Option<&str>,
  platform_os: Option<&str>,
  platform_arch: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "runtime.catalog.list",
    json!({
      "ns": ns,
      "family": family,
      "status": status,
      "platform_os": platform_os,
      "platform_arch": platform_arch,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_runtime_alias_set(
  client: &SupervisorClient,
  ns: &str,
  family: &str,
  alias: &str,
  version: &str,
) -> Result<()> {
  let outputs = client.call(
    "runtime.alias.set",
    json!({
      "ns": ns,
      "family": family,
      "alias": alias,
      "version": version,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_runtime_alias_list(client: &SupervisorClient, ns: &str, family: Option<&str>) -> Result<()> {
  let outputs = client.call(
    "runtime.alias.list",
    json!({
      "ns": ns,
      "family": family,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_runtime_alias_resolve(
  client: &SupervisorClient,
  ns: &str,
  family: &str,
  alias: &str,
  platform_os: Option<&str>,
  platform_arch: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "runtime.alias.resolve",
    json!({
      "ns": ns,
      "family": family,
      "alias": alias,
      "platform_os": platform_os,
      "platform_arch": platform_arch,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn parse_csv_values(raw: &str) -> Vec<String> {
  raw
    .split(',')
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(str::to_string)
    .collect()
}

fn cmd_security_compose_verdict(
  client: &SupervisorClient,
  ns: &str,
  runtime_ref_id: &str,
  inputs_csv: &str,
  policy_id: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "security.compose_verdict",
    json!({
      "ns": ns,
      "runtime_ref_id": runtime_ref_id,
      "inputs": parse_csv_values(inputs_csv),
      "policy_id": policy_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_security_verdict_get(client: &SupervisorClient, ns: &str, verdict_id: &str) -> Result<()> {
  let outputs = client.call(
    "security.verdict.get",
    json!({
      "ns": ns,
      "verdict_id": verdict_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_security_verdict_lookup(
  client: &SupervisorClient,
  ns: &str,
  runtime_sha256: &str,
  app_sha256: &str,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "security.verdict.lookup",
    json!({
      "ns": ns,
      "runtime_sha256": runtime_sha256,
      "app_sha256": app_sha256,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_blueprint_bundle_publish(
  client: &SupervisorClient,
  ns: &str,
  name: &str,
  version: &str,
  bundle_ref_id: &str,
  sha256: &str,
  attestation_ref_id: Option<&str>,
  status: Option<&str>,
  manifest_file: Option<&PathBuf>,
) -> Result<()> {
  let manifest_json = if let Some(file) = manifest_file {
    let raw = std::fs::read_to_string(file)
      .with_context(|| format!("read blueprint manifest {}", file.display()))?;
    serde_json::from_str::<Value>(&raw).with_context(|| format!("parse {}", file.display()))?
  } else {
    json!({})
  };
  let outputs = client.call(
    "blueprint.bundle.publish",
    json!({
      "ns": ns,
      "name": name,
      "version": version,
      "bundle_ref_id": bundle_ref_id,
      "sha256": sha256,
      "attestation_ref_id": attestation_ref_id,
      "status": status,
      "manifest_json": manifest_json,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_blueprint_bundle_list(
  client: &SupervisorClient,
  ns: &str,
  name: Option<&str>,
  status: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "blueprint.bundle.list",
    json!({
      "ns": ns,
      "name": name,
      "status": status,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_blueprint_bundle_get(client: &SupervisorClient, ns: &str, bundle_id: &str) -> Result<()> {
  let outputs = client.call(
    "blueprint.bundle.get",
    json!({
      "ns": ns,
      "bundle_id": bundle_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_blueprint_bundle_approve(
  client: &SupervisorClient,
  ns: &str,
  bundle_id: &str,
) -> Result<()> {
  let outputs = client.call(
    "blueprint.bundle.approve",
    json!({
      "ns": ns,
      "bundle_id": bundle_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_blueprint_bundle_revoke(client: &SupervisorClient, ns: &str, bundle_id: &str) -> Result<()> {
  let outputs = client.call(
    "blueprint.bundle.revoke",
    json!({
      "ns": ns,
      "bundle_id": bundle_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_create(
  client: &SupervisorClient,
  ns: &str,
  service_name: &str,
  channel: &str,
  blueprint_name: &str,
  params_file: &PathBuf,
  overrides_file: Option<&PathBuf>,
) -> Result<()> {
  let params_raw = std::fs::read_to_string(params_file)
    .with_context(|| format!("read service params {}", params_file.display()))?;
  let params: Value = serde_json::from_str(&params_raw)
    .with_context(|| format!("parse {}", params_file.display()))?;
  let overrides = if let Some(file) = overrides_file {
    let raw = std::fs::read_to_string(file)
      .with_context(|| format!("read service overrides {}", file.display()))?;
    Some(serde_json::from_str::<Value>(&raw).with_context(|| format!("parse {}", file.display()))?)
  } else {
    None
  };
  let outputs = client.call(
    "service.create",
    json!({
      "ns": ns,
      "service_name": service_name,
      "channel": channel,
      "blueprint_name": blueprint_name,
      "params": params,
      "overrides": overrides,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_update_params(
  client: &SupervisorClient,
  ns: &str,
  service_name: &str,
  channel: &str,
  params_file: &PathBuf,
) -> Result<()> {
  let raw = std::fs::read_to_string(params_file)
    .with_context(|| format!("read service params {}", params_file.display()))?;
  let params: Value =
    serde_json::from_str(&raw).with_context(|| format!("parse {}", params_file.display()))?;
  let outputs = client.call(
    "service.update_params",
    json!({
      "ns": ns,
      "service_name": service_name,
      "channel": channel,
      "params": params,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_get(
  client: &SupervisorClient,
  ns: &str,
  service_name: &str,
  channel: &str,
) -> Result<()> {
  let outputs = client.call(
    "service.get",
    json!({
      "ns": ns,
      "service_name": service_name,
      "channel": channel,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_list(
  client: &SupervisorClient,
  ns: &str,
  channel: Option<&str>,
  blueprint_name: Option<&str>,
  status: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "service.list",
    json!({
      "ns": ns,
      "channel": channel,
      "blueprint_name": blueprint_name,
      "status": status,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_release_create(
  client: &SupervisorClient,
  ns: &str,
  service_name: &str,
  channel: &str,
  blueprint_bundle_id: &str,
  notes: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "service.release.create",
    json!({
      "ns": ns,
      "service_name": service_name,
      "channel": channel,
      "blueprint_bundle_id": blueprint_bundle_id,
      "notes": notes,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_release_approve(
  client: &SupervisorClient,
  ns: &str,
  release_id: &str,
) -> Result<()> {
  let outputs = client.call(
    "service.release.approve",
    json!({
      "ns": ns,
      "release_id": release_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_release_activate(
  client: &SupervisorClient,
  ns: &str,
  release_id: &str,
) -> Result<()> {
  let outputs = client.call(
    "service.release.activate",
    json!({
      "ns": ns,
      "release_id": release_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_release_rollback(
  client: &SupervisorClient,
  ns: &str,
  release_id: &str,
) -> Result<()> {
  let outputs = client.call(
    "service.release.rollback",
    json!({
      "ns": ns,
      "release_id": release_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_render_preview(
  client: &SupervisorClient,
  ns: &str,
  service_name: &str,
  channel: &str,
  blueprint_bundle_id: Option<&str>,
  params_file: Option<&PathBuf>,
  validate_only: bool,
  dry_run: bool,
) -> Result<()> {
  let params = if let Some(file) = params_file {
    let raw = std::fs::read_to_string(file)
      .with_context(|| format!("read service params {}", file.display()))?;
    Some(serde_json::from_str::<Value>(&raw).with_context(|| format!("parse {}", file.display()))?)
  } else {
    None
  };
  let outputs = client.call(
    "service.render.preview",
    json!({
      "ns": ns,
      "service_name": service_name,
      "channel": channel,
      "blueprint_bundle_id": blueprint_bundle_id,
      "params": params,
      "validate_only": validate_only,
      "dry_run": dry_run,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_render_get_latest(
  client: &SupervisorClient,
  ns: &str,
  service_name: &str,
  channel: &str,
) -> Result<()> {
  let outputs = client.call(
    "service.render.get_latest",
    json!({
      "ns": ns,
      "service_name": service_name,
      "channel": channel,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_node_hello(
  client: &SupervisorClient,
  node_id: &str,
  zone: Option<&str>,
  region: Option<&str>,
  host: Option<&str>,
  endpoint: Option<&str>,
  labels_json: Option<&str>,
  caps_csv: Option<&str>,
  capacity_json: Option<&str>,
  lease_ms: Option<u64>,
) -> Result<()> {
  let outputs = client.call(
    "node.hello",
    json!({
      "node_id": node_id,
      "zone": zone,
      "region": region,
      "host": host,
      "endpoint": endpoint,
      "labels_json": parse_json_arg(labels_json, "labels_json", json!({}))?,
      "caps": caps_csv.map(parse_csv_values),
      "capacity_json": parse_json_arg(capacity_json, "capacity_json", json!({}))?,
      "lease_ms": lease_ms,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_node_heartbeat(
  client: &SupervisorClient,
  node_id: &str,
  status: Option<&str>,
  zone: Option<&str>,
  region: Option<&str>,
  host: Option<&str>,
  endpoint: Option<&str>,
  labels_json: Option<&str>,
  caps_csv: Option<&str>,
  capacity_json: Option<&str>,
  lease_ms: Option<u64>,
) -> Result<()> {
  let outputs = client.call(
    "node.heartbeat",
    json!({
      "node_id": node_id,
      "status": status,
      "zone": zone,
      "region": region,
      "host": host,
      "endpoint": endpoint,
      "labels_json": labels_json.map(|value| parse_json_arg(Some(value), "labels_json", json!({}))).transpose()?,
      "caps": caps_csv.map(parse_csv_values),
      "capacity_json": capacity_json.map(|value| parse_json_arg(Some(value), "capacity_json", json!({}))).transpose()?,
      "lease_ms": lease_ms,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_node_set_status(client: &SupervisorClient, node_id: &str, status: &str) -> Result<()> {
  let outputs = client.call(
    "node.set_status",
    json!({
      "node_id": node_id,
      "status": status,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_node_list(
  client: &SupervisorClient,
  status: Option<&str>,
  zone: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "node.list",
    json!({
      "status": status,
      "zone": zone,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_replica_assignment_upsert(
  client: &SupervisorClient,
  ns: &str,
  rs_id: &str,
  replica_ordinal: u32,
  process_id: &str,
  assigned_node: Option<&str>,
  status: Option<&str>,
  service_name: Option<&str>,
  channel: Option<&str>,
  release_id: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "replica.assignment.upsert",
    json!({
      "ns": ns,
      "rs_id": rs_id,
      "replica_ordinal": replica_ordinal,
      "process_id": process_id,
      "assigned_node": assigned_node,
      "status": status,
      "service_name": service_name,
      "channel": channel,
      "release_id": release_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_replica_assignment_list(
  client: &SupervisorClient,
  ns: &str,
  rs_id: Option<&str>,
  assigned_node: Option<&str>,
  status: Option<&str>,
  service_name: Option<&str>,
  channel: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "replica.assignment.list",
    json!({
      "ns": ns,
      "rs_id": rs_id,
      "assigned_node": assigned_node,
      "status": status,
      "service_name": service_name,
      "channel": channel,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_endpoint_upsert(
  client: &SupervisorClient,
  ns: &str,
  service_name: &str,
  channel: &str,
  process_id: &str,
  node_id: &str,
  host: &str,
  port: u16,
  status: Option<&str>,
  release_id: Option<&str>,
  rs_id: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "service.endpoint.upsert",
    json!({
      "ns": ns,
      "service_name": service_name,
      "channel": channel,
      "process_id": process_id,
      "node_id": node_id,
      "host": host,
      "port": port,
      "status": status,
      "release_id": release_id,
      "rs_id": rs_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_service_endpoints_get(
  client: &SupervisorClient,
  ns: &str,
  service_name: &str,
  channel: &str,
  status: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "service.endpoints.get",
    json!({
      "ns": ns,
      "service_name": service_name,
      "channel": channel,
      "status": status,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn parse_json_arg(raw: Option<&str>, label: &str, default: Value) -> Result<Value> {
  match raw {
    Some(text) if !text.trim().is_empty() => {
      serde_json::from_str(text).with_context(|| format!("parse {} JSON", label))
    }
    _ => Ok(default),
  }
}

fn cmd_tenant_create(
  client: &SupervisorClient,
  name: &str,
  tenant_id: Option<&str>,
  owner_json: Option<&str>,
  status: Option<&str>,
) -> Result<()> {
  let owner = parse_json_arg(owner_json, "owner_json", json!({}))?;
  let outputs = client.call(
    "tenant.create",
    json!({
      "name": name,
      "tenant_id": tenant_id,
      "owner_json": owner,
      "status": status,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_tenant_get(client: &SupervisorClient, tenant_id: &str) -> Result<()> {
  let outputs = client.call(
    "tenant.get",
    json!({
      "tenant_id": tenant_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_tenant_list(
  client: &SupervisorClient,
  status: Option<&str>,
  name: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "tenant.list",
    json!({
      "status": status,
      "name": name,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_tenant_update(
  client: &SupervisorClient,
  tenant_id: &str,
  name: Option<&str>,
  owner_json: Option<&str>,
  status: Option<&str>,
) -> Result<()> {
  let mut payload = json!({
    "tenant_id": tenant_id,
    "name": name,
    "status": status,
  });
  if let Some(raw) = owner_json {
    let owner = parse_json_arg(Some(raw), "owner_json", json!({}))?;
    if let Some(obj) = payload.as_object_mut() {
      obj.insert("owner_json".to_string(), owner);
    }
  }
  let outputs = client.call("tenant.update", payload)?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_tenant_suspend(client: &SupervisorClient, tenant_id: &str) -> Result<()> {
  let outputs = client.call(
    "tenant.suspend",
    json!({
      "tenant_id": tenant_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_tenant_attach_namespace(client: &SupervisorClient, tenant_id: &str, ns: &str) -> Result<()> {
  let outputs = client.call(
    "tenant.attach_namespace",
    json!({
      "tenant_id": tenant_id,
      "ns": ns,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_tenant_detach_namespace(client: &SupervisorClient, tenant_id: &str, ns: &str) -> Result<()> {
  let outputs = client.call(
    "tenant.detach_namespace",
    json!({
      "tenant_id": tenant_id,
      "ns": ns,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_tenant_namespaces_list(
  client: &SupervisorClient,
  tenant_id: &str,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "tenant.namespaces.list",
    json!({
      "tenant_id": tenant_id,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_quota_set(
  client: &SupervisorClient,
  tenant_id: &str,
  policy_id: Option<&str>,
  scope_json: Option<&str>,
  limits_json: Option<&str>,
) -> Result<()> {
  let scope = parse_json_arg(scope_json, "scope_json", json!({}))?;
  let limits = parse_json_arg(limits_json, "limits_json", json!({}))?;
  let outputs = client.call(
    "quota.set",
    json!({
      "tenant_id": tenant_id,
      "policy_id": policy_id,
      "scope_json": scope,
      "limits_json": limits,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_quota_get(
  client: &SupervisorClient,
  tenant_id: &str,
  policy_id: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "quota.get",
    json!({
      "tenant_id": tenant_id,
      "policy_id": policy_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_budget_set(
  client: &SupervisorClient,
  tenant_id: &str,
  policy_id: Option<&str>,
  period: Option<&str>,
  currency: Option<&str>,
  budget_minor: i64,
  thresholds_json: Option<&str>,
  actions_json: Option<&str>,
) -> Result<()> {
  let thresholds = parse_json_arg(thresholds_json, "thresholds_json", json!({}))?;
  let actions = parse_json_arg(actions_json, "actions_json", json!({}))?;
  let outputs = client.call(
    "budget.set",
    json!({
      "tenant_id": tenant_id,
      "policy_id": policy_id,
      "period": period,
      "currency": currency,
      "budget_minor": budget_minor,
      "thresholds_json": thresholds,
      "actions_json": actions,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_budget_get(
  client: &SupervisorClient,
  tenant_id: &str,
  policy_id: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "budget.get",
    json!({
      "tenant_id": tenant_id,
      "policy_id": policy_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_budget_state_upsert(
  client: &SupervisorClient,
  tenant_id: &str,
  policy_id: Option<&str>,
  period_key: &str,
  spent_minor: i64,
  budget_minor: i64,
  level: Option<&str>,
  last_enforcement_change_id: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "budget.state.upsert",
    json!({
      "tenant_id": tenant_id,
      "policy_id": policy_id,
      "period_key": period_key,
      "spent_minor": spent_minor,
      "budget_minor": budget_minor,
      "level": level,
      "last_enforcement_change_id": last_enforcement_change_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_budget_state_get(
  client: &SupervisorClient,
  tenant_id: &str,
  policy_id: Option<&str>,
  period_key: &str,
) -> Result<()> {
  let outputs = client.call(
    "budget.state.get",
    json!({
      "tenant_id": tenant_id,
      "policy_id": policy_id,
      "period_key": period_key,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_usage_rollup_upsert(
  client: &SupervisorClient,
  tenant_id: &str,
  ns: &str,
  ts_ms: u128,
  region: Option<&str>,
  cpu_millis_avg: u64,
  cpu_millis_hour: u64,
  mem_bytes_avg: u64,
  mem_bytes_hour: u64,
  proc_count_avg: u64,
  artifact_bytes_avg: Option<u64>,
  egress_bytes: Option<u64>,
  gateway_req_count: Option<u64>,
  gateway_err5xx_count: Option<u64>,
) -> Result<()> {
  let outputs = client.call(
    "usage.rollup.upsert",
    json!({
      "tenant_id": tenant_id,
      "ns": ns,
      "ts_ms": ts_ms,
      "region": region,
      "cpu_millis_avg": cpu_millis_avg,
      "cpu_millis_hour": cpu_millis_hour,
      "mem_bytes_avg": mem_bytes_avg,
      "mem_bytes_hour": mem_bytes_hour,
      "proc_count_avg": proc_count_avg,
      "artifact_bytes_avg": artifact_bytes_avg,
      "egress_bytes": egress_bytes,
      "gateway_req_count": gateway_req_count,
      "gateway_err5xx_count": gateway_err5xx_count,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_usage_query(
  client: &SupervisorClient,
  tenant_id: &str,
  ns: Option<&str>,
  from_ts_ms: Option<u128>,
  to_ts_ms: Option<u128>,
  region: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "usage.query",
    json!({
      "tenant_id": tenant_id,
      "ns": ns,
      "from_ts_ms": from_ts_ms,
      "to_ts_ms": to_ts_ms,
      "region": region,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_cost_rollup_upsert(
  client: &SupervisorClient,
  tenant_id: &str,
  ts_ms: u128,
  currency: Option<&str>,
  cost_minor: i64,
  rate_card_id: &str,
  breakdown_json: Option<&str>,
) -> Result<()> {
  let breakdown = parse_json_arg(breakdown_json, "breakdown_json", json!({}))?;
  let outputs = client.call(
    "cost.rollup.upsert",
    json!({
      "tenant_id": tenant_id,
      "ts_ms": ts_ms,
      "currency": currency,
      "cost_minor": cost_minor,
      "rate_card_id": rate_card_id,
      "breakdown_json": breakdown,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_cost_query(
  client: &SupervisorClient,
  tenant_id: &str,
  from_ts_ms: Option<u128>,
  to_ts_ms: Option<u128>,
  rate_card_id: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "cost.query",
    json!({
      "tenant_id": tenant_id,
      "from_ts_ms": from_ts_ms,
      "to_ts_ms": to_ts_ms,
      "rate_card_id": rate_card_id,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_rate_card_create(
  client: &SupervisorClient,
  name: &str,
  card_id: Option<&str>,
  currency: Option<&str>,
  rates_json: Option<&str>,
  valid_from_ms: u128,
  valid_to_ms: Option<u128>,
  status: Option<&str>,
) -> Result<()> {
  let rates = parse_json_arg(rates_json, "rates_json", json!({}))?;
  let outputs = client.call(
    "rate_card.create",
    json!({
      "name": name,
      "card_id": card_id,
      "currency": currency,
      "rates_json": rates,
      "valid_from_ms": valid_from_ms,
      "valid_to_ms": valid_to_ms,
      "status": status,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_rate_card_activate(client: &SupervisorClient, card_id: &str) -> Result<()> {
  let outputs = client.call(
    "rate_card.activate",
    json!({
      "card_id": card_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_rate_card_get(client: &SupervisorClient, card_id: &str) -> Result<()> {
  let outputs = client.call(
    "rate_card.get",
    json!({
      "card_id": card_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_rate_card_list(
  client: &SupervisorClient,
  status: Option<&str>,
  currency: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "rate_card.list",
    json!({
      "status": status,
      "currency": currency,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_invoice_generate(
  client: &SupervisorClient,
  tenant_id: &str,
  period: &str,
  currency: Option<&str>,
  from_ts_ms: Option<u128>,
  to_ts_ms: Option<u128>,
  rate_card_id: Option<&str>,
  invoice_id: Option<&str>,
  report_ref_id: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "invoice.generate",
    json!({
      "tenant_id": tenant_id,
      "period": period,
      "currency": currency,
      "from_ts_ms": from_ts_ms,
      "to_ts_ms": to_ts_ms,
      "rate_card_id": rate_card_id,
      "invoice_id": invoice_id,
      "report_ref_id": report_ref_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_invoice_get(client: &SupervisorClient, tenant_id: &str, invoice_id: &str) -> Result<()> {
  let outputs = client.call(
    "invoice.get",
    json!({
      "tenant_id": tenant_id,
      "invoice_id": invoice_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_invoice_list(
  client: &SupervisorClient,
  tenant_id: &str,
  period: Option<&str>,
  status: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "invoice.list",
    json!({
      "tenant_id": tenant_id,
      "period": period,
      "status": status,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_forecast_profile_set(
  client: &SupervisorClient,
  ns: &str,
  profile_id: Option<&str>,
  scope_kind: &str,
  scope_id: &str,
  channel: Option<&str>,
  model_kind: Option<&str>,
  horizon_ms: Option<u64>,
  step_ms: Option<u64>,
  features_json: Option<&str>,
  guardrails_json: Option<&str>,
) -> Result<()> {
  let features = parse_json_arg(features_json, "features_json", json!({}))?;
  let guardrails = parse_json_arg(guardrails_json, "guardrails_json", json!({}))?;
  let outputs = client.call(
    "forecast.profile.set",
    json!({
      "ns": ns,
      "profile_id": profile_id,
      "scope_kind": scope_kind,
      "scope_id": scope_id,
      "channel": channel,
      "model_kind": model_kind,
      "horizon_ms": horizon_ms,
      "step_ms": step_ms,
      "features_json": features,
      "guardrails_json": guardrails,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_forecast_profile_get(client: &SupervisorClient, ns: &str, profile_id: &str) -> Result<()> {
  let outputs = client.call(
    "forecast.profile.get",
    json!({
      "ns": ns,
      "profile_id": profile_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_forecast_profile_list(
  client: &SupervisorClient,
  ns: &str,
  scope_kind: Option<&str>,
  scope_id: Option<&str>,
  channel: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "forecast.profile.list",
    json!({
      "ns": ns,
      "scope_kind": scope_kind,
      "scope_id": scope_id,
      "channel": channel,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_forecast_run_latest(
  client: &SupervisorClient,
  ns: &str,
  scope_kind: &str,
  scope_id: &str,
  channel: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "forecast.run.latest",
    json!({
      "ns": ns,
      "scope_kind": scope_kind,
      "scope_id": scope_id,
      "channel": channel,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_predictive_run_now(
  client: &SupervisorClient,
  ns: &str,
  scope_kind: &str,
  scope_id: &str,
  channel: Option<&str>,
  profile_id: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "predictive.run.now",
    json!({
      "ns": ns,
      "scope_kind": scope_kind,
      "scope_id": scope_id,
      "channel": channel,
      "profile_id": profile_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_predictive_plan_get(client: &SupervisorClient, ns: &str, plan_id: &str) -> Result<()> {
  let outputs = client.call(
    "predictive.plan.get",
    json!({
      "ns": ns,
      "plan_id": plan_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_predictive_plan_list(
  client: &SupervisorClient,
  ns: &str,
  scope_kind: Option<&str>,
  scope_id: Option<&str>,
  channel: Option<&str>,
  status: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "predictive.plan.list",
    json!({
      "ns": ns,
      "scope_kind": scope_kind,
      "scope_id": scope_id,
      "channel": channel,
      "status": status,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_simulation_run(
  client: &SupervisorClient,
  ns: &str,
  source_kind: &str,
  source_id: &str,
  base_ref_json: Option<&str>,
  proposed_ref_json: Option<&str>,
  env_snapshot_ref_id: Option<&str>,
) -> Result<()> {
  let base_ref = parse_json_arg(base_ref_json, "base_ref_json", json!({}))?;
  let proposed_ref = parse_json_arg(proposed_ref_json, "proposed_ref_json", json!({}))?;
  let outputs = client.call(
    "simulation.run",
    json!({
      "ns": ns,
      "source_kind": source_kind,
      "source_id": source_id,
      "base_ref_json": base_ref,
      "proposed_ref_json": proposed_ref,
      "env_snapshot_ref_id": env_snapshot_ref_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_simulation_get_latest(
  client: &SupervisorClient,
  ns: &str,
  source_kind: &str,
  source_id: &str,
) -> Result<()> {
  let outputs = client.call(
    "simulation.get_latest",
    json!({
      "ns": ns,
      "source_kind": source_kind,
      "source_id": source_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_alert_rule_set(
  client: &SupervisorClient,
  ns: &str,
  rule_id: Option<&str>,
  name: &str,
  source_kind: &str,
  selector_json: Option<&str>,
  window_ms: Option<u64>,
  eval_every_ms: Option<u64>,
  condition_json: Option<&str>,
  severity: Option<&str>,
  dedupe_key_template: Option<&str>,
  labels_json: Option<&str>,
  autopilot_json: Option<&str>,
  status: Option<&str>,
) -> Result<()> {
  let selector = parse_json_arg(selector_json, "selector_json", json!({}))?;
  let condition = parse_json_arg(condition_json, "condition_json", json!({}))?;
  let labels = parse_json_arg(labels_json, "labels_json", json!({}))?;
  let autopilot = parse_json_arg(autopilot_json, "autopilot_json", json!({}))?;
  let outputs = client.call(
    "alert.rule.set",
    json!({
      "ns": ns,
      "rule_id": rule_id,
      "name": name,
      "source_kind": source_kind,
      "selector_json": selector,
      "window_ms": window_ms,
      "eval_every_ms": eval_every_ms,
      "condition_json": condition,
      "severity": severity,
      "dedupe_key_template": dedupe_key_template,
      "labels_json": labels,
      "autopilot_json": autopilot,
      "status": status,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_alert_rule_get(client: &SupervisorClient, ns: &str, rule_id: &str) -> Result<()> {
  let outputs = client.call(
    "alert.rule.get",
    json!({
      "ns": ns,
      "rule_id": rule_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_alert_rule_list(
  client: &SupervisorClient,
  ns: &str,
  source_kind: Option<&str>,
  severity: Option<&str>,
  status: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "alert.rule.list",
    json!({
      "ns": ns,
      "source_kind": source_kind,
      "severity": severity,
      "status": status,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_alert_rule_disable(client: &SupervisorClient, ns: &str, rule_id: &str) -> Result<()> {
  let outputs = client.call(
    "alert.rule.disable",
    json!({
      "ns": ns,
      "rule_id": rule_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_alert_evaluate(
  client: &SupervisorClient,
  ns: &str,
  rule_id: &str,
  dedupe_key: Option<&str>,
  value_json: Option<&str>,
) -> Result<()> {
  let value = if let Some(raw) = value_json {
    parse_json_arg(Some(raw), "value_json", json!({}))?
  } else {
    Value::Null
  };
  let outputs = client.call(
    "alert.evaluate",
    json!({
      "ns": ns,
      "rule_id": rule_id,
      "dedupe_key": dedupe_key,
      "value_json": value,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_incident_get(client: &SupervisorClient, ns: &str, incident_id: &str) -> Result<()> {
  let outputs = client.call(
    "incident.get",
    json!({
      "ns": ns,
      "incident_id": incident_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_incident_list(
  client: &SupervisorClient,
  ns: &str,
  status: Option<&str>,
  severity: Option<&str>,
  primary_object_kind: Option<&str>,
  primary_object_id: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "incident.list",
    json!({
      "ns": ns,
      "status": status,
      "severity": severity,
      "primary_object_kind": primary_object_kind,
      "primary_object_id": primary_object_id,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_incident_ack(
  client: &SupervisorClient,
  ns: &str,
  incident_id: &str,
  owner_json: Option<&str>,
) -> Result<()> {
  let owner = if let Some(raw) = owner_json {
    parse_json_arg(Some(raw), "owner_json", json!({}))?
  } else {
    Value::Null
  };
  let outputs = client.call(
    "incident.ack",
    json!({
      "ns": ns,
      "incident_id": incident_id,
      "owner_json": owner,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_incident_note(
  client: &SupervisorClient,
  ns: &str,
  incident_id: &str,
  message: &str,
  meta_json: Option<&str>,
) -> Result<()> {
  let meta = if let Some(raw) = meta_json {
    parse_json_arg(Some(raw), "meta_json", json!({}))?
  } else {
    Value::Null
  };
  let outputs = client.call(
    "incident.note",
    json!({
      "ns": ns,
      "incident_id": incident_id,
      "message": message,
      "meta_json": meta,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_incident_resolve(
  client: &SupervisorClient,
  ns: &str,
  incident_id: &str,
  summary: Option<&str>,
  root_cause: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "incident.resolve",
    json!({
      "ns": ns,
      "incident_id": incident_id,
      "summary": summary,
      "root_cause": root_cause,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_runbook_set(
  client: &SupervisorClient,
  ns: &str,
  runbook_id: Option<&str>,
  name: &str,
  version: Option<&str>,
  bundle_ref_id: Option<&str>,
  sha256: Option<&str>,
  policy_json: Option<&str>,
  steps_json: Option<&str>,
  status: Option<&str>,
) -> Result<()> {
  let policy = parse_json_arg(policy_json, "policy_json", json!({}))?;
  let steps = parse_json_arg(steps_json, "steps_json", json!([]))?;
  let outputs = client.call(
    "runbook.set",
    json!({
      "ns": ns,
      "runbook_id": runbook_id,
      "name": name,
      "version": version,
      "bundle_ref_id": bundle_ref_id,
      "sha256": sha256,
      "policy_json": policy,
      "steps": steps,
      "status": status,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_runbook_get(client: &SupervisorClient, ns: &str, name: &str) -> Result<()> {
  let outputs = client.call(
    "runbook.get",
    json!({
      "ns": ns,
      "name": name,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_runbook_list(
  client: &SupervisorClient,
  ns: &str,
  status: Option<&str>,
  name: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "runbook.list",
    json!({
      "ns": ns,
      "status": status,
      "name": name,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_runbook_execute(
  client: &SupervisorClient,
  ns: &str,
  incident_id: &str,
  runbook_name: &str,
  mode: Option<&str>,
  auto_run: Option<bool>,
) -> Result<()> {
  let outputs = client.call(
    "runbook.execute",
    json!({
      "ns": ns,
      "incident_id": incident_id,
      "runbook_name": runbook_name,
      "mode": mode,
      "auto_run": auto_run,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_postmortem_generate(
  client: &SupervisorClient,
  ns: &str,
  incident_id: &str,
  pm_id: Option<&str>,
  report_ref_id: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    "postmortem.generate",
    json!({
      "ns": ns,
      "incident_id": incident_id,
      "pm_id": pm_id,
      "report_ref_id": report_ref_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_postmortem_get(client: &SupervisorClient, ns: &str, incident_id: &str) -> Result<()> {
  let outputs = client.call(
    "postmortem.get",
    json!({
      "ns": ns,
      "incident_id": incident_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_chaos_experiment_publish(
  client: &SupervisorClient,
  ns: &str,
  exp_id: Option<&str>,
  name: &str,
  bundle_ref_id: &str,
  sha256: &str,
  spec_json: Option<&str>,
  status: Option<&str>,
) -> Result<()> {
  let spec = parse_json_arg(spec_json, "spec_json", json!({}))?;
  let outputs = client.call(
    "chaos.experiment.publish",
    json!({
      "ns": ns,
      "exp_id": exp_id,
      "name": name,
      "bundle_ref_id": bundle_ref_id,
      "sha256": sha256,
      "spec_json": spec,
      "status": status,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_chaos_experiment_get(client: &SupervisorClient, ns: &str, exp_id: &str) -> Result<()> {
  let outputs = client.call(
    "chaos.experiment.get",
    json!({
      "ns": ns,
      "exp_id": exp_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_chaos_experiment_list(
  client: &SupervisorClient,
  ns: &str,
  status: Option<&str>,
  name: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "chaos.experiment.list",
    json!({
      "ns": ns,
      "status": status,
      "name": name,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_chaos_experiment_approve(client: &SupervisorClient, ns: &str, exp_id: &str) -> Result<()> {
  let outputs = client.call(
    "chaos.experiment.approve",
    json!({
      "ns": ns,
      "exp_id": exp_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_chaos_experiment_disable(client: &SupervisorClient, ns: &str, exp_id: &str) -> Result<()> {
  let outputs = client.call(
    "chaos.experiment.disable",
    json!({
      "ns": ns,
      "exp_id": exp_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_chaos_run_start(client: &SupervisorClient, ns: &str, exp_id: &str) -> Result<()> {
  let outputs = client.call(
    "chaos.run.start",
    json!({
      "ns": ns,
      "exp_id": exp_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_chaos_run_abort(client: &SupervisorClient, ns: &str, run_id: &str) -> Result<()> {
  let outputs = client.call(
    "chaos.run.abort",
    json!({
      "ns": ns,
      "run_id": run_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_chaos_run_get(client: &SupervisorClient, ns: &str, run_id: &str) -> Result<()> {
  let outputs = client.call(
    "chaos.run.get",
    json!({
      "ns": ns,
      "run_id": run_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_chaos_run_latest(client: &SupervisorClient, ns: &str, exp_id: &str) -> Result<()> {
  let outputs = client.call(
    "chaos.run.latest",
    json!({
      "ns": ns,
      "exp_id": exp_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_remediation_plan_create(
  client: &SupervisorClient,
  ns: &str,
  file: Option<&PathBuf>,
  risk: &str,
  summary: Option<&str>,
  trigger_json: Option<&str>,
  steps_json: Option<&str>,
  invocation_id: Option<&str>,
  session_id: Option<&str>,
) -> Result<()> {
  let payload = if let Some(path) = file {
    let raw = std::fs::read_to_string(path)
      .with_context(|| format!("read remediation plan payload {}", path.display()))?;
    let mut value: Value =
      serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    if let Some(obj) = value.as_object_mut() {
      obj.entry("ns".to_string()).or_insert_with(|| json!(ns));
    }
    value
  } else {
    json!({
      "ns": ns,
      "risk": risk,
      "summary": summary,
      "trigger": parse_json_arg(trigger_json, "trigger", json!({}))?,
      "steps": parse_json_arg(steps_json, "steps", json!([]))?,
      "invocation_id": invocation_id,
      "session_id": session_id,
    })
  };
  let outputs = client.call("remediation.plan.create", payload)?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_remediation_plan_get(client: &SupervisorClient, ns: &str, plan_id: &str) -> Result<()> {
  let outputs = client.call(
    "remediation.plan.get",
    json!({
      "ns": ns,
      "plan_id": plan_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_remediation_plan_list(
  client: &SupervisorClient,
  ns: &str,
  status: Option<&str>,
  risk: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "remediation.plan.list",
    json!({
      "ns": ns,
      "status": status,
      "risk": risk,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_remediation_plan_escalate(
  client: &SupervisorClient,
  ns: &str,
  plan_id: &str,
  title: Option<&str>,
  reason: Option<&str>,
  source: Option<&str>,
  risk: Option<&str>,
  auto_run: bool,
) -> Result<()> {
  let outputs = client.call(
    "remediation.plan.escalate",
    json!({
      "ns": ns,
      "plan_id": plan_id,
      "title": title,
      "reason": reason,
      "source": source,
      "risk": risk,
      "auto_run": auto_run,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_change_get(client: &SupervisorClient, ns: &str, change_id: &str) -> Result<()> {
  let outputs = client.call(
    "change.get",
    json!({
      "ns": ns,
      "change_id": change_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_change_list(
  client: &SupervisorClient,
  ns: &str,
  status: Option<&str>,
  risk: Option<&str>,
  limit: usize,
) -> Result<()> {
  let outputs = client.call(
    "change.list",
    json!({
      "ns": ns,
      "status": status,
      "risk": risk,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_change_decision(
  client: &SupervisorClient,
  op: &str,
  ns: &str,
  change_id: &str,
  note: Option<&str>,
) -> Result<()> {
  let outputs = client.call(
    op,
    json!({
      "ns": ns,
      "change_id": change_id,
      "note": note,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_change_run(client: &SupervisorClient, ns: &str, change_id: &str) -> Result<()> {
  let outputs = client.call(
    "change.run",
    json!({
      "ns": ns,
      "change_id": change_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_change_cancel(client: &SupervisorClient, ns: &str, change_id: &str) -> Result<()> {
  let outputs = client.call(
    "change.cancel",
    json!({
      "ns": ns,
      "change_id": change_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_job_submit(
  client: &SupervisorClient,
  ns: &str,
  kind: &str,
  params: Option<&str>,
  dedupe_key: Option<&str>,
  priority: u32,
  invocation_id: Option<&str>,
  session_id: Option<&str>,
  max_attempt: u32,
) -> Result<()> {
  let params_json: Value = match params {
    Some(raw) if !raw.trim().is_empty() => {
      serde_json::from_str(raw).context("parse --params JSON")?
    }
    _ => json!({}),
  };
  let outputs = client.call(
    "job.submit",
    json!({
      "ns": ns,
      "kind": kind,
      "params": params_json,
      "dedupe_key": dedupe_key,
      "priority": priority,
      "invocation_id": invocation_id,
      "session_id": session_id,
      "max_attempt": max_attempt,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_job_get(client: &SupervisorClient, ns: &str, job_id: &str) -> Result<()> {
  let outputs = client.call(
    "job.get",
    json!({
      "ns": ns,
      "job_id": job_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_job_list(
  client: &SupervisorClient,
  ns: &str,
  state: Option<&str>,
  kind_prefix: Option<&str>,
  invocation_id: Option<&str>,
  session_id: Option<&str>,
  limit: usize,
) -> Result<()> {
  let state_values = state.map(|raw| {
    raw
      .split(',')
      .map(str::trim)
      .filter(|value| !value.is_empty())
      .map(str::to_string)
      .collect::<Vec<_>>()
  });
  let outputs = client.call(
    "job.list",
    json!({
      "ns": ns,
      "state": state_values,
      "kind_prefix": kind_prefix,
      "invocation_id": invocation_id,
      "session_id": session_id,
      "limit": limit,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_job_cancel(client: &SupervisorClient, ns: &str, job_id: &str) -> Result<()> {
  let outputs = client.call(
    "job.cancel",
    json!({
      "ns": ns,
      "job_id": job_id,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_job_logs(
  client: &SupervisorClient,
  ns: &str,
  job_id: &str,
  n: usize,
  max_bytes: usize,
) -> Result<()> {
  let outputs = client.call(
    "job.logs.tail",
    json!({
      "ns": ns,
      "job_id": job_id,
      "n": n,
      "max_bytes": max_bytes,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_job_wait(client: &SupervisorClient, ns: &str, job_id: &str, timeout_ms: u64) -> Result<()> {
  let outputs = client.call(
    "job.wait",
    json!({
      "ns": ns,
      "job_id": job_id,
      "timeout_ms": timeout_ms,
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_install_systemd(root: &PathBuf, source_root: Option<&PathBuf>, dry_run: bool) -> Result<()> {
  let source_root = match source_root {
    Some(path) => path.clone(),
    None => std::env::current_dir().context("resolve current directory for source root")?,
  };
  let unit_src = source_root.join("packaging/systemd/pnix-supervisor.service");
  let tokens_src = source_root.join("config/templates/tokens.json");
  let backends_src = source_root.join("config/templates/backends.json");
  let bootstrap_src = source_root.join("config/templates/bootstrap.desired.json");

  let unit_dst = root.join("etc/systemd/system/pnix-supervisor.service");
  let etc_pnix = root.join("etc/pnix");
  let tokens_dst = etc_pnix.join("tokens.json");
  let backends_dst = etc_pnix.join("backends.json");
  let bootstrap_dst = etc_pnix.join("bootstrap.desired.json");

  let operations = vec![
    (unit_src, unit_dst),
    (tokens_src, tokens_dst),
    (backends_src, backends_dst),
    (bootstrap_src, bootstrap_dst),
  ];

  if dry_run {
    println!(
      "{}",
      serde_json::to_string_pretty(&json!({
        "ok": true,
        "dry_run": true,
        "source_root": source_root,
        "root": root,
        "operations": operations.iter().map(|(src,dst)| {
          json!({ "copy": src, "to": dst })
        }).collect::<Vec<_>>()
      }))?
    );
    return Ok(());
  }

  std::fs::create_dir_all(root.join("etc/systemd/system"))
    .with_context(|| format!("create {}", root.join("etc/systemd/system").display()))?;
  std::fs::create_dir_all(&etc_pnix).with_context(|| format!("create {}", etc_pnix.display()))?;

  for (src, dst) in operations {
    if !src.exists() {
      anyhow::bail!("missing source template: {}", src.display());
    }
    std::fs::copy(&src, &dst)
      .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
  }

  println!(
    "{}",
    serde_json::to_string_pretty(&json!({
      "ok": true,
      "installed": true,
      "root": root,
      "unit": root.join("etc/systemd/system/pnix-supervisor.service"),
      "config_dir": root.join("etc/pnix"),
      "next_steps": [
        "edit /etc/pnix/tokens.json and replace token placeholders",
        "systemctl daemon-reload",
        "systemctl enable --now pnix-supervisor",
        "pnixctl doctor --endpoint uds:/run/pnix/supervisor.sock --ns system"
      ]
    }))?
  );
  Ok(())
}

fn cmd_status(client: &SupervisorClient, ns: &str, limit: usize) -> Result<()> {
  let outputs = client.call(
    "process.query",
    json!({
      "ns": ns,
      "limit": limit,
      "order_by": [{"field":"id","dir":"asc"}],
      "select": [
        "ns","id","status","pid","generation","base_url",
        "rss_bytes","threads_count","fd_count","cpu_pct",
        "desired_present","fail_count","paused","last_error"
      ]
    }),
  )?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}

fn cmd_desired_apply(client: &SupervisorClient, file: &PathBuf, ns: Option<&str>) -> Result<()> {
  let raw = std::fs::read_to_string(file)
    .with_context(|| format!("read desired file {}", file.display()))?;
  let mut payload: Value =
    serde_json::from_str(&raw).with_context(|| format!("parse {}", file.display()))?;
  if let Some(ns) = ns {
    if let Some(obj) = payload.as_object_mut() {
      obj.insert("ns".to_string(), json!(ns));
    }
  }
  let outputs = client.call("desired.apply", payload)?;
  println!("{}", serde_json::to_string_pretty(&outputs)?);
  Ok(())
}
