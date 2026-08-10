# pnix-clj / clj-meta Separation Plan

Updated: 2026-07-01 KST

This document reconciles the requested separation plan ("pure clj-meta
compiler/evaluator layer + pnix-clj runtime/interop layer") with the **actual
current branch**. Every "stays / moves / interop" assignment cites a real file on
this branch.

**Branch reality (READ FIRST).** This branch — `feat/clj-meta-metacircular` — is a
clean parallel rewrite: a Clojure/JVM-hosted pnix runtime on the clj-meta host
proof floor (parse -> evaluate -> lower -> clj-meta lane -> .px runtime lane ->
mirror/receipt). It diverged from `origin/main` at merge-base `5a8db4d8` and is
**0 behind / 407 ahead**. `origin/main` is a DIFFERENT, older design line
(content-addressed `cas.clj`, append-only `store.clj`, `stage.clj`, `purity.clj`,
`term.clj`/`stm.clj`/`resolve.clj`, gate-graph, 67-language emit, Korean modules).
Those main modules are **REFERENCE ASSETS (MAIN-ONLY)** — useful to port from when
a roadmap item calls for it, but they must NOT be judged as "already present" on
this branch, nor as "absent pillars this branch must fill". Classify this branch
only by its own files: BUILT / PARTIAL / TARGET / MOVED / HELD, and use MAIN-ONLY
for anything that lives on `origin/main` but not here. Do not pull main modules in
unless explicitly porting.

Read this alongside `todo.md` and its `## Completeness Roadmap`. This is the
architectural map; the roadmap is the feature backlog.

---

## 0. Core correction: meta-circular is not just "mirror"

The earlier framing treated meta-circular capability as something that only
exists when a *mirror* exists. That is too narrow. A mirror is **one observation
surface**. Meta-circular capability is the whole set:

```
reader · parser · form-as-data · AST-as-data · canonical form · content hash ·
eval/apply · macroexpand · namespace/Var/metadata reflection · stage bootstrap ·
compiled-class-artifact proof · roundtrip · drift detection · witness/proof ·
gate/capability · interop · self-hosting ladder
```

The mirror's current many-pieces shape (separate `mirror.clj`, `mirror_pair`,
`mirror_error`, `clojure_projection`, `clojure_form` lanes) was a design result,
not a requirement. Section 6 corrects it toward a singleton runtime mirror with
trace facets.

Three layers, one bridge:

```
clj-meta   = Clojure/JVM meta-circular compiler/evaluator PROOF lane (host floor)
pnix-clj   = pnix runtime hosted on clj-meta, plus pnix <-> Clojure/JVM interop
interop    = explicit bidirectional bridge at the boundary (not a mirror)
```

Layering & sequencing (the sharper model): **clj-meta is pnix-agnostic.** Its job
is to *complete* Clojure(JVM) meta-circularity on its own — self-hosting ladder
(stage1 -> 7 -> ... -> N), kernel, import hook, artifact reproduction, host
introspection (clj-meta's own todo). It knows nothing about pnix. **pnix-clj is
purely the pnix layer on top of that finished host, and its "host" *is*
clj-meta.** So pnix-clj must NOT re-do host proof: any "is host Clojure
faithful?" work is Clojure <-> Clojure and belongs to clj-meta's domain (reached
via interop), not to the pnix runtime core.

Consequence for this branch: `clojure_form.clj` (host Clojure `eval` vs clj-meta
compile agreement) and the host-reflection half of `clojure_projection.clj` (Var/
NS/Java/reflection snapshots) are **Clojure-about-Clojure host-domain** work, not
pnix-runtime core. pnix-clj's actual core is `parser` / `evaluator` / `lowering` /
`px_runtime` / `mirror` / `receipt` (the pnix language). The host-domain pieces
move behind interop (Phases B/C) and conceptually belong to clj-meta's lane.

---

## 1. Current reality (verified — read before refactoring)

Git root is `~/pnix-clj`. Siblings:

```
clj-meta/                 mature Clojure->JVM-bytecode meta-circular compiler
pnix-clj/                 this project (the pnix runtime)
clojure-clojure-1.12.5/   host Clojure source corpus
pnix-mirror-runtime/ pnixc-pnix/ stdlib/ corpus/ docs/ ingest/ scripts/
```

- `pnix-clj/deps.edn`: `pnix/clj-meta {:local/root "../clj-meta"}` — clj-meta is
  the one declared backend dependency.
- **clj-meta already owns the host proof lane.** It is not a stub. Its `src/pnix/
  clj_meta/` includes: `compiler.clj` (`compile-form`, `compile-form*`,
  `eval-form`, `compile-form-strict`, `compile-ns`, `load-compiled-ns`,
  `compile-classes`, `compile-to-dir`), `verified_compile.clj`
  (`compile-classes-verified`), `bytecode_verifier.clj`, `bytecode_witness.clj`,
  `determinism_policy.clj`, `translation_validation.clj`, `conformance.clj`,
  `fuzz_conformance.clj`, `kernel.clj`, `selfhost.clj`, `runtime_selfhost.clj`,
  `frontend_selfhost.clj`, `crosshost.clj`, `cross_host_ddc.clj`, `mirror.clj`,
  `stm.clj`, `gate.clj`, `language_surface.clj`, and more.
  > Consequence: **"move to clj-meta" almost always means "consume clj-meta's
  > existing API" or "relocate a thin host-reflection helper behind a clj-meta
  > interop API" — not build something new.** Do not reinvent what clj-meta has.
- **Host machinery in pnix-clj is confined to exactly three files** (verified by
  grepping `requiring-resolve` / host `(eval ...)` / reflection across
  `src/pnix_clj/*.clj`): `clj_meta.clj`, `clojure_form.clj`,
  `clojure_projection.clj`. Everything else is pnix-native.
- **Not on this branch** (these are MAIN-ONLY reference assets — they live on
  `origin/main`'s different design line, not here, so do not treat them as either
  "already present" or "absent pillars to fill" on this branch): `cas.clj`
  (content-addressing: `normalize-term`/`canonical-form`/`term-hash`/`term-key`),
  `store.clj` (append-only event log), `term.clj`, `stm.clj`, `resolve.clj`,
  `purity.clj`, `stage.clj`, `evidence.clj`, `verifier.clj`, `search.clj`, plus a
  `README`. This branch is the parse -> evaluate -> lower -> (clj-meta lane) ->
  (.px runtime lane) -> mirror/receipt design, with the cross-lane
  `receipt/verdict` as the acceptance gate. If a roadmap item later needs
  content-addressed terms or an event store, PORT from `origin/main`'s `cas.clj` /
  `store.clj` (explicit branch comparison) rather than inventing from scratch.

---

## 2. Per-file ownership map (the heart, grounded in real files)

Legend: **PNIX** = pnix-runtime, stays; **HOST** = Clojure/JVM host machinery,
belongs to clj-meta (move or consume); **INTEROP** = the boundary bridge;
**CHECK** = a report/verification harness over the runtime, not a separate
runtime mirror.

| file | bucket | notes |
|---|---|---|
| `parser.clj` | PNIX | pnix tokenizer/parser/AST; the pnix language surface. |
| `evaluator.clj` | PNIX | pnix semantics, value model, builtins, env (now lazy `let` + call-by-need args + `with`/`assert`/`a.b or d`/`@`-patterns). |
| `lowering.clj` | PNIX (output crosses INTEROP) | the pnix->Clojure lowering *policy* is pnix-clj's; the *compile/eval* of the emitted forms is clj-meta's. Keep here; treat the emitted form as crossing the boundary. |
| `clj_meta.clj` | **INTEROP** | already the seam to clj-meta's compiler. Formalize as the clj-meta interop client (Section 5). It currently *re-derives* determinism/strict/bytecode/verified evidence that clj-meta already owns — delegate instead (Section 3.4). |
| `clojure_form.clj` | **HOST-DOMAIN (Clojure<->Clojure)** | NOT pnix: it checks host Clojure `eval` vs clj-meta compile agreement — a Clojure self-host proof that conceptually belongs to clj-meta. Host eval already routed through `interop`; the agreement is a host CHECK, not pnix "projection". |
| `clojure_projection.clj` | **HOST reflection (Clojure<->Clojure)** + PNIX term-mapping | the largest misplacement: raw Clojure/JVM introspection (host domain). Split: host snapshots move behind interop; only the pnix-term mapping (`project-reader-value`) + `.px` validation (`validate-term`) stay in pnix-clj (Section 3.1). |
| `px_runtime.clj` | PNIX | internal `.px` runtime: pnix-in-pnix parse/eval, import graph/cache/cycle. |
| `mirror.clj` | PNIX | runtime mirror rows + cross-mirror verdict; consolidate to singleton (Section 6). |
| `receipt.clj` | PNIX | verdict / lane-summary / summarize (the acceptance gate). |
| `core.clj` | PNIX | `run-source` / `report` orchestration; the natural home of the singleton `run-mirror`. |
| `error.clj` | PNIX | pnix structured error envelope (`:pnix-clj.error.v0`). |
| `version.clj` `math.clj` `json.clj` | PNIX | pnix builtin helpers. |
| `oracle.clj` `rust_batch.clj` `stage7_core.clj` | PNIX | pnix corpus/fixtures. |
| `mirror_pair.clj` `mirror_error.clj` | CHECK | report harnesses over the runtime; reframe as check categories, not separate mirrors. |
| `report_artifact.clj` `runtime_plan.clj` `smoke.clj` `benchmark.clj` | PNIX tooling | stay. |
| `stage15.clj` `stage15_plan.clj` | **HOST stage control** | a clj-meta backend gate plan, currently a NOT-executed control plan (`:stage15-gates-not-executed`). Conceptually clj-meta's gate/stage lane; pnix-clj should *consume* a clj-meta-provided stage proof, not own the plan. |

---

## 3. What moves to (or is delegated to) clj-meta

All of this lives in the three host-touching files today.

### 3.1 Host reflection/introspection in `clojure_projection.clj` (the big one)

These functions are pure Clojure/JVM host introspection and belong to the host
proof/interop lane, surfaced to pnix-clj as **host snapshots through an interop
API** (clj-meta can host them; minimally they move behind a clearly-marked
`pnix-clj.interop` host-side namespace):

```
project-var-value · project-namespace-value · project-throwable-value ·
class-term · java-object-term · macroexpand-source-term ·
dynamic-binding-source-term · java-interop-source-term · reflection-source-term ·
classloader-source-term · namespace-resolution-source-term ·
host-object-construction-source-term · polymorphism-source-term ·
metadata-source-term · state-effect-source-term · lazy-evaluation-source-term ·
concurrency-source-term · coordination-source-term · control-flow-source-term
```

What **stays** in pnix-clj from this file: `project-reader-value` (host value ->
pnix projection term mapping) and `validate-term` / `projection-runtime` (validate
a pnix term through the internal `.px` projection artifact). That is the
pnix-side of the bridge.

Enforce the **opaque-ref rule** here: `java-object-term` currently embeds a
`JavaObject` envelope directly. JVM/Clojure objects must not enter pnix canonical
terms as themselves; they cross as opaque refs unless explicitly converted to
pure pnix values (Section 5).

### 3.2 Host `eval` / `macroexpand` / fresh-ns in `clojure_form.clj`

The `(eval form)` in a freshly created namespace is host-eval machinery. Route it
through the clj-meta host-eval interop API (clj-meta `compiler/eval-form` already
exists; add a host-oracle eval if a true host-Clojure oracle is wanted distinct
from clj-meta). Keep the **host-vs-clj-meta agreement** as a CHECK in pnix-clj.

### 3.3 stage15 control plan

`stage15.clj` / `stage15_plan.clj` describe clj-meta backend gate commands and
are not executed. This is a clj-meta gate/stage concern; pnix-clj should consume
a clj-meta-provided, *executed* stage proof rather than carry the plan. (See the
roadmap Axis-3 item "execute stage15 rather than plan it".)

### 3.4 Compile-proof re-derivation in `clj_meta.clj`

`clj_meta.clj` rebuilds determinism / strict / bytecode-artifact / verified-
compile evidence around clj-meta's `compile-form*`. clj-meta already owns
`determinism_policy`, `verified_compile`, `bytecode_witness`, `bytecode_verifier`.
Delegate to those instead of re-deriving, so the host proof has a single owner.

---

## 4. What stays in pnix-clj (pnix-runtime meta-circular)

Because pnix-clj *is* the pnix runtime, it owns the pnix-native meta-circular
surface:

- pnix tokenizer/parser/AST (`parser.clj`).
- pnix evaluator / apply / value model / builtins / env (`evaluator.clj`),
  including the laziness work (memoized-thunk `let`, call-by-need args) and the
  grammar (`with`, `assert`, `a.b or d`, `@`-patterns).
- pnix lowering *policy* (`lowering.clj`) — the pnix->Clojure mapping; its output
  crosses the interop boundary into clj-meta.
- internal `.px` runtime (`px_runtime.clj`) — pnix-in-pnix evaluator + import
  graph/cache/cycle. This is itself a pnix-side meta-circular artifact.
- pnix runtime mirror (`mirror.clj`, to become singleton), receipt/verdict
  (`receipt.clj`), orchestration (`core.clj`).
- pnix error model (`error.clj`), pnix helpers (`version`/`math`/`json`), pnix
  corpus/fixtures/reports.

TARGET (future, do not claim as present): a pnix CAS / canonical-term store /
event log / snapshot-resolve / stage tower. If adopted, these are pnix-runtime
and stay in pnix-clj — but they do not exist today.

---

## 5. The interop boundary (Clojure/JVM <-> pnix)

**Interop is not mirror.** Interop converts values/functions/modules/effects and
must work even with mirror disabled. Mirror may *observe* interop; it does not
define it.

- Host side (clj-meta): object inspection, IFn invocation, namespace load, Var
  resolution, macroexpand, exception capture, JVM reflection, classpath/
  classloader control.
- pnix side (pnix-clj): pnix value/function/module/error representation, opaque
  host refs, the interop call form, the interop witness.

Shared protocol fields (attach to every crossing):

```
interop/id · direction · source-language · target-language ·
input-kind · output-kind ·
loss-status      = lossless | lossy | opaque | effectful | unsupported | dangerous
effect-class     = pure | host-call | require | resolve-var | file-read |
                   file-write | thread/future | time | random | process |
                   network | global-mutation | namespace-mutation |
                   var-mutation | unknown
capability-required · host-object-policy · witness-id
```

Value mapping (pnix <-> Clojure/JVM):

```
null<->nil  bool<->Boolean  int<->integer  float<->floating  string<->String
list/vector<->vector  attrset<->map  symbol<->symbol  keyword<->keyword
function<->IFn wrapper  module<->namespace/module wrapper  error<->ExceptionInfo
opaque JVM object<->pnix opaque ref
```

**Rule:** Clojure/JVM objects must not enter pnix canonical terms directly — wrap
as opaque refs unless explicitly converted to pure pnix values. (Directly fixes
the `java-object-term` embedding noted in 3.1.)

pnix-clj already has the seeds of this: `clj_meta.clj` (host compile/eval seam)
and `error.clj` (structured envelope). The work is to make the protocol explicit
and bidirectional, not to invent it from zero.

---

## 6. Mirror correction: one runtime mirror, many trace facets

Current (fragmented): `mirror.clj` rows are assembled in `core/run-source`, while
`mirror_pair`, `mirror_error`, `clojure_projection`, `clojure_form` are separate
report lanes, and clj-meta has its own `mirror.clj`. No single canonical runtime
mirror route, result hash, or trace id.

Target:

```
pnix-clj.mirror/run-mirror(source, opts)
  parse -> (term) -> (resolve) -> eval -> record trace facets ->
  one result hash, one trace id, one witness
```

Allowed trace facets (NOT separate mirrors):

```
:host/parse :host/term :host/resolve :inner/eval-step :inner/value
:inner/effect :inner/error :interop/call :witness/event
```

clj-meta keeps multiple host **CHECK categories** (compiler / macroexpand /
namespace / Var / class-artifact / host-eval checks) — that is correct; they are
host proof checks organized as categories, not competing pnix runtime mirrors.

Why singleton: one canonical route, one result hash, one trace id, one
convergence target; less duplicate parse/eval; better performance, analysis,
debugging; less drift.

---

## 7. Current vs target (honesty ledger)

| concept | status |
|---|---|
| clj-meta as host proof lane | **present** (mature) |
| pnix parser/evaluator/lowering/.px-runtime/mirror/receipt | **present** |
| host machinery isolated to 3 files | **present** (the misplacement to fix) |
| explicit interop protocol (loss/effect/capability/witness) | **target** (seeds in `clj_meta.clj`/`error.clj`) |
| singleton `run-mirror` | **target** (today: `mirror.clj` + report lanes) |
| opaque-ref discipline for JVM objects | **target** (today: embedded envelopes) |
| CAS / event store / term store / snapshot-resolve | **MAIN-ONLY** (origin/main `cas.clj`/`store.clj`/`term.clj`/`resolve.clj`; not on this branch — port if a roadmap item needs it) |
| README | **MAIN-ONLY** (origin/main has one; this branch does not) |

The current acceptance discipline is the cross-lane `receipt/verdict`
(evaluator / clj-meta / `.px` runtime / pnix-mirror) — an N-version **heuristic**
differential check, not a formal proof (see the roadmap's framing invariant).

---

## 8. Phased refactor (incremental, gate-green per step)

- **Phase A — formalize the interop seam.** Make `clj_meta.clj` the explicit
  clj-meta interop client; attach an interop receipt carrying
  loss/effect/capability/witness. Low risk (rename + metadata).
- **Phase B — extract host reflection from `clojure_projection.clj`.** Move the
  host-snapshot functions (3.1) behind a host-side interop API; leave pnix-clj
  owning `project-reader-value` + `validate-term`. Enforce the opaque-ref rule.
- **Phase C — route host eval/macroexpand** from `clojure_form.clj` through the
  clj-meta interop API; keep the agreement as a CHECK.
- **Phase D — consolidate the runtime mirror** into a singleton `run-mirror` with
  trace facets; reframe `mirror_pair`/`mirror_error`/`clojure_projection`/
  `clojure_form` as CHECK categories over the one mirror.
- **Phase E — delegate compile proof** (determinism/verified/bytecode) in
  `clj_meta.clj` to clj-meta's `determinism_policy`/`verified_compile`/
  `bytecode_witness` instead of re-deriving.
- **Phase F (only if a roadmap item needs it)** — PORT content-addressed terms /
  event store / snapshot-resolve from `origin/main` (`cas.clj`/`store.clj`/
  `term.clj`/`resolve.clj`) as an explicit branch comparison, adapting to this
  branch's value model. Not a standing scope; pulled in per concrete need.

Each phase keeps `bin/pnix-clj-gate`, `clojure -M:test`, and the report lanes
green, and is committed/pushed as its own slice.

---

## 9. Final architecture (corrected statement)

```
clj-meta = Clojure/JVM meta-circular compiler/evaluator PROOF lane
  owns: Clojure forms, macroexpand, eval/compile oracle, namespace/Var/metadata
  reflection, JVM/classpath/class artifacts, dynamic loading, host introspection,
  host-side interop, host witnesses and gates. (Already mature; consume it.)

pnix-clj = pnix runtime on top of clj-meta
  owns: pnix tokenizer/parser/AST/eval/value/builtins/env, lowering policy, the
  internal .px runtime, the pnix mirror, receipt/verdict, pnix error model,
  corpus/reports. (TARGET additions: CAS/term-store/stage tower.)

interop = explicit bidirectional bridge
  Clojure/JVM host objects and pnix values/functions/modules convert only through
  loss-marked, effect-classified, capability-checked adapters. JVM objects cross
  as opaque refs unless converted to pure pnix values.

mirror = not the source of meta-circularity
  one observation/proof entrypoint on the pnix runtime side; many trace facets,
  one canonical run. clj-meta keeps separate host CHECK categories.
```

Core principle: **do not make pnix-clj a pile of fragmented mirrors.** Make
clj-meta the Clojure/JVM host meta-circular proof layer, make pnix-clj the pnix
runtime layer, make interop explicit, keep the pnix runtime mirror singleton, and
make every conversion, effect, replay, drift, and stage result produce a witness.

---

## 10. Research-grounded interop boundary + capability distribution (2026-07-01)

A `/deep-research` pass (94 agents, adversarially verified) on hosting a guest
language on a host language confirms the layer split and sharpens the interop
boundary. Cited reference systems: **GraalVM Truffle** (deny-by-default
`@HostAccess.Export` allowlist; orthogonal per-effect switches: host-access /
reflection (`allowHostClassLookup`) / native / IO), **object-capability theory**
(unforgeable handles bundling designation+authority; "only connectivity begets
connectivity"; least authority), **static/algebraic effect systems** (CallE
`restrict[ε]`; Wyvern lifts a single `system.FFI` into domain effects),
**opaque-handle FFI** (Kernel-FFI stores the host object under a UUID and passes
only a reference), and **content-addressed code** (Unison: hash-of-normalized-AST
identity, names kept as separate metadata).

### Interop boundary design principles (host <-> pnix)
1. **Deny-by-default.** Nothing from the host floor is reachable from the pnix
   runtime until explicitly exported. The host (clj-meta side) owns the allowlist;
   pnix-clj receives only granted capabilities and never reaches host machinery
   ambiently. Build the boundary as an allowlist, add one capability at a time.
2. **Classify every crossing by effect class** (pure / host-call / reflection /
   require / file / network / mutation / time / random / thread). The interop
   layer is the single place that tags and gates them; the pnix core sees only
   already-classified, already-gated capabilities.
3. **Opaque handles, never value-serialization.** A host (Clojure/JVM) object
   crosses as an opaque ref (designation + authority), NOT serialized into a pnix
   value, and MUST NOT enter a pnix canonical / content-addressed term. (Directly
   fixes today's `java-object-term`, which embeds a host-object envelope.)
4. **Object-capability discipline.** Authority travels only by passing a handle;
   no ambient/global naming confers it; least authority per crossing.
5. **Content-addressed cross-layer trust.** Bind the host floor to guest evidence
   via a content-addressed host version id; identify pnix terms by hash of the
   *normalized* AST, with human names kept as separate metadata (Unison model).

### Honest caveats (do not overclaim)
- Cross-layer agreement (the receipt/verdict N-version check) is **heuristic, not
  sound** — a host "floor proof" does not hand pnix its semantics or soundness.
  Confirmed by the research: **a lazy Nix-like guest on an eager Clojure host MUST
  implement laziness as explicit guest-layer thunks** — which is exactly the lazy
  `let` + call-by-need-args work already landed.
- Effect-system soundness rests on **honest foreign annotations**; controlling
  which references cross is necessary but NOT sufficient — pair reference control
  with effect typing + membranes. (`HostAccess.SCOPED`-style handle-escape
  prevention was refuted as unreliable; do not lean on it.)
- Coarse host grants (class loading, native, IO) "effectively grant all access" —
  keep grants fine-grained or the boundary proof collapses to a heuristic.
- The **singleton-mirror** preference is OUR design choice (less duplication, one
  convergence target), NOT an externally proven law — the research did not back
  the "one canonical run with trace facets" pattern either way. Keep it as a
  rationale, not a proof.

### Capability distribution table (host / guest / interop; feat-branch status)

| capability | layer | feat-branch status |
|---|---|---|
| Clojure form read/normalize/macroexpand/eval/compile oracle | **clj-meta (host)** | clj-meta mature; pnix-clj `clojure_form` host-eval routed via `interop` (Phase C done) |
| namespace/Var/metadata/classpath/class-artifact reflection; dynamic require/resolve | **clj-meta (host)** | clj-meta domain; pnix-clj `clojure_projection` host reflection = MOVE (Phase B) |
| host mutation/pollution detection; host introspection | **clj-meta (host)** | clj-meta domain |
| deny-by-default allowlist + effect-class gating + capability grants | **interop** | TARGET (build incrementally) |
| value/function/module marshalling; opaque host refs | **interop** | seam started (`pnix-clj.interop`, `clj_meta.clj`); opaque-ref rule TARGET |
| pnix tokenizer/parser/AST | **pnix-clj (guest)** | BUILT |
| pnix evaluator/value/builtins/env; **laziness (thunks)** | **pnix-clj (guest)** | BUILT (lazy let + cbn args; lazy attrset/list TARGET) |
| pnix lowering policy (pnix -> Clojure) | **pnix-clj (guest)** | BUILT (output crosses interop) |
| canonical term / CAS / content hash (names as separate metadata) | **pnix-clj (guest)** | TARGET — PORT from origin/main `cas.clj` |
| append-only event/evidence store; event hash/index; pointer-as-event | **pnix-clj (guest)** | TARGET — PORT from origin/main `store.clj` |
| stage tower (stage1..7); snapshot/resolve; purity/determinism | **pnix-clj (guest)** | TARGET — PORT from origin/main `stage.clj`/`stm.clj`/`resolve.clj`/`purity.clj` |
| singleton `run-mirror` + trace facets | **pnix-clj (guest)** | NOT YET (today: `mirror.clj` + report lanes; Phase D) |
| witness / gate / loss schema | **pnix-clj (guest)** + interop fields | seed in `error.clj` + `interop-meta`; TARGET |

### Distribution sequencing (guest side, feat-branch)
1. **Interop hardening**: opaque-ref rule for host objects (fix `java-object-term`),
   effect-class on every crossing, deny-by-default grants.
2. **Separation Phase B/C/E**: move host reflection/eval behind interop; delegate
   compile proof to clj-meta.
3. **PORT CAS** (`cas.clj`) + **event store** (`store.clj`) from origin/main,
   adapted to this branch's value model and the names-as-metadata rule.
4. **Singleton `run-mirror`** (Phase D) once CAS/store land.
5. **Stage tower + snapshot/purity** (PORT `stage`/`stm`/`resolve`/`purity`), each
   a stage with an explicit witness.

Each step stays gate-green and is committed as its own slice. Heavy lexer / large
refactor / PORT steps are best done supervised, not unattended.
