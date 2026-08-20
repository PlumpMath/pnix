//! Rewrite 시스템
//!
//! pnix-old의 symbolic_core/rewrite에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 규칙 정의만, 실행 없음
//!
//! ## 모듈 구성
//!
//! - `egg_rules`: 대수/삼각/지수/미분 규칙 정의
//! - `engine`: 엔진 구조 및 가드 함수

pub mod egg_rules;
pub mod engine;

pub use egg_rules::{
  all_rules, find_rule, RewriteRule, BASIC_RULES, DIFF_RULES, EXP_LOG_RULES, TRIG_RULES,
};
pub use engine::{is_pure_scalar, CtHookResult, CtHooks, ResourceLimitKind, SimplifyStats};
