//! 로그 매크로: pnix-core용 최소 no-op 트레이싱 매크로
//!
//! 런타임 로깅 의존성 없이 사용 가능한 최소한의 트레이싱 매크로

#[allow(unused_macros)]
macro_rules! debug {
  ($($tt:tt)*) => {};
}

#[allow(unused_macros)]
macro_rules! trace {
  ($($tt:tt)*) => {};
}
