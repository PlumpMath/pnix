//! 공통 런타임 커널 (기본적으로 결정론적)
//!
//! 이 크레이트는 시간, 태스크 스케줄링, 이펙트 수집을 담당합니다.
//! 언어 프론트엔드는 FxCore로 lowering하여 커널을 통해 실행해야 합니다.

mod clock;
mod effects;
mod kernel;
mod scheduler;

pub use clock::{ClockConfig, ClockMode, ClockTick, KernelClock};
pub use effects::{EffectEvent, EffectHost, EffectZone};
pub use kernel::{Kernel, KernelConfig, KernelStats};
pub use scheduler::{KernelTask, TaskId, TaskScheduler};

pub type KernelResult<T> = Result<T, pnix_runtime_api::RuntimeError>;
