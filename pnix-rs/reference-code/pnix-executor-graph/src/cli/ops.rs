//! Ops 모드: supervisor RPC thin wrapper

use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use pnix_runtime_supervisor::client::SupervisorClient;
use serde_json::{json, Value};
use pnix_hash::{Digest, Sha256};

#[derive(Debug)]
struct OpsArgs {
  endpoint: String,
  op: String,
  payload: Value,
  caps: Vec<String>,
  emit_payload: Option<String>,
  emit_preflight_report: Option<String>,
  emit_change_snapshot: Option<String>,
  emit_bundle: Option<String>,
  preflight_max_age_ms: u64,
  yes: bool,
  no_confirm: bool,
  confirm: Option<String>,
  require_preflight: Option<String>,
  require_bundle: Option<String>,
  retry_preflight_only: bool,
  resume_after_refresh: bool,
  retry_max_attempts: u32,
  retry_backoff_ms: Vec<u64>,
  execute: bool,
}

#[derive(Debug, Clone)]
struct RequiredBundle {
  source_path: String,
  binding: serde_json::Map<String, Value>,
  preflight_report: Value,
  expected_plan_core_sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RiskLevel {
  Low,
  Medium,
  High,
}

#[derive(Debug)]
struct OpsStructuredError {
  code: OpsErrorCode,
  message: String,
  details: Value,
}

impl std::fmt::Display for OpsStructuredError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl std::error::Error for OpsStructuredError {}

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
enum OpsExitCode {
  Usage = 2,
  Denied = 3,
  NotFound = 4,
  Conflict = 5,
  Unauthorized = 6,
  Timeout = 7,
  Transport = 8,
  Internal = 10,
  Partial = 11,
}

impl OpsExitCode {
  fn as_i32(self) -> i32 {
    self as i32
  }
}

#[derive(Debug, Clone, Copy)]
enum OpsErrorCode {
  Usage,
  Denied,
  NotFound,
  Conflict,
  Unauthorized,
  Timeout,
  Transport,
  Internal,
  Partial,
}

impl OpsErrorCode {
  fn as_str(self) -> &'static str {
    match self {
      OpsErrorCode::Usage => "USAGE",
      OpsErrorCode::Denied => "DENIED",
      OpsErrorCode::NotFound => "NOT_FOUND",
      OpsErrorCode::Conflict => "CONFLICT",
      OpsErrorCode::Unauthorized => "UNAUTHORIZED",
      OpsErrorCode::Timeout => "TIMEOUT",
      OpsErrorCode::Transport => "TRANSPORT",
      OpsErrorCode::Internal => "INTERNAL",
      OpsErrorCode::Partial => "PARTIAL",
    }
  }
}

pub(super) fn is_ops_invocation(argv: &[String]) -> bool {
  if argv.get(1).map(|arg| arg == "ops").unwrap_or(false) {
    return true;
  }
  if argv.iter().any(|arg| {
    arg
      .strip_prefix("--mode=")
      .is_some_and(|value| value == "ops")
  }) {
    return true;
  }
  for window in argv.windows(2) {
    if window[0] == "--mode" && window[1] == "ops" {
      return true;
    }
  }
  false
}

pub(super) fn run_ops_invocation(argv: &[String]) -> Result<()> {
  let parsed_args = parse_ops_args(argv).ok();
  match run_ops_invocation_inner(argv) {
    Ok(()) => Ok(()),
    Err(err) => {
      let (exit_code, error_code, details) = classify_ops_error(&err);
      let (op, payload) = op_hint_from_argv(argv);
      let resolved_op = resolve_alias(op.as_str());
      let enriched_details =
        enrich_error_details_with_hint(resolved_op.as_str(), payload.as_ref(), error_code, details);
      if let Some(args) = parsed_args.as_ref() {
        if let Some(success_envelope) = maybe_retry_preflight_only(
          args,
          resolved_op.as_str(),
          payload.as_ref(),
          error_code,
          &enriched_details,
        )? {
          println!("{}", serde_json::to_string_pretty(&success_envelope)?);
          return Ok(());
        }
      }
      let ts_ms = now_ms();
      let response = response_error_envelope(
        resolved_op.as_str(),
        payload
          .as_ref()
          .and_then(|v| payload_string_field(v, "ns"))
          .as_deref(),
        payload
          .as_ref()
          .and_then(|v| payload_string_field(v, "channel"))
          .as_deref(),
        None,
        ts_ms,
        error_code,
        err.to_string(),
        enriched_details,
      );
      eprintln!("{}", serde_json::to_string_pretty(&response)?);
      std::process::exit(exit_code.as_i32());
    }
  }
}

fn run_ops_invocation_inner(argv: &[String]) -> Result<()> {
  let args = parse_ops_args(argv)?;
  let op_name = resolve_alias(args.op.as_str());
  let risk_level = op_risk_level(op_name.as_str());
  let ts_ms = now_ms();
  let request_id =
    payload_string_field(&args.payload, "request_id").unwrap_or_else(|| format!("req_{}", ts_ms));

  if let Some(path) = args.emit_payload.as_deref() {
    if let Some(parent) = Path::new(path).parent() {
      if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent)
          .with_context(|| format!("create payload directory {}", parent.display()))?;
      }
    }
    fs::write(
      path,
      format!("{}\n", serde_json::to_string_pretty(&args.payload)?),
    )
    .with_context(|| format!("write payload file {}", path))?;
  }
  let mut required_preflight_report: Option<Value> = None;
  let mut required_bundle: Option<RequiredBundle> = None;
  if op_name == "change.run" {
    if args.require_bundle.is_some() {
      required_bundle = enforce_required_bundle(args.require_bundle.as_deref(), &args.payload)?;
      if let Some(bundle) = required_bundle.as_ref() {
        required_preflight_report = Some(bundle.preflight_report.clone());
      }
    } else {
      required_preflight_report =
        enforce_required_preflight(args.require_preflight.as_deref(), &args.payload)?;
    }
  }

  let verify_only_with_bundle =
    !args.execute && op_name == "change.run" && required_bundle.is_some();
  if !args.execute && !verify_only_with_bundle {
    let result = json!({
      "dry_run": true,
      "endpoint": args.endpoint,
      "emit_payload": args.emit_payload,
      "payload": args.payload,
    });
    let envelope = response_ok_envelope(
      op_name.as_str(),
      payload_string_field(&args.payload, "ns").as_deref(),
      payload_string_field(&args.payload, "channel").as_deref(),
      request_id.as_str(),
      ts_ms,
      result,
    );
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    return Ok(());
  }

  if args.execute {
    enforce_confirmation(&args, op_name.as_str(), risk_level)?;
  }

  let client = SupervisorClient::connect(args.endpoint.clone())
    .with_context(|| format!("connect supervisor endpoint {}", args.endpoint))?;

  if op_name == "change.run" {
    if let Some(report) = required_preflight_report.as_ref() {
      let preflight_context_path = args
        .require_preflight
        .as_deref()
        .or(args.require_bundle.as_deref())
        .unwrap_or("");
      enforce_change_get_binding(
        report,
        preflight_context_path,
        &args.payload,
        &client,
        &args.caps,
      )?;
    }
    if let Some(bundle) = required_bundle.as_ref() {
      enforce_bundle_change_get_binding(bundle, &args.payload, &client, &args.caps)?;
    }
    if !args.execute {
      let result = json!({
        "ready_to_execute": true,
        "executed": false,
        "mode": "verify_only",
        "bundle_path": args.require_bundle,
        "checks": {
          "preflight_ok": required_preflight_report.is_some(),
          "binding_ok": true,
          "plan_unchanged": true,
        }
      });
      let envelope = response_ok_envelope(
        op_name.as_str(),
        payload_string_field(&args.payload, "ns").as_deref(),
        payload_string_field(&args.payload, "channel").as_deref(),
        request_id.as_str(),
        ts_ms,
        result,
      );
      println!("{}", serde_json::to_string_pretty(&envelope)?);
      return Ok(());
    }
  }

  if op_name == "ping" {
    let morphisms = client.list()?;
    let result = json!({
      "ok": true,
      "endpoint": args.endpoint,
      "morphisms_count": morphisms.len(),
    });
    let envelope = response_ok_envelope(
      op_name.as_str(),
      payload_string_field(&args.payload, "ns").as_deref(),
      payload_string_field(&args.payload, "channel").as_deref(),
      request_id.as_str(),
      ts_ms,
      result,
    );
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    return Ok(());
  }

  if op_name == "list" || op_name == "ops.list" {
    let morphisms = client.list()?;
    let envelope = response_ok_envelope(
      op_name.as_str(),
      payload_string_field(&args.payload, "ns").as_deref(),
      payload_string_field(&args.payload, "channel").as_deref(),
      request_id.as_str(),
      ts_ms,
      json!({
        "endpoint": args.endpoint,
        "morphisms": morphisms,
      }),
    );
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    return Ok(());
  }

  if op_name == "report.risk" {
    let cap_refs = args.caps.iter().map(String::as_str).collect::<Vec<_>>();
    let ns = payload_string_field(&args.payload, "ns").unwrap_or_else(|| "default".to_string());
    let pending = client.call_with(
      "change.list",
      &cap_refs,
      json!({"ns": ns, "status": "pending", "limit": 500}),
    )?;
    let high_risk = client.call_with(
      "change.list",
      &cap_refs,
      json!({"ns": ns, "status": "pending", "risk": "high", "limit": 500}),
    )?;
    let breakglass = client.call_with(
      "breakglass.session.list",
      &cap_refs,
      json!({"ns": ns, "status": "active", "limit": 500}),
    )?;
    let result = json!({
      "bg_count": detect_collection_len(&breakglass),
      "change_counts": {
        "pending_total": detect_collection_len(&pending),
        "pending_high_risk": detect_collection_len(&high_risk),
      },
      "risk_snapshot": {
        "breakglass": breakglass,
        "pending": pending,
        "pending_high_risk": high_risk
      }
    });
    let envelope = response_ok_envelope(
      op_name.as_str(),
      payload_string_field(&args.payload, "ns").as_deref(),
      payload_string_field(&args.payload, "channel").as_deref(),
      request_id.as_str(),
      ts_ms,
      result,
    );
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    return Ok(());
  }

  let cap_refs = args.caps.iter().map(String::as_str).collect::<Vec<_>>();
  let outputs = client
    .call_with(op_name.as_str(), &cap_refs, args.payload.clone())
    .with_context(|| format!("supervisor op {} failed", op_name))?;
  maybe_emit_preflight_report(
    &args,
    op_name.as_str(),
    request_id.as_str(),
    ts_ms,
    &outputs,
  )?;
  maybe_emit_change_snapshot(
    &args,
    op_name.as_str(),
    request_id.as_str(),
    ts_ms,
    &outputs,
  )?;
  maybe_emit_safe_run_bundle(
    &args,
    op_name.as_str(),
    request_id.as_str(),
    &client,
    required_preflight_report.as_ref(),
  )?;
  let envelope = response_ok_envelope(
    op_name.as_str(),
    payload_string_field(&args.payload, "ns").as_deref(),
    payload_string_field(&args.payload, "channel").as_deref(),
    request_id.as_str(),
    ts_ms,
    outputs,
  );
  println!("{}", serde_json::to_string_pretty(&envelope)?);
  Ok(())
}

fn maybe_retry_preflight_only(
  args: &OpsArgs,
  op_name: &str,
  payload: Option<&Value>,
  _error_code: OpsErrorCode,
  details: &Value,
) -> Result<Option<Value>> {
  if !args.retry_preflight_only || !args.execute || op_name != "change.run" {
    return Ok(None);
  }
  if !hint_allows_auto_retry(details) {
    return Ok(None);
  }
  let Some(require_preflight) = args.require_preflight.as_deref() else {
    return Ok(None);
  };

  let report_path = normalize_payload_file_ref(require_preflight);
  let source = match fs::read_to_string(report_path.as_path()) {
    Ok(raw) => raw,
    Err(_) => return Ok(None),
  };
  let report = match serde_json::from_str::<Value>(source.as_str()) {
    Ok(value) => value,
    Err(_) => return Ok(None),
  };
  let preflight_payload = match build_preflight_payload_from_report(&report, payload) {
    Ok(value) => value,
    Err(_) => return Ok(None),
  };

  let retry_of = details
    .get("hint")
    .and_then(|hint| hint.get("workdir"))
    .and_then(Value::as_str)
    .map(ToString::to_string);
  let invocation_id = format!("inv_{}_{}", now_ms(), std::process::id());
  let workdir = PathBuf::from(".pnix/ops_runs").join(invocation_id.as_str());
  fs::create_dir_all(&workdir)
    .with_context(|| format!("create preflight refresh workdir {}", workdir.display()))?;

  write_json_atomic(
    &workdir.join("00_meta.json"),
    &json!({
      "schema": "pnix-safe-run-preflight-refresh-meta@0.1",
      "invocation_id": invocation_id,
      "retry_of": retry_of,
      "retry_mode": if args.resume_after_refresh { "refresh_and_verify" } else { "preflight_only" },
      "op": op_name,
      "endpoint": args.endpoint,
      "change_id": payload.and_then(|v| payload_string_field(v, "change_id")),
      "subject_id": preflight_payload
        .get("subject")
        .and_then(Value::as_object)
        .and_then(|subject| subject.get("id"))
        .and_then(Value::as_str),
      "started_ms": now_ms(),
    }),
  )?;
  write_json_atomic(
    &workdir.join("02_preflight_inputs.json"),
    &preflight_payload,
  )?;

  let cap_refs = args.caps.iter().map(String::as_str).collect::<Vec<_>>();
  let client = match SupervisorClient::connect(args.endpoint.clone()) {
    Ok(client) => client,
    Err(_) => return Ok(None),
  };

  let mut outputs: Option<Value> = None;
  let mut last_error: Option<String> = None;
  let mut attempts_used = 0u32;
  let mut retry_events: Vec<Value> = Vec::new();
  for attempt in 0..args.retry_max_attempts {
    attempts_used = attempt + 1;
    retry_events.push(json!({
      "attempt": attempts_used,
      "state": "attempt_started",
      "ts_ms": now_ms(),
    }));
    match client.call_with("admission.check", &cap_refs, preflight_payload.clone()) {
      Ok(value) => {
        retry_events.push(json!({
          "attempt": attempts_used,
          "state": "attempt_succeeded",
          "ts_ms": now_ms(),
        }));
        outputs = Some(value);
        break;
      }
      Err(err) => {
        let error_text = format!("{:#}", err);
        last_error = Some(error_text.clone());
        if attempt + 1 < args.retry_max_attempts {
          let backoff =
            retry_backoff_for_attempt(args.retry_backoff_ms.as_slice(), attempt as usize);
          retry_events.push(json!({
            "attempt": attempts_used,
            "state": "attempt_failed_retryable",
            "backoff_ms": backoff,
            "error": error_text,
            "ts_ms": now_ms(),
          }));
          std::thread::sleep(Duration::from_millis(backoff));
        } else {
          retry_events.push(json!({
            "attempt": attempts_used,
            "state": "attempt_failed_terminal",
            "error": error_text,
            "ts_ms": now_ms(),
          }));
        }
      }
    }
  }

  let Some(outputs) = outputs else {
    write_json_atomic(
      &workdir.join("09_summary.json"),
      &json!({
        "schema": "pnix-safe-run-preflight-refresh@0.1",
        "ok": false,
        "retry_class": "retryable",
        "mode": "preflight_refresh",
        "executed": false,
        "invocation_id": invocation_id,
        "retry_of": retry_of,
        "attempts_used": attempts_used,
        "last_error": last_error,
        "retry_events": retry_events,
      }),
    )?;
    return Ok(None);
  };

  write_json_atomic(&workdir.join("03_preflight_response.json"), &outputs)?;
  let refresh_request_id = format!("req_refresh_{}", now_ms());
  let refreshed_report = build_preflight_report_from_payload(
    &preflight_payload,
    &outputs,
    args.endpoint.as_str(),
    refresh_request_id.as_str(),
    args.preflight_max_age_ms,
  )?;
  let refreshed_report_path = workdir.join("04_preflight_report.json");
  write_json_atomic(&refreshed_report_path, &refreshed_report)?;

  let allow = outputs
    .get("allow")
    .and_then(Value::as_bool)
    .unwrap_or_else(|| {
      outputs
        .get("decision")
        .and_then(Value::as_str)
        .is_some_and(|decision| decision.eq_ignore_ascii_case("allow"))
    });

  let mut refreshed_bundle_path: Option<PathBuf> = None;
  let mut ready_to_execute = false;
  if allow && args.resume_after_refresh {
    let Some(run_payload) = payload.cloned() else {
      return Ok(None);
    };
    let refreshed_path_label = refreshed_report_path.to_string_lossy().to_string();
    let validated_report = match validate_preflight_report(
      &refreshed_report,
      &run_payload,
      refreshed_path_label.as_str(),
    ) {
      Ok(report) => report,
      Err(_) => return Ok(None),
    };
    if enforce_change_get_binding(
      &validated_report,
      refreshed_path_label.as_str(),
      &run_payload,
      &client,
      &args.caps,
    )
    .is_err()
    {
      return Ok(None);
    }
    let ns = run_payload
      .get("ns")
      .and_then(Value::as_str)
      .unwrap_or("default");
    let change_id = match run_payload
      .get("change_id")
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|value| !value.is_empty())
    {
      Some(change_id) => change_id,
      None => return Ok(None),
    };
    let change_get = match client.call_with(
      "change.get",
      &cap_refs,
      json!({
        "ns": ns,
        "change_id": change_id
      }),
    ) {
      Ok(value) => value,
      Err(_) => return Ok(None),
    };
    let snapshot =
      match build_change_snapshot(args, refresh_request_id.as_str(), now_ms(), &change_get) {
        Ok(snapshot) => snapshot,
        Err(_) => return Ok(None),
      };
    let snapshot_path = workdir.join("06_change_snapshot_after.json");
    if write_json_atomic(&snapshot_path, &snapshot).is_err() {
      return Ok(None);
    }
    let bundle = match build_safe_run_bundle(
      args,
      &run_payload,
      &validated_report,
      &snapshot,
      invocation_id.as_str(),
      &workdir,
      args.preflight_max_age_ms,
    ) {
      Ok(bundle) => bundle,
      Err(_) => return Ok(None),
    };
    let bundle_path = workdir.join("10_bundle.json");
    if write_json_atomic(&bundle_path, &bundle).is_err() {
      return Ok(None);
    }
    refreshed_bundle_path = Some(bundle_path);
    ready_to_execute = true;
  }

  let summary_schema = if args.resume_after_refresh {
    "pnix-safe-run-refresh-verify@0.1"
  } else {
    "pnix-safe-run-preflight-refresh@0.1"
  };
  let summary_mode = if args.resume_after_refresh {
    "refresh_and_verify"
  } else {
    "preflight_refresh"
  };

  let summary = json!({
    "schema": summary_schema,
    "ok": allow,
    "retry_class": "retryable",
    "mode": summary_mode,
    "executed": false,
    "ready_to_execute": ready_to_execute,
    "invocation_id": invocation_id,
    "retry_of": retry_of,
    "attempts_used": attempts_used,
    "retry_events": retry_events,
    "artifacts": {
      "preflight_report": format!("file://{}", refreshed_report_path.display()),
      "workdir": format!("file://{}", workdir.display()),
      "bundle": refreshed_bundle_path
        .as_ref()
        .map(|path| format!("file://{}", path.display())),
    },
    "next": if allow {
      if let Some(bundle_path) = refreshed_bundle_path.as_ref() {
        json!({
          "recommended": "verification completed; rerun safe-run with --require-bundle to execute",
          "cmd_verify": format!(
            "pnix ops --op safe-run --payload '{{\"ns\":\"{}\",\"channel\":\"{}\",\"change_id\":\"{}\"}}' --require-bundle @{}",
            payload
              .and_then(|v| payload_string_field(v, "ns"))
              .unwrap_or_else(|| "default".to_string()),
            payload
              .and_then(|v| payload_string_field(v, "channel"))
              .unwrap_or_else(|| "prod".to_string()),
            payload
              .and_then(|v| payload_string_field(v, "change_id"))
              .unwrap_or_else(|| "chg_REPLACE_ME".to_string()),
            bundle_path.display(),
          ),
          "cmd_execute": format!(
            "pnix ops --op safe-run --payload '{{\"ns\":\"{}\",\"channel\":\"{}\",\"change_id\":\"{}\"}}' --require-bundle @{} --execute",
            payload
              .and_then(|v| payload_string_field(v, "ns"))
              .unwrap_or_else(|| "default".to_string()),
            payload
              .and_then(|v| payload_string_field(v, "channel"))
              .unwrap_or_else(|| "prod".to_string()),
            payload
              .and_then(|v| payload_string_field(v, "change_id"))
              .unwrap_or_else(|| "chg_REPLACE_ME".to_string()),
            bundle_path.display(),
          ),
        })
      } else {
        json!({
          "recommended": "rerun safe-run using --require-preflight",
          "cmd": format!(
            "pnix ops safe-run {} --subject {} --require-preflight @{} --execute",
            payload
              .and_then(|v| payload_string_field(v, "change_id"))
              .unwrap_or_else(|| "chg_REPLACE_ME".to_string()),
            preflight_payload
              .get("subject")
              .and_then(Value::as_object)
              .and_then(|subject| subject.get("id"))
              .and_then(Value::as_str)
              .unwrap_or("subject_REPLACE_ME"),
            refreshed_report_path.display(),
          ),
        })
      }
    } else {
      json!({
        "recommended": "inspect refreshed preflight report and fix deny reasons before retry",
      })
    }
  });
  write_json_atomic(&workdir.join("09_summary.json"), &summary)?;

  if !allow {
    return Ok(None);
  }

  let result = json!({
    "schema": summary_schema,
    "ok": true,
    "retry_class": "retryable",
    "mode": summary_mode,
    "executed": false,
    "ready_to_execute": ready_to_execute,
    "invocation_id": invocation_id,
    "retry_of": retry_of,
    "attempts_used": attempts_used,
    "retry_events": retry_events,
    "artifacts": {
      "preflight_report": format!("file://{}", refreshed_report_path.display()),
      "workdir": format!("file://{}", workdir.display()),
      "bundle": refreshed_bundle_path
        .as_ref()
        .map(|path| format!("file://{}", path.display())),
    }
  });
  let response = response_ok_envelope(
    if args.resume_after_refresh {
      "safe-run.refresh-and-verify"
    } else {
      "safe-run.preflight-refresh"
    },
    payload
      .and_then(|v| payload_string_field(v, "ns"))
      .as_deref(),
    payload
      .and_then(|v| payload_string_field(v, "channel"))
      .as_deref(),
    refresh_request_id.as_str(),
    now_ms(),
    result,
  );
  Ok(Some(response))
}

fn hint_has_recommended_action(details: &Value, action_id: &str) -> bool {
  details
    .get("hint")
    .and_then(|hint| hint.get("recommended_actions"))
    .and_then(Value::as_array)
    .is_some_and(|actions| {
      actions.iter().any(|action| {
        action
          .get("id")
          .and_then(Value::as_str)
          .is_some_and(|id| id == action_id)
      })
    })
}

fn hint_retry_class(details: &Value) -> Option<&str> {
  details
    .get("hint")
    .and_then(|hint| hint.get("retry_class"))
    .and_then(Value::as_str)
}

fn hint_allows_auto_retry(details: &Value) -> bool {
  matches!(hint_retry_class(details), Some("retryable"))
    || hint_has_recommended_action(details, "AUTO_RETRY_RETRYABLE")
    || hint_has_recommended_action(details, "AUTO_RETRY_OK")
}

fn normalize_payload_file_ref(raw: &str) -> PathBuf {
  if let Some(path) = raw.strip_prefix('@') {
    PathBuf::from(path)
  } else {
    PathBuf::from(raw)
  }
}

fn retry_backoff_for_attempt(backoff: &[u64], attempt_idx: usize) -> u64 {
  backoff
    .get(attempt_idx)
    .copied()
    .or_else(|| backoff.last().copied())
    .unwrap_or(500)
}

fn build_preflight_payload_from_report(
  report: &Value,
  fallback_payload: Option<&Value>,
) -> Result<Value> {
  let input = report
    .get("input")
    .and_then(Value::as_object)
    .ok_or_else(|| anyhow::anyhow!("preflight report missing input object"))?;
  let request = input.get("request").and_then(Value::as_object);
  let subject = input
    .get("subject")
    .and_then(Value::as_object)
    .cloned()
    .unwrap_or_default();
  let mut payload = serde_json::Map::new();
  payload.insert(
    "ns".to_string(),
    json!(request
      .and_then(|req| req.get("ns"))
      .or_else(|| fallback_payload.and_then(|v| v.get("ns")))
      .and_then(Value::as_str)
      .unwrap_or("default")),
  );
  payload.insert(
    "channel".to_string(),
    json!(request
      .and_then(|req| req.get("channel"))
      .or_else(|| fallback_payload.and_then(|v| v.get("channel")))
      .and_then(Value::as_str)
      .unwrap_or("prod")),
  );
  payload.insert(
    "target".to_string(),
    json!(request
      .and_then(|req| req.get("target"))
      .and_then(Value::as_str)
      .unwrap_or("exec.admit")),
  );
  payload.insert(
    "actor".to_string(),
    input
      .get("actor")
      .cloned()
      .or_else(|| fallback_payload.and_then(|v| v.get("actor").cloned()))
      .unwrap_or_else(|| json!({})),
  );
  payload.insert("subject".to_string(), Value::Object(subject.clone()));
  if let Some(process_spec) = subject.get("process_spec") {
    payload.insert("process_spec".to_string(), process_spec.clone());
  } else if let Some(fallback) = fallback_payload
    .and_then(|v| v.get("process_spec"))
    .cloned()
  {
    payload.insert("process_spec".to_string(), fallback);
  }
  payload.insert(
    "desired".to_string(),
    input.get("desired").cloned().unwrap_or_else(|| json!({})),
  );
  payload.insert(
    "system".to_string(),
    input.get("system").cloned().unwrap_or_else(|| json!({})),
  );
  payload.insert(
    "evidence".to_string(),
    input.get("evidence").cloned().unwrap_or_else(|| json!({})),
  );
  payload.insert(
    "links".to_string(),
    input.get("links").cloned().unwrap_or_else(|| json!({})),
  );
  payload.insert(
    "breakglass".to_string(),
    json!(input
      .get("breakglass")
      .and_then(Value::as_bool)
      .unwrap_or(false)),
  );
  Ok(Value::Object(payload))
}

fn build_preflight_report_from_payload(
  payload: &Value,
  outputs: &Value,
  endpoint: &str,
  request_id: &str,
  max_age_ms: u64,
) -> Result<Value> {
  let issued_ms = now_ms();
  let max_age_ms = max_age_ms as i64;
  let expires_ms = issued_ms.saturating_add(max_age_ms);
  let ns = payload_string_field(payload, "ns").unwrap_or_else(|| "default".to_string());
  let channel = payload_string_field(payload, "channel").unwrap_or_else(|| "prod".to_string());
  let target = payload_string_field(payload, "target").unwrap_or_else(|| "exec.admit".to_string());
  let subject = payload
    .get("subject")
    .and_then(Value::as_object)
    .cloned()
    .unwrap_or_default();
  let subject_kind = subject
    .get("kind")
    .and_then(Value::as_str)
    .unwrap_or("process")
    .to_string();
  let subject_id = subject
    .get("id")
    .and_then(Value::as_str)
    .unwrap_or("")
    .to_string();
  let links = normalize_preflight_links(payload, request_id, "admission.check", endpoint);

  let input_payload =
    build_preflight_input_payload(payload, issued_ms, &ns, &channel, &target, &links);
  let output_payload =
    build_preflight_output_payload(outputs, &target, &subject_kind, &subject_id, &channel);
  let input_sha = sha256_value_hex(&input_payload)?;
  let output_sha = sha256_value_hex(&output_payload)?;
  let process_spec_sha256 = payload
    .get("process_spec")
    .map(canonical_sha256_value_hex)
    .transpose()?;

  Ok(json!({
    "schema": "pnix-preflight-report@0.1",
    "issued_ms": issued_ms,
    "max_age_ms": max_age_ms,
    "expires_ms": expires_ms,
    "binding": {
      "ns": ns,
      "channel": channel,
      "target": target,
      "subject_kind": subject_kind,
      "subject_id": subject_id,
      "change_id": payload.get("change_id").and_then(Value::as_str),
      "process_spec_sha256": process_spec_sha256,
      "links": links,
    },
    "input": input_payload,
    "output": output_payload,
    "digests": {
      "input_sha256": input_sha,
      "output_sha256": output_sha,
      "request_id": request_id,
      "breakglass": payload
        .get("breakglass")
        .and_then(Value::as_bool)
        .unwrap_or(false),
    }
  }))
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<()> {
  if let Some(parent) = path.parent() {
    if !parent.as_os_str().is_empty() {
      fs::create_dir_all(parent)
        .with_context(|| format!("create directory {}", parent.display()))?;
    }
  }
  let tmp = path.with_extension(format!(
    "{}.tmp",
    path
      .extension()
      .and_then(|ext| ext.to_str())
      .unwrap_or("json")
  ));
  fs::write(&tmp, format!("{}\n", serde_json::to_string_pretty(value)?))
    .with_context(|| format!("write temp json {}", tmp.display()))?;
  fs::rename(&tmp, path)
    .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
  Ok(())
}

fn parse_ops_args(argv: &[String]) -> Result<OpsArgs> {
  let bin_name = argv
    .first()
    .and_then(|raw| Path::new(raw).file_name().and_then(|s| s.to_str()))
    .unwrap_or("pnix")
    .to_string();

  let mut op: Option<String> = None;
  let mut payload_raw: Option<String> = None;
  let mut caps: Vec<String> = Vec::new();
  let mut supervisor_sock: Option<String> = None;
  let mut emit_payload: Option<String> = None;
  let mut emit_preflight_report: Option<String> = None;
  let mut emit_change_snapshot: Option<String> = None;
  let mut emit_bundle: Option<String> = None;
  let mut preflight_max_age_ms: u64 = 300_000;
  let mut execute_flag = false;
  let mut dry_run_flag = false;
  let mut yes_flag = false;
  let mut no_confirm_flag = false;
  let mut confirm: Option<String> = None;
  let mut require_preflight: Option<String> = None;
  let mut require_bundle: Option<String> = None;
  let mut retry_preflight_only = false;
  let mut resume_after_refresh = false;
  let mut retry_max_attempts: u32 = 3;
  let mut retry_backoff_ms: Vec<u64> = vec![500, 1_500, 4_000];

  let mut i = if argv.get(1).map(|arg| arg == "ops").unwrap_or(false) {
    2
  } else {
    1
  };

  while i < argv.len() {
    match argv[i].as_str() {
      raw if raw.starts_with("--mode=") => {
        let value = raw.trim_start_matches("--mode=");
        if value != "ops" {
          anyhow::bail!(
            "ops invocation only supports --mode ops, got --mode {}",
            value
          );
        }
      }
      "--mode" => {
        i += 1;
        let value = argv
          .get(i)
          .ok_or_else(|| anyhow::anyhow!("--mode requires a value"))?;
        if value != "ops" {
          anyhow::bail!(
            "ops invocation only supports --mode ops, got --mode {}",
            value
          );
        }
      }
      "--supervisor-sock" => {
        i += 1;
        supervisor_sock = Some(
          argv
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("--supervisor-sock requires a value"))?
            .clone(),
        );
      }
      "--endpoint" => {
        i += 1;
        supervisor_sock = Some(
          argv
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("--endpoint requires a value"))?
            .clone(),
        );
      }
      "--op" => {
        i += 1;
        op = Some(
          argv
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("--op requires a value"))?
            .clone(),
        );
      }
      "--payload" => {
        i += 1;
        payload_raw = Some(
          argv
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("--payload requires a value"))?
            .clone(),
        );
      }
      "--emit-payload" => {
        i += 1;
        emit_payload = Some(
          argv
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("--emit-payload requires a value"))?
            .clone(),
        );
      }
      "--emit-preflight-report" => {
        i += 1;
        emit_preflight_report = Some(
          argv
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("--emit-preflight-report requires a value"))?
            .clone(),
        );
      }
      "--emit-change-snapshot" => {
        i += 1;
        emit_change_snapshot = Some(
          argv
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("--emit-change-snapshot requires a value"))?
            .clone(),
        );
      }
      "--emit-bundle" => {
        i += 1;
        emit_bundle = Some(
          argv
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("--emit-bundle requires a value"))?
            .clone(),
        );
      }
      "--preflight-max-age-ms" => {
        i += 1;
        let raw = argv
          .get(i)
          .ok_or_else(|| anyhow::anyhow!("--preflight-max-age-ms requires a value"))?;
        preflight_max_age_ms = parse_u64_text(raw, "preflight_max_age_ms")?;
      }
      "--execute" => execute_flag = true,
      "--dry-run" => dry_run_flag = true,
      "--yes" => yes_flag = true,
      "--no-confirm" => no_confirm_flag = true,
      "--confirm" => {
        i += 1;
        confirm = Some(
          argv
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("--confirm requires a value"))?
            .clone(),
        );
      }
      "--require-preflight" => {
        i += 1;
        require_preflight = Some(
          argv
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("--require-preflight requires a value"))?
            .clone(),
        );
      }
      "--require-bundle" => {
        i += 1;
        require_bundle = Some(
          argv
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("--require-bundle requires a value"))?
            .clone(),
        );
      }
      "--retry-preflight-only" => retry_preflight_only = true,
      "--resume-after-refresh" => resume_after_refresh = true,
      "--retry-max-attempts" => {
        i += 1;
        let raw = argv
          .get(i)
          .ok_or_else(|| anyhow::anyhow!("--retry-max-attempts requires a value"))?;
        let parsed = parse_u64_text(raw, "retry_max_attempts")?;
        if parsed == 0 || parsed > u32::MAX as u64 {
          anyhow::bail!("retry_max_attempts must be in range [1..={}]", u32::MAX);
        }
        retry_max_attempts = parsed as u32;
      }
      "--retry-backoff-ms" => {
        i += 1;
        let raw = argv
          .get(i)
          .ok_or_else(|| anyhow::anyhow!("--retry-backoff-ms requires a value"))?;
        retry_backoff_ms = parse_backoff_list(raw)?;
      }
      "--caps" => {
        i += 1;
        let raw = argv
          .get(i)
          .ok_or_else(|| anyhow::anyhow!("--caps requires a value"))?;
        caps.extend(parse_caps(raw));
      }
      "--help" | "-h" => {
        print_ops_help(bin_name.as_str());
        std::process::exit(0);
      }
      raw if raw.starts_with('-') => {
        anyhow::bail!("unknown ops flag '{}'", raw);
      }
      raw => {
        if op.is_none() {
          op = Some(raw.to_string());
        } else if payload_raw.is_none() {
          payload_raw = Some(raw.to_string());
        } else {
          anyhow::bail!("unexpected ops argument '{}'", raw);
        }
      }
    }
    i += 1;
  }

  let endpoint = resolve_supervisor_endpoint(supervisor_sock.as_deref());
  let op = op.unwrap_or_else(|| "list".to_string());
  let op_name = resolve_alias(op.as_str());
  let payload = normalize_payload_for_op(op_name.as_str(), parse_payload(payload_raw.as_deref())?)?;
  if dry_run_flag && execute_flag {
    anyhow::bail!("--dry-run and --execute cannot be used together");
  }
  let execute = execute_flag;
  if !execute && require_preflight.is_some() {
    anyhow::bail!("--require-preflight requires --execute");
  }
  if !execute && retry_preflight_only {
    anyhow::bail!("--retry-preflight-only requires --execute");
  }
  if !execute && emit_preflight_report.is_some() {
    anyhow::bail!("--emit-preflight-report requires --execute");
  }
  if !execute && emit_change_snapshot.is_some() {
    anyhow::bail!("--emit-change-snapshot requires --execute");
  }
  if !execute && emit_bundle.is_some() {
    anyhow::bail!("--emit-bundle requires --execute");
  }
  if preflight_max_age_ms == 0 {
    anyhow::bail!("--preflight-max-age-ms must be > 0");
  }
  if no_confirm_flag && yes_flag {
    anyhow::bail!("--no-confirm and --yes cannot be used together");
  }
  if retry_preflight_only && op_name != "change.run" {
    anyhow::bail!("--retry-preflight-only is only valid for change.run");
  }
  if retry_preflight_only && require_preflight.is_none() {
    anyhow::bail!("--retry-preflight-only requires --require-preflight");
  }
  if resume_after_refresh && !retry_preflight_only {
    anyhow::bail!("--resume-after-refresh requires --retry-preflight-only");
  }
  if require_preflight.is_some() && require_bundle.is_some() {
    anyhow::bail!("--require-preflight and --require-bundle are mutually exclusive");
  }
  if op_name != "change.run"
    && (require_bundle.is_some() || emit_bundle.is_some() || resume_after_refresh)
  {
    anyhow::bail!(
      "--require-bundle/--emit-bundle/--resume-after-refresh are only valid for change.run"
    );
  }
  if retry_backoff_ms.is_empty() {
    anyhow::bail!("--retry-backoff-ms must contain at least one duration");
  }

  Ok(OpsArgs {
    endpoint,
    op: op_name,
    payload,
    caps,
    emit_payload,
    emit_preflight_report,
    emit_change_snapshot,
    emit_bundle,
    preflight_max_age_ms,
    yes: yes_flag,
    no_confirm: no_confirm_flag,
    confirm,
    require_preflight,
    require_bundle,
    retry_preflight_only,
    resume_after_refresh,
    retry_max_attempts,
    retry_backoff_ms,
    execute,
  })
}

fn parse_caps(raw: &str) -> Vec<String> {
  raw
    .split(',')
    .map(str::trim)
    .filter(|v| !v.is_empty())
    .map(ToString::to_string)
    .collect()
}

fn parse_backoff_list(raw: &str) -> Result<Vec<u64>> {
  let mut out = Vec::new();
  for (idx, token) in raw.split(',').enumerate() {
    let text = token.trim();
    if text.is_empty() {
      anyhow::bail!("retry_backoff_ms entry {} must not be empty", idx);
    }
    let value = parse_duration_ms(text)
      .with_context(|| format!("retry_backoff_ms entry {} is invalid", idx))?;
    if value == 0 {
      anyhow::bail!("retry_backoff_ms entry {} must be > 0", idx);
    }
    out.push(value);
  }
  if out.is_empty() {
    anyhow::bail!("retry_backoff_ms must contain at least one entry");
  }
  Ok(out)
}

fn parse_payload(raw: Option<&str>) -> Result<Value> {
  let Some(raw) = raw else {
    return Ok(json!({}));
  };
  let text = if let Some(path) = raw.strip_prefix('@') {
    fs::read_to_string(path).with_context(|| format!("read payload file {}", path))?
  } else {
    raw.to_string()
  };
  if text.trim().is_empty() {
    return Ok(json!({}));
  }
  serde_json::from_str(text.as_str()).context("parse --payload JSON")
}

fn normalize_payload_for_op(op: &str, payload: Value) -> Result<Value> {
  let mut object = match payload {
    Value::Object(map) => map,
    Value::Null => serde_json::Map::new(),
    _ => anyhow::bail!("ops payload must be a JSON object"),
  };

  normalize_string_list(&mut object, "roles");
  normalize_string_list(&mut object, "caps");
  normalize_actor_fields(&mut object);

  if let Some(ns) = object.get("ns").and_then(Value::as_str) {
    validate_ns_or_channel("ns", ns)?;
  }
  if let Some(channel) = object.get("channel").and_then(Value::as_str) {
    validate_ns_or_channel("channel", channel)?;
  }

  normalize_duration_field(&mut object, "duration_ms", 24 * 60 * 60 * 1000)?;

  match op {
    "admission.check" => {
      normalize_admission_payload(&mut object)?;
    }
    "change.get" | "change.approve" | "change.reject" | "change.run" | "change.cancel" => {
      require_nonempty_string(&object, "change_id")?;
    }
    "remote.execute" => {
      require_nonempty_string(&object, "change_id")?;
      let capability_manifest = object
        .get("capability_manifest")
        .ok_or_else(|| anyhow::anyhow!("remote.execute requires capability_manifest"))?;
      if !capability_manifest.is_object() {
        anyhow::bail!("remote.execute capability_manifest must be an object");
      }
      let preflight = object
        .get("preflight")
        .ok_or_else(|| anyhow::anyhow!("remote.execute requires preflight"))?;
      if !preflight.is_object() {
        anyhow::bail!("remote.execute preflight must be an object");
      }
    }
    "change.list" => {
      normalize_limit_field(&mut object, "limit", 1, 10_000)?;
    }
    "breakglass.request" => {
      let reason = require_nonempty_string(&object, "reason")?;
      if reason.trim().chars().count() < 8 {
        anyhow::bail!("breakglass.request reason must be at least 8 chars");
      }
      normalize_u64_field(&mut object, "required_approvals", 1, 64)?;
      normalize_roles_json(&mut object, "required_roles_json");
    }
    "breakglass.session.list" => {
      normalize_limit_field(&mut object, "limit", 1, 10_000)?;
    }
    "dr.drill.start" => {
      require_nonempty_string(&object, "runbook_id")?;
      require_nonempty_string(&object, "scenario")?;
    }
    "compliance.policy.set" => {
      let policy_json = object
        .get("policy_json")
        .ok_or_else(|| anyhow::anyhow!("compliance.policy.set requires policy_json"))?;
      if !policy_json.is_object() {
        anyhow::bail!("compliance.policy.set policy_json must be an object");
      }
      let mode = policy_json
        .as_object()
        .and_then(|obj| obj.get("mode"))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
      if mode.is_empty() {
        anyhow::bail!("compliance.policy.set policy_json.mode is required");
      }
    }
    "compliance.exception.create" => {
      require_nonempty_string(&object, "target")?;
      require_nonempty_string(&object, "reason")?;
      normalize_u64_field(&mut object, "approvals_required", 1, 64)?;
    }
    _ => {}
  }

  Ok(Value::Object(object))
}

fn normalize_admission_payload(object: &mut serde_json::Map<String, Value>) -> Result<()> {
  let target = object
    .get("target")
    .and_then(Value::as_str)
    .map(str::trim)
    .unwrap_or("");
  if target.eq_ignore_ascii_case("exec.admit") {
    let subject = object
      .get_mut("subject")
      .and_then(Value::as_object_mut)
      .ok_or_else(|| {
        anyhow::anyhow!("admission.check target=exec.admit requires subject object")
      })?;
    let subject_id = subject
      .get("id")
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|value| !value.is_empty())
      .ok_or_else(|| anyhow::anyhow!("admission.check subject.id is required"))?
      .to_string();
    subject
      .entry("kind".to_string())
      .or_insert_with(|| Value::String("process".to_string()));

    let process_spec = object
      .get_mut("process_spec")
      .and_then(Value::as_object_mut)
      .ok_or_else(|| anyhow::anyhow!("admission.check target=exec.admit requires process_spec"))?;
    if process_spec
      .get("id")
      .and_then(Value::as_str)
      .unwrap_or("")
      .trim()
      .is_empty()
    {
      process_spec.insert("id".to_string(), Value::String(subject_id));
    }
    let exec = process_spec
      .get("exec")
      .and_then(Value::as_object)
      .ok_or_else(|| anyhow::anyhow!("admission.check process_spec.exec is required"))?;
    let entrypoint = exec
      .get("entrypoint")
      .and_then(Value::as_str)
      .map(str::trim)
      .unwrap_or("");
    if entrypoint.is_empty() {
      anyhow::bail!("admission.check process_spec.exec.entrypoint is required");
    }
    let exec_ref_id = exec
      .get("exec_ref_id")
      .and_then(Value::as_str)
      .map(str::trim)
      .unwrap_or("");
    if exec_ref_id.is_empty() {
      anyhow::bail!("admission.check process_spec.exec.exec_ref_id is required");
    }
  }
  Ok(())
}

fn normalize_actor_fields(object: &mut serde_json::Map<String, Value>) {
  if let Some(actor) = object.get_mut("actor").and_then(Value::as_object_mut) {
    if let Some(id) = actor.get("id").and_then(Value::as_str).map(str::trim) {
      actor.insert("id".to_string(), Value::String(id.to_string()));
    }
    if let Some(roles) = actor.get_mut("roles") {
      normalize_string_list_value(roles);
    }
  }
}

fn normalize_string_list(object: &mut serde_json::Map<String, Value>, field: &str) {
  if let Some(value) = object.get_mut(field) {
    normalize_string_list_value(value);
  }
}

fn normalize_roles_json(object: &mut serde_json::Map<String, Value>, field: &str) {
  if let Some(value) = object.get_mut(field) {
    normalize_string_list_value(value);
  }
}

fn normalize_string_list_value(value: &mut Value) {
  match value {
    Value::Array(items) => {
      let mut normalized = Vec::with_capacity(items.len());
      for item in items.iter() {
        if let Some(entry) = item
          .as_str()
          .map(str::trim)
          .filter(|entry| !entry.is_empty())
        {
          normalized.push(Value::String(entry.to_string()));
        }
      }
      *value = Value::Array(normalized);
    }
    Value::String(raw) => {
      let normalized = raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| Value::String(entry.to_string()))
        .collect::<Vec<_>>();
      *value = Value::Array(normalized);
    }
    _ => {}
  }
}

fn normalize_limit_field(
  object: &mut serde_json::Map<String, Value>,
  field: &str,
  min: u64,
  max: u64,
) -> Result<()> {
  if object.get(field).is_none() {
    return Ok(());
  }
  normalize_u64_field(object, field, min, max)
}

fn normalize_u64_field(
  object: &mut serde_json::Map<String, Value>,
  field: &str,
  min: u64,
  max: u64,
) -> Result<()> {
  let Some(value) = object.get(field) else {
    return Ok(());
  };
  let parsed = match value {
    Value::Number(number) => number
      .as_u64()
      .ok_or_else(|| anyhow::anyhow!("{} must be a non-negative integer", field))?,
    Value::String(raw) => parse_u64_text(raw, field)?,
    _ => anyhow::bail!("{} must be integer or integer-string", field),
  };
  if parsed < min || parsed > max {
    anyhow::bail!("{} must be in range [{}..={}]", field, min, max);
  }
  object.insert(
    field.to_string(),
    Value::Number(serde_json::Number::from(parsed)),
  );
  Ok(())
}

fn normalize_duration_field(
  object: &mut serde_json::Map<String, Value>,
  field: &str,
  max_ms: u64,
) -> Result<()> {
  let Some(value) = object.get(field) else {
    return Ok(());
  };
  let duration_ms = match value {
    Value::Number(number) => number
      .as_u64()
      .ok_or_else(|| anyhow::anyhow!("{} must be a non-negative integer", field))?,
    Value::String(text) => parse_duration_ms(text)?,
    _ => anyhow::bail!("{} must be integer or duration string (e.g. 30m)", field),
  };
  if duration_ms == 0 || duration_ms > max_ms {
    anyhow::bail!("{} must be in range [1..={}]", field, max_ms);
  }
  object.insert(
    field.to_string(),
    Value::Number(serde_json::Number::from(duration_ms)),
  );
  Ok(())
}

fn parse_duration_ms(raw: &str) -> Result<u64> {
  let text = raw.trim();
  if text.is_empty() {
    anyhow::bail!("duration value cannot be empty");
  }
  if let Ok(ms) = text.parse::<u64>() {
    return Ok(ms);
  }

  let (digits, unit) = text.split_at(text.len().saturating_sub(1));
  let base = parse_u64_text(digits, "duration")?;
  let factor = match unit {
    "s" | "S" => 1_000,
    "m" | "M" => 60 * 1_000,
    "h" | "H" => 60 * 60 * 1_000,
    _ => anyhow::bail!("duration unit must be one of s/m/h"),
  };
  base
    .checked_mul(factor)
    .ok_or_else(|| anyhow::anyhow!("duration overflow"))
}

fn parse_u64_text(raw: &str, field: &str) -> Result<u64> {
  raw
    .trim()
    .parse::<u64>()
    .with_context(|| format!("{} must be a non-negative integer", field))
}

fn require_nonempty_string<'a>(
  object: &'a serde_json::Map<String, Value>,
  field: &str,
) -> Result<&'a str> {
  object
    .get(field)
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .ok_or_else(|| anyhow::anyhow!("{} is required", field))
}

fn validate_ns_or_channel(name: &str, value: &str) -> Result<()> {
  if value.is_empty() {
    anyhow::bail!("{} must not be empty", name);
  }
  let valid = value
    .chars()
    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'));
  if !valid {
    anyhow::bail!(
      "{} contains invalid characters; allowed: [A-Za-z0-9._-]",
      name
    );
  }
  Ok(())
}

fn enforce_required_preflight(path: Option<&str>, payload: &Value) -> Result<Option<Value>> {
  let Some(path) = path else {
    return Ok(None);
  };
  let source = if path.trim_start().starts_with('@') {
    path.to_string()
  } else {
    format!("@{}", path)
  };
  let report = parse_payload(Some(source.as_str())).map_err(|err| {
    usage_error(
      format!("preflight report parse failed: {}", err),
      json!({
        "preflight_path": path,
        "reason": "PREFLIGHT_PARSE_FAILED",
      }),
    )
  })?;
  let validated = validate_preflight_report(&report, payload, path)?;
  Ok(Some(validated))
}

fn validate_preflight_report(
  report: &Value,
  payload: &Value,
  preflight_path: &str,
) -> Result<Value> {
  if !report.is_object() {
    return Err(usage_error(
      "preflight report must be a JSON object",
      json!({
        "preflight_path": preflight_path,
        "reason": "PREFLIGHT_SCHEMA_MISMATCH",
      }),
    ));
  }
  let schema = report.get("schema").and_then(Value::as_str).unwrap_or("");
  if schema != "pnix-preflight-report@0.1" {
    return Err(usage_error(
      format!(
        "preflight report schema mismatch: expected pnix-preflight-report@0.1, got {}",
        schema
      ),
      json!({
        "preflight_path": preflight_path,
        "reason": "PREFLIGHT_SCHEMA_MISMATCH",
      }),
    ));
  }

  let now = now_ms();
  let expires_ms = report
    .get("expires_ms")
    .and_then(Value::as_i64)
    .ok_or_else(|| {
      usage_error(
        "preflight report missing expires_ms",
        json!({
          "preflight_path": preflight_path,
          "reason": "PREFLIGHT_SCHEMA_MISMATCH",
        }),
      )
    })?;
  if now > expires_ms {
    return Err(denied_error(
      "preflight report expired",
      json!({
        "preflight_path": preflight_path,
        "reason": "PREFLIGHT_EXPIRED",
        "expires_ms": expires_ms,
        "now_ms": now,
      }),
    ));
  }

  let decision = report
    .get("result")
    .and_then(Value::as_object)
    .and_then(|result| result.get("decision"))
    .and_then(Value::as_str)
    .or_else(|| {
      report
        .get("output")
        .and_then(Value::as_object)
        .and_then(|output| output.get("decision"))
        .and_then(Value::as_str)
    })
    .or_else(|| report.get("decision").and_then(Value::as_str))
    .map(str::trim)
    .unwrap_or("");
  let allow = report
    .get("output")
    .and_then(Value::as_object)
    .and_then(|output| output.get("allow"))
    .and_then(Value::as_bool)
    .or_else(|| report.get("allow").and_then(Value::as_bool))
    .unwrap_or(decision.eq_ignore_ascii_case("allow"));
  if !allow || !decision.eq_ignore_ascii_case("allow") {
    let admission_reasons = report
      .get("output")
      .and_then(Value::as_object)
      .and_then(|output| output.get("reasons"))
      .cloned()
      .unwrap_or_else(|| json!([]));
    return Err(denied_error(
      "preflight denied",
      json!({
        "preflight_path": preflight_path,
        "reason": "PREFLIGHT_DENIED",
        "admission_reasons": admission_reasons,
      }),
    ));
  }

  let binding = report
    .get("binding")
    .and_then(Value::as_object)
    .ok_or_else(|| {
      usage_error(
        "preflight report missing binding object",
        json!({
          "preflight_path": preflight_path,
          "reason": "PREFLIGHT_SCHEMA_MISMATCH",
        }),
      )
    })?;
  let current_ns = payload
    .get("ns")
    .and_then(Value::as_str)
    .unwrap_or("default");
  let current_channel = payload
    .get("channel")
    .and_then(Value::as_str)
    .unwrap_or("prod");
  let binding_ns = binding.get("ns").and_then(Value::as_str).unwrap_or("");
  let binding_channel = binding.get("channel").and_then(Value::as_str).unwrap_or("");
  if binding_ns != current_ns || binding_channel != current_channel {
    return Err(denied_error(
      "preflight binding mismatch (ns/channel)",
      json!({
        "preflight_path": preflight_path,
        "reason": "PREFLIGHT_BINDING_MISMATCH",
        "binding_expected": { "ns": current_ns, "channel": current_channel },
        "binding_got": { "ns": binding_ns, "channel": binding_channel },
      }),
    ));
  }
  let target = binding.get("target").and_then(Value::as_str).unwrap_or("");
  if target != "exec.admit" {
    return Err(denied_error(
      "preflight binding target must be exec.admit",
      json!({
        "preflight_path": preflight_path,
        "reason": "PREFLIGHT_BINDING_MISMATCH",
        "binding_expected": { "target": "exec.admit" },
        "binding_got": { "target": target },
      }),
    ));
  }

  if let Some(change_id) = payload.get("change_id").and_then(Value::as_str) {
    if let Some(report_change_id) = binding.get("change_id").and_then(Value::as_str) {
      if report_change_id != change_id {
        return Err(denied_error(
          format!(
            "preflight report change_id mismatch (expected {}, got {})",
            change_id, report_change_id
          ),
          json!({
            "preflight_path": preflight_path,
            "reason": "PREFLIGHT_BINDING_MISMATCH",
            "binding_expected": { "change_id": change_id },
            "binding_got": { "change_id": report_change_id },
          }),
        ));
      }
    }
  }

  let report_breakglass = report
    .get("input")
    .and_then(Value::as_object)
    .and_then(|input| input.get("breakglass"))
    .and_then(Value::as_bool)
    .or_else(|| {
      report
        .get("input")
        .and_then(Value::as_object)
        .and_then(|input| input.get("actor"))
        .and_then(Value::as_object)
        .and_then(|actor| actor.get("breakglass"))
        .and_then(Value::as_bool)
    })
    .unwrap_or(false);
  let current_breakglass = payload
    .get("breakglass")
    .and_then(Value::as_bool)
    .or_else(|| {
      payload
        .get("actor")
        .and_then(Value::as_object)
        .and_then(|actor| actor.get("breakglass"))
        .and_then(Value::as_bool)
    })
    .unwrap_or(false);
  if report_breakglass && !current_breakglass {
    return Err(denied_error(
      "preflight breakglass context mismatch",
      json!({
        "preflight_path": preflight_path,
        "reason": "PREFLIGHT_BREAKGLASS_MISMATCH",
        "binding_expected": { "breakglass": report_breakglass },
        "binding_got": { "breakglass": current_breakglass },
      }),
    ));
  }
  Ok(report.clone())
}

fn enforce_required_bundle(path: Option<&str>, payload: &Value) -> Result<Option<RequiredBundle>> {
  let Some(path) = path else {
    return Ok(None);
  };
  let source = if path.trim_start().starts_with('@') {
    path.to_string()
  } else {
    format!("@{}", path)
  };
  let bundle = parse_payload(Some(source.as_str())).map_err(|err| {
    usage_error(
      format!("bundle parse failed: {}", err),
      json!({
        "bundle_path": path,
        "reason": "BUNDLE_PARSE_FAILED",
      }),
    )
  })?;
  if !bundle.is_object() {
    return Err(usage_error(
      "bundle must be a JSON object",
      json!({
        "bundle_path": path,
        "reason": "BUNDLE_SCHEMA_MISMATCH",
      }),
    ));
  }
  let schema = bundle.get("schema").and_then(Value::as_str).unwrap_or("");
  if schema != "pnix-safe-run-bundle@0.1" {
    return Err(usage_error(
      format!(
        "bundle schema mismatch: expected pnix-safe-run-bundle@0.1, got {}",
        schema
      ),
      json!({
        "bundle_path": path,
        "reason": "BUNDLE_SCHEMA_MISMATCH",
      }),
    ));
  }

  let policy = bundle
    .get("policy")
    .and_then(Value::as_object)
    .ok_or_else(|| {
      usage_error(
        "bundle missing policy object",
        json!({
          "bundle_path": path,
          "reason": "BUNDLE_SCHEMA_MISMATCH",
        }),
      )
    })?;
  let expires_ms = policy
    .get("expires_ms")
    .and_then(Value::as_i64)
    .ok_or_else(|| {
      usage_error(
        "bundle policy missing expires_ms",
        json!({
          "bundle_path": path,
          "reason": "BUNDLE_SCHEMA_MISMATCH",
        }),
      )
    })?;
  let now = now_ms();
  if now > expires_ms {
    return Err(denied_error(
      "bundle expired",
      json!({
        "bundle_path": path,
        "reason": "BUNDLE_EXPIRED",
        "expires_ms": expires_ms,
        "now_ms": now,
      }),
    ));
  }

  let binding = bundle
    .get("binding")
    .and_then(Value::as_object)
    .ok_or_else(|| {
      usage_error(
        "bundle missing binding object",
        json!({
          "bundle_path": path,
          "reason": "BUNDLE_SCHEMA_MISMATCH",
        }),
      )
    })?;
  let current_ns = payload
    .get("ns")
    .and_then(Value::as_str)
    .unwrap_or("default");
  let current_channel = payload
    .get("channel")
    .and_then(Value::as_str)
    .unwrap_or("prod");
  let current_change_id = payload
    .get("change_id")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .ok_or_else(|| {
      usage_error(
        "change.run payload must include change_id when --require-bundle is used",
        json!({
          "bundle_path": path,
          "reason": "BUNDLE_BINDING_MISMATCH",
        }),
      )
    })?;
  let expected_ns = binding.get("ns").and_then(Value::as_str).unwrap_or("");
  let expected_channel = binding.get("channel").and_then(Value::as_str).unwrap_or("");
  let expected_change_id = binding
    .get("change_id")
    .and_then(Value::as_str)
    .unwrap_or("");
  if expected_ns != current_ns
    || expected_channel != current_channel
    || expected_change_id != current_change_id
  {
    return Err(denied_error(
      "bundle binding mismatch (ns/channel/change_id)",
      json!({
        "bundle_path": path,
        "reason": "BUNDLE_BINDING_MISMATCH",
        "binding_expected": {
          "ns": current_ns,
          "channel": current_channel,
          "change_id": current_change_id
        },
        "binding_got": {
          "ns": expected_ns,
          "channel": expected_channel,
          "change_id": expected_change_id
        },
      }),
    ));
  }
  let expected_target = binding
    .get("target")
    .and_then(Value::as_str)
    .unwrap_or("exec.admit");
  if expected_target != "exec.admit" {
    return Err(denied_error(
      "bundle binding target must be exec.admit",
      json!({
        "bundle_path": path,
        "reason": "BUNDLE_BINDING_MISMATCH",
        "binding_expected": { "target": "exec.admit" },
        "binding_got": { "target": expected_target },
      }),
    ));
  }
  if let Some(current_subject_id) = payload
    .get("subject")
    .and_then(Value::as_object)
    .and_then(|subject| subject.get("id"))
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    if let Some(expected_subject_id) = binding
      .get("subject_id")
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|value| !value.is_empty())
    {
      if expected_subject_id != current_subject_id {
        return Err(denied_error(
          "bundle binding mismatch (subject_id)",
          json!({
            "bundle_path": path,
            "reason": "BUNDLE_BINDING_MISMATCH",
            "binding_expected": { "subject_id": current_subject_id },
            "binding_got": { "subject_id": expected_subject_id },
          }),
        ));
      }
    }
  }

  let embedded = bundle
    .get("embedded")
    .and_then(Value::as_object)
    .ok_or_else(|| {
      usage_error(
        "bundle missing embedded object",
        json!({
          "bundle_path": path,
          "reason": "BUNDLE_SCHEMA_MISMATCH",
        }),
      )
    })?;
  let preflight_report = embedded.get("preflight_report").cloned().ok_or_else(|| {
    usage_error(
      "bundle missing embedded.preflight_report",
      json!({
        "bundle_path": path,
        "reason": "BUNDLE_SCHEMA_MISMATCH",
      }),
    )
  })?;
  let change_snapshot = embedded.get("change_snapshot").cloned().ok_or_else(|| {
    usage_error(
      "bundle missing embedded.change_snapshot",
      json!({
        "bundle_path": path,
        "reason": "BUNDLE_SCHEMA_MISMATCH",
      }),
    )
  })?;
  let _ = validate_preflight_report(&preflight_report, payload, path)?;
  let snapshot_schema = change_snapshot
    .get("schema")
    .and_then(Value::as_str)
    .unwrap_or("");
  if snapshot_schema != "pnix-change-snapshot@0.1" {
    return Err(usage_error(
      format!(
        "bundle embedded change_snapshot schema mismatch: expected pnix-change-snapshot@0.1, got {}",
        snapshot_schema
      ),
      json!({
        "bundle_path": path,
        "reason": "BUNDLE_SCHEMA_MISMATCH",
      }),
    ));
  }

  let digests = bundle.get("digests").and_then(Value::as_object);
  if let Some(expected) = digests
    .and_then(|obj| obj.get("preflight_report_sha256"))
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    let got = sha256_value_hex(&preflight_report)?;
    if expected != got {
      return Err(denied_error(
        "bundle digest mismatch (preflight_report_sha256)",
        json!({
          "bundle_path": path,
          "reason": "BUNDLE_BINDING_MISMATCH",
          "binding_expected": { "preflight_report_sha256": expected },
          "binding_got": { "preflight_report_sha256": got },
        }),
      ));
    }
  }
  if let Some(expected) = digests
    .and_then(|obj| obj.get("change_snapshot_sha256"))
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    let got = sha256_value_hex(&change_snapshot)?;
    if expected != got {
      return Err(denied_error(
        "bundle digest mismatch (change_snapshot_sha256)",
        json!({
          "bundle_path": path,
          "reason": "BUNDLE_BINDING_MISMATCH",
          "binding_expected": { "change_snapshot_sha256": expected },
          "binding_got": { "change_snapshot_sha256": got },
        }),
      ));
    }
  }
  let snapshot_plan_core_sha256 = change_snapshot
    .get("digests")
    .and_then(Value::as_object)
    .and_then(|obj| obj.get("plan_core_sha256"))
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(ToString::to_string);
  let expected_plan_core_sha256 = digests
    .and_then(|obj| obj.get("change_plan_core_sha256"))
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(ToString::to_string)
    .or(snapshot_plan_core_sha256);
  if let Some(expected) = expected_plan_core_sha256.as_deref() {
    if let Some(got) = change_snapshot
      .get("digests")
      .and_then(Value::as_object)
      .and_then(|obj| obj.get("plan_core_sha256"))
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|value| !value.is_empty())
    {
      if expected != got {
        return Err(denied_error(
          "bundle digest mismatch (change_plan_core_sha256)",
          json!({
            "bundle_path": path,
            "reason": "BUNDLE_BINDING_MISMATCH",
            "binding_expected": { "change_plan_core_sha256": expected },
            "binding_got": { "change_plan_core_sha256": got },
          }),
        ));
      }
    }
  }

  Ok(Some(RequiredBundle {
    source_path: path.to_string(),
    binding: binding.clone(),
    preflight_report,
    expected_plan_core_sha256,
  }))
}

fn enforce_change_get_binding(
  report: &Value,
  preflight_path: &str,
  payload: &Value,
  client: &SupervisorClient,
  caps: &[String],
) -> Result<()> {
  let change_id = payload
    .get("change_id")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .ok_or_else(|| {
      usage_error(
        "change.run payload must include change_id when --require-preflight is used",
        json!({
          "preflight_path": preflight_path,
          "reason": "PREFLIGHT_BINDING_MISMATCH",
        }),
      )
    })?;
  let ns = payload
    .get("ns")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .unwrap_or("default");
  let binding = report
    .get("binding")
    .and_then(Value::as_object)
    .ok_or_else(|| {
      usage_error(
        "preflight report missing binding object",
        json!({
          "preflight_path": preflight_path,
          "change_id": change_id,
          "reason": "PREFLIGHT_SCHEMA_MISMATCH",
        }),
      )
    })?;
  let report_change_id = binding
    .get("change_id")
    .and_then(Value::as_str)
    .or_else(|| {
      report
        .get("input")
        .and_then(Value::as_object)
        .and_then(|input| input.get("links"))
        .and_then(Value::as_object)
        .and_then(|links| links.get("change_id"))
        .and_then(Value::as_str)
    });

  let cap_refs = caps.iter().map(String::as_str).collect::<Vec<_>>();
  let change_get = client
    .call_with(
      "change.get",
      &cap_refs,
      json!({
        "ns": ns,
        "change_id": change_id
      }),
    )
    .map_err(|err| {
      let error = err.to_string();
      let code = classify_change_get_error_code(error.as_str());
      anyhow::Error::new(OpsStructuredError {
        code,
        message: format!("change.get failed: {err:#}"),
        details: json!({
          "preflight_path": preflight_path,
          "change_id": change_id,
          "reason": "CHANGE_GET_FAILED",
          "change_get_error": error,
        }),
      })
    })?;

  let change_ns = change_get
    .get("ns")
    .and_then(Value::as_str)
    .unwrap_or_default();
  if change_ns != ns {
    return Err(denied_error(
      "preflight binding mismatch (change.get ns)",
      json!({
        "preflight_path": preflight_path,
        "change_id": change_id,
        "reason": "PREFLIGHT_BINDING_MISMATCH",
        "binding_expected": { "ns": ns },
        "binding_got": { "ns": change_ns },
      }),
    ));
  }

  let change = change_get
    .get("change")
    .and_then(Value::as_object)
    .ok_or_else(|| {
      anyhow::Error::new(OpsStructuredError {
        code: OpsErrorCode::Internal,
        message: "change.get response missing change object".to_string(),
        details: json!({
          "preflight_path": preflight_path,
          "change_id": change_id,
          "reason": "CHANGE_GET_FAILED",
        }),
      })
    })?;

  let change_obj_id = change
    .get("change_id")
    .and_then(Value::as_str)
    .unwrap_or_default();
  if change_obj_id != change_id {
    return Err(denied_error(
      "preflight binding mismatch (change_id)",
      json!({
        "preflight_path": preflight_path,
        "change_id": change_id,
        "reason": "PREFLIGHT_BINDING_MISMATCH",
        "binding_expected": { "change_id": change_id },
        "binding_got": { "change_id": change_obj_id },
      }),
    ));
  }

  if let Some(expected_change_id) = report_change_id {
    if expected_change_id != change_obj_id {
      return Err(denied_error(
        "preflight report change_id mismatch with change.get",
        json!({
          "preflight_path": preflight_path,
          "change_id": change_id,
          "reason": "PREFLIGHT_BINDING_MISMATCH",
          "binding_expected": { "change_id": expected_change_id },
          "binding_got": { "change_id": change_obj_id },
        }),
      ));
    }
  }

  if let Some(expected_invocation_id) = report
    .get("input")
    .and_then(Value::as_object)
    .and_then(|input| input.get("links"))
    .and_then(Value::as_object)
    .and_then(|links| links.get("invocation_id"))
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    if let Some(got_invocation_id) = change
      .get("invocation_id")
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|value| !value.is_empty())
    {
      if got_invocation_id != expected_invocation_id {
        return Err(denied_error(
          "preflight invocation_id mismatch",
          json!({
            "preflight_path": preflight_path,
            "change_id": change_id,
            "reason": "PREFLIGHT_BINDING_MISMATCH",
            "binding_expected": { "invocation_id": expected_invocation_id },
            "binding_got": { "invocation_id": got_invocation_id },
          }),
        ));
      }
    }
  }

  let change_value = Value::Object(change.clone());
  let current_plan_sha = canonical_sha256_value_hex(&change_plan_material(&change_value))
    .context("compute change plan digest")?;
  let current_plan_core_sha = canonical_sha256_value_hex(&change_plan_core_material(&change_value))
    .context("compute change plan core digest")?;
  let current_plan_meta_sha = canonical_sha256_value_hex(&change_plan_meta_material(&change_value))
    .context("compute change plan meta digest")?;

  let plan_diff_view_or_unknown = build_change_plan_diff(
    binding,
    &change_value,
    "PLAN_VIEW_OR_UNKNOWN_CHANGED",
    ns,
    change_id,
  );
  let plan_diff_core =
    build_change_plan_diff(binding, &change_value, "PLAN_CORE_CHANGED", ns, change_id);
  let plan_diff_meta = build_change_plan_diff(
    binding,
    &change_value,
    "PLAN_META_ONLY_CHANGED",
    ns,
    change_id,
  );

  if let Some(expected_plan_sha) = binding
    .get("change_plan_sha256")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    if expected_plan_sha != current_plan_sha {
      return Err(denied_error(
        "preflight binding mismatch (change_plan_sha256)",
        json!({
          "preflight_path": preflight_path,
          "change_id": change_id,
          "reason": "PREFLIGHT_BINDING_MISMATCH",
          "plan_change_class": "PLAN_VIEW_OR_UNKNOWN_CHANGED",
          "binding_expected": { "change_plan_sha256": expected_plan_sha },
          "binding_got": { "change_plan_sha256": current_plan_sha },
          "plan_diff": plan_diff_view_or_unknown,
        }),
      ));
    }
  }
  if let Some(expected_core_sha) = binding
    .get("change_plan_core_sha256")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    if expected_core_sha != current_plan_core_sha {
      return Err(denied_error(
        "preflight binding mismatch (change_plan_core_sha256)",
        json!({
          "preflight_path": preflight_path,
          "change_id": change_id,
          "reason": "PREFLIGHT_BINDING_MISMATCH",
          "plan_change_class": "PLAN_CORE_CHANGED",
          "binding_expected": { "change_plan_core_sha256": expected_core_sha },
          "binding_got": { "change_plan_core_sha256": current_plan_core_sha },
          "plan_diff": plan_diff_core,
        }),
      ));
    }
  }
  if let Some(expected_meta_sha) = binding
    .get("change_plan_meta_sha256")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    if expected_meta_sha != current_plan_meta_sha {
      return Err(denied_error(
        "preflight binding mismatch (change_plan_meta_sha256)",
        json!({
          "preflight_path": preflight_path,
          "change_id": change_id,
          "reason": "PREFLIGHT_BINDING_MISMATCH",
          "plan_change_class": "PLAN_META_ONLY_CHANGED",
          "binding_expected": { "change_plan_meta_sha256": expected_meta_sha },
          "binding_got": { "change_plan_meta_sha256": current_plan_meta_sha },
          "plan_diff": plan_diff_meta,
        }),
      ));
    }
  }
  let current_view_sha = canonical_sha256_value_hex(&json!({
    "ns": change_ns,
    "change": change,
  }))
  .context("compute change view digest")?;
  if let Some(expected_view_sha) = binding
    .get("change_view_sha256")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    if expected_view_sha != current_view_sha {
      return Err(denied_error(
        "preflight binding mismatch (change_view_sha256)",
        json!({
          "preflight_path": preflight_path,
          "change_id": change_id,
          "reason": "PREFLIGHT_BINDING_MISMATCH",
          "binding_expected": { "change_view_sha256": expected_view_sha },
          "binding_got": { "change_view_sha256": current_view_sha },
        }),
      ));
    }
  }

  // Optional strict fields: compare only when both sides expose the field.
  if let Some(expected_channel) = binding
    .get("channel")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    if let Some(got_channel) = change
      .get("channel")
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|value| !value.is_empty())
    {
      if got_channel != expected_channel {
        return Err(denied_error(
          "preflight binding mismatch (channel)",
          json!({
            "preflight_path": preflight_path,
            "change_id": change_id,
            "reason": "PREFLIGHT_BINDING_MISMATCH",
            "binding_expected": { "channel": expected_channel },
            "binding_got": { "channel": got_channel },
          }),
        ));
      }
    }
  }
  if let Some(expected_subject) = binding
    .get("subject_id")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    if let Some(got_subject) = change
      .get("subject_id")
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|value| !value.is_empty())
    {
      if got_subject != expected_subject {
        return Err(denied_error(
          "preflight binding mismatch (subject_id)",
          json!({
            "preflight_path": preflight_path,
            "change_id": change_id,
            "reason": "PREFLIGHT_BINDING_MISMATCH",
            "binding_expected": { "subject_id": expected_subject },
            "binding_got": { "subject_id": got_subject },
          }),
        ));
      }
    }
  }
  if let Some(expected_target) = binding
    .get("target")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    if let Some(got_target) = change
      .get("target")
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|value| !value.is_empty())
    {
      if got_target != expected_target {
        return Err(denied_error(
          "preflight binding mismatch (target)",
          json!({
            "preflight_path": preflight_path,
            "change_id": change_id,
            "reason": "PREFLIGHT_BINDING_MISMATCH",
            "binding_expected": { "target": expected_target },
            "binding_got": { "target": got_target },
          }),
        ));
      }
    }
  }
  if let Some(expected_spec_digest) = binding
    .get("process_spec_sha256")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    if let Some(got_spec_digest) = change
      .get("process_spec_sha256")
      .and_then(Value::as_str)
      .map(str::trim)
      .filter(|value| !value.is_empty())
    {
      if got_spec_digest != expected_spec_digest {
        return Err(denied_error(
          "preflight binding mismatch (process_spec_sha256)",
          json!({
            "preflight_path": preflight_path,
            "change_id": change_id,
            "reason": "PREFLIGHT_BINDING_MISMATCH",
            "binding_expected": { "process_spec_sha256": expected_spec_digest },
            "binding_got": { "process_spec_sha256": got_spec_digest },
          }),
        ));
      }
    }
  }

  Ok(())
}

fn enforce_bundle_change_get_binding(
  bundle: &RequiredBundle,
  payload: &Value,
  client: &SupervisorClient,
  caps: &[String],
) -> Result<()> {
  let change_id = payload
    .get("change_id")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .ok_or_else(|| {
      usage_error(
        "change.run payload must include change_id when --require-bundle is used",
        json!({
          "bundle_path": bundle.source_path,
          "reason": "BUNDLE_BINDING_MISMATCH",
        }),
      )
    })?;
  let ns = payload
    .get("ns")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .unwrap_or("default");
  let cap_refs = caps.iter().map(String::as_str).collect::<Vec<_>>();
  let change_get = client
    .call_with(
      "change.get",
      &cap_refs,
      json!({
        "ns": ns,
        "change_id": change_id
      }),
    )
    .map_err(|err| {
      let error = err.to_string();
      let code = classify_change_get_error_code(error.as_str());
      anyhow::Error::new(OpsStructuredError {
        code,
        message: format!("change.get failed: {err:#}"),
        details: json!({
          "bundle_path": bundle.source_path,
          "change_id": change_id,
          "reason": "CHANGE_GET_FAILED",
          "change_get_error": error,
        }),
      })
    })?;
  let change = change_get
    .get("change")
    .and_then(Value::as_object)
    .ok_or_else(|| {
      anyhow::Error::new(OpsStructuredError {
        code: OpsErrorCode::Internal,
        message: "change.get response missing change object".to_string(),
        details: json!({
          "bundle_path": bundle.source_path,
          "change_id": change_id,
          "reason": "CHANGE_GET_FAILED",
        }),
      })
    })?;
  let change_value = Value::Object(change.clone());
  let current_plan_core_sha = canonical_sha256_value_hex(&change_plan_core_material(&change_value))
    .context("compute current change plan core digest")?;

  if let Some(expected_plan_core_sha) = bundle.expected_plan_core_sha256.as_deref() {
    if expected_plan_core_sha != current_plan_core_sha {
      let plan_diff = build_change_plan_diff(
        &bundle.binding,
        &change_value,
        "PLAN_CORE_CHANGED",
        ns,
        change_id,
      );
      return Err(denied_error(
        "change plan changed since bundle creation",
        json!({
          "bundle_path": bundle.source_path,
          "change_id": change_id,
          "reason": "CHANGE_PLAN_CHANGED_SINCE_BUNDLE",
          "binding_expected": { "change_plan_core_sha256": expected_plan_core_sha },
          "binding_got": { "change_plan_core_sha256": current_plan_core_sha },
          "plan_diff": plan_diff,
        }),
      ));
    }
  }
  Ok(())
}

fn classify_change_get_error_code(message: &str) -> OpsErrorCode {
  let msg = message.to_ascii_lowercase();
  if msg.contains("not found") {
    return OpsErrorCode::NotFound;
  }
  if msg.contains("timeout") || msg.contains("timed out") {
    return OpsErrorCode::Timeout;
  }
  if msg.contains("connect")
    || msg.contains("connection refused")
    || msg.contains("transport")
    || msg.contains("tls")
  {
    return OpsErrorCode::Transport;
  }
  if msg.contains("unauthorized")
    || msg.contains("permission denied")
    || msg.contains("forbidden")
    || (msg.contains("token") && msg.contains("invalid"))
  {
    return OpsErrorCode::Unauthorized;
  }
  OpsErrorCode::Denied
}

fn op_risk_level(op_name: &str) -> RiskLevel {
  match op_name {
    "change.run" | "remote.execute" | "breakglass.request" | "compliance.policy.set" => {
      RiskLevel::High
    }
    "change.approve" => RiskLevel::Medium,
    _ => RiskLevel::Low,
  }
}

fn maybe_emit_preflight_report(
  args: &OpsArgs,
  op_name: &str,
  request_id: &str,
  ts_ms: i64,
  outputs: &Value,
) -> Result<()> {
  let Some(path) = args.emit_preflight_report.as_deref() else {
    return Ok(());
  };
  if op_name != "admission.check" {
    return Err(usage_error(
      "--emit-preflight-report is only valid for admission.check",
      json!({
        "reason": "PREFLIGHT_REPORT_OP_MISMATCH",
        "op": op_name,
      }),
    ));
  }
  let issued_ms = ts_ms;
  let max_age_ms = args.preflight_max_age_ms as i64;
  let expires_ms = issued_ms.saturating_add(max_age_ms);
  let ns = payload_string_field(&args.payload, "ns").unwrap_or_else(|| "default".to_string());
  let channel =
    payload_string_field(&args.payload, "channel").unwrap_or_else(|| "prod".to_string());
  let target =
    payload_string_field(&args.payload, "target").unwrap_or_else(|| "exec.admit".to_string());
  let subject = args
    .payload
    .get("subject")
    .and_then(Value::as_object)
    .cloned()
    .unwrap_or_default();
  let subject_kind = subject
    .get("kind")
    .and_then(Value::as_str)
    .unwrap_or("process")
    .to_string();
  let subject_id = subject
    .get("id")
    .and_then(Value::as_str)
    .unwrap_or("")
    .to_string();
  let links = normalize_preflight_links(&args.payload, request_id, op_name, &args.endpoint);
  let change_id = links
    .get("change_id")
    .and_then(Value::as_str)
    .or_else(|| args.payload.get("change_id").and_then(Value::as_str));
  let change_plan_sha256 = links
    .get("change_plan_sha256")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty());
  let change_plan_core_sha256 = links
    .get("change_plan_core_sha256")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty());
  let change_plan_meta_sha256 = links
    .get("change_plan_meta_sha256")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty());
  let change_view_sha256 = links
    .get("change_view_sha256")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty());
  let process_spec_sha256 = args
    .payload
    .get("process_spec")
    .map(canonical_sha256_value_hex)
    .transpose()?;
  let breakglass = args
    .payload
    .get("breakglass")
    .and_then(Value::as_bool)
    .or_else(|| {
      args
        .payload
        .get("actor")
        .and_then(Value::as_object)
        .and_then(|actor| actor.get("breakglass"))
        .and_then(Value::as_bool)
    })
    .unwrap_or(false);

  let input_payload =
    build_preflight_input_payload(&args.payload, issued_ms, &ns, &channel, &target, &links);
  let output_payload =
    build_preflight_output_payload(outputs, &target, &subject_kind, &subject_id, &channel);
  let input_sha = sha256_value_hex(&input_payload)?;
  let output_sha = sha256_value_hex(&output_payload)?;

  let report = json!({
    "schema": "pnix-preflight-report@0.1",
    "issued_ms": issued_ms,
    "max_age_ms": max_age_ms,
    "expires_ms": expires_ms,
    "binding": {
      "ns": ns,
      "channel": channel,
      "target": target,
      "subject_kind": subject_kind,
      "subject_id": subject_id,
      "change_id": change_id,
      "process_spec_sha256": process_spec_sha256,
      "change_plan_sha256": change_plan_sha256,
      "change_plan_core_sha256": change_plan_core_sha256,
      "change_plan_meta_sha256": change_plan_meta_sha256,
      "change_view_sha256": change_view_sha256,
      "links": links,
    },
    "input": input_payload,
    "output": output_payload,
    "digests": {
      "input_sha256": input_sha,
      "output_sha256": output_sha,
      "request_id": request_id,
      "breakglass": breakglass
    }
  });

  if let Some(parent) = Path::new(path).parent() {
    if !parent.as_os_str().is_empty() {
      fs::create_dir_all(parent)
        .with_context(|| format!("create preflight report directory {}", parent.display()))?;
    }
  }
  fs::write(
    path,
    format!("{}\n", serde_json::to_string_pretty(&report)?),
  )
  .with_context(|| format!("write preflight report {}", path))?;
  Ok(())
}

fn maybe_emit_change_snapshot(
  args: &OpsArgs,
  op_name: &str,
  request_id: &str,
  ts_ms: i64,
  outputs: &Value,
) -> Result<()> {
  let Some(path) = args.emit_change_snapshot.as_deref() else {
    return Ok(());
  };
  if op_name != "change.get" {
    return Err(usage_error(
      "--emit-change-snapshot is only valid for change.get",
      json!({
        "reason": "CHANGE_SNAPSHOT_OP_MISMATCH",
        "op": op_name,
      }),
    ));
  }
  let snapshot = build_change_snapshot(args, request_id, ts_ms, outputs)?;
  if let Some(parent) = Path::new(path).parent() {
    if !parent.as_os_str().is_empty() {
      fs::create_dir_all(parent)
        .with_context(|| format!("create change snapshot directory {}", parent.display()))?;
    }
  }
  fs::write(
    path,
    format!("{}\n", serde_json::to_string_pretty(&snapshot)?),
  )
  .with_context(|| format!("write change snapshot {}", path))?;
  Ok(())
}

fn maybe_emit_safe_run_bundle(
  args: &OpsArgs,
  op_name: &str,
  request_id: &str,
  client: &SupervisorClient,
  preflight_report: Option<&Value>,
) -> Result<()> {
  let Some(path) = args.emit_bundle.as_deref() else {
    return Ok(());
  };
  if op_name != "change.run" {
    return Err(usage_error(
      "--emit-bundle is only valid for change.run",
      json!({
        "reason": "SAFE_RUN_BUNDLE_OP_MISMATCH",
        "op": op_name,
      }),
    ));
  }
  let Some(preflight_report) = preflight_report else {
    return Err(usage_error(
      "--emit-bundle requires --require-preflight or --require-bundle",
      json!({
        "reason": "SAFE_RUN_BUNDLE_REQUIRES_PREFLIGHT",
        "op": op_name,
      }),
    ));
  };

  let ns = payload_string_field(&args.payload, "ns").unwrap_or_else(|| "default".to_string());
  let change_id = args
    .payload
    .get("change_id")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .ok_or_else(|| {
      usage_error(
        "change.run payload must include change_id when --emit-bundle is used",
        json!({
          "reason": "SAFE_RUN_BUNDLE_BINDING_MISMATCH",
        }),
      )
    })?;
  let cap_refs = args.caps.iter().map(String::as_str).collect::<Vec<_>>();
  let change_get = client
    .call_with(
      "change.get",
      &cap_refs,
      json!({
        "ns": ns,
        "change_id": change_id,
      }),
    )
    .map_err(|err| {
      let error = err.to_string();
      anyhow::Error::new(OpsStructuredError {
        code: classify_change_get_error_code(error.as_str()),
        message: format!("change.get failed while emitting bundle: {err:#}"),
        details: json!({
          "reason": "CHANGE_GET_FAILED",
          "change_id": change_id,
          "change_get_error": error,
        }),
      })
    })?;
  let snapshot = build_change_snapshot(args, request_id, now_ms(), &change_get)?;
  let workdir = PathBuf::from(".pnix/ops_runs").join(format!("inv_{}", request_id));
  let bundle = build_safe_run_bundle(
    args,
    &args.payload,
    preflight_report,
    &snapshot,
    request_id,
    &workdir,
    args.preflight_max_age_ms,
  )?;
  write_json_atomic(Path::new(path), &bundle)
    .with_context(|| format!("write safe-run bundle {}", path))?;
  Ok(())
}

fn build_change_snapshot(
  args: &OpsArgs,
  request_id: &str,
  ts_ms: i64,
  outputs: &Value,
) -> Result<Value> {
  let change = outputs
    .get("change")
    .and_then(Value::as_object)
    .ok_or_else(|| {
      usage_error(
        "change.get response missing change object",
        json!({
          "reason": "CHANGE_SNAPSHOT_SCHEMA_MISMATCH",
        }),
      )
    })?;
  let change_id = change
    .get("change_id")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .ok_or_else(|| {
      usage_error(
        "change.get response missing change.change_id",
        json!({
          "reason": "CHANGE_SNAPSHOT_SCHEMA_MISMATCH",
        }),
      )
    })?;
  let ns = outputs
    .get("ns")
    .and_then(Value::as_str)
    .or_else(|| args.payload.get("ns").and_then(Value::as_str))
    .unwrap_or("default")
    .trim()
    .to_string();
  let change_view = json!({
    "ns": ns,
    "change": change,
  });
  let change_value = Value::Object(change.clone());
  let plan_material = change_plan_material(&change_value);
  let mutable_material = change_mutable_material(&change_value);
  let derived = derive_change_view(change);

  let actor = args
    .payload
    .get("actor")
    .cloned()
    .unwrap_or_else(|| json!({}));
  let source = json!({
    "supervisor_endpoint": args.endpoint,
    "supervisor_version": outputs
      .get("supervisor_version")
      .and_then(Value::as_str)
      .unwrap_or("unknown"),
    "client_version": env!("CARGO_PKG_VERSION"),
    "request_id": request_id,
    "actor": actor,
  });

  Ok(json!({
    "schema": "pnix-change-snapshot@0.1",
    "fetched_ms": ts_ms,
    "source": source,
    "binding": {
      "ns": ns,
      "change_id": change_id,
    },
    "change_view": change_view,
    "derived": derived,
    "digests": {
      "plan_sha256": canonical_sha256_value_hex(&plan_material)?,
      "plan_core_sha256": canonical_sha256_value_hex(&change_plan_core_material(&change_value))?,
      "plan_meta_sha256": canonical_sha256_value_hex(&change_plan_meta_material(&change_value))?,
      "mutable_sha256": canonical_sha256_value_hex(&mutable_material)?,
      "view_sha256": canonical_sha256_value_hex(&change_view)?,
    }
  }))
}

fn build_safe_run_bundle(
  args: &OpsArgs,
  payload: &Value,
  preflight_report: &Value,
  change_snapshot: &Value,
  invocation_id: &str,
  workdir: &Path,
  max_bundle_age_ms: u64,
) -> Result<Value> {
  let created_ms = now_ms();
  let expires_ms = created_ms.saturating_add(max_bundle_age_ms as i64);
  let bundle_id = format!("bundle_{}_{}", created_ms, std::process::id());

  let binding_ns = payload
    .get("ns")
    .and_then(Value::as_str)
    .unwrap_or("default");
  let binding_channel = payload
    .get("channel")
    .and_then(Value::as_str)
    .unwrap_or("prod");
  let binding_change_id = payload
    .get("change_id")
    .and_then(Value::as_str)
    .unwrap_or("chg_REPLACE_ME");
  let binding_subject_id = payload
    .get("subject")
    .and_then(Value::as_object)
    .and_then(|subject| subject.get("id"))
    .and_then(Value::as_str)
    .or_else(|| {
      preflight_report
        .get("binding")
        .and_then(Value::as_object)
        .and_then(|binding| binding.get("subject_id"))
        .and_then(Value::as_str)
    })
    .unwrap_or("subject_REPLACE_ME");
  let binding_target = preflight_report
    .get("binding")
    .and_then(Value::as_object)
    .and_then(|binding| binding.get("target"))
    .and_then(Value::as_str)
    .unwrap_or("exec.admit");
  let change_plan_core_sha256 = change_snapshot
    .get("digests")
    .and_then(Value::as_object)
    .and_then(|digests| digests.get("plan_core_sha256"))
    .and_then(Value::as_str)
    .unwrap_or("sha256:unknown")
    .to_string();

  let bundle = json!({
    "schema": "pnix-safe-run-bundle@0.1",
    "created_ms": created_ms,
    "bundle_id": bundle_id,
    "binding": {
      "ns": binding_ns,
      "channel": binding_channel,
      "change_id": binding_change_id,
      "subject_id": binding_subject_id,
      "target": binding_target,
    },
    "policy": {
      "max_bundle_age_ms": max_bundle_age_ms as i64,
      "expires_ms": expires_ms,
      "require_plan_unchanged": true,
      "require_confirmation": "TYPE_CHANGE_ID",
    },
    "embedded": {
      "preflight_report": preflight_report,
      "change_snapshot": change_snapshot,
    },
    "digests": {
      "preflight_report_sha256": sha256_value_hex(preflight_report)?,
      "change_snapshot_sha256": sha256_value_hex(change_snapshot)?,
      "change_plan_core_sha256": change_plan_core_sha256,
    },
    "provenance": {
      "invocation_id": invocation_id,
      "workdir": format!("file://{}", workdir.display()),
      "client_version": env!("CARGO_PKG_VERSION"),
      "git_commit": option_env!("GIT_COMMIT").unwrap_or("unknown"),
      "endpoint": args.endpoint,
    }
  });
  Ok(bundle)
}

fn derive_change_view(change: &serde_json::Map<String, Value>) -> Value {
  let mut step_kinds = Vec::new();
  let mut step_spec_sha256 = Vec::new();
  if let Some(steps) = change.get("steps").and_then(Value::as_array) {
    for step in sorted_steps(steps) {
      if let Some(kind) = step.get("kind").and_then(Value::as_str) {
        step_kinds.push(kind.to_string());
      }
      if let Some(spec) = step.get("spec") {
        if let Ok(digest) = canonical_sha256_value_hex(spec) {
          step_spec_sha256.push(digest);
        }
      }
    }
  }

  let approvals = change
    .get("approvals")
    .and_then(Value::as_array)
    .cloned()
    .unwrap_or_default();
  let approvers = approvals
    .iter()
    .filter_map(|entry| {
      entry
        .as_object()
        .and_then(|obj| obj.get("approver_token_id").or_else(|| obj.get("approver")))
        .and_then(Value::as_str)
        .map(ToString::to_string)
    })
    .collect::<Vec<_>>();
  let last_approval_ms = approvals
    .iter()
    .filter_map(|entry| entry.get("ts_ms").and_then(Value::as_i64))
    .max();

  json!({
    "step_kinds": step_kinds,
    "step_spec_sha256": step_spec_sha256,
    "approvals_summary": {
      "count": approvals.len(),
      "approvers": approvers,
      "last_approval_ms": last_approval_ms,
    }
  })
}

fn sorted_steps(steps: &[Value]) -> Vec<Value> {
  let mut indexed = steps
    .iter()
    .enumerate()
    .map(|(idx, step)| {
      let step_no = step
        .get("step_no")
        .and_then(Value::as_i64)
        .unwrap_or(idx as i64 + 1);
      (step_no, idx, step.clone())
    })
    .collect::<Vec<_>>();
  indexed.sort_by_key(|(step_no, idx, _)| (*step_no, *idx));
  indexed.into_iter().map(|(_, _, step)| step).collect()
}

fn build_change_plan_diff(
  binding: &serde_json::Map<String, Value>,
  change_value: &Value,
  kind: &str,
  ns: &str,
  change_id: &str,
) -> Value {
  let before_plan_sha = binding
    .get("change_plan_sha256")
    .and_then(Value::as_str)
    .filter(|v| !v.trim().is_empty())
    .map(str::to_string);
  let before_plan_core_sha = binding
    .get("change_plan_core_sha256")
    .and_then(Value::as_str)
    .filter(|v| !v.trim().is_empty())
    .map(str::to_string);
  let before_plan_meta_sha = binding
    .get("change_plan_meta_sha256")
    .and_then(Value::as_str)
    .filter(|v| !v.trim().is_empty())
    .map(str::to_string);

  let before_subject_id = binding
    .get("subject_id")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|v| !v.is_empty())
    .map(str::to_string);

  let before_step_kinds = binding
    .get("step_kinds")
    .and_then(Value::as_array)
    .map(|items| {
      items
        .iter()
        .filter_map(|item| item.as_str().map(str::to_string))
        .collect::<Vec<_>>()
    })
    .unwrap_or_default();
  let before_step_spec_sha256 = binding
    .get("step_spec_sha256")
    .and_then(Value::as_array)
    .map(|items| {
      items
        .iter()
        .filter_map(|item| item.as_str().map(str::to_string))
        .collect::<Vec<_>>()
    })
    .unwrap_or_default();
  let before_step_count = binding
    .get("step_count")
    .and_then(Value::as_u64)
    .map(|value| value as usize)
    .unwrap_or_else(|| before_step_kinds.len().max(before_step_spec_sha256.len()));

  let after_plan_sha = canonical_sha256_value_hex(&change_plan_material(change_value)).ok();
  let after_plan_core_sha =
    canonical_sha256_value_hex(&change_plan_core_material(change_value)).ok();
  let after_plan_meta_sha =
    canonical_sha256_value_hex(&change_plan_meta_material(change_value)).ok();

  let after_subject_id = change_value
    .get("subject_id")
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|v| !v.is_empty())
    .map(str::to_string)
    .or_else(|| before_subject_id.clone());

  let sorted_after_steps = change_value
    .get("steps")
    .and_then(Value::as_array)
    .map(|steps| sorted_steps(steps))
    .unwrap_or_default();
  let after_step_count = sorted_after_steps.len();
  let mut after_step_kinds = Vec::with_capacity(after_step_count);
  let mut after_step_spec_sha256 = Vec::with_capacity(after_step_count);
  for step in &sorted_after_steps {
    after_step_kinds.push(
      step
        .get("kind")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| "unknown".to_string()),
    );
    let spec_digest = step
      .get("spec")
      .and_then(|spec| canonical_sha256_value_hex(spec).ok())
      .unwrap_or_else(|| "sha256:unknown".to_string());
    after_step_spec_sha256.push(spec_digest);
  }

  let subject_changed = matches!(
    (before_subject_id.as_deref(), after_subject_id.as_deref()),
    (Some(before), Some(after)) if before != after
  );
  let step_count_delta = after_step_count as i64 - before_step_count as i64;
  let step_kinds_changed = !before_step_kinds.is_empty() && before_step_kinds != after_step_kinds;
  let step_specs_changed =
    !before_step_spec_sha256.is_empty() && before_step_spec_sha256 != after_step_spec_sha256;

  let max_len = before_step_count.max(after_step_count);
  let mut changes = Vec::new();
  for idx in 0..max_len {
    let before_kind = before_step_kinds.get(idx).cloned();
    let before_spec = before_step_spec_sha256.get(idx).cloned();
    let after_kind = after_step_kinds.get(idx).cloned();
    let after_spec = after_step_spec_sha256.get(idx).cloned();
    let change_type = if before_kind.is_none() && before_spec.is_none() {
      if after_kind.is_none() && after_spec.is_none() {
        None
      } else {
        Some("ADDED")
      }
    } else if after_kind.is_none() && after_spec.is_none() {
      Some("REMOVED")
    } else if before_kind == after_kind && before_spec == after_spec {
      None
    } else {
      Some("MODIFIED")
    };
    if let Some(change_type) = change_type {
      changes.push(json!({
        "index": idx,
        "before": if before_kind.is_none() && before_spec.is_none() {
          Value::Null
        } else {
          json!({
            "kind": before_kind,
            "spec_sha256": before_spec,
          })
        },
        "after": if after_kind.is_none() && after_spec.is_none() {
          Value::Null
        } else {
          json!({
            "kind": after_kind,
            "spec_sha256": after_spec,
          })
        },
        "type": change_type,
      }));
    }
  }

  let mut human_summary = Vec::new();
  if subject_changed {
    human_summary.push(format!(
      "subject_id changed: {} -> {}",
      before_subject_id.as_deref().unwrap_or("unknown"),
      after_subject_id.as_deref().unwrap_or("unknown")
    ));
  }
  if step_count_delta != 0 {
    human_summary.push(format!(
      "step_count: {} -> {} ({:+})",
      before_step_count, after_step_count, step_count_delta
    ));
  }
  if step_kinds_changed {
    human_summary.push("step_kinds changed".to_string());
  }
  if step_specs_changed {
    human_summary.push("step_spec_sha256 changed".to_string());
  }
  if human_summary.is_empty() {
    human_summary
      .push("digest mismatch detected; prior step-level material unavailable".to_string());
  }

  json!({
    "schema": "pnix-change-plan-diff@0.1",
    "computed_ms": now_ms(),
    "binding": {
      "ns": ns,
      "change_id": change_id,
    },
    "before": {
      "plan_sha256": before_plan_sha,
      "plan_core_sha256": before_plan_core_sha,
      "plan_meta_sha256": before_plan_meta_sha,
      "subject_id": before_subject_id,
      "step_count": before_step_count,
      "step_kinds": before_step_kinds,
      "step_spec_sha256": before_step_spec_sha256,
    },
    "after": {
      "plan_sha256": after_plan_sha,
      "plan_core_sha256": after_plan_core_sha,
      "plan_meta_sha256": after_plan_meta_sha,
      "subject_id": after_subject_id,
      "step_count": after_step_count,
      "step_kinds": after_step_kinds,
      "step_spec_sha256": after_step_spec_sha256,
    },
    "classification": {
      "kind": kind,
      "core_changed": kind == "PLAN_CORE_CHANGED",
      "meta_only_changed": kind == "PLAN_META_ONLY_CHANGED",
      "view_drift_suspected": kind == "PLAN_VIEW_OR_UNKNOWN_CHANGED",
    },
    "diff": {
      "subject_changed": subject_changed,
      "step_count_delta": step_count_delta,
      "step_kinds_changed": step_kinds_changed,
      "step_specs_changed": step_specs_changed,
      "changes": changes,
    },
    "human_summary": human_summary,
  })
}

fn change_plan_material(change: &Value) -> Value {
  let steps = change
    .get("steps")
    .and_then(Value::as_array)
    .map(|items| {
      sorted_steps(items)
        .into_iter()
        .map(|step| {
          json!({
            "step_no": step.get("step_no").cloned().unwrap_or(Value::Null),
            "kind": step.get("kind").cloned().unwrap_or(Value::Null),
            "spec": step.get("spec").cloned().unwrap_or(Value::Null),
          })
        })
        .collect::<Vec<_>>()
    })
    .unwrap_or_default();
  json!({
    "change_id": change.get("change_id").cloned().unwrap_or(Value::Null),
    "source": change.get("source").cloned().unwrap_or(Value::Null),
    "risk": change.get("risk").cloned().unwrap_or(Value::Null),
    "title": change.get("title").cloned().unwrap_or(Value::Null),
    "reason": change.get("reason").cloned().unwrap_or(Value::Null),
    "scheduled_start_ms": change.get("scheduled_start_ms").cloned().unwrap_or(Value::Null),
    "scheduled_end_ms": change.get("scheduled_end_ms").cloned().unwrap_or(Value::Null),
    "steps": steps,
  })
}

fn change_plan_core_material(change: &Value) -> Value {
  let steps = change
    .get("steps")
    .and_then(Value::as_array)
    .map(|items| {
      sorted_steps(items)
        .into_iter()
        .map(|step| {
          json!({
            "step_no": step.get("step_no").cloned().unwrap_or(Value::Null),
            "kind": step.get("kind").cloned().unwrap_or(Value::Null),
            "spec": step.get("spec").cloned().unwrap_or(Value::Null),
          })
        })
        .collect::<Vec<_>>()
    })
    .unwrap_or_default();
  json!({
    "change_id": change.get("change_id").cloned().unwrap_or(Value::Null),
    "subject_id": change.get("subject_id").cloned().unwrap_or(Value::Null),
    "target": change.get("target").cloned().unwrap_or(Value::Null),
    "steps": steps,
  })
}

fn change_plan_meta_material(change: &Value) -> Value {
  json!({
    "source": change.get("source").cloned().unwrap_or(Value::Null),
    "risk": change.get("risk").cloned().unwrap_or(Value::Null),
    "title": change.get("title").cloned().unwrap_or(Value::Null),
    "reason": change.get("reason").cloned().unwrap_or(Value::Null),
    "scheduled_start_ms": change.get("scheduled_start_ms").cloned().unwrap_or(Value::Null),
    "scheduled_end_ms": change.get("scheduled_end_ms").cloned().unwrap_or(Value::Null),
  })
}

fn change_mutable_material(change: &Value) -> Value {
  let steps = change
    .get("steps")
    .and_then(Value::as_array)
    .map(|items| {
      sorted_steps(items)
        .into_iter()
        .map(|step| {
          json!({
            "step_no": step.get("step_no").cloned().unwrap_or(Value::Null),
            "status": step.get("status").cloned().unwrap_or(Value::Null),
            "started_ms": step.get("started_ms").cloned().unwrap_or(Value::Null),
            "finished_ms": step.get("finished_ms").cloned().unwrap_or(Value::Null),
            "last_error": step.get("last_error").cloned().unwrap_or(Value::Null),
          })
        })
        .collect::<Vec<_>>()
    })
    .unwrap_or_default();
  json!({
    "status": change.get("status").cloned().unwrap_or(Value::Null),
    "updated_ms": change.get("updated_ms").cloned().unwrap_or(Value::Null),
    "approvals": change.get("approvals").cloned().unwrap_or_else(|| json!([])),
    "last_error": change.get("last_error").cloned().unwrap_or(Value::Null),
    "steps": steps,
  })
}

fn build_preflight_input_payload(
  payload: &Value,
  issued_ms: i64,
  ns: &str,
  channel: &str,
  target: &str,
  links: &Value,
) -> Value {
  let mut subject = payload
    .get("subject")
    .cloned()
    .unwrap_or_else(|| json!({ "kind": "process", "id": "" }));
  if let Some(process_spec) = payload.get("process_spec") {
    if let Some(subject_obj) = subject.as_object_mut() {
      subject_obj.insert("process_spec".to_string(), process_spec.clone());
    }
  }
  json!({
    "schema": "pnix-admission-input@0.1",
    "request": {
      "target": target,
      "ns": ns,
      "channel": channel,
      "ts_ms": issued_ms
    },
    "actor": payload.get("actor").cloned().unwrap_or_else(|| json!({})),
    "subject": subject,
    "desired": payload.get("desired").cloned().unwrap_or_else(|| json!({})),
    "system": payload.get("system").cloned().unwrap_or_else(|| json!({ "channel": channel })),
    "evidence": payload.get("evidence").cloned().unwrap_or_else(|| json!({})),
    "links": links,
    "breakglass": payload.get("breakglass").and_then(Value::as_bool)
      .or_else(|| payload.get("actor").and_then(Value::as_object).and_then(|actor| actor.get("breakglass")).and_then(Value::as_bool))
      .unwrap_or(false)
  })
}

fn build_preflight_output_payload(
  outputs: &Value,
  target: &str,
  subject_kind: &str,
  subject_id: &str,
  channel: &str,
) -> Value {
  let decision = outputs
    .get("decision")
    .and_then(Value::as_str)
    .map(str::trim)
    .unwrap_or(
      if outputs
        .get("allow")
        .and_then(Value::as_bool)
        .unwrap_or(false)
      {
        "allow"
      } else {
        "deny"
      },
    );
  let allow = outputs
    .get("allow")
    .and_then(Value::as_bool)
    .unwrap_or(decision.eq_ignore_ascii_case("allow"));
  json!({
    "schema": "pnix-admission-decision@0.1",
    "allow": allow,
    "decision": decision,
    "reasons": outputs.get("reasons").cloned().unwrap_or_else(|| json!([])),
    "warnings": outputs.get("warnings").cloned().unwrap_or_else(|| json!([])),
    "target": outputs.get("target").cloned().unwrap_or_else(|| json!(target)),
    "subject_kind": outputs.get("subject_kind").cloned().unwrap_or_else(|| json!(subject_kind)),
    "subject_id": outputs.get("subject_id").cloned().unwrap_or_else(|| json!(subject_id)),
    "policy": outputs.get("policy").cloned().unwrap_or_else(|| json!({"channel": channel})),
  })
}

fn normalize_preflight_links(
  payload: &Value,
  request_id: &str,
  op_name: &str,
  endpoint: &str,
) -> Value {
  let mut links_obj = serde_json::Map::new();
  if let Some(existing) = payload.get("links") {
    if let Some(obj) = existing.as_object() {
      for (key, value) in obj {
        links_obj.insert(key.to_string(), value.clone());
      }
    }
  }
  if let Some(change_id) = payload.get("change_id").and_then(Value::as_str) {
    links_obj.insert("change_id".to_string(), json!(change_id));
  }
  if let Some(change_plan_sha256) = payload.get("change_plan_sha256").and_then(Value::as_str) {
    links_obj.insert("change_plan_sha256".to_string(), json!(change_plan_sha256));
  }
  if let Some(change_plan_core_sha256) = payload
    .get("change_plan_core_sha256")
    .and_then(Value::as_str)
  {
    links_obj.insert(
      "change_plan_core_sha256".to_string(),
      json!(change_plan_core_sha256),
    );
  }
  if let Some(change_plan_meta_sha256) = payload
    .get("change_plan_meta_sha256")
    .and_then(Value::as_str)
  {
    links_obj.insert(
      "change_plan_meta_sha256".to_string(),
      json!(change_plan_meta_sha256),
    );
  }
  if let Some(change_view_sha256) = payload.get("change_view_sha256").and_then(Value::as_str) {
    links_obj.insert("change_view_sha256".to_string(), json!(change_view_sha256));
  }
  if !links_obj.contains_key("invocation_id") {
    links_obj.insert("invocation_id".to_string(), json!(request_id));
  }
  if !links_obj.contains_key("command") {
    links_obj.insert(
      "command".to_string(),
      json!(format!("pnix ops --op {}", op_name)),
    );
  }
  if !links_obj.contains_key("host") {
    let host = std::env::var("HOSTNAME")
      .ok()
      .or_else(|| std::env::var("COMPUTERNAME").ok())
      .filter(|value| !value.trim().is_empty())
      .unwrap_or_else(|| "unknown".to_string());
    links_obj.insert("host".to_string(), json!(host));
  }
  if !links_obj.contains_key("cli_version") {
    links_obj.insert("cli_version".to_string(), json!(env!("CARGO_PKG_VERSION")));
  }
  if !links_obj.contains_key("git_commit") {
    links_obj.insert(
      "git_commit".to_string(),
      json!(option_env!("GIT_COMMIT").unwrap_or("unknown")),
    );
  }
  if !links_obj.contains_key("repo_dirty") {
    if let Some(raw) = option_env!("GIT_DIRTY") {
      let dirty = matches!(raw, "1" | "true" | "TRUE" | "yes" | "YES");
      links_obj.insert("repo_dirty".to_string(), json!(dirty));
    }
  }
  if !links_obj.contains_key("supervisor_endpoint") {
    links_obj.insert("supervisor_endpoint".to_string(), json!(endpoint));
  }
  Value::Object(links_obj)
}

fn canonical_sha256_value_hex(value: &Value) -> Result<String> {
  let canonical = canonicalize_value(value);
  sha256_value_hex(&canonical)
}

fn canonicalize_value(value: &Value) -> Value {
  match value {
    Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => value.clone(),
    Value::Array(items) => Value::Array(items.iter().map(canonicalize_value).collect::<Vec<_>>()),
    Value::Object(map) => {
      let mut keys = map.keys().cloned().collect::<Vec<_>>();
      keys.sort_unstable();
      let mut normalized = serde_json::Map::new();
      for key in keys {
        if let Some(child) = map.get(key.as_str()) {
          normalized.insert(key, canonicalize_value(child));
        }
      }
      Value::Object(normalized)
    }
  }
}

fn sha256_value_hex(value: &Value) -> Result<String> {
  let bytes = serde_json::to_vec(value).context("serialize preflight report hash input")?;
  let mut hasher = Sha256::new();
  hasher.update(bytes);
  let digest = hasher.finalize();
  let mut out = String::with_capacity(digest.len() * 2 + 7);
  out.push_str("sha256:");
  for byte in digest {
    out.push_str(format!("{:02x}", byte).as_str());
  }
  Ok(out)
}

fn usage_error(message: impl Into<String>, details: Value) -> anyhow::Error {
  anyhow::Error::new(OpsStructuredError {
    code: OpsErrorCode::Usage,
    message: message.into(),
    details,
  })
}

fn denied_error(message: impl Into<String>, details: Value) -> anyhow::Error {
  anyhow::Error::new(OpsStructuredError {
    code: OpsErrorCode::Denied,
    message: message.into(),
    details,
  })
}

fn expected_confirmation_token(op_name: &str, payload: &Value) -> Option<String> {
  match op_name {
    "change.run" => Some(format!(
      "run:{}",
      payload
        .get("change_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
    )),
    "remote.execute" => Some(format!(
      "remote:{}",
      payload
        .get("change_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
    )),
    "change.approve" => Some(format!(
      "approve:{}",
      payload
        .get("change_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
    )),
    "breakglass.request" => Some("BREAKGLASS".to_string()),
    "compliance.policy.set" => Some("COMPLIANCE_SET".to_string()),
    _ => None,
  }
}

fn enforce_confirmation(args: &OpsArgs, op_name: &str, risk: RiskLevel) -> Result<()> {
  if !args.execute || risk == RiskLevel::Low {
    return Ok(());
  }
  if args.yes {
    return Ok(());
  }
  let expected =
    expected_confirmation_token(op_name, &args.payload).unwrap_or_else(|| "YES".to_string());
  if args.no_confirm {
    if risk == RiskLevel::High {
      return Err(usage_error(
        "confirmation required for high-risk op",
        json!({
          "reason": "CONFIRMATION_REQUIRED",
          "op": op_name,
          "expected_token": expected,
        }),
      ));
    }
    return Ok(());
  }

  if let Some(token) = args.confirm.as_deref() {
    if token == expected {
      return Ok(());
    }
    return Err(usage_error(
      format!("invalid confirmation token: expected '{}'", expected),
      json!({
        "reason": "CONFIRMATION_REQUIRED",
        "op": op_name,
        "expected_token": expected,
        "provided_token": token,
      }),
    ));
  }

  let interactive = io::stdin().is_terminal() && io::stderr().is_terminal();
  if !interactive {
    return Err(usage_error(
      format!(
        "confirmation required for {} in non-interactive mode; use --yes or --confirm '{}'",
        op_name, expected
      ),
      json!({
        "reason": "CONFIRMATION_REQUIRED",
        "op": op_name,
        "expected_token": expected,
      }),
    ));
  }

  eprintln!(
    "confirm {} ns={} channel={} token='{}'",
    op_name,
    payload_string_field(&args.payload, "ns").unwrap_or_else(|| "default".to_string()),
    payload_string_field(&args.payload, "channel").unwrap_or_else(|| "prod".to_string()),
    expected
  );
  eprint!("Type confirmation token to continue: ");
  io::stderr().flush().context("flush confirmation prompt")?;
  let mut line = String::new();
  io::stdin()
    .read_line(&mut line)
    .context("read confirmation token")?;
  if line.trim() != expected {
    return Err(usage_error(
      "confirmation required: token mismatch",
      json!({
        "reason": "CONFIRMATION_REQUIRED",
        "op": op_name,
        "expected_token": expected,
        "provided_token": line.trim(),
      }),
    ));
  }
  Ok(())
}

fn now_ms() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|dur| dur.as_millis() as i64)
    .unwrap_or(0)
}

fn payload_string_field(payload: &Value, key: &str) -> Option<String> {
  payload
    .as_object()
    .and_then(|obj| obj.get(key))
    .and_then(Value::as_str)
    .map(ToString::to_string)
}

fn detect_collection_len(value: &Value) -> usize {
  if let Some(arr) = value.as_array() {
    return arr.len();
  }
  if let Some(obj) = value.as_object() {
    for key in ["changes", "sessions", "items", "rows", "entries", "data"] {
      if let Some(arr) = obj.get(key).and_then(Value::as_array) {
        return arr.len();
      }
    }
  }
  0
}

fn response_ok_envelope(
  op: &str,
  ns: Option<&str>,
  channel: Option<&str>,
  request_id: &str,
  ts_ms: i64,
  result: Value,
) -> Value {
  json!({
    "schema": "pnix-ops-response@0.1",
    "ok": true,
    "op": op,
    "ns": ns.unwrap_or("default"),
    "channel": channel.unwrap_or("prod"),
    "request_id": request_id,
    "ts_ms": ts_ms,
    "result": result,
    "error": Value::Null,
  })
}

fn response_error_envelope(
  op: &str,
  ns: Option<&str>,
  channel: Option<&str>,
  request_id: Option<&str>,
  ts_ms: i64,
  code: OpsErrorCode,
  message: String,
  details: Value,
) -> Value {
  json!({
    "schema": "pnix-ops-response@0.1",
    "ok": false,
    "op": op,
    "ns": ns.unwrap_or("default"),
    "channel": channel.unwrap_or("prod"),
    "request_id": request_id.unwrap_or(""),
    "ts_ms": ts_ms,
    "result": json!({}),
    "error": {
      "schema": "pnix-ops-error@0.1",
      "code": code.as_str(),
      "message": message,
      "details": details,
    }
  })
}

fn classify_ops_error(err: &anyhow::Error) -> (OpsExitCode, OpsErrorCode, Value) {
  if let Some(structured) = err.downcast_ref::<OpsStructuredError>() {
    let exit = match structured.code {
      OpsErrorCode::Usage => OpsExitCode::Usage,
      OpsErrorCode::Denied => OpsExitCode::Denied,
      OpsErrorCode::NotFound => OpsExitCode::NotFound,
      OpsErrorCode::Conflict => OpsExitCode::Conflict,
      OpsErrorCode::Unauthorized => OpsExitCode::Unauthorized,
      OpsErrorCode::Timeout => OpsExitCode::Timeout,
      OpsErrorCode::Transport => OpsExitCode::Transport,
      OpsErrorCode::Internal => OpsExitCode::Internal,
      OpsErrorCode::Partial => OpsExitCode::Partial,
    };
    return (exit, structured.code, structured.details.clone());
  }

  let msg = err.to_string().to_ascii_lowercase();
  if msg.contains("parse --payload json")
    || msg.contains("requires a value")
    || msg.contains("unknown ops flag")
    || msg.contains("unexpected ops argument")
    || msg.contains("supports --mode ops")
    || msg.contains("cannot be used together")
    || msg.contains("must be")
    || msg.contains("is required")
    || msg.contains("confirmation required")
    || msg.contains("invalid confirmation token")
  {
    return (OpsExitCode::Usage, OpsErrorCode::Usage, json!({}));
  }
  if msg.contains("timeout") || msg.contains("timed out") {
    return (OpsExitCode::Timeout, OpsErrorCode::Timeout, json!({}));
  }
  if msg.contains("connect supervisor endpoint")
    || msg.contains("connection refused")
    || msg.contains("no such file")
    || msg.contains("transport")
    || msg.contains("tls")
  {
    return (OpsExitCode::Transport, OpsErrorCode::Transport, json!({}));
  }
  if msg.contains("unauthorized")
    || msg.contains("permission denied")
    || (msg.contains("token") && msg.contains("invalid"))
    || msg.contains("forbidden")
  {
    return (
      OpsExitCode::Unauthorized,
      OpsErrorCode::Unauthorized,
      json!({}),
    );
  }
  if msg.contains("denied") || msg.contains("approval") {
    return (OpsExitCode::Denied, OpsErrorCode::Denied, json!({}));
  }
  if msg.contains("not found") {
    return (OpsExitCode::NotFound, OpsErrorCode::NotFound, json!({}));
  }
  if msg.contains("conflict") || msg.contains("already") {
    return (OpsExitCode::Conflict, OpsErrorCode::Conflict, json!({}));
  }
  if msg.contains("partial") {
    return (OpsExitCode::Partial, OpsErrorCode::Partial, json!({}));
  }
  (OpsExitCode::Internal, OpsErrorCode::Internal, json!({}))
}

fn enrich_error_details_with_hint(
  op: &str,
  payload: Option<&Value>,
  code: OpsErrorCode,
  details: Value,
) -> Value {
  let hint = build_ops_hint(op, payload, code, &details);
  match (details, hint) {
    (Value::Object(mut obj), Some(hint)) => {
      obj.insert("hint".to_string(), hint);
      Value::Object(obj)
    }
    (Value::Object(obj), None) => Value::Object(obj),
    (other, Some(hint)) => json!({
      "raw_details": other,
      "hint": hint,
    }),
    (other, None) => other,
  }
}

fn build_ops_hint(
  op: &str,
  payload: Option<&Value>,
  code: OpsErrorCode,
  details: &Value,
) -> Option<Value> {
  let details_obj = details.as_object();
  let reason = details_obj
    .and_then(|obj| obj.get("reason"))
    .and_then(Value::as_str);
  let plan_change_class = details_obj
    .and_then(|obj| obj.get("plan_change_class"))
    .and_then(Value::as_str);

  let ns = payload
    .and_then(|v| payload_string_field(v, "ns"))
    .unwrap_or_else(|| "default".to_string());
  let change_id = details_obj
    .and_then(|obj| obj.get("change_id"))
    .and_then(Value::as_str)
    .map(ToString::to_string)
    .or_else(|| payload.and_then(|v| payload_string_field(v, "change_id")));
  let subject_id = payload
    .and_then(|v| v.get("subject"))
    .and_then(Value::as_object)
    .and_then(|subject| subject.get("id"))
    .and_then(Value::as_str)
    .map(ToString::to_string);
  let preflight_path = details_obj
    .and_then(|obj| obj.get("preflight_path"))
    .and_then(Value::as_str)
    .map(ToString::to_string);
  let workdir = details_obj
    .and_then(|obj| obj.get("workdir"))
    .cloned()
    .unwrap_or(Value::Null);

  let (category, severity, summary, mut recommended_actions, scenario) = match reason {
    Some("PREFLIGHT_DENIED") => (
      "PREFLIGHT",
      "HIGH",
      "Preflight denied. Fix prerequisites or request approved breakglass.",
      vec![
        json!({"id":"CHECK_DENY_REASONS","text":"Inspect admission reasons and missing prerequisites."}),
        json!({"id":"REFRESH_CHANGE_CONTEXT","text":"Review the latest change context before retry."}),
      ],
      HintScenario::PreflightDenied,
    ),
    Some("PREFLIGHT_EXPIRED") => (
      "PREFLIGHT",
      "MEDIUM",
      "Preflight report expired. Re-run preflight and retry change.run.",
      vec![
        json!({"id":"RERUN_PREFLIGHT","text":"Generate a new preflight report in the current execution window."}),
      ],
      HintScenario::PreflightExpired,
    ),
    Some("PREFLIGHT_BINDING_MISMATCH") => match plan_change_class {
      Some("PLAN_CORE_CHANGED") => (
        "TOCTOU",
        "HIGH",
        "Change core plan drifted after preflight. Re-approve updated plan before running.",
        vec![
          json!({"id":"REVIEW_PLAN_DIFF","text":"Inspect plan delta and changed execution steps."}),
          json!({"id":"REAPPROVE_CHANGE","text":"Re-approve the updated change if drift is intentional."}),
        ],
        HintScenario::BindingPlanCore,
      ),
      Some("PLAN_META_ONLY_CHANGED") => (
        "TOCTOU",
        "MEDIUM",
        "Change metadata drift detected. Refresh preflight and retry.",
        vec![
          json!({"id":"REFRESH_PREFLIGHT","text":"Run one fresh preflight for the current metadata."}),
        ],
        HintScenario::BindingPlanMeta,
      ),
      _ => (
        "BINDING",
        "HIGH",
        "Preflight binding mismatch. Align ns/channel/subject/change_id and retry.",
        vec![
          json!({"id":"ALIGN_BINDING","text":"Ensure run payload matches preflight binding fields."}),
        ],
        HintScenario::BindingGeneric,
      ),
    },
    Some("CHANGE_GET_FAILED") => (
      "TRANSPORT",
      "MEDIUM",
      "Unable to fetch current change state from supervisor. Check endpoint and retry.",
      vec![json!({"id":"CHECK_SUPERVISOR","text":"Verify supervisor reachability before retry."})],
      HintScenario::ChangeGetFailed,
    ),
    Some("CONFIRMATION_REQUIRED") => (
      "CONFIRMATION",
      "LOW",
      "Execution requires explicit confirmation for this risk level.",
      vec![
        json!({"id":"RETRY_WITH_CONFIRMATION","text":"Retry with --yes or explicit --confirm token."}),
      ],
      HintScenario::ConfirmationRequired,
    ),
    _ => {
      if matches!(code, OpsErrorCode::Unauthorized) {
        (
          "AUTHZ",
          "HIGH",
          "Authorization/approval context is missing. Human approval or token refresh is required.",
          vec![
            json!({"id":"REFRESH_AUTHZ_CONTEXT","text":"Refresh token/role approval before retry."}),
          ],
          HintScenario::AuthzDenied,
        )
      } else if matches!(code, OpsErrorCode::Transport | OpsErrorCode::Timeout) {
        (
          "TRANSPORT",
          "MEDIUM",
          "Transport/timeout error. Check supervisor endpoint and retry.",
          vec![
            json!({"id":"PING_SUPERVISOR","text":"Run ping and retry once transport is healthy."}),
          ],
          HintScenario::TransportFallback,
        )
      } else {
        return None;
      }
    }
  };
  let can_repreflight = preflight_path.is_some();
  let auto_retry = decide_auto_retry(reason, category, severity, code, can_repreflight);
  recommended_actions.insert(
    0,
    json!({
      "id": auto_retry.action_id,
      "text": auto_retry.action_text,
    }),
  );

  let mut artifacts = serde_json::Map::new();
  if let Some(path) = preflight_path.as_ref() {
    artifacts.insert("preflight_report".to_string(), json!(path));
  }
  if let Some(plan_diff) = details_obj.and_then(|obj| obj.get("plan_diff")).cloned() {
    artifacts.insert("plan_diff".to_string(), plan_diff);
  }
  if let Some(snapshot_before) = details_obj
    .and_then(|obj| obj.get("snapshot_before"))
    .and_then(Value::as_str)
    .map(ToString::to_string)
  {
    artifacts.insert("snapshot_before".to_string(), json!(snapshot_before));
  }
  if let Some(snapshot_after) = details_obj
    .and_then(|obj| obj.get("snapshot_after"))
    .and_then(Value::as_str)
    .map(ToString::to_string)
  {
    artifacts.insert("snapshot_after".to_string(), json!(snapshot_after));
  }
  if let Some(change_id) = change_id.as_ref() {
    artifacts.insert("change_id".to_string(), json!(change_id));
  }
  if let Some(subject_id) = subject_id.as_ref() {
    artifacts.insert("subject_id".to_string(), json!(subject_id));
  }

  let suggested_commands = build_hint_suggested_commands(
    scenario,
    op,
    ns.as_str(),
    change_id.as_deref(),
    subject_id.as_deref(),
    preflight_path.as_deref(),
    details_obj,
  );

  let mut hint = json!({
    "schema": "pnix-ops-hint@0.1",
    "summary": summary,
    "category": category,
    "severity": severity,
    "retry_class": auto_retry.retry_class.as_str(),
    "recommended_actions": recommended_actions,
    "suggested_commands": suggested_commands,
    "workdir": workdir,
    "artifacts": Value::Object(artifacts),
  });
  if let Some(retry_policy) = auto_retry.retry_policy {
    if let Some(hint_obj) = hint.as_object_mut() {
      hint_obj.insert("retry_policy".to_string(), retry_policy);
    }
  }

  Some(hint)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HintScenario {
  PreflightDenied,
  PreflightExpired,
  BindingPlanCore,
  BindingPlanMeta,
  BindingGeneric,
  ChangeGetFailed,
  ConfirmationRequired,
  AuthzDenied,
  TransportFallback,
}

struct AutoRetryDecision {
  action_id: &'static str,
  action_text: &'static str,
  retry_class: RetryClass,
  retry_policy: Option<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RetryClass {
  Retryable,
  NonRetryable,
  RequiresHuman,
}

impl RetryClass {
  fn as_str(self) -> &'static str {
    match self {
      RetryClass::Retryable => "retryable",
      RetryClass::NonRetryable => "non-retryable",
      RetryClass::RequiresHuman => "requires-human",
    }
  }
}

fn decide_auto_retry(
  reason: Option<&str>,
  category: &str,
  severity: &str,
  code: OpsErrorCode,
  can_repreflight: bool,
) -> AutoRetryDecision {
  let reason_upper = reason.unwrap_or("").to_ascii_uppercase();

  if matches!(
    reason_upper.as_str(),
    "CHANGE_PLAN_CHANGED" | "CHANGE_PLAN_CORE_CHANGED"
  ) || (category == "TOCTOU" && severity == "HIGH")
  {
    return AutoRetryDecision {
      action_id: "AUTO_RETRY_NON_RETRYABLE",
      action_text: "Automatic retry is forbidden for TOCTOU/plan-drift failures.",
      retry_class: RetryClass::NonRetryable,
      retry_policy: None,
    };
  }
  if matches!(
    reason_upper.as_str(),
    "PREFLIGHT_BINDING_MISMATCH" | "SNAPSHOT_BINDING_MISMATCH"
  ) || category == "BINDING"
  {
    return AutoRetryDecision {
      action_id: "AUTO_RETRY_NON_RETRYABLE",
      action_text:
        "Automatic retry is forbidden for binding mismatch; manual alignment is required.",
      retry_class: RetryClass::NonRetryable,
      retry_policy: None,
    };
  }
  if reason_upper.contains("BREAKGLASS") {
    return AutoRetryDecision {
      action_id: "AUTO_RETRY_REQUIRES_HUMAN",
      action_text:
        "Automatic retry is forbidden for break-glass gated failures; human approval is required.",
      retry_class: RetryClass::RequiresHuman,
      retry_policy: None,
    };
  }
  if matches!(code, OpsErrorCode::Unauthorized)
    || matches!(
      reason_upper.as_str(),
      "INSUFFICIENT_APPROVALS" | "AUTHZ_DENIED"
    )
    || category == "AUTHZ"
  {
    return AutoRetryDecision {
      action_id: "AUTO_RETRY_REQUIRES_HUMAN",
      action_text: "Automatic retry is forbidden for authorization/approval failures.",
      retry_class: RetryClass::RequiresHuman,
      retry_policy: None,
    };
  }
  if matches!(code, OpsErrorCode::Usage) || reason_upper == "CONFIRMATION_REQUIRED" {
    return AutoRetryDecision {
      action_id: "AUTO_RETRY_REQUIRES_HUMAN",
      action_text: "Automatic retry is forbidden for usage/confirmation failures.",
      retry_class: RetryClass::RequiresHuman,
      retry_policy: None,
    };
  }
  if reason_upper == "PREFLIGHT_EXPIRED" && can_repreflight {
    return AutoRetryDecision {
      action_id: "AUTO_RETRY_RETRYABLE",
      action_text: "Automatic retry is allowed: refresh preflight and retry within policy limits.",
      retry_class: RetryClass::Retryable,
      retry_policy: Some(default_retry_policy()),
    };
  }
  if matches!(code, OpsErrorCode::Transport | OpsErrorCode::Timeout) || category == "TRANSPORT" {
    return AutoRetryDecision {
      action_id: "AUTO_RETRY_RETRYABLE",
      action_text:
        "Automatic retry is allowed for transport/timeout failures within bounded backoff.",
      retry_class: RetryClass::Retryable,
      retry_policy: Some(default_retry_policy()),
    };
  }
  if reason_upper == "VIEW_DRIFT_SUSPECTED" {
    return AutoRetryDecision {
      action_id: "AUTO_RETRY_NON_RETRYABLE",
      action_text:
        "Automatic retry is forbidden for view drift; explicit operator override is required.",
      retry_class: RetryClass::NonRetryable,
      retry_policy: None,
    };
  }

  AutoRetryDecision {
    action_id: "AUTO_RETRY_NON_RETRYABLE",
    action_text: "Automatic retry is forbidden by default for unknown failure classes.",
    retry_class: RetryClass::NonRetryable,
    retry_policy: None,
  }
}

fn default_retry_policy() -> Value {
  json!({
    "max_attempts": 3,
    "backoff_ms": [500, 1500, 4000],
    "jitter_ms": 200,
    "retryable_categories": ["TRANSPORT", "TIMEOUT", "PREFLIGHT_EXPIRED"],
  })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HintCommandSlot {
  Inspect,
  Query,
  Retry,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HintCommandRisk {
  Safe,
  High,
}

struct HintCommandCandidate {
  slot: HintCommandSlot,
  priority: u8,
  risk: HintCommandRisk,
  command: String,
}

fn build_hint_suggested_commands(
  scenario: HintScenario,
  op: &str,
  ns: &str,
  change_id: Option<&str>,
  subject_id: Option<&str>,
  preflight_path: Option<&str>,
  details_obj: Option<&serde_json::Map<String, Value>>,
) -> Vec<String> {
  let mut candidates = Vec::<HintCommandCandidate>::new();
  if let Some(inspect) = build_inspect_command(scenario, preflight_path, details_obj) {
    candidates.push(HintCommandCandidate {
      slot: HintCommandSlot::Inspect,
      priority: 0,
      risk: HintCommandRisk::Safe,
      command: inspect,
    });
  }
  if let Some(query) = build_query_command(scenario, op, ns, change_id) {
    candidates.push(HintCommandCandidate {
      slot: HintCommandSlot::Query,
      priority: 0,
      risk: HintCommandRisk::Safe,
      command: query,
    });
  }
  if let Some(retry) = build_retry_command(scenario, op, ns, change_id, subject_id, preflight_path)
  {
    let risk = if scenario == HintScenario::ConfirmationRequired {
      HintCommandRisk::High
    } else {
      HintCommandRisk::Safe
    };
    candidates.push(HintCommandCandidate {
      slot: HintCommandSlot::Retry,
      priority: 0,
      risk,
      command: retry,
    });
  }
  select_hint_commands(candidates, scenario == HintScenario::ConfirmationRequired)
}

fn build_inspect_command(
  scenario: HintScenario,
  preflight_path: Option<&str>,
  details_obj: Option<&serde_json::Map<String, Value>>,
) -> Option<String> {
  let workdir = details_obj
    .and_then(|obj| obj.get("workdir"))
    .and_then(Value::as_str)
    .map(shell_path_from_ref);
  let from_workdir = |file: &str| {
    workdir.as_ref().map(|dir| {
      let path = Path::new(dir).join(file).to_string_lossy().to_string();
      format!("cat {}", shell_quote(path.as_str()))
    })
  };

  match scenario {
    HintScenario::BindingPlanCore | HintScenario::BindingPlanMeta => {
      from_workdir("05_plan_diff.json")
        .or_else(|| from_workdir("05_binding_check.json"))
        .or_else(|| Some("cat .pnix/ops_runs/<invocation>/05_plan_diff.json".to_string()))
    }
    HintScenario::BindingGeneric => from_workdir("05_binding_check.json")
      .or_else(|| from_workdir("09_summary.json"))
      .or_else(|| Some("cat .pnix/ops_runs/<invocation>/05_binding_check.json".to_string())),
    HintScenario::PreflightDenied | HintScenario::PreflightExpired => preflight_path
      .map(shell_path_from_ref)
      .map(|path| format!("cat {}", shell_quote(path.as_str())))
      .or_else(|| from_workdir("04_preflight_report.json")),
    HintScenario::ConfirmationRequired => from_workdir("09_summary.json"),
    HintScenario::AuthzDenied => from_workdir("09_summary.json"),
    HintScenario::ChangeGetFailed | HintScenario::TransportFallback => {
      from_workdir("09_summary.json")
    }
  }
}

fn build_query_command(
  scenario: HintScenario,
  op: &str,
  ns: &str,
  change_id: Option<&str>,
) -> Option<String> {
  let default_change = change_id.unwrap_or("chg_REPLACE_ME");
  let change_get_payload = json!({
    "ns": ns,
    "change_id": default_change,
  });
  let compliance_get_payload = json!({
    "ns": ns,
    "channel": "prod",
  });
  let breakglass_ls_payload = json!({
    "ns": ns,
    "status": "active",
    "limit": 200,
  });
  match scenario {
    HintScenario::ChangeGetFailed | HintScenario::TransportFallback => {
      Some("pnix ops --op ping --execute".to_string())
    }
    HintScenario::AuthzDenied => Some(format!(
      "pnix ops --op change.get --payload {} --execute",
      shell_quote(change_get_payload.to_string().as_str())
    )),
    _ if op.starts_with("compliance.") => Some(format!(
      "pnix ops --op compliance.get --payload {} --execute",
      shell_quote(compliance_get_payload.to_string().as_str())
    )),
    _ if op.starts_with("breakglass.") => Some(format!(
      "pnix ops --op bg.ls --payload {} --execute",
      shell_quote(breakglass_ls_payload.to_string().as_str())
    )),
    _ => Some(format!(
      "pnix ops --op change.get --payload {} --execute",
      shell_quote(change_get_payload.to_string().as_str())
    )),
  }
}

fn build_retry_command(
  scenario: HintScenario,
  op: &str,
  ns: &str,
  change_id: Option<&str>,
  subject_id: Option<&str>,
  preflight_path: Option<&str>,
) -> Option<String> {
  let default_change = change_id.unwrap_or("chg_REPLACE_ME");
  let default_subject = subject_id.unwrap_or("subject_REPLACE_ME");
  let change_run_payload = json!({
    "ns": ns,
    "change_id": default_change,
  });
  match scenario {
    HintScenario::ConfirmationRequired => Some(format!(
      "pnix ops --op {} --payload @payload.json --execute --yes",
      shell_quote(op)
    )),
    HintScenario::AuthzDenied => Some(format!(
      "pnix ops safe-run {} --subject {} --preflight-spec @process.json",
      shell_quote(default_change),
      shell_quote(default_subject)
    )),
    HintScenario::PreflightDenied
    | HintScenario::PreflightExpired
    | HintScenario::BindingGeneric => Some(format!(
      "pnix ops safe-run {} --subject {} --preflight-spec @process.json",
      shell_quote(default_change),
      shell_quote(default_subject)
    )),
    HintScenario::BindingPlanCore | HintScenario::BindingPlanMeta => Some(format!(
      "pnix ops safe-run {} --subject {} --preflight-spec @process.json",
      shell_quote(default_change),
      shell_quote(default_subject)
    )),
    HintScenario::ChangeGetFailed | HintScenario::TransportFallback => Some(format!(
      "pnix ops --op change.run --payload {}{}",
      shell_quote(change_run_payload.to_string().as_str()),
      preflight_path
        .map(|path| format!(
          " --require-preflight {}",
          shell_quote(shell_path_from_ref(path).as_str())
        ))
        .unwrap_or_default()
    )),
  }
}

fn select_hint_commands(
  candidates: Vec<HintCommandCandidate>,
  allow_high_risk_retry: bool,
) -> Vec<String> {
  let mut selected = Vec::<String>::new();
  let mut seen = HashSet::<String>::new();

  for slot in [
    HintCommandSlot::Inspect,
    HintCommandSlot::Query,
    HintCommandSlot::Retry,
  ] {
    let mut slot_candidates = candidates
      .iter()
      .filter(|candidate| candidate.slot == slot)
      .collect::<Vec<_>>();
    slot_candidates.sort_by_key(|candidate| candidate.priority);

    for candidate in slot_candidates {
      if candidate.risk == HintCommandRisk::High
        && !(allow_high_risk_retry && slot == HintCommandSlot::Retry)
      {
        continue;
      }
      if seen.insert(candidate.command.clone()) {
        selected.push(candidate.command.clone());
        break;
      }
    }
  }

  if !allow_high_risk_retry {
    let mut high_count = 0usize;
    selected.retain(|command| {
      if is_high_risk_hint_command(command) {
        high_count += 1;
        return high_count <= 1;
      }
      true
    });
  }

  if selected.len() > 3 {
    selected.truncate(3);
  }
  selected
}

fn is_high_risk_hint_command(command: &str) -> bool {
  let lowered = command.to_ascii_lowercase();
  lowered.contains("breakglass.request")
    || lowered.contains("compliance.policy.set")
    || (lowered.contains("--execute")
      && (lowered.contains("change.run")
        || lowered.contains("change.approve")
        || lowered.contains("change.reject")
        || lowered.contains("change.cancel")))
}

fn shell_path_from_ref(reference: &str) -> String {
  reference
    .strip_prefix("file://")
    .map(ToString::to_string)
    .unwrap_or_else(|| reference.to_string())
}

fn shell_quote(raw: &str) -> String {
  if raw.is_empty() {
    return "''".to_string();
  }
  format!("'{}'", raw.replace('\'', r"'\''"))
}

fn op_hint_from_argv(argv: &[String]) -> (String, Option<Value>) {
  let mut op = "unknown".to_string();
  let mut payload_raw: Option<String> = None;
  let mut i = 1usize;
  while i < argv.len() {
    match argv[i].as_str() {
      "--op" => {
        i += 1;
        if let Some(value) = argv.get(i) {
          op = value.clone();
        }
      }
      "--payload" => {
        i += 1;
        if let Some(value) = argv.get(i) {
          payload_raw = Some(value.clone());
        }
      }
      raw if !raw.starts_with('-') && op == "unknown" => op = raw.to_string(),
      _ => {}
    }
    i += 1;
  }
  let payload = payload_raw
    .as_deref()
    .and_then(|raw| parse_payload(Some(raw)).ok());
  (op, payload)
}

fn resolve_alias(raw: &str) -> String {
  match raw.trim() {
    "ping" => "ping".to_string(),
    "report.risk" => "report.risk".to_string(),
    "safe-run" => "change.run".to_string(),
    "preflight" => "admission.check".to_string(),
    "preflight.exec-admit" => "admission.check".to_string(),
    "changes" => "change.list".to_string(),
    "change.ls" => "change.list".to_string(),
    "change.get" => "change.get".to_string(),
    "change.approve" => "change.approve".to_string(),
    "change.reject" => "change.reject".to_string(),
    "change.run" => "change.run".to_string(),
    "change.cancel" => "change.cancel".to_string(),
    "remote.execute" => "remote.execute".to_string(),
    "remote.run" => "remote.execute".to_string(),
    "breakglass-sessions" => "breakglass.session.list".to_string(),
    "bg.request" => "breakglass.request".to_string(),
    "bg.ls" => "breakglass.session.list".to_string(),
    "bg.get" => "breakglass.session.get".to_string(),
    "bg.revoke" => "breakglass.session.revoke".to_string(),
    "dr.start" => "dr.drill.start".to_string(),
    "dr.update" => "dr.drill.update".to_string(),
    "dr.get" => "dr.drill.get".to_string(),
    "dr.ls" => "dr.drill.list".to_string(),
    "compliance.get" => "compliance.policy.get".to_string(),
    "compliance.set" => "compliance.policy.set".to_string(),
    "compliance.except.create" => "compliance.exception.create".to_string(),
    "compliance.except.list" => "compliance.exception.list".to_string(),
    "compliance.scan" => "compliance.scan".to_string(),
    "compliance.report" => "compliance.report.latest".to_string(),
    "gateway-stats" => "gateway.decision.stats".to_string(),
    other => other.to_string(),
  }
}

fn resolve_supervisor_endpoint(supervisor_sock: Option<&str>) -> String {
  if let Some(raw) = supervisor_sock.map(str::trim).filter(|v| !v.is_empty()) {
    return super::normalize_supervisor_endpoint(raw);
  }
  if let Some(raw) = std::env::var("PNIX_SUPERVISOR_ENDPOINT")
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
  {
    return super::normalize_supervisor_endpoint(raw.as_str());
  }
  if let Some(raw) = std::env::var("PNIX_SUPERVISOR_SOCK")
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
  {
    return super::normalize_supervisor_endpoint(raw.as_str());
  }
  super::normalize_supervisor_endpoint("/tmp/pnix-supervisor.sock")
}

fn print_ops_help(bin_name: &str) {
  eprintln!("Usage: {} --mode ops [options]", bin_name);
  eprintln!("       {} ops [options]", bin_name);
  eprintln!();
  eprintln!("Options:");
  eprintln!(
    "  --supervisor-sock <path|endpoint>  UDS path or endpoint (uds:/path, tls://host:port)"
  );
  eprintln!("  --endpoint <path|endpoint>         alias of --supervisor-sock");
  eprintln!("  --op <name>                        supervisor op name (default: list)");
  eprintln!("  --payload <json|@file>             op payload JSON or @file path");
  eprintln!("  --caps <c1,c2,...>                 capability list passed to call_with");
  eprintln!(
    "  --emit-payload <file>              write payload JSON and skip execute unless --execute"
  );
  eprintln!(
    "  --emit-preflight-report <file>     write pnix-preflight-report@0.1 for admission.check"
  );
  eprintln!("  --emit-change-snapshot <file>      write pnix-change-snapshot@0.1 for change.get");
  eprintln!("  --emit-bundle <file>               write pnix-safe-run-bundle@0.1 for change.run");
  eprintln!(
    "  --preflight-max-age-ms <ms>        max preflight report age used for expires_ms (default: 300000)"
  );
  eprintln!("  --dry-run                          render/print payload only, do not call RPC");
  eprintln!("  --execute                          perform RPC call (default is render only)");
  eprintln!("  --yes                              skip confirmation prompts");
  eprintln!("  --no-confirm                       skip prompt for medium-risk ops");
  eprintln!("  --confirm <token>                  explicit non-interactive confirmation token");
  eprintln!(
    "  --require-preflight <file>         require preflight decision=allow before change.run"
  );
  eprintln!(
    "  --require-bundle <file>            require pnix-safe-run-bundle@0.1 before change.run"
  );
  eprintln!(
    "  --retry-preflight-only             on retryable failure class, refresh preflight only and exit"
  );
  eprintln!(
    "  --resume-after-refresh             after preflight refresh, complete verify-only stage"
  );
  eprintln!("  --retry-max-attempts <n>           max preflight refresh attempts (default: 3)");
  eprintln!(
    "  --retry-backoff-ms <a,b,c>         retry backoff durations in ms/s/m/h (default: 500,1500,4000)"
  );
  eprintln!("  --help                             show this message");
  eprintln!();
  eprintln!("Examples:");
  eprintln!("  {} --mode ops --op ping", bin_name);
  eprintln!("  {} --mode ops --op list", bin_name);
  eprintln!(
    "  {} --mode ops --op preflight --payload @preflight.json",
    bin_name
  );
  eprintln!(
    "  {} --mode ops --op changes --payload '{{\"status\":\"pending\"}}'",
    bin_name
  );
  eprintln!(
    "  {} --mode ops --op kpi.snapshot --payload '{{\"ns\":\"default\"}}'",
    bin_name
  );
  eprintln!(
    "  {} --mode ops --op kpi.ledger --payload '{{\"ns\":\"default\",\"week_id\":\"2026w08\"}}'",
    bin_name
  );
  eprintln!(
    "  {} --mode ops --op admission.check --payload @preflight.json",
    bin_name
  );
  eprintln!(
    "  {} ops --op change.list --payload '{{\"ns\":\"default\"}}'",
    bin_name
  );
  eprintln!(
    "  {} ops --op preflight.exec-admit --payload @payloads/admission/admission.check.exec_admit.json --dry-run",
    bin_name
  );
  eprintln!(
    "  {} ops --op change.approve --payload @payloads/change/change.approve.json --emit-payload /tmp/change.approve.json",
    bin_name
  );
  eprintln!(
    "  {} ops --op change.run --payload @payloads/change/change.run.json --execute --yes --require-preflight preflight.json",
    bin_name
  );
  eprintln!(
    "  {} ops --op report.risk --payload '{{\"ns\":\"default\"}}' --execute",
    bin_name
  );
  eprintln!(
    "  {} ops --op change.get --payload '{{\"ns\":\"default\",\"change_id\":\"chg_001\"}}' --execute --emit-change-snapshot /tmp/change.snapshot.json",
    bin_name
  );
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn detects_ops_invocation_styles() {
    let by_mode = vec![
      "pnix".to_string(),
      "--mode".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "list".to_string(),
    ];
    let by_mode_equals = vec![
      "pnix".to_string(),
      "--mode=ops".to_string(),
      "--op".to_string(),
      "list".to_string(),
    ];
    let by_subcommand = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "list".to_string(),
    ];
    let non_ops = vec!["pnix".to_string(), "--mode".to_string(), "run".to_string()];
    assert!(is_ops_invocation(&by_mode));
    assert!(is_ops_invocation(&by_mode_equals));
    assert!(is_ops_invocation(&by_subcommand));
    assert!(!is_ops_invocation(&non_ops));
  }

  #[test]
  fn parse_mode_equals_ops_style() {
    let argv = vec![
      "pnix".to_string(),
      "--mode=ops".to_string(),
      "--op".to_string(),
      "change.list".to_string(),
      "--payload".to_string(),
      "{}".to_string(),
    ];
    let args = parse_ops_args(&argv).expect("parse mode=ops style");
    assert_eq!(args.op, "change.list");
    assert_eq!(args.payload, json!({}));
  }

  #[test]
  fn resolves_aliases() {
    assert_eq!(resolve_alias("ping"), "ping");
    assert_eq!(resolve_alias("report.risk"), "report.risk");
    assert_eq!(resolve_alias("safe-run"), "change.run");
    assert_eq!(resolve_alias("preflight"), "admission.check");
    assert_eq!(resolve_alias("preflight.exec-admit"), "admission.check");
    assert_eq!(resolve_alias("changes"), "change.list");
    assert_eq!(resolve_alias("change.ls"), "change.list");
    assert_eq!(resolve_alias("remote.run"), "remote.execute");
    assert_eq!(resolve_alias("bg.request"), "breakglass.request");
    assert_eq!(resolve_alias("dr.start"), "dr.drill.start");
    assert_eq!(
      resolve_alias("compliance.report"),
      "compliance.report.latest"
    );
    assert_eq!(
      resolve_alias("breakglass-sessions"),
      "breakglass.session.list"
    );
    assert_eq!(resolve_alias("gateway-stats"), "gateway.decision.stats");
    assert_eq!(resolve_alias("custom.op"), "custom.op");
  }

  #[test]
  fn defaults_to_render_without_execute() {
    let argv = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "change.list".to_string(),
      "--payload".to_string(),
      "{}".to_string(),
    ];
    let args = parse_ops_args(&argv).expect("parse ops args");
    assert!(!args.execute);
  }

  #[test]
  fn execute_flag_enables_rpc_call() {
    let argv = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "change.list".to_string(),
      "--payload".to_string(),
      "{}".to_string(),
      "--execute".to_string(),
    ];
    let args = parse_ops_args(&argv).expect("parse ops args");
    assert!(args.execute);
  }

  #[test]
  fn emit_without_execute_remains_render_only() {
    let argv = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "change.list".to_string(),
      "--payload".to_string(),
      "{}".to_string(),
      "--emit-payload".to_string(),
      "/tmp/test.json".to_string(),
    ];
    let args = parse_ops_args(&argv).expect("parse ops args");
    assert!(!args.execute);
  }

  #[test]
  fn rejects_execute_dry_run_conflict() {
    let argv = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "change.list".to_string(),
      "--payload".to_string(),
      "{}".to_string(),
      "--dry-run".to_string(),
      "--execute".to_string(),
    ];
    let err = parse_ops_args(&argv).expect_err("must reject conflict");
    assert!(err.to_string().contains("cannot be used together"));
  }

  #[test]
  fn normalizes_duration_string_to_ms() {
    let argv = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "bg.request".to_string(),
      "--payload".to_string(),
      "{\"reason\":\"12345678\", \"duration_ms\":\"30m\"}".to_string(),
    ];
    let args = parse_ops_args(&argv).expect("parse ops args");
    let duration = args
      .payload
      .get("duration_ms")
      .and_then(Value::as_u64)
      .expect("duration");
    assert_eq!(duration, 1_800_000);
  }

  #[test]
  fn classifies_usage_error() {
    let err = anyhow::anyhow!("parse --payload JSON");
    let (exit_code, error_code, details) = classify_ops_error(&err);
    assert_eq!(exit_code.as_i32(), 2);
    assert_eq!(error_code.as_str(), "USAGE");
    assert_eq!(details, json!({}));
  }

  #[test]
  fn classifies_denied_error() {
    let err = anyhow::anyhow!("admission denied: missing evidence");
    let (exit_code, error_code, details) = classify_ops_error(&err);
    assert_eq!(exit_code.as_i32(), 3);
    assert_eq!(error_code.as_str(), "DENIED");
    assert_eq!(details, json!({}));
  }

  #[test]
  fn emits_standard_response_shape() {
    let envelope = response_ok_envelope(
      "change.run",
      Some("default"),
      Some("prod"),
      "req_1",
      1_700_000_000_000,
      json!({"x": 1}),
    );
    assert_eq!(
      envelope.get("schema").and_then(Value::as_str),
      Some("pnix-ops-response@0.1")
    );
    assert_eq!(envelope.get("ok").and_then(Value::as_bool), Some(true));
    assert!(envelope.get("result").is_some());
  }

  #[test]
  fn assigns_risk_level_for_ops() {
    assert_eq!(op_risk_level("change.run"), RiskLevel::High);
    assert_eq!(op_risk_level("remote.execute"), RiskLevel::High);
    assert_eq!(op_risk_level("change.approve"), RiskLevel::Medium);
    assert_eq!(op_risk_level("change.list"), RiskLevel::Low);
  }

  #[test]
  fn expected_confirmation_token_is_deterministic() {
    let payload = json!({"change_id": "chg_001"});
    assert_eq!(
      expected_confirmation_token("change.run", &payload).as_deref(),
      Some("run:chg_001")
    );
    assert_eq!(
      expected_confirmation_token("change.approve", &payload).as_deref(),
      Some("approve:chg_001")
    );
    assert_eq!(
      expected_confirmation_token("remote.execute", &payload).as_deref(),
      Some("remote:chg_001")
    );
  }

  #[test]
  fn reject_emit_preflight_report_without_execute() {
    let argv = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "admission.check".to_string(),
      "--payload".to_string(),
      "{}".to_string(),
      "--emit-preflight-report".to_string(),
      "/tmp/preflight.report.json".to_string(),
    ];
    let err = parse_ops_args(&argv).expect_err("must reject emit-preflight-report without execute");
    assert!(err
      .to_string()
      .contains("--emit-preflight-report requires --execute"));
  }

  #[test]
  fn reject_emit_change_snapshot_without_execute() {
    let argv = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "change.get".to_string(),
      "--payload".to_string(),
      "{\"ns\":\"default\",\"change_id\":\"chg_001\"}".to_string(),
      "--emit-change-snapshot".to_string(),
      "/tmp/change.snapshot.json".to_string(),
    ];
    let err = parse_ops_args(&argv).expect_err("must reject emit-change-snapshot without execute");
    assert!(err
      .to_string()
      .contains("--emit-change-snapshot requires --execute"));
  }

  #[test]
  fn enforce_required_preflight_accepts_valid_report() {
    let report_path =
      std::env::temp_dir().join(format!("pnix-preflight-ok-{}.json", std::process::id()));
    let now = now_ms();
    let report = json!({
      "schema": "pnix-preflight-report@0.1",
      "expires_ms": now + 60_000,
      "binding": {
        "ns": "default",
        "channel": "prod",
        "target": "exec.admit",
        "change_id": "chg_001",
      },
      "input": {
        "breakglass": false
      },
      "output": {
        "allow": true,
        "decision": "allow",
      }
    });
    fs::write(
      &report_path,
      serde_json::to_string_pretty(&report).expect("serialize report"),
    )
    .expect("write report");

    let payload = json!({
      "ns": "default",
      "channel": "prod",
      "change_id": "chg_001",
      "breakglass": false
    });
    enforce_required_preflight(report_path.to_str(), &payload)
      .expect("valid preflight report must pass");

    let _ = fs::remove_file(report_path);
  }

  #[test]
  fn enforce_required_preflight_rejects_expired_report() {
    let report_path = std::env::temp_dir().join(format!(
      "pnix-preflight-expired-{}.json",
      std::process::id()
    ));
    let now = now_ms();
    let report = json!({
      "schema": "pnix-preflight-report@0.1",
      "expires_ms": now - 1,
      "binding": {
        "ns": "default",
        "channel": "prod",
        "target": "exec.admit",
        "change_id": "chg_001",
      },
      "input": {
        "breakglass": false
      },
      "output": {
        "allow": true,
        "decision": "allow",
      }
    });
    fs::write(
      &report_path,
      serde_json::to_string_pretty(&report).expect("serialize report"),
    )
    .expect("write report");

    let payload = json!({
      "ns": "default",
      "channel": "prod",
      "change_id": "chg_001",
      "breakglass": false
    });
    let err = enforce_required_preflight(report_path.to_str(), &payload)
      .expect_err("expired preflight report must be rejected");
    assert!(err.to_string().contains("preflight report expired"));
    let _ = fs::remove_file(report_path);
  }

  #[test]
  fn hint_commands_are_capped_and_deduplicated() {
    let details = json!({
      "workdir": "file://.pnix/ops_runs/inv_123/",
    });
    let commands = build_hint_suggested_commands(
      HintScenario::BindingPlanCore,
      "change.run",
      "default",
      Some("chg_001"),
      Some("svc.web"),
      Some("/tmp/preflight.json"),
      details.as_object(),
    );
    assert!(commands.len() <= 3);
    let uniq = commands.iter().collect::<std::collections::HashSet<_>>();
    assert_eq!(uniq.len(), commands.len());
    assert!(commands
      .first()
      .is_some_and(|cmd| cmd.contains("05_plan_diff")));
  }

  #[test]
  fn hint_commands_block_high_risk_by_default() {
    let details = json!({});
    let commands = build_hint_suggested_commands(
      HintScenario::PreflightDenied,
      "change.run",
      "default",
      Some("chg_001"),
      Some("svc.web"),
      Some("/tmp/preflight.json"),
      details.as_object(),
    );
    assert!(commands.len() <= 3);
    assert!(commands.iter().all(|cmd| !is_high_risk_hint_command(cmd)));
    assert!(commands
      .iter()
      .all(|cmd| !cmd.contains("breakglass.request")));
  }

  #[test]
  fn hint_commands_allow_single_high_risk_for_confirmation() {
    let commands = build_hint_suggested_commands(
      HintScenario::ConfirmationRequired,
      "change.run",
      "default",
      Some("chg_001"),
      Some("svc.web"),
      None,
      None,
    );
    assert!(commands.len() <= 3);
    let high_risk = commands
      .iter()
      .filter(|cmd| is_high_risk_hint_command(cmd))
      .count();
    assert_eq!(high_risk, 1);
    assert!(commands.iter().any(|cmd| cmd.contains("--execute --yes")));
  }

  #[test]
  fn shell_quote_escapes_single_quote() {
    assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    assert_eq!(shell_quote(""), "''");
  }

  #[test]
  fn hint_commands_quote_shell_sensitive_values() {
    let commands = build_hint_suggested_commands(
      HintScenario::TransportFallback,
      "change.run",
      "prod'; touch /tmp/pwn #",
      Some("chg 001"),
      Some("svc'; whoami #"),
      Some("/tmp/preflight report;rm -rf /.json"),
      None,
    );
    let joined = commands.join("\n");
    assert!(joined.contains("--payload '"));
    assert!(joined.contains("'\\''"));
    assert!(joined.contains("--require-preflight '/tmp/preflight report;rm -rf /.json'"));
  }

  #[test]
  fn inspect_command_quotes_preflight_path() {
    let command = build_inspect_command(
      HintScenario::PreflightDenied,
      Some("/tmp/preflight report;cat /etc/passwd"),
      None,
    )
    .expect("inspect command");
    assert_eq!(command, "cat '/tmp/preflight report;cat /etc/passwd'");
  }

  #[test]
  fn auto_retry_forbidden_for_toctou() {
    let decision = decide_auto_retry(
      Some("CHANGE_PLAN_CORE_CHANGED"),
      "TOCTOU",
      "HIGH",
      OpsErrorCode::Denied,
      true,
    );
    assert_eq!(decision.action_id, "AUTO_RETRY_NON_RETRYABLE");
    assert_eq!(decision.retry_class, RetryClass::NonRetryable);
    assert!(decision.retry_policy.is_none());
  }

  #[test]
  fn auto_retry_ok_for_transport() {
    let decision = decide_auto_retry(
      Some("CHANGE_GET_FAILED"),
      "TRANSPORT",
      "MEDIUM",
      OpsErrorCode::Transport,
      false,
    );
    assert_eq!(decision.action_id, "AUTO_RETRY_RETRYABLE");
    assert_eq!(decision.retry_class, RetryClass::Retryable);
    assert!(decision.retry_policy.is_some());
  }

  #[test]
  fn auto_retry_requires_human_for_breakglass_failures() {
    let decision = decide_auto_retry(
      Some("PREFLIGHT_BREAKGLASS_MISMATCH"),
      "PREFLIGHT",
      "HIGH",
      OpsErrorCode::Denied,
      true,
    );
    assert_eq!(decision.action_id, "AUTO_RETRY_REQUIRES_HUMAN");
    assert_eq!(decision.retry_class, RetryClass::RequiresHuman);
    assert!(decision.retry_policy.is_none());
  }

  #[test]
  fn parse_retry_preflight_only_flags() {
    let argv = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "change.run".to_string(),
      "--payload".to_string(),
      "{\"change_id\":\"chg_001\"}".to_string(),
      "--execute".to_string(),
      "--require-preflight".to_string(),
      "/tmp/preflight.json".to_string(),
      "--retry-preflight-only".to_string(),
      "--retry-max-attempts".to_string(),
      "4".to_string(),
      "--retry-backoff-ms".to_string(),
      "500,1500,4s".to_string(),
    ];
    let args = parse_ops_args(&argv).expect("parse retry-preflight options");
    assert!(args.retry_preflight_only);
    assert_eq!(args.retry_max_attempts, 4);
    assert_eq!(args.retry_backoff_ms, vec![500, 1500, 4000]);
  }

  #[test]
  fn parse_require_bundle_without_execute_for_verify_only() {
    let argv = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "safe-run".to_string(),
      "--payload".to_string(),
      "{\"ns\":\"default\",\"channel\":\"prod\",\"change_id\":\"chg_001\"}".to_string(),
      "--require-bundle".to_string(),
      "/tmp/safe-run.bundle.json".to_string(),
    ];
    let args = parse_ops_args(&argv).expect("parse require-bundle verify-only");
    assert_eq!(args.op, "change.run");
    assert!(!args.execute);
    assert_eq!(
      args.require_bundle.as_deref(),
      Some("/tmp/safe-run.bundle.json")
    );
  }

  #[test]
  fn reject_resume_without_retry_preflight_only() {
    let argv = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "change.run".to_string(),
      "--payload".to_string(),
      "{\"change_id\":\"chg_001\"}".to_string(),
      "--execute".to_string(),
      "--require-preflight".to_string(),
      "/tmp/preflight.json".to_string(),
      "--resume-after-refresh".to_string(),
    ];
    let err = parse_ops_args(&argv).expect_err("must reject resume without retry");
    assert!(err
      .to_string()
      .contains("--resume-after-refresh requires --retry-preflight-only"));
  }

  #[test]
  fn reject_emit_bundle_without_execute() {
    let argv = vec![
      "pnix".to_string(),
      "ops".to_string(),
      "--op".to_string(),
      "change.run".to_string(),
      "--payload".to_string(),
      "{\"change_id\":\"chg_001\"}".to_string(),
      "--emit-bundle".to_string(),
      "/tmp/safe-run.bundle.json".to_string(),
    ];
    let err = parse_ops_args(&argv).expect_err("must reject emit-bundle without execute");
    assert!(err.to_string().contains("--emit-bundle requires --execute"));
  }
}
