//! Phase 145++++++++++(zzzzd-0.5) (vpl-gate.md 10.7 헌법, 2026-06-01):
//! pnix coding-agent retention CLI 가 *실제로 doghouse relay 경유*
//! 로만 동작함을 testlock. fake doghouse HTTP server 를 띄우고 CLI 가
//! 그 서버에 정확한 closed-action body 만 보내는지 검증.
//!
//! Requirements: cargo build --release --bin pnix (release binary).
//!
//! Coverage (4 cases):
//!   1. happy path JSON  — fake relay receives exact body shape +
//!      CLI prints BridgeReceipt JSON
//!   2. happy path text  — same body + CLI text output has required
//!      fields (substrate-action / retention-policy-id /
//!      total-artifact-count / byte-io-op-count)
//!   3. dry-run          — fake relay receives 0 requests (CLI bails
//!      before HTTP)
//!   4. non-2xx          — fake relay returns 503; CLI bail message
//!      includes status + body

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn workspace_root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(|p| p.parent())
    .expect("workspace root")
    .to_path_buf()
}

fn pnix_binary() -> PathBuf {
  workspace_root().join("target/release/pnix")
}

#[derive(Debug, Default, Clone)]
struct FakeRequestLog {
  requests: Vec<RecordedRequest>,
}

#[derive(Debug, Clone)]
struct RecordedRequest {
  method: String,
  target: String,
  body: String,
}

#[derive(Clone)]
struct FakeRelayConfig {
  status_line: String,
  response_body: String,
}

/// Spawn a fake doghouse relay on a free port. Returns (url, log
/// handle, shutdown_signal). The relay accepts ONE POST then closes.
fn spawn_fake_relay(cfg: FakeRelayConfig) -> (String, Arc<Mutex<FakeRequestLog>>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake relay");
  let port = listener.local_addr().unwrap().port();
  let url = format!("http://127.0.0.1:{}", port);
  let log = Arc::new(Mutex::new(FakeRequestLog::default()));
  let log_clone = Arc::clone(&log);
  thread::spawn(move || {
    // accept up to 1 connection; tests that intend zero connections
    // (dry-run) just don't connect at all.
    listener.set_nonblocking(false).ok();
    if let Ok((mut stream, _)) = listener.accept() {
      handle_one(&mut stream, &cfg, &log_clone);
    }
  });
  (url, log)
}

fn handle_one(stream: &mut TcpStream, cfg: &FakeRelayConfig, log: &Arc<Mutex<FakeRequestLog>>) {
  let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
  let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

  let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
  let mut request_line = String::new();
  if reader.read_line(&mut request_line).is_err() {
    return;
  }
  let mut parts = request_line.split_whitespace();
  let method = parts.next().unwrap_or("").to_string();
  let target = parts.next().unwrap_or("").to_string();

  // Read headers until empty line; extract Content-Length.
  let mut content_length: usize = 0;
  loop {
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
      return;
    }
    if line == "\r\n" || line.is_empty() {
      break;
    }
    if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
      content_length = rest.trim().parse().unwrap_or(0);
    }
  }
  let mut body = vec![0u8; content_length];
  if reader.read_exact(&mut body).is_err() {
    return;
  }
  let body_str = String::from_utf8_lossy(&body).to_string();

  log.lock().unwrap().requests.push(RecordedRequest {
    method,
    target,
    body: body_str,
  });

  let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        cfg.status_line,
        cfg.response_body.len(),
        cfg.response_body
    );
  let _ = stream.write_all(response.as_bytes());
}

fn canned_bridge_receipt_json() -> String {
  // Shape mirror of pnixc_meta::store::BridgeReceipt.
  let body = serde_json::json!({
      "lens_path": "stdlib/lib/gate/coding-memory-retention-plan.px",
      "lens_verdict": {
          "verdict": "retention-plan-built",
          "policy": {
              "policy_id": "pnixc-meta.coding-memory-retention.v1",
              "compact_after_ms": 2_592_000_000u64,
              "max_artifacts_per_family": 128,
          },
          "evaluated_at_ms": 1_800_000_000_000u64,
          "total_artifact_count": 3,
          "summary": {
              "keep_count": 1,
              "compact_candidate_count": 2,
              "protected_count": 0,
          },
          "proof_refs": [
              "compact-after-ms:2592000000",
              "compact-candidate-count:2",
              "retention-policy-id:pnixc-meta.coding-memory-retention.v1",
          ],
      },
      "execution": {
          "plan_id": "coding-retention-plan::pnixc-meta-coding-memory-retention-v1::1800000000000",
          "successful_op_count": 3,
          "read_results": [],
      }
  });
  serde_json::to_string(&body).unwrap()
}

fn run_pnix_retention(env_url: Option<&str>, args: &[&str]) -> (i32, String, String) {
  let bin = pnix_binary();
  assert!(
    bin.exists(),
    "release binary missing at {:?} — run `cargo build --release --bin pnix` first",
    bin
  );
  let mut cmd = Command::new(&bin);
  cmd
    .arg("coding-agent")
    .arg("retention")
    .args(args)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .env("PNIX_WORKSPACE_ROOT", workspace_root());
  if let Some(u) = env_url {
    cmd.env("PNIX_GATE_DOGHOUSE_URL", u);
  } else {
    cmd.env_remove("PNIX_GATE_DOGHOUSE_URL");
  }
  let out = cmd.output().expect("spawn pnix coding-agent retention");
  (
    out.status.code().unwrap_or(-1),
    String::from_utf8_lossy(&out.stdout).to_string(),
    String::from_utf8_lossy(&out.stderr).to_string(),
  )
}

#[test]
fn retention_json_path_posts_closed_body_and_prints_bridge_receipt() {
  let cfg = FakeRelayConfig {
    status_line: "200 OK".to_string(),
    response_body: canned_bridge_receipt_json(),
  };
  let (url, log) = spawn_fake_relay(cfg);

  let (code, stdout, stderr) = run_pnix_retention(Some(&url), &["--output-format", "json"]);
  assert_eq!(code, 0, "stderr: {}", stderr);

  let req = {
    let l = log.lock().unwrap();
    assert_eq!(l.requests.len(), 1, "fake relay received 0 requests");
    l.requests[0].clone()
  };

  // (1) request shape
  assert_eq!(req.method, "POST");
  assert_eq!(req.target, "/store/execute-native-action");

  let body: serde_json::Value =
    serde_json::from_str(&req.body).expect("relay received non-JSON body");
  let obj = body.as_object().expect("body must be object");

  // exactly the two allowed keys
  assert_eq!(
    obj.len(),
    2,
    "body keys: {:?}",
    obj.keys().collect::<Vec<_>>()
  );
  assert_eq!(obj["action_name"], "coding-memory-retention");
  assert!(obj["evaluated_at_ms"].is_u64());

  // forbidden keys absent
  for k in &[
    "policy",
    "lens_path",
    "entry_fn",
    "request_nix_expr",
    "store_path",
  ] {
    assert!(
      !obj.contains_key(*k),
      "forbidden key `{}` must NOT be in CLI request body",
      k
    );
  }

  // (2) CLI output is the BridgeReceipt JSON
  let printed: serde_json::Value =
    serde_json::from_str(stdout.trim()).expect("CLI stdout not JSON");
  assert_eq!(
    printed["lens_verdict"]["policy"]["policy_id"],
    "pnixc-meta.coding-memory-retention.v1"
  );
  assert_eq!(printed["execution"]["successful_op_count"], 3);
  assert!(!stdout.contains("doghouse"));
}

#[test]
fn retention_text_path_prints_required_fields() {
  let cfg = FakeRelayConfig {
    status_line: "200 OK".to_string(),
    response_body: canned_bridge_receipt_json(),
  };
  let (url, _log) = spawn_fake_relay(cfg);

  let (code, stdout, stderr) = run_pnix_retention(Some(&url), &[]);
  assert_eq!(code, 0, "stderr: {}", stderr);

  // Required text fields per zzzzd-0 contract.
  for required in &[
    "substrate-action:",
    "retention-policy-id: pnixc-meta.coding-memory-retention.v1",
    "total-artifact-count: 3",
    "byte-io-op-count: 3",
  ] {
    assert!(
      stdout.contains(required),
      "text output missing `{}`. full output:\n{}",
      required,
      stdout
    );
  }
  // honesty check: no stale "planning only" wording.
  assert!(!stdout.contains("planning is append-only"));
  assert!(!stdout.contains("delete/compaction executor"));
  assert!(!stdout.contains("doghouse"));
}

#[test]
fn retention_dry_run_sends_zero_requests_and_bails_with_typed_message() {
  // Start a relay but expect 0 connections — we wait briefly afterwards.
  let cfg = FakeRelayConfig {
    status_line: "200 OK".to_string(),
    response_body: canned_bridge_receipt_json(),
  };
  let (url, log) = spawn_fake_relay(cfg);

  let (code, stdout, stderr) = run_pnix_retention(Some(&url), &["--dry-run"]);
  assert_ne!(code, 0, "dry-run must fail; stdout: {}", stdout);
  assert!(
    stderr.contains("dry-run unsupported"),
    "stderr missing typed message: {}",
    stderr
  );

  assert_eq!(
    log.lock().unwrap().requests.len(),
    0,
    "fake relay must NOT receive any requests under --dry-run"
  );
}

#[test]
fn retention_non_2xx_propagates_status_and_body() {
  let cfg = FakeRelayConfig {
    status_line: "503 Service Unavailable".to_string(),
    response_body:
      "{\"error\":\"store-not-configured: PNIXC_META_STORE_PATH env not set or empty\"}".to_string(),
  };
  let (url, _log) = spawn_fake_relay(cfg);

  let (code, stdout, stderr) = run_pnix_retention(Some(&url), &[]);
  assert_ne!(code, 0, "503 must fail; stdout: {}", stdout);
  assert!(
    stderr.contains("503"),
    "stderr must include status code: {}",
    stderr
  );
  assert!(
    stderr.contains("store-not-configured"),
    "stderr must include relay body: {}",
    stderr
  );
}
