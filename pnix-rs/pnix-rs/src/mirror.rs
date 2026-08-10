//! Singleton pnix mirror run for the pnix-rs lane (P1).
//!
//! One canonical entrypoint (`mirror_run`) evaluates a .px source once and
//! emits every mirror facet from that single route: token count, evaluated
//! canonical value, emitted px source, reparse/re-eval comparison, emit fixed
//! point, content hashes, and a roundtrip status from the fixed vocabulary
//!
//!   lossless | lossy-ok | held | rejected
//!
//! (same vocabulary as the pnix-hy mirror lane). Any older or narrower view of
//! the mirror must project fields out of this record instead of owning its own
//! parse/eval route — the runtime in `px.rs` stays the single sacred evaluator.
//!
//! Hashes are FNV-1a 64 for now (explicitly labeled `fnv64`); an in-house
//! SHA-256 upgrade is a separate roadmap item (todo §4.1 P3).

use crate::px;

pub const MIRROR_SCHEMA: &str = "pnix-rs.mirror.v0";

pub struct MirrorRecord {
    pub schema: &'static str,
    pub source_fnv: u64,
    pub tokens: usize,
    pub value: Option<String>,
    pub value_fnv: Option<u64>,
    pub emitted: Option<String>,
    pub emitted_fnv: Option<u64>,
    pub reparse_ok: bool,
    pub revalue_match: bool,
    pub emit_fixed_point: bool,
    pub status: &'static str,
    pub error: Option<String>,
}

pub fn fnv64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn rejected(source: &str, tokens: usize, error: String) -> MirrorRecord {
    MirrorRecord {
        schema: MIRROR_SCHEMA,
        source_fnv: fnv64(source),
        tokens,
        value: None,
        value_fnv: None,
        emitted: None,
        emitted_fnv: None,
        reparse_ok: false,
        revalue_match: false,
        emit_fixed_point: false,
        status: "rejected",
        error: Some(error),
    }
}

/// Canonical singleton mirror run: parse -> eval -> emit -> reparse -> re-eval
/// -> emit again, all facets from this one route.
pub fn mirror_run(source: &str) -> MirrorRecord {
    let tokens = match px::px_tokens(source) {
        Ok(n) => n,
        Err(e) => return rejected(source, 0, e),
    };
    let ast = match px::px_parse(source) {
        Ok(a) => a,
        Err(e) => return rejected(source, tokens, e),
    };
    let env = Vec::new();
    let v1 = match px::px_eval(&ast, &env) {
        Ok(v) => v,
        Err(e) => return rejected(source, tokens, format!("eval: {}", e)),
    };
    let value = px::px_print(&v1);
    let value_fnv = fnv64(&value);
    let emitted = px::px_emit(&ast);
    let emitted_fnv = fnv64(&emitted);

    // Opaque leaves (closures/builtins) make value comparison unfaithful:
    // the mirror holds rather than overclaims.
    if px::px_value_has_opaque(&v1) {
        return MirrorRecord {
            schema: MIRROR_SCHEMA,
            source_fnv: fnv64(source),
            tokens,
            value: Some(value),
            value_fnv: Some(value_fnv),
            emitted: Some(emitted),
            emitted_fnv: Some(emitted_fnv),
            reparse_ok: false,
            revalue_match: false,
            emit_fixed_point: false,
            status: "held",
            error: Some(String::from("value contains opaque leaves (lambda/builtin)")),
        };
    }

    let ast2 = match px::px_parse(&emitted) {
        Ok(a) => a,
        Err(e) => {
            return MirrorRecord {
                schema: MIRROR_SCHEMA,
                source_fnv: fnv64(source),
                tokens,
                value: Some(value),
                value_fnv: Some(value_fnv),
                emitted: Some(emitted),
                emitted_fnv: Some(emitted_fnv),
                reparse_ok: false,
                revalue_match: false,
                emit_fixed_point: false,
                status: "rejected",
                error: Some(format!("reparse of emitted source failed: {}", e)),
            }
        }
    };
    let revalue_match = match px::px_eval(&ast2, &env) {
        Ok(v2) => px::px_print(&v2) == value,
        Err(_e) => false,
    };
    let emit_fixed_point = px::px_emit(&ast2) == emitted;

    let status = if revalue_match && emit_fixed_point {
        "lossless"
    } else if revalue_match {
        "lossy-ok"
    } else {
        "rejected"
    };
    MirrorRecord {
        schema: MIRROR_SCHEMA,
        source_fnv: fnv64(source),
        tokens,
        value: Some(value),
        value_fnv: Some(value_fnv),
        emitted: Some(emitted),
        emitted_fnv: Some(emitted_fnv),
        reparse_ok: true,
        revalue_match,
        emit_fixed_point,
        status,
        error: None,
    }
}

/// Stable text receipt of a mirror record (one facet per line).
pub fn render(record: &MirrorRecord) -> String {
    let mut out = String::new();
    out.push_str(&format!("schema {}\n", record.schema));
    out.push_str(&format!("source_fnv {:016x}\n", record.source_fnv));
    out.push_str(&format!("tokens {}\n", record.tokens));
    match &record.value {
        Some(v) => out.push_str(&format!("value {}\n", v)),
        None => out.push_str("value -\n"),
    }
    match record.value_fnv {
        Some(h) => out.push_str(&format!("value_fnv {:016x}\n", h)),
        None => out.push_str("value_fnv -\n"),
    }
    match &record.emitted {
        Some(v) => out.push_str(&format!("emit {}\n", v)),
        None => out.push_str("emit -\n"),
    }
    match record.emitted_fnv {
        Some(h) => out.push_str(&format!("emit_fnv {:016x}\n", h)),
        None => out.push_str("emit_fnv -\n"),
    }
    out.push_str(&format!("reparse_ok {}\n", record.reparse_ok));
    out.push_str(&format!("revalue_match {}\n", record.revalue_match));
    out.push_str(&format!("emit_fixed_point {}\n", record.emit_fixed_point));
    out.push_str(&format!("status {}\n", record.status));
    match &record.error {
        Some(e) => out.push_str(&format!("error {}\n", e)),
        None => {}
    }
    out
}
