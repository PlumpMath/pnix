# clr-meta TODO / continuation note

Full target definitions, stage numbering, and the promotion ordering live in
`STAGE15_N_ROADMAP.md` — this file does not repeat them. Read that doc first
for what each stage number *means*; read `STATUS.md` for what is currently
verified closed. This file is only the prioritized "what's left" map.

## Current Remaining Work (verified 2026-08-11)

Verification method this pass: read `STAGE15_N_ROADMAP.md` + `STATUS.md` in
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

4. **Compiler self-reproduction / B==C fixed point.** State: false. Stage3–7
   prove same-source recompile plus structural-description equality to the
   immediate parent, but not that a stage reproduces itself byte-identically
   (the "kernelB compiles kernelC, B==C" pattern hy-meta and rs-meta both
   closed). **Size: medium.** Partially de-risked by Stage8: the byte-level
   reproducibility machinery (canonicalization, sha256/cmp comparison) that a
   fixed-point proof would need already exists and is proven working — what's
   still missing is compiling the compiler's *own* source through itself
   (not just recompiling the frozen kernel through successive generations).

5. **Stage10 (sandbox/session isolation) and Stage11–15/N (multi-domain
   adapters, self-improvement quarantine, long-horizon replay, cross-host
   law, open-world evidence, constitutional extension) — NEXT PRIORITY now
   that Stage8/9 are both closed.** State: none of this scaffolding exists in
   clr-meta yet — no adapter matrix, no quarantine storage, no cross-host
   export/import commands, nothing. Every other host (`hy-meta`, `rs-meta`
   confirmed by direct read of their `todo.md`s this session) has fully
   closed stages 10–15/N, each stage having expanded into dozens of checklist
   items — so this is a real, large, and *known-shaped* gap: the pattern to
   port exists, it just hasn't been built for CLR yet. **Size: large** — this
   is the single biggest remaining item by volume on this list.

6. **Independent-interpreter DDC track (distinct from the compiler-backend
   DDC work already closed).** State: not started at all. The DDC gap that
   *is* closed covers the Compiler Stage1–7 family (a second, from-scratch
   *compiler* backend). A second, from-scratch tree-walking *interpreter*
   that cross-checks the gen0→1→2 evaluator lane is a separate, explicitly
   flagged-as-not-started track per `STATUS.md`'s own text — "an interpreter
   alone would not clear the full Wheeler bar even if added," i.e. it's
   necessary but not sufficient on its own. **Size: medium** — similar
   scope to the mini-backend work just finished (~1 session); the target
   corpus (the focused evaluator corpus already proven across gen0–2) is
   already defined.

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
