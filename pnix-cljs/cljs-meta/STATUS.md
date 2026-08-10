# cljs-meta status (peer host-meta floor)

Last verified: 2026-08-07.

## Peer-floor statement

**cljs-meta** is the ClojureScript host mechanism for PNIX. Its practical peer
floor is a **fixed-point self-hosted compiler**: successive self-recompile
artifacts become byte-identical (stage2 == stage3), after a minimum stage
count (≥15 generations).

| Peer | Peer floor | cljs-meta counterpart |
|---|---|---|
| hy-meta | bootstrap fixed point | stage2/stage3 artifact + source-closure identity |
| rs-meta | stage3 B==C evaluator | fixed-point receipt + runtime evaluate/compile |
| clj-meta | bytecode selfhost | self-compiled analyzer/compiler/cljs.js payload |
| clr-meta | C0–C3 Stage1/2 | JVM stage0 → self-hosted stage1+ fixed point |

Explicit trust root remains: Node.js, Google Closure runtime, `cljs.core`,
reader runtime, macro bootstrap kernel, stage harness, analysis cache.
Stage0 JVM compiler is **not** packaged in the fixed artifact.

Cross-platform: only `x86_64-darwin` is checked closed in FIXED-POINT.md;
other platforms are `platform-pending`.

## Closed claims

Live-verified this session (2026-08-07):

```text
./bin/build-cljs stage0 artifacts         OK (cli/module/stage-runtime)
fixed-point builder                       OK  stage_count=15, fixed_point=true
  stage2_artifact_sha256 == stage3        true (1789364bda06a674…)
  source_closure_equal                    true
  stage0_compiler_embedded                false
  bootstrap_only_markers_absent           true
  compiler_payload_self_hosted            true
node cljs-meta/test/self_test.js          PASS
node cljs-meta/test/fixed_point_test.js   PASS
./cljs-meta/bin/cljs-meta-gate            PASS
```

## Open claims (do not claim)

```text
multi_platform_byte_determinism = platform-pending (non-x86_64-darwin)
trusting-trust_defense = false
pnix_language_semantics_ownership = false
independent_of_Node_Closure_cljs.core = false
full_ClojureScript_product_replacement = false
```

## Primary gate

```sh
# From pnix-cljs/
./cljs-meta/bin/cljs-meta-gate           # uses dist if present; builds if missing
./cljs-meta/bin/cljs-meta-gate --rebuild # force ./bin/build-cljs
```

Requires: JDK + Clojure CLI + Node.js. Cold fixed-point rebuild is multi-minute.

## Last run (this machine, 2026-08-07)

| Gate | Result | Notes |
|---|---|---|
| stage0 JVM compile | **PASS** | local cljs snapshot clojurescript-r1.12.145 |
| fixed-point (≥15 gens) | **PASS** | receipt.fixed_point=true |
| `cljs-meta-gate` | **PASS** | self_test + fixed_point_test |
| env | Node v26.7.0, OpenJDK 21 | OK |
