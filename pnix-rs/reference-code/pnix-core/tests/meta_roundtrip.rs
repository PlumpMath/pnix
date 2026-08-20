//! STEP 4-3: MetaFx → DSL → FxCore round-trip 구조 동일성 테스트
//!
//! 목표: FxCore → MetaFx → DSL → FxCore' 변환 후
//!       FxCore ≡ FxCore' (의미적·구조적 동형) 증명
//!
//! 제약:
//! - contract는 lowering에서 재계산되므로 비교 대상에서 제외
//! - effect는 이름 prefix에서 추론되므로 비교 대상에서 제외
//! - meta 필드는 lowering에서 기본값으로 초기화됨

use pnix_core::ast::parse_module;
use pnix_core::contracts::effect::Effect;
use pnix_core::core::{
  CostHint, EdgeCond, ExecutionContract, FxCoreMeta, FxCoreModule, FxEdge, FxInput, FxMorphism,
  FxNode, FxPort, FxScope, NodeKind, ScopePolicy,
};
use pnix_core::diagnostics::Diagnostics;
use pnix_core::meta::{describe_fxcore, generate_pnix_dsl};
use pnix_core::passes::lowering::{lower_to_fxcore, lower_to_surface};

/// FxCore를 비교 가능한 정규화된 형태로 변환
///
/// 비교 대상:
/// - types, inputs, morphisms (effect 제외)
/// - nodes (contract 제외)
/// - edges
/// - scopes
///
/// 비교 제외:
/// - meta (version, stage, replay_hash)
/// - contract (lowering에서 재계산)
/// - effect (이름 prefix로 추론)
#[derive(Debug, PartialEq)]
struct NormalizedFxCore {
  name: String,
  types: Vec<String>,
  inputs: Vec<(String, String)>, // (name, ty)
  morphisms: Vec<NormalizedMorphism>,
  nodes: Vec<NormalizedNode>,
  edges: Vec<NormalizedEdge>,
  scopes: Vec<NormalizedScope>,
}

#[derive(Debug, PartialEq)]
struct NormalizedMorphism {
  name: String,
  input: String,
  output: String,
  inputs: Vec<(String, String)>, // (name, ty)
  outputs: Vec<(String, String)>, // (name, ty)
                                 // effect 제외 - 이름 prefix로 추론됨
}

#[derive(Debug, PartialEq)]
struct NormalizedNode {
  name: String,
  uses: String,
  kind: String,
  optional: bool,
  scope: String,
  cost: String,
  priority: i32,
  // contract 제외 - lowering에서 재계산
}

#[derive(Debug, PartialEq)]
struct NormalizedEdge {
  from: String,
  to: String,
  from_port: Option<String>,
  to_port: Option<String>,
  cond: Option<String>,
}

#[derive(Debug, PartialEq)]
struct NormalizedScope {
  name: String,
  policy: String,
  // nodes는 AST에서 직접 추출할 수 없음 (노드의 scope 필드로 관리)
}

fn normalize_fxcore(fx: &FxCoreModule) -> NormalizedFxCore {
  let mut types = fx.types.clone();
  types.sort();

  let mut inputs: Vec<_> = fx
    .inputs
    .iter()
    .map(|i| (i.name.clone(), i.ty.clone()))
    .collect();
  inputs.sort_by(|a, b| a.0.cmp(&b.0));

  let mut morphisms: Vec<_> = fx
    .morphisms
    .iter()
    .map(|m| NormalizedMorphism {
      name: m.name.clone(),
      input: m.input.clone(),
      output: m.output.clone(),
      inputs: m
        .inputs
        .iter()
        .map(|p| (p.name.clone(), p.ty.clone()))
        .collect(),
      outputs: m
        .outputs
        .iter()
        .map(|p| (p.name.clone(), p.ty.clone()))
        .collect(),
    })
    .collect();
  morphisms.sort_by(|a, b| a.name.cmp(&b.name));

  let mut nodes: Vec<_> = fx
    .nodes
    .iter()
    .map(|n| NormalizedNode {
      name: n.name.clone(),
      uses: n.uses.clone(),
      kind: match n.kind {
        NodeKind::Normal => "normal".into(),
        NodeKind::Gate => "gate".into(),
      },
      optional: n.optional,
      scope: n.scope.clone(),
      cost: match n.cost {
        CostHint::Tiny => "tiny".into(),
        CostHint::Light => "light".into(),
        CostHint::Medium => "medium".into(),
        CostHint::Heavy => "heavy".into(),
        CostHint::XHeavy => "xheavy".into(),
      },
      priority: n.priority,
    })
    .collect();
  nodes.sort_by(|a, b| a.name.cmp(&b.name));

  let mut edges: Vec<_> = fx
    .edges
    .iter()
    .map(|e| NormalizedEdge {
      from: e.from.clone(),
      to: e.to.clone(),
      from_port: e.from_port.clone(),
      to_port: e.to_port.clone(),
      cond: e.cond.as_ref().map(|c| match c {
        EdgeCond::When(g) => format!("when:{}", g),
        EdgeCond::Unless(g) => format!("unless:{}", g),
        EdgeCond::OnFail(n) => format!("onfail:{}", n),
        EdgeCond::WhenUnless { when, unless } => format!("when_unless:{}:{}", when, unless),
        EdgeCond::AllWhen(gates) => format!("all_when:{}", gates.join(",")),
        EdgeCond::AllUnless(gates) => format!("all_unless:{}", gates.join(",")),
        EdgeCond::Unknown => "unknown".to_string(),
      }),
    })
    .collect();
  edges.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));

  let mut scopes: Vec<_> = fx
    .scopes
    .iter()
    .map(|s| NormalizedScope {
      name: s.name.clone(),
      policy: match s.policy {
        ScopePolicy::FailFast => "failfast".into(),
        ScopePolicy::Isolate => "isolate".into(),
        ScopePolicy::BestEffort => "besteffort".into(),
      },
    })
    .collect();
  scopes.sort_by(|a, b| a.name.cmp(&b.name));

  NormalizedFxCore {
    name: fx.name.clone(),
    types,
    inputs,
    morphisms,
    nodes,
    edges,
    scopes,
  }
}

/// Stage-1/2/3/4 복합 테스트 그래프 생성
fn make_comprehensive_test_graph() -> FxCoreModule {
  FxCoreModule {
    meta: FxCoreMeta {
      version: "fxcore@0.1".into(),
      stage: 4,
      replay_hash: Some("test_roundtrip_hash".into()),
    },
    name: "roundtrip-test".into(),
    types: vec!["Data".into(), "Result".into(), "Bool".into()],
    adt_types: vec![],
    adttypes: vec![],
    inputs: vec![FxInput {
      name: "rawData".into(),
      ty: "Data".into(),
    }],
    morphisms: vec![
      FxMorphism {
        name: "py.normalize".into(),
        input: "Data".into(),
        output: "Data".into(),
        inputs: vec![FxPort {
          name: "in".into(),
          ty: "Data".into(),
        }],
        outputs: vec![FxPort {
          name: "out".into(),
          ty: "Data".into(),
        }],
        effect: Effect::Pure,
      },
      FxMorphism {
        name: "gate.validate".into(),
        input: "Data".into(),
        output: "Bool".into(),
        inputs: vec![FxPort {
          name: "in".into(),
          ty: "Data".into(),
        }],
        outputs: vec![FxPort {
          name: "out".into(),
          ty: "Bool".into(),
        }],
        effect: Effect::Pure,
      },
      FxMorphism {
        name: "deno.render".into(),
        input: "Data".into(),
        output: "Result".into(),
        inputs: vec![FxPort {
          name: "in".into(),
          ty: "Data".into(),
        }],
        outputs: vec![FxPort {
          name: "out".into(),
          ty: "Result".into(),
        }],
        effect: Effect::Pure,
      },
    ],
    nodes: vec![
      FxNode {
        name: "normalize".into(),
        uses: "py.normalize".into(),
        kind: NodeKind::Normal,
        optional: false,
        scope: "global".into(),
        cost: CostHint::Medium,
        priority: 0,
        contract: ExecutionContract::default(),

        meta: None,
      },
      FxNode {
        name: "validate".into(),
        uses: "gate.validate".into(),
        kind: NodeKind::Gate,
        optional: false,
        scope: "global".into(),
        cost: CostHint::Tiny,
        priority: 10,
        contract: ExecutionContract::default(),

        meta: None,
      },
      FxNode {
        name: "render".into(),
        uses: "deno.render".into(),
        kind: NodeKind::Normal,
        optional: true,
        scope: "output".into(),
        cost: CostHint::Heavy,
        priority: 0,
        contract: ExecutionContract::default(),

        meta: None,
      },
    ],
    edges: vec![
      FxEdge::simple("normalize".into(), "validate".into()),
      FxEdge::simple("validate".into(), "render".into())
        .with_cond(EdgeCond::When("validate".into())),
    ],
    scopes: vec![FxScope {
      name: "output".into(),
      nodes: vec!["render".into()],
      policy: ScopePolicy::Isolate,
    }],
  }
}

#[test]
fn roundtrip_structural_equivalence() {
  // Step 1: 원본 FxCore 생성
  let original = make_comprehensive_test_graph();

  // Step 2: MetaFx로 변환
  let meta = describe_fxcore(&original);

  // Step 3: DSL 생성
  let dsl = generate_pnix_dsl(&meta);

  // DSL 출력 확인 (디버깅용)
  println!("=== Generated DSL ===\n{}", dsl);

  // Step 4: DSL 파싱
  let mut diags = Diagnostics::default();
  let ast = parse_module(&dsl, "roundtrip-test", &mut diags).expect("DSL parsing failed");

  // 파싱 오류 확인
  if !diags.items.is_empty() {
    for diag in &diags.items {
      eprintln!("Parse error: {:?}", diag);
    }
    panic!("DSL parsing had errors");
  }

  // Step 5: Surface IR로 변환
  let surface = lower_to_surface(&ast, &mut diags).expect("Surface lowering failed");

  // Step 6: FxCore로 변환
  let roundtrip = lower_to_fxcore(&surface, &mut diags).expect("FxCore lowering failed");

  // Step 7: 정규화하여 비교
  let norm_original = normalize_fxcore(&original);
  let norm_roundtrip = normalize_fxcore(&roundtrip);

  // 개별 필드 비교 (더 나은 오류 메시지)
  assert_eq!(norm_original.name, norm_roundtrip.name, "name mismatch");
  assert_eq!(norm_original.types, norm_roundtrip.types, "types mismatch");
  assert_eq!(
    norm_original.inputs, norm_roundtrip.inputs,
    "inputs mismatch"
  );
  assert_eq!(
    norm_original.morphisms, norm_roundtrip.morphisms,
    "morphisms mismatch"
  );
  assert_eq!(norm_original.nodes, norm_roundtrip.nodes, "nodes mismatch");
  assert_eq!(norm_original.edges, norm_roundtrip.edges, "edges mismatch");
  assert_eq!(
    norm_original.scopes, norm_roundtrip.scopes,
    "scopes mismatch"
  );

  // 전체 구조 동일성 확인
  assert_eq!(
    norm_original, norm_roundtrip,
    "Round-trip structural equivalence failed"
  );
}

#[test]
fn roundtrip_dsl_generation_is_deterministic() {
  let fx = make_comprehensive_test_graph();
  let meta = describe_fxcore(&fx);

  let dsl1 = generate_pnix_dsl(&meta);
  let dsl2 = generate_pnix_dsl(&meta);

  assert_eq!(dsl1, dsl2, "DSL generation must be deterministic");
}

#[test]
fn roundtrip_preserves_all_node_attributes() {
  let original = make_comprehensive_test_graph();
  let meta = describe_fxcore(&original);
  let dsl = generate_pnix_dsl(&meta);

  let mut diags = Diagnostics::default();
  let ast = parse_module(&dsl, "test", &mut diags).unwrap();
  let surface = lower_to_surface(&ast, &mut diags).unwrap();
  let roundtrip = lower_to_fxcore(&surface, &mut diags).unwrap();

  // Gate 노드 검증
  let orig_gate = original
    .nodes
    .iter()
    .find(|n| n.name == "validate")
    .unwrap();
  let rt_gate = roundtrip
    .nodes
    .iter()
    .find(|n| n.name == "validate")
    .unwrap();
  assert_eq!(orig_gate.kind, rt_gate.kind, "gate kind preserved");
  assert_eq!(orig_gate.priority, rt_gate.priority, "priority preserved");
  assert_eq!(orig_gate.cost, rt_gate.cost, "cost preserved");

  // Optional 노드 검증
  let orig_opt = original.nodes.iter().find(|n| n.name == "render").unwrap();
  let rt_opt = roundtrip.nodes.iter().find(|n| n.name == "render").unwrap();
  assert_eq!(orig_opt.optional, rt_opt.optional, "optional preserved");
  assert_eq!(orig_opt.scope, rt_opt.scope, "scope preserved");
}

#[test]
fn roundtrip_preserves_conditional_edges() {
  let original = make_comprehensive_test_graph();
  let meta = describe_fxcore(&original);
  let dsl = generate_pnix_dsl(&meta);

  let mut diags = Diagnostics::default();
  let ast = parse_module(&dsl, "test", &mut diags).unwrap();
  let surface = lower_to_surface(&ast, &mut diags).unwrap();
  let roundtrip = lower_to_fxcore(&surface, &mut diags).unwrap();

  // 조건부 엣지 검증
  let cond_edge = roundtrip
    .edges
    .iter()
    .find(|e| e.cond.is_some())
    .expect("conditional edge should exist");

  assert!(
    matches!(&cond_edge.cond, Some(EdgeCond::When(g)) if g == "validate"),
    "when condition preserved"
  );
}

#[test]
fn roundtrip_preserves_scope_definitions() {
  let original = make_comprehensive_test_graph();
  let meta = describe_fxcore(&original);
  let dsl = generate_pnix_dsl(&meta);

  let mut diags = Diagnostics::default();
  let ast = parse_module(&dsl, "test", &mut diags).unwrap();
  let surface = lower_to_surface(&ast, &mut diags).unwrap();
  let roundtrip = lower_to_fxcore(&surface, &mut diags).unwrap();

  // 스코프 정의 검증
  let scope = roundtrip
    .scopes
    .iter()
    .find(|s| s.name == "output")
    .expect("output scope should exist");

  assert_eq!(scope.policy, ScopePolicy::Isolate, "scope policy preserved");

  // 스코프에 속한 노드 검증
  let render_node = roundtrip.nodes.iter().find(|n| n.name == "render").unwrap();
  assert_eq!(
    render_node.scope, "output",
    "node scope assignment preserved"
  );
}
