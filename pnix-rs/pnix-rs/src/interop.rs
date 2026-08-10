//! Host-call boundary for the pnix-rs lane (P5).
//!
//! Every host contact this lane makes goes through this module — nothing else
//! spawns processes or touches the filesystem. Today the lane's entire host
//! surface is exactly two effects:
//!
//!   file-read  — corpus/source file reads
//!   host-call  — the rs-meta bootstrap subprocess (the substrate contract)
//!
//! Each call is capability-checked against a granted-effects list. Grants are
//! not user flags: each CLI command *declares* the minimal capabilities its
//! purpose requires, and that declaration is what shows up in witnesses.
//!
//! Invariant (documented, enforced by construction): `PxVal` has no host-object
//! variant — host results always cross this boundary as plain data (strings).
//! If an opaque host handle ever seems necessary, that is a held boundary and a
//! proposal, not a new variant.
//!
//! Explicitly not claimed: OS-level sandboxing/filesystem isolation. The
//! capability check is the lane's admission discipline, not a security sandbox.

use crate::gate;
use crate::sha256::sha256_hex;
use std::process::Command;

#[path = "../../rs-meta/src/io.rs"]
mod meta_io;

/// Admission check: the effect must be in the granted list.
pub fn check_capability(effect: &str, granted: &[String]) -> Result<(), String> {
    if granted.iter().any(|g| g == effect) {
        Ok(())
    } else {
        Err(format!("capability denied: {}", effect))
    }
}

/// SES-style capability ATTENUATION (proposal, maps pnix-hy 23): derive a
/// STRICTLY NARROWER grant by removing effects. Attenuation only ever removes
/// (least-privilege): the result is always a subset of the input, so a
/// derivation chain can only weaken, never re-widen — an attenuated handle
/// cannot recover an effect the parent dropped.
pub fn attenuate(granted: &[String], remove: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for g in granted {
        if !remove.iter().any(|r| r == g) {
            out.push(g.clone());
        }
    }
    out
}

/// Revoke: the empty grant denies every effect.
pub fn revoke() -> Vec<String> {
    Vec::new()
}

/// True iff `child` is an attenuation (subset) of `parent` — used to gate that
/// a derived handle never gains an effect the parent lacked.
pub fn is_attenuation_of(child: &[String], parent: &[String]) -> bool {
    child.iter().all(|c| parent.iter().any(|p| p == c))
}

/// The single filesystem-read gate.
pub fn host_read_file(path: &str, granted: &[String]) -> Result<String, String> {
    meta_io::read_utf8(path, granted)
        .map_err(|e| format!("{}: {}", e.error_class, e.message))
}

/// The single directory-listing gate (same `file-read` capability class as
/// `host_read_file` — a listing is a read). Returns (path, is_dir) entries,
/// sorted, so callers can walk a tree without touching std::fs themselves.
pub fn host_list_dir(path: &str, granted: &[String]) -> Result<Vec<(String, bool)>, String> {
    let rows = meta_io::read_dir(path, granted)
        .map_err(|e| format!("{}: {}", e.error_class, e.message))?;
    let mut entries = Vec::new();
    for (name, kind) in rows {
        let full = std::path::Path::new(path).join(name);
        entries.push((full.to_string_lossy().into_owned(), kind == "directory"));
    }
    entries.sort();
    Ok(entries)
}

#[derive(Clone, Debug)]
pub enum EffectValue {
    None,
    Bool(bool),
    Text(String),
    Directory(Vec<(String, String)>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectOperation {
    PathExists,
    Open,
    FileType,
    ReadDir,
}

impl EffectOperation {
    fn from_id(operation_id: &str) -> Option<Self> {
        match operation_id {
            "fs.path-exists" => Some(Self::PathExists),
            "fs.open" => Some(Self::Open),
            "fs.file-type" => Some(Self::FileType),
            "fs.read-dir" => Some(Self::ReadDir),
            _ => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::PathExists => "fs.path-exists",
            Self::Open => "fs.open",
            Self::FileType => "fs.file-type",
            Self::ReadDir => "fs.read-dir",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectFailurePhase {
    EffectContract,
    Effect,
}

impl EffectFailurePhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::EffectContract => "effect-contract",
            Self::Effect => "effect",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectFailureClass {
    UnknownEffectOperation,
    InvalidEffectArgs,
    EffectDenied,
    EffectAdapterError,
}

impl EffectFailureClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::UnknownEffectOperation => "unknown-effect-operation",
            Self::InvalidEffectArgs => "invalid-effect-args",
            Self::EffectDenied => "effect-denied",
            Self::EffectAdapterError => "effect-adapter-error",
        }
    }
}

#[derive(Clone, Debug)]
pub struct EffectReceipt {
    pub effect: String,
    pub risk_tier: String,
    pub capability_id: String,
    pub executed: bool,
    pub adapter: String,
}

#[derive(Clone, Debug)]
pub enum EffectResponse {
    Executed {
        operation: EffectOperation,
        value: EffectValue,
        receipt: EffectReceipt,
    },
    Failed {
        phase: EffectFailurePhase,
        class: EffectFailureClass,
        operation_id: String,
        receipt: EffectReceipt,
    },
}

impl EffectResponse {
    fn value(&self) -> Option<&EffectValue> {
        match self {
            Self::Executed { value, .. } => Some(value),
            Self::Failed { .. } => None,
        }
    }

    fn receipt(&self) -> &EffectReceipt {
        match self {
            Self::Executed { receipt, .. } | Self::Failed { receipt, .. } => receipt,
        }
    }

    fn error_projection(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Failed { phase, class, .. } => Some((phase.as_str(), class.as_str())),
            Self::Executed { .. } => None,
        }
    }
}

fn effect_response(
    operation: EffectOperation,
    capability_id: &str,
    risk_tier: &str,
    result: Result<EffectValue, meta_io::MetaIoError>,
) -> EffectResponse {
    match result {
        Ok(value) => EffectResponse::Executed {
            operation,
            value,
            receipt: EffectReceipt {
                effect: String::from(operation.id()),
                risk_tier: String::from(risk_tier),
                capability_id: String::from(capability_id),
                executed: true,
                adapter: String::from("host-meta-io-v1"),
            },
        },
        Err(err) => EffectResponse::Failed {
            phase: EffectFailurePhase::Effect,
            class: if err.error_class == "capability-denied" {
                EffectFailureClass::EffectDenied
            } else {
                EffectFailureClass::EffectAdapterError
            },
            operation_id: String::from(operation.id()),
            receipt: EffectReceipt {
                effect: String::from(operation.id()),
                risk_tier: String::from(risk_tier),
                capability_id: String::from(capability_id),
                executed: false,
                adapter: String::from("host-meta-io-v1"),
            },
        },
    }
}

/// Execute one already-validated pnix-meta read-only effect request.
pub fn apply_effect_request(
    operation_id: &str,
    path: Option<&str>,
    capability_id: &str,
    risk_tier: &str,
    granted: &[String],
) -> EffectResponse {
    let operation = match EffectOperation::from_id(operation_id) {
        Some(operation) => operation,
        None => return EffectResponse::Failed {
            phase: EffectFailurePhase::EffectContract,
            class: EffectFailureClass::UnknownEffectOperation,
            operation_id: String::from(operation_id),
            receipt: EffectReceipt {
                effect: String::from(operation_id),
                risk_tier: String::from(risk_tier),
                capability_id: String::from(capability_id),
                executed: false,
                adapter: String::from("host-meta-io-v1"),
            },
        },
    };
    let path = match path {
        Some(path) => path,
        None => return EffectResponse::Failed {
            phase: EffectFailurePhase::EffectContract,
            class: EffectFailureClass::InvalidEffectArgs,
            operation_id: String::from(operation.id()),
            receipt: EffectReceipt {
                effect: String::from(operation.id()),
                risk_tier: String::from(risk_tier),
                capability_id: String::from(capability_id),
                executed: false,
                adapter: String::from("host-meta-io-v1"),
            },
        },
    };
    match operation {
        EffectOperation::PathExists => effect_response(
            operation,
            capability_id,
            risk_tier,
            meta_io::path_exists(path, granted).map(EffectValue::Bool),
        ),
        EffectOperation::Open => effect_response(
            operation,
            capability_id,
            risk_tier,
            meta_io::read_utf8(path, granted).map(EffectValue::Text),
        ),
        EffectOperation::FileType => effect_response(
            operation,
            capability_id,
            risk_tier,
            meta_io::file_type(path, granted).map(EffectValue::Text),
        ),
        EffectOperation::ReadDir => effect_response(
            operation,
            capability_id,
            risk_tier,
            meta_io::read_dir(path, granted).map(EffectValue::Directory),
        ),
    }
}

fn json_quote(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        if ch == '\"' {
            out.push_str("\\\"");
        } else if ch == '\\' {
            out.push_str("\\\\");
        } else if ch == '\n' {
            out.push_str("\\n");
        } else if ch == '\r' {
            out.push_str("\\r");
        } else if ch == '\t' {
            out.push_str("\\t");
        } else {
            out.push(ch);
        }
    }
    out.push('\"');
    out
}

pub fn io_probe_json(root: &str) -> Result<String, String> {
    let note = std::path::Path::new(root).join("note.txt");
    let missing = std::path::Path::new(root).join("missing.txt");
    let note_text = note.to_string_lossy().into_owned();
    let missing_text = missing.to_string_lossy().into_owned();
    let grants = vec![String::from("file-read")];
    let cap = "pnix.io.file-read.v1";
    let risk = "bounded-read";

    let exists = apply_effect_request("fs.path-exists", Some(&note_text), cap, risk, &grants);
    let missing_result = apply_effect_request("fs.path-exists", Some(&missing_text), cap, risk, &grants);
    let opened = apply_effect_request("fs.open", Some(&note_text), cap, risk, &grants);
    let typed = apply_effect_request("fs.file-type", Some(&note_text), cap, risk, &grants);
    let listed = apply_effect_request("fs.read-dir", Some(root), cap, risk, &grants);
    let denied = apply_effect_request("fs.open", Some(&note_text), cap, risk, &Vec::new());
    let adapter_error = apply_effect_request("fs.open", Some(&missing_text), cap, risk, &grants);
    let invalid = apply_effect_request("fs.open", None, cap, risk, &grants);
    let unsupported = apply_effect_request("fs.unknown", Some(&note_text), cap, risk, &grants);

    let path_exists = match exists.value() { Some(EffectValue::Bool(v)) => *v, _ => return Err(String::from("path-exists probe failed")) };
    let missing_exists = match missing_result.value() { Some(EffectValue::Bool(v)) => *v, _ => return Err(String::from("missing path probe failed")) };
    let open = match opened.value() { Some(EffectValue::Text(v)) => v.clone(), _ => return Err(String::from("open probe failed")) };
    let file_type = match typed.value() { Some(EffectValue::Text(v)) => v.clone(), _ => return Err(String::from("file-type probe failed")) };
    let directory = match listed.value() { Some(EffectValue::Directory(v)) => v.clone(), _ => return Err(String::from("read-dir probe failed")) };
    let denied_error = denied.error_projection().ok_or_else(|| String::from("denial was not failed"))?;
    let adapter_error_projection = adapter_error.error_projection().ok_or_else(|| String::from("adapter error was not failed"))?;
    let invalid_error = invalid.error_projection().ok_or_else(|| String::from("invalid request was not failed"))?;
    let unsupported_error = unsupported.error_projection().ok_or_else(|| String::from("unsupported request was not failed"))?;
    let mut dir_json = String::from("{");
    for (index, (name, kind)) in directory.iter().enumerate() {
        if index > 0 { dir_json.push(','); }
        dir_json.push_str(&json_quote(name));
        dir_json.push(':');
        dir_json.push_str(&json_quote(kind));
    }
    dir_json.push('}');
    let all_ok = adapter_error_projection == ("effect", "effect-adapter-error")
        && path_exists
        && !missing_exists
        && open == "hello"
        && file_type == "regular"
        && directory == vec![
            (String::from("note.txt"), String::from("regular")),
            (String::from("subdir"), String::from("directory")),
        ]
        && denied_error == ("effect", "effect-denied")
        && invalid_error == ("effect-contract", "invalid-effect-args")
        && unsupported_error == ("effect-contract", "unknown-effect-operation")
        && matches!(opened, EffectResponse::Executed { operation: EffectOperation::Open, .. })
        && opened.receipt().effect == "fs.open"
        && opened.receipt().risk_tier == risk
        && opened.receipt().capability_id == cap
        && opened.receipt().adapter == "host-meta-io-v1"
        && opened.receipt().executed;
    Ok(format!(
        "{{\"adapter_error\":{{\"class\":{},\"phase\":{}}},\"all_ok\":{},\"denied\":{{\"class\":{},\"phase\":{}}},\"file_type\":{},\"invalid\":{{\"class\":{},\"phase\":{}}},\"missing_exists\":{},\"open\":{},\"path_exists\":{},\"read_dir\":{},\"receipt_adapter\":{},\"schema\":\"pnix-meta.host-io-probe.v1\",\"unsupported\":{{\"class\":{},\"phase\":{}}}}}",
        json_quote(adapter_error_projection.1),
        json_quote(adapter_error_projection.0),
        all_ok,
        json_quote(denied_error.1),
        json_quote(denied_error.0),
        json_quote(&file_type),
        json_quote(invalid_error.1),
        json_quote(invalid_error.0),
        missing_exists,
        json_quote(&open),
        path_exists,
        dir_json,
        json_quote(&opened.receipt().adapter),
        json_quote(unsupported_error.1),
        json_quote(unsupported_error.0),
    ))
}

/// The single subprocess gate: run the rs-meta bootstrap over source files.
pub fn host_run_bootstrap(
    bootstrap: &str,
    mode: &str,
    files: &[&str],
    granted: &[String],
) -> Result<String, String> {
    check_capability("host-call", granted)?;
    let mut cmd = Command::new(bootstrap);
    cmd.arg(mode);
    for f in files {
        cmd.arg("-f");
        cmd.arg(f);
    }
    let output = cmd.output().map_err(|e| {
        format!(
            "failed to invoke rs-meta bootstrap at {} (set RS_META_BOOTSTRAP): {}",
            bootstrap, e
        )
    })?;
    if !output.status.success() {
        return Err(format!(
            "bootstrap {} failed: {}",
            mode,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Witness for one host call (13-field shared schema).
pub fn host_call_witness(mode: &str, in_desc: &str, out: &str, granted: &[String]) -> gate::Witness {
    let mut sorted: Vec<String> = granted.to_vec();
    sorted.sort();
    gate::Witness {
        direction: String::from("host-call"),
        source_lang: String::from("px-lane"),
        target_lang: String::from("rust-substrate"),
        input_kind: String::from(mode),
        output_kind: String::from("stdout"),
        loss_status: String::from("lossless"),
        effect_class: String::from("host-call"),
        capability_required: String::from("host-call"),
        in_hash: sha256_hex(in_desc.as_bytes()),
        out_hash: sha256_hex(out.as_bytes()),
        env_hash: sha256_hex(format!("granted={}", sorted.join(",")).as_bytes()),
        status: String::from("ok"),
        loss: String::from("none"),
    }
}

/// Subprocess gate for inline source (`bootstrap run|native-run -c <code>`).
pub fn host_run_bootstrap_inline(
    bootstrap: &str,
    mode: &str,
    code: &str,
    granted: &[String],
) -> Result<String, String> {
    check_capability("host-call", granted)?;
    let output = Command::new(bootstrap)
        .arg(mode)
        .arg("-c")
        .arg(code)
        .output()
        .map_err(|e| {
            format!(
                "failed to invoke rs-meta bootstrap at {} (set RS_META_BOOTSTRAP): {}",
                bootstrap, e
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "bootstrap {} failed: {}",
            mode,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Subprocess gate for clean-process self-replay (`pnix-rs <check-cmd>`).
pub fn host_run_self(args: &[&str], granted: &[String]) -> Result<(bool, String), String> {
    check_capability("host-call", granted)?;
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
    let output = Command::new(exe)
        .args(args)
        .output()
        .map_err(|e| format!("failed to spawn self: {}", e))?;
    Ok((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
    ))
}

/// The single filesystem-write gate (receipts, generated docs).
pub fn host_write_file(path: &str, content: &str, granted: &[String]) -> Result<(), String> {
    check_capability("file-write", granted)?;
    std::fs::write(path, content).map_err(|e| format!("cannot write {}: {}", path, e))
}

/// Directory-creation gate (receipt/cache directories).
pub fn host_ensure_dir(path: &str, granted: &[String]) -> Result<(), String> {
    check_capability("file-write", granted)?;
    std::fs::create_dir_all(path).map_err(|e| format!("cannot create {}: {}", path, e))
}

/// File-removal gate (scratch stores in checks).
pub fn host_remove_file(path: &str, granted: &[String]) -> Result<(), String> {
    check_capability("file-write", granted)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("cannot remove {}: {}", path, e)),
    }
}
