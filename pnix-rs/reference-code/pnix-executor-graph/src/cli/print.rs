//! CLI 출력: 도움말 및 정보 출력

use anyhow::Result;

use super::args::{AgentVerb, ExecMode, GateAbsorbVerb, GateForwardVerb, GateReadVerb};

pub(super) fn print_ir_eval_ops() -> Result<()> {
  println!(
    "{}",
    serde_json::to_string(&pnix_runtime_legacy::ir::ir_eval_support_table_json())?
  );
  Ok(())
}

pub(super) fn mode_label(mode: ExecMode) -> &'static str {
  match mode {
    ExecMode::Run => "run",
    ExecMode::Interpret => "interpret",
    ExecMode::Compile => "compile",
    ExecMode::Graph => "graph",
    ExecMode::LegacyEval => "legacy-eval",
    ExecMode::LegacyFrp => "legacy-frp",
    ExecMode::Ct => "ct",
    ExecMode::Llvm => "llvm",
    ExecMode::Test => "test",
    ExecMode::Fmt => "fmt",
    ExecMode::Lint => "lint",
  }
}

pub(super) fn print_inputs_schema() -> Result<()> {
  let schema = serde_json::json!({
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "title": "pnix executor inputs",
      "description": "External inputs for FxCore graph execution",
      "type": "object",
      "additionalProperties": true
  });
  let text = serde_json::to_string_pretty(&schema)?;
  println!("{}", text);
  Ok(())
}

pub(super) fn print_modes() {
  println!("coding-agent");
  println!("gate-absorb");
  println!("gate-forward");
  println!("gate-read");
  println!("compile");
  println!("run");
  println!("interpret");
  println!("test");
  println!("ops");
  println!("fmt");
  println!("lint");
}

pub(super) fn print_version(bin_name: &str) {
  println!(
    "{} {} (runtime-api {})",
    bin_name,
    env!("CARGO_PKG_VERSION"),
    pnix_runtime_api::RUNTIME_API_VERSION
  );
}

pub(super) fn print_help(bin_name: &str) {
  eprintln!("Usage: {} [--dist <dir>] [options]", bin_name);
  eprintln!("       {} fmt [--check]", bin_name);
  eprintln!("       {} lint", bin_name);
  eprintln!(
    "       {} coding-agent <ask|plan|patch|verify|rollback|decide|retention> [--help]",
    bin_name
  );
  eprintln!(
    "       {} gate-absorb <help|url|topic|conversation|events> [--help]",
    bin_name
  );
  eprintln!("       {} gate-forward [help] [--help]", bin_name);
  eprintln!(
    "       {} gate-read <help|status|state-sink-contract|store-budget|artifact-ref-ratio|storage-telemetry|recent-events|candidates|brain-bundle-contract|validate-brain-bundle|curriculum-current-target|ontology-lookup-related|recipe-match-current|query-context> [--help]",
    bin_name
  );
  eprintln!(
    "       {} serve [FILE|-] [--expr <code>] [--host <host>] [--port <port>]",
    bin_name
  );
  eprintln!();
  eprintln!("Options:");
  eprintln!("  --mode <mode>         compile|run|interpret|test|ops");
  eprintln!("  --run                 Alias for --mode run");
  eprintln!("  --interpret           Alias for --mode interpret");
  eprintln!("  --engine <engine>     Select engine within run/interpret (see below)");
  eprintln!("  --result <node[.port]> Select result for run --engine ir-eval");
  eprintln!("  --list-modes          Print supported mode names and exit");
  eprintln!("  --version             Print executor + runtime-api versions and exit");
  eprintln!("  --dist <dir>          dist directory (compile/run/graph) or source dist (emit)");
  eprintln!("  --emit                Emit backend code or AOT artifacts from dist");
  eprintln!("  --emit-target <t>     js|ts|python|clojure|nix|all|aot[:target]");
  eprintln!("  --target <t>          AOT target alias (sets --emit-target aot:<target>)");
  eprintln!("  --binary              shortcut for --emit-target aot (compile to binary)");
  eprintln!("  --emit-out <dir>      output directory for emit artifacts");
  eprintln!("  --emit-manifest <p>   write emit summary JSON to path");
  eprintln!(
    "  --live                enable live state path (run/interpret --engine ui, or interpret --engine legacy-eval/eval)"
  );
  eprintln!(
    "  --live-dir <dir>      override live directory (implies --live; REPL state for interpret legacy-eval/eval)"
  );
  eprintln!("  --fmt                 Alias for 'pnix fmt'");
  eprintln!("  --lint                Alias for 'pnix lint'");
  eprintln!("  coding-agent          bounded coding-agent namespace (CAX.1a skeleton)");
  eprintln!("  gate-absorb           standalone absorb lane (url/topic/conversation/events)");
  eprintln!("  gate-forward          standalone candidate -> doghouse ingress transport lane");
  eprintln!("  gate-read             standalone read-only gate operator lane");
  eprintln!("  --op <name>           Supervisor RPC op name (ops mode)");
  eprintln!("  --payload <json|@file> Supervisor RPC payload JSON (ops mode)");
  eprintln!("  --caps <c1,c2,...>    Capability list passed to supervisor (ops mode)");
  eprintln!("  --check               Validate formatting (fmt only)");
  eprintln!(
    "  --patch <file>        apply patch (run/graph/interpret/legacy-eval/legacy-frp/llvm)"
  );
  eprintln!("  --source <file>       source file (compile/interpret; also run compiles first)");
  eprintln!("  --expr <code>         inline code (compile/interpret; also run compiles first)");
  eprintln!("  --json                legacy-eval compat output (json)");
  eprintln!("  --raw                 legacy-eval compat output (raw string)");
  eprintln!("  --pretty              legacy-eval compat output (pretty)");
  eprintln!("  --edn                 legacy-eval compat output (edn)");
  eprintln!("  --stack-limit <n>     legacy-eval/serve compat stack limit");
  eprintln!("  --filter <pattern>    test filter (mode test only)");
  eprintln!("  --inputs <file>       JSON object file for external inputs");
  eprintln!("  --inputs-json <json>  JSON object string for external inputs");
  eprintln!("  --input <k=v>         single input (value parsed as JSON if possible)");
  eprintln!("  --inputs-schema       Print JSON schema for external inputs and exit");
  eprintln!("  --list-ir-eval-ops    Print ir-eval supported ops JSON and exit");
  eprintln!("  --dry-run             Skip apply/write (run|graph|compile)");
  eprintln!("  --seed <u64>          deterministic seed for runtime");
  eprintln!("  --now <ms>            override current time (milliseconds)");
  eprintln!("  --clock-step <ms>     fixed time step for runtime ticks");
  eprintln!("  --dt <seconds>        legacy-frp tick delta (used when --clock-step is unset)");
  eprintln!("  --clojure-url <url>   Clojure backend URL (default: http://localhost:7777)");
  eprintln!("  --python-url <url>    Python backend URL (default: http://localhost:7778)");
  eprintln!("  --deno-url <url>      Deno backend URL (default: http://localhost:7779)");
  eprintln!("  --blenderpy-url <url> BlenderPy backend URL (default: http://localhost:7781)");
  eprintln!("  --supervisor-sock <p> UDS path for process supervisor (graph/ops mode)");
  eprintln!("  --backend-specs <p>   JSON file of backend process specs to ensure via supervisor");
  eprintln!("  --replay <path>       Replay trace jsonl path (graph mode)");
  eprintln!("  --replay-mode <m>     Replay mode: off|strict|nondet-safe|verify");
  eprintln!(
    "  --replay-allow <cls>  Allow executing replay class even in replay mode (repeatable)"
  );
  eprintln!("  --invocation-id <id>  Invocation id stamped into trace meta");
  eprintln!("  --rpc-timeout-ms <u64> Backend RPC timeout in milliseconds (default: 30000)");
  eprintln!("  --rpc-retry-attempts <n> Backend RPC retry attempts (default: 3)");
  eprintln!("  --rpc-retry-backoff-ms <u64> Backend RPC retry backoff base (default: 100)");
  eprintln!("  --max-nodes <n>       Max nodes allowed in FxCore graph (default: 10000)");
  eprintln!("  --max-edges <n>       Max edges allowed in FxCore graph (default: 50000)");
  eprintln!("  --max-input-bytes <n> Max external input JSON bytes (default: 10485760)");
  eprintln!("  --no-batch            Disable batch apply_graph op (use individual calls)");
  eprintln!("  --non-deterministic   Allow non-deterministic runtime behavior");
  eprintln!("  --lenient-ct          Disable strict CT checks");
  eprintln!(
    "  env PNIX_ALLOW_NON_ATOMIC_EFFECTS=1  Allow world/unknown effects (no rollback guarantee)"
  );
  eprintln!();
  eprintln!("Ops mode:");
  eprintln!("  {} --mode ops --op list", bin_name);
  eprintln!(
    "  {} --mode ops --op preflight --payload @preflight.json",
    bin_name
  );
  eprintln!(
    "  {} --mode ops --op kpi.snapshot --payload '{{\"ns\":\"default\"}}'",
    bin_name
  );
  eprintln!(
    "  {} ops --op admission.check --payload @preflight.json",
    bin_name
  );
  eprintln!();
  eprintln!("Run engines (use with --mode run):");
  eprintln!("  --engine auto         Auto-select graph vs ir-eval based on FxCore");
  eprintln!("  --engine graph        Apply FxCore graph (default)");
  eprintln!("  --engine ir-eval      Evaluate FxCore via ir-eval (offline)");
  eprintln!("  --engine ssa          Evaluate dist/ir/ssa.canon.json (subset)");
  eprintln!("  --engine parity       Batch ir/ssa/legacy/llvm eval (env-driven)");
  eprintln!("  --engine ui           Evaluate and emit FramePacket JSON (ui spec)");
  eprintln!("  --engine emit         Emit backend code / AOT artifacts from dist");
  eprintln!("  --engine llvm         JIT-execute dist/ir/fxcore.canon.json (feature-gated)");
  eprintln!();
  eprintln!("Interpret engines (use with --mode interpret):");
  eprintln!(
    "  --engine legacy-eval  Evaluate Pnix expression (default, compat warning on explicit use)"
  );
  eprintln!("  --engine ct           CT verification for an expression");
  eprintln!("  --engine legacy-frp   Run legacy FRP graph JSON (compat warning)");
  eprintln!("  --engine ui           Evaluate expression and emit FramePacket JSON");
  eprintln!();
  eprintln!("Serve subcommand (compat):");
  eprintln!("  pnix serve <file|-> [--expr <code>] [--host <host>] [--port <port>]");
  eprintln!("  --max-body-bytes <n>  max request payload bytes (default: 1048576)");
  eprintln!("  --stack-limit <n>     evaluation stack limit");
  eprintln!();
  eprintln!("Legacy-eval compat (no --mode):");
  eprintln!("  pnix [FILE|-] [--expr <code>] [--json|--raw|--pretty|--edn]");
  eprintln!("  (maps to legacy evaluator semantics)");
  eprintln!();
  eprintln!("Advanced/compat (still supported):");
  eprintln!("  --mode graph|legacy-eval|legacy-frp|ct|llvm (legacy-* modes print compat warnings)");
  eprintln!("  --emit (graph-only legacy flag; prefer --mode run --engine emit)");
}

pub(super) fn print_agent_help(bin_name: &str, verb: Option<AgentVerb>) {
  match verb {
    None => {
      eprintln!(
        "Usage: {} coding-agent <ask|plan|patch|verify|rollback|decide|retention> [--help]",
        bin_name
      );
      eprintln!();
      eprintln!("Coding-agent verbs:");
      eprintln!("  ask       normalize coding request and capture bounded task intent");
      eprintln!("  plan      render proposal-first bounded plan surface");
      eprintln!("  patch     open typed patch-proposal/apply-intent lane");
      eprintln!("  verify    open before/after verify-receipt lane");
      eprintln!("  rollback  open typed rollback-handle/receipt lane");
      eprintln!("  decide    record human promotion judgement packet");
      eprintln!("  retention plan append-only coding memory retention receipt");
      eprintln!();
      eprintln!("Request normalization flags:");
      eprintln!("  --request <text>                task/request text");
      eprintln!("  --target-path <path>            repeatable target scope path");
      eprintln!("  --project-pack-root <path>      repeatable attached project pack root");
      eprintln!("  --history-pack-root <path>      repeatable attached history pack root");
      eprintln!("  --approved-command <cmd>        repeatable allowed command");
      eprintln!("  --forbidden-path <path>         repeatable blocked path");
      eprintln!("  --workspace-policy <bit>        repeatable policy bit");
      eprintln!("  --current-plan-ref <ref>        current plan artifact ref");
      eprintln!("  --rollback-handle-ref <ref>     rollback handle artifact ref");
      eprintln!("  --last-verification-ref <ref>   last verification artifact ref");
      eprintln!(
        "  --promotion-boundary-ref <ref>  prior promotion-boundary receipt ref for verify join"
      );
      eprintln!("  --source-apply-artifact-ref <ref>");
      eprintln!(
        "                                  apply-result ref paired with --promotion-boundary-ref"
      );
      eprintln!("  --source-handoff-ref <ref>      optional apply-handoff-proof ref paired with verify join");
      eprintln!("  --promotion-boundary-join-ref <ref>");
      eprintln!("                                  prior promotion-boundary-join receipt ref for human decision");
      eprintln!("  --promotion-decision <accepted|rejected|held>");
      eprintln!(
        "                                  human judgement decision for coding-agent decide"
      );
      eprintln!("  --candidate-patch <path>        quarantine provider/generated patch text as candidate-only");
      eprintln!("  --provider-feedback-request-ref <ref>");
      eprintln!("                                  mark --candidate-patch as revised feedback response lineage");
      eprintln!("  --agent-request-out <path>      write coding.request JSON artifact");
      eprintln!("  --agent-plan-out <path>         write coding.plan JSON artifact");
      eprintln!("  --agent-patch-out <path>        write coding.patch-proposal JSON artifact");
      eprintln!("  --agent-verify-out <path>       write coding.verify-receipt JSON artifact");
      eprintln!(
        "  --agent-rollback-out <path>     write coding.rollback artifact JSON (handle or receipt)"
      );
      eprintln!(
        "  --agent-decision-out <path>     write coding.human-promotion-decision JSON artifact"
      );
      eprintln!();
      eprintln!("Current status:");
      eprintln!("  CAX.1a namespace/help/listing is landed.");
      eprintln!("  CAX.1b request/workspace snapshot artifact is landed.");
      eprintln!("  CAX.1c bounded plan/status artifact is landed for `coding-agent plan`.");
      eprintln!("  CAX.2a grounding seed (`path/language/parser_backend/parser_capability`) is landed in request/plan artifacts.");
      eprintln!("  CAX.2b repo graph seed (`file/symbol/use/reference/test/runtime`) is landed as bounded multi-file project summary with explicit non-project-cache status markers.");
      eprintln!("  CAX.2c joined manual/docset evidence seed is landed as read-side evidence only; it never justifies patch apply by itself.");
      eprintln!(
        "  CAX.3a partial patch-proposal/apply-intent artifact is landed for `coding-agent patch`."
      );
      eprintln!(
        "  CAX.3b partial verify-receipt/before-after proof artifact is landed for `coding-agent verify`."
      );
      eprintln!(
        "  CAX.3c partial rollback-handle/rollback-receipt/effect-contract artifact is landed for `coding-agent rollback`."
      );
      eprintln!(
        "  CAX.4a landed store-side coding artifact tables/indexes; CAX.4b partial optional append + read-only compare/replay query are wired behind DOGHOUSE_STORE_PATH and doghouse retrieval."
      );
      eprintln!(
        "  CAX.4b retention receipt CLI is landed for `coding-agent retention`; it appends a store-owned plan and never deletes artifacts."
      );
      eprintln!(
        "  CAX.4c read-only coding memory projection is landed in doghouse retrieval/auto-learn/conversation."
      );
      eprintln!(
        "  CAX.4d partial project/history pack attach roots are surfaced in coding-agent request artifacts."
      );
      eprintln!(
        "  CAX.5a pnix/Rust language profile skeleton is landed as semantic/effect/verify-target records inside request artifacts; adapters do not judge or promote."
      );
      eprintln!(
        "  CAX.5b planned/unsupported adapters now emit diagnostic-record, failure-pattern-match, and context-demand candidates instead of opening mutation."
      );
      eprintln!(
        "  CAX.5c Python/TypeScript/Nix/Clojure planned adapters now emit record-producer-only semantic/effect/verify-target candidates."
      );
      eprintln!(
        "  CAX.5d language verify-target candidates are linked into execution-plan/execution-request, but candidate commands stay non-executable until approved."
      );
      eprintln!(
        "  CAX.5e failed/blocked verify execution-results now lower into diagnostic/failure-pattern/context-demand candidates without promoting raw stdout/stderr refs."
      );
      eprintln!(
        "  CAX.5f patch proposals now carry semantic-patch-review with meaning-impact, decision-link, and narrative-regression candidates."
      );
      eprintln!(
        "  CAX.5g patch proposals now replay prior verify context-demands and semantic reviews from coding memory as candidate-only next-patch requirements."
      );
      eprintln!(
        "  CAX.5h patch proposals now replay prior learning-card repair patterns from coding memory as candidate-only repair recipes."
      );
      eprintln!(
        "  CAX.5i patch proposals now quarantine provider/generated patch text as candidate-only generated-patch-candidate artifacts."
      );
      eprintln!(
        "  CAX.5j generated patch candidates now produce review receipts and context-demand candidates for invalid, mismatched, or unverifiable patches."
      );
      eprintln!(
        "  CAX.5k generated patch review context-demands now project to provider-feedback-request packets without making provider output truth/apply owner."
      );
      eprintln!(
        "  CAX.5l provider feedback responses now reingest through revised generated-patch-candidate lineage only."
      );
      eprintln!(
        "  CAX.5m generated patch feedback retry loops now stop at feedback-retry-guard and human review escalation."
      );
      eprintln!(
        "  CAX.5n reviewed generated patches now require apply-handoff-proof before explicit --patch mutation."
      );
      eprintln!(
        "  CAX.5o successful apply results now emit promotion-boundary-receipt instead of becoming promotion proof."
      );
      eprintln!(
        "  CAX.5p verify receipts now join prior promotion-boundary receipts to apply/handoff lineage without opening promotion ownership."
      );
      eprintln!(
        "  CAX.5q human promotion decisions now emit append-only judgement packets without opening merge/release ownership."
      );
      eprintln!("  Generated patch apply and generic checkpoint rollback execution are not implemented yet.");
      eprintln!("  Upstream executor remains the owner; downstream wrappers attach later.");
    }
    Some(verb) => {
      eprintln!(
        "Usage: {} coding-agent {} [--help]",
        bin_name,
        verb.as_str()
      );
      eprintln!();
      eprintln!("Current status:");
      eprintln!(
        "  '{}' is parsed and listed as a first-class agent verb.",
        verb.as_str()
      );
      match verb {
        AgentVerb::Plan => {
          eprintln!(
            "  This lane emits `coding.plan` with bounded step family, expected verification, and failure policy."
          );
          eprintln!(
            "  `--agent-plan-out` writes the plan artifact; patch/apply/verify execution still lands in later CAX bundles."
          );
        }
        AgentVerb::Patch => {
          eprintln!(
            "  This lane emits `coding.patch-proposal` with target paths, edit family, diff ref, risk class, and separated apply intent."
          );
          eprintln!(
            "  `--candidate-patch` reads provider/generated patch text as quarantine-only evidence; it is never applied by this flag."
          );
          eprintln!(
            "  `--provider-feedback-request-ref` links that candidate to a prior feedback request without making provider prose truth."
          );
          eprintln!(
            "  `--agent-patch-out` writes the patch proposal artifact; generated patch auto-apply and generic rollback execution still land later."
          );
        }
        AgentVerb::Verify => {
          eprintln!(
            "  This lane emits `coding.verify-receipt` with repo snapshot ref, before/after artifact refs, target commands, diff ref, and proof refs."
          );
          eprintln!(
            "  `--promotion-boundary-ref` + `--source-apply-artifact-ref` emits a CAX.5p promotion-boundary-join-receipt."
          );
          eprintln!(
            "  `--agent-verify-out` writes the verify receipt artifact; approved commands run only through the bounded direct runner unless --dry-run is set."
          );
        }
        AgentVerb::Rollback => {
          eprintln!(
            "  This lane emits `coding.rollback-handle` by default, and `coding.rollback-receipt` when `--rollback-handle-ref` is supplied."
          );
          eprintln!(
            "  `--agent-rollback-out` writes the rollback artifact; explicit inverse diff rollback is supported, while generic checkpoint rollback still lands later."
          );
        }
        AgentVerb::Decide => {
          eprintln!(
            "  This lane emits `coding.human-promotion-decision` from a prior promotion-boundary-join receipt."
          );
          eprintln!(
            "  `--promotion-decision accepted|rejected|held` records judgement only; merge/release/promotion execution remains a separate owner."
          );
          eprintln!("  `--agent-decision-out` writes the decision artifact JSON.");
        }
        AgentVerb::Retention => {
          eprintln!(
            "  This lane reads DOGHOUSE_STORE_PATH and emits `coding.retention-receipt` with keep/compact-candidate decisions."
          );
          eprintln!(
            "  It appends the receipt unless `--dry-run` is set; it never deletes or compacts artifacts."
          );
        }
        _ => {
          eprintln!(
            "  This lane exposes parse/help plus request/workspace snapshot artifact output."
          );
          eprintln!(
            "  Grounded planning is partially landed via repo/manual evidence seeds; verify and rollback artifacts land in later CAX bundles."
          );
        }
      }
    }
  }
}

pub(super) fn print_gate_absorb_help(bin_name: &str, verb: Option<&GateAbsorbVerb>) {
  match verb {
    None => {
      println!("pnix-gate-absorb 0.6.1 — standalone knowledge absorption CLI");
      println!();
      println!("Usage:");
      println!("  {} gate-absorb help", bin_name);
      println!(
        "  {} gate-absorb url <URL> [--follow-related N] [--dry-run]",
        bin_name
      );
      println!("  {} gate-absorb topic <QUERY> [--dry-run]", bin_name);
      println!("  {} gate-absorb conversation <FILE> [--dry-run]", bin_name);
      println!(
        "  {} gate-absorb events [EVENTS.jsonl] [--limit N] [--reset] [--dry-run]",
        bin_name
      );
      println!();
      println!("Current status:");
      println!("  URL dry-run is upstream pnix-owned (file:// + http(s) fetch + sha256 summary).");
      println!("  Conversation dry-run is upstream pnix-owned (JSON/plain transcript parse + language/token summary).");
      println!("  Hook event distill/ObservationAtom emit owner is moving to upstream `pnix gate-absorb events`.");
      println!("  Topic remains skeleton; non-dry-run emit/promotion is not implemented yet.");
      println!("  Babashka absorb entrypoints are retired from operator-facing flow.");
    }
    Some(GateAbsorbVerb::Url) => {
      println!(
        "Usage: {} gate-absorb url <URL> [--follow-related N] [--dry-run]",
        bin_name
      );
      println!("  dry-run: fetch + hash + content-type summary only");
      println!("  non-dry-run emit is not implemented yet");
    }
    Some(GateAbsorbVerb::Topic) => {
      println!("Usage: {} gate-absorb topic <QUERY> [--dry-run]", bin_name);
      println!("  topic traversal remains skeleton in current phase");
    }
    Some(GateAbsorbVerb::Conversation) => {
      println!(
        "Usage: {} gate-absorb conversation <FILE> [--dry-run]",
        bin_name
      );
      println!("  dry-run: parse transcript + language/token summary only");
      println!("  non-dry-run emit is not implemented yet");
    }
    Some(GateAbsorbVerb::Events) => {
      println!(
        "Usage: {} gate-absorb events [EVENTS.jsonl] [--limit N] [--reset] [--dry-run]",
        bin_name
      );
      println!("  emit: lower pnix-gate hook events.jsonl into ObservationAtom candidates");
      println!("  default path: <gate-store>/events.jsonl");
    }
    Some(other) => {
      println!(
        "Usage: {} gate-absorb <help|url|topic|conversation|events>",
        bin_name
      );
      println!("  unknown gate-absorb verb: {}", other.as_str());
    }
  }
}

pub(super) fn print_gate_forward_help(bin_name: &str, verb: Option<&GateForwardVerb>) {
  match verb {
    None | Some(GateForwardVerb::Run) => {
      eprintln!(
        "Usage: {} gate-forward [--limit N] [--kind PREFIX] [--dry-run] [--reset] [--url URL]",
        bin_name
      );
      eprintln!();
      eprintln!("Forward flags:");
      eprintln!("  --limit <n>       process at most N candidate files (default: 20)");
      eprintln!("  --kind <prefix>   filename prefix filter (example: observation-atom)");
      eprintln!("  --dry-run         report unique forward set without transport/write");
      eprintln!("  --reset           ignore prior sent state and resend");
      eprintln!("  --url <base>      doghouse-http base URL override");
      eprintln!("  --output-format <text|json>  output report surface");
      eprintln!();
      eprintln!("Current status:");
      eprintln!(
        "  Babashka `pnix-gate-mcp.forwarder/forward!` owner has been replaced by upstream `pnix gate-forward`."
      );
      eprintln!(
        "  Transport semantics are: prefer doghouse HTTP `POST /candidate`, fall back to runtime `px-candidates/` file-drop, else report local-only."
      );
      eprintln!(
        "  Semantic dedupe is keyed by `kind + content-hash + provenance tuple + convergence-closes`, not filename only."
      );
    }
    Some(GateForwardVerb::Help) => {
      print_gate_forward_help(bin_name, None);
    }
    Some(GateForwardVerb::Unknown(other)) => {
      eprintln!("unknown gate-forward subcommand: {}", other);
      print_gate_forward_help(bin_name, None);
    }
  }
}

pub(super) fn print_gate_read_help(bin_name: &str, verb: Option<&GateReadVerb>) {
  match verb {
    None => {
      println!("pnix gate-read 0.5.0 — standalone read-only gate operator CLI");
      println!();
      println!("Usage:");
      println!("  {} gate-read help", bin_name);
      println!("  {} gate-read status", bin_name);
      println!("  {} gate-read state-sink-contract", bin_name);
      println!("  {} gate-read ontology-coverage", bin_name);
      println!("  {} gate-read meaning-bridges", bin_name);
      println!("  {} gate-read self-capabilities", bin_name);
      println!("  {} gate-read meta-protocols", bin_name);
      println!("  {} gate-read lift-rule-coverage", bin_name);
      println!("  {} gate-read store-budget", bin_name);
      println!("  {} gate-read artifact-ref-ratio", bin_name);
      println!("  {} gate-read storage-telemetry", bin_name);
      println!("  {} gate-read provenance-floor", bin_name);
      println!("  {} gate-read unsupported-kind-floor", bin_name);
      println!("  {} gate-read lineage-floor", bin_name);
      println!(
        "  {} gate-read recent-events [--limit N] [--event-type <NAME>]...",
        bin_name
      );
      println!(
        "  {} gate-read candidates [--limit N] [--kind <PREFIX>]",
        bin_name
      );
      println!("  {} gate-read brain-ankh-policy [--limit N]", bin_name);
      println!("  {} gate-read brain-bundle-contract", bin_name);
      println!(
        "  {} gate-read validate-brain-bundle --path <FILE> [--proof-path <FILE>] [--schema-path <FILE>] [--expected-bundle-kind <KIND>] [--expected-lobe-profile <PROFILE>] [--expected-proof-kind <KIND>]",
        bin_name
      );
      println!("  {} gate-read curriculum-current-target", bin_name);
      println!(
        "  {} gate-read ontology-lookup-related --context <TEXT> [--predicate <PRED>] [--limit N] [--min-confidence F]",
        bin_name
      );
      println!(
        "  {} gate-read recipe-match-current --tool-name <TOOL> [--context <TEXT>] [--arg-predicate <PRED>]... [--limit N] [--min-confidence F]",
        bin_name
      );
      println!(
        "  {} gate-read query-context --topic <TEXT> [--limit N]",
        bin_name
      );
      println!();
      println!("This lane is read-only. It reads gate state, control-plane snapshots, and owner-backed lookup/ranking surfaces without invoking Babashka directly.");
    }
    Some(GateReadVerb::Status) => {
      println!("Usage: {} gate-read status", bin_name);
      println!(
        "Reads the gate store directly and reports event/candidate totals without invoking Babashka."
      );
    }
    Some(GateReadVerb::StateSinkContract) => {
      println!("Usage: {} gate-read state-sink-contract", bin_name);
      println!(
        "Reads the profile-neutral lifecycle sink ABI and current materialized presence directly from pnix."
      );
    }
    Some(GateReadVerb::OntologyCoverage) => {
      println!("Usage: {} gate-read ontology-coverage", bin_name);
      println!(
        "Reads live `.px` ontology surfaces directly and reports structural coverage without invoking Babashka."
      );
    }
    Some(GateReadVerb::MeaningBridges) => {
      println!("Usage: {} gate-read meaning-bridges", bin_name);
      println!(
        "Reads live meaning-bridges.px directly and reports latent A-surface -> hidden -> B-surface bridge structure without invoking Babashka."
      );
    }
    Some(GateReadVerb::SelfCapabilities) => {
      println!("Usage: {} gate-read self-capabilities", bin_name);
      println!(
        "Reads live self-capabilities.px directly and reports explicit self-observation and upgrade-path coverage without invoking Babashka."
      );
    }
    Some(GateReadVerb::MetaProtocols) => {
      println!("Usage: {} gate-read meta-protocols", bin_name);
      println!(
        "Reads live meta-protocols.px directly and reports reuse-loop protocol readiness without invoking Babashka."
      );
    }
    Some(GateReadVerb::LiftRuleCoverage) => {
      println!("Usage: {} gate-read lift-rule-coverage", bin_name);
      println!(
        "Reads live lift-rules.px directly and reports canonical 7-kind lowering coverage without invoking Babashka."
      );
    }
    Some(GateReadVerb::StoreBudget) => {
      println!("Usage: {} gate-read store-budget", bin_name);
      println!(
        "Reads the hot-store backpressure budget and HotStoreBudgetCheckpoint summary directly from pnix."
      );
    }
    Some(GateReadVerb::ArtifactRefRatio) => {
      println!("Usage: {} gate-read artifact-ref-ratio", bin_name);
      println!(
        "Reads candidate + durable state sink `.px` field-line artifact_ref coverage directly from pnix."
      );
    }
    Some(GateReadVerb::StorageTelemetry) => {
      println!("Usage: {} gate-read storage-telemetry", bin_name);
      println!(
        "Reads composite storage telemetry by composing upstream gate-read subreports and control-plane storage snapshots."
      );
    }
    Some(GateReadVerb::ProvenanceFloor) => {
      println!("Usage: {} gate-read provenance-floor", bin_name);
      println!(
        "Reads candidate/state sink inventory directly and reports whether weak Accepted provenance is quarantined into missing-provenance instead of leaking."
      );
    }
    Some(GateReadVerb::UnsupportedKindFloor) => {
      println!("Usage: {} gate-read unsupported-kind-floor", bin_name);
      println!(
        "Reads candidate/state sink inventory directly and reports whether non-canonical kinds are quarantined into unsupported-kind instead of leaking."
      );
    }
    Some(GateReadVerb::LineageFloor) => {
      println!("Usage: {} gate-read lineage-floor", bin_name);
      println!(
        "Reads candidate/state sink inventory directly and reports whether canonical source-session/source-turn and reopen/retire lineage anchors are present without invoking Babashka."
      );
    }
    Some(GateReadVerb::RecentEvents) => {
      println!(
        "Usage: {} gate-read recent-events [--limit N] [--event-type <NAME>]...",
        bin_name
      );
      println!(
        "Reads .store/events.jsonl directly and supports the same last-N + event-name filter shape as legacy list_recent_events."
      );
    }
    Some(GateReadVerb::Candidates) => {
      println!(
        "Usage: {} gate-read candidates [--limit N] [--kind <PREFIX>]",
        bin_name
      );
      println!(
        "Reads .store/px/candidates directly and supports the same filename-prefix filter shape as legacy list_candidates."
      );
    }
    Some(GateReadVerb::BrainAnkhPolicy) => {
      println!(
        "Usage: {} gate-read brain-ankh-policy [--limit N]",
        bin_name
      );
      println!(
        "Reads existing gate candidate kinds plus DOGHOUSE_STORE_PATH coding.* artifacts and projects read-only ankh policy-loop packets without opening an ankh.* store kind."
      );
    }
    Some(GateReadVerb::BrainBundleContract) => {
      println!("Usage: {} gate-read brain-bundle-contract", bin_name);
      println!(
        "Reads the portable brain-bundle contract and example validation surface directly from pnix."
      );
    }
    Some(GateReadVerb::ValidateBrainBundle) => {
      println!(
        "Usage: {} gate-read validate-brain-bundle --path <FILE> [--proof-path <FILE>] [--schema-path <FILE>] [--expected-bundle-kind <KIND>] [--expected-lobe-profile <PROFILE>] [--expected-proof-kind <KIND>]",
        bin_name
      );
      println!(
        "Validates a portable brain-bundle JSON payload and its proof/schema references without invoking Babashka."
      );
    }
    Some(GateReadVerb::CurriculumCurrentTarget) => {
      println!("Usage: {} gate-read curriculum-current-target", bin_name);
      println!(
        "Reads .store/control-plane/curriculum-state.json and projects the current target ABI."
      );
    }
    Some(GateReadVerb::OntologyLookupRelated) => {
      println!(
        "Usage: {} gate-read ontology-lookup-related --context <TEXT> [--predicate <PRED>] [--limit N] [--min-confidence F]",
        bin_name
      );
      println!("Runs the owner-backed lookup-rules/lookup-select path directly from pnix.");
    }
    Some(GateReadVerb::RecipeMatchCurrent) => {
      println!(
        "Usage: {} gate-read recipe-match-current --tool-name <TOOL> [--context <TEXT>] [--arg-predicate <PRED>]... [--limit N] [--min-confidence F]",
        bin_name
      );
      println!(
        "Reads live repair-recipes owner surface, preserves typed warning/blocked lineage ABI, and appends runtime telemetry to gate events.jsonl."
      );
    }
    Some(GateReadVerb::QueryContext) => {
      println!(
        "Usage: {} gate-read query-context --topic <TEXT> [--limit N]",
        bin_name
      );
      println!(
        "Aggregates accepted fact hits, candidate hits, event hits, and recipe hits around one topic."
      );
    }
    Some(GateReadVerb::Missing) => {
      println!(
        "Usage: {} gate-read <help|status|state-sink-contract|ontology-coverage|meaning-bridges|self-capabilities|meta-protocols|lift-rule-coverage|store-budget|artifact-ref-ratio|storage-telemetry|provenance-floor|unsupported-kind-floor|lineage-floor|recent-events|candidates|brain-ankh-policy|brain-bundle-contract|validate-brain-bundle|curriculum-current-target|ontology-lookup-related|recipe-match-current|query-context>",
        bin_name
      );
    }
    Some(GateReadVerb::Help) => {
      print_gate_read_help(bin_name, None);
    }
    Some(GateReadVerb::Unknown(other)) => {
      println!(
        "Usage: {} gate-read <help|status|state-sink-contract|ontology-coverage|meaning-bridges|self-capabilities|meta-protocols|lift-rule-coverage|store-budget|artifact-ref-ratio|storage-telemetry|provenance-floor|unsupported-kind-floor|lineage-floor|recent-events|candidates|brain-ankh-policy|brain-bundle-contract|validate-brain-bundle|curriculum-current-target|ontology-lookup-related|recipe-match-current|query-context>",
        bin_name
      );
      println!("  unknown gate-read verb: {}", other.as_str());
    }
  }
}
