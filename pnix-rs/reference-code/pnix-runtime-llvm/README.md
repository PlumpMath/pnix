# pnix-runtime-llvm


> 2026-06-02 update: former client/control runtime material has been absorbed into pnixc-meta mirror primitives. Legacy client/control names below are fixture/schema/path compatibility or historical migration evidence; new implementation work should target pnixc-meta `.px` owners and replacement host adapters.

## Current convergence note (2026-03-13)

This crate document describes one implementation, testing, or audit surface within the current convergence plan.
It does not redefine the repository-wide ontology: the canonical base remains the shared substrate for state, meaning, observation, plan, and evidence.
Read this crate as one adapter/runtime/lowering surface under that substrate, with `pnix` as code/execution projection, `freecat` as spatial/world-model projection, and replacement projection adapters as non-owner control/governance surfaces; former `puck` labels are historical.

Feature-gated LLVM JIT/AOT runtime for FxCore modules.

## Quick Links

- [QA Guide](QA.md): Testing commands and expected results
- [Test Fixtures](fixtures/): Fixture documentation and usage

## Overview

- Provides JIT and AOT entry points behind `pnix-runtime-api` traits.
- Uses `inkwell` when the `llvm` feature is enabled.
- Real codegen: FxCore -> LLVM IR lowering with binary operations support.
- Execution: ExecutionEngine for JIT, TargetMachine for AOT object emission.

## Status

- **JIT compile**: FxCore -> LLVM IR lowering implemented.
  - Supports: const literals, binary ops (add/sub/mul/div), float unary ops (sin/cos/sqrt/floor/ceil), FxCore inputs as function params.
  - Returns: `JitModule` with compiled LLVM IR.
- **JIT eval**: ExecutionEngine path implemented.
  - Executes compiled function and returns Int/Float result as JSON.
  - Feature-gated: requires `llvm` feature and LLVM installation.
- **AOT compile**: TargetMachine object emission implemented.
  - Host-only: compiles to object file for current platform.
  - Deterministic: same input produces same output (size/hash stable).
- **Artifact packaging + manifest**: implemented and deterministic.

**Current Limitations**:

### Supported Operations
Canonical subset definition: `../../docs/llvm-subset.md` (compile-time rejection policy).
See `../../docs/ssa-coverage.md` for the per-runtime SSA op coverage matrix.
- **Binary arithmetic**: `add`, `sub`, `mul`, `div`, `mod` (signed integer division, modulo)
  - All five operations are tested: `test_jit_constant_add`, `test_jit_constant_sub`, `test_jit_constant_mul`, `test_jit_constant_div`, `test_jit_mod_operation`
  - Input parameters tested: `test_jit_input_parameters_smoke`
- **Power**: `pow`, `**` (Int/Float, uses LLVM pow intrinsic)
  - Tested: `test_jit_pow_operation`
- **Bitwise operations**: `shl`, `shr`, `bitand`, `bitor`, `bitxor`, `bitnot` (Int only, **LLVM-only**)
  - Tested: `test_jit_bitwise_operations`, `test_jit_bitwise_float_error`
  - Float type rejected with clear error message
  - **Note**: Bitwise operations are LLVM-only and not available in other runtimes (ir-eval, legacy-eval, etc.)
  - These operations are not part of core `MeaningOpId` or language syntax; they are only available when using LLVM backend
- **Comparisons**: `eq`, `ne`, `lt`, `le`, `gt`, `ge` (returns Bool → JSON boolean)
- **Boolean ops**: `and`, `or`, `not` (Bool-only, requires pure inputs for short-circuit ops)
- **Conditional select**: `if`/`select` (3 inputs: cond, then, else)
- **Float math**: `sin`, `cos`, `sqrt`, `floor`, `ceil`
- **String operations**: `concat` (limited support - basic structure only)
  - Tested: `test_jit_string_concat`
  - Note: Full string constant initialization and concatenation requires runtime helpers (deferred)
- **Input types**: `i32`/`Int`, `f64`/`Float`, `Bool` (treated as 0/1), and `String` inputs supported (String has limited support)
- **Output types**: `i32`/`Int`, `f64`/`Float`, `Bool`, and `String` return values supported (String has limited support)
- **Constants**: Integer/Float literals supported (parsed from `from_input` string)
- **String literals**: String literals in `from_input` (quoted strings) are parsed but initialization is simplified

### Unsupported Operations
- **Other types**: `List`, `AttrSet` (not yet implemented)
- **Full string support**: Complete string constant initialization and concatenation (requires runtime helpers)

### Integer Range
- **Input values**: Standard i32 range (-2^31 to 2^31-1)
- **Overflow**: LLVM i32 operations wrap on overflow (undefined behavior in some contexts)

### Cross-Compilation
- **Target triple override**: `AotConfig.target_triple_override` allows explicit LLVM triple selection.
- **Behavior**: If LLVM lacks the target backend, compilation fails with a guided error.
- **Limitation**: AOT still emits object files only (linking/execution is external).
- **Workaround**: For full cross-platform execution, use backend-legacy codegen or a platform toolchain.

## Known Gaps

### Unsupported Operations
The following FxCore operations are not yet supported in LLVM lowering:

- **Conditional edges**: `when`, `unless` (Stage-3 edge conditions)
- **List/AttrSet operations**: List/AttrSet manipulation operations
- **Full string operations**: Complete string constant initialization and concatenation (basic structure exists but requires runtime helpers)

**Error Message Example**:
When an unsupported operation is encountered, you'll see an error like:
```
LLVM config error: Unsupported morphism operation: 'unknown_op'. Supported operations: add, sub, mul, div, mod, pow (Int/Float), bitwise (shl/shr/bitand/bitor/bitxor/bitnot, Int only), comparisons (eq/ne/lt/le/gt/ge), if/select, float math (sin/cos/sqrt/floor/ceil), and string concat (limited). Operation 'unknown_op' is not yet implemented in LLVM lowering.
```

This clearly indicates which operation is unsupported and what operations are currently available.

### Type Limitations
- **Input types**: `Int`/`Float` only (single numeric kind per module). `String`, `List`, `AttrSet` not yet implemented.
- **Output types**: `Int`/`Float` return values supported. Other return types not yet supported.
- **Type conversion**: No automatic type coercion or conversion.

### Performance Considerations
- **JIT compilation**: Each module is compiled on first use. No persistent cache across process restarts.
- **AOT compilation**: Object files are generated but not optimized for size (optimization level configurable).
- **Memory**: Large modules may consume significant memory during compilation.

### Future Work
- [ ] Support for String/List/AttrSet types
- [ ] Cross-compilation toolchain automation (linkers, runtime libs)
- [ ] Persistent JIT cache across process restarts
- [ ] Size-optimized AOT builds

## Feature Flags

- `llvm`: enables inkwell + LLVM paths.

**Without `llvm` feature**:
- `JitEngine::compile()` returns `RuntimeError::unimplemented("LLVM compilation requires 'llvm' feature...")`
- `JitEngine::execute()` returns `RuntimeError::unimplemented("jit execution requires llvm feature")`
- `AotEngine::compile_from_ir()` returns `RuntimeError::unimplemented("aot compilation requires llvm feature")`

**Important**: Stub modules are never returned. All methods return explicit errors when the `llvm` feature is not enabled.

## Build Prerequisites (when `feature=llvm`)

- **LLVM toolchain**: Must be installed and match the version required by `inkwell`/`llvm-sys` crates.
  - Check `inkwell` documentation for compatible LLVM versions (typically LLVM 14+).
  - On macOS: `brew install llvm@14` or `brew install llvm@14`
  - On Linux: Install via package manager (e.g., `apt-get install llvm-dev libclang-dev`)
  - On Windows: Use LLVM pre-built binaries or vcpkg
- **llvm-config**: Must be available on PATH or set via `LLVM_SYS_<version>_PREFIX` environment variable.
  - Example: `export LLVM_SYS_140_PREFIX=/usr/local/opt/llvm@14`
  - Verify: `llvm-config --version` should print the installed version
- **libclang**: Required for LLVM bindings compilation.
  - Usually included with LLVM installation
  - On macOS: May need `xcode-select --install` for command line tools

## LLVM Installation and Detection Tips

### Finding LLVM Installation

If `llvm-config` is not found automatically, you can help `llvm-sys` locate LLVM:

1. **Find llvm-config**:
   ```bash
   # Check if llvm-config is in PATH
   which llvm-config
   
   # Or search for it
   find /usr -name llvm-config 2>/dev/null
   find /opt -name llvm-config 2>/dev/null
   ```

2. **Set LLVM_SYS_*_PREFIX environment variable**:
   ```bash
   # For LLVM 14.0
   export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14
   
   # For LLVM 14.0
   export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14
   
   # Or use the directory containing llvm-config
   export LLVM_SYS_140_PREFIX=$(dirname $(dirname $(which llvm-config)))
   ```

3. **Verify LLVM detection**:
   ```bash
   # Check version
   llvm-config --version
   
   # Check libraries
   llvm-config --libs
   
   # Check include path
   llvm-config --includedir
   ```

### Platform-Specific Installation

**macOS (Homebrew)**:
```bash
# Install LLVM 14
brew install llvm@14

# Add to PATH
export PATH="/opt/homebrew/opt/llvm@14/bin:$PATH"

# Set prefix (if needed)
export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14
```

**Linux (apt)**:
```bash
# Install LLVM development packages
sudo apt-get update
sudo apt-get install llvm-14-dev libclang-14-dev

# Verify installation
llvm-config-14 --version
```

**Linux (manual detection)**:
```bash
# If llvm-config is versioned (llvm-config-14)
export LLVM_SYS_140_PREFIX=/usr/lib/llvm-14

# Or find the installation
export LLVM_SYS_140_PREFIX=$(dirname $(dirname $(which llvm-config-14)))
```

### Common Detection Issues

- **"No suitable version of LLVM was found"**: Set `LLVM_SYS_<version>_PREFIX` explicitly
- **Version mismatch**: Ensure LLVM version matches inkwell/llvm-sys requirements
- **Multiple LLVM versions**: Use `LLVM_SYS_<version>_PREFIX` to specify which version to use
- **Non-standard installation**: Point `LLVM_SYS_*_PREFIX` to the LLVM root directory (contains `bin/`, `lib/`, `include/`)

## Behavior Matrix

```
Mode / Feature     llvm=off                     llvm=on
----------------  ---------------------------- -------------------------------
JIT compile        unimplemented error          returns JitModule (IR built)
JIT eval           unimplemented error          executes via ExecutionEngine (i32 return)
compile_and_run    unimplemented error          compiles + executes (returns JSON)
AOT compile        unimplemented error          emits object file via TargetMachine
Artifacts/manifest available                    available
```

## Feature Matrix (JIT/AOT/Host-only)

| Feature | JIT | AOT | Host-only | Cross-compile | Status |
|---------|-----|-----|-----------|---------------|--------|
| **Compilation** | ✅ | ✅ | ✅ | ❌ | JIT: ExecutionEngine, AOT: TargetMachine |
| **Execution** | ✅ | ❌ | N/A | N/A | JIT executes immediately, AOT requires linking |
| **Input Types** | i32 only | i32 only | ✅ | ❌ | Float/String/List/AttrSet not supported |
| **Operations** | add/sub/mul/div | add/sub/mul/div | ✅ | ❌ | Mod/pow/comparison not supported |
| **Determinism** | ✅ | ✅ | ✅ | ❌ | Same input → same output (hash stable) |
| **Caching** | Module cache | Artifact cache | ✅ | ❌ | JIT: in-memory, AOT: file-based |
| **Performance** | Compile+run | Compile only | ✅ | ❌ | JIT: slower first run, AOT: faster builds |
| **Platform** | Host only | Host only | ✅ | ❌ | Cross-compilation not implemented |

**Legend**:
- ✅: Supported
- ❌: Not supported
- N/A: Not applicable

**Notes**:
- **JIT**: Just-In-Time compilation and execution. Requires `llvm` feature and LLVM installation.
- **AOT**: Ahead-Of-Time compilation to object files. Requires `llvm` feature and LLVM installation.
- **Host-only**: Default behavior. AOT can override target triple when LLVM has the target backend.
- **Cross-compile**: Partial support via `target_triple_override`; still requires toolchain/linker setup.

**Current Implementation**:
- JIT: Lowering supports constants, binary ops (add/sub/mul/div), and FxCore inputs as function parameters. ExecutionEngine executes and returns i32 results as JSON.
- AOT: Host-only object file emission via TargetMachine. Object files are deterministic (same input -> same output). Cross-compilation not yet implemented.

## AOT Artifact Layout

The AOT layout is deterministic and uses a fixed tree under a base output directory:

```
<base_dir>/
  dist/
    bin/
      <name>            (or <name>.exe on Windows)
    lib/
      lib<name>.so      (Linux)
      lib<name>.dylib   (macOS)
      lib<name>.dll     (Windows)
    manifest/
      <name>.json
```

`AotArtifactManifest` paths are relative to the base output directory:
- `binary_path`: `bin/<name>` (or `bin/<name>.exe`)
- `library_path`: `lib/lib<name>.<ext>`
- Manifest file: `manifest/<name>.json`

## Manifest Schema (AotArtifactManifest)

Fields:
- `name`: module name
- `target_triple`: LLVM target triple
- `version`: artifact version
- `entry_point`: entry symbol (default: `pnix_entry`)
- `binary_path`: `bin/<name>` or `bin/<name>.exe`
- `library_path`: `lib/lib<name>.<ext>`
- `build_timestamp`: omitted for determinism
- `build_config`: `{ opt_level, debug, output_format }`
- `metadata`: optional map

### AOT Manifest JSON Example

The AOT compilation produces a manifest JSON file:

```json
{
  "name": "my_module",
  "target_triple": "x86_64-unknown-linux-gnu",
  "version": "1.0.0",
  "entry_point": "pnix_entry",
  "binary_path": "bin/my_module",
  "library_path": "lib/libmy_module.so",
  "build_timestamp": null,
  "build_config": {
    "opt_level": 2,
    "debug": false,
    "output_format": "object"
  },
  "metadata": {}
}
```

**Fields**:
- `name`: Module name (string)
- `target_triple`: LLVM target triple (string, e.g., `"x86_64-unknown-linux-gnu"`)
- `version`: Artifact version (string)
- `entry_point`: Entry function name (string, default: `"pnix_entry"`)
- `binary_path`: Relative path to executable binary (string)
- `library_path`: Relative path to shared library, or `null` if not generated (string or null)
- `build_timestamp`: Always `null` for deterministic builds (null)
- `build_config`: Build configuration object
  - `opt_level`: Optimization level 0-3 (integer)
  - `debug`: Whether debug symbols included (boolean)
  - `output_format`: Output format, e.g., `"object"` (string)
- `metadata`: Optional metadata map (object)

**Usage**:
```rust
let manifest = layout.create_manifest("my_module", AotTarget::LinuxX86_64, "pnix_entry".to_string());
let manifest_json = manifest.to_json()?;
// Write to file: dist/manifest/my_module.json
```

## Error Mapping

`LlvmRuntimeError` -> `RuntimeError::message(...)`:
- `CompilationError`: "LLVM compilation error: ..."
- `VerificationError`: "LLVM verification error: ..."
- `ExecutionError`: "LLVM execution error: ..."
- `ConfigError`: "LLVM config error: ..."

## Determinism Config

LLVM JIT execution does not yet wire `EvalConfig` deterministic knobs into runtime behavior.
To avoid "accepted but ignored" behavior, JIT execution rejects non-`None` values for:
- `seed`
- `now_ms`
- `clock_step_ms`

Use `pnix-runtime-legacy` for deterministic time/random semantics until LLVM execution supports them.

## Quick Links

- [Examples](EXAMPLES.md): JIT/AOT 실행 예제와 출력 형태, LLVM 탐지 팁

## Enabling the Feature

Examples:

```
# executor with llvm feature
cargo run -p pnix-executor-graph --features llvm -- --mode llvm --source path/to/fxcore.json

# runtime-llvm tests with llvm feature
cargo test -p pnix-runtime-llvm --features llvm
```

## LLVM Discovery Env Vars

Common variables (from llvm-sys):
- `LLVM_SYS_<version>_PREFIX` (e.g., `LLVM_SYS_160_PREFIX`)
- `LLVM_SYS_<version>_IGNORE_BLACKLIST=1`

These are used to point llvm-sys to the correct LLVM install.

## Testing and Verification

### Without LLVM Feature

Run tests without LLVM:
```bash
cargo test -p pnix-runtime-llvm --lib
```

**Expected behavior**:
- 20 tests pass
- 2 tests ignored (feature-gated: `test_jit_with_llvm_feature`, `test_aot_with_llvm_feature`)
- JIT eval/AOT compile return `RuntimeError::unimplemented` errors

### With LLVM Feature

Run tests with LLVM:
```bash
cargo test -p pnix-runtime-llvm --lib --features llvm
```

**Expected behavior** (when LLVM is installed):
- All tests pass including feature-gated tests
- JIT execution succeeds for simple modules
- AOT compilation produces object files

**If LLVM is not installed**:
- Build may fail with "No suitable version of LLVM was found"
- Set `LLVM_SYS_<version>_PREFIX` environment variable to point to LLVM installation

### Executor Usage

Test with executor:
```bash
# Without llvm feature: returns unimplemented error
cargo run -p pnix-executor-graph -- --mode llvm --source path/to/module.json

# With llvm feature: compiles and executes
cargo run -p pnix-executor-graph --features llvm -- --mode llvm --source path/to/module.json
```

**Output format**: JSON with `ok: true`, `value_bytes` containing JSON-encoded result, and `value` when decoding succeeds.

## Troubleshooting

### LLVM Installation Issues

- **`llvm-config not found`**: Ensure LLVM is installed and on PATH.
  ```bash
  # macOS
  brew install llvm@14
  export PATH="/opt/homebrew/opt/llvm@14/bin:$PATH"
  
  # Linux
  sudo apt-get install llvm-14-dev
  ```

- **`libclang` errors**: Install libclang and set `LIBCLANG_PATH` if needed.
  ```bash
  export LIBCLANG_PATH=/path/to/libclang
  ```

- **Version mismatch**: Align your LLVM install with the inkwell/llvm-sys versions.
  - Check `Cargo.toml` for required LLVM version
  - Verify: `llvm-config --version` matches requirements

- **LLVM context creation fails**: Set `LLVM_SYS_<version>_PREFIX`.
  ```bash
  # Example for LLVM 14.0
  export LLVM_SYS_140_PREFIX=/opt/homebrew/opt/llvm@14
  ```

### Example: llvm-config Path Issue

If `llvm-config` is not in PATH:
```bash
# Find llvm-config
which llvm-config
# or
find /usr -name llvm-config 2>/dev/null

# Set prefix
export LLVM_SYS_140_PREFIX=$(dirname $(dirname $(which llvm-config)))
```

### Feature and Compilation Issues

- **Feature-gated errors**: Enable `llvm` feature: `cargo build --features llvm` or `cargo test --features llvm`
  - Without feature: JIT eval/AOT compile return `RuntimeError::unimplemented`

- **Compilation errors**: 
  - Check LLVM installation: `llvm-config --libs`
  - Verify inkwell can find LLVM: Check build output for LLVM_SYS errors
  - Try setting `LLVM_SYS_<version>_PREFIX` explicitly

### Runtime Errors

- **ExecutionEngine errors**: May indicate LLVM installation issues or unsupported module structure
  - Check for unsupported module structure (non-i32 types, unsupported operations)
  - Verify module uses only supported operations: `add`, `sub`, `mul`, `div`

- **Lowering errors**: Check that module uses supported operations (add/sub/mul/div) and i32 types only
  - Error message will indicate which operation is unsupported
  - See "Known Gaps" section for list of unsupported operations

- **Verification errors**: LLVM IR verification failed (invalid IR structure)
  - Check module structure: nodes, edges, morphisms must be valid
  - Ensure all referenced nodes/morphisms exist

### AOT Compilation Issues

- **Object file generation fails**:
  - Verify target triple is supported: `llvm-config --host-target`
  - Check TargetMachine initialization (may fail if LLVM targets not initialized)
  - Ensure output directory is writable

- **Manifest generation issues**:
  - Check file paths are valid
  - Verify artifact layout structure matches expected format

### Test Failures

- **Test failures without llvm feature**: Expected. Feature-gated tests are ignored.
- **ExecutionEngine errors**: May indicate LLVM installation issues or unsupported module structure.
- **Lowering errors**: Check that module uses supported operations (add/sub/mul/div) and i32 types only.

## Target/Host Matrix (Current)

- JIT: host-only, real execution via ExecutionEngine.
- AOT: object file emission with optional target triple override.
- Cross-compile: possible if LLVM target backend is installed (fails with guidance otherwise).

### Target Triple Mapping

| AotTarget | LLVM Target Triple |
|-----------|-------------------|
| `LinuxX86_64` | `x86_64-unknown-linux-gnu` |
| `MacOSX86_64` | `x86_64-apple-darwin` |
| `MacOSArm64` | `aarch64-apple-darwin` |
| `WindowsX86_64` | `x86_64-pc-windows-msvc` |

## Usage Examples

### JIT Compilation and Execution

```rust
use pnix_runtime_llvm::JitEngine;
use pnix_runtime_api::EvalConfig;

let mut engine = JitEngine::new();
let config = EvalConfig::default();

// Compile FxCore module (JSON bytes)
let module = engine.compile("my_module", &fxcore_json_bytes)?;

// Execute
let result = engine.eval(&module, &config)?;

// Result contains JSON-encoded i32 value
let result_value: i32 = serde_json::from_slice(&result.value.data)?;
```

**Example FxCore module** (minimal with constants):
```json
{
  "name": "add_example",
  "inputs": [],
  "morphisms": [{"name": "add", "inputs": [{"name": "x", "ty": "Int"}], "outputs": [{"name": "sum", "ty": "Int"}], "effect": "pure"}],
  "nodes": [{"name": "result", "uses": "add", "kind": "normal"}],
  "edges": [
    {"from": "input", "to": "result", "from_input": "2"},
    {"from": "input", "to": "result", "from_input": "3"}
  ]
}
```

**Example FxCore module** (with input parameters):
```json
{
  "name": "add_inputs",
  "inputs": [
    {"name": "a", "ty": "Int"},
    {"name": "b", "ty": "Int"}
  ],
  "morphisms": [{"name": "add", "inputs": [{"name": "x", "ty": "Int"}], "outputs": [{"name": "sum", "ty": "Int"}], "effect": "pure"}],
  "nodes": [{"name": "result", "uses": "add", "kind": "normal"}],
  "edges": [
    {"from": "input", "to": "result", "from_input": "a"},
    {"from": "input", "to": "result", "from_input": "b"}
  ]
}
```

### AOT Compilation

```rust
use pnix_runtime_llvm::{AotEngine, AotTarget};

let engine = AotEngine::with_config(AotConfig {
    target: AotTarget::LinuxX86_64,
    opt_level: 2,
    ..Default::default()
});

// Compile to object file
let output = engine.compile("my_module")?;

// Package artifacts (no file system writes)
let layout = engine.package_artifacts("my_module", &output)?;

// Explicitly write to disk if needed
engine.write_artifacts_to_disk(&layout, &output, "dist")?;
```

**Fixtures**: See `fixtures/` directory for example FxCore modules:
- `simple_module.json`: Single input module
- `minimal_const.json`: No inputs, constants only
- `two_inputs.json`: Two input parameters

## Examples Section

### JIT Example: Constant Addition

```rust
use pnix_runtime_llvm::JitEngine;
use pnix_runtime_api::EvalConfig;

let mut engine = JitEngine::new();
let fxcore_json = r#"{
  "name": "add_const",
  "inputs": [],
  "morphisms": [{"name": "add", "inputs": [{"name": "x", "ty": "Int"}], "outputs": [{"name": "sum", "ty": "Int"}], "effect": "pure"}],
  "nodes": [{"name": "result", "uses": "add", "kind": "normal"}],
  "edges": [
    {"from": "input", "to": "result", "from_input": "2"},
    {"from": "input", "to": "result", "from_input": "3"}
  ]
}"#;

let module = engine.compile("add_const", fxcore_json.as_bytes())?;
let config = EvalConfig::default();
let result = engine.eval(&module, &config)?;
// result.value.data contains JSON: "5"
```

### JIT Output JSON Example

The JIT execution returns a `EvalResult` with JSON-encoded output:

```json
{
  "ok": true,
  "value": {
    "data": "5"
  },
  "error": null
}
```

**Fields**:
- `ok`: `true` if execution succeeded, `false` if failed
- `value.data`: JSON-encoded result (for i32, this is a JSON number string like `"5"`)
- `error`: `null` on success, error message string on failure

**Parsing the result**:
```rust
use serde_json;

// Parse JSON-encoded i32 result
let result_json: serde_json::Value = serde_json::from_slice(&result.value.data)?;
let result_value: i32 = result_json.as_i64().unwrap() as i32;
// result_value = 5
```

**Error case**:
```json
{
  "ok": false,
  "value": null,
  "error": "LLVM execution error: function returned invalid value"
}
```

### AOT Example: Object File Generation

```rust
use pnix_runtime_llvm::{AotEngine, AotTarget, AotConfig};

let engine = AotEngine::with_config(AotConfig {
    target: AotTarget::LinuxX86_64,
    opt_level: 2,
    ..Default::default()
});

let output = engine.compile("my_module")?;
let layout = engine.package_artifacts("my_module", &output)?;
// layout contains paths: dist/bin/my_module, dist/lib/libmy_module.so, dist/manifest/my_module.json
```

## Sample AOT Output Tree

```
<base_dir>/
  dist/
    bin/
      demo_app
    lib/
      libdemo_app.so
    manifest/
      demo_app.json
```

## Performance Notes

### JIT Compilation Performance
- **First compilation**: Slower (LLVM IR generation + compilation)
- **Cached modules**: Faster (reuses compiled module from cache)
- **Module size**: Larger modules take longer to compile
- **Optimization level**: Higher opt_level (0-3) increases compile time but may improve runtime performance

### AOT Compilation Performance
- **Object file generation**: Typically faster than JIT (no execution engine setup)
- **Deterministic builds**: Same input produces same output (enables caching)
- **Target platform**: Host-only compilation is fastest; cross-compilation adds overhead

### Runtime Performance
- **JIT execution**: Compiled code runs at native speed
- **AOT execution**: Requires linking and loading (one-time cost)
- **Input marshalling**: JSON parsing overhead for inputs (minimal for small inputs)

### Optimization Tips
- Use `opt_level: 2` for balanced compile/run performance
- Cache compiled modules when possible (by module name or replay_hash)
- Prefer AOT for production deployments (no JIT overhead)
- Use deterministic configs (`seed`, `now_ms`) for reproducible builds

## Test Fixtures

The `fixtures/` directory contains sample FxCore modules used for testing JIT and AOT compilation:

### Available Fixtures

- **`simple_module.json`**: Basic module with one integer input and an `add` operation.
  - Used by: `test_fixture_module_loading`, `test_jit_smoke_fixture`
  - Tests: Single input parameter handling, basic binary operation lowering

- **`minimal_const.json`**: Minimal module with no inputs, morphisms, or nodes (empty module).
  - Used by: `test_fixtures_expanded`, `test_fixture_reusability`
  - Tests: Edge case handling for empty modules, fixture parsing robustness

- **`two_inputs.json`**: Module with two integer inputs (`a`, `b`) and an `add` operation.
  - Used by: `test_fixtures_expanded`, `test_jit_input_parameters_smoke`
  - Tests: Multiple input parameter handling, parameter ordering stability

- **`expected_aot_manifest.json`**: Expected AOT manifest structure for validation tests.
  - Used by: `test_aot_artifact_layout_validation`, `test_aot_manifest_fields_validation`
  - Tests: Manifest schema compliance, field presence and types

### Using Fixtures in Tests

Fixtures are loaded via `std::fs::read_to_string` and parsed as `FxCoreModule`:

```rust
let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("fixtures")
    .join("simple_module.json");
let fx_module: FxCoreModule = serde_json::from_str(&fs::read_to_string(&fixture_path)?)?;
```

Tests that use fixtures gracefully handle missing or invalid fixtures (skip with warning) to allow fixture format evolution.

## Real Run Notes

### Test Results (without llvm feature)

**Date**: 2024
**Environment**: macOS (darwin 25.2.0)
**Command**: `cargo test -p pnix-runtime-llvm --lib`

**Results**:
- ✅ 20 tests passed
- ⏭️ 2 tests ignored (feature-gated: `test_jit_with_llvm_feature`, `test_aot_with_llvm_feature`)
- ❌ 0 tests failed

**Behavior**:
- JIT compile: Returns `RuntimeError::unimplemented("LLVM compilation requires 'llvm' feature...")`
- JIT eval: Returns `RuntimeError::unimplemented("jit execution requires llvm feature")`
- AOT compile: Returns `RuntimeError::unimplemented("aot compilation requires llvm feature")`
- Artifact packaging: Works correctly (no LLVM required)

### Test Results (with llvm feature)

**Status**: Requires LLVM installation
**Expected behavior** (when LLVM is available):
- All tests pass including feature-gated tests
- JIT execution succeeds for modules with const literals and binary ops
- AOT compilation produces deterministic object files

**Known limitations**:
- LLVM version must match inkwell/llvm-sys requirements
- Cross-compilation not yet supported
- Only i32/Int types supported for inputs/outputs

### AOT Artifact Layout Verification

**Host target**: macOS ARM64 (aarch64-apple-darwin)
**Layout paths** (relative to base output directory):
- Binary: `dist/bin/<name>` (no extension on Unix)
- Library: `dist/lib/lib<name>.dylib` (macOS)
- Manifest: `dist/manifest/<name>.json`

**Manifest fields verified**:
- ✅ `name`: Module name
- ✅ `target_triple`: LLVM target triple (e.g., "aarch64-apple-darwin")
- ✅ `version`: Artifact version ("0.1.0")
- ✅ `entry_point`: Entry symbol ("pnix_entry")
- ✅ `binary_path`: Relative path to binary
- ✅ `library_path`: Relative path to library
- ✅ `build_timestamp`: Omitted (deterministic builds)
- ✅ `build_config`: Optimization level, debug flag, output format

**Determinism verified**:
- Same input produces same object file size
- Same input produces same manifest hash
- No timestamps in manifest or paths
