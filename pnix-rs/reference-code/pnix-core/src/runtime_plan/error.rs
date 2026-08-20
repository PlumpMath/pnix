//! 런타임 계획 에러 타입 정의

use thiserror::Error;

/// 런타임 계획 에러 타입: 런타임 계획 생성 중 발생하는 에러 타입
#[derive(Debug, Error)]
pub enum RuntimePlanError {
  /// 지원하지 않는 노드: UnifiedExpr에서 지원하지 않는 노드 타입
  #[error("unsupported unified expr node: {node}")]
  UnsupportedNode {
    /// 노드 이름 (지원하지 않는 노드 타입 이름)
    node: &'static str,
  },

  /// 지원하지 않는 Derived 연산: Derived 연산에서 지원하지 않는 연산
  #[error("unsupported derived op: {detail}")]
  UnsupportedDerived {
    /// 상세 정보 (에러 상세 메시지)
    detail: String,
  },

  /// 시그널 해결 필요: 런타임 계획 생성 전에 시그널 해결이 필요함
  #[error("signal resolution required before runtime plan: {detail}")]
  SignalResolutionRequired {
    /// 상세 정보 (에러 상세 메시지)
    detail: String,
  },
}

pub type RuntimePlanResult<T> = Result<T, RuntimePlanError>;
