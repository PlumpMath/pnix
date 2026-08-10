# pnix-clj lane registry (generated — do not edit)

Regenerate with `clojure -M:lane-registry`.

This document is generated from top-level `src/pnix_clj/*.clj` `lane-classification` maps. It records the constitutional boundary between core runtime substrate, proof-only evidence lanes, and experimental self-change/dev lanes.

## Counts

- `:core`: 44
- `:experimental`: 7
- `:proof-only`: 28

## Registry

| lane | namespace | scope | product-runtime | semantic-authority | admission | allowed-output |
| --- | --- | --- | --- | --- | --- | --- |
| core | pnix-clj.cached-eval | content-addressed-eval-cache | allowed | forbidden | forbidden | cached-or-fresh-eval-result |
| core | pnix-clj.capabilities | capability-index-generator | allowed | index-only | drift-check-gated | capability-index-or-drift-verdict |
| core | pnix-clj.cas | content-addressed-term-identity | allowed | identity-filter-not-proof | structural-confirmation-required | term-key-or-structural-equivalence-verdict |
| core | pnix-clj.classfile-receipt | classfile-artifact-receipt | allowed | artifact-evidence-only | verified-compile-required | deterministic-classfile-report |
| core | pnix-clj.clj-meta | clj-meta-host-proof-interop | allowed | requires-receipt | gated-by-compile-receipt | host-eval-result-with-compile-receipt |
| core | pnix-clj.clj-meta-executor | clj-meta-host-execution | allowed | pnix-meta-px |  | runtime-result |
| core | pnix-clj.convenience | explicit-filesystem-convenience-boundary | allowed | pnix-meta-px | none | runtime-result |
| core | pnix-clj.core | public-runtime-orchestration | allowed | coordinates-core-lanes | none-on-basic-path | runtime-result |
| core | pnix-clj.determinism | evaluation-determinism-audit | allowed | evidence-only | fail-closed | determinism-report |
| core | pnix-clj.error | structured-error-envelope | allowed | error-shape-only | forbidden | pnix-failure-or-policy-hold-envelope |
| core | pnix-clj.evaluator | semantic-evaluator | allowed | core-evaluator | receipt-gated-upstream | eval-result-or-held-frontier |
| core | pnix-clj.form-analysis | host-form-static-analysis-boundary | allowed | static-classification-only | forbidden | form-analysis-host-surface-report |
| core | pnix-clj.hash | deterministic-hash-primitive | allowed | forbidden | forbidden | stable-content-digest |
| core | pnix-clj.interop | meta-circular-runtime-interop-boundary | allowed | capability-gate-only | capability-checked | interop-result-with-crossing-witness |
| core | pnix-clj.io-probe | read-only-host-io-probe-adapter | allowed | loads-pnix-meta-owns-no-semantics | tri-meta-io-gate | canonical-effect-receipt |
| core | pnix-clj.json | json-value-codec | allowed | codec-only | forbidden | json-value-or-json-string |
| core | pnix-clj.lane-registry | lane-classification-registry | allowed | registry-only | drift-check-gated | lane-registry-document-or-drift-verdict |
| core | pnix-clj.lowering | ast-to-host-form-lowering | allowed | requires-clj-meta-receipt | forbidden | lowered-host-form-or-held-frontier |
| core | pnix-clj.machine-outcome | host-machine-outcome-adapter | allowed | implements-pnix-meta-outcome-abi | tri-machine-outcome-gate | canonical-host-boundary-observation |
| core | pnix-clj.math | numeric-runtime-helper | allowed | helper-only | forbidden | numeric-result |
| core | pnix-clj.mirror | cross-runtime-mirror-evidence | allowed | cross-lane-evidence-only | mirror-verdict-only | mirror-row-or-cross-mirror-verdict |
| core | pnix-clj.mirror-chain | temporal-mirror-chain-evidence | allowed | evidence-only | fail-closed-on-drift | mirror-chain-report-or-drift-event |
| core | pnix-clj.nrepl | meta-circular-interactive-control-surface | allowed | eval-routes-through-core-only | forbidden | interactive-eval-session |
| core | pnix-clj.parser | source-to-ast-boundary | allowed | syntax-only | forbidden | parsed-ast-or-held-parse-result |
| core | pnix-clj.persist | durable-evidence-persistence-boundary | allowed | durable-evidence-only | forbidden | persistent-store-handle-or-integrity-report |
| core | pnix-clj.pnix-meta | external-common-px-loader-and-conformance | allowed | loads-pnix-meta-owns-no-semantics | conformance-verdict | loaded-value-or-conformance-report |
| core | pnix-clj.primitive-kernel | production-checked-i64-primitive-kernel | allowed | pnix-meta-manifest-only | forbidden | sealed-primitive-outcome |
| core | pnix-clj.purity | purity-determinism-event-spine | allowed | evidence-only | fail-closed | purity-event-or-violation-report |
| core | pnix-clj.px-runtime | px-runtime-artifact-boundary | allowed | runtime-artifact-evidence-only | forbidden | runtime-boundary-or-artifact-report |
| core | pnix-clj.receipt | multi-lane-receipt-verdict | allowed | verdict-from-evidence-only | all-lanes-agree | receipt-summary-or-verdict |
| core | pnix-clj.reflect | host-runtime-reflection-identity | allowed | host-identity-evidence-only | forbidden | reflection-snapshot-or-host-lane-id |
| core | pnix-clj.replay | durable-witness-replay-evidence | allowed | reproduction-evidence-only | forbidden | reproduced-diverged-or-missing-replay-verdict |
| core | pnix-clj.report-artifact | report-artifact-registry | allowed | report-packaging-only | forbidden | report-artifact-or-unknown-kind-verdict |
| core | pnix-clj.safe-eval | bounded-safe-evaluation-gate | allowed | bounded-gate-only | forbidden | safe-eval-result-or-limit-verdict |
| core | pnix-clj.search | evidence-spine-search-and-similarity | allowed | proposal-only | forbidden | search-result-or-similarity-candidate |
| core | pnix-clj.snapshot | runtime-snapshot-pinning | allowed | runtime-match-gate-only | fail-closed-on-mismatch | snapshot-or-runtime-match-verdict |
| core | pnix-clj.store | append-only-event-store | allowed | evidence-storage-only | forbidden | event-log-or-chain-verification-report |
| core | pnix-clj.strict-audit | strictness-audit-evidence | allowed | audit-only | forbidden | strict-audit-or-strict-gate-report |
| core | pnix-clj.trust | trust-and-provenance-risk-report | allowed | risk-report-only | default-held-on-uncertainty | trust-risk-report |
| core | pnix-clj.unparse | ast-to-source-rendering | allowed | rendering-only | roundtrip-gated-upstream | pnix-source-string |
| core | pnix-clj.version | runtime-version-metadata | allowed | metadata-only | forbidden | version-comparison-or-runtime-metadata |
| core | pnix-clj.wiki | self-documenting-capability-and-roadmap-substrate | allowed | documentation-index-only | drift-and-integrity-check-gated | wiki-document-or-drift-verdict |
| core | pnix-clj.witness | witness-admission-lattice | allowed | admission-lattice-only | explicit-lattice-transition | content-addressed-witness-or-admission-result |
| core | pnix-clj.witnessed-run | integrated-witnessed-run-spine | allowed | admission-evidence-pipeline | witness-lattice | witnessed-run-result-or-report |
| experimental | pnix-clj.benchmark | local-performance-measurement | forbidden | forbidden | forbidden | benchmark-report |
| experimental | pnix-clj.cegis | bounded-candidate-generation |  |  | forbidden | candidate-set |
| experimental | pnix-clj.generate | bounded-candidate-generation |  |  | forbidden | candidate-set |
| experimental | pnix-clj.repl | developer-repl-entrypoint | forbidden | forbidden | forbidden | rendered-repl-value-or-error |
| experimental | pnix-clj.self-improve | bounded-proof-experiment |  |  | owner-gated | held-review-queue |
| experimental | pnix-clj.self-mod-gate | gate-only |  |  |  | proposal-decision-event |
| experimental | pnix-clj.synthesize | bounded-candidate-synthesis |  |  | forbidden | pnix-candidate-source |
| proof-only | pnix-clj.arith-proof | arithmetic-fragment-equivalence-proof | forbidden |  | forbidden | arithmetic-proof-report |
| proof-only | pnix-clj.bool-proof | boolean-fragment-equivalence-proof | forbidden |  | forbidden | boolean-proof-report |
| proof-only | pnix-clj.clojure-form | clojure-form-fixture-corpus | forbidden | forbidden | forbidden | clojure-form-evidence-report |
| proof-only | pnix-clj.clojure-projection | clojure-projection-fixture-corpus | forbidden | forbidden | forbidden | clojure-projection-report |
| proof-only | pnix-clj.coverage | dynamic-evaluator-coverage-evidence | forbidden | forbidden | forbidden | coverage-report |
| proof-only | pnix-clj.emit-form-roundtrip | emit-form-roundtrip-evidence | forbidden | forbidden | forbidden | emit-form-roundtrip-report |
| proof-only | pnix-clj.forward-reference | forward-reference-fixture-corpus | forbidden | forbidden | forbidden | forward-reference-evidence-report |
| proof-only | pnix-clj.futamura | futamura-projection-evidence | forbidden | projection-evidence-only | forbidden | futamura-projection-report |
| proof-only | pnix-clj.grammar-fuzzer | bounded-grammar-fuzz-evidence | forbidden |  | forbidden | fuzz-evidence-report |
| proof-only | pnix-clj.import-module | import-module-fixture-corpus | forbidden | forbidden | forbidden | import-module-fixture-set |
| proof-only | pnix-clj.live-oracle | optional-external-reference-oracle | forbidden |  | forbidden | oracle-comparison-report |
| proof-only | pnix-clj.machine | derived-abstract-machine | forbidden | evaluator-shared-value-algebra-only | forbidden | whnf-or-realized-eval-result |
| proof-only | pnix-clj.mirror-error | negative-mirror-error-fixture-corpus | forbidden | forbidden | forbidden | mirror-error-alignment-report |
| proof-only | pnix-clj.mirror-pair | four-lane-cross-check-fixture-corpus | forbidden | forbidden | forbidden | mirror-pair-evidence-report |
| proof-only | pnix-clj.oracle | static-oracle-fixtures | forbidden |  | forbidden | oracle-fixture-set |
| proof-only | pnix-clj.property-fuzzer | bounded-property-fuzz-evidence | forbidden |  | forbidden | shrunk-counterexample-or-proof-report |
| proof-only | pnix-clj.runtime-plan | px-runtime-plan-printer | forbidden | forbidden | forbidden | runtime-plan-text-report |
| proof-only | pnix-clj.rust-batch | cross-implementation-invariance-corpus | forbidden |  | forbidden | equivalence-report |
| proof-only | pnix-clj.smoke | fast-smoke-evidence | forbidden | forbidden | forbidden | smoke-report |
| proof-only | pnix-clj.specialize | partial-evaluation-equivalence-proof | forbidden |  | forbidden | specialization-equivalence-report |
| proof-only | pnix-clj.stage15 | bounded-stage-tower-control | forbidden |  | forbidden | stage-control-plan-or-execution-report |
| proof-only | pnix-clj.stage15-execute | manual-stage15-execution-entrypoint | forbidden | forbidden | forbidden | stage15-execution-receipt |
| proof-only | pnix-clj.stage15-plan | stage15-control-plan-printer | forbidden | forbidden | forbidden | stage15-control-plan-text-report |
| proof-only | pnix-clj.stage7-core | stage7-core-lockin-regression | forbidden |  | forbidden | stage-closure-report |
| proof-only | pnix-clj.tower | meta-circular-tower-collapse-evidence | forbidden | tower-evidence-only | forbidden | tower-collapse-report |
| proof-only | pnix-clj.translation-validation | translation-validation | forbidden |  | forbidden | equivalence-verdict |
| proof-only | pnix-clj.value-roundtrip | value-roundtrip-evidence | forbidden | forbidden | forbidden | value-roundtrip-report |
| proof-only | pnix-clj.weval | ir-level-partial-evaluation-spike | forbidden |  | forbidden | weval-report |

## Registry hash

`00d0c387993fbe13abf395c3518acc44616fab2acfe28202e48d2c4ea9cdca2f`
