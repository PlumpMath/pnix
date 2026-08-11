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

The fixed-point proof (stage2 == stage3, byte-identical after >=15
self-recompiles) is *reproducibility* evidence, not Trusting-Trust defense — a
backdoor baked into stage0/stage1 would reproduce itself identically forever
and that check would still pass. No independently-authored third-party
ClojureScript compiler exists to lean on either: alternatives like shadow-cljs
still compile through the same official `cljs.core`/analyzer lineage the
fixed point depends on, so they would not catch a defect in that shared
lineage.

**Independent mini backend added this session (2026-08-11):**
`independent_mini_backend.js` is a new, from-scratch ClojureScript-subset-to-JS
emitter — its own tokenizer/reader plus direct JS source-text emission via
`new Function(...)`, sharing zero code with `cljs.js`/`cljs.compiler`/
`cljs.analyzer`. `new Function` and the JS engine itself remain trusted host
substrate, the same honest role the JVM classfile format plays for clj-meta's
`frontend_selfhost.clj` and Python's `ast`/`compile()` play for hy-meta's
`independent_mini_backend.py`.

Covers 8 fixtures (`let`, `if`, `+`/`-`/`*`, `<`/`>`/`<=`/`>=`/`=`, booleans,
keyword literals). Cross-validated against the real self-hosted compiler
(`dist/cljs-meta-module.js`'s `evaluate()`, the actual cljs.js-backed
production path) — both agree on all 8. Wired into
`test/independent_mini_backend_test.js`, run from both
`cljs-meta/bin/cljs-meta-gate` and the top-level `bin/pnix-cljs-gate`.
Verified live this session: `independent mini backend DDC: PASS (8
fixtures)`, full `pnix-cljs aggregate gate: PASS`, no regressions in
self-test, runtime matrix, fixed-point runtime, or identity gates.

**What this closes and what it still doesn't:** this is now a genuine 2-way
behavioral comparison (self-hosted cljs.js-backed compiler ≡ independent
from-scratch mini backend) on a small fixture set, not merely documented as a
plan. It is still only 8 fixtures, not the conformance surface, and — same
honest bar as clj-meta and hy-meta already settled on — behavior equivalence,
not bit-identical JS text, since two independently-authored emitters
targeting the same language by coincidence would not be expected to produce
identical source text. **Next concrete step:** widen the fixture set (`do`,
data literals — vectors/maps/keywords-as-values rather than just as return
values, string handling) toward parity with clj-meta's ~50-fixture
`frontend_selfhost.clj` scope.

Node.js, the Google Closure runtime, and `cljs.core` itself remain shared
trust-root substrate even with this closed — DDC at the compiler level does
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
