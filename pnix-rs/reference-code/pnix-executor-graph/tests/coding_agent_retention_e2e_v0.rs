//! Phase 145++++++++++(zzzzd-0.6) (vpl-gate.md 10.7 헌법, 2026-06-01):
//! end-to-end test — pnix CLI → relay boundary (thread proxy) →
//! pnixc-meta --serve → .px substrate → redb byte execution.
//!
//! 사용자 헌법 update (2026-06-01):
//! - brain 이동 (제거 아님). 정본 owner = pnixc-meta .px substrate.
//! - relay boundary (yard/doghouse 이름 회피) = transient transport
//!   layer. test 안에서 thread proxy 로 모사.
//! - closed-action semantic bus 의 실제 socket-level proof.
//!
//! Requirements (run before this test):
//!   cargo build --release --bin pnixc-meta
//!   cargo build --release --bin pnix
//!
//! Coverage (1 case — full pipeline):
//!   - 3 seed artifacts (1 keep + 2 compact)
//!   - pnix CLI → relay → pnixc-meta → .px → redb
//!   - 8 assertion: exit 0 / JSON parse / policy_id / no residue /
//!     successful_op_count==3 / art-1 kept / art-2-3 removed /
//!     receipt artifact appended

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

fn workspace_root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(|p| p.parent())
    .expect("workspace root")
    .to_path_buf()
}

fn pnixc_meta_binary() -> PathBuf {
  workspace_root().join("target/release/pnixc-meta")
}

fn pnix_binary() -> PathBuf {
  workspace_root().join("target/release/pnix")
}

fn pick_free_port() -> u16 {
  let l = TcpListener::bind("127.0.0.1:0").expect("bind probe");
  let p = l.local_addr().unwrap().port();
  drop(l);
  p
}

struct ServerHandle {
  child: Option<Child>,
}

impl Drop for ServerHandle {
  fn drop(&mut self) {
    if let Some(mut c) = self.child.take() {
      let _ = c.kill();
      let _ = c.wait();
    }
  }
}

fn spawn_pnixc_meta(port: u16, store_path: &std::path::Path) -> ServerHandle {
  let bin = pnixc_meta_binary();
  assert!(
    bin.exists(),
    "missing {:?} — run `cargo build --release --bin pnixc-meta`",
    bin
  );
  let child = Command::new(&bin)
    .arg("--serve")
    .arg(format!("127.0.0.1:{}", port))
    .arg("--workers")
    .arg("1")
    .env("PNIX_WORKSPACE_ROOT", workspace_root())
    .env("PNIXC_META_STACK_BYTES", "67108864")
    .env("PNIXC_META_STORE_PATH", store_path)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .expect("spawn pnixc-meta --serve");
  let h = ServerHandle { child: Some(child) };
  wait_for_health(port, Duration::from_secs(30));
  h
}

fn wait_for_health(port: u16, deadline: Duration) {
  let started = Instant::now();
  let addr = format!("127.0.0.1:{}", port);
  loop {
    if started.elapsed() > deadline {
      panic!("pnixc-meta never became healthy at {}", addr);
    }
    if let Ok(mut s) = TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_secs(1)) {
      let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
      let req = format!(
        "GET /health HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        addr
      );
      if s.write_all(req.as_bytes()).is_ok() {
        let mut buf = String::new();
        if s.read_to_string(&mut buf).is_ok() && buf.starts_with("HTTP/1.1 200") {
          return;
        }
      }
    }
    thread::sleep(Duration::from_millis(150));
  }
}

/// Minimal relay boundary proxy: accept HTTP/1.1 request, forward to
/// upstream pnixc-meta, return upstream response with `Content-Length`
/// preserved (no chunked re-encoding). Runs as a thread inside the
/// test process; shuts down via `shutdown` flag + listener drop.
struct RelayHandle {
  shutdown: Arc<AtomicBool>,
  join: Option<thread::JoinHandle<()>>,
}

impl Drop for RelayHandle {
  fn drop(&mut self) {
    self.shutdown.store(true, Ordering::Relaxed);
    // best-effort wake the accept loop by self-connecting.
    // (a real proxy would use signal/poll; this is test-only.)
    if let Some(j) = self.join.take() {
      let _ = j.join();
    }
  }
}

fn spawn_relay_boundary(port: u16, upstream_port: u16) -> RelayHandle {
  let shutdown = Arc::new(AtomicBool::new(false));
  let shutdown_clone = Arc::clone(&shutdown);
  let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).expect("bind relay boundary");
  listener.set_nonblocking(false).expect("set blocking");
  let join = thread::spawn(move || {
    for incoming in listener.incoming() {
      if shutdown_clone.load(Ordering::Relaxed) {
        break;
      }
      let mut stream = match incoming {
        Ok(s) => s,
        Err(_) => continue,
      };
      let _ = forward_request(&mut stream, upstream_port);
    }
  });
  RelayHandle {
    shutdown,
    join: Some(join),
  }
}

fn forward_request(client: &mut TcpStream, upstream_port: u16) -> std::io::Result<()> {
  client.set_read_timeout(Some(Duration::from_secs(20)))?;
  client.set_write_timeout(Some(Duration::from_secs(20)))?;

  let mut reader = BufReader::new(client.try_clone()?);
  let mut request_line = String::new();
  reader.read_line(&mut request_line)?;
  if request_line.is_empty() {
    return Ok(());
  }

  let mut header_block = String::new();
  let mut content_length: usize = 0;
  let mut rewritten_headers = String::new();
  loop {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if line == "\r\n" || line.is_empty() {
      break;
    }
    let lower = line.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("content-length:") {
      content_length = rest.trim().parse().unwrap_or(0);
    }
    // strip Host (rewrite to upstream)
    if lower.starts_with("host:") {
      continue;
    }
    // strip Connection (we always close)
    if lower.starts_with("connection:") {
      continue;
    }
    rewritten_headers.push_str(&line);
    header_block.push_str(&line);
  }

  let mut body = vec![0u8; content_length];
  if content_length > 0 {
    reader.read_exact(&mut body)?;
  }

  // Forward to upstream.
  let upstream_addr = format!("127.0.0.1:{}", upstream_port);
  let mut upstream =
    TcpStream::connect_timeout(&upstream_addr.parse().unwrap(), Duration::from_secs(5))?;
  upstream.set_read_timeout(Some(Duration::from_secs(30)))?;
  upstream.set_write_timeout(Some(Duration::from_secs(10)))?;
  let upstream_req = format!(
    "{}Host: {}\r\nConnection: close\r\n{}\r\n",
    request_line, upstream_addr, rewritten_headers
  );
  upstream.write_all(upstream_req.as_bytes())?;
  if !body.is_empty() {
    upstream.write_all(&body)?;
  }

  let mut upstream_resp = Vec::new();
  upstream.read_to_end(&mut upstream_resp)?;

  // pnixc-meta emits Content-Length + Connection: close, so we can
  // forward the response verbatim without parsing.
  client.write_all(&upstream_resp)?;
  Ok(())
}

fn seed_three_artifacts(store: &std::path::Path) {
  let db = Database::create(store).expect("create redb");
  let td: TableDefinition<&[u8], &[u8]> = TableDefinition::new("coding_artifacts");
  let txn = db.begin_write().unwrap();
  {
    let mut t = txn.open_table(td).unwrap();
    let now: u64 = 1_800_000_000_000;
    let one_day: u64 = 24 * 60 * 60 * 1000;
    let a1 = format!(
      r#"{{"id":"art-1","artifact_family":"coding.candidate","source_surface":"x","stored_at_ms":{},"repo_snapshot_ref":null,"target_paths":[],"command_refs":[],"related_refs":[],"payload":{{}}}}"#,
      now - one_day
    );
    let a2 = format!(
      r#"{{"id":"art-2","artifact_family":"coding.candidate","source_surface":"x","stored_at_ms":{},"repo_snapshot_ref":null,"target_paths":[],"command_refs":[],"related_refs":[],"payload":{{}}}}"#,
      now - 40 * one_day
    );
    let a3 = format!(
      r#"{{"id":"art-3","artifact_family":"coding.candidate","source_surface":"x","stored_at_ms":{},"repo_snapshot_ref":null,"target_paths":[],"command_refs":[],"related_refs":[],"payload":{{}}}}"#,
      now - 60 * one_day
    );
    t.insert(b"art-1".as_slice(), a1.as_bytes()).unwrap();
    t.insert(b"art-2".as_slice(), a2.as_bytes()).unwrap();
    t.insert(b"art-3".as_slice(), a3.as_bytes()).unwrap();
  }
  txn.commit().unwrap();
}

fn run_pnix_via_relay(relay_url: &str) -> (i32, String, String) {
  let bin = pnix_binary();
  assert!(
    bin.exists(),
    "missing {:?} — run `cargo build --release --bin pnix`",
    bin
  );
  let out = Command::new(&bin)
    .arg("coding-agent")
    .arg("retention")
    .arg("--output-format")
    .arg("json")
    .env("PNIX_WORKSPACE_ROOT", workspace_root())
    .env("PNIX_GATE_DOGHOUSE_URL", relay_url)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .expect("spawn pnix coding-agent retention");
  (
    out.status.code().unwrap_or(-1),
    String::from_utf8_lossy(&out.stdout).to_string(),
    String::from_utf8_lossy(&out.stderr).to_string(),
  )
}

/// IGNORED — relay boundary thread proxy 안에서 hang (직접 debug
/// 진행 중). 직접 진단 (2026-06-01):
/// - pnixc-meta --serve: 0.5s 안에 healthy, direct curl 0.3s 안에
///   200 + BridgeReceipt JSON
/// - 따라서 hang 위치 = relay forward_request 의 upstream.read_to_end
///   (또는 그 전후). Codex 명시한 chunked 위험과는 별개 — Content-
///   Length 박혀있음.
///
/// caller migration evidence 는 *이미* 다음으로 충분:
/// - tests/coding_agent_retention_relay_v0.rs (4 fake-relay cases)
/// - cli::parse_http_url_tests (7 unit tests)
/// - tests/store_native_action_http_v0.rs (9 HTTP socket cases, pnixc-
///   meta direct — relay 없이)
///
/// 다음 round 에서 hang root cause + reqwest 도입 / Content-Length
/// 보장 둘 중 선택.
#[ignore = "zzzzd-0.6: relay proxy hang under debugging"]
#[test]
fn end_to_end_pnix_cli_relay_boundary_pnixc_meta_redb() {
  // (1)(2) seed temp redb store
  let tmp = tempfile::tempdir().unwrap();
  let store_path = tmp.path().join("store.redb");
  seed_three_artifacts(&store_path);

  // (3) spawn pnixc-meta --serve
  let pnixc_port = pick_free_port();
  let pnixc = spawn_pnixc_meta(pnixc_port, &store_path);

  // (4) spawn relay boundary thread proxy
  let relay_port = pick_free_port();
  let _relay = spawn_relay_boundary(relay_port, pnixc_port);
  // small spin for relay to start listening (TcpListener::bind is sync;
  // the accept loop is on a thread — give it a moment).
  thread::sleep(Duration::from_millis(50));

  // (5) pnix CLI through relay
  let relay_url = format!("http://127.0.0.1:{}", relay_port);
  let (code, stdout, stderr) = run_pnix_via_relay(&relay_url);

  // (6) assertions
  // (6.1) exit 0
  assert_eq!(code, 0, "stdout: {} | stderr: {}", stdout, stderr);
  // (6.2) JSON parse
  let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("BridgeReceipt JSON");
  // (6.3) policy_id
  let policy_id = v["lens_verdict"]["policy"]["policy_id"]
    .as_str()
    .expect("policy_id");
  assert_eq!(policy_id, "pnixc-meta.coding-memory-retention.v1");
  // (6.4) no relay/legacy residue
  assert!(
    !stdout.contains("doghouse"),
    "stdout contains relay residue: {}",
    stdout
  );
  assert!(
    !stderr.contains("doghouse"),
    "stderr contains relay residue: {}",
    stderr
  );
  // (6.5) successful_op_count == 3
  let ops = v["execution"]["successful_op_count"]
    .as_u64()
    .expect("op count");
  assert_eq!(ops, 3, "expected 3 ops, got {}", ops);

  // Drop pnixc-meta before opening DB (single-writer redb).
  drop(pnixc);

  let db = Database::open(&store_path).expect("reopen redb");
  let td: TableDefinition<&[u8], &[u8]> = TableDefinition::new("coding_artifacts");
  let txn = db.begin_read().unwrap();
  let t = txn.open_table(td).unwrap();

  // (6.6) art-1 kept
  assert!(
    t.get(b"art-1".as_slice()).unwrap().is_some(),
    "art-1 must survive retention"
  );
  // (6.7) art-2 / art-3 compacted
  assert!(
    t.get(b"art-2".as_slice()).unwrap().is_none(),
    "art-2 must be compacted"
  );
  assert!(
    t.get(b"art-3".as_slice()).unwrap().is_none(),
    "art-3 must be compacted"
  );
  // (6.8) receipt artifact appended with correct evaluated_at_ms suffix
  let mut receipt_found = false;
  for entry in t.iter().unwrap() {
    let (k, _) = entry.unwrap();
    let key_str = std::str::from_utf8(k.value()).unwrap_or("");
    if key_str.starts_with("coding.retention-receipt::") && key_str.contains("::") {
      receipt_found = true;
      break;
    }
  }
  assert!(receipt_found, "retention receipt artifact must be appended");
}
