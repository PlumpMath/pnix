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

## Trusting-Trust defense roadmap (Diverse Double-Compiling)

**Nothing closed yet on this axis.** The existing fixed-point proof
(stage2 == stage3, byte-identical after >=15 self-recompiles) is *reproducibility*
evidence, not Trusting-Trust defense — a backdoor baked into stage0/stage1 would
reproduce itself identically forever and this check would still pass. No
independently-authored third-party ClojureScript compiler exists to lean on
either: alternatives like shadow-cljs still compile through the same official
`cljs.core`/analyzer lineage this fixed point already depends on, so they would
not catch a defect in that shared lineage.

Concrete plan, following clj-meta's already-closed U6 pattern (same host
language family, JS emit instead of JVM bytecode):

```text
Step 1 — independent minimal 2nd backend
    Author a small, algorithmically independent ClojureScript-subset-to-JS
    emitter: its own tiny reader/analyzer/emit path, zero shared code with the
    self-hosted analyzer/compiler/cljs.js payload this fixed point produces.
    Bound the target surface the same way clj-meta's frontend_selfhost.clj
    does (fn/if/do/let/loop-recur/arithmetic/compare/literals/quote is a
    reasonable first slice) rather than attempting full ClojureScript.

Step 2 — cross-validate against the fixed-point compiler
    Compile the same bounded fixture set through both: the primary
    self-hosted (stage2/stage3) compiler, and the new independent emitter.
    Require behavioral agreement (same evaluated result) on every fixture; a
    divergence would indicate a defect unique to one lineage.

Step 3 — widen coverage, keep the honest boundary explicit
    Same scoping discipline clj-meta already settled on: record exactly how
    much of the conformance surface the independent backend covers, keep
    claiming only that scoped subset as "independently cross-validated," and
    leave "full Wheeler DDC across the whole language" held until coverage
    actually matches.
```

Node.js, the Google Closure runtime, and `cljs.core` itself would remain shared
trust-root substrate even after this closes — DDC at the compiler level does
not touch that lower layer, and no claim should be made that it does.

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
