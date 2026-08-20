# pnix-runtime-ct QA Guide


> 2026-06-02 update: former client/control runtime material has been absorbed into pnixc-meta mirror primitives. Legacy client/control names below are fixture/schema/path compatibility or historical migration evidence; new implementation work should target pnixc-meta `.px` owners and replacement host adapters.

## Current convergence note (2026-03-13)

This crate document describes one implementation, testing, or audit surface within the current convergence plan.
It does not redefine the repository-wide ontology: the canonical base remains the shared substrate for state, meaning, observation, plan, and evidence.
Read this crate as one adapter/runtime/lowering surface under that substrate, with `pnix` as code/execution projection, `freecat` as spatial/world-model projection, and replacement projection adapters as non-owner control/governance surfaces; former `puck` labels are historical.

## What to validate

`pnix-runtime-ct` currently provides:
- expression parsing into a `CTDiagram`
- optional diagram extraction into `CtDiagramOutput`
- strict vs lenient error surface controlled by `CtConfig.strict`

It does not expose structured `errors`/`warnings` arrays in `CtCheckResult` (use `notes` + `RuntimeError`).

## Test commands

```bash
# All tests
cargo test -p pnix-runtime-ct

# Verbose output
cargo test -p pnix-runtime-ct -- --nocapture

# List tests (use this instead of keeping a static list in docs)
cargo test -p pnix-runtime-ct -- --list
```

## Strict vs lenient behavior

- `CtConfig { strict: true, .. }`
  - parse failure: `Err(RuntimeError)`
- `CtConfig { strict: false, .. }`
  - parse failure: `Ok(CtCheckResult { success: false, notes: ["Parse error: ..."], diagram: None })`

## Determinism knobs

`CtConfig.seed` / `now_ms` / `clock_step_ms` are accepted for API consistency.
The current engine does not use time or randomness, so these values are currently unused.

## Caching runtime

`CachingCtRuntime` caches verification results by:
- expression string
- `extract_diagram` flag
- optional `seed` (part of the cache key for deterministic hashing)
