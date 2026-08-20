//! 텐서/인덱스 타입 시스템
//!
//! pnix-old의 symbolic_core/src/ast/tensor.rs에서 마이그레이션
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 구조 타입, 값 연산 없음
//!
//! ## 사용 목적
//!
//! - 물리학/수학 텐서 표기법 지원
//! - CT 타입 시스템 확장
//! - 인덱스 공간 추적 (spacetime, gauge group 등)

use serde::{Deserialize, Serialize};

/// 인덱스 위치: 상첨자(contravariant) / 하첨자(covariant)
///
/// # 예시
/// - `Up`: V^μ (contravariant, 상첨자)
/// - `Down`: V_μ (covariant, 하첨자)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Variance {
  /// Contravariant (상첨자) - V^μ
  Up,
  /// Covariant (하첨자) - V_μ
  Down,
}

impl Variance {
  /// 반대 variance 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn opposite(&self) -> Self {
    match self {
      Variance::Up => Variance::Down,
      Variance::Down => Variance::Up,
    }
  }

  /// LaTeX 표기법 접두사
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 접근만, 값 계산 없음
  pub fn latex_prefix(&self) -> &'static str {
    match self {
      Variance::Up => "^",
      Variance::Down => "_",
    }
  }
}

/// 텐서 인덱스
///
/// # 필드
/// - `name`: 인덱스 이름 (μ, ν, a, b, i, j 등)
/// - `variance`: Up(상첨자) / Down(하첨자)
/// - `space`: 인덱스 공간 ("spacetime", "worldsheet", "gauge(SU(3))" 등)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Index {
  /// 인덱스 이름 (μ, ν, a, b, i, j 등)
  pub name: String,
  /// 상첨자/하첨자
  pub variance: Variance,
  /// 인덱스 공간 ("spacetime", "worldsheet", "gauge(SU(3))" 등)
  pub space: String,
}

impl Index {
  /// 새 인덱스 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(name: impl Into<String>, variance: Variance, space: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      variance,
      space: space.into(),
    }
  }

  /// 상첨자 인덱스 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn up(name: impl Into<String>, space: impl Into<String>) -> Self {
    Self::new(name, Variance::Up, space)
  }

  /// 하첨자 인덱스 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn down(name: impl Into<String>, space: impl Into<String>) -> Self {
    Self::new(name, Variance::Down, space)
  }

  /// spacetime 공간 상첨자 인덱스
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn spacetime_up(name: impl Into<String>) -> Self {
    Self::up(name, "spacetime")
  }

  /// spacetime 공간 하첨자 인덱스
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn spacetime_down(name: impl Into<String>) -> Self {
    Self::down(name, "spacetime")
  }

  /// 같은 이름, 반대 variance인지 확인 (수축 조건)
  ///
  /// 텐서 수축 시 상첨자와 하첨자가 쌍을 이루어야 함
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn contracts_with(&self, other: &Index) -> bool {
    self.name == other.name && self.space == other.space && self.variance != other.variance
  }

  /// LaTeX 표기법
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn to_latex(&self) -> String {
    format!("{}{{{}}}", self.variance.latex_prefix(), self.name)
  }
}

/// 텐서 대칭성
///
/// # 변형
/// - `Symmetric`: 특정 인덱스 위치들이 대칭 (g_{μν} = g_{νμ})
/// - `AntiSymmetric`: 특정 인덱스 위치들이 반대칭 (F_{μν} = -F_{νμ})
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// 텐서 대칭성: 인덱스의 대칭/반대칭 관계
#[serde(tag = "type", content = "indices")]
pub enum Symmetry {
  /// 대칭 인덱스 위치들: g_{μν} = g_{νμ}
  Symmetric(
    /// 대칭 인덱스 위치 목록
    Vec<usize>,
  ),
  /// 반대칭 인덱스 위치들: F_{μν} = -F_{νμ}
  AntiSymmetric(
    /// 반대칭 인덱스 위치 목록
    Vec<usize>,
  ),
}

impl Symmetry {
  /// 두 인덱스 위치가 대칭
  pub fn symmetric_pair(i: usize, j: usize) -> Self {
    Symmetry::Symmetric(vec![i, j])
  }

  /// 두 인덱스 위치가 반대칭
  pub fn antisymmetric_pair(i: usize, j: usize) -> Self {
    Symmetry::AntiSymmetric(vec![i, j])
  }

  /// 관련 인덱스 위치 반환
  pub fn indices(&self) -> &[usize] {
    match self {
      Symmetry::Symmetric(idx) => idx,
      Symmetry::AntiSymmetric(idx) => idx,
    }
  }

  /// 대칭인지 여부
  pub fn is_symmetric(&self) -> bool {
    matches!(self, Symmetry::Symmetric(_))
  }

  /// 반대칭인지 여부
  pub fn is_antisymmetric(&self) -> bool {
    matches!(self, Symmetry::AntiSymmetric(_))
  }
}

/// 텐서 심볼
///
/// # 필드
/// - `name`: 텐서 이름 (g, F, R, T, X, A 등)
/// - `indices`: 인덱스 목록
/// - `symmetries`: 대칭성 정보
/// - `bundle`: (선택) fiber bundle / representation
///
/// # 예시
/// - g_{μν}: 메트릭 텐서 (대칭)
/// - F_{μν}: 전자기장 텐서 (반대칭)
/// - R^ρ_{σμν}: 리만 곡률 텐서
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TensorSymbol {
  /// 텐서 이름 (g, F, R, T, X, A 등)
  pub name: String,
  /// 인덱스 목록
  pub indices: Vec<Index>,
  /// 대칭성 정보
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub symmetries: Vec<Symmetry>,
  /// Fiber bundle / representation (선택)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bundle: Option<String>,
}

impl TensorSymbol {
  /// 새 텐서 심볼 생성
  pub fn new(name: impl Into<String>, indices: Vec<Index>) -> Self {
    Self {
      name: name.into(),
      indices,
      symmetries: vec![],
      bundle: None,
    }
  }

  /// 스칼라 (인덱스 없는 텐서)
  pub fn scalar(name: impl Into<String>) -> Self {
    Self::new(name, vec![])
  }

  /// 대칭성 추가
  pub fn with_symmetry(mut self, sym: Symmetry) -> Self {
    self.symmetries.push(sym);
    self
  }

  /// 대칭 텐서로 설정 (처음 두 인덱스)
  pub fn symmetric(mut self) -> Self {
    if self.indices.len() >= 2 {
      self.symmetries.push(Symmetry::symmetric_pair(0, 1));
    }
    self
  }

  /// 반대칭 텐서로 설정 (처음 두 인덱스)
  pub fn antisymmetric(mut self) -> Self {
    if self.indices.len() >= 2 {
      self.symmetries.push(Symmetry::antisymmetric_pair(0, 1));
    }
    self
  }

  /// Bundle 정보 추가
  pub fn with_bundle(mut self, bundle: impl Into<String>) -> Self {
    self.bundle = Some(bundle.into());
    self
  }

  /// 자유 인덱스 (수축되지 않는 인덱스) 추출
  pub fn free_indices(&self) -> Vec<&Index> {
    self.indices.iter().collect()
  }

  /// 인덱스 이름으로 검색
  pub fn get_index(&self, name: &str) -> Option<&Index> {
    self.indices.iter().find(|i| i.name == name)
  }

  /// Rank (총 인덱스 개수)
  pub fn rank(&self) -> usize {
    self.indices.len()
  }

  /// 상첨자 개수 (contravariant rank)
  pub fn contravariant_rank(&self) -> usize {
    self
      .indices
      .iter()
      .filter(|i| i.variance == Variance::Up)
      .count()
  }

  /// 하첨자 개수 (covariant rank)
  pub fn covariant_rank(&self) -> usize {
    self
      .indices
      .iter()
      .filter(|i| i.variance == Variance::Down)
      .count()
  }

  /// 혼합 텐서인지 (상첨자와 하첨자 모두 있음)
  pub fn is_mixed(&self) -> bool {
    self.contravariant_rank() > 0 && self.covariant_rank() > 0
  }

  /// LaTeX 표기법
  pub fn to_latex(&self) -> String {
    if self.indices.is_empty() {
      return self.name.clone();
    }

    let indices_latex: String = self.indices.iter().map(|i| i.to_latex()).collect();
    format!("{}{}", self.name, indices_latex)
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_variance() {
    assert_eq!(Variance::Up.opposite(), Variance::Down);
    assert_eq!(Variance::Down.opposite(), Variance::Up);
    assert_eq!(Variance::Up.latex_prefix(), "^");
    assert_eq!(Variance::Down.latex_prefix(), "_");
  }

  #[test]
  fn test_index_creation() {
    let mu_up = Index::up("μ", "spacetime");
    assert_eq!(mu_up.name, "μ");
    assert_eq!(mu_up.variance, Variance::Up);
    assert_eq!(mu_up.space, "spacetime");

    let nu_down = Index::down("ν", "spacetime");
    assert_eq!(nu_down.variance, Variance::Down);
  }

  #[test]
  fn test_index_contraction() {
    let mu_up = Index::up("μ", "spacetime");
    let mu_down = Index::down("μ", "spacetime");
    let nu_down = Index::down("ν", "spacetime");

    assert!(mu_up.contracts_with(&mu_down));
    assert!(mu_down.contracts_with(&mu_up));
    assert!(!mu_up.contracts_with(&nu_down));
    assert!(!mu_up.contracts_with(&Index::up("μ", "spacetime"))); // same variance
  }

  #[test]
  fn test_index_latex() {
    assert_eq!(Index::up("μ", "spacetime").to_latex(), "^{μ}");
    assert_eq!(Index::down("ν", "spacetime").to_latex(), "_{ν}");
  }

  #[test]
  fn test_symmetry() {
    let sym = Symmetry::symmetric_pair(0, 1);
    assert!(sym.is_symmetric());
    assert!(!sym.is_antisymmetric());
    assert_eq!(sym.indices(), &[0, 1]);

    let asym = Symmetry::antisymmetric_pair(0, 1);
    assert!(!asym.is_symmetric());
    assert!(asym.is_antisymmetric());
  }

  #[test]
  fn test_tensor_symbol_scalar() {
    let phi = TensorSymbol::scalar("φ");
    assert_eq!(phi.rank(), 0);
    assert_eq!(phi.to_latex(), "φ");
  }

  #[test]
  fn test_tensor_symbol_vector() {
    let v = TensorSymbol::new("V", vec![Index::up("μ", "spacetime")]);
    assert_eq!(v.rank(), 1);
    assert_eq!(v.contravariant_rank(), 1);
    assert_eq!(v.covariant_rank(), 0);
    assert!(!v.is_mixed());
    assert_eq!(v.to_latex(), "V^{μ}");
  }

  #[test]
  fn test_tensor_symbol_metric() {
    let g = TensorSymbol::new(
      "g",
      vec![Index::down("μ", "spacetime"), Index::down("ν", "spacetime")],
    )
    .symmetric();

    assert_eq!(g.rank(), 2);
    assert_eq!(g.covariant_rank(), 2);
    assert!(g.symmetries[0].is_symmetric());
    assert_eq!(g.to_latex(), "g_{μ}_{ν}");
  }

  #[test]
  fn test_tensor_symbol_field_strength() {
    let f = TensorSymbol::new(
      "F",
      vec![Index::down("μ", "spacetime"), Index::down("ν", "spacetime")],
    )
    .antisymmetric();

    assert_eq!(f.rank(), 2);
    assert!(f.symmetries[0].is_antisymmetric());
  }

  #[test]
  fn test_tensor_symbol_mixed() {
    let riemann = TensorSymbol::new(
      "R",
      vec![
        Index::up("ρ", "spacetime"),
        Index::down("σ", "spacetime"),
        Index::down("μ", "spacetime"),
        Index::down("ν", "spacetime"),
      ],
    );

    assert_eq!(riemann.rank(), 4);
    assert_eq!(riemann.contravariant_rank(), 1);
    assert_eq!(riemann.covariant_rank(), 3);
    assert!(riemann.is_mixed());
  }

  #[test]
  fn test_tensor_symbol_get_index() {
    let t = TensorSymbol::new(
      "T",
      vec![Index::up("a", "spacetime"), Index::down("b", "spacetime")],
    );

    assert!(t.get_index("a").is_some());
    assert!(t.get_index("b").is_some());
    assert!(t.get_index("c").is_none());
  }

  #[test]
  fn test_tensor_serde_roundtrip() {
    let t = TensorSymbol::new(
      "g",
      vec![Index::down("μ", "spacetime"), Index::down("ν", "spacetime")],
    )
    .symmetric();

    let json = serde_json::to_string(&t).unwrap();
    let restored: TensorSymbol = serde_json::from_str(&json).unwrap();
    assert_eq!(t, restored);
  }
}
