use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use pnix_hash::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::canon::canonicalize_value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayMode {
  Off,
  Strict,
  NondetSafe,
  Verify,
}

impl ReplayMode {
  pub fn parse(raw: Option<&str>) -> Result<Self> {
    match raw.unwrap_or("nondet-safe").to_ascii_lowercase().as_str() {
      "off" => Ok(Self::Off),
      "strict" => Ok(Self::Strict),
      "nondet-safe" | "nondetsafe" => Ok(Self::NondetSafe),
      "verify" => Ok(Self::Verify),
      other => anyhow::bail!(
        "unknown replay mode `{}` (use off|strict|nondet-safe|verify)",
        other
      ),
    }
  }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct TraceLine {
  pub node: String,
  pub uses: String,
  #[serde(default)]
  pub status: String,
  #[serde(default)]
  pub input: Value,
  #[serde(default)]
  pub output: Value,
  #[serde(default)]
  pub replay_key: Option<String>,
  #[serde(default)]
  pub invocation_id: Option<String>,
  #[serde(default)]
  pub origin: Option<String>,
  #[serde(default)]
  pub nondet: Option<bool>,
  #[serde(default)]
  pub replay_class: Option<String>,
  #[serde(default)]
  pub meta: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ReplayEntry {
  pub node: String,
  pub uses: String,
  pub input_canon: Value,
  pub output: Value,
  pub replay_key: Option<String>,
}

#[derive(Debug, Default)]
pub struct ReplayDB {
  pub by_replay_key: HashMap<String, ReplayEntry>,
  pub by_node: HashMap<String, ReplayEntry>,
  pub by_key: HashMap<String, ReplayEntry>,
}

#[derive(Debug)]
pub struct ReplayConfig {
  pub mode: ReplayMode,
  pub trace_path: String,
  pub db: ReplayDB,
  pub allow_classes: HashSet<String>,
}

impl ReplayDB {
  pub fn load(path: &Path) -> Result<Self> {
    let text = std::fs::read_to_string(path)
      .with_context(|| format!("read replay trace {}", path.display()))?;
    let mut db = Self::default();

    for (index, raw_line) in text.lines().enumerate() {
      let line = raw_line.trim();
      if line.is_empty() {
        continue;
      }
      let parsed: TraceLine = serde_json::from_str(line)
        .with_context(|| format!("parse replay trace line {}", index + 1))?;
      if !parsed.status.eq_ignore_ascii_case("ok") {
        continue;
      }

      let input_canon = canonicalize_value(&parsed.input);
      let replay_key = parsed
        .replay_key
        .clone()
        .or_else(|| replay_key_from_meta(parsed.meta.as_ref()));
      let entry = ReplayEntry {
        node: parsed.node.clone(),
        uses: parsed.uses.clone(),
        input_canon: input_canon.clone(),
        output: parsed.output.clone(),
        replay_key: replay_key.clone(),
      };

      if let Some(key) = replay_key {
        db.by_replay_key.insert(key, entry.clone());
      }
      db.by_node.insert(parsed.node, entry.clone());
      db.by_key
        .insert(replay_fallback_key(&entry.uses, &input_canon), entry);
    }

    Ok(db)
  }

  pub fn lookup<'a>(
    &'a self,
    node_name: &str,
    uses: &str,
    input_canon: &Value,
    replay_key: Option<&str>,
  ) -> Option<&'a ReplayEntry> {
    if let Some(key) = replay_key {
      if let Some(entry) = self.by_replay_key.get(key) {
        return Some(entry);
      }
    }
    if let Some(entry) = self.by_node.get(node_name) {
      return Some(entry);
    }
    let key = replay_fallback_key(uses, input_canon);
    self.by_key.get(&key)
  }

  pub fn contains_replay_key(&self, replay_key: &str) -> bool {
    self.by_replay_key.contains_key(replay_key)
  }
}

pub fn replay_fallback_key(uses: &str, input_canon: &Value) -> String {
  let payload = serde_json::json!({
    "uses": uses,
    "input": input_canon,
  });
  let bytes = serde_json::to_vec(&payload).unwrap_or_default();
  let mut hasher = Sha256::new();
  hasher.update(bytes);
  let digest = hasher.finalize();
  let mut hex = String::with_capacity(digest.len() * 2);
  for byte in digest {
    hex.push_str(&format!("{:02x}", byte));
  }
  format!("sha256:{}", hex)
}

fn replay_key_from_meta(meta: Option<&Value>) -> Option<String> {
  let obj = meta?.as_object()?;
  obj
    .get("replay_key")
    .and_then(|v| v.as_str())
    .map(ToString::to_string)
}
