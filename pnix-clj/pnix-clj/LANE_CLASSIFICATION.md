# pnix-clj Lane Classification

This document classifies pnix-clj namespaces and feature surfaces after the scope lock.

`SCOPE_LOCK.md` is authoritative. This file explains how existing lanes should be treated.

## Classification labels

### CORE

Allowed in pnix-clj core gate.

These are part of the Clojure-hosted pnix meta-circular proof lane.

### PROOF-ONLY

Allowed only when the lane produces bounded proof/equivalence/witness evidence.

These must not become product behavior, autonomous action, NL routing, or coding-agent execution.

### EXPERIMENTAL

Allowed only as a bounded research/proof experiment.

These should remain gated, documented, and non-authoritative.

### QUARANTINE

Out of pnix-clj core.

These must not enter `src`, test gate, or core runtime unless split into a side repo or explicitly reclassified.

---

## CORE lanes

| Lane | Classification | Reason |
|---|---|---|
| parser | CORE | pnix source to AST |
| lowering | CORE | AST to canonical/lower form |
| core evaluator | CORE | pnix eval-source / eval-from-ast |
| px-runtime | CORE | runtime lane for pnix evaluation |
| CAS | CORE | content-addressed identity |
| store | CORE | append-only evidence / term storage |
| snapshot | CORE | deterministic pinned state |
| persist | CORE | durable replay support |
| mirror | CORE | runtime mirror evidence |
| mirror-chain | CORE | repeated mirror convergence |
| mirror-pair | CORE | equivalence comparison between mirror routes |
| mirror-error | CORE | structured mirror failure evidence |
| determinism | CORE | repeated-run stability |
| purity | CORE | effect and determinism discipline |
| replay | CORE | witness re-verification |
| witness | CORE | proof/evidence object surface |
| witnessed-run | CORE | run + witness binding |
| receipt | CORE | content-bound receipt |
| safe-eval | CORE | bounded eval surface |
| capabilities | CORE | effect/capability discipline |
| trust | CORE | trust boundary evidence |
| classfile-receipt | CORE | JVM/class artifact witness |
| version | CORE | runtime/compiler version binding |
| clj-meta host reflection | CORE | host-language proof lane |

---

## PROOF-ONLY lanes

| Lane | Classification | Rule |
|---|---|---|
| Futamura / specialize | PROOF-ONLY | allowed as projection/equivalence evidence only |
| translation-validation | PROOF-ONLY | allowed as equivalence validation only |
| stage7-core | PROOF-ONLY | allowed as staged closure proof |
| stage15 | PROOF-ONLY | allowed as bounded tower/self-hosting proof |
| oracle / live-oracle | PROOF-ONLY | allowed only as bounded comparison oracle |
| coverage | PROOF-ONLY | allowed only as proof surface coverage |
| grammar-fuzzer | PROOF-ONLY | allowed only as parser/runtime robustness evidence |
| property-fuzzer | PROOF-ONLY | allowed only as bounded property evidence |
| arith-proof | PROOF-ONLY | allowed only as arithmetic proof fixture |
| bool-proof | PROOF-ONLY | allowed only as boolean proof fixture |
| value-roundtrip | PROOF-ONLY | allowed only as value bridge evidence |
| emit-form-roundtrip | PROOF-ONLY | allowed only for Clojure form roundtrip evidence, not multi-language codegen |

---

## EXPERIMENTAL lanes

| Lane | Classification | Required restraint |
|---|---|---|
| synthesize | EXPERIMENTAL | bounded candidate generation only; no autonomous admission |
| generate | EXPERIMENTAL | bounded generation only; no NL/coding-agent expansion |
| self-improve | EXPERIMENTAL | must remain held/candidate/gated; no autonomous mutation |
| self-mod-gate | EXPERIMENTAL | gate only; no direct mutation admission |
| rust-batch | EXPERIMENTAL | only if it remains a proof/equivalence batch, not a Rust product lane |
| clojure-projection | EXPERIMENTAL | only as Clojure-host projection evidence |
| clojure-form | EXPERIMENTAL | only as host form analysis/roundtrip evidence |
| form-analysis | EXPERIMENTAL | only as Clojure form proof analysis |
| benchmark | EXPERIMENTAL | measurement only; not semantic authority |
| wiki | EXPERIMENTAL | documentation/index only; not runtime truth |

---

## QUARANTINE lanes

The following are explicitly outside pnix-clj core.

| Lane | Classification | Reason |
|---|---|---|
| Hangul codec | QUARANTINE | NL/meaning lane, not pnix meta-circular proof |
| MSV / meaning sentence variants | QUARANTINE | NL semantic generation lane |
| Korean dictionary | QUARANTINE | language knowledge lane |
| Korean mirror | QUARANTINE | NL mirror lane |
| domain-token / domain-generic matching | QUARANTINE | semantic routing/matching lane |
| graph-gate / gate-graph | QUARANTINE | agent graph/emit lane |
| multi-language emit registry | QUARANTINE | coding-agent/codegen lane |
| behavior-atom emit | QUARANTINE | coding-agent behavior surface |
| puck-cli bridge | QUARANTINE | external executor bridge |
| tick-runner | QUARANTINE | autonomous loop/scheduler |
| redb ingest brain | QUARANTINE | external knowledge/memory ingestion |
| NL corpus / meaning graph | QUARANTINE | natural-language semantic memory |
| answer composer | QUARANTINE | NL response generation lane |

---

## Rule for future work

Before adding a namespace, test, alias, or app runner, classify it here.

If a lane cannot be classified as CORE, PROOF-ONLY, or EXPERIMENTAL under the scope lock, it must not enter pnix-clj core.

When uncertain, classify as QUARANTINE.

---

## Current identity lock addendum

The generated source of truth is `docs/LANE_REGISTRY.md`.

Current top-level registry counts:

- CORE: 38
- EXPERIMENTAL: 6
- PROOF-ONLY: 26
- TOTAL: 70

The following surfaces are CORE identity surfaces:

- interop: Clojure runtime ↔ pnix runtime meta-circular crossing boundary
- nREPL: meta-circular interactive control surface; eval routes through core only
- wiki: self-documenting capability and roadmap substrate
- lane-registry: generated lane classification registry

`nrepl`, `wiki`, and `interop` are not disposable dev-only surfaces.

The following remain QUARANTINE and must not enter pnix-clj core:

- Hangul codec
- MSV / meaning sentence variants
- graph-gate / gate-graph
- multi-language emit registry
- puck-cli bridge
- tick runner
- redb ingest brain
