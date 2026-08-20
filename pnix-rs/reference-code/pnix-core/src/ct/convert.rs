//! CTAST Conversion utilities
//!
//! pnix-old ctast_convert.rs를 pnix-new 그래프 패러다임에 적응
//!
//! Conversions:
//! - FxCoreModule → CTAST: 그래프를 범주론 AST로 변환 (ctast.rs에 구현)
//! - CTAST → FxCoreModule: 범주론 AST를 그래프로 역변환
//! - CTDiagram ↔ CTAST: 다이어그램과 AST 상호 변환
//!
//! 헌법 P0-1 준수: 구조 변환만, 값 계산 없음

use super::ctast::{CTMorphismOp, CTNode, CTType, CTAST};
use crate::contracts::effect::Effect;
use crate::core::{FxCoreModule, FxEdge, FxMorphism, FxNode, NodeKind};
use crate::effects::EffectZone;
use crate::meta::{CTDiagram, CTMorphism, CTObject, CTType as DiagramCTType};
use std::collections::HashMap;

// ============================================================
// CTAST → FxCoreModule
// ============================================================

/// CTAST를 FxCoreModule로 역변환
///
/// 범주론 AST의 노드 구조를 그래프 노드/엣지로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn ctast_to_fxcore_module(ctast: &CTAST, name: &str) -> FxCoreModule {
  let mut nodes = Vec::new();
  let mut edges = Vec::new();
  let mut morphisms = Vec::new();
  let mut types = Vec::new();

  // CTAST 노드들을 FxCore 노드로 변환
  for ct_node in &ctast.nodes {
    // Morphism 생성
    let morph_name = format!("{:?}", ct_node.op);
    let input_type = cttype_to_string(&ct_node.src);
    let output_type = cttype_to_string(&ct_node.tgt);

    // EffectZone을 Effect로 변환
    let effect = zone_to_effect(ct_node.zone);

    if !morphisms.iter().any(|m: &FxMorphism| m.name == morph_name) {
      morphisms.push(FxMorphism::simple(
        morph_name.clone(),
        input_type.clone(),
        output_type.clone(),
        effect,
      ));
    }

    // 타입 등록
    if !types.contains(&input_type) {
      types.push(input_type.clone());
    }
    if !types.contains(&output_type) {
      types.push(output_type);
    }

    // FxNode 생성
    nodes.push(FxNode {
      name: ct_node.name.clone(),
      uses: morph_name,
      kind: NodeKind::Normal,
      optional: false,
      scope: "global".into(),
      meta: None,
      ..Default::default()
    });
  }

  // CTAST edges를 FxEdge로 변환
  for (from, to) in &ctast.edges {
    edges.push(FxEdge::simple(from.clone(), to.clone()));
  }

  FxCoreModule {
    meta: Default::default(),
    name: name.into(),
    types,
    adt_types: vec![],
    adttypes: vec![],
    inputs: vec![],
    morphisms,
    nodes,
    edges,
    scopes: vec![],
  }
}

// ============================================================
// CTDiagram → CTAST
// ============================================================

/// CTDiagram을 CTAST로 변환
///
/// 다이어그램의 morphism들을 범주론 AST 노드로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn ct_diagram_to_ctast(diagram: &CTDiagram) -> CTAST {
  let mut nodes = Vec::new();
  let mut edges = Vec::new();
  let mut node_zones = Vec::new();

  // 오브젝트 타입 맵
  let obj_types: HashMap<usize, CTType> = diagram
    .objects
    .iter()
    .map(|o| (o.id, diagram_type_to_cttype(&o.ct_type)))
    .collect();

  // morphism → CT 노드
  for morph in &diagram.morphisms {
    let src = obj_types
      .get(&morph.source)
      .cloned()
      .unwrap_or(CTType::Unit);
    let tgt = obj_types
      .get(&morph.target)
      .cloned()
      .unwrap_or(CTType::Unit);

    // morphism name을 CTMorphismOp로 변환
    let op = name_to_ctop(&morph.name);

    // effect 문자열을 EffectZone으로 변환
    let zone = effect_string_to_zone(&morph.effect);

    // 입력 노드 찾기 (이전 morphism들 중 target이 이 source인 것)
    let inputs: Vec<String> = diagram
      .morphisms
      .iter()
      .filter(|m| m.target == morph.source)
      .map(|m| format!("m{}", m.id))
      .collect();

    let node_name = format!("m{}", morph.id);

    nodes.push(CTNode {
      name: node_name.clone(),
      op,
      src,
      tgt,
      zone,
      inputs,
    });
    node_zones.push(zone);
  }

  // 다이어그램의 morphism 연결을 edge로 변환
  for morph in &diagram.morphisms {
    // source에서 오는 morphism들과 연결
    for prev_morph in &diagram.morphisms {
      if prev_morph.target == morph.source {
        edges.push((format!("m{}", prev_morph.id), format!("m{}", morph.id)));
      }
    }
  }

  let zone = EffectZone::join_all(node_zones);

  CTAST { nodes, edges, zone }
}

// ============================================================
// CTAST → CTDiagram
// ============================================================

/// CTAST를 CTDiagram으로 변환
///
/// 범주론 AST를 시각화 가능한 다이어그램으로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn ctast_to_ct_diagram(ctast: &CTAST) -> CTDiagram {
  let mut objects = Vec::new();
  let mut morphisms = Vec::new();
  let mut obj_counter = 0;

  // 오브젝트 맵: CTType → Object ID
  let mut type_to_obj: HashMap<String, usize> = HashMap::new();

  fn get_or_create_object(
    ty: &CTType,
    type_to_obj: &mut HashMap<String, usize>,
    objects: &mut Vec<CTObject>,
    counter: &mut usize,
  ) -> usize {
    let key = format!("{:?}", ty);
    if let Some(&id) = type_to_obj.get(&key) {
      return id;
    }

    let id = *counter;
    *counter += 1;

    objects.push(CTObject {
      id,
      name: key.clone(),
      ct_type: cttype_to_diagram_type(ty),
    });
    type_to_obj.insert(key, id);
    id
  }

  // CTAST 노드들을 다이어그램 morphism으로 변환
  // 노드 이름에서 ID 추출을 위한 맵
  let mut name_to_id: HashMap<String, usize> = HashMap::new();
  for (morph_id, ct_node) in ctast.nodes.iter().enumerate() {
    let source = get_or_create_object(
      &ct_node.src,
      &mut type_to_obj,
      &mut objects,
      &mut obj_counter,
    );
    let target = get_or_create_object(
      &ct_node.tgt,
      &mut type_to_obj,
      &mut objects,
      &mut obj_counter,
    );

    name_to_id.insert(ct_node.name.clone(), morph_id);

    // EffectZone을 effect 문자열로 변환
    let effect_str = zone_to_effect_string(ct_node.zone);

    morphisms.push(CTMorphism {
      id: morph_id,
      name: format!("{:?}", ct_node.op),
      source,
      target,
      effect: effect_str,
    });
  }

  CTDiagram { objects, morphisms }
}

// ============================================================
// 타입 변환 헬퍼
// ============================================================

/// CTType을 문자열로 변환
fn cttype_to_string(ty: &CTType) -> String {
  match ty {
    CTType::Unit => "Unit".into(),
    CTType::Bool => "Bool".into(),
    CTType::Int => "Int".into(),
    CTType::Real => "Real".into(),
    CTType::String => "String".into(),
    CTType::List(inner) => format!("List<{}>", cttype_to_string(inner)),
    CTType::Arrow(a, b) => format!("{} -> {}", cttype_to_string(a), cttype_to_string(b)),
    CTType::Product(a, b) => format!("({}, {})", cttype_to_string(a), cttype_to_string(b)),
    CTType::Sum(a, b) => format!("{} | {}", cttype_to_string(a), cttype_to_string(b)),
    CTType::Signal(inner) => format!("Signal<{}>", cttype_to_string(inner)),
    CTType::Effect(zone, inner) => format!("Effect<{:?}, {}>", zone, cttype_to_string(inner)),
    CTType::Named(name) => name.clone(),
    CTType::Var(name) => format!("'{}", name),
  }
}

/// DiagramCTType을 CTType으로 변환
fn diagram_type_to_cttype(ty: &DiagramCTType) -> CTType {
  match ty {
    DiagramCTType::Real => CTType::Real,
    DiagramCTType::Int => CTType::Int,
    DiagramCTType::Bool => CTType::Bool,
    DiagramCTType::String => CTType::String,
    DiagramCTType::Unit => CTType::Unit,
    DiagramCTType::Product(a, b) => CTType::Product(
      Box::new(diagram_type_to_cttype(a)),
      Box::new(diagram_type_to_cttype(b)),
    ),
    DiagramCTType::Named(name) => CTType::Named(name.clone()),
    DiagramCTType::Unknown => CTType::Unit,
  }
}

/// CTType을 DiagramCTType으로 변환
fn cttype_to_diagram_type(ty: &CTType) -> DiagramCTType {
  match ty {
    CTType::Real => DiagramCTType::Real,
    CTType::Int => DiagramCTType::Int,
    CTType::Bool => DiagramCTType::Bool,
    CTType::String => DiagramCTType::String,
    CTType::Unit => DiagramCTType::Unit,
    CTType::Signal(_) => DiagramCTType::Unknown, // No Signal in DiagramCTType
    CTType::Product(a, b) => DiagramCTType::Product(
      Box::new(cttype_to_diagram_type(a)),
      Box::new(cttype_to_diagram_type(b)),
    ),
    CTType::Named(name) => DiagramCTType::Named(name.clone()),
    _ => DiagramCTType::Unknown,
  }
}

/// 문자열 이름을 CTMorphismOp으로 변환
fn name_to_ctop(name: &str) -> CTMorphismOp {
  match name.to_lowercase().as_str() {
    "id" | "identity" => CTMorphismOp::Id,
    "compose" => CTMorphismOp::Compose,
    "pair" | "product" => CTMorphismOp::Pair,
    "fst" | "first" => CTMorphismOp::Fst,
    "snd" | "second" => CTMorphismOp::Snd,
    "inl" | "left" => CTMorphismOp::Inl,
    "inr" | "right" => CTMorphismOp::Inr,
    "case" | "match" => CTMorphismOp::Case,
    "fmap" | "map" => CTMorphismOp::Fmap,
    "return" | "pure" => CTMorphismOp::Return,
    "bind" | "flatmap" => CTMorphismOp::Bind,
    "join" | "flatten" => CTMorphismOp::Join,
    "lift" => CTMorphismOp::Lift,
    "time" => CTMorphismOp::Time,
    "hold" => CTMorphismOp::Hold,
    "add" | "plus" => CTMorphismOp::Add,
    "sub" | "minus" => CTMorphismOp::Sub,
    "mul" | "multiply" => CTMorphismOp::Mul,
    "div" | "divide" => CTMorphismOp::Div,
    "neg" | "negate" => CTMorphismOp::Neg,
    "abs" => CTMorphismOp::Abs,
    "sqrt" => CTMorphismOp::Sqrt,
    "sin" => CTMorphismOp::Sin,
    "cos" => CTMorphismOp::Cos,
    "lt" | "less" => CTMorphismOp::Lt,
    "le" => CTMorphismOp::Le,
    "gt" | "greater" => CTMorphismOp::Gt,
    "ge" => CTMorphismOp::Ge,
    "eq" | "equal" => CTMorphismOp::Eq,
    "ne" | "notequal" => CTMorphismOp::Ne,
    "and" => CTMorphismOp::And,
    "or" => CTMorphismOp::Or,
    "not" => CTMorphismOp::Not,
    "if" | "cond" => CTMorphismOp::If,
    "lam" | "lambda" => CTMorphismOp::Lam,
    "app" | "apply" => CTMorphismOp::App,
    "curry" => CTMorphismOp::Curry,
    "uncurry" => CTMorphismOp::Uncurry,
    _ => CTMorphismOp::Extern, // unknown ops map to Extern
  }
}

// ============================================================
// Effect 변환 헬퍼
// ============================================================

/// effect 문자열을 EffectZone으로 변환
fn effect_string_to_zone(effect: &str) -> EffectZone {
  match effect.to_lowercase().as_str() {
    "pure" => EffectZone::Pure,
    "symbolic" => EffectZone::Symbolic,
    "frp" => EffectZone::Frp,
    "animation" => EffectZone::Animation,
    "stm" => EffectZone::Stm,
    "interop" => EffectZone::Interop,
    "world" => EffectZone::World,
    _ => EffectZone::Pure,
  }
}

/// EffectZone을 effect 문자열로 변환
fn zone_to_effect_string(zone: EffectZone) -> String {
  match zone {
    EffectZone::Pure => "pure".to_string(),
    EffectZone::Symbolic => "symbolic".to_string(),
    EffectZone::Frp => "frp".to_string(),
    EffectZone::Animation => "animation".to_string(),
    EffectZone::Stm => "stm".to_string(),
    EffectZone::Interop => "interop".to_string(),
    EffectZone::World => "world".to_string(),
  }
}

/// EffectZone을 Effect로 변환
/// (Effect has only Pure and World, so non-pure zones map to World)
fn zone_to_effect(zone: EffectZone) -> Effect {
  match zone {
    EffectZone::Pure => Effect::Pure,
    _ => Effect::World, // all non-pure effects map to World
  }
}

// ============================================================
// 변환 통계
// ============================================================

/// 변환 통계
#[derive(Debug, Clone, Default)]
pub struct ConversionStats {
  pub nodes_converted: usize,
  pub edges_converted: usize,
  pub types_discovered: usize,
  pub morphisms_generated: usize,
}

/// FxCoreModule → CTAST 통계와 함께 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn fxcore_to_ctast_with_stats(fx: &FxCoreModule) -> (CTAST, ConversionStats) {
  let ctast = CTAST::from_fxcore(fx);
  let stats = ConversionStats {
    nodes_converted: ctast.nodes.len(),
    edges_converted: ctast.edges.len(),
    types_discovered: fx.types.len(),
    morphisms_generated: fx.morphisms.len(),
  };
  (ctast, stats)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::FxCoreMeta;

  fn make_simple_module() -> FxCoreModule {
    FxCoreModule {
      meta: FxCoreMeta::default(),
      name: "test".into(),
      types: vec!["Int".into(), "Real".into()],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![FxMorphism::simple(
        "add".into(),
        "Int".into(),
        "Int".into(),
        Effect::Pure,
      )],
      nodes: vec![
        FxNode {
          name: "a".into(),
          uses: "add".into(),
          meta: None,
          ..Default::default()
        },
        FxNode {
          name: "b".into(),
          uses: "add".into(),
          meta: None,
          ..Default::default()
        },
      ],
      edges: vec![FxEdge::simple("a".into(), "b".into())],
      scopes: vec![],
    }
  }

  #[test]
  fn test_fxcore_to_ctast_roundtrip() {
    let original = make_simple_module();
    let ctast = CTAST::from_fxcore(&original);
    let back = ctast_to_fxcore_module(&ctast, "roundtrip");

    // 노드 수 보존
    assert_eq!(back.nodes.len(), original.nodes.len());
  }

  #[test]
  fn test_ctast_to_ct_diagram() {
    let fx = make_simple_module();
    let ctast = CTAST::from_fxcore(&fx);
    let diagram = ctast_to_ct_diagram(&ctast);

    // 오브젝트와 morphism이 생성됨
    assert!(!diagram.objects.is_empty());
    assert!(!diagram.morphisms.is_empty());
  }

  #[test]
  fn test_ct_diagram_to_ctast() {
    let diagram = CTDiagram {
      objects: vec![
        CTObject {
          id: 0,
          name: "A".into(),
          ct_type: DiagramCTType::Real,
        },
        CTObject {
          id: 1,
          name: "B".into(),
          ct_type: DiagramCTType::Real,
        },
      ],
      morphisms: vec![CTMorphism {
        id: 0,
        name: "sin".into(),
        source: 0,
        target: 1,
        effect: "pure".into(),
      }],
    };

    let ctast = ct_diagram_to_ctast(&diagram);
    assert_eq!(ctast.nodes.len(), 1);
    assert!(matches!(ctast.nodes[0].op, CTMorphismOp::Sin));
  }

  #[test]
  fn test_name_to_ctop() {
    assert!(matches!(name_to_ctop("id"), CTMorphismOp::Id));
    assert!(matches!(name_to_ctop("Identity"), CTMorphismOp::Id));
    assert!(matches!(name_to_ctop("fmap"), CTMorphismOp::Fmap));
    assert!(matches!(name_to_ctop("bind"), CTMorphismOp::Bind));
    assert!(matches!(name_to_ctop("unknown"), CTMorphismOp::Extern));
  }

  #[test]
  fn test_cttype_to_string() {
    assert_eq!(cttype_to_string(&CTType::Int), "Int");
    assert_eq!(
      cttype_to_string(&CTType::List(Box::new(CTType::Real))),
      "List<Real>"
    );
    assert_eq!(
      cttype_to_string(&CTType::Arrow(
        Box::new(CTType::Int),
        Box::new(CTType::Bool)
      )),
      "Int -> Bool"
    );
  }

  #[test]
  fn test_conversion_stats() {
    let fx = make_simple_module();
    let (ctast, stats) = fxcore_to_ctast_with_stats(&fx);

    assert_eq!(stats.nodes_converted, ctast.nodes.len());
    assert!(stats.types_discovered > 0);
  }

  #[test]
  fn test_diagram_type_conversion() {
    // DiagramCTType → CTType
    assert_eq!(diagram_type_to_cttype(&DiagramCTType::Real), CTType::Real);
    assert_eq!(diagram_type_to_cttype(&DiagramCTType::Int), CTType::Int);
    assert_eq!(diagram_type_to_cttype(&DiagramCTType::Bool), CTType::Bool);

    // CTType → DiagramCTType
    assert_eq!(cttype_to_diagram_type(&CTType::Real), DiagramCTType::Real);
    assert_eq!(cttype_to_diagram_type(&CTType::Int), DiagramCTType::Int);
  }

  #[test]
  fn test_full_roundtrip_diagram() {
    // FxCoreModule → CTAST → CTDiagram → CTAST
    let fx = make_simple_module();
    let ctast1 = CTAST::from_fxcore(&fx);
    let diagram = ctast_to_ct_diagram(&ctast1);
    let ctast2 = ct_diagram_to_ctast(&diagram);

    // 노드 수가 보존됨
    assert_eq!(ctast1.nodes.len(), ctast2.nodes.len());
  }

  #[test]
  fn test_effect_conversion() {
    // String → EffectZone
    assert_eq!(effect_string_to_zone("pure"), EffectZone::Pure);
    assert_eq!(effect_string_to_zone("world"), EffectZone::World);
    assert_eq!(effect_string_to_zone("frp"), EffectZone::Frp);

    // EffectZone → String
    assert_eq!(zone_to_effect_string(EffectZone::Pure), "pure");
    assert_eq!(zone_to_effect_string(EffectZone::World), "world");

    // EffectZone → Effect (only Pure and World exist)
    assert_eq!(zone_to_effect(EffectZone::Pure), Effect::Pure);
    assert_eq!(zone_to_effect(EffectZone::Frp), Effect::World); // non-pure maps to World
    assert_eq!(zone_to_effect(EffectZone::World), Effect::World);
  }
}
