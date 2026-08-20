# pnix-runtime-ct Examples


> 2026-06-02 update: former client/control runtime material has been absorbed into pnixc-meta mirror primitives. Legacy client/control names below are fixture/schema/path compatibility or historical migration evidence; new implementation work should target pnixc-meta `.px` owners and replacement host adapters.

## Current convergence note (2026-03-13)

This crate document describes one implementation, testing, or audit surface within the current convergence plan.
It does not redefine the repository-wide ontology: the canonical base remains the shared substrate for state, meaning, observation, plan, and evidence.
Read this crate as one adapter/runtime/lowering surface under that substrate, with `pnix` as code/execution projection, `freecat` as spatial/world-model projection, and replacement projection adapters as non-owner control/governance surfaces; former `puck` labels are historical.

API-accurate snippets for CT types, diagrams, and `CtRuntimeEngine`.

> Note: `pnix-runtime-ct` currently focuses on **expression parsing + CT diagram extraction**.
> It does not expose `errors`/`warnings` lists in `CtCheckResult`. Failures surface as:
> - **strict** (`CtConfig.strict=true`): `Err(RuntimeError)`
> - **lenient** (`CtConfig.strict=false`): `Ok(CtCheckResult { success: false, notes: [...], diagram: None })`

## CTType parsing

```rust
use pnix_runtime_ct::CTType;

fn main() {
    let angle = CTType::from_str("angle");
    assert_eq!(angle, CTType::Angle);
}
```

## MorphismOp normalization

```rust
use pnix_runtime_ct::MorphismOp;

fn main() {
    let sin = MorphismOp::from_name("sin").unwrap();
    assert_eq!(sin.canonical_name(), "sin");
    assert_eq!(sin.as_str(), "sin");
}
```

## Build a diagram manually

```rust
use pnix_runtime_ct::{CTDiagram, CTType, MorphismOp};

fn main() {
    let mut diagram = CTDiagram::new();
    let angle = diagram.add_object("angle", CTType::Angle);
    let result = diagram.add_object("result", CTType::Real);
    diagram.add_morphism("sin", angle, result, MorphismOp::Sin);

    let out = diagram.to_output_deterministic();
    println!("objects={}", out.objects.len());
    println!("morphisms={}", out.morphisms.len());
}
```

## Verify an expression (diagram extraction)

```rust
use pnix_runtime_ct::CtRuntimeEngine;
use pnix_runtime_api::{CtConfig, CtRuntime, CtSpec, RuntimeError};

fn main() -> Result<(), RuntimeError> {
    let mut runtime = CtRuntimeEngine::new();
    let spec = CtSpec::new("sin(t)").with_diagram(true);
    let config = CtConfig::default();

    let check = runtime.verify(&spec, &config)?;
    assert!(check.success);
    println!("{:?}", check.notes);
    Ok(())
}
```

## Lenient parse errors

```rust
use pnix_runtime_ct::CtRuntimeEngine;
use pnix_runtime_api::{CtConfig, CtRuntime, CtSpec};

fn main() {
    let mut runtime = CtRuntimeEngine::new();
    let spec = CtSpec::new("this_is_not_valid(");

    let strict = CtConfig { strict: true, ..Default::default() };
    assert!(runtime.verify(&spec, &strict).is_err());

    let lenient = CtConfig { strict: false, ..Default::default() };
    let check = runtime.verify(&spec, &lenient).unwrap();
    assert!(!check.success);
    println!("{:?}", check.notes);
}
```

## Output shape

### `CtCheckResult` (executor output uses this shape)

```json
{
  "success": true,
  "notes": ["..."],
  "diagram": {
    "objects": [{ "id": 0, "name": "t", "ct_type": "Time" }],
    "morphisms": [{ "name": "sin", "source": "Angle", "target": "Real" }]
  }
}
```
