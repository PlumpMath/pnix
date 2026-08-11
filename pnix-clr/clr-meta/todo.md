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

2. **Stage8 — reproducible assembly artifact closure — NEXT PRIORITY.** State: not started
   (no design doc, no gate/builder scripts, `stage8: false` stamped
   everywhere). Done = an explicit policy for PE metadata, MVIDs, debug
   info, paths, and timestamps, plus a gate proving two independent builds
   from the same frozen source produce artifacts equal under that policy —
   same shape as the five already-shipped `clr-meta-build/gate-compiler-
   selfhost-stage{3..7}` pairs. **Size: medium** — the gate/builder
   scaffolding pattern is proven and copyable, but the PE/MVID/timestamp
   determinism policy for `System.Reflection.Emit.PersistedAssemblyBuilder`
   output is genuinely new work, not a port.

3. **Stage9 — clean-process compiler/runtime replay.** State: not started;
   depends on Stage8's artifact existing. Done = the Stage8 artifact runs in
   an actual fresh `dotnet` process (not in-process eval) and reproduces the
   same canonical result. **Size: small–medium** once Stage8 lands — rs-meta
   and hy-meta both have a working stage9 pattern (clean subprocess +
   manifest binding) to follow.

4. **Compiler self-reproduction / B==C fixed point.** State: false. Stage3–7
   prove same-source recompile plus structural-description equality to the
   immediate parent, but not that a stage reproduces itself byte-identically
   (the "kernelB compiles kernelC, B==C" pattern hy-meta and rs-meta both
   closed). **Size: medium** — likely folds naturally out of the Stage8
   determinism-policy work rather than needing a fully separate track.

5. **Stage10 (sandbox/session isolation) and Stage11–15/N (multi-domain
   adapters, self-improvement quarantine, long-horizon replay, cross-host
   law, open-world evidence, constitutional extension).** State: none of
   this scaffolding exists in clr-meta yet — no adapter matrix, no
   quarantine storage, no cross-host export/import commands, nothing. Every
   other host (`hy-meta`, `rs-meta` confirmed by direct read of their
   `todo.md`s this session) has fully closed stages 10–15/N, each stage
   having expanded into dozens of checklist items — so this is a real,
   large, and *known-shaped* gap: the pattern to port exists, it just hasn't
   been built for CLR yet. **Size: large** — this is the single biggest
   remaining item by volume, but it is sequenced behind Stage8/9/10 by the
   roadmap's own ordering (`STAGE15_N_ROADMAP.md` "Ordering toward an actual
   replacement," step 5), so it is not the next thing to pick up.

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
