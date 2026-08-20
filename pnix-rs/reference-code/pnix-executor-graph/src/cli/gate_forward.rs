use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};

use super::args::{Args, GateForwardVerb, OutputFormat};

const EX_OK: i32 = 0;
const EX_USAGE: i32 = 64;
const GATE_FORWARD_VERSION: &str = "0.1.0";
const GATE_FORWARD_USAGE: &str =
  "usage: pnix gate-forward [help] [--limit N] [--kind PREFIX] [--dry-run] [--reset] [--url URL]";
const DEFAULT_DOGHOUSE_URL: &str = "http://127.0.0.1:8787";
const DEFAULT_TIMEOUT_MS: u64 = 2_000;
const DEFAULT_ALIVE_TIMEOUT_MS: u64 = 500;
const DEFAULT_USER_AGENT: &str = "pnix-gate-forward/0.1 (+https://pnix.local)";

#[derive(Debug, Clone, Serialize)]
struct GateForwardSummary {
  url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  target: Option<String>,
  alive: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  transport: Option<&'static str>,
  attempted: usize,
  sent: usize,
  skipped: usize,
  failed: usize,
  results: Vec<GateForwardResult>,
  dry_run: bool,
  reset: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  note: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct GateForwardResult {
  filename: String,
  status: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  code: Option<u16>,
  #[serde(skip_serializing_if = "Option::is_none")]
  at: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  url: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  transport: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  error: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  dedupe_key: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  skip_reason: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  sink: Option<GateForwardSink>,
}

#[derive(Debug, Clone, Serialize)]
struct GateForwardSink {
  path: String,
  filename: String,
  status: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct GateForwardStateRecord {
  status: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  code: Option<u16>,
  at: String,
  url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  transport: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  error: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  dedupe_key: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  skip_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct GateForwardCandidateEntry {
  path: PathBuf,
  filename: String,
  body: String,
  dedupe_key: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct SentStateIndex {
  filenames: HashSet<String>,
  dedupe_keys: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateForwardTransport {
  Http,
  FileDrop,
}

impl GateForwardTransport {
  fn as_str(self) -> &'static str {
    match self {
      Self::Http => "http",
      Self::FileDrop => "file-drop",
    }
  }
}

pub(super) fn run_gate_forward(args: &Args, verb: &GateForwardVerb) -> Result<i32> {
  match verb {
    GateForwardVerb::Help => {
      print_help_banner();
      Ok(EX_OK)
    }
    GateForwardVerb::Unknown(raw) => {
      eprintln!("pnix-gate-forward: unknown subcommand: {:?}", raw);
      eprintln!("{GATE_FORWARD_USAGE}");
      Ok(EX_USAGE)
    }
    GateForwardVerb::Run => {
      let summary = execute_gate_forward(args)?;
      match args.output_format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&summary)?),
        OutputFormat::Text => print_text_summary(&summary),
      }
      Ok(EX_OK)
    }
  }
}

fn print_help_banner() {
  println!(
    "pnix-gate-forward {} — candidate `.px` -> doghouse ingress transport",
    GATE_FORWARD_VERSION
  );
  println!();
  println!("usage:");
  println!("  pnix gate-forward [--limit N] [--kind PREFIX] [--dry-run] [--reset] [--url URL]");
  println!("  pnix gate-forward help");
}

fn print_text_summary(summary: &GateForwardSummary) {
  println!("pnix-gate-forward {}", GATE_FORWARD_VERSION);
  println!(
    "alive={} transport={} attempted={} sent={} skipped={} failed={} dry_run={} reset={}",
    summary.alive,
    summary.transport.unwrap_or("none"),
    summary.attempted,
    summary.sent,
    summary.skipped,
    summary.failed,
    summary.dry_run,
    summary.reset
  );
  if let Some(target) = summary.target.as_deref() {
    println!("target={}", target);
  }
  if let Some(note) = summary.note {
    println!("note={}", note);
  }
  for result in &summary.results {
    println!(
      "result {} status={} transport={} skip_reason={} url={}",
      result.filename,
      result.status,
      result.transport.as_deref().unwrap_or("none"),
      result.skip_reason.as_deref().unwrap_or("-"),
      result.url.as_deref().unwrap_or("-")
    );
  }
}

fn execute_gate_forward(args: &Args) -> Result<GateForwardSummary> {
  let base_url = args
    .gate_forward_url
    .clone()
    .unwrap_or_else(default_doghouse_url);
  let http_up = alive(&base_url);
  let file_drop = default_doghouse_candidate_drop();
  let file_drop_up = file_drop_available(file_drop.as_deref());
  let transport = if http_up {
    Some(GateForwardTransport::Http)
  } else if file_drop_up {
    Some(GateForwardTransport::FileDrop)
  } else {
    None
  };
  let target = match transport {
    Some(GateForwardTransport::Http) => Some(format!("{}/candidate", base_url)),
    Some(GateForwardTransport::FileDrop) => Some(path_to_slash(
      file_drop.as_ref().context("missing candidate drop path")?,
    )),
    None => None,
  };
  let store_root = gate_store_root();
  let state = read_state(&store_root);
  let sent_index = sent_state_index(&state);
  let files = list_candidate_files(&store_root, args.gate_forward_kind.as_deref(), 1000)?;
  let Some(transport) = transport else {
    return Ok(GateForwardSummary {
      url: base_url,
      target: None,
      alive: false,
      transport: None,
      attempted: 0,
      sent: 0,
      skipped: 0,
      failed: 0,
      results: Vec::new(),
      dry_run: args.dry_run,
      reset: args.gate_forward_reset,
      note: Some("doghouse-http 미기동/비응답 + runtime file-drop unavailable — local-only"),
    });
  };

  let limit = args.gate_forward_limit.unwrap_or(20);
  let results = if args.dry_run {
    preview_forward_results(
      &files,
      sent_index,
      limit,
      args.gate_forward_reset,
      target.as_deref().unwrap_or_default(),
      transport,
    )
  } else {
    execute_forward_results(
      &files,
      sent_index,
      limit,
      args.gate_forward_reset,
      &base_url,
      target.as_deref().unwrap_or_default(),
      transport,
      &store_root,
      file_drop.as_deref(),
    )?
  };
  let sent = results.iter().filter(|row| row.status == "sent").count();
  let skipped = results.iter().filter(|row| row.status == "skipped").count();
  let failed = results
    .iter()
    .filter(|row| {
      matches!(
        row.status.as_str(),
        "http_error" | "connection_error" | "not_yet_implemented"
      )
    })
    .count();
  Ok(GateForwardSummary {
    url: base_url,
    target,
    alive: true,
    transport: Some(transport.as_str()),
    attempted: results.len(),
    sent,
    skipped,
    failed,
    results,
    dry_run: args.dry_run,
    reset: args.gate_forward_reset,
    note: None,
  })
}

fn trimmed_env(name: &str) -> Option<String> {
  env::var(name)
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
}

fn default_doghouse_url() -> String {
  trimmed_env("PNIX_GATE_DOGHOUSE_URL").unwrap_or_else(|| DEFAULT_DOGHOUSE_URL.to_string())
}

fn default_doghouse_runtime_dir() -> Option<PathBuf> {
  if let Some(runtime) = trimmed_env("DOGHOUSE_RUNTIME_DIR") {
    return Some(PathBuf::from(runtime));
  }
  let state_home = trimmed_env("XDG_STATE_HOME")
    .or_else(|| trimmed_env("HOME").map(|home| format!("{home}/.local/state")))?;
  Some(PathBuf::from(format!("{state_home}/uppnix/doghouse")))
}

fn default_doghouse_socket() -> Option<PathBuf> {
  default_doghouse_runtime_dir().map(|root| root.join("higloo-conversation.sock"))
}

fn default_doghouse_candidate_drop() -> Option<PathBuf> {
  default_doghouse_runtime_dir().map(|root| root.join("px-candidates"))
}

fn gate_store_root() -> PathBuf {
  if let Some(root) = trimmed_env("PNIX_GATE_STORE_DIR") {
    return PathBuf::from(root);
  }
  if let Some(runtime) = default_doghouse_runtime_dir() {
    return runtime.join("pnix-gate");
  }
  if let Some(home) = trimmed_env("HOME") {
    return PathBuf::from(format!("{home}/pnix/pnix-gate/.store"));
  }
  PathBuf::from(".store")
}

fn state_path(store_root: &Path) -> PathBuf {
  store_root.join("forward-state.json")
}

fn candidate_dir(store_root: &Path) -> PathBuf {
  store_root.join("px").join("candidates")
}

fn completed_dir(store_root: &Path) -> PathBuf {
  store_root.join("px").join("completed")
}

fn iso_now() -> String {
  let now = chrono_like_now();
  now
}

fn chrono_like_now() -> String {
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default();
  format!("unix:{}", now.as_secs())
}

fn read_state(store_root: &Path) -> BTreeMap<String, GateForwardStateRecord> {
  let path = state_path(store_root);
  if !path.exists() {
    return BTreeMap::new();
  }
  let Ok(raw) = fs::read_to_string(&path) else {
    return BTreeMap::new();
  };
  serde_json::from_str(&raw).unwrap_or_default()
}

fn write_state(store_root: &Path, state: &BTreeMap<String, GateForwardStateRecord>) -> Result<()> {
  fs::create_dir_all(store_root)
    .with_context(|| format!("create gate store root {}", store_root.display()))?;
  let path = state_path(store_root);
  fs::write(&path, serde_json::to_string_pretty(state)?)
    .with_context(|| format!("write gate forward state {}", path.display()))?;
  Ok(())
}

fn record_forward(store_root: &Path, filename: &str, record: GateForwardStateRecord) -> Result<()> {
  let mut state = read_state(store_root);
  state.insert(filename.to_string(), record);
  write_state(store_root, &state)
}

fn sent_state_index(state: &BTreeMap<String, GateForwardStateRecord>) -> SentStateIndex {
  let mut index = SentStateIndex::default();
  for (filename, record) in state {
    if record.status == "sent" {
      index.filenames.insert(filename.clone());
      if let Some(dedupe_key) = record.dedupe_key.as_ref().filter(|value| !value.is_empty()) {
        index.dedupe_keys.insert(dedupe_key.clone());
      }
    }
  }
  index
}

fn already_sent(index: &SentStateIndex, filename: &str, dedupe_key: Option<&str>) -> bool {
  index.filenames.contains(filename)
    || dedupe_key
      .filter(|value| !value.is_empty())
      .is_some_and(|value| index.dedupe_keys.contains(value))
}

fn alive(url: &str) -> bool {
  let client = Client::builder()
    .timeout(std::time::Duration::from_millis(DEFAULT_ALIVE_TIMEOUT_MS))
    .build();
  let Ok(client) = client else {
    return false;
  };
  client
    .get(url)
    .send()
    .map(|response| response.status().is_success())
    .unwrap_or(false)
}

fn file_drop_available(candidate_drop: Option<&Path>) -> bool {
  let Some(runtime) = default_doghouse_runtime_dir() else {
    return false;
  };
  let socket_exists = default_doghouse_socket()
    .as_deref()
    .is_some_and(Path::exists);
  let runtime_exists = runtime.exists();
  let drop_ready = candidate_drop.is_some();
  drop_ready && (socket_exists || runtime_exists)
}

fn list_candidate_files(
  store_root: &Path,
  kind: Option<&str>,
  limit: usize,
) -> Result<Vec<GateForwardCandidateEntry>> {
  let root = candidate_dir(store_root);
  if !root.exists() {
    return Ok(Vec::new());
  }
  let mut paths = Vec::new();
  collect_px_files(&root, &mut paths)?;
  let mut rows = Vec::new();
  for path in paths {
    let Some(filename) = path
      .file_name()
      .and_then(|value| value.to_str())
      .map(|value| value.to_string())
    else {
      continue;
    };
    if let Some(prefix) = kind {
      if !filename.starts_with(prefix) {
        continue;
      }
    }
    let body = fs::read_to_string(&path)
      .with_context(|| format!("read candidate file {}", path.display()))?;
    rows.push(GateForwardCandidateEntry {
      dedupe_key: candidate_forward_dedupe_key(&body),
      path,
      filename,
      body,
    });
  }
  rows.sort_by(|left, right| left.filename.cmp(&right.filename));
  if rows.len() > limit {
    Ok(rows.split_off(rows.len() - limit))
  } else {
    Ok(rows)
  }
}

fn collect_px_files(root: &Path, acc: &mut Vec<PathBuf>) -> Result<()> {
  for entry in
    fs::read_dir(root).with_context(|| format!("read candidate dir {}", root.display()))?
  {
    let entry = entry?;
    let path = entry.path();
    if path.is_dir() {
      collect_px_files(&path, acc)?;
    } else if path.extension().and_then(|value| value.to_str()) == Some("px") {
      acc.push(path);
    }
  }
  Ok(())
}

fn preview_forward_results(
  files: &[GateForwardCandidateEntry],
  mut historical: SentStateIndex,
  limit: usize,
  reset: bool,
  target: &str,
  transport: GateForwardTransport,
) -> Vec<GateForwardResult> {
  let mut results = Vec::new();
  for entry in files.iter().take(limit) {
    if !reset && already_sent(&historical, &entry.filename, entry.dedupe_key.as_deref()) {
      continue;
    }
    if let Some(dedupe_key) = entry.dedupe_key.as_ref().filter(|value| !value.is_empty()) {
      historical.dedupe_keys.insert(dedupe_key.clone());
    }
    results.push(GateForwardResult {
      filename: entry.filename.clone(),
      status: "dry_run".to_string(),
      code: None,
      at: None,
      url: Some(target.to_string()),
      transport: Some(transport.as_str().to_string()),
      error: None,
      dedupe_key: entry.dedupe_key.clone(),
      skip_reason: None,
      sink: None,
    });
  }
  results
}

fn execute_forward_results(
  files: &[GateForwardCandidateEntry],
  mut historical: SentStateIndex,
  limit: usize,
  reset: bool,
  base_url: &str,
  target: &str,
  transport: GateForwardTransport,
  store_root: &Path,
  candidate_drop: Option<&Path>,
) -> Result<Vec<GateForwardResult>> {
  let mut sent_in_run = HashSet::new();
  let mut results = Vec::new();
  for entry in files.iter().take(limit) {
    if entry
      .dedupe_key
      .as_ref()
      .is_some_and(|value| sent_in_run.contains(value))
    {
      let row = skipped_equivalent_result(store_root, entry, target, "same-run-equivalent")?;
      results.push(row);
      continue;
    }
    if !reset && already_sent(&historical, &entry.filename, entry.dedupe_key.as_deref()) {
      let row = skipped_equivalent_result(store_root, entry, target, "sent-equivalent")?;
      results.push(row);
      continue;
    }

    let response = match transport {
      GateForwardTransport::Http => post_candidate(base_url, &entry.filename, &entry.body),
      GateForwardTransport::FileDrop => {
        file_drop_candidate(candidate_drop, &entry.filename, &entry.body)
      }
    };
    let mut record = response;
    record.dedupe_key = entry.dedupe_key.clone();
    let mut sink = None;
    if record.status == "sent" {
      sink = Some(move_candidate_to_completed(store_root, &entry.path)?);
      historical.filenames.insert(entry.filename.clone());
      if let Some(dedupe_key) = entry.dedupe_key.as_ref().filter(|value| !value.is_empty()) {
        historical.dedupe_keys.insert(dedupe_key.clone());
        sent_in_run.insert(dedupe_key.clone());
      }
    }
    record_forward(store_root, &entry.filename, record.clone())?;
    results.push(GateForwardResult {
      filename: entry.filename.clone(),
      status: record.status,
      code: record.code,
      at: Some(record.at),
      url: Some(record.url),
      transport: record.transport,
      error: record.error,
      dedupe_key: record.dedupe_key,
      skip_reason: record.skip_reason,
      sink,
    });
  }
  Ok(results)
}

fn skipped_equivalent_result(
  store_root: &Path,
  entry: &GateForwardCandidateEntry,
  target: &str,
  skip_reason: &'static str,
) -> Result<GateForwardResult> {
  let sink = move_candidate_to_completed(store_root, &entry.path)?;
  let record = GateForwardStateRecord {
    status: "skipped".to_string(),
    code: None,
    at: iso_now(),
    url: target.to_string(),
    transport: None,
    error: None,
    dedupe_key: entry.dedupe_key.clone(),
    skip_reason: Some(skip_reason.to_string()),
  };
  record_forward(store_root, &entry.filename, record.clone())?;
  Ok(GateForwardResult {
    filename: entry.filename.clone(),
    status: record.status,
    code: record.code,
    at: Some(record.at),
    url: Some(record.url),
    transport: None,
    error: None,
    dedupe_key: record.dedupe_key,
    skip_reason: Some(skip_reason.to_string()),
    sink: Some(sink),
  })
}

fn post_candidate(base_url: &str, filename: &str, body: &str) -> GateForwardStateRecord {
  let endpoint = format!("{}/candidate", base_url);
  let client = Client::builder()
    .timeout(std::time::Duration::from_millis(DEFAULT_TIMEOUT_MS))
    .build();
  let now = iso_now();
  let Ok(client) = client else {
    return GateForwardStateRecord {
      status: "connection_error".to_string(),
      code: None,
      at: now,
      url: endpoint,
      transport: Some("http".to_string()),
      error: Some("failed to create HTTP client".to_string()),
      dedupe_key: None,
      skip_reason: None,
    };
  };
  match client
    .post(endpoint.clone())
    .header(CONTENT_TYPE, "text/plain; charset=utf-8")
    .header(USER_AGENT, DEFAULT_USER_AGENT)
    .header("X-Pnix-Gate-Candidate", filename)
    .body(body.to_string())
    .send()
  {
    Ok(response) => {
      let code = response.status().as_u16();
      if response.status().is_success() {
        GateForwardStateRecord {
          status: "sent".to_string(),
          code: Some(code),
          at: now,
          url: endpoint,
          transport: Some("http".to_string()),
          error: None,
          dedupe_key: None,
          skip_reason: None,
        }
      } else if code == 404 {
        GateForwardStateRecord {
          status: "not_yet_implemented".to_string(),
          code: Some(code),
          at: now,
          url: endpoint,
          transport: Some("http".to_string()),
          error: Some(
            "doghouse 가 canonical /candidate endpoint 를 제공하지 않음 (stale/foreign build 가능성)"
              .to_string(),
          ),
          dedupe_key: None,
          skip_reason: None,
        }
      } else {
        GateForwardStateRecord {
          status: "http_error".to_string(),
          code: Some(code),
          at: now,
          url: endpoint,
          transport: Some("http".to_string()),
          error: Some(format!("HTTP {}", code)),
          dedupe_key: None,
          skip_reason: None,
        }
      }
    }
    Err(error) => GateForwardStateRecord {
      status: "connection_error".to_string(),
      code: None,
      at: now,
      url: endpoint,
      transport: Some("http".to_string()),
      error: Some(error.to_string()),
      dedupe_key: None,
      skip_reason: None,
    },
  }
}

fn normalize_candidate_filename(filename: &str) -> Result<String> {
  let raw = filename.trim();
  if raw.is_empty() {
    bail!("invalid candidate filename");
  }
  if raw.contains('/')
    || raw.contains('\\')
    || raw.contains("..")
    || raw.chars().any(char::is_control)
  {
    bail!("invalid candidate filename");
  }
  if raw.ends_with(".px") {
    Ok(raw.to_string())
  } else {
    Ok(format!("{raw}.px"))
  }
}

fn file_drop_candidate(
  candidate_drop: Option<&Path>,
  filename: &str,
  body: &str,
) -> GateForwardStateRecord {
  let now = iso_now();
  let Some(target_dir) = candidate_drop else {
    return GateForwardStateRecord {
      status: "connection_error".to_string(),
      code: None,
      at: now,
      url: String::new(),
      transport: Some("file-drop".to_string()),
      error: Some("doghouse runtime candidate drop unavailable".to_string()),
      dedupe_key: None,
      skip_reason: None,
    };
  };
  let normalized = match normalize_candidate_filename(filename) {
    Ok(value) => value,
    Err(error) => {
      return GateForwardStateRecord {
        status: "http_error".to_string(),
        code: None,
        at: now,
        url: path_to_slash(target_dir),
        transport: Some("file-drop".to_string()),
        error: Some(error.to_string()),
        dedupe_key: None,
        skip_reason: None,
      };
    }
  };
  let target = target_dir.join(normalized);
  let write_result = (|| -> Result<()> {
    fs::create_dir_all(target_dir)
      .with_context(|| format!("create candidate drop {}", target_dir.display()))?;
    fs::write(&target, body).with_context(|| format!("write candidate {}", target.display()))?;
    Ok(())
  })();
  match write_result {
    Ok(()) => GateForwardStateRecord {
      status: "sent".to_string(),
      code: None,
      at: now,
      url: path_to_slash(&target),
      transport: Some("file-drop".to_string()),
      error: None,
      dedupe_key: None,
      skip_reason: None,
    },
    Err(error) => GateForwardStateRecord {
      status: "connection_error".to_string(),
      code: None,
      at: now,
      url: path_to_slash(target_dir),
      transport: Some("file-drop".to_string()),
      error: Some(error.to_string()),
      dedupe_key: None,
      skip_reason: None,
    },
  }
}

fn move_candidate_to_completed(store_root: &Path, path: &Path) -> Result<GateForwardSink> {
  let filename = path
    .file_name()
    .and_then(|value| value.to_str())
    .context("candidate path missing filename")?
    .to_string();
  let target_dir = completed_dir(store_root);
  fs::create_dir_all(&target_dir)
    .with_context(|| format!("create completed dir {}", target_dir.display()))?;
  let target = target_dir.join(&filename);
  if path != target {
    if target.exists() {
      fs::remove_file(&target)
        .with_context(|| format!("replace completed candidate {}", target.display()))?;
    }
    fs::rename(path, &target)
      .with_context(|| format!("move candidate {} -> {}", path.display(), target.display()))?;
  }
  Ok(GateForwardSink {
    path: path_to_slash(&target),
    filename,
    status: "completed",
  })
}

fn candidate_forward_dedupe_key(content: &str) -> Option<String> {
  let kind = px_field_string(content, "kind");
  let content_hash = px_field_string(content, "content-hash");
  let source_rule = px_field_string(content, "source-rule");
  let provider = px_field_string(content, "provider");
  let model = px_field_string(content, "model");
  let session_id = candidate_session_id(content);
  let turn_id = candidate_turn_id(content);
  let tool_call_id = px_field_string(content, "tool-call-id")
    .or_else(|| px_field_string(content, "tool_call_id"))
    .or_else(|| px_field_string(content, "tool_use_id"));
  let truth_regime =
    px_field_string(content, "truth-regime").or_else(|| px_field_string(content, "truth_regime"));
  let mut convergence_closes = px_field_list(content, "convergence-closes").unwrap_or_default();
  let mut provenance = px_field_list(content, "provenance").unwrap_or_default();
  convergence_closes.sort();
  provenance.sort();
  let content_hash = content_hash?;
  Some(format!(
    "[{} {} {} {} {} {} {} {} {} {} {}]",
    edn_optional_string(kind.as_deref()),
    edn_optional_string(Some(content_hash.as_str())),
    edn_optional_string(source_rule.as_deref()),
    edn_optional_string(provider.as_deref()),
    edn_optional_string(model.as_deref()),
    edn_optional_string(session_id.as_deref()),
    edn_optional_string(turn_id.as_deref()),
    edn_optional_string(tool_call_id.as_deref()),
    edn_optional_string(truth_regime.as_deref()),
    edn_string_vec(&convergence_closes),
    edn_string_vec(&provenance)
  ))
}

fn candidate_session_id(content: &str) -> Option<String> {
  px_field_string(content, "session-id")
    .or_else(|| px_field_string(content, "session_id"))
    .or_else(|| px_field_string(content, "source-session-id"))
    .or_else(|| px_field_string(content, "source_session_id"))
}

fn candidate_turn_id(content: &str) -> Option<String> {
  px_field_string(content, "turn-id")
    .or_else(|| px_field_string(content, "turn_id"))
    .or_else(|| px_field_string(content, "source-turn-id"))
    .or_else(|| px_field_string(content, "source_turn_id"))
}

fn px_field_string(content: &str, field: &str) -> Option<String> {
  let prefix = format!("{field} = ");
  for line in content.lines() {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix(&prefix) {
      return parse_px_quoted_string(rest.trim());
    }
  }
  None
}

fn px_field_list(content: &str, field: &str) -> Option<Vec<String>> {
  let marker = format!("{field} = [");
  let start = content.find(&marker)?;
  let rest = &content[start + marker.len()..];
  let end = rest.find("];")?;
  let slice = &rest[..end];
  Some(extract_px_quoted_strings(slice))
}

fn parse_px_quoted_string(raw: &str) -> Option<String> {
  let trimmed = raw.trim();
  let mut chars = trimmed.chars();
  if chars.next()? != '"' {
    return None;
  }
  let mut value = String::new();
  let mut escaped = false;
  for ch in chars {
    if escaped {
      value.push(ch);
      escaped = false;
      continue;
    }
    match ch {
      '\\' => escaped = true,
      '"' => return Some(value),
      other => value.push(other),
    }
  }
  None
}

fn extract_px_quoted_strings(raw: &str) -> Vec<String> {
  let mut values = Vec::new();
  let mut chars = raw.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch != '"' {
      continue;
    }
    let mut value = String::new();
    let mut escaped = false;
    while let Some(next) = chars.next() {
      if escaped {
        value.push(next);
        escaped = false;
        continue;
      }
      match next {
        '\\' => escaped = true,
        '"' => {
          values.push(value);
          break;
        }
        other => value.push(other),
      }
    }
  }
  values
}

fn edn_quote(value: &str) -> String {
  format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn edn_optional_string(value: Option<&str>) -> String {
  value.map(edn_quote).unwrap_or_else(|| "nil".to_string())
}

fn edn_string_vec(values: &[String]) -> String {
  if values.is_empty() {
    "[]".to_string()
  } else {
    format!(
      "[{}]",
      values
        .iter()
        .map(|value| edn_quote(value))
        .collect::<Vec<_>>()
        .join(" ")
    )
  }
}

fn path_to_slash(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/")
}
