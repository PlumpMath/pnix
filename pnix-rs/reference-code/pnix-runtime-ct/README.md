# pnix-runtime-ct


> 2026-06-02 update: former client/control runtime material has been absorbed into pnixc-meta mirror primitives. Legacy client/control names below are fixture/schema/path compatibility or historical migration evidence; new implementation work should target pnixc-meta `.px` owners and replacement host adapters.

## Current convergence note (2026-03-13)

This crate document describes one implementation, testing, or audit surface within the current convergence plan.
It does not redefine the repository-wide ontology: the canonical base remains the shared substrate for state, meaning, observation, plan, and evidence.
Read this crate as one adapter/runtime/lowering surface under that substrate, with `pnix` as code/execution projection, `freecat` as spatial/world-model projection, and replacement projection adapters as non-owner control/governance surfaces; former `puck` labels are historical.

Category Theory (CT) runtime for **expression parsing** and **diagram extraction**.

## Scope (v0.1)

- Input: `pnix_runtime_api::CtSpec` (expression string + `extract_diagram` flag)
- Output: `pnix_runtime_api::CtCheckResult` (`success`, `notes`, optional `diagram`)
- Strict vs lenient:
  - `CtConfig.strict=true`: parse failures return `Err(RuntimeError)`
  - `CtConfig.strict=false`: parse failures return `Ok(CtCheckResult { success: false, notes: [...], diagram: None })`

This crate does **not** expose `errors`/`warnings` arrays in `CtCheckResult`.

## CT types

```rust
use pnix_runtime_ct::CTType;

let angle = CTType::from_str("angle"); // case-insensitive
assert_eq!(angle, CTType::Angle);
```

## Morphism operations

```rust
use pnix_runtime_ct::MorphismOp;

let sin = MorphismOp::from_name("sin").unwrap();
assert_eq!(sin.canonical_name(), "sin");
```

## CT diagrams

```rust
use pnix_runtime_ct::{CTDiagram, CTType, MorphismOp};

let mut diagram = CTDiagram::new();
let angle = diagram.add_object("angle", CTType::Angle);
let result = diagram.add_object("result", CTType::Real);
diagram.add_morphism("sin", angle, result, MorphismOp::Sin);

let output = diagram.to_output_deterministic();
assert_eq!(output.objects.len(), 2);
assert_eq!(output.morphisms.len(), 1);
```

## Runtime engine (`CtRuntimeEngine`)

Implements `pnix_runtime_api::CtRuntime`:

```rust
use pnix_runtime_ct::CtRuntimeEngine;
use pnix_runtime_api::{CtConfig, CtRuntime, CtSpec, RuntimeError};

fn main() -> Result<(), RuntimeError> {
    let mut runtime = CtRuntimeEngine::new();
    let spec = CtSpec::new("sin(t)").with_diagram(true);
    let config = CtConfig::default();

    let check = runtime.verify(&spec, &config)?;
    assert!(check.success);
    Ok(())
}
```

## Determinism knobs

`CtConfig.seed` / `now_ms` / `clock_step_ms` are accepted for API consistency.
The current CT engine does not use time or randomness, so these values are currently unused.

## References

- `crates/pnix-runtime-ct/EXAMPLES.md`
- `crates/pnix-runtime-ct/QA.md`
