//! Runtime shared traits
//!
//! 공통 런타임 인터페이스 정의만 포함 (헌법 P0-1)

use crate::runtime::context::RuntimeContext;

/// 런타임 컨텍스트 접근 공통 인터페이스
pub trait RuntimeContextAccess {
  fn context(&self) -> &RuntimeContext;
}
