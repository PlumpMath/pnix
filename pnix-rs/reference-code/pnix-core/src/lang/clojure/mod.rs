//! Clojure Language Frontend - 타입 정의 및 Lowering
//!
//! pnix-old의 lang_clojure에서 마이그레이션.
//!
//! ## 역할 (pnix-core)
//!
//! - AST 타입 정의
//! - UnifiedExpr → FxCoreExpr lowering
//! - 에러 타입 정의
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의 및 파서 규칙만, 실행 로직 제외
//! **스레드/시간 관련 코드는 pnix-runtime-legacy에 위치**
//!
//! ## 스레드 오케스트레이션 (pnix-runtime-legacy에서 구현)
//!
//! pnix는 단일 스레드, 시간 없음 (참조투명, lazy)
//! .clj 파일이 존재하면 Clojure 런타임이 스레드 관리
//! → `pnix-runtime-legacy/src/clojure_thread.rs` 참조
//!
//! ## 비활성화된 기능 (JVM Clojure interop 예정)
//!
//! - JVM Clojure 런타임 연동
//! - ClojureScript 런타임 연동
//! - 매크로 확장
//!
//! 비활성 코드는 향후 JVM/CLJS interop 시 활성화
//!
//! 단, plugin nREPL subset wire/profile 타입은 runtime과 계약을 맞추기 위해
//! 여기서 구조만 노출한다. transport/daemon/실행 로직은 runtime에 위치한다.

pub mod lower;
pub mod parse;
pub mod plugin_nrepl;

// 비활성: JVM Clojure interop (향후 활성화 예정)
// pub mod interop;

pub use lower::lower_clj_to_fx_core;
pub use parse::{parse_clj_expr, parse_clj_forms, CljForm};
pub use plugin_nrepl::{
  plugin_nrepl_reason_codes, plugin_nrepl_supported_ops, plugin_nrepl_terminal_statuses,
  plugin_nrepl_unsupported_ops, PluginNreplOp, PluginNreplProfile, PluginNreplTerminalStatus,
  PLUGIN_NREPL_CLONE_SEMANTICS, PLUGIN_NREPL_PROFILE_ID, PLUGIN_NREPL_REASON_CLONE_BLOCKED,
  PLUGIN_NREPL_REASON_EVAL_FAILED, PLUGIN_NREPL_REASON_INTERRUPT_IDLE,
  PLUGIN_NREPL_REASON_LOAD_FILE_FAILED, PLUGIN_NREPL_REASON_SESSION_CLOSED,
  PLUGIN_NREPL_REASON_SWITCH_NS_FAILED,
};
// UnifiedExpr는 lang_pnix에서 재사용
pub use crate::lang::pnix::UnifiedExpr;
