use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::backend_catalog::BackendCatalog;

pub struct BackendSupervisor {
  client: pnix_runtime_supervisor::client::SupervisorClient,
  catalog: BackendCatalog,
  urls: Arc<Mutex<HashMap<String, String>>>,
}

impl BackendSupervisor {
  pub fn new(sock: String, catalog: BackendCatalog) -> Result<Self> {
    let client = pnix_runtime_supervisor::client::SupervisorClient::connect(sock)
      .context("connect supervisor client")?;
    let supervisor = Self {
      client,
      catalog,
      urls: Arc::new(Mutex::new(HashMap::new())),
    };
    supervisor.refresh_cached_urls();
    Ok(supervisor)
  }

  pub fn from_file(sock: String, spec_path: &Path) -> Result<Self> {
    let catalog = BackendCatalog::from_file(spec_path)?;
    Self::new(sock, catalog)
  }

  pub fn has_backend(&self, backend_name: &str) -> bool {
    self.catalog.has_backend(backend_name)
  }

  pub fn ensure_backend(&self, backend_name: &str) -> Result<Value> {
    let spec = self
      .catalog
      .spec(backend_name)
      .with_context(|| format!("missing backend spec `{}`", backend_name))?;
    let handle = self
      .client
      .call_with(
        "process.ensure",
        &["ProcessSpawn", "ProcessObserve"],
        serde_json::json!({ "spec": spec }),
      )
      .with_context(|| format!("ensure backend `{}`", backend_name))?;

    if let Some(url) = extract_base_url(&handle) {
      self
        .urls
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(backend_name.to_string(), url);
    }

    Ok(handle)
  }

  pub fn backend_id(&self, backend_name: &str) -> Option<String> {
    self.catalog.spec_id(backend_name).map(ToString::to_string)
  }

  pub fn base_url(&self, backend_name: &str) -> Option<String> {
    self
      .urls
      .lock()
      .unwrap_or_else(|e| e.into_inner())
      .get(backend_name)
      .cloned()
  }

  pub fn refresh_cached_urls(&self) {
    for (backend_name, _spec) in self.catalog.backends() {
      let Some(logical_id) = self.catalog.spec_id(backend_name) else {
        continue;
      };
      let handle = self.client.call_with(
        "process.handle.by_id",
        &["ProcessObserve"],
        serde_json::json!({ "id": logical_id }),
      );
      let Ok(handle) = handle else {
        continue;
      };
      if let Some(url) = extract_base_url(&handle) {
        self
          .urls
          .lock()
          .unwrap_or_else(|e| e.into_inner())
          .insert(backend_name.to_string(), url);
      }
    }
  }

  pub fn logs_tail(&self, handle: &Value, lines: u64) -> Result<Value> {
    self.client.call_with(
      "process.logs.tail",
      &["ProcessObserve"],
      serde_json::json!({ "handle": handle, "n": lines, "stream": "all" }),
    )
  }

  pub fn logs_tail_by_id(&self, backend_name: &str, lines: u64) -> Result<Value> {
    let logical_id = self
      .backend_id(backend_name)
      .with_context(|| format!("backend `{}` is missing spec.id", backend_name))?;
    self.client.call_with(
      "process.logs.tail.by_id",
      &["ProcessObserve"],
      serde_json::json!({ "id": logical_id, "n": lines, "stream": "all" }),
    )
  }
}

fn extract_base_url(handle: &Value) -> Option<String> {
  handle
    .get("base_url")
    .and_then(|v| v.as_str())
    .map(ToString::to_string)
    .or_else(|| {
      handle
        .get("http")
        .and_then(|v| v.get("base_url"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
    })
}
