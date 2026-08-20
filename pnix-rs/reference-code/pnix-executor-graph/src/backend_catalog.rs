use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct BackendSpecsFile {
  backends: Vec<BackendEntry>,
}

#[derive(Debug, Deserialize)]
struct BackendEntry {
  name: String,
  spec: Value,
}

#[derive(Clone, Debug)]
pub struct BackendCatalog {
  specs: HashMap<String, Value>,
}

impl BackendCatalog {
  pub fn load(
    explicit_path: Option<&Path>,
    env_path: Option<&Path>,
    default_path: Option<&Path>,
  ) -> Result<Option<Self>> {
    if let Some(path) = explicit_path {
      return Ok(Some(Self::from_file(path)?));
    }
    if let Some(path) = env_path {
      return Ok(Some(Self::from_file(path)?));
    }
    if let Some(path) = default_path {
      if path.exists() {
        return Ok(Some(Self::from_file(path)?));
      }
    }
    Ok(None)
  }

  pub fn from_file(path: &Path) -> Result<Self> {
    let text = std::fs::read_to_string(path)
      .with_context(|| format!("read backend specs: {}", path.display()))?;
    let parsed: BackendSpecsFile =
      serde_json::from_str(&text).context("parse backend specs json")?;
    let mut specs = HashMap::new();
    let mut seen = HashSet::new();
    for entry in parsed.backends {
      validate_backend_name(&entry.name)
        .with_context(|| format!("validate backend name `{}`", entry.name))?;
      validate_backend_spec(&entry.name, &entry.spec)
        .with_context(|| format!("validate backend spec `{}`", entry.name))?;
      if !seen.insert(entry.name.clone()) {
        bail!("duplicate backend name `{}`", entry.name);
      }
      specs.insert(entry.name, entry.spec);
    }
    Ok(Self { specs })
  }

  pub fn spec(&self, backend_name: &str) -> Option<&Value> {
    self.specs.get(backend_name)
  }

  pub fn spec_id(&self, backend_name: &str) -> Option<&str> {
    self
      .spec(backend_name)
      .and_then(|spec| spec.get("id"))
      .and_then(|value| value.as_str())
  }

  pub fn has_backend(&self, backend_name: &str) -> bool {
    self.specs.contains_key(backend_name)
  }

  pub fn backends(&self) -> impl Iterator<Item = (&str, &Value)> {
    self.specs.iter().map(|(k, v)| (k.as_str(), v))
  }

  pub fn source_hint(explicit_path: Option<&Path>, env_path: Option<&Path>) -> Option<PathBuf> {
    explicit_path
      .map(Path::to_path_buf)
      .or_else(|| env_path.map(Path::to_path_buf))
      .or_else(|| {
        let default = PathBuf::from("config/backends.json");
        if default.exists() {
          Some(default)
        } else {
          None
        }
      })
  }
}

fn validate_backend_name(name: &str) -> Result<()> {
  let mut chars = name.chars();
  let Some(first) = chars.next() else {
    bail!("name must not be empty");
  };
  if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
    bail!("name must start with lowercase letter or digit");
  }
  for ch in chars {
    let allowed = ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-');
    if !allowed {
      bail!("name contains invalid character `{}`", ch);
    }
  }
  Ok(())
}

fn validate_backend_spec(backend_name: &str, spec: &Value) -> Result<()> {
  let obj = spec
    .as_object()
    .ok_or_else(|| anyhow!("`{}` spec must be object", backend_name))?;

  expect_non_empty_string(
    obj.get("id"),
    &format!("`{}` spec.id must be non-empty string", backend_name),
  )?;
  let argv = obj.get("argv").and_then(Value::as_array).ok_or_else(|| {
    anyhow!(
      "`{}` spec.argv must be non-empty string array",
      backend_name
    )
  })?;
  if argv.is_empty() {
    bail!(
      "`{}` spec.argv must be non-empty string array",
      backend_name
    );
  }
  for (idx, value) in argv.iter().enumerate() {
    let arg = value
      .as_str()
      .ok_or_else(|| anyhow!("`{}` spec.argv[{}] must be string", backend_name, idx))?;
    if arg.is_empty() {
      bail!(
        "`{}` spec.argv[{}] must be non-empty string",
        backend_name,
        idx
      );
    }
  }

  if let Some(cwd) = obj.get("cwd") {
    expect_string(cwd, &format!("`{}` spec.cwd must be string", backend_name))?;
  }

  if let Some(env) = obj.get("env") {
    let env_obj = env
      .as_object()
      .ok_or_else(|| anyhow!("`{}` spec.env must be object", backend_name))?;
    for (key, value) in env_obj {
      expect_string(
        value,
        &format!("`{}` spec.env.{} must be string", backend_name, key),
      )?;
    }
  }

  if let Some(ready) = obj.get("ready") {
    validate_ready(backend_name, ready)?;
  }
  if let Some(http) = obj.get("http") {
    validate_http(backend_name, http)?;
  }
  if let Some(logs) = obj.get("logs") {
    validate_logs(backend_name, logs)?;
  }
  if let Some(reconcile) = obj.get("reconcile") {
    validate_reconcile(backend_name, reconcile)?;
  }
  if let Some(interop) = obj.get("interop") {
    validate_interop(backend_name, interop)?;
  }

  Ok(())
}

fn validate_ready(backend_name: &str, ready: &Value) -> Result<()> {
  let obj = ready
    .as_object()
    .ok_or_else(|| anyhow!("`{}` spec.ready must be object", backend_name))?;
  let ready_type = expect_non_empty_string(
    obj.get("type"),
    &format!(
      "`{}` spec.ready.type must be non-empty string",
      backend_name
    ),
  )?;
  match ready_type {
    "tcp" => {
      expect_non_empty_string(
        obj.get("host"),
        &format!(
          "`{}` spec.ready.host must be non-empty string",
          backend_name
        ),
      )?;
      let port = expect_u64(
        obj.get("port"),
        &format!("`{}` spec.ready.port must be integer", backend_name),
      )?;
      if port > 65535 {
        bail!("`{}` spec.ready.port must be <= 65535", backend_name);
      }
      if let Some(timeout_ms) = obj.get("timeout_ms") {
        let timeout = expect_u64(
          Some(timeout_ms),
          &format!("`{}` spec.ready.timeout_ms must be integer", backend_name),
        )?;
        if timeout < 1 {
          bail!("`{}` spec.ready.timeout_ms must be >= 1", backend_name);
        }
      }
    }
    "sleep" => {
      let ms = expect_u64(
        obj.get("ms"),
        &format!("`{}` spec.ready.ms must be integer", backend_name),
      )?;
      if ms < 1 {
        bail!("`{}` spec.ready.ms must be >= 1", backend_name);
      }
    }
    other => {
      bail!(
        "`{}` spec.ready.type `{}` is unsupported (allowed: tcp|sleep)",
        backend_name,
        other
      );
    }
  }
  Ok(())
}

fn validate_http(backend_name: &str, http: &Value) -> Result<()> {
  let obj = http
    .as_object()
    .ok_or_else(|| anyhow!("`{}` spec.http must be object", backend_name))?;
  expect_non_empty_string(
    obj.get("host"),
    &format!("`{}` spec.http.host must be non-empty string", backend_name),
  )?;
  let port = expect_u64(
    obj.get("port"),
    &format!("`{}` spec.http.port must be integer", backend_name),
  )?;
  if port > 65535 {
    bail!("`{}` spec.http.port must be <= 65535", backend_name);
  }
  if let Some(scheme) = obj.get("scheme") {
    let scheme = expect_non_empty_string(
      Some(scheme),
      &format!("`{}` spec.http.scheme must be string", backend_name),
    )?;
    if scheme != "http" && scheme != "https" {
      bail!(
        "`{}` spec.http.scheme `{}` is unsupported (allowed: http|https)",
        backend_name,
        scheme
      );
    }
  }
  if let Some(argv_flag) = obj.get("argv_flag") {
    expect_string(
      argv_flag,
      &format!("`{}` spec.http.argv_flag must be string", backend_name),
    )?;
  }
  if let Some(env_key) = obj.get("env_key") {
    expect_string(
      env_key,
      &format!("`{}` spec.http.env_key must be string", backend_name),
    )?;
  }
  Ok(())
}

fn validate_logs(backend_name: &str, logs: &Value) -> Result<()> {
  let obj = logs
    .as_object()
    .ok_or_else(|| anyhow!("`{}` spec.logs must be object", backend_name))?;
  if let Some(capture) = obj.get("capture") {
    if !capture.is_boolean() {
      bail!("`{}` spec.logs.capture must be boolean", backend_name);
    }
  }
  if let Some(tee) = obj.get("tee") {
    if !tee.is_boolean() {
      bail!("`{}` spec.logs.tee must be boolean", backend_name);
    }
  }
  if let Some(max_lines) = obj.get("max_lines") {
    let n = expect_u64(
      Some(max_lines),
      &format!("`{}` spec.logs.max_lines must be integer", backend_name),
    )?;
    if n < 1 {
      bail!("`{}` spec.logs.max_lines must be >= 1", backend_name);
    }
  }
  Ok(())
}

fn validate_reconcile(backend_name: &str, reconcile: &Value) -> Result<()> {
  let obj = reconcile
    .as_object()
    .ok_or_else(|| anyhow!("`{}` spec.reconcile must be object", backend_name))?;
  if let Some(drift) = obj.get("drift") {
    let drift = expect_non_empty_string(
      Some(drift),
      &format!("`{}` spec.reconcile.drift must be string", backend_name),
    )?;
    if drift != "restart" && drift != "ignore" && drift != "error" {
      bail!(
        "`{}` spec.reconcile.drift `{}` is unsupported (allowed: restart|ignore|error)",
        backend_name,
        drift
      );
    }
  }
  if let Some(grace_ms) = obj.get("grace_ms") {
    let ms = expect_u64(
      Some(grace_ms),
      &format!("`{}` spec.reconcile.grace_ms must be integer", backend_name),
    )?;
    if ms < 1 {
      bail!("`{}` spec.reconcile.grace_ms must be >= 1", backend_name);
    }
  }
  Ok(())
}

fn validate_interop(backend_name: &str, interop: &Value) -> Result<()> {
  let obj = interop
    .as_object()
    .ok_or_else(|| anyhow!("`{}` spec.interop must be object", backend_name))?;
  if let Some(runtime) = obj.get("runtime") {
    expect_string(
      runtime,
      &format!("`{}` spec.interop.runtime must be string", backend_name),
    )?;
  }
  if let Some(methods) = obj.get("methods") {
    let methods = methods
      .as_array()
      .ok_or_else(|| anyhow!("`{}` spec.interop.methods must be array", backend_name))?;
    for (idx, method) in methods.iter().enumerate() {
      expect_string(
        method,
        &format!(
          "`{}` spec.interop.methods[{}] must be string",
          backend_name, idx
        ),
      )?;
    }
  }
  if let Some(rpc) = obj.get("rpc") {
    let rpc_obj = rpc
      .as_object()
      .ok_or_else(|| anyhow!("`{}` spec.interop.rpc must be object", backend_name))?;
    if let Some(transport) = rpc_obj.get("transport") {
      let transport = expect_non_empty_string(
        Some(transport),
        &format!(
          "`{}` spec.interop.rpc.transport must be string",
          backend_name
        ),
      )?;
      if transport != "http+tcp" && transport != "http+uds" && transport != "stdio" {
        bail!(
          "`{}` spec.interop.rpc.transport `{}` is unsupported",
          backend_name,
          transport
        );
      }
    }
    if let Some(base_path) = rpc_obj.get("base_path") {
      expect_string(
        base_path,
        &format!(
          "`{}` spec.interop.rpc.base_path must be string",
          backend_name
        ),
      )?;
    }
  }
  Ok(())
}

fn expect_string<'a>(value: &'a Value, err: &str) -> Result<&'a str> {
  value.as_str().ok_or_else(|| anyhow!(err.to_string()))
}

fn expect_non_empty_string<'a>(value: Option<&'a Value>, err: &str) -> Result<&'a str> {
  let value = value.ok_or_else(|| anyhow!(err.to_string()))?;
  let s = value.as_str().ok_or_else(|| anyhow!(err.to_string()))?;
  if s.is_empty() {
    bail!("{err}");
  }
  Ok(s)
}

fn expect_u64(value: Option<&Value>, err: &str) -> Result<u64> {
  let value = value.ok_or_else(|| anyhow!(err.to_string()))?;
  value.as_u64().ok_or_else(|| anyhow!(err.to_string()))
}

#[cfg(test)]
mod tests {
  use super::BackendCatalog;
  use std::fs;
  use std::path::{Path, PathBuf};
  use std::time::{SystemTime, UNIX_EPOCH};

  fn make_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock before unix epoch")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-backend-catalog-{}-{}-{}",
      label,
      std::process::id(),
      nanos
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
  }

  fn write_fixture(path: &Path, backend_name: &str, spec_id: &str) {
    let payload = format!(
      r#"{{
  "backends": [
    {{
      "name": "{backend_name}",
      "spec": {{
        "id": "{spec_id}",
        "argv": ["echo", "{backend_name}"]
      }}
    }}
  ]
}}"#
    );
    fs::write(path, payload).expect("write backend fixture");
  }

  fn write_raw_fixture(path: &Path, payload: &str) {
    fs::write(path, payload).expect("write raw backend fixture");
  }

  #[test]
  fn load_prefers_explicit_over_env_and_default() {
    let dir = make_temp_dir("explicit-priority");
    let explicit = dir.join("explicit.json");
    let env = dir.join("env.json");
    let default = dir.join("default.json");
    write_fixture(explicit.as_path(), "explicit", "backend.explicit");
    write_fixture(env.as_path(), "env", "backend.env");
    write_fixture(default.as_path(), "default", "backend.default");

    let catalog = BackendCatalog::load(
      Some(explicit.as_path()),
      Some(env.as_path()),
      Some(default.as_path()),
    )
    .expect("load backend catalog")
    .expect("catalog should exist");

    assert!(catalog.has_backend("explicit"));
    assert!(!catalog.has_backend("env"));
    assert_eq!(catalog.spec_id("explicit"), Some("backend.explicit"));

    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn load_uses_env_when_explicit_absent() {
    let dir = make_temp_dir("env-priority");
    let env = dir.join("env.json");
    let default = dir.join("default.json");
    write_fixture(env.as_path(), "env", "backend.env");
    write_fixture(default.as_path(), "default", "backend.default");

    let catalog = BackendCatalog::load(None, Some(env.as_path()), Some(default.as_path()))
      .expect("load backend catalog")
      .expect("catalog should exist");

    assert!(catalog.has_backend("env"));
    assert!(!catalog.has_backend("default"));
    assert_eq!(catalog.spec_id("env"), Some("backend.env"));

    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn load_uses_default_when_only_default_exists() {
    let dir = make_temp_dir("default-priority");
    let default = dir.join("default.json");
    write_fixture(default.as_path(), "default", "backend.default");

    let catalog = BackendCatalog::load(None, None, Some(default.as_path()))
      .expect("load backend catalog")
      .expect("catalog should exist");

    assert!(catalog.has_backend("default"));
    assert_eq!(catalog.spec_id("default"), Some("backend.default"));

    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn from_file_rejects_duplicate_backend_names() {
    let dir = make_temp_dir("duplicate-name");
    let fixture = dir.join("backend.json");
    write_raw_fixture(
      fixture.as_path(),
      r#"{
  "backends": [
    { "name": "python", "spec": { "id": "backend.python.a", "argv": ["python3"] } },
    { "name": "python", "spec": { "id": "backend.python.b", "argv": ["python3"] } }
  ]
}"#,
    );

    let result = BackendCatalog::from_file(fixture.as_path());
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(err.contains("duplicate backend name"));

    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn from_file_rejects_invalid_spec_shape() {
    let dir = make_temp_dir("invalid-spec");
    let fixture = dir.join("backend.json");
    write_raw_fixture(
      fixture.as_path(),
      r#"{
  "backends": [
    {
      "name": "Python",
      "spec": {
        "id": "",
        "argv": [],
        "ready": { "type": "tcp", "port": 70000 }
      }
    }
  ]
}"#,
    );

    let result = BackendCatalog::from_file(fixture.as_path());
    assert!(result.is_err());
    let err = result.err().unwrap().to_string();
    assert!(err.contains("validate backend name"));

    let _ = fs::remove_dir_all(dir);
  }
}
