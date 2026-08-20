//! TruthDomain 정의 (CT 규칙화 가능한 24개 분야)
//!
//! pnix-old의 pnix_llm/src/truth_domain.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 분류 구조 정의, 값 계산 없음

use serde::{Deserialize, Serialize};

/// CT(Category Theory)로 형식화 가능한 학문 분야
///
/// SETO가 객관적 진리 판단을 내릴 수 있는 24개 분야.
/// 각 분야는 공리계, 추론 규칙, 검증 가능한 명제를 갖춘다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TruthDomain {
  // 형식과학 (Formal Sciences)
  /// 수학: 집합론, 대수학, 해석학, 기하학 등
  Mathematics,
  /// 논리학: 명제논리, 술어논리, 양상논리 등
  Logic,
  /// 계산이론: 튜링기계, 람다계산, 복잡도이론 등
  Computation,
  /// 통계학: 확률론, 추론통계, 베이지안 분석 등
  Statistics,
  /// 정보이론: 엔트로피, 채널용량, 코딩이론 등
  InformationTheory,
  /// 게임이론: 내쉬균형, 메커니즘 디자인 등
  GameTheory,

  // 자연과학 (Natural Sciences)
  /// 물리학: 역학, 전자기학, 양자역학, 상대성이론 등
  Physics,
  /// 화학: 원자구조, 화학결합, 반응역학 등
  Chemistry,
  /// 생물학: 세포생물학, 진화론, 생태학 등
  Biology,
  /// 천문학: 천체역학, 우주론, 관측천문학 등
  Astronomy,
  /// 지구과학: 지질학, 기상학, 해양학 등
  EarthScience,
  /// 유전학: 분자유전학, 집단유전학, 유전체학 등
  Genetics,

  // 생명/의료과학 (Life/Medical Sciences)
  /// 의학: 병리학, 진단학, 치료학 등
  Medicine,
  /// 약리학: 약물동역학, 약물상호작용 등
  Pharmacology,
  /// 신경과학: 신경생리학, 인지신경과학 등
  Neuroscience,

  // 인지/행동과학 (Cognitive/Behavioral Sciences)
  /// 정신분석학: 정신역동, 무의식 구조 등
  Psychoanalysis,
  /// 언어학: 음운론, 통사론, 의미론 등
  Linguistics,

  // 공학/응용과학 (Engineering/Applied Sciences)
  /// 컴퓨터과학: 알고리즘, 자료구조, 시스템 등
  ComputerScience,
  /// 공학: 기계, 전기, 토목, 화학공학 등
  Engineering,
  /// 암호학: 대칭/비대칭 암호, 해시, 영지식증명 등
  Cryptography,
  /// 네트워크이론: 그래프이론, 복잡네트워크 등
  NetworkTheory,
  /// 시스템이론: 피드백, 제어이론, 복잡계 등
  SystemsTheory,

  // 사회/인문과학 (Social/Human Sciences)
  /// 경제학: 미시, 거시, 계량경제학 등
  Economics,
  /// 음악이론: 화성학, 대위법, 음향학 등
  MusicTheory,
}

impl TruthDomain {
  /// 모든 TruthDomain은 CT로 형식화 가능하다는 전제
  ///
  /// SETO의 핵심 가정: 이 24개 분야는 공리계와 추론 규칙으로
  /// 객관적 진위 판정이 가능하다.
  #[inline]
  pub const fn is_ct_formalizable(&self) -> bool {
    true
  }

  /// 토픽 문자열을 단순 키워드 매칭으로 분류 (프로토타입)
  ///
  /// 실제 구현에서는 더 정교한 NLP 분류기 사용 예정
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
    } else if contains("computer") {
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
    } else if contains("econom") || contains("경제") {
      Self::Economics
    } else if contains("music") || contains("음악이론") {
      Self::MusicTheory
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
