//! EffectZone: Effect lattice 정의
//!
//! pnix-old의 zone_inference.rs에서 효과 격자를 가져옴.
//!
//! ## Lattice 구조
//!
//! ```text
//! Pure (0) < Symbolic (1) < Frp (2) < Animation (3) < STM (4) < Interop (5) < World (6)
//! ```
//!
//! - **Pure**: 순수 계산, 부작용 없음
//! - **Symbolic**: 심볼릭 연산 (단순화 가능)
//! - **Frp**: Functional Reactive Programming
//! - **Animation**: 애니메이션/시간 의존
//! - **STM**: Software Transactional Memory
//! - **Interop**: 외부 언어 호출 (Clojure, Python, etc.)
//! - **World**: IO/시간/전역상태

use crate::contracts::effect::Effect;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;

const EFFECT_ZONE_TOML: &str = include_str!("../../../../data/seto/effect_zones.seto.toml");

/// Effect Zone - effect 레벨을 나타내는 격자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EffectZone {
  /// 순수 계산 (레벨 0)
  #[default]
  Pure,
  /// 심볼릭 연산 (레벨 1)
  Symbolic,
  /// FRP 연산 (레벨 2)
  Frp,
  /// 애니메이션 (레벨 3)
  Animation,
  /// STM 연산 (레벨 4)
  Stm,
  /// 외부 호출 (레벨 5)
  Interop,
  /// World effect (레벨 6)
  World,
}

impl EffectZone {
  /// Zone 레벨 (격자에서의 위치)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn level(self) -> u8 {
    match self {
      EffectZone::Pure => 0,
      EffectZone::Symbolic => 1,
      EffectZone::Frp => 2,
      EffectZone::Animation => 3,
      EffectZone::Stm => 4,
      EffectZone::Interop => 5,
      EffectZone::World => 6,
    }
  }

  /// 레벨 값에서 EffectZone으로 변환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn from_level(level: u8) -> Option<Self> {
    match level {
      0 => Some(EffectZone::Pure),
      1 => Some(EffectZone::Symbolic),
      2 => Some(EffectZone::Frp),
      3 => Some(EffectZone::Animation),
      4 => Some(EffectZone::Stm),
      5 => Some(EffectZone::Interop),
      6 => Some(EffectZone::World),
      _ => None,
    }
  }

  /// Join (LUB): 두 zone 중 상위 레벨 반환
  ///
  /// 두 효과를 결합하면 더 높은 효과 레벨이 된다.
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 비교만, 값 계산 없음
  pub fn join(self, other: EffectZone) -> EffectZone {
    if self.level() >= other.level() {
      self
    } else {
      other
    }
  }

  /// Meet (GLB): 두 zone 중 하위 레벨 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 비교만, 값 계산 없음
  pub fn meet(self, other: EffectZone) -> EffectZone {
    if self.level() <= other.level() {
      self
    } else {
      other
    }
  }

  /// self가 other의 subzone인지 검사 (self <= other)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 비교만, 값 계산 없음
  pub fn is_subzone_of(self, other: EffectZone) -> bool {
    self.level() <= other.level()
  }

  /// self가 other보다 엄격히 작은지 검사 (self < other)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 비교만, 값 계산 없음
  pub fn is_strictly_below(self, other: EffectZone) -> bool {
    self.level() < other.level()
  }

  /// Effect에서 변환
  ///
  /// **주의**: Effect enum이 Pure/World만 지원하므로 정보 손실 발생
  /// Symbolic/Interop/STM 등은 모두 World로 매핑됨
  /// 향후 Effect enum 확장 필요
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn from_effect(effect: Effect) -> Self {
    match effect {
      Effect::Pure => EffectZone::Pure,
      Effect::World => EffectZone::World,
      // Effect enum에 없는 경우 기본값 (향후 확장 필요)
      _ => EffectZone::World,
    }
  }

  /// Effect로 변환 (lossy - Pure/World만 구분)
  ///
  /// **정보 손실**: Symbolic, Interop, STM 등은 모두 World로 변환됨
  /// 이는 Effect enum의 제한사항으로, 향후 Effect enum 확장 필요
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn to_effect(self) -> Effect {
    if self == EffectZone::Pure {
      Effect::Pure
    } else {
      // Symbolic, Interop, STM 등 모두 World로 변환 (정보 손실)
      Effect::World
    }
  }

  /// 문자열에서 파싱
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn parse(s: &str) -> Option<Self> {
    let lowered = s.trim().to_ascii_lowercase();
    let full_id = if lowered.starts_with("effect/zone/") {
      lowered
    } else {
      format!("effect/zone/{}", lowered)
    };
    let ids = effect_zone_ids();
    ids
      .iter()
      .position(|id| id == &full_id)
      .and_then(|idx| EffectZone::from_level(idx as u8))
  }

  /// 여러 zone들의 join (최상위 레벨)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 비교만, 값 계산 없음
  pub fn join_all(zones: impl IntoIterator<Item = EffectZone>) -> EffectZone {
    zones.into_iter().fold(EffectZone::Pure, EffectZone::join)
  }
}

fn effect_zone_ids() -> &'static Vec<String> {
  static IDS: OnceLock<Vec<String>> = OnceLock::new();
  IDS.get_or_init(|| {
    let mut ids = Vec::new();
    for line in EFFECT_ZONE_TOML.lines() {
      if let Some(id) = parse_effect_zone_id(line) {
        if id.starts_with("effect/zone/") {
          ids.push(id);
        }
      }
    }
    ids
  })
}

fn parse_effect_zone_id(line: &str) -> Option<String> {
  let line = line.trim();
  if line.is_empty() || line.starts_with('#') {
    return None;
  }
  let (key, value) = line.split_once('=')?;
  if key.trim() != "id" {
    return None;
  }
  let value = value.trim();
  let trimmed = value.strip_prefix('"')?;
  let end = trimmed.find('"')?;
  Some(trimmed[..end].to_ascii_lowercase())
}

impl fmt::Display for EffectZone {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      EffectZone::Pure => write!(f, "Pure"),
      EffectZone::Symbolic => write!(f, "Symbolic"),
      EffectZone::Frp => write!(f, "Frp"),
      EffectZone::Animation => write!(f, "Animation"),
      EffectZone::Stm => write!(f, "STM"),
      EffectZone::Interop => write!(f, "Interop"),
      EffectZone::World => write!(f, "World"),
    }
  }
}

impl PartialOrd for EffectZone {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for EffectZone {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.level().cmp(&other.level())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_zone_level() {
    assert_eq!(EffectZone::Pure.level(), 0);
    assert_eq!(EffectZone::Symbolic.level(), 1);
    assert_eq!(EffectZone::Frp.level(), 2);
    assert_eq!(EffectZone::Animation.level(), 3);
    assert_eq!(EffectZone::Stm.level(), 4);
    assert_eq!(EffectZone::Interop.level(), 5);
    assert_eq!(EffectZone::World.level(), 6);
  }

  #[test]
  fn test_zone_join() {
    use EffectZone::*;

    assert_eq!(Pure.join(Pure), Pure);
    assert_eq!(Pure.join(Stm), Stm);
    assert_eq!(Stm.join(Interop), Interop);
    assert_eq!(Interop.join(World), World);
    assert_eq!(Pure.join(World), World);

    // Commutativity
    assert_eq!(Stm.join(Pure), Pure.join(Stm));
    assert_eq!(World.join(Interop), Interop.join(World));
  }

  #[test]
  fn test_zone_meet() {
    use EffectZone::*;

    assert_eq!(Pure.meet(Pure), Pure);
    assert_eq!(Stm.meet(Pure), Pure);
    assert_eq!(World.meet(Interop), Interop);
    assert_eq!(World.meet(Pure), Pure);
  }

  #[test]
  fn test_zone_lattice_examples() {
    assert_eq!(EffectZone::Pure.join(EffectZone::Frp), EffectZone::Frp);
    assert_eq!(EffectZone::Frp.meet(EffectZone::Stm), EffectZone::Frp);
  }

  #[test]
  fn test_zone_subzone() {
    use EffectZone::*;

    assert!(Pure.is_subzone_of(Pure));
    assert!(Pure.is_subzone_of(Stm));
    assert!(Pure.is_subzone_of(World));
    assert!(Stm.is_subzone_of(Stm));
    assert!(Stm.is_subzone_of(Interop));
    assert!(!Stm.is_subzone_of(Pure));
    assert!(!World.is_subzone_of(Interop));

    // Symbolic tests
    assert!(Pure.is_subzone_of(Symbolic));
    assert!(Symbolic.is_subzone_of(Symbolic));
    assert!(Symbolic.is_subzone_of(Stm));
    assert!(Symbolic.is_subzone_of(World));
    assert!(!Symbolic.is_subzone_of(Pure));
  }

  #[test]
  fn test_zone_ordering() {
    use EffectZone::*;

    assert!(Pure < Symbolic);
    assert!(Symbolic < Frp);
    assert!(Frp < Animation);
    assert!(Animation < Stm);
    assert!(Stm < Interop);
    assert!(Interop < World);
  }

  #[test]
  fn test_zone_parse() {
    assert_eq!(EffectZone::parse("pure"), Some(EffectZone::Pure));
    assert_eq!(EffectZone::parse("Pure"), Some(EffectZone::Pure));
    assert_eq!(EffectZone::parse("WORLD"), Some(EffectZone::World));
    assert_eq!(EffectZone::parse("Frp"), Some(EffectZone::Frp));
    assert_eq!(EffectZone::parse("stm"), Some(EffectZone::Stm));
    assert_eq!(
      EffectZone::parse("effect/zone/interop"),
      Some(EffectZone::Interop)
    );
    assert_eq!(EffectZone::parse("unknown"), None);
  }

  #[test]
  fn test_zone_join_all() {
    use EffectZone::*;

    assert_eq!(EffectZone::join_all(vec![Pure, Pure, Pure]), Pure);
    assert_eq!(EffectZone::join_all(vec![Pure, Stm, Pure]), Stm);
    assert_eq!(EffectZone::join_all(vec![Interop, Pure, Stm]), Interop);
    assert_eq!(EffectZone::join_all(vec![World]), World);
    assert_eq!(EffectZone::join_all(vec![]), Pure);
  }

  #[test]
  fn test_effect_conversion() {
    use EffectZone::*;

    // From Effect
    assert_eq!(EffectZone::from_effect(Effect::Pure), Pure);
    assert_eq!(EffectZone::from_effect(Effect::World), World);

    // To Effect (lossy)
    assert_eq!(Pure.to_effect(), Effect::Pure);
    assert_eq!(Symbolic.to_effect(), Effect::World);
    assert_eq!(Stm.to_effect(), Effect::World);
    assert_eq!(World.to_effect(), Effect::World);
  }
}

// ============================================================
// TimeKind: FRP 시간 분류
// ============================================================

/// 시간 성질 (FRP용)
///
/// FRP(Functional Reactive Programming)에서 시간의 성질을 분류합니다.
///
/// # 헌법 준수 (P0-1)
///
/// 순수 분류 enum, 값 연산 없음
///
/// # 예시
/// ```ignore
/// use pnix_core::effects::TimeKind;
///
/// let time = TimeKind::Continuous;
/// assert!(time.is_dynamic());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeKind {
  /// 정적 값 (상수, 시간 독립)
  #[default]
  Static,
  /// 연속 시간 (t)
  Continuous,
  /// 이산 시간 (tick(dt))
  Discrete,
}

impl TimeKind {
  /// 정적 시간 생성
  pub fn static_time() -> Self {
    TimeKind::Static
  }

  /// 연속 시간 생성
  pub fn continuous() -> Self {
    TimeKind::Continuous
  }

  /// 이산 시간 생성
  pub fn discrete() -> Self {
    TimeKind::Discrete
  }

  /// 동적(시간 의존적)인지 확인
  pub fn is_dynamic(&self) -> bool {
    matches!(self, TimeKind::Continuous | TimeKind::Discrete)
  }

  /// 정적인지 확인
  pub fn is_static(&self) -> bool {
    matches!(self, TimeKind::Static)
  }

  /// 이름 반환
  pub fn name(&self) -> &'static str {
    match self {
      TimeKind::Static => "static",
      TimeKind::Continuous => "continuous",
      TimeKind::Discrete => "discrete",
    }
  }

  /// 문자열에서 TimeKind 파싱
  pub fn parse(s: &str) -> Option<Self> {
    match s.trim().to_ascii_lowercase().as_str() {
      "static" | "stat" => Some(TimeKind::Static),
      "continuous" | "cont" => Some(TimeKind::Continuous),
      "discrete" | "disc" => Some(TimeKind::Discrete),
      _ => None,
    }
  }

  /// 두 TimeKind를 결합 (더 동적인 쪽 반환)
  ///
  /// Static < Discrete < Continuous
  pub fn join(self, other: TimeKind) -> TimeKind {
    match (self, other) {
      (TimeKind::Continuous, _) | (_, TimeKind::Continuous) => TimeKind::Continuous,
      (TimeKind::Discrete, _) | (_, TimeKind::Discrete) => TimeKind::Discrete,
      _ => TimeKind::Static,
    }
  }
}

impl fmt::Display for TimeKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.name())
  }
}

#[cfg(test)]
mod time_kind_tests {
  use super::*;

  #[test]
  fn test_time_kind_default() {
    assert_eq!(TimeKind::default(), TimeKind::Static);
  }

  #[test]
  fn test_time_kind_constructors() {
    assert_eq!(TimeKind::static_time(), TimeKind::Static);
    assert_eq!(TimeKind::continuous(), TimeKind::Continuous);
    assert_eq!(TimeKind::discrete(), TimeKind::Discrete);
  }

  #[test]
  fn test_time_kind_is_dynamic() {
    assert!(!TimeKind::Static.is_dynamic());
    assert!(TimeKind::Continuous.is_dynamic());
    assert!(TimeKind::Discrete.is_dynamic());
  }

  #[test]
  fn test_time_kind_is_static() {
    assert!(TimeKind::Static.is_static());
    assert!(!TimeKind::Continuous.is_static());
    assert!(!TimeKind::Discrete.is_static());
  }

  #[test]
  fn test_time_kind_name() {
    assert_eq!(TimeKind::Static.name(), "static");
    assert_eq!(TimeKind::Continuous.name(), "continuous");
    assert_eq!(TimeKind::Discrete.name(), "discrete");
  }

  #[test]
  fn test_time_kind_parse() {
    assert_eq!(TimeKind::parse("static"), Some(TimeKind::Static));
    assert_eq!(TimeKind::parse("STAT"), Some(TimeKind::Static));
    assert_eq!(TimeKind::parse("continuous"), Some(TimeKind::Continuous));
    assert_eq!(TimeKind::parse("cont"), Some(TimeKind::Continuous));
    assert_eq!(TimeKind::parse("discrete"), Some(TimeKind::Discrete));
    assert_eq!(TimeKind::parse("disc"), Some(TimeKind::Discrete));
    assert_eq!(TimeKind::parse("unknown"), None);
  }

  #[test]
  fn test_time_kind_join() {
    use TimeKind::*;

    // Continuous dominates
    assert_eq!(Static.join(Continuous), Continuous);
    assert_eq!(Continuous.join(Static), Continuous);
    assert_eq!(Continuous.join(Discrete), Continuous);
    assert_eq!(Discrete.join(Continuous), Continuous);

    // Discrete dominates Static
    assert_eq!(Static.join(Discrete), Discrete);
    assert_eq!(Discrete.join(Static), Discrete);

    // Same returns same
    assert_eq!(Static.join(Static), Static);
    assert_eq!(Discrete.join(Discrete), Discrete);
    assert_eq!(Continuous.join(Continuous), Continuous);
  }

  #[test]
  fn test_time_kind_display() {
    assert_eq!(format!("{}", TimeKind::Static), "static");
    assert_eq!(format!("{}", TimeKind::Continuous), "continuous");
    assert_eq!(format!("{}", TimeKind::Discrete), "discrete");
  }

  #[test]
  fn test_time_kind_serde() {
    let time = TimeKind::Continuous;
    let json = serde_json::to_string(&time).unwrap();
    assert_eq!(json, "\"continuous\"");

    let restored: TimeKind = serde_json::from_str(&json).unwrap();
    assert_eq!(time, restored);
  }
}
