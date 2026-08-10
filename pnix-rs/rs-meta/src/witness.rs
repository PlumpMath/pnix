//! Witness records (Plan Phase E2): one record per FACET of a program's
//! journey through the evaluator — source, tokens, AST (the mirror-proven
//! canonical serialization), emit, value (interpreter stdout). Each record is
//! kind/stage/input_hash/output_hash/status/error_kind; rendering is a stable
//! TSV. The drift check (check.rs) recomputes the corpus witness table twice
//! and requires byte-identical reports — hash mismatches between runs become
//! machine-readable drift instead of silence.
//!
//! The native facet (rustc artifacts) is deliberately NOT part of the default
//! witness table: artifact receipts are stage8's organ (cost + separation);
//! the witness table stays interpreter-cheap.

use crate::emit;
use crate::hash;
use crate::interp::Interp;
use crate::lexer::lex;
use crate::parser::parse_program;
use crate::sig;
use crate::typeck;

pub struct Witness {
    pub kind: String,
    pub stage: String,
    pub input_hash: String,
    pub output_hash: String,
    pub status: String,
    pub error_kind: String,
}

/// Field order of the TSV rendering (schema rs-meta.witness.v0).
pub const WITNESS_HEADER: &str = "kind\tstage\tinput_hash\toutput_hash\tstatus\terror_kind";

fn witness_ok(kind: &str, stage: &str, input: &str, output: &str) -> Witness {
    Witness {
        kind: String::from(kind),
        stage: String::from(stage),
        input_hash: hash::text_hash_hex(input),
        output_hash: hash::text_hash_hex(output),
        status: String::from("ok"),
        error_kind: String::from("-"),
    }
}

fn witness_err(kind: &str, stage: &str, input: &str, error_kind: &str) -> Witness {
    Witness {
        kind: String::from(kind),
        stage: String::from(stage),
        input_hash: hash::text_hash_hex(input),
        output_hash: String::from("-"),
        status: String::from("error"),
        error_kind: String::from(error_kind),
    }
}

/// Facet witnesses for one program. Stops at the first failing facet (later
/// facets are unreachable by construction) but the failure itself is a
/// record, never a silent hole.
pub fn facet_witnesses(name: &str, src: &str) -> Vec<Witness> {
    let mut out = Vec::new();
    out.push(witness_ok("source", name, src, src));

    let toks = match lex(src) {
        Ok(t) => t,
        Err(_) => {
            out.push(witness_err("tokens", name, src, "lex"));
            return out;
        }
    };
    let mut tok_text = String::new();
    for t in &toks {
        tok_text.push_str(&format!("{:?};", t));
    }
    out.push(witness_ok("tokens", name, src, &tok_text));

    let prog = match parse_program(&toks) {
        Ok(p) => p,
        Err(_) => {
            out.push(witness_err("ast", name, &tok_text, "parse"));
            return out;
        }
    };
    let sig_text = sig::sig_program(&prog);
    out.push(witness_ok("ast", name, &tok_text, &sig_text));

    let emitted = emit::emit_program(&prog);
    out.push(witness_ok("emit", name, &sig_text, &emitted));

    if let Err(_) = typeck::check(&prog) {
        out.push(witness_err("value", name, &sig_text, "typeck"));
        return out;
    }
    let interp = match Interp::new(&prog) {
        Ok(i) => i,
        Err(_) => {
            out.push(witness_err("value", name, &sig_text, "eval"));
            return out;
        }
    };
    let value = match interp.run_main() {
        Ok(v) => v,
        Err(_) => {
            out.push(witness_err("value", name, &sig_text, "eval"));
            return out;
        }
    };
    out.push(witness_ok("value", name, &sig_text, &value));
    out
}

pub fn render_witness(w: &Witness) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        w.kind, w.stage, w.input_hash, w.output_hash, w.status, w.error_kind
    )
}

/// The whole witness table for a named program list, as a stable TSV report.
pub fn witness_report(programs: &Vec<(String, String)>) -> String {
    let mut out = String::from("# schema rs-meta.witness.v0\n");
    out.push_str(WITNESS_HEADER);
    out.push('\n');
    for (name, src) in programs {
        for w in facet_witnesses(name, src) {
            out.push_str(&render_witness(&w));
            out.push('\n');
        }
    }
    out
}
