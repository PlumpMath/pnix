use anyhow::{Context, Result};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::UnixStream;

#[derive(Debug, Clone)]
enum SupervisorEndpoint {
  Uds(String),
  Tls { addr: String, server_name: String },
}

#[derive(Debug, Clone)]
pub struct BatchCall {
  pub name: String,
  pub inputs: Value,
  pub caps: Vec<String>,
}

impl BatchCall {
  pub fn new(name: impl Into<String>, inputs: Value) -> Self {
    Self {
      name: name.into(),
      inputs,
      caps: Vec::new(),
    }
  }

  pub fn with_caps(mut self, caps: &[&str]) -> Self {
    self.caps = caps.iter().map(|cap| (*cap).to_string()).collect();
    self
  }
}

pub struct SupervisorClient {
  endpoint: SupervisorEndpoint,
  next_id: AtomicU64,
  timeout: Duration,
}

impl SupervisorClient {
  pub fn connect(endpoint: impl Into<String>) -> Result<Self> {
    Ok(Self {
      endpoint: parse_endpoint(&endpoint.into())?,
      next_id: AtomicU64::new(1),
      timeout: Duration::from_millis(2_000),
    })
  }

  pub fn with_timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  pub fn call(&self, name: &str, inputs: Value) -> Result<Value> {
    self.call_with(name, &[], inputs)
  }

  pub fn call_with(&self, name: &str, caps: &[&str], inputs: Value) -> Result<Value> {
    let id = self.next_id.fetch_add(1, Ordering::Relaxed);
    let token = std::env::var("PNIX_SUPERVISOR_TOKEN").unwrap_or_default();
    let request = if inputs.is_object() {
      json!({
        "op": "call",
        "name": name,
        "inputs": inputs,
        "id": id,
        "token": token,
        "caps": caps,
      })
    } else {
      json!({
        "op": "call",
        "name": name,
        "args": inputs,
        "id": id,
        "token": token,
        "caps": caps,
      })
    };

    let response = self.request(request)?;
    let response_id = response
      .get("id")
      .and_then(|v| v.as_u64())
      .context("supervisor response missing id")?;
    if response_id != id {
      anyhow::bail!(
        "supervisor response id mismatch: expected {}, got {}",
        id,
        response_id
      );
    }

    let status = response
      .get("status")
      .and_then(|v| v.as_str())
      .unwrap_or("error");
    if !status.eq_ignore_ascii_case("ok") {
      anyhow::bail!("supervisor call {} failed: {}", name, response);
    }

    response
      .get("outputs")
      .or_else(|| response.get("result"))
      .cloned()
      .context("supervisor response missing outputs/result")
  }

  pub fn call_batch(&self, calls: &[BatchCall]) -> Result<Vec<Value>> {
    let id = self.next_id.fetch_add(1, Ordering::Relaxed);
    let token = std::env::var("PNIX_SUPERVISOR_TOKEN").unwrap_or_default();
    let payload_calls = calls
      .iter()
      .map(|call| {
        json!({
          "name": call.name,
          "inputs": call.inputs,
          "caps": call.caps,
        })
      })
      .collect::<Vec<_>>();
    let request = json!({
      "op": "batch",
      "id": id,
      "token": token,
      "inputs": {
        "calls": payload_calls,
      },
    });

    let response = self.request(request)?;
    let response_id = response
      .get("id")
      .and_then(|v| v.as_u64())
      .context("supervisor batch response missing id")?;
    if response_id != id {
      anyhow::bail!(
        "supervisor batch response id mismatch: expected {}, got {}",
        id,
        response_id
      );
    }
    let status = response
      .get("status")
      .and_then(|v| v.as_str())
      .unwrap_or("error");
    if !status.eq_ignore_ascii_case("ok") {
      anyhow::bail!("supervisor batch failed: {}", response);
    }
    let results = response
      .get("results")
      .and_then(|v| v.as_array())
      .context("supervisor batch response missing results")?;
    Ok(results.clone())
  }

  pub fn hello(&self) -> Result<Value> {
    let id = self.next_id.fetch_add(1, Ordering::Relaxed);
    let response = self.request(json!({
      "op": "hello",
      "id": id,
      "proto": ["pnix-rpc@0.1"],
      "enc": ["jsonl"],
      "features": ["batch", "watch", "events"]
    }))?;
    let response_id = response
      .get("id")
      .and_then(|v| v.as_u64())
      .context("supervisor hello response missing id")?;
    if response_id != id {
      anyhow::bail!(
        "supervisor hello response id mismatch: expected {}, got {}",
        id,
        response_id
      );
    }
    let status = response
      .get("status")
      .and_then(|v| v.as_str())
      .unwrap_or("error");
    if !status.eq_ignore_ascii_case("ok") {
      anyhow::bail!("supervisor hello failed: {}", response);
    }
    response
      .get("result")
      .cloned()
      .or_else(|| response.get("outputs").cloned())
      .context("supervisor hello response missing result")
  }

  pub fn list(&self) -> Result<Vec<String>> {
    let id = self.next_id.fetch_add(1, Ordering::Relaxed);
    let response = self.request(json!({ "op": "list", "id": id }))?;

    let response_id = response
      .get("id")
      .and_then(|v| v.as_u64())
      .context("supervisor response missing id")?;
    if response_id != id {
      anyhow::bail!(
        "supervisor response id mismatch: expected {}, got {}",
        id,
        response_id
      );
    }

    let status = response
      .get("status")
      .and_then(|v| v.as_str())
      .unwrap_or("error");
    if !status.eq_ignore_ascii_case("ok") {
      anyhow::bail!("supervisor list failed: {}", response);
    }

    let morphisms = response
      .get("morphisms")
      .and_then(|v| v.as_array())
      .context("supervisor list missing morphisms")?;

    Ok(
      morphisms
        .iter()
        .filter_map(|v| v.as_str().map(ToString::to_string))
        .collect(),
    )
  }

  fn request(&self, request: Value) -> Result<Value> {
    match &self.endpoint {
      SupervisorEndpoint::Uds(path) => self.request_uds(path, request),
      SupervisorEndpoint::Tls { addr, server_name } => self.request_tls(addr, server_name, request),
    }
  }

  fn request_uds(&self, path: &str, request: Value) -> Result<Value> {
    #[cfg(not(unix))]
    {
      let _ = (path, request);
      anyhow::bail!("UDS endpoint requires unix");
    }

    #[cfg(unix)]
    {
      let mut stream =
        UnixStream::connect(path).with_context(|| format!("connect UDS {}", path))?;
      stream.set_read_timeout(Some(self.timeout))?;
      stream.set_write_timeout(Some(self.timeout))?;
      write_request_and_read_response(&mut stream, &request)
    }
  }

  fn request_tls(&self, addr: &str, server_name: &str, request: Value) -> Result<Value> {
    let tcp = TcpStream::connect(addr).with_context(|| format!("connect TLS {}", addr))?;
    tcp.set_read_timeout(Some(self.timeout))?;
    tcp.set_write_timeout(Some(self.timeout))?;

    let tls_config = build_tls_client_config()?;
    let server_name = ServerName::try_from(server_name.to_string())
      .with_context(|| format!("invalid TLS server name `{}`", server_name))?;
    let connection =
      ClientConnection::new(tls_config, server_name).context("initialize TLS client connection")?;
    let mut stream = StreamOwned::new(connection, tcp);
    write_request_and_read_response(&mut stream, &request)
  }
}

fn write_request_and_read_response<S>(stream: &mut S, request: &Value) -> Result<Value>
where
  S: std::io::Read + std::io::Write,
{
  let request_string = serde_json::to_string(request)?;
  stream.write_all(request_string.as_bytes())?;
  stream.write_all(b"\n")?;
  stream.flush()?;

  let mut reader = BufReader::new(stream);
  let mut line = String::new();
  reader.read_line(&mut line)?;
  if line.trim().is_empty() {
    anyhow::bail!("empty supervisor response");
  }
  serde_json::from_str(&line).context("parse supervisor response")
}

fn parse_endpoint(raw: &str) -> Result<SupervisorEndpoint> {
  let value = raw.trim();
  if value.is_empty() {
    anyhow::bail!("supervisor endpoint is empty");
  }
  if let Some(path) = value.strip_prefix("uds:") {
    if path.trim().is_empty() {
      anyhow::bail!("invalid uds endpoint `{}`", value);
    }
    return Ok(SupervisorEndpoint::Uds(path.trim().to_string()));
  }
  if let Some(addr) = value.strip_prefix("tls://") {
    let (addr, server_name) = parse_tls_addr(addr)?;
    return Ok(SupervisorEndpoint::Tls { addr, server_name });
  }
  if let Some(addr) = value.strip_prefix("tls:") {
    let (addr, server_name) = parse_tls_addr(addr)?;
    return Ok(SupervisorEndpoint::Tls { addr, server_name });
  }
  Ok(SupervisorEndpoint::Uds(value.to_string()))
}

fn parse_tls_addr(raw: &str) -> Result<(String, String)> {
  let addr = raw.trim().trim_start_matches("//").trim();
  if addr.is_empty() {
    anyhow::bail!("invalid tls endpoint `{}`", raw);
  }

  let server_name = if let Some(stripped) = addr.strip_prefix('[') {
    let end = stripped
      .find(']')
      .context("invalid tls endpoint: missing `]` for IPv6 host")?;
    stripped[..end].to_string()
  } else {
    addr
      .split(':')
      .next()
      .unwrap_or_default()
      .trim()
      .to_string()
  };

  if server_name.is_empty() {
    anyhow::bail!("invalid tls endpoint `{}`: missing host", raw);
  }
  Ok((addr.to_string(), server_name))
}

fn build_tls_client_config() -> Result<Arc<ClientConfig>> {
  let ca_path = std::env::var("PNIX_SUPERVISOR_TLS_CA")
    .context("PNIX_SUPERVISOR_TLS_CA is required for tls:// supervisor endpoint")?;
  let roots = load_root_store(Path::new(&ca_path))?;

  let cert_path = std::env::var("PNIX_SUPERVISOR_TLS_CERT").ok();
  let key_path = std::env::var("PNIX_SUPERVISOR_TLS_KEY").ok();

  let config = match (cert_path, key_path) {
    (Some(cert), Some(key)) => {
      let certs = load_cert_chain(Path::new(&cert))?;
      let key = load_private_key(Path::new(&key))?;
      ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(certs, key)
        .context("build tls client config with mTLS")?
    }
    (None, None) => ClientConfig::builder()
      .with_root_certificates(roots)
      .with_no_client_auth(),
    _ => anyhow::bail!(
      "PNIX_SUPERVISOR_TLS_CERT and PNIX_SUPERVISOR_TLS_KEY must be set together for mTLS"
    ),
  };

  Ok(Arc::new(config))
}

fn load_cert_chain(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
  let cert = fs::read(path).with_context(|| format!("read cert file {}", path.display()))?;
  let cert_chain = vec![CertificateDer::from(cert)];
  if cert_chain.is_empty() {
    anyhow::bail!("certificate file is empty: {}", path.display());
  }
  Ok(cert_chain)
}

fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
  let key_bytes = fs::read(path).with_context(|| format!("read key file {}", path.display()))?;
  PrivateKeyDer::try_from(key_bytes)
    .map_err(|error| {
      anyhow::anyhow!(
        "{}: parse key file {} (expected DER encoded PKCS#8/SEC1/PKCS#1 key)",
        error,
        path.display()
      )
    })
    .with_context(|| format!("invalid key material in {}", path.display()))
}

fn load_root_store(path: &Path) -> Result<RootCertStore> {
  let cert_chain = load_cert_chain(path)?;
  let mut roots = RootCertStore::empty();
  for cert in cert_chain {
    roots
      .add(cert)
      .with_context(|| format!("add CA cert from {}", path.display()))?;
  }
  Ok(roots)
}
