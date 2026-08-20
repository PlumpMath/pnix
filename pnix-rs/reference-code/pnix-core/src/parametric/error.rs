//! Parametric 에러 타입 정의

use thiserror::Error;

/// Parametric 에러 타입
#[derive(Debug, Error)]
pub enum ParametricError {
  #[error("duplicate parameter name: {name}")]
  DuplicateParam {
    /// 파라미터 이름
    name: String,
  },

  #[error("duplicate signal name: {name}")]
  DuplicateSignal {
    /// 시그널 이름
    name: String,
  },

  #[error("reserved parameter name: {name}")]
  ReservedParamName {
    /// 파라미터 이름
    name: String,
  },

  #[error("reserved signal name: {name}")]
  ReservedSignalName {
    /// 시그널 이름
    name: String,
  },

  #[error("duplicate constraint id: {id}")]
  DuplicateConstraint {
    /// 제약 ID
    id: String,
  },

  #[error("target variable not found in params: {name}")]
  TargetNotFound {
    /// 변수 이름
    name: String,
  },

  #[error("signal '{name}' is not allowed in pure context")]
  SignalInPureContext {
    /// 시그널 이름
    name: String,
  },

  #[error("unknown signal name: {name}")]
  UnknownSignal {
    /// 시그널 이름
    name: String,
  },

  #[error("unknown parameter name: {name}")]
  UnknownParam {
    /// 파라미터 이름
    name: String,
  },

  #[error("unsupported constraint kind: {kind}")]
  UnsupportedConstraint {
    /// 제약 종류
    kind: &'static str,
  },

  #[error("no constraint contains target: {name}")]
  ConstraintMissingTarget {
    /// 타겟 이름
    name: String,
  },

  #[error("multiple constraints contain target: {name} (count={count})")]
  MultipleConstraintsForTarget {
    /// 타겟 이름
    name: String,
    /// 제약 개수
    count: usize,
  },

  #[error("non-linear or ambiguous target occurrence: {name}")]
  NonLinearTarget {
    /// 타겟 이름
    name: String,
  },

  #[error("constraint inconsistency: {detail}")]
  ConstraintInconsistent {
    /// 상세 정보
    detail: String,
  },

  #[error("unit mismatch: {left} vs {right} in {op}")]
  UnitMismatch {
    /// 왼쪽 단위
    left: String,
    /// 오른쪽 단위
    right: String,
    /// 연산자
    op: &'static str,
  },

  #[error("unsupported unit operation: {op}")]
  UnitUnsupportedOp {
    /// 연산자
    op: &'static str,
  },

  #[error("unit conversion factor must be finite and > 0: {factor}")]
  UnitConversionInvalidFactor {
    /// 변환 계수
    factor: f64,
  },

  #[error("unit conversion missing: {from} -> {to}")]
  UnitConversionMissing {
    /// 출발 단위
    from: String,
    /// 도착 단위
    to: String,
  },

  #[error("unit conversion factor mismatch: {from} -> {to} expected={expected} found={found}")]
  UnitConversionFactorMismatch {
    /// 출발 단위
    from: String,
    /// 도착 단위
    to: String,
    /// 예상 계수
    expected: f64,
    /// 실제 계수
    found: f64,
  },

  #[error("duplicate unit conversion: {from} -> {to}")]
  DuplicateUnitConversion {
    /// 출발 단위
    from: String,
    /// 도착 단위
    to: String,
  },

  #[error("unit conversion arg unit mismatch: expected {expected}, found {found}")]
  UnitConversionArgUnit {
    /// 예상 단위
    expected: String,
    /// 실제 단위
    found: String,
  },

  #[error("unsupported call function: {name}")]
  UnsupportedCall {
    /// 함수 이름
    name: String,
  },

  #[error("unsupported call arity: {name} expects {expected}, found {found}")]
  UnsupportedCallArity {
    /// 함수 이름
    name: String,
    /// 예상 인자 개수
    expected: usize,
    /// 실제 인자 개수
    found: usize,
  },

  #[error("duplicate call policy entry: {name}")]
  DuplicateCallPolicy {
    /// 함수 이름
    name: String,
  },

  #[error("invalid call arity in policy: {name} arity={arity}")]
  InvalidCallArity {
    /// 함수 이름
    name: String,
    /// 인자 개수
    arity: usize,
  },

  #[error("invalid constant expression: {detail}")]
  InvalidConstantExpr {
    /// 상세 정보
    detail: String,
  },

  #[error("fixture '{fixture}' has unknown {kind} binding: {name}")]
  FixtureUnknownBinding {
    /// Fixture 이름
    fixture: String,
    /// 바인딩 종류
    kind: &'static str,
    /// 바인딩 이름
    name: String,
  },

  #[error("fixture '{fixture}' missing {kind} binding: {name}")]
  FixtureMissingBinding {
    /// Fixture 이름
    fixture: String,
    /// 바인딩 종류
    kind: &'static str,
    /// 바인딩 이름
    name: String,
  },

  #[error("fixture '{fixture}' invalid {kind} value for {name}: {detail}")]
  FixtureInvalidValue {
    /// Fixture 이름
    fixture: String,
    /// 값 종류
    kind: &'static str,
    /// 이름
    name: String,
    /// 상세 정보
    detail: String,
  },

  #[error("fixture '{fixture}' constraint '{constraint}' failed: {detail}")]
  FixtureConstraintFailed {
    /// Fixture 이름
    fixture: String,
    /// 제약 이름
    constraint: String,
    /// 상세 정보
    detail: String,
  },

  #[error("json error: {detail}")]
  JsonError {
    /// 상세 정보
    detail: String,
  },

  #[error("unsupported solve pattern: {detail}")]
  UnsupportedSolve {
    /// 상세 정보
    detail: String,
  },

  #[error("emit unsupported: {detail}")]
  EmitUnsupported {
    /// 상세 정보
    detail: String,
  },

  #[error("symbolic bridge unsupported: {detail}")]
  SymbolicBridgeUnsupported {
    /// 상세 정보
    detail: String,
  },
}

pub type ParametricResult<T> = Result<T, ParametricError>;
