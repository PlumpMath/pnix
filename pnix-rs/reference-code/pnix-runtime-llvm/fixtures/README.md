# Test Fixtures


> 2026-06-02 update: former client/control runtime material has been absorbed into pnixc-meta mirror primitives. Legacy client/control names below are fixture/schema/path compatibility or historical migration evidence; new implementation work should target pnixc-meta `.px` owners and replacement host adapters.

## Current convergence note (2026-03-13)

This crate document describes one implementation, testing, or audit surface within the current convergence plan.
It does not redefine the repository-wide ontology: the canonical base remains the shared substrate for state, meaning, observation, plan, and evidence.
Read this crate as one adapter/runtime/lowering surface under that substrate, with `pnix` as code/execution projection, `freecat` as spatial/world-model projection, and replacement projection adapters as non-owner control/governance surfaces; former `puck` labels are historical.

This directory contains sample FxCore modules used for testing JIT and AOT compilation in `pnix-runtime-llvm`.

## Purpose

Fixtures provide stable, known-good FxCore module examples for:
- **Unit testing**: Verify JIT/AOT compilation paths work correctly
- **Integration testing**: Test end-to-end compilation and execution
- **Regression testing**: Ensure changes don't break existing functionality
- **Documentation**: Show example FxCore module structures

## Fixture Summary Table

| Fixture | Purpose | Inputs | Operations | Used By Tests | Test Focus |
|---------|---------|--------|------------|--------------|------------|
| `simple_module.json` | Basic single-input module | 1 (Int) | `add` | `test_fixture_module_loading`, `test_jit_smoke_fixture` | Single input, basic binary op |
| `minimal_const.json` | Empty module edge case | 0 | None | `test_fixtures_expanded`, `test_fixture_reusability` | Empty module handling |
| `two_inputs.json` | Multi-input module | 2 (Int) | `add` | `test_fixtures_expanded`, `test_jit_input_parameters_smoke` | Multiple inputs, ordering |
| `expected_aot_manifest.json` | Manifest schema example | N/A | N/A | `test_aot_artifact_layout_validation`, `test_aot_manifest_fields_validation` | Manifest validation |

## Available Fixtures

### `simple_module.json`

Basic module with one integer input and an `add` operation.

**Structure**:
- 1 input: `value` (Int)
- 1 morphism: `add` (binary addition)
- 1 node: `result` (uses `add`)
- 1 edge: connects input to result

**Used by**:
- `test_fixture_module_loading`
- `test_jit_smoke_fixture`

**Tests**:
- Single input parameter handling
- Basic binary operation lowering
- JIT compilation and execution

### `minimal_const.json`

Minimal module with no inputs, morphisms, or nodes (empty module).

**Structure**:
- Empty module (minimal valid FxCoreModule)
- No inputs, morphisms, nodes, or edges

**Used by**:
- `test_fixtures_expanded`
- `test_fixture_reusability`

**Tests**:
- Edge case handling for empty modules
- Fixture parsing robustness
- Module validation

### `two_inputs.json`

Module with two integer inputs (`a`, `b`) and an `add` operation.

**Structure**:
- 2 inputs: `a` (Int), `b` (Int)
- 1 morphism: `add` (binary addition)
- 1 node: `result` (uses `add`)
- 2 edges: connect inputs `a` and `b` to result

**Used by**:
- `test_fixtures_expanded`
- `test_jit_input_parameters_smoke`

**Tests**:
- Multiple input parameter handling
- Parameter ordering stability
- Multi-input binary operations

### `expected_aot_manifest.json`

Expected AOT manifest structure for validation tests.

**Structure**:
- JSON schema matching `AotArtifactManifest`
- Example manifest fields and values

**Used by**:
- `test_aot_artifact_layout_validation`
- `test_aot_manifest_fields_validation`

**Tests**:
- Manifest schema compliance
- Field presence and types
- Manifest serialization/deserialization

## Using Fixtures in Tests

Fixtures are loaded via `std::fs::read_to_string` and parsed as `FxCoreModule`:

```rust
use std::fs;
use std::path::PathBuf;
use pnix_core::core::FxCoreModule;

let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("fixtures")
    .join("simple_module.json");

let fixture_content = fs::read_to_string(&fixture_path)?;
let fx_module: FxCoreModule = serde_json::from_str(&fixture_content)?;

// Use fx_module for testing
let mut engine = JitEngine::new();
let ir_json = serde_json::to_vec(&fx_module)?;
let module = engine.compile("test_module", &ir_json)?;
```

## Fixture Format Evolution

Tests that use fixtures gracefully handle missing or invalid fixtures:
- Missing fixtures: Test skips with warning
- Invalid JSON: Test skips with warning
- Schema changes: Tests adapt to fixture format changes

This allows fixture format evolution without breaking all tests.

## Adding New Fixtures

When adding new fixtures:

1. **Naming**: Use descriptive names (e.g., `three_inputs.json`, `nested_operations.json`)
2. **Structure**: Follow FxCoreModule schema
3. **Documentation**: Update this README with fixture description and usage
4. **Tests**: Add tests that use the new fixture
5. **Validation**: Ensure fixture is valid FxCoreModule JSON

## Fixture Maintenance

- **Keep fixtures minimal**: Focus on specific test scenarios
- **Update tests**: When fixture structure changes, update tests accordingly
- **Version control**: Fixtures are version-controlled and should remain stable
- **Documentation**: Keep this README up-to-date with fixture changes
