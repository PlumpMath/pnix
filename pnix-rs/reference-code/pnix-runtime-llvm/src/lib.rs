//! LLVM JIT/AOT Runtime Engine
//!
//! Provides JIT compilation and AOT (Ahead-of-Time) compilation of FxCore modules using LLVM.
//!
//! ## Status
//!
//! This is a skeleton implementation. Full functionality requires:
//! - LLVM bindings (e.g., `inkwell` or `llvm-sys`)
//! - IR generation from FxCoreModule
//! - Function compilation and linking
//! - Runtime value representation
//!
//! ## API Usage Example
//!
//! ```rust,no_run
//! use pnix_runtime_llvm::JitEngine;
//! use pnix_runtime_api::{EvalConfig, EvalEngine};
//!
//! # fn main() -> pnix_runtime_api::RuntimeResult<()> {
//! // Create JIT engine
//! let mut engine = JitEngine::new();
//!
//! // Compile FxCore module (JSON bytes)
//! let fxcore_json = r#"{"name": "test", "types": [], "inputs": [], "morphisms": [], "nodes": [], "edges": [], "scopes": []}"#;
//! let module = engine.compile("test", fxcore_json.as_bytes())?;
//!
//! // Execute (deterministic knobs are not yet supported; use defaults)
//! let config = EvalConfig::default();
//! let result = engine.eval(&module, &config)?;
//!
//! // Parse result (JSON-encoded i64)
//! let result_value: i64 = serde_json::from_slice(&result.value.data).unwrap();
//! # Ok(())
//! # }
//! ```
//!
//! ## AOT Compilation Status
//!
//! ### Implemented Features
//! - [x] AOT compilation pipeline (real implementation):
//!   - [x] Generate LLVM IR from FxCoreModule (real IR generation with binary ops)
//!   - [x] Set target triple based on AotTarget
//!   - [x] Compile IR to object file (.o) for target platform (via TargetMachine, requires LLVM installation)
//!   - [x] Return binary blob (real object file bytes)
//! - [x] Support multiple target platforms:
//!   - [x] Linux x86_64 (real object file generation)
//!   - [x] macOS x86_64 (real object file generation)
//!   - [x] macOS ARM64 (real object file generation)
//!   - [x] Windows x86_64 (real object file generation)
//! - [x] Artifact packaging with manifest
//!   - [x] Executor-friendly API (package_artifacts + write_artifacts_to_disk)
//! - [x] Entry point configuration
//!
//! ### Limitations (Current)
//! - **Host-only**: AOT compilation targets current platform only
//!   - Cross-compilation requires target-specific LLVM toolchain setup (not yet implemented)
//! - **Binary format**: Object files are generated in platform-native format (ELF/Mach-O/PE)
//!   - Format selection is automatic based on target triple
//!   - Explicit format options not yet implemented
//!
//! ## AOT Artifact Output
//!
//! The AOT compilation produces artifacts in the following structure:
//!
//! ```text
//! dist/
//! ├── bin/
//! │   └── <module_name>          # Executable binary (or <module_name>.exe on Windows)
//! ├── lib/
//! │   └── lib<module_name>.so    # Shared library (Linux: .so, macOS: .dylib, Windows: .dll)
//! └── manifest/
//!     └── <module_name>.json     # Artifact manifest
//! ```
//!
//! ### Manifest Schema
//!
//! The `AotArtifactManifest` includes:
//! - `name`: Module name
//! - `target_triple`: LLVM target triple (e.g., "x86_64-unknown-linux-gnu")
//! - `version`: Artifact version
//! - `entry_point`: Entry point function name (default: "pnix_entry")
//! - `binary_path`: Relative path to binary (e.g., "bin/test_module")
//! - `library_path`: Relative path to library (if applicable)
//! - `build_config`: Build configuration (opt_level, debug, output_format)
//! - `build_timestamp`: None (omitted for deterministic builds)
//!
//! ### API Usage
//!
//! ```rust,no_run
//! use pnix_runtime_llvm::{AotEngine, AotTarget, AotConfig};
//!
//! # fn main() -> pnix_runtime_api::RuntimeResult<()> {
//! let engine = AotEngine::with_config(AotConfig {
//!     target: AotTarget::LinuxX86_64,
//!     opt_level: 2,
//!     ..Default::default()
//! });
//!
//! let ir_json = br#"{"name": "my_module", "types": [], "inputs": [], "morphisms": [], "nodes": [], "edges": [], "scopes": []}"#;
//! // Compile from IR bytes
//! let output = engine.compile_from_ir("my_module", ir_json)?;
//!
//! // Package artifacts (no file system writes)
//! let layout = engine.package_artifacts("my_module", &output)?;
//!
//! // Explicitly write to disk if needed
//! engine.write_artifacts_to_disk(&layout, &output, "dist")?;
//!
//! // Access manifest
//! let manifest = layout.create_manifest(
//!     "my_module".to_string(),
//!     AotTarget::LinuxX86_64,
//!     "pnix_entry".to_string(),
//! );
//! let manifest_json = manifest.to_json().unwrap();
//! # Ok(())
//! # }
//! ```
//!
//! ## Configuration
//!
//! The runtime does not yet wire `EvalConfig` deterministic knobs into JIT execution.
//! To avoid "accepted but ignored" behavior, JIT execution rejects non-`None` values for:
//! - `seed`
//! - `now_ms`
//! - `clock_step_ms`
//!
//! Use `pnix-runtime-legacy` for deterministic time/random semantics until LLVM execution supports them.
//!
//! ### Seed (`seed: Option<u64>`)
//! - Random number generator seed for deterministic execution
//! - When provided, all random operations use this seed
//! - When `None`, uses system random (non-deterministic)
//! - **Current behavior**: Rejected in JIT execution (explicit error)
//! - **Usage**: Set `config.seed = Some(12345)` for reproducible runs
//!
//! ### Now (`now_ms: Option<i64>`)
//! - Current time in milliseconds (Unix timestamp)
//! - Overrides system time for deterministic execution
//! - When provided, `param.system_time` uses this value
//! - When `None`, uses actual system time (non-deterministic)
//! - **Current behavior**: Rejected in JIT execution (explicit error)
//! - **Usage**: Set `config.now_ms = Some(1609459200000)` for fixed time
//!
//! ### Clock Step (`clock_step_ms: Option<i64>`)
//! - Time increment per tick/step in milliseconds
//! - Used for controlled time advance in FRP/event loops
//! - When provided, time advances by this amount per step
//! - When `None`, uses actual elapsed time (non-deterministic)
//! - **Current behavior**: Rejected in JIT execution (explicit error)
//! - **Usage**: Set `config.clock_step_ms = Some(16)` for 60 FPS simulation
//!
//! ### Example
//!
//! ```rust,no_run
//! use pnix_runtime_llvm::JitEngine;
//! use pnix_runtime_api::EvalConfig;
//!
//! let mut engine = JitEngine::new();
//! let config = EvalConfig::default();
//! // deterministic knobs are currently rejected (explicit error) in JIT execution.
//! ```
//!
//! ## JIT Input ABI (Application Binary Interface)
//!
//! ### Value Encoding
//!
//! Inputs are provided as a JSON object where keys are input names and values are encoded as follows:
//!
//! - **Integers**: JSON number (e.g., `42`)
//! - **Floats**: JSON number (e.g., `3.14`)
//! - **Booleans**: JSON boolean (e.g., `true`)
//! - **Strings**: JSON string (e.g., `"hello"`)
//! - **Lists**: JSON array (e.g., `[1, 2, 3]`)
//! - **Attrsets**: JSON object (e.g., `{"x": 1, "y": 2}`)
//!
//! ### Return Format
//!
//! The compiled function returns a `JitValue` which contains:
//!
//! - **Data**: `Vec<u8>` containing the serialized result
//! - **Format**: JSON-encoded value matching the output type
//!
//! Example:
//! ```text
//! Input: {"x": 42, "y": 10}
//! Function: add(x, y)
//! Output: JitValue { data: b"52" } // JSON-encoded integer
//! ```
//!
//! ### Deterministic Execution
//!
//! Deterministic knobs (`seed`/`now_ms`/`clock_step_ms`) are not yet supported for LLVM JIT execution.
//! Until wired end-to-end, JIT rejects non-`None` values to avoid fake determinism.
//!
//! ## Executor-Friendly API Design
//!
//! The AOT packaging API is designed to be executor-friendly:
//! - `package_artifacts()`: Returns artifact layout and manifest **without** writing to disk
//! - `write_artifacts_to_disk()`: Explicit method for file system writes (called by executor)
//!
//! This separation ensures that:
//! - Executor has full control over when/where files are written
//! - No unexpected file system side effects during compilation
//! - Easy to test without actual file I/O
//!
//! ## Integration Points
//!
//! - `pnix-core/src/codegen/llvm.rs`: May contain LLVM IR generation helpers
//! - `pnix-executor-graph`: Will call JIT/AOT engines via `pnix-runtime-api` traits
//! - `pnix-ir-adapter`: May need to convert FxCoreModule to LLVM-friendly format

#[cfg(feature = "llvm")]
use pnix_runtime_api::RuntimeResult;
use pnix_runtime_api::{ExecutionErrorKind, RuntimeError};

mod aot;
mod ffi;
mod jit;

pub use aot::{
  AotArtifactLayout, AotArtifactManifest, AotBuildConfig, AotConfig, AotEngine, AotOutput,
  AotOutputFormat, AotTarget,
};
#[cfg(feature = "llvm")]
pub use ffi::DynamicLibrary;
pub use jit::{JitConfig, JitEngine, JitModule, JitValue};

/// Runtime-llvm specific error types
///
/// # Example
/// ```rust
/// use pnix_runtime_llvm::LlvmRuntimeError;
/// let err = LlvmRuntimeError::ConfigError("missing input".to_string());
/// assert!(matches!(err, LlvmRuntimeError::ConfigError(_)));
/// ```
#[derive(Debug, Clone)]
pub enum LlvmRuntimeError {
  /// LLVM compilation error
  CompilationError(String),
  /// LLVM IR verification error
  VerificationError(String),
  /// Execution error
  ExecutionError(String),
  /// Configuration error
  ConfigError(String),
  /// Resource exhaustion error (e.g., memory, stack, limits)
  ResourceExhausted(String),
  /// Memory access error
  MemoryError(String),
  /// IO error
  IoError(String),
}

#[cfg_attr(not(feature = "llvm"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumericKind {
  Int,
  Float,
}

#[cfg_attr(not(feature = "llvm"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueKind {
  Int,
  Float,
  Bool,
  String,
  List,
  AttrSet,
}

#[cfg_attr(not(feature = "llvm"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OutputKind {
  Scalar(ValueKind),
  Tuple(Vec<ValueKind>),
}

impl OutputKind {
  #[cfg_attr(not(feature = "llvm"), allow(dead_code))]
  fn as_scalar(&self) -> Option<ValueKind> {
    match self {
      OutputKind::Scalar(kind) => Some(*kind),
      OutputKind::Tuple(_) => None,
    }
  }

  #[cfg_attr(not(feature = "llvm"), allow(dead_code))]
  fn is_tuple(&self) -> bool {
    matches!(self, OutputKind::Tuple(_))
  }

  #[cfg_attr(not(feature = "llvm"), allow(dead_code))]
  fn has_ptr(&self) -> bool {
    match self {
      OutputKind::Scalar(kind) => kind.is_ptr(),
      OutputKind::Tuple(kinds) => kinds.iter().any(|kind| kind.is_ptr()),
    }
  }
}

impl std::fmt::Display for OutputKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      OutputKind::Scalar(kind) => write!(f, "{:?}", kind),
      OutputKind::Tuple(kinds) => {
        let parts: Vec<String> = kinds.iter().map(|k| format!("{:?}", k)).collect();
        write!(f, "({})", parts.join(", "))
      }
    }
  }
}

#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_DIV_ZERO_INT: u32 = 1;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_DIV_ZERO_FLOAT: u32 = 2;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_MOD_ZERO_INT: u32 = 3;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_MOD_ZERO_FLOAT: u32 = 4;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_INPUT_LEN_MISMATCH: u32 = 5;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_POW_OVERFLOW: u32 = 6;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_SHIFT_OUT_OF_RANGE: u32 = 7;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_INT_OVERFLOW: u32 = 8;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_STRING_LEN_OVERFLOW: u32 = 9;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_OOM: u32 = 10;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_DOMAIN_ERROR: u32 = 11;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_COND_MISSING_INPUT: u32 = 12;
#[cfg(feature = "llvm")]
pub(crate) const RUNTIME_ERROR_COND_DUP_INPUT: u32 = 13;

impl ValueKind {
  #[cfg_attr(not(feature = "llvm"), allow(dead_code))]
  fn as_numeric(self) -> Option<NumericKind> {
    match self {
      Self::Int => Some(NumericKind::Int),
      Self::Float => Some(NumericKind::Float),
      Self::Bool | Self::String | Self::List | Self::AttrSet => None,
    }
  }

  #[cfg_attr(not(feature = "llvm"), allow(dead_code))]
  fn is_ptr(self) -> bool {
    matches!(self, Self::String | Self::List | Self::AttrSet)
  }
}

impl std::fmt::Display for LlvmRuntimeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::CompilationError(msg) => write!(f, "compilation error: {}", msg),
      Self::VerificationError(msg) => write!(f, "verification error: {}", msg),
      Self::ExecutionError(msg) => write!(f, "execution error: {}", msg),
      Self::ConfigError(msg) => write!(f, "config error: {}", msg),
      Self::ResourceExhausted(msg) => write!(f, "resource exhausted: {}", msg),
      Self::MemoryError(msg) => write!(f, "memory error: {}", msg),
      Self::IoError(msg) => write!(f, "io error: {}", msg),
    }
  }
}

impl std::error::Error for LlvmRuntimeError {}

impl From<LlvmRuntimeError> for RuntimeError {
  fn from(err: LlvmRuntimeError) -> Self {
    let message = err.to_string();
    RuntimeError::execution(ExecutionErrorKind::LLVM, message.clone()).with_source(message)
  }
}

#[cfg(feature = "llvm")]
macro_rules! b {
  ($expr:expr) => {
    $expr.map_err(|e| LlvmRuntimeError::CompilationError(format!("LLVM builder error: {:?}", e)))?
  };
}

#[cfg_attr(not(feature = "llvm"), allow(dead_code))]
fn normalize_type_name(ty: &str) -> String {
  ty.trim().to_ascii_lowercase()
}

#[cfg_attr(not(feature = "llvm"), allow(dead_code))]
fn type_name_to_kind(ty: &str) -> Option<ValueKind> {
  match normalize_type_name(ty).as_str() {
    "int" | "i32" | "i64" => Some(ValueKind::Int),
    "float" | "f64" | "real" => Some(ValueKind::Float),
    "bool" | "boolean" => Some(ValueKind::Bool),
    "string" | "str" => Some(ValueKind::String),
    "list" | "array" => Some(ValueKind::List),
    "attrset" | "attrs" | "set" | "map" => Some(ValueKind::AttrSet),
    _ => None,
  }
}

#[cfg_attr(not(feature = "llvm"), allow(dead_code))]
fn is_numeric_alias(ty: &str) -> bool {
  matches!(normalize_type_name(ty).as_str(), "num" | "number")
}

#[cfg(feature = "llvm")]
fn infer_numeric_kind(
  fx_module: &pnix_core::core::FxCoreModule,
) -> Result<NumericKind, LlvmRuntimeError> {
  let mut seen: Option<NumericKind> = None;
  let mut conflicts: Vec<String> = Vec::new();
  let mut aliases: Vec<String> = Vec::new();

  let mut track_kind = |ty: &str| {
    if is_numeric_alias(ty) {
      aliases.push(ty.to_string());
      return;
    }
    if let Some(kind) = type_name_to_kind(ty).and_then(|k| k.as_numeric()) {
      if let Some(prev) = seen {
        if prev != kind {
          conflicts.push(format!("{} vs {:?}", ty, prev));
        }
      } else {
        seen = Some(kind);
      }
    }
  };

  for input in &fx_module.inputs {
    track_kind(&input.ty);
  }
  for morphism in &fx_module.morphisms {
    // Y13a-21: 포트 타입 기반 numeric kind 추론 - morphism.inputs/outputs 전 포트 타입 검사
    // Stage-1 호환: morphism.input/output도 확인 (하위 호환성)
    if morphism.inputs.is_empty() {
      // Stage-1 morphism: input/output 필드 사용
      track_kind(&morphism.input);
      track_kind(&morphism.output);
    } else {
      // Stage-2 morphism: inputs/outputs 포트 타입 검사
      for port in &morphism.inputs {
        track_kind(&port.ty);
      }
      for port in &morphism.outputs {
        track_kind(&port.ty);
      }
    }
  }

  if !aliases.is_empty() {
    aliases.sort();
    aliases.dedup();
    return Err(LlvmRuntimeError::ConfigError(format!(
      "LLVM requires explicit Int or Float types; Num/Number is not supported (found: {}).",
      aliases.join(", ")
    )));
  }

  if !conflicts.is_empty() {
    return Err(LlvmRuntimeError::ConfigError(format!(
      "Mixed numeric types in module '{}': {}. \
            LLVM lowering currently requires a single numeric kind (Int or Float). \
            See docs/llvm-policy.md for policy options (module separation recommended).",
      fx_module.name,
      conflicts.join(", ")
    )));
  }

  Ok(seen.unwrap_or(NumericKind::Int))
}

#[cfg(feature = "llvm")]
fn infer_output_kind(
  fx_module: &pnix_core::core::FxCoreModule,
  default_numeric: NumericKind,
) -> Result<OutputKind, LlvmRuntimeError> {
  let output_node = fx_module
    .nodes
    .iter()
    .find(|n| n.name == "result")
    .or_else(|| fx_module.nodes.last());

  if let Some(node) = output_node {
    if let Some(morphism) = fx_module.morphisms.iter().find(|m| m.name == node.uses) {
      let op = morphism.name.as_str();
      if matches!(op, "eq" | "ne" | "lt" | "le" | "gt" | "ge") {
        if morphism.outputs.len() > 1 {
          return Err(LlvmRuntimeError::ConfigError(format!(
            "Comparison morphism '{}' on node '{}' cannot return multiple outputs.",
            morphism.name, node.name
          )));
        }
        return Ok(OutputKind::Scalar(ValueKind::Bool));
      }

      // Y13a-21: 포트 타입 기반 output kind 추론 - morphism.outputs 전 포트 타입 검사
      if morphism.outputs.is_empty() {
        // Stage-1 morphism: output 필드 사용
        let output_ty = morphism.output.trim();
        if output_ty.is_empty() {
          return Err(LlvmRuntimeError::ConfigError(format!(
            "Output type is empty for output node '{}' (morphism '{}'). \
Supported output types: Int/i64, Float/f64, Bool, String (limited support).",
            node.name, morphism.name
          )));
        }
        if is_numeric_alias(output_ty) {
          return Err(LlvmRuntimeError::ConfigError(format!(
            "LLVM requires explicit output type; numeric aliases are not supported (found '{}'). \
Use Int/i64 or Float/f64.",
            output_ty
          )));
        }
        if let Some(kind) = type_name_to_kind(output_ty) {
          return Ok(OutputKind::Scalar(kind));
        }
        return Err(LlvmRuntimeError::ConfigError(format!(
          "Unsupported output type '{}' for output node '{}' (morphism '{}') in module '{}'. \
Supported output types: Int/i64, Float/f64, Bool, String (limited), List/AttrSet (limited/opaque).",
          output_ty, node.name, morphism.name, fx_module.name
        )));
      } else {
        // Stage-2 morphism: outputs 포트 타입 검사
        if morphism.outputs.len() > 1 {
          let mut kinds = Vec::with_capacity(morphism.outputs.len());
          for output_port in &morphism.outputs {
            let output_ty = output_port.ty.trim();
            if output_ty.is_empty() {
              return Err(LlvmRuntimeError::ConfigError(format!(
                "Output port '{}' type is empty for output node '{}' (morphism '{}'). \
Supported output types: Int/i64, Float/f64, Bool, String (limited support).",
                output_port.name, node.name, morphism.name
              )));
            }
            if is_numeric_alias(output_ty) {
              return Err(LlvmRuntimeError::ConfigError(format!(
                "LLVM requires explicit output type; numeric aliases are not supported (found '{}' on port '{}'). \
Use Int/i64 or Float/f64.",
                output_ty, output_port.name
              )));
            }
            let kind = type_name_to_kind(output_ty).ok_or_else(|| {
              LlvmRuntimeError::ConfigError(format!(
                "Unsupported output port '{}' type '{}' for output node '{}' (morphism '{}') in module '{}'. \
Supported output types: Int/i64, Float/f64, Bool, String (limited), List/AttrSet (limited/opaque).",
                output_port.name, output_ty, node.name, morphism.name, fx_module.name
              ))
            })?;
            kinds.push(kind);
          }
          return Ok(OutputKind::Tuple(kinds));
        }
        let output_port = &morphism.outputs[0];
        let output_ty = output_port.ty.trim();
        if output_ty.is_empty() {
          return Err(LlvmRuntimeError::ConfigError(format!(
            "Output port '{}' type is empty for output node '{}' (morphism '{}'). \
Supported output types: Int/i64, Float/f64, Bool, String (limited support).",
            output_port.name, node.name, morphism.name
          )));
        }
        if is_numeric_alias(output_ty) {
          return Err(LlvmRuntimeError::ConfigError(format!(
            "LLVM requires explicit output type; numeric aliases are not supported (found '{}' on port '{}'). \
Use Int/i64 or Float/f64.",
            output_ty, output_port.name
          )));
        }
        if let Some(kind) = type_name_to_kind(output_ty) {
          return Ok(OutputKind::Scalar(kind));
        }
        return Err(LlvmRuntimeError::ConfigError(format!(
          "Unsupported output port '{}' type '{}' for output node '{}' (morphism '{}') in module '{}'. \
Supported output types: Int/i64, Float/f64, Bool, String (limited), List/AttrSet (limited/opaque).",
          output_port.name, output_ty, node.name, morphism.name, fx_module.name
        )));
      }
    }
  }

  Ok(OutputKind::Scalar(match default_numeric {
    NumericKind::Int => ValueKind::Int,
    NumericKind::Float => ValueKind::Float,
  }))
}

/// Lower FxCoreModule to LLVM IR (shared between JIT and AOT)
///
/// This function generates LLVM IR from a FxCoreModule, creating a main-like function
/// (named by `main_symbol`) and implementing the computation graph.
#[cfg(feature = "llvm")]
fn lower_fxcore_to_llvm_module<'ctx>(
  context: &'ctx inkwell::context::Context,
  module: &inkwell::module::Module<'ctx>,
  fx_module: &pnix_core::core::FxCoreModule,
  numeric_kind: NumericKind,
  output_kind: OutputKind,
  main_symbol: &str,
) -> RuntimeResult<()> {
  use inkwell::intrinsics::Intrinsic;
  use inkwell::types::BasicType;
  use inkwell::values::BasicValue;
  use inkwell::AddressSpace;
  use inkwell::{FloatPredicate, IntPredicate};
  use pnix_core::core::{EdgeCond, NodeKind};
  use std::collections::{HashMap, HashSet};

  fn sanitize_llvm_symbol(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
      if ch.is_ascii_alphanumeric() || ch == '_' {
        out.push(ch);
      } else {
        out.push('_');
      }
    }
    if out.is_empty() {
      out.push_str("module");
    }
    out
  }

  // Y13a-4: 모듈별 문자열 리터럴 ID (모듈 내부 이름 충돌 방지)
  let module_prefix = sanitize_llvm_symbol(&fx_module.name);
  let mut string_literal_counter: usize = 0;
  let mut next_string_literal_id = || {
    let id = string_literal_counter;
    string_literal_counter += 1;
    id
  };

  #[derive(Clone, Copy)]
  enum LlvmValue<'a> {
    Int(inkwell::values::IntValue<'a>),
    Float(inkwell::values::FloatValue<'a>),
    Bool(inkwell::values::IntValue<'a>),
    String(inkwell::values::PointerValue<'a>), // i8* pointer
    List(inkwell::values::PointerValue<'a>),   // i8* pointer (JSON-encoded list)
    AttrSet(inkwell::values::PointerValue<'a>), // i8* pointer (JSON-encoded object)
  }

  enum NodeValue<'a> {
    Single(LlvmValue<'a>),
    Multi(Vec<LlvmValue<'a>>),
  }

  impl<'a> NodeValue<'a> {
    fn as_single(&self) -> Result<LlvmValue<'a>, LlvmRuntimeError> {
      match self {
        NodeValue::Single(val) => Ok(*val),
        NodeValue::Multi(values) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected single output, got {} outputs",
          values.len()
        ))),
      }
    }

    fn output_at(&self, idx: usize) -> Result<LlvmValue<'a>, LlvmRuntimeError> {
      match self {
        NodeValue::Single(val) => {
          if idx == 0 {
            Ok(*val)
          } else {
            Err(LlvmRuntimeError::ConfigError(
              "output index out of range".to_string(),
            ))
          }
        }
        NodeValue::Multi(values) => values
          .get(idx)
          .copied()
          .ok_or_else(|| LlvmRuntimeError::ConfigError("output index out of range".to_string())),
      }
    }
  }

  impl<'a> LlvmValue<'a> {
    fn kind(self) -> ValueKind {
      match self {
        Self::Int(_) => ValueKind::Int,
        Self::Float(_) => ValueKind::Float,
        Self::Bool(_) => ValueKind::Bool,
        Self::String(_) => ValueKind::String,
        Self::List(_) => ValueKind::List,
        Self::AttrSet(_) => ValueKind::AttrSet,
      }
    }

    fn as_int(self) -> Result<inkwell::values::IntValue<'a>, LlvmRuntimeError> {
      match self {
        Self::Int(v) => Ok(v),
        Self::Bool(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Int value, got Bool (i1)"
        ))),
        Self::Float(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Int value, got Float"
        ))),
        Self::String(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Int value, got String"
        ))),
        Self::List(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Int value, got List"
        ))),
        Self::AttrSet(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Int value, got AttrSet"
        ))),
      }
    }

    fn as_float(self) -> Result<inkwell::values::FloatValue<'a>, LlvmRuntimeError> {
      match self {
        Self::Float(v) => Ok(v),
        Self::Int(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Float value, got Int"
        ))),
        Self::Bool(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Float value, got Bool"
        ))),
        Self::String(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Float value, got String"
        ))),
        Self::List(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Float value, got List"
        ))),
        Self::AttrSet(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Float value, got AttrSet"
        ))),
      }
    }

    fn as_bool(self) -> Result<inkwell::values::IntValue<'a>, LlvmRuntimeError> {
      match self {
        Self::Bool(v) => Ok(v),
        Self::Int(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Bool value, got Int"
        ))),
        Self::Float(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Bool value, got Float"
        ))),
        Self::String(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Bool value, got String"
        ))),
        Self::List(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Bool value, got List"
        ))),
        Self::AttrSet(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected Bool value, got AttrSet"
        ))),
      }
    }

    fn as_string(self) -> Result<inkwell::values::PointerValue<'a>, LlvmRuntimeError> {
      match self {
        Self::String(v) => Ok(v),
        Self::Int(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected String value, got Int"
        ))),
        Self::Float(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected String value, got Float"
        ))),
        Self::Bool(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected String value, got Bool"
        ))),
        Self::List(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected String value, got List"
        ))),
        Self::AttrSet(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected String value, got AttrSet"
        ))),
      }
    }

    fn as_list(self) -> Result<inkwell::values::PointerValue<'a>, LlvmRuntimeError> {
      match self {
        Self::List(v) => Ok(v),
        Self::String(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected List value, got String"
        ))),
        Self::Int(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected List value, got Int"
        ))),
        Self::Float(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected List value, got Float"
        ))),
        Self::Bool(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected List value, got Bool"
        ))),
        Self::AttrSet(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected List value, got AttrSet"
        ))),
      }
    }

    fn as_attrset(self) -> Result<inkwell::values::PointerValue<'a>, LlvmRuntimeError> {
      match self {
        Self::AttrSet(v) => Ok(v),
        Self::String(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected AttrSet value, got String"
        ))),
        Self::Int(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected AttrSet value, got Int"
        ))),
        Self::Float(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected AttrSet value, got Float"
        ))),
        Self::Bool(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected AttrSet value, got Bool"
        ))),
        Self::List(_) => Err(LlvmRuntimeError::ConfigError(format!(
          "expected AttrSet value, got List"
        ))),
      }
    }
  }

  fn default_value_for_kind<'ctx>(
    kind: ValueKind,
    i64_type: inkwell::types::IntType<'ctx>,
    f64_type: inkwell::types::FloatType<'ctx>,
    i1_type: inkwell::types::IntType<'ctx>,
    i8_ptr_type: inkwell::types::PointerType<'ctx>,
  ) -> LlvmValue<'ctx> {
    match kind {
      ValueKind::Int => LlvmValue::Int(i64_type.const_int(0, false)),
      ValueKind::Float => LlvmValue::Float(f64_type.const_float(0.0)),
      ValueKind::Bool => LlvmValue::Bool(i1_type.const_int(0, false)),
      ValueKind::String => LlvmValue::String(i8_ptr_type.const_null()),
      ValueKind::List => LlvmValue::List(i8_ptr_type.const_null()),
      ValueKind::AttrSet => LlvmValue::AttrSet(i8_ptr_type.const_null()),
    }
  }

  fn select_value<'ctx>(
    builder: &inkwell::builder::Builder<'ctx>,
    cond: inkwell::values::IntValue<'ctx>,
    on_true: LlvmValue<'ctx>,
    on_false: LlvmValue<'ctx>,
    label: &str,
  ) -> Result<LlvmValue<'ctx>, LlvmRuntimeError> {
    match (on_true, on_false) {
      (LlvmValue::Int(t), LlvmValue::Int(f)) => Ok(LlvmValue::Int(
        b!(builder.build_select(cond, t, f, label)).into_int_value(),
      )),
      (LlvmValue::Float(t), LlvmValue::Float(f)) => Ok(LlvmValue::Float(
        b!(builder.build_select(cond, t, f, label)).into_float_value(),
      )),
      (LlvmValue::Bool(t), LlvmValue::Bool(f)) => Ok(LlvmValue::Bool(
        b!(builder.build_select(cond, t, f, label)).into_int_value(),
      )),
      (LlvmValue::String(t), LlvmValue::String(f)) => Ok(LlvmValue::String(
        b!(builder.build_select(cond, t, f, label)).into_pointer_value(),
      )),
      (LlvmValue::List(t), LlvmValue::List(f)) => Ok(LlvmValue::List(
        b!(builder.build_select(cond, t, f, label)).into_pointer_value(),
      )),
      (LlvmValue::AttrSet(t), LlvmValue::AttrSet(f)) => Ok(LlvmValue::AttrSet(
        b!(builder.build_select(cond, t, f, label)).into_pointer_value(),
      )),
      (t, f) => Err(LlvmRuntimeError::ConfigError(format!(
        "conditional edge type mismatch: {:?} vs {:?}",
        t.kind(),
        f.kind()
      ))),
    }
  }

  fn build_float_intrinsic<'a>(
    module: &inkwell::module::Module<'a>,
    builder: &inkwell::builder::Builder<'a>,
    name: &str,
    arg: inkwell::values::FloatValue<'a>,
    label: &str,
  ) -> Result<inkwell::values::FloatValue<'a>, LlvmRuntimeError> {
    let intrinsic = Intrinsic::find(name).ok_or_else(|| {
      LlvmRuntimeError::ConfigError(format!("LLVM intrinsic '{}' not found", name))
    })?;
    let decl = intrinsic
      .get_declaration(module, &[arg.get_type().into()])
      .ok_or_else(|| {
        LlvmRuntimeError::ConfigError(format!("LLVM intrinsic '{}' declaration missing", name))
      })?;
    let call = b!(builder.build_call(decl, &[arg.into()], label));
    let value = match call.try_as_basic_value() {
      inkwell::values::ValueKind::Basic(value) => value,
      _ => {
        return Err(LlvmRuntimeError::ConfigError(format!(
          "LLVM intrinsic '{}' returned void",
          name
        )))
      }
    };
    Ok(value.into_float_value())
  }

  fn build_int_overflow_intrinsic<'a>(
    module: &inkwell::module::Module<'a>,
    builder: &inkwell::builder::Builder<'a>,
    name: &str,
    lhs: inkwell::values::IntValue<'a>,
    rhs: inkwell::values::IntValue<'a>,
    label: &str,
  ) -> Result<(inkwell::values::IntValue<'a>, inkwell::values::IntValue<'a>), LlvmRuntimeError> {
    let intrinsic = Intrinsic::find(name).ok_or_else(|| {
      LlvmRuntimeError::ConfigError(format!("LLVM intrinsic '{}' not found", name))
    })?;
    let decl = intrinsic
      .get_declaration(module, &[lhs.get_type().into()])
      .ok_or_else(|| {
        LlvmRuntimeError::ConfigError(format!("LLVM intrinsic '{}' declaration missing", name))
      })?;
    let call = b!(builder.build_call(decl, &[lhs.into(), rhs.into()], label));
    let value = match call.try_as_basic_value() {
      inkwell::values::ValueKind::Basic(value) => value,
      _ => {
        return Err(LlvmRuntimeError::ConfigError(format!(
          "LLVM intrinsic '{}' returned void",
          name
        )))
      }
    };
    let struct_val = value.into_struct_value();
    let value_label = format!("{}_value", label);
    let overflow_label = format!("{}_overflow", label);
    let result = b!(builder.build_extract_value(struct_val, 0, &value_label)).into_int_value();
    let overflow = b!(builder.build_extract_value(struct_val, 1, &overflow_label)).into_int_value();
    Ok((result, overflow))
  }

  let builder = context.create_builder();
  let i32_type = context.i32_type();
  let i64_type = context.i64_type();
  let f64_type = context.f64_type();
  let i1_type = context.bool_type();
  let i8_type = context.i8_type();
  let void_type = context.void_type();
  let i8_ptr_type = i8_type.ptr_type(AddressSpace::default());
  let runtime_error_code = module
    .get_global("pnix_runtime_error_code")
    .unwrap_or_else(|| {
      let global = module.add_global(i32_type, None, "pnix_runtime_error_code");
      global.set_linkage(inkwell::module::Linkage::Internal);
      global.set_initializer(&i32_type.const_int(0, false));
      global
    });
  let set_runtime_error = |condition: inkwell::values::IntValue<'_>,
                           code: u32,
                           label: &str|
   -> Result<(), LlvmRuntimeError> {
    let current_error_val = b!(builder.build_load(
      i32_type,
      runtime_error_code.as_pointer_value(),
      "runtime_error_code"
    ));
    if let Some(inst) = current_error_val.as_instruction_value() {
      let _ = inst.set_volatile(true);
    }
    let current_error = current_error_val.into_int_value();
    let new_error = i32_type.const_int(code as u64, false);
    let merged_error =
      b!(builder.build_select(condition, new_error, current_error, label)).into_int_value();
    let error_store = b!(builder.build_store(runtime_error_code.as_pointer_value(), merged_error));
    let _ = error_store.set_volatile(true);
    Ok(())
  };
  let helper_builder = context.create_builder();
  let reset_fn = module.add_function(
    "pnix_runtime_reset_error_state",
    void_type.fn_type(&[], false),
    None,
  );
  let reset_block = context.append_basic_block(reset_fn, "entry");
  helper_builder.position_at_end(reset_block);
  let reset_store = b!(helper_builder.build_store(
    runtime_error_code.as_pointer_value(),
    i32_type.const_int(0, false),
  ));
  let _ = reset_store.set_volatile(true);
  b!(helper_builder.build_return(None));

  let get_error_fn = module.add_function(
    "pnix_runtime_get_error_code",
    i32_type.fn_type(&[], false),
    None,
  );
  let get_error_block = context.append_basic_block(get_error_fn, "entry");
  helper_builder.position_at_end(get_error_block);
  let error_code_val = b!(helper_builder.build_load(
    i32_type,
    runtime_error_code.as_pointer_value(),
    "error_code"
  ));
  if let Some(inst) = error_code_val.as_instruction_value() {
    let _ = inst.set_volatile(true);
  }
  b!(helper_builder.build_return(Some(&error_code_val)));

  let free_fn_type = void_type.fn_type(&[i8_ptr_type.into()], false);
  let free_fn = module.add_function("free", free_fn_type, None);
  free_fn.set_linkage(inkwell::module::Linkage::External);

  let free_string_fn = module.add_function(
    "pnix_runtime_free_string",
    void_type.fn_type(&[i8_ptr_type.into()], false),
    None,
  );
  let free_entry = context.append_basic_block(free_string_fn, "entry");
  let free_do = context.append_basic_block(free_string_fn, "do_free");
  let free_done = context.append_basic_block(free_string_fn, "done");
  helper_builder.position_at_end(free_entry);
  let free_arg = free_string_fn
    .get_first_param()
    .ok_or_else(|| LlvmRuntimeError::ConfigError("free arg missing".to_string()))?
    .into_pointer_value();
  let free_is_null = b!(helper_builder.build_is_null(free_arg, "free_is_null"));
  b!(helper_builder.build_conditional_branch(free_is_null, free_done, free_do));
  helper_builder.position_at_end(free_do);
  b!(helper_builder.build_call(free_fn, &[free_arg.into()], "free_call"));
  b!(helper_builder.build_unconditional_branch(free_done));
  helper_builder.position_at_end(free_done);
  b!(helper_builder.build_return(None));
  // Check if module uses pointer-like types (String/List/AttrSet)
  let has_ptr_type = fx_module.inputs.iter().any(|i| {
    type_name_to_kind(&i.ty)
      .map(ValueKind::is_ptr)
      .unwrap_or(false)
  }) || fx_module.morphisms.iter().any(|m| {
    type_name_to_kind(&m.input)
      .map(ValueKind::is_ptr)
      .unwrap_or(false)
      || type_name_to_kind(&m.output)
        .map(ValueKind::is_ptr)
        .unwrap_or(false)
  });

  // Y05c-3: Mixed input types (pointer + numeric) 금지
  let has_ptr_input = fx_module.inputs.iter().any(|i| {
    type_name_to_kind(&i.ty)
      .map(ValueKind::is_ptr)
      .unwrap_or(false)
  });
  let has_numeric_input = fx_module.inputs.iter().any(|i| {
    let kind = type_name_to_kind(&i.ty);
    kind == Some(ValueKind::Int) || kind == Some(ValueKind::Float) || kind == Some(ValueKind::Bool)
  });

  if has_ptr_input && has_numeric_input {
    return Err(
      LlvmRuntimeError::ConfigError(format!(
        "Mixed input types (pointer + numeric) are not supported in LLVM runtime. \
        Module '{}' has both pointer (String/List/AttrSet) and numeric (Int/Float/Bool) inputs. \
        Please use separate modules for pointer-only and numeric-only operations, \
        or use from_input literals instead of entry inputs. \
        See docs/llvm-policy.md for ABI/memory model details.",
        fx_module.name
      ))
      .into(),
    );
  }

  if has_ptr_type {
    // Pointer-like type support is limited: String/List/AttrSet values are opaque JSON pointers.
    // For now, we only support pointer literals (String) in from_input and passing pointer values through extern calls.
    // ABI/Memory model: See docs/llvm-policy.md for ownership and lifetime rules.
  }

  // Validate input types against numeric kind (or pointer types).
  for input in &fx_module.inputs {
    let kind = type_name_to_kind(&input.ty);
    let ok = matches!(
      (numeric_kind, kind),
      (NumericKind::Int, Some(ValueKind::Int))
        | (NumericKind::Int, Some(ValueKind::Bool))
        | (NumericKind::Float, Some(ValueKind::Float))
    ) || kind.map(ValueKind::is_ptr).unwrap_or(false);
    if !ok {
      return Err(LlvmRuntimeError::ConfigError(format!(
        "Unsupported input type '{}' for input '{}'. Supported: Int/i64 (Bool allowed as 0/1), Float/f64 (single numeric kind only), or String/List/AttrSet (limited/opaque).",
        input.ty,
        input.name
      ))
      .into());
    }
  }

  // Determine input parameter types (pointer-only or numeric-only)
  // Y13a-7: Bool 입력 타입 보존 - Bool 입력은 i1로 전달
  let input_params: Vec<_> = fx_module
    .inputs
    .iter()
    .map(|input| {
      match type_name_to_kind(&input.ty) {
        Some(ValueKind::String) | Some(ValueKind::List) | Some(ValueKind::AttrSet) => {
          i8_ptr_type.into()
        }
        Some(ValueKind::Bool) => i1_type.into(), // Bool 입력은 i1로 전달
        _ => match numeric_kind {
          NumericKind::Int => i64_type.into(),
          NumericKind::Float => f64_type.into(),
        },
      }
    })
    .collect();

  let i8_type = context.i8_type();
  let i8_ptr_type = i8_type.ptr_type(AddressSpace::default());
  let output_field_type = |kind: ValueKind| -> inkwell::types::BasicTypeEnum<'ctx> {
    match kind {
      ValueKind::Int => i64_type.into(),
      ValueKind::Float => f64_type.into(),
      ValueKind::Bool => i32_type.into(), // Bool 출력은 i32로 유지 (ABI 호환)
      ValueKind::String | ValueKind::List | ValueKind::AttrSet => i8_ptr_type.into(),
    }
  };
  let tuple_struct_type = match &output_kind {
    OutputKind::Tuple(kinds) => {
      let field_types: Vec<inkwell::types::BasicTypeEnum> =
        kinds.iter().copied().map(output_field_type).collect();
      Some(context.struct_type(&field_types, false))
    }
    OutputKind::Scalar(_) => None,
  };
  let fn_type = match &output_kind {
    OutputKind::Scalar(kind) => match kind {
      ValueKind::Int => i64_type.fn_type(&input_params, false),
      ValueKind::Float => f64_type.fn_type(&input_params, false),
      ValueKind::Bool => i32_type.fn_type(&input_params, false),
      ValueKind::String => i8_ptr_type.fn_type(&input_params, false),
      ValueKind::List | ValueKind::AttrSet => {
        // List/AttrSet는 opaque JSON 포인터로 처리 (향후 구조체 포인터로 변경 가능)
        i8_ptr_type.fn_type(&input_params, false)
      }
    },
    OutputKind::Tuple(_) => tuple_struct_type
      .as_ref()
      .ok_or_else(|| LlvmRuntimeError::ConfigError("missing tuple output type".to_string()))?
      .fn_type(&input_params, false),
  };
  let function = module.add_function(main_symbol, fn_type, None);
  if main_symbol != "main" {
    function.set_linkage(inkwell::module::Linkage::Internal);
  }

  for (idx, input) in fx_module.inputs.iter().enumerate() {
    if let Some(param) = function.get_nth_param(idx as u32) {
      param.set_name(&input.name);
    }
  }

  let basic_block = context.append_basic_block(function, "entry");
  builder.position_at_end(basic_block);

  let mut input_values_map: HashMap<String, LlvmValue> = HashMap::new();
  for (idx, input) in fx_module.inputs.iter().enumerate() {
    if let Some(param) = function.get_nth_param(idx as u32) {
      let value = match type_name_to_kind(&input.ty) {
        Some(ValueKind::String) => LlvmValue::String(param.into_pointer_value()),
        Some(ValueKind::List) => LlvmValue::List(param.into_pointer_value()),
        Some(ValueKind::AttrSet) => LlvmValue::AttrSet(param.into_pointer_value()),
        Some(ValueKind::Bool) => {
          // Y13a-7: Bool 입력 타입 보존 - i1 파라미터를 LlvmValue::Bool로 변환
          LlvmValue::Bool(param.into_int_value())
        }
        _ => match numeric_kind {
          NumericKind::Int => LlvmValue::Int(param.into_int_value()),
          NumericKind::Float => LlvmValue::Float(param.into_float_value()),
        },
      };
      input_values_map.insert(input.name.clone(), value);
    }
  }

  // Y13a-2: 노드 평가 순서 - 위상 정렬 사용
  use pnix_core::passes::dep_analysis::analyze_dependencies;
  let dep_analysis = analyze_dependencies(fx_module)
    .map_err(|e| LlvmRuntimeError::ConfigError(format!("Dependency analysis failed: {}", e)))?;

  // 위상 정렬된 노드 순서 사용 (노드만 필터링, 입력 제외)
  let node_names: HashSet<&str> = fx_module.nodes.iter().map(|n| n.name.as_str()).collect();
  let node_order: Vec<String> = dep_analysis
    .topo_order
    .into_iter()
    .filter(|name| node_names.contains(name.as_str()))
    .collect();

  let node_map: HashMap<&str, &pnix_core::core::FxNode> = fx_module
    .nodes
    .iter()
    .map(|n| (n.name.as_str(), n))
    .collect();
  let morphism_map: HashMap<&str, &pnix_core::core::FxMorphism> = fx_module
    .morphisms
    .iter()
    .map(|m| (m.name.as_str(), m))
    .collect();

  let mut node_values: HashMap<String, NodeValue> = HashMap::new();
  let mut gate_results: HashMap<String, inkwell::values::IntValue<'ctx>> = HashMap::new();
  let mut node_failed: HashMap<String, inkwell::values::IntValue<'ctx>> = HashMap::new();

  let load_runtime_error =
    |label: &str| -> Result<inkwell::values::IntValue<'ctx>, LlvmRuntimeError> {
      let current_error_val =
        b!(builder.build_load(i32_type, runtime_error_code.as_pointer_value(), label));
      if let Some(inst) = current_error_val.as_instruction_value() {
        let _ = inst.set_volatile(true);
      }
      Ok(current_error_val.into_int_value())
    };

  // 노드를 위상 정렬 순서로 평가
  for to_node in &node_order {
    let node = *node_map
      .get(to_node.as_str())
      .ok_or_else(|| LlvmRuntimeError::ConfigError(format!("Node not found: {}", to_node)))?;

    let morphism_name = &node.uses;
    // Extract base operation name for matching:
    // "builtins.String.concat" -> "concat"
    // "String.concat" -> "concat"
    // "concat" -> "concat"
    let op_name = morphism_name
      .as_str()
      .strip_prefix("builtins.")
      .unwrap_or(morphism_name.as_str());
    let op_name = op_name.strip_prefix("String.").unwrap_or(op_name);
    let morphism = *morphism_map.get(morphism_name.as_str()).ok_or_else(|| {
      LlvmRuntimeError::ConfigError(format!("Morphism not found: {}", morphism_name))
    })?;

    // Y05a-1: Type signature consistency check
    // mod operation should only accept Int types (spec: "Int → Int → Int")
    if op_name == "mod" || op_name == "%" || op_name == "modulo" {
      // 복합 타입 시그니처도 감지하도록 type_name_to_kind 사용
      let input_kind = type_name_to_kind(&morphism.input);
      let output_kind = type_name_to_kind(&morphism.output);
      if input_kind == Some(ValueKind::Float) || output_kind == Some(ValueKind::Float) {
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Mod operation requires Int types, but morphism '{}' has input='{}' output='{}'. \
                    Spec signature: Int → Int → Int",
            morphism_name, morphism.input, morphism.output
          ))
          .into(),
        );
      }
    }

    // Y13a-1: 입력 포트 순서 보장 - morphism.inputs 순서 기준으로 정렬
    let mut input_edges: Vec<_> = fx_module
      .edges
      .iter()
      .filter(|e| e.to == *to_node)
      .collect();

    // Stage-2 포트 기반 morphism인 경우 to_port 기준으로 정렬
    if !morphism.inputs.is_empty() {
      // Y13a-16: 기본 포트 매핑 - to_port가 없는 Stage-1 edge를 기본 포트(inputs[0])로 간주
      // morphism.inputs가 비어있지 않으므로 첫 번째 입력 포트를 기본 포트로 사용
      let default_port = &morphism.inputs[0].name;

      input_edges.sort_by(|a, b| {
        // to_port가 없으면 기본 포트로 간주
        let a_port = a
          .to_port
          .as_ref()
          .map(|p| p.as_str())
          .unwrap_or(default_port);
        let b_port = b
          .to_port
          .as_ref()
          .map(|p| p.as_str())
          .unwrap_or(default_port);

        // morphism.inputs에서 포트 인덱스 찾기
        let a_idx = morphism.inputs.iter().position(|p| p.name == a_port);
        let b_idx = morphism.inputs.iter().position(|p| p.name == b_port);

        match (a_idx, b_idx) {
          (Some(ai), Some(bi)) => ai.cmp(&bi),
          (Some(_), None) => std::cmp::Ordering::Less,
          (None, Some(_)) => std::cmp::Ordering::Greater,
          (None, None) => std::cmp::Ordering::Equal,
        }
      });
    } else {
      // Y13a-10: Stage-1 edge 순서 결정성 - 비가환 연산 순서 보장
      // Stage-1 morphism (포트 없음)에서 비가환 연산(sub/div 등)의 경우 순서가 중요
      // edges를 결정론적으로 정렬 (from 노드 이름 기준)
      let is_non_commutative = matches!(
        op_name,
        "sub" | "-" | "subtract" | "div" | "/" | "divide" | "mod" | "%" | "modulo" | "pow" | "**"
      );

      if is_non_commutative && input_edges.len() > 1 {
        // 비가환 연산인 경우, from 노드 이름으로 정렬하여 결정론적 순서 보장
        input_edges.sort_by(|a, b| {
          // from_input이 있는 경우 우선순위 높임 (입력은 항상 먼저)
          match (&a.from_input, &b.from_input) {
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(a_input), Some(b_input)) => a_input.cmp(b_input),
            (None, None) => a.from.cmp(&b.from),
          }
        });
      } else {
        // 가환 연산이거나 단일 입력인 경우, 결정론적 순서를 위해 from 노드 이름으로 정렬
        input_edges.sort_by(|a, b| match (&a.from_input, &b.from_input) {
          (Some(_), None) => std::cmp::Ordering::Less,
          (None, Some(_)) => std::cmp::Ordering::Greater,
          (Some(a_input), Some(b_input)) => a_input.cmp(b_input),
          (None, None) => a.from.cmp(&b.from),
        });
      }
    }

    // Y13a-3: 입력 arity/포트 검증
    // morphism.inputs와 edges의 일치 여부 검증
    let mut port_edges: HashMap<&str, Vec<&pnix_core::core::FxEdge>> = HashMap::new();

    if !morphism.inputs.is_empty() {
      // Stage-2 포트 기반 morphism: 포트 이름으로 매칭
      // Y13a-16: 기본 포트 매핑 - to_port가 없는 Stage-1 edge를 기본 포트(inputs[0])로 간주
      // morphism.inputs가 비어있지 않으므로 첫 번째 입력 포트를 기본 포트로 사용
      let default_port = &morphism.inputs[0].name;

      for edge in &input_edges {
        // to_port가 없으면 기본 포트로 간주
        let port_name = edge
          .to_port
          .as_ref()
          .map(|p| p.as_str())
          .unwrap_or(default_port);
        port_edges
          .entry(port_name)
          .or_insert_with(Vec::new)
          .push(edge);
      }

      // 1. 누락된 포트 검증
      for port in &morphism.inputs {
        if !port_edges.contains_key(port.name.as_str()) {
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "Missing input port '{}' for morphism '{}' on node '{}'. \
               Required ports: [{}]",
              port.name,
              morphism.name,
              to_node,
              morphism
                .inputs
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
            ))
            .into(),
          );
        }
      }

      // 2. 중복된 포트 검증
      for (port_name, edges) in &port_edges {
        let unconditional = edges.iter().filter(|e| e.cond.is_none()).count();
        if unconditional > 1 {
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "Duplicate input port '{}' for morphism '{}' on node '{}'. \
               Found {} unconditional edges connecting to the same port: {:?}",
              port_name,
              morphism.name,
              to_node,
              unconditional,
              edges
                .iter()
                .filter(|e| e.cond.is_none())
                .map(|e| format!("{} -> {}", e.from, e.to))
                .collect::<Vec<_>>()
            ))
            .into(),
          );
        }
        if unconditional == 1 && edges.len() > 1 {
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "Input port '{}' for morphism '{}' on node '{}' mixes unconditional and conditional edges. \
               runtime-llvm requires conditional edges to be the only incoming edges for a port.",
              port_name, morphism.name, to_node
            ))
            .into(),
          );
        }
      }

      // 3. 여분/잘못된 포트 검증
      // Y13a-16: 기본 포트 매핑 - to_port가 없는 edge는 이미 기본 포트로 매핑되었으므로
      // 빈 문자열 체크는 불필요 (기본 포트로 매핑됨)
      for port_name in port_edges.keys() {
        if !morphism.inputs.iter().any(|p| p.name == *port_name) {
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "Unknown input port '{}' for morphism '{}' on node '{}'. \
               Valid ports: [{}]",
              port_name,
              morphism.name,
              to_node,
              morphism
                .inputs
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
            ))
            .into(),
          );
        }
      }
    } else {
      // Stage-1 호환: 포트가 없으면 edges 수만 확인
      // (현재는 특별한 arity 검증 없음, 필요시 추가 가능)
    }

    let requires_pure_inputs = matches!(op_name, "if" | "select" | "and" | "&&" | "or" | "||");
    if requires_pure_inputs {
      let mut non_pure_sources = Vec::new();
      for edge_input in &input_edges {
        if edge_input.from == "input" {
          continue;
        }
        let from_node = *node_map.get(edge_input.from.as_str()).ok_or_else(|| {
          LlvmRuntimeError::ConfigError(format!("Node not found: {}", edge_input.from))
        })?;
        let from_morphism = *morphism_map.get(from_node.uses.as_str()).ok_or_else(|| {
          LlvmRuntimeError::ConfigError(format!("Morphism not found: {}", from_node.uses))
        })?;
        let is_extern = from_morphism.name.starts_with("extern:");
        let is_pure = matches!(from_morphism.effect, pnix_core::core::Effect::Pure) && !is_extern;
        if !is_pure {
          non_pure_sources.push(edge_input.from.clone());
        }
      }
      if !non_pure_sources.is_empty() {
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Short-circuit op '{}' requires pure inputs, but got non-pure source(s): {}",
            op_name,
            non_pure_sources.join(", ")
          ))
          .into(),
        );
      }
    }

    let error_before = load_runtime_error("runtime_error_before")?;

    let mut resolve_edge_value =
      |edge_input: &pnix_core::core::FxEdge| -> RuntimeResult<LlvmValue<'ctx>> {
        if edge_input.from == "input" {
          if let Some(ref from_input) = edge_input.from_input {
            // Y05c-6: Try parsing as string literal first (if it starts and ends with quotes)
            // JSON string 파서를 사용하여 escape/UTF-8 처리 보장
            let string_literal = if from_input.starts_with('"')
              && from_input.ends_with('"')
              && from_input.len() >= 2
            {
              // JSON string 파서 사용 (escape 처리 및 UTF-8 보장)
              let parsed_string: Result<String, _> = serde_json::from_str(from_input);
              let content = match parsed_string {
                Ok(s) => s,
                Err(_e) => {
                  // JSON 파싱 실패 시 fallback: 수동 escape 처리
                  // Y05c-12: 문자열 NUL 정책 - NUL 문자 금지
                  // 수동 escape 처리에서도 \0을 허용하지 않음
                  let content = &from_input[1..from_input.len() - 1];
                  let mut bytes = Vec::new();
                  let mut chars = content.chars().peekable();
                  while let Some(ch) = chars.next() {
                    if ch == '\\' {
                      if let Some(next) = chars.next() {
                        match next {
                          'n' => bytes.push(b'\n'),
                          't' => bytes.push(b'\t'),
                          'r' => bytes.push(b'\r'),
                          '\\' => bytes.push(b'\\'),
                          '"' => bytes.push(b'"'),
                          '0' => {
                            // Y05c-12: NUL 문자 금지 - \0 escape 시퀀스를 에러로 처리
                            return Err(
                              LlvmRuntimeError::ConfigError(format!(
                                "String literal in from_input contains null byte escape sequence (\\0). \
                                 Null bytes are not allowed in runtime-llvm string literals. \
                                 Please remove \\0 from the string literal: {}",
                                from_input
                              ))
                              .into(),
                            );
                          }
                          _ => {
                            // 알 수 없는 escape 시퀀스는 그대로 유지
                            // non-ASCII 문자 처리: char::encode_utf8() 사용
                            bytes.push(b'\\');
                            let mut buf = [0u8; 4];
                            let encoded = next.encode_utf8(&mut buf);
                            bytes.extend_from_slice(encoded.as_bytes());
                          }
                        }
                      } else {
                        bytes.push(b'\\');
                      }
                    } else {
                      // UTF-8 문자를 바이트로 변환
                      let mut buf = [0u8; 4];
                      let encoded = ch.encode_utf8(&mut buf);
                      bytes.extend_from_slice(encoded.as_bytes());
                    }
                  }
                  let parsed_content =
                    String::from_utf8(bytes).unwrap_or_else(|_| content.to_string());
                  // Y05c-12: 파싱된 문자열에도 NUL 문자가 있는지 확인 (이중 체크)
                  if parsed_content.contains('\0') {
                    return Err(
                      LlvmRuntimeError::ConfigError(format!(
                        "String literal in from_input contains null byte (\\0). \
                         Null bytes are not allowed in runtime-llvm string literals. \
                         Please remove null bytes from the string literal: {}",
                        from_input
                      ))
                      .into(),
                    );
                  }
                  parsed_content
                }
              };

              // Y05c-12: 문자열 NUL 정책 - NUL 문자 금지
              // C 문자열과의 호환성을 위해 문자열 리터럴에 NUL 문자(\0)를 허용하지 않음
              // NUL 문자가 있으면 명시적 에러 반환
              if content.contains('\0') {
                return Err(
                  LlvmRuntimeError::ConfigError(format!(
                    "String literal in from_input contains null byte (\\0). \
                     Null bytes are not allowed in runtime-llvm string literals. \
                     Please remove null bytes from the string literal: {}",
                    from_input
                  ))
                  .into(),
                );
              }

              // Y05c-1: 실제 바이트 배열로 초기화
              // const_string을 사용하여 문자열 상수 생성 (null terminator 자동 추가)
              // const_string의 두 번째 인자가 true이면 null terminator를 자동으로 추가
              // LOW: 문자열 리터럴 UTF-8 검증 지연 수정 완료
              // const_string은 bytes를 받으므로 UTF-8 검증은 파서 단계에서 이미 완료됨
              // Pnix 파서가 문자열 리터럴을 파싱할 때 UTF-8 검증을 수행하므로, 여기서는 안전함
              let bytes = content.as_bytes();
              let str_const = context.const_string(bytes, true);
              // str_const는 ArrayValue를 반환하며, 타입은 [N x i8] (N = content_without_null.len() + 1)
              // str_const의 타입을 확인하고, 그것과 일치하는 배열 타입으로 global 생성
              let str_array_type = str_const.get_type();
              // Y13a-4: 전역 유니크 ID 사용 (노드별 재사용 방지)
              let unique_id = next_string_literal_id();
              let global_name = format!("{}_str_lit_{}", module_prefix, unique_id);
              let string_global = module.add_global(str_array_type, None, &global_name);

              string_global.set_linkage(inkwell::module::Linkage::Internal);
              string_global.set_constant(true);
              string_global.set_initializer(&str_const);

              // Get pointer to first element (i8*)
              let zero = i32_type.const_int(0, false);
              let indices = [zero, zero];
              let str_ptr = unsafe {
                b!(builder.build_gep(
                  str_array_type,
                  string_global.as_pointer_value(),
                  &indices,
                  &format!("{}_ptr", global_name),
                ))
              };
              Some(LlvmValue::String(str_ptr))
            } else {
              None
            };

            if let Some(literal) = string_literal {
              return Ok(literal);
            }

            // Y13a-19: pointer-only 입력에서 from_input 입력 이름 허용
            // 문자열 리터럴이 아니더라도 input param을 참조하도록 허용
            if has_ptr_input {
              // pointer-only 모듈에서 문자열 리터럴이 아닌 경우, 입력 파라미터 이름인지 확인
              if let Some(param_val) = input_values_map.get(from_input) {
                // 입력 파라미터를 찾았으면 사용
                return Ok(*param_val);
              }
              // 입력 파라미터도 아니면 에러
              return Err(
                LlvmRuntimeError::ConfigError(format!(
                  "Pointer-only module: '{}' is not a valid string literal (must be quoted) \
                  and is not a valid input parameter name. \
                  Use string literals like \"hello\" in from_input, or reference an input parameter.",
                  from_input
                ))
                .into(),
              );
            }

            // Y13a-15: Bool 리터럴 타입 정책 - Bool-only 조건(if/and/or)에서 사용 가능하도록 항상 Bool로 유지
            // numeric_kind가 Int여도 Bool 리터럴은 Bool 타입으로 유지하여 Bool-only 조건과 호환
            // Int가 필요한 경우 명시적 캐스트(eq/ne 등)를 사용하거나, Int 리터럴(0/1)을 사용
            let bool_literal = match from_input.as_str() {
              "true" => Some(LlvmValue::Bool(i1_type.const_int(1, false))),
              "false" => Some(LlvmValue::Bool(i1_type.const_int(0, false))),
              _ => None,
            };

            let literal = if let Some(bool_val) = bool_literal {
              Some(bool_val)
            } else {
              // Try numeric literal
              match numeric_kind {
                NumericKind::Int => from_input
                  .parse::<i64>()
                  .ok()
                  .map(|v| LlvmValue::Int(i64_type.const_int(v as u64, false))),
                NumericKind::Float => from_input
                  .parse::<f64>()
                  .ok()
                  .map(|v| LlvmValue::Float(f64_type.const_float(v))),
              }
            };

            if let Some(literal) = literal {
              return Ok(literal);
            }
            if let Some(param_val) = input_values_map.get(from_input) {
              return Ok(*param_val);
            }
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Input '{}' not found in function parameters",
                from_input
              ))
              .into(),
            );
          }
          return Err(
            LlvmRuntimeError::ConfigError(
              "Edge from 'input' but no from_input specified".to_string(),
            )
            .into(),
          );
        }

        // Y13a-20: 출력 포트(from_port) 처리 - 단일 출력 morphism에서 기본 포트 허용
        // from_port가 있는 경우, 해당 노드의 morphism 정보를 확인하여 검증
        if let Some(from_port) = &edge_input.from_port {
          // from 노드의 morphism 정보 확인
          let from_node = fx_module
            .nodes
            .iter()
            .find(|n| n.name == edge_input.from)
            .ok_or_else(|| {
              LlvmRuntimeError::ConfigError(format!("Node not found: {}", edge_input.from))
            })?;

          let from_morphism_name = &from_node.uses;
          let from_morphism = fx_module
            .morphisms
            .iter()
            .find(|m| m.name == *from_morphism_name)
            .ok_or_else(|| {
              LlvmRuntimeError::ConfigError(format!("Morphism not found: {}", from_morphism_name))
            })?;

          if from_morphism.outputs.len() == 1 {
            // 기본 포트는 "out" 또는 outputs[0].name
            let default_port = if from_morphism.outputs[0].name == "out" {
              "out"
            } else {
              &from_morphism.outputs[0].name
            };

            if from_port == default_port || from_port == "out" {
              // 기본 포트이면 허용
              if let Some(val) = node_values.get(&edge_input.from) {
                return val.as_single().map_err(|e| e.into());
              }
              return Err(
                LlvmRuntimeError::ConfigError(format!(
                  "Node '{}' value not found (dependency order issue?)",
                  edge_input.from
                ))
                .into(),
              );
            }
            // 기본 포트가 아니면 에러
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Edge from '{}' to '{}' references output port '{}', but morphism '{}' on node '{}' \
                 only has one output port '{}'. Use the default port '{}' or omit from_port.",
                edge_input.from,
                edge_input.to,
                from_port,
                from_morphism_name,
                edge_input.from,
                default_port,
                default_port
              ))
              .into(),
            );
          }

          // 다중 출력 morphism에서 from_port 매핑
          let output_idx = from_morphism
            .outputs
            .iter()
            .position(|p| p.name == *from_port)
            .ok_or_else(|| {
              LlvmRuntimeError::ConfigError(format!(
                "Edge from '{}' to '{}' references output port '{}', but morphism '{}' on node '{}' \
                 does not define that output port. Available ports: [{}]",
                edge_input.from,
                edge_input.to,
                from_port,
                from_morphism_name,
                edge_input.from,
                from_morphism
                  .outputs
                  .iter()
                  .map(|p| p.name.as_str())
                  .collect::<Vec<_>>()
                  .join(", ")
              ))
            })?;
          if let Some(val) = node_values.get(&edge_input.from) {
            return val.output_at(output_idx).map_err(|e| e.into());
          }
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "Node '{}' value not found (dependency order issue?)",
              edge_input.from
            ))
            .into(),
          );
        }

        // from_port가 없는 경우, 노드의 기본 출력 사용
        if let Some(val) = node_values.get(&edge_input.from) {
          return val.as_single().map_err(|e| e.into());
        }
        Err(
          LlvmRuntimeError::ConfigError(format!(
            "Node '{}' value not found (dependency order issue?)",
            edge_input.from
          ))
          .into(),
        )
      };

    let resolve_gate_bool = |gate_name: &str| -> RuntimeResult<inkwell::values::IntValue<'ctx>> {
      if let Some(val) = gate_results.get(gate_name) {
        return Ok(*val);
      }
      if let Some(val) = input_values_map.get(gate_name) {
        if val.kind() == ValueKind::Bool {
          return Ok(val.as_bool()?);
        }
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Gate '{}' must be Bool input, got {:?}",
            gate_name,
            val.kind()
          ))
          .into(),
        );
      }
      if let Some(val) = node_values.get(gate_name) {
        let single = val.as_single().map_err(RuntimeError::from)?;
        if single.kind() == ValueKind::Bool {
          return Ok(single.as_bool()?);
        }
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Gate '{}' must be Bool output, got {:?}",
            gate_name,
            single.kind()
          ))
          .into(),
        );
      }
      Err(
        LlvmRuntimeError::ConfigError(format!(
          "Gate '{}' not executed yet (missing dependency?)",
          gate_name
        ))
        .into(),
      )
    };

    let resolve_edge_cond = |cond: &EdgeCond| -> RuntimeResult<inkwell::values::IntValue<'ctx>> {
      match cond {
        EdgeCond::When(gate) => resolve_gate_bool(gate),
        EdgeCond::Unless(gate) => {
          let gate_val = resolve_gate_bool(gate)?;
          Ok(b!(builder.build_not(gate_val, "edge_unless")))
        }
        EdgeCond::OnFail(node_name) => node_failed.get(node_name).copied().ok_or_else(|| {
          LlvmRuntimeError::ConfigError(format!(
            "OnFail condition references node '{}' which has not executed yet",
            node_name
          ))
          .into()
        }),
        EdgeCond::WhenUnless { when, unless } => {
          let when_val = resolve_gate_bool(when)?;
          let unless_val = resolve_gate_bool(unless)?;
          let not_unless = b!(builder.build_not(unless_val, "edge_when_unless_not"));
          Ok(b!(builder.build_and(
            when_val,
            not_unless,
            "edge_when_unless"
          )))
        }
        EdgeCond::AllWhen(gates) => {
          if gates.is_empty() {
            return Err(
              LlvmRuntimeError::ConfigError(
                "all_when edge condition requires at least one gate".to_string(),
              )
              .into(),
            );
          }
          let mut acc = i1_type.const_int(1, false);
          for gate in gates {
            let gate_val = resolve_gate_bool(gate)?;
            acc = b!(builder.build_and(acc, gate_val, "edge_all_when"));
          }
          Ok(acc)
        }
        EdgeCond::AllUnless(gates) => {
          if gates.is_empty() {
            return Err(
              LlvmRuntimeError::ConfigError(
                "all_unless edge condition requires at least one gate".to_string(),
              )
              .into(),
            );
          }
          let mut acc = i1_type.const_int(1, false);
          for gate in gates {
            let gate_val = resolve_gate_bool(gate)?;
            let not_gate = b!(builder.build_not(gate_val, "edge_all_unless_not"));
            acc = b!(builder.build_and(acc, not_gate, "edge_all_unless"));
          }
          Ok(acc)
        }
        EdgeCond::Unknown => {
          Err(LlvmRuntimeError::ConfigError("unknown edge condition".to_string()).into())
        }
      }
    };

    let mut input_vals = Vec::new();

    if morphism.inputs.is_empty() {
      // Stage-1: conditional edges are ambiguous without ports
      if input_edges.iter().any(|e| e.cond.is_some()) {
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Conditional edges require ported morphisms (Stage-2). \
             Node '{}' uses Stage-1 morphism '{}' with conditional edges, which is unsupported in runtime-llvm.",
            to_node, morphism.name
          ))
          .into(),
        );
      }
      for edge_input in &input_edges {
        let value = resolve_edge_value(edge_input)?;
        input_vals.push(value);
      }
    } else {
      for port in &morphism.inputs {
        let edges = port_edges.get(port.name.as_str()).ok_or_else(|| {
          LlvmRuntimeError::ConfigError(format!(
            "Missing input port '{}' for morphism '{}' on node '{}'",
            port.name, morphism.name, to_node
          ))
        })?;

        let port_kind = type_name_to_kind(&port.ty).ok_or_else(|| {
          LlvmRuntimeError::ConfigError(format!(
            "Unsupported input port '{}' type '{}' for morphism '{}' on node '{}'",
            port.name, port.ty, morphism.name, to_node
          ))
        })?;

        let mut selected =
          default_value_for_kind(port_kind, i64_type, f64_type, i1_type, i8_ptr_type);
        let mut active_any = i1_type.const_int(0, false);
        let mut active_multi = i1_type.const_int(0, false);

        for edge_input in edges {
          let edge_value = resolve_edge_value(edge_input)?;
          let edge_active = match &edge_input.cond {
            Some(cond) => resolve_edge_cond(cond)?,
            None => i1_type.const_int(1, false),
          };

          let active_prev = active_any;
          let multi = b!(builder.build_and(active_prev, edge_active, "cond_edge_multi"));
          active_multi = b!(builder.build_or(active_multi, multi, "cond_edge_multi_acc"));

          let first_cond = b!(builder.build_not(active_prev, "cond_edge_first"));
          let first_select =
            b!(builder.build_and(first_cond, edge_active, "cond_edge_select_cond"));
          selected = select_value(
            &builder,
            first_select,
            edge_value,
            selected,
            "cond_edge_select",
          )?;
          active_any = b!(builder.build_or(active_any, edge_active, "cond_edge_any"));
        }

        let has_conditional = edges.iter().any(|e| e.cond.is_some());
        if has_conditional {
          let missing = b!(builder.build_not(active_any, "cond_edge_missing"));
          set_runtime_error(
            missing,
            RUNTIME_ERROR_COND_MISSING_INPUT,
            "cond_edge_missing_input",
          )?;
          set_runtime_error(
            active_multi,
            RUNTIME_ERROR_COND_DUP_INPUT,
            "cond_edge_dup_input",
          )?;
        }

        input_vals.push(selected);
      }
    }

    let result = if op_name.starts_with("extern:") {
      // C 함수 이름 추출 (예: "extern:add" -> "add")
      let c_func_name = &op_name[7..]; // "extern:".len() == 7

      let output_ports: Vec<(&str, &str)> = if morphism.outputs.is_empty() {
        vec![("out", morphism.output.as_str())]
      } else {
        morphism
          .outputs
          .iter()
          .map(|p| (p.name.as_str(), p.ty.as_str()))
          .collect()
      };

      // 입력 타입을 LLVM 타입으로 변환
      let input_types: Vec<inkwell::types::BasicMetadataTypeEnum> = morphism
        .inputs
        .iter()
        .map(|port| {
          match normalize_type_name(&port.ty).as_str() {
            "int" | "i64" => Ok(i64_type.into()),
            "float" | "f64" => Ok(f64_type.into()),
            "bool" => Ok(i32_type.into()), // C ABI: bool은 i32로 전달
            "string" => Ok(i8_ptr_type.into()),
            "list" | "array" => Ok(i8_ptr_type.into()),
            "attrset" | "attrs" | "set" | "map" => Ok(i8_ptr_type.into()),
            _ => Err::<inkwell::types::BasicMetadataTypeEnum, _>(
              LlvmRuntimeError::ConfigError(format!(
                "Unsupported extern function input type '{}' for port '{}' in morphism '{}'. \
                 Supported types: Int, Float, Bool, String, List (limited), AttrSet (limited)",
                port.ty, port.name, morphism_name
              ))
              .into(),
            ),
          }
        })
        .collect::<Result<Vec<_>, RuntimeError>>()?;

      let output_types: Vec<inkwell::types::BasicTypeEnum> = output_ports
        .iter()
        .map(|(name, ty)| {
          match normalize_type_name(ty).as_str() {
            "int" | "i64" => Ok(i64_type.into()),
            "float" | "f64" => Ok(f64_type.into()),
            "bool" => Ok(i32_type.into()), // C ABI: bool은 i32로 반환
            "string" => Ok(i8_ptr_type.into()),
            "list" | "array" => Ok(i8_ptr_type.into()),
            "attrset" | "attrs" | "set" | "map" => Ok(i8_ptr_type.into()),
            _ => Err::<inkwell::types::BasicTypeEnum, _>(
              LlvmRuntimeError::ConfigError(format!(
                "Unsupported extern function output type '{}' for port '{}' in morphism '{}'. \
                 Supported types: Int, Float, Bool, String, List (limited), AttrSet (limited)",
                ty, name, morphism_name
              ))
              .into(),
            ),
          }
        })
        .collect::<Result<Vec<_>, RuntimeError>>()?;

      // 입력 값 개수 검증
      if input_vals.len() != morphism.inputs.len() {
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Extern function '{}' expects {} input(s), but got {} input(s). \
             Required ports: [{}]",
            c_func_name,
            morphism.inputs.len(),
            input_vals.len(),
            morphism
              .inputs
              .iter()
              .map(|p| p.name.as_str())
              .collect::<Vec<_>>()
              .join(", ")
          ))
          .into(),
        );
      }

      // 입력 값을 LLVM 값으로 변환 (타입 변환 필요 시)
      let mut call_args = Vec::new();
      for (i, (port, val)) in morphism.inputs.iter().zip(input_vals.iter()).enumerate() {
        let llvm_val = match normalize_type_name(&port.ty).as_str() {
          "int" | "i64" => {
            let int_val = val.as_int()?;
            int_val.into()
          }
          "float" | "f64" => {
            let float_val = val.as_float()?;
            float_val.into()
          }
          "bool" => {
            let bool_val = val.as_bool()?;
            let i32_val =
              b!(builder.build_int_z_extend(bool_val, i32_type, &format!("bool_to_i32_{}", i)));
            i32_val.into()
          }
          "string" => {
            let str_val = val.as_string()?;
            str_val.into()
          }
          "list" | "array" => {
            let list_val = val.as_list()?;
            list_val.into()
          }
          "attrset" | "attrs" | "set" | "map" => {
            let attr_val = val.as_attrset()?;
            attr_val.into()
          }
          _ => {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Unsupported extern function input type '{}' for port '{}'",
                port.ty, port.name
              ))
              .into(),
            );
          }
        };
        call_args.push(llvm_val);
      }

      // extern 함수 선언 (External linkage, cdecl 호출 규약)
      let fn_type = if output_types.len() == 1 {
        output_types[0].fn_type(&input_types, false)
      } else {
        context
          .struct_type(&output_types, false)
          .fn_type(&input_types, false)
      };
      let extern_fn = module.add_function(c_func_name, fn_type, None);
      extern_fn.set_linkage(inkwell::module::Linkage::External);

      let call = b!(builder.build_call(extern_fn, &call_args, &format!("call_{}", c_func_name)));

      let convert_output = |port_ty: &str,
                            value: inkwell::values::BasicValueEnum<'ctx>,
                            label: &str|
       -> Result<LlvmValue<'ctx>, RuntimeError> {
        match normalize_type_name(port_ty).as_str() {
          "int" | "i64" => Ok(LlvmValue::Int(value.into_int_value())),
          "float" | "f64" => Ok(LlvmValue::Float(value.into_float_value())),
          "bool" => {
            let i32_val = value.into_int_value();
            let zero = i32_type.const_int(0, false);
            let is_nonzero = b!(builder.build_int_compare(
              inkwell::IntPredicate::NE,
              i32_val,
              zero,
              &format!("{}_bool", label),
            ));
            Ok(LlvmValue::Bool(is_nonzero))
          }
          "string" => Ok(LlvmValue::String(value.into_pointer_value())),
          "list" | "array" => Ok(LlvmValue::List(value.into_pointer_value())),
          "attrset" | "attrs" | "set" | "map" => Ok(LlvmValue::AttrSet(value.into_pointer_value())),
          _ => Err(
            LlvmRuntimeError::ConfigError(format!(
              "Unsupported extern function output type '{}'",
              port_ty
            ))
            .into(),
          ),
        }
      };

      if output_ports.len() == 1 {
        let ret_val = match call.try_as_basic_value() {
          inkwell::values::ValueKind::Basic(value) => value,
          _ => {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Extern function '{}' returned void",
                c_func_name
              ))
              .into(),
            )
          }
        };
        let output = convert_output(output_ports[0].1, ret_val, c_func_name)?;
        NodeValue::Single(output)
      } else {
        let struct_val = match call.try_as_basic_value() {
          inkwell::values::ValueKind::Basic(value) => value.into_struct_value(),
          _ => {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Extern function '{}' returned void",
                c_func_name
              ))
              .into(),
            )
          }
        };
        let mut outputs = Vec::with_capacity(output_ports.len());
        for (idx, (_, port_ty)) in output_ports.iter().enumerate() {
          let field = b!(builder.build_extract_value(
            struct_val,
            idx as u32,
            &format!("{}_out_{}", c_func_name, idx)
          ));
          let out = convert_output(port_ty, field, &format!("{}_out_{}", c_func_name, idx))?;
          outputs.push(out);
        }
        NodeValue::Multi(outputs)
      }
    } else {
      if morphism.outputs.len() > 1 {
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Multi-output morphism not yet supported for builtins in runtime-llvm. \
             Morphism '{}' on node '{}' has {} output ports: [{}]. Use extern:<name> or single-output morphisms.",
            morphism_name,
            to_node,
            morphism.outputs.len(),
            morphism.outputs.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")
          ))
          .into(),
        );
      }
      let result = match op_name {
        "add" | "+" => match numeric_kind {
          NumericKind::Int => {
            if input_vals.is_empty() {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Add operation requires at least 1 input".to_string(),
                )
                .into(),
              );
            }
            let mut sum = input_vals[0].as_int()?;
            for (idx, val) in input_vals.iter().skip(1).enumerate() {
              let rhs = val.as_int()?;
              let label = format!("add_{}", idx);
              let (next, overflow) = build_int_overflow_intrinsic(
                module,
                &builder,
                "llvm.sadd.with.overflow",
                sum,
                rhs,
                &label,
              )?;
              let overflow_label = format!("add_overflow_{}", idx);
              set_runtime_error(overflow, RUNTIME_ERROR_INT_OVERFLOW, &overflow_label)?;
              sum = next;
            }
            LlvmValue::Int(sum)
          }
          NumericKind::Float => {
            if input_vals.is_empty() {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Add operation requires at least 1 input".to_string(),
                )
                .into(),
              );
            }
            let mut sum = input_vals[0].as_float()?;
            for val in input_vals.iter().skip(1) {
              sum = b!(builder.build_float_add(sum, val.as_float()?, "add"));
            }
            LlvmValue::Float(sum)
          }
        },
        "sub" | "-" | "subtract" => match numeric_kind {
          NumericKind::Int => {
            if input_vals.is_empty() {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Sub operation requires at least 1 input".to_string(),
                )
                .into(),
              );
            }
            let mut diff = input_vals[0].as_int()?;
            for (idx, val) in input_vals.iter().skip(1).enumerate() {
              let rhs = val.as_int()?;
              let label = format!("sub_{}", idx);
              let (next, overflow) = build_int_overflow_intrinsic(
                module,
                &builder,
                "llvm.ssub.with.overflow",
                diff,
                rhs,
                &label,
              )?;
              let overflow_label = format!("sub_overflow_{}", idx);
              set_runtime_error(overflow, RUNTIME_ERROR_INT_OVERFLOW, &overflow_label)?;
              diff = next;
            }
            LlvmValue::Int(diff)
          }
          NumericKind::Float => {
            if input_vals.is_empty() {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Sub operation requires at least 1 input".to_string(),
                )
                .into(),
              );
            }
            let mut diff = input_vals[0].as_float()?;
            for val in input_vals.iter().skip(1) {
              diff = b!(builder.build_float_sub(diff, val.as_float()?, "sub"));
            }
            LlvmValue::Float(diff)
          }
        },
        "mul" | "*" | "multiply" => match numeric_kind {
          NumericKind::Int => {
            if input_vals.is_empty() {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Mul operation requires at least 1 input".to_string(),
                )
                .into(),
              );
            }
            let mut prod = input_vals[0].as_int()?;
            for (idx, val) in input_vals.iter().skip(1).enumerate() {
              let rhs = val.as_int()?;
              let label = format!("mul_{}", idx);
              let (next, overflow) = build_int_overflow_intrinsic(
                module,
                &builder,
                "llvm.smul.with.overflow",
                prod,
                rhs,
                &label,
              )?;
              let overflow_label = format!("mul_overflow_{}", idx);
              set_runtime_error(overflow, RUNTIME_ERROR_INT_OVERFLOW, &overflow_label)?;
              prod = next;
            }
            LlvmValue::Int(prod)
          }
          NumericKind::Float => {
            if input_vals.is_empty() {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Mul operation requires at least 1 input".to_string(),
                )
                .into(),
              );
            }
            let mut prod = input_vals[0].as_float()?;
            for val in input_vals.iter().skip(1) {
              prod = b!(builder.build_float_mul(prod, val.as_float()?, "mul"));
            }
            LlvmValue::Float(prod)
          }
        },
        "div" | "/" | "divide" => match numeric_kind {
          NumericKind::Int => {
            if input_vals.is_empty() {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Div operation requires at least 1 input".to_string(),
                )
                .into(),
              );
            }
            let mut quot = input_vals[0].as_int()?;
            for val in input_vals.iter().skip(1) {
              // HIGH: 나눗셈 floor 조정 버그 수정 - 원래 피제수 사용
              // 체인 나눗셈에서 dividend_neg 검사는 원래 피제수를 사용해야 함
              // quot는 이미 이전 나눗셈 결과이므로 원래 피제수가 아님
              let original_dividend = quot; // 현재 quot가 이 반복에서의 원래 피제수
              let divisor = val.as_int()?;
              // Y218: 0으로 나누기 체크 - 런타임 에러 방지
              let zero = i64_type.const_int(0, false);
              let is_zero = b!(builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                divisor,
                zero,
                "divisor_is_zero",
              ));
              let min_value = i64_type.const_int(i64::MIN as u64, true);
              let minus_one = i64_type.const_int((-1i64) as u64, true);
              let is_min = b!(builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                original_dividend,
                min_value,
                "dividend_is_min",
              ));
              let is_neg_one = b!(builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                divisor,
                minus_one,
                "divisor_is_neg_one",
              ));
              let is_overflow = b!(builder.build_and(is_min, is_neg_one, "div_overflow"));
              let is_invalid = b!(builder.build_or(is_zero, is_overflow, "div_invalid"));
              let safe_divisor = b!(builder.build_select(
                is_invalid,
                i64_type.const_int(1, false),
                divisor,
                "divisor_safe",
              ))
              .into_int_value();
              // HIGH: 나눗셈/모듈로 체인에서 에러 코드 덮어쓰기 수정
              // 여러 에러 발생 시 첫 번째 에러를 보존하도록 개선
              let current_error_val = b!(builder.build_load(
                i32_type,
                runtime_error_code.as_pointer_value(),
                "runtime_error_code"
              ));
              if let Some(inst) = current_error_val.as_instruction_value() {
                let _ = inst.set_volatile(true);
              }
              let current_error = current_error_val.into_int_value();
              let zero_error = i32_type.const_int(RUNTIME_ERROR_DIV_ZERO_INT as u64, false);
              let overflow_error = i32_type.const_int(RUNTIME_ERROR_INT_OVERFLOW as u64, false);
              let new_error =
                b!(builder.build_select(is_zero, zero_error, overflow_error, "div_error_select"))
                  .into_int_value();
              let has_error = b!(builder.build_int_compare(
                inkwell::IntPredicate::NE,
                current_error,
                i32_type.const_zero(),
                "div_has_error",
              ));
              // 첫 번째 에러 보존: 이미 에러가 있으면 유지, 없으면 새 에러 설정
              let error_candidate = b!(builder.build_select(
                is_invalid,
                new_error,
                current_error,
                "div_error_candidate"
              ))
              .into_int_value();
              // has_error가 true면 current_error 유지 (첫 번째 에러 보존)
              // has_error가 false면 error_candidate 사용 (새 에러 또는 기존 에러 없음)
              let merged_error = b!(builder.build_select(
                has_error,
                current_error,
                error_candidate,
                "div_error_code"
              ))
              .into_int_value();
              let error_store =
                b!(builder.build_store(runtime_error_code.as_pointer_value(), merged_error));
              let _ = error_store.set_volatile(true);
              let raw_div =
                b!(builder.build_int_signed_div(original_dividend, safe_divisor, "div"));
              let rem =
                b!(builder.build_int_signed_rem(original_dividend, safe_divisor, "div_rem"));
              // Nix-style floor division for negative values.
              // HIGH: 원래 피제수를 사용하여 floor division 조정 계산
              let dividend_neg = b!(builder.build_int_compare(
                IntPredicate::SLT,
                original_dividend,
                zero,
                "dividend_neg"
              ));
              let divisor_neg =
                b!(builder.build_int_compare(IntPredicate::SLT, safe_divisor, zero, "divisor_neg"));
              let sign_diff = b!(builder.build_xor(dividend_neg, divisor_neg, "div_sign_diff"));
              let rem_nonzero =
                b!(builder.build_int_compare(IntPredicate::NE, rem, zero, "div_rem_nonzero"));
              let needs_adjust = b!(builder.build_and(sign_diff, rem_nonzero, "div_floor_adjust"));
              let adjusted =
                b!(builder.build_int_sub(raw_div, i64_type.const_int(1, false), "div_floor"));
              quot = b!(builder.build_select(needs_adjust, adjusted, raw_div, "div_floor_select"))
                .into_int_value();
            }
            LlvmValue::Int(quot)
          }
          NumericKind::Float => {
            if input_vals.is_empty() {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Div operation requires at least 1 input".to_string(),
                )
                .into(),
              );
            }
            let mut quot = input_vals[0].as_float()?;
            for val in input_vals.iter().skip(1) {
              let divisor = val.as_float()?;
              // Y218: 0으로 나누기 체크 - 런타임 에러 방지
              let zero = f64_type.const_float(0.0);
              let is_zero = b!(builder.build_float_compare(
                inkwell::FloatPredicate::OEQ,
                divisor,
                zero,
                "divisor_is_zero",
              ));
              let safe_divisor = b!(builder.build_select(
                is_zero,
                f64_type.const_float(1.0),
                divisor,
                "divisor_safe"
              ))
              .into_float_value();
              let current_error_val = b!(builder.build_load(
                i32_type,
                runtime_error_code.as_pointer_value(),
                "runtime_error_code"
              ));
              if let Some(inst) = current_error_val.as_instruction_value() {
                let _ = inst.set_volatile(true);
              }
              let current_error = current_error_val.into_int_value();
              let new_error = i32_type.const_int(RUNTIME_ERROR_DIV_ZERO_FLOAT as u64, false);
              let merged_error =
                b!(builder.build_select(is_zero, new_error, current_error, "div_error_code"))
                  .into_int_value();
              let error_store =
                b!(builder.build_store(runtime_error_code.as_pointer_value(), merged_error));
              let _ = error_store.set_volatile(true);
              quot = b!(builder.build_float_div(quot, safe_divisor, "div"));
            }
            LlvmValue::Float(quot)
          }
        },
        "mod" | "%" | "modulo" => match numeric_kind {
          NumericKind::Int => {
            if input_vals.len() != 2 {
              return Err(
                LlvmRuntimeError::ConfigError(format!(
                  "Mod operation requires 2 inputs, got {}",
                  input_vals.len()
                ))
                .into(),
              );
            }
            let divisor = input_vals[1].as_int()?;
            // Y218: 0으로 모듈로 체크 - 런타임 에러 방지
            let zero = i64_type.const_int(0, false);
            let is_zero = b!(builder.build_int_compare(
              inkwell::IntPredicate::EQ,
              divisor,
              zero,
              "mod_divisor_is_zero",
            ));
            let dividend = input_vals[0].as_int()?;
            let min_value = i64_type.const_int(i64::MIN as u64, true);
            let minus_one = i64_type.const_int((-1i64) as u64, true);
            let is_min = b!(builder.build_int_compare(
              inkwell::IntPredicate::EQ,
              dividend,
              min_value,
              "mod_dividend_is_min",
            ));
            let is_neg_one = b!(builder.build_int_compare(
              inkwell::IntPredicate::EQ,
              divisor,
              minus_one,
              "mod_divisor_is_neg_one",
            ));
            let is_overflow = b!(builder.build_and(is_min, is_neg_one, "mod_overflow"));
            let is_invalid = b!(builder.build_or(is_zero, is_overflow, "mod_invalid"));
            let safe_divisor = b!(builder.build_select(
              is_invalid,
              i64_type.const_int(1, false),
              divisor,
              "mod_divisor_safe",
            ))
            .into_int_value();
            let current_error_val = b!(builder.build_load(
              i32_type,
              runtime_error_code.as_pointer_value(),
              "runtime_error_code"
            ));
            if let Some(inst) = current_error_val.as_instruction_value() {
              let _ = inst.set_volatile(true);
            }
            let current_error = current_error_val.into_int_value();
            let zero_error = i32_type.const_int(RUNTIME_ERROR_MOD_ZERO_INT as u64, false);
            let overflow_error = i32_type.const_int(RUNTIME_ERROR_INT_OVERFLOW as u64, false);
            let new_error =
              b!(builder.build_select(is_zero, zero_error, overflow_error, "mod_error_select"))
                .into_int_value();
            // HIGH: 나눗셈/모듈로 체인에서 에러 코드 덮어쓰기 수정
            // 여러 에러 발생 시 첫 번째 에러를 보존
            let has_error = b!(builder.build_int_compare(
              inkwell::IntPredicate::NE,
              current_error,
              i32_type.const_zero(),
              "mod_has_error",
            ));
            let error_candidate =
              b!(builder.build_select(is_invalid, new_error, current_error, "mod_error_candidate"))
                .into_int_value();
            // has_error가 true면 current_error 유지 (첫 번째 에러 보존)
            let merged_error =
              b!(builder.build_select(has_error, current_error, error_candidate, "mod_error_code"))
                .into_int_value();
            let error_store =
              b!(builder.build_store(runtime_error_code.as_pointer_value(), merged_error));
            let _ = error_store.set_volatile(true);
            let rem = b!(builder.build_int_signed_rem(dividend, safe_divisor, "srem"));
            // Nix uses floor modulo (not truncated modulo like LLVM srem)
            // -7 % 3 = 2 (Nix) vs -7 % 3 = -1 (LLVM srem)
            // If remainder is nonzero and signs differ, add divisor
            let zero = i64_type.const_int(0, false);
            let rem_nonzero =
              b!(builder.build_int_compare(inkwell::IntPredicate::NE, rem, zero, "rem_nonzero"));
            let dividend_neg = b!(builder.build_int_compare(
              inkwell::IntPredicate::SLT,
              dividend,
              zero,
              "dividend_neg"
            ));
            let divisor_neg = b!(builder.build_int_compare(
              inkwell::IntPredicate::SLT,
              safe_divisor,
              zero,
              "divisor_neg",
            ));
            let sign_diff = b!(builder.build_xor(dividend_neg, divisor_neg, "sign_diff"));
            let needs_adjust = b!(builder.build_and(sign_diff, rem_nonzero, "needs_adjust"));
            let adjusted = b!(builder.build_int_add(rem, safe_divisor, "adjusted_mod"));
            // LOW: 비트 시프트 범위 검증 없음
            // 64 이상 시프트 시 UB 발생 가능
            // 현재는 시프트 범위 검증 없이 LLVM에 전달
            // 향후 개선: 시프트 범위 검증 추가 필요
            let result =
              b!(builder.build_select(needs_adjust, adjusted, rem, "floor_mod")).into_int_value();
            LlvmValue::Int(result)
          }
          NumericKind::Float => {
            if input_vals.len() != 2 {
              return Err(
                LlvmRuntimeError::ConfigError(format!(
                  "Mod operation requires 2 inputs, got {}",
                  input_vals.len()
                ))
                .into(),
              );
            }
            let divisor = input_vals[1].as_float()?;
            // Y218: 0으로 모듈로 체크 - 런타임 에러 방지
            let zero = f64_type.const_float(0.0);
            let is_zero = b!(builder.build_float_compare(
              inkwell::FloatPredicate::OEQ,
              divisor,
              zero,
              "mod_divisor_is_zero",
            ));
            let safe_divisor = b!(builder.build_select(
              is_zero,
              f64_type.const_float(1.0),
              divisor,
              "mod_divisor_safe",
            ))
            .into_float_value();
            let current_error_val = b!(builder.build_load(
              i32_type,
              runtime_error_code.as_pointer_value(),
              "runtime_error_code"
            ));
            if let Some(inst) = current_error_val.as_instruction_value() {
              let _ = inst.set_volatile(true);
            }
            let current_error = current_error_val.into_int_value();
            let new_error = i32_type.const_int(RUNTIME_ERROR_MOD_ZERO_FLOAT as u64, false);
            let merged_error =
              b!(builder.build_select(is_zero, new_error, current_error, "mod_error_code"))
                .into_int_value();
            let error_store =
              b!(builder.build_store(runtime_error_code.as_pointer_value(), merged_error));
            let _ = error_store.set_volatile(true);
            let rem = b!(builder.build_float_rem(input_vals[0].as_float()?, safe_divisor, "mod"));
            LlvmValue::Float(rem)
          }
        },
        "sin" => {
          let arg = input_vals
            .get(0)
            .ok_or_else(|| LlvmRuntimeError::ConfigError("Sin requires 1 input".to_string()))?
            .as_float()?;
          let value = build_float_intrinsic(module, &builder, "llvm.sin", arg, "sin")?;
          LlvmValue::Float(value)
        }
        "cos" => {
          let arg = input_vals
            .get(0)
            .ok_or_else(|| LlvmRuntimeError::ConfigError("Cos requires 1 input".to_string()))?
            .as_float()?;
          let value = build_float_intrinsic(module, &builder, "llvm.cos", arg, "cos")?;
          LlvmValue::Float(value)
        }
        "sqrt" => {
          let arg = input_vals
            .get(0)
            .ok_or_else(|| LlvmRuntimeError::ConfigError("Sqrt requires 1 input".to_string()))?
            .as_float()?;
          // 음수 입력 확인 (legacy evaluator와 일치)
          let zero = f64_type.const_float(0.0);
          let is_negative =
            b!(builder.build_float_compare(FloatPredicate::OLT, arg, zero, "sqrt_is_negative"));
          set_runtime_error(is_negative, RUNTIME_ERROR_DOMAIN_ERROR, "sqrt_domain_error")?;
          let value = build_float_intrinsic(module, &builder, "llvm.sqrt", arg, "sqrt")?;
          // 음수인 경우 NaN 대신 0 반환 (에러 코드는 이미 설정됨)
          let safe_value =
            b!(builder.build_select(is_negative, f64_type.const_float(0.0), value, "sqrt_safe"));
          LlvmValue::Float(safe_value.into_float_value())
        }
        "floor" => {
          let arg = input_vals
            .get(0)
            .ok_or_else(|| LlvmRuntimeError::ConfigError("Floor requires 1 input".to_string()))?
            .as_float()?;
          let value = build_float_intrinsic(module, &builder, "llvm.floor", arg, "floor")?;
          LlvmValue::Float(value)
        }
        "ceil" => {
          let arg = input_vals
            .get(0)
            .ok_or_else(|| LlvmRuntimeError::ConfigError("Ceil requires 1 input".to_string()))?
            .as_float()?;
          let value = build_float_intrinsic(module, &builder, "llvm.ceil", arg, "ceil")?;
          LlvmValue::Float(value)
        }
        "pow" | "**" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Pow operation requires 2 inputs (base, exponent), got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          match numeric_kind {
            NumericKind::Int => {
              // MEDIUM: pow 음수^소수 의미론 미정의 수정 완료
              // 음수의 소수 거듭제곱은 복소수 결과이므로 수학적으로 미정의
              // LLVM pow intrinsic은 음수^소수에 대해 NaN을 반환
              // 현재 구현: NaN을 감지하여 에러 코드 설정 (RUNTIME_ERROR_POW_OVERFLOW)
              // Int pow: convert to float, compute, convert back
              let base_f =
                b!(builder.build_signed_int_to_float(input_vals[0].as_int()?, f64_type, "base_f"));
              let exp_f =
                b!(builder.build_signed_int_to_float(input_vals[1].as_int()?, f64_type, "exp_f"));
              // LLVM pow intrinsic: get_declaration with return type only (inkwell auto-mangles)
              let intrinsic = Intrinsic::find("llvm.pow.f64").ok_or_else(|| {
                LlvmRuntimeError::ConfigError("LLVM intrinsic 'llvm.pow.f64' not found".to_string())
              })?;
              let decl = intrinsic
                .get_declaration(module, &[f64_type.into()])
                .ok_or_else(|| {
                  LlvmRuntimeError::ConfigError(
                    "LLVM intrinsic 'llvm.pow.f64' declaration missing".to_string(),
                  )
                })?;
              let call = b!(builder.build_call(decl, &[base_f.into(), exp_f.into()], "pow"));
              let result_f = match call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(value) => value.into_float_value(),
                _ => {
                  return Err(
                    LlvmRuntimeError::ConfigError(
                      "LLVM intrinsic 'llvm.pow.f64' returned void".to_string(),
                    )
                    .into(),
                  )
                }
              };
              // Check for Infinity/NaN: result != result (NaN) or |result| == Infinity
              // NaN은 음수^소수 또는 기타 도메인 에러를 나타냄
              let result_eq_self = b!(builder.build_float_compare(
                FloatPredicate::OEQ,
                result_f,
                result_f,
                "pow_result_eq_self",
              ));
              let inf = f64_type.const_float(f64::INFINITY);
              let neg_inf = f64_type.const_float(f64::NEG_INFINITY);
              let is_inf =
                b!(builder.build_float_compare(FloatPredicate::OEQ, result_f, inf, "pow_is_inf"));
              let is_neg_inf = b!(builder.build_float_compare(
                FloatPredicate::OEQ,
                result_f,
                neg_inf,
                "pow_is_neg_inf"
              ));
              let is_special = b!(builder.build_or(is_inf, is_neg_inf, "pow_is_special"));
              let not_special = b!(builder.build_not(is_special, "pow_not_special"));
              let is_finite = b!(builder.build_and(result_eq_self, not_special, "pow_is_finite"));
              let min_f = f64_type.const_float(i64::MIN as f64);
              let max_f = f64_type.const_float(i64::MAX as f64);
              let within_min =
                b!(builder.build_float_compare(FloatPredicate::OGE, result_f, min_f, "pow_ge_min"));
              let within_max =
                b!(builder.build_float_compare(FloatPredicate::OLE, result_f, max_f, "pow_le_max"));
              let within_range = b!(builder.build_and(within_min, within_max, "pow_within_range"));
              let is_valid = b!(builder.build_and(is_finite, within_range, "pow_valid"));
              let current_error_val = b!(builder.build_load(
                i32_type,
                runtime_error_code.as_pointer_value(),
                "runtime_error_code"
              ));
              if let Some(inst) = current_error_val.as_instruction_value() {
                let _ = inst.set_volatile(true);
              }
              let current_error = current_error_val.into_int_value();
              let new_error = i32_type.const_int(RUNTIME_ERROR_POW_OVERFLOW as u64, false);
              let merged_error =
                b!(builder.build_select(is_valid, current_error, new_error, "pow_error_code"))
                  .into_int_value();
              let error_store =
                b!(builder.build_store(runtime_error_code.as_pointer_value(), merged_error));
              let _ = error_store.set_volatile(true);
              let safe_result_f =
                b!(builder.build_select(is_valid, result_f, f64_type.const_float(0.0), "pow_safe"))
                  .into_float_value();
              // CRITICAL: f64→i64 변환 범위 검사
              // build_float_to_signed_int는 범위를 벗어나면 UB 발생 가능
              // 이미 is_valid로 범위 검사했지만, 추가 검증 필요
              let result_i =
                b!(builder.build_float_to_signed_int(safe_result_f, i64_type, "pow_i"));
              LlvmValue::Int(result_i)
            }
            NumericKind::Float => {
              let base = input_vals[0].as_float()?;
              let exp = input_vals[1].as_float()?;
              // LLVM pow intrinsic: get_declaration with return type only (inkwell auto-mangles)
              let intrinsic = Intrinsic::find("llvm.pow.f64").ok_or_else(|| {
                LlvmRuntimeError::ConfigError("LLVM intrinsic 'llvm.pow.f64' not found".to_string())
              })?;
              let decl = intrinsic
                .get_declaration(module, &[f64_type.into()])
                .ok_or_else(|| {
                  LlvmRuntimeError::ConfigError(
                    "LLVM intrinsic 'llvm.pow.f64' declaration missing".to_string(),
                  )
                })?;
              let call = b!(builder.build_call(decl, &[base.into(), exp.into()], "pow"));
              let value = match call.try_as_basic_value() {
                inkwell::values::ValueKind::Basic(value) => value,
                _ => {
                  return Err(
                    LlvmRuntimeError::ConfigError(
                      "LLVM intrinsic 'llvm.pow.f64' returned void".to_string(),
                    )
                    .into(),
                  )
                }
              };
              LlvmValue::Float(value.into_float_value())
            }
          }
        }
        "shl" | "<<" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Shl operation requires 2 inputs (value, shift), got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          match numeric_kind {
            NumericKind::Int => {
              let value = input_vals[0].as_int()?;
              let shift = input_vals[1].as_int()?;
              // LOW: 비트 시프트 범위 검증 없음 수정 완료
              // 64 이상 시프트 시 UB 방지를 위해 범위 검증 구현됨
              // 음수 시프트와 64 이상 시프트 모두 검증하여 RUNTIME_ERROR_SHIFT_OUT_OF_RANGE 설정
              let zero = i64_type.const_int(0, false);
              let max_shift = i64_type.const_int(64, false);
              let shift_negative =
                b!(builder.build_int_compare(IntPredicate::SLT, shift, zero, "shl_neg"));
              let shift_too_large =
                b!(builder.build_int_compare(IntPredicate::SGE, shift, max_shift, "shl_oor"));
              let shift_invalid =
                b!(builder.build_or(shift_negative, shift_too_large, "shl_invalid"));
              let safe_shift =
                b!(builder.build_select(shift_invalid, zero, shift, "shl_safe")).into_int_value();
              let current_error_val = b!(builder.build_load(
                i32_type,
                runtime_error_code.as_pointer_value(),
                "runtime_error_code"
              ));
              if let Some(inst) = current_error_val.as_instruction_value() {
                let _ = inst.set_volatile(true);
              }
              let current_error = current_error_val.into_int_value();
              let new_error = i32_type.const_int(RUNTIME_ERROR_SHIFT_OUT_OF_RANGE as u64, false);
              let merged_error =
                b!(builder.build_select(shift_invalid, new_error, current_error, "shl_error_code"))
                  .into_int_value();
              let error_store =
                b!(builder.build_store(runtime_error_code.as_pointer_value(), merged_error));
              let _ = error_store.set_volatile(true);
              let result = b!(builder.build_left_shift(value, safe_shift, "shl"));
              LlvmValue::Int(result)
            }
            NumericKind::Float => {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Bitwise shift operations (shl) are only supported for Int type, not Float"
                    .to_string(),
                )
                .into(),
              );
            }
          }
        }
        "shr" | ">>" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Shr operation requires 2 inputs (value, shift), got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          match numeric_kind {
            NumericKind::Int => {
              let value = input_vals[0].as_int()?;
              let shift = input_vals[1].as_int()?;
              let zero = i64_type.const_int(0, false);
              let max_shift = i64_type.const_int(64, false);
              let shift_negative =
                b!(builder.build_int_compare(IntPredicate::SLT, shift, zero, "shr_neg"));
              let shift_too_large =
                b!(builder.build_int_compare(IntPredicate::SGE, shift, max_shift, "shr_oor"));
              let shift_invalid =
                b!(builder.build_or(shift_negative, shift_too_large, "shr_invalid"));
              let safe_shift =
                b!(builder.build_select(shift_invalid, zero, shift, "shr_safe")).into_int_value();
              let current_error_val = b!(builder.build_load(
                i32_type,
                runtime_error_code.as_pointer_value(),
                "runtime_error_code"
              ));
              if let Some(inst) = current_error_val.as_instruction_value() {
                let _ = inst.set_volatile(true);
              }
              let current_error = current_error_val.into_int_value();
              let new_error = i32_type.const_int(RUNTIME_ERROR_SHIFT_OUT_OF_RANGE as u64, false);
              let merged_error =
                b!(builder.build_select(shift_invalid, new_error, current_error, "shr_error_code"))
                  .into_int_value();
              let error_store =
                b!(builder.build_store(runtime_error_code.as_pointer_value(), merged_error));
              let _ = error_store.set_volatile(true);
              // Arithmetic right shift (signed)
              let result = b!(builder.build_right_shift(value, safe_shift, true, "shr"));
              LlvmValue::Int(result)
            }
            NumericKind::Float => {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Bitwise shift operations (shr) are only supported for Int type, not Float"
                    .to_string(),
                )
                .into(),
              );
            }
          }
        }
        "bitand" | "&" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Bitwise and operation requires 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          match numeric_kind {
            NumericKind::Int => {
              let lhs = input_vals[0].as_int()?;
              let rhs = input_vals[1].as_int()?;
              let result = b!(builder.build_and(lhs, rhs, "bitand"));
              LlvmValue::Int(result)
            }
            NumericKind::Float => {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Bitwise and operation is only supported for Int type, not Float".to_string(),
                )
                .into(),
              );
            }
          }
        }
        "bitor" | "|" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Bitwise or operation requires 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          match numeric_kind {
            NumericKind::Int => {
              let lhs = input_vals[0].as_int()?;
              let rhs = input_vals[1].as_int()?;
              let result = b!(builder.build_or(lhs, rhs, "bitor"));
              LlvmValue::Int(result)
            }
            NumericKind::Float => {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Bitwise or operation is only supported for Int type, not Float".to_string(),
                )
                .into(),
              );
            }
          }
        }
        "bitxor" | "^" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Bitwise xor operation requires 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          match numeric_kind {
            NumericKind::Int => {
              let lhs = input_vals[0].as_int()?;
              let rhs = input_vals[1].as_int()?;
              let result = b!(builder.build_xor(lhs, rhs, "bitxor"));
              LlvmValue::Int(result)
            }
            NumericKind::Float => {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Bitwise xor operation is only supported for Int type, not Float".to_string(),
                )
                .into(),
              );
            }
          }
        }
        "bitnot" | "~" => {
          if input_vals.len() != 1 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Bitwise not operation requires 1 input, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          match numeric_kind {
            NumericKind::Int => {
              let value = input_vals[0].as_int()?;
              let result = b!(builder.build_not(value, "bitnot"));
              LlvmValue::Int(result)
            }
            NumericKind::Float => {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Bitwise not operation is only supported for Int type, not Float".to_string(),
                )
                .into(),
              );
            }
          }
        }
        "if" | "select" => {
          if input_vals.len() != 3 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "If/select requires 3 inputs (cond, then, else), got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          // Y05c-8: if 조건 타입 정책 통일 - Bool만 허용 (runtime-legacy/타입 추론과 일치)
          // Y13a-15: Bool 리터럴 타입 정책 - Bool 리터럴("true"/"false")은 항상 Bool로 유지되므로
          // Bool-only 조건에서 사용 가능. Int는 여전히 에러로 처리 (명시적 캐스트 필요)
          let cond = match input_vals[0].kind() {
          ValueKind::Bool => input_vals[0].as_bool()?,
          ValueKind::Int => {
            return Err(
              LlvmRuntimeError::ConfigError(
                "If/select condition must be Bool type, not Int. Use comparison operators (eq/ne/lt/le/gt/ge) to convert Int to Bool, or use Bool literals (\"true\"/\"false\") which are always Bool type.".to_string()
              )
                .into(),
            )
          }
          ValueKind::Float => {
            return Err(
              LlvmRuntimeError::ConfigError("If/select condition must be Bool type, not Float".to_string())
                .into(),
            )
          }
          ValueKind::String => {
            return Err(
              LlvmRuntimeError::ConfigError("If/select condition must be Bool type, not String".to_string())
                .into(),
            )
          }
          ValueKind::List | ValueKind::AttrSet => {
            return Err(
              LlvmRuntimeError::ConfigError(
                "If/select condition must be Bool type, not List/AttrSet".to_string(),
              )
              .into(),
            )
          }
        };

          // Y217: if/select 비수치 타입 정책 - Bool/String 선택을 명시적 에러로 제한
          let then_kind = input_vals[1].kind();
          let else_kind = input_vals[2].kind();

          // Bool/String 타입은 명시적 에러로 제한
          if then_kind == ValueKind::Bool || else_kind == ValueKind::Bool {
            return Err(
            LlvmRuntimeError::ConfigError(
              "If/select operation does not support Bool values for then/else branches. Use numeric types (Int/Float) or String type.".to_string()
            )
            .into(),
          );
          }
          if then_kind == ValueKind::String || else_kind == ValueKind::String {
            return Err(
            LlvmRuntimeError::ConfigError(
              "If/select operation does not support String values for then/else branches. Use numeric types (Int/Float) or implement String selection separately.".to_string()
            )
            .into(),
          );
          }

          // 타입 일치 확인
          if then_kind != else_kind {
            return Err(
            LlvmRuntimeError::ConfigError(format!(
              "If/select operation requires then and else branches to have the same type, got {:?} and {:?}",
              then_kind, else_kind
            ))
            .into(),
          );
          }

          match numeric_kind {
            NumericKind::Int => {
              let then_val = input_vals[1].as_int()?;
              let else_val = input_vals[2].as_int()?;
              let selected = b!(builder.build_select(cond, then_val, else_val, "if"));
              LlvmValue::Int(selected.into_int_value())
            }
            NumericKind::Float => {
              let then_val = input_vals[1].as_float()?;
              let else_val = input_vals[2].as_float()?;
              let selected = b!(builder.build_select(cond, then_val, else_val, "if"));
              LlvmValue::Float(selected.into_float_value())
            }
          }
        }
        "eq" | "==" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Eq requires 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          let cmp = match numeric_kind {
            NumericKind::Int => b!(builder.build_int_compare(
              IntPredicate::EQ,
              input_vals[0].as_int()?,
              input_vals[1].as_int()?,
              "eq",
            )),
            NumericKind::Float => {
              // LOW: ARM64 타겟 트리플 정규화 없음
              // arm64 vs aarch64 구분 없음
              // 현재는 타겟 트리플을 그대로 사용하여 arm64/aarch64 구분 없음
              b!(builder.build_float_compare(
                FloatPredicate::OEQ,
                input_vals[0].as_float()?,
                input_vals[1].as_float()?,
                "eq",
              ))
            }
          };
          LlvmValue::Bool(cmp)
        }
        "ne" | "!=" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Ne requires 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          let cmp = match numeric_kind {
            NumericKind::Int => b!(builder.build_int_compare(
              IntPredicate::NE,
              input_vals[0].as_int()?,
              input_vals[1].as_int()?,
              "ne",
            )),
            NumericKind::Float => b!(builder.build_float_compare(
              FloatPredicate::UNE,
              input_vals[0].as_float()?,
              input_vals[1].as_float()?,
              "ne",
            )),
          };
          LlvmValue::Bool(cmp)
        }
        "lt" | "<" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Lt requires 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          let cmp = match numeric_kind {
            NumericKind::Int => b!(builder.build_int_compare(
              IntPredicate::SLT,
              input_vals[0].as_int()?,
              input_vals[1].as_int()?,
              "lt",
            )),
            NumericKind::Float => b!(builder.build_float_compare(
              FloatPredicate::OLT,
              input_vals[0].as_float()?,
              input_vals[1].as_float()?,
              "lt",
            )),
          };
          LlvmValue::Bool(cmp)
        }
        "le" | "<=" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Le requires 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          let cmp = match numeric_kind {
            NumericKind::Int => b!(builder.build_int_compare(
              IntPredicate::SLE,
              input_vals[0].as_int()?,
              input_vals[1].as_int()?,
              "le",
            )),
            NumericKind::Float => b!(builder.build_float_compare(
              FloatPredicate::OLE,
              input_vals[0].as_float()?,
              input_vals[1].as_float()?,
              "le",
            )),
          };
          LlvmValue::Bool(cmp)
        }
        "gt" | ">" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Gt requires 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          let cmp = match numeric_kind {
            NumericKind::Int => b!(builder.build_int_compare(
              IntPredicate::SGT,
              input_vals[0].as_int()?,
              input_vals[1].as_int()?,
              "gt",
            )),
            NumericKind::Float => b!(builder.build_float_compare(
              FloatPredicate::OGT,
              input_vals[0].as_float()?,
              input_vals[1].as_float()?,
              "gt",
            )),
          };
          LlvmValue::Bool(cmp)
        }
        "ge" | ">=" => {
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Ge requires 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          let cmp = match numeric_kind {
            NumericKind::Int => b!(builder.build_int_compare(
              IntPredicate::SGE,
              input_vals[0].as_int()?,
              input_vals[1].as_int()?,
              "ge",
            )),
            NumericKind::Float => b!(builder.build_float_compare(
              FloatPredicate::OGE,
              input_vals[0].as_float()?,
              input_vals[1].as_float()?,
              "ge",
            )),
          };
          LlvmValue::Bool(cmp)
        }
        "and" | "&&" => {
          // Y13a-8: 논리 연산 and 지원 - Bool 타입만 허용
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Logical and operation requires 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          // Y13a-15: Bool 리터럴 타입 정책 - Bool 리터럴("true"/"false")은 항상 Bool로 유지되므로
          // 논리 연산에서 사용 가능. Int는 여전히 에러로 처리 (명시적 캐스트 필요)
          let lhs = match input_vals[0].kind() {
            ValueKind::Bool => input_vals[0].as_bool()?,
            _ => {
              return Err(
              LlvmRuntimeError::ConfigError(
                "Logical and operation requires Bool type for both operands. Use Bool literals (\"true\"/\"false\") which are always Bool type, or use comparison operators (eq/ne/lt/le/gt/ge) to convert Int to Bool.".to_string(),
              )
              .into(),
            );
            }
          };
          let rhs = match input_vals[1].kind() {
            ValueKind::Bool => input_vals[1].as_bool()?,
            _ => {
              return Err(
              LlvmRuntimeError::ConfigError(
                "Logical and operation requires Bool type for both operands. Use Bool literals (\"true\"/\"false\") which are always Bool type, or use comparison operators (eq/ne/lt/le/gt/ge) to convert Int to Bool.".to_string(),
              )
              .into(),
            );
            }
          };
          // LLVM의 build_and는 bitwise AND이지만, Bool(i1) 타입에서는 논리 AND와 동일
          let result = b!(builder.build_and(lhs, rhs, "and"));
          LlvmValue::Bool(result)
        }
        "or" | "||" => {
          // Y13a-8: 논리 연산 or 지원 - Bool 타입만 허용
          if input_vals.len() != 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Logical or operation requires 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          // Y13a-15: Bool 리터럴 타입 정책 - Bool 리터럴("true"/"false")은 항상 Bool로 유지되므로
          // 논리 연산에서 사용 가능. Int는 여전히 에러로 처리 (명시적 캐스트 필요)
          let lhs = match input_vals[0].kind() {
            ValueKind::Bool => input_vals[0].as_bool()?,
            _ => {
              return Err(
              LlvmRuntimeError::ConfigError(
                "Logical or operation requires Bool type for both operands. Use Bool literals (\"true\"/\"false\") which are always Bool type, or use comparison operators (eq/ne/lt/le/gt/ge) to convert Int to Bool.".to_string(),
              )
              .into(),
            );
            }
          };
          let rhs = match input_vals[1].kind() {
            ValueKind::Bool => input_vals[1].as_bool()?,
            _ => {
              return Err(
              LlvmRuntimeError::ConfigError(
                "Logical or operation requires Bool type for both operands. Use Bool literals (\"true\"/\"false\") which are always Bool type, or use comparison operators (eq/ne/lt/le/gt/ge) to convert Int to Bool.".to_string(),
              )
              .into(),
            );
            }
          };
          // LLVM의 build_or는 bitwise OR이지만, Bool(i1) 타입에서는 논리 OR와 동일
          let result = b!(builder.build_or(lhs, rhs, "or"));
          LlvmValue::Bool(result)
        }
        "not" | "!" => {
          // Y13a-8: 논리 연산 not 지원 - Bool 타입만 허용
          if input_vals.len() != 1 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Logical not operation requires 1 input, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          // Y13a-15: Bool 리터럴 타입 정책 - Bool 리터럴("true"/"false")은 항상 Bool로 유지되므로
          // 논리 연산에서 사용 가능. Int는 여전히 에러로 처리 (명시적 캐스트 필요)
          let value = match input_vals[0].kind() {
            ValueKind::Bool => input_vals[0].as_bool()?,
            _ => {
              return Err(
              LlvmRuntimeError::ConfigError(
                "Logical not operation requires Bool type. Use Bool literals (\"true\"/\"false\") which are always Bool type, or use comparison operators (eq/ne/lt/le/gt/ge) to convert Int to Bool.".to_string(),
              )
              .into(),
            );
            }
          };
          // LLVM의 build_not는 bitwise NOT이지만, Bool(i1) 타입에서는 논리 NOT와 동일
          let result = b!(builder.build_not(value, "not"));
          LlvmValue::Bool(result)
        }
        "concat" | "++" | "String.concat" | "builtins.String.concat" => {
          if input_vals.len() < 2 {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Concat operation requires at least 2 inputs, got {}",
                input_vals.len()
              ))
              .into(),
            );
          }
          // String concatenation: allocate memory and copy strings
          // For simplicity, we'll use a helper approach: create a function that concatenates strings
          // This is a simplified implementation - full string support would require more complex memory management

          // Check if all inputs are strings
          for val in &input_vals {
            if val.kind() != ValueKind::String {
              return Err(
                LlvmRuntimeError::ConfigError(
                  "Concat operation requires all inputs to be String type".to_string(),
                )
                .into(),
              );
            }
          }

          // Y05c-2: 실제 문자열 concat 구현
          // 컴파일 타임 문자열 리터럴의 경우 직접 연결
          // 런타임 문자열의 경우 malloc/strlen/memcpy 사용

          if input_vals.is_empty() {
            return Err(
              LlvmRuntimeError::ConfigError(
                "Concat requires at least one string input".to_string(),
              )
              .into(),
            );
          }

          // 컴파일 타임 문자열 리터럴인지 확인
          // 현재는 from_input 문자열 리터럴만 지원하므로, 모든 입력이 컴파일 타임 상수일 가능성이 높음
          // 하지만 런타임 문자열도 고려해야 하므로, 두 가지 경로를 모두 지원
          // LOW: 입력 배열 접근 경계 미검증 수정 완료
          // input_vals.iter().enumerate()를 사용하여 배열 경계 검증 보장
          // input_vals는 이미 morphism.inputs.len()과 일치하는지 검증됨 (line 1510)
          // 따라서 인덱스 접근은 항상 안전함

          // Y05c-2: 실제 문자열 concat 구현
          // malloc + strlen + memcpy를 사용하여 런타임에 문자열 연결
          // Fix: Determine size_t dynamically based on target pointer width
          // size_t는 포인터 크기와 동일해야 하며, 32비트 시스템에서는 i32, 64비트에서는 i64
          let i64_type = context.i64_type();
          let i32_type = context.i32_type();

          // Prefer data layout pointer size; fall back to target triple heuristics if missing.
          let pointer_size_bits_from_layout = |layout: &str| -> Option<u64> {
            let mut default_ptr = None;
            for part in layout.split('-') {
              if let Some(rest) = part.strip_prefix("p0:") {
                if let Some(bits) = rest.split(':').next() {
                  if let Ok(value) = bits.parse::<u64>() {
                    return Some(value);
                  }
                }
              } else if let Some(rest) = part.strip_prefix("p:") {
                if let Some(bits) = rest.split(':').next() {
                  if let Ok(value) = bits.parse::<u64>() {
                    default_ptr = Some(value);
                  }
                }
              }
            }
            default_ptr
          };

          let data_layout = module.get_data_layout();
          let layout_str = data_layout.as_str().to_str().unwrap_or("");
          let target_triple = module.get_triple();
          let target_triple_str = target_triple.as_str().to_str().unwrap_or("");
          // LOW: ARM64 타겟 트리플 정규화 없음 수정 완료
          // arm64를 aarch64로 정규화하여 일관성 유지
          // LLVM은 aarch64를 표준으로 사용하므로 arm64를 aarch64로 변환
          let normalized_triple = target_triple_str.replace("arm64", "aarch64");
          let pointer_size_bits = pointer_size_bits_from_layout(layout_str).unwrap_or_else(|| {
          if normalized_triple.contains("x86_64")
            || normalized_triple.contains("aarch64")
            || normalized_triple.contains("mips64")
            || normalized_triple.contains("powerpc64")
            || normalized_triple.contains("riscv64")
            || normalized_triple.contains("sparc64")
          {
            64
          } else if normalized_triple.contains("i386")
            || normalized_triple.contains("i686")
            || normalized_triple.contains("armv7")
            || normalized_triple.contains("armv6")
            || normalized_triple.contains("mips")
            || normalized_triple.contains("powerpc")
            || normalized_triple.contains("riscv32")
            || normalized_triple.contains("wasm32")
          {
            32
          } else {
            eprintln!(
              "warning: Unknown target triple '{}' and missing data layout, assuming 64-bit pointer size.",
              target_triple_str
            );
            64
          }
        });
          let size_t_type = if pointer_size_bits == 32 {
            i32_type
          } else {
            // Default to i64 for 64-bit and other sizes
            i64_type
          };

          // 1. strlen 함수 선언 (size_t 반환)
          // Y05c-10: libc 의존성 명시 - External linkage로 선언하여 링커가 libc에서 해석하도록 함
          // AOT 컴파일 시 링커가 libc를 링크하여 이 심볼들을 해석함
          let strlen_fn_type = size_t_type.fn_type(&[i8_ptr_type.into()], false);
          let strlen_fn = module.add_function("strlen", strlen_fn_type, None);
          strlen_fn.set_linkage(inkwell::module::Linkage::External);
          // Note: strlen은 표준 C 라이브러리 함수로, 링커가 libc에서 자동으로 해석함
          // Linux/macOS: libc.so/libc.dylib에서 해석
          // Windows: MSVCRT.dll에서 해석

          // 2. malloc 함수 선언 (size_t 인자)
          let malloc_fn_type = i8_ptr_type.fn_type(&[size_t_type.into()], false);
          let malloc_fn = module.add_function("malloc", malloc_fn_type, None);
          malloc_fn.set_linkage(inkwell::module::Linkage::External);
          // Note: malloc은 표준 C 라이브러리 함수로, 링커가 libc에서 자동으로 해석함

          // 3. memcpy 함수 선언 (void* dest, const void* src, size_t n)
          // memcpy는 void 반환이므로 void 타입 사용
          let void_type = context.void_type();
          let memcpy_fn_type = void_type.fn_type(
            &[i8_ptr_type.into(), i8_ptr_type.into(), size_t_type.into()],
            false,
          );
          let memcpy_fn = module.add_function("memcpy", memcpy_fn_type, None);
          memcpy_fn.set_linkage(inkwell::module::Linkage::External);
          // Note: memcpy는 표준 C 라이브러리 함수로, 링커가 libc에서 자동으로 해석함
          // AOT 링킹 시 libc가 링크되어 이 심볼들이 해석됨 (aot.rs의 link_object_to_executable 참조)

          // 참고: 메모리 누수/해제 전략
          // Fix: Document memory ownership and cleanup strategy
          // - malloc으로 할당된 메모리는 반환값으로 사용되므로 호출자가 해제해야 함
          // - Y05c-11: JIT 실행 경로에서 반환된 문자열 포인터는 자동으로 해제됨 (jit.rs에서 free 호출)
          //   - jit.rs의 eval_with_llvm_inputs_* 함수들이 반환된 포인터를 읽은 후 free() 호출
          // - AOT 실행 경로에서는 호출자가 반환된 포인터를 해제해야 함
          //   - AOT/FFI 호출자는 `pnix_runtime_free_string`으로 해제 (allocator 일치 보장)
          // - 현재 구현: JIT 경로는 자동 해제 완료, AOT 경로는 호출자 해제

          // 4. 총 길이 계산 (size_t 타입 사용, 오버플로우 감지)
          let zero_size_t = size_t_type.const_int(0, false);
          let mut total_len = zero_size_t;
          let mut len_overflow = i1_type.const_int(0, false);
          for (idx, val) in input_vals.iter().enumerate() {
            let str_ptr = val.as_string()?;
            let len_call = b!(builder.build_call(strlen_fn, &[str_ptr.into()], "strlen_call"));
            let len_val = match len_call.try_as_basic_value() {
              inkwell::values::ValueKind::Basic(value) => value.into_int_value(),
              _ => {
                return Err(
                  LlvmRuntimeError::ConfigError("strlen returned void".to_string()).into(),
                )
              }
            };
            // CRITICAL: strlen 오버플로우 미검사 수정
            // strlen 결과가 MAX_STRING_LENGTH를 초과하는지 체크
            const MAX_STRING_LENGTH: u64 = 1024 * 1024; // 1MB
            let max_len_val = size_t_type.const_int(MAX_STRING_LENGTH, false);
            let len_too_large = b!(builder.build_int_compare(
              IntPredicate::UGT,
              len_val,
              max_len_val,
              "strlen_too_large"
            ));
            let clamped_len =
              b!(builder.build_select(len_too_large, max_len_val, len_val, "strlen_clamped"))
                .into_int_value();
            let (next_len, overflow) = build_int_overflow_intrinsic(
              module,
              &builder,
              "llvm.uadd.with.overflow",
              total_len,
              clamped_len,
              &format!("add_len_{}", idx),
            )?;
            total_len = next_len;
            let overflow_or = b!(builder.build_or(len_overflow, overflow, "len_overflow"));
            len_overflow =
              b!(builder.build_or(overflow_or, len_too_large, "len_overflow_or_too_large"));
            // strlen이 너무 크면 에러 코드 설정
            let current_error = b!(builder.build_load(
              i32_type,
              runtime_error_code.as_pointer_value(),
              "runtime_error_code"
            ))
            .into_int_value();
            let strlen_error = i32_type.const_int(RUNTIME_ERROR_STRING_LEN_OVERFLOW as u64, false);
            let merged_error = b!(builder.build_select(
              len_too_large,
              strlen_error,
              current_error,
              "strlen_error_code",
            ))
            .into_int_value();
            let _ = b!(builder.build_store(runtime_error_code.as_pointer_value(), merged_error));
          }

          // 5. 메모리 할당 (total_len + 1 for null terminator, size_t 타입 사용)
          let (alloc_size_raw, alloc_overflow) = build_int_overflow_intrinsic(
            module,
            &builder,
            "llvm.uadd.with.overflow",
            total_len,
            size_t_type.const_int(1, false),
            "alloc_size",
          )?;
          len_overflow = b!(builder.build_or(len_overflow, alloc_overflow, "alloc_overflow"));
          let alloc_size = b!(builder.build_select(
            len_overflow,
            size_t_type.const_int(1, false),
            alloc_size_raw,
            "alloc_size_safe",
          ))
          .into_int_value();
          let malloc_call = b!(builder.build_call(malloc_fn, &[alloc_size.into()], "malloc_call"));
          let result_ptr = match malloc_call.try_as_basic_value() {
            inkwell::values::ValueKind::Basic(value) => value.into_pointer_value(),
            _ => {
              return Err(LlvmRuntimeError::ConfigError("malloc returned void".to_string()).into())
            }
          };
          let malloc_is_null = b!(builder.build_is_null(result_ptr, "malloc_is_null"));
          let no_overflow = b!(builder.build_not(len_overflow, "concat_no_overflow"));
          let oom_condition = b!(builder.build_and(malloc_is_null, no_overflow, "concat_oom"));
          set_runtime_error(
            len_overflow,
            RUNTIME_ERROR_STRING_LEN_OVERFLOW,
            "concat_len_overflow",
          )?;
          set_runtime_error(oom_condition, RUNTIME_ERROR_OOM, "concat_oom")?;

          let concat_invalid = b!(builder.build_or(len_overflow, malloc_is_null, "concat_invalid"));
          let concat_ok = b!(builder.build_not(concat_invalid, "concat_ok"));
          let concat_id = node_values.len();
          let copy_block =
            context.append_basic_block(function, &format!("concat_copy_{}", concat_id));
          let fail_block =
            context.append_basic_block(function, &format!("concat_fail_{}", concat_id));
          let done_block =
            context.append_basic_block(function, &format!("concat_done_{}", concat_id));
          b!(builder.build_conditional_branch(concat_ok, copy_block, fail_block));

          // 6. 각 문자열 복사 (성공 경로)
          builder.position_at_end(copy_block);
          let mut current_ptr = result_ptr;
          for val in &input_vals {
            let str_ptr = val.as_string()?;
            let len_call = b!(builder.build_call(strlen_fn, &[str_ptr.into()], "strlen_call"));
            let len_val = match len_call.try_as_basic_value() {
              inkwell::values::ValueKind::Basic(value) => value.into_int_value(),
              _ => {
                return Err(
                  LlvmRuntimeError::ConfigError("strlen returned void".to_string()).into(),
                )
              }
            };
            b!(builder.build_call(
              memcpy_fn,
              &[current_ptr.into(), str_ptr.into(), len_val.into()],
              "memcpy_call",
            ));
            // current_ptr을 len_val만큼 증가 (i8 포인터이므로 len_val이 바이트 수)
            current_ptr =
              unsafe { b!(builder.build_gep(i8_type, current_ptr, &[len_val], "next_ptr")) };
          }

          // 7. null terminator 추가 (성공 경로)
          let null_byte = i8_type.const_int(0, false);
          b!(builder.build_store(current_ptr, null_byte));
          b!(builder.build_unconditional_branch(done_block));

          // 실패 경로: 가능한 경우 빈 문자열로 초기화
          builder.position_at_end(fail_block);
          let fail_store_block =
            context.append_basic_block(function, &format!("concat_fail_store_{}", concat_id));
          let fail_done_block =
            context.append_basic_block(function, &format!("concat_fail_done_{}", concat_id));
          b!(builder.build_conditional_branch(malloc_is_null, fail_done_block, fail_store_block));

          builder.position_at_end(fail_store_block);
          b!(builder.build_store(result_ptr, null_byte));
          b!(builder.build_unconditional_branch(fail_done_block));

          builder.position_at_end(fail_done_block);
          b!(builder.build_unconditional_branch(done_block));

          builder.position_at_end(done_block);
          let result_ptr = b!(builder.build_select(
            malloc_is_null,
            i8_ptr_type.const_null(),
            result_ptr,
            "concat_result_ptr",
          ))
          .into_pointer_value();

          LlvmValue::String(result_ptr)
        }
        _ => {
          return Err(LlvmRuntimeError::ConfigError(format!(
                    "Unsupported morphism operation '{}' (node: '{}', input_type: '{}', output_type: '{}'). \
                    Supported operations: add, sub, mul, div, mod, pow (Int/Float), bitwise (shl/shr/bitand/bitor/bitxor/bitnot, Int only, LLVM-only), \
                    comparisons (eq/ne/lt/le/gt/ge), if/select, float math (sin/cos/sqrt/floor/ceil), and string concat (limited). \
                    Operation '{}' is not yet implemented in LLVM lowering. \
                    Note: Bitwise operations are LLVM-only and not available in other runtimes. \
                    See docs/llvm-subset.md for the supported subset.",
                    morphism_name, to_node, morphism.input, morphism.output, morphism_name
                ))
                .into());
        }
      };
      NodeValue::Single(result)
    };

    let error_after = load_runtime_error("runtime_error_after")?;
    let failed =
      b!(builder.build_int_compare(IntPredicate::NE, error_before, error_after, "node_failed"));
    node_failed.insert(to_node.clone(), failed);

    if node.kind == NodeKind::Gate {
      let gate_value = match &result {
        NodeValue::Single(val) => match val.kind() {
          ValueKind::Bool => val.as_bool()?,
          _ => {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Gate node '{}' must return Bool, got {:?}",
                to_node,
                val.kind()
              ))
              .into(),
            );
          }
        },
        NodeValue::Multi(values) => {
          return Err(
            LlvmRuntimeError::ConfigError(format!(
              "Gate node '{}' must return single Bool output, got {} outputs",
              to_node,
              values.len()
            ))
            .into(),
          );
        }
      };
      gate_results.insert(to_node.clone(), gate_value);
    }

    node_values.insert(to_node.clone(), result);
  }

  let output_node = fx_module
    .nodes
    .iter()
    .find(|n| n.name == "result")
    .or_else(|| fx_module.nodes.last());

  let to_output_basic = |kind: ValueKind,
                         value: LlvmValue<'ctx>|
   -> Result<inkwell::values::BasicValueEnum<'ctx>, RuntimeError> {
    let basic = match kind {
      ValueKind::Int => value.as_int()?.into(),
      ValueKind::Float => value.as_float()?.into(),
      ValueKind::Bool => {
        b!(builder.build_int_z_extend(value.as_bool()?, i32_type, "bool_to_i32_main")).into()
      }
      ValueKind::String => value.as_string()?.into(),
      ValueKind::List => value.as_list()?.into(),
      ValueKind::AttrSet => value.as_attrset()?.into(),
    };
    Ok(basic)
  };

  let return_value: inkwell::values::BasicValueEnum = match &output_kind {
    OutputKind::Scalar(kind) => {
      let return_value = if let Some(node) = output_node {
        match node_values.get(&node.name) {
          Some(val) => Some(val.as_single()?),
          None => None,
        }
      } else {
        None
      };
      match (*kind, return_value) {
        (ValueKind::Int, Some(value)) => value.as_int()?.into(),
        (ValueKind::Float, Some(value)) => value.as_float()?.into(),
        (ValueKind::Bool, Some(value)) => {
          b!(builder.build_int_z_extend(value.as_bool()?, i32_type, "bool_to_i32_main")).into()
        }
        (ValueKind::String, Some(value)) => value.as_string()?.into(),
        (ValueKind::List, Some(value)) => value.as_list()?.into(),
        (ValueKind::AttrSet, Some(value)) => value.as_attrset()?.into(),
        (ValueKind::Int, None) => i64_type.const_int(0, false).into(),
        (ValueKind::Float, None) => f64_type.const_float(0.0).into(),
        (ValueKind::Bool, None) => i32_type.const_int(0, false).into(),
        (ValueKind::String, None) => {
          // Return empty string constant
          // Note: Full string constant initialization requires proper LLVM IR construction
          // For now, create a placeholder - full implementation deferred
          let empty_array_type = i8_type.array_type(1);
          let empty_global = module.add_global(empty_array_type, None, "empty_str");
          empty_global.set_linkage(inkwell::module::Linkage::Internal);
          empty_global.set_constant(true);
          // Set initializer to zero to avoid LLVM verification error
          let zero_array = empty_array_type.const_zero();
          empty_global.set_initializer(&zero_array);
          // CRITICAL: String GEP 안전하지 않은 포인터 연산 수정
          // GEP 인덱스는 포인터 크기에 맞는 타입 사용 (일반적으로 i64)
          // 빈 문자열 배열의 첫 번째 바이트를 가리키므로 [0, 0] 인덱스는 안전
          let zero_i64 = context.i64_type().const_int(0, false);
          let indices = [zero_i64, zero_i64];
          let empty_ptr = unsafe {
            b!(builder.build_gep(
              empty_array_type,
              empty_global.as_pointer_value(),
              &indices,
              "empty_str_ptr"
            ))
          };
          empty_ptr.into()
        }
        (ValueKind::List, None) | (ValueKind::AttrSet, None) => i8_ptr_type.const_null().into(),
      }
    }
    OutputKind::Tuple(kinds) => {
      let tuple_struct_type = tuple_struct_type
        .as_ref()
        .ok_or_else(|| LlvmRuntimeError::ConfigError("missing tuple output type".to_string()))?;
      let values = if let Some(node) = output_node {
        match node_values.get(&node.name) {
          Some(NodeValue::Multi(values)) => {
            if values.len() != kinds.len() {
              return Err(
                LlvmRuntimeError::ConfigError(format!(
                  "Output node '{}' returned {} values, expected {}",
                  node.name,
                  values.len(),
                  kinds.len()
                ))
                .into(),
              );
            }
            values.clone()
          }
          Some(NodeValue::Single(_)) => {
            return Err(
              LlvmRuntimeError::ConfigError(format!(
                "Output node '{}' must return {} values for tuple output",
                node.name,
                kinds.len()
              ))
              .into(),
            );
          }
          None => Vec::new(),
        }
      } else {
        Vec::new()
      };

      let mut struct_value = tuple_struct_type.const_zero();
      for (idx, kind) in kinds.iter().enumerate() {
        let value = values.get(idx).copied().unwrap_or_else(|| {
          default_value_for_kind(*kind, i64_type, f64_type, i1_type, i8_ptr_type)
        });
        let field = to_output_basic(*kind, value)?;
        let inserted =
          b!(builder.build_insert_value(struct_value, field, idx as u32, "tuple_insert"));
        struct_value = inserted.into_struct_value();
      }
      struct_value.into()
    }
  };

  b!(builder.build_return(Some(&return_value)));

  // Entry function input pointer type
  // Y05c-3: Mixed input types는 이미 위에서 검증됨 (has_ptr_input && has_numeric_input)
  // 따라서 여기서는 pointer-only 또는 numeric-only 케이스만 처리
  let entry_input_ptr: inkwell::types::BasicMetadataTypeEnum = if has_ptr_input {
    // Pointer-only: use i8** (array of string/json pointers)
    i8_ptr_type.ptr_type(AddressSpace::default()).into()
  } else {
    // Numeric-only: use numeric pointer type
    match numeric_kind {
      NumericKind::Int => i64_type.ptr_type(AddressSpace::default()).into(),
      NumericKind::Float => f64_type.ptr_type(AddressSpace::default()).into(),
    }
  };

  let entry_fn_type = match &output_kind {
    OutputKind::Scalar(kind) => {
      let entry_return_type: inkwell::types::BasicTypeEnum = match kind {
        ValueKind::Float => f64_type.into(),
        ValueKind::Int => i64_type.into(),
        ValueKind::Bool => i32_type.into(),
        ValueKind::String => i8_ptr_type.into(),
        ValueKind::List | ValueKind::AttrSet => i8_ptr_type.into(),
      };
      entry_return_type.fn_type(&[entry_input_ptr, i32_type.into()], false)
    }
    OutputKind::Tuple(_) => {
      let tuple_struct_type = tuple_struct_type
        .as_ref()
        .ok_or_else(|| LlvmRuntimeError::ConfigError("missing tuple output type".to_string()))?;
      let out_ptr_type = tuple_struct_type
        .ptr_type(AddressSpace::default())
        .as_basic_type_enum();
      void_type.fn_type(
        &[out_ptr_type.into(), entry_input_ptr, i32_type.into()],
        false,
      )
    }
  };
  let entry_fn = module.add_function("pnix_entry", entry_fn_type, None);
  let entry_block = context.append_basic_block(entry_fn, "entry");
  builder.position_at_end(entry_block);

  let (out_ptr, inputs_ptr, inputs_len) = if output_kind.is_tuple() {
    let out_ptr = entry_fn
      .get_nth_param(0)
      .ok_or_else(|| LlvmRuntimeError::ConfigError("missing entry output ptr".to_string()))?
      .into_pointer_value();
    let inputs_ptr = entry_fn
      .get_nth_param(1)
      .ok_or_else(|| LlvmRuntimeError::ConfigError("missing entry inputs ptr".to_string()))?
      .into_pointer_value();
    let inputs_len = entry_fn
      .get_nth_param(2)
      .ok_or_else(|| LlvmRuntimeError::ConfigError("missing entry inputs len".to_string()))?
      .into_int_value();
    (Some(out_ptr), inputs_ptr, inputs_len)
  } else {
    let inputs_ptr = entry_fn
      .get_nth_param(0)
      .ok_or_else(|| LlvmRuntimeError::ConfigError("missing entry inputs ptr".to_string()))?
      .into_pointer_value();
    let inputs_len = entry_fn
      .get_nth_param(1)
      .ok_or_else(|| LlvmRuntimeError::ConfigError("missing entry inputs len".to_string()))?
      .into_int_value();
    (None, inputs_ptr, inputs_len)
  };

  let entry_reset = b!(builder.build_store(
    runtime_error_code.as_pointer_value(),
    i32_type.const_int(0, false),
  ));
  let _ = entry_reset.set_volatile(true);

  // Y226: 입력 길이 검증 - AOT/FFI 안전성
  let expected_len = i32_type.const_int(fx_module.inputs.len() as u64, false);
  let len_mismatch = b!(builder.build_int_compare(
    inkwell::IntPredicate::NE,
    inputs_len,
    expected_len,
    "inputs_len_mismatch",
  ));
  let error_block = context.append_basic_block(entry_fn, "entry_len_error");
  let continue_block = context.append_basic_block(entry_fn, "entry_continue");
  b!(builder.build_conditional_branch(len_mismatch, error_block, continue_block));

  // 에러 블록: 런타임 에러 코드 설정 후 기본값 반환
  builder.position_at_end(error_block);
  let error_code = i32_type.const_int(RUNTIME_ERROR_INPUT_LEN_MISMATCH as u64, false);
  let error_store = b!(builder.build_store(runtime_error_code.as_pointer_value(), error_code));
  let _ = error_store.set_volatile(true);
  match &output_kind {
    OutputKind::Scalar(kind) => {
      let error_return: inkwell::values::BasicValueEnum = match kind {
        ValueKind::Int => i64_type.const_int(0, false).into(),
        ValueKind::Float => f64_type.const_float(0.0).into(),
        ValueKind::Bool => i32_type.const_int(0, false).into(),
        ValueKind::String => i8_ptr_type.const_null().into(),
        ValueKind::List | ValueKind::AttrSet => i8_ptr_type.const_null().into(),
      };
      b!(builder.build_return(Some(&error_return)));
    }
    OutputKind::Tuple(_) => {
      let tuple_struct_type = tuple_struct_type
        .as_ref()
        .ok_or_else(|| LlvmRuntimeError::ConfigError("missing tuple output type".to_string()))?;
      if let Some(out_ptr) = out_ptr {
        let zero_struct = tuple_struct_type.const_zero();
        b!(builder.build_store(out_ptr, zero_struct));
      }
      b!(builder.build_return(None));
    }
  }

  // 정상 블록: 입력 로딩 계속
  builder.position_at_end(continue_block);

  // Y05c-3: 타입별 로딩 처리
  // String 입력의 경우 i8**에서 i8*를 로드
  // Numeric 입력의 경우 numeric*에서 numeric 값을 로드
  // LOW: 입력 배열 접근 경계 미검증 수정 완료
  // 경계 검증: line 3209-3212에서 len_mismatch 체크로 inputs_len == fx_module.inputs.len() 보장
  // idx는 fx_module.inputs.iter().enumerate()로 생성되므로 항상 idx < fx_module.inputs.len()
  // 따라서 idx < inputs_len이 보장되어 오버런 불가능
  let inputs_elem_type: inkwell::types::BasicTypeEnum = if has_ptr_input {
    i8_ptr_type.into()
  } else {
    match numeric_kind {
      NumericKind::Int => i64_type.into(),
      NumericKind::Float => f64_type.into(),
    }
  };
  let mut call_args = Vec::with_capacity(input_params.len());
  for (idx, input) in fx_module.inputs.iter().enumerate() {
    let idx_val = i32_type.const_int(idx as u64, false);
    // Note: idx는 항상 fx_module.inputs.len()보다 작으므로 (enumerate로 생성)
    // len_mismatch 체크 후에는 idx < inputs_len이 보장됨
    let ptr = unsafe {
      b!(builder.build_gep(
        inputs_elem_type,
        inputs_ptr,
        &[idx_val],
        &format!("input_ptr_{}", idx)
      ))
    };

    let input_kind = type_name_to_kind(&input.ty);
    let loaded = match input_kind {
      Some(ValueKind::String) => {
        // String 입력: i8**에서 i8* 로드
        let str_ptr =
          b!(builder.build_load(inputs_elem_type, ptr, &format!("input_str_ptr_{}", idx)));
        str_ptr.into()
      }
      Some(ValueKind::Int) => {
        // Int 입력: i64*에서 i64 로드
        let int_val = b!(builder.build_load(inputs_elem_type, ptr, &format!("input_int_{}", idx)));
        int_val.into()
      }
      Some(ValueKind::Bool) => {
        // Y13a-7: Bool 입력 타입 보존 - i64로 로드된 후 i1로 변환
        // Bool 입력은 i64로 전달되지만, 내부적으로는 i1로 변환하여 Bool-only 조건과 호환
        let int_val_enum =
          b!(builder.build_load(inputs_elem_type, ptr, &format!("input_int_{}", idx)));
        let int_val = int_val_enum.into_int_value();
        // i64를 i1로 변환 (0이 아니면 true)
        let bool_val = b!(builder.build_int_compare(
          inkwell::IntPredicate::NE,
          int_val,
          i64_type.const_int(0, false),
          &format!("input_bool_{}", idx),
        ));
        bool_val.into()
      }
      Some(ValueKind::Float) => {
        // Float 입력: f64*에서 f64 로드
        let float_val =
          b!(builder.build_load(inputs_elem_type, ptr, &format!("input_float_{}", idx)));
        float_val.into()
      }
      Some(ValueKind::List) | Some(ValueKind::AttrSet) => {
        let ptr_val = b!(builder.build_load(inputs_elem_type, ptr, &format!("input_ptr_{}", idx)));
        ptr_val.into()
      }
      None => {
        return Err(
          LlvmRuntimeError::ConfigError(format!(
            "Unknown input type '{}' for input '{}'",
            input.ty, input.name
          ))
          .into(),
        );
      }
    };
    call_args.push(loaded);
  }
  let call = b!(builder.build_call(function, &call_args, "call_main"));

  match &output_kind {
    OutputKind::Tuple(_) => {
      let call_result = match call.try_as_basic_value() {
        inkwell::values::ValueKind::Basic(value) => value,
        _ => {
          return Err(
            LlvmRuntimeError::ConfigError("tuple entry call returned void".to_string()).into(),
          )
        }
      };
      if let Some(out_ptr) = out_ptr {
        b!(builder.build_store(out_ptr, call_result));
      }
      b!(builder.build_return(None));
    }
    OutputKind::Scalar(kind) => {
      let call_result = match call.try_as_basic_value() {
        inkwell::values::ValueKind::Basic(value) => Some(value),
        _ => None,
      };
      let entry_return: inkwell::values::BasicValueEnum = match kind {
        ValueKind::Float => match call_result {
          Some(value) => value.into_float_value().into(),
          None => f64_type.const_float(0.0).into(),
        },
        ValueKind::Int => match call_result {
          Some(value) => value.into_int_value().into(),
          None => i64_type.const_int(0, false).into(),
        },
        ValueKind::Bool => {
          let raw_val = match call_result {
            Some(value) => value.into_int_value(),
            None => i32_type.const_int(0, false),
          };
          let bool_val = if raw_val.get_type() == i32_type {
            raw_val
          } else {
            b!(builder.build_int_z_extend(raw_val, i32_type, "bool_to_i32"))
          };
          bool_val.into()
        }
        ValueKind::String => {
          let str_val = match call_result {
            Some(value) => value.into_pointer_value(),
            None => {
              // Create empty string constant
              // Note: Full string constant initialization requires proper LLVM IR construction
              // For now, create a placeholder - full implementation deferred
              let empty_array_type = i8_type.array_type(1);
              let empty_global = module.add_global(empty_array_type, None, "empty_str_entry");
              empty_global.set_linkage(inkwell::module::Linkage::Internal);
              empty_global.set_constant(true);
              // Set initializer to zero to avoid LLVM verification error
              let zero_array = empty_array_type.const_zero();
              empty_global.set_initializer(&zero_array);
              // CRITICAL: GEP 인덱스는 포인터 크기에 맞는 타입 사용 (일반적으로 i64)
              // i32 대신 i64 사용하여 포인터 크기와 일치
              let zero_i64 = context.i64_type().const_int(0, false);
              let indices = [zero_i64, zero_i64];
              unsafe {
                b!(builder.build_gep(
                  empty_array_type,
                  empty_global.as_pointer_value(),
                  &indices,
                  "empty_str_entry_ptr",
                ))
              }
            }
          };
          str_val.into()
        }
        ValueKind::List | ValueKind::AttrSet => match call_result {
          Some(value) => value.into_pointer_value().into(),
          None => i8_ptr_type.const_null().into(),
        },
      };

      b!(builder.build_return(Some(&entry_return)));
    }
  }

  module.verify().map_err(|e| {
    LlvmRuntimeError::VerificationError(format!("LLVM IR verification failed: {}", e))
  })?;

  Ok(())
}

/// Runtime configuration for JIT/AOT execution
///
/// Controls JIT compilation and AOT compilation behavior.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
  /// JIT configuration
  pub jit: JitConfig,
  /// AOT configuration
  pub aot: AotConfig,
  /// Enable JIT mode (default: true)
  pub jit_enabled: bool,
  /// Enable AOT mode (default: false)
  pub aot_enabled: bool,
}

impl Default for RuntimeConfig {
  fn default() -> Self {
    Self {
      jit: JitConfig::default(),
      aot: AotConfig::default(),
      jit_enabled: true,
      aot_enabled: false,
    }
  }
}

#[cfg(test)]
mod tests;
