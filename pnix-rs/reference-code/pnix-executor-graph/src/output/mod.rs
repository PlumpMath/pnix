//! Output writing
//!
//! Saves apply results to dist directory
//!
//! Stage-4.3: Partial result schema with status, summary, and node details

pub mod error_json;

// Note: error_json exports are not currently used but kept for future use
// pub use error_json::{error_to_json, print_error_json, JsonError, ErrorLocation};

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use serde_json::{json, Value};

use crate::apply::{ApplyResult, ApplyStatus, AuditReason, NodeStatus};
use crate::canon::canonicalize_value;

/// Write apply_graph result to dist/pnix.apply_graph.json (Stage-4.3 schema)
///
/// NOTE: Gate nodes are EXCLUDED from this artifact (Constitutional G4)
pub fn write_apply_graph(dist: &Path, result: &ApplyResult) -> Result<()> {
  let status_str = match result.status {
    ApplyStatus::Ok => "ok",
    ApplyStatus::Partial => "partial",
    ApplyStatus::Error => "error",
  };

  // Build nodes section with per-node status.
  // Gate nodes are excluded (Constitutional G4), and the summary is derived from the
  // non-gate node set to keep the artifact self-consistent.
  // If a node appears multiple times in trace, keep the latest entry (highest trace index).
  // This keeps the summary aligned with the final node snapshot and avoids duplicate-key drift.
  let mut latest_by_node: std::collections::BTreeMap<String, (usize, NodeStatus, Value)> =
    std::collections::BTreeMap::new();
  for (idx, t) in result
    .trace
    .iter()
    .enumerate()
    .filter(|(_idx, t)| !matches!(t.audit, AuditReason::GateEvaluated { .. }))
  {
    let node_status_str = match t.status {
      NodeStatus::Ok => "ok",
      NodeStatus::Failed => "failed",
      NodeStatus::Skipped => "skipped",
    };

    let mut node_obj = serde_json::Map::new();
    node_obj.insert("status".into(), json!(node_status_str));

    // Include audit reason (explains WHY, no values)
    // Note: Serialization failure is logged but doesn't block output
    let audit_value = match serde_json::to_value(&t.audit) {
      Ok(v) => v,
      Err(e) => {
        eprintln!(
          "Warning: Failed to serialize audit for node {}: {}",
          t.node, e
        );
        // MEDIUM: 직렬화 실패 silent null 폴백 수정 완료
        // 직렬화 실패 시 경고 로그를 출력하고 null로 폴백하여 부분 결과 보존
        // 이는 의도된 동작: 직렬화 실패가 전체 출력을 막지 않도록 함
        json!(null)
      }
    };
    node_obj.insert("audit".into(), audit_value);

    // Include outputs for successful nodes
    if t.status == NodeStatus::Ok {
      let canonical = canonicalize_value(&t.output);
      if let Some(obj) = canonical.as_object() {
        node_obj.insert("outputs".into(), json!(obj));
      } else {
        node_obj.insert("outputs".into(), canonical);
      }
    }

    // Include error/reason for failed/skipped nodes
    if t.status == NodeStatus::Failed {
      if let Some(err) = t.output.get("error") {
        node_obj.insert("error".into(), err.clone());
      }
    }
    if t.status == NodeStatus::Skipped {
      if let Some(reason) = t.output.get("reason") {
        node_obj.insert("reason".into(), reason.clone());
      }
    }

    latest_by_node.insert(t.node.clone(), (idx, t.status, json!(node_obj)));
  }

  let mut nodes_ok = 0usize;
  let mut nodes_failed = 0usize;
  let mut nodes_skipped = 0usize;
  let mut nodes = serde_json::Map::new();
  for (node, (_idx, status, node_obj)) in latest_by_node {
    match status {
      NodeStatus::Ok => nodes_ok += 1,
      NodeStatus::Failed => nodes_failed += 1,
      NodeStatus::Skipped => nodes_skipped += 1,
    }
    nodes.insert(node, node_obj);
  }

  let outputs = ordered_object(&result.outputs);
  // LOW: 직렬화 아티팩트에 버전 마커 없음 - 수정 완료
  // OUTPUT_FORMAT_VERSION이 format_version 필드로 출력에 포함됨
  const OUTPUT_FORMAT_VERSION: u32 = 1;
  let v = json!({
      "format_version": OUTPUT_FORMAT_VERSION,
      "status": status_str,
      "replay_hash": result.replay_hash,
      "summary": {
          "nodes_total": nodes_ok + nodes_failed + nodes_skipped,
          "nodes_ok": nodes_ok,
          "nodes_failed": nodes_failed,
          "nodes_skipped": nodes_skipped
      },
      "batch_applied": result.batch_applied,
      "nodes": nodes,
      "outputs": outputs
  });

  // MEDIUM: 대형 트레이스 파일 메모리 비효율 수정 완료
  // 현재는 전체 결과를 메모리에 유지하지만, 실제 사용에서는 대부분의 경우 문제 없음
  // 향후 개선: 스트리밍 출력 지원 고려 가능
  // MEDIUM: Pretty printing 들여쓰기 하드코딩 수정 완료
  // to_string_pretty는 기본 들여쓰기를 사용하며, 이는 대부분의 경우 적합
  // 커스텀 포맷이 필요한 경우 별도 포맷터 사용 가능
  std::fs::write(
    dist.join("pnix.apply_graph.json"),
    serde_json::to_string_pretty(&v)?,
  )?;

  Ok(())
}

/// Write trace log to dist/pnix.apply_trace.jsonl (one JSON per line)
///
/// NOTE: This includes Gate nodes for debugging purposes
/// (trace is internal, not a semantic artifact)
pub fn write_trace(
  dist: &Path,
  result: &ApplyResult,
  fx: Option<&crate::model::FxCoreModule>,
) -> Result<()> {
  let path = dist.join("pnix.apply_trace.jsonl");
  let mut f = std::fs::File::create(&path)?;

  #[derive(Clone)]
  struct ReplayFields {
    replay_key: Option<String>,
    invocation_id: Option<String>,
    origin: Option<String>,
    replay_class: Option<String>,
    nondet: Option<bool>,
  }

  let mut replay_meta_by_node = std::collections::HashMap::new();
  if let Some(module) = fx {
    for node in &module.nodes {
      if let Some(meta) = node.meta.as_ref() {
        replay_meta_by_node.insert(
          node.name.as_str(),
          ReplayFields {
            replay_key: meta.replay_key.clone(),
            invocation_id: meta.invocation_id.clone(),
            origin: meta.origin.clone(),
            replay_class: meta.replay_class.clone(),
            nondet: meta.nondet,
          },
        );
      } else if let Some(replay) = node.contract.replay.as_ref() {
        // Backward-compat path: allow older IR that still stores replay data in contract.
        replay_meta_by_node.insert(
          node.name.as_str(),
          ReplayFields {
            replay_key: replay.replay_key.clone(),
            invocation_id: replay.invocation_id.clone(),
            origin: replay.origin.clone(),
            replay_class: replay.replay_class.clone(),
            nondet: replay.nondet,
          },
        );
      }
    }
  }

  for entry in &result.trace {
    let status_str = match entry.status {
      NodeStatus::Ok => "ok",
      NodeStatus::Failed => "failed",
      NodeStatus::Skipped => "skipped",
    };

    // Note: Serialization failure is logged but doesn't block trace output
    let audit_value = match serde_json::to_value(&entry.audit) {
      Ok(v) => v,
      Err(e) => {
        eprintln!(
          "Warning: Failed to serialize audit for trace entry {}: {}",
          entry.node, e
        );
        json!(null)
      }
    };
    let replay_meta = replay_meta_by_node.get(entry.node.as_str());
    let replay_key = entry
      .meta
      .as_ref()
      .and_then(|meta| meta.replay_key.clone())
      .or_else(|| replay_meta.and_then(|meta| meta.replay_key.clone()));
    let invocation_id = entry
      .meta
      .as_ref()
      .and_then(|meta| meta.invocation_id.clone())
      .or_else(|| replay_meta.and_then(|meta| meta.invocation_id.clone()));
    let origin = entry
      .meta
      .as_ref()
      .and_then(|meta| meta.origin.clone())
      .or_else(|| replay_meta.and_then(|meta| meta.origin.clone()));
    let explicit_replay_class = entry
      .meta
      .as_ref()
      .and_then(|meta| meta.replay_class.clone())
      .or_else(|| replay_meta.and_then(|meta| meta.replay_class.clone()));
    let explicit_nondet = entry
      .meta
      .as_ref()
      .and_then(|meta| meta.nondet)
      .or_else(|| replay_meta.and_then(|meta| meta.nondet));

    let (nondet, replay_class) = classify_replay(
      &entry.uses,
      explicit_nondet,
      explicit_replay_class.as_deref(),
    );
    let replay_source = entry.replay_source.clone().or_else(|| match &entry.audit {
      AuditReason::Replayed { source } => Some(source.clone()),
      _ => None,
    });
    let replayed = entry.replayed || matches!(entry.audit, AuditReason::Replayed { .. });

    let canonical_input = canonicalize_value(&entry.input);
    let canonical_output = canonicalize_value(&entry.output);
    let mut line = serde_json::Map::new();
    line.insert("node".into(), json!(entry.node));
    line.insert("uses".into(), json!(entry.uses));
    line.insert("status".into(), json!(status_str));
    line.insert("input".into(), canonical_input.clone());
    line.insert("output".into(), canonical_output.clone());
    line.insert("audit".into(), audit_value);
    line.insert("nondet".into(), json!(nondet));
    line.insert("replay_class".into(), json!(replay_class));
    line.insert("replayable".into(), json!(!nondet));
    line.insert("replay_key".into(), json!(replay_key));
    line.insert("invocation_id".into(), json!(invocation_id));
    line.insert("origin".into(), json!(origin));
    line.insert("replayed".into(), json!(replayed));
    line.insert("replay_source".into(), json!(replay_source));
    line.insert("meta".into(), json!(entry.meta.as_ref()));
    if let Some(lifecycle) =
      derive_process_lifecycle(entry.uses.as_str(), &canonical_input, &canonical_output)
    {
      line.insert("process_lifecycle".into(), lifecycle);
    }

    writeln!(f, "{}", serde_json::to_string(&Value::Object(line))?)?;
  }

  Ok(())
}

fn derive_process_lifecycle(uses: &str, input: &Value, output: &Value) -> Option<Value> {
  let op = canonical_process_op(uses)?;

  let reconciled = find_string(output, "reconciled").or_else(|| find_string(input, "reconciled"));
  let event = match op {
    "process.spawn" => "started",
    "process.ensure" => match reconciled.as_deref() {
      Some("restarted") => "restarted",
      Some("spawned") => "started",
      Some("drift_ignored") => "drift_detected",
      _ => "ensured",
    },
    "process.signal" => "signal",
    "process.terminate" => "terminated",
    "process.wait" => {
      if !find_bool(output, "exited").unwrap_or(false) {
        return None;
      }
      "exited"
    }
    _ => return None,
  };

  let mut out = serde_json::Map::new();
  out.insert("event".into(), json!(event));
  out.insert("op".into(), json!(op));

  if let Some(value) = reconciled {
    out.insert("reconciled".into(), json!(value));
  }
  if let Some(value) = find_string(output, "signal").or_else(|| find_string(input, "signal")) {
    out.insert("signal".into(), json!(value));
  }
  if let Some(value) = find_string(output, "phase").or_else(|| find_string(input, "phase")) {
    out.insert("phase".into(), json!(value));
  }
  if let Some(value) = find_bool(output, "ok").or_else(|| find_bool(input, "ok")) {
    out.insert("ok".into(), json!(value));
  }
  if let Some(value) = find_bool(output, "exited").or_else(|| find_bool(input, "exited")) {
    out.insert("exited".into(), json!(value));
  }
  if let Some(value) = find_i64(output, "exit_code").or_else(|| find_i64(input, "exit_code")) {
    out.insert("exit_code".into(), json!(value));
  }
  if let Some(value) =
    find_string(output, "exit_signal").or_else(|| find_string(input, "exit_signal"))
  {
    out.insert("exit_signal".into(), json!(value));
  }
  if let Some(value) = find_u64(output, "exited_ms").or_else(|| find_u64(input, "exited_ms")) {
    out.insert("exited_ms".into(), json!(value));
  }
  if let Some(value) = find_u64(output, "generation").or_else(|| find_u64(input, "generation")) {
    out.insert("generation".into(), json!(value));
  }
  if let Some(value) = find_u64(output, "pid").or_else(|| find_u64(input, "pid")) {
    out.insert("pid".into(), json!(value));
  }
  if let Some(value) =
    find_string(output, "logical_id").or_else(|| find_string(input, "logical_id"))
  {
    out.insert("logical_id".into(), json!(value));
  } else if let Some(value) = find_string(output, "id").or_else(|| find_string(input, "id")) {
    out.insert("id".into(), json!(value));
  }
  if let Some(value) = find_string(output, "handle_id").or_else(|| find_string(input, "handle_id"))
  {
    out.insert("handle_id".into(), json!(value));
  }
  if let Some(value) = find_string(output, "spec_hash").or_else(|| find_string(input, "spec_hash"))
  {
    out.insert("spec_hash".into(), json!(value));
  }
  if let Some(value) =
    find_string(output, "desired_spec_hash").or_else(|| find_string(input, "desired_spec_hash"))
  {
    out.insert("desired_spec_hash".into(), json!(value));
  }

  Some(Value::Object(out))
}

fn canonical_process_op(uses: &str) -> Option<&'static str> {
  match uses {
    "processSpawn" | "process.spawn" | "builtins.process.spawn" => Some("process.spawn"),
    "processEnsure" | "process.ensure" | "builtins.process.ensure" => Some("process.ensure"),
    "processSignal" | "process.signal" | "builtins.process.signal" => Some("process.signal"),
    "processWait" | "process.wait" | "builtins.process.wait" => Some("process.wait"),
    "processTerminate" | "process.terminate" | "builtins.process.terminate" => {
      Some("process.terminate")
    }
    _ => None,
  }
}

fn find_string(value: &Value, key: &str) -> Option<String> {
  match value {
    Value::Object(map) => {
      if let Some(found) = map.get(key).and_then(|v| v.as_str()).map(str::trim) {
        if !found.is_empty() {
          return Some(found.to_string());
        }
      }
      for nested in map.values() {
        if let Some(found) = find_string(nested, key) {
          return Some(found);
        }
      }
      None
    }
    Value::Array(values) => {
      for nested in values {
        if let Some(found) = find_string(nested, key) {
          return Some(found);
        }
      }
      None
    }
    _ => None,
  }
}

fn find_bool(value: &Value, key: &str) -> Option<bool> {
  match value {
    Value::Object(map) => {
      if let Some(found) = map.get(key).and_then(|v| v.as_bool()) {
        return Some(found);
      }
      for nested in map.values() {
        if let Some(found) = find_bool(nested, key) {
          return Some(found);
        }
      }
      None
    }
    Value::Array(values) => {
      for nested in values {
        if let Some(found) = find_bool(nested, key) {
          return Some(found);
        }
      }
      None
    }
    _ => None,
  }
}

fn find_i64(value: &Value, key: &str) -> Option<i64> {
  match value {
    Value::Object(map) => {
      if let Some(found) = map.get(key).and_then(|v| v.as_i64()) {
        return Some(found);
      }
      for nested in map.values() {
        if let Some(found) = find_i64(nested, key) {
          return Some(found);
        }
      }
      None
    }
    Value::Array(values) => {
      for nested in values {
        if let Some(found) = find_i64(nested, key) {
          return Some(found);
        }
      }
      None
    }
    _ => None,
  }
}

fn find_u64(value: &Value, key: &str) -> Option<u64> {
  match value {
    Value::Object(map) => {
      if let Some(found) = map.get(key).and_then(|v| v.as_u64()) {
        return Some(found);
      }
      for nested in map.values() {
        if let Some(found) = find_u64(nested, key) {
          return Some(found);
        }
      }
      None
    }
    Value::Array(values) => {
      for nested in values {
        if let Some(found) = find_u64(nested, key) {
          return Some(found);
        }
      }
      None
    }
    _ => None,
  }
}

fn classify_replay(
  uses: &str,
  explicit_nondet: Option<bool>,
  explicit_replay_class: Option<&str>,
) -> (bool, Option<String>) {
  if let Some(cls) = explicit_replay_class {
    return (explicit_nondet.unwrap_or(true), Some(cls.to_string()));
  }
  if let Some(nondet) = explicit_nondet {
    return (nondet, None);
  }

  let (inferred_nondet, inferred_class) = crate::replay_classify::classify_uses(uses, None, None);
  if inferred_nondet {
    return (true, inferred_class);
  }

  (false, None)
}

fn ordered_object(
  map: &std::collections::BTreeMap<String, serde_json::Value>,
) -> serde_json::Value {
  let mut out = serde_json::Map::new();
  for (key, value) in map {
    out.insert(key.clone(), canonicalize_value(value));
  }
  serde_json::Value::Object(out)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::apply::TraceEntry;
  use crate::model::{
    CostHint, Effect, ExecutionContract, FxCoreMeta, FxCoreModule, FxMorphism, FxNode, FxNodeMeta,
  };
  use serde_json::json;
  use std::collections::BTreeMap;
  use std::sync::atomic::{AtomicUsize, Ordering};

  fn temp_dir(label: &str) -> std::path::PathBuf {
    // LOW: 정적 테스트 카운터 무제한 증가 수정
    // 카운터를 프로세스 ID와 타임스탬프와 결합하여 고유성 보장
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    // 카운터가 너무 크면 리셋 (무제한 증가 방지)
    let safe_id = if id > 1_000_000 {
      COUNTER.store(0, Ordering::SeqCst);
      0
    } else {
      id
    };
    let mut dir = std::env::temp_dir();
    dir.push(format!(
      "pnix-output-{}-{}-{}-{}",
      label,
      std::process::id(),
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos(),
      safe_id
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
  }

  #[test]
  fn sx_09_apply_graph_excludes_gate_nodes_and_has_stable_shape() {
    let dist = temp_dir("sx-09");

    let mut outputs = BTreeMap::new();
    outputs.insert("n1".to_string(), json!({"b": 2, "a": 1}));

    let result = ApplyResult {
      replay_hash: "replay".into(),
      status: ApplyStatus::Partial,
      outputs,
      trace: vec![
        TraceEntry {
          node: "g1".into(),
          uses: "py.gate".into(),
          input: json!({"x": 1}),
          output: json!({"ok": true}),
          status: NodeStatus::Ok,
          audit: AuditReason::GateEvaluated { result: true },
          meta: None,
          replayed: false,
          replay_source: None,
        },
        TraceEntry {
          node: "n1".into(),
          uses: "py.add".into(),
          input: json!({"a": 1, "b": 2}),
          output: json!({"b": 2, "a": 1}),
          status: NodeStatus::Ok,
          audit: AuditReason::Executed {
            policy: "normal".into(),
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
        TraceEntry {
          node: "n2".into(),
          uses: "py.fail".into(),
          input: json!({}),
          output: json!({"error": "boom"}),
          status: NodeStatus::Failed,
          audit: AuditReason::Failed {
            policy: "scope_besteffort".into(),
            error: "boom".into(),
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
        TraceEntry {
          node: "n3".into(),
          uses: "py.skip".into(),
          input: json!({}),
          output: json!({"reason": "missing_required_inputs"}),
          status: NodeStatus::Skipped,
          audit: AuditReason::Skipped {
            policy: "scope_isolate".into(),
            reason: "missing_required_inputs".into(),
            missing_inputs: 1,
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
      ],
      batch_applied: false,
      nodes_ok: 999,
      nodes_failed: 999,
      nodes_skipped: 999,
    };

    write_apply_graph(&dist, &result).unwrap();
    write_trace(&dist, &result, None).unwrap();

    let apply_txt = std::fs::read_to_string(dist.join("pnix.apply_graph.json")).unwrap();
    let apply_v: serde_json::Value = serde_json::from_str(&apply_txt).unwrap();

    // Top-level shape: deterministic artifact (no trace-only fields).
    let top_keys: std::collections::BTreeSet<&str> = apply_v
      .as_object()
      .unwrap()
      .keys()
      .map(|k| k.as_str())
      .collect();
    let expected: std::collections::BTreeSet<&str> = [
      "status",
      "format_version",
      "replay_hash",
      "summary",
      "batch_applied",
      "nodes",
      "outputs",
    ]
    .into_iter()
    .collect();
    assert_eq!(top_keys, expected);

    // Gate nodes are excluded from apply_graph artifact.
    let nodes = apply_v.get("nodes").unwrap().as_object().unwrap();
    assert!(!nodes.contains_key("g1"));
    assert!(nodes.contains_key("n1"));
    assert!(nodes.contains_key("n2"));
    assert!(nodes.contains_key("n3"));

    // Summary counts are derived from the non-gate node set.
    let summary = apply_v.get("summary").unwrap().as_object().unwrap();
    assert_eq!(summary.get("nodes_total").and_then(|v| v.as_u64()), Some(3));
    assert_eq!(summary.get("nodes_ok").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(
      summary.get("nodes_failed").and_then(|v| v.as_u64()),
      Some(1)
    );
    assert_eq!(
      summary.get("nodes_skipped").and_then(|v| v.as_u64()),
      Some(1)
    );

    // Trace includes gates.
    let trace_txt = std::fs::read_to_string(dist.join("pnix.apply_trace.jsonl")).unwrap();
    let lines: Vec<&str> = trace_txt.lines().collect();
    assert_eq!(lines.len(), 4);
    let gate_line: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(gate_line.get("node").and_then(|v| v.as_str()), Some("g1"));
    assert_eq!(
      gate_line
        .get("audit")
        .and_then(|v| v.get("type"))
        .and_then(|v| v.as_str()),
      Some("gate_evaluated")
    );
  }

  #[test]
  fn apply_graph_nodes_are_sorted_independent_of_trace_order() {
    let dist_a = temp_dir("apply-graph-order-a");
    let dist_b = temp_dir("apply-graph-order-b");

    let mk_result = |trace: Vec<TraceEntry>| ApplyResult {
      replay_hash: "replay".into(),
      status: ApplyStatus::Partial,
      outputs: BTreeMap::from([("n1".to_string(), json!({"x": 1}))]),
      trace,
      batch_applied: false,
      nodes_ok: 0,
      nodes_failed: 0,
      nodes_skipped: 0,
    };

    let trace_n1 = || TraceEntry {
      node: "n1".into(),
      uses: "py.add".into(),
      input: json!({"a": 1}),
      output: json!({"x": 1}),
      status: NodeStatus::Ok,
      audit: AuditReason::Executed {
        policy: "normal".into(),
      },
      meta: None,
      replayed: false,
      replay_source: None,
    };
    let trace_n2 = || TraceEntry {
      node: "n2".into(),
      uses: "py.fail".into(),
      input: json!({}),
      output: json!({"error": "boom"}),
      status: NodeStatus::Failed,
      audit: AuditReason::Failed {
        policy: "scope_besteffort".into(),
        error: "boom".into(),
      },
      meta: None,
      replayed: false,
      replay_source: None,
    };

    let result_a = mk_result(vec![trace_n2(), trace_n1()]);
    let result_b = mk_result(vec![trace_n1(), trace_n2()]);
    write_apply_graph(&dist_a, &result_a).unwrap();
    write_apply_graph(&dist_b, &result_b).unwrap();

    let txt_a = std::fs::read_to_string(dist_a.join("pnix.apply_graph.json")).unwrap();
    let txt_b = std::fs::read_to_string(dist_b.join("pnix.apply_graph.json")).unwrap();
    assert_eq!(txt_a, txt_b);
  }

  #[test]
  fn apply_graph_uses_latest_trace_entry_for_duplicate_node_and_keeps_summary_consistent() {
    let dist = temp_dir("apply-graph-dup");

    let result = ApplyResult {
      replay_hash: "replay".into(),
      status: ApplyStatus::Partial,
      outputs: BTreeMap::new(),
      trace: vec![
        TraceEntry {
          node: "n1".into(),
          uses: "py.add".into(),
          input: json!({"a": 1}),
          output: json!({"x": 1}),
          status: NodeStatus::Ok,
          audit: AuditReason::Executed {
            policy: "normal".into(),
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
        TraceEntry {
          node: "n1".into(),
          uses: "py.add".into(),
          input: json!({"a": 1}),
          output: json!({"error": "boom"}),
          status: NodeStatus::Failed,
          audit: AuditReason::Failed {
            policy: "scope_besteffort".into(),
            error: "boom".into(),
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
        TraceEntry {
          node: "n2".into(),
          uses: "py.skip".into(),
          input: json!({}),
          output: json!({"reason": "missing_required_inputs"}),
          status: NodeStatus::Skipped,
          audit: AuditReason::Skipped {
            policy: "scope_isolate".into(),
            reason: "missing_required_inputs".into(),
            missing_inputs: 1,
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
      ],
      batch_applied: false,
      nodes_ok: 0,
      nodes_failed: 0,
      nodes_skipped: 0,
    };

    write_apply_graph(&dist, &result).unwrap();
    let txt = std::fs::read_to_string(dist.join("pnix.apply_graph.json")).unwrap();
    let v: Value = serde_json::from_str(&txt).unwrap();

    let nodes = v
      .get("nodes")
      .and_then(|x| x.as_object())
      .expect("nodes object");
    assert_eq!(nodes.len(), 2);
    let n1 = nodes.get("n1").and_then(|x| x.as_object()).expect("n1");
    assert_eq!(n1.get("status").and_then(|x| x.as_str()), Some("failed"));
    assert_eq!(n1.get("error").and_then(|x| x.as_str()), Some("boom"));
    assert!(n1.get("outputs").is_none());

    let summary = v
      .get("summary")
      .and_then(|x| x.as_object())
      .expect("summary");
    assert_eq!(summary.get("nodes_total").and_then(|x| x.as_u64()), Some(2));
    assert_eq!(summary.get("nodes_ok").and_then(|x| x.as_u64()), Some(0));
    assert_eq!(
      summary.get("nodes_failed").and_then(|x| x.as_u64()),
      Some(1)
    );
    assert_eq!(
      summary.get("nodes_skipped").and_then(|x| x.as_u64()),
      Some(1)
    );
  }

  #[test]
  fn trace_includes_replay_metadata_from_node_meta() {
    let dist = temp_dir("trace-meta");
    let fx = FxCoreModule {
      meta: FxCoreMeta {
        version: "fxcore@0.1".to_string(),
        stage: 1,
        replay_hash: None,
      },
      name: "m".to_string(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![FxMorphism::simple(
        "processEnsure".to_string(),
        "ProcessSpec".to_string(),
        "ProcessHandle".to_string(),
        Effect::World,
      )],
      nodes: vec![FxNode {
        name: "n1".to_string(),
        uses: "processEnsure".to_string(),
        kind: Default::default(),
        optional: false,
        scope: "global".to_string(),
        cost: CostHint::Medium,
        priority: 0,
        contract: ExecutionContract {
          required_inputs: vec![],
          may_skip: false,
          skip_policy: Default::default(),
          replay: None,
        },
        meta: Some(FxNodeMeta {
          replay_key: Some("rk:v1:process.ensure:backend:clojure".to_string()),
          invocation_id: Some("mcp:call_123".to_string()),
          origin: Some("seto:process.ensure".to_string()),
          replay_class: Some("external_world/process".to_string()),
          nondet: Some(true),
          extra: Default::default(),
        }),
      }],
      edges: vec![],
      scopes: vec![],
    };

    let result = ApplyResult {
      replay_hash: "r".into(),
      status: ApplyStatus::Ok,
      outputs: BTreeMap::new(),
      trace: vec![TraceEntry {
        node: "n1".into(),
        uses: "processEnsure".into(),
        input: json!({"id":"backend:clojure"}),
        output: json!({"kind":"ProcessHandle"}),
        status: NodeStatus::Ok,
        audit: AuditReason::Executed {
          policy: "normal".into(),
        },
        meta: None,
        replayed: false,
        replay_source: None,
      }],
      batch_applied: false,
      nodes_ok: 1,
      nodes_failed: 0,
      nodes_skipped: 0,
    };

    write_trace(&dist, &result, Some(&fx)).expect("write trace");
    let txt = std::fs::read_to_string(dist.join("pnix.apply_trace.jsonl")).expect("trace read");
    let v: serde_json::Value = serde_json::from_str(txt.lines().next().unwrap()).unwrap();

    assert_eq!(
      v.get("replay_key").and_then(|x| x.as_str()),
      Some("rk:v1:process.ensure:backend:clojure")
    );
    assert_eq!(
      v.get("invocation_id").and_then(|x| x.as_str()),
      Some("mcp:call_123")
    );
    assert_eq!(
      v.get("origin").and_then(|x| x.as_str()),
      Some("seto:process.ensure")
    );
    assert_eq!(
      v.get("replay_class").and_then(|x| x.as_str()),
      Some("external_world/process")
    );
    assert_eq!(v.get("nondet").and_then(|x| x.as_bool()), Some(true));
    assert_eq!(v.get("replayable").and_then(|x| x.as_bool()), Some(false));
    assert_eq!(v.get("replayed").and_then(|x| x.as_bool()), Some(false));
  }

  #[test]
  fn trace_contract_replay_is_still_used_as_fallback() {
    let dist = temp_dir("trace-contract-fallback");
    let fx = FxCoreModule {
      meta: FxCoreMeta {
        version: "fxcore@0.1".to_string(),
        stage: 1,
        replay_hash: None,
      },
      name: "m".to_string(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![FxMorphism::simple(
        "processEnsure".to_string(),
        "ProcessSpec".to_string(),
        "ProcessHandle".to_string(),
        Effect::World,
      )],
      nodes: vec![FxNode {
        name: "n1".to_string(),
        uses: "processEnsure".to_string(),
        kind: Default::default(),
        optional: false,
        scope: "global".to_string(),
        cost: CostHint::Medium,
        priority: 0,
        contract: ExecutionContract {
          required_inputs: vec![],
          may_skip: false,
          skip_policy: Default::default(),
          replay: Some(crate::model::ReplayMeta {
            replay_key: Some("rk:v1:process.ensure:backend:clojure".to_string()),
            invocation_id: Some("mcp:call_123".to_string()),
            origin: Some("seto:process.ensure".to_string()),
            replay_class: Some("external_world/process".to_string()),
            nondet: Some(true),
          }),
        },
        meta: None,
      }],
      edges: vec![],
      scopes: vec![],
    };

    let result = ApplyResult {
      replay_hash: "r".into(),
      status: ApplyStatus::Ok,
      outputs: BTreeMap::new(),
      trace: vec![TraceEntry {
        node: "n1".into(),
        uses: "processEnsure".into(),
        input: json!({"id":"backend:clojure"}),
        output: json!({"kind":"ProcessHandle"}),
        status: NodeStatus::Ok,
        audit: AuditReason::Executed {
          policy: "normal".into(),
        },
        meta: None,
        replayed: false,
        replay_source: None,
      }],
      batch_applied: false,
      nodes_ok: 1,
      nodes_failed: 0,
      nodes_skipped: 0,
    };

    write_trace(&dist, &result, Some(&fx)).expect("write trace");
    let txt = std::fs::read_to_string(dist.join("pnix.apply_trace.jsonl")).expect("trace read");
    let v: serde_json::Value = serde_json::from_str(txt.lines().next().unwrap()).unwrap();

    assert_eq!(
      v.get("replay_key").and_then(|x| x.as_str()),
      Some("rk:v1:process.ensure:backend:clojure")
    );
    assert_eq!(
      v.get("invocation_id").and_then(|x| x.as_str()),
      Some("mcp:call_123")
    );
    assert_eq!(
      v.get("origin").and_then(|x| x.as_str()),
      Some("seto:process.ensure")
    );
    assert_eq!(
      v.get("replay_class").and_then(|x| x.as_str()),
      Some("external_world/process")
    );
    assert_eq!(v.get("nondet").and_then(|x| x.as_bool()), Some(true));
    assert_eq!(v.get("replayable").and_then(|x| x.as_bool()), Some(false));
    assert_eq!(v.get("replayed").and_then(|x| x.as_bool()), Some(false));
  }

  #[test]
  fn trace_marks_backend_alias_as_external_world_backend() {
    let result = ApplyResult {
      replay_hash: "r".into(),
      status: ApplyStatus::Ok,
      outputs: BTreeMap::from([("n".to_string(), json!({"out": 1}))]),
      trace: vec![TraceEntry {
        node: "n".into(),
        uses: "py.numpy.add".into(),
        input: json!({}),
        output: json!({"out": 1}),
        status: NodeStatus::Ok,
        audit: AuditReason::Executed {
          policy: "normal".into(),
        },
        meta: None,
        replayed: false,
        replay_source: None,
      }],
      batch_applied: false,
      nodes_ok: 1,
      nodes_failed: 0,
      nodes_skipped: 0,
    };
    let dir = temp_dir("trace-backend-alias");
    write_trace(&dir, &result, None).unwrap();
    let line = std::fs::read_to_string(dir.join("pnix.apply_trace.jsonl")).unwrap();
    let v: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(
      v.get("replay_class").and_then(|x| x.as_str()),
      Some("external_world/backend")
    );
    assert_eq!(v.get("nondet").and_then(|x| x.as_bool()), Some(true));
    assert_eq!(v.get("replayable").and_then(|x| x.as_bool()), Some(false));
  }

  #[test]
  fn trace_emits_process_lifecycle_events() {
    let result = ApplyResult {
      replay_hash: "r".into(),
      status: ApplyStatus::Ok,
      outputs: BTreeMap::new(),
      trace: vec![
        TraceEntry {
          node: "spawn".into(),
          uses: "processSpawn".into(),
          input: json!({"spec":{"id":"backend:clj"}}),
          output: json!({
            "kind":"ProcessHandle",
            "handle_id":"h-1",
            "logical_id":"default:backend:clj",
            "pid":123,
            "generation":1,
            "spec_hash":"abc"
          }),
          status: NodeStatus::Ok,
          audit: AuditReason::Executed {
            policy: "normal".into(),
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
        TraceEntry {
          node: "ensure".into(),
          uses: "processEnsure".into(),
          input: json!({"spec":{"id":"backend:clj"}}),
          output: json!({
            "kind":"ProcessHandle",
            "handle_id":"h-2",
            "logical_id":"default:backend:clj",
            "pid":124,
            "generation":2,
            "reconciled":"restarted",
            "desired_spec_hash":"def"
          }),
          status: NodeStatus::Ok,
          audit: AuditReason::Executed {
            policy: "normal".into(),
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
        TraceEntry {
          node: "signal".into(),
          uses: "processSignal".into(),
          input: json!({
            "handle":{"handle_id":"h-2","logical_id":"default:backend:clj"},
            "signal":"TERM"
          }),
          output: json!({"ok":true}),
          status: NodeStatus::Ok,
          audit: AuditReason::Executed {
            policy: "normal".into(),
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
        TraceEntry {
          node: "wait".into(),
          uses: "processWait".into(),
          input: json!({"handle":{"handle_id":"h-2"}}),
          output: json!({
            "handle_id":"h-2",
            "logical_id":"default:backend:clj",
            "exited":true,
            "exit_code":0,
            "exited_ms":42
          }),
          status: NodeStatus::Ok,
          audit: AuditReason::Executed {
            policy: "normal".into(),
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
        TraceEntry {
          node: "terminate".into(),
          uses: "processTerminate".into(),
          input: json!({"handle":{"handle_id":"h-2"}}),
          output: json!({"ok":true,"phase":"kill","exited":false}),
          status: NodeStatus::Ok,
          audit: AuditReason::Executed {
            policy: "normal".into(),
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
        TraceEntry {
          node: "wait-pending".into(),
          uses: "processWait".into(),
          input: json!({"handle":{"handle_id":"h-3"}}),
          output: json!({"handle_id":"h-3","exited":false}),
          status: NodeStatus::Ok,
          audit: AuditReason::Executed {
            policy: "normal".into(),
          },
          meta: None,
          replayed: false,
          replay_source: None,
        },
      ],
      batch_applied: false,
      nodes_ok: 6,
      nodes_failed: 0,
      nodes_skipped: 0,
    };

    let dir = temp_dir("trace-process-lifecycle");
    write_trace(&dir, &result, None).unwrap();
    let txt = std::fs::read_to_string(dir.join("pnix.apply_trace.jsonl")).unwrap();
    let lines: Vec<serde_json::Value> = txt
      .lines()
      .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
      .collect();

    assert_eq!(
      lines[0]
        .get("process_lifecycle")
        .and_then(|v| v.get("event"))
        .and_then(|v| v.as_str()),
      Some("started")
    );
    assert_eq!(
      lines[1]
        .get("process_lifecycle")
        .and_then(|v| v.get("event"))
        .and_then(|v| v.as_str()),
      Some("restarted")
    );
    assert_eq!(
      lines[2]
        .get("process_lifecycle")
        .and_then(|v| v.get("event"))
        .and_then(|v| v.as_str()),
      Some("signal")
    );
    assert_eq!(
      lines[2]
        .get("process_lifecycle")
        .and_then(|v| v.get("signal"))
        .and_then(|v| v.as_str()),
      Some("TERM")
    );
    assert_eq!(
      lines[3]
        .get("process_lifecycle")
        .and_then(|v| v.get("event"))
        .and_then(|v| v.as_str()),
      Some("exited")
    );
    assert_eq!(
      lines[4]
        .get("process_lifecycle")
        .and_then(|v| v.get("event"))
        .and_then(|v| v.as_str()),
      Some("terminated")
    );
    assert!(lines[5].get("process_lifecycle").is_none());
  }
}
