//! Thin action-checkpoint layer for px semantic actions (P12).
//!
//! This module deliberately owns no evaluator, mirror, gate, or host machinery.
//! It composes the existing surfaces into one verdict record for a single
//! proposed px action: admitted only when the gate admits it AND its mirror
//! roundtrip is lossless.

use crate::gate;
use crate::ir;
use crate::mirror;

pub const ACTION_SCHEMA: &str = "pnix-rs.action.v0";

pub struct ActionVerdict {
    pub schema: &'static str,
    pub gate_allowed: bool,
    pub mirror_status: String,
    pub ir_sha256: String,
    pub witness: gate::Witness,
    pub allowed: bool,
}

pub fn action_check(px_source: &str, granted: &[String]) -> Result<ActionVerdict, String> {
    let gate_record = gate::gate_check(px_source, granted);
    let mirror_record = mirror::mirror_run(px_source);
    let ir_record = ir::ir_of(px_source)?;
    let witness = gate::eval_witness(px_source, granted)?;
    let allowed = gate_record.allowed && mirror_record.status == "lossless";
    Ok(ActionVerdict {
        schema: ACTION_SCHEMA,
        gate_allowed: gate_record.allowed,
        mirror_status: String::from(mirror_record.status),
        ir_sha256: ir_record.ir_sha256,
        witness,
        allowed,
    })
}

pub fn render(v: &ActionVerdict) -> String {
    let mut out = String::new();
    out.push_str(&format!("schema {}\n", v.schema));
    out.push_str(&format!("gate_allowed {}\n", v.gate_allowed));
    out.push_str(&format!("mirror_status {}\n", v.mirror_status));
    out.push_str(&format!("ir_sha256 {}\n", v.ir_sha256));
    out.push_str(&format!("allowed {}\n", v.allowed));
    out.push_str(&gate::render_witness(&v.witness));
    out
}
