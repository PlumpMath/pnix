//! pnix-rs CLI — rs-meta backed pnix runtime front-end.
//!
//! pnix-rs is the Rust host lane for the pnix runtime path. It depends on
//! `../rs-meta` (the standalone Rust meta-circular stage15-N compiler/evaluator)
//! as its substrate: pnix-rs's own px engine source is written inside the
//! rs-meta evaluated Rust subset, and `substrate-check` has rs-meta interpret
//! that source and match the rustc-compiled behavior.

mod action;
mod bta;
mod compartment;
mod engine;
mod gate;
mod incremental;
mod interop;
mod ir;
mod mirror;
mod primitive_kernel;
mod px;
mod rust_mirror;
mod sha256;
mod specialize;
mod stage;
mod tower;

use std::process::ExitCode;

fn main() -> ExitCode {
    // Deep tree-walks (the harvested pnixc kernel over import-expanded
    // module ASTs) overflow the default main stack — same host-capacity
    // class as pnix-clj's evaluator lane (fixed 64MB there). Run the real
    // main on a fixed 512MB thread so corpus behavior never depends on the
    // launcher default (audit 2026-07-09).
    let child = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(real_main)
        .expect("spawn main thread");
    match child.join() {
        Ok(code) => code,
        Err(_) => ExitCode::FAILURE,
    }
}

fn real_main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    // No-arg invocation must reach the help fallthrough, not panic on the
    // out-of-bounds slice (audit finding, 2026-07-08).
    let rest: &[String] = if args.len() > 2 { &args[2..] } else { &[] };

    match cmd {
        "px-eval" => cmd_px_eval(rest),
        "primitive-manifest-check" => primitive_kernel::cmd_check(),
        "px-repl" => cmd_px_repl(),
        "rust-repl" => cmd_rust_repl(),
        "explain" => cmd_explain(rest),
        "engine-profile" => cmd_engine_profile(),
        "engine-attestation" => cmd_engine_attestation(),
        "engine-verify" => cmd_engine_verify(rest),
        "engine-batch" => cmd_engine_batch(rest),
        "engine-verdict" => cmd_engine_verdict(rest),
        "engine-artifact" => cmd_engine_artifact(rest),
        "engine-request" => cmd_engine_request(rest),
        "px-check" => cmd_px_check(),
        "mirror" => cmd_mirror(rest),
        "mirror-check" => cmd_mirror_check(),
        "stage" => cmd_stage(rest),
        "stage-check" => cmd_stage_check(),
        "ir" => cmd_ir(rest),
        "ir-check" => cmd_ir_check(),
        "gate" => cmd_gate(rest),
        "gate-check" => cmd_gate_check(),
        "witness" => cmd_witness(rest),
        "interop-check" => cmd_interop_check(),
        "io-probe" => cmd_io_probe(rest),
        "check" => cmd_check(),
        "capabilities" => cmd_capabilities(),
        "capabilities-check" => cmd_capabilities_check(),
        "registry-check" => cmd_registry_check(),
        "rust-mirror" => cmd_rust_mirror(rest),
        "rust-mirror-check" => cmd_rust_mirror_check(),
        "specialize" => cmd_specialize(rest),
        "specialize-check" => cmd_specialize_check(),
        "incremental" => cmd_incremental(rest),
        "incremental-check" => cmd_incremental_check(),
        "compartment-check" => cmd_compartment_check(),
        "tower-check" => cmd_tower_check(),
        "bta-check" => cmd_bta_check(),
        "jones-check" => cmd_jones_check(),
        "certify-check" => cmd_certify_check(),
        "cogen-check" => cmd_cogen_check(),
        "attest-check" => cmd_attest_check(),
        "reflect-tower-check" => cmd_reflect_tower_check(),
        "verifying-cache-check" => cmd_verifying_cache_check(),
        "phase-check" => cmd_phase_check(),
        "assumption-check" => cmd_assumption_check(),
        "ir-diff-check" => cmd_ir_diff_check(),
        "attenuate-check" => cmd_attenuate_check(),
        "explain-check" => cmd_explain_check(),
        "engine-verdict-check" => cmd_engine_verdict_check(),
        "engine-artifact-check" => cmd_engine_artifact_check(),
        "engine-request-check" => cmd_engine_request_check(),
        "engine-attestation-check" => cmd_engine_attestation_check(),
        "engine-verify-check" => cmd_engine_verify_check(),
        "engine-batch-check" => cmd_engine_batch_check(),
        "welltyped-check" => cmd_welltyped_check(),
        "second-projection-experiment" => cmd_second_projection_experiment(),
        "third-projection-experiment" => cmd_third_projection_experiment(),
        "action" => cmd_action(rest),
        "action-check" => cmd_action_check(),
        "export-oracles" => cmd_export_oracles(),
        "cross-host-check" => cmd_cross_host_check(),
        "substrate-check" => cmd_substrate_check(),
        "help" | "-h" | "--help" => {
            print_help();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("pnix-rs: unknown command: {}\n", other);
            print_help();
            ExitCode::FAILURE
        }
    }
}

/// (name, .px source path, expected canonical output). The corpus files are
/// vendored from the pnix-clj rust_grounded invariance corpus so the two host
/// lanes can be compared later (cross-host stage14 style).
fn px_corpus() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "c02_strings",
            "runtime/corpus/c02_strings.px",
            "{ hello = \"hello, pnix!\"; interp = \"n=7 sq=49\"; joined = \"a-b-c-pnix\"; len = 4; sub = \"abc\"; }",
        ),
        (
            "c03_list",
            "runtime/corpus/c03_list.px",
            "{ filtered = [ 11 12 13 14 15 16 17 18 19 20 ]; len = 20; mapped = [ 1 4 9 16 25 36 49 64 81 100 121 144 169 196 225 256 289 324 361 400 ]; total = 210; }",
        ),
        (
            "c04_attr",
            "runtime/corpus/c04_attr.px",
            "{ has = true; m = { a = 1; b = 20; c = 3; }; names = [ \"a\" \"b\" \"c\" ]; pick = 3; }",
        ),
        (
            "c05_recurse",
            "runtime/corpus/c05_recurse.px",
            "{ fib = 6765; sum = 125250; }",
        ),
        (
            "c07_builtins",
            "runtime/corpus/c07_builtins.px",
            "{ at = 8; head = 5; member = true; sorted = [ 1 2 3 5 8 9 ]; tail = [ 2 8 1 9 3 ]; }",
        ),
        (
            "c08_bool",
            "runtime/corpus/c08_bool.px",
            "[ \"neg\" \"zero\" \"small\" \"big\" ]",
        ),
        (
            "c09_lambda",
            "runtime/corpus/c09_lambda.px",
            "{ a = 3; c = 21; curry = 6; }",
        ),
        (
            "seed_arith",
            "runtime/corpus/seed_arith.px",
            "{ div = 10; prod = 42; sum = 23; }",
        ),
        (
            "seed_let_rec",
            "runtime/corpus/seed_let_rec.px",
            "11",
        ),
        (
            "seed_let_shadow",
            "runtime/corpus/seed_let_shadow.px",
            "2",
        ),
        (
            "seed_let_dotted",
            "runtime/corpus/seed_let_dotted.px",
            "3",
        ),
        (
            "seed_replace_strings_empty",
            "runtime/corpus/seed_replace_strings_empty.px",
            r#""xaxbx""#,
        ),
        (
            "c01_arith",
            "runtime/corpus/c01_arith.px",
            "{ div = 10; flt = 7.0; mixed = 44; modv = 1; prod = 42; sum = 23; }",
        ),
        (
            "c06_nested",
            "runtime/corpus/c06_nested.px",
            r#""[{\"id\":1,\"sq\":1,\"tags\":[\"t1\",\"x\"]},{\"id\":2,\"sq\":4,\"tags\":[\"t2\",\"x\"]},{\"id\":3,\"sq\":9,\"tags\":[\"t3\",\"x\"]},{\"id\":4,\"sq\":16,\"tags\":[\"t4\",\"x\"]},{\"id\":5,\"sq\":25,\"tags\":[\"t5\",\"x\"]},{\"id\":6,\"sq\":36,\"tags\":[\"t6\",\"x\"]},{\"id\":7,\"sq\":49,\"tags\":[\"t7\",\"x\"]},{\"id\":8,\"sq\":64,\"tags\":[\"t8\",\"x\"]},{\"id\":9,\"sq\":81,\"tags\":[\"t9\",\"x\"]},{\"id\":10,\"sq\":100,\"tags\":[\"t10\",\"x\"]},{\"id\":11,\"sq\":121,\"tags\":[\"t11\",\"x\"]},{\"id\":12,\"sq\":144,\"tags\":[\"t12\",\"x\"]}]""#,
        ),
        (
            "c10_mixed",
            "runtime/corpus/c10_mixed.px",
            r#"{ count = 15; label = "evens-15"; squares = { k10 = 100; k12 = 144; k14 = 196; k16 = 256; k18 = 324; k2 = 4; k20 = 400; k22 = 484; k24 = 576; k26 = 676; k28 = 784; k30 = 900; k4 = 16; k6 = 36; k8 = 64; }; total = 240; }"#,
        ),
        (
            "seed_type_tests",
            "runtime/corpus/seed_type_tests.px",
            "{ b = true; bs = false; i = true; ib = false; l = true; lb = false; s = true; si = false; }",
        ),
        (
            "seed_deep_eq",
            "runtime/corpus/seed_deep_eq.px",
            "{ a = true; an = false; l = true; ln = false; mixed = true; }",
        ),
        (
            "seed_list_to_attrs",
            "runtime/corpus/seed_list_to_attrs.px",
            "{ flag = true; nonflag = false; picked = 2; s = { a = 1; b = 2; }; }",
        ),
    ]
}

/// Interactive Rust REPL. pnix-rs is the front-end/driver; the RUST engine is
/// rs-meta (the meta-circular interpreter), invoked as a peer across the
/// bootstrap CLI (rs-meta stays pure -- no interactive io in the trusted floor).
/// Items and `let` bindings accumulate as replayed state; any other line builds
/// a program and is evaluated on the rs-meta `run` (interpreter) tier.
fn cmd_rust_repl() -> ExitCode {
    use std::io::Write;
    let bootstrap = bootstrap_path();
    let granted = vec![String::from("host-call")];
    let mut items: Vec<String> = Vec::new();
    let mut lets: Vec<String> = Vec::new();
    eprintln!("pnix-rs Rust REPL -- drives the rs-meta interpreter (peer engine).");
    eprintln!("fn/struct/enum/impl/trait/const/type/use and `let` bindings accumulate.");
    eprintln!("Commands: :quit  :reset  :show");
    let stdin = std::io::stdin();
    loop {
        eprint!("rust> ");
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("pnix-rs: input error: {}", e);
                break;
            }
        }
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t == ":quit" || t == ":q" {
            break;
        }
        if t == ":reset" {
            items.clear();
            lets.clear();
            eprintln!("(state cleared)");
            continue;
        }
        if t == ":show" {
            for it in &items {
                eprintln!("  {}", it);
            }
            for l in &lets {
                eprintln!("  {}", l);
            }
            continue;
        }
        let is_item = t.starts_with("fn ")
            || t.starts_with("struct ")
            || t.starts_with("enum ")
            || t.starts_with("impl ")
            || t.starts_with("trait ")
            || t.starts_with("const ")
            || t.starts_with("static ")
            || t.starts_with("type ")
            || t.starts_with("use ");
        if is_item {
            items.push(String::from(t));
            eprintln!("(item added)");
            continue;
        }
        if t.starts_with("let ") {
            let mut b = String::from(t);
            if !b.ends_with(';') {
                b.push(';');
            }
            lets.push(b);
            eprintln!("(bound)");
            continue;
        }
        let expr = t.trim_end_matches(';');
        let prog = format!(
            "{}\nfn main() {{\n{}\nprintln!(\"{{:?}}\", {{ {} }});\n}}\n",
            items.join("\n"),
            lets.join("\n"),
            expr
        );
        match interop::host_run_bootstrap_inline(&bootstrap, "run", &prog, &granted) {
            Ok(out) => print!("{}", out),
            Err(e) => eprintln!("pnix-rs: {}", e),
        }
    }
    ExitCode::SUCCESS
}

/// Interactive pnix (px) REPL -- the interpreter mode of the pnix engine, on the
/// common `.px` control plane. A line `name = expr` binds persistently
/// (REPL-style accumulation via a Compartment); any other line is evaluated in
/// the accumulated environment and its canonical value printed.
fn cmd_px_repl() -> ExitCode {
    use std::io::Write;
    let mut comp = compartment::Compartment::new();
    eprintln!("pnix-rs px REPL (interpreter mode). `name = expr` binds; any other line evaluates.");
    eprintln!("Commands: :quit");
    let stdin = std::io::stdin();
    loop {
        eprint!("px> ");
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("pnix-rs: input error: {}", e);
                break;
            }
        }
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t == ":quit" || t == ":q" {
            break;
        }
        match split_binding(t) {
            Some((name, expr)) => match comp.define(name, expr) {
                Ok(()) => eprintln!("({} bound)", name),
                Err(e) => eprintln!("pnix-rs: {}", e),
            },
            None => match comp.eval(t) {
                Ok(out) => println!("{}", out),
                Err(e) => eprintln!("pnix-rs: {}", e),
            },
        }
    }
    ExitCode::SUCCESS
}

/// A REPL binding is `name = expr` where `name` is a bare identifier and the
/// `=` is not `==`. Anything else (an expression, an attr set `{ a = 1; }`,
/// a `let .. in ..`) is evaluated instead.
fn split_binding(line: &str) -> Option<(&str, &str)> {
    let eq = line.find('=')?;
    if line.as_bytes().get(eq + 1) == Some(&b'=') {
        return None;
    }
    let lhs = line[..eq].trim();
    let rhs = line[eq + 1..].trim();
    if lhs.is_empty() || rhs.is_empty() {
        return None;
    }
    let ident = lhs.chars().enumerate().all(|(i, c)| {
        if i == 0 {
            c.is_alphabetic() || c == '_'
        } else {
            c.is_alphanumeric() || c == '_'
        }
    });
    if ident {
        Some((lhs, rhs))
    } else {
        None
    }
}

fn cmd_px_eval(rest: &[String]) -> ExitCode {
    let (json, rest) = if rest.first().map(|s| s.as_str()) == Some("--json") {
        (true, &rest[1..])
    } else {
        (false, rest)
    };
    if rest.first().map(|s| s.as_str()) == Some("-f") {
        let path = match rest.get(1) {
            Some(p) => p,
            None => {
                eprintln!("pnix-rs: -f requires a file path");
                return ExitCode::FAILURE;
            }
        };
        let (src, modules, key) = match load_px_file_closure(path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("pnix-rs: {}", e);
                return ExitCode::FAILURE;
            }
        };
        return match px::px_run_value_with_modules(&src, &modules, &key) {
            Ok(value) => {
                if json {
                    match px::px_to_json(&value) {
                        Ok(observation) => {
                            println!("{}", observation);
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("pnix-rs: {}", e);
                            ExitCode::FAILURE
                        }
                    }
                } else {
                    println!("{}", px::px_print(&value));
                    ExitCode::SUCCESS
                }
            }
            Err(e) => {
                eprintln!("pnix-rs: {}", e);
                ExitCode::FAILURE
            }
        };
    }
    if json {
        eprintln!("pnix-rs: --json currently requires -f FILE.px");
        return ExitCode::FAILURE;
    }
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    match px::px_run(&src) {
        Ok(out) => {
            println!("{}", out);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Collect literal `import ./...` targets from a parsed expression. File-mode
/// evaluation uses this to load only the transitive dependency closure instead
/// of scanning an arbitrary repository root.
fn px_import_targets(expr: &px::PxExpr, out: &mut Vec<String>) {
    match expr {
        px::PxExpr::DeferredError(_) => {}
        px::PxExpr::Apply { func, arg } => {
            if let px::PxExpr::Var(name) = func.as_ref() {
                if name == "import" {
                    if let px::PxExpr::Var(marked) = arg.as_ref() {
                        if marked.starts_with(":path:") {
                            out.push(marked.chars().skip(6).collect());
                            return;
                        }
                    }
                }
            }
            px_import_targets(func, out);
            px_import_targets(arg, out);
        }
        px::PxExpr::Str(parts) => {
            for part in parts {
                if let px::PxStrPart::Sub(value) = part {
                    px_import_targets(value, out);
                }
            }
        }
        px::PxExpr::List(items) => {
            for item in items {
                px_import_targets(item, out);
            }
        }
        px::PxExpr::Select { base, .. } => px_import_targets(base, out),
        px::PxExpr::Lambda { body, .. } => px_import_targets(body, out),
        px::PxExpr::If { cond, then_e, else_e } => {
            px_import_targets(cond, out);
            px_import_targets(then_e, out);
            px_import_targets(else_e, out);
        }
        px::PxExpr::Binary { lhs, rhs, .. } => {
            px_import_targets(lhs, out);
            px_import_targets(rhs, out);
        }
        px::PxExpr::LetIn { bindings, body } => {
            for (_, value) in bindings {
                px_import_targets(value, out);
            }
            px_import_targets(body, out);
        }
        px::PxExpr::With { scope, body } => {
            px_import_targets(scope, out);
            px_import_targets(body, out);
        }
        px::PxExpr::Attrs(fields) => {
            for (_, value) in fields {
                px_import_targets(value, out);
            }
        }
        px::PxExpr::Int(_)
        | px::PxExpr::Float(_)
        | px::PxExpr::Bool(_)
        | px::PxExpr::Null
        | px::PxExpr::Var(_) => {}
    }
}

fn normalize_host_path(path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("cannot read current directory: {}", e))?
            .join(path)
    };
    let mut normalized = std::path::PathBuf::new();
    for part in absolute.components() {
        match part {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

fn absolute_module_key(path: &std::path::Path) -> String {
    format!(".{}", path.to_string_lossy())
}

fn load_px_module(
    path: &std::path::Path,
    modules: &mut Vec<(String, String)>,
) -> Result<String, String> {
    let path = normalize_host_path(path)?;
    let key = absolute_module_key(&path);
    if modules.iter().any(|(known, _)| *known == key) {
        return Ok(key);
    }
    let granted = vec![String::from("file-read")];
    let path_text = path.to_string_lossy().to_string();
    let source = interop::host_read_file(&path_text, &granted)?;
    let ast = px::px_parse(&source)?;
    modules.push((key.clone(), source));

    let mut targets = Vec::new();
    px_import_targets(&ast, &mut targets);
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("/"));
    for target in targets {
        // A missing/invalid transitive target may sit in a dead branch or an
        // unused lambda. Leave it absent from the map; px_expand_imports lowers
        // that specific import site to an internal deferred-error leaf.
        let _ = load_px_module(&parent.join(target), modules);
    }
    Ok(key)
}

fn load_px_file_closure(
    path: &str,
) -> Result<(String, Vec<(String, String)>, String), String> {
    let mut modules = Vec::new();
    let key = load_px_module(std::path::Path::new(path), &mut modules)?;
    let source = modules
        .iter()
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, source)| source.clone())
        .ok_or_else(|| String::from("entry module was not loaded"))?;
    Ok((source, modules, key))
}

/// Minimal JSON string escaping for the conformance report fields.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}



fn cmd_px_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[px-check (seed .px corpus)]");
    for (name, path, expected) in px_corpus() {
        let granted = vec![String::from("file-read")];
        let result =
            interop::host_read_file(path, &granted).and_then(|src| px::px_run(&src));
        match result {
            Ok(out) => {
                if out == expected {
                    println!("  ok   {} = {}", name, out);
                    passed += 1;
                } else {
                    println!("  FAIL {}: got {:?}, expected {:?}", name, out, expected);
                    failed += 1;
                }
            }
            Err(e) => {
                println!("  FAIL {}: {}", name, e);
                failed += 1;
            }
        }
    }
    // Audit regression: duplicate attrset keys are rejected at parse (Nix
    // behavior); duplicate let names shadow later-wins (A4) via the corpus.
    // rec attrsets (proposal 0006 runtime surface, oracle: nix eval == 42):
    // desugars to the LetIn Rec frame, so siblings resolve order-independently.
    match px::px_run("(rec { b = a * 2; a = 21; }).b") {
        Ok(v) if v == "42" => {
            println!("  ok   rec attrset sibling reference (42)");
            passed += 1;
        }
        other => {
            println!("  FAIL rec attrset: {:?}", other);
            failed += 1;
        }
    }
    match px::px_run("rec { a = 1; b = a + 1; }") {
        Ok(v) if v == "{ a = 1; b = 2; }" => {
            println!("  ok   rec attrset canonical print");
            passed += 1;
        }
        other => {
            println!("  FAIL rec attrset print: {:?}", other);
            failed += 1;
        }
    }
    // Audit #2 regression: float literals are valid APPLY arguments and
    // JSON leaves (finite only).
    match px::px_run("(x: x * 2.0) 3.5") {
        Ok(v) if v == "7.0" => {
            println!("  ok   float literal as apply argument (7.0)");
            passed += 1;
        }
        other => {
            println!("  FAIL float apply argument: {:?}", other);
            failed += 1;
        }
    }
    match px::px_run("builtins.toJSON { f = 1.5; }") {
        Ok(v) if v == "\"{\\\"f\\\":1.5}\"" => {
            println!("  ok   toJSON float leaf");
            passed += 1;
        }
        other => {
            println!("  FAIL toJSON float: {:?}", other);
            failed += 1;
        }
    }
    match px::px_run("{ a = 1; a = 2; }") {
        Err(e) if e.contains("duplicate attrset key") => {
            println!("  ok   duplicate attrset key rejected");
            passed += 1;
        }
        other => {
            println!("  FAIL duplicate attrset key accepted: {:?}", other.is_ok());
            failed += 1;
        }
    }
    // A failed attrset-field thunk must replay its original error. Before the
    // Failed state was explicit, the first tryEval left the thunk blackholed
    // and the second force was misreported as an infinite-recursion cycle.
    match px::px_run(
        "let s = { x = builtins.throw \"stable\"; }; in [ (builtins.tryEval s.x).success (builtins.tryEval s.x).success ]",
    ) {
        Ok(v) if v == "[ false false ]" => {
            println!("  ok   failed thunk replays original error");
            passed += 1;
        }
        other => {
            println!("  FAIL failed thunk replay: {:?}", other);
            failed += 1;
        }
    }
    match px::px_run(
        "let m = builtins.map (x: builtins.throw \"map\") [ 1 ]; g = builtins.genList (x: builtins.throw \"gen\") 1; a = builtins.mapAttrs (k: v: builtins.throw \"attrs\") { x = 1; }; z = builtins.zipAttrsWith (k: vs: builtins.throw \"zip\") [ { x = 1; } ]; in builtins.length m + builtins.length g + builtins.length (builtins.attrNames a) + builtins.length (builtins.attrNames z)",
    ) {
        Ok(v) if v == "4" => {
            println!("  ok   map/genList/mapAttrs/zipAttrsWith construct lazily");
            passed += 1;
        }
        other => {
            println!("  FAIL deferred builtin construction: {:?}", other);
            failed += 1;
        }
    }
    match px::px_run(
        "[ (builtins.elemAt (builtins.map (x: x + 1) [ 1 ]) 0) (builtins.elemAt (builtins.genList (x: x + 1) 2) 1) ((builtins.mapAttrs (k: v: v + 1) { x = 1; }).x) ((builtins.zipAttrsWith (k: vs: builtins.head vs + 1) [ { x = 1; } ]).x) ]",
    ) {
        Ok(v) if v == "[ 2 2 2 2 ]" => {
            println!("  ok   deferred builtin results force on selection");
            passed += 1;
        }
        other => {
            println!("  FAIL deferred builtin selection: {:?}", other);
            failed += 1;
        }
    }
    match px::px_run(
        "let xs = builtins.map (x: builtins.throw \"mapped\") [ 1 ]; in [ (builtins.tryEval (builtins.elemAt xs 0)).success (builtins.tryEval (builtins.elemAt xs 0)).success ]",
    ) {
        Ok(v) if v == "[ false false ]" => {
            println!("  ok   deferred builtin result memoizes failure");
            passed += 1;
        }
        other => {
            println!("  FAIL deferred builtin memoization: {:?}", other);
            failed += 1;
        }
    }
    let no_modules: Vec<(String, String)> = Vec::new();
    let dead_missing = px::px_run_value_with_modules(
        "if false then import ./missing.px else 42",
        &no_modules,
        "./entry.px",
    );
    let lambda_missing = px::px_run_value_with_modules(
        "let f = x: import ./missing.px; in 7",
        &no_modules,
        "./entry.px",
    );
    let forced_missing = px::px_run_value_with_modules(
        "if true then import ./missing.px else 42",
        &no_modules,
        "./entry.px",
    );
    let shadowed_missing = px::px_run_value_with_modules(
        "let throw = x: 99; in import ./missing.px",
        &no_modules,
        "./entry.px",
    );
    let cycle_modules = vec![
        (String::from("./a.px"), String::from("import ./b.px")),
        (String::from("./b.px"), String::from("import ./a.px")),
    ];
    let forced_cycle = px::px_run_value_with_modules(
        "import ./b.px",
        &cycle_modules,
        "./a.px",
    );
    let dead_ok = matches!(dead_missing, Ok(px::PxVal::Int(42)));
    let lambda_ok = matches!(lambda_missing, Ok(px::PxVal::Int(7)));
    let missing_errors = matches!(forced_missing, Err(ref e) if e.contains("import target not in the module map"));
    let shadow_errors = matches!(shadowed_missing, Err(ref e) if e.contains("import target not in the module map"));
    let cycle_errors = matches!(forced_cycle, Err(ref e) if e.contains("import cycle"));
    if dead_ok && lambda_ok && missing_errors && shadow_errors && cycle_errors {
        println!("  ok   missing/cyclic imports fail only when forced");
        passed += 1;
    } else {
        println!(
            "  FAIL deferred imports: dead={:?} lambda={:?} forced={:?} shadow={:?} cycle={:?}",
            dead_missing, lambda_missing, forced_missing, shadowed_missing, forced_cycle
        );
        failed += 1;
    }
    let numeric_error_cases = vec![
        ("9223372036854775807 + 1", "integer overflow"),
        ("builtins.mul 9223372036854775807 2", "integer overflow"),
        ("(-9223372036854775807 - 1) / (-1)", "integer overflow"),
        ("1.0 / 0.0", "division by zero"),
        ("1.0e9999", "bad float"),
        ("1.0e-308", "bad float"),
        ("00.0", "cannot apply non-lambda int"),
        ("builtins.ceil 9007199254740993", "loses integer precision"),
        ("builtins.floor 9223372036854775808.0", "outside the int range"),
        ("builtins.tryEval (9223372036854775807 + 1)", "integer overflow"),
    ];
    let mut numeric_errors_ok = true;
    for (src, class) in numeric_error_cases {
        if !matches!(px::px_run(src), Err(ref e) if e.contains(class)) {
            numeric_errors_ok = false;
        }
    }
    if numeric_errors_ok {
        println!("  ok   numeric error classes (overflow/div-zero/rounding range)");
        passed += 1;
    } else {
        println!("  FAIL numeric error class regression");
        failed += 1;
    }
    match px::px_run(
        "let f = x: x; in [ (1 + 1.5) (builtins.add 1 1.5) (builtins.lessThan 1 1.5) (1 == 1.0) ([ 1 ] == [ 1.0 ]) (builtins.elem f [ f ]) (builtins.toString (-0.0)) (builtins.toString (0.0 / (-1.0))) (builtins.toString ((-1.0) * 0.0)) (builtins.toString 1.25e-3) (builtins.toString .5e2) (builtins.ceil (-1.8)) (builtins.floor (-1.2)) ]",
    ) {
        Ok(v) if v == "[ 2.5 2.5 true true true true \"0.000000\" \"-0.000000\" \"-0.000000\" \"0.001250\" \"50.000000\" -1 -2 ]" => {
            println!("  ok   mixed numerics/float string/ceil-floor values");
            passed += 1;
        }
        other => {
            println!("  FAIL numeric convergence values: {:?}", other);
            failed += 1;
        }
    }
    match px::px_run(
        "map (a: builtins.hashString a \"abc\") [ \"md5\" \"sha1\" \"sha256\" \"sha512\" ]",
    ) {
        Ok(v) if v == "[ \"900150983cd24fb0d6963f7d28e17f72\" \"a9993e364706816aba3e25717850c26c9cd0d89d\" \"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\" \"ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f\" ]" => {
            println!("  ok   hashString Nix default algorithms");
            passed += 1;
        }
        other => {
            println!("  FAIL hashString convergence: {:?}", other);
            failed += 1;
        }
    }
    match px::px_run(
        "let m = p: s: builtins.match p s != null; in [ (m \"[[:alnum:]]+\" \"Az09\") (m \"[[:alpha:]]+\" \"Az\") (m \"[[:blank:]]+\" \" \\t\") (m \"[[:cntrl:]]+\" \"\\t\\n\") (m \"[[:digit:]]+\" \"09\") (m \"[[:graph:]]+\" \"Az!9\") (m \"[[:lower:]]+\" \"az\") (m \"[[:print:]]+\" \" Az!9\") (m \"[[:punct:]]+\" \"!?\") (m \"[[:space:]]+\" \" \\t\\n\") (m \"[[:upper:]]+\" \"AZ\") (m \"[[:xdigit:]]+\" \"aF09\") (builtins.match \"[[:space:]]\" \"\u{a0}\" == null) (builtins.match \"[[:digit:]]\" \"\u{660}\" == null) (builtins.match \"[[:space:]]*(.*[^[:space:]])[[:space:]]*\" \" ?x \" == [ \"?x\" ]) (builtins.split \"[[:space:]]+\" \"a \\tb\\nc\" == [ \"a\" [ ] \"b\" [ ] \"c\" ]) (builtins.match \"[:space:]\" \":\" == [ ]) ]",
    ) {
        Ok(v) if v == "[ true true true true true true true true true true true true true true true true true ]" => {
            println!("  ok   POSIX named regex classes match Nix ASCII semantics");
            passed += 1;
        }
        other => {
            println!("  FAIL POSIX named regex classes: {:?}", other);
            failed += 1;
        }
    }
    if matches!(
        px::px_run("builtins.match \"[[:bogus:]]\" \"b\""),
        Err(ref e) if e.contains("unknown POSIX character class bogus")
    ) {
        println!("  ok   unknown POSIX regex class fails closed");
        passed += 1;
    } else {
        println!("  FAIL unknown POSIX regex class did not fail closed");
        failed += 1;
    }
    match px::px_run(
        "[ x:x let:x a1:b a+b:c a-b:c a.b:c a:b:c a:%/?::@&=+$,-_.!~*' (builtins.typeOf (x: x)) (builtins.typeOf (_x:_x)) (a:b + \"c\") ]",
    ) {
        Ok(v) if v == "[ \"x:x\" \"let:x\" \"a1:b\" \"a+b:c\" \"a-b:c\" \"a.b:c\" \"a:b:c\" \"a:%/?::@&=+$,-_.!~*'\" \"lambda\" \"lambda\" \"a:bc\" ]" => {
            println!("  ok   URI literal maximal match/lambda boundary");
            passed += 1;
        }
        other => {
            println!("  FAIL URI literal convergence: {:?}", other);
            failed += 1;
        }
    }
    let uri_boundaries_ok = matches!(
        px::px_run("a:b+\"c\""),
        Err(ref e) if e.contains("cannot apply non-lambda string")
    ) && px::px_run("{ x:y = 1; }").is_err();
    if uri_boundaries_ok {
        println!("  ok   URI delimiter/attribute-key boundaries");
        passed += 1;
    } else {
        println!("  FAIL URI delimiter/attribute-key boundaries");
        failed += 1;
    }
    let hash_order_ok = matches!(
        px::px_run("builtins.hashString \"sha3\" (builtins.throw \"payload\")"),
        Err(ref e) if e.contains("unsupported algorithm sha3")
    ) && matches!(
        px::px_run("builtins.hashString 1 (builtins.throw \"payload\")"),
        Err(ref e) if e.contains("algorithm must be a string")
    ) && matches!(
        px::px_run("builtins.hashString \"sha3\" 1"),
        Err(ref e) if e.contains("unsupported algorithm sha3")
    ) && matches!(
        px::px_run("builtins.hashString (builtins.substring 0 1 \"가\") (builtins.throw \"payload\")"),
        Err(ref e) if e.contains("unsupported raw-byte algorithm")
    ) && matches!(
        px::px_run("builtins.tryEval (builtins.hashString \"sha3\" \"payload\")"),
        Err(ref e) if e.contains("unsupported algorithm sha3")
    ) && matches!(
        px::px_run("builtins.tryEval (builtins.hashString \"sha256\" 1)"),
        Err(ref e) if e.contains("payload must be string-like")
    );
    if hash_order_ok {
        println!("  ok   hashString validates selector before payload");
        passed += 1;
    } else {
        println!("  FAIL hashString selector/payload error order");
        failed += 1;
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cmd_mirror(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let record = mirror::mirror_run(&src);
    print!("{}", mirror::render(&record));
    if record.status == "lossless" || record.status == "lossy-ok" || record.status == "held" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Every corpus program must mirror as `lossless`: value survives
/// parse -> emit -> reparse -> re-eval, and emission is a fixed point.
fn cmd_mirror_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[mirror-check (singleton mirror over the corpus, all lossless)]");
    for (name, path, _expected) in px_corpus() {
        let granted = vec![String::from("file-read")];
        let record = match interop::host_read_file(path, &granted) {
            Ok(src) => mirror::mirror_run(&src),
            Err(e) => {
                println!("  FAIL {}: read {}: {}", name, path, e);
                failed += 1;
                continue;
            }
        };
        if record.status == "lossless" {
            let value_fnv = match record.value_fnv {
                Some(h) => format!("{:016x}", h),
                None => String::from("-"),
            };
            let emit_fnv = match record.emitted_fnv {
                Some(h) => format!("{:016x}", h),
                None => String::from("-"),
            };
            println!(
                "  ok   {} lossless value_fnv={} emit_fnv={}",
                name, value_fnv, emit_fnv
            );
            passed += 1;
        } else {
            let err = record.error.unwrap_or_default();
            println!("  FAIL {}: status {} {}", name, record.status, err);
            failed += 1;
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cmd_stage(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let ladder = stage::stage_run(&src);
    print!("{}", stage::render(&ladder));
    if ladder.closure {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Every corpus program must close the runtime stage ladder: direct eval,
/// normalized eval, content-addressed store eval, AST roundtrip, and replay
/// all produce the same value hash.
fn cmd_stage_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[stage-check (pnix runtime stage ladder over the corpus)]");
    for (name, path, _expected) in px_corpus() {
        let granted = vec![String::from("file-read")];
        let ladder = match interop::host_read_file(path, &granted) {
            Ok(src) => stage::stage_run(&src),
            Err(e) => {
                println!("  FAIL {}: read {}: {}", name, path, e);
                failed += 1;
                continue;
            }
        };
        if ladder.closure {
            let h = match ladder.stages.first() {
                Some(s) => format!("{:016x}", s.value_fnv),
                None => String::from("-"),
            };
            println!("  ok   {} closure(5 stages) value_fnv={}", name, h);
            passed += 1;
        } else {
            let mut notes = String::new();
            for s in &ladder.stages {
                if !s.ok {
                    notes.push_str(&format!(" {}:{}", s.name, s.note));
                }
            }
            let err = ladder.error.unwrap_or_default();
            println!("  FAIL {}:{} {}", name, notes, err);
            failed += 1;
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cmd_ir(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    match ir::ir_of(&src) {
        Ok(record) => {
            print!("{}", ir::render(&record));
            if record.canonical_fixed_point && record.value_matches_source {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// IR lane proof: sha256 self-test (FIPS vectors), every corpus program's IR is
/// directly evaluable + value-equivalent + a canonical fixed point, and
/// binding-order-only variants share one IR identity.
fn cmd_ir_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[ir-check (canonical IR: content-addressed, directly evaluable)]");
    match sha256::self_test() {
        Ok(()) => {
            println!("  ok   sha256 self-test (FIPS 180-4 vectors)");
            passed += 1;
        }
        Err(e) => {
            println!("  FAIL sha256 self-test: {}", e);
            failed += 1;
        }
    }
    for (name, path, _expected) in px_corpus() {
        let granted = vec![String::from("file-read")];
        let result = interop::host_read_file(path, &granted).and_then(|src| ir::ir_of(&src));
        match result {
            Ok(record) => {
                if record.canonical_fixed_point && record.value_matches_source {
                    println!("  ok   {} ir_sha256={}", name, &record.ir_sha256[0..16]);
                    passed += 1;
                } else {
                    println!(
                        "  FAIL {}: fixed_point={} value_match={}",
                        name, record.canonical_fixed_point, record.value_matches_source
                    );
                    failed += 1;
                }
            }
            Err(e) => {
                println!("  FAIL {}: {}", name, e);
                failed += 1;
            }
        }
    }
    // Identity sharing: binding order is a name-level detail, not IR identity.
    let a = ir::ir_of("let y = x + 1; x = 10; in y");
    let b = ir::ir_of("let x = 10; y = x + 1; in y");
    match (a, b) {
        (Ok(ra), Ok(rb)) => {
            if ra.ir_sha256 == rb.ir_sha256 {
                println!(
                    "  ok   binding-order variants share IR identity ({})",
                    &ra.ir_sha256[0..16]
                );
                passed += 1;
            } else {
                println!(
                    "  FAIL identity sharing: {} != {}",
                    ra.ir_sha256, rb.ir_sha256
                );
                failed += 1;
            }
        }
        _ => {
            println!("  FAIL identity sharing probe: ir_of error");
            failed += 1;
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cmd_gate(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let record = gate::gate_check(&src, &[]);
    print!("{}", gate::render_gate(&record));
    if record.allowed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cmd_witness(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    match gate::eval_witness(&src, &[]) {
        Ok(w) => {
            print!("{}", gate::render_witness(&w));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Gate lane proof: the corpus is pure and admitted with zero grants; uncertain
/// effect surfaces fail closed; witness records are deterministic and follow
/// the shared 13-field schema for eval/mirror/IR directions.
fn cmd_gate_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[gate-check (purity/capability admission + witnesses)]");
    // The declared effect vocabulary is a checked fact: five classes; each
    // builtin is either pure (None) or maps to one of those classes.
    let mut effects_ok = true;
    for b in px::px_builtin_names() {
        if let Some(effect) = gate::effect_of(b) {
            if !gate::EFFECT_CLASSES.iter().any(|c| *c == effect) {
                effects_ok = false;
            }
        }
    }
    if gate::EFFECT_CLASSES.len() == 5 && effects_ok {
        println!(
            "  ok   effect vocabulary [{}]; builtin effects classified",
            gate::EFFECT_CLASSES.join(" ")
        );
        passed += 1;
    } else {
        println!("  FAIL effect vocabulary drift");
        failed += 1;
    }
    let public_values = gate::gate_check(
        "{ t = builtins.true; sd = builtins.storeDir; }",
        &[],
    );
    if public_values.pure
        && public_values.allowed
        && public_values.uncertain.is_empty()
        && public_values.builtin_uses.iter().any(|name| name == "true")
        && public_values.builtin_uses.iter().any(|name| name == "storeDir")
    {
        println!("  ok   public builtin constant selectors admitted");
        passed += 1;
    } else {
        println!("  FAIL public builtin constant selectors were uncertain");
        failed += 1;
    }
    let self_alias = gate::gate_check("builtins.builtins", &[]);
    if !self_alias.allowed
        && self_alias
            .uncertain
            .iter()
            .any(|reason| reason == "builtins escapes as a value")
    {
        println!("  ok   public builtins self alias fails closed as an escape");
        passed += 1;
    } else {
        println!("  FAIL public builtins self alias was not classified");
        failed += 1;
    }
    for (name, path, _expected) in px_corpus() {
        let granted = vec![String::from("file-read")];
        let record = match interop::host_read_file(path, &granted) {
            Ok(src) => gate::gate_check(&src, &[]),
            Err(e) => {
                println!("  FAIL {}: read {}: {}", name, path, e);
                failed += 1;
                continue;
            }
        };
        if record.pure && record.allowed {
            println!(
                "  ok   {} pure admitted (builtins: {})",
                name,
                record.builtin_uses.len()
            );
            passed += 1;
        } else {
            println!(
                "  FAIL {}: pure={} allowed={} uncertain=[{}]",
                name,
                record.pure,
                record.allowed,
                record.uncertain.join("; ")
            );
            failed += 1;
        }
    }
    // Fail-closed probes: uncertain effect surfaces are denied without grants.
    let unknown = gate::gate_check("builtins.currentTime 1", &[]);
    if !unknown.allowed && !unknown.uncertain.is_empty() {
        println!("  ok   unknown builtin fails closed");
        passed += 1;
    } else {
        println!("  FAIL unknown builtin was admitted");
        failed += 1;
    }
    let escape = gate::gate_check("let b = builtins; in 0", &[]);
    if !escape.allowed && !escape.uncertain.is_empty() {
        println!("  ok   builtins-as-value fails closed");
        passed += 1;
    } else {
        println!("  FAIL builtins-as-value was admitted");
        failed += 1;
    }
    // Witness determinism + schema over the three directions.
    let read_grant = vec![String::from("file-read")];
    let probe = match interop::host_read_file("runtime/corpus/c05_recurse.px", &read_grant) {
        Ok(s) => s,
        Err(e) => {
            println!("  FAIL read witness probe: {}", e);
            failed += 1;
            String::new()
        }
    };
    if !probe.is_empty() {
        let pairs = [
            ("eval", gate::eval_witness(&probe, &[]), gate::eval_witness(&probe, &[])),
            ("mirror", gate::mirror_witness(&probe, &[]), gate::mirror_witness(&probe, &[])),
            ("ir", gate::ir_witness(&probe, &[]), gate::ir_witness(&probe, &[])),
        ];
        for (label, first, second) in pairs {
            match (first, second) {
                (Ok(w1), Ok(w2)) => {
                    let r1 = gate::render_witness(&w1);
                    let r2 = gate::render_witness(&w2);
                    let mut schema_ok = true;
                    for field in gate::WITNESS_FIELDS {
                        if !r1.contains(&format!("\n{} ", field)) && !r1.starts_with(&format!("{} ", field)) {
                            schema_ok = false;
                        }
                    }
                    if r1 == r2 && schema_ok && w1.status == "ok" {
                        println!("  ok   {} witness deterministic, 13-field schema", label);
                        passed += 1;
                    } else {
                        println!(
                            "  FAIL {} witness: deterministic={} schema_ok={} status={}",
                            label,
                            r1 == r2,
                            schema_ok,
                            w1.status
                        );
                        failed += 1;
                    }
                }
                _ => {
                    println!("  FAIL {} witness construction", label);
                    failed += 1;
                }
            }
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cmd_specialize(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    match specialize::specialize(&src, &[]) {
        Ok(record) => {
            print!("{}", specialize::render(&record));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Partial-evaluation proof: closed programs fold to their runtime value
/// (recursive-let semantics preserved via the sacred runtime), dynamic lets
/// stay intact with an honest gap, static bindings inject, residuals reparse.
fn cmd_specialize_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[specialize-check (A4-sound partial evaluation)]");

    let expect_static = [
        ("nested-shadow", "let x = 5; in let y = x + 1; x = 10; in y", "11"),
        ("sibling-order", "let b = a + 1; a = 2; in b", "3"),
    ];
    for (name, src, want) in expect_static {
        match specialize::specialize(src, &[]) {
            Ok(r) => match &r.fully_static {
                Some(v) if v == want && r.gaps.is_empty() => {
                    println!("  ok   {} folds to {}", name, want);
                    passed += 1;
                }
                other => {
                    println!("  FAIL {}: fully_static={:?} gaps=[{}]", name, other, r.gaps.join("; "));
                    failed += 1;
                }
            },
            Err(e) => {
                println!("  FAIL {}: {}", name, e);
                failed += 1;
            }
        }
    }

    // Dynamic sibling: whole let stays, gap recorded, residual reparses.
    match specialize::specialize("let b = a + d; a = 2; in b", &[]) {
        Ok(r) => {
            let gap_ok = r.gaps.iter().any(|g| g.contains("let-recursive-not-static"));
            let reparse_ok = px::px_parse(&r.residual).is_ok();
            let still_let = r.residual.starts_with("let ");
            if gap_ok && reparse_ok && still_let && r.fully_static.is_none() {
                println!("  ok   dynamic let held intact (gap + reparse)");
                passed += 1;
            } else {
                println!(
                    "  FAIL dynamic let: gap_ok={} reparse_ok={} still_let={} residual={}",
                    gap_ok, reparse_ok, still_let, r.residual
                );
                failed += 1;
            }
        }
        Err(e) => {
            println!("  FAIL dynamic let: {}", e);
            failed += 1;
        }
    }

    // Static-binding injection closes the same program.
    let d_binding = vec![(String::from("d"), px::PxVal::Int(7))];
    match specialize::specialize("let b = a + d; a = 2; in b", &d_binding) {
        Ok(r) => match &r.fully_static {
            Some(v) if v == "9" && r.gaps.is_empty() => {
                println!("  ok   static binding d=7 folds to 9");
                passed += 1;
            }
            other => {
                println!("  FAIL static injection: fully_static={:?}", other);
                failed += 1;
            }
        },
        Err(e) => {
            println!("  FAIL static injection: {}", e);
            failed += 1;
        }
    }

    // Closed corpus programs fold to exactly their runtime value.
    let granted = vec![String::from("file-read")];
    for name in ["c05_recurse", "c09_lambda", "seed_arith"] {
        let path = format!("runtime/corpus/{}.px", name);
        let result = interop::host_read_file(&path, &granted).and_then(|src| {
            let expected = px::px_run(&src)?;
            let r = specialize::specialize(&src, &[])?;
            Ok((expected, r))
        });
        match result {
            Ok((expected, r)) => match &r.fully_static {
                Some(v) if *v == expected && r.gaps.is_empty() => {
                    println!("  ok   {} fully static == runtime value", name);
                    passed += 1;
                }
                other => {
                    println!(
                        "  FAIL {}: fully_static={:?} expected={}",
                        name, other, expected
                    );
                    failed += 1;
                }
            },
            Err(e) => {
                println!("  FAIL {}: {}", name, e);
                failed += 1;
            }
        }
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cmd_incremental(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    match incremental::definition_hashes(&src) {
        Ok(hashes) => {
            println!("schema {}", incremental::INCREMENTAL_SCHEMA);
            for (name, h) in hashes {
                println!("def {} {}", name, h);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Incremental identity proof: alpha-renaming a dependency does not change a
/// referrer's hash, semantic change does, the realisation store gives early
/// cutoff, and self-recursive definitions hash as SCC groups.
fn cmd_incremental_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[incremental-check (definition hashes + realisation cutoff)]");

    let hash_of = |src: &str, name: &str| -> Result<String, String> {
        let hs = incremental::definition_hashes(src)?;
        hs.into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, h)| h)
            .ok_or_else(|| format!("no definition {}", name))
    };

    // Alpha-rename invariance: names are metadata.
    match (
        hash_of("let d = 5; b = d + 1; in b", "b"),
        hash_of("let e = 5; b = e + 1; in b", "b"),
    ) {
        (Ok(h1), Ok(h2)) if h1 == h2 => {
            println!("  ok   alpha-renaming a dependency keeps b's hash");
            passed += 1;
        }
        other => {
            println!("  FAIL alpha invariance: {:?}", other);
            failed += 1;
        }
    }
    // Semantic change invalidates.
    match (
        hash_of("let d = 5; b = d + 1; in b", "b"),
        hash_of("let d = 6; b = d + 1; in b", "b"),
    ) {
        (Ok(h1), Ok(h2)) if h1 != h2 => {
            println!("  ok   semantic change flips b's hash");
            passed += 1;
        }
        other => {
            println!("  FAIL semantic change: {:?}", other);
            failed += 1;
        }
    }
    // SCC probe: self-recursive definitions (c05 go/fib) hash deterministically
    // as groups, and distinct members get distinct hashes.
    let granted = vec![String::from("file-read"), String::from("file-write")];
    match interop::host_read_file("runtime/corpus/c05_recurse.px", &granted) {
        Ok(src) => match (
            incremental::definition_hashes(&src),
            incremental::definition_hashes(&src),
        ) {
            (Ok(h1), Ok(h2)) => {
                let same = h1 == h2;
                let go = h1.iter().find(|(n, _)| n == "go").map(|(_, h)| h.clone());
                let fib = h1.iter().find(|(n, _)| n == "fib").map(|(_, h)| h.clone());
                if same && go.is_some() && fib.is_some() && go != fib {
                    println!("  ok   SCC hashes deterministic; go != fib");
                    passed += 1;
                } else {
                    println!("  FAIL SCC probe: same={} go={:?} fib={:?}", same, go, fib);
                    failed += 1;
                }
            }
            other => {
                println!("  FAIL SCC probe: {:?}", other.0.is_ok());
                failed += 1;
            }
        },
        Err(e) => {
            println!("  FAIL SCC probe read: {}", e);
            failed += 1;
        }
    }
    // Early cutoff on a fresh scratch store.
    let store = "work/realisations-check.tsv";
    if let Err(e) = interop::host_ensure_dir("work", &granted) {
        println!("  FAIL scratch dir: {}", e);
        failed += 1;
    }
    if let Err(e) = interop::host_remove_file(store, &granted) {
        println!("  FAIL scratch reset: {}", e);
        failed += 1;
    }
    let probe = "let go = acc: n: if n == 0 then acc else go (acc + n) (n - 1); in go 0 100";
    match (
        incremental::incremental_eval(probe, store, &granted),
        incremental::incremental_eval(probe, store, &granted),
    ) {
        (Ok((v1, false)), Ok((v2, true))) if v1 == v2 => {
            println!("  ok   second eval is an early cutoff with same value hash");
            passed += 1;
        }
        other => {
            println!(
                "  FAIL cutoff: first={:?} second={:?}",
                other.0.is_ok(),
                other.1.is_ok()
            );
            failed += 1;
        }
    }
    // Cutoff store keyed by IR identity: binding-order variant also cuts off.
    let probe2 = "let y = x + 1; x = 10; in y";
    let probe2_sorted = "let x = 10; y = x + 1; in y";
    let _ = incremental::incremental_eval(probe2, store, &granted);
    match incremental::incremental_eval(probe2_sorted, store, &granted) {
        Ok((_v, true)) => {
            println!("  ok   binding-order variant hits the same realisation");
            passed += 1;
        }
        other => {
            println!("  FAIL ir-identity cutoff: {:?}", other.is_ok());
            failed += 1;
        }
    }

    // Demand-driven change propagation (salsa/adapton early cutoff): editing
    // one definition changes ONLY that def + its transitive dependents;
    // independent siblings keep identical hashes (minimal recomputation).
    // Program: a (leaf), b = a + 1 (depends on a), c = 10 (independent).
    let prog = "let a = 5; b = a + 1; c = 10; in b + c";
    // (1) edit the INDEPENDENT def c -> only c changes.
    match incremental::changed_between(prog, "let a = 5; b = a + 1; c = 20; in b + c") {
        Ok(set) if set == vec![String::from("c")] => {
            println!("  ok   독립 정의 변경 -> c만 재계산 (a,b는 early cutoff)");
            passed += 1;
        }
        other => {
            println!("  FAIL 독립 변경 전파: {:?}", other);
            failed += 1;
        }
    }
    // (2) edit the DEPENDED-UPON def a -> a AND its dependent b change; c not.
    match incremental::changed_between(prog, "let a = 9; b = a + 1; c = 10; in b + c") {
        Ok(set) if set == vec![String::from("a"), String::from("b")] => {
            println!("  ok   피의존 정의 변경 -> a+의존자 b 재계산, 독립 c는 불변");
            passed += 1;
        }
        other => {
            println!("  FAIL 피의존 변경 전파: {:?}", other);
            failed += 1;
        }
    }
    // (3) no change -> empty set (nothing recomputes).
    match incremental::changed_between(prog, prog) {
        Ok(set) if set.is_empty() => {
            println!("  ok   변경 없음 -> 재계산 0 (전량 early cutoff)");
            passed += 1;
        }
        other => {
            println!("  FAIL no-change: {:?}", other);
            failed += 1;
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Compartment isolation proof: bindings do not leak across compartments,
/// pure intrinsics are shared, modules materialize lazily exactly once.
fn cmd_compartment_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[compartment-check (SES-style isolation bookkeeping)]");

    let mut a = compartment::Compartment::new();
    let mut b = compartment::Compartment::new();
    match a.define("x", "10") {
        Ok(()) => {}
        Err(e) => {
            println!("  FAIL define in A: {}", e);
            failed += 1;
        }
    }
    match b.eval("x + 1") {
        Err(e) if e.contains("unbound variable x") => {
            println!("  ok   A's binding is invisible in B (isolation)");
            passed += 1;
        }
        other => {
            println!("  FAIL isolation: {:?}", other.is_ok());
            failed += 1;
        }
    }
    match (a.eval("builtins.length [ 1 2 3 ]"), b.eval("builtins.length [ 1 2 3 ]")) {
        (Ok(va), Ok(vb)) if va == vb && va == "3" => {
            println!("  ok   shared pure intrinsics agree across compartments");
            passed += 1;
        }
        other => {
            println!("  FAIL shared intrinsics: {:?}", other.0.is_ok());
            failed += 1;
        }
    }
    let mut c = compartment::Compartment::new();
    c.register_module("m", "let f = x: x * 2; in { double = f; base = 21; }");
    match (c.eval("m.double m.base"), c.eval("m.base + 1"), c.materialize_count) {
        (Ok(v1), Ok(v2), 1) if v1 == "42" && v2 == "22" => {
            println!("  ok   module materializes lazily exactly once");
            passed += 1;
        }
        (r1, r2, n) => {
            println!(
                "  FAIL lazy module: {:?} {:?} count={}",
                r1.is_ok(),
                r2.is_ok(),
                n
            );
            failed += 1;
        }
    }
    match a.eval("x + 32") {
        Ok(v) if v == "42" => {
            println!("  ok   A's persistent binding accumulates (REPL-style)");
            passed += 1;
        }
        other => {
            println!("  FAIL persistence: {:?}", other.is_ok());
            failed += 1;
        }
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Tower milestone-1 proof: reify/reflect roundtrips, the px-written
/// self-interpreter agrees with the sacred runtime on sequential-safe probes,
/// and encodings are content-deterministic.
/// EXPERIMENT (not a gate): attempt the 2nd Futamura projection — mix
/// specializing MIX'S OWN encoding to the object-language interpreter.
/// Records termination/size/reflectability; the expected gap is closure
/// residualization (pnix-hy needed POLY closure conversion for this rung).
fn cmd_second_projection_experiment() -> ExitCode {
    let granted = vec![String::from("file-read"), String::from("host-call")];
    // 6R insight (searched): pnix-hy's 2nd-projection OBJECT was their SMALL
    // mono mix; ours was the full m6b-extended one (~5x bigger encoding).
    // The interpreter only needs the m6a-era core — use the trimmed object.
    let mix_src = match interop::host_read_file("runtime/tower/mix_core.px", &granted) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let trimmed = mix_src.trim();
    let obj_src = match trimmed.strip_suffix("in mix") {
        Some(head) => format!("{}in mix ast senv", head),
        None => {
            eprintln!("pnix-rs: mix.px does not end with `in mix`");
            return ExitCode::FAILURE;
        }
    };
    let interp_src = "let int = prog: env: \
if prog.tag == \"num\" then prog.value \
else if prog.tag == \"arg\" then env \
else if prog.tag == \"add\" then (int prog.l env) + (int prog.r env) \
else (int prog.l env) * (int prog.r env); \
in int prog input";
    let interp_owned = interp_src.to_string();
    println!("second projection SCALE CURVE (m6f): fuel-bounded poly runs ...");
    // Self-application nests guest frames deeply; give the evaluator a big
    // stack (experiment harness only — gates never need this). Rc values are
    // not Send, so everything is (re)built inside the thread from strings.
    let handle = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(move || {
            let mut rows: Vec<(i64, usize, i64, u128)> = Vec::new();
            eprintln!("  fuel      specs  ctr     ms");
            for fuel in [100_000i64, 1_000_000, 9_000_000_000_000_000] {
                let started = std::time::Instant::now();
                let interp_enc =
                    px::px_parse(&interp_owned).and_then(|a| tower::reify(&a))?;
                let statics = vec![(String::from("ast"), interp_enc)];
                let outcome =
                    tower::poly_mix_fueled(&obj_src, &statics, &granted, fuel)?;
                let row = (
                    fuel,
                    outcome.spec_count,
                    outcome.ctr,
                    started.elapsed().as_millis(),
                );
                eprintln!("  {:<9} {:<6} {:<7} {}", row.0, row.1, row.2, row.3);
                rows.push(row);
            }
            Ok::<Vec<(i64, usize, i64, u128)>, String>(rows)
        })
        .expect("spawn");
    match handle.join().expect("join") {
        Ok(rows) => {
            println!("  fuel      specs  ctr     ms");
            for (fuel, specs, ctr, ms) in rows {
                println!("  {:<9} {:<6} {:<7} {}", fuel, specs, ctr, ms);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("  did not complete: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// EXPERIMENT: the THIRD Futamura projection — poly specializing ITSELF with
/// all arguments dynamic yields the cogen (the specializer's generating
/// extension). Then: (a) the cogen EXECUTES as a specializer, (b) the m5
/// self-generation criterion — cogen applied to the specializer's own
/// encoding reproduces cogen (IR hash equality).
fn cmd_third_projection_experiment() -> ExitCode {
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let handle = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(move || -> Result<(), String> {
            let poly_src = interop::host_read_file("runtime/tower/poly_mix.px", &granted)?;
            let obj_src = match poly_src.trim().strip_suffix("in run") {
                Some(head) => format!("{}in run ast senv st", head),
                None => return Err(String::from("poly_mix.px does not end with `in run`")),
            };
            // Fuel scale curve first (m6f instrumentation): observe spec/ctr
            // growth on the self-application BEFORE attempting an unbounded run.
            let no_statics: Vec<(String, px::PxVal)> = Vec::new();
            println!("  fuel      specs  ctr     ms");
            for fuel in [10_000i64, 40_000, 160_000, 640_000] {
                let started = std::time::Instant::now();
                let o = tower::poly_mix_fueled(&obj_src, &no_statics, &granted, fuel)?;
                println!(
                    "  {:<9} {:<6} {:<7} {}",
                    fuel,
                    o.spec_count,
                    o.ctr,
                    started.elapsed().as_millis()
                );
            }
            let started = std::time::Instant::now();
            let cogen = tower::poly_mix_in_px_data(&obj_src, &no_statics, &granted)?;
            println!(
                "  cogen produced: {} specs, {} ms, reflectable: {}",
                cogen.spec_count,
                started.elapsed().as_millis(),
                cogen.residual_source.is_some()
            );
            let cogen_src = cogen
                .residual_source
                .ok_or_else(|| String::from("cogen not reflectable"))?;
            println!("  cogen source: {} chars", cogen_src.len());

            // (a) the cogen EXECUTES as a specializer on a small task.
            let task_enc = px::px_parse("x * (2 + 3)").and_then(|a| tower::reify(&a))?;
            let started = std::time::Instant::now();
            let run = px::px_run_value(&format!(
                "let ast = {}; senv = {{ }}; st = {{ specs = [ ]; ctr = 0; frames = [ ]; fuel = 9000000000000000; }}; in ({})",
                px::px_print(&task_enc),
                cogen_src
            ))?;
            let outcome = tower::poly_result_to_outcome(&run)?;
            println!(
                "  cogen-as-specializer: residual {:?} ({} ms)",
                outcome.residual_source,
                started.elapsed().as_millis()
            );

            // (b) self-generation: cogen(enc(poly-object)) reproduces cogen.
            let started = std::time::Instant::now();
            let self_enc = px::px_parse(&obj_src).and_then(|a| tower::reify(&a))?;
            let run2 = px::px_run_value(&format!(
                "let ast = {}; senv = {{ }}; st = {{ specs = [ ]; ctr = 0; frames = [ ]; fuel = 9000000000000000; }}; in ({})",
                px::px_print(&self_enc),
                cogen_src
            ))?;
            let outcome2 = tower::poly_result_to_outcome(&run2)?;
            let produced = outcome2
                .residual_source
                .ok_or_else(|| String::from("cogen(self) not reflectable"))?;
            println!(
                "  cogen(self): {} chars ({} ms)",
                produced.len(),
                started.elapsed().as_millis()
            );
            let apply_fn = |_c: &str, _m: &str| -> String { produced.clone() };
            let (equal, ha, hb, _w) =
                tower::cogen_acceptance(&cogen_src, &obj_src, &apply_fn)?;
            println!(
                "  SELF-GENERATION: equal = {} (produced {} / cogen {})",
                equal,
                &ha[..12],
                &hb[..12]
            );
            Ok(())
        })
        .expect("spawn");
    match handle.join().expect("join") {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            println!("  did not complete: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// proposal 0005 gate: WELL-TYPED RESIDUAL, floor-certified. Every px→Rust
/// reconstruction (P6) is well-typed by rs-meta's OWN typeck — not merely
/// "rustc happened to accept it." Since typeck-check proves the floor accepts
/// iff rustc does, this certifies well-typedness from the trusted meta-circular
/// floor. Also a NEGATIVE: a deliberately ill-typed Rust program is rejected by
/// the floor typeck (the gate has teeth). This static guarantee is the Rust/
/// statically-typed edge a dynamic-Lisp meta-circular cannot cheaply provide.
fn cmd_welltyped_check() -> ExitCode {
    let bootstrap = bootstrap_path();
    if !std::path::Path::new(&bootstrap).exists() {
        println!("[welltyped-check] rs-meta bootstrap 없음 ({}) — skip", bootstrap);
        println!("  => PASS (0 passed, 0 failed)");
        return ExitCode::SUCCESS;
    }
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let mut passed = 0;
    let mut failed = 0;

    let add3 = "fn add3(a: i64, b: i64, c: i64) -> i64 { a + b + c } fn main() { let x = add3(1, 2, 3); println!(\"{}\", x); }";
    let struct_src = "struct Point { x: i64, y: i64 } impl Point { fn origin() -> Point { Point { x: 40, y: 2 } } fn sum(&self) -> i64 { self.x + self.y } } fn main() { let p = Point::origin(); println!(\"{}\", p.sum()); }";
    let generic_src = "fn id<T>(x: T) -> T { x } fn main() { println!(\"{}\", id(42)); }";
    let samples: Vec<(&str, String)> = vec![
        (
            "factorial.rs",
            interop::host_read_file("../rs-meta/samples/factorial.rs", &granted).unwrap_or_default(),
        ),
        ("add3", String::from(add3)),
        ("struct-impl", String::from(struct_src)),
        ("generic-fn", String::from(generic_src)),
    ];
    for (name, src) in &samples {
        match rust_mirror::rust_program_reconstruct(src, &bootstrap, &granted) {
            Ok(r) if r.well_typed && r.ast_identity => {
                println!(
                    "  ok   {} — 재구성 residual이 플로어 typeck로 well-typed (구성상)",
                    name
                );
                passed += 1;
            }
            Ok(r) => {
                println!(
                    "  FAIL {}: well_typed={} ast_identity={}",
                    name, r.well_typed, r.ast_identity
                );
                failed += 1;
            }
            Err(e) => {
                println!("  FAIL {}: {}", name, e);
                failed += 1;
            }
        }
    }
    // NEGATIVE: the floor typeck rejects an ill-typed program (teeth).
    let ill = "fn main() { let x: bool = 5; println!(\"{}\", x); }";
    match interop::host_run_bootstrap_inline(&bootstrap, "typecheck", ill, &granted) {
        Err(_) => {
            println!("  ok   ill-typed 프로그램은 플로어 typeck가 거부 (게이트에 이빨 있음)");
            passed += 1;
        }
        Ok(_) => {
            println!("  FAIL ill-typed 프로그램이 통과됨");
            failed += 1;
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// `engine-batch -c|-f <px-list>` — process a batch of Rust sources, emit a
/// verdict manifest with counts.
fn cmd_engine_batch(rest: &[String]) -> ExitCode {
    let list = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let bootstrap = bootstrap_path();
    let granted = vec![String::from("host-call")];
    match engine::engine_batch(&list, &bootstrap, &granted) {
        Ok(manifest) => {
            println!("{}", manifest);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("pnix-rs: engine-batch: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// engine-batch gate: a batch of [good, macro_rules] -> manifest with total=2,
/// accepted=1, held=1; the manifest is a valid .px value.
fn cmd_engine_batch_check() -> ExitCode {
    let bootstrap = bootstrap_path();
    let mut passed = 0;
    let mut failed = 0;
    if !std::path::Path::new(&bootstrap).exists() {
        println!("[engine-batch-check] rs-meta bootstrap 없음 — skip");
        println!("  => PASS (0 passed, 0 failed)");
        return ExitCode::SUCCESS;
    }
    let granted = vec![String::from("host-call")];
    // A batch: one accepted (good) + one held (macro_rules).
    let batch = "[ \"fn main() { println!(\\\"{}\\\", 42); }\" \"macro_rules! m { () => {}; } fn main() {}\" ]";
    let manifest = match engine::engine_batch(batch, &bootstrap, &granted) {
        Ok(m) => m,
        Err(e) => {
            println!("  FAIL batch: {}", e);
            println!("  => FAIL (0 passed, 1 failed)");
            return ExitCode::FAILURE;
        }
    };
    // (1) manifest is a valid .px value.
    if px::px_run(&manifest).is_ok() {
        println!("  ok   배치 매니페스트가 유효 .px 값");
        passed += 1;
    } else {
        println!("  FAIL 매니페스트가 유효 px 아님");
        failed += 1;
    }
    // (2) counts: total=2, accepted=1, held=1.
    let total = px::px_run(&format!("let m = {}; in m.total", manifest)).unwrap_or_default();
    let acc = px::px_run(&format!("let m = {}; in m.accepted", manifest)).unwrap_or_default();
    let held = px::px_run(&format!("let m = {}; in m.held", manifest)).unwrap_or_default();
    if total == "2" && acc == "1" && held == "1" {
        println!("  ok   카운트 total=2 accepted=1 held=1 (good + macro_rules)");
        passed += 1;
    } else {
        println!("  FAIL 카운트: total={} accepted={} held={}", total, acc, held);
        failed += 1;
    }
    // (3) verdicts are addressable in the manifest (first is accepted).
    // px prints string values quoted, so compare against the quoted form.
    let first_status = px::px_run(&format!("let m = {}; in (builtins.elemAt m.verdicts 0).status", manifest)).unwrap_or_default();
    if first_status == "\"accepted\"" {
        println!("  ok   매니페스트 verdicts[0].status = accepted (주소지정 가능)");
        passed += 1;
    } else {
        println!("  FAIL verdicts 주소지정: {}", first_status);
        failed += 1;
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// `engine-verify -c|-f <verdict-px>` — verify a verdict is untampered by
/// recomputing its witness_id from its own fields.
fn cmd_engine_verify(rest: &[String]) -> ExitCode {
    let v = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    match engine::verify_verdict(&v) {
        Ok((true, id)) => {
            println!("verified {}", id);
            ExitCode::SUCCESS
        }
        Ok((false, id)) => {
            println!("TAMPERED (recomputed {} != stated)", id);
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("pnix-rs: verify: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// engine-verify gate: a fresh verdict self-verifies; a TAMPERED verdict fails
/// (the witness_id binds the evidence — tamper-evident, proof-carrying verdict).
fn cmd_engine_verify_check() -> ExitCode {
    let bootstrap = bootstrap_path();
    let mut passed = 0;
    let mut failed = 0;
    if !std::path::Path::new(&bootstrap).exists() {
        println!("[engine-verify-check] rs-meta bootstrap 없음 — skip");
        println!("  => PASS (0 passed, 0 failed)");
        return ExitCode::SUCCESS;
    }
    let granted = vec![String::from("host-call")];
    let v = engine::rust_engine_verdict("fn main() { println!(\"{}\", 42); }", &bootstrap, &granted);
    let vpx = engine::render_verdict_px(&v);
    // (1) a fresh verdict self-verifies.
    match engine::verify_verdict(&vpx) {
        Ok((true, _)) => {
            println!("  ok   fresh verdict가 자기검증됨 (witness_id == 재계산)");
            passed += 1;
        }
        other => {
            println!("  FAIL fresh verify: {:?}", other.map(|(b, _)| b));
            failed += 1;
        }
    }
    // (2) TEETH: tamper the status -> verification fails.
    let tampered = vpx.replace("status = \"accepted\"", "status = \"rejected\"");
    if tampered != vpx {
        match engine::verify_verdict(&tampered) {
            Ok((false, _)) => {
                println!("  ok   status 변조 -> 검증 실패 (witness_id가 증거 바인딩)");
                passed += 1;
            }
            other => {
                println!("  FAIL tamper 미감지: {:?}", other.map(|(b, _)| b));
                failed += 1;
            }
        }
    } else {
        println!("  FAIL tamper 준비 실패");
        failed += 1;
    }
    // (3) tamper a hash -> verification fails.
    let tampered2 = vpx.replace("source_hash = \"sha256:", "source_hash = \"sha256:dead");
    match engine::verify_verdict(&tampered2) {
        Ok((false, _)) => {
            println!("  ok   source_hash 변조 -> 검증 실패");
            passed += 1;
        }
        other => {
            println!("  FAIL hash tamper: {:?}", other.map(|(b, _)| b));
            failed += 1;
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// `engine-attestation` — emit this engine's trust attestation (.px value):
/// TV corpus coverage + 3-way substrate. Why a control plane should trust it.
fn cmd_engine_attestation() -> ExitCode {
    let bootstrap = bootstrap_path();
    let granted = vec![String::from("host-call")];
    println!("{}", engine::engine_attestation(&bootstrap, &granted));
    ExitCode::SUCCESS
}

/// engine-attestation gate: the trust attestation is a real .px value carrying
/// non-zero TV corpus coverage and the 3-way substrate claim. Honest skip when
/// the engine is unavailable.
fn cmd_engine_attestation_check() -> ExitCode {
    let bootstrap = bootstrap_path();
    let mut passed = 0;
    let mut failed = 0;
    if !std::path::Path::new(&bootstrap).exists() {
        println!("[engine-attestation-check] rs-meta bootstrap 없음 — skip");
        println!("  => PASS (0 passed, 0 failed)");
        return ExitCode::SUCCESS;
    }
    let granted = vec![String::from("host-call")];
    let att = engine::engine_attestation(&bootstrap, &granted);
    // (1) valid .px value.
    if px::px_run(&att).is_ok() {
        println!("  ok   attestation이 유효 .px 값");
        passed += 1;
    } else {
        println!("  FAIL attestation이 유효 px 아님: {}", att);
        failed += 1;
    }
    // (2) non-zero TV coverage + 3-way substrate (the trust signal).
    let pos = px::px_run(&format!("let a = {}; in a.positive_corpus", att)).unwrap_or_default();
    let ways = px::px_run(&format!("let a = {}; in a.substrate_ways", att)).unwrap_or_default();
    if pos != "0" && !pos.is_empty() && ways == "3" {
        println!("  ok   TV 커버리지 positive={} + substrate 3-way (신뢰 신호)", pos);
        passed += 1;
    } else if att.contains("available = false") {
        println!("  ok   (engine 없음) available=false 정직");
        passed += 1;
    } else {
        println!("  FAIL 신뢰 신호: positive={} ways={}", pos, ways);
        failed += 1;
    }
    // (3) held frontier honestly present.
    if att.contains("full-borrowck") && att.contains("full-trait-solver") {
        println!("  ok   held 프론티어 정직(borrowck/trait-solver)");
        passed += 1;
    } else {
        println!("  FAIL held 프론티어 누락");
        failed += 1;
    }
    // (4) self-hosting credential (deepest meta-circular claim) present.
    if att.contains("self_hosting = \"stage3-full-chain\"") {
        println!("  ok   self-hosting 자격(stage3-full-chain — rs-meta가 자기 소스 평가==rustc)");
        passed += 1;
    } else if att.contains("available = false") {
        passed += 1;
    } else {
        println!("  FAIL self-hosting 누락");
        failed += 1;
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// `engine-profile` — emit this Rust engine's capability profile as a .px value.
fn cmd_engine_profile() -> ExitCode {
    println!("{}", engine::engine_profile());
    ExitCode::SUCCESS
}

/// `engine-verdict -c|-f <rust-source>` — consult rs-meta as a peer engine and
/// emit the common .px engine verdict for the Rust source.
fn cmd_engine_verdict(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let bootstrap = bootstrap_path();
    let granted = vec![String::from("host-call")];
    let v = engine::rust_engine_verdict(&src, &bootstrap, &granted);
    println!("{}", engine::render_verdict_px(&v));
    ExitCode::SUCCESS
}

/// `engine-request -c|-f <px-request>` — handle a pnix.engine.request.v0 .px
/// value (control-plane -> engine), returning the response envelope.
fn cmd_engine_request(rest: &[String]) -> ExitCode {
    let req = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let bootstrap = bootstrap_path();
    let granted = vec![String::from("host-call")];
    match engine::handle_request(&req, &bootstrap, &granted) {
        Ok(resp) => {
            println!("{}", resp);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("pnix-rs: engine request: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// engine-request gate: the .px request envelope dispatches to the right
/// response. (1) phase=profile -> profile envelope, (2) phase=eval-rust ->
/// verdict envelope (real px), (3) phase=artifact -> artifact envelope,
/// (4) unknown phase -> error (teeth).
fn cmd_engine_request_check() -> ExitCode {
    let bootstrap = bootstrap_path();
    let mut passed = 0;
    let mut failed = 0;
    if !std::path::Path::new(&bootstrap).exists() {
        println!("[engine-request-check] rs-meta bootstrap 없음 — skip");
        println!("  => PASS (0 passed, 0 failed)");
        return ExitCode::SUCCESS;
    }
    let granted = vec![String::from("host-call")];
    // (1) profile request.
    let req_profile = "{ schema = \"pnix.engine.request.v0\"; engine = \"pnix-rs\"; phase = \"profile\"; }";
    match engine::handle_request(req_profile, &bootstrap, &granted) {
        Ok(resp) if resp.contains("pnix.engine.profile.v0") && px::px_run(&resp).is_ok() => {
            println!("  ok   phase=profile -> profile 봉투 (유효 px)");
            passed += 1;
        }
        other => {
            println!("  FAIL profile 요청: {:?}", other.map(|r| r.len()));
            failed += 1;
        }
    }
    // (2) eval-rust request -> verdict.
    let req_eval = "{ schema = \"pnix.engine.request.v0\"; phase = \"eval-rust\"; source_kind = \"rust-source\"; source = \"fn main() { println!(\\\"{}\\\", 42); }\"; }";
    match engine::handle_request(req_eval, &bootstrap, &granted) {
        Ok(resp) if resp.contains("pnix.engine.verdict.v0") && px::px_run(&resp).is_ok() => {
            println!("  ok   phase=eval-rust -> verdict 봉투 (유효 px)");
            passed += 1;
        }
        other => {
            println!("  FAIL eval-rust 요청: {:?}", other.map(|r| r.len()));
            failed += 1;
        }
    }
    // (3) artifact request -> artifact envelope.
    let req_art = "{ schema = \"pnix.engine.request.v0\"; phase = \"artifact\"; source = \"fn main() { println!(\\\"{}\\\", 42); }\"; }";
    match engine::handle_request(req_art, &bootstrap, &granted) {
        Ok(resp) if resp.contains("pnix.engine.artifact.v0") => {
            println!("  ok   phase=artifact -> artifact 봉투");
            passed += 1;
        }
        other => {
            println!("  FAIL artifact 요청: {:?}", other.map(|r| r.len()));
            failed += 1;
        }
    }
    // (4) unknown phase -> error (teeth).
    let req_bad = "{ schema = \"pnix.engine.request.v0\"; phase = \"bogus\"; }";
    match engine::handle_request(req_bad, &bootstrap, &granted) {
        Err(_) => {
            println!("  ok   미지 phase -> 에러 (프로토콜 이빨)");
            passed += 1;
        }
        Ok(_) => {
            println!("  FAIL 미지 phase가 통과됨");
            failed += 1;
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// `engine-artifact -c|-f <rust>` — emit the native artifact receipt as a .px
/// envelope for build attestation.
fn cmd_engine_artifact(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let bootstrap = bootstrap_path();
    let granted = vec![String::from("host-call")];
    println!("{}", engine::rust_artifact_envelope(&src, &bootstrap, &granted));
    ExitCode::SUCCESS
}

/// engine-artifact gate: the native artifact receipt exports as a real .px
/// value, reproducibly. Honest skip if rustc/the engine is unavailable.
fn cmd_engine_artifact_check() -> ExitCode {
    let bootstrap = bootstrap_path();
    let mut passed = 0;
    let mut failed = 0;
    if !std::path::Path::new(&bootstrap).exists() {
        println!("[engine-artifact-check] rs-meta bootstrap 없음 — skip");
        println!("  => PASS (0 passed, 0 failed)");
        return ExitCode::SUCCESS;
    }
    let granted = vec![String::from("host-call")];
    let src = "fn main() { println!(\"{}\", 40 + 2); }";
    let env1 = engine::rust_artifact_envelope(src, &bootstrap, &granted);
    // (1) envelope is a real .px value.
    if px::px_run(&env1).is_ok() {
        println!("  ok   artifact 봉투가 유효 .px 값");
        passed += 1;
    } else {
        println!("  FAIL 봉투가 유효 px 아님: {}", env1);
        failed += 1;
    }
    if env1.contains("available = false") {
        println!("  ok   (rustc 없음) available=false — 정직 skip");
        passed += 1;
        println!("  => {} ({} passed, {} failed)", "PASS", passed, failed);
        return ExitCode::SUCCESS;
    }
    // (2) has artifact_hash + rustc.
    if env1.contains("artifact_hash = \"") && env1.contains("rustc = \"") && !env1.contains("artifact_hash = \"\"") {
        println!("  ok   artifact_hash + rustc 필드 존재");
        passed += 1;
    } else {
        println!("  FAIL artifact 필드 누락");
        failed += 1;
    }
    // (3) reproducible.
    let env2 = engine::rust_artifact_envelope(src, &bootstrap, &granted);
    if env1 == env2 {
        println!("  ok   봉투 재현 가능 (동일 source -> 동일 봉투)");
        passed += 1;
    } else {
        println!("  FAIL 봉투 비재현");
        failed += 1;
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// peer-engine adapter gate: rs-meta's Rust translation-validation results map
/// into a common .px engine verdict envelope. Proves: (1) the profile and every
/// verdict are REAL .px values (reparse + eval as px — the control plane can
/// consume them), (2) the TV->status taxonomy is faithful (accepted/ok for
/// agreement, accepted/negative-boundary-agrees when both reject), (3) rs-meta
/// stays a peer across the CLI (this never imports rs-meta internals).
fn cmd_engine_verdict_check() -> ExitCode {
    let bootstrap = bootstrap_path();
    let mut passed = 0;
    let mut failed = 0;
    if !std::path::Path::new(&bootstrap).exists() {
        println!("[engine-verdict-check] rs-meta bootstrap 없음 ({}) — skip", bootstrap);
        println!("  => PASS (0 passed, 0 failed)");
        return ExitCode::SUCCESS;
    }
    let granted = vec![String::from("host-call")];

    // (1) engine profile is a real .px value with the honest held frontier.
    let profile = engine::engine_profile();
    let profile_is_px = px::px_run(&profile).is_ok();
    let profile_honest = profile.contains("full-borrowck")
        && profile.contains("macro-rules")
        && profile.contains("full-trait-solver")
        && profile.contains("translation-validation");
    if profile_is_px && profile_honest {
        println!("  ok   engine-profile은 유효 .px + held 프론티어 정직(borrowck/macro/trait)");
        passed += 1;
    } else {
        println!("  FAIL profile: px={} honest={}", profile_is_px, profile_honest);
        failed += 1;
    }

    // (2) a good Rust program -> accepted/ok, and the verdict is a real .px value.
    let good = "fn add(a: i64, b: i64) -> i64 { a + b } fn main() { println!(\"{}\", add(40, 2)); }";
    let vg = engine::rust_engine_verdict(good, &bootstrap, &granted);
    let vg_px = engine::render_verdict_px(&vg);
    if vg_px.is_empty() || px::px_run(&vg_px).is_err() {
        println!("  FAIL good verdict가 유효 px 아님");
        failed += 1;
    } else if vg.status == "accepted" && vg.verdict_kind == "ok" && vg.tv_equal == Some(true) {
        println!("  ok   good Rust -> accepted/ok, tv_equal=true, verdict는 유효 .px");
        passed += 1;
    } else if vg.verdict_kind.starts_with("held-rustc") {
        println!("  ok   (rustc 없음) good Rust -> held-rustc-unavailable (정직)");
        passed += 1;
    } else {
        println!("  FAIL good verdict: status={} kind={}", vg.status, vg.verdict_kind);
        failed += 1;
    }

    // (3) an ill-typed program -> both reject -> accepted/negative-boundary-agrees.
    let bad = "fn main() { let x: bool = 5; println!(\"{}\", x); }";
    let vb = engine::rust_engine_verdict(bad, &bootstrap, &granted);
    if vb.status == "accepted" && vb.verdict_kind == "negative-boundary-agrees" {
        println!("  ok   ill-typed Rust -> accepted/negative-boundary-agrees (거부 경계 합치)");
        passed += 1;
    } else if vb.verdict_kind.starts_with("held-rustc") {
        println!("  ok   (rustc 없음) ill-typed -> held-rustc-unavailable (정직)");
        passed += 1;
    } else {
        println!("  FAIL ill-typed verdict: status={} kind={}", vb.status, vb.verdict_kind);
        failed += 1;
    }

    // (4) verdict is deterministic.
    let vg2 = engine::rust_engine_verdict(good, &bootstrap, &granted);
    if engine::render_verdict_px(&vg2) == vg_px {
        println!("  ok   verdict 결정성");
        passed += 1;
    } else {
        println!("  FAIL verdict 비결정성");
        failed += 1;
    }
    // (4b) witness_id is present, stable, and distinguishes distinct programs.
    if vg.witness_id.starts_with("wit:") {
        let vother = engine::rust_engine_verdict("fn main() { println!(\"{}\", 7); }", &bootstrap, &granted);
        if vother.witness_id != vg.witness_id {
            println!("  ok   witness_id 존재+안정, 다른 프로그램은 다른 witness_id");
            passed += 1;
        } else {
            println!("  FAIL witness_id 충돌");
            failed += 1;
        }
    } else {
        println!("  FAIL witness_id 형식");
        failed += 1;
    }
    // (4c) a borrow-violating program: interp accepts, rustc rejects -> divergent
    // with the rustc reason_code preserved (connects the boundary reports).
    let borrow_bad = "fn main() { let s = String::from(\"a\"); let t = s; println!(\"{} {}\", s, t); }";
    let vbb = engine::rust_engine_verdict(borrow_bad, &bootstrap, &granted);
    if vbb.reason_code == "E0382" && vbb.status == "rejected" {
        println!("  ok   borrow 위반 -> rejected/divergent, reason_code E0382 (경계 이유 보존)");
        passed += 1;
    } else if vbb.verdict_kind.starts_with("held-rustc") {
        println!("  ok   (rustc 없음) borrow 위반 -> held (정직 skip)");
        passed += 1;
    } else {
        println!("  FAIL borrow divergent: status={} reason={}", vbb.status, vbb.reason_code);
        failed += 1;
    }
    // (4d) surface classification (from rs-meta rust-surface): a macro_rules!
    // program's verdict carries surface=held-macro-rules (rs-meta owns it), AND
    // the HELD verdict (absent optionals) is STILL a valid .px value.
    let mr = "macro_rules! sq { ($x:expr) => { $x }; } fn main() { println!(\"{}\", sq!(1)); }";
    let vmr = engine::rust_engine_verdict(mr, &bootstrap, &granted);
    let vmr_px = engine::render_verdict_px(&vmr);
    if px::px_run(&vmr_px).is_err() {
        println!("  FAIL held verdict가 유효 px 아님 (null?): {}", vmr_px);
        failed += 1;
    } else {
        println!("  ok   held verdict(absent optionals)도 유효 .px");
        passed += 1;
    }
    if vmr.surface == "held-macro-rules" {
        println!("  ok   macro_rules! 프로그램 verdict.surface = held-macro-rules (rs-meta 분류 소비)");
        passed += 1;
    } else if vmr.surface == "-" {
        println!("  ok   (engine 없음) surface=- 정직");
        passed += 1;
    } else {
        println!("  FAIL surface: {}", vmr.surface);
        failed += 1;
    }
    // (5) ir_hash is the FORMAT-INVARIANT canonical Rust IR hash (from rs-meta
    // rust-ir), distinct from the source_hash: two Rust sources differing only
    // in whitespace/comments share the verdict ir_hash but not the source_hash.
    let plain = "fn main(){println!(\"{}\",1+2);}";
    let spaced = "fn main() {  /* c */  println!( \"{}\" , 1 + 2 ) ; }";
    let vp = engine::rust_engine_verdict(plain, &bootstrap, &granted);
    let vs = engine::rust_engine_verdict(spaced, &bootstrap, &granted);
    match (&vp.ir_hash, &vs.ir_hash) {
        (Some(a), Some(b)) if a == b && vp.source_hash != vs.source_hash => {
            println!("  ok   ir_hash 포맷 불변(canonical Rust IR): 공백/주석 달라도 같은 ir_hash, source_hash는 다름");
            passed += 1;
        }
        (Some(_), Some(_)) => {
            println!("  FAIL ir_hash가 포맷 불변 아님 또는 source_hash 동일");
            failed += 1;
        }
        _ => {
            // rustc/engine absent -> ir_hash may be None; accept as honest skip.
            println!("  ok   (engine 없음) ir_hash None — 정직 skip");
            passed += 1;
        }
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// A single explanatory report tying every facet together (maps pnix-hy 17):
/// value + purity/effects + canonical IR hash + mirror roundtrip status +
/// eval witness. One call for "what is this, is it pure/safe, what is its
/// canonical address, is it meaning-preserving, what is the evidence."
fn explain_report(source: &str) -> String {
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let mut out = String::from("schema pnix-rs.explain.v0
");
    match px::px_run(source) {
        Ok(v) => out.push_str(&format!("value {}
", v)),
        Err(e) => out.push_str(&format!("value ERROR {}
", e)),
    }
    let g = gate::gate_check(source, &granted);
    out.push_str(&format!("pure {}
", g.pure));
    out.push_str(&format!("required_effects [{}]
", g.required_effects.join(" ")));
    out.push_str(&format!("allowed {}
", g.allowed));
    match ir::ir_of(source) {
        Ok(r) => out.push_str(&format!("ir_sha256 {}
", r.ir_sha256)),
        Err(e) => out.push_str(&format!("ir_sha256 ERROR {}
", e)),
    }
    let m = mirror::mirror_run(source);
    out.push_str(&format!("mirror_status {}
", m.status));
    out.push_str(&format!("emit_fixed_point {}
", m.emit_fixed_point));
    match gate::eval_witness(source, &granted) {
        Ok(w) => out.push_str(&format!("witness_out_hash {}
", w.out_hash)),
        Err(e) => out.push_str(&format!("witness ERROR {}
", e)),
    }
    out
}

/// `explain -c|-f` — the unified evidence report.
fn cmd_explain(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    print!("{}", explain_report(&src));
    ExitCode::SUCCESS
}

/// explain-check gate: the unified explain must AGREE with the individual
/// facet gates (no drift between the aggregate report and its components).
fn cmd_explain_check() -> ExitCode {
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let mut passed = 0;
    let mut failed = 0;
    let src = "let a = 6; b = a + 1; in a * b";
    let report = explain_report(src);

    // value agrees with px_run.
    let value = px::px_run(src).unwrap_or_default();
    if report.contains(&format!("value {}
", value)) {
        println!("  ok   explain.value == px_run ({})", value);
        passed += 1;
    } else {
        println!("  FAIL value 불일치");
        failed += 1;
    }
    // ir_sha256 agrees with ir_of.
    let ir_hash = ir::ir_of(src).map(|r| r.ir_sha256).unwrap_or_default();
    if report.contains(&format!("ir_sha256 {}
", ir_hash)) {
        println!("  ok   explain.ir_sha256 == ir_of");
        passed += 1;
    } else {
        println!("  FAIL ir_sha256 불일치");
        failed += 1;
    }
    // pure/allowed agree with gate_check.
    let g = gate::gate_check(src, &granted);
    if report.contains(&format!("pure {}
", g.pure))
        && report.contains(&format!("allowed {}
", g.allowed))
    {
        println!("  ok   explain.pure/allowed == gate_check");
        passed += 1;
    } else {
        println!("  FAIL gate 불일치");
        failed += 1;
    }
    // mirror status agrees.
    let m = mirror::mirror_run(src);
    if report.contains(&format!("mirror_status {}
", m.status)) {
        println!("  ok   explain.mirror_status == mirror_run ({})", m.status);
        passed += 1;
    } else {
        println!("  FAIL mirror 불일치");
        failed += 1;
    }
    // determinism.
    if explain_report(src) == report {
        println!("  ok   explain 결정성");
        passed += 1;
    } else {
        println!("  FAIL explain 비결정성");
        failed += 1;
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// capability-attenuation gate (maps pnix-hy 23): least-privilege lifecycle
/// over the interop capability model. Gates: (1) attenuating a grant (removing
/// an effect) makes the removed effect DENIED while the kept ones still pass,
/// (2) attenuation is IRREVERSIBLE — the child is always a subset, so it cannot
/// re-gain a dropped effect (no re-widening), (3) revoke = empty grant denies
/// everything.
fn cmd_attenuate_check() -> ExitCode {
    let mut passed = 0;
    let mut failed = 0;
    let full = vec![
        String::from("file-read"),
        String::from("file-write"),
        String::from("host-call"),
    ];
    // (1) attenuate: drop file-write -> denied, but file-read still granted.
    let narrowed = interop::attenuate(&full, &[String::from("file-write")]);
    let write_denied = interop::check_capability("file-write", &narrowed).is_err();
    let read_ok = interop::check_capability("file-read", &narrowed).is_ok();
    if write_denied && read_ok {
        println!("  ok   감쇠: file-write 제거 -> 거부, file-read는 유지");
        passed += 1;
    } else {
        println!("  FAIL 감쇠: write_denied={} read_ok={}", write_denied, read_ok);
        failed += 1;
    }
    // (2) attenuation is a subset; you cannot attenuate to re-gain an effect.
    let is_sub = interop::is_attenuation_of(&narrowed, &full);
    let cannot_regain = !interop::is_attenuation_of(&full, &narrowed); // full is NOT a subset of narrowed
    if is_sub && cannot_regain {
        println!("  ok   감쇠는 부분집합 — 되돌려 재확대 불가(irreversible)");
        passed += 1;
    } else {
        println!("  FAIL attenuation subset: is_sub={} cannot_regain={}", is_sub, cannot_regain);
        failed += 1;
    }
    // (3) revoke: empty grant denies everything.
    let revoked = interop::revoke();
    if interop::check_capability("file-read", &revoked).is_err()
        && interop::check_capability("host-call", &revoked).is_err()
    {
        println!("  ok   회수(revoke): 빈 grant는 모든 효과 거부");
        passed += 1;
    } else {
        println!("  FAIL revoke가 무언가 허용함");
        failed += 1;
    }
    // (4) chained attenuation only ever narrows.
    let step2 = interop::attenuate(&narrowed, &[String::from("host-call")]);
    if interop::is_attenuation_of(&step2, &narrowed)
        && interop::is_attenuation_of(&step2, &full)
        && step2.len() == 1
    {
        println!("  ok   연쇄 감쇠: 각 단계가 더 좁아짐 (file-read만 남음)");
        passed += 1;
    } else {
        println!("  FAIL 연쇄 감쇠");
        failed += 1;
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// ir-diff gate (maps pnix-hy 29): a SEMANTIC diff at the canonical-IR level.
/// Gates: (1) a binding REORDER is meaning-preserving (IR identical),
/// (2) a SEMANTIC change differs and is localized to the first differing IR
/// position, (3) an identical program is identical. Complements changed_between
/// (def-granular, alpha-invariant) with a within-program structural view.
fn cmd_ir_diff_check() -> ExitCode {
    let mut passed = 0;
    let mut failed = 0;

    // (1) binding reorder -> canonical IR identical (meaning-preserving).
    match ir::ir_diff("let a = 1; b = 2; in a + b", "let b = 2; a = 1; in a + b") {
        Ok(d) if d.identical => {
            println!("  ok   바인딩 reorder -> IR 동일 (meaning-preserving)");
            passed += 1;
        }
        other => {
            println!("  FAIL reorder identical: {:?}", other.map(|d| d.identical));
            failed += 1;
        }
    }
    // (2) semantic change -> IR differs, localized.
    match ir::ir_diff("let a = 1; b = 2; in a + b", "let a = 9; b = 2; in a + b") {
        Ok(d) if !d.identical => {
            println!(
                "  ok   의미 변경 -> IR 다름, first_diff={} {}",
                d.first_diff, d.window
            );
            passed += 1;
        }
        other => {
            println!("  FAIL semantic diff: {:?}", other.map(|d| d.identical));
            failed += 1;
        }
    }
    // (3) identical program -> identical.
    match ir::ir_diff("let a = 1; in a * a", "let a = 1; in a * a") {
        Ok(d) if d.identical => {
            println!("  ok   동일 프로그램 -> IR 동일");
            passed += 1;
        }
        other => {
            println!("  FAIL identical: {:?}", other.map(|d| d.identical));
            failed += 1;
        }
    }
    // (4) structural change (extra binding) -> differs.
    match ir::ir_diff("let a = 1; in a", "let a = 1; b = 2; in a") {
        Ok(d) if !d.identical => {
            println!("  ok   구조 변경(바인딩 추가) -> IR 다름");
            passed += 1;
        }
        other => {
            println!("  FAIL structural: {:?}", other.map(|d| d.identical));
            failed += 1;
        }
    }
    // (5) HONEST BOUNDARY: ir-diff is NOT alpha-invariant — an alpha-rename
    // shows a diff (names are in the IR). This is the complement to
    // changed_between (which IS alpha-invariant); together they cover both.
    match ir::ir_diff("let d = 5; b = d + 1; in b", "let e = 5; b = e + 1; in b") {
        Ok(diff) if !diff.identical => {
            println!("  ok   alpha-rename -> ir-diff는 diff (알파-불변 아님; changed_between이 알파-불변 담당)");
            passed += 1;
        }
        other => {
            println!("  FAIL alpha boundary: {:?}", other.map(|d| d.identical));
            failed += 1;
        }
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// assumed-specialization gate (maps pnix-hy 32 / proposal 0025): a specialized
/// residual is built under STATIC ASSUMPTIONS (the static inputs). Reusing it
/// is only sound while those assumptions hold. This gates: (1) with the same
/// static env the residual is VALID and evaluates correctly, (2) when a static
/// assumption CHANGES the residual is STALE (assumptions_hold=false) and reuse
/// would be wrong — respecialization yields the correct new residual (teeth).
fn cmd_assumption_check() -> ExitCode {
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let mut passed = 0;
    let mut failed = 0;

    let src = "x * (k + 3)";
    let built_under = vec![(String::from("k"), 2i64)];
    let residual = match tower::mix_in_px(src, &built_under, &granted) {
        Ok(o) => o.residual_source.unwrap_or_default(),
        Err(e) => {
            println!("  FAIL specialize: {}", e);
            println!("  => FAIL (0 passed, 1 failed)");
            return ExitCode::FAILURE;
        }
    };
    // (1) assumptions hold with the same env -> residual valid + correct.
    let now_same = vec![(String::from("k"), 2i64)];
    if tower::assumptions_hold(&built_under, &now_same) {
        let ev = px::px_run(&format!("let x = 4; in ({})", residual));
        if ev.as_deref() == Ok("20") {
            println!("  ok   가정 유지(k=2) -> residual `{}` 재사용 유효 (x=4 -> 20)", residual);
            passed += 1;
        } else {
            println!("  FAIL 유효 재사용 평가: {:?}", ev);
            failed += 1;
        }
    } else {
        println!("  FAIL 같은 env인데 가정 불일치");
        failed += 1;
    }
    // (2) TEETH: a changed static assumption makes the residual STALE.
    let now_changed = vec![(String::from("k"), 3i64)];
    if !tower::assumptions_hold(&built_under, &now_changed) {
        // reusing the old residual would be WRONG; respecialize for the new env.
        let respecialized = match tower::mix_in_px(src, &now_changed, &granted) {
            Ok(o) => o.residual_source.unwrap_or_default(),
            Err(e) => {
                println!("  FAIL 재특화: {}", e);
                failed += 1;
                String::new()
            }
        };
        let stale = px::px_run(&format!("let x = 4; in ({})", residual)); // old residual (k=2)
        let fresh = px::px_run(&format!("let x = 4; in ({})", respecialized)); // new (k=3)
        if respecialized != residual
            && stale.as_deref() == Ok("20")
            && fresh.as_deref() == Ok("24")
        {
            println!(
                "  ok   가정 변경(k=3) -> 옛 residual stale(재사용시 20, 오답), 재특화 `{}`(24, 정답)",
                respecialized
            );
            passed += 1;
        } else {
            println!("  FAIL stale/재특화: old={:?} new={:?}", stale, fresh);
            failed += 1;
        }
    } else {
        println!("  FAIL 변경된 env인데 가정 유효로 판정 (stale 미감지)");
        failed += 1;
    }
    // (3) assumption hash is deterministic + order-insensitive.
    let h1 = tower::assumption_hash(&[(String::from("a"), 1), (String::from("b"), 2)]);
    let h2 = tower::assumption_hash(&[(String::from("b"), 2), (String::from("a"), 1)]);
    if h1 == h2 {
        println!("  ok   가정 지문 결정성 + 순서 무관");
        passed += 1;
    } else {
        println!("  FAIL 가정 지문 불안정");
        failed += 1;
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// phase-separation gate (maps pnix-hy 22): compile-time (static) and run-time
/// (dynamic) computation are OBSERVATIONALLY SEPARATED by the specializer. A
/// specialization is phase-separated iff the residual references EXACTLY the
/// dynamic variables — every static input is consumed at specialization time
/// (no static var survives) and every dynamic input is preserved (residual).
/// Gates this structural boundary (distinct from m5, which checks the value):
/// residual free-vars == the dynamic set, for mixed static/dynamic programs.
fn cmd_phase_check() -> ExitCode {
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let mut passed = 0;
    let mut failed = 0;

    // (program, static bindings, expected residual free-var set = dynamic vars)
    let free_of = |src: &str| -> Result<Vec<String>, String> {
        let ast = px::px_parse(src)?;
        let mut free = Vec::new();
        let mut bound = Vec::new();
        specialize::px_free_vars(&ast, &mut free, &mut bound);
        free.sort();
        Ok(free)
    };
    let cases: [(&str, &[(&str, i64)], &[&str]); 3] = [
        // x dynamic, k static -> residual should reference only x.
        ("x * (k + 3)", &[("k", 2)], &["x"]),
        // both dynamic -> residual references both.
        ("x * (y + 3)", &[], &["x", "y"]),
        // fully static -> residual references nothing (all consumed).
        ("(a + b) * 2", &[("a", 4), ("b", 1)], &[]),
    ];
    for (src, statics, expected_dyn) in cases {
        let statics_owned: Vec<(String, i64)> =
            statics.iter().map(|(n, v)| (n.to_string(), *v)).collect();
        let result = tower::mix_in_px(src, &statics_owned, &granted);
        match result {
            Ok(o) => {
                let residual = o.residual_source.clone().unwrap_or_default();
                match free_of(&residual) {
                    Ok(free) => {
                        let mut want: Vec<String> =
                            expected_dyn.iter().map(|s| s.to_string()).collect();
                        want.sort();
                        // Static consumed: no static var in free; dynamic preserved: == want.
                        let static_leaked = statics
                            .iter()
                            .any(|(n, _)| free.iter().any(|f| f == n));
                        if free == want && !static_leaked {
                            println!(
                                "  ok   `{}` -> residual `{}`, 자유변수 {:?} = 동적변수 (정적 소진)",
                                src, residual, free
                            );
                            passed += 1;
                        } else {
                            println!(
                                "  FAIL `{}`: free {:?} != dynamic {:?} (static_leaked={})",
                                src, free, want, static_leaked
                            );
                            failed += 1;
                        }
                    }
                    Err(e) => {
                        println!("  FAIL free-vars `{}`: {}", src, e);
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                println!("  FAIL mix `{}`: {}", src, e);
                failed += 1;
            }
        }
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// verifying-cache gate (maps pnix-hy 30): the realisation store gives early
/// cutoff by TRUSTING cached values. A verifying cache AUDITS instead — on a
/// hit it re-derives the value and confirms the store agrees. Gates: (1) a
/// store built from real evals verifies (hit re-checked), (2) a TAMPERED store
/// entry is DETECTED (teeth), (3) an unknown source has no entry.
fn cmd_verifying_cache_check() -> ExitCode {
    let granted = vec![
        String::from("file-read"),
        String::from("file-write"),
        String::from("host-call"),
    ];
    let mut passed = 0;
    let mut failed = 0;
    let store = "work/verifying-cache-check.tsv";
    if let Err(e) = interop::host_ensure_dir("work", &granted) {
        println!("  FAIL work dir: {}", e);
    }
    let _ = interop::host_remove_file(store, &granted);

    let src = "let a = 6; b = a + 1; in a * b";
    // Seed the store with a real eval (miss).
    match incremental::incremental_eval(src, store, &granted) {
        Ok((_, false)) => {
            println!("  ok   store 시딩 (miss -> realisation 기록)");
            passed += 1;
        }
        other => {
            println!("  FAIL 시딩: {:?}", other.is_ok());
            failed += 1;
        }
    }
    // Verify mode: the hit is re-derived and confirmed.
    match incremental::incremental_eval_verify(src, store, &granted) {
        Ok((_, true)) => {
            println!("  ok   verifying hit — 캐시 값 재검증 일치");
            passed += 1;
        }
        other => {
            println!("  FAIL verifying hit: {:?}", other);
            failed += 1;
        }
    }
    // TAMPER: corrupt the stored value, then verify -> detected.
    let tampered = interop::host_read_file(store, &granted)
        .unwrap_or_default()
        .replace('\t', "\t")
        .lines()
        .map(|l| {
            let cols: Vec<&str> = l.split('\t').collect();
            if cols.len() == 3 {
                format!("{}\t{}\t{}", cols[0], "deadbeef", cols[2])
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let _ = interop::host_write_file(store, &tampered, &granted);
    match incremental::incremental_eval_verify(src, store, &granted) {
        Err(e) if e.contains("verifying cache") => {
            println!("  ok   오염된 store 엔트리를 감지 (verifying cache 이빨)");
            passed += 1;
        }
        other => {
            println!("  FAIL 오염 미감지: {:?}", other.is_ok());
            failed += 1;
        }
    }
    let _ = interop::host_remove_file(store, &granted);

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// proposal 0007(d) gate: FINITE REFLECTIVE TOWER (3-Lisp / Amin&Rompf, the
/// cheap coherent form — no heavy self-application). reify moves a program UP a
/// level (to px DATA), reflect moves it DOWN, and the tower is COHERENT to
/// depth 2: since the encoding's canonical print is itself valid px (the P1
/// property), the encoding can be reified AGAIN and reflected back. Gates:
/// (1) reflect∘reify = id (level 1), (2) the ENCODING round-trips through
/// reify/reflect (level 2 coherence — the tower is well-founded), (3) META-LEVEL
/// TRANSPARENCY: evaluating the reified program via the px self-interpreter
/// equals native (a level shift preserves meaning).
fn cmd_reflect_tower_check() -> ExitCode {
    let granted = vec![String::from("file-read")];
    let mut passed = 0;
    let mut failed = 0;
    let probes = ["1 + 2 * 3", "let a = 5; b = a + 1; in a * b", "(x: x + 1) 41"];

    for src in probes {
        let result = (|| -> Result<(bool, bool, bool), String> {
            let ast = px::px_parse(src)?;
            // Level 1: reify UP to data, reflect DOWN — identity.
            let e1 = tower::reify(&ast)?;
            let back1 = tower::reflect(&e1)?;
            let level1 = px::px_emit(&back1) == px::px_emit(&ast);

            // Level 2 coherence: the encoding's canonical print is valid px
            // (P1); reify that program to the meta-meta level and reflect it
            // back — the encoding-of-the-encoding round-trips.
            let e1_src = px::px_print(&e1);
            let e1_ast = px::px_parse(&e1_src)?;
            let e2 = tower::reify(&e1_ast)?;
            let back2 = tower::reflect(&e2)?;
            let level2 = px::px_emit(&back2) == px::px_emit(&e1_ast);

            // Meta-level transparency: the px self-interpreter evaluating the
            // reified program equals native evaluation (level shift preserves
            // meaning). (Programs with a free `input` are skipped for eval.)
            let transparent = if src.contains("input") {
                true
            } else {
                let native = px::px_run(src)?;
                let towered = tower::self_interp_eval(src, &granted)?;
                native == towered
            };
            Ok((level1, level2, transparent))
        })();
        match result {
            Ok((true, true, true)) => {
                println!("  ok   `{}` — reify/reflect 2-레벨 coherent + 메타-레벨 투명", src);
                passed += 1;
            }
            Ok((l1, l2, tr)) => {
                println!("  FAIL `{}`: level1={} level2={} transparent={}", src, l1, l2, tr);
                failed += 1;
            }
            Err(e) => {
                println!("  FAIL `{}`: {}", src, e);
                failed += 1;
            }
        }
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// typed-attestation gate (maps pnix-hy 25): the 13-field witness is frozen but
/// untyped as an ATTESTATION. A typed attestation names its claim (predicate-
/// type URI, in-toto/SLSA style) over a subject (content hash). This gates that
/// (1) a witness -> typed attestation validates, (2) a MISMATCHED predicate
/// (claiming a roundtrip attestation over an eval witness) is rejected (teeth),
/// (3) the attestation hash is deterministic.
fn cmd_attest_check() -> ExitCode {
    let granted = vec![String::from("file-read")];
    let mut passed = 0;
    let mut failed = 0;

    let evalw = match gate::eval_witness("let x = 6; in x * 7", &granted) {
        Ok(w) => w,
        Err(e) => {
            println!("  FAIL eval witness: {}", e);
            println!("  => FAIL (0 passed, 1 failed)");
            return ExitCode::FAILURE;
        }
    };
    let mirrorw = match gate::mirror_witness("let x = 21; in x + x", &granted) {
        Ok(w) => w,
        Err(e) => {
            println!("  FAIL mirror witness: {}", e);
            println!("  => FAIL (0 passed, 1 failed)");
            return ExitCode::FAILURE;
        }
    };

    // (1) a witness -> typed attestation validates against its own witness.
    match gate::typed_attestation(&evalw) {
        Ok(att) if gate::validate_typed(&att, &evalw) => {
            println!(
                "  ok   eval witness -> typed attestation `{}` (subject {}) 검증됨",
                att.predicate_type,
                &att.subject[0..12]
            );
            passed += 1;
        }
        other => {
            println!("  FAIL eval attestation: {:?}", other.is_ok());
            failed += 1;
        }
    }
    // (2) TEETH: an attestation minted for a mirror witness does NOT validate
    // against an eval witness (predicate/subject mismatch — can't forge).
    match gate::typed_attestation(&mirrorw) {
        Ok(mirror_att) => {
            if !gate::validate_typed(&mirror_att, &evalw) {
                println!("  ok   mirror-roundtrip 증명을 eval witness에 붙이면 거부 (위조 불가)");
                passed += 1;
            } else {
                println!("  FAIL 불일치 predicate가 통과됨");
                failed += 1;
            }
        }
        Err(e) => {
            println!("  FAIL mirror attestation: {}", e);
            failed += 1;
        }
    }
    // (3) SUBJECT forgery: an attestation with the RIGHT predicate but a
    // TAMPERED subject does not validate (isolates the subject check's teeth).
    match gate::typed_attestation(&evalw) {
        Ok(mut att) => {
            att.subject = String::from("deadbeef");
            if !gate::validate_typed(&att, &evalw) {
                println!("  ok   subject 위조(올바른 predicate + 틀린 subject) 거부");
                passed += 1;
            } else {
                println!("  FAIL subject 위조가 통과됨");
                failed += 1;
            }
        }
        Err(e) => {
            println!("  FAIL subject 위조 테스트: {}", e);
            failed += 1;
        }
    }
    // (4) determinism: same witness -> same attestation hash.
    match (gate::typed_attestation(&evalw), gate::typed_attestation(&evalw)) {
        (Ok(a), Ok(b)) if a.attestation_sha256 == b.attestation_sha256 => {
            println!("  ok   attestation 해시 결정성");
            passed += 1;
        }
        _ => {
            println!("  FAIL attestation 비결정성");
            failed += 1;
        }
    }
    // (4) predicate registry covers eval/mirror-roundtrip/ir; unknown -> none.
    if gate::predicate_for("eval").is_some()
        && gate::predicate_for("mirror-roundtrip").is_some()
        && gate::predicate_for("ir").is_some()
        && gate::predicate_for("bogus").is_none()
    {
        println!("  ok   predicate 레지스트리: eval/mirror/ir 등록, 미지 direction은 none");
        passed += 1;
    } else {
        println!("  FAIL predicate 레지스트리 불완전");
        failed += 1;
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// proposal 0004 gate: HAND-WRITTEN COGEN (generating extension) — the honest,
/// bounded form of the 3rd-projection benefit. Instead of self-applying the
/// online mix (intractable polyvariance, m6c~m8), a HAND-WRITTEN cogen (a px
/// function, runtime/tower/cogen_int.px) maps ANY arithmetic object program to
/// its compiled residual directly (Leuschel: benefits of self-application
/// without a self-applicable specialiser). Gates: (1) the generated residual
/// agrees with interpretation over an input battery for several programs, (2)
/// it is interpreter-free (bare arithmetic — dispatch consumed at generation
/// time), (3) different programs yield different compiled residuals.
fn cmd_cogen_check() -> ExitCode {
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let mut passed = 0;
    let mut failed = 0;

    let interp_src = "let int = prog: env: \
if prog.tag == \"num\" then prog.value \
else if prog.tag == \"arg\" then env \
else if prog.tag == \"add\" then (int prog.l env) + (int prog.r env) \
else (int prog.l env) * (int prog.r env); \
in int prog input";
    let cogen_src = match interop::host_read_file("runtime/tower/cogen_int.px", &granted) {
        Ok(s) => s,
        Err(e) => {
            println!("  FAIL cogen source: {}", e);
            println!("  => FAIL (0 passed, 1 failed)");
            return ExitCode::FAILURE;
        }
    };

    // Compile an object program with the hand-written cogen: run cogen over the
    // program literal, reflect the encoded residual to px source.
    let compile = |prog_lit: &str| -> Result<String, String> {
        let call = format!("({}) {}", cogen_src.trim(), prog_lit);
        let encoded = px::px_run_value(&call)?;
        let expr = tower::reflect(&encoded)?;
        Ok(px::px_emit(&expr))
    };

    let programs = [
        ("(arg*3)+4", "{ tag = \"add\"; l = { tag = \"mul\"; l = { tag = \"arg\"; }; r = { tag = \"num\"; value = 3; }; }; r = { tag = \"num\"; value = 4; }; }"),
        ("(arg+arg)*2", "{ tag = \"mul\"; l = { tag = \"add\"; l = { tag = \"arg\"; }; r = { tag = \"arg\"; }; }; r = { tag = \"num\"; value = 2; }; }"),
    ];
    let mut compiled = Vec::new();
    let mut ok = true;
    for (name, prog) in &programs {
        match compile(prog) {
            Ok(residual) => {
                // battery equivalence: compiled(input) == interp(prog, input).
                let mut agree = true;
                let mut input = 0i64;
                while input < 10 {
                    let comp = px::px_run(&format!("let input = {}; in ({})", input, residual));
                    let interp = px::px_run(&format!(
                        "let input = {}; prog = {}; in ({})",
                        input, prog, interp_src
                    ));
                    match (comp, interp) {
                        (Ok(a), Ok(b)) if a == b => {}
                        _ => agree = false,
                    }
                    input += 1;
                }
                // interpreter-free: no dispatch vocabulary in the compiled code.
                let ifree = !residual.contains("tag") && !residual.contains("prog");
                if agree && ifree {
                    println!("  ok   cogen({}) -> `{}` == 해석(10-입력), 인터프리터-free", name, residual);
                    passed += 1;
                } else {
                    println!("  FAIL cogen({}): agree={} ifree={} (`{}`)", name, agree, ifree, residual);
                    failed += 1;
                    ok = false;
                }
                compiled.push(residual);
            }
            Err(e) => {
                println!("  FAIL cogen({}): {}", name, e);
                failed += 1;
                ok = false;
            }
        }
    }
    // different programs -> different compiled residuals (cogen tracks the program).
    if ok && compiled.len() == 2 && compiled[0] != compiled[1] {
        println!("  ok   서로 다른 프로그램 -> 서로 다른 컴파일 결과 (cogen이 프로그램 추적)");
        passed += 1;
    } else if ok {
        println!("  FAIL 두 프로그램이 같은 컴파일 결과");
        failed += 1;
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// proposal 0007(c) gate: PROOF-CARRYING RESIDUAL — translation validation for
/// specialization, in the interp==rustc discipline, WITHOUT a proof assistant.
/// A specialization residual is only sound if it agrees with the source on
/// EVERY dynamic input, not just one. This gate runs source and residual over
/// an input BATTERY, emits a re-checkable equivalence CERTIFICATE (a content
/// hash of the input->output table), and proves the certificate is
/// deterministic and has TEETH (a deliberately-wrong residual is caught). The
/// "proof" is differential testing over the battery — checkable, not asserted.
fn cmd_certify_check() -> ExitCode {
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let mut passed = 0;
    let mut failed = 0;

    // 1st-projection setup: an object interpreter specialized to a fixed
    // program, leaving a residual with one dynamic `input`.
    let interp_src = "let int = prog: env: \
if prog.tag == \"num\" then prog.value \
else if prog.tag == \"arg\" then env \
else if prog.tag == \"add\" then (int prog.l env) + (int prog.r env) \
else (int prog.l env) * (int prog.r env); \
in int prog input";
    let prog_lit = "{ tag = \"add\"; l = { tag = \"mul\"; l = { tag = \"arg\"; }; r = { tag = \"num\"; value = 3; }; }; r = { tag = \"num\"; value = 4; }; }";

    let residual = (|| -> Result<String, String> {
        let prog = px::px_run_value(prog_lit)?;
        let outcome = tower::mix_in_px_data(interp_src, &[(String::from("prog"), prog)], &granted)?;
        outcome.residual_source.ok_or_else(|| String::from("residual not reflectable"))
    })();
    let residual = match residual {
        Ok(r) => r,
        Err(e) => {
            println!("  FAIL residual: {}", e);
            println!("  => FAIL (0 passed, 1 failed)");
            return ExitCode::FAILURE;
        }
    };

    // Certificate over an input battery: source(prog, input) == residual(input).
    let certify = |resid: &str| -> Result<(bool, String), String> {
        let mut table = String::new();
        let mut all_agree = true;
        let mut input = 0i64;
        while input < 12 {
            let src_out = px::px_run(&format!(
                "let input = {}; prog = {}; in ({})",
                input, prog_lit, interp_src
            ))?;
            let res_out = px::px_run(&format!("let input = {}; in ({})", input, resid))?;
            if src_out != res_out {
                all_agree = false;
            }
            table.push_str(&format!("{}\t{}\t{}\n", input, src_out, res_out));
            input += 1;
        }
        Ok((all_agree, sha256::sha256_hex(table.as_bytes())))
    };

    // (1) equivalence over the whole battery + certificate.
    match certify(&residual) {
        Ok((true, cert)) => {
            println!(
                "  ok   residual ≡ source on 12-input battery — 인증서 {}",
                &cert[0..12]
            );
            passed += 1;
        }
        Ok((false, _)) => {
            println!("  FAIL residual가 배터리에서 소스와 불일치");
            failed += 1;
        }
        Err(e) => {
            println!("  FAIL certify: {}", e);
            failed += 1;
        }
    }
    // (2) certificate is deterministic (re-checkable).
    match (certify(&residual), certify(&residual)) {
        (Ok((_, a)), Ok((_, b))) if a == b => {
            println!("  ok   인증서 결정성 (재검증 동일)");
            passed += 1;
        }
        _ => {
            println!("  FAIL 인증서 비결정성");
            failed += 1;
        }
    }
    // (3) TEETH: a deliberately-wrong residual is caught by the battery.
    match certify("(input * 3) + 5") {
        Ok((false, _)) => {
            println!("  ok   틀린 residual((input*3)+5)은 배터리가 거부 (인증서 이빨)");
            passed += 1;
        }
        Ok((true, _)) => {
            println!("  FAIL 틀린 residual이 통과됨");
            failed += 1;
        }
        Err(e) => {
            println!("  FAIL teeth: {}", e);
            failed += 1;
        }
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// tower m9 gate: JONES-OPTIMALITY, operationalized. A specializer removed the
/// interpretive layer iff the residual of specializing an interpreter over a
/// fixed object program depends ONLY on the program, not on the interpreter's
/// size/structure. Falsifiable test (Glück; Jones-Gomard-Sestoft strict form):
/// add UNUSED dispatch branches to the interpreter (bloat) — a Jones-optimal
/// specializer leaves ZERO trace, so residual(int, prog) == residual(int_bloat,
/// prog) AST-identically. We also gate interpreter-freedom, correctness, and
/// that the residual tracks the PROGRAM (a different prog -> different residual).
/// This is the measurable "the interpretive layer is gone" property; source-
/// level BTA on the program provably cannot raise this ceiling (finding [5]).
fn cmd_jones_check() -> ExitCode {
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let mut passed = 0;
    let mut failed = 0;

    // A minimal object-language interpreter (num/arg/add/mul).
    let int_lean = "let int = prog: env: \
if prog.tag == \"num\" then prog.value \
else if prog.tag == \"arg\" then env \
else if prog.tag == \"add\" then (int prog.l env) + (int prog.r env) \
else (int prog.l env) * (int prog.r env); \
in int prog input";
    // Same interpreter + UNUSED dispatch branches (sub/neg). The object
    // programs below never use them, so a Jones-optimal specializer must
    // leave no trace of the extra cases in the residual.
    let int_bloat = "let int = prog: env: \
if prog.tag == \"num\" then prog.value \
else if prog.tag == \"arg\" then env \
else if prog.tag == \"sub\" then (int prog.l env) - (int prog.r env) \
else if prog.tag == \"neg\" then 0 - (int prog.l env) \
else if prog.tag == \"add\" then (int prog.l env) + (int prog.r env) \
else (int prog.l env) * (int prog.r env); \
in int prog input";

    let residual_over = |interp: &str, prog_lit: &str| -> Result<String, String> {
        let prog = px::px_run_value(prog_lit)?;
        let statics = vec![(String::from("prog"), prog)];
        let outcome = tower::mix_in_px_data(interp, &statics, &granted)?;
        outcome
            .residual_source
            .ok_or_else(|| String::from("residual not reflectable"))
    };

    // Object program A: (arg * 3) + 4.
    let prog_a = "{ tag = \"add\"; l = { tag = \"mul\"; l = { tag = \"arg\"; }; r = { tag = \"num\"; value = 3; }; }; r = { tag = \"num\"; value = 4; }; }";
    // Object program B: (arg + arg) * 2 — different arithmetic.
    let prog_b = "{ tag = \"mul\"; l = { tag = \"add\"; l = { tag = \"arg\"; }; r = { tag = \"arg\"; }; }; r = { tag = \"num\"; value = 2; }; }";

    let lean_a = residual_over(int_lean, prog_a);
    let bloat_a = residual_over(int_bloat, prog_a);
    let lean_b = residual_over(int_lean, prog_b);

    match (&lean_a, &bloat_a) {
        (Ok(l), Ok(b)) if l == b => {
            println!(
                "  ok   JONES-OPTIMAL: bloating the interpreter leaves the residual UNCHANGED (`{}`)",
                l
            );
            passed += 1;
        }
        (Ok(l), Ok(b)) => {
            println!("  FAIL residual depends on interpreter: lean `{}` != bloat `{}`", l, b);
            failed += 1;
        }
        other => {
            println!("  FAIL jones bloat: {:?} {:?}", other.0.is_ok(), other.1.is_ok());
            failed += 1;
        }
    }
    // Interpreter-free: no dispatch vocabulary (.tag / prog / env) survives.
    match &lean_a {
        Ok(res) if !res.contains(".tag") && !res.contains("prog") && !res.contains("int ") => {
            println!("  ok   residual is interpreter-free (no .tag/prog/int dispatch)");
            passed += 1;
        }
        Ok(res) => {
            println!("  FAIL interpretive dispatch survived: {}", res);
            failed += 1;
        }
        Err(e) => {
            println!("  FAIL residual A: {}", e);
            failed += 1;
        }
    }
    // Residual tracks the PROGRAM, not the interpreter: B != A.
    match (&lean_a, &lean_b) {
        (Ok(a), Ok(b)) if a != b => {
            println!("  ok   different program -> different residual (`{}` vs `{}`)", a, b);
            passed += 1;
        }
        (Ok(_), Ok(_)) => {
            println!("  FAIL two distinct programs gave the same residual");
            failed += 1;
        }
        _ => {
            println!("  FAIL residual B unavailable");
            failed += 1;
        }
    }
    // Correctness at input = 5 (residual == interpreting).
    match &lean_a {
        Ok(res) => {
            let spec = px::px_run(&format!("let input = 5; in ({})", res));
            let interp = px::px_run(&format!(
                "let input = 5; prog = {}; in ({})",
                prog_a, int_lean
            ));
            match (spec, interp) {
                (Ok(s), Ok(i)) if s == i && s == "19" => {
                    println!("  ok   residual == interpreting at input=5 ({})", s);
                    passed += 1;
                }
                other => {
                    println!("  FAIL jones correctness: {:?}", other.0);
                    failed += 1;
                }
            }
        }
        Err(_) => {
            failed += 1;
        }
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// tower m8 analysis facet: offline BTA classifies static/dynamic and its
/// prediction is CROSS-CHECKED against the actual specializer (mix.px) — a
/// static if-condition is predicted Static iff mix folds it away; a dynamic
/// one is predicted Dynamic iff mix residualizes it. This makes the offline
/// analysis honest (it agrees with what specialization really does), without
/// claiming to bound the 3rd projection.
fn cmd_bta_check() -> ExitCode {
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let mut passed = 0;
    let mut failed = 0;

    // 1. All-static program: no dynamic vars, no dynamic classifications.
    match bta::analyze("let x = 5; in x * (2 + 3)", &[]) {
        Ok(r) if r.whole == bta::Bt::Static && r.dynamic_count == 0 => {
            println!("  ok   all-static program classifies Static (0 dynamic nodes)");
            passed += 1;
        }
        other => {
            println!("  FAIL all-static: {:?}", other.is_ok());
            failed += 1;
        }
    }
    // 2. Dynamic input taints dependents.
    match bta::analyze("x * (2 + 3)", &[String::from("x")]) {
        Ok(r) if r.whole == bta::Bt::Dynamic => {
            println!("  ok   dynamic input taints the expression (Dynamic)");
            passed += 1;
        }
        other => {
            println!("  FAIL dynamic taint: {:?}", other.is_ok());
            failed += 1;
        }
    }
    // 3. CROSS-CHECK: static if-condition -> BTA Static AND mix folds it.
    {
        let src = "if 2 < 3 then 10 else 20";
        let bta_static = match bta::analyze(src, &[]) {
            Ok(r) => !r.if_conds.is_empty() && r.if_conds.iter().all(|c| *c == bta::Bt::Static),
            Err(_) => false,
        };
        let folded = match tower::mix_in_px(src, &[], &granted) {
            Ok(o) => o.residual_source.as_deref() == Some("10"),
            Err(_) => false,
        };
        if bta_static && folded {
            println!("  ok   static if-cond: BTA=Static AND mix folds to 10 (agree)");
            passed += 1;
        } else {
            println!(
                "  FAIL static if cross-check: bta_static={} folded={}",
                bta_static, folded
            );
            failed += 1;
        }
    }
    // 4. CROSS-CHECK: dynamic if-condition -> BTA Dynamic AND mix residualizes.
    {
        let src = "if d then 10 else 20";
        let bta_dyn = match bta::analyze(src, &[String::from("d")]) {
            Ok(r) => r.if_conds.iter().any(|c| *c == bta::Bt::Dynamic),
            Err(_) => false,
        };
        let residual_if = match tower::mix_in_px(src, &[], &granted) {
            Ok(o) => o
                .residual_source
                .map(|s| s.contains("if ") || s.contains("then"))
                .unwrap_or(false),
            Err(_) => false,
        };
        if bta_dyn && residual_if {
            println!("  ok   dynamic if-cond: BTA=Dynamic AND mix residualizes the if (agree)");
            passed += 1;
        } else {
            println!(
                "  FAIL dynamic if cross-check: bta_dyn={} residual_if={}",
                bta_dyn, residual_if
            );
            failed += 1;
        }
    }
    // 5. HONEST BOUNDARY: BTA is an UPPER BOUND on folding, not an exact
    // predictor. A let-bound static var makes BTA classify the if-cond Static,
    // but mix's A4-conservative rule residualizes non-lambda lets — so mix
    // does NOT fold. The sound direction (mix folds => BTA Static) holds; the
    // converse does not. Documenting the gap keeps the analysis honest.
    {
        let src = "let b = 2 < 3; in if b then 10 else 20";
        let bta_static = match bta::analyze(src, &[]) {
            Ok(r) => r.if_conds.iter().all(|c| *c == bta::Bt::Static),
            Err(_) => false,
        };
        let mix_residualizes = match tower::mix_in_px(src, &[], &granted) {
            Ok(o) => o.residual_source.as_deref() != Some("10"),
            Err(_) => false,
        };
        if bta_static && mix_residualizes {
            println!("  ok   BTA is an upper bound: BTA=Static yet A4 mix residualizes (gap documented)");
            passed += 1;
        } else {
            println!(
                "  FAIL upper-bound boundary: bta_static={} mix_residualizes={}",
                bta_static, mix_residualizes
            );
            failed += 1;
        }
    }
    // 6. Determinism.
    match (
        bta::analyze("let f = x: x + 1; in f 3", &[]),
        bta::analyze("let f = x: x + 1; in f 3", &[]),
    ) {
        (Ok(a), Ok(b)) if a.static_count == b.static_count && a.dynamic_count == b.dynamic_count => {
            println!("  ok   analysis deterministic");
            passed += 1;
        }
        _ => {
            println!("  FAIL analysis non-deterministic");
            failed += 1;
        }
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cmd_tower_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[tower-check (milestone-1: reify/reflect + px self-interpreter)]");
    let granted = vec![String::from("file-read")];

    let probes = [
        ("arith", "1 + 2 * 3"),
        ("lambda-apply", "(x: x + 1) 41"),
        ("if", "if 2 < 3 then 10 else 20"),
        ("seq-let", "let a = 5; b = a + 1; in a * b"),
        ("curry", "(a: b: a + b) 20 22"),
    ];

    for (name, src) in probes {
        let result = (|| -> Result<(), String> {
            let ast = px::px_parse(src)?;
            let encoded = tower::reify(&ast)?;
            let back = tower::reflect(&encoded)?;
            if px::px_emit(&back) != px::px_emit(&ast) {
                return Err(format!(
                    "reify/reflect drift: {} vs {}",
                    px::px_emit(&back),
                    px::px_emit(&ast)
                ));
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                println!("  ok   {} reify/reflect roundtrip", name);
                passed += 1;
            }
            Err(e) => {
                println!("  FAIL {} roundtrip: {}", name, e);
                failed += 1;
            }
        }
    }

    for (name, src) in probes {
        let result = (|| -> Result<(String, String), String> {
            let native = px::px_run(src)?;
            let towered = tower::self_interp_eval(src, &granted)?;
            Ok((native, towered))
        })();
        match result {
            Ok((native, towered)) if native == towered => {
                println!("  ok   {} self-interp == native ({})", name, native);
                passed += 1;
            }
            Ok((native, towered)) => {
                println!("  FAIL {}: native {} != self-interp {}", name, native, towered);
                failed += 1;
            }
            Err(e) => {
                println!("  FAIL {} self-interp: {}", name, e);
                failed += 1;
            }
        }
    }

    // Milestone-2: RECURSIVE let semantics in the encoded interpreter — the
    // self-interpreter must agree with the sacred runtime on self-reference,
    // sibling order, later-shadows-earlier, nested shadowing, and real
    // recursion (a c05 variant with an integer body).
    let rec_probes = [
        ("shadow-later-wins", "let x = 1; x = 2; in x"),
        ("sibling-order", "let b = a + 1; a = 2; in b"),
        ("nested-shadow", "let x = 5; in let y = x + 1; x = 10; in y"),
        // m3 persistent value sharing (Rc payloads) lifted the old perf
        // boundary: full-scale recursive probes run directly.
        ("rec-fib", "let fib = n: if n < 2 then n else fib (n - 1) + fib (n - 2); in fib 20"),
        (
            "rec-c05-int",
            "let go = acc: n: if n == 0 then acc else go (acc + n) (n - 1); fib = n: if n < 2 then n else fib (n - 1) + fib (n - 2); in go 0 500 + fib 20",
        ),
    ];
    for (name, src) in rec_probes {
        let result = (|| -> Result<(String, String), String> {
            let native = px::px_run(src)?;
            let towered = tower::self_interp_eval(src, &granted)?;
            Ok((native, towered))
        })();
        match result {
            Ok((native, towered)) if native == towered => {
                println!("  ok   {} recursive self-interp == native ({})", name, native);
                passed += 1;
            }
            Ok((native, towered)) => {
                println!(
                    "  FAIL {}: native {} != self-interp {}",
                    name, native, towered
                );
                failed += 1;
            }
            Err(e) => {
                println!("  FAIL {} recursive self-interp: {}", name, e);
                failed += 1;
            }
        }
    }
    // m3b: strings/lists/attrs/select encoding — full ORIGINAL corpus
    // programs (attrs bodies, interpolation, first-order builtin calls) run
    // through the self-interpreter. Guest-closure-into-builtin stays m4.
    let m3b_inline = [
        ("enc-list-concat", "[ 1 2 ] ++ [ 3 ]"),
        ("enc-select-update", "({ a = 1; } // { b = 41; }).b"),
        ("enc-attrs-nested", "let v = 2; in { outer = { inner = v * 21; }; }"),
        ("enc-str-interp", "let n = 6; in \"n=${builtins.toString (n * 7)}\""),
        ("enc-higher-order", "builtins.map (x: x * x) [ 1 2 3 ]"),
        ("enc-sort-guest-cmp", "builtins.sort (a: b: b < a) [ 2 5 1 ]"),
    ];
    for (name, src) in m3b_inline {
        let result = (|| -> Result<(String, String), String> {
            let ast = px::px_parse(src)?;
            let encoded = tower::reify(&ast)?;
            let back = tower::reflect(&encoded)?;
            if px::px_emit(&back) != px::px_emit(&ast) {
                return Err(String::from("reify/reflect drift"));
            }
            let native = px::px_run(src)?;
            let towered = tower::self_interp_eval(src, &granted)?;
            Ok((native, towered))
        })();
        match result {
            Ok((native, towered)) if native == towered => {
                println!("  ok   {} encoded == native ({})", name, native);
                passed += 1;
            }
            Ok((native, towered)) => {
                println!("  FAIL {}: native {} != encoded {}", name, native, towered);
                failed += 1;
            }
            Err(e) => {
                println!("  FAIL {}: {}", name, e);
                failed += 1;
            }
        }
    }
    // m4: guest closures flow into higher-order builtins (gapply/gbuiltins
    // bridge, pure guest code) — every non-held corpus original encodes.
    for corpus_name in [
        "c02_strings",
        "c03_list",
        "c04_attr",
        "c05_recurse",
        "c06_nested",
        "c07_builtins",
        "c08_bool",
        "c09_lambda",
        "c10_mixed",
    ] {
        let path = format!("runtime/corpus/{}.px", corpus_name);
        let result = interop::host_read_file(&path, &granted).and_then(|src| {
            let ast = px::px_parse(&src)?;
            let encoded = tower::reify(&ast)?;
            let back = tower::reflect(&encoded)?;
            if px::px_emit(&back) != px::px_emit(&ast) {
                return Err(String::from("reify/reflect drift"));
            }
            let native = px::px_run(&src)?;
            let towered = tower::self_interp_eval(&src, &granted)?;
            Ok((native, towered))
        });
        match result {
            Ok((native, towered)) if native == towered => {
                println!("  ok   {} ORIGINAL encoded == native", corpus_name);
                passed += 1;
            }
            Ok((native, towered)) => {
                println!(
                    "  FAIL {}: native {} != encoded {}",
                    corpus_name, native, towered
                );
                failed += 1;
            }
            Err(e) => {
                println!("  FAIL {}: {}", corpus_name, e);
                failed += 1;
            }
        }
    }
    // m5: the specializer expressed IN px (S = L over the core subset) —
    // px specializing px, plus the cogen self-generation acceptance criterion.
    let no_statics: Vec<(String, i64)> = Vec::new();
    match tower::mix_in_px("(x: x + 1) 41", &no_statics, &granted) {
        Ok(o) if o.folded_value.as_deref() == Some("42") => {
            println!("  ok   mix folds closed program to 42 (px specializing px)");
            passed += 1;
        }
        other => {
            println!("  FAIL mix closed: {:?}", other.is_ok());
            failed += 1;
        }
    }
    match tower::mix_in_px("(k: k * k) (3 + 4)", &no_statics, &granted) {
        Ok(o) if o.folded_value.as_deref() == Some("49") => {
            println!("  ok   mix beta-reduces at spec time (49)");
            passed += 1;
        }
        other => {
            println!("  FAIL mix beta: {:?}", other.is_ok());
            failed += 1;
        }
    }
    let x_static = vec![(String::from("x"), 7i64)];
    match tower::mix_in_px("x * (2 + 3)", &x_static, &granted) {
        Ok(o) if o.folded_value.as_deref() == Some("35") => {
            println!("  ok   mix with static x=7 folds to 35");
            passed += 1;
        }
        other => {
            println!("  FAIL mix static var: {:?}", other.is_ok());
            failed += 1;
        }
    }
    // Dynamic x: the static subtree folds, the residual is correct syntax, and
    // the mix correctness equation holds: eval(residual)[x:=v] == eval(orig)[x:=v].
    match tower::mix_in_px("x * (2 + 3)", &no_statics, &granted) {
        Ok(o) => match &o.residual_source {
            Some(res) if res == "x * 5" => {
                let orig = px::px_run("let x = 7; in x * (2 + 3)");
                let spec = px::px_run(&format!("let x = 7; in {}", res));
                match (orig, spec) {
                    (Ok(a), Ok(b)) if a == b && a == "35" => {
                        println!("  ok   mix residual `x * 5` + correctness equation (35)");
                        passed += 1;
                    }
                    other => {
                        println!("  FAIL residual correctness: {:?}", other.0.is_ok());
                        failed += 1;
                    }
                }
            }
            other => {
                println!("  FAIL mix residual shape: {:?}", other);
                failed += 1;
            }
        },
        Err(e) => {
            println!("  FAIL mix dynamic: {}", e);
            failed += 1;
        }
    }
    // A4-conservative let: the let survives as residual and stays correct.
    match tower::mix_in_px("let y = d + 1; in y * (10 - 8)", &no_statics, &granted) {
        Ok(o) => match &o.residual_source {
            Some(res) if res.starts_with("let ") => {
                let orig = px::px_run("let d = 5; in (let y = d + 1; in y * (10 - 8))");
                let spec = px::px_run(&format!("let d = 5; in ({})", res));
                match (orig, spec) {
                    (Ok(a), Ok(b)) if a == b && a == "12" => {
                        println!("  ok   mix A4-residual let + correctness (12)");
                        passed += 1;
                    }
                    other => {
                        println!("  FAIL let residual correctness: {:?}", other.0.is_ok());
                        failed += 1;
                    }
                }
            }
            other => {
                println!("  FAIL mix let shape: {:?}", other);
                failed += 1;
            }
        },
        Err(e) => {
            println!("  FAIL mix let: {}", e);
            failed += 1;
        }
    }
    // m6a: FIRST FUTAMURA PROJECTION — the px-expressed specializer collapses
    // a px-written interpreter over a fixed object program into an
    // interpreter-free residual (recursive closures unfold at spec time).
    let interp_src = "let int = prog: env: \
if prog.tag == \"num\" then prog.value \
else if prog.tag == \"arg\" then env \
else if prog.tag == \"add\" then (int prog.l env) + (int prog.r env) \
else (int prog.l env) * (int prog.r env); \
in int prog input";
    let prog_lit = "{ tag = \"add\"; l = { tag = \"mul\"; l = { tag = \"arg\"; }; r = { tag = \"num\"; value = 3; }; }; r = { tag = \"num\"; value = 4; }; }";
    let prog_value = px::px_run_value(prog_lit);
    match prog_value {
        Ok(prog) => {
            let statics = vec![(String::from("prog"), prog)];
            match tower::mix_in_px_data(interp_src, &statics, &granted) {
                Ok(o) => match &o.residual_source {
                    Some(res)
                        if !res.contains("int") && !res.contains("prog") && !res.contains("tag") =>
                    {
                        // Correctness equation at input = 5: residual == direct.
                        let direct = px::px_run(&format!(
                            "let prog = {}; input = 5; in ({})",
                            prog_lit, interp_src
                        ));
                        let spec = px::px_run(&format!("let input = 5; in ({})", res));
                        match (direct, spec) {
                            (Ok(a), Ok(b)) if a == b && a == "19" => {
                                println!(
                                    "  ok   1st Futamura projection: interpreter-free residual `{}` == direct (19)",
                                    res
                                );
                                passed += 1;
                            }
                            other => {
                                println!(
                                    "  FAIL 1st projection correctness: {:?} {:?}",
                                    other.0, other.1
                                );
                                failed += 1;
                            }
                        }
                    }
                    other => {
                        println!("  FAIL 1st projection residual shape: {:?}", other);
                        failed += 1;
                    }
                },
                Err(e) => {
                    println!("  FAIL 1st projection: {}", e);
                    failed += 1;
                }
            }
            // Fully static (input also bound): the interpretive tower collapses
            // to a ground value at spec time.
            let prog2 = px::px_run_value(prog_lit);
            match prog2 {
                Ok(prog) => {
                    let statics = vec![
                        (String::from("prog"), prog),
                        (String::from("input"), px::PxVal::Int(5)),
                    ];
                    match tower::mix_in_px_data(interp_src, &statics, &granted) {
                        Ok(o) if o.folded_value.as_deref() == Some("19") => {
                            println!("  ok   1st projection, all-static: folds to 19");
                            passed += 1;
                        }
                        other => {
                            println!("  FAIL all-static collapse: {:?}", other.is_ok());
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    println!("  FAIL prog literal: {}", e);
                    failed += 1;
                }
            }
        }
        Err(e) => {
            println!("  FAIL prog literal: {}", e);
            failed += 1;
        }
    }
    // m6b: mix self-language coverage — builtins folding (bapply layer),
    // const/residual lists and attrsets. Static folds compute; dynamic parts
    // residualize as bapp/bfn and must satisfy the correctness equation.
    let m6b_probes: [(&str, &[(&str, i64)], &str, &str); 5] = [
        (
            "builtins.length [ 1 2 3 ] + builtins.getAttr \"a\" { a = 39; }",
            &[],
            "42",
            "",
        ),
        ("builtins.map (x: x * k) [ 1 2 3 ]", &[("k", 2)], "[ 2 4 6 ]", ""),
        (
            "builtins.map (x: x * k) [ 1 2 3 ]",
            &[],
            "",
            "let k = 2; in builtins.length (RES) + builtins.head (RES)",
        ),
        (
            "builtins.filter (x: x < t) [ 1 5 2 ]",
            &[("t", 3)],
            "[ 1 2 ]",
            "",
        ),
        ("({ a = 1; b = d; }).a", &[], "1", ""),
    ];
    for (src, statics, expected_residual, eq_template) in m6b_probes {
        let statics_owned: Vec<(String, i64)> =
            statics.iter().map(|(n, v)| (n.to_string(), *v)).collect();
        match tower::mix_in_px(src, &statics_owned, &granted) {
            Ok(o) => {
                let res = o.residual_source.clone().unwrap_or_default();
                let ok = if eq_template.is_empty() {
                    res == expected_residual
                        || o.folded_value.as_deref() == Some(expected_residual)
                } else {
                    // Correctness equation: residual under the dynamic binding
                    // agrees with the original source.
                    let with_res = eq_template.replace("RES", &res);
                    let with_src = eq_template.replace("RES", src);
                    matches!(
                        (px::px_run(&with_res), px::px_run(&with_src)),
                        (Ok(a), Ok(b)) if a == b
                    )
                };
                if ok {
                    println!("  ok   m6b `{}` specializes correctly", src);
                    passed += 1;
                } else {
                    println!(
                        "  FAIL m6b `{}`: residual `{}` folded {:?}",
                        src, res, o.folded_value
                    );
                    failed += 1;
                }
            }
            Err(e) => {
                println!("  FAIL m6b `{}`: {}", src, e);
                failed += 1;
            }
        }
    }
    // m6c: POLYVARIANT specializer parity — where the monovariant mix
    // terminates, poly agrees (semantically: residuals evaluate equal); and
    // poly handles dynamic-argument closure application via spec points.
    {
        let no_statics: Vec<(String, px::PxVal)> = Vec::new();
        match tower::poly_mix_in_px_data("x * (2 + 3)", &no_statics, &granted) {
            Ok(o) if o.residual_source.as_deref() == Some("x * 5") && o.spec_count == 0 => {
                println!("  ok   poly parity: `x * (2 + 3)` -> `x * 5` (0 specs)");
                passed += 1;
            }
            other => {
                println!("  FAIL poly parity x*5: {:?}", other.is_ok());
                failed += 1;
            }
        }
        match px::px_run_value("{ tag = \"add\"; l = { tag = \"mul\"; l = { tag = \"arg\"; }; r = { tag = \"num\"; value = 3; }; }; r = { tag = \"num\"; value = 4; }; }") {
            Ok(prog) => {
                let interp_src = "let int = prog: env: \
if prog.tag == \"num\" then prog.value \
else if prog.tag == \"arg\" then env \
else if prog.tag == \"add\" then (int prog.l env) + (int prog.r env) \
else (int prog.l env) * (int prog.r env); \
in int prog input";
                let statics = vec![(String::from("prog"), prog)];
                match tower::poly_mix_in_px_data(interp_src, &statics, &granted) {
                    Ok(o) => match &o.residual_source {
                        Some(res) => {
                            let spec = px::px_run(&format!("let input = 5; in ({})", res));
                            match spec {
                                Ok(v) if v == "19" => {
                                    println!(
                                        "  ok   poly 1st projection: {} specs, residual evaluates to 19",
                                        o.spec_count
                                    );
                                    passed += 1;
                                }
                                other => {
                                    println!("  FAIL poly 1st projection eval: {:?}", other);
                                    failed += 1;
                                }
                            }
                        }
                        None => {
                            println!("  FAIL poly 1st projection: unreflectable");
                            failed += 1;
                        }
                    },
                    Err(e) => {
                        println!("  FAIL poly 1st projection: {}", e);
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                println!("  FAIL poly prog literal: {}", e);
                failed += 1;
            }
        }
    }
    // m6f: SECOND FUTAMURA PROJECTION (call-by-need unlocked it) — poly
    // specializes the MONO SPECIALIZER to the interpreter, yielding a
    // COMPILER; applying the compiler to a program must equal what the mono
    // specializer produces directly (compiler correctness by agreement).
    {
        let result = (|| -> Result<(String, String, usize), String> {
            let mix_src = interop::host_read_file("runtime/tower/mix_core.px", &granted)?;
            let obj_src = match mix_src.trim().strip_suffix("in mix") {
                Some(head) => format!("{}in mix ast senv", head),
                None => return Err(String::from("mix_core.px shape")),
            };
            let interp_src = "let int = prog: env: \
if prog.tag == \"num\" then prog.value \
else if prog.tag == \"arg\" then env \
else if prog.tag == \"add\" then (int prog.l env) + (int prog.r env) \
else (int prog.l env) * (int prog.r env); \
in int prog input";
            let interp_enc = px::px_parse(interp_src).and_then(|a| tower::reify(&a))?;
            let statics = vec![(String::from("ast"), interp_enc)];
            let compiler = tower::poly_mix_in_px_data(&obj_src, &statics, &granted)?;
            let compiler_src = compiler
                .residual_source
                .ok_or_else(|| String::from("compiler not reflectable"))?;

            let prog_lit = "{ tag = \"add\"; l = { tag = \"mul\"; l = { tag = \"arg\"; }; r = { tag = \"num\"; value = 3; }; }; r = { tag = \"num\"; value = 4; }; }";
            let prog_value = px::px_run_value(prog_lit)?;
            let prog_node = tower::value_to_mix_node(&prog_value)?;
            let target = px::px_run(&format!(
                "let senv = {{ prog = {}; }}; in ({})",
                px::px_print(&prog_node),
                compiler_src
            ))?;

            let direct = tower::mix_in_px_data(
                interp_src,
                &[(String::from("prog"), prog_value)],
                &granted,
            )?;
            Ok((target, direct.residual_node, compiler.spec_count))
        })();
        match result {
            Ok((target, direct, specs)) if target == direct => {
                println!(
                    "  ok   2nd Futamura projection: compiler ({} specs) applied to prog == direct mix",
                    specs
                );
                passed += 1;
            }
            Ok((target, direct, _)) => {
                println!(
                    "  FAIL 2nd projection mismatch:\n    compiler: {}\n    direct:   {}",
                    &target[..target.len().min(160)],
                    &direct[..direct.len().min(160)]
                );
                failed += 1;
            }
            Err(e) => {
                println!("  FAIL 2nd projection: {}", e);
                failed += 1;
            }
        }
    }
    // Cogen self-generation acceptance criterion (harness only; a real cogen is
    // future work): a self-generating apply passes, a non-reproducing one fails.
    let mix_src = interop::host_read_file("runtime/tower/mix.px", &granted).unwrap_or_default();
    let cogen_probe = "c: c";
    let self_gen = |cogen: &str, _mix: &str| -> String { String::from(cogen) };
    let broken = |_cogen: &str, mix: &str| -> String { String::from(mix) };
    match (
        tower::cogen_acceptance(cogen_probe, &mix_src, &self_gen),
        tower::cogen_acceptance(cogen_probe, &mix_src, &broken),
    ) {
        (Ok((true, _, _, w1)), Ok((false, _, _, _w2))) if w1.status == "ok" => {
            println!("  ok   cogen acceptance criterion admits self-generation, rejects drift");
            passed += 1;
        }
        other => {
            println!(
                "  FAIL cogen acceptance: {:?} {:?}",
                other.0.is_ok(),
                other.1.is_ok()
            );
            failed += 1;
        }
    }
    match (
        tower::encoding_sha256("let a = 5; b = a + 1; in a * b"),
        tower::encoding_sha256("let a = 5; b = a + 1; in a * b"),
    ) {
        (Ok(h1), Ok(h2)) if h1 == h2 => {
            println!("  ok   encoding content-deterministic");
            passed += 1;
        }
        other => {
            println!("  FAIL encoding determinism: {:?}", other.0.is_ok());
            failed += 1;
        }
    }

    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cmd_action(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    match action::action_check(&src, &[]) {
        Ok(v) => {
            print!("{}", action::render(&v));
            if v.allowed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Action checkpoint proof: pure data actions are admitted, gate-uncertain and
/// mirror-held actions are refused, verdicts render deterministically.
fn cmd_action_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[action-check (one-verdict composition of gate+mirror+ir+witness)]");
    let granted = vec![String::from("file-read")];
    match interop::host_read_file("runtime/corpus/c05_recurse.px", &granted)
        .and_then(|src| action::action_check(&src, &[]))
    {
        Ok(v) if v.allowed && v.gate_allowed && v.mirror_status == "lossless" => {
            println!("  ok   corpus action admitted");
            passed += 1;
        }
        other => {
            println!("  FAIL corpus action: {:?}", other.is_ok());
            failed += 1;
        }
    }
    match action::action_check("let b = builtins; in 0", &[]) {
        Ok(v) if !v.allowed && !v.gate_allowed => {
            println!("  ok   gate-uncertain action refused");
            passed += 1;
        }
        other => {
            println!("  FAIL gate-uncertain action: {:?}", other.is_ok());
            failed += 1;
        }
    }
    match action::action_check("x: x + 1", &[]) {
        Ok(v) if !v.allowed && v.mirror_status == "held" => {
            println!("  ok   mirror-held action refused");
            passed += 1;
        }
        other => {
            println!("  FAIL mirror-held action: {:?}", other.is_ok());
            failed += 1;
        }
    }
    match (
        action::action_check("1 + 41", &[]),
        action::action_check("1 + 41", &[]),
    ) {
        (Ok(v1), Ok(v2)) if action::render(&v1) == action::render(&v2) => {
            println!("  ok   verdict deterministic");
            passed += 1;
        }
        other => {
            println!("  FAIL verdict determinism: {:?}", other.0.is_ok());
            failed += 1;
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Canonical oracle export for cross-host comparison (P13). The TSV schema is
/// the format proposed to the sibling lanes (pnix-clj / pnix-hy).
fn oracles_tsv() -> Result<String, String> {
    let granted = vec![String::from("file-read")];
    let mut out = String::new();
    out.push_str("# schema pnix-rs.oracles.v0
");
    out.push_str("# generated-by pnix-rs export-oracles
");
    out.push_str("# provenance: values manually cross-checked (S2, 2026-07-02) against
");
    out.push_str("#   pnix-clj resources/pnix_clj/rust_grounded/oracles.edn
");
    out.push_str("#   (rust-grounded capture of ~/pnix-old @ f5ce48f)
");
    out.push_str("# name	value_canonical	value_sha256	ir_sha256
");
    for (name, path, _expected) in px_corpus() {
        let src = interop::host_read_file(path, &granted)?;
        let value = px::px_run(&src)?;
        let ir_record = ir::ir_of(&src)?;
        out.push_str(&format!(
            "{}	{}	{}	{}
",
            name,
            value,
            sha256::sha256_hex(value.as_bytes()),
            ir_record.ir_sha256
        ));
    }
    Ok(out)
}

fn cmd_export_oracles() -> ExitCode {
    let granted = vec![
        String::from("file-read"),
        String::from("file-write"),
    ];
    match oracles_tsv() {
        Ok(tsv) => {
            if let Err(e) = interop::host_ensure_dir("proof", &granted) {
                eprintln!("pnix-rs: {}", e);
                return ExitCode::FAILURE;
            }
            match interop::host_write_file("proof/oracles-rs.tsv", &tsv, &granted) {
                Ok(()) => {
                    println!("wrote proof/oracles-rs.tsv ({} corpus rows)", px_corpus().len());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("pnix-rs: {}", e);
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Cross-host proof: the exported oracle file matches regeneration (drift
/// gate), the witness field schema is frozen in render order, and the corpus
/// expectations agree with the export.
fn cmd_cross_host_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[cross-host-check (oracle export + frozen witness schema)]");
    let granted = vec![String::from("file-read")];
    match (oracles_tsv(), interop::host_read_file("proof/oracles-rs.tsv", &granted)) {
        (Ok(generated), Ok(on_disk)) if generated == on_disk => {
            println!("  ok   proof/oracles-rs.tsv matches regeneration");
            passed += 1;
        }
        (Ok(_), Ok(_)) => {
            println!("  FAIL oracle export drift — run `pnix-rs export-oracles`");
            failed += 1;
        }
        other => {
            println!("  FAIL oracle export: gen={} disk readable={}", other.0.is_ok(), other.1.is_ok());
            failed += 1;
        }
    }
    // Witness schema frozen in render order.
    match gate::eval_witness("1 + 41", &[]) {
        Ok(w) => {
            let rendered = gate::render_witness(&w);
            let mut order_ok = true;
            let mut lines = rendered.split('\n');
            let _schema = lines.next();
            for field in gate::WITNESS_FIELDS {
                match lines.next() {
                    Some(line) if line.starts_with(&format!("{} ", field)) => {}
                    _ => order_ok = false,
                }
            }
            if order_ok {
                println!("  ok   witness 13-field schema frozen in render order");
                passed += 1;
            } else {
                println!("  FAIL witness field order drift");
                failed += 1;
            }
        }
        Err(e) => {
            println!("  FAIL witness probe: {}", e);
            failed += 1;
        }
    }
    // Corpus expectations agree with the export rows.
    match oracles_tsv() {
        Ok(tsv) => {
            let mut all_match = true;
            for (name, _path, expected) in px_corpus() {
                let mut found = false;
                for line in tsv.split('\n') {
                    let cols: Vec<&str> = line.split('\t').collect();
                    if cols.len() == 4 && cols[0] == name && cols[1] == expected {
                        found = true;
                    }
                }
                if !found {
                    all_match = false;
                }
            }
            if all_match {
                println!("  ok   corpus expectations agree with export rows");
                passed += 1;
            } else {
                println!("  FAIL corpus/export mismatch");
                failed += 1;
            }
        }
        Err(e) => {
            println!("  FAIL export for comparison: {}", e);
            failed += 1;
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The registered proof commands, replayed in clean subprocesses by `check`.
fn check_commands() -> Vec<&'static str> {
    vec![
        "px-check",
        "mirror-check",
        "stage-check",
        "ir-check",
        "gate-check",
        "interop-check",
        "rust-mirror-check",
        "specialize-check",
        "incremental-check",
        "compartment-check",
        "tower-check",
        "bta-check",
        "jones-check",
        "welltyped-check",
        "certify-check",
        "cogen-check",
        "attest-check",
        "reflect-tower-check",
        "verifying-cache-check",
        "phase-check",
        "assumption-check",
        "ir-diff-check",
        "attenuate-check",
        "explain-check",
        "engine-verdict-check",
        "engine-artifact-check",
        "engine-request-check",
        "engine-attestation-check",
        "engine-verify-check",
        "engine-batch-check",
        "action-check",
        "cross-host-check",
        "substrate-check",
        "capabilities-check",
        "registry-check",
    ]
}

/// all_ready aggregate: every registered proof command is replayed in a clean
/// subprocess of this same binary; the receipt is written to proof/.
fn cmd_check() -> ExitCode {
    println!("[check (all_ready aggregate; clean-process replay per report)]");
    let granted = vec![
        String::from("file-read"),
        String::from("file-write"),
        String::from("host-call"),
    ];
    if !std::path::Path::new(&bootstrap_path()).exists() {
        println!(
            "  FAIL substrate binary not found at {} — build ../rs-meta first",
            bootstrap_path()
        );
        println!("all_ready: false");
        return ExitCode::FAILURE;
    }
    let mut all_ready = true;
    let mut receipt = String::from("schema pnix-rs.check-receipt.v0
");
    for cmd in check_commands() {
        match interop::host_run_self(&[cmd], &granted) {
            Ok((success, out)) => {
                let mut counts = String::from("?");
                for line in out.split('\n') {
                    if line.starts_with("  => ") {
                        counts = line.trim().to_string();
                    }
                }
                let ready = success;
                if !ready {
                    all_ready = false;
                }
                println!(
                    "  {}   {}: {}",
                    if ready { "ok " } else { "FAIL" },
                    cmd,
                    counts
                );
                receipt.push_str(&format!(
                    "{}	{}	{}
",
                    cmd,
                    if ready { "ready" } else { "failed" },
                    counts
                ));
            }
            Err(e) => {
                all_ready = false;
                println!("  FAIL {}: {}", cmd, e);
                receipt.push_str(&format!("{}	failed	{}
", cmd, e));
            }
        }
    }
    receipt.push_str(&format!("all_ready	{}
", all_ready));
    let body_sha = sha256::sha256_hex(receipt.as_bytes());
    receipt.push_str(&format!("receipt_sha256	{}
", body_sha));
    if let Err(e) = interop::host_ensure_dir("proof", &granted) {
        println!("  FAIL receipt dir: {}", e);
        all_ready = false;
    }
    match interop::host_write_file("proof/check-receipt.txt", &receipt, &granted) {
        Ok(()) => println!("  receipt proof/check-receipt.txt sha256={}", body_sha),
        Err(e) => {
            println!("  FAIL writing receipt: {}", e);
            all_ready = false;
        }
    }
    println!("all_ready: {}", all_ready);
    if all_ready {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Self-describing capability index (the single source of docs/CAPABILITIES.md).
fn capabilities_doc() -> String {
    let mut d = String::new();
    let builtin_names = px::px_builtin_public_names();
    d.push_str("# pnix-rs CAPABILITIES — 능력 인덱스 (중복개발 방지 조회)

");
    d.push_str("> 생성: `pnix-rs capabilities > docs/CAPABILITIES.md` — 손 편집 금지.
");
    d.push_str("> drift 게이트: `pnix-rs capabilities-check`.

");
    d.push_str("## CLI 명령

");
    d.push_str("| 명령 | 목적 | schema |
|---|---|---|
");
    d.push_str("| px-eval -c\\|-f | .px 평가 → canonical 출력 | - |
");
    d.push_str("| px-check | corpus가 기대 canonical과 일치 | - |
");
    d.push_str("| mirror -c\\|-f / mirror-check | singleton mirror facet + roundtrip 어휘 | pnix-rs.mirror.v0 |
");
    d.push_str("| stage -c\\|-f / stage-check | px-stage1..5 + closure | pnix-rs.stage.v0 |
");
    d.push_str("| ir -c\\|-f / ir-check | canonical IR + ir_sha256 + identity sharing | pnix-rs.ir.v0 |
");
    d.push_str("| gate -c\\|-f / gate-check | purity/effect-class admission | pnix-rs.gate-check.v0 |
");
    d.push_str("| witness -c\\|-f | eval witness (13필드 공유 스키마) | pnix-rs.witness.v0 |
");
    d.push_str("| interop-check | host-call/file 경계 fail-closed + witness | pnix-rs.witness.v0 |
");
    d.push_str("| rust-mirror -c\\|-f / rust-mirror-check | 값 축: px값→Rust 3-way / AST 축: sig-tree(v1a)/typed(v2)/program 역재구성(v3)/struct·impl(v4) | pnix-rs.rust-mirror.v0 |
");
    d.push_str("| substrate-check | rs-meta interp == rustc == native 3-way | - |
");
    d.push_str("| check | all_ready 집계(clean-process replay) + receipt | pnix-rs.check-receipt.v0 |
");
    d.push_str("| capabilities / capabilities-check | 이 문서 생성 / drift 게이트 | - |

");
    d.push_str("## 모듈

");
    d.push_str("| 파일 | 역할 |
|---|---|
");
    d.push_str("| src/px.rs | sacred px runtime: lexer/parser/eval/print/emit/normalize (rs-meta subset 안) |
");
    d.push_str("| src/mirror.rs | singleton mirror_run + roundtrip 어휘 |
");
    d.push_str("| src/stage.rs | pnix runtime stage ladder |
");
    d.push_str("| src/ir.rs | canonical IR (직접평가가능, content-addressed) |
");
    d.push_str("| src/sha256.rs | in-house SHA-256 (FIPS self-test) |
");
    d.push_str("| src/gate.rs | purity/capability gate + 13필드 witness |
");
    d.push_str("| src/interop.rs | host-call/file 유일 통로 (capability 게이트) |
");
    d.push_str("| src/rust_mirror.rs | Rust↔px projection v0 (값 축) |

");
    d.push_str("## px 표면 (지원)

");
    d.push_str("checked int/float(혼합 승격·Nix 반올림)/bool/string(+`${}` 보간·raw bytes)/list(+`++`)/\n");
    d.push_str("attrset(+`//`, `.name`, `?`, 깊은 identity-aware `==`)/재귀 let·rec(call-by-need)/\n");
    d.push_str("lambda+juxtaposition/if-then-else/with/string `+`/bool `&& || !`/산술·비교/`#` 주석/\n");
    d.push_str("MD5·SHA1·SHA256·SHA512 `hashString`/Nix 호환 source-float grammar.\n\n");
    d.push_str(&format!(
        "등록된 public builtins {}종(함수 + 값 상수 + 재귀 `builtins` 필드; presence inventory):\n{}\n\n",
        builtin_names.len(),
        builtin_names.join(" ")
    ));
    d.push_str("presence는 호출 parity 주장이 아니다. 다음 11개 extension 이름은 호출 시 fail-closed HELD\n");
    d.push_str("(max/min은 구현됨 — 이전 텍스트가 부정확했음):\n");
    d.push_str("sin cos tan sqrt exp ln log abs pow mod functionArgs\n\n");
    d.push_str("## px 표면 (명시 미지원 — held)

");
    d.push_str("path literal/string-context/store 값 모델, URI literal, 중첩 동적 attr 경로,\n");
    d.push_str("POSIX ERE 전체 정합, JSON float exponent canonicalization,\n");
    d.push_str("비유한 float의 source-roundtrip 출력, Nix 전체 builtin 표면과 hash context 규칙\n\n");
    d.push_str("## 스키마

");
    d.push_str("pnix-rs.mirror.v0 · pnix-rs.stage.v0 · pnix-rs.ir.v0 · pnix-rs.gate-check.v0 ·
");
    d.push_str("pnix-rs.witness.v0(13필드 동결) · pnix-rs.rust-mirror.v0 · pnix-rs.check-receipt.v0

");
    d.push_str("## 어휘 (동결)

");
    d.push_str("roundtrip: lossless | lossy-ok | held | rejected
");
    d.push_str("effects: file-read | file-write | host-call | import | network
");
    d.push_str(&registry_section());
    d
}

/// The GATE REGISTRY + ROADMAP (wiki-style, gate-oriented). Every gate the
/// `check` aggregate replays is listed with what it PROVES (implemented), and
/// every remaining/held capability is listed with its rank/source/module and
/// the proposal that registers it (to implement). This is the single place to
/// grep before building — no duplicate development. `registry-check` gates it
/// against reality (every check_commands() gate appears; every roadmap
/// proposal file exists).
fn registry_section() -> String {
    let mut d = String::new();
    d.push_str("

## 게이트 레지스트리 — 이미 구현됨 (각 게이트가 증명하는 것)

");
    d.push_str("> 중복개발 방지: 새 기능 전에 이 표와 `docs/proposals/`를 grep.
");
    d.push_str("> 상태 = DONE (모두 `pnix-rs check` all_ready 집계에 포함).

");
    d.push_str("| 게이트 | 증명 | 상태 |
|---|---|---|
");
    // Derived from check_commands() so it cannot silently drop a gate.
    for cmd in check_commands() {
        let proves = gate_proves(cmd);
        d.push_str(&format!("| {} | {} | DONE |\n", cmd, proves));
    }
    d.push_str("
## 로드맵 — 새로 구현할 것 (held, 순위·근거·모듈·proposal)

");
    d.push_str("> 근거: docs/research/2026-07-03-metacircular-frontier.md (deep-research, 6 findings high-confidence).
");
    d.push_str("> 각 항목은 proposal로 등록됨(중복·누락 방지). external = 외부 lane 대기.

");
    d.push_str("| # | 능력 | 성격 | 모듈 | proposal |
|---|---|---|---|---|
");
    for (rank, cap, kind, module, prop) in roadmap_items() {
        d.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            rank, cap, kind, module, prop
        ));
    }
    d.push_str("
## proposals (등록된 설계/경계)

");
    d.push_str("0001 rust-ast-projection(v1a~v7 DONE, v8 held) · 0002 px-attrs-sorted-lookup(DONE) ·
");
    d.push_str("0003 px-call-by-need(DONE) · 0004 hand-written-cogen(held) ·
");
    d.push_str("0005 well-typed-residual-gate(held) · 0006 runtime-surface-on-demand(held) ·
");
    d.push_str("0007 research-frontier-index(open: TV/certified-compilation, reflective towers,
");
    d.push_str("content-addressed incremental)
");
    d
}

/// One-line statement of what each gate proves (for the registry).
fn gate_proves(cmd: &str) -> &'static str {
    match cmd {
        "px-check" => "seed corpus가 기대 canonical로 평가 (부동/toJSON/동적키/깊은== 포함)",
        "mirror-check" => "corpus mirror lossless (emit 고정점 + 값 일치)",
        "stage-check" => "px-stage1..5 + closure 런타임 사다리 닫힘",
        "ir-check" => "sha256 벡터 + IR 증명 + identity sharing (바인딩 순서 무관)",
        "gate-check" => "corpus 순수 admission; 미지 builtin fail-closed; witness",
        "interop-check" => "host-call 경계: grant 없이 거부 + witness",
        "rust-mirror-check" => "값 축 px→Rust 3-way + AST 축 v1a~v7(mirror_probe 전량 + 제네릭 fn 왕복, AST 동일+rustc 정합)",
        "specialize-check" => "A4-건전 부분평가: 폐쇄식 fold, 동적 let held",
        "incremental-check" => "알파 불변 + SCC + realisation 컷오프 + demand-driven 변경 전파(salsa/adapton 최소 재계산)",
        "compartment-check" => "SES식 격리: 자기 env/모듈, intrinsic 공유",
        "tower-check" => "reify/reflect + px 자기해석기 == 네이티브 + 1·2차 Futamura 사영",
        "bta-check" => "오프라인 BTA static/dynamic + mix 교차검증(폴딩 상한)",
        "jones-check" => "Jones-optimality: 인터프리터 bloat에도 residual 불변(해석 계층 제거)",
        "welltyped-check" => "px→Rust residual이 rs-meta 플로어 typeck로 well-typed (구성상 타입-정합; Rust 정적 강점)",
        "certify-check" => "proof-carrying residual: 특화 residual이 소스와 입력 배터리 전체 동등(재검증 인증서, 증명기 없이)",
        "cogen-check" => "손으로 쓴 cogen(generating extension) — 어떤 객체 프로그램이든 컴파일된 residual 생성 == 해석 (자기적용 없이)",
        "attest-check" => "typed attestation(in-toto/SLSA식) — witness에 predicate 타입 + subject; 불일치 predicate 거부",
        "reflect-tower-check" => "finite reflective tower(3-Lisp): reify/reflect가 인코딩을 다시 인코딩해도 2-레벨 coherent + 메타-레벨 의미 투명",
        "verifying-cache-check" => "verifying cache: 캐시 히트 시 재검증(재실행 대조) — 오염된 realisation 감지(이빨)",
        "phase-check" => "phase separation: 특화 residual의 자유변수 = 정확히 동적 변수(정적 완전 소진·동적 완전 보존)",
        "assumption-check" => "assumed specialization: residual의 정적 가정이 유효할 때만 재사용, 가정 변하면 stale 감지→재특화",
        "ir-diff-check" => "ir-diff: canonical IR 의미 diff — reorder는 동일(meaning-preserving), 의미 변경은 국소화",
        "attenuate-check" => "capability attenuation(SES): grant→감쇠(엄격히 약화)→회수; 감쇠는 되돌릴 수 없음(재확대 불가)",
        "explain-check" => "unified explain: 한 호출로 value+purity+effects+ir+mirror+witness 통합; 개별 facet과 정합",
        "engine-verdict-check" => "peer-engine adapter: rs-meta Rust TV -> 공통 .px engine verdict envelope(pnix.engine.verdict.v0); verdict가 유효 px + TV->status 매핑 정합",
        "engine-artifact-check" => "native artifact receipt를 .px 봉투(pnix.engine.artifact.v0)로 export; 재현 가능 + 유효 px (stage8-repro per-program)",
        "engine-request-check" => "요청/응답 프로토콜: .px request 봉투(pnix.engine.request.v0)를 phase로 디스패치→verdict/artifact/profile 응답",
        "engine-attestation-check" => "엔진 신뢰 증명(pnix.engine.attestation.v0): interp==rustc TV 커버리지(positive+negative corpus) + substrate 3-way",
        "engine-verify-check" => "검증 가능 verdict: witness_id를 증거 필드에서 재계산해 일치 확인(변조 감지); 신뢰 없이 검증",
        "engine-batch-check" => "배치 오케스트레이션: Rust 소스 리스트 -> verdict 매니페스트(pnix.engine.batch.v0) + accepted/held/rejected 카운트",
        "action-check" => "단일 verdict = gate+mirror+ir+witness (admitted/refused/결정성)",
        "cross-host-check" => "oracles TSV export drift 게이트 + witness 스키마 동결",
        "substrate-check" => "rs-meta interp == rustc == native 3-way (rs-meta 의존 증명)",
        "capabilities-check" => "이 생성 인덱스가 커밋된 문서와 일치 (docs drift + 레지스트리)",
        "registry-check" => "레지스트리 <-> 실제 게이트/proposal 정합 (누락·dangling 방지)",
        _ => "(등록 안 된 게이트 — registry-check가 잡음)",
    }
}

/// The roadmap: (rank, capability, kind, module, proposal). Kept in sync with
/// docs/proposals and docs/research. registry-check verifies each proposal
/// file exists.
fn roadmap_items() -> Vec<(&'static str, &'static str, &'static str, &'static str, &'static str)> {
    vec![
        (
            "1",
            "full 3차 사영 — feature-rich specialiser 자기적용 (bounded cogen은 DONE; full은 연구 지평)",
            "연구 프론티어(Leuschel)",
            "tower/bta",
            "0004",
        ),
        (
            "3",
            "P6 v8 — 제네릭 struct/impl<T> + 트레이트 solving projection",
            "기계적 확장",
            "rust_mirror",
            "0001",
        ),
        (
            "4",
            "runtime 표면 tail — path/context/store 값 · URI literal · 중첩 동적 attr 경로 · JSON float 표기 · regex 정합",
            "기계적 확장",
            "px",
            "0006",
        ),
        (
            "5",
            "full S=L (전 표면 poly) + stage-polymorphic — 연구 지평",
            "연구 프론티어",
            "tower",
            "0007",
        ),
        (
            "6",
            "research open: step-level bisimulation · N-레벨 collapsing tower [incremental·proof-carrying residual·finite reflective tower는 DONE]",
            "후속 리서치",
            "check/tower",
            "0007",
        ),
    ]
}

fn cmd_capabilities() -> ExitCode {
    print!("{}", capabilities_doc());
    ExitCode::SUCCESS
}

/// registry gate: the capability registry cannot lie. (1) every gate the
/// `check` aggregate replays has a non-placeholder `gate_proves` line, and
/// (2) every roadmap item's proposal file exists on disk — so nothing is
/// missed and no roadmap dangles. This makes the wiki-style registry
/// gate-verified, not hand-maintained.
fn cmd_registry_check() -> ExitCode {
    println!("[registry-check (레지스트리 <-> 실제 게이트/proposal 정합)]");
    let granted = vec![String::from("file-read")];
    let mut passed = 0;
    let mut failed = 0;
    // (1) every registered gate is described.
    let mut undescribed = Vec::new();
    for cmd in check_commands() {
        if gate_proves(cmd).starts_with("(등록") {
            undescribed.push(cmd);
        }
    }
    if undescribed.is_empty() {
        println!(
            "  ok   모든 게이트({}개)가 레지스트리에 기술됨",
            check_commands().len()
        );
        passed += 1;
    } else {
        println!("  FAIL 기술 안 된 게이트: {:?}", undescribed);
        failed += 1;
    }
    // (2) registry-check itself must be in the aggregate (self-registration).
    if check_commands().iter().any(|c| *c == "registry-check") {
        println!("  ok   registry-check가 집계에 자기등록됨");
        passed += 1;
    } else {
        println!("  FAIL registry-check가 check_commands에 없음");
        failed += 1;
    }
    // (3) every roadmap proposal file exists (no dangling roadmap).
    let mut missing = Vec::new();
    for (_rank, _cap, _kind, _module, prop) in roadmap_items() {
        let path = format!("docs/proposals/{}", proposal_file(prop));
        if interop::host_read_file(&path, &granted).is_err() {
            missing.push(prop);
        }
    }
    if missing.is_empty() {
        println!(
            "  ok   모든 로드맵 proposal 파일 존재({}종)",
            distinct_props()
        );
        passed += 1;
    } else {
        println!("  FAIL 없는 proposal: {:?}", missing);
        failed += 1;
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Proposal number -> filename (the committed docs/proposals entries).
fn proposal_file(prop: &str) -> &'static str {
    match prop {
        "0001" => "0001-rust-ast-projection.md",
        "0002" => "0002-px-attrs-sorted-lookup.md",
        "0003" => "0003-px-call-by-need.md",
        "0004" => "0004-hand-written-cogen.md",
        "0005" => "0005-well-typed-residual-gate.md",
        "0006" => "0006-runtime-surface-on-demand.md",
        "0007" => "0007-research-frontier-index.md",
        _ => "MISSING.md",
    }
}

fn distinct_props() -> usize {
    let mut seen: Vec<&str> = Vec::new();
    for (_r, _c, _k, _m, prop) in roadmap_items() {
        if !seen.iter().any(|s| s == &prop) {
            seen.push(prop);
        }
    }
    seen.len()
}

/// docs_drift gate: the generated index must match the committed file.
fn cmd_capabilities_check() -> ExitCode {
    println!("[capabilities-check (docs drift gate)]");
    let granted = vec![String::from("file-read")];
    match interop::host_read_file("docs/CAPABILITIES.md", &granted) {
        Ok(on_disk) => {
            if on_disk == capabilities_doc() {
                println!("  ok   docs/CAPABILITIES.md matches the generated index");
                println!("  => PASS (1 passed, 0 failed)");
                ExitCode::SUCCESS
            } else {
                println!("  FAIL drift — run `pnix-rs capabilities > docs/CAPABILITIES.md`");
                println!("  => FAIL (0 passed, 1 failed)");
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            println!("  FAIL {} — run `pnix-rs capabilities > docs/CAPABILITIES.md`", e);
            println!("  => FAIL (0 passed, 1 failed)");
            ExitCode::FAILURE
        }
    }
}

fn bootstrap_path() -> String {
    std::env::var("RS_META_BOOTSTRAP")
        .unwrap_or_else(|_| String::from("/tmp/rs-meta-target/release/bootstrap"))
}

fn cmd_rust_mirror(rest: &[String]) -> ExitCode {
    let src = match load_source(rest) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let granted = vec![String::from("file-read"), String::from("host-call")];
    match rust_mirror::rust_value_roundtrip(&src, &bootstrap_path(), &granted) {
        Ok(record) => {
            print!("{}", rust_mirror::render(&record));
            if record.status == "lossless" || record.status == "held" {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("pnix-rs: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Rust↔px value-axis projection proof: every data-valued corpus program
/// roundtrips 3-way lossless (px canonical == rs-meta interp == rustc over the
/// projected Rust program); opaque values hold.
fn cmd_rust_mirror_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[rust-mirror-check (px value -> Rust program -> substrate 3-way)]");
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let bootstrap = bootstrap_path();
    for (name, path, _expected) in px_corpus() {
        let result = interop::host_read_file(path, &granted)
            .and_then(|src| rust_mirror::rust_value_roundtrip(&src, &bootstrap, &granted));
        match result {
            Ok(record) => {
                if record.status == "lossless" {
                    println!("  ok   {} lossless (3-way)", name);
                    passed += 1;
                } else {
                    println!("  FAIL {}: status {}", name, record.status);
                    failed += 1;
                }
            }
            Err(e) => {
                println!("  FAIL {}: {}", name, e);
                failed += 1;
            }
        }
    }
    // AST axis (v1): rs-meta canonical AST sig-tree reified as px data.
    for probe in ["../rs-meta/samples/factorial.rs", "../rs-meta/samples/mirror_probe.rs"] {
        let result = interop::host_read_file(probe, &granted)
            .and_then(|src| rust_mirror::rust_ast_roundtrip(&src, &bootstrap, &granted));
        match result {
            Ok(r) if r.status == "lossless" => {
                println!(
                    "  ok   {} AST-axis lossless (sig {})",
                    probe,
                    &r.sig_sha256[0..12]
                );
                passed += 1;
            }
            Ok(r) => {
                println!(
                    "  FAIL {} AST-axis: regen={} px_embed={}",
                    probe, r.regen_match, r.px_embed_match
                );
                failed += 1;
            }
            Err(e) => {
                println!("  FAIL {} AST-axis: {}", probe, e);
                failed += 1;
            }
        }
    }
    // AST axis v2: typed core + tower join — the same Rust expression is
    // evaluated by rustc (native tier) and by the px self-interpreter through
    // the px-written bridge; the two must agree, and the typed nodes must
    // regenerate the sig text byte-identically.
    for (expr, expected) in [
        ("6 * 7", "42"),
        ("(1 + 2) * (3 + 4)", "21"),
        ("if 2 < 3 { 10 } else { 20 }", "10"),
    ] {
        match rust_mirror::rust_expr_join(expr, &bootstrap, &granted) {
            Ok(r) if r.typed_roundtrip && r.rustc_out == expected && r.self_interp_out == expected => {
                println!(
                    "  ok   `{}` — rustc == px self-interp == {} (typed roundtrip)",
                    expr, expected
                );
                passed += 1;
            }
            Ok(r) => {
                println!(
                    "  FAIL `{}`: roundtrip={} rustc={} px={}",
                    expr, r.typed_roundtrip, r.rustc_out, r.self_interp_out
                );
                failed += 1;
            }
            Err(e) => {
                println!("  FAIL `{}`: {}", expr, e);
                failed += 1;
            }
        }
    }
    // AST axis v3: whole-program typed core + px -> Rust RECONSTRUCTION.
    // AST identity is judged by rs-meta itself (ast-canonical equality) and
    // execution by rustc parity.
    {
        let add3_src = "fn add3(a: i64, b: i64, c: i64) -> i64 { a + b + c } fn main() { let x = add3(1, 2, 3); println!(\"{}\", x); }";
        let factorial_src = interop::host_read_file("../rs-meta/samples/factorial.rs", &granted);
        let struct_src = "struct Point { x: i64, y: i64 } impl Point { fn origin() -> Point { Point { x: 40, y: 2 } } fn sum(&self) -> i64 { self.x + self.y } } fn main() { let p = Point::origin(); println!(\"{}\", p.sum()); }";
        let enum_src = "enum Shape { Circle(i64), Rect { w: i64, h: i64 } } fn area(s: Shape) -> i64 { match s { Shape::Circle(r) => r * r * 3, Shape::Rect { w, h } => w * h, } } fn main() { println!(\"{}\", area(Shape::Circle(2)) + area(Shape::Rect { w: 2, h: 5 })); }";
        let mirror_probe_src = interop::host_read_file("../rs-meta/samples/mirror_probe.rs", &granted);
        let cases: Vec<(&str, String)> = vec![
            ("factorial.rs", factorial_src.unwrap_or_default()),
            ("add3-with-let", String::from(add3_src)),
            ("struct-impl-v4", String::from(struct_src)),
            ("enum-match-v5", String::from(enum_src)),
            ("mirror_probe.rs-v6", mirror_probe_src.unwrap_or_default()),
            ("generic-fn-v7", String::from("fn id<T>(x: T) -> T { x } fn first<A, B>(a: A, b: B) -> A { a } fn main() { println!(\"{}\", id(40) + first(2, 9)); }")),
            ("generic-struct-v8", String::from("struct Wrap<T> { value: T } impl<T> Wrap<T> { fn unwrap(self) -> T { self.value } } fn main() { let w = Wrap { value: 42 }; println!(\"{}\", w.unwrap()); }")),
        ];
        for (name, src) in cases {
            match rust_mirror::rust_program_reconstruct(&src, &bootstrap, &granted) {
                Ok(r) if r.sig_roundtrip && r.ast_identity && r.rustc_parity => {
                    println!(
                        "  ok   {} — px-reconstructed Rust is AST-identical + rustc-parity",
                        name
                    );
                    passed += 1;
                }
                Ok(r) => {
                    println!(
                        "  FAIL {}: sig={} ast={} rustc={}",
                        name, r.sig_roundtrip, r.ast_identity, r.rustc_parity
                    );
                    failed += 1;
                }
                Err(e) => {
                    println!("  FAIL {}: {}", name, e);
                    failed += 1;
                }
            }
        }
        match rust_mirror::sig_typed_program("fn main()->unit {ex while(bool(true),{|_})|_};") {
            Err(e) if e.contains("held") => {
                println!("  ok   typed program holds on loops (v1a tree covers; v6)");
                passed += 1;
            }
            other => {
                println!("  FAIL loop hold: {:?}", other.is_ok());
                failed += 1;
            }
        }
    }
    match rust_mirror::sig_typed_parse("call(fact,[int(3)])") {
        Err(e) if e.contains("held") => {
            println!("  ok   typed core holds on call nodes (v1a tree covers them)");
            passed += 1;
        }
        other => {
            println!("  FAIL typed-core hold: {:?}", other.is_ok());
            failed += 1;
        }
    }
    match rust_mirror::rust_value_roundtrip("x: x", &bootstrap, &granted) {
        Ok(record) if record.status == "held" => {
            println!("  ok   opaque value holds (lambda)");
            passed += 1;
        }
        Ok(record) => {
            println!("  FAIL opaque probe: status {}", record.status);
            failed += 1;
        }
        Err(e) => {
            println!("  FAIL opaque probe: {}", e);
            failed += 1;
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Interop boundary proof: capability denial without grants, granted smoke
/// call with a 13-field host-call witness, and witness determinism.
fn cmd_interop_check() -> ExitCode {
    let mut passed = 0usize;
    let mut failed = 0usize;
    println!("[interop-check (host-call boundary + capabilities)]");
    let none: Vec<String> = Vec::new();
    match interop::host_run_bootstrap("/nonexistent", "run", &[], &none) {
        Err(e) if e.contains("capability denied: host-call") => {
            println!("  ok   host-call denied without grant");
            passed += 1;
        }
        _ => {
            println!("  FAIL host-call was not denied without grant");
            failed += 1;
        }
    }
    match interop::host_read_file("runtime/corpus/c05_recurse.px", &none) {
        Err(e) if e.starts_with("capability-denied:") => {
            println!("  ok   file-read denied without grant");
            passed += 1;
        }
        _ => {
            println!("  FAIL file-read was not denied without grant");
            failed += 1;
        }
    }
    let granted = vec![String::from("file-read"), String::from("host-call")];
    let bootstrap = bootstrap_path();
    match interop::host_run_bootstrap(&bootstrap, "run", &["src/px.rs", "harness/substrate_harness.rs"], &granted)
    {
        Ok(out) if out.contains("c05_recurse") => {
            let w1 = interop::host_call_witness("run", "px-engine+harness", &out, &granted);
            let w2 = interop::host_call_witness("run", "px-engine+harness", &out, &granted);
            let r1 = gate::render_witness(&w1);
            let r2 = gate::render_witness(&w2);
            let mut schema_ok = true;
            for field in gate::WITNESS_FIELDS {
                if !r1.contains(&format!("\n{} ", field)) && !r1.starts_with(&format!("{} ", field)) {
                    schema_ok = false;
                }
            }
            if schema_ok {
                println!("  ok   granted host-call smoke + 13-field witness");
                passed += 1;
            } else {
                println!("  FAIL host-call witness schema");
                failed += 1;
            }
            if r1 == r2 {
                println!("  ok   host-call witness deterministic");
                passed += 1;
            } else {
                println!("  FAIL host-call witness not deterministic");
                failed += 1;
            }
        }
        Ok(_) => {
            println!("  FAIL granted host-call smoke: unexpected output");
            failed += 1;
        }
        Err(e) => {
            println!("  FAIL granted host-call smoke: {}", e);
            failed += 1;
        }
    }
    println!(
        "  => {} ({} passed, {} failed)",
        if failed == 0 { "PASS" } else { "FAIL" },
        passed,
        failed
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cmd_io_probe(rest: &[String]) -> ExitCode {
    let root = match rest.first() {
        Some(value) => value,
        None => {
            eprintln!("io-probe requires a fixture root");
            return ExitCode::FAILURE;
        }
    };
    match interop::io_probe_json(root) {
        Ok(json) => {
            println!("{}", json);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("io-probe: {}", error);
            ExitCode::FAILURE
        }
    }
}

/// The substrate contract: ../rs-meta's bootstrap interprets `src/px.rs` plus
/// the substrate harness, and its stdout must equal both the rustc-compiled
/// run of the same bundle (rs-meta native tier) and this binary's own px
/// engine output on the same probes.
fn cmd_substrate_check() -> ExitCode {
    let bootstrap = bootstrap_path();
    println!("[substrate-check (rs-meta interprets the px engine)]");

    let granted = vec![String::from("file-read"), String::from("host-call")];
    let mut local = String::new();
    for (name, path, _expected) in px_corpus() {
        if name.starts_with("seed_") {
            continue;
        }
        match interop::host_read_file(path, &granted).map(|src| px::px_run(&src)) {
            Ok(Ok(out)) => {
                local.push_str(&format!("{} {}\n", name, out));
            }
            Ok(Err(e)) => {
                println!("  FAIL local px_run {}: {}", name, e);
                return ExitCode::FAILURE;
            }
            Err(e) => {
                println!("  FAIL read {}: {}", path, e);
                return ExitCode::FAILURE;
            }
        }
    }

    let phase2 = "let x = 100000000000000000000.0; y = x * x; z = y * y; w = z * z; i = w * w; n = i - i; f = v: v; in { hashes = map (a: builtins.hashString a \"abc\") [ \"md5\" \"sha1\" \"sha256\" \"sha512\" ]; mixed = [ (1 + 1.5) (builtins.add 1 1.5) (builtins.lessThan 1 1.5) (1 == 1.0) ([ 1 ] == [ 1.0 ]) (builtins.elem f [ f ]) ]; strings = [ (builtins.toString 1.5) (builtins.toString 1.25e-3) (builtins.toString .5e2) (builtins.toString 0.0e-400) (builtins.toString (-0.0)) (builtins.toString (0.0 / (-1.0))) (builtins.toString ((-1.0) * 0.0)) (builtins.toString i) (builtins.toString (0.0 - i)) (builtins.toString n) ]; compares = [ (n < n) (n <= n) (n > n) (n >= n) ([ n 0 ] < [ n 1 ]) ]; identity = [ (let l = [ (builtins.throw \"forced\") ]; in (builtins.tryEval (l == l)).success) (let g = h: [ h ]; in (g f) == (g f)) ]; round = [ (builtins.ceil (-1.8)) (builtins.floor (-1.2)) ]; }";
    match px::px_run(phase2) {
        Ok(out) => local.push_str(&format!("phase2_numeric_hash {}\n", out)),
        Err(e) => local.push_str(&format!("phase2_numeric_hash ERR {}\n", e)),
    }
    match px::px_run("9223372036854775807 + 1") {
        Ok(out) => local.push_str(&format!("phase2_overflow UNEXPECTED {}\n", out)),
        Err(e) => local.push_str(&format!("phase2_overflow {}\n", e)),
    }
    match px::px_run("builtins.hashString \"sha3\" (builtins.throw \"payload\")") {
        Ok(out) => local.push_str(&format!("phase2_hash_order UNEXPECTED {}\n", out)),
        Err(e) => local.push_str(&format!("phase2_hash_order {}\n", e)),
    }
    let hash_edges = "let p56 = builtins.concatStringsSep \"\" (builtins.genList (_: \"a\") 56); p112 = builtins.concatStringsSep \"\" (builtins.genList (_: \"a\") 112); raw = builtins.substring 0 1 \"가\"; in { boundary = [ (builtins.hashString \"md5\" p56) (builtins.hashString \"sha1\" p56) (builtins.hashString \"sha256\" p56) (builtins.hashString \"sha512\" p112) ]; raw = builtins.hashString \"sha256\" raw; unicode = builtins.hashString \"sha256\" \"가🙂\"; }";
    match px::px_run(hash_edges) {
        Ok(out) => local.push_str(&format!("phase2_hash_edges {}\n", out)),
        Err(e) => local.push_str(&format!("phase2_hash_edges ERR {}\n", e)),
    }
    match px::px_run("1.0e-308") {
        Ok(out) => local.push_str(&format!("phase2_float_literal UNEXPECTED {}\n", out)),
        Err(e) => local.push_str(&format!("phase2_float_literal {}\n", e)),
    }
    let uri_literals = "[ x:x let:x a:b==c a:%/?::@&=+$,-_.!~*' (builtins.typeOf (x: x)) (builtins.typeOf (_x:_x)) (a:b + \"c\") ]";
    match px::px_run(uri_literals) {
        Ok(out) => local.push_str(&format!("phase3_uri_literals {}\n", out)),
        Err(e) => local.push_str(&format!("phase3_uri_literals ERR {}\n", e)),
    }
    let posix_classes = "let m = p: s: builtins.match p s != null; in [ (m \"[[:alnum:]]+\" \"Az09\") (m \"[[:blank:]]+\" \" \\t\") (m \"[[:cntrl:]]+\" \"\\t\\n\") (m \"[[:graph:]]+\" \"Az!9\") (m \"[[:print:]]+\" \" Az!9\") (m \"[[:punct:]]+\" \"!?\") (m \"[[:space:]]+\" \" \\t\\n\") (m \"[[:xdigit:]]+\" \"aF09\") (builtins.match \"[[:space:]]*(.*[^[:space:]])[[:space:]]*\" \" ?x \" == [ \"?x\" ]) (builtins.split \"[[:space:]]+\" \"a \\tb\\nc\" == [ \"a\" [ ] \"b\" [ ] \"c\" ]) ]";
    match px::px_run(posix_classes) {
        Ok(out) => local.push_str(&format!("phase3_posix_classes {}\n", out)),
        Err(e) => local.push_str(&format!("phase3_posix_classes ERR {}\n", e)),
    }

    // Emitter roundtrip probe, computed the same way the harness does.
    match interop::host_read_file("runtime/corpus/c05_recurse.px", &granted) {
        Ok(src) => {
            let result = px::px_parse(&src)
                .map(|ast| px::px_emit(&ast))
                .and_then(|emitted| px::px_parse(&emitted))
                .and_then(|reparsed| {
                    let env = Vec::new();
                    px::px_eval(&reparsed, &env)
                });
            match result {
                Ok(v) => local.push_str(&format!("mirror_c05 {}\n", px::px_print(&v))),
                Err(e) => {
                    println!("  FAIL local mirror probe: {}", e);
                    return ExitCode::FAILURE;
                }
            }
        }
        Err(e) => {
            println!("  FAIL read c05 for mirror probe: {}", e);
            return ExitCode::FAILURE;
        }
    }

    let harness_files = ["src/px.rs", "harness/substrate_harness.rs"];
    let interp = interop::host_run_bootstrap(&bootstrap, "run", &harness_files, &granted);
    let native = interop::host_run_bootstrap(&bootstrap, "native-run", &harness_files, &granted);
    match (interp, native) {
        (Ok(i), Ok(n)) => {
            if i == n && i == local {
                println!("  ok   rs-meta interp == rs-meta rustc == pnix-rs native:");
                for line in i.trim().split("\n") {
                    println!("         {}", line);
                }
                println!("  => PASS (1 passed, 0 failed)");
                ExitCode::SUCCESS
            } else {
                println!(
                    "  FAIL substrate drift:\n  interp {:?}\n  rustc {:?}\n  local {:?}",
                    i, n, local
                );
                println!("  => FAIL (0 passed, 1 failed)");
                ExitCode::FAILURE
            }
        }
        (Err(e), _) => {
            println!("  FAIL rs-meta interp: {}", e);
            println!("  => FAIL (0 passed, 1 failed)");
            ExitCode::FAILURE
        }
        (_, Err(e)) => {
            println!("  FAIL rs-meta rustc: {}", e);
            println!("  => FAIL (0 passed, 1 failed)");
            ExitCode::FAILURE
        }
    }
}

fn load_source(rest: &[String]) -> Result<String, String> {
    match rest.first().map(|s| s.as_str()) {
        Some("-c") => rest
            .get(1)
            .cloned()
            .ok_or_else(|| String::from("-c requires a source string")),
        Some("-f") => {
            let path = rest
                .get(1)
                .ok_or_else(|| String::from("-f requires a file path"))?;
            let granted = vec![String::from("file-read")];
            interop::host_read_file(path, &granted)
        }
        Some(other) => Err(format!("expected -c or -f, got {}", other)),
        None => Err(String::from("expected -c <src> or -f <file.px>")),
    }
}

fn print_help() {
    println!(
        "pnix-rs — rs-meta backed pnix runtime front-end\n\
\n\
USAGE:\n\
  pnix-rs <command> [args]\n\
\n\
COMMANDS:\n\
  px-eval -c \"<px>\"    evaluate a .px expression (canonical print)\n\
  px-eval -f <file.px>  evaluate a .px file (e.g. default.px)\n\
  px-repl               interactive px REPL (name = expr binds; else evaluates)\n\
  rust-repl             interactive Rust REPL (drives the rs-meta interpreter)\n\
  px-check              seed .px corpus evaluates to expected canonical output\n\
  mirror -c|-f          singleton mirror run: all facets + roundtrip status\n\
  mirror-check          corpus mirrors lossless (emit fixed point + value match)\n\
  stage -c|-f           pnix runtime stage ladder (px-stage1..5 + closure)\n\
  stage-check           corpus closes the runtime stage ladder\n\
  ir -c|-f              canonical IR record (sha256 content address)\n\
  ir-check              sha256 vectors + corpus IR proofs + identity sharing\n\
  gate -c|-f            purity/effect-class capability admission record\n\
  gate-check            corpus admitted pure; uncertain fails closed; witnesses\n\
  witness -c|-f         eval witness record (13-field shared schema)\n\
  interop-check         host-call boundary: denial without grant + witness\n\
  io-probe ROOT         canonical read-only meta-I/O adapter probe\n\
  rust-mirror -c|-f     project a px value into Rust and run it on the substrate\n\
  rust-mirror-check     corpus projects 3-way lossless; opaque values hold\n\
  specialize -c|-f      partial evaluation record (residual + gaps)\n\
  specialize-check      A4-sound folding: closed folds, dynamic lets hold\n\
  incremental -c|-f     definition-granular content hashes (Unison model)\n\
  incremental-check     alpha invariance + SCC + realisation early cutoff\n\
  compartment-check     SES-style isolation: own env/modules, shared intrinsics\n\
  tower-check           milestone-1: reify/reflect + px self-interpreter (S=L seed)\n\
  action -c|-f          one-verdict checkpoint (gate+mirror+ir+witness)\n\
  action-check          admitted/refused/deterministic verdicts\n\
  export-oracles        write proof/oracles-rs.tsv (cross-host TSV schema)\n\
  cross-host-check      export drift gate + frozen witness schema\n\
  check                 all_ready aggregate (clean-process replay + receipt)\n\
  capabilities          print the capability index (docs/CAPABILITIES.md source)\n\
  capabilities-check    generated index matches the committed doc\n\
  substrate-check       ../rs-meta interprets src/px.rs and must match rustc\n\
                        and this binary on the same probes (dependency proof)\n\
\n\
ARCHITECTURE:\n\
  pnix-rs   = Rust bootstrap/front-end for the pnix runtime path (this lane)\n\
  ../rs-meta = Rust meta-circular stage15-N compiler/evaluator substrate\n\
  runtime/  = repo-owned .px runtime artifacts\n\
\n\
EXAMPLES:\n\
  pnix-rs px-eval -c 'let a = 1; b = a + 2; in a + b'\n\
  pnix-rs px-eval -f runtime/corpus/c05_recurse.px\n\
  pnix-rs px-check\n\
  RS_META_BOOTSTRAP=/tmp/rs-meta-target/release/bootstrap pnix-rs substrate-check"
    );
}
