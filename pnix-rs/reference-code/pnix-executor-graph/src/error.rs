//! 실행 에러 타입 정의

use anyhow::Error;
use std::{error::Error as StdError, fmt};

use pnix_core::diagnostics::CompileError;
use pnix_runtime_api::RuntimeError;

/// 실행 에러: 실행 중 발생한 에러
#[derive(Debug)]
pub struct ExecutionError {
  /// 원본 에러
  source: Error,
}

impl ExecutionError {
  pub fn new(source: Error) -> Self {
    Self { source }
  }
}

impl fmt::Display for ExecutionError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "execution error: {}", self.source)
  }
}

impl StdError for ExecutionError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    Some(self.source.as_ref())
  }
}

impl From<Error> for ExecutionError {
  fn from(err: Error) -> Self {
    Self::new(err)
  }
}

impl From<RuntimeError> for ExecutionError {
  fn from(err: RuntimeError) -> Self {
    Self::new(Error::new(err))
  }
}

impl From<CompileError> for ExecutionError {
  fn from(err: CompileError) -> Self {
    Self::new(Error::new(err))
  }
}

pub type ExecutionResult<T> = Result<T, ExecutionError>;
