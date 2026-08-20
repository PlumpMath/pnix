//! Runtime API: PNIX 런타임 평가 API

/// 평가 설정: PNIX 표현식 평가를 위한 설정
#[derive(Debug, Clone)]
pub struct EvalConfig {
  /// 결정론적 모드 여부
  pub deterministic: bool,
  /// 랜덤 시드 (결정론적 모드에서 사용)
  pub seed: Option<u64>,
  /// 현재 시간 (밀리초, None이면 기본값 사용)
  pub now_ms: Option<i64>,
  /// 클럭 스텝 간격 (밀리초, None이면 기본값 사용)
  pub clock_step_ms: Option<i64>,
  /// 부동소수점 패턴 허용 오차
  pub float_pattern_tolerance: Option<f64>,
  /// 상세 모드 플래그 (traceVerbose 내장 함수용)
  pub verbose: bool,
}

impl Default for EvalConfig {
  fn default() -> Self {
    Self {
      deterministic: true,
      seed: None,
      now_ms: None,
      clock_step_ms: None,
      float_pattern_tolerance: None,
      verbose: false,
    }
  }
}

impl EvalConfig {
  pub fn nondeterministic(mut self) -> Self {
    self.deterministic = false;
    self
  }

  pub fn with_seed(mut self, seed: u64) -> Self {
    self.seed = Some(seed);
    self
  }

  pub fn with_now_ms(mut self, now_ms: i64) -> Self {
    self.now_ms = Some(now_ms);
    self
  }

  pub fn with_clock_step_ms(mut self, clock_step_ms: i64) -> Self {
    self.clock_step_ms = Some(clock_step_ms);
    self
  }

  pub fn with_float_pattern_tolerance(mut self, tolerance: f64) -> Self {
    self.float_pattern_tolerance = Some(tolerance);
    self
  }

  pub fn with_verbose(mut self, verbose: bool) -> Self {
    self.verbose = verbose;
    self
  }
}

/// FRP 설정: 함수형 반응형 프로그래밍 실행을 위한 설정
#[derive(Debug, Clone, Default)]
pub struct FrpConfig {
  /// 현재 틱 인덱스
  pub tick_index: u64,
  /// 랜덤 시드
  pub seed: Option<u64>,
  /// 현재 시간 (밀리초)
  pub now_ms: Option<i64>,
  /// 클럭 스텝 간격 (밀리초)
  pub clock_step_ms: Option<i64>,
}

impl FrpConfig {
  pub fn with_tick_index(mut self, tick_index: u64) -> Self {
    self.tick_index = tick_index;
    self
  }

  pub fn with_seed(mut self, seed: u64) -> Self {
    self.seed = Some(seed);
    self
  }

  pub fn with_now_ms(mut self, now_ms: i64) -> Self {
    self.now_ms = Some(now_ms);
    self
  }

  pub fn with_clock_step_ms(mut self, clock_step_ms: i64) -> Self {
    self.clock_step_ms = Some(clock_step_ms);
    self
  }
}

/// CT 설정: 카테고리 이론 검증을 위한 설정
#[derive(Debug, Clone)]
pub struct CtConfig {
  /// 엄격 모드 여부
  pub strict: bool,
  /// 랜덤 시드
  pub seed: Option<u64>,
  /// 현재 시간 (밀리초)
  pub now_ms: Option<i64>,
  /// 클럭 스텝 간격 (밀리초)
  pub clock_step_ms: Option<i64>,
}

impl Default for CtConfig {
  fn default() -> Self {
    Self {
      strict: true,
      seed: None,
      now_ms: None,
      clock_step_ms: None,
    }
  }
}

impl CtConfig {
  pub fn permissive(mut self) -> Self {
    self.strict = false;
    self
  }

  pub fn with_seed(mut self, seed: u64) -> Self {
    self.seed = Some(seed);
    self
  }

  pub fn with_now_ms(mut self, now_ms: i64) -> Self {
    self.now_ms = Some(now_ms);
    self
  }

  pub fn with_clock_step_ms(mut self, clock_step_ms: i64) -> Self {
    self.clock_step_ms = Some(clock_step_ms);
    self
  }
}

/// 평가 결과: 평가된 값 포함
#[derive(Debug, Clone)]
pub struct EvalResult<V> {
  /// 평가된 값
  pub value: V,
}

/// FRP 틱 결과: 한 틱 동안의 출력 포함
#[derive(Debug, Clone)]
pub struct FrpTickResult<O> {
  /// 틱 출력
  pub output: O,
}

/// CT 검증 결과: 카테고리 이론 검증 결과 및 다이어그램
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CtCheckResult {
  /// 검증 성공 여부
  #[serde(rename = "ok", alias = "success")]
  pub success: bool,
  /// 검증 노트 (메시지 목록)
  pub notes: Vec<String>,
  /// 추출된 다이어그램 (있는 경우)
  pub diagram: Option<CtDiagramOutput>,
}

/// CT 다이어그램 출력: 검증 결과에서 추출된 카테고리 이론 다이어그램
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CtDiagramOutput {
  /// 객체 목록
  pub objects: Vec<CtObjectInfo>,
  /// 사상(morphism) 목록
  pub morphisms: Vec<CtMorphismInfo>,
}

/// CT 객체 메타데이터: 카테고리 이론 객체 정보
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CtObjectInfo {
  /// 객체 ID
  pub id: usize,
  /// 객체 이름
  pub name: String,
  /// CT 타입
  pub ct_type: String,
}

/// CT 사상 메타데이터: 카테고리 이론 사상(morphism) 정보
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CtMorphismInfo {
  /// 사상 이름
  pub name: String,
  /// 소스 객체
  pub source: String,
  /// 타겟 객체
  pub target: String,
}

/// CT 스펙: 검증할 표현식 및 옵션
#[derive(Debug, Clone)]
pub struct CtSpec {
  /// 평가할 표현식 (예: "sin(t)", "(mod (floor t) 60)")
  pub expr: String,
  /// 표현식에서 CT 다이어그램 추출 여부
  pub extract_diagram: bool,
}

impl CtSpec {
  pub fn new(expr: impl Into<String>) -> Self {
    Self {
      expr: expr.into(),
      extract_diagram: true,
    }
  }

  pub fn with_diagram(mut self, extract: bool) -> Self {
    self.extract_diagram = extract;
    self
  }
}

impl Default for CtSpec {
  fn default() -> Self {
    Self {
      expr: String::new(),
      extract_diagram: true,
    }
  }
}

// ========================================
// U03: Unified Error Type
// ========================================

/// Error category for adapter errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterErrorKind {
  /// Feature not yet implemented
  Unimplemented,
  /// Feature not supported
  Unsupported,
  /// Conversion failed
  ConversionError,
}

impl std::fmt::Display for AdapterErrorKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Unimplemented => write!(f, "unimplemented"),
      Self::Unsupported => write!(f, "unsupported"),
      Self::ConversionError => write!(f, "conversion error"),
    }
  }
}

/// Error category for runtime execution errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionErrorKind {
  /// SSA interpreter error
  SSA,
  /// FRP signal graph error
  FRP,
  /// Category Theory verification error
  CT,
  /// LLVM JIT/AOT error
  LLVM,
  /// Symbolic computation error
  Symbolic,
  /// IR evaluation error
  IrEval,
  /// Generic execution error
  Generic,
}

impl std::fmt::Display for ExecutionErrorKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::SSA => write!(f, "SSA"),
      Self::FRP => write!(f, "FRP"),
      Self::CT => write!(f, "CT"),
      Self::LLVM => write!(f, "LLVM"),
      Self::Symbolic => write!(f, "symbolic"),
      Self::IrEval => write!(f, "IR eval"),
      Self::Generic => write!(f, "runtime"),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorCode(pub &'static str);

impl std::fmt::Display for ErrorCode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

pub mod error_codes {
  use super::ErrorCode;

  pub const RUNTIME_UNIMPLEMENTED: ErrorCode = ErrorCode("E0401");
  pub const RUNTIME_MESSAGE: ErrorCode = ErrorCode("E0402");
  pub const RUNTIME_ADAPTER: ErrorCode = ErrorCode("E0403");
  pub const RUNTIME_CANCELLED: ErrorCode = ErrorCode("E0404");
  pub const RUNTIME_EXEC_SSA: ErrorCode = ErrorCode("E0410");
  pub const RUNTIME_EXEC_FRP: ErrorCode = ErrorCode("E0411");
  pub const RUNTIME_EXEC_CT: ErrorCode = ErrorCode("E0412");
  pub const RUNTIME_EXEC_LLVM: ErrorCode = ErrorCode("E0413");
  pub const RUNTIME_EXEC_SYMBOLIC: ErrorCode = ErrorCode("E0414");
  pub const RUNTIME_EXEC_IR_EVAL: ErrorCode = ErrorCode("E0415");
  pub const RUNTIME_EXEC_GENERIC: ErrorCode = ErrorCode("E0416");
  pub const RUNTIME_SETO_ABI: ErrorCode = ErrorCode("E0461");
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTraceFrame {
  pub node: String,
  pub morphism: String,
  pub message: String,
}

const AUTO_TRACE_NODE: &str = "<unknown>";
const AUTO_TRACE_MORPHISM: &str = "<runtime>";

fn auto_trace_frame(message: &str) -> RuntimeTraceFrame {
  RuntimeTraceFrame::new(AUTO_TRACE_NODE, AUTO_TRACE_MORPHISM, message.to_string())
}

fn is_auto_trace_frame(frame: &RuntimeTraceFrame) -> bool {
  frame.node == AUTO_TRACE_NODE && frame.morphism == AUTO_TRACE_MORPHISM
}

fn strip_auto_trace(trace: &mut Vec<RuntimeTraceFrame>) {
  if trace.len() == 1 && is_auto_trace_frame(&trace[0]) {
    trace.clear();
  }
}

impl RuntimeTraceFrame {
  pub fn new(
    node: impl Into<String>,
    morphism: impl Into<String>,
    message: impl Into<String>,
  ) -> Self {
    Self {
      node: node.into(),
      morphism: morphism.into(),
      message: message.into(),
    }
  }
}

impl std::fmt::Display for RuntimeTraceFrame {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "at node {}, morphism {} failed because {}",
      self.node, self.morphism, self.message
    )
  }
}

/// Unified runtime error type for all Pnix runtime operations.
///
/// This error type implements `std::error::Error` for standard Rust
/// error handling compatibility, including use with `?` operator
/// and error chaining.
///
/// # Error Categories
///
/// - `Unimplemented`: Feature not yet implemented (static message)
/// - `Message`: Generic error message (backward compatibility)
/// - `Adapter`: IR adapter conversion errors with category
/// - `Execution`: Runtime execution errors with category
///
/// # Example
///
/// ```
/// use pnix_runtime_api::{RuntimeError, ExecutionErrorKind};
///
/// fn example() -> Result<(), RuntimeError> {
///     Err(RuntimeError::execution(ExecutionErrorKind::SSA, "division by zero"))
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeErrorSource {
  message: String,
}

impl RuntimeErrorSource {
  pub fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }
}

impl From<String> for RuntimeErrorSource {
  fn from(value: String) -> Self {
    Self::new(value)
  }
}

impl From<&str> for RuntimeErrorSource {
  fn from(value: &str) -> Self {
    Self::new(value)
  }
}

impl std::fmt::Display for RuntimeErrorSource {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl std::error::Error for RuntimeErrorSource {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
  /// Feature not yet implemented (backward compatibility)
  Unimplemented {
    code: ErrorCode,
    area: &'static str,
    source: Option<RuntimeErrorSource>,
    trace: Vec<RuntimeTraceFrame>,
  },
  /// Generic error message (backward compatibility)
  Message {
    code: ErrorCode,
    message: String,
    source: Option<RuntimeErrorSource>,
    trace: Vec<RuntimeTraceFrame>,
  },
  /// Adapter conversion error with category
  Adapter {
    code: ErrorCode,
    kind: AdapterErrorKind,
    message: String,
    source: Option<RuntimeErrorSource>,
    trace: Vec<RuntimeTraceFrame>,
  },
  /// Runtime execution error with category
  Execution {
    code: ErrorCode,
    kind: ExecutionErrorKind,
    message: String,
    source: Option<RuntimeErrorSource>,
    trace: Vec<RuntimeTraceFrame>,
  },
}

impl RuntimeError {
  /// Create an unimplemented error (backward compatible)
  pub fn unimplemented(area: &'static str) -> Self {
    Self::Unimplemented {
      code: error_codes::RUNTIME_UNIMPLEMENTED,
      area,
      source: None,
      trace: vec![auto_trace_frame(area)],
    }
  }

  /// Create a generic message error (backward compatible)
  pub fn message(message: impl Into<String>) -> Self {
    let message = message.into();
    Self::Message {
      code: error_codes::RUNTIME_MESSAGE,
      message: message.clone(),
      source: None,
      trace: vec![auto_trace_frame(&message)],
    }
  }

  /// Create a message error with a custom code.
  pub fn message_with_code(code: ErrorCode, message: impl Into<String>) -> Self {
    let message = message.into();
    Self::Message {
      code,
      message: message.clone(),
      source: None,
      trace: vec![auto_trace_frame(&message)],
    }
  }

  /// Create a cancellation error.
  pub fn cancelled(message: impl Into<String>) -> Self {
    Self::message_with_code(error_codes::RUNTIME_CANCELLED, message)
  }

  /// Create an adapter error with category
  pub fn adapter(kind: AdapterErrorKind, message: impl Into<String>) -> Self {
    let message = message.into();
    Self::Adapter {
      code: error_codes::RUNTIME_ADAPTER,
      kind,
      message: message.clone(),
      source: None,
      trace: vec![auto_trace_frame(&message)],
    }
  }

  /// Create an execution error with category
  pub fn execution(kind: ExecutionErrorKind, message: impl Into<String>) -> Self {
    let message = message.into();
    Self::Execution {
      code: execution_code(kind),
      kind,
      message: message.clone(),
      source: None,
      trace: vec![auto_trace_frame(&message)],
    }
  }

  /// Attach a source error message.
  pub fn with_source(self, source: impl Into<RuntimeErrorSource>) -> Self {
    let source = Some(source.into());
    match self {
      Self::Unimplemented {
        area, code, trace, ..
      } => Self::Unimplemented {
        area,
        code,
        source,
        trace,
      },
      Self::Message {
        message,
        code,
        trace,
        ..
      } => Self::Message {
        message,
        code,
        source,
        trace,
      },
      Self::Adapter {
        kind,
        message,
        code,
        trace,
        ..
      } => Self::Adapter {
        kind,
        message,
        code,
        source,
        trace,
      },
      Self::Execution {
        kind,
        message,
        code,
        trace,
        ..
      } => Self::Execution {
        kind,
        message,
        code,
        source,
        trace,
      },
    }
  }

  /// Attach an optional source error message.
  pub fn with_source_opt(self, source: Option<RuntimeErrorSource>) -> Self {
    match source {
      Some(src) => self.with_source(src),
      None => self,
    }
  }

  /// Get the error kind as a string (for logging/display)
  pub fn kind_str(&self) -> &'static str {
    match self {
      Self::Unimplemented { .. } => "unimplemented",
      Self::Message { .. } => "error",
      Self::Adapter { .. } => "adapter",
      Self::Execution { .. } => "execution",
    }
  }

  pub fn code(&self) -> ErrorCode {
    match self {
      Self::Unimplemented { code, .. }
      | Self::Message { code, .. }
      | Self::Adapter { code, .. }
      | Self::Execution { code, .. } => *code,
    }
  }

  pub fn source(&self) -> Option<&RuntimeErrorSource> {
    match self {
      Self::Unimplemented { source, .. }
      | Self::Message { source, .. }
      | Self::Adapter { source, .. }
      | Self::Execution { source, .. } => source.as_ref(),
    }
  }

  pub fn trace(&self) -> &[RuntimeTraceFrame] {
    match self {
      Self::Unimplemented { trace, .. }
      | Self::Message { trace, .. }
      | Self::Adapter { trace, .. }
      | Self::Execution { trace, .. } => trace.as_slice(),
    }
  }

  pub fn with_trace(mut self, trace: Vec<RuntimeTraceFrame>) -> Self {
    match &mut self {
      Self::Unimplemented { trace: slot, .. }
      | Self::Message { trace: slot, .. }
      | Self::Adapter { trace: slot, .. }
      | Self::Execution { trace: slot, .. } => {
        *slot = trace;
      }
    }
    self
  }

  pub fn push_trace_frame(mut self, frame: RuntimeTraceFrame) -> Self {
    match &mut self {
      Self::Unimplemented { trace, .. }
      | Self::Message { trace, .. }
      | Self::Adapter { trace, .. }
      | Self::Execution { trace, .. } => {
        strip_auto_trace(trace);
        trace.push(frame);
      }
    }
    self
  }

  pub fn trace_is_auto_only(&self) -> bool {
    let trace = self.trace();
    trace.len() == 1 && is_auto_trace_frame(&trace[0])
  }
}

fn execution_code(kind: ExecutionErrorKind) -> ErrorCode {
  match kind {
    ExecutionErrorKind::SSA => error_codes::RUNTIME_EXEC_SSA,
    ExecutionErrorKind::FRP => error_codes::RUNTIME_EXEC_FRP,
    ExecutionErrorKind::CT => error_codes::RUNTIME_EXEC_CT,
    ExecutionErrorKind::LLVM => error_codes::RUNTIME_EXEC_LLVM,
    ExecutionErrorKind::Symbolic => error_codes::RUNTIME_EXEC_SYMBOLIC,
    ExecutionErrorKind::IrEval => error_codes::RUNTIME_EXEC_IR_EVAL,
    ExecutionErrorKind::Generic => error_codes::RUNTIME_EXEC_GENERIC,
  }
}

fn write_trace(f: &mut std::fmt::Formatter<'_>, trace: &[RuntimeTraceFrame]) -> std::fmt::Result {
  for frame in trace {
    write!(f, "\n  {}", frame)?;
  }
  Ok(())
}

impl std::fmt::Display for RuntimeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let result = match self {
      Self::Unimplemented { code, area, .. } => {
        write!(f, "[{}] unimplemented: {}", code, area)
      }
      Self::Message { code, message, .. } => write!(f, "[{}] {}", code, message),
      Self::Adapter {
        code,
        kind,
        message,
        ..
      } => write!(f, "[{}] adapter {}: {}", code, kind, message),
      Self::Execution {
        code,
        kind,
        message,
        ..
      } => write!(f, "[{}] {} error: {}", code, kind, message),
    };
    result?;
    write_trace(f, self.trace())?;
    Ok(())
  }
}

impl std::error::Error for RuntimeError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::Unimplemented { source, .. }
      | Self::Message { source, .. }
      | Self::Adapter { source, .. }
      | Self::Execution { source, .. } => source.as_ref().map(|err| err as &dyn std::error::Error),
    }
  }
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

pub trait EvalEngine {
  type Module;
  type Value;

  fn eval(
    &mut self,
    module: &Self::Module,
    config: &EvalConfig,
  ) -> RuntimeResult<EvalResult<Self::Value>>;
}

// ========================================
// E03a: Eval Patch (draft)
// ========================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvalPatchResult {
  #[serde(rename = "ok", alias = "success")]
  pub success: bool,
  pub message: String,
}

impl EvalPatchResult {
  pub fn ok() -> Self {
    Self {
      success: true,
      message: String::new(),
    }
  }

  pub fn error(msg: impl Into<String>) -> Self {
    Self {
      success: false,
      message: msg.into(),
    }
  }
}

/// Eval runtime patch hook (draft)
pub trait EvalPatchable {
  type Patch;

  fn apply_patch(&mut self, patch: &Self::Patch) -> RuntimeResult<EvalPatchResult>;

  fn apply_patches(&mut self, patches: &[Self::Patch]) -> RuntimeResult<Vec<EvalPatchResult>> {
    let mut results = Vec::with_capacity(patches.len());
    for patch in patches {
      results.push(self.apply_patch(patch)?);
    }
    Ok(results)
  }
}

pub trait FrpEngine {
  type Graph;
  type Input;
  type Output;

  fn tick(
    &mut self,
    graph: &Self::Graph,
    input: Self::Input,
    config: &FrpConfig,
  ) -> RuntimeResult<FrpTickResult<Self::Output>>;
}

pub trait CtRuntime {
  type Spec;

  fn verify(&mut self, spec: &Self::Spec, config: &CtConfig) -> RuntimeResult<CtCheckResult>;
}

// ========================================
// E23c: FRP Patch Runtime Hook
// ========================================

/// Patch operation for hot-swap (E23)
#[derive(Debug, Clone)]
pub enum FrpPatchOp {
  /// Add a new signal
  AddSignal { name: String, kind: String },
  /// Remove a signal
  RemoveSignal { name: String },
  /// Update signal expression
  UpdateSignal { name: String, expr: Option<String> },
  /// Set constant numeric value (FRP signals are f64-only)
  SetConstant { name: String, value: f64 },
  /// Set external numeric input (FRP signals are f64-only)
  SetInput { name: String, value: f64 },
  /// Reset state to initial
  ResetState { name: String },
}

/// Patch result from FRP runtime
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrpPatchResult {
  #[serde(rename = "ok", alias = "success")]
  pub success: bool,
  pub message: String,
}

impl FrpPatchResult {
  pub fn ok() -> Self {
    Self {
      success: true,
      message: String::new(),
    }
  }

  pub fn error(msg: impl Into<String>) -> Self {
    Self {
      success: false,
      message: msg.into(),
    }
  }
}

/// State snapshot for migration (E23)
#[derive(Debug, Clone, Default)]
pub struct FrpStateInfo {
  /// Signal name -> value mappings
  pub signal_values: Vec<(String, f64)>,
  /// State name -> value mappings
  pub state_values: Vec<(String, f64)>,
  /// Current time
  pub current_time: f64,
  /// Tick count
  pub tick_count: u64,
}

/// Migration strategy for hot-swap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrpMigrationStrategy {
  /// Match signals by name and preserve values
  #[default]
  MatchByName,
  /// Preserve all state from snapshot
  Preserve,
  /// Reset all state to defaults
  Reset,
}

/// FRP runtime that supports hot-swap/patching (E23c)
pub trait FrpPatchable {
  /// Take a state snapshot for migration
  fn snapshot(&self) -> FrpStateInfo;

  /// Restore state from snapshot with migration strategy
  fn restore(&mut self, snapshot: &FrpStateInfo, strategy: FrpMigrationStrategy);

  /// Apply a single patch operation
  fn apply_patch(&mut self, patch: &FrpPatchOp) -> FrpPatchResult;

  /// Apply multiple patches (best-effort, not guaranteed to be atomic).
  /// Returns per-patch results in order.
  fn apply_patches(&mut self, patches: &[FrpPatchOp]) -> Vec<FrpPatchResult>;
}

// ========================================
// E24b: Telemetry Export Hook
// ========================================

/// Telemetry trace event type (E24)
#[derive(Debug, Clone)]
pub enum TelemetryEventKind {
  FrameStart,
  FrameEnd,
  SignalCompute { name: String, value: f64 },
  StateMutation { name: String, old: f64, new: f64 },
  CacheHit { key: String },
  CacheMiss { key: String },
  CtVerify { expr: String, ok: bool },
  Custom { category: String, message: String },
}

/// Telemetry event with metadata (E24)
#[derive(Debug, Clone)]
pub struct TelemetryEvent {
  pub timestamp_ms: i64,
  pub tick: u64,
  pub kind: TelemetryEventKind,
}

impl TelemetryEvent {
  pub fn new(timestamp_ms: i64, tick: u64, kind: TelemetryEventKind) -> Self {
    Self {
      timestamp_ms,
      tick,
      kind,
    }
  }

  pub fn frame_start(timestamp_ms: i64, tick: u64) -> Self {
    Self::new(timestamp_ms, tick, TelemetryEventKind::FrameStart)
  }

  pub fn frame_end(timestamp_ms: i64, tick: u64) -> Self {
    Self::new(timestamp_ms, tick, TelemetryEventKind::FrameEnd)
  }
}

/// Telemetry export hook callback type
pub type TelemetryHookFn = Box<dyn Fn(&TelemetryEvent) + Send + Sync>;

/// Telemetry exporter trait for executor integration (E24b)
pub trait TelemetryExporter {
  /// Export a telemetry event
  fn export(&mut self, event: &TelemetryEvent);

  /// Flush any buffered events
  fn flush(&mut self) -> RuntimeResult<()>;

  /// Get export statistics
  fn stats(&self) -> TelemetryExportStats;
}

/// Telemetry export statistics
#[derive(Debug, Clone, Default)]
pub struct TelemetryExportStats {
  pub events_exported: u64,
  pub bytes_written: u64,
  pub errors: u64,
}

pub const RUNTIME_API_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod capability_example;
pub mod plugin;

#[cfg(feature = "debugger")]
pub mod debugger;

// ========================================
// V01: Async Runtime Support (feature-gated)
// ========================================

#[cfg(feature = "async-runtime")]
pub mod async_runtime {
  //! Async runtime traits for non-blocking execution.
  //!
  //! This module provides async versions of the core runtime traits.
  //! Enable with the `async-runtime` feature flag.
  //!
  //! # Status
  //!
  //! **Experimental**: This is a POC implementation. The API may change.
  //!
  //! # Threading
  //!
  //! Legacy runtimes rely on thread-local storage. Using them on a multi-threaded
  //! async executor can drop per-task state when tasks migrate across threads.
  //! Prefer single-threaded executors or sync evaluation until task-local storage
  //! is implemented for these runtimes.
  //!
  //! # Example
  //!
  //! ```ignore
  //! use pnix_runtime_api::async_runtime::AsyncEvalEngine;
  //!
  //! struct MyAsyncEngine;
  //!
  //! impl AsyncEvalEngine for MyAsyncEngine {
  //!     // ...
  //! }
  //! ```

  use super::*;
  use std::future::Future;
  use std::pin::Pin;
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::sync::Arc;

  /// Async version of EvalResult
  pub type AsyncRuntimeResult<T> = Pin<Box<dyn Future<Output = RuntimeResult<T>> + Send>>;

  /// Cancellation token for async runtimes.
  #[derive(Clone, Default)]
  pub struct AsyncCancelToken {
    cancelled: Arc<AtomicBool>,
  }

  impl AsyncCancelToken {
    pub fn new() -> Self {
      Self::default()
    }

    pub fn cancel(&self) {
      self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
      self.cancelled.load(Ordering::SeqCst)
    }

    pub fn check_cancelled(&self) -> RuntimeResult<()> {
      if self.is_cancelled() {
        Err(RuntimeError::cancelled("async operation cancelled"))
      } else {
        Ok(())
      }
    }
  }

  /// Shared context for async requests.
  #[derive(Clone, Default)]
  pub struct AsyncRequestContext {
    pub cancel: AsyncCancelToken,
  }

  impl AsyncRequestContext {
    pub fn new(cancel: AsyncCancelToken) -> Self {
      Self { cancel }
    }
  }

  /// Async evaluation engine trait.
  ///
  /// This is the async counterpart to `EvalEngine`. Implementations should
  /// return a future that resolves to the evaluation result.
  pub trait AsyncEvalEngine {
    type Module: Send + Sync;
    type Value: Send;

    /// Evaluate a module asynchronously.
    fn eval_async(
      &mut self,
      module: &Self::Module,
      config: &EvalConfig,
      ctx: &AsyncRequestContext,
    ) -> AsyncRuntimeResult<EvalResult<Self::Value>>;
  }

  /// Async FRP engine trait.
  ///
  /// This is the async counterpart to `FrpEngine`. Useful for FRP graphs
  /// that interact with async I/O or external services.
  pub trait AsyncFrpEngine {
    type Graph: Send + Sync;
    type Input: Send;
    type Output: Send;

    /// Process a tick asynchronously.
    fn tick_async(
      &mut self,
      graph: &Self::Graph,
      input: Self::Input,
      config: &FrpConfig,
      ctx: &AsyncRequestContext,
    ) -> AsyncRuntimeResult<FrpTickResult<Self::Output>>;
  }

  /// Async CT runtime trait.
  ///
  /// This is the async counterpart to `CtRuntime`. Useful for verification
  /// that may involve external theorem provers or SMT solvers.
  pub trait AsyncCtRuntime {
    type Spec: Send + Sync;

    /// Verify a specification asynchronously.
    fn verify_async(
      &mut self,
      spec: &Self::Spec,
      config: &CtConfig,
      ctx: &AsyncRequestContext,
    ) -> AsyncRuntimeResult<CtCheckResult>;
  }

  /// Marker trait for runtimes that support async operations.
  ///
  /// Runtimes implementing this trait can be used in async contexts.
  pub trait AsyncCapable {
    /// Returns true if the runtime supports async operations.
    fn supports_async(&self) -> bool {
      true
    }
  }

  /// Adapter to convert sync engine to async (blocking wrapper).
  ///
  /// This is a simple adapter that wraps synchronous calls in a future.
  /// Note: This blocks the executor thread and should only be used for
  /// compatibility purposes.
  pub struct SyncToAsyncAdapter<E> {
    inner: E,
  }

  impl<E> SyncToAsyncAdapter<E> {
    pub fn new(engine: E) -> Self {
      Self { inner: engine }
    }

    pub fn inner(&self) -> &E {
      &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut E {
      &mut self.inner
    }

    pub fn into_inner(self) -> E {
      self.inner
    }
  }

  impl<E: EvalEngine> AsyncEvalEngine for SyncToAsyncAdapter<E>
  where
    E::Module: Send + Sync,
    E::Value: Send + 'static,
  {
    type Module = E::Module;
    type Value = E::Value;

    fn eval_async(
      &mut self,
      _module: &Self::Module,
      _config: &EvalConfig,
      ctx: &AsyncRequestContext,
    ) -> AsyncRuntimeResult<EvalResult<Self::Value>> {
      let cancelled = ctx.cancel.is_cancelled();
      // For now, return unimplemented error
      // In a real implementation, this would use spawn_blocking or similar
      Box::pin(async move {
        if cancelled {
          return Err(RuntimeError::cancelled("async evaluation cancelled"));
        }
        Err(RuntimeError::unimplemented(
          "[ASYNC-001] experimental async adapter path is not implemented; \
           use EvalEngine::eval (sync)",
        ))
      })
    }
  }

  #[cfg(test)]
  mod tests {
    use super::*;

    #[test]
    fn test_async_runtime_feature_enabled() {
      // This test only runs when the feature is enabled
      assert!(true, "async-runtime feature is enabled");
    }

    #[test]
    fn test_sync_to_async_adapter_creation() {
      struct DummyEngine;

      impl EvalEngine for DummyEngine {
        type Module = ();
        type Value = i64;

        fn eval(
          &mut self,
          _module: &Self::Module,
          _config: &EvalConfig,
        ) -> RuntimeResult<EvalResult<Self::Value>> {
          Ok(EvalResult { value: 42 })
        }
      }

      let engine = DummyEngine;
      let adapter = SyncToAsyncAdapter::new(engine);

      // Verify we can access the inner engine
      let _inner = adapter.inner();
    }

    #[test]
    fn test_async_cancel_token() {
      let token = AsyncCancelToken::new();
      assert!(!token.is_cancelled());
      token.cancel();
      assert!(token.is_cancelled());
    }
  }
}

// ========================================
// W06: RuntimeCapabilities (Spec-as-Data)
// ========================================

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Runtime capability (지원하는 기능)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeCapability {
  /// 순수 계산 (부작용 없음)
  Pure,
  /// 세계 효과 (IO/시간/상태)
  World,
  /// IO 연산
  Io,
  /// 네트워크 연산
  Network,
  /// 파일 시스템 연산
  FileSystem,
  /// 수학 연산
  Math,
  /// 산술 연산
  Arithmetic,
  /// 삼각 함수
  Trigonometry,
  /// 비교 연산
  Comparison,
  /// 논리 연산
  Logic,
  /// SSA 평가
  SSAEval,
  /// FRP 틱
  FRPTick,
  /// CT 검증
  CTVerify,
  /// LLVM JIT/AOT
  LLVM,
  /// 프로세스 기능(umbrella)
  Process,
  /// 프로세스 실행/스폰
  ProcessSpawn,
  /// 시그널/종료 제어
  ProcessSignal,
  /// 상태/관측
  ProcessObserve,
}

impl std::fmt::Display for RuntimeCapability {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Pure => write!(f, "pure"),
      Self::World => write!(f, "world"),
      Self::Io => write!(f, "io"),
      Self::Network => write!(f, "network"),
      Self::FileSystem => write!(f, "filesystem"),
      Self::Math => write!(f, "math"),
      Self::Arithmetic => write!(f, "arithmetic"),
      Self::Trigonometry => write!(f, "trigonometry"),
      Self::Comparison => write!(f, "comparison"),
      Self::Logic => write!(f, "logic"),
      Self::SSAEval => write!(f, "ssa_eval"),
      Self::FRPTick => write!(f, "frp_tick"),
      Self::CTVerify => write!(f, "ct_verify"),
      Self::LLVM => write!(f, "llvm"),
      Self::Process => write!(f, "process"),
      Self::ProcessSpawn => write!(f, "process_spawn"),
      Self::ProcessSignal => write!(f, "process_signal"),
      Self::ProcessObserve => write!(f, "process_observe"),
    }
  }
}

/// Runtime capabilities (지원하는 기능 집합)
///
/// executor가 dist spec/contract와 대조하여 "지원 불가 기능"을 명시적으로 거부할 수 있습니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
  /// 지원하는 capability 집합
  pub capabilities: BTreeSet<RuntimeCapability>,
  /// 지원하는 엔진 타입 (예: "legacy-eval", "frp", "ct", "llvm")
  pub engines: BTreeSet<String>,
  /// 지원하는 옵션 (예: "deterministic", "hot-swap")
  pub options: BTreeSet<String>,
}

impl RuntimeCapabilities {
  /// 빈 capabilities 생성
  pub fn new() -> Self {
    Self {
      capabilities: BTreeSet::new(),
      engines: BTreeSet::new(),
      options: BTreeSet::new(),
    }
  }

  /// Capability 추가
  pub fn with_capability(mut self, cap: RuntimeCapability) -> Self {
    self.capabilities.insert(cap);
    self
  }

  /// 엔진 추가
  pub fn with_engine(mut self, engine: impl Into<String>) -> Self {
    self.engines.insert(engine.into());
    self
  }

  /// 옵션 추가
  pub fn with_option(mut self, option: impl Into<String>) -> Self {
    self.options.insert(option.into());
    self
  }

  /// Capability 지원 여부 확인
  pub fn supports(&self, cap: &RuntimeCapability) -> bool {
    self.capabilities.contains(cap)
  }

  /// 엔진 지원 여부 확인
  pub fn supports_engine(&self, engine: &str) -> bool {
    self.engines.contains(engine)
  }

  /// 옵션 지원 여부 확인
  pub fn supports_option(&self, option: &str) -> bool {
    self.options.contains(option)
  }

  /// Capability 요구사항 확인 (모두 지원하는지)
  pub fn supports_all(&self, required: &[RuntimeCapability]) -> bool {
    required.iter().all(|cap| self.supports(cap))
  }

  /// Capability 요구사항 확인 (하나라도 지원하는지)
  pub fn supports_any(&self, required: &[RuntimeCapability]) -> bool {
    required.iter().any(|cap| self.supports(cap))
  }
}

impl Default for RuntimeCapabilities {
  fn default() -> Self {
    Self::new()
  }
}

/// Capability 검증 결과
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CapabilityCheckResult {
  /// 모든 요구사항을 지원하는지
  #[serde(rename = "ok", alias = "success")]
  pub success: bool,
  /// 누락된 capabilities
  pub missing_capabilities: Vec<RuntimeCapability>,
  /// 누락된 엔진
  pub missing_engines: Vec<String>,
  /// 누락된 옵션
  pub missing_options: Vec<String>,
  /// 에러 메시지
  pub message: String,
}

impl CapabilityCheckResult {
  /// 성공 결과
  pub fn ok() -> Self {
    Self {
      success: true,
      missing_capabilities: vec![],
      missing_engines: vec![],
      missing_options: vec![],
      message: String::new(),
    }
  }

  /// 실패 결과
  pub fn error(
    missing_capabilities: Vec<RuntimeCapability>,
    missing_engines: Vec<String>,
    missing_options: Vec<String>,
  ) -> Self {
    let mut parts = Vec::new();
    if !missing_capabilities.is_empty() {
      parts.push(format!(
        "missing capabilities: {}",
        missing_capabilities
          .iter()
          .map(|c| c.to_string())
          .collect::<Vec<_>>()
          .join(", ")
      ));
    }
    if !missing_engines.is_empty() {
      parts.push(format!("missing engines: {}", missing_engines.join(", ")));
    }
    if !missing_options.is_empty() {
      parts.push(format!("missing options: {}", missing_options.join(", ")));
    }
    let message = parts.join("; ");

    Self {
      success: false,
      missing_capabilities,
      missing_engines,
      missing_options,
      message,
    }
  }
}

/// Capability 검증 트레잇
///
/// executor가 dist spec/contract와 대조하여 "지원 불가 기능"을 명시적으로 거부할 수 있습니다.
pub trait CapabilityChecker {
  /// Runtime capabilities 조회
  fn capabilities(&self) -> RuntimeCapabilities;

  /// 요구사항 검증
  fn check_capabilities(
    &self,
    required_capabilities: &[RuntimeCapability],
    required_engines: &[String],
    required_options: &[String],
  ) -> CapabilityCheckResult {
    let caps = self.capabilities();
    let mut missing_caps = Vec::new();
    let mut missing_engines = Vec::new();
    let mut missing_options = Vec::new();

    for cap in required_capabilities {
      if !caps.supports(cap) {
        missing_caps.push(*cap);
      }
    }

    for engine in required_engines {
      if !caps.supports_engine(engine) {
        missing_engines.push(engine.clone());
      }
    }

    for option in required_options {
      if !caps.supports_option(option) {
        missing_options.push(option.clone());
      }
    }

    if missing_caps.is_empty() && missing_engines.is_empty() && missing_options.is_empty() {
      CapabilityCheckResult::ok()
    } else {
      CapabilityCheckResult::error(missing_caps, missing_engines, missing_options)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_runtime_error_display() {
    let e1 = RuntimeError::unimplemented("async support");
    assert_eq!(
      e1.to_string(),
      "[E0401] unimplemented: async support\n  at node <unknown>, morphism <runtime> failed because async support"
    );

    let e2 = RuntimeError::message("something went wrong");
    assert_eq!(
      e2.to_string(),
      "[E0402] something went wrong\n  at node <unknown>, morphism <runtime> failed because something went wrong"
    );

    let e2b = RuntimeError::message_with_code(error_codes::RUNTIME_SETO_ABI, "bad abi");
    assert_eq!(
      e2b.to_string(),
      "[E0461] bad abi\n  at node <unknown>, morphism <runtime> failed because bad abi"
    );

    let e3 = RuntimeError::adapter(AdapterErrorKind::Unsupported, "SignalVar not supported");
    assert_eq!(
      e3.to_string(),
      "[E0403] adapter unsupported: SignalVar not supported\n  at node <unknown>, morphism <runtime> failed because SignalVar not supported"
    );

    let e4 = RuntimeError::execution(ExecutionErrorKind::SSA, "division by zero");
    assert_eq!(
      e4.to_string(),
      "[E0410] SSA error: division by zero\n  at node <unknown>, morphism <runtime> failed because division by zero"
    );
  }

  #[test]
  fn test_runtime_error_is_std_error() {
    fn assert_std_error<E: std::error::Error>(_: &E) {}

    let e = RuntimeError::message("test");
    assert_std_error(&e);
  }

  #[test]
  fn test_runtime_error_kind_str() {
    assert_eq!(RuntimeError::unimplemented("x").kind_str(), "unimplemented");
    assert_eq!(RuntimeError::message("x").kind_str(), "error");
    assert_eq!(
      RuntimeError::adapter(AdapterErrorKind::Unsupported, "x").kind_str(),
      "adapter"
    );
    assert_eq!(
      RuntimeError::execution(ExecutionErrorKind::FRP, "x").kind_str(),
      "execution"
    );
  }

  #[test]
  fn test_error_categories() {
    // Adapter error kinds
    assert_eq!(AdapterErrorKind::Unimplemented.to_string(), "unimplemented");
    assert_eq!(AdapterErrorKind::Unsupported.to_string(), "unsupported");
    assert_eq!(
      AdapterErrorKind::ConversionError.to_string(),
      "conversion error"
    );

    // Execution error kinds
    assert_eq!(ExecutionErrorKind::SSA.to_string(), "SSA");
    assert_eq!(ExecutionErrorKind::FRP.to_string(), "FRP");
    assert_eq!(ExecutionErrorKind::CT.to_string(), "CT");
    assert_eq!(ExecutionErrorKind::LLVM.to_string(), "LLVM");
    assert_eq!(ExecutionErrorKind::Symbolic.to_string(), "symbolic");
    assert_eq!(ExecutionErrorKind::IrEval.to_string(), "IR eval");
    assert_eq!(ExecutionErrorKind::Generic.to_string(), "runtime");
  }

  #[test]
  fn test_runtime_capabilities() {
    let caps = RuntimeCapabilities::new()
      .with_capability(RuntimeCapability::Pure)
      .with_capability(RuntimeCapability::Math)
      .with_engine("legacy-eval")
      .with_option("deterministic");

    assert!(caps.supports(&RuntimeCapability::Pure));
    assert!(caps.supports(&RuntimeCapability::Math));
    assert!(!caps.supports(&RuntimeCapability::Network));
    assert!(caps.supports_engine("legacy-eval"));
    assert!(caps.supports_option("deterministic"));
  }

  #[test]
  fn test_capability_check_result() {
    let result = CapabilityCheckResult::ok();
    assert!(result.success);
    assert!(result.missing_capabilities.is_empty());

    let result = CapabilityCheckResult::error(
      vec![RuntimeCapability::Network],
      vec!["llvm".to_string()],
      vec![],
    );
    assert!(!result.success);
    assert_eq!(result.missing_capabilities.len(), 1);
    assert_eq!(result.missing_engines.len(), 1);
    assert!(result.message.contains("missing capabilities"));
    assert!(result.message.contains("missing engines"));
  }

  #[test]
  fn test_engine_traits_are_object_safe() {
    struct DummyEval;
    impl EvalEngine for DummyEval {
      type Module = ();
      type Value = ();

      fn eval(
        &mut self,
        _module: &Self::Module,
        _config: &EvalConfig,
      ) -> RuntimeResult<EvalResult<Self::Value>> {
        Ok(EvalResult { value: () })
      }
    }

    struct DummyFrp;
    impl FrpEngine for DummyFrp {
      type Graph = ();
      type Input = ();
      type Output = ();

      fn tick(
        &mut self,
        _graph: &Self::Graph,
        _input: Self::Input,
        _config: &FrpConfig,
      ) -> RuntimeResult<FrpTickResult<Self::Output>> {
        Ok(FrpTickResult { output: () })
      }
    }

    struct DummyCt;
    impl CtRuntime for DummyCt {
      type Spec = ();

      fn verify(&mut self, _spec: &Self::Spec, _config: &CtConfig) -> RuntimeResult<CtCheckResult> {
        Ok(CtCheckResult {
          success: true,
          notes: Vec::new(),
          diagram: None,
        })
      }
    }

    let _eval_obj: Box<dyn EvalEngine<Module = (), Value = ()>> = Box::new(DummyEval);
    let _frp_obj: Box<dyn FrpEngine<Graph = (), Input = (), Output = ()>> = Box::new(DummyFrp);
    let _ct_obj: Box<dyn CtRuntime<Spec = ()>> = Box::new(DummyCt);
  }
}
