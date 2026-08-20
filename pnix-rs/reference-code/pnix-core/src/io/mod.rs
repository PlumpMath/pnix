//! IO Monad 구조 정의
//!
//! pnix-old의 pnix_io_runtime에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! IO Monad의 run() 메서드 및 실제 I/O 실행은 executor에서 수행

pub mod cancellation;
pub mod effect;
pub mod os_error;
pub mod parametric;
pub mod task;
pub mod timeout;

pub use cancellation::{CancellableIO, CancellationToken};
pub use effect::{Effect, EffectType, EffectValidator};
pub use os_error::OsError;
pub use parametric::{AttrPath, EnvBinding, ExprCell, ExprValue};
pub use task::{TaskCancelHandle, TaskGroup};
pub use timeout::{TimeoutError, TimeoutIO};

/// Monad 트레이트
///
/// **주의**: bind, return_m 구현은 executor에서 수행합니다.
pub trait Monad<T> {
  type M<U>;

  /// bind 연산 (executor에서 구현)
  fn bind<U, F>(self, f: F) -> Self::M<U>
  where
    F: FnOnce(T) -> Self::M<U>;

  /// return 연산 (executor에서 구현)
  fn return_m(value: T) -> Self::M<T>;
}

/// Functor 트레이트
///
/// **주의**: fmap 구현은 executor에서 수행합니다.
pub trait Functor<T> {
  type F<U>;

  /// fmap 연산 (executor에서 구현)
  fn fmap<U, F>(self, f: F) -> Self::F<U>
  where
    F: FnOnce(T) -> U;
}

/// IO Monad 타입 (구조 정의만)
///
/// **주의**: 실제 `IO<T>` 구현 및 run() 메서드는 executor에서 수행합니다.
/// pnix-core에는 타입 정의만 포함합니다.
pub struct IO<T> {
  _phantom: std::marker::PhantomData<T>,
}
