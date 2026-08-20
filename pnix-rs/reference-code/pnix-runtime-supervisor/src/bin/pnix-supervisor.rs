use clap::{ArgAction, Parser};
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Args {
  /// UDS socket path
  #[arg(long, default_value = "/tmp/pnix-supervisor.sock")]
  uds: String,

  /// Listen endpoint (repeatable): uds:/path.sock or tls:0.0.0.0:7443
  #[arg(long = "listen")]
  listen_endpoints: Vec<String>,

  /// Remove existing socket before bind
  #[arg(long)]
  force: bool,

  /// Supervisor durable state directory
  #[arg(long, default_value = "/tmp/pnix-supervisor-state")]
  state_dir: String,

  /// Supervisor logs directory (default: <state-dir>/logs)
  #[arg(long)]
  log_dir: Option<String>,

  /// Bootstrap desired file applied on startup
  #[arg(long)]
  bootstrap: Option<String>,

  /// Recovery mode on startup: none|lazy|eager
  #[arg(long, default_value = "lazy")]
  recover: String,

  /// cgroup v2 root path for process resource enforcement
  #[arg(long)]
  cgroup_root: Option<String>,

  /// Require cgroup enforcement when process spec includes cgroup settings
  #[arg(long, default_value_t = false, action = ArgAction::Set)]
  cgroup_require: bool,

  /// Print a smoke message and exit
  #[arg(long)]
  smoke: bool,

  /// Require token for call requests
  #[arg(long)]
  token: Option<String>,

  /// Token scope policy file (JSON)
  #[arg(long)]
  token_file: Option<String>,

  /// Policy reload polling interval (ms). 0 disables hot-reload.
  #[arg(long, default_value_t = 1000)]
  policy_reload_ms: u64,

  /// Reconcile tick interval (ms). 0 disables periodic reconcile.
  #[arg(long, default_value_t = 1000)]
  reconcile_tick_ms: u64,

  /// In-memory supervisor event ring buffer capacity
  #[arg(long, default_value_t = 10000)]
  event_buffer_capacity: usize,

  /// Number of background job worker threads
  #[arg(long, default_value_t = 2)]
  job_workers: usize,

  /// Job lease duration while running (ms)
  #[arg(long, default_value_t = 60_000)]
  job_lease_ms: u64,

  /// Job queue poll interval (ms)
  #[arg(long, default_value_t = 200)]
  job_poll_ms: u64,

  /// Housekeeping scheduler interval (ms), 0 disables scheduler
  #[arg(long, default_value_t = 60_000)]
  housekeeping_interval_ms: u64,

  /// Bundle retention window in days, 0 disables bundle pruning
  #[arg(long, default_value_t = 30)]
  retention_bundle_days: u64,

  /// Allow any executable (dangerous)
  #[arg(long, default_value_t = false, action = ArgAction::Set)]
  allow_any: bool,

  /// Allow exact executable path or basename (repeatable)
  #[arg(long)]
  allow_exec: Vec<String>,

  /// Allow executable path prefix (repeatable)
  #[arg(long)]
  allow_prefix: Vec<String>,

  /// Deny env key (repeatable)
  #[arg(long)]
  deny_env: Vec<String>,

  /// Allow process spawn/ensure operations
  #[arg(long, default_value_t = true, action = ArgAction::Set)]
  allow_spawn: bool,

  /// Allow signal/terminate operations
  #[arg(long, default_value_t = false, action = ArgAction::Set)]
  allow_signal: bool,

  /// Allow status/wait/logs operations
  #[arg(long, default_value_t = true, action = ArgAction::Set)]
  allow_observe: bool,

  /// TLS server certificate path (DER)
  #[arg(long)]
  tls_cert: Option<String>,

  /// TLS server private key path (DER)
  #[arg(long)]
  tls_key: Option<String>,

  /// TLS client CA path for mTLS client certificate verification (DER)
  #[arg(long)]
  tls_client_ca: Option<String>,

  /// Require TLS client certificate auth (mTLS)
  #[arg(long, default_value_t = true, action = ArgAction::Set)]
  tls_require_client_auth: bool,

  /// Enable append-only audit logging
  #[arg(long, default_value_t = true, action = ArgAction::Set)]
  audit_enabled: bool,

  /// Audit log directory (default: <state-dir>/audit)
  #[arg(long)]
  audit_dir: Option<String>,

  /// Max active audit log size before rotation
  #[arg(long, default_value_t = 64 * 1024 * 1024)]
  audit_max_bytes: u64,

  /// Number of rotated audit files to retain
  #[arg(long, default_value_t = 20)]
  audit_retain_files: usize,

  /// Flush audit writer every N events
  #[arg(long, default_value_t = 1)]
  audit_flush_every: usize,
}

#[cfg(not(unix))]
fn main() {
  eprintln!("pnix-supervisor: unix only (UDS required)");
  std::process::exit(1);
}

#[cfg(unix)]
fn main() -> anyhow::Result<()> {
  let args = Args::parse();

  if args.smoke {
    println!(
      "{}",
      serde_json::json!({
        "status": "ok",
        "component": "pnix-supervisor",
        "uds": args.uds,
        "listen": args.listen_endpoints,
      })
    );
    return Ok(());
  }

  let recover_mode = pnix_runtime_supervisor::server::RecoverMode::parse(&args.recover)?;
  let state_dir = std::path::PathBuf::from(&args.state_dir);
  let log_dir = args
    .log_dir
    .as_ref()
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| state_dir.join("logs"));
  let audit_dir = args
    .audit_dir
    .as_ref()
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| state_dir.join("audit"));

  let policy = pnix_runtime_supervisor::server::Policy {
    token: args.token,
    token_rules: Vec::new(),
    default_rate_limits: pnix_runtime_supervisor::rate_limit::RateLimitsByClass::with_defaults(),
    allow_any: args.allow_any,
    allow_exec: args.allow_exec,
    allow_prefix: args.allow_prefix,
    deny_env_keys: if args.deny_env.is_empty() {
      vec![
        "LD_PRELOAD".to_string(),
        "DYLD_INSERT_LIBRARIES".to_string(),
      ]
    } else {
      args.deny_env
    },
    allow_spawn: args.allow_spawn,
    allow_signal: args.allow_signal,
    allow_observe: args.allow_observe,
  };
  let runtime = pnix_runtime_supervisor::server::RuntimeConfig {
    state_dir,
    log_dir,
    bootstrap_file: args.bootstrap.as_ref().map(PathBuf::from),
    cgroup_root: args.cgroup_root.as_ref().map(PathBuf::from),
    cgroup_require: args.cgroup_require,
    recover_mode,
    token_file: args.token_file.as_ref().map(PathBuf::from),
    policy_reload_ms: args.policy_reload_ms,
    audit_enabled: args.audit_enabled,
    audit: pnix_runtime_supervisor::audit::AuditConfig {
      dir: audit_dir,
      max_bytes: args.audit_max_bytes,
      retain_files: args.audit_retain_files,
      flush_every: args.audit_flush_every.max(1),
    },
    reconcile_tick_ms: args.reconcile_tick_ms,
    event_buffer_capacity: args.event_buffer_capacity.max(64),
    job_workers: args.job_workers.max(1),
    job_lease_ms: args.job_lease_ms.max(1_000),
    job_poll_ms: args.job_poll_ms.max(20),
    housekeeping_interval_ms: args.housekeeping_interval_ms,
    retention_bundle_days: args.retention_bundle_days,
  };

  let listeners = if args.listen_endpoints.is_empty() {
    vec![format!("uds:{}", args.uds)]
  } else {
    args.listen_endpoints.clone()
  };

  let tls = match (args.tls_cert.as_ref(), args.tls_key.as_ref()) {
    (Some(cert), Some(key)) => Some(pnix_runtime_supervisor::server::TlsServerConfig {
      cert_path: PathBuf::from(cert),
      key_path: PathBuf::from(key),
      client_ca_path: args.tls_client_ca.as_ref().map(PathBuf::from),
      require_client_auth: args.tls_require_client_auth,
    }),
    (None, None) => None,
    _ => anyhow::bail!("--tls-cert and --tls-key must be provided together"),
  };

  pnix_runtime_supervisor::server::SupervisorServer::serve_with_policy_runtime_and_transport(
    &listeners, args.force, policy, runtime, tls,
  )
}
