//! SSA 평가 컨텍스트 구조 정의
//!
//! pnix-old의 meaning_core/src/context.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, eval_ssa 실행 로직은 executor로 이동
//!
//! ## 설계 원칙
//!
//! - SSAEvalContext: SSA 평가를 위한 컨텍스트 구조
//! - 실제 평가 실행 로직은 executor에서 구현

use crate::fx::SignalId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SSA 평가 컨텍스트 구조
///
/// SSA 블록 평가를 위한 런타임 상태를 담는 구조입니다.
/// 실제 평가 실행 로직은 executor에서 구현합니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSAEvalContext {
  /// 현재 시스템 시간
  pub system_time: f64,
  /// 델타 시간
  pub delta_time: f64,
  /// Signal 값들 (런타임 상태)
  pub signals: HashMap<SignalId, f64>,
  /// 변수 바인딩 (런타임 상태)
  pub vars: HashMap<String, f64>,
}

impl SSAEvalContext {
  /// 새 컨텍스트 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(system_time: f64) -> Self {
    Self {
      system_time,
      delta_time: 0.016, // 60fps default
      signals: HashMap::new(),
      vars: HashMap::new(),
    }
  }

  /// 델타 시간 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_delta(mut self, dt: f64) -> Self {
    self.delta_time = dt;
    self
  }

  /// Signal 값 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_signal(mut self, id: SignalId, value: f64) -> Self {
    self.signals.insert(id, value);
    self
  }

  /// 변수 바인딩 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_var(mut self, name: impl Into<String>, value: f64) -> Self {
    self.vars.insert(name.into(), value);
    self
  }
}

impl Default for SSAEvalContext {
  fn default() -> Self {
    Self::new(0.0)
  }
}

// ─────────────────────────────────────────────
// 참고: eval_ssa 실행 로직은 executor로 이동
// ─────────────────────────────────────────────
//
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - eval_ssa(block: &SSABlock, ctx: &SSAEvalContext) -> f64
// - eval_op(op: &SSAOp, regs: &[f64], ctx: &SSAEvalContext) -> f64
//
// 이 함수들은 값 계산을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ssa_eval_context_creation() {
    let ctx = SSAEvalContext::new(100.0);
    assert_eq!(ctx.system_time, 100.0);
    assert_eq!(ctx.delta_time, 0.016);
    assert!(ctx.signals.is_empty());
    assert!(ctx.vars.is_empty());
  }

  #[test]
  fn test_ssa_eval_context_with_delta() {
    let ctx = SSAEvalContext::new(0.0).with_delta(0.033);
    assert_eq!(ctx.delta_time, 0.033);
  }

  #[test]
  fn test_ssa_eval_context_with_signal() {
    let signal_id = SignalId(0);
    let ctx = SSAEvalContext::new(0.0).with_signal(signal_id, 42.0);
    assert_eq!(ctx.signals.get(&signal_id), Some(&42.0));
  }

  #[test]
  fn test_ssa_eval_context_with_var() {
    let ctx = SSAEvalContext::new(0.0).with_var("x", 10.0);
    assert_eq!(ctx.vars.get("x"), Some(&10.0));
  }
}
