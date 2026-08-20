//! pnix package mount/run builtins — effectful; import cache must treat as impure.
//! Execution delegates to pnixc-meta CLI, which consults
//! `stdlib/lib/gate/runtime-package-mount-gate.px` (single policy authority).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use serde_json::Value as JsonValue;

use crate::markup::json_to_value;
use crate::value::Value;

fn pnix_state_dir() -> Result<PathBuf> {
  let home = std::env::var("HOME").context("HOME not set; cannot locate pnix state dir")?;
  Ok(PathBuf::from(home).join(".local/state/pnix"))
}

fn pnix_meta_exe() -> Result<PathBuf> {
  if let Ok(p) = std::env::var("PNIXC") {
    let pb = PathBuf::from(p);
    if pb.is_file() {
      return Ok(pb);
    }
  }
  std::env::current_exe().context("cannot resolve pnixc-meta executable path")
}

fn run_pnix_meta(subargs: &[&str]) -> Result<()> {
  let exe = pnix_meta_exe()?;
  let status = Command::new(&exe)
    .args(subargs)
    .status()
    .with_context(|| format!("failed to launch {}", exe.display()))?;
  if !status.success() {
    return Err(anyhow!(
      "pnixc-meta {} failed (exit {:?})",
      subargs.join(" "),
      status.code()
    ));
  }
  Ok(())
}

fn mount_receipt_path(name: &str) -> Result<PathBuf> {
  let dir = pnix_state_dir()?.join("mounts");
  fs::create_dir_all(&dir)
    .with_context(|| format!("create mounts dir {}", dir.display()))?;
  Ok(dir.join(format!("{name}.json")))
}

fn read_mount_json(name: &str) -> Result<Value> {
  let path = mount_receipt_path(name)?;
  let text = fs::read_to_string(&path)
    .with_context(|| format!("read mount receipt {} (is `{}` mounted?)", path.display(), name))?;
  let json: JsonValue =
    serde_json::from_str(&text).with_context(|| format!("parse mount receipt {}", path.display()))?;
  Ok(json_to_value(&json))
}

fn list_mount_json() -> Result<Value> {
  let dir = pnix_state_dir()?.join("mounts");
  if !dir.is_dir() {
    return Ok(json_to_value(&serde_json::json!({
      "schema": "pnix.mounts.v0",
      "count": 0,
      "mounts": [],
    })));
  }
  let mut mounts = Vec::new();
  for entry in fs::read_dir(&dir)? {
    let entry = entry?;
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) != Some("json") {
      continue;
    }
    let text = fs::read_to_string(&path)?;
    let json: JsonValue = serde_json::from_str(&text)?;
    mounts.push(json);
  }
  mounts.sort_by(|a, b| {
    a.get("name")
      .and_then(|v| v.as_str())
      .unwrap_or("")
      .cmp(b.get("name").and_then(|v| v.as_str()).unwrap_or(""))
  });
  Ok(json_to_value(&serde_json::json!({
    "schema": "pnix.mounts.v0",
    "count": mounts.len(),
    "mounts": mounts,
  })))
}

fn expect_name(args: &[Value], builtin: &str) -> Result<String> {
  args.first()
    .and_then(|v| v.as_str())
    .map(|s| s.to_string())
    .ok_or_else(|| anyhow!("{builtin}: first argument must be a string package name"))
}

pub(crate) fn builtin_pnix_mount(args: &[Value]) -> Result<Value> {
  let name = expect_name(args, "builtins.pnixMount")?;
  run_pnix_meta(&["mount", &name])?;
  read_mount_json(&name)
}

pub(crate) fn builtin_pnix_umount(args: &[Value]) -> Result<Value> {
  let name = expect_name(args, "builtins.pnixUmount")?;
  run_pnix_meta(&["umount", &name])?;
  Ok(Value::Bool(true))
}

pub(crate) fn builtin_pnix_mounts(args: &[Value]) -> Result<Value> {
  if let Some(name) = args.first().and_then(|v| v.as_str()) {
    return read_mount_json(name);
  }
  list_mount_json()
}

pub(crate) fn builtin_pnix_run(args: &[Value]) -> Result<Value> {
  let name = expect_name(args, "builtins.pnixRun")?;
  let mut cmd_args: Vec<String> = vec!["run".into(), name];
  if args.len() >= 2 {
    let Value::List(items) = &args[1] else {
      return Err(anyhow!("builtins.pnixRun: second argument must be a list of strings"));
    };
    if !items.is_empty() {
      cmd_args.push("--".into());
      for item in items.iter() {
        let Some(s) = item.as_str() else {
          return Err(anyhow!("builtins.pnixRun: argument list must contain only strings"));
        };
        cmd_args.push(s.to_string());
      }
    }
  }
  let refs: Vec<&str> = cmd_args.iter().map(String::as_str).collect();
  run_pnix_meta(&refs)?;
  Ok(Value::Bool(true))
}
