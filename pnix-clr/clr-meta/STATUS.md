# clr-meta status (peer host-meta floor)

Last verified: 2026-08-07.

## Peer-floor statement

**clr-meta** is the PNIX-agnostic ClojureCLR host bootstrap beneath `pnix-clr`.
Practical peer floor matches other host metas for **product substrate**, with
an honest Stage3–15/N ladder still open:

| Peer | Peer floor | clr-meta counterpart |
|---|---|---|
| JVM Clojure host | bytecode selfhost | eval gen0–2 + C0–C3 Stage1/2 |
| Hy host | stage ladder / fixed-point | C3 Stage2 + source-hidden fresh-target replay |
| Rust host | TV + stage chain | checked-I64 Stage1 + selfhost PE emit |
| ClojureScript host | fixed-point compiler | Stage2 same-source recompile (not full IL fixed point) |

Meta-first order: `clr-meta` before `pnix-clr`. Artifact builder + hash-bound
load path closed. Stage3–15/N remains roadmap (`STAGE15_N_ROADMAP.md`), same
honesty as not claiming Stage15 replacement on clj/rs/cljs.

## Closed claims

Live-verified this session (2026-08-07) via `./bin/clr-meta-gate eval-only`:

```text
bootstrap-test (gen0→1→2 self-interpretation)  ready=true
  18 tests / 171 assertions, 0 fail / 0 error
  all corpus cases stage-values agree across gens
tool-gate (-e / file gen2 + strict reader)     PASS
  (+ 20 22) => 42 via evaluator-generation-2
  reader-eval / tagged / trailing / map rejected
```

Documented closed (heavy C1–C3 gates; not re-run full chain this session):

```text
checked-I64 Compiler Stage1 family
selfhost C1 admission  receipt 3a163588…
selfhost C2 executable Stage1 artifact
selfhost C3 Stage2 + source-hidden fresh-target replay
host-clojureclr-aot runtime artifact builder
```

## Closed this wave (2026-08-07) — Compiler Stage3–7 + path fix

```text
./scripts/clr-meta-compiler-selfhost-stage3-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage4-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage5-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage6-gate   PASS
./scripts/clr-meta-compiler-selfhost-stage7-gate   PASS
  compiler_stage{5,6,7} = true; stageN_fresh_target_replay = true
  structural_description_equal_to_parent = true
  same_source_recompile chain Stage2→…→Stage7
  promotion/allowed? = false
  self_reproduction / stage8 / stage15_n / fixed_point = false
receipts: work/compiler-selfhost-stage{3–7}-gate.receipt.json
builders: scripts/clr-meta-build-compiler-selfhost-stage{3–7}
CLIs:     --build-compiler-selfhost-stage{3–7}
design:   STAGE{3–7}_DESIGN.md
gate chain: scripts/clr-meta-gate → stage3…stage7

pnix-clr relative FILE.px path fix (caller cwd before artifact cd)
pnix-clr common-slice: live five-host floor (URI, JSON, dynamic attrs, exact int,
  mixed float, non-finite observation, POSIX ERE classes, failed-thunk replay,
  kernel/math guest modules). See RESIDUAL_SURFACE.md for open principles.
  promotion/allowed? = false
```

## Open claims (do not claim)

```text
compiler_stage8_through_15_n = false
compiler_self_reproduction = false
clr_il_fixed_point = false
raw_aot_rebuild_determinism = false
broad_clojureclr_compatibility / replacement = false
pnix_common_compiler_integration = false
cross_host_canonical_equivalence / clr_host_promotion = false
```

## Trusting-Trust defense roadmap (Diverse Double-Compiling)

**Nothing closed yet on this axis, and no shortcut exists.** Unlike Rust
(`mrustc`), there is no independently-authored third-party ClojureCLR compiler
in the wild to lean on — a second, independent backend would have to be built
in-house, same constraint clj-meta already worked through for the JVM host.

Concrete plan, adapted from clj-meta's already-closed U5/U6 pattern (same host
language family, directly reusable lessons):

```text
Step 1 — independent interpreter cross-check (clj-meta's U5 analogue)
    The existing gen0-2 nested evaluator lane already interprets rather than
    compiles. Extend it (or add a sibling tree-walking evaluator) to cover the
    same corpus the Compiler Stage1/2 PE-emitting lane targets, and compare
    behavior: PE-emitted output vs interpreted output on identical inputs.
    This catches a backdoor unique to either lane, though — same honest scope
    clj-meta records for its own kernel — an interpreter is not a second
    *compiler*, so this alone would not be the full Wheeler bar.

Step 2 — independent minimal 2nd PE emitter (clj-meta's U6 analogue)
    Author a small, algorithmically independent second backend that emits CLR
    PE bytes directly for a bounded fixture set (mirrors clj-meta's tiny
    reader+analyzer+ASM-emitter that covers 17 fixtures with zero calls into
    the shared recognizer/emit path). Must not reuse the C2/C3 compiler
    kernel's lowering-owner or PE-sink code — a shared emit path defeats the
    purpose.
    Cross-validate: the Compiler Stage1/2 chain's output vs this independent
    emitter's output must agree (behaviorally, and ideally byte-for-byte for
    the specific fixture PE format) on the shared fixture set.

Step 3 — widen the fixture set toward the real conformance corpus
    Same honest scoping as clj-meta U5/U6: grow coverage incrementally, record
    exactly how much of the corpus each side of the DDC actually spans, and
    keep "full Wheeler DDC" held until the independent backend's coverage
    matches the primary compiler's.
```

This is a genuinely large, from-scratch build (the "high difficulty, must be
built in-house" case clj-meta's own `todo.md` R4 flags for any host without an
existing independent compiler) — expect it to be its own multi-session track,
sequenced after Stage8+ same-lineage work stabilizes, not before it.

## Primary gate

```sh
# From pnix-clr/clr-meta/  (prefer real rg on PATH)
export PATH="/usr/local/bin:/usr/bin:/bin:$PATH"
./bin/clr-meta-gate              # full family, --no-build default
./bin/clr-meta-gate --build      # rebuild bootstrap first
./bin/clr-meta-gate eval-only    # gen0–2 + tool only (lighter peer floor)
```

Full script chain: bootstrap-test → tool-gate → compiler-stage1-gate →
selfhost-stage1-gate → selfhost-stage2-gate.

## Tooling note

Gate scripts expect `rg` (ripgrep). Prefer **`/usr/local/bin/rg`**. Do **not**
put `pnix-clr/bin` first on `PATH` — that tree may ship an old `rg` shim.

## Last run (this machine, 2026-08-07)

| Gate | Result | Notes |
|---|---|---|
| `./bin/clr-meta-gate eval-only` | **PASS** | ready=true; tool-gate PASS |
| full C1–C3 chain | not re-run this session | scripts/*-gate; docs claim closed |
| `./scripts/clr-meta-compiler-selfhost-stage3-gate` | **PASS** | Stage2→Stage3 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage4-gate` | **PASS** | Stage3→Stage4 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage5-gate` | **PASS** | Stage4→Stage5 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage6-gate` | **PASS** | Stage5→Stage6 + source-hidden replay |
| `./scripts/clr-meta-compiler-selfhost-stage7-gate` | **PASS** | Stage6→Stage7 + source-hidden replay |
| env | dotnet 10.0.302, published Clojure.Main.dll | OK |
