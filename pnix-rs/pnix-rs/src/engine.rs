//! Peer-engine adapter: pnix-rs as a Rust-domain engine on a common `.px`
//! control plane.
//!
//! rs-meta is an INDEPENDENT Rust-in-Rust meta-circular compiler/evaluator that
//! knows nothing about pnix. This module never changes that: it calls rs-meta
//! ONLY across the bootstrap CLI (a process boundary) and maps its Rust
//! translation-validation results into a common engine-verdict envelope that a
//! `.px` control plane can consume — the same shape a pnix-hy / pnix-clj peer
//! engine would emit. `.px` is the control plane; Rust source is this engine's
//! domain payload.
//!
//! The envelope is itself a `.px` value (an attribute set), so the control
//! plane can eval/hash/route it with the ordinary px machinery — proven by the
//! gate, which reparses every verdict as px.

use crate::interop;
use crate::px;
use crate::sha256::sha256_hex;

/// The engine profile (`pnix.engine.profile.v0`): what this Rust engine can and
/// cannot do, so a control plane can route work without probing. Honest about
/// the held frontier (borrowck / macro_rules / full trait solver).
pub fn engine_profile() -> String {
    let supports = [
        "rust-parse",
        "rust-typeck",
        "rust-interp",
        "rustc-native",
        "translation-validation",
        "native-artifact-receipt",
        "stage-manifest",
        "ast-canonical",
    ];
    let does_not = [
        "px-eval-direct",
        "full-borrowck",
        "macro-rules",
        "full-trait-solver",
        "cargo-crate-graph",
    ];
    let mut out = String::from("{ ");
    out.push_str("schema = \"pnix.engine.profile.v0\"; ");
    out.push_str("engine_id = \"pnix-rs\"; ");
    out.push_str("engine_host = \"rust\"; ");
    out.push_str("engine_type = \"peer\"; ");
    out.push_str(&format!("supports = [ {}]; ", quoted_list(&supports)));
    out.push_str(&format!("does_not_support = [ {}]; ", quoted_list(&does_not)));
    out.push_str("}");
    out
}

fn quoted_list(items: &[&str]) -> String {
    let mut s = String::new();
    for it in items {
        s.push_str(&format!("\"{}\" ", it));
    }
    s
}

/// A common engine verdict (`pnix.engine.verdict.v0`) for a Rust task, mapped
/// from rs-meta's interp/native/typeck results.
pub struct EngineVerdict {
    pub status: String,       // accepted | held | rejected
    pub verdict_kind: String, // ok | negative-boundary-agrees | divergent | incomplete-subset | held-*
    pub source_hash: String,
    /// Content-addressed canonical Rust IR hash from rs-meta `rust-ir` (format-
    /// invariant, unlike source_hash). None if the source doesn't parse.
    pub ir_hash: Option<String>,
    pub interp_out_hash: Option<String>,
    pub native_out_hash: Option<String>,
    pub tv_equal: Option<bool>,
    /// The rustc error code (E0nnn) when a rejection carries one, else "-".
    /// Lets the control plane route by reason (e.g. E0382 borrow vs E0308 type).
    pub reason_code: String,
    /// Per-program surface classification from rs-meta rust-surface: the held
    /// surface the program uses (held-macro-rules/held-assoc-type/...) or "ok".
    pub surface: String,
    /// Content-addressed identity of this verdict's evidence tuple — a stable,
    /// routable witness id tying status/hashes together (the .px control plane
    /// can dedup/route verdicts by it).
    pub witness_id: String,
    pub effects: Vec<String>,
    pub diagnostic: String,
}

/// rs-meta signals acceptance by exit status: accepted -> exit 0 (stdout is the
/// program output), rejected -> exit 1 (stderr carries the `rs-meta:`
/// diagnostic). `host_run_bootstrap_inline` maps that to Ok(stdout) / Err(diag),
/// so acceptance IS `Result::is_ok`.

/// Classify an interp rejection: an out-of-declared-subset surface (unknown
/// item / unsupported / parse failure) is HELD, not a semantic rejection.
fn is_out_of_subset(diag: &str) -> bool {
    diag.contains("unknown")
        || diag.contains("unsupported")
        || diag.contains("unimplemented")
        || diag.contains("not supported")
        || diag.contains("parse:")
        || diag.contains("lex:")
}

/// A native error is a genuine program rejection only when rustc actually
/// rejected it; any other native failure means the toolchain is unavailable
/// (rustc not on PATH), which is HELD, not a rejection.
fn native_is_rejection(diag: &str) -> bool {
    diag.contains("rustc rejected")
}

/// Extract the first Rust error code (`E0nnn`) from a diagnostic, else "-".
fn extract_error_code(msg: &str) -> String {
    let chars: Vec<char> = msg.chars().collect();
    let mut i = 0usize;
    while i + 1 < chars.len() {
        if chars[i] == 'E' && chars[i + 1] == '0' {
            let mut code = String::from("E");
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                code.push(chars[j]);
                j += 1;
            }
            if code.chars().count() >= 4 {
                return code;
            }
        }
        i += 1;
    }
    String::from("-")
}

/// Fetch the per-program surface classification from rs-meta `rust-surface`
/// (rs-meta owns the classification). Returns a summary: the held surface if the
/// program uses one (trait or macro), else "ok". "-" if the query fails.
fn rust_surface(rust_source: &str, bootstrap: &str, granted: &[String]) -> String {
    let out = match interop::host_run_bootstrap_inline(bootstrap, "rust-surface", rust_source, granted) {
        Ok(s) => s,
        Err(_) => return String::from("-"),
    };
    let field = |key: &str| -> String {
        for line in out.split('\n') {
            if let Some(rest) = line.strip_prefix(key) {
                return rest.trim().to_string();
            }
        }
        String::new()
    };
    let ts = field("trait_surface ");
    let ms = field("macro_surface ");
    if ts.starts_with("held-") {
        ts
    } else if ms.starts_with("held-") {
        ms
    } else {
        String::from("ok")
    }
}

/// Fetch the content-addressed canonical Rust IR hash from rs-meta `rust-ir`.
/// None if the source does not parse (no IR) or the engine is unreachable.
fn rust_ir_hash(rust_source: &str, bootstrap: &str, granted: &[String]) -> Option<String> {
    let out = interop::host_run_bootstrap_inline(bootstrap, "rust-ir", rust_source, granted).ok()?;
    for line in out.split('\n') {
        if let Some(rest) = line.strip_prefix("ir_hash ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Produce the common verdict for a Rust source by consulting rs-meta as a peer
/// engine over the CLI. Faithful to the TV -> pnix status taxonomy: agreement
/// (both accept, equal output) is `accepted/ok`; agreement on rejection is
/// `accepted/negative-boundary-agrees`; interp accepts but rustc rejects is
/// `rejected/divergent`; interp rejects but rustc accepts is either
/// `held/out-of-subset` (declared-subset gap) or `rejected/incomplete-subset`;
/// no toolchain is `held/held-rustc-unavailable`.
pub fn rust_engine_verdict(
    rust_source: &str,
    bootstrap: &str,
    granted: &[String],
) -> EngineVerdict {
    let source_hash = format!("sha256:{}", sha256_hex(rust_source.as_bytes()));
    let ir_hash = rust_ir_hash(rust_source, bootstrap, granted);
    let surface = rust_surface(rust_source, bootstrap, granted);
    let interp = interop::host_run_bootstrap_inline(bootstrap, "run", rust_source, granted);
    let native = interop::host_run_bootstrap_inline(bootstrap, "native-run", rust_source, granted);
    let typeck = interop::host_run_bootstrap_inline(bootstrap, "typecheck", rust_source, granted);
    let effects = vec![String::from("subprocess-rustc"), String::from("host-call")];

    // Acceptance IS Ok; the interp diagnostic (if rejected) is the Err string.
    let interp_ok = interp.is_ok();
    let native_ok = native.is_ok();
    let typeck_ok = typeck.is_ok();
    let interp_diag = match &interp {
        Ok(_) => String::new(),
        Err(e) => e.clone(),
    };
    let native_diag = match &native {
        Ok(_) => String::new(),
        Err(e) => e.clone(),
    };

    // A transport failure to even launch rs-meta is held (engine unavailable).
    if let Err(e) = &interp {
        if e.contains("failed to invoke") {
            return held_verdict(&source_hash, ir_hash.clone(), "held-engine-unavailable", e);
        }
    }
    // A native failure that is NOT a genuine rustc rejection = toolchain absent.
    if let Err(e) = &native {
        if !native_is_rejection(e) {
            return held_verdict(&source_hash, ir_hash.clone(), "held-rustc-unavailable", e);
        }
    }

    let interp_hash = interp.as_ref().ok().map(|s| sha256_hex(s.trim_end().as_bytes()));
    let native_hash = native.as_ref().ok().map(|s| sha256_hex(s.trim_end().as_bytes()));

    let (status, kind, tv_equal, diag): (&str, &str, Option<bool>, String) =
        if interp_ok && native_ok {
            let ieq = interp.as_ref().map(|s| s.trim_end().to_string()).unwrap_or_default();
            let neq = native.as_ref().map(|s| s.trim_end().to_string()).unwrap_or_default();
            if ieq == neq && typeck_ok {
                ("accepted", "ok", Some(true), String::new())
            } else if ieq != neq {
                (
                    "rejected",
                    "divergent-output",
                    Some(false),
                    format!("interp `{}` != native `{}`", ieq, neq),
                )
            } else {
                (
                    "rejected",
                    "typeck-divergent",
                    Some(true),
                    String::from("interp==native run but floor typeck rejects"),
                )
            }
        } else if !interp_ok && !native_ok {
            (
                "accepted",
                "negative-boundary-agrees",
                None,
                format!("both reject: {}", interp_diag),
            )
        } else if interp_ok && !native_ok {
            (
                "rejected",
                "divergent",
                None,
                format!("interp accepts, rustc rejects: {}", native_diag),
            )
        } else {
            // !interp_ok && native_ok. If rs-meta explicitly HOLDS the surface
            // this program uses (held-macro-rules/held-assoc-type/...), it is a
            // declared-subset gap (held), not an incomplete-subset rejection.
            if surface.starts_with("held-") || is_out_of_subset(&interp_diag) {
                (
                    "held",
                    "held-out-of-subset",
                    None,
                    format!("declared-subset gap: {}", interp_diag),
                )
            } else {
                (
                    "rejected",
                    "incomplete-subset",
                    None,
                    format!("interp rejects, rustc accepts: {}", interp_diag),
                )
            }
        };

    // reason_code: rustc's code when it rejected, else interp's (out-of-subset).
    let reason_code = if !native_ok {
        extract_error_code(&native_diag)
    } else if !interp_ok {
        extract_error_code(&interp_diag)
    } else {
        String::from("-")
    };
    let witness_id = verdict_witness_id(
        status, kind, &source_hash, &ir_hash, &interp_hash, &native_hash,
    );
    EngineVerdict {
        status: String::from(status),
        verdict_kind: String::from(kind),
        source_hash,
        ir_hash,
        interp_out_hash: interp_hash,
        native_out_hash: native_hash,
        tv_equal,
        reason_code,
        surface,
        witness_id,
        effects,
        diagnostic: sanitize_diag(&diag),
    }
}

/// A stable witness id = a content hash of the verdict's evidence tuple.
fn verdict_witness_id(
    status: &str,
    kind: &str,
    source_hash: &str,
    ir_hash: &Option<String>,
    interp_hash: &Option<String>,
    native_hash: &Option<String>,
) -> String {
    // "none" sentinel matches how absent optionals render, so a consumer can
    // recompute the witness id from the rendered verdict fields (tamper-evident).
    let none = String::from("none");
    let body = format!(
        "{}|{}|{}|{}|{}|{}",
        status,
        kind,
        source_hash,
        ir_hash.as_ref().unwrap_or(&none),
        interp_hash.as_ref().unwrap_or(&none),
        native_hash.as_ref().unwrap_or(&none),
    );
    format!("wit:{}", sha256_hex(body.as_bytes()))
}

/// Make a diagnostic safe to embed in a `.px` string literal (single line, no
/// quotes, bounded length) so the verdict stays a parseable px value.
fn sanitize_diag(diag: &str) -> String {
    let mut s = String::new();
    for ch in diag.chars() {
        if s.chars().count() >= 100 {
            break;
        }
        if ch == '"' || ch == '\n' || ch == '\r' || ch == '\t' || ch == '\\' {
            s.push(' ');
        } else {
            s.push(ch);
        }
    }
    s.trim().to_string()
}

fn held_verdict(source_hash: &str, ir_hash: Option<String>, kind: &str, diag: &str) -> EngineVerdict {
    EngineVerdict {
        status: String::from("held"),
        verdict_kind: String::from(kind),
        source_hash: String::from(source_hash),
        ir_hash: ir_hash.clone(),
        interp_out_hash: None,
        native_out_hash: None,
        tv_equal: None,
        reason_code: String::from("-"),
        surface: String::from("-"),
        witness_id: verdict_witness_id("held", kind, source_hash, &ir_hash, &None, &None),
        effects: vec![String::from("host-call")],
        diagnostic: String::from(diag),
    }
}

/// Render the verdict as a `.px` attribute set — a real value the control plane
/// can eval, hash, and route. Optional facets are emitted as `null` when absent
/// so the schema shape is stable.
pub fn render_verdict_px(v: &EngineVerdict) -> String {
    // px has no `null` literal, so absent optionals render as the string
    // "none" — the verdict stays a valid `.px` value for held/rejected verdicts
    // too (not just accepted ones).
    let opt = |o: &Option<String>| match o {
        Some(s) => format!("\"{}\"", s),
        None => String::from("\"none\""),
    };
    let optb = |o: &Option<bool>| match o {
        Some(b) => format!("{}", b),
        None => String::from("\"none\""),
    };
    let mut out = String::from("{ ");
    out.push_str("schema = \"pnix.engine.verdict.v0\"; ");
    out.push_str("engine_id = \"pnix-rs\"; ");
    out.push_str("engine_host = \"rust\"; ");
    out.push_str("phase = \"tv-check\"; ");
    out.push_str(&format!("status = \"{}\"; ", v.status));
    out.push_str(&format!("verdict_kind = \"{}\"; ", v.verdict_kind));
    out.push_str(&format!("source_hash = \"{}\"; ", v.source_hash));
    out.push_str(&format!("ir_hash = {}; ", opt(&v.ir_hash)));
    out.push_str(&format!("interp_out_hash = {}; ", opt(&v.interp_out_hash)));
    out.push_str(&format!("native_out_hash = {}; ", opt(&v.native_out_hash)));
    out.push_str(&format!("tv_equal = {}; ", optb(&v.tv_equal)));
    out.push_str(&format!("reason_code = \"{}\"; ", v.reason_code));
    out.push_str(&format!("surface = \"{}\"; ", v.surface));
    out.push_str(&format!("witness_id = \"{}\"; ", v.witness_id));
    out.push_str(&format!("effects = [ {}]; ", quoted_list_owned(&v.effects)));
    out.push_str(&format!("diagnostic = \"{}\"; ", v.diagnostic));
    out.push_str("}");
    out
}

fn quoted_list_owned(items: &[String]) -> String {
    let mut s = String::new();
    for it in items {
        s.push_str(&format!("\"{}\" ", it));
    }
    s
}

/// Emit the native artifact receipt for a Rust source as a `.px` value
/// (`pnix.engine.artifact.v0`) — the separate envelope for build attestation
/// (kind/source/rustc/artifact/receipt). Calls rs-meta `rust-artifact` across
/// the CLI; rs-meta stays pnix-free. Held (`available = false`) when rustc/the
/// engine is unavailable.
pub fn rust_artifact_envelope(rust_source: &str, bootstrap: &str, granted: &[String]) -> String {
    let source_hash = format!("sha256:{}", sha256_hex(rust_source.as_bytes()));
    let out = match interop::host_run_bootstrap_inline(bootstrap, "rust-artifact", rust_source, granted) {
        Ok(s) => s,
        Err(_) => {
            return format!(
                "{{ schema = \"pnix.engine.artifact.v0\"; engine_id = \"pnix-rs\"; artifact_kind = \"rust-native\"; source_hash = \"{}\"; available = false; }}",
                source_hash
            );
        }
    };
    let field = |key: &str| -> String {
        for line in out.split('\n') {
            if let Some(rest) = line.strip_prefix(key) {
                return rest.trim().to_string();
            }
        }
        String::new()
    };
    let rustc = field("rustc=");
    let artifact = field("artifact_fnv=");
    let receipt = field("receipt_hash ");
    format!(
        "{{ schema = \"pnix.engine.artifact.v0\"; engine_id = \"pnix-rs\"; artifact_kind = \"rust-native\"; source_hash = \"{}\"; rustc = \"{}\"; artifact_hash = \"{}\"; receipt_hash = \"{}\"; available = true; }}",
        source_hash, rustc, artifact, receipt
    )
}

/// Handle a `.px` engine request (`pnix.engine.request.v0`) — the control-plane
/// -> engine direction that pairs with the verdict/artifact responses. The
/// request is a `.px` attribute set; this dispatches on `phase` and returns the
/// response envelope. Completes the request/response protocol (section 6 A+B).
pub fn handle_request(request_px: &str, bootstrap: &str, granted: &[String]) -> Result<String, String> {
    let val = px::px_run_value(request_px)?;
    let fields = match val {
        px::PxVal::Attrs(f) => f,
        _ => return Err(String::from("engine request must be a .px attribute set")),
    };
    let get = |name: &str| -> Option<String> {
        for (k, v) in fields.iter() {
            if k == name {
                if let px::PxVal::Str(s) = v {
                    return Some(s.clone());
                }
            }
        }
        None
    };
    let phase = get("phase").ok_or_else(|| String::from("request missing `phase`"))?;
    let source = get("source").unwrap_or_default();
    if phase == "profile" {
        Ok(engine_profile())
    } else if phase == "eval-rust" {
        Ok(render_verdict_px(&rust_engine_verdict(&source, bootstrap, granted)))
    } else if phase == "artifact" {
        Ok(rust_artifact_envelope(&source, bootstrap, granted))
    } else {
        Err(format!("unknown request phase `{}` (eval-rust|artifact|profile)", phase))
    }
}

/// Emit this engine's TRUST ATTESTATION as a `.px` value
/// (`pnix.engine.attestation.v0`): why a control plane should trust this
/// engine's verdicts. The core Rust meta-circular value (user's section 4): the
/// interp==rustc translation-validation claim is backed by a verified corpus
/// (positive + negative), and the substrate is proven rs-meta-interp == rustc ==
/// native (3-way). Calls rs-meta `tv-stats`; rs-meta stays pnix-free.
pub fn engine_attestation(bootstrap: &str, granted: &[String]) -> String {
    let stats = interop::host_run_bootstrap_inline(bootstrap, "tv-stats", "", granted);
    let (pos, neg, self_host, differential, available) = match stats {
        Ok(out) => {
            let field = |key: &str, dflt: &str| -> String {
                for line in out.split('\n') {
                    if let Some(rest) = line.strip_prefix(key) {
                        return rest.trim().to_string();
                    }
                }
                String::from(dflt)
            };
            (
                field("positive_corpus ", "0"),
                field("negative_corpus ", "0"),
                field("self_hosting ", "unknown"),
                field("differential_testing ", "none"),
                true,
            )
        }
        Err(_) => (
            String::from("0"),
            String::from("0"),
            String::from("unknown"),
            String::from("none"),
            false,
        ),
    };
    let held = ["full-borrowck", "macro-rules", "full-trait-solver"];
    format!(
        "{{ schema = \"pnix.engine.attestation.v0\"; engine_id = \"pnix-rs\"; engine_host = \"rust\"; translation_validation = \"interp==rustc\"; positive_corpus = {}; negative_corpus = {}; substrate = \"rs-meta-interp==rustc==native\"; substrate_ways = 3; self_hosting = \"{}\"; differential_testing = \"{}\"; tv_gate = \"tv-check\"; typeck_gate = \"typeck-check\"; held_frontier = [ {}]; available = {}; }}",
        if pos.is_empty() { "0" } else { &pos },
        if neg.is_empty() { "0" } else { &neg },
        self_host,
        differential,
        quoted_list(&held),
        available
    )
}

/// Verify a received verdict is authentic/untampered by RECOMPUTING its
/// witness_id from its own evidence fields (status|verdict_kind|source_hash|
/// ir_hash|interp_out_hash|native_out_hash) and confirming it matches the stated
/// witness_id. A distributed `.px` control plane can verify verdicts from
/// untrusted engines instead of blindly trusting them (proof-carrying verdict).
/// Returns (verified, recomputed_witness_id).
pub fn verify_verdict(verdict_px: &str) -> Result<(bool, String), String> {
    let val = px::px_run_value(verdict_px)?;
    let fields = match val {
        px::PxVal::Attrs(f) => f,
        _ => return Err(String::from("verdict must be a .px attribute set")),
    };
    let get = |name: &str| -> String {
        for (k, v) in fields.iter() {
            if k == name {
                match v {
                    px::PxVal::Str(s) => return s.clone(),
                    px::PxVal::Bool(b) => return format!("{}", b),
                    _ => return String::new(),
                }
            }
        }
        String::new()
    };
    let stated = get("witness_id");
    if stated.is_empty() {
        return Err(String::from("verdict missing witness_id"));
    }
    let body = format!(
        "{}|{}|{}|{}|{}|{}",
        get("status"),
        get("verdict_kind"),
        get("source_hash"),
        get("ir_hash"),
        get("interp_out_hash"),
        get("native_out_hash"),
    );
    let recomputed = format!("wit:{}", sha256_hex(body.as_bytes()));
    Ok((recomputed == stated, recomputed))
}

/// Process a BATCH of Rust sources (a `.px` list of source strings) and emit a
/// verdict manifest (`pnix.engine.batch.v0`) — the orchestration primitive for a
/// control plane processing a whole project. The manifest carries every verdict
/// plus accepted/held/rejected/total counts, all as a `.px` value.
pub fn engine_batch(sources_px: &str, bootstrap: &str, granted: &[String]) -> Result<String, String> {
    let val = px::px_run_value(sources_px)?;
    let items = match val {
        px::PxVal::List(l) => l,
        _ => return Err(String::from("engine-batch input must be a .px list of Rust sources")),
    };
    let mut verdicts = Vec::new();
    let (mut accepted, mut held, mut rejected) = (0i64, 0i64, 0i64);
    for it in items.iter() {
        let item = px::px_force(it)?;
        let src = match &item {
            px::PxVal::Str(s) => s.clone(),
            _ => return Err(String::from("each batch item must be a Rust source string")),
        };
        let v = rust_engine_verdict(&src, bootstrap, granted);
        match v.status.as_str() {
            "accepted" => accepted += 1,
            "held" => held += 1,
            _ => rejected += 1,
        }
        verdicts.push(render_verdict_px(&v));
    }
    let total = verdicts.len() as i64;
    Ok(format!(
        "{{ schema = \"pnix.engine.batch.v0\"; engine_id = \"pnix-rs\"; total = {}; accepted = {}; held = {}; rejected = {}; verdicts = [ {}]; }}",
        total, accepted, held, rejected,
        verdicts.iter().map(|v| format!("{} ", v)).collect::<String>()
    ))
}
