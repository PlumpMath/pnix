//! Runtime 구조 정의
//!
//! pnix-old의 pnix_runtime에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 타입/트레잇 정의만, 실행 로직 제외
//! 런타임 상태(values HashMap 등)는 executor로 이관

pub mod builtin;
pub mod clojure_runtime;
pub mod config;
pub mod context;
pub mod error;
pub mod event_loop;
pub mod frp_eval;
pub mod interp;
pub mod machines_heart;
pub mod pnix_runtime;
pub mod process;
pub mod repl;
pub mod sym_block;
pub mod symbolic_bridge;
pub mod task_loop;
pub mod value;

pub use builtin::{BuiltinCategory, BuiltinFunction, BuiltinRegistry};
pub use clojure_runtime::{ClojureRuntime, ClojureRuntimeError};
pub use config::{InitMode, RuntimeConfig, SymbolicConfig, ValidationLevel};
pub use context::RuntimeContext;
pub use error::RuntimeError;
pub use event_loop::{
  Event, EventLoopConfig, HandlerId, HandlerInfo, HandlerPriority, InputState, MouseButton,
};
pub use frp_eval::{CacheStats, EvalContext, FrpEvaluator, FrpValue};
pub use interp::{Backend, EffectDescriptor, InterpError, InterpValue};
pub use machines_heart::{
  CTLaw, CTVerificationError, CTVerificationState, ConscienceInterface, EthicalRule,
  EvolutionAction, EvolutionEvidence, EvolutionLoopState, EvolutionResult, EvolutionStep,
  FilterAction, IntentionVector, MemoryItem, MemoryState, ModificationType, PrefixFilter,
  RuleChange, RuleEvolutionEngineState, RuleEvolutionSuggestion, RuleModification, RuleSet,
  RuleSnapshot, SafetyRule, SuggestionType,
};
pub use pnix_runtime::{PnixRuntime, PnixRuntimeError};
pub use process::{Process, ProcessError, ProcessId, ProcessManager, ProcessMessage, ProcessState};
pub use repl::{ReplCommand, ReplState, SymbolicResult};
pub use sym_block::{OutputMode, SymBlock, SymBlockKind};
pub use symbolic_bridge::{ResourceLimits, SymbolicBridge};
pub use task_loop::{BlockReason, Task, TaskId, TaskLoopState};
pub use value::{LambdaClosure, RuntimeValue};
