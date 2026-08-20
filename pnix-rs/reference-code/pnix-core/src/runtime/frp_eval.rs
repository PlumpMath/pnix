//! FRP 평가기 구조 정의
//!
//! pnix-old의 meaning_core/src/unified_meaning/frp_eval.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만 포함하며, 실제 `eval_frame` 실행 로직은 런타임 크레이트(예: `pnix-runtime-legacy`)로 이동합니다.
//! executor(`pnix-executor-graph`)는 런타임을 호출(wiring)하고 결정론 config를 주입합니다.
//!
//! ## 설계 원칙
//!
//! - FrpValue: FRP 평가 런타임 값 타입
//! - EvalContext: 변수 바인딩 및 시그널 레지스트리
//! - FrpEvaluator: 캐시 인식 평가기 구조
//! - 실제 평가 실행 로직은 런타임 크레이트에서 구현

use crate::ir::IrExpr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// FRP 평가 런타임 값
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FrpValue {
  /// 실수 값
  Float(f64),
  /// 정수 값
  Int(i64),
  /// 불리언 값
  Bool(bool),
  /// 문자열 값
  String(String),
  /// 리스트 값
  List(Vec<FrpValue>),
  /// 속성 집합 (키-값 쌍)
  AttrSet(Vec<(String, FrpValue)>),
  /// Lambda 클로저 (평가되지 않음, 저장만)
  Closure {
    /// 파라미터 목록
    params: Vec<String>,
    /// 함수 본문
    body: Box<IrExpr>,
  },
  /// 정의되지 않음/에러 값
  Undefined,
}

// 헌법 준수 (P0-1): 값 계산 함수 제거
// as_float(), as_bool() 등의 값 변환 함수는 런타임 크레이트(예: `pnix-runtime-legacy`)에서 구현하세요.

/// 캐시 히트/미스 통계
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CacheStats {
  /// 캐시 히트 수
  pub hits: u64,
  /// 캐시 미스 수
  pub misses: u64,
  /// 캐시 가능한 키 개수
  pub cached_keys_count: usize,
}

// 헌법 준수 (P0-1): 값 계산 및 상태 변경 함수 제거
// hit_rate(), reset() 등의 값 계산/상태 변경 함수는 런타임 크레이트(예: `pnix-runtime-legacy`)에서 구현하세요.

/// 평가 컨텍스트 (변수 바인딩용)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EvalContext {
  /// 변수 바인딩 (런타임 상태)
  pub vars: HashMap<String, FrpValue>,
  /// 시그널 레지스트리 (SignalRef의 usize로 인덱싱)
  pub signals: Vec<FrpValue>,
  /// 현재 시간
  pub time: f64,
  /// 델타 시간
  pub dt: f64,
}

impl EvalContext {
  /// 새 컨텍스트 생성
  pub fn new(time: f64, dt: f64) -> Self {
    Self {
      vars: HashMap::new(),
      signals: Vec::new(),
      time,
      dt,
    }
  }

  /// 사전 할당된 시그널 슬롯으로 컨텍스트 생성
  pub fn with_signals(time: f64, dt: f64, signal_count: usize) -> Self {
    Self {
      vars: HashMap::new(),
      signals: vec![FrpValue::Undefined; signal_count],
      time,
      dt,
    }
  }

  /// 변수 설정 (빌더 패턴)
  pub fn with_var(mut self, name: impl Into<String>, value: FrpValue) -> Self {
    self.vars.insert(name.into(), value);
    self
  }

  // 헌법 준수 (P0-1): 상태 변경 함수 제거
  // set_var()는 런타임 상태 변경이므로 런타임 크레이트에서 구현하세요.

  /// 변수 가져오기
  pub fn get_var(&self, name: &str) -> Option<&FrpValue> {
    self.vars.get(name)
  }

  // 헌법 준수 (P0-1): 상태 변경 함수 제거
  // set_signal()는 런타임 상태 변경이므로 런타임 크레이트에서 구현하세요.

  /// 인덱스로 시그널 값 가져오기
  pub fn get_signal(&self, index: usize) -> Option<&FrpValue> {
    self.signals.get(index)
  }

  // 헌법 준수 (P0-1): 상태 변경 함수 제거
  // with_signal()는 런타임 상태 변경이므로 런타임 크레이트에서 구현하세요.

  /// 현재 시그널 개수 가져오기
  pub fn signal_count(&self) -> usize {
    self.signals.len()
  }
}

/// FRP 캐시 인식 평가기 구조
///
/// 시간 독립적 서브트리를 위한 메모이제이션 캐시를 유지하여
/// 시간만 변경될 때 프레임 간 재계산을 피합니다.
///
/// **주의**: 실제 평가 실행 로직은 런타임 크레이트에서 구현합니다.
pub struct FrpEvaluator {
  /// 메모이제이션 캐시: stable_hash -> 계산된 값 (런타임 상태)
  /// 실제 구현은 런타임 크레이트에서 수행
  _cache_placeholder: (),
  /// 캐시 가능한 키들 (FRP 캐시 플랜에서)
  /// 실제 구현은 런타임 크레이트에서 수행
  _cacheable_keys_placeholder: (),
  /// 마지막 파라미터 해시 (무효화용)
  pub last_params_hash: u64,
  /// 통계
  pub stats: CacheStats,
}

impl FrpEvaluator {
  /// 새 평가기 생성
  pub fn new() -> Self {
    Self {
      _cache_placeholder: (),
      _cacheable_keys_placeholder: (),
      last_params_hash: 0,
      stats: CacheStats::default(),
    }
  }

  /// 통계 가져오기
  pub fn stats(&self) -> CacheStats {
    self.stats.clone()
  }

  // 헌법 준수 (P0-1): 상태 변경 함수 제거
  // invalidate()는 런타임 상태 변경이므로 런타임 크레이트에서 구현하세요.
}

impl Default for FrpEvaluator {
  fn default() -> Self {
    Self::new()
  }
}

// ─────────────────────────────────────────────
// 참고: eval_frame 실행 로직은 런타임 크레이트로 이동
// ─────────────────────────────────────────────
//
// 다음 함수들은 런타임 크레이트에서 구현하세요:
// - eval_frame(expr, plan, params_hash, ctx) -> FrpValue
// - eval_uncached(expr, ctx) -> FrpValue
// - eval_impl(expr, ctx, use_cache) -> FrpValue
// - eval_node(expr, ctx, use_cache) -> FrpValue
//
// 이 함수들은 값 계산 및 런타임 상태(HashMap, RefCell)를 사용하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  // 헌법 준수 (P0-1): 값 계산 테스트 제거
  // as_float(), as_bool() 테스트는 런타임 크레이트에서 수행하세요.

  #[test]
  fn test_eval_context_creation() {
    let ctx = EvalContext::new(100.0, 0.016);
    assert_eq!(ctx.time, 100.0);
    assert_eq!(ctx.dt, 0.016);
  }

  #[test]
  fn test_eval_context_with_var() {
    let ctx = EvalContext::new(0.0, 0.016).with_var("x", FrpValue::Int(42));
    assert_eq!(ctx.get_var("x"), Some(&FrpValue::Int(42)));
  }

  // 헌법 준수 (P0-1): 값 계산 테스트 제거
  // hit_rate() 테스트는 런타임 크레이트에서 수행하세요.
}
