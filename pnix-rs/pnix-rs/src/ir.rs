//! The explicit pnix IR (canonical runtime representation) layer (P3).
//!
//! px is a small core functional language, so its surface AST is already close
//! to a core IR: the IR here is the NORMALIZED (position-free, structurally
//! canonical) AST — and crucially it is DIRECTLY EVALUABLE (`px_eval` runs it
//! unchanged) and value-equivalent to evaluating the source, so it is a genuine
//! runtime representation, not just a relabeled AST.
//!
//! Key principle (bolted; same line as rs-meta's IR-vs-cache constitution):
//! **pnix IR is the canonical pnix representation; host artifacts (rs-meta
//! emission, rustc binaries) are execution artifacts/caches, NOT the IR.**
//! The IR is content-addressed (`ir_sha256` over the canonical IR text) so
//! identical programs — including programs that differ only in binding order —
//! share one IR identity.

use crate::mirror::fnv64;
use crate::px;
use crate::sha256;

pub const IR_SCHEMA: &str = "pnix-rs.ir.v0";

pub struct IrRecord {
    pub schema: &'static str,
    pub source_fnv: u64,
    /// Canonical IR text: the emission of the normalized AST.
    pub ir_text: String,
    pub ir_sha256: String,
    /// IR canonical fixed point: normalize(reparse(ir_text)) emits ir_text.
    pub canonical_fixed_point: bool,
    /// The IR is directly evaluable and value-equivalent to the source.
    pub direct_eval_value: String,
    pub value_matches_source: bool,
}

pub fn ir_of(source: &str) -> Result<IrRecord, String> {
    let ast = px::px_parse(source)?;
    let env = Vec::new();
    let source_value = px::px_print(&px::px_eval(&ast, &env)?);

    let ir = px::px_normalize(&ast);
    let ir_text = px::px_emit(&ir);
    let ir_sha256 = sha256::sha256_hex(ir_text.as_bytes());

    // Directly evaluable: the IR itself runs through the sacred runtime.
    let ir_value = px::px_print(&px::px_eval(&ir, &env)?);
    let value_matches_source = ir_value == source_value;

    // Canonical fixed point: the IR text re-enters as exactly itself.
    let reparsed = px::px_parse(&ir_text)?;
    let renormalized = px::px_normalize(&reparsed);
    let canonical_fixed_point = px::px_emit(&renormalized) == ir_text;

    Ok(IrRecord {
        schema: IR_SCHEMA,
        source_fnv: fnv64(source),
        ir_text,
        ir_sha256,
        canonical_fixed_point,
        direct_eval_value: ir_value,
        value_matches_source,
    })
}

pub fn render(record: &IrRecord) -> String {
    let mut out = String::new();
    out.push_str(&format!("schema {}\n", record.schema));
    out.push_str(&format!("source_fnv {:016x}\n", record.source_fnv));
    out.push_str(&format!("ir {}\n", record.ir_text));
    out.push_str(&format!("ir_sha256 {}\n", record.ir_sha256));
    out.push_str(&format!(
        "canonical_fixed_point {}\n",
        record.canonical_fixed_point
    ));
    out.push_str(&format!("value {}\n", record.direct_eval_value));
    out.push_str(&format!(
        "value_matches_source {}\n",
        record.value_matches_source
    ));
    out.push_str("principle IR-is-canonical; host artifacts are cache\n");
    out
}

/// IR diff (maps pnix-hy 29): a SEMANTIC diff between two programs at the
/// canonical-IR level. Two programs with the same canonical IR are
/// meaning-equivalent up to normalization (binding reorder, literal merge) —
/// so an edit that leaves the IR unchanged is meaning-preserving, and one that
/// changes it is localized to the first differing IR position. Complements the
/// definition-granular `changed_between` (which is alpha-invariant) with a
/// within-program structural view.
pub struct IrDiff {
    pub identical: bool,
    pub a_ir: String,
    pub b_ir: String,
    /// Char offset of the first difference (a_ir.len() if b is a prefix, etc).
    pub first_diff: usize,
    /// A short window around the first difference (for localization).
    pub window: String,
}

pub fn ir_diff(source_a: &str, source_b: &str) -> Result<IrDiff, String> {
    let a = ir_of(source_a)?.ir_text;
    let b = ir_of(source_b)?.ir_text;
    if a == b {
        return Ok(IrDiff {
            identical: true,
            a_ir: a,
            b_ir: b,
            first_diff: 0,
            window: String::new(),
        });
    }
    let ca: Vec<char> = a.chars().collect();
    let cb: Vec<char> = b.chars().collect();
    let mut i = 0usize;
    while i < ca.len() && i < cb.len() && ca[i] == cb[i] {
        i += 1;
    }
    // window: up to 20 chars of each side from the first difference.
    let mut wa = String::new();
    let mut k = i;
    while k < ca.len() && k < i + 20 {
        wa.push(ca[k]);
        k += 1;
    }
    let mut wb = String::new();
    let mut k = i;
    while k < cb.len() && k < i + 20 {
        wb.push(cb[k]);
        k += 1;
    }
    Ok(IrDiff {
        identical: false,
        a_ir: a,
        b_ir: b,
        first_diff: i,
        window: format!("a:`{}` | b:`{}`", wa, wb),
    })
}
