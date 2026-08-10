//! rs-meta `bootstrap` CLI.
//!
//! rs-meta is the **Rust meta-circular compiler/evaluator** (stage15-N): written
//! in Rust, it evaluates Rust — directly via an in-Rust interpreter, and via the
//! `rustc` native tier (the Evcxr mechanism). The two are kept equal by
//! translation validation. Standalone; not tied to any other language or project.

mod ast;
mod cap;
mod check;
mod diag;
mod emit;
mod hash;
mod interp;
mod io;
mod lexer;
mod native;
mod parser;
mod sig;
mod typeck;
mod witness;

use native::{default_workdir, native_run};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    let rest = &args[2..];

    match cmd {
        "self-check" => run_report(check::self_check()),
        "tv-check" => run_report(check::tv_check()),
        "typeck-check" => run_report(check::typeck_check()),
        "roundtrip-check" => run_report(check::roundtrip_check()),
        "emit-tv-check" => run_report(check::emit_tv_check()),
        "emit-self-host-check" => run_report(check::emit_self_host_check()),
        "witness-check" => run_report(check::witness_check()),
        "cap-check" => run_report(check::cap_check()),
        "io-check" => {
            if cap::CAP_FS_READ == io::FILE_READ_CAPABILITY && io::io_check() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        "trace-check" => run_report(check::trace_check()),
        "diag-check" => run_report(check::diag_check()),
        "ast-canonical-check" => run_report(check::ast_canonical_check()),
        "ast-diff-check" => run_report(check::ast_diff_check()),
        "rust-ir-check" => run_report(check::rust_ir_check()),
        "borrow-boundary-check" => run_report(check::borrow_boundary_check()),
        "rust-artifact-check" => run_report(check::rust_artifact_check()),
        "rust-surface-check" => run_report(check::rust_surface_check()),
        "tv-stats-check" => run_report(check::tv_stats_check()),
        "fuzz-check" => run_report(check::fuzz_diff_check()),
        "emi-check" => run_report(check::emi_check()),
        "fuzz-corpus-check" => run_report(check::fuzz_corpus_check()),
        "selfhost-audit-check" => run_report(check::selfhost_audit_check()),
        "shrink-check" => run_report(check::shrink_check()),
        "boundary-check" => run_report(check::boundary_check()),
        "trait-boundary-check" => run_report(check::trait_boundary_check()),
        "macro-boundary-check" => run_report(check::macro_boundary_check()),
        "source-ast-check" => run_report(check::source_ast_check()),
        "source-bundle-check" => run_report(check::source_bundle_check()),
        "stage2-chain-check" => run_report(check::stage2_chain_check()),
        "stage2-probe-check" => run_report(check::stage2_probe_check()),
        "stage3-chain-check" => run_report(check::stage3_chain_check()),
        "stage3-all-source-smoke-check" => run_report(check::stage3_all_source_smoke_check()),
        "stage3-core-mini-check" => run_report(check::stage3_core_mini_check()),
        "stage3-core-prefix-check" => run_report(check::stage3_core_prefix_check()),
        "stage3-core-middle-check" => run_report(check::stage3_core_middle_check()),
        "stage3-core-suffix-check" => run_report(check::stage3_core_suffix_check()),
        "stage3-core-feature-check" => run_report(check::stage3_core_feature_check()),
        "stage3-core-negative-check" => run_report(check::stage3_core_negative_check()),
        "stage3-core-negative-middle-check" => {
            run_report(check::stage3_core_negative_middle_check())
        }
        "stage3-core-negative-suffix-check" => {
            run_report(check::stage3_core_negative_suffix_check())
        }
        "stage3-full-chain-check" => run_report(check::stage3_full_chain_check()),
        "stage3-mirror-check" => run_report(check::stage3_mirror_check()),
        "stage3-fixedpoint-check" => run_report(check::stage3_fixedpoint_check()),
        "stage3-full-held-check" => run_report(check::stage3_full_held_check()),
        "stage8-repro-check" => run_report(check::stage8_repro_check()),
        "stage8-selfhost-repro-check" => run_report(check::stage8_selfhost_repro_check()),
        "manifest-check" => run_report(check::manifest_check()),
        "isolation-check" => run_report(check::isolation_check()),
        "constitution-check" => run_report(check::constitution_check()),
        "actions-disabled-check" => run_report(check::actions_disabled_check()),
        "native-cache-check" => run_report(check::native_cache_check()),
        "stage9-replay-check" => run_report(check::stage9_replay_check()),
        "stage9-proof-matrix-check" => run_report(check::stage9_proof_matrix_check()),
        "stage9-aggregate-replay-check" => run_report(check::stage9_aggregate_replay_check()),
        "stage10-session-check" => run_report(check::stage10_session_check()),
        "stage10-sandbox-check" => run_report(check::stage10_sandbox_check()),
        "stage11-adapter-check" => run_report(check::stage11_adapter_check()),
        "stage11-adapter-replay-check" => run_report(check::stage11_adapter_replay_check()),
        "stage12-quarantine-check" => run_report(check::stage12_quarantine_check()),
        "stage12-quarantine-replay-check" => run_report(check::stage12_quarantine_replay_check()),
        "stage13-horizon-check" => run_report(check::stage13_horizon_check()),
        "stage13-horizon-replay-check" => run_report(check::stage13_horizon_replay_check()),
        "stage14-cross-impl-check" => run_report(check::stage14_cross_impl_check()),
        "stage14-cross-impl-replay-check" => run_report(check::stage14_cross_impl_replay_check()),
        "stage15-evidence-check" => run_report(check::stage15_evidence_check()),
        "stage15-evidence-replay-check" => run_report(check::stage15_evidence_replay_check()),
        "stageN-extension-check" => run_report(check::stagen_extension_check()),
        "stageN-extension-replay-check" => run_report(check::stagen_extension_replay_check()),
        "check" => {
            let reports = [
                check::self_check(),
                check::tv_check(),
                check::typeck_check(),
                check::roundtrip_check(),
                check::emit_tv_check(),
                check::emit_self_host_check(),
                check::witness_check(),
                check::cap_check(),
                check::trace_check(),
                check::diag_check(),
                check::ast_canonical_check(),
                check::ast_diff_check(),
                check::rust_ir_check(),
                check::borrow_boundary_check(),
                check::trait_boundary_check(),
                check::macro_boundary_check(),
                check::rust_artifact_check(),
                check::rust_surface_check(),
                check::tv_stats_check(),
                check::fuzz_diff_check(),
                check::emi_check(),
                check::fuzz_corpus_check(),
                check::selfhost_audit_check(),
                check::shrink_check(),
                check::boundary_check(),
                check::source_ast_check(),
                check::source_bundle_check(),
                check::stage2_chain_check(),
                check::stage2_probe_check(),
                check::stage3_chain_check(),
                check::stage3_all_source_smoke_check(),
                check::stage3_core_mini_check(),
                check::stage3_core_prefix_check(),
                check::stage3_core_middle_check(),
                check::stage3_core_suffix_check(),
                check::stage3_core_feature_check(),
                check::stage3_core_negative_check(),
                check::stage3_core_negative_middle_check(),
                check::stage3_core_negative_suffix_check(),
                check::stage3_mirror_check(),
                check::stage3_fixedpoint_check(),
                check::stage3_full_held_check(),
                check::stage8_repro_check(),
                check::stage8_selfhost_repro_check(),
                check::manifest_check(),
                check::isolation_check(),
                check::constitution_check(),
                check::actions_disabled_check(),
                check::native_cache_check(),
                check::stage9_replay_check(),
                check::stage9_proof_matrix_check(),
                check::stage10_session_check(),
                check::stage10_sandbox_check(),
                check::stage11_adapter_check(),
                check::stage11_adapter_replay_check(),
                check::stage12_quarantine_check(),
                check::stage12_quarantine_replay_check(),
                check::stage13_horizon_check(),
                check::stage13_horizon_replay_check(),
                check::stage14_cross_impl_check(),
                check::stage14_cross_impl_replay_check(),
                check::stage15_evidence_check(),
                check::stage15_evidence_replay_check(),
                check::stagen_extension_check(),
                check::stagen_extension_replay_check(),
            ];
            let mut all_green = true;
            for r in &reports {
                r.print();
                all_green &= r.green();
            }
            all_green &= cap::CAP_FS_READ == io::FILE_READ_CAPABILITY && io::io_check();
            if std::env::var("RSMETA_SKIP_STAGE9_AGGREGATE").is_err() {
                let r = check::stage9_aggregate_replay_check();
                r.print();
                all_green &= r.green();
            }
            if all_green {
                println!("\ncheck: PASS");
                ExitCode::SUCCESS
            } else {
                println!("\ncheck: FAIL");
                ExitCode::FAILURE
            }
        }
        "stage-status" => {
            check::stage_status();
            ExitCode::SUCCESS
        }
        "run" => cmd_run(rest, Backend::Interp),
        "native-run" => cmd_run(rest, Backend::Native),
        "ast" => cmd_ast(rest),
        "ast-canonical" => cmd_ast_canonical(rest),
        "rust-ir" => cmd_rust_ir(rest),
        "rust-artifact" => cmd_rust_artifact(rest),
        "rust-surface" => cmd_rust_surface(rest),
        "tv-stats" => cmd_tv_stats(),
        "fuzz-gen" => cmd_fuzz_gen(rest),
        "fuzz-mint" => cmd_fuzz_mint(rest),
        "fuzz-scale" => cmd_fuzz_scale(rest),
        "typecheck" => cmd_typecheck(rest),
        "witness" => cmd_witness(rest),
        "trace-run" => cmd_trace_run(rest),
        "emit" => cmd_emit(rest),
        "help" | "-h" | "--help" => {
            print_help();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("rs-meta: unknown command: {}\n", other);
            print_help();
            ExitCode::FAILURE
        }
    }
}

enum Backend {
    Interp,
    Native,
}

fn run_report(r: check::Report) -> ExitCode {
    r.print();
    if r.green() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// `run -c "<rust>"` | `run -f <file.rs>` | `native-run ...`
fn cmd_run(rest: &[String], backend: Backend) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rs-meta: {}", e);
            return ExitCode::FAILURE;
        }
    };
    match backend {
        Backend::Interp => match check::interp_run(&src) {
            Ok(out) => {
                print!("{}", out);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("rs-meta: {}", diag::render_error(&src, &e));
                ExitCode::FAILURE
            }
        },
        Backend::Native => match native_run(&src, &default_workdir()) {
            Ok(out) => {
                print!("{}", out);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("rs-meta: {}", diag::render_error(&src, &e));
                ExitCode::FAILURE
            }
        },
    }
}

/// `ast-canonical -c|-f` — print the stable canonical AST serialization
/// (machine-parseable; the same rendering the stage3 mirror proves
/// byte-identical across evaluation levels).
fn cmd_ast_canonical(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rs-meta: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let result = (|| -> Result<String, String> {
        let toks = lexer::lex(&src)?;
        let prog = parser::parse_program(&toks)?;
        Ok(sig::sig_program(&prog))
    })();
    match result {
        Ok(out) => {
            println!("{}", out);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rs-meta: {}", diag::render_error(&src, &e));
            ExitCode::FAILURE
        }
    }
}

/// `rust-ir -c|-f` — content-addressed canonical Rust IR. Emits the mirror-
/// proven ast-canonical serialization + a stable, format-invariant ir_hash + an
/// `evaluable` flag (the IR re-emits to parseable Rust). Consumed by the pnix-rs
/// peer-engine adapter to fill the verdict ir_hash. rs-meta stays pnix-free.
fn cmd_rust_ir(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rs-meta: {}", e);
            return ExitCode::FAILURE;
        }
    };
    match check::rust_ir_of(&src) {
        Ok((canonical, ir_hash, evaluable)) => {
            println!("schema rs-meta.rust-ir.v0");
            println!("ir {}", canonical);
            println!("ir_hash {}", ir_hash);
            println!("evaluable {}", evaluable);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rs-meta: {}", diag::render_error(&src, &e));
            ExitCode::FAILURE
        }
    }
}

/// `fuzz-gen <seed>` — emit one deterministic, well-defined generated Rust
/// program (for inspection or minting a divergence into the corpus).
fn cmd_fuzz_gen(rest: &[String]) -> ExitCode {
    // parse as i64 (interp turbofish supports i64/f64, not u64) then cast, so
    // rs-meta can interpret its OWN source (source-bundle self-host constraint).
    let seed = rest
        .get(0)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(1) as u64;
    println!("{}", check::fuzz_gen(seed));
    ExitCode::SUCCESS
}

/// `fuzz-scale <n>` — deep differential search over n generated programs.
fn cmd_fuzz_scale(rest: &[String]) -> ExitCode {
    let n = rest.get(0).and_then(|s| s.parse::<i64>().ok()).unwrap_or(100) as u64;
    let (at, msg) = check::fuzz_scale(n);
    if msg == "no divergence" {
        println!("fuzz-scale: {} programs, interp==rustc (no divergence)", at);
        ExitCode::SUCCESS
    } else {
        println!("{}", msg);
        ExitCode::FAILURE
    }
}

/// `fuzz-mint <n>` — mint verified generated programs into proofs/fuzz-corpus.tsv.
fn cmd_fuzz_mint(rest: &[String]) -> ExitCode {
    let n = rest.get(0).and_then(|s| s.parse::<i64>().ok()).unwrap_or(24) as u64;
    match check::fuzz_mint(n) {
        Ok(kept) => {
            println!("minted {} verified programs -> proofs/fuzz-corpus.tsv", kept);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rs-meta: fuzz-mint: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// `tv-stats` — translation-validation corpus coverage (a trust signal for the
/// pnix-rs peer-engine attestation). rs-meta stays pnix-free.
fn cmd_tv_stats() -> ExitCode {
    let (pos, neg) = check::tv_stats_report();
    println!("schema rs-meta.tv-stats.v0");
    println!("positive_corpus {}", pos);
    println!("negative_corpus {}", neg);
    println!("tv_gate tv-check");
    println!("typeck_gate typeck-check");
    println!("self_hosting {}", check::self_host_gate());
    println!("differential_testing fuzz-check+emi-check+boundary-check");
    ExitCode::SUCCESS
}

/// `rust-surface -c|-f` — per-program trait+macro surface classification
/// (supported vs held), for the pnix-rs peer-engine verdict surface field.
fn cmd_rust_surface(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rs-meta: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let (trait_surface, macro_surface, parses) = check::rust_surface_report(&src);
    println!("schema rs-meta.rust-surface.v0");
    println!("trait_surface {}", trait_surface);
    println!("macro_surface {}", macro_surface);
    println!("parses {}", parses);
    ExitCode::SUCCESS
}

/// `rust-artifact -c|-f` — per-program native artifact receipt (stage8-repro,
/// exposed for the pnix-rs peer-engine artifact envelope). rs-meta stays pnix-free.
fn cmd_rust_artifact(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rs-meta: {}", e);
            return ExitCode::FAILURE;
        }
    };
    match check::rust_artifact_receipt(&src) {
        Ok((receipt, receipt_hash)) => {
            println!("schema rs-meta.rust-artifact.v0");
            print!("{}", receipt);
            println!("receipt_hash {}", receipt_hash);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rs-meta: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// `typecheck -c|-f` — parse + type-check; print `ok: well-typed` or the typeck
/// error. A pnix-free generic surface: the trusted interpreter floor certifies
/// well-typedness, and `typeck-check` proves the floor accepts iff rustc does —
/// so a caller (e.g. pnix-rs) can gate "residual is well-typed by the floor"
/// without invoking the native tier.
fn cmd_typecheck(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rs-meta: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let result = (|| -> Result<(), String> {
        let toks = lexer::lex(&src)?;
        let prog = parser::parse_program(&toks)?;
        typeck::check(&prog)
    })();
    match result {
        Ok(()) => {
            println!("ok: well-typed");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rs-meta: {}", diag::render_error(&src, &e));
            ExitCode::FAILURE
        }
    }
}

/// `trace-run -c|-f` — run under the eval trace and print facets + output.
fn cmd_trace_run(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rs-meta: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let result = (|| -> Result<(Vec<String>, Result<String, String>), String> {
        let toks = lexer::lex(&src)?;
        let prog = parser::parse_program(&toks)?;
        typeck::check(&prog)?;
        let mut interp = interp::Interp::new(&prog)?;
        interp.enable_trace();
        let run = interp.run_main();
        Ok((interp.take_trace(), run))
    })();
    match result {
        Ok((trace, run)) => {
            for line in trace {
                println!("trace\t{}", line);
            }
            match run {
                Ok(out) => {
                    print!("{}", out);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("rs-meta: {}", e);
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("rs-meta: {}", diag::render_error(&src, &e));
            ExitCode::FAILURE
        }
    }
}

/// `witness -c|-f` — print the facet witness records for one program.
fn cmd_witness(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rs-meta: {}", e);
            return ExitCode::FAILURE;
        }
    };
    println!("# schema rs-meta.witness.v0");
    println!("{}", witness::WITNESS_HEADER);
    for w in witness::facet_witnesses("cli", &src) {
        println!("{}", witness::render_witness(&w));
    }
    ExitCode::SUCCESS
}

/// `emit -c|-f` — print the Rust source regenerated from the parsed AST.
fn cmd_emit(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rs-meta: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let result = (|| -> Result<String, String> {
        let toks = lexer::lex(&src)?;
        let prog = parser::parse_program(&toks)?;
        Ok(emit::emit_program(&prog))
    })();
    match result {
        Ok(out) => {
            print!("{}", out);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rs-meta: {}", diag::render_error(&src, &e));
            ExitCode::FAILURE
        }
    }
}

/// `ast -c|-f` — print the parsed AST (debug view of the front-end).
fn cmd_ast(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rs-meta: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let result = (|| -> Result<String, String> {
        let toks = lexer::lex(&src)?;
        let prog = parser::parse_program(&toks)?;
        Ok(format!("{:#?}", prog))
    })();
    match result {
        Ok(dump) => {
            println!("{}", dump);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rs-meta: {}", diag::render_error(&src, &e));
            ExitCode::FAILURE
        }
    }
}

fn load_source(rest: &[String]) -> Result<String, String> {
    match rest.first().map(|s| s.as_str()) {
        Some("-c") => rest
            .get(1)
            .cloned()
            .ok_or_else(|| "-c requires a source string".to_string()),
        Some("-f") => {
            let mut out = String::new();
            let mut i = 0usize;
            while i < rest.len() {
                if rest.get(i).map(|s| s.as_str()) != Some("-f") {
                    return Err(format!("expected -f <file>, got {}", rest[i]));
                }
                let path = rest
                    .get(i + 1)
                    .ok_or_else(|| "-f requires a file path".to_string())?;
                out.push_str(&format!("\n// ---- {} ----\n", path));
                out.push_str(
                    &std::fs::read_to_string(path)
                        .map_err(|e| format!("cannot read {}: {}", path, e))?,
                );
                out.push('\n');
                i += 2;
            }
            Ok(out)
        }
        Some(other) => Err(format!("expected -c or -f, got {}", other)),
        None => Err("expected -c <src> or -f <file>".to_string()),
    }
}

fn print_help() {
    println!(
        "rs-meta bootstrap — Rust meta-circular compiler/evaluator (stage15-N)\n\
\n\
USAGE:\n\
  bootstrap <command> [args]\n\
\n\
PROOF COMMANDS:\n\
  self-check          interpreter runs the corpus; output matches expected\n\
  tv-check            interpreter stdout == rustc stdout (translation validation)\n\
  typeck-check        interpreter rejects a program iff rustc rejects it\n\
  roundtrip-check     parse -> emit -> reparse AST identity + interp(emit) parity\n\
  emit-tv-check       rustc(emit(parse(src))) == expected (emitted-source parity)\n\
  source-ast-check    rs-meta src/*.rs parses under the rs-meta front-end\n\
  source-bundle-check all-source bundle stdout matches rustc\n\
  stage2-chain-check  all-source evaluator' replays positive corpus and matches rustc\n\
  stage2-probe-check  lexer/parser/typeck/interp source slices agree with rustc\n\
  stage3-chain-check  slim evaluator stage2 -> stage2' chain agrees with rustc\n\
  stage3-all-source-smoke-check slimmed evaluator-core stage2 -> stage2' smoke\n\
  stage3-core-mini-check evaluator-core stage2' mini-corpus replay\n\
  stage3-core-prefix-check evaluator-core stage2' positive corpus prefix replay\n\
  stage3-core-middle-check evaluator-core stage2' positive corpus middle replay\n\
  stage3-core-suffix-check evaluator-core stage2' positive corpus suffix replay\n\
  stage3-core-feature-check evaluator-core stage2' named feature corpus replay\n\
  stage3-core-negative-check evaluator-core stage2' named negative corpus rejection\n\
  stage3-core-negative-middle-check evaluator-core stage2' negative corpus middle rejection\n\
  stage3-core-negative-suffix-check evaluator-core stage2' negative corpus suffix rejection\n\
  stage3-mirror-check stage1/stage2/stage2' canonical AST + output mirror\n\
  stage3-fixedpoint-check stage2 (B) == stage2' (C) evaluator transcript fixed point\n\
  stage3-full-chain-check all-source evaluator stage2' full-chain replay (budgeted)\n\
  stage3-full-held-check full all-source stage3 boundary matches manifest\n\
  stage8-repro-check  same Rust source yields same native artifact receipt\n\
  stage8-selfhost-repro-check stage2 evaluator artifact reproducibility\n\
  manifest-check      validate proofs/stage-manifest.tsv\n\
  isolation-check     fresh interpreter runs do not leak state\n\
  constitution-check  zero-dep/local-only/determinism guard\n\
  actions-disabled-check GitHub Actions disabled; local verification only\n\
  native-cache-check  native rustc compile cache probe\n\
  stage9-replay-check clean-process bootstrap help replay seed\n\
  stage9-proof-matrix-check clean-process proof command matrix\n\
  stage9-aggregate-replay-check bounded proof aggregate replay\n\
  stage10-session-check deterministic clean-process session replay seed\n\
  stage10-sandbox-check client/server/session/sandbox closure\n\
  stage11-adapter-check adapter schema/held/conflict seed\n\
  stage11-adapter-replay-check multi-domain adapter replay closure\n\
  stage12-quarantine-check self-improvement quarantine seed\n\
  stage12-quarantine-replay-check quarantine replay closure\n\
  stage13-horizon-check long-horizon stale/boundary seed\n\
  stage13-horizon-replay-check long-horizon organism replay closure\n\
  stage14-cross-impl-check cross-implementation export seed\n\
  stage14-cross-impl-replay-check cross-implementation replay closure\n\
  stage15-evidence-check open-world evidence federation seed\n\
  stage15-evidence-replay-check open-world evidence replay closure\n\
  stageN-extension-check versioned constitutional extension seed\n\
  stageN-extension-replay-check versioned extension replay closure\n\
  check               self/tv/typeck/source/stage2/stage3/stage8/manifest/isolation/constitution/actions/cache/stage9/stage10/stage11/stage12/stage13/stage14/stage15/stageN checks\n\
  stage-status        print the stage0..stageN ladder + honest status\n\
\n\
EVAL COMMANDS:\n\
  run -c \"<rust>\"      evaluate Rust source with the in-Rust interpreter\n\
  run -f <file.rs>     evaluate Rust file(s); repeat -f for ordered multi-file load\n\
  native-run -c|-f     evaluate the same Rust via rustc (the Evcxr mechanism)\n\
  ast -c|-f            print the parsed AST (rustc-derive Debug; no stability promise)\n\
  ast-canonical -c|-f  print the stable canonical AST serialization\n\
  typecheck -c|-f      certify well-typedness (floor typeck; ok: well-typed | error)\n\
  emit -c|-f           print Rust source regenerated from the parsed AST\n\
\n\
EXAMPLES:\n\
  bootstrap run -c 'fn main() {{ println!(\"{{}}\", 1 + 2 * 3); }}'\n\
  bootstrap native-run -f samples/factorial.rs\n\
  bootstrap check"
    );
}
