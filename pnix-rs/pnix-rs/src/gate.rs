//! pnix runtime gate + witnesses for the pnix-rs lane (P4).
//!
//! Capability-aware admission on top of a static purity check: every builtin a
//! program can reach is classified against an effect table, and the program is
//! admitted only when every effect it requires is granted AND nothing about its
//! effect surface is uncertain (fail-closed). The seed builtin set is entirely
//! pure, so today's table declares the effect vocabulary
//!
//!   file-read | file-write | host-call | import | network
//!
//! as the boundary future host builtins must enter through — no impure builtin
//! exists yet, and that is stated rather than implied.
//!
//! Witnesses: deterministic content-hashed records (SHA-256) so eval / mirror /
//! IR conversions leave a verifiable trail. The field schema is shared with the
//! sibling lanes and must not drift:
//!
//!   direction source_lang target_lang input_kind output_kind loss_status
//!   effect_class capability_required in_hash out_hash env_hash status loss

use crate::ir;
use crate::mirror;
use crate::px;
use crate::sha256::sha256_hex;

pub const GATE_SCHEMA: &str = "pnix-rs.gate-check.v0";
pub const WITNESS_SCHEMA: &str = "pnix-rs.witness.v0";

/// The declared effect vocabulary (pnix-hy EFFECT_CLASSES boundary).
pub const EFFECT_CLASSES: [&str; 5] =
    ["file-read", "file-write", "host-call", "import", "network"];

/// impure builtin name -> effect class. Pure builtins return None. Host-touching
/// builtins declare their effect class so gate admission can require grants.
pub fn effect_of(builtin: &str) -> Option<&'static str> {
    if builtin == "readFile" || builtin == "readDir" || builtin == "pathExists" {
        Some("file-read")
    } else if builtin == "toFile" {
        Some("file-write")
    } else if builtin == "fetchurl" || builtin == "fetchTarball" || builtin == "fetchGit" {
        Some("network")
    } else if builtin == "getEnv" {
        // Reads the host process environment (std::env::var) — impure, and
        // closest of the five declared classes to "ask the host for
        // something outside the pure expression" (2026-08-19 tranche).
        Some("host-call")
    } else {
        None
    }
}

pub struct GateRecord {
    pub schema: &'static str,
    pub pure: bool,
    pub builtin_uses: Vec<String>,
    pub required_effects: Vec<String>,
    pub uncertain: Vec<String>,
    pub denials: Vec<String>,
    pub allowed: bool,
    pub parse_error: Option<String>,
}

/// Static purity walk: collect `builtins.<name>` uses and anything that makes
/// the effect surface uncertain (unknown builtin names, or `builtins` escaping
/// as a first-class value).
fn purity_walk(e: &px::PxExpr, uses: &mut Vec<String>, uncertain: &mut Vec<String>) {
    match e {
        // An internal deferred import failure has no effect if it remains dead;
        // reaching it deterministically raises a pure evaluation error.
        px::PxExpr::DeferredError(_) => {}
        px::PxExpr::Select { base, name } => {
            if let px::PxExpr::Var(base_name) = base.as_ref() {
                if base_name == "builtins" {
                    let known = px::px_builtin_public_names();
                    if known.iter().any(|k| *k == name.as_str()) {
                        // `builtins.builtins` is a real public value, but it is
                        // the whole table and therefore preserves the existing
                        // fail-closed rule for builtins escaping as a value.
                        if name == "builtins" {
                            uncertain.push(String::from("builtins escapes as a value"));
                        } else {
                            uses.push(name.clone());
                        }
                    } else {
                        uncertain.push(format!("unknown builtin {}", name));
                    }
                    return;
                }
            }
            purity_walk(base, uses, uncertain);
        }
        px::PxExpr::Var(name) => {
            if name == "builtins" {
                uncertain.push(String::from("builtins escapes as a value"));
            }
        }
        px::PxExpr::Int(_)
        | px::PxExpr::Float(_) => {}
        px::PxExpr::Bool(_) => {}
        px::PxExpr::Null => {}
        px::PxExpr::With { scope, body } => {
            purity_walk(scope, uses, uncertain);
            purity_walk(body, uses, uncertain);
        }
        px::PxExpr::Str(parts) => {
            for part in parts {
                if let px::PxStrPart::Sub(sub) = part {
                    purity_walk(sub, uses, uncertain);
                }
            }
        }
        px::PxExpr::List(items) => {
            for item in items {
                purity_walk(item, uses, uncertain);
            }
        }
        px::PxExpr::Lambda { body, .. } => purity_walk(body, uses, uncertain),
        px::PxExpr::Apply { func, arg } => {
            purity_walk(func, uses, uncertain);
            purity_walk(arg, uses, uncertain);
        }
        px::PxExpr::If { cond, then_e, else_e } => {
            purity_walk(cond, uses, uncertain);
            purity_walk(then_e, uses, uncertain);
            purity_walk(else_e, uses, uncertain);
        }
        px::PxExpr::Binary { lhs, rhs, .. } => {
            purity_walk(lhs, uses, uncertain);
            purity_walk(rhs, uses, uncertain);
        }
        px::PxExpr::LetIn { bindings, body } => {
            for (bound, value) in bindings {
                // A local binding named `builtins` shadows the global attrset;
                // uses under it resolve to the binding, which the walk treats
                // conservatively: shadowing keeps the walk sound because the
                // shadowed value itself is walked here.
                let _ = bound;
                purity_walk(value, uses, uncertain);
            }
            purity_walk(body, uses, uncertain);
        }
        px::PxExpr::Attrs(fields) => {
            for (_name, value) in fields {
                purity_walk(value, uses, uncertain);
            }
        }
    }
}

/// Capability-aware admission: classify required effects, admit only when every
/// required effect is granted and nothing is uncertain (fail-closed).
pub fn gate_check(source: &str, granted: &[String]) -> GateRecord {
    let ast = match px::px_parse(source) {
        Ok(a) => a,
        Err(e) => {
            return GateRecord {
                schema: GATE_SCHEMA,
                pure: false,
                builtin_uses: Vec::new(),
                required_effects: Vec::new(),
                uncertain: Vec::new(),
                denials: Vec::new(),
                allowed: false,
                parse_error: Some(e),
            }
        }
    };
    let mut uses = Vec::new();
    let mut uncertain = Vec::new();
    purity_walk(&ast, &mut uses, &mut uncertain);

    let mut required_effects: Vec<String> = Vec::new();
    for u in &uses {
        if let Some(effect) = effect_of(u) {
            if !required_effects.iter().any(|r| r == effect) {
                required_effects.push(String::from(effect));
            }
        }
    }
    let denials: Vec<String> = required_effects
        .iter()
        .filter(|e| !granted.iter().any(|g| g == *e))
        .cloned()
        .collect();
    let pure = required_effects.is_empty() && uncertain.is_empty();
    let allowed = denials.is_empty() && uncertain.is_empty();
    GateRecord {
        schema: GATE_SCHEMA,
        pure,
        builtin_uses: uses,
        required_effects,
        uncertain,
        denials,
        allowed,
        parse_error: None,
    }
}

pub fn render_gate(r: &GateRecord) -> String {
    let mut out = String::new();
    out.push_str(&format!("schema {}\n", r.schema));
    out.push_str(&format!("pure {}\n", r.pure));
    out.push_str(&format!("builtin_uses [{}]\n", r.builtin_uses.join(" ")));
    out.push_str(&format!(
        "required_effects [{}]\n",
        r.required_effects.join(" ")
    ));
    out.push_str(&format!("uncertain [{}]\n", r.uncertain.join("; ")));
    out.push_str(&format!("denials [{}]\n", r.denials.join(" ")));
    out.push_str(&format!("allowed {}\n", r.allowed));
    match &r.parse_error {
        Some(e) => out.push_str(&format!("parse_error {}\n", e)),
        None => {}
    }
    out
}

// ---- witnesses -----------------------------------------------------------------

/// Shared witness field schema (order fixed; do not drift — cross-host P13
/// compares these records with the sibling lanes).
pub const WITNESS_FIELDS: [&str; 13] = [
    "direction",
    "source_lang",
    "target_lang",
    "input_kind",
    "output_kind",
    "loss_status",
    "effect_class",
    "capability_required",
    "in_hash",
    "out_hash",
    "env_hash",
    "status",
    "loss",
];

pub struct Witness {
    pub direction: String,
    pub source_lang: String,
    pub target_lang: String,
    pub input_kind: String,
    pub output_kind: String,
    pub loss_status: String,
    pub effect_class: String,
    pub capability_required: String,
    pub in_hash: String,
    pub out_hash: String,
    pub env_hash: String,
    pub status: String,
    pub loss: String,
}

fn env_hash(granted: &[String]) -> String {
    let mut sorted: Vec<String> = granted.to_vec();
    sorted.sort();
    sha256_hex(format!("granted={}", sorted.join(",")).as_bytes())
}

/// Witness for one pure evaluation: px source -> canonical value.
pub fn eval_witness(source: &str, granted: &[String]) -> Result<Witness, String> {
    let ast = px::px_parse(source)?;
    let env = Vec::new();
    let value = px::px_print(&px::px_eval(&ast, &env)?);
    Ok(Witness {
        direction: String::from("eval"),
        source_lang: String::from("px"),
        target_lang: String::from("px-value"),
        input_kind: String::from("source"),
        output_kind: String::from("canonical-value"),
        loss_status: String::from("lossless"),
        effect_class: String::from("pure"),
        capability_required: String::from("-"),
        in_hash: sha256_hex(source.as_bytes()),
        out_hash: sha256_hex(value.as_bytes()),
        env_hash: env_hash(granted),
        status: String::from("ok"),
        loss: String::from("none"),
    })
}

/// Witness for one mirror roundtrip: px source -> emitted px.
pub fn mirror_witness(source: &str, granted: &[String]) -> Result<Witness, String> {
    let record = mirror::mirror_run(source);
    let emitted = match &record.emitted {
        Some(e) => e.clone(),
        None => return Err(record.error.unwrap_or(String::from("mirror rejected"))),
    };
    Ok(Witness {
        direction: String::from("mirror-roundtrip"),
        source_lang: String::from("px"),
        target_lang: String::from("px"),
        input_kind: String::from("source"),
        output_kind: String::from("emitted-source"),
        loss_status: String::from(record.status),
        effect_class: String::from("pure"),
        capability_required: String::from("-"),
        in_hash: sha256_hex(source.as_bytes()),
        out_hash: sha256_hex(emitted.as_bytes()),
        env_hash: env_hash(granted),
        status: if record.status == "lossless" {
            String::from("ok")
        } else {
            String::from(record.status)
        },
        loss: String::from("none"),
    })
}

/// Witness for one IR derivation: px source -> canonical IR.
pub fn ir_witness(source: &str, granted: &[String]) -> Result<Witness, String> {
    let record = ir::ir_of(source)?;
    Ok(Witness {
        direction: String::from("ir"),
        source_lang: String::from("px"),
        target_lang: String::from("px-ir"),
        input_kind: String::from("source"),
        output_kind: String::from("canonical-ir"),
        loss_status: String::from("lossless"),
        effect_class: String::from("pure"),
        capability_required: String::from("-"),
        in_hash: sha256_hex(source.as_bytes()),
        out_hash: record.ir_sha256,
        env_hash: env_hash(granted),
        status: String::from("ok"),
        loss: String::from("none"),
    })
}

pub fn render_witness(w: &Witness) -> String {
    format!(
        "schema {}\ndirection {}\nsource_lang {}\ntarget_lang {}\ninput_kind {}\noutput_kind {}\nloss_status {}\neffect_class {}\ncapability_required {}\nin_hash {}\nout_hash {}\nenv_hash {}\nstatus {}\nloss {}\n",
        WITNESS_SCHEMA,
        w.direction,
        w.source_lang,
        w.target_lang,
        w.input_kind,
        w.output_kind,
        w.loss_status,
        w.effect_class,
        w.capability_required,
        w.in_hash,
        w.out_hash,
        w.env_hash,
        w.status,
        w.loss
    )
}

// ---- typed attestation (proposal, maps pnix-hy 25-typed-attestation) --------
//
// The 13-field witness is FROZEN and structurally typed, but untyped as an
// ATTESTATION: a consumer can't tell WHAT claim it makes or route/validate it.
// A typed attestation is a separate layer (frozen schema untouched, in-toto/
// SLSA style): a predicate-type URI naming the claim + a subject (the content
// hash the claim is about) + the witness's status. `validate_typed` rejects an
// attestation whose predicate does not match the underlying witness kind
// (direction) or whose subject doesn't match — so you cannot forge "this is a
// lossless-roundtrip attestation" over an eval witness.

pub struct TypedAttestation {
    pub predicate_type: String,
    pub subject: String,
    pub direction: String,
    pub status: String,
    pub attestation_sha256: String,
}

/// The predicate-type URI expected for a witness of the given `direction`.
/// (This is the registry that gives an attestation its meaning.)
pub fn predicate_for(direction: &str) -> Option<&'static str> {
    if direction == "eval" {
        Some("pnix-rs/attest/eval-purity.v0")
    } else if direction == "mirror-roundtrip" {
        Some("pnix-rs/attest/roundtrip-lossless.v0")
    } else if direction == "ir" {
        Some("pnix-rs/attest/ir-content-address.v0")
    } else {
        None
    }
}

/// Build the canonical typed attestation for a witness (predicate derived from
/// the witness's direction; subject = the witness output content hash).
pub fn typed_attestation(w: &Witness) -> Result<TypedAttestation, String> {
    let predicate = predicate_for(&w.direction)
        .ok_or_else(|| format!("no attestation predicate for direction {}", w.direction))?;
    let body = format!(
        "{}\t{}\t{}\t{}",
        predicate, w.out_hash, w.direction, w.status
    );
    Ok(TypedAttestation {
        predicate_type: String::from(predicate),
        subject: w.out_hash.clone(),
        direction: w.direction.clone(),
        status: w.status.clone(),
        attestation_sha256: sha256_hex(body.as_bytes()),
    })
}

/// Validate a typed attestation against a witness: the predicate must be the
/// one registered for the witness's direction, and the subject must match the
/// witness output hash. A mismatched predicate or subject is rejected.
pub fn validate_typed(att: &TypedAttestation, w: &Witness) -> bool {
    match predicate_for(&w.direction) {
        Some(expected) => {
            att.predicate_type == expected
                && att.subject == w.out_hash
                && att.direction == w.direction
        }
        None => false,
    }
}

pub fn render_attestation(att: &TypedAttestation) -> String {
    format!(
        "predicate_type {}\nsubject {}\ndirection {}\nstatus {}\nattestation_sha256 {}",
        att.predicate_type, att.subject, att.direction, att.status, att.attestation_sha256
    )
}
