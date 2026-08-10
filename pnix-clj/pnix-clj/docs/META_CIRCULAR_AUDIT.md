# Meta-Circular Capability Audit — pnix-clj / clj-meta

Honest, evidence-based scorecard of the repo against the owner's 58-capability
"Pure Meta-Circular Capability Checklist" (2026-07-04). Verified against actual
code (`file:symbol`), not memory. No overclaiming — the constitution's rule
(history ≠ truth; verify before claiming).

## ✅ UPDATE 2026-07-04 — the evidence-store SPINE is now REBUILT on feat

Following the research-verified plan (docs/SPINE_ROADMAP.md, option C =
rebuild fresh in the clean-rewrite style), all 8 spine gap capabilities are now
LANDED on feat, each gate-pinned, in dependency order:

| § | capability | module | status |
|---|---|---|---|
| §3 | content-addressed TERM store; **α-canonical** (de Bruijn + correct shadowing); hash = propose filter, confirmed exactly | `cas.clj` | ✅ (incl. §3b) |
| §5 | append-only tamper-evident EVENT log (verifying trace + hermeticity guard) | `store.clj` | ✅ |
| §10/§13.1 | Clojure/JVM reflection snapshots (deterministic, pure EDN) | `reflect.clj` | ✅ |
| §8 | snapshot runtime pin + fail-closed match gate | `snapshot.clj` | ✅ |
| §9 | purity/determinism as EVENTS (witnessed by re-run, first-divergent anchor) | `purity.clj` | ✅ |
| §17 | content-address + event + structural-similarity search (+ §3c open-term summary) | `search.clj` | ✅ |
| §6.6-6.7 | mirror chain convergence + drift events | `mirror_chain.clj` | ✅ |
| §15 | witness schema + admission lattice (CAPSTONE) | `witness.clj` | ✅ |
| **integration** | **witnessed-run** — one run ties the spine to the pillars (term-keyed, snapshot-pinned, tower+chain+determinism as one §5 log, residual content-addressed, §15-admitted) | `witnessed_run.clj` | ✅ |
| **§14.3** | **self-modification gate** — the constitution's NO-AUTO-PROMOTION as a runtime gate (admitted witness HELD until owner authorizes) | `self_mod_gate.clj` | ✅ |
| **durability** | **persist** — content-addressed on-disk backing for §3 terms + §5 events, reverified on load (Unison/Nix-store shape) | `persist.clj` | ✅ |

The spine is now LIVE in the run path (witnessed-run), gated for self-* (no
auto-promotion), and durable (persist). Each of the three runner lanes
(pnix / pnix-clj-clj / clj-meta) has a first-class nREPL server, and clj-meta's
nREPL routes eval through its own bytecode backend.

Open follow-up (registered in roadmap.edn): §11 pnix-macro/reader lane (low fit
— Nix has no Lisp-style macros; clj-meta already has macroexpand).
The section-by-section scorecard below reflects the PRE-spine state; treat the
spine rows (§3/5/8/9/10/13.1/17/6.6-6.7/15) + §14.3 as now ✅.

## ⚠ Two design lines (read this first)

The pnix-clj repo has **two divergent branches** and the checklist's capabilities
are split across them:

- **`origin/main`** — carries the checklist's **evidence-infrastructure spine**:
  `cas.clj`, `store.clj`, `term.clj`, `stage.clj`, `purity.clj`, `stm.clj`,
  `resolve.clj`, `evidence.clj`, `mirror_journal.clj`, `verifier.clj`,
  `dirty.clj`, `search.clj`. → checklist §3, §5, §7(store-tower), §8, §9, §15,
  §17.
- **`feat/clj-meta-metacircular`** (the working branch) — a **clean rewrite** that
  emphasizes the metacircular **capability pillars**: a 4-substrate self-hosting
  COLLAPSE tower, Futamura 1st+2nd projection, measured Jones-optimality,
  safe-eval sandbox, content-addressed eval cache, capabilities drift-gate,
  synthesize projection, form-analysis, property fuzzer, arith/bool PROVEN
  equivalence, interop (value bridge + witness + effect + capability gate).

So the honest one-line verdict: **the capability PILLARS are strong on feat (and
in the projection/proof direction actually go BEYOND the checklist); the
checklist's evidence-STORE spine (§3/5/8/9/17) is NOT on feat — it lives on
`origin/main`.** Bringing the two together is an owner decision (see bottom).

## Scorecard (feat branch unless noted)

Legend: ✅ present · 🟡 partial/different-shape · ⬜ absent-on-feat (→ main =
exists on origin/main) · ➕ beyond-checklist

| § | capability | status | evidence |
|---|---|---|---|
| 1 | clj-meta stage3 host floor / host-proof separation | 🟡 | `bin/pnix-clj-gate`; clj-meta = `../clj-meta` (host proof), pnix-clj = runtime. No launcher stage3-jar refusal. |
| 1.3 | content-bound runtime/compiler/evaluator versions | 🟡 | `version.clj`, clj-meta compile-receipt determinism; not a full content-bound-version lattice. |
| 1.4 | class artifact hash | ✅ | `classfile_receipt.clj`, clj-meta `bytecode_witness`/`jarproof`. |
| 2 | tokenizer / parser / parse-error reification | ✅ | `parser.clj` (tokenize, parse-*), `error.clj` (pnix-error, spans). |
| 3 | pure pnix AST / canonical form / content-addr term hash | ⬜→main | AST is pure data; `hash.clj` data-hash. But NO `normalize-term`/`canonical-form`/term-store on feat → `cas.clj`/`term.clj` on main. |
| 3.1 | mutable runtime object guard | ⬜→main | no CAS guard on feat (`cas.clj` on main). |
| 3.4 | open-term structural summary / alpha | ⬜ | not built. |
| 4 | eval-source / eval-from-ast / apply layer | ✅ | `evaluator.clj` (eval-ast, apply-callable), `core.clj` (run-source, eval-source). |
| 4.5 | runtime mirror mode | ✅ | `mirror.clj` run-mirror, `px_runtime.clj` runMirror receipt. |
| 5 | append-only event store / event hash / index / pointer | ⬜→main | NO event log on feat → `store.clj`/`evidence.clj` on main. Evidence is receipt-shaped (`receipt.clj`) not an append-only log. |
| 6 | single mirror law / host+inner mirror / trace | ✅ | `mirror.clj`, cross-mirror-verdict; run-source is the one entrypoint. |
| 6.6-6.7 | mirror convergence / drift / chain stability | 🟡 | cross-mirror agree/reject per run; NO drift-event or repeated-run chain-convergence log. |
| 7 | stage tower (stage1..7 store-backed) | 🟡 | `tower.clj` is a **4-substrate COLLAPSE tower** (read→emit→direct→specialize→lowering→clj-meta→px→mirror), NOT the stage1-7 store/snapshot tower → `stage.clj` on main. `stage7_core.clj`, `stage15*.clj` exist differently. |
| 8 | snapshot version pin / runtime-match gate / resolve | ⬜→main | not on feat → `resolve.clj` on main. |
| 9 | purity / determinism / mutation isolation / threaded stress | 🟡→main | `determinism.clj` (repeat-eval hash stability) present; full purity-event/mutation-isolation → `purity.clj` on main. |
| 10 | namespace / Var / metadata / dynamic-binding reflection | ⬜ | not built on either lane (clj-meta has `host_reflection.clj`, partial). |
| 11 | macroexpand trace / pnix macro / reader form control / hygiene | 🟡 | clj-meta `compiler.clj` uses tools.analyzer macroexpand; `clojure_projection.clj`. No pnix-macro layer, no reader-form control lane. |
| 12 | requiring-resolve witness / namespace load gate | 🟡 | `interop.clj` host-eval-form + capability gate; no explicit require-witness event. |
| 13.1 | classpath / JVM version snapshot | ⬜ | not built on feat (clj-meta `jarproof` has jar hashing). |
| 13.3 | pnix value ↔ Clojure value bridge / roundtrip | ✅ | `interop.clj` `to-host`/`from-host`, `value_roundtrip.clj`, `value-loss` markers. |
| 13.4-13.5 | Java opaque ref / IFn boundary / gate | ✅ | `interop.clj` make-opaque-host-ref, opaque-ref-deref, host-object?, gated crossings. |
| 14.1 | effect classification | ✅ | `interop.clj` effect-class?, `safe_eval.clj` impure-builtins static classification. |
| 14.2 | capability gate | ✅ | `interop.clj` check-capability, `safe_eval.clj` pure-only gate. |
| 14.3 | self-modification gate | ⬜ | not built (deferred by checklist §6 too). |
| 15 | explicit witness schema / event-witness / admission lattice | 🟡 | `interop.clj` make-witness/crossing-witness, `receipt.clj` lane-summary/verdict, accepted/held/rejected statuses everywhere. Not the full :witness/* event schema. |
| 16 | roundtrip checks (source/store/stage/value/form) | ✅ | `emit_form_roundtrip.clj`, `value_roundtrip.clj`, `unparse.clj` roundtrip; store/stage roundtrips → main. |
| 17 | content-address / event / structural search | ⬜→main | `search.clj` on main; not on feat. |
| 18 | source/AST cache / compile artifact cache / correctness | ✅ | `cached_eval.clj` (content-addressed EVAL cache), lowering cache, `specialize` cache. |
| 19 | debug / explain reports | 🟡 | every capability has a `*/report` + `report-artifact`; no `explain-*` narrative fns or `proof/*.edn` dir on feat. |
| 20 | file layout (src/pnix_clj/meta/, bin/, proof/) | ⬜ | no `meta/` subdir, only `bin/pnix-clj-gate`, no `proof/` dir. |

### ➕ Beyond the checklist (feat adds, not in the 58)

| capability | evidence |
|---|---|
| Futamura **2nd projection** (generating extension, cogen-free) | `futamura.clj` |
| **measured** Jones-optimality witness | `futamura.clj` jones-optimality-witness |
| **PROVEN** equivalence — arithmetic (polynomial) + boolean (truth table) | `arith_proof.clj`, `bool_proof.clj` |
| property-based **differential fuzzer** with shrinking (found+fixed 2 bugs) | `property_fuzzer.clj` |
| capability index + **drift gate** + machine wiki registry | `capabilities.clj`, `wiki.clj`, `roadmap.edn` |
| reverse projection Clojure→pnix + analyzer cross-check | `synthesize.clj` × `form_analysis.clj` |

## Honest bottom line

Against the 58-item checklist, feat is roughly: **~24 present/✅, ~10 partial/🟡,
~9 absent-on-feat/⬜ (5 of those exist on `origin/main`), + 6 beyond-checklist**.

The metacircular **capability pillars** (projection, self-hosting collapse,
proofs, cross-check, interop) are being built well and honestly (gate-pinned,
receipt-carrying). The checklist's **evidence-store spine** (canonical CAS term
store §3, append-only event log §5, snapshot/resolve §8, purity-as-events §9,
search §17) is the real gap on feat — and it is exactly what `origin/main`
already implements.

## → Owner decision (boundary discipline — not auto-promoted)

This is a genuine owner call, not something to silently pick:

**(A) Port the `origin/main` evidence-spine (cas/store/term/stage/purity/resolve/
evidence/search) onto feat**, unifying the two lines — feat gains the full
checklist substrate.

**(B) Keep feat as the pillar-focused line** and treat the store-spine as a
separate concern on main.

**(C) Rebuild a minimal spine on feat** (canonical term hash + append-only event
log + snapshot pin + purity events) fresh, in the feat rewrite's style.

Gaps are registered in `resources/pnix_clj/roadmap.edn` (§ items as `:planned`)
so none are lost regardless of the decision.
