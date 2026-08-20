//! CT 검증 요청/응답 (순수 타입)
//!
//! pnix-old의 pnix_llm/src/verification.rs에서 마이그레이션.
//! duration 타입 → u64 (밀리초)로 대체.

use crate::llm::truth_domain::TruthDomain;
use serde::{Deserialize, Serialize};

/// SETO 노드 ID
pub type SetoNodeId = String;

/// 진리 검증 결과: SETO에서 문장의 진리성을 검증한 결과
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TruthVerification {
  /// 검증됨 (문장이 참임이 증명됨)
  Verified {
    /// 검증 도메인
    domain: TruthDomain,
    /// 증명 문자열
    proof: String,
  },
  /// 반증됨 (문장이 거짓임이 증명됨)
  Refuted {
    /// 검증 도메인
    domain: TruthDomain,
    /// 반례 문자열
    counterexample: String,
  },
  /// 범위 외 (검증할 수 없는 문장)
  OutOfScope {
    /// 이유
    reason: String,
  },
  /// 잠김 (LockedDomain으로 인해 검증 불가)
  Locked,
  /// 대기 중 (검증이 진행 중)
  Pending {
    /// 예상 단계 수
    estimated_steps: usize,
  },
}

/// 증명 단계: 증명 과정의 한 단계
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofStep {
  /// 단계 설명 (이 단계에서 수행한 작업 설명)
  pub description: String,
}

/// 검증 요청: SETO에 문장의 진리성을 검증하도록 요청
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationRequest {
  /// 검증할 문장 (검증 대상 문장)
  pub statement: String,
  /// 대상 도메인 (검증할 도메인)
  pub domain: TruthDomain,
  /// 컨텍스트 노드 ID (검증에 사용할 컨텍스트 노드 목록)
  pub context: Vec<SetoNodeId>,
  /// 타임아웃 (밀리초, 검증 최대 소요 시간)
  pub timeout_ms: u64,
}

/// 검증 응답: SETO의 검증 요청에 대한 응답
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationResponse {
  /// 검증 결과 (검증 결과)
  pub result: TruthVerification,
  /// 증명 단계 (증명 과정의 단계 목록)
  pub proof_steps: Vec<ProofStep>,
  /// 신뢰도 (0.0 ~ 1.0, 검증 결과의 신뢰도)
  pub confidence: f32,
  /// 소요 시간 (밀리초, 검증에 걸린 시간)
  pub duration_ms: u64,
}

impl VerificationResponse {
  /// 검증됨 응답 생성
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn verified_helper() {
    let res = VerificationResponse::verified(
      TruthDomain::Mathematics,
      vec![ProofStep {
        description: "axiom".into(),
      }],
      10,
    );
    assert!(matches!(res.result, TruthVerification::Verified { .. }));
    assert_eq!(res.proof_steps.len(), 1);
  }
}
