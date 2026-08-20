//! CTAST - Category Theory AST for pnix-new
//!
//! pnix-old의 ctast.rs를 그래프 기반 pnix-new에 맞게 재설계.
//!
//! ## 핵심 개념
//!
//! - **CTType**: 범주의 대상 (Object)
//! - **CTNode**: 범주의 사상 (Morphism) 표현 노드
//! - **CTAST**: FxCoreModule의 범주론적 표현
//!
//! ## CT 법칙 검증
//!
//! - Functor laws: fmap id = id, fmap (f . g) = fmap f . fmap g
//! - Monad laws: return x >>= f = f x, m >>= return = m
//! - Natural transformation: η_B ∘ F(f) = G(f) ∘ η_A

use crate::core::FxCoreModule;
use crate::effects::EffectZone;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// ============================================================
// CT Type System
// ============================================================

/// CT Type - 범주의 대상 (Object): 범주론적 타입 시스템의 타입
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CTType {
  /// Unit type (terminal object)
  #[default]
  Unit,
  /// Boolean
  Bool,
  /// Integer
  Int,
  /// Real number
  Real,
  /// String
  String,
  /// List of type (Functor)
  List(
    /// 내부 타입
    Box<CTType>,
  ),
  /// Function type A → B (exponential object)
  Arrow(
    /// 입력 타입
    Box<CTType>,
    /// 출력 타입
    Box<CTType>,
  ),
  /// Product type A × B
  Product(
    /// 첫 번째 타입
    Box<CTType>,
    /// 두 번째 타입
    Box<CTType>,
  ),
  /// Sum type A + B (coproduct)
  Sum(
    /// 첫 번째 타입
    Box<CTType>,
    /// 두 번째 타입
    Box<CTType>,
  ),
  /// Signal (FRP behavior - time-varying value)
  Signal(
    /// 내부 타입
    Box<CTType>,
  ),
  /// Effect wrapper with zone
  Effect(
    /// 효과 영역
    EffectZone,
    /// 내부 타입
    Box<CTType>,
  ),
  /// Named type (from DSL)
  Named(
    /// 타입 이름
    std::string::String,
  ),
  /// Type variable for polymorphism
  Var(
    /// 타입 변수 이름
    std::string::String,
  ),
}

impl CTType {
  // Constructors
  /// Real 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn real() -> Self {
    CTType::Real
  }
  /// Bool 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn bool() -> Self {
    CTType::Bool
  }
  /// Int 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn int() -> Self {
    CTType::Int
  }
  /// Unit 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn unit() -> Self {
    CTType::Unit
  }
  /// String 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn string() -> Self {
    CTType::String
  }

  /// List 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn list(inner: CTType) -> Self {
    CTType::List(Box::new(inner))
  }

  /// 함수 타입 생성 (A → B)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn arrow(from: CTType, to: CTType) -> Self {
    CTType::Arrow(Box::new(from), Box::new(to))
  }

  /// 곱 타입 생성 (A × B)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn product(a: CTType, b: CTType) -> Self {
    CTType::Product(Box::new(a), Box::new(b))
  }

  /// 합 타입 생성 (A + B)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn sum(a: CTType, b: CTType) -> Self {
    CTType::Sum(Box::new(a), Box::new(b))
  }

  /// Signal 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn signal(inner: CTType) -> Self {
    CTType::Signal(Box::new(inner))
  }

  /// Effect 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn effect(zone: EffectZone, inner: CTType) -> Self {
    CTType::Effect(zone, Box::new(inner))
  }

  /// 명명된 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn named(s: impl Into<std::string::String>) -> Self {
    CTType::Named(s.into())
  }

  /// 타입 변수 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn var(s: impl Into<std::string::String>) -> Self {
    CTType::Var(s.into())
  }

  /// Check if this type is pure (no effects)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_pure(&self) -> bool {
    match self {
      CTType::Effect(zone, _) => *zone == EffectZone::Pure,
      _ => true,
    }
  }

  /// Parse from type string
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 파싱만, 값 계산 없음
  pub fn parse(s: &str) -> Self {
    let s = s.trim();
    match s {
      "" | "()" | "Unit" | "unit" => CTType::Unit,
      "Int" | "Integer" | "int" | "i64" => CTType::Int,
      "Real" | "Float" | "Double" | "f64" | "real" => CTType::Real,
      "Bool" | "Boolean" | "bool" => CTType::Bool,
      "String" | "Str" | "string" => CTType::String,
      _ if s.starts_with("List<") || s.starts_with("[") => {
        // Simplified: just extract inner type
        CTType::list(CTType::var("T"))
      }
      _ if s.starts_with("Signal<") => CTType::signal(CTType::var("T")),
      _ => CTType::Named(s.to_string()),
    }
  }

  /// CT 카테고리 기호 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn symbol(&self) -> &'static str {
    match self {
      CTType::Real => "ℝ",
      CTType::Int => "ℤ",
      CTType::Bool => "𝔹",
      CTType::String => "𝕊",
      CTType::Unit => "1",
      CTType::Product(_, _) => "×",
      CTType::Sum(_, _) => "+",
      CTType::Arrow(_, _) => "→",
      CTType::List(_) => "[]",
      CTType::Signal(_) => "𝕋→",
      CTType::Effect(_, _) => "◇",
      CTType::Named(_) => "•",
      CTType::Var(_) => "α",
    }
  }
}

// ============================================================
// CT Literal
// ============================================================

/// CT Literal values: CT 리터럴 값 타입
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CTLit {
  Unit,
  Bool(
    /// 불리언 값
    bool,
  ),
  Int(
    /// 정수 값
    i64,
  ),
  Real(
    /// 실수 값
    f64,
  ),
  String(
    /// 문자열 값
    std::string::String,
  ),
}

impl CTLit {
  /// 리터럴의 타입 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn ty(&self) -> CTType {
    match self {
      CTLit::Unit => CTType::Unit,
      CTLit::Bool(_) => CTType::Bool,
      CTLit::Int(_) => CTType::Int,
      CTLit::Real(_) => CTType::Real,
      CTLit::String(_) => CTType::String,
    }
  }
}

// ============================================================
// CT Morphism Operations
// ============================================================

/// CT Morphism 연산: 범주론적 기본 연산
///
/// pnix-old의 UnifiedMeaningOp (200+ ops)에서 필수 범주론 연산만으로 단순화됨.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CTMorphismOp {
  // ─── Identity & Composition ───
  /// 항등 사상: id_A : A → A
  Id,
  /// 합성: g ∘ f : A → C (f: A → B, g: B → C일 때)
  Compose,

  // ─── Product (×) ───
  /// Pair: <f, g> : A → B × C
  Pair,
  /// First projection: π₁ : A × B → A
  Fst,
  /// Second projection: π₂ : A × B → B
  Snd,

  // ─── Sum (+) ───
  /// Left injection: inl : A → A + B
  Inl,
  /// Right injection: inr : B → A + B
  Inr,
  /// Case analysis: [f, g] : A + B → C
  Case,

  // ─── Exponential (→) ───
  /// Lambda abstraction: λ
  Lam,
  /// Application: eval : (A → B) × A → B
  App,
  /// Curry: curry(f) : A → (B → C) when f: A × B → C
  Curry,
  /// Uncurry: uncurry(f) : A × B → C when f: A → (B → C)
  Uncurry,

  // ─── Functor operations ───
  /// Functor map: fmap : (A → B) → F A → F B
  Fmap,

  // ─── Monad operations ───
  /// Monadic return: return : A → M A
  Return,
  /// Monadic bind: (>>=) : M A → (A → M B) → M B
  Bind,
  /// Monadic join: join : M (M A) → M A
  Join,

  // ─── FRP Signal operations ───
  /// Time signal: time : 1 → Signal Real
  Time,
  /// Signal lift: lift : (A → B) → Signal A → Signal B
  Lift,
  /// Signal hold: hold : A → Event A → Signal A
  Hold,

  // ─── Arithmetic (Pure) ───
  Add,
  Sub,
  Mul,
  Div,
  Neg,
  Abs,
  Sqrt,
  Sin,
  Cos,

  // ─── Comparison (Pure) ───
  Lt,
  Le,
  Gt,
  Ge,
  Eq,
  Ne,

  // ─── Logic (Pure) ───
  And,
  Or,
  Not,

  // ─── Control flow ───
  If,

  // ─── External/Effect ───
  Extern,
}

impl CTMorphismOp {
  /// 연산의 효과 영역 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn zone(self) -> EffectZone {
    match self {
      // Pure operations
      CTMorphismOp::Id
      | CTMorphismOp::Compose
      | CTMorphismOp::Pair
      | CTMorphismOp::Fst
      | CTMorphismOp::Snd
      | CTMorphismOp::Inl
      | CTMorphismOp::Inr
      | CTMorphismOp::Case
      | CTMorphismOp::Lam
      | CTMorphismOp::App
      | CTMorphismOp::Curry
      | CTMorphismOp::Uncurry
      | CTMorphismOp::Fmap
      | CTMorphismOp::Return
      | CTMorphismOp::Bind
      | CTMorphismOp::Join
      | CTMorphismOp::Add
      | CTMorphismOp::Sub
      | CTMorphismOp::Mul
      | CTMorphismOp::Div
      | CTMorphismOp::Neg
      | CTMorphismOp::Abs
      | CTMorphismOp::Sqrt
      | CTMorphismOp::Sin
      | CTMorphismOp::Cos
      | CTMorphismOp::Lt
      | CTMorphismOp::Le
      | CTMorphismOp::Gt
      | CTMorphismOp::Ge
      | CTMorphismOp::Eq
      | CTMorphismOp::Ne
      | CTMorphismOp::And
      | CTMorphismOp::Or
      | CTMorphismOp::Not
      | CTMorphismOp::If => EffectZone::Pure,

      // FRP operations
      CTMorphismOp::Time | CTMorphismOp::Lift | CTMorphismOp::Hold => EffectZone::Frp,

      // External operations
      CTMorphismOp::Extern => EffectZone::Interop,
    }
  }

  /// Functor map 연산인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_fmap(self) -> bool {
    matches!(self, CTMorphismOp::Fmap | CTMorphismOp::Lift)
  }

  /// Monad 연산인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_monad(self) -> bool {
    matches!(
      self,
      CTMorphismOp::Return | CTMorphismOp::Bind | CTMorphismOp::Join
    )
  }
}

// ============================================================
// CT Node (Morphism representation)
// ============================================================

/// CT 노드: 범주에서 사상을 나타내는 노드
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CTNode {
  /// 노드 이름 (FxNode.name에서 가져옴)
  pub name: String,
  /// 사상 연산
  pub op: CTMorphismOp,
  /// 소스 타입 (정의역)
  pub src: CTType,
  /// 타겟 타입 (공역)
  pub tgt: CTType,
  /// 효과 영역
  pub zone: EffectZone,
  /// 입력 노드 목록 (의존성)
  pub inputs: Vec<String>,
}

impl CTNode {
  /// 새 CT 노드 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(
    name: impl Into<String>,
    op: CTMorphismOp,
    src: CTType,
    tgt: CTType,
    inputs: Vec<String>,
  ) -> Self {
    Self {
      name: name.into(),
      op,
      src,
      tgt,
      zone: op.zone(),
      inputs,
    }
  }

  /// 항등 사상인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_identity(&self) -> bool {
    self.op == CTMorphismOp::Id || (self.inputs.len() == 1 && self.src == self.tgt)
  }
}

// ============================================================
// CTAST - Category Theory AST for FxCoreModule
// ============================================================

/// CTAST: FxCoreModule의 범주론적 표현
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CTAST {
  /// 노드 목록 (사상들)
  pub nodes: Vec<CTNode>,
  /// 엣지 목록 (합성 경로)
  pub edges: Vec<(String, String)>,
  /// 전체 효과 영역 (모든 노드의 join)
  pub zone: EffectZone,
}

impl CTAST {
  /// 새 CTAST 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self::default()
  }

  /// FxCoreModule에서 CTAST 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn from_fxcore(module: &FxCoreModule) -> Self {
    let mut ctast = CTAST::new();
    let mut node_zones = Vec::new();

    // Convert nodes to CT nodes
    for node in &module.nodes {
      // Find morphism definition
      let morphism = module.morphisms.iter().find(|m| m.name == node.uses);

      let (src, tgt, _zone) = if let Some(m) = morphism {
        let src = CTType::parse(&m.input);
        let tgt = CTType::parse(&m.output);
        let zone = EffectZone::from_effect(m.effect);
        (src, tgt, zone)
      } else {
        (CTType::Unit, CTType::var("?"), EffectZone::Pure)
      };

      // Infer operation from morphism name
      let op = infer_op_from_name(&node.uses);

      // Collect input nodes from edges
      let inputs: Vec<String> = module
        .edges
        .iter()
        .filter(|e| e.to == node.name)
        .map(|e| {
          if let Some(input_name) = &e.from_input {
            format!("input.{}", input_name)
          } else {
            e.from.clone()
          }
        })
        .collect();

      let ct_node = CTNode::new(&node.name, op, src, tgt, inputs);
      node_zones.push(ct_node.zone);
      ctast.nodes.push(ct_node);
    }

    // Convert edges
    for edge in &module.edges {
      let from = if let Some(input_name) = &edge.from_input {
        format!("input.{}", input_name)
      } else {
        edge.from.clone()
      };
      ctast.edges.push((from, edge.to.clone()));
    }

    // Overall zone is join of all node zones
    ctast.zone = EffectZone::join_all(node_zones);

    ctast
  }

  /// 이름으로 노드 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get_node(&self, name: &str) -> Option<&CTNode> {
    self.nodes.iter().find(|n| n.name == name)
  }

  /// 위상 정렬 순서로 노드 목록 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn topo_order(&self) -> Vec<&CTNode> {
    // Build deterministic node map
    let mut nodes_by_name: BTreeMap<&str, &CTNode> = BTreeMap::new();
    for node in &self.nodes {
      nodes_by_name.insert(node.name.as_str(), node);
    }

    // Build dependency map (deterministic ordering)
    let mut in_degree: BTreeMap<&str, usize> = BTreeMap::new();
    let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for (&name, node) in &nodes_by_name {
      in_degree.entry(name).or_insert(0);
      for input in &node.inputs {
        *in_degree.entry(name).or_insert(0) += 1;
        outgoing.entry(input.as_str()).or_default().push(name);
      }
    }

    for neighbors in outgoing.values_mut() {
      neighbors.sort();
    }

    // Kahn's algorithm with deterministic queue
    let mut queue: BTreeSet<&str> = in_degree
      .iter()
      .filter(|(_, d)| **d == 0)
      .map(|(n, _)| *n)
      .collect();

    let mut order: Vec<&CTNode> = Vec::new();

    while let Some(name) = queue.pop_first() {
      if let Some(node) = nodes_by_name.get(name) {
        order.push(*node);
      }

      if let Some(neighbors) = outgoing.get(name) {
        for &neighbor in neighbors {
          if let Some(deg) = in_degree.get_mut(neighbor) {
            *deg = deg.saturating_sub(1);
            if *deg == 0 {
              queue.insert(neighbor);
            }
          }
        }
      }
    }

    order
  }

  /// 통계 정보 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn stats(&self) -> CTASTStats {
    CTASTStats {
      num_nodes: self.nodes.len(),
      num_edges: self.edges.len(),
      pure_nodes: self
        .nodes
        .iter()
        .filter(|n| n.zone == EffectZone::Pure)
        .count(),
      effect_nodes: self
        .nodes
        .iter()
        .filter(|n| n.zone != EffectZone::Pure)
        .count(),
    }
  }
}

/// CTAST 통계: CTAST의 통계 정보
#[derive(Debug, Clone)]
pub struct CTASTStats {
  /// 노드 개수
  pub num_nodes: usize,
  /// 엣지 개수
  pub num_edges: usize,
  /// 순수 노드 개수 (Pure zone)
  pub pure_nodes: usize,
  /// 효과 노드 개수 (Pure이 아닌 zone)
  pub effect_nodes: usize,
}

/// Infer CT operation from morphism name
fn infer_op_from_name(name: &str) -> CTMorphismOp {
  let lower = name.to_lowercase();
  match lower.as_str() {
    "id" | "identity" => CTMorphismOp::Id,
    "fst" | "first" | "proj1" => CTMorphismOp::Fst,
    "snd" | "second" | "proj2" => CTMorphismOp::Snd,
    "pair" => CTMorphismOp::Pair,
    "inl" | "left" => CTMorphismOp::Inl,
    "inr" | "right" => CTMorphismOp::Inr,
    "case" | "match" => CTMorphismOp::Case,
    "fmap" | "map" | "listmap" => CTMorphismOp::Fmap,
    "return" | "pure" | "unit" => CTMorphismOp::Return,
    "bind" | "flatmap" | "chain" => CTMorphismOp::Bind,
    "join" | "flatten" => CTMorphismOp::Join,
    "time" | "systemtime" => CTMorphismOp::Time,
    "lift" | "signalmap" => CTMorphismOp::Lift,
    "hold" => CTMorphismOp::Hold,
    "add" | "plus" => CTMorphismOp::Add,
    "sub" | "minus" | "subtract" => CTMorphismOp::Sub,
    "mul" | "multiply" | "times" => CTMorphismOp::Mul,
    "div" | "divide" => CTMorphismOp::Div,
    "neg" | "negate" => CTMorphismOp::Neg,
    "abs" | "absolute" => CTMorphismOp::Abs,
    "sqrt" => CTMorphismOp::Sqrt,
    "sin" => CTMorphismOp::Sin,
    "cos" => CTMorphismOp::Cos,
    "lt" | "less" => CTMorphismOp::Lt,
    "le" | "lessequal" => CTMorphismOp::Le,
    "gt" | "greater" => CTMorphismOp::Gt,
    "ge" | "greaterequal" => CTMorphismOp::Ge,
    "eq" | "equal" => CTMorphismOp::Eq,
    "ne" | "notequal" => CTMorphismOp::Ne,
    "and" => CTMorphismOp::And,
    "or" => CTMorphismOp::Or,
    "not" => CTMorphismOp::Not,
    "if" | "cond" | "conditional" => CTMorphismOp::If,
    _ if lower.contains("extern") || lower.contains("interop") => CTMorphismOp::Extern,
    _ => CTMorphismOp::Id, // default to identity
  }
}

// ============================================================
// CT Law Verification
// ============================================================

/// CTAST에서 Functor 법칙 검증
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn verify_functor_laws(ctast: &CTAST) -> Vec<String> {
  let mut violations = Vec::new();

  for node in &ctast.nodes {
    if node.op.is_fmap() {
      // Check for fmap id = id pattern
      if node.inputs.len() == 2 {
        // fmap f x where f might be id
        if let Some(f_node) = ctast.get_node(&node.inputs[0]) {
          if f_node.op == CTMorphismOp::Id {
            violations.push(format!(
              "Functor identity: fmap id should be id at node '{}'",
              node.name
            ));
          }
        }
      }
    }
  }

  // Check for fmap f . fmap g pattern (should be fmap (f . g))
  for (from, to) in &ctast.edges {
    if let (Some(f_node), Some(g_node)) = (ctast.get_node(from), ctast.get_node(to)) {
      if f_node.op.is_fmap() && g_node.op.is_fmap() {
        violations.push(format!(
          "Functor composition: fmap f . fmap g should be fmap (f . g) at '{}' -> '{}'",
          from, to
        ));
      }
    }
  }

  violations
}

/// CTAST에서 Monad 법칙 검증
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn verify_monad_laws(ctast: &CTAST) -> Vec<String> {
  let mut violations = Vec::new();

  for node in &ctast.nodes {
    if node.op == CTMorphismOp::Bind {
      // Check for return x >>= f pattern (left identity)
      if !node.inputs.is_empty() {
        if let Some(m_node) = ctast.get_node(&node.inputs[0]) {
          if m_node.op == CTMorphismOp::Return {
            violations.push(format!(
              "Monad left identity: return x >>= f should be f x at node '{}'",
              node.name
            ));
          }
        }
      }

      // Check for m >>= return pattern (right identity)
      if node.inputs.len() >= 2 {
        if let Some(k_node) = ctast.get_node(&node.inputs[1]) {
          if k_node.op == CTMorphismOp::Return {
            violations.push(format!(
              "Monad right identity: m >>= return should be m at node '{}'",
              node.name
            ));
          }
        }
      }
    }
  }

  violations
}

/// CTAST에서 Natural transformation 법칙 검증
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn verify_nat_trans_laws(ctast: &CTAST) -> Vec<String> {
  let mut violations = Vec::new();

  // Look for lift . fmap pattern (should satisfy naturality)
  for node in &ctast.nodes {
    if node.op == CTMorphismOp::Lift {
      for input in &node.inputs {
        if let Some(input_node) = ctast.get_node(input) {
          if input_node.op == CTMorphismOp::Fmap {
            // Check naturality: lift . fmap f = fmap f . lift
            // This is a hint for potential optimization
            violations.push(format!(
              "Natural transformation hint: lift . fmap can be reordered at '{}'",
              node.name
            ));
          }
        }
      }
    }
  }

  violations
}

// ============================================================
// CT Law Preservation
// ============================================================

/// CT 법칙 보존 추적: 최적화 전후 CT 법칙 보존 여부 추적
#[derive(Debug, Clone, Default)]
pub struct CTLawPreservation {
  /// 최적화 전 Functor 법칙 준수 여부
  pub functor_before: bool,
  /// 최적화 후 Functor 법칙 준수 여부
  pub functor_after: bool,
  /// 최적화 전 Monad 법칙 준수 여부
  pub monad_before: bool,
  /// 최적화 후 Monad 법칙 준수 여부
  pub monad_after: bool,
  /// 최적화 전 Natural transformation 준수 여부
  pub nat_trans_before: bool,
  /// 최적화 후 Natural transformation 준수 여부
  pub nat_trans_after: bool,
  /// 최적화 전 법칙 위반 목록
  pub violations_before: Vec<String>,
  /// 최적화 후 법칙 위반 목록
  pub violations_after: Vec<String>,
  /// 법칙이 보존되었는지 여부 (모든 법칙이 전후 모두 준수)
  pub preserved: bool,
}

impl CTLawPreservation {
  /// 새 법칙 보존 추적 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      functor_before: true,
      functor_after: true,
      monad_before: true,
      monad_after: true,
      nat_trans_before: true,
      nat_trans_after: true,
      violations_before: Vec::new(),
      violations_after: Vec::new(),
      preserved: true,
    }
  }

  /// 법칙이 보존되었는지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_preserved(&self) -> bool {
    let functor_preserved = !self.functor_before || self.functor_after;
    let monad_preserved = !self.monad_before || self.monad_after;
    let nat_trans_preserved = !self.nat_trans_before || self.nat_trans_after;
    functor_preserved && monad_preserved && nat_trans_preserved
  }

  /// 법칙 보존 요약 문자열 반환
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn summary(&self) -> String {
    if self.is_preserved() && self.violations_after.is_empty() {
      "All CT laws preserved".to_string()
    } else {
      let mut parts = Vec::new();
      if !self.functor_after && self.functor_before {
        parts.push("Functor: VIOLATED");
      }
      if !self.monad_after && self.monad_before {
        parts.push("Monad: VIOLATED");
      }
      if !self.nat_trans_after && self.nat_trans_before {
        parts.push("NatTrans: VIOLATED");
      }
      if parts.is_empty() {
        format!("{} optimization hints", self.violations_after.len())
      } else {
        parts.join(", ")
      }
    }
  }
}

/// 변환 전후 CT 법칙 보존 검증
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 값 계산 없음
pub fn verify_preservation(before: &CTAST, after: &CTAST) -> CTLawPreservation {
  let functor_violations_before = verify_functor_laws(before);
  let monad_violations_before = verify_monad_laws(before);
  let nat_trans_violations_before = verify_nat_trans_laws(before);

  let functor_violations_after = verify_functor_laws(after);
  let monad_violations_after = verify_monad_laws(after);
  let nat_trans_violations_after = verify_nat_trans_laws(after);

  let functor_before = functor_violations_before.is_empty();
  let functor_after = functor_violations_after.is_empty();
  let monad_before = monad_violations_before.is_empty();
  let monad_after = monad_violations_after.is_empty();
  let nat_trans_before = nat_trans_violations_before.is_empty();
  let nat_trans_after = nat_trans_violations_after.is_empty();

  let mut all_violations_before = Vec::new();
  all_violations_before.extend(functor_violations_before);
  all_violations_before.extend(monad_violations_before);
  all_violations_before.extend(nat_trans_violations_before);

  let mut all_violations_after = Vec::new();
  all_violations_after.extend(functor_violations_after);
  all_violations_after.extend(monad_violations_after);
  all_violations_after.extend(nat_trans_violations_after);

  let mut result = CTLawPreservation {
    functor_before,
    functor_after,
    monad_before,
    monad_after,
    nat_trans_before,
    nat_trans_after,
    violations_before: all_violations_before,
    violations_after: all_violations_after,
    preserved: true,
  };

  result.preserved = result.is_preserved();
  result
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
  use super::*;
  use crate::contracts::effect::Effect;
  use crate::core::{FxCoreModule, FxEdge, FxMorphism, FxNode};

  fn make_test_module() -> FxCoreModule {
    FxCoreModule {
      meta: Default::default(),
      name: "test".to_string(),
      types: vec!["Real".to_string()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![
        FxMorphism::simple("sin".into(), "Real".into(), "Real".into(), Effect::Pure),
        FxMorphism::simple("cos".into(), "Real".into(), "Real".into(), Effect::Pure),
      ],
      nodes: vec![
        FxNode {
          name: "n1".into(),
          uses: "sin".into(),
          meta: None,
          ..Default::default()
        },
        FxNode {
          name: "n2".into(),
          uses: "cos".into(),
          meta: None,
          ..Default::default()
        },
      ],
      edges: vec![FxEdge::simple("n1".into(), "n2".into())],
      scopes: vec![],
    }
  }

  #[test]
  fn test_cttype_parse() {
    assert_eq!(CTType::parse("Int"), CTType::Int);
    assert_eq!(CTType::parse("Real"), CTType::Real);
    assert_eq!(CTType::parse("Bool"), CTType::Bool);
    assert_eq!(CTType::parse("()"), CTType::Unit);
    assert!(matches!(CTType::parse("Position"), CTType::Named(_)));
  }

  #[test]
  fn test_cttype_symbol() {
    assert_eq!(CTType::Int.symbol(), "ℤ");
    assert_eq!(CTType::Real.symbol(), "ℝ");
    assert_eq!(CTType::Bool.symbol(), "𝔹");
    assert_eq!(CTType::Unit.symbol(), "1");
  }

  #[test]
  fn test_ctast_from_fxcore() {
    let module = make_test_module();
    let ctast = CTAST::from_fxcore(&module);

    assert_eq!(ctast.nodes.len(), 2);
    assert_eq!(ctast.edges.len(), 1);
    assert_eq!(ctast.zone, EffectZone::Pure);
  }

  #[test]
  fn test_ctast_topo_order() {
    let module = make_test_module();
    let ctast = CTAST::from_fxcore(&module);

    let order = ctast.topo_order();
    assert_eq!(order.len(), 2);
    // n1 should come before n2 (n1 -> n2)
    let names: Vec<&str> = order.iter().map(|n| n.name.as_str()).collect();
    assert!(names.iter().position(|&n| n == "n1") < names.iter().position(|&n| n == "n2"));
  }

  #[test]
  fn test_ct_morphism_op_zone() {
    assert_eq!(CTMorphismOp::Add.zone(), EffectZone::Pure);
    assert_eq!(CTMorphismOp::Sin.zone(), EffectZone::Pure);
    assert_eq!(CTMorphismOp::Time.zone(), EffectZone::Frp);
    assert_eq!(CTMorphismOp::Extern.zone(), EffectZone::Interop);
  }

  #[test]
  fn test_infer_op_from_name() {
    assert_eq!(infer_op_from_name("sin"), CTMorphismOp::Sin);
    assert_eq!(infer_op_from_name("fmap"), CTMorphismOp::Fmap);
    assert_eq!(infer_op_from_name("return"), CTMorphismOp::Return);
    assert_eq!(infer_op_from_name("bind"), CTMorphismOp::Bind);
  }

  #[test]
  fn test_ctast_stats() {
    let module = make_test_module();
    let ctast = CTAST::from_fxcore(&module);
    let stats = ctast.stats();

    assert_eq!(stats.num_nodes, 2);
    assert_eq!(stats.num_edges, 1);
    assert_eq!(stats.pure_nodes, 2);
    assert_eq!(stats.effect_nodes, 0);
  }

  #[test]
  fn test_ct_law_preservation() {
    let module = make_test_module();
    let ctast = CTAST::from_fxcore(&module);

    // Same CTAST should preserve all laws
    let preservation = verify_preservation(&ctast, &ctast);
    assert!(preservation.is_preserved());
    assert_eq!(preservation.summary(), "All CT laws preserved");
  }

  #[test]
  fn test_ct_law_preservation_summary() {
    let mut pres = CTLawPreservation::new();
    assert_eq!(pres.summary(), "All CT laws preserved");

    pres.functor_before = true;
    pres.functor_after = false;
    assert!(pres.summary().contains("VIOLATED"));
  }
}
