//! TruthDomain 정의 (CT 규칙화 가능한 24개 분야)
//!
//! pnix-old의 pnix_llm/src/truth_domain.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! enum 정의 및 키워드 분류만, 런타임 실행 로직 제외

/// 진리 도메인: CT로 형식화 가능한 24개 분야
///
/// 모든 TruthDomain은 Category Theory로 형식화 가능하다는 전제 하에 정의됩니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TruthDomain {
  // 형식과학
  /// 수학
  Mathematics,
  /// 논리학
  Logic,
  /// 계산 이론
  Computation,
  /// 통계학
  Statistics,
  /// 정보 이론
  InformationTheory,
  /// 게임 이론
  GameTheory,

  // 자연과학
  /// 물리학
  Physics,
  /// 화학
  Chemistry,
  /// 생물학
  Biology,
  /// 천문학
  Astronomy,
  /// 지구과학
  EarthScience,
  /// 유전학
  Genetics,

  // 생명/의료과학
  /// 의학
  Medicine,
  /// 약리학
  Pharmacology,
  /// 신경과학
  Neuroscience,

  // 인지/행동과학
  /// 정신분석학
  Psychoanalysis,
  /// 언어학
  Linguistics,

  // 공학/응용과학
  /// 컴퓨터 과학
  ComputerScience,
  /// 공학
  Engineering,
  /// 암호학
  Cryptography,
  /// 네트워크 이론
  NetworkTheory,
  /// 시스템 이론
  SystemsTheory,

  // 사회/인문과학
  /// 경제학
  Economics,
  /// 음악 이론
  MusicTheory,
}

impl TruthDomain {
  /// 모든 TruthDomain은 CT로 형식화 가능하다는 전제
  pub const fn is_ct_formalizable(&self) -> bool {
    true
  }

  /// 토픽 문자열을 단순 키워드 매칭으로 분류 (프로토타입)
  pub fn classify(topic: &str) -> Option<Self> {
    let t = topic.to_lowercase();
    let contains = |kw: &str| t.contains(kw);
    let contains_token = |kw: &str| {
      t.split(|c: char| !c.is_alphanumeric())
        .any(|token| token == kw)
    };

    Some(if contains("math") || contains("수학") {
      Self::Mathematics
    } else if contains("logic") || contains("논리") {
      Self::Logic
    } else if contains("computer science") || contains("컴퓨터과학") {
      Self::ComputerScience
    } else if contains("compu") || contains("계산") || contains_token("cs") {
      Self::Computation
    } else if contains("stat") || contains("통계") {
      Self::Statistics
    } else if contains("information") || contains("정보이론") {
      Self::InformationTheory
    } else if contains("game theory") || contains("게임이론") {
      Self::GameTheory
    } else if contains("physics") || contains("물리") {
      Self::Physics
    } else if contains("chem") || contains("화학") {
      Self::Chemistry
    } else if contains("bio") || contains("생물") {
      Self::Biology
    } else if contains("astron") || contains("우주") || contains("천문") {
      Self::Astronomy
    } else if contains("earth") || contains("지구과학") {
      Self::EarthScience
    } else if contains("genetic") || contains("유전") {
      Self::Genetics
    } else if contains("medicine") || contains("의학") {
      Self::Medicine
    } else if contains("pharmac") || contains("약리") {
      Self::Pharmacology
    } else if contains("neuro") || contains("신경") {
      Self::Neuroscience
    } else if contains("psycho") || contains("정신분석") {
      Self::Psychoanalysis
    } else if contains("lingu") || contains("언어학") {
      Self::Linguistics
    } else if contains("crypt") || contains("암호") {
      Self::Cryptography
    } else if contains("network") || contains("네트워크") {
      Self::NetworkTheory
    } else if contains("systems") || contains("시스템") {
      Self::SystemsTheory
    } else if contains("engineer") || contains("공학") {
      Self::Engineering
    } else if contains("computer") {
      Self::ComputerScience
    } else if contains("econom") || contains("경제") {
      Self::Economics
    } else if contains("music") || contains("음악이론") {
      Self::MusicTheory
    } else {
      return None;
    })
  }
}

#[cfg(test)]
mod tests {
  use super::TruthDomain as TD;

  #[test]
  fn classify_basic() {
    assert_eq!(TD::classify("수학"), Some(TD::Mathematics));
    assert_eq!(TD::classify("logic programming"), Some(TD::Logic));
    assert_eq!(TD::classify("암호학"), Some(TD::Cryptography));
    assert_eq!(TD::classify("경제 분석"), Some(TD::Economics));
    assert_eq!(TD::classify("음악이론"), Some(TD::MusicTheory));
    assert_eq!(TD::classify("unknown field"), None);
  }

  #[test]
  fn classify_cs_token() {
    assert_eq!(TD::classify("CS"), Some(TD::Computation));
    assert_eq!(TD::classify("cs"), Some(TD::Computation));
    assert_eq!(TD::classify("computer science"), Some(TD::ComputerScience));
    // "physics" should still match Physics despite containing "cs" as a substring.
    assert_eq!(TD::classify("physics"), Some(TD::Physics));
  }

  #[test]
  fn all_formalizable() {
    for d in [
      TD::Mathematics,
      TD::Logic,
      TD::Computation,
      TD::Statistics,
      TD::InformationTheory,
      TD::GameTheory,
      TD::Physics,
      TD::Chemistry,
      TD::Biology,
      TD::Astronomy,
      TD::EarthScience,
      TD::Genetics,
      TD::Medicine,
      TD::Pharmacology,
      TD::Neuroscience,
      TD::Psychoanalysis,
      TD::Linguistics,
      TD::ComputerScience,
      TD::Engineering,
      TD::Cryptography,
      TD::NetworkTheory,
      TD::SystemsTheory,
      TD::Economics,
      TD::MusicTheory,
    ] {
      assert!(d.is_ct_formalizable());
    }
  }
}
