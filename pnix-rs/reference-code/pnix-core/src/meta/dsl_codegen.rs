//! MetaFx → pnix DSL 선언 생성기
//!
//! Meta-circular Stage 1: pnix이 자기 자신을 pnix 언어로 기술
//!
//! 실행 ❌ / 의미 검증 ❌ / 컴파일 ❌
//! 순수 텍스트 생성만

use super::{MetaFxEdge, MetaFxInput, MetaFxModule, MetaFxMorphism, MetaFxNode, MetaFxScope};

/// Generate pnix DSL from MetaFx (pure text generation)
///
/// 입력: MetaFxModule (self-description IR)
/// 출력: pnix DSL 선언 텍스트
///
/// 제약:
/// - FxCore 참조 ❌
/// - executor 참조 ❌
/// - verify/lowering ❌
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn generate_pnix_dsl(meta: &MetaFxModule) -> String {
  let mut out = String::new();

  // 헤더 (고정)
  out.push_str("# Generated from MetaFx (self-description)\n");
  out.push_str("# DO NOT EDIT\n");
  out.push_str(&format!("# name: {}\n", meta.name));
  out.push_str(&format!("# stage: {}\n", meta.stage));
  if let Some(ref hash) = meta.replay_hash {
    out.push_str(&format!("# replay_hash: {}\n", hash));
  }
  out.push('\n');

  // 타입 선언
  for ty in &meta.types {
    out.push_str(&format!("type {}\n", ty));
  }
  if !meta.types.is_empty() {
    out.push('\n');
  }

  // 입력 선언 (Stage-2)
  for input in &meta.inputs {
    out.push_str(&format_input(input));
    out.push('\n');
  }
  if !meta.inputs.is_empty() {
    out.push('\n');
  }

  // extern 선언 (morphism)
  for morphism in &meta.morphisms {
    out.push_str(&format_morphism(morphism));
    out.push('\n');
  }
  if !meta.morphisms.is_empty() {
    out.push('\n');
  }

  // 스코프 선언 (global 제외)
  for scope in &meta.scopes {
    out.push_str(&format_scope(scope));
    out.push('\n');
  }
  if !meta.scopes.is_empty() {
    out.push('\n');
  }

  // 노드 선언
  for node in &meta.nodes {
    out.push_str(&format_node(node));
    out.push('\n');
  }
  out.push('\n');

  // 엣지 선언
  for edge in &meta.edges {
    out.push_str(&format_edge(edge));
    out.push('\n');
  }

  out
}

/// 입력을 DSL로 변환
fn format_input(i: &MetaFxInput) -> String {
  format!("input {} : {}", i.name, i.ty)
}

/// morphism(extern)을 DSL로 변환
///
/// 규칙:
/// - Stage-1: extern name : In -> Out
/// - Stage-2: extern name : (p: T, ...) -> (p: T, ...)
fn format_morphism(m: &MetaFxMorphism) -> String {
  if m.inputs.is_empty() && m.outputs.is_empty() {
    // Stage-1 simple syntax
    format!("extern {} : {} -> {}", m.name, m.input, m.output)
  } else {
    // Stage-2 ported syntax
    let inputs_str = m
      .inputs
      .iter()
      .map(|p| format!("{}: {}", p.name, p.ty))
      .collect::<Vec<_>>()
      .join(", ");
    let outputs_str = m
      .outputs
      .iter()
      .map(|p| format!("{}: {}", p.name, p.ty))
      .collect::<Vec<_>>()
      .join(", ");
    format!("extern {} : ({}) -> ({})", m.name, inputs_str, outputs_str)
  }
}

/// 노드를 DSL로 변환
///
/// 규칙:
/// - default 값은 출력하지 않음
/// - kind=normal, scope=global, cost=medium, priority=0 은 생략
fn format_node(n: &MetaFxNode) -> String {
  let mut parts = vec![format!("node {} uses {}", n.name, n.uses)];

  // gate 키워드 (kind != normal)
  if n.kind == "gate" {
    parts.push("gate".into());
  }

  // optional 키워드
  if n.optional {
    parts.push("optional".into());
  }

  // scope (global 아니면 출력)
  if n.scope != "global" {
    parts.push(format!("scope {}", n.scope));
  }

  // cost (medium 아니면 출력)
  if n.cost != "medium" {
    parts.push(format!("cost {}", n.cost));
  }

  // priority (0 아니면 출력)
  if n.priority != 0 {
    parts.push(format!("priority {}", n.priority));
  }

  parts.join(" ")
}

/// 엣지를 DSL로 변환
///
/// 규칙:
/// - 포트 정보 있으면 a.port -> b.port 형식
/// - 조건 있으면 when/unless/onfail 추가
fn format_edge(e: &MetaFxEdge) -> String {
  let mut from_str = e.from.clone();
  let mut to_str = e.to.clone();

  // 포트 정보 추가
  if let Some(ref port_info) = e.port_info {
    if let Some(ref fp) = port_info.from_port {
      from_str = format!("{}.{}", e.from, fp);
    }
    if let Some(ref tp) = port_info.to_port {
      to_str = format!("{}.{}", e.to, tp);
    }
  }

  let mut result = format!("edge {} -> {}", from_str, to_str);

  // 조건 추가 (when:g1 → when g1 형식으로 변환)
  if let Some(ref cond) = e.cond {
    if let Some(gate) = cond.strip_prefix("when:") {
      result.push_str(&format!(" when {}", gate));
    } else if let Some(gate) = cond.strip_prefix("unless:") {
      result.push_str(&format!(" unless {}", gate));
    } else if let Some(node) = cond.strip_prefix("onfail:") {
      result.push_str(&format!(" onfail {}", node));
    }
  }

  result
}

/// 스코프를 DSL로 변환
fn format_scope(s: &MetaFxScope) -> String {
  format!("scope {} policy {}", s.name, s.policy)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::{
    CostHint, EdgeCond, ExecutionContract, FxCoreMeta, FxCoreModule, FxEdge, FxNode, FxScope,
    NodeKind, ScopePolicy,
  };
  use crate::meta::describe_fxcore;

  fn make_test_graph() -> FxCoreModule {
    FxCoreModule {
      meta: FxCoreMeta {
        version: "fxcore@0.1".into(),
        stage: 4,
        replay_hash: Some("test_hash_123".into()),
      },
      name: "test-graph".into(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![
        FxNode {
          name: "solve".into(),
          uses: "clojure.solve-linear".into(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".into(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
        FxNode {
          name: "check".into(),
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
          uses: "deno.render-html".into(),
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
        FxEdge::simple("solve".into(), "check".into()),
        FxEdge::simple("check".into(), "render".into()).with_cond(EdgeCond::When("check".into())),
      ],
      scopes: vec![FxScope {
        name: "output".into(),
        nodes: vec!["render".into()],
        policy: ScopePolicy::Isolate,
      }],
    }
  }

  #[test]
  fn generates_basic_pnix_dsl() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let dsl = generate_pnix_dsl(&meta);

    assert!(dsl.contains("node"));
    assert!(dsl.contains("edge"));
    assert!(dsl.contains("# Generated from MetaFx"));
  }

  #[test]
  fn dsl_generation_is_deterministic() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);

    let dsl1 = generate_pnix_dsl(&meta);
    let dsl2 = generate_pnix_dsl(&meta);

    assert_eq!(dsl1, dsl2, "DSL generation must be deterministic");
  }

  #[test]
  fn dsl_does_not_output_defaults() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let dsl = generate_pnix_dsl(&meta);

    // default 값은 출력하지 않음
    assert!(
      !dsl.contains("cost medium"),
      "should not output default cost"
    );
    assert!(
      !dsl.contains("scope global"),
      "should not output default scope"
    );
    assert!(
      !dsl.contains("priority 0"),
      "should not output default priority"
    );
  }

  #[test]
  fn dsl_outputs_non_default_values() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let dsl = generate_pnix_dsl(&meta);

    // non-default 값은 출력
    assert!(dsl.contains("gate"), "should output gate keyword");
    assert!(dsl.contains("optional"), "should output optional keyword");
    assert!(
      dsl.contains("scope output"),
      "should output non-global scope"
    );
    assert!(dsl.contains("cost tiny"), "should output non-medium cost");
    assert!(dsl.contains("cost heavy"), "should output non-medium cost");
    assert!(
      dsl.contains("priority 10"),
      "should output non-zero priority"
    );
  }

  #[test]
  fn dsl_outputs_edges_with_conditions() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let dsl = generate_pnix_dsl(&meta);

    assert!(
      dsl.contains("edge solve -> check"),
      "should output simple edge"
    );
    assert!(dsl.contains("when check"), "should output conditional edge");
  }

  #[test]
  fn dsl_outputs_scope_declarations() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let dsl = generate_pnix_dsl(&meta);

    assert!(
      dsl.contains("scope output policy isolate"),
      "should output scope declaration"
    );
  }

  #[test]
  fn dsl_contains_header_metadata() {
    let fx = make_test_graph();
    let meta = describe_fxcore(&fx);
    let dsl = generate_pnix_dsl(&meta);

    assert!(dsl.contains("# name: test-graph"));
    assert!(dsl.contains("# stage: 4"));
    assert!(dsl.contains("# replay_hash: test_hash_123"));
  }

  #[test]
  fn format_node_with_all_options() {
    use crate::meta::MetaExecutionContract;

    let node = MetaFxNode {
      name: "heavy-compute".into(),
      uses: "py.compute".into(),
      kind: "gate".into(),
      optional: true,
      scope: "compute".into(),
      cost: "xheavy".into(),
      priority: 99,
      contract: MetaExecutionContract::default(),
    };

    let result = format_node(&node);

    assert!(result.contains("node heavy-compute uses py.compute"));
    assert!(result.contains("gate"));
    assert!(result.contains("optional"));
    assert!(result.contains("scope compute"));
    assert!(result.contains("cost xheavy"));
    assert!(result.contains("priority 99"));
  }

  #[test]
  fn format_edge_with_ports() {
    let edge = MetaFxEdge {
      from: "a".into(),
      to: "b".into(),
      cond: None,
      port_info: Some(super::super::MetaFxPortInfo {
        from_port: Some("out".into()),
        to_port: Some("in".into()),
      }),
    };

    let result = format_edge(&edge);
    assert_eq!(result, "edge a.out -> b.in");
  }

  #[test]
  fn format_edge_with_unless_condition() {
    let edge = MetaFxEdge {
      from: "a".into(),
      to: "b".into(),
      cond: Some("unless:gate1".into()),
      port_info: None,
    };

    let result = format_edge(&edge);
    assert_eq!(result, "edge a -> b unless gate1");
  }

  #[test]
  fn format_edge_with_onfail_condition() {
    let edge = MetaFxEdge {
      from: "fallback".into(),
      to: "recover".into(),
      cond: Some("onfail:main".into()),
      port_info: None,
    };

    let result = format_edge(&edge);
    assert_eq!(result, "edge fallback -> recover onfail main");
  }
}
