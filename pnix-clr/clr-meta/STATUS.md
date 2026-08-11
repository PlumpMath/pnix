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

Unlike Rust (`mrustc`), there is no independently-authored third-party
ClojureCLR compiler in the wild to lean on — a second, independent backend
has to be built in-house, the same constraint the reference JVM host
(the `frontend_selfhost.clj`/`diverse_double_compile.clj` pair) already
worked through for its own DDC witness, and the same pattern followed here.

**Independent mini backend added this session (2026-08-11):**
`independent_mini_backend.clj` is a new, from-scratch Int64
tokenizer/reader + analyzer + `System.Reflection.Emit.DynamicMethod` IL
emitter, sharing zero code with the Compiler Stage1-7 family
(`compiler_stage1.clj`, `compiler_selfhost_*.clj`), which uses
`System.Reflection.Emit.PersistedAssemblyBuilder` to produce full PE
executables. `DynamicMethod` JITs a method in memory and hands back an
invokable handle directly — it never touches the assembly/PE-writing path
the Stage1-7 family shares. The pinned ClojureCLR runtime and the CLR itself
remain trusted host substrate, the same honest role the JVM classfile format
plays for the reference host's tiny frontend witness.

Covers 8 fixtures (`+`/`-`/`*` checked-overflow arithmetic, `<`/`>`/`<=`/`>=`/
`=` comparisons, `if`, 0/1/2-arg functions). Cross-validated against real
host ClojureCLR `eval` — both agree on all 8. Wired into
`independent-mini-backend-test` (`clr-meta/test/pnix/clr_meta/`), which now
runs as part of the aggregate `bootstrap-test` entry point invoked by
`scripts/clr-meta-gate`. Verified live this session: 19 tests / 187
assertions, 0 failures, `:ready true`; full `pnix-clr-gate` re-run green with
no regressions.

**What this closes and what it still doesn't:** a genuine 2-way behavioral
comparison (real host `eval` ≡ from-scratch `DynamicMethod`-based mini
backend) now exists and passes, not just a documented plan. It is still only
8 fixtures, scoped to the same checked-Int64 arithmetic/comparison/`if`
surface the Compiler Stage1 profile itself targets — not the full
conformance corpus, and (same honest bar settled on for every host this
session) behavior equivalence, not byte-identical IL, since a
`DynamicMethod`-JITted method and a `PersistedAssemblyBuilder`-written PE are
different CLR artifact kinds by construction.

**Next concrete step:** widen the fixture set (nested `if`, more arg
arities, the checked-overflow negative cases the Stage1 gate itself already
exercises) toward the same corpus the Compiler Stage1 profile covers, so the
comparison stops being "a bounded subset" and starts being "the whole
profile, independently cross-validated." Building an independent
interpreter (as opposed to a second compiler) to cross-check the gen0-2
evaluator lane remains a separate, not-yet-started track — an interpreter
alone would not clear the full Wheeler bar even if added.

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
