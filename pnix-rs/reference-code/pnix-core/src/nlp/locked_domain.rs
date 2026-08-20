//! LockedDomain 정의 (잠금 영역)
//!
//! pnix-old의 pnix_llm/src/locked_domain.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! enum 정의 및 키워드 분류만, 실행 없음

/// 잠금 영역: 민감한 주제로 분류되는 도메인
///
/// 이러한 도메인은 CT로 형식화하기 어렵거나 주관적 판단이 필요한 영역입니다.
/// 헌법 준수: 키워드 분류만, 실행 없음
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockedDomain {
  /// 정치
  Politics,
  /// 종교
  Religion,
  /// 이념
  Ideology,
  /// 도덕적 상대주의
  MoralRelativism,
  /// 문화적 가치
  CulturalValues,
  /// 역사 해석
  HistoricalInterpretation,
  /// 미적 판단
  AestheticJudgment,
  /// 개인적 선호
  PersonalPreference,
}

impl LockedDomain {
  /// 토픽을 잠금 영역으로 분류 (단순 키워드 매칭)
  ///
  /// 헌법 준수: 키워드 분류만, 실행 없음
  pub fn classify(topic: &str) -> Option<Self> {
    let t = topic.to_lowercase();
    let contains = |kw: &str| t.contains(kw);

    Some(if contains("정치") || contains("politic") {
      Self::Politics
    } else if contains("종교") || contains("relig") {
      Self::Religion
    } else if contains("이념") || contains("ideolog") {
      Self::Ideology
    } else if contains("상대주의") || contains("relativ") {
      Self::MoralRelativism
    } else if contains("문화") || contains("가치") {
      Self::CulturalValues
    } else if contains("역사 해석") || contains("histor") {
      Self::HistoricalInterpretation
    } else if contains("미적") || contains("aesthetic") || contains("미학") {
      Self::AestheticJudgment
    } else if contains("취향") || contains("선호") || contains("preference") {
      Self::PersonalPreference
    } else {
      return None;
    })
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::LockedDomain as LD;

  #[test]
  fn test_classify_locked() {
    assert_eq!(LD::classify("정치 토론"), Some(LD::Politics));
    assert_eq!(LD::classify("religion debate"), Some(LD::Religion));
    assert_eq!(LD::classify("미적 판단"), Some(LD::AestheticJudgment));
    assert_eq!(LD::classify("개인적 취향"), Some(LD::PersonalPreference));
    assert_eq!(LD::classify("일반 대화"), None);
  }
}
