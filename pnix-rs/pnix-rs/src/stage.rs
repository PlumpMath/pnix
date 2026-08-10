//! pnix runtime stage ladder for the pnix-rs lane (P2).
//!
//! Distinct from rs-meta's stage ladder, which proves the *host* (Rust)
//! compiler/evaluator. These stages prove the *pnix runtime* itself is stable:
//! a program's value survives normalization, content-addressed store
//! evaluation, AST roundtrip, and deterministic replay — all through the one
//! sacred runtime in `px.rs` (no second evaluator).
//!
//!   px-stage1  direct eval
//!   px-stage2  parse -> normalized AST -> eval
//!   px-stage3  content-addressed store-backed eval
//!   px-stage4  AST roundtrip integrity (parse -> emit -> reparse, hash-stable)
//!   px-stage5  deterministic replay (fresh second run, identical hashes)
//!   closure    every stage produced the same value hash

use crate::mirror::fnv64;
use crate::px;

pub const STAGE_SCHEMA: &str = "pnix-rs.stage.v0";

pub struct StageResult {
    pub name: &'static str,
    pub ok: bool,
    pub value_fnv: u64,
    pub note: String,
}

pub struct StageLadder {
    pub schema: &'static str,
    pub source_fnv: u64,
    pub stages: Vec<StageResult>,
    pub closure: bool,
    pub error: Option<String>,
}

fn eval_print(ast: &px::PxExpr) -> Result<String, String> {
    let env = Vec::new();
    let v = px::px_eval(ast, &env)?;
    Ok(px::px_print(&v))
}

/// Run the full runtime stage ladder for one source.
pub fn stage_run(source: &str) -> StageLadder {
    let source_fnv = fnv64(source);
    let mut stages: Vec<StageResult> = Vec::new();

    let ast = match px::px_parse(source) {
        Ok(a) => a,
        Err(e) => {
            return StageLadder {
                schema: STAGE_SCHEMA,
                source_fnv,
                stages,
                closure: false,
                error: Some(format!("parse: {}", e)),
            }
        }
    };

    // px-stage1: direct eval.
    let stage1_value = match eval_print(&ast) {
        Ok(v) => v,
        Err(e) => {
            return StageLadder {
                schema: STAGE_SCHEMA,
                source_fnv,
                stages,
                closure: false,
                error: Some(format!("px-stage1 eval: {}", e)),
            }
        }
    };
    let expected_fnv = fnv64(&stage1_value);
    stages.push(StageResult {
        name: "px-stage1-direct",
        ok: true,
        value_fnv: expected_fnv,
        note: String::from("direct eval"),
    });

    // px-stage2: normalized AST eval.
    let normalized = px::px_normalize(&ast);
    match eval_print(&normalized) {
        Ok(v) => {
            let h = fnv64(&v);
            stages.push(StageResult {
                name: "px-stage2-normalized",
                ok: h == expected_fnv,
                value_fnv: h,
                note: String::from("eval of normalized AST"),
            });
        }
        Err(e) => stages.push(StageResult {
            name: "px-stage2-normalized",
            ok: false,
            value_fnv: 0,
            note: format!("eval failed: {}", e),
        }),
    }

    // px-stage3: content-addressed store eval. The normalized emission is the
    // stored artifact; it is fetched back by content hash and evaluated.
    let stored = px::px_emit(&normalized);
    let store_key = fnv64(&stored);
    let mut store: Vec<(u64, String)> = Vec::new();
    store.push((store_key, stored));
    let mut fetched: Option<String> = None;
    for (key, text) in &store {
        if *key == store_key {
            fetched = Some(text.clone());
        }
    }
    match fetched {
        Some(text) => match px::px_parse(&text) {
            Ok(a) => match eval_print(&a) {
                Ok(v) => {
                    let h = fnv64(&v);
                    stages.push(StageResult {
                        name: "px-stage3-store",
                        ok: h == expected_fnv,
                        value_fnv: h,
                        note: format!("store key {:016x}", store_key),
                    });
                }
                Err(e) => stages.push(StageResult {
                    name: "px-stage3-store",
                    ok: false,
                    value_fnv: 0,
                    note: format!("store eval failed: {}", e),
                }),
            },
            Err(e) => stages.push(StageResult {
                name: "px-stage3-store",
                ok: false,
                value_fnv: 0,
                note: format!("store parse failed: {}", e),
            }),
        },
        None => stages.push(StageResult {
            name: "px-stage3-store",
            ok: false,
            value_fnv: 0,
            note: String::from("store fetch failed"),
        }),
    }

    // px-stage4: AST roundtrip integrity (hash-stable emission).
    let emitted = px::px_emit(&ast);
    let stage4 = match px::px_parse(&emitted) {
        Ok(reparsed) => {
            let stable = px::px_emit(&reparsed) == emitted;
            let value_ok = match eval_print(&reparsed) {
                Ok(v) => fnv64(&v) == expected_fnv,
                Err(_) => false,
            };
            StageResult {
                name: "px-stage4-roundtrip",
                ok: stable && value_ok,
                value_fnv: fnv64(&emitted),
                note: format!("emit_fixed_point {} value_match {}", stable, value_ok),
            }
        }
        Err(e) => StageResult {
            name: "px-stage4-roundtrip",
            ok: false,
            value_fnv: 0,
            note: format!("reparse failed: {}", e),
        },
    };
    stages.push(stage4);

    // px-stage5: deterministic replay — a fresh second pass over stages 1-4
    // must reproduce the same hashes.
    let replay_ok = match px::px_parse(source) {
        Ok(a2) => {
            let d = match eval_print(&a2) {
                Ok(v) => fnv64(&v) == expected_fnv,
                Err(_) => false,
            };
            let n2 = px::px_normalize(&a2);
            let n_ok = match eval_print(&n2) {
                Ok(v) => fnv64(&v) == expected_fnv,
                Err(_) => false,
            };
            let e_ok = fnv64(&px::px_emit(&a2)) == fnv64(&emitted);
            d && n_ok && e_ok
        }
        Err(_) => false,
    };
    stages.push(StageResult {
        name: "px-stage5-replay",
        ok: replay_ok,
        value_fnv: expected_fnv,
        note: String::from("fresh second run reproduces all hashes"),
    });

    let mut closure = true;
    for s in &stages {
        if !s.ok {
            closure = false;
        }
    }
    StageLadder {
        schema: STAGE_SCHEMA,
        source_fnv,
        stages,
        closure,
        error: None,
    }
}

/// Stable text receipt of a stage ladder (one stage per line).
pub fn render(ladder: &StageLadder) -> String {
    let mut out = String::new();
    out.push_str(&format!("schema {}\n", ladder.schema));
    out.push_str(&format!("source_fnv {:016x}\n", ladder.source_fnv));
    for s in &ladder.stages {
        out.push_str(&format!(
            "{} {} value_fnv={:016x} ({})\n",
            s.name,
            if s.ok { "ok" } else { "FAIL" },
            s.value_fnv,
            s.note
        ));
    }
    out.push_str(&format!("closure {}\n", ladder.closure));
    match &ladder.error {
        Some(e) => out.push_str(&format!("error {}\n", e)),
        None => {}
    }
    out
}
