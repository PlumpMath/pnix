# clr-meta TODO / continuation note

Full target definitions, stage numbering, and the promotion ordering live in
`STAGE15_N_ROADMAP.md` — this file does not repeat them. Read that doc first
for what each stage number *means*; read `STATUS.md` for what is currently
verified closed. This file is only the prioritized "what's left" map.

## Current Remaining Work (verified 2026-08-11, updated 2026-08-12)

**2026-08-12 update:** items 1, 2, 3, and 5 below are now all DONE (mini
backend widening, Stage8, Stage9, and Stage10–15/N respectively) — see each
item's own entry for what closed and how it was verified. Only items 4, 6,
and 7 remain open.

Verification method this pass (2026-08-11 baseline, still accurate for what
it covers): read `STAGE15_N_ROADMAP.md` + `STATUS.md` in
full, confirmed no `todo.md`/`SCOPE_LOCK.md` existed yet (checked recursively
across `pnix-clr/`), and spot-checked claims against real files rather than
trusting doc prose:

- `independent_mini_backend.clj` (177 lines) and its test (29 lines, 8
  fixtures, `deftest independent-mini-backend-agrees-with-host-eval`) exist
  and are wired into `bootstrap_test.clj` exactly as `STATUS.md` claims —
  confirmed done, not re-flagged below.
- `work/compiler-selfhost-stage{3,4,5,6,7}-gate.receipt.json` all exist,
  timestamped today 15:14–15:18, `ready: true`, and the stage7 receipt's
  `claims` block reads exactly as documented (`compiler_stage7: true`,
  `same_source_recompile: true`, `stage7_fresh_target_replay: true`,
  `stage8: false`, `self_reproduction: false`, `fixed_point: false`,
  `stage15_n: false`, `clojureclr_replacement: false`) — Stage3–7 closure is
  real, not stale doc text.
- No `STAGE8_DESIGN.md` and no `stage8` gate/builder script exist anywhere
  under `scripts/`; every stage7 artifact (`stage7-contract.edn`,
  `clr-meta-build-compiler-selfhost-stage7`, the stage7 gate) explicitly
  stamps `stage8: false` — Stage8 genuinely has not been started.
- Housekeeping nit (not a functional gap): `STAGE15_N_ROADMAP.md`'s trailing
  "Open claims" block still has a single `compiler_stage6_through_15_n =
  false` line lumping stage6/7 in with the unstarted stage8–15/N range, even
  though the same file's own opening line and `STATUS.md` both say
  Stage3–7 are closed. Worth splitting into `compiler_stage6 = true` /
  `compiler_stage7 = true` / `compiler_stage8_through_15_n = false` next time
  that doc is touched — cosmetic, doesn't affect gate behavior.

### Priority-ordered remaining items

1. **Widen the independent mini-backend fixture set — DONE (2026-08-11, later pass).**
   State was: 8 fixtures (checked `+`/`-`/`*`, comparisons, `if`, 0–2 arg
   fns), not the full Compiler Stage1 `checked-i64-expression` profile —
   missing nested `if`, more arities, the checked-overflow negative cases the
   Stage1 gate itself already exercises. Now: 15 value-returning fixtures
   (added nested `if`, 3-arg, 4-arg functions) plus 4 checked-overflow
   negative fixtures (`Int64.MaxValue`/`Int64.MinValue` boundary cases for
   `+`/`-`/`*`, asserting both real-host `eval` and the mini backend reject
   them — the mini backend's `Add_Ovf`/`Sub_Ovf`/`Mul_Ovf` IL opcodes were
   always checked, just untested until now). Verified:
   `independent-mini-backend-test` namespace 38/38 assertions, full
   `bin/clr-meta-gate --no-build` 209/209 assertions, `:ready true`, no
   regressions. Do not re-flag as open.

2. **Stage8 — reproducible assembly artifact closure — DONE (2026-08-12).**
   Was: not started (no design doc, no gate/builder scripts, `stage8: false`
   stamped everywhere). Now: found the *actual* non-determinism by measurement
   (two builds of the same frozen source, byte-diffed) rather than assuming
   the roadmap's generic list — exactly two fields varied: PE COFF
   `TimeDateStamp` and the module `Mvid`; no PDB/debug-info variance exists in
   this codegen path (checked, not assumed). `PeSink.Finish()` now
   canonicalizes both (`compiler-selfhost-runtime/PeSink.cs`
   `CanonicalizeForReproducibility`); a new `describe-determinism` verb
   independently re-reads both fields from a finished artifact. New gate
   `scripts/clr-meta-compiler-selfhost-stage8-gate` builds Stage7 twice from
   the same frozen Stage6 and requires byte-identical output — PASS on first
   run. Policy recorded in `compiler-selfhost/stage8-contract.edn`; design in
   `STAGE8_DESIGN.md`; wired into `scripts/clr-meta-gate`. Verified: full
   `bin/clr-meta-gate --no-build` still green (209/209 assertions, all
   Stage1–8 gates PASS) — no regressions. Unplanned bonus observed live:
   Stage3–7's own compiler DLLs are now all sha256-identical to each other
   too, not just structurally equal, since canonicalization removed the only
   two things that varied between otherwise-identical recompiles of the same
   frozen kernel. Do not re-flag Stage8 as open or as "not started."

3. **Stage9 — clean-process compiler/runtime replay — DONE (2026-08-12, same
   day as Stage8).** Was: not started. Found the actual gap by checking what
   Stage1-8 do NOT cover: every one of them calls
   `compiler-selfhost-runtime`'s support DLL directly, or runs
   `bootstrap-test` in-process inheriting the calling shell's environment —
   none of them exercise `bin/clr-meta` itself (the thing a user actually
   runs) under a fully cleared environment (`env -i`, nothing inherited).
   New gate `scripts/clr-meta-compiler-selfhost-stage9-gate` runs a 4-case
   entrypoint matrix (`--gate`, `-e` eval, single-file, a reader-conditional
   negative case) through `bin/clr-meta` under `env -i`, each case run
   *twice* independently and required to produce byte-identical stdout — the
   replay property, not just correctness. All 4 cases passed with content
   verified (not just self-consistency) on the first run. Design in
   `STAGE9_DESIGN.md`; wired into `scripts/clr-meta-gate`. Verified: full
   `bin/clr-meta-gate --no-build` still green, all Stage1–9 gates PASS, no
   regressions. Do not re-flag Stage9 as open or as "not started."

4. **Compiler self-reproduction / B==C fixed point — DONE (2026-08-12, same
   day as Stage10-15/N).** Was: State false — Stage3–7 proved same-source
   recompile plus structural-description equality to the immediate parent,
   but not that a stage reproduces itself byte-identically. Turned out to
   already be TRUE as an unplanned consequence of Stage8's canonicalization,
   just never formally checked or claimed: Stage8's own gate output had
   already logged Stage3-7 sharing one compiled-artifact sha256 as a bonus
   observation. Verified this pass (not assumed) by rebuilding Stage1
   through Stage7 fresh in a NEW dedicated
   `scripts/clr-meta-compiler-self-reproduction-check`: all seven stages —
   not just an adjacent pair, and including Stage1's host-seeded build
   itself — share the exact same sha256
   (`19872f28ed3576cbdf50001649fb9fd773023778fa0bb5c3d7aee45a61baecb7`).
   Every stage compiles the same frozen `compiler_kernel.clj` through the
   same `PersistedAssemblyBuilder` codegen path Stage8 canonicalized, so
   once the only two non-deterministic PE fields are removed, nothing is
   left to differ between generations — this is the "kernelB compiles
   kernelC, B==C" pattern hy-meta/rs-meta close, at its strongest possible
   form (all generations identical, not just one adjacent pair). Verified
   the shared bytes are not vacuously identical-but-broken: a live
   compile+execute of an unseen target through the shared Stage7 artifact
   still returns the correct result. Design in `SELF_REPRODUCTION_DESIGN.md`;
   wired into `scripts/clr-meta-gate`. Verified full aggregate gate still
   green, no regressions. Do not re-flag as open or "state: false" — check
   `SELF_REPRODUCTION_DESIGN.md`'s explicit scope note before assuming this
   generalizes beyond the Compiler Stage1-7 `PersistedAssemblyBuilder`
   artifact family (a general CLR IL fixed point is a broader, still-open
   claim).

5. **Stage10 (sandbox/session isolation) and Stage11–15/N (multi-domain
   adapters, self-improvement quarantine, long-horizon replay, cross-host
   law, open-world evidence, constitutional extension) — DONE (2026-08-12,
   same day as Stage8/9).** Was: none of this scaffolding existed — no
   adapter matrix, no quarantine storage, no cross-host export/import
   commands, nothing, while every other host (`hy-meta`, `rs-meta`) had
   already closed this whole range. Built following the same pattern those
   hosts use (a policy TSV under `proofs/` per stage, declaring an explicit
   DONE/GROW/HELD/DISABLED stance for every relevant boundary, plus a live
   replay of whatever's DONE), adapted to clr-meta's actual surfaces:
   `proofs/session-sandbox.tsv` (Stage10, load-context shadow rejection +
   session replay), `adapter-schema.tsv` (Stage11), `quarantine-policy.tsv`
   (Stage12), `horizon-policy.tsv` (Stage13), `cross-impl-schema.tsv`
   (Stage14 — includes `independent-mini-backend` as the one row already
   closed to a genuine Trusting-Trust bar, not just local self-consistency),
   `evidence-federation.tsv` (Stage15), `extension-policy.tsv` (StageN), plus
   a new `proofs/stage-manifest.tsv` and `scripts/clr-meta-manifest-check`
   reused as a common replay anchor.
   **Cost-shape bug found and fixed while building this:** the first draft
   had every stage replay its predecessor's *entire* gate twice, compounding
   to quadratic cost by StageN. Fixed so every stage from Stage11 onward
   calls its predecessor exactly once, and the two references to the
   expensive Stage8 rebuild (from Stage11 and Stage14) read its latest
   checked receipt instead of re-running it.
   **Also fixed live:** `clr-meta-manifest-check`'s first draft used
   `declare -A` (bash 4+ associative arrays), which fails outright under
   macOS's system `/bin/bash` (3.2) — this aggregate gate actually runs
   under that bash in this environment, and the failure surfaced immediately
   on the first real aggregate run. Rewritten to plain string matching.
   Verified: full `bin/clr-meta-gate --no-build` PASS end-to-end, Stage1
   through StageN all green, no regressions. Designs in
   `STAGE{10,11,12,13,14,15,N}_DESIGN.md`. Do not re-flag this range as open
   or as "not started."

6. **Independent-interpreter DDC track (distinct from the compiler-backend
   DDC work already closed) — DONE (2026-08-12, same day as items 1-5).**
   Was: not started at all. Built
   `src/pnix/clr_meta/independent_mini_interpreter.clj`: a from-scratch
   tokenizer/reader + tree-walking interpreter for the small,
   environment-driven Lisp subset `bootstrap.clj`'s own 9-case
   `conformance-cases` corpus proves (`quote`/`if`/`let`/`fn` including named
   recursion and `&` variadic rest), sharing zero code with
   `pnix.clr-meta.main`'s reader or `pnix.clr-meta.bootstrap/evaluate`.
   Cross-validated against the *real, textual* `bin/clr-meta -e`
   evaluator-generation-2 tool-eval path (not pre-parsed data — confirmed
   live that ordinary arithmetic/comparison/vector symbols already resolve
   there with no injected environment, unlike `conformance-cases`'s own
   test harness which injects placeholder names per case) via new
   `scripts/clr-meta-independent-mini-interpreter-gate`. Verified: 9/9
   fixtures accepted on the first run, full aggregate gate still green, no
   regressions. Design in `INDEPENDENT_MINI_INTERPRETER_DESIGN.md`. Do not
   re-flag as "not started" — an interpreter alone still does not clear the
   full Wheeler bar by itself (same honest bar as the mini-backend's own
   scope note), but this track itself is now built and gated.

7. **Broad ClojureCLR compatibility/replacement, `pnix_common_compiler_
   integration`, `cross_host_canonical_equivalence`, `clr_host_promotion`.**
   State: false, and explicitly deferred by the roadmap's own ordering
   (steps 6–9: admit exact `-e`/file/REPL/compile/AOT/namespace/tooling
   profiles individually, only then expand `bin/clojure-clr` beyond its
   current facade, only then connect to the common PNIX compiler/machine
   model). **Size: large / long-horizon** — correctly gated behind
   everything above; not actionable until Stage8–15/N closes.

### Explicitly not re-flagged (already done, verified this pass)

- Stage1–7 same-source recompile ladder (C0–C3 checkpoints, Stage3–7 gates).
- The generic `clr-meta` CLR artifact builder / `host-clojureclr-aot` /
  `pnix-clr` 9-namespace-DLL manifest binding.
- The Trusting-Trust independent mini backend (compiler-side DDC), 8
  fixtures, wired into `bootstrap-test`.
- `gen0→1→2` evaluator-generation self-interpretation agreement.

## Verification commands

```sh
# From pnix-clr/clr-meta/
export PATH="/usr/local/bin:/usr/bin:/bin:$PATH"
./bin/clr-meta-gate                # full family, --no-build default
./scripts/clr-meta-compiler-selfhost-stage7-gate   # newest closed stage
```

See `STATUS.md` "Primary gate" section for the full script chain and
`STAGE15_N_ROADMAP.md` for stage definitions and the promotion ordering this
list follows.

## Host toolchain / library export (from dot-nix integration, 2026-08-13)

dot-nix can already expose **CLI runners** (`pnix-clr`, `pnix-clr-pnix`,
`clojure-clr` alias, `pnix-clr-refs` helper). The following are **not**
doable in home-manager alone — they need product work in this tree.
**Do not claim them closed from packaging wrappers.**

### Missing product surfaces

1. **Shareable library package (NuGet and/or `lib/` layout)** — **landed 2026-08-13**  
   - `bin/export-pnix-clr-library` → `lib/{net8,net10}/Pnix.Clr.dll` + guest AOT
     + `build/Pnix.Clr.props|.targets` + `share/pnix-clr/refs.env`.  
   - Flake: `packages/apps.pnix-clr-library`, `pnix-clr-refs`, `clojure-clr`.  
   - C# API: `Pnix.Clr.Eval.Source` / `Eval.File` (process→CLI JSON).  
   - Optional `dotnet pack` / multi-machine nupkg: **dropped** with nuget.org
     (owner local feed only, 2026-08-14).

2. **`clojure-clr` as a real host substrate, not only a name alias** — **partial**  
   - Flake/dot-nix expose `clojure-clr` → `bin/clojure-clr` (clr-meta `-e`/file).  
   - Still missing: full “Clojure on CLR for arbitrary .clj projects”
     (deps.edn / project.clj on CLR) beyond the focused facade.

3. **Stable `DOTNET_*` / Reference env contract** — **landed 2026-08-13**  
   - `PNIX_CLR`, `PNIX_CLR_ROOT`, `PNIX_CLR_ARTIFACT` (+ legacy
     `PNIX_CLR_RUNTIME_ARTIFACT`), `PNIX_CLR_LIBRARY`.  
   - Manifest still gates AOT integrity inside `bin/pnix-clr`.  
   - Optional later: dedicated gate that fails if export layout drifts.

4. **Developer identity of “stock CLR tools”**  
   - Need clear split: Rhino/net8 plugin SDK vs pnix-clr net10 host SDK so
     overlays never silently mix TFMs. (`Pnix.Clr` multi-targets net8+net10
     so Rhino-side C# can reference the managed Eval API on net8.)


## Host-language import of pnix product library (user intent, 2026-08-13)

**Canonical doctrine:** [`../../HOST_DEV_ENV.md`](../../HOST_DEV_ENV.md)  
**HM mirror:** `~/dot-nix/dev/PNIX-HOSTS.md`  
**C# surface:** [`../csharp/Pnix.Clr/README.md`](../csharp/Pnix.Clr/README.md)

Context from home-manager (`dot-nix`) integration:

- `pnix-<host>-pnix` = pnix-language surface (REPL/eval of `.px`) on this host.
- `pnix-<host>-<lang>` = host-language interpreter/compiler used for day-to-day
  host development.
- Libraries produced by the **pnix product half** of this host are **host-
  language libraries**: they must load in *this* host language. They are **not**
  assumed to be portable common bytecode for other hosts.
- A future **common portable `.px` library** track (historical pnix-meta style)
  is deferred; do not block host-local import work on that.

dot-nix can only set PATH/env (classpath, PYTHONPATH, link paths, NODE_PATH,
DLL HintPath). Anything that requires a real packaging format is product work
below.


### clr — status (2026-08-14)

1. **Library package** — **done**: `export-pnix-clr-library` + flake
   `pnix-clr-library` / `pnix-clr-refs` + C# `Pnix.Clr.Eval` + MSBuild props.
2. **ClojureCLR host library story** — **partial**: `clojure-clr` facade +
   guest AOT Reference via props; full arbitrary-.clj project story still open.
3. Versioned env contract — **done**: `PNIX_CLR_*` (+ library path).
4. Dual-axis docs — **done**: monorepo `HOST_DEV_ENV.md` + host `CLAUDE.md` /
   `README.md` + HM matrix.
5. Optional local NuGet — **landed enough**: `bin/pack-pnix-clr-nupkg` +
   `bin/pnix-clr-nupkg-smoke` + `csharp/Directory.Build.props.sample`
   (local feed only; not nuget.org).
6. Explicit note: runtime-artifact `.clj.dll` is **host-bound** (CLR), not a
   common multi-host .px package. (Still true; document, do not claim otherwise.)

## Post host-env plan (2026-08-14) — plan only unless owner pulls

Host library export (`export-pnix-clr-library`, `Pnix.Clr.Eval`, MSBuild props,
local nupkg pack) is **closed enough** for C# day-to-day. See monorepo
`HOST_ENV_P2_P3.md`.

### P3 full ClojureCLR project (detail)
**Goal:** beyond focused `clojure-clr -e` / single-file facade — multi-file
`.clj` projects with stable substrate References.

**Acceptance sketch:**
1. Documented "plain ClojureCLR REPL" entry separate from `pnix-clr` guest eval.
2. Project template: deps or .csproj that References Clojure NuGet pin + optional
   guest AOT path via `PNIX_CLR_ARTIFACT`.
3. Gate: smoke that loads 2 namespaces from disk without pnix product CLI.
4. Honest claims only — no Stage15/N, no "clojure-clr replaces ClojureCLR".

**Order:**
1. ~~Inventory~~ → `docs/CLOJURE_CLR_ADMITTED_SURFACE.md` (2026-08-14).
2. ~~TFM story~~ → `docs/TFM_POLICY.md` (2026-08-14).
3. ~~Template + smoke (bootstrap multi-ns)~~ →
   `examples/clojure-clr-project/` (`./run` / `./smoke` → 42) via
   **clojure-clr-bootstrap**, not the facade (2026-08-14).
4. ~~Named profiles + dual smoke~~ → `bin/clojure-clr --help`,
   `bin/clojure-clr-profiles-smoke` (2026-08-14).
5. ~~tool-eval-multi~~ → `--multi-form FILE` +
   `scripts/clr-meta-tool-eval-multi-gate` in `clr-meta-gate` (2026-08-14).
6. ~~Wire profiles-smoke into aggregate~~ → `bin/pnix-clr-gate` runs
   `clojure-clr-profiles-smoke` after `clr-meta-gate` (~17s, 2026-08-14).
7. ~~Local nupkg pack smoke~~ → `bin/pnix-clr-nupkg-smoke` (export layout +
   pack dual-TFM; local feed only, 2026-08-14).
8. ~~nuget.org~~ → **dropped** (owner: personal/local feed only, 2026-08-14).
9. ~~tool-eval-multi-e / stdin~~ → `--multi-e FORM`, `--multi-form -`
   in multi-gate (2026-08-14); default `-e` still single-form.
10. **Next:** further tool-eval surfaces only with new named gates;
    isolated ALC still held.

### clr-meta residual (product, not packaging)
- Continue stage ladder honesty via STATUS.md + design docs.
- Widen guest eval only with artifact plan + hash gates.
- Do not renumber stages for packaging work.

### Host-import hard
- [x] In-process C# evaluator **design** — `docs/IN_PROCESS_EVAL.md` (2026-08-14).
- [x] In-process **spike** — `InProcessEval.cs` + `SourceInProcess` (net10),
  parity gate `bin/pnix-clr-inprocess-eval-gate` (17-pass). Process-spawn
  remains the supported API default; gate auto when substrate present.
- [x] Local NuGet pack path — `pack-pnix-clr-nupkg` + `pnix-clr-nupkg-smoke`.
  nuget.org publish **dropped** (owner local-only, 2026-08-14).
- [x] In-process broader corpus (14 sources + file + negatives) — gate 17-pass
- [x] In-process in `pnix-clr-gate` when substrate+artifact present
  (`PNIX_CLR_INPROCESS_GATE=0` to skip). Isolated ALC still held.
