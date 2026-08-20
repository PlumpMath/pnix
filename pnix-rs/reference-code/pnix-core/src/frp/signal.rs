//! FRP Signal Types - Structure Definitions Only
//!
//! pnix-old의 meaning_core/src/frp_runtime.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! - 타입 정의만, 값 계산/상태 변경 없음
//! - SignalKind::Scan의 accumulated 필드는 **제외** (런타임 상태)

use serde::{Deserialize, Serialize};

use crate::fx::SignalId;
use crate::ssa::SsaBlock;

// ============================================================
// State ID (STM Atom Handle)
// ============================================================

/// STM State ID (Atom 핸들)
///
/// Software Transactional Memory에서 외부 변경 가능한 상태를 참조
/// State ID (STM State signal identifier)
///
/// NOTE: FrpGraph는 SignalId/StateId를 단일 카운터에서 할당해
/// 직렬화 시 충돌을 방지한다. (수동 생성은 별도 주의)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateId(pub usize);

impl StateId {
  /// 새 StateId 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(id: usize) -> Self {
    Self(id)
  }

  /// 내부 값 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn value(&self) -> usize {
    self.0
  }
}

// ============================================================
// Signal Kind
// ============================================================

/// Signal 종류 (FRP 노드 타입)
///
/// ## 헌법 준수
///
/// - Constant(f64): AST 리터럴, 실행 아님 ✅
/// - Derived/Combine2: deps와 ssa 구조만, 값 계산 없음 ✅
/// - Scan: source와 ssa만, accumulated 제외 (런타임 상태)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalKind {
  /// 외부 입력 (마우스, 키보드 등)
  Input,

  /// 상수 값 (AST 리터럴)
  Constant(f64),

  /// 시스템 시간 참조 (심볼릭, 실제 값 아님)
  Time,

  /// 델타 시간 (이전 tick과의 시간 차이, 심볼릭)
  DeltaTime,

  /// 파생 신호 (다른 신호들로부터 계산)
  ///
  /// executor에서 deps를 평가하고 ssa를 실행
  Derived {
    /// 의존하는 신호들
    deps: Vec<SignalId>,
    /// 계산 로직 (SSA로 표현)
    ssa: SsaBlock,
  },

  /// STM State signal (Atom-like, 외부에서 swap 가능)
  State {
    /// 연결된 State ID
    state_id: StateId,
  },

  /// Scan signal - FRP scan/fold (누적 값)
  ///
  /// ## 헌법 준수
  ///
  /// - source, ssa: 구조 정의 ✅
  /// - accumulated: 런타임 상태 → executor ❌ (제외됨)
  Scan {
    /// 소스 signal (스캔할 값)
    source: SignalId,
    /// 누적 함수 (SSA로 표현)
    ssa: SsaBlock,
    /// 초기값 (AST 상수)
    initial: f64,
  },

  /// Combine2 signal - 두 signal 조합
  Combine2 {
    /// 첫 번째 signal
    source1: SignalId,
    /// 두 번째 signal
    source2: SignalId,
    /// 조합 함수 (SSA로 표현)
    ssa: SsaBlock,
  },
}

// ============================================================
// Cached Signal Info
// ============================================================

/// Extended signal info for caching
///
/// pnix-old의 meaning_core/src/cached_frp_runtime.rs에서 마이그레이션.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 캐시 실행 로직은 executor로 이동
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSignalInfo {
  /// Original IR expression (for cache planning)
  pub ir_expr: crate::ir::IrExpr,
  /// Effect zone (determines caching strategy)
  pub zone: crate::effects::EffectZone,
  /// Last computed hash for params (구조적 메타데이터)
  pub last_params_hash: u64,
}

impl SignalKind {
  /// 이 signal이 다른 signal에 의존하는지
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn has_dependencies(&self) -> bool {
    matches!(
      self,
      SignalKind::Derived { .. } | SignalKind::Scan { .. } | SignalKind::Combine2 { .. }
    )
  }

  /// 의존하는 signal ID들 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn dependencies(&self) -> Vec<SignalId> {
    match self {
      SignalKind::Derived { deps, .. } => deps.clone(),
      SignalKind::Scan { source, .. } => vec![*source],
      SignalKind::Combine2 {
        source1, source2, ..
      } => vec![*source1, *source2],
      _ => vec![],
    }
  }

  /// 이 signal이 상수인지
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_constant(&self) -> bool {
    matches!(self, SignalKind::Constant(_))
  }

  /// 이 signal이 입력인지
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_input(&self) -> bool {
    matches!(self, SignalKind::Input)
  }

  /// 이 signal이 시간 관련인지
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_time_related(&self) -> bool {
    matches!(self, SignalKind::Time | SignalKind::DeltaTime)
  }
}

// ============================================================
// Signal Node
// ============================================================

/// Signal 노드 (FRP 그래프의 정점)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalNode {
  /// Signal ID
  pub id: SignalId,
  /// Signal 이름 (디버깅/시각화용)
  pub name: String,
  /// Signal 종류
  pub kind: SignalKind,
}

impl SignalNode {
  /// 새 SignalNode 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(id: SignalId, name: impl Into<String>, kind: SignalKind) -> Self {
    Self {
      id,
      name: name.into(),
      kind,
    }
  }

  /// 입력 signal 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn input(id: SignalId, name: impl Into<String>) -> Self {
    Self::new(id, name, SignalKind::Input)
  }

  /// 상수 signal 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn constant(id: SignalId, name: impl Into<String>, value: f64) -> Self {
    Self::new(id, name, SignalKind::Constant(value))
  }

  /// 시간 signal 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn time(id: SignalId) -> Self {
    Self::new(id, "time", SignalKind::Time)
  }

  /// 델타 시간 signal 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn delta_time(id: SignalId) -> Self {
    Self::new(id, "dt", SignalKind::DeltaTime)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_signal_kind_dependencies() {
    let input = SignalKind::Input;
    assert!(!input.has_dependencies());
    assert!(input.dependencies().is_empty());

    let constant = SignalKind::Constant(42.0);
    assert!(constant.is_constant());
    assert!(!constant.has_dependencies());
  }

  #[test]
  fn test_signal_node_creation() {
    let node = SignalNode::input(SignalId(0), "mouse_x");
    assert_eq!(node.id, SignalId(0));
    assert_eq!(node.name, "mouse_x");
    assert!(node.kind.is_input());
  }

  #[test]
  fn test_state_id() {
    let state = StateId::new(5);
    assert_eq!(state.value(), 5);
  }
}
