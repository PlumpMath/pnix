//! CT 검증 요청/응답 구조
//!
//! pnix-old의 pnix_llm/src/verification.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 데이터 구조만, 실행 없음

use super::truth_domain::TruthDomain;

/// Seto 노드 ID 타입: SETO 지식 그래프의 노드 식별자
pub type SetoNodeId = String;

/// 진리 검증 결과: 진술의 진리 검증 결과 타입
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TruthVerification {
  /// 검증됨
  Verified {
    /// 도메인
    domain: TruthDomain,
    /// 증명
    proof: String,
  },
  /// 반증됨
  Refuted {
    /// 도메인
    domain: TruthDomain,
    /// 반례
    counterexample: String,
  },
  /// 범위 밖
  OutOfScope {
    /// 이유
    reason: String,
  },
  /// 잠금됨
  Locked,
  /// 대기 중
  Pending {
    /// 예상 단계 수
    estimated_steps: usize,
  },
}

/// 증명 단계: 증명 과정의 단일 단계
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofStep {
  /// 단계 설명
  pub description: String,
}

/// 검증 요청: 진리 검증을 위한 요청 구조
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRequest {
  /// 검증할 진술
  pub statement: String,
  /// 도메인
  pub domain: TruthDomain,
  /// 컨텍스트 (노드 ID 목록)
  pub context: Vec<SetoNodeId>,
  /// 타임아웃 (밀리초)
  pub timeout_ms: u64,
}

/// 검증 응답: 진리 검증의 응답 구조
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationResponse {
  /// 검증 결과
  pub result: TruthVerification,
  /// 증명 단계 목록
  pub proof_steps: Vec<ProofStep>,
  /// 신뢰도 (0.0 ~ 1.0)
  pub confidence: f32,
  /// 소요 시간 (밀리초)
  pub duration_ms: u64,
}

impl VerificationResponse {
  /// 검증된 응답 생성 헬퍼
  pub fn verified(domain: TruthDomain, proof_steps: Vec<ProofStep>, duration_ms: u64) -> Self {
    Self {
      result: TruthVerification::Verified {
        domain,
        proof: "ok".into(),
      },
      proof_steps,
      confidence: 1.0,
      duration_ms,
    }
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_verified_helper() {
    let res = VerificationResponse::verified(
      TruthDomain::Mathematics,
      vec![ProofStep {
        description: "axiom".into(),
      }],
      10,
    );
    assert!(matches!(res.result, TruthVerification::Verified { .. }));
    assert_eq!(res.proof_steps.len(), 1);
    assert_eq!(res.confidence, 1.0);
  }
}
