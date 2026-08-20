use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use pnix_hash::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub struct AuditConfig {
  pub dir: PathBuf,
  pub max_bytes: u64,
  pub retain_files: usize,
  pub flush_every: usize,
}

impl AuditConfig {
  pub fn with_dir(dir: PathBuf) -> Self {
    Self {
      dir,
      max_bytes: 64 * 1024 * 1024,
      retain_files: 20,
      flush_every: 1,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeerAuditInfo {
  pub uid: Option<u32>,
  pub gid: Option<u32>,
  pub pid: Option<u32>,
  pub tls_subject: Option<String>,
  pub tls_spki_sha256: Option<String>,
  pub remote_addr: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditTarget {
  pub id: Option<String>,
  pub pid: Option<u32>,
  pub handle_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuditEvent {
  pub ts_ms: u128,
  pub latency_ms: u64,
  pub transport: String,
  pub peer: PeerAuditInfo,
  pub token_id: Option<String>,
  pub decision: String,
  pub deny_reason: Option<String>,
  pub req_id: u64,
  pub op: String,
  pub caps: Vec<String>,
  pub target: Option<AuditTarget>,
  pub params_hash: String,
  pub params_redacted: Option<Value>,
  pub status: String,
  pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditLine {
  seq: u64,
  ts_ms: u128,
  latency_ms: u64,
  transport: String,
  peer: PeerAuditInfo,
  token_id: Option<String>,
  decision: String,
  deny_reason: Option<String>,
  req_id: u64,
  op: String,
  caps: Vec<String>,
  target: Option<AuditTarget>,
  params_hash: String,
  params_redacted: Option<Value>,
  status: String,
  error: Option<String>,
  prev_hash: String,
  hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuditHead {
  seq: u64,
  hash: String,
}

#[derive(Clone)]
pub struct AuditLogger {
  tx: mpsc::Sender<AuditEvent>,
}

impl AuditLogger {
  pub fn start(config: AuditConfig) -> Result<Self> {
    fs::create_dir_all(&config.dir)
      .with_context(|| format!("create audit dir {}", config.dir.display()))?;

    let (tx, rx) = mpsc::channel::<AuditEvent>();
    std::thread::spawn(move || {
      let mut writer = match AuditWriter::new(&config) {
        Ok(writer) => writer,
        Err(error) => {
          eprintln!("audit init failed: {error:#}");
          return;
        }
      };

      while let Ok(event) = rx.recv() {
        if let Err(error) = writer.write_event(&config, event) {
          eprintln!("audit write failed: {error:#}");
        }
      }
    });
    Ok(Self { tx })
  }

  pub fn emit(&self, event: AuditEvent) {
    let _ = self.tx.send(event);
  }
}

struct AuditWriter {
  seq: u64,
  last_hash: String,
  file: File,
  path: PathBuf,
  bytes: u64,
  pending_flush: usize,
  segment_from: u64,
}

impl AuditWriter {
  fn new(config: &AuditConfig) -> Result<Self> {
    let head = load_head(config.dir.as_path())?;
    let path = config.dir.join("audit.jsonl");
    let file = OpenOptions::new()
      .create(true)
      .append(true)
      .open(&path)
      .with_context(|| format!("open audit log {}", path.display()))?;
    let bytes = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    Ok(Self {
      seq: head.seq,
      last_hash: head.hash,
      file,
      path,
      bytes,
      pending_flush: 0,
      segment_from: head.seq.saturating_add(1),
    })
  }

  fn write_event(&mut self, config: &AuditConfig, event: AuditEvent) -> Result<()> {
    self.seq = self.seq.saturating_add(1);
    let prev_hash = self.last_hash.clone();
    let hash = hash_audit_line(self.seq, &prev_hash, &event)?;
    let line = AuditLine {
      seq: self.seq,
      ts_ms: event.ts_ms,
      latency_ms: event.latency_ms,
      transport: event.transport,
      peer: event.peer,
      token_id: event.token_id,
      decision: event.decision,
      deny_reason: event.deny_reason,
      req_id: event.req_id,
      op: event.op,
      caps: event.caps,
      target: event.target,
      params_hash: event.params_hash,
      params_redacted: event.params_redacted,
      status: event.status,
      error: event.error.map(|msg| truncate_msg(msg, 1024)),
      prev_hash,
      hash: hash.clone(),
    };

    let bytes = serde_json::to_vec(&line)?;
    self.file.write_all(&bytes)?;
    self.file.write_all(b"\n")?;
    self.bytes = self.bytes.saturating_add(bytes.len() as u64 + 1);
    self.pending_flush = self.pending_flush.saturating_add(1);
    self.last_hash = hash;

    if self.pending_flush >= config.flush_every.max(1) {
      self.file.flush()?;
      self.pending_flush = 0;
    }
    save_head(config.dir.as_path(), self.seq, &self.last_hash)?;

    if self.bytes >= config.max_bytes.max(1) {
      self.rotate(config)?;
    }
    Ok(())
  }

  fn rotate(&mut self, config: &AuditConfig) -> Result<()> {
    self.file.flush()?;
    let ts = current_time_ms();
    let segment_to = self.seq;
    let rotated = config.dir.join(format!(
      "audit-{}-seq{}-{}.jsonl",
      ts, self.segment_from, segment_to
    ));
    fs::rename(&self.path, &rotated).with_context(|| {
      format!(
        "rotate audit log {} -> {}",
        self.path.display(),
        rotated.display()
      )
    })?;
    self.file = OpenOptions::new()
      .create(true)
      .append(true)
      .open(&self.path)
      .with_context(|| format!("open audit log {}", self.path.display()))?;
    self.bytes = 0;
    self.pending_flush = 0;
    self.segment_from = self.seq.saturating_add(1);
    gc_rotated_files(config)?;
    Ok(())
  }
}

pub fn redact_value(value: &Value) -> Value {
  match value {
    Value::Object(map) => {
      let mut out = serde_json::Map::new();
      for (key, child) in map {
        let lower = key.to_ascii_lowercase();
        if lower.contains("token")
          || lower.contains("password")
          || lower.contains("secret")
          || lower == "authorization"
        {
          out.insert(key.clone(), Value::String("<redacted>".to_string()));
          continue;
        }
        if lower == "env" {
          if let Some(env_obj) = child.as_object() {
            let mut env_out = serde_json::Map::new();
            for (ek, ev) in env_obj {
              if let Some(raw) = ev.as_str() {
                if raw.starts_with("${ENV:") && raw.ends_with('}') {
                  env_out.insert(ek.clone(), Value::String(raw.to_string()));
                } else {
                  env_out.insert(ek.clone(), Value::String("<redacted>".to_string()));
                }
              } else {
                env_out.insert(ek.clone(), Value::String("<redacted>".to_string()));
              }
            }
            out.insert(key.clone(), Value::Object(env_out));
          } else {
            out.insert(key.clone(), Value::String("<redacted>".to_string()));
          }
          continue;
        }
        out.insert(key.clone(), redact_value(child));
      }
      Value::Object(out)
    }
    Value::Array(items) => Value::Array(items.iter().map(redact_value).collect()),
    _ => value.clone(),
  }
}

pub fn hash_params(value: &Value) -> Result<String> {
  let canonical = canonicalize_json(value);
  let bytes = serde_json::to_vec(&canonical)?;
  let mut hasher = Sha256::new();
  hasher.update(bytes);
  let digest = hasher.finalize();
  Ok(format!("sha256:{}", hex_lower(&digest)))
}

fn hash_audit_line(seq: u64, prev_hash: &str, event: &AuditEvent) -> Result<String> {
  let payload = serde_json::json!({
    "seq": seq,
    "ts_ms": event.ts_ms,
    "latency_ms": event.latency_ms,
    "transport": event.transport,
    "peer": event.peer,
    "token_id": event.token_id,
    "decision": event.decision,
    "deny_reason": event.deny_reason,
    "req_id": event.req_id,
    "op": event.op,
    "caps": event.caps,
    "target": event.target,
    "params_hash": event.params_hash,
    "params_redacted": event.params_redacted,
    "status": event.status,
    "error": event.error,
  });
  let canonical = canonicalize_json(&payload);
  let bytes = serde_json::to_vec(&canonical)?;
  let mut hasher = Sha256::new();
  hasher.update(prev_hash.as_bytes());
  hasher.update([0_u8]);
  hasher.update(bytes);
  let digest = hasher.finalize();
  Ok(format!("sha256:{}", hex_lower(&digest)))
}

fn canonicalize_json(value: &Value) -> Value {
  match value {
    Value::Object(map) => {
      let mut sorted = BTreeMap::new();
      for (key, item) in map {
        sorted.insert(key.clone(), canonicalize_json(item));
      }
      let mut out = serde_json::Map::new();
      for (key, item) in sorted {
        out.insert(key, item);
      }
      Value::Object(out)
    }
    Value::Array(items) => Value::Array(items.iter().map(canonicalize_json).collect()),
    _ => value.clone(),
  }
}

fn load_head(dir: &Path) -> Result<AuditHead> {
  let path = dir.join("audit.head.json");
  if !path.exists() {
    return Ok(AuditHead {
      seq: 0,
      hash: "sha256:genesis".to_string(),
    });
  }
  let raw = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
  serde_json::from_slice(&raw).with_context(|| format!("parse {}", path.display()))
}

fn save_head(dir: &Path, seq: u64, hash: &str) -> Result<()> {
  let path = dir.join("audit.head.json");
  let tmp = dir.join("audit.head.json.tmp");
  let bytes = serde_json::to_vec_pretty(&AuditHead {
    seq,
    hash: hash.to_string(),
  })?;
  fs::write(&tmp, bytes)?;
  fs::rename(&tmp, &path)?;
  Ok(())
}

fn gc_rotated_files(config: &AuditConfig) -> Result<()> {
  if config.retain_files == 0 {
    return Ok(());
  }
  let mut files: Vec<PathBuf> = fs::read_dir(&config.dir)?
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| {
      path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with("audit-") && name.ends_with(".jsonl"))
        .unwrap_or(false)
    })
    .collect();
  files.sort();
  if files.len() <= config.retain_files {
    return Ok(());
  }
  let remove_count = files.len().saturating_sub(config.retain_files);
  for path in files.into_iter().take(remove_count) {
    let _ = fs::remove_file(path);
  }
  Ok(())
}

fn hex_lower(bytes: &[u8]) -> String {
  let mut out = String::with_capacity(bytes.len() * 2);
  for byte in bytes {
    out.push_str(&format!("{byte:02x}"));
  }
  out
}

fn truncate_msg(mut value: String, max_len: usize) -> String {
  if value.len() > max_len {
    value.truncate(max_len);
    value.push_str("...(truncated)");
  }
  value
}

fn current_time_ms() -> u128 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|value| value.as_millis())
    .unwrap_or(0)
}
