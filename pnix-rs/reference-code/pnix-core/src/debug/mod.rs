//! Debug 모듈
//!
//! 디버깅 관련 구조 정의
//!
//! pnix-old의 pnix_debug_console에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외

pub mod breakpoint;
pub mod debugger;
pub mod log;
pub mod watch;

pub use breakpoint::{
  Breakpoint, BreakpointCondition, BreakpointId, BreakpointLocation, HitCountOperator,
};
pub use debugger::{CallFrame, DebuggerState, StepType, VariableInfo};
pub use log::{ConsoleFilter, ConsoleStats, LogEntry};
pub use watch::{WatchExpression, WatchId};
